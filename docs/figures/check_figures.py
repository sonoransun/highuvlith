#!/usr/bin/env python3
"""Check, catalogue and sync every registered asset — both registries (DESIGN §8.3, §9.0 rule 4).

    python docs/figures/check_figures.py --check [--strict | --tolerant] [--pages <paths>]
    python docs/figures/check_figures.py --catalog [--pages <paths>] [--only NAME …]
    python docs/figures/check_figures.py --sync-embeds --pages <paths> | --site-wide
    python docs/figures/check_figures.py --export-registry [path]
    python docs/figures/check_figures.py --platform-independent [--regenerate]
    python docs/figures/check_figures.py --caption-labels          # diagnostic: every caption clause and how it resolves

``--check`` runs with ``STRICT_DEFAULT = True`` since INT flipped it (``--tolerant`` restores
the pre-INT mode; ``HUV_FIGCHECK_STRICT=1`` forces strict). **Asset-side findings are errors in every mode**: a missing variant, a
size over its kind cap (§10.2; a ``max_kb`` override without ``max_kb_reason``), SMIL or
``<script>`` in an SVG, ``-anim-`` without ``-still-``, a missing ``<title>``/``<desc>``,
the reduced-motion rule missing where ``@keyframes`` is present, a scripted SVG whose
``font-family`` is not ``tokens.FONT_SVG``, a sha256 mismatch on a capture, a legend block
that is present but differs, a matrix-map tile count that differs from the matrix, and
every caption ``Model:`` clause that does not resolve to a matrix row with the row's
headline glyph (:func:`check_captions`, DESIGN §5.0). **Page-side findings are warnings
while tolerant**: a listed page that does not embed the asset (or does not exist yet), a
page caption that differs from the registry, an unregistered file under
``docs/assets/images/``, an absent ``huv-legend`` marker block, an alt over 300 characters.
The readiness check (V4c's ``future/check_readiness.py`` + ``readiness.csv``) is skipped
with a printed note while those files are absent, and an error in strict mode.

``--sync-embeds`` takes the same ``--pages`` restriction as ``--check``: without it the
command is refused unless ``--site-wide`` (INT's flag) is given (DESIGN §9.1).
"""

from __future__ import annotations

import argparse
import html as html_mod
import importlib.util
import os
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE / "reference"))
sys.path.insert(0, str(HERE / "sim"))

import matrix_parse  # noqa: E402
import registry  # noqa: E402
import tokens  # noqa: E402

REPO = registry.REPO
IMAGES = registry.IMAGES
STRICT_DEFAULT = True  # flipped by INT (DESIGN §9.0 rule 4); --tolerant restores the pre-INT mode

#: Generator modules holding AssetSpecs, by path under docs/figures/; absent ones are tolerated with a note.
ASSET_MODULES: tuple[tuple[str, str], ...] = (
    ("diagrams/make_spectrum_strip.py", "V0"),
    ("diagrams/make_pipeline_diagrams.py", "V0"),
    ("reference/make_matrix_map.py", "V0"),
    ("diagrams/make_physics_schematics.py", "V1c"),
    ("diagrams/make_process_schematics.py", "V2c"),
    ("diagrams/make_source_schematics.py", "V3c"),
    ("history/make_history_figures.py", "V4a"),
    ("history/make_history_schematics.py", "V4a"),
    ("history/make_history_animation.py", "V4a"),
    ("nodes/make_node_figures.py", "V4b"),
    ("nodes/make_node_schematics.py", "V4b"),
    ("future/make_future_figures.py", "V4c"),
    ("future/make_future_schematics.py", "V4c"),
    ("gui/make_gui_media.py", "V5"),
    ("playground/make_playground_figures.py", "V5"),
)

_loaded_assets = False


def _import_by_path(path: Path):
    name = "huvfig_" + path.relative_to(HERE).with_suffix("").as_posix().replace("/", "_")
    if name in sys.modules:
        return sys.modules[name]
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


def load_asset_modules(verbose: bool = True) -> None:
    """Import every generator module that registers AssetSpecs (absent ones print a note)."""
    global _loaded_assets
    if _loaded_assets:
        return
    _loaded_assets = True
    for rel, group in ASSET_MODULES:
        path = HERE / rel
        if not path.exists():
            if verbose:
                print(f"  note  docs/figures/{rel} not present (lands with {group})")
            continue
        try:
            _import_by_path(path)
        except Exception as exc:  # noqa: BLE001
            print(f"  FAIL  importing docs/figures/{rel}: {type(exc).__name__}: {exc}")
            raise


