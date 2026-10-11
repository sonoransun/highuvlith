"""Caption ``Model:`` trailers name matrix rows with the rows' headline glyphs (DESIGN §5.0).

A thin wrapper over ``docs/figures/check_figures.check_captions()`` — the same function
``make_all.py --check`` runs in every mode — so the test and the gate cannot disagree. Labels
resolve in order: the exact Capability text of a matrix row → the spec's own
``extra["model_rows"]`` → the frozen ``matrix_parse.CAPTION_ALIASES`` (the 47 baseline
captions only). Skips when the figure modules cannot be imported (no matplotlib).
"""

from __future__ import annotations

import sys
from pathlib import Path
from types import SimpleNamespace

import pytest

ROOT = Path(__file__).resolve().parents[2]
for sub in ("docs/figures", "docs/figures/reference", "docs/figures/sim"):
    p = str(ROOT / sub)
    if p not in sys.path:
        sys.path.insert(0, p)

pytest.importorskip("matplotlib")
check_figures = pytest.importorskip("check_figures")
matrix_parse = pytest.importorskip("matrix_parse")


def _spec(caption: str, model_rows: dict | None = None):
    return SimpleNamespace(name="fixture", caption=caption, extra={"model_rows": model_rows or {}})


def test_every_registered_caption_resolves():
    specs = check_figures.load_all(verbose=False)
    assert specs, "no specs registered"
    errors = check_figures.check_captions(specs)
    assert errors == [], "\n".join(errors)


def test_exact_row_with_headline_glyph_passes():
    row = matrix_parse.rows()[0]
    spec = _spec(f"<strong>x.</strong> y. <em>Model: {row.name} {row.headline}.</em>")
    assert check_figures.check_captions([spec]) == []


def test_wrong_glyph_is_an_error():
    row = next(r for r in matrix_parse.rows() if r.headline == "✅")
    spec = _spec(f"<em>Model: {row.name} 🔶.</em>")
    errs = check_figures.check_captions([spec])
    assert len(errs) == 1 and "🔶" in errs[0] and row.name in errs[0]


def test_unresolvable_label_is_an_error():
    spec = _spec("<em>Model: warp drive ✅.</em>")
    errs = check_figures.check_captions([spec])
    assert len(errs) == 1 and "warp drive" in errs[0]


def test_glyph_in_no_badge_trailer_is_an_error():
    spec = _spec("<strong>A schematic.</strong> <em>Schematic, not to scale ✅.</em>")
    errs = check_figures.check_captions([spec])
    assert len(errs) == 1 and "no-badge" in errs[0]


def test_no_badge_trailers_pass_without_glyphs():
    for trailer in ("Schematic, not to scale.", "Closed form; not the engine.", "Data chart from litho_tools.csv; no model badge.",
                    "Drawn from the matrix itself.", "Chips copied from the matrix."):
        assert check_figures.check_captions([_spec(f"<strong>t.</strong> <em>{trailer}</em>")]) == []


def test_model_rows_resolve_abbreviations():
    spec = _spec("<em>Model: Hopkins imaging ✅ · EUV projection optics 🔶 (obscuration assumed).</em>",
                 {"Hopkins imaging": "Hopkins imaging (factorized TCC/SOCS)",
                  "EUV projection optics": "EUV scanner projection optics (NA 0.33 / High-NA 0.55)"})
    assert check_figures.check_captions([spec]) == []


def test_model_rows_list_checks_every_row():
    spec = _spec("<em>Model: refractive and immersion optics ✅.</em>",
                 {"refractive and immersion optics": ["Refractive projection optics (dry)", "Immersion lithography (193i)"]})
    assert check_figures.check_captions([spec]) == []
    bad = _spec("<em>Model: refractive and immersion optics ✅.</em>",
                {"refractive and immersion optics": ["Refractive projection optics (dry)", "Flare"]})
    assert len(check_figures.check_captions([bad])) == 1


def test_qualifier_glyphs_and_note_clauses_are_allowed():
    spec = _spec("<em>Model: Synchrotron (bending magnet / undulator) ✅ (🔶 coherence bound) · thin mask, no flare.</em>")
    assert check_figures.check_captions([spec]) == []


def test_aliases_are_frozen_to_the_baseline_labels():
    # the frozen dict resolves only through check_figures; every value names a real row
    for key, val in matrix_parse.CAPTION_ALIASES.items():
        names = [val["row"]] if isinstance(val, dict) else ([val] if isinstance(val, str) else list(val))
        for n in names:
            assert matrix_parse.find(n) is not None, f"alias {key!r} -> {n!r} is not a matrix row"
