"""Tests for page_kit.py. Run: python -m pytest docs/hooks -q"""

from __future__ import annotations

import importlib.util
import struct
import sys
import zlib
from pathlib import Path

import pytest

import page_kit
from page_kit import (
    apply,
    asset_path,
    css_size,
    export_scale,
    image_size,
    kit_images,
    kit_paragraphs,
    kit_paths,
    kit_previews,
    kit_rungs,
    kit_strips,
    kit_tables,
    relative_url,
    resolve_path,
    resolve_srcset,
    still_url,
    trailer_role,
)

ROOT = Path(__file__).resolve().parents[2]

TABLE = (
    "<table>\n<thead>\n<tr>\n<th>Process</th>\n<th>Volume production</th>\n</tr>\n</thead>\n"
    "<tbody>\n<tr>\n<td>Intel 90 nm</td>\n<td>2004</td>\n</tr>\n<tr>\n<td>TSMC 28 nm</td>\n<td>2011</td>\n</tr>\n</tbody>\n</table>"
)


# --- tables ------------------------------------------------------------------------------------


def test_table_hint_adds_class_and_material_wrapper() -> None:
    out = kit_tables("<!-- huv-table: compact -->\n" + TABLE)
    assert out.startswith('<div class="md-typeset__scrollwrap"><div class="md-typeset__table"><table class="huv-compact">')
    assert out.endswith("</table></div></div>")
    assert "huv-table" not in out  # the hint comment is consumed


def test_stack_cells_get_data_label_from_the_header() -> None:
    out = kit_tables("<!-- huv-table: stack -->\n\n" + TABLE)
    assert '<td data-label="Process">Intel 90 nm</td>' in out
    assert '<td data-label="Volume production">2011</td>' in out
    assert out.count("data-label=") == 4


def test_stack_label_keeps_a_space_after_a_chip_glyph() -> None:
    chip = ('<span class="huv-chip huv-chip--implemented"><span class="huv-chip__glyph" aria-hidden="true">✅</span>'
            "Implemented</span> · 44")
    table = f"<table><thead><tr><th>{chip}</th></tr></thead><tbody><tr><td>Computed by code</td></tr></tbody></table>"
    out = kit_tables("<!-- huv-table: stack -->\n" + table)
    assert '<td data-label="✅ Implemented · 44">Computed by code</td>' in out


def test_key_table_is_labelled_and_hint_inside_paragraph_is_accepted() -> None:
    out = kit_tables("<p><!-- huv-table: key --></p>\n" + TABLE)
    assert 'class="huv-key"' in out
    assert '<td data-label="Process">' in out
    assert "<p></p>" not in out


def test_unknown_kind_and_plain_tables_are_untouched() -> None:
    assert kit_tables("<!-- huv-table: fancy -->\n" + TABLE) == "<!-- huv-table: fancy -->\n" + TABLE
    assert kit_tables(TABLE) == TABLE


def test_grid_rows_get_grade_from_the_status_cell() -> None:
    table = (
        "<!-- huv-table: grid -->\n<table>\n<thead><tr><th>Capability</th><th>Status</th><th>Code</th></tr></thead>\n"
        '<tbody><tr><td><a href="pipeline/">Hopkins imaging (factorized TCC/SOCS)</a></td>'
        '<td><span class="huv-badge">✅</span></td><td><code>aerial.rs</code></td></tr>\n'
        "<tr><td>Process window / ED analysis</td><td>🔶 (✅ lines)</td><td>x</td></tr>\n"
        "<tr><td>Planned thing</td><td>🗺️</td><td>y</td></tr>\n"
        "<tr><td>No grade</td><td>—</td><td>z</td></tr></tbody></table>"
    )
    out = kit_tables(table, matrix=True)
    assert '<tr class="huv-row--implemented" data-huv-grade="implemented" id="cm-hopkins-imaging-factorized-tcc-socs">' in out
    assert '<tr class="huv-row--simplified" data-huv-grade="simplified" id="cm-process-window-ed-analysis">' in out
    assert '<tr class="huv-row--planned" data-huv-grade="planned" id="cm-planned-thing">' in out
    assert '<tr><td data-label="Capability">No grade</td>' in out
    assert '<td data-label="Code"><code>aerial.rs</code></td>' in out  # grid tables stack on phones too
    elsewhere = kit_tables(table, matrix=False)
    assert 'data-huv-grade="implemented"' in elsewhere and "id=\"cm-" not in elsewhere


def test_grid_duplicate_row_names_get_distinct_ids() -> None:
    table = (
        "<!-- huv-table: grid -->\n<table><thead><tr><th>A</th><th>B</th></tr></thead>"
        "<tbody><tr><td>Same</td><td>✅</td></tr><tr><td>Same</td><td>🧪</td></tr></tbody></table>"
    )
    out = kit_tables(table, matrix=True)
    assert 'id="cm-same"' in out and 'id="cm-same-2"' in out


# --- Physics-Strip eyebrows --------------------------------------------------------------------