def load_all(verbose: bool = True) -> list:
    """Both registries: the sim FigureSpecs (via make_all.load_registry) and every AssetSpec."""
    import make_all  # noqa: PLC0415

    make_all.load_registry(verbose=verbose)
    load_asset_modules(verbose=verbose)
    return registry.all_specs()


def select(specs, only: list[str] | None = None, pages: list[str] | None = None) -> list:
    import fnmatch  # noqa: PLC0415

    out = []
    for s in specs:
        if only and not any(pat in s.name or fnmatch.fnmatch(s.name, pat) for pat in only):
            continue
        if pages and not any(registry._page_selected(p, pages) for p in s.pages):
            continue
        out.append(s)
    return out


# ---------------------------------------------------------------------------
# Captions: the Model trailer -> matrix rows (DESIGN §5.0)
# ---------------------------------------------------------------------------

_GLYPH = re.compile("[✅🔶🧪🗺]️?")
_TAG = re.compile(r"<[^>]+>")
_WS = re.compile(r"\s+")
_LEAD = re.compile(r"^(?:and|with|the|a|an|plus)\s+", re.I)


def strip_html(s: str) -> str:
    return _WS.sub(" ", html_mod.unescape(_TAG.sub("", s))).strip()


def caption_trailer(caption: str) -> tuple[str, str]:
    """(mode, text): ``canonical`` = the ``<em>…</em>`` trailer; ``legacy`` = the text after the last
    ``Model(s):``; ``freeform`` = the whole caption (a baseline caption with no Model sentence)."""
    m = re.search(r"<em>(.*?)</em>\s*$", caption.strip(), re.S)
    if m:
        return "canonical", strip_html(m.group(1))
    plain = strip_html(caption)
    hits = list(re.finditer(r"\bModels?:\s*", plain))
    if hits:
        return "legacy", plain[hits[-1].end():]
    return "freeform", plain


def split_top_level(text: str, seps: tuple[str, ...]) -> list[str]:
    """Split on any of ``seps`` outside parentheses."""
    out, depth, cur, i = [], 0, "", 0
    while i < len(text):
        ch = text[i]
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth = max(0, depth - 1)
        if depth == 0:
            hit = next((sp for sp in seps if text.startswith(sp, i)), None)
            if hit:
                out.append(cur)
                cur = ""
                i += len(hit)
                continue
        cur += ch
        i += 1
    out.append(cur)
    return [p for p in out if p.strip()]


def _label_of(part: str) -> tuple[str, str, str] | None:
    """(label, glyph, qualifier) of one clause, or None when it holds no glyph."""
    m = _GLYPH.search(part)
    if not m:
        return None
    label = part[: m.start()].strip()
    label = label.rstrip(" ,;:—–-(").strip()
    label = _LEAD.sub("", label).strip()
    rest = part[m.end():]
    q = re.match(r"\s*\(([^()]*)\)", rest)
    return label, m.group(0).replace("️", ""), (q.group(1).strip() if q else "")


def caption_clauses(caption: str) -> tuple[str, list[tuple[str, str, str]], list[str]]:
    """``(mode, clauses, errors)`` — clauses are (label, headline glyph, qualifier text)."""
    mode, text = caption_trailer(caption)
    errors: list[str] = []
    clauses: list[tuple[str, str, str]] = []
    if mode == "canonical":
        hit = re.search(r"\bModels?:\s*", text)
        if not hit:  # a no-badge trailer: any glyph is an error
            if _GLYPH.search(text):
                errors.append(f"no-badge trailer carries a grade glyph: {text!r}")
            return mode, clauses, errors
        head, body = text[: hit.start()], text[hit.end():]
        if _GLYPH.search(head):
            errors.append(f"glyph before 'Model:' in the trailer: {head!r}")
        for part in split_top_level(body, (" · ",)):
            lab = _label_of(part)
            if lab is None:
                continue  # a note clause without a glyph ("thin mask, no flare")
            if not lab[0]:
                errors.append(f"clause without a label before its glyph: {part.strip()!r}")
                continue
            clauses.append(lab)
        return mode, clauses, errors
    seps = (" · ", "; ", ", ", " — ") if mode == "legacy" else (". ", "; ", " — ")
    for part in split_top_level(text, seps):
        lab = _label_of(part)
        if lab is None or not lab[0]:
            continue
        clauses.append(lab)
    return mode, clauses, errors


def _norm_key(label: str) -> str:
    return _WS.sub(" ", label.replace("️", "")).strip().lower()


