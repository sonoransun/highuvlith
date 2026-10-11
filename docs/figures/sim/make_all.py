#!/usr/bin/env python3
"""Regenerate, verify and check every simulator-driven figure (light + dark variants).

Usage (from the repository root, with the extension built; see docs/figures/README.md):

    python docs/figures/sim/make_all.py                 # everything -> docs/assets/images/sim/
    python docs/figures/sim/make_all.py --only talbot   # names containing "talbot"
    python docs/figures/sim/make_all.py --group sources # imaging | masks | sources | deep | optim | site | home | anim | materials | frontends | deep_b | sources_b
    python docs/figures/sim/make_all.py --fast          # coarse sampling -> docs/figures/sim/_fast/
    python docs/figures/sim/make_all.py --fast --frames 4   # animations with 4 frames (smoke run)
    python docs/figures/sim/make_all.py --redraw        # reuse cached simulation data (_cache/)
    python docs/figures/sim/make_all.py --require cli   # a missing capability is an error, not a skip
    python docs/figures/sim/make_all.py --verify        # run every compute() and its asserts; draw nothing, write nothing (what CI runs)
    python docs/figures/sim/make_all.py --list          # name, group, pages, status
    python docs/figures/sim/make_all.py --catalog [--pages <paths>]     # Markdown embed snippets (page groups paste these)
    python docs/figures/sim/make_all.py --check [--strict|--tolerant] [--pages <paths>]   # both registries, sizes, pages, captions, legend, matrix map, readiness
    python docs/figures/sim/make_all.py --sync-embeds --pages <your pages>   # rewrite existing embeds on YOUR pages only
    python docs/figures/sim/make_all.py --sync-embeds --site-wide            # INT only, once
    python docs/figures/sim/make_all.py --tables [--only NAME] [--write]     # marker tables declared by extra["table"] (print; --write = INT / nightly)
    python docs/figures/sim/make_all.py --export-registry [path]            # diagnostic JSON (gitignored, nothing reads it)

Every figure is computed by the simulator (``highuvlith._native``) or read from a
cited dataset; nothing is hand-drawn. Physical asserts live in ``compute()`` so
``--verify`` exercises them without rendering (DESIGN §8.3; CI never renders a
platform-dependent figure).
"""

from __future__ import annotations

import argparse
import fnmatch
import pickle
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))

import common  # noqa: E402
import registry  # noqa: E402
import style  # noqa: E402

#: (module, group that lands it) — a module whose file is absent is skipped with a printed note
FIGURE_MODULES = (
    ("fig_imaging", "baseline"), ("fig_masks", "baseline"), ("fig_sources", "baseline"), ("fig_deep", "baseline"),
    ("fig_optim", "baseline"), ("fig_site", "baseline"),
    ("fig_home", "V1a"), ("fig_anim", "V1c"), ("fig_materials", "V2b"), ("fig_frontends", "V5"),
    ("fig_deep_b", "V2b"), ("fig_sources_b", "V3b"),
)
GROUPS = ("imaging", "masks", "sources", "deep", "optim", "site", "home", "anim", "materials", "frontends", "deep_b", "sources_b")

_loaded = False


def load_registry(verbose: bool = True) -> list[common.FigureSpec]:
    """Import every figure module (absent ones print a note) and return the FigureSpec registry."""
    global _loaded
    import importlib  # noqa: PLC0415

    if not _loaded:
        _loaded = True
        for mod, group in FIGURE_MODULES:
            if (HERE / f"{mod}.py").exists():
                importlib.import_module(mod)
            elif verbose:
                print(f"  note  {mod}.py not present (lands with {group})")
    return common.REGISTRY


def select(specs, only: list[str] | None, group: str | None, include_disabled: bool, pages: list[str] | None = None):
    chosen = []
    for spec in specs:
        if group and spec.group != group:
            continue
        if only and not any(pat in spec.name or fnmatch.fnmatch(spec.name, pat) for pat in only):
            continue
        if not spec.enabled and not include_disabled and not only:
            continue
        if pages and not any(registry._page_selected(p, pages) for p in spec.pages):
            continue
        chosen.append(spec)
    return chosen


def _require(caps: dict[str, bool], required: list[str] | None) -> None:
    missing = [r for r in (required or []) if not caps.get(r, False)]
    if missing:
        raise SystemExit(f"required capability missing: {', '.join(missing)} (build the CLI with `cargo build --release -p highuvlith-cli`, or set HUV_CLI)")