STRIP = (
    '<div class="huv-physics">\n'
    '<figure><img src="a.svg"><figcaption><strong>Mechanism.</strong> Poles and orders. <em>Schematic, not to scale.</em></figcaption></figure>\n'
    '<figure><a class="glightbox" href="b.png"><img src="b.png"></a>'
    "<figcaption><strong>How one image is built.</strong> 65 nm lines. <em>Model: Hopkins imaging ✅.</em></figcaption></figure>\n"
    '<div class="md-typeset__scrollwrap"><div class="md-typeset__table"><table><tr><td>x</td></tr></table></div></div>\n'
    "<p><strong>Same thing elsewhere:</strong> Python</p>\n</div>\n"
    "<figure><figcaption><strong>Outside the strip.</strong> Untouched.</figcaption></figure>"
)


def test_strip_captions_get_eyebrows_by_position() -> None:
    out = kit_strips(STRIP)
    # a literal "Mechanism." lead is replaced by the eyebrow (never shown twice); the detail text follows
    assert '<figcaption><span class="huv-eyebrow">Mechanism</span>Poles and orders. <em>' in out
    # a sentence lead keeps its bold and case; the second figure is "In the engine"
    assert '<figcaption><span class="huv-eyebrow">In the engine</span><strong>How one image is built.</strong> 65 nm' in out
    # the nested scroll-wrapper divs do not end the strip early; figures outside it are untouched
    assert "<figcaption><strong>Outside the strip.</strong> Untouched.</figcaption>" in out
    assert out.count("huv-eyebrow") == 2
    assert kit_strips(out) == out  # idempotent


def test_single_strip_and_colon_lead() -> None:
    html = (
        '<div class="huv-physics huv-physics--single" markdown="1"><figure><figcaption>'
        "<strong>Your first image: the numbers above, drawn.</strong> 65 nm. <em>Model: X ✅.</em></figcaption></figure></div>"
    )
    out = kit_strips(html)
    assert '<span class="huv-eyebrow">In the engine</span><strong>Your first image: the numbers above, drawn.</strong> 65 nm.' in out
    colon = '<div class="huv-physics"><figure><figcaption><strong>In the engine: four pupils at one unit.</strong> Detail.</figcaption></figure></div>'
    out = kit_strips(colon)
    assert '<span class="huv-eyebrow">In the engine</span><strong>Four pupils at one unit.</strong> Detail.' in out
    # a two-figure strip whose first lead says "In the engine." keeps the author's word
    swapped = '<div class="huv-physics"><figure><figcaption><strong>In the engine.</strong> A.</figcaption></figure><figure><figcaption>B.</figcaption></figure></div>'
    out = kit_strips(swapped)
    assert '<span class="huv-eyebrow">In the engine</span>A.' in out and '<span class="huv-eyebrow">In the engine</span>B.' in out
    assert kit_strips("<p>no strips</p>") == "<p>no strips</p>"


def test_strip_eyebrow_follows_the_caption_trailer() -> None:
    # pipeline.md / masks.md after the r1 stacking fix: a --single strip holding a closed-form
    # pupil figure above the engine figure; the eyebrow must not say "In the engine" over a
    # trailer that says "not the engine" (DESIGN §3.4 D3 grammar, §4.6 trailer vocabulary)
    stacked = (
        '<div class="huv-physics huv-physics--single" markdown="1">\n'
        '<figure><img src="p.svg"><figcaption><strong>Three pupils, one rule.</strong> Detail. '
        "<em>Closed form and geometry; not the engine. No model badge.</em></figcaption></figure>\n"
        '<figure><img src="e.png"><figcaption><strong>How one image is built.</strong> Detail. '
        "<em>Model: Hopkins imaging ✅ · exact thin-mask spectra ✅.</em></figcaption></figure>\n</div>"
    )
    out = kit_strips(stacked)
    assert '<span class="huv-eyebrow">Mechanism</span><strong>Three pupils, one rule.</strong>' in out
    assert '<span class="huv-eyebrow">In the engine</span><strong>How one image is built.</strong>' in out
    # architecture.md: the only figure of a --single strip is a schematic, not engine output
    schematic = ('<div class="huv-physics huv-physics--single"><figure><figcaption><strong>One physics crate, thin adapters.</strong> '
                 "Detail. <em>Schematic of the crate layout; no model badge.</em></figcaption></figure></div>")
    assert '<span class="huv-eyebrow">Mechanism</span><strong>One physics crate' in kit_strips(schematic)
    # the trailer also wins the other way: a first-of-two figure whose trailer is a Model line is engine output
    engine_first = ('<div class="huv-physics"><figure><figcaption><strong>A.</strong> <em>Model: X ✅.</em></figcaption></figure>'
                    '<figure><figcaption><strong>B.</strong> <em>Model: Y ✅.</em></figcaption></figure></div>')
    out = kit_strips(engine_first)
    assert out.count('<span class="huv-eyebrow">In the engine</span>') == 2 and "Mechanism" not in out
    # a literal lead still beats the trailer; a caption without a recognisable trailer falls back to position
    lead_wins = ('<div class="huv-physics"><figure><figcaption><strong>Mechanism.</strong> <em>Model: X ✅.</em></figcaption></figure>'
                 "<figure><figcaption>Bare.</figcaption></figure></div>")
    out = kit_strips(lead_wins)
    assert '<span class="huv-eyebrow">Mechanism</span><em>Model: X ✅.</em>' in out
    assert '<span class="huv-eyebrow">In the engine</span>Bare.' in out
    assert trailer_role("<strong>L.</strong> <em>Data chart from history/tool-data.md; no model badge.</em>") == "Mechanism"
    assert trailer_role("<strong>L.</strong> <em>Real capture, dark theme; nothing is mocked.</em>") == "Mechanism"
    assert trailer_role("<strong>L.</strong> <em>Drawn from the matrix itself.</em>") == "Mechanism"
    assert trailer_role("<strong>L.</strong> <em>Model: process window / ED analysis 🔶.</em>") == "In the engine"
    assert trailer_role("<strong>L.</strong> <em>Something else.</em>") is None
    # only the last <em> is the trailer (figcaption > em:last-child); badge spans inside it are fine
    assert trailer_role('<strong>L.</strong> An <em>earlier</em> emphasis. <em>Model: X <span class="huv-badge">✅</span>.</em>') == "In the engine"
    assert trailer_role("<strong>L.</strong> An <em>earlier</em> emphasis. <em>Schematic, not to scale.</em>") == "Mechanism"
    assert trailer_role("<strong>L.</strong> no trailer <em>early</em> text") is None