def resolve_label(label: str, spec) -> list[matrix_parse.Row] | None:
    """Rows a caption label names: exact row text → ``spec.extra['model_rows']`` → the frozen aliases."""
    candidates = [label, re.sub(r"\s*\([^()]*\)\s*$", "", label).strip()]
    for cand in candidates:
        row = matrix_parse.find(cand)
        if row is not None:
            return [row]
    model_rows = (getattr(spec, "extra", {}) or {}).get("model_rows") or {}
    for cand in candidates:
        for key, val in model_rows.items():
            if _norm_key(key) == _norm_key(cand):
                names = [val] if isinstance(val, str) else list(val)
                rows = [matrix_parse.find(n) for n in names]
                if any(r is None for r in rows):
                    return []  # a model_rows entry naming a row that does not exist: reported by the caller
                return rows
    for cand in candidates:
        val = matrix_parse.CAPTION_ALIASES.get(_norm_key(cand))
        if val is not None:
            if isinstance(val, dict):  # a frozen legacy alias pinning the glyph of a graded part
                row = matrix_parse.find(val["row"])
                return [] if row is None else [(row, val.get("glyph", row.headline))]
            names = [val] if isinstance(val, str) else list(val)
            rows = [matrix_parse.find(n) for n in names]
            return [] if any(r is None for r in rows) else rows
    return None


def _expected(row_or_pair):
    """(row, expected glyph) for an item of :func:`resolve_label`."""
    if isinstance(row_or_pair, tuple):
        return row_or_pair
    return row_or_pair, row_or_pair.headline


def check_captions(specs) -> list[str]:
    """Errors (never warnings) for every caption whose Model trailer does not match the matrix."""
    errors: list[str] = []
    for spec in specs:
        mode, clauses, errs = caption_clauses(spec.caption)
        errors += [f"{spec.name}: {e}" for e in errs]
        for label, glyph, _qual in clauses:
            rows = resolve_label(label, spec)
            if rows is None:
                errors.append(f"{spec.name}: caption label {label!r} resolves to no matrix row (name the row exactly or add it to this spec's extra['model_rows'])")
                continue
            if rows == []:
                errors.append(f"{spec.name}: caption label {label!r} maps to a row name that is not in the matrix")
                continue
            for item in rows:
                row, want = _expected(item)
                if want != glyph:
                    errors.append(f"{spec.name}: caption gives {label!r} the glyph {glyph} but matrix row {row.name!r} is {row.headline}")
    return errors


# ---------------------------------------------------------------------------
# The check
# ---------------------------------------------------------------------------

_SMIL = re.compile(r"<(animate|set|animateTransform|animateMotion|animateColor)\b")
_NEG_SIZE = re.compile(r'\s(?:width|height)="-')


def check_svg(path: Path, spec, errors: list[str], warnings: list[str]) -> None:
    text = path.read_text(encoding="utf-8", errors="replace")
    rel = registry.rel_to_repo(path)
    if "<script" in text:
        errors.append(f"{rel}: contains <script>")
    if _SMIL.search(text):
        errors.append(f"{rel}: contains SMIL animation (use CSS @keyframes)")
    if re.search(r"<a\s", text):
        errors.append(f"{rel}: contains an <a> element (links are inert inside <img>; navigate in Markdown)")
    if m := _NEG_SIZE.search(text):
        errors.append(f"{rel}: an element has a negative {m.group(0).strip()[:-3]} (invalid SVG, browsers draw nothing)")
    if "<title" not in text or "<desc" not in text:
        errors.append(f"{rel}: missing <title> or <desc>")
    if "@keyframes" in text and "prefers-reduced-motion" not in text:
        errors.append(f"{rel}: animated without the in-file prefers-reduced-motion rule")
    if getattr(spec, "platform_independent", False):
        fonts = set(re.findall(r'font-family="([^"]*)"', text))
        bad = {f for f in fonts if f != tokens.FONT_SVG and "Mono" not in f and "mono" not in f}
        if bad:
            errors.append(f"{rel}: font-family is not tokens.FONT_SVG: {sorted(bad)[:2]}")


def _readiness_check(strict: bool) -> tuple[list[str], list[str]]:
    script = HERE / "future" / "check_readiness.py"
    csv_ = HERE / "future" / "readiness.csv"
    if not script.exists() or not csv_.exists():
        note = "readiness check skipped: docs/figures/future/check_readiness.py or readiness.csv absent (lands with V4c)"
        print(f"  note  {note}")
        return ([note] if strict else []), []
    proc = subprocess.run([sys.executable, str(script)], capture_output=True, text=True, cwd=str(REPO))
    if proc.returncode != 0:
        tail = (proc.stdout + proc.stderr).strip().splitlines()[-3:]
        return [f"readiness check failed: {' | '.join(tail)}"], []
    return [], []