def run(specs, out_dir: Path, fast: bool, redraw: bool, required: list[str] | None = None, frames: int | None = None) -> int:
    caps = common.capabilities()
    _require(caps, required)
    failures = 0
    t_all = time.perf_counter()
    common.FRAME_LIMIT = frames
    for spec in specs:
        if spec.is_crop:
            continue  # written by the parent's render
        missing = [r for r in spec.requires if not caps.get(r, False)]
        if missing:
            print(f"  skip  {spec.name:44s} needs {', '.join(missing)} (not in this build)")
            continue
        if not spec.enabled:
            print(f"  note  {spec.name:44s} disabled by default: {spec.disabled_reason}")
        cache = common.CACHE_DIR / f"{spec.name}{'-fast' if fast else ''}.pkl"
        t0 = time.perf_counter()
        try:
            if redraw and cache.exists():
                data = pickle.loads(cache.read_bytes())
            else:
                data = spec.compute(fast)
                cache.parent.mkdir(parents=True, exist_ok=True)
                cache.write_bytes(pickle.dumps(data))
            t1 = time.perf_counter()
            meta = {
                "Title": spec.name,
                "Description": spec.alt,
                "Source": f"docs/figures/sim/{spec.compute.__module__}.py (make_all.py --only {spec.name})",
                "Software": "highuvlith figure scripts (docs/figures/sim)",
            }
            stem = out_dir / spec.rel_stem
            crops = {c.name[len(spec.name) + 1:]: c.extra.get("box") for c in common.REGISTRY if c.extra.get("crop_of") == spec.name}
            style.render_variants(spec.draw, data, stem, meta, spec.fmt, spec=spec, crops=crops, frames_limit=frames)
            paths = [p for p in spec.image_paths(out_dir) if p.exists()]
            sizes = sum(p.stat().st_size for p in paths)
            print(f"  ok    {spec.name:44s} sim {t1 - t0:6.1f} s  draw {time.perf_counter() - t1:5.1f} s  {sizes / 1024:6.0f} KiB")
        except Exception as exc:  # keep going; report at the end
            failures += 1
            print(f"  FAIL  {spec.name:44s} {type(exc).__name__}: {exc}")
    print(f"done in {time.perf_counter() - t_all:.1f} s, {failures} failure(s)")
    return failures


def verify(specs, fast: bool, required: list[str] | None = None) -> int:
    """Run every selected spec's ``compute()`` (and the physical asserts inside it); nothing is drawn or written."""
    caps = common.capabilities()
    _require(caps, required)
    failures = 0
    t_all = time.perf_counter()
    for spec in specs:
        if spec.is_crop:
            continue
        missing = [r for r in spec.requires if not caps.get(r, False)]
        if missing:
            print(f"  skip  {spec.name:44s} needs {', '.join(missing)} (not in this build)")
            continue
        t0 = time.perf_counter()
        try:
            spec.compute(fast)
            print(f"  ok    {spec.name:44s} compute {time.perf_counter() - t0:6.1f} s")
        except Exception as exc:  # noqa: BLE001
            failures += 1
            print(f"  FAIL  {spec.name:44s} {type(exc).__name__}: {exc}")
    print(f"verified {len(specs)} spec(s) in {time.perf_counter() - t_all:.1f} s, {failures} failure(s); nothing written")
    return failures