def test_apply_runs_strips_before_tables() -> None:
    html = '<div class="huv-physics"><figure><figcaption>Lead.</figcaption></figure>\n<!-- huv-table: compact -->\n' + TABLE + "\n</div>"
    out = apply(html)
    assert '<span class="huv-eyebrow">In the engine</span>Lead.' in out and 'class="huv-compact"' in out


# --- images ------------------------------------------------------------------------------------


def _png(path: Path, width: int, height: int) -> None:
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    chunk = b"IHDR" + ihdr
    data = b"\x89PNG\r\n\x1a\n" + struct.pack(">I", 13) + chunk + struct.pack(">I", zlib.crc32(chunk) & 0xFFFFFFFF)
    path.write_bytes(data + b"\x00" * 16)


def test_image_size_png_svg_gif(tmp_path: Path) -> None:
    _png(tmp_path / "a.png", 1600, 900)
    assert image_size(tmp_path / "a.png") == (1600, 900)
    (tmp_path / "b.svg").write_text('<?xml version="1.0"?>\n<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 960 180.4"><title>t</title></svg>')
    assert image_size(tmp_path / "b.svg") == (960, 180)
    (tmp_path / "c.svg").write_text('<svg width="600px" height="360"></svg>')
    assert image_size(tmp_path / "c.svg") == (600, 360)
    (tmp_path / "d.svg").write_text('<svg width="100%" height="50%"></svg>')
    assert image_size(tmp_path / "d.svg") is None
    (tmp_path / "e.gif").write_bytes(b"GIF89a" + struct.pack("<HH", 800, 450) + b"\x00" * 8)
    assert image_size(tmp_path / "e.gif") == (800, 450)
    (tmp_path / "f.png").write_bytes(b"not an image")
    assert image_size(tmp_path / "f.png") is None
    assert image_size(tmp_path / "missing.png") is None


def test_svg_size_is_the_browser_intrinsic_size(tmp_path: Path) -> None:
    # a matplotlib chart declares pt (1 pt = 96/72 px): the browser shows it 705 px wide, not 529
    (tmp_path / "k1.svg").write_text('<?xml version="1.0"?>\n<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" '
                                     'width="529.117578pt" height="307.5pt" viewBox="0 0 529.117578 307.5" version="1.1">')
    assert image_size(tmp_path / "k1.svg") == (705, 410)
    # unitless or px declared sizes are already px (the 206 hand-built diagrams); the viewBox agrees
    (tmp_path / "d.svg").write_text('<svg width="960" height="401" viewBox="0 0 960 401">')
    assert image_size(tmp_path / "d.svg") == (960, 401)
    (tmp_path / "mm.svg").write_text('<svg width="254mm" height="127mm">')
    assert image_size(tmp_path / "mm.svg") == (960, 480)
    (tmp_path / "in.svg").write_text('<svg height="2in" width="1in">')
    assert image_size(tmp_path / "in.svg") == (96, 192)
    # a relative or percentage size has no intrinsic px value: the viewBox gives the aspect ratio
    (tmp_path / "pct.svg").write_text('<svg width="100%" height="100%" viewBox="0 0 800 300">')
    assert image_size(tmp_path / "pct.svg") == (800, 300)
    (tmp_path / "em.svg").write_text('<svg width="40em" height="20em" viewBox="0 0 640 320">')
    assert image_size(tmp_path / "em.svg") == (640, 320)
    (tmp_path / "none.svg").write_text('<svg width="auto" height="0">')
    assert image_size(tmp_path / "none.svg") is None


def test_pt_chart_embed_gets_its_px_size(tmp_path: Path) -> None:
    (tmp_path / "assets" / "images" / "nodes").mkdir(parents=True)
    (tmp_path / "assets" / "images" / "nodes" / "k1-vs-pitch-light.svg").write_text(
        '<svg xmlns="http://www.w3.org/2000/svg" width="529.117578pt" height="307.5pt" viewBox="0 0 529.117578 307.5">')
    html = '<figure><img alt="k" src="../assets/images/nodes/k1-vs-pitch-light.svg#gh-light-mode-only" /></figure>'
    assert 'width="705" height="410" />' in kit_images(html, tmp_path)


