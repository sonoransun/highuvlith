#!/usr/bin/env python3
"""Regenerate every simulator-driven figure (light + dark PNG variants).

Usage (from the repository root, with the extension built; see docs/figures/README.md):

    python docs/figures/sim/make_all.py                 # everything -> docs/assets/images/sim/
    python docs/figures/sim/make_all.py --only talbot   # names containing "talbot"
    python docs/figures/sim/make_all.py --group sources
    python docs/figures/sim/make_all.py --fast          # coarse sampling -> docs/figures/sim/_fast/
    python docs/figures/sim/make_all.py --redraw        # reuse cached simulation data (_cache/)
    python docs/figures/sim/make_all.py --list          # name, group, pages, status
    python docs/figures/sim/make_all.py --catalog       # Markdown embed snippets
    python docs/figures/sim/make_all.py --check         # outputs exist + every listed page embeds them
    python docs/figures/sim/make_all.py --sync-embeds   # rewrite existing page embeds from the current alt/caption

Every figure is computed by the simulator (``highuvlith._native``) or read from a
cited dataset; nothing is hand-drawn. A full release-build run takes a few minutes.
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

import common  # noqa: E402
import style  # noqa: E402

FIGURE_MODULES = ("fig_imaging", "fig_masks", "fig_sources", "fig_deep", "fig_optim", "fig_site")


def load_registry() -> list[common.FigureSpec]:
    import importlib

    for mod in FIGURE_MODULES:
        if (HERE / f"{mod}.py").exists():
            importlib.import_module(mod)
    return common.REGISTRY


def select(specs, only: list[str] | None, group: str | None, include_disabled: bool):
    chosen = []
    for spec in specs:
        if group and spec.group != group:
            continue
        if only and not any(pat in spec.name or fnmatch.fnmatch(spec.name, pat) for pat in only):
            continue
        if not spec.enabled and not include_disabled and not only:
            continue
        chosen.append(spec)
    return chosen


def run(specs, out_dir: Path, fast: bool, redraw: bool) -> int:
    caps = common.capabilities()
    failures = 0
    t_all = time.perf_counter()
    for spec in specs:
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
            style.render_variants(spec.draw, data, stem, meta, spec.fmt)
            sizes = sum(p.stat().st_size for p in spec.image_paths(out_dir))
            print(f"  ok    {spec.name:44s} sim {t1 - t0:6.1f} s  draw {time.perf_counter() - t1:5.1f} s  {sizes / 1024:6.0f} KiB")
        except Exception as exc:  # keep going; report at the end
            failures += 1
            print(f"  FAIL  {spec.name:44s} {type(exc).__name__}: {exc}")
    print(f"done in {time.perf_counter() - t_all:.1f} s, {failures} failure(s)")
    return failures


def check(specs, out_dir: Path) -> int:
    problems = 0
    for spec in specs:
        for p in spec.image_paths(out_dir):
            if not p.exists():
                print(f"missing image: {p.relative_to(common.REPO)}")
                problems += 1
        for page in spec.pages:
            path = common.REPO / page if page == "README.md" else common.DOCS / page
            if not path.exists():
                print(f"{spec.name}: page {page} does not exist (yet)")
                continue
            text = path.read_text(encoding="utf-8")
            for p in spec.image_paths(out_dir):
                if f"/{spec.rel_stem}-{p.stem.rsplit('-', 1)[1]}.{spec.fmt}" not in text:
                    print(f"{spec.name}: {page} does not embed {p.name}")
                    problems += 1
    total = sum(p.stat().st_size for s in specs for p in s.image_paths(out_dir) if p.exists())
    print(f"{len(specs)} figures, {total / 2**20:.2f} MiB of images, {problems} problem(s)")
    return problems


def sync_embeds(specs) -> int:
    """Replace every existing embed of ``specs`` in their pages with the current ``md_embed`` text.

    Only blocks that already exist are rewritten (``<figure>…</figure>`` on docs pages,
    ``<picture>…</picture>`` in the README), so captions edited in the figure modules
    propagate without hand-editing the pages; new embeds are still placed by hand.
    """
    changed = 0
    for spec in specs:
        for page in spec.pages:
            path = common.REPO / page if page == "README.md" else common.DOCS / page
            if not path.exists():
                continue
            text = path.read_text(encoding="utf-8")
            open_tag, close_tag = ("<picture>", "</picture>") if page == "README.md" else ('<figure markdown="span">', "</figure>")
            marker = f"/{spec.rel_stem}-light.{spec.fmt}"
            pos = text.find(marker)
            if pos < 0:
                continue
            start = text.rfind(open_tag, 0, pos)
            end = text.find(close_tag, pos)
            if start < 0 or end < 0:
                print(f"{spec.name}: could not delimit the embed in {page}")
                continue
            end += len(close_tag)
            new = common.md_embed(spec, page)
            if text[start:end] != new:
                path.write_text(text[:start] + new + text[end:], encoding="utf-8")
                print(f"updated {spec.name} in {page}")
                changed += 1
    print(f"{changed} embed(s) updated")
    return 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--only", nargs="+", metavar="NAME", help="figures whose name contains NAME (or matches a glob)")
    ap.add_argument("--group", help="imaging | masks | sources | deep | optim | site")
    ap.add_argument("--fast", action="store_true", help="coarse sampling for a quick smoke run (writes to _fast/)")
    ap.add_argument("--redraw", action="store_true", help="reuse cached simulation data where available")
    ap.add_argument("--out", type=Path, help="output directory (default docs/assets/images/sim, or _fast/)")
    ap.add_argument("--all", action="store_true", help="include figures that are disabled by default")
    ap.add_argument("--list", action="store_true")
    ap.add_argument("--catalog", action="store_true", help="print Markdown embed snippets for every page")
    ap.add_argument("--check", action="store_true", help="verify outputs exist and pages embed them")
    ap.add_argument("--sync-embeds", action="store_true", help="rewrite existing page embeds from the current captions")
    args = ap.parse_args(argv)

    specs = select(load_registry(), args.only, args.group, args.all)
    out_dir = args.out or (common.FAST_DIR if args.fast else common.OUT_DIR)

    if args.list:
        for s in specs:
            flag = "" if s.enabled else f"  [disabled: {s.disabled_reason}]"
            print(f"{s.name:44s} {s.group:8s} {', '.join(s.pages) or '-'}{flag}")
        return 0
    if args.catalog:
        for s in specs:
            for page in s.pages or ("docs/<page>.md",):
                print(f"<!-- {s.name} -> {page} -->")
                print(common.md_embed(s, page))
                print()
        return 0
    if args.check:
        return 1 if check(specs, out_dir) else 0
    if args.sync_embeds:
        return sync_embeds(specs)
    return 1 if run(specs, out_dir, args.fast, args.redraw) else 0


if __name__ == "__main__":
    sys.exit(main())