def tables(specs, write: bool, fast: bool) -> int:
    """Print (or ``--write`` between markers) every table a spec declares through ``extra['table']``."""
    found = 0
    for spec in specs:
        decl = spec.extra.get("table")
        if not decl:
            continue
        found += 1
        render = decl.get("render")
        if render is None:
            print(f"error: {spec.name}: extra['table'] has no 'render' callable")
            return 1
        cache = common.CACHE_DIR / f"{spec.name}{'-fast' if fast else ''}.pkl"
        data = pickle.loads(cache.read_bytes()) if cache.exists() else spec.compute(fast)
        text = render(data).strip("\n")
        page, marker = decl["page"], decl["marker"]
        start, end = f"<!-- {marker}:start -->", f"<!-- {marker}:end -->"
        if not write:
            print(f"<!-- {spec.name} -> {page} between {start} / {end} -->")
            print(text)
            print()
            continue
        path = registry.page_path(page)
        if not path.exists():
            print(f"warning: {page} does not exist; nothing written")
            continue
        body = path.read_text(encoding="utf-8")
        a, b = body.find(start), body.find(end)
        if a < 0 or b < 0:
            print(f"warning: {page} has no {start} … {end} block; nothing written")
            continue
        new = body[:a] + f"{start}\n{text}\n" + body[b:]
        if new != body:
            path.write_text(new, encoding="utf-8")
            print(f"updated {marker} table in {page}")
    if not found:
        print("no spec declares extra['table']")
    return 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--only", nargs="+", metavar="NAME", help="figures whose name contains NAME (or matches a glob)")
    ap.add_argument("--group", choices=GROUPS)
    ap.add_argument("--fast", action="store_true", help="coarse sampling for a quick smoke run (writes to _fast/)")
    ap.add_argument("--frames", type=int, metavar="N", help="cap animation frame lists at N (smoke runs)")
    ap.add_argument("--redraw", action="store_true", help="reuse cached simulation data where available")
    ap.add_argument("--out", type=Path, help="output directory (default docs/assets/images/sim, or _fast/)")
    ap.add_argument("--all", action="store_true", help="include figures that are disabled by default")
    ap.add_argument("--require", nargs="+", metavar="CAP", help="capabilities that must be present (cli, illumination, …)")
    ap.add_argument("--verify", action="store_true", help="run every compute() without drawing or writing")
    ap.add_argument("--list", action="store_true")
    ap.add_argument("--catalog", action="store_true", help="print Markdown embed snippets for every page")
    ap.add_argument("--pages", nargs="+", metavar="PATH", help="restrict --check / --catalog / --sync-embeds to these pages")
    ap.add_argument("--check", action="store_true", help="verify outputs, pages, captions, legend, matrix map, readiness")
    ap.add_argument("--strict", action="store_true")
    ap.add_argument("--tolerant", action="store_true")
    ap.add_argument("--sync-embeds", action="store_true", help="rewrite existing page embeds from the current captions")
    ap.add_argument("--site-wide", action="store_true", help="INT only: sync every page")
    ap.add_argument("--tables", action="store_true", help="print the marker tables specs declare")
    ap.add_argument("--write", action="store_true", help="with --tables: replace the tables in place (INT / nightly)")
    ap.add_argument("--export-registry", nargs="?", const="-", metavar="PATH", help="diagnostic JSON export")
    args = ap.parse_args(argv)

    specs = select(load_registry(), args.only, args.group, args.all, args.pages)
    out_dir = args.out or (common.FAST_DIR if args.fast else common.OUT_DIR)

    if args.list:
        for s in specs:
            flag = "" if s.enabled else f"  [disabled: {s.disabled_reason}]"
            print(f"{s.name:44s} {s.group:10s} {s.kind:9s} {', '.join(s.pages) or '-'}{flag}")
        return 0
    if args.catalog or args.check or args.sync_embeds or args.export_registry is not None:
        import check_figures  # noqa: PLC0415

        check_figures.load_asset_modules(verbose=True)
        everything = registry.all_specs()
        chosen = check_figures.select(everything, args.only, args.pages) if (args.only or args.pages) else everything
        if args.group:
            chosen = [s for s in chosen if getattr(s, "group", "") == args.group]
        if args.catalog:
            print(registry.catalog(chosen, args.pages))
            return 0
        if args.sync_embeds:
            registry.sync_embeds(chosen, args.pages, site_wide=args.site_wide)
            return 0
        if args.export_registry is not None:
            text = registry.export_json(chosen, None if args.export_registry == "-" else Path(args.export_registry))
            if args.export_registry == "-":
                print(text)
            else:
                print(f"wrote {args.export_registry} (diagnostic; gitignored)")
            return 0
        import os  # noqa: PLC0415

        strict = check_figures.STRICT_DEFAULT or os.environ.get("HUV_FIGCHECK_STRICT") == "1"
        strict = True if args.strict else (False if args.tolerant else strict)
        errors, warnings = check_figures.run_check(chosen, strict=strict, pages=args.pages, out_dir=args.out)
        return check_figures.report(errors, warnings, "make_all --check" + (" (strict)" if strict else " (tolerant)"))
    if args.tables:
        return tables(specs, args.write, args.fast)
    if args.verify:
        return 1 if verify(specs, args.fast, args.require) else 0
    return 1 if run(specs, out_dir, args.fast, args.redraw, args.require, args.frames) else 0


if __name__ == "__main__":
    sys.exit(main())