def test_asset_path_resolves_the_assets_suffix_only() -> None:
    docs = Path("/docs")
    assert asset_path("../assets/images/sim/x-light.png#gh-light-mode-only", docs) == docs / "assets/images/sim/x-light.png"
    assert asset_path("assets/images/a%20b.svg?v=2", docs) == docs / "assets/images/a b.svg"
    assert asset_path("https://example.org/x.png", docs) is None
    assert asset_path("../assets/images/../hooks/x.png", docs) is None
    assert asset_path("assets/images/x.png", None) is None


def test_lazy_loading_after_the_first_figure_image(tmp_path: Path) -> None:
    html = (
        '<p><img alt="inline" src="x.png" /></p>\n'
        '<figure><img alt="a" src="assets/images/sim/one-light.png#gh-light-mode-only" />'
        '<img alt="a" src="assets/images/sim/one-dark.png#gh-dark-mode-only" /><figcaption>c</figcaption></figure>\n'
        '<span class="twemoji"><img class="twemoji" src="icon.svg" /></span>\n'
        '<figure><img alt="b" src="assets/images/sim/two-light.png#gh-light-mode-only" /></figure>'
    )
    out = kit_images(html, tmp_path)
    assert '<img alt="inline" src="x.png" />' in out  # before the first figure: untouched
    assert 'src="assets/images/sim/one-light.png#gh-light-mode-only" />' in out  # the first figure image itself
    assert 'src="assets/images/sim/one-dark.png#gh-dark-mode-only" loading="lazy" decoding="async" />' in out
    assert 'src="assets/images/sim/two-light.png#gh-light-mode-only" loading="lazy" decoding="async" />' in out
    assert '<img class="twemoji" src="icon.svg" />' in out


def test_width_and_height_come_from_the_asset_file(tmp_path: Path) -> None:
    (tmp_path / "assets" / "images" / "sim").mkdir(parents=True)
    _png(tmp_path / "assets" / "images" / "sim" / "fig-light.png", 1600, 1000)
    html = (
        '<figure><img alt="f" src="../assets/images/sim/fig-light.png#gh-light-mode-only" />'
        '<img alt="f" src="../assets/images/sim/fig-dark.png#gh-dark-mode-only" /></figure>'
    )
    out = kit_images(html, tmp_path)
    # a sim PNG is a 2x render (style.py EXPORT_SCALE, no style.py here -> the DESIGN §5.0 default): CSS px = IHDR / 2
    assert 'src="../assets/images/sim/fig-light.png#gh-light-mode-only" width="800" height="500" />' in out
    assert 'fig-dark.png#gh-dark-mode-only" loading="lazy" decoding="async" />' in out  # unreadable: no size
    assert kit_images(html, None).count("width=") == 0


def _style_py(docs: Path, scale: int) -> None:
    (docs / "figures" / "sim").mkdir(parents=True, exist_ok=True)
    (docs / "figures" / "sim" / "style.py").write_text(
        f"#: Logical resolution (px per inch) at which sizes are specified; PNGs are\n"
        f"#: written at 2x this for sharp rendering on high-density screens.\n"
        f"LOGICAL_DPI = 100\nEXPORT_SCALE = {scale}\n\nFULL_WIDTH = 8.0\n"
    )


def test_sim_png_crops_are_laid_out_at_their_designed_1x_size(tmp_path: Path) -> None:
    # style.py renders every static sim PNG at EXPORT_SCALE = 2x and cuts crops from that render
    # (style.write_crop); the browser sizes an <img> by its pixels, so without the division the 338 px
    # High-NA pupil crop and the 389 px CLI panel (both narrower than the 688 px column) render at
    # twice their designed size (round-3 visual review)
    _style_py(tmp_path, 2)
    sim = tmp_path / "assets" / "images" / "sim"
    (sim / "anim").mkdir(parents=True)
    _png(sim / "optics-four-pupils-high-na-light.png", 338, 823)  # crop of the 1867 x 932 sheet
    _png(sim / "cli-output-gallery-simulate-light.png", 389, 563)  # crop of the 2086 x 619 sheet
    _png(sim / "home-generations-light.png", 1905, 606)  # a full sheet
    _png(sim / "anim" / "te-tm-still-light.png", 1600, 900)  # stills render at 2x (DESIGN §5.0)
    _png(sim / "anim" / "through-focus-rest-light.png", 1608, 846)  # the resting frame, a static 2x figure
    _png(sim / "anim" / "te-tm-anim-light.png", 800, 450)  # animation frames render at 1x and 800 px wide
    (sim / "anim" / "te-tm-anim-dark.gif").write_bytes(b"GIF89a" + struct.pack("<HH", 800, 450) + b"\x00" * 8)
    (tmp_path / "assets" / "images" / "gui").mkdir()
    _png(tmp_path / "assets" / "images" / "gui" / "gui-source-badge-light.png", 872, 106)  # another generator
    assert css_size(sim / "optics-four-pupils-high-na-light.png", tmp_path) == (169, 412)  # 823 / 2 rounded half up
    assert css_size(sim / "cli-output-gallery-simulate-light.png", tmp_path) == (195, 282)
    assert css_size(sim / "home-generations-light.png", tmp_path) == (953, 303)
    assert css_size(sim / "anim" / "te-tm-still-light.png", tmp_path) == (800, 450)  # = the frames it swaps with
    assert css_size(sim / "anim" / "through-focus-rest-light.png", tmp_path) == (804, 423)
    assert css_size(sim / "anim" / "te-tm-anim-light.png", tmp_path) == (800, 450)
    assert css_size(sim / "anim" / "te-tm-anim-dark.gif", tmp_path) == (800, 450)
    assert css_size(tmp_path / "assets" / "images" / "gui" / "gui-source-badge-light.png", tmp_path) == (872, 106)
    assert css_size(sim / "missing.png", tmp_path) is None
    html = (
        '<figure><img alt="p" src="../assets/images/sim/optics-four-pupils-high-na-light.png#gh-light-mode-only" /></figure>'
        '<figure><img alt="c" src="assets/images/sim/cli-output-gallery-simulate-light.png#gh-light-mode-only" /></figure>'
        '<figure><img alt="a" src="../assets/images/sim/anim/te-tm-anim-light.png#gh-light-mode-only" /></figure>'
        '<figure><img alt="g" src="../assets/images/gui/gui-source-badge-light.png#gh-light-mode-only" /></figure>'
    )
    out = kit_images(html, tmp_path)
    assert 'optics-four-pupils-high-na-light.png#gh-light-mode-only" width="169" height="412" />' in out
    assert 'cli-output-gallery-simulate-light.png#gh-light-mode-only" loading="lazy" decoding="async" width="195" height="282" />' in out
    assert 'te-tm-anim-light.png#gh-light-mode-only" loading="lazy" decoding="async" width="800" height="450"' in out
    assert 'gui-source-badge-light.png#gh-light-mode-only" loading="lazy" decoding="async" width="872" height="106" />' in out