def run_check(specs, *, strict: bool, pages: list[str] | None = None, out_dir: Path | None = None,
              registries: bool = True) -> tuple[list[str], list[str]]:
    """Every rule of the module docstring; returns ``(errors, warnings)`` and prints the image total."""
    errors: list[str] = []
    warnings: list[str] = []
    soft = errors if strict else warnings
    total_bytes = 0
    for spec in specs:
        if not getattr(spec, "enabled", True) and not strict:
            pass
        files = registry.all_files(spec, out_dir)
        if spec.extra.get("max_kb") and not spec.extra.get("max_kb_reason"):
            errors.append(f"{spec.name}: extra['max_kb'] override needs extra['max_kb_reason']")
        if spec.extra.get("max_kb"):
            print(f"  note  {spec.name}: size cap overridden to {spec.extra['max_kb']} KB — {spec.extra.get('max_kb_reason')}")
        if len(spec.alt) > 300:
            soft.append(f"{spec.name}: alt is {len(spec.alt)} characters (≤ 300)")
        if re.search(r"\b(chart|plot|diagram|image|figure) of\b", spec.alt[:40], re.I):
            warnings.append(f"{spec.name}: alt starts with filler ('… of'); say what the image shows")
        for variant, path in files.items():
            if not path.exists():
                errors.append(f"{spec.name}: missing {registry.rel_to_repo(path)}")
                continue
            size = path.stat().st_size
            total_bytes += size
            cap, warn = registry.cap_for(spec, variant)
            kb = size / 1024
            if kb > cap:
                errors.append(f"{spec.name}: {path.name} is {kb:.0f} KB, over the {cap} KB cap for kind {spec.kind!r}")
            elif kb > warn:
                warnings.append(f"{spec.name}: {path.name} is {kb:.0f} KB (warn at {warn} KB of the {cap} KB cap)")
            if path.suffix == ".svg":
                check_svg(path, spec, errors, warnings)
            if spec.kind == "capture":
                pinned = spec.pinned_sha256
                want = pinned if isinstance(pinned, str) else (pinned or {}).get(path.name)
                if not want:
                    errors.append(f"{spec.name}: capture without a pinned sha256")
                elif registry.sha256_of(path) != want:
                    errors.append(f"{spec.name}: sha256 of {path.name} differs from the pinned value")
        if registry.is_animated(spec):
            for t in registry.THEMES:
                if not registry.still_file(spec, t, out_dir).exists():
                    errors.append(f"{spec.name}: animated without its -still- variant for {t}")
    # page side
    e, w = registry.check_embeds(specs, pages=pages, strict=strict)
    errors += e
    warnings += w
    # unregistered files
    known = {p.resolve() for s in specs for p in registry.all_files(s).values()}
    for p in sorted(IMAGES.rglob("*")):
        if p.is_file() and p.suffix.lower() in (".png", ".svg", ".gif", ".webp", ".jpg", ".jpeg", ".apng"):
            if p.resolve() not in known:
                soft.append(f"unregistered file: {registry.rel_to_repo(p)}")
    # captions (error in every mode)
    errors += check_captions(specs)
    if registries:
        try:
            import make_legend  # noqa: PLC0415

            le, lw = make_legend.check(strict=strict)
            errors += le
            warnings += lw
        except ImportError:
            warnings.append("make_legend.py not importable")
        for modname in ("make_matrix_map", "huvfig_reference_make_matrix_map"):
            mod = sys.modules.get(modname)
            if mod is not None and hasattr(mod, "check"):
                me, mw = mod.check(strict=strict)
                errors += me
                warnings += mw
                break
        for modname in ("huvfig_diagrams_make_pipeline_diagrams", "huvfig_diagrams_make_spectrum_strip"):
            mod = sys.modules.get(modname)
            if mod is not None and hasattr(mod, "check"):
                me, mw = mod.check()
                errors += me
                warnings += mw
        re_, rw = _readiness_check(strict)
        errors += re_
        warnings += rw
    print(f"  {len(specs)} specs, {total_bytes / 2**20:.2f} MiB of registered images, {sum(1 for _ in IMAGES.rglob('*') if _.is_file())} files under docs/assets/images")
    return errors, warnings


def report(errors: list[str], warnings: list[str], label: str = "check") -> int:
    for w in warnings:
        print(f"warning: {w}")
    for e in errors:
        print(f"error: {e}")
    print(f"{label}: {len(errors)} error(s), {len(warnings)} warning(s)")
    return 1 if errors else 0