def test_export_scale_is_read_from_style_py(tmp_path: Path) -> None:
    assert export_scale(None) == 2
    assert export_scale(tmp_path) == 2  # no style.py: the DESIGN §5.0 default
    _style_py(tmp_path, 3)
    assert export_scale(tmp_path) == 3
    sim = tmp_path / "assets" / "images" / "sim"
    sim.mkdir(parents=True)
    _png(sim / "x-light.png", 900, 301)
    assert css_size(sim / "x-light.png", tmp_path) == (300, 100)
    (tmp_path / "figures" / "sim" / "style.py").write_text("LOGICAL_DPI = 100\n")  # silent: the default again
    import os
    os.utime(tmp_path / "figures" / "sim" / "style.py", ns=(1, 1))  # a fresh mtime, not the cached one
    assert export_scale(tmp_path) == 2


def test_existing_sizes_and_loading_are_kept(tmp_path: Path) -> None:
    (tmp_path / "assets" / "images").mkdir(parents=True)
    _png(tmp_path / "assets" / "images" / "p.png", 10, 20)
    html = '<figure><img src="assets/images/p.png" width="5" /></figure><figure><img src="assets/images/p.png" loading="eager" /></figure>'
    out = kit_images(html, tmp_path)
    assert '<img src="assets/images/p.png" width="5" />' in out
    assert '<img src="assets/images/p.png" loading="eager" width="10" height="20" />' in out


def test_unpaired_image_gets_its_own_gallery() -> None:
    # gui.md / getting-started.md: a single-variant capture has no #gh-* fragment, so auto_themed
    # names no gallery and GLightbox would page it through every anchor on the page (DESIGN §4.6)
    neutral = ('<figure><a class="glightbox" data-type="image" data-width="auto" data-height="auto" href="assets/images/gui/gui-aerial-euv.png" data-desc-position="bottom">'
               '<img alt="g" src="assets/images/gui/gui-aerial-euv.png" /></a><figcaption>c</figcaption></figure>')
    out = kit_images(neutral)
    assert 'data-desc-position="bottom" data-gallery="any"><img' in out
    assert kit_images(out) == out  # idempotent
    paired = ('<figure><a class="glightbox" href="x-light.png#gh-light-mode-only" data-gallery="light"><img src="x-light.png#gh-light-mode-only" /></a>'
              '<a class="glightbox" href="x-dark.png#gh-dark-mode-only" data-gallery="dark"><img src="x-dark.png#gh-dark-mode-only" loading="lazy" /></a></figure>')
    assert kit_images(paired) == paired  # auto_themed galleries are kept
    authored = '<a class="glightbox" href="a.png" data-gallery="tour"><img src="a.png" /></a>'
    assert kit_images(authored) == authored
    plain = '<a class="md-button" href="assets/images/gui/gui-aerial-euv.png">x</a>'
    assert kit_images(plain) == plain  # only lightbox anchors


def test_anim_image_gets_class_and_still_without_fragment() -> None:
    html = (
        '<figure><picture><source media="(prefers-reduced-motion: reduce)" srcset="../assets/images/sim/anim/x-still-light.png">'
        '<img class="huv-no-zoom" src="../assets/images/sim/anim/x-anim-light.png#gh-light-mode-only" alt="a"></picture></figure>'
    )
    out = kit_images(html)
    assert 'class="huv-no-zoom huv-anim" src="../assets/images/sim/anim/x-anim-light.png#gh-light-mode-only" alt="a" data-huv-still="../assets/images/sim/anim/x-still-light.png">' in out
    kept = kit_images('<img src="a-anim-dark.svg#gh-dark-mode-only" data-huv-still="custom.svg">')
    assert 'data-huv-still="custom.svg"' in kept and kept.count("data-huv-still") == 1 and 'class="huv-anim"' in kept
    assert still_url("x-still-light.png") is None
    assert still_url("d/animals-anim-light.svg?v=3#gh-light-mode-only") == "d/animals-still-light.svg"


# --- paragraphs, rungs, previews ----------------------------------------------------------------


def test_where_next_rosetta_and_strip_paragraphs_get_their_classes() -> None:
    html = (
        "<p><strong>Where next:</strong> ← <a href=\"a/\">a</a> · <a href=\"b/\">b</a> →</p>\n"
        "<p><strong>Read next:</strong> x</p>\n<p><strong>Part of Processes · see also:</strong> y</p>\n"
        "<p><strong>Same thing elsewhere:</strong> Python <code>x</code></p>\n<p><strong>Five limits:</strong> <a href=\"#k\">k₁ ≥ 0.25</a></p>\n"
        "<p><strong>Where next</strong> without the colon</p>"
    )
    out = kit_paragraphs(html)
    assert out.count('<p class="huv-next">') == 3
    assert '<p class="huv-rosetta-line"><strong>Same thing elsewhere:</strong>' in out
    assert '<p class="huv-strip"><strong>Five limits:</strong>' in out
    assert "<p><strong>Where next</strong> without" in out


def test_rung_words_become_chips_only_inside_table_cells() -> None:
    html = "<p><strong>In production</strong></p><table><tr><td><strong>In production</strong> (selected layers)</td><td><strong>Roadmap</strong></td><td><strong>Bold</strong></td></tr></table>"
    out = kit_rungs(html)
    assert out.startswith("<p><strong>In production</strong></p>")
    assert '<td><strong class="huv-rung">In production</strong> (selected layers)</td>' in out
    assert '<td><strong class="huv-rung">Roadmap</strong></td>' in out
    assert "<td><strong>Bold</strong></td>" in out


def test_matrix_links_get_data_preview_except_on_the_matrix_page() -> None:
    html = '<a href="../capability-matrix/#core-imaging">matrix</a> <a href="../pipeline/">p</a> <a href="https://github.com/x/capability-matrix.md">gh</a>'
    out = kit_previews(html)
    assert '<a href="../capability-matrix/#core-imaging" data-preview="true">matrix</a>' in out
    assert '<a href="../pipeline/">p</a>' in out
    assert '<a href="https://github.com/x/capability-matrix.md">gh</a>' in out
    assert kit_previews(html, matrix=True) == html
    assert kit_previews(out).count("data-preview") == 1


def test_code_blocks_are_never_rewritten() -> None:
    html = (
        '<pre><code>&lt;!-- huv-table: stack --&gt;\n| a | b |\n&lt;img src="assets/images/x-anim-light.svg"&gt;\n'
        "**Where next:** &lt;td&gt;&lt;strong&gt;Shipping&lt;/strong&gt;</code></pre>"
    )
    assert apply(html) == html


def test_on_page_content_uses_the_page_and_config(tmp_path: Path) -> None:
    class File:
        src_uri = "capability-matrix.md"

    class Page:
        file = File()
        url = "capability-matrix/"

    html = '<!-- huv-table: grid -->\n<table><thead><tr><th>Capability</th><th>Status</th></tr></thead><tbody><tr><td>X</td><td>✅</td></tr></tbody></table>\n<a href="../capability-matrix/#x">m</a>'
    out = page_kit.on_page_content(html, Page(), {"docs_dir": str(tmp_path)}, None)
    assert 'id="cm-x"' in out and "data-preview" not in out
    File.src_uri = "pipeline.md"
    Page.url = "pipeline/"
    out = page_kit.on_page_content(html, Page(), {"docs_dir": str(tmp_path)}, None)
    assert 'id="cm-' not in out and 'data-preview="true"' in out


# --- the runner finds examples inside the kit's wrappers (DESIGN §9.0 rule 6) -------------------

FIXTURE = '''# Fixture

<div class="huv-rosetta" markdown>

**Python**

<!-- verify-example -->
```python
import highuvlith as huv
result = huv.simulate_line_space(65.0, 180.0, na=0.75)
print(f"Contrast: {result.contrast:.3f}, NILS: {result.nils:.2f}")
```

???+ success "Output"

    ```text
    Contrast: 0.454, NILS: 0.65
    ```

**CLI**

```bash
highuvlith simulate --config examples/sim.toml
```

</div>

<div class="huv-physics" markdown>

<figure markdown="span">

![alt](assets/images/sim/x-light.png#gh-light-mode-only)

<figcaption>c</figcaption>
</figure>

<!-- verify-example -->
```python
print("physics")
```

???+ success "Output"

    ```text
    physics
    ```

</div>

<details markdown="1">
<summary>Validation — 2 tests</summary>

| Claim | Test | Tolerance |
|---|---|---|
| contrast | `test_contrast` | 1e-9 |

<!-- verify-example -->
```python
print("details")
```

???+ success "Output"

    ```text
    details
    ```

</details>

<!-- example-skip: fragment -->
```python
fragment
```
'''