# ---------------------------------------------------------------------------
# Platform-independent subset (what the nightly may regenerate and diff)
# ---------------------------------------------------------------------------


def platform_independent_paths(specs) -> list[Path]:
    out: list[Path] = []
    for s in specs:
        if not getattr(s, "platform_independent", False):
            continue
        dep = set(s.extra.get("platform_dependent_variants", ()))
        for key, p in registry.all_files(s).items():
            if key.split("-")[0] in dep:
                continue
            out.append(p)
    data = HERE / "diagrams" / "data" / "presets.json"
    if data.exists():
        out.append(data)
    return sorted(set(out))


def regenerate_platform_independent(specs) -> None:
    done = set()
    for s in specs:
        if not getattr(s, "platform_independent", False) or s.generator is None or s.extra.get("crop_of"):
            continue
        key = (getattr(s, "module", ""), s.generator)
        if key in done:
            continue
        done.add(key)
        if "svg_only" in getattr(s.generator, "__code__", type("c", (), {"co_varnames": ()})).co_varnames:
            s.generator(svg_only=True)
        else:
            s.generator()
    for writer in ("reference/make_legend.py --write", "history/make_history_figures.py --table --write",
                   "future/check_readiness.py --table --write", "sim/make_all.py --tables --write"):
        script = HERE / writer.split()[0]
        if script.exists():
            subprocess.run([sys.executable, str(script), *writer.split()[1:]], cwd=str(REPO), check=False)
    strip = HERE / "diagrams" / "make_spectrum_strip.py"
    if strip.exists():
        subprocess.run([sys.executable, str(strip), "--refresh"], cwd=str(REPO), check=False)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--strict", action="store_true")
    ap.add_argument("--tolerant", action="store_true")
    ap.add_argument("--pages", nargs="+", metavar="PATH", help="restrict page-side checks / the sync to these pages or directories")
    ap.add_argument("--only", nargs="+", metavar="NAME")
    ap.add_argument("--catalog", action="store_true")
    ap.add_argument("--list", action="store_true")
    ap.add_argument("--sync-embeds", action="store_true")
    ap.add_argument("--site-wide", action="store_true", help="INT only: sync every page")
    ap.add_argument("--export-registry", nargs="?", const="-", metavar="PATH")
    ap.add_argument("--platform-independent", action="store_true")
    ap.add_argument("--regenerate", action="store_true")
    ap.add_argument("--caption-labels", action="store_true", help="diagnostic: print every caption clause and its resolution")
    ap.add_argument("--quiet", action="store_true")
    args = ap.parse_args(argv)

    specs = select(load_all(verbose=not args.quiet), args.only, None)
    strict = STRICT_DEFAULT or os.environ.get("HUV_FIGCHECK_STRICT") == "1"
    if args.strict:
        strict = True
    if args.tolerant:
        strict = False

    if args.list:
        for s in specs:
            print(f"{s.name:44s} {s.kind:9s} {getattr(s, 'group', ''):10s} {', '.join(s.pages) or '-'}")
        return 0
    if args.caption_labels:
        for s in specs:
            mode, clauses, errs = caption_clauses(s.caption)
            for label, glyph, qual in clauses:
                rows = resolve_label(label, s)
                res = "UNRESOLVED" if rows is None else ("BAD-ROW" if rows == [] else " / ".join(f"{_expected(r)[0].name} {_expected(r)[1]}" for r in rows))
                print(f"{s.name:40s} {mode:9s} {glyph} {label!r} -> {res}")
            for e in errs:
                print(f"{s.name:40s} {mode:9s} ERROR {e}")
        return 0
    if args.catalog:
        print(registry.catalog(specs, args.pages))
        return 0
    if args.sync_embeds:
        registry.sync_embeds(specs, args.pages, site_wide=args.site_wide)
        return 0
    if args.export_registry is not None:
        text = registry.export_json(specs, None if args.export_registry == "-" else Path(args.export_registry))
        if args.export_registry == "-":
            print(text)
        else:
            print(f"wrote {args.export_registry} (diagnostic; gitignored, nothing reads it)")
        return 0
    if args.platform_independent:
        paths = platform_independent_paths(specs)
        if args.regenerate:
            regenerate_platform_independent(specs)
        for p in paths:
            print(registry.rel_to_repo(p))
        return 0
    if args.check:
        errors, warnings = run_check(specs, strict=strict, pages=args.pages)
        return report(errors, warnings, "check_figures" + (" (strict)" if strict else " (tolerant)"))
    ap.print_help()
    return 0


if __name__ == "__main__":
    sys.exit(main())