def _runner():
    path = ROOT / "tests" / "docs" / "test_site_examples.py"
    if not path.exists():
        pytest.skip("tests/docs/test_site_examples.py is absent")
    spec = importlib.util.spec_from_file_location("huv_site_examples_runner", path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules.setdefault("huv_site_examples_runner", module)
    spec.loader.exec_module(module)
    return module


def test_runner_extracts_examples_inside_wrappers(tmp_path: Path) -> None:
    runner = _runner()
    page = tmp_path / "docs" / "fixture.md"
    page.parent.mkdir()
    page.write_text(FIXTURE, encoding="utf-8")
    blocks = runner.extract(page)
    kinds = [b.kind for b in blocks]
    assert kinds == ["verify", "verify", "verify", "skip"]
    lines = page.read_text(encoding="utf-8").splitlines()
    outputs = [runner.find_output(lines, b) for b in blocks[:3]]
    assert [o.text for o in outputs] == ["Contrast: 0.454, NILS: 0.65", "physics", "details"]
    assert all(b.indent == "" for b in blocks)  # the fences sit at column 0 inside every wrapper
    assert all(o.indent == "    " for o in outputs)  # the admonition's text fence is indented by four
    assert blocks[0].code.startswith("import highuvlith as huv")


# --- raw image paths resolve like Markdown images (orchestrator ruling; DESIGN §4.6 forms 2–3) ----


def _tree(tmp_path: Path) -> Path:
    for rel in ("assets/images/diagrams/pipeline-stages-light.svg", "assets/images/diagrams/pipeline-stages-narrow-light.svg",
                "assets/images/sim/anim/socs-focus-anim-light.png", "assets/images/sim/anim/socs-focus-still-light.png",
                "assets/images/history/euv-optical-path-light.svg", "history/local.png"):
        (tmp_path / rel).parent.mkdir(parents=True, exist_ok=True)
        (tmp_path / rel).write_bytes(b"x")
    return tmp_path


def test_relative_url_matches_mkdocs_directory_urls() -> None:
    assert relative_url("assets/images/x.png", "") == "assets/images/x.png"
    assert relative_url("assets/images/x.png", "pipeline/") == "../assets/images/x.png"
    assert relative_url("assets/images/x.png", "history/euv/") == "../../assets/images/x.png"
    assert relative_url("assets/images/x.png", "history/") == "../assets/images/x.png"
    assert relative_url("assets/images/x.png", "history/euv.html") == "../assets/images/x.png"  # use_directory_urls: false
    assert relative_url("history/local.png", "history/euv/") == "../local.png"


def test_top_level_page_raw_src_is_rewritten_to_the_directory_url(tmp_path: Path) -> None:
    docs = _tree(tmp_path)
    # DESIGN §4.6 form 3 on pipeline.md (URL pipeline/): the source-relative path would 404 in the browser
    src = "assets/images/sim/anim/socs-focus-anim-light.png#gh-light-mode-only"
    assert resolve_path(src, "pipeline.md", "pipeline/", docs) == "../assets/images/sim/anim/socs-focus-anim-light.png#gh-light-mode-only"
    assert resolve_path("assets/images/diagrams/pipeline-stages-light.svg?v=2#gh-light-mode-only", "pipeline.md", "pipeline/", docs) == "../assets/images/diagrams/pipeline-stages-light.svg?v=2#gh-light-mode-only"


def test_nested_page_and_index_page(tmp_path: Path) -> None:
    docs = _tree(tmp_path)
    # history/euv.md (URL history/euv/, one level deeper than its source file)
    assert resolve_path("../assets/images/history/euv-optical-path-light.svg#gh-light-mode-only", "history/euv.md", "history/euv/", docs) == "../../assets/images/history/euv-optical-path-light.svg#gh-light-mode-only"
    assert resolve_path("local.png", "history/euv.md", "history/euv/", docs) == "../local.png"
    # history/index.md (URL history/): source directory == URL directory, so the path is unchanged
    assert resolve_path("../assets/images/history/euv-optical-path-light.svg", "history/index.md", "history/", docs) == "../assets/images/history/euv-optical-path-light.svg"
    # the home page (URL "")
    assert resolve_path("assets/images/diagrams/pipeline-stages-light.svg", "index.md", "", docs) == "assets/images/diagrams/pipeline-stages-light.svg"


def test_already_rewritten_markdown_image_paths_are_idempotent(tmp_path: Path) -> None:
    docs = _tree(tmp_path)
    done = "../../assets/images/history/euv-optical-path-light.svg#gh-light-mode-only"
    assert resolve_path(done, "history/euv.md", "history/euv/", docs) == done
    done = "../assets/images/diagrams/pipeline-stages-light.svg#gh-dark-mode-only"
    assert resolve_path(done, "pipeline.md", "pipeline/", docs) == done


def test_docs_root_fallback_and_untouched_cases(tmp_path: Path) -> None:
    docs = _tree(tmp_path)
    # a docs-root-relative path on a nested page falls back to the root interpretation
    assert resolve_path("assets/images/diagrams/pipeline-stages-light.svg", "history/euv.md", "history/euv/", docs) == "../../assets/images/diagrams/pipeline-stages-light.svg"
    # absolute URLs, root-absolute paths, data URIs and fragments are left alone
    for untouched in ("https://example.org/x.png", "//cdn/x.png", "/highuvlith/assets/x.png", "data:image/png;base64,AAAA", "#gh-light-mode-only", ""):
        assert resolve_path(untouched, "pipeline.md", "pipeline/", docs) is None
    # a path that resolves nowhere is left alone (MkDocs' own validation reports it)
    assert resolve_path("assets/images/missing.png", "pipeline.md", "pipeline/", docs) is None
    assert resolve_path("../../escapes.png", "pipeline.md", "pipeline/", docs) is None
    assert resolve_path("assets/images/diagrams/pipeline-stages-light.svg", "pipeline.md", "pipeline/", None) is None


def test_srcset_with_two_candidates_and_data_huv_still(tmp_path: Path) -> None:
    docs = _tree(tmp_path)
    srcset = "assets/images/diagrams/pipeline-stages-narrow-light.svg 600w, assets/images/diagrams/pipeline-stages-light.svg 960w"
    assert resolve_srcset(srcset, "pipeline.md", "pipeline/", docs) == "../assets/images/diagrams/pipeline-stages-narrow-light.svg 600w, ../assets/images/diagrams/pipeline-stages-light.svg 960w"
    assert resolve_srcset("https://example.org/a.svg 600w", "pipeline.md", "pipeline/", docs) is None
    html = (
        '<figure><picture><source media="(prefers-reduced-motion: reduce)" srcset="assets/images/sim/anim/socs-focus-still-light.png">'
        '<img class="huv-no-zoom huv-anim" src="assets/images/sim/anim/socs-focus-anim-light.png#gh-light-mode-only" alt="a" '
        'data-huv-still="assets/images/sim/anim/socs-focus-still-light.png"></picture></figure>'
    )
    out = kit_images(html, docs, "pipeline.md", "pipeline/")
    assert '<source media="(prefers-reduced-motion: reduce)" srcset="../assets/images/sim/anim/socs-focus-still-light.png">' in out
    assert 'src="../assets/images/sim/anim/socs-focus-anim-light.png#gh-light-mode-only"' in out
    assert 'data-huv-still="../assets/images/sim/anim/socs-focus-still-light.png"' in out
    assert out.count("data-huv-still") == 1 and out.count("huv-anim") == 1


def test_narrow_strip_picture_on_a_nested_page(tmp_path: Path) -> None:
    docs = _tree(tmp_path)
    html = (
        '<figure><picture><source media="(max-width: 600px)" srcset="../assets/images/diagrams/pipeline-stages-narrow-light.svg">'
        '<img class="huv-no-zoom" src="../assets/images/diagrams/pipeline-stages-light.svg#gh-light-mode-only" alt="s"></picture></figure>'
    )
    out = kit_images(html, docs, "history/euv.md", "history/euv/")
    assert 'srcset="../../assets/images/diagrams/pipeline-stages-narrow-light.svg"' in out
    assert 'src="../../assets/images/diagrams/pipeline-stages-light.svg#gh-light-mode-only"' in out
    assert kit_images(out, docs, "history/euv.md", "history/euv/") == out  # idempotent
    assert kit_images(html, docs) == html  # no page given: paths untouched (the fixture files have no readable size)


def test_glightbox_anchor_href_follows_its_image(tmp_path: Path) -> None:
    docs = _tree(tmp_path)
    html = ('<figure><a class="glightbox" href="assets/images/sim/anim/socs-focus-anim-light.png" data-type="image">'
            '<img src="assets/images/sim/anim/socs-focus-anim-light.png#gh-light-mode-only" alt="a"></a></figure>')
    out = kit_images(html, docs, "pipeline.md", "pipeline/")
    assert 'href="../assets/images/sim/anim/socs-focus-anim-light.png"' in out
    assert 'src="../assets/images/sim/anim/socs-focus-anim-light.png#gh-light-mode-only"' in out
    plain = '<a href="assets/images/sim/anim/socs-focus-anim-light.png">x</a>'
    assert kit_images(plain, docs, "pipeline.md", "pipeline/") == plain  # ordinary links are MkDocs' business


def test_kit_paths_rewrites_only_resolvable_attributes(tmp_path: Path) -> None:
    docs = _tree(tmp_path)
    tag = '<img src="assets/images/diagrams/pipeline-stages-light.svg" data-huv-still="assets/images/nope.png">'
    out = kit_paths(tag, {"src": "assets/images/diagrams/pipeline-stages-light.svg", "data-huv-still": "assets/images/nope.png"}, "pipeline.md", "pipeline/", docs)
    assert out == '<img src="../assets/images/diagrams/pipeline-stages-light.svg" data-huv-still="assets/images/nope.png">'


def test_on_page_content_resolves_through_the_page_url(tmp_path: Path) -> None:
    docs = _tree(tmp_path)

    class File:
        src_uri = "history/euv.md"

    class Page:
        file = File()
        url = "history/euv/"

    html = '<figure><img class="huv-no-zoom" src="../assets/images/history/euv-optical-path-light.svg#gh-light-mode-only" alt="e"></figure>'
    out = page_kit.on_page_content(html, Page(), {"docs_dir": str(docs)}, None)
    assert 'src="../../assets/images/history/euv-optical-path-light.svg#gh-light-mode-only"' in out
