#!/usr/bin/env python3
"""D5 — the capability-matrix tile map (DESIGN §5.1).

One rounded tile per matrix row, one column per domain H2, fill = the headline grade
colour, split grades as diagonal two-tone tiles with both glyph marks (headline first),
a ``SHORT`` label map for long names, the four-chip legend and per-grade counts in the
footer. Drawn from the matrix itself through ``matrix_parse.py``, so a grade change
reflows the map. The tiles carry no links (an SVG inside ``<img>`` is inert on both
surfaces); the jump list under the figure carries navigation.

Outputs: ``docs/assets/images/reference/matrix-map-{light,dark}.svg`` (svgkit, platform
independent — what the nightly regenerates with ``--svg-only``) and, for the README,
``matrix-map-{light,dark}.png`` (matplotlib at 2×, platform dependent, rendered on the
figure machine only).

    python docs/figures/reference/make_matrix_map.py            # SVG pair + PNG pair
    python docs/figures/reference/make_matrix_map.py --svg-only
    python docs/figures/reference/make_matrix_map.py --check    # tile count == rows, counts == legend, SHORT covers every row
"""

from __future__ import annotations

import html
import math
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))

import matrix_parse  # noqa: E402
import registry  # noqa: E402
import svgkit as K  # noqa: E402
import tokens  # noqa: E402

REPO = matrix_parse.REPO
OUT = registry.IMAGES / "reference"
NAME = "matrix-map"
GENERATOR = "docs/figures/reference/make_matrix_map.py"

#: Row name -> up to two short lines (≤ 11 characters each). The row text is authoritative;
#: these are labels for the tiles only.
SHORT: dict[str, tuple[str, ...]] = {
    "Hopkins imaging (factorized TCC/SOCS)": ("Hopkins", "TCC/SOCS"),
    "Defocus / through-focus imaging": ("Through-", "focus"),
    "Broadband imaging (`compute_multiwavelength`)": ("Broadband", "per λ"),
    "Narrow-band polychromatic (`compute_polychromatic`)": ("Polychrom.", "narrow"),
    "Vector (polarized) high-NA imaging": ("Vector", "TE / TM"),
    "Image normalization": ("Clear-field", "normalis."),
    "Illumination pupil fills": ("Pupil fills",),
    "Flare": ("Flare",),
    "Parallel, deterministic compute": ("Parallel", "determin."),
    "Refractive projection optics (dry)": ("Refractive", "optics"),
    "Immersion lithography (193i)": ("Immersion", "193i"),
    "EUV scanner projection optics (NA 0.33 / High-NA 0.55)": ("EUV optics", "0.33, 0.55"),
    "Schwarzschild reflective objective": ("Schwarz-", "schild"),
    "EUV/BEUV multilayer pupil (opt-in)": ("Multilayer", "pupil"),
    "Fresnel zone plate": ("Zone plate",),
    "Zernike aberrations": ("Zernike", "aberrations"),
    "Optics honesty guard": ("Optics", "guard"),
    "Exact thin-mask spectrum": ("Thin-mask", "spectrum"),
    "Commensurate periodic grids": ("Commens.", "grids"),
    "Antialiased rasterization": ("Antialiased", "raster"),
    "Mask model and types": ("Mask", "types"),
    "CD / ILS / NILS / MEEF metrics": ("CD · NILS", "MEEF"),
    "Process window / ED analysis": ("Process", "window"),
    "VUV excimer (F₂ 157.63 nm; Ar₂ 126 nm)": ("VUV", "excimer"),
    "DUV/UV heritage (ArF 193.368, KrF 248.3, Hg g/h/i)": ("DUV / UV", "heritage"),
    "LPA-FEL": ("LPA-FEL",),
    "Laser-produced plasma (Sn 13.5; Gd 6.7; Tb 6.5 nm)": ("LPP Sn", "Gd · Tb"),
    "Discharge-produced plasma (DPP/LDP, Xe/Sn 13.5 nm)": ("DPP / LDP",),
    "Synchrotron (bending magnet / undulator)": ("Synchro-", "tron"),
    "High-harmonic generation": ("HHG",),
    "XFEL (SASE / seeded)": ("XFEL",),
    "Plasma soft-X-ray laser (Ne-like Ar 46.9, Ni-like Ag 13.9 / Cd 13.2 / Mo 18.9 nm)": ("Soft X-ray", "laser"),
    "Hard X-ray tube (W / Mo / Cu / Rh, 5–300 kV)": ("X-ray tube",),
    "Inverse Compton scattering": ("Inverse", "Compton"),
    "Laser-wakefield betatron X-rays": ("Betatron",),
    "SSMB storage ring": ("SSMB",),
    "Smith–Purcell free-electron grating": ("Smith–", "Purcell"),
    "Entangled-photon (NOON) source": ("Entangled", "NOON"),
    "Machine-parameter physics layer": ("Machine", "physics"),
    "Dose-limited wafer throughput": ("Through-", "put"),
    "2D resist exposure (Dill ABC)": ("2-D Dill", "exposure"),
    "2D development (Mack / threshold)": ("2-D Mack", "develop"),
    "Post-exposure bake (anisotropic Gaussian)": ("Gaussian", "PEB"),
    "Chemically amplified bake (acid/quencher reaction–diffusion)": ("CAR bake",),
    "Volumetric (z-resolved) exposure": ("Volumetric", "exposure"),
    "3D development (fast marching)": ("3-D FMM", "develop"),
    "3D development (level set)": ("3-D level", "set"),
    "Grayscale lithography": ("Grayscale",),
    "LIGA / deep X-ray depth dose and exposure time": ("LIGA depth", "dose"),
    "LIGA proximity diffraction": ("LIGA", "proximity"),
    "Multi-beam interference / two-photon": ("Interfer.", "2-photon"),
    "Talbot / DTL / ATL proximity lithography": ("Talbot", "DTL / ATL"),
    "Two-grating EUV interference lithography": ("EUV-IL",),
    "ILT mask optimization": ("ILT",),
    "OPC (rule, uniform model, fragment model)": ("OPC",),
    "SRAF insertion": ("SRAF",),
    "Multiple patterning (LELE, SADP, SAQP)": ("Multiple", "patterning"),
    "DSA block copolymer": ("DSA",),
    "Photon shot noise": ("Shot noise",),
    "Shot-to-shot dose jitter": ("Dose jitter",),
    "LER/LWR Monte Carlo": ("LER/LWR",),
    "Ptychography (ePIE)": ("Ptycho-", "graphy"),
    "Quantum lithography (N-photon / N00N)": ("Quantum", "N00N"),
    "MNSL (moiré nanosphere emission)": ("MNSL",),
    "X-ray/EUV/BEUV optical constants (CXRO/Henke)": ("Henke", "constants"),
    "X-ray attenuation (NIST)": ("NIST", "attenuation"),
    "Materials database": ("Materials", "database"),
    "EUV/BEUV multilayer mirrors": ("Multilayer", "mirrors"),
    "Thin-film transfer matrix": ("Thin-film", "matrix"),
    "Python bindings": ("Python",),
    "CLI (TOML, strict keys)": ("CLI",),
    "Desktop GUI": ("GUI",),
    "Jupyter notebooks": ("Notebooks",),
    "Documentation site": ("Docs site",),
    "GPU compute backend": ("GPU", "backend"),
}

#: Domain H2 -> short column header (≤ 2 lines).
DOMAIN_SHORT = {
    "Core imaging": ("Core", "imaging"),
    "Optical systems": ("Optical", "systems"),
    "Masks, metrics and process window": ("Masks &", "metrics"),
    "Light sources": ("Light", "sources"),
    "Resist and development": ("Resist &", "development"),
    "Deep-layer and non-projection processes": ("Deep-layer", "processes"),
    "Optimization and patterning": ("Optimization", "& patterning"),
    "Stochastics": ("Stochastics",),
    "Research modules": ("Research", "modules"),
    "Materials and film optics": ("Materials &", "film optics"),
    "Frontends and infrastructure": ("Front ends &", "infrastructure"),
}

W = 960
MARGIN = 12
TRAILER_GAP = 8  # between the last per-grade count and the legend trailer (both start-anchored, one baseline)
COL_GAP = 5
TILE_H = 46
ROW_GAP = 5
HEADER_H = 52
FOOTER_H = 62
TEXT = 12.5
#: Split-grade wedge (the qualifier's colour): from WEDGE_TOP of the tile height down the right edge and
#: WEDGE_LEFT of the width along the bottom — the top-right corner (headline mark) stays in the headline colour.
WEDGE_TOP = 0.5
WEDGE_LEFT = 0.6
#: Width the second line of a two-line split tile leaves for the small qualifier mark at the bottom-right.
QUAL_MARK_W = 10


def _short(name: str) -> tuple[str, ...]:
    if name in SHORT:
        return SHORT[name]
    words = name.split()
    lines, cur = [], ""
    for w in words:
        if len(cur) + len(w) + (1 if cur else 0) <= 11:
            cur = f"{cur} {w}".strip()
        else:
            if cur:
                lines.append(cur)
            cur = w[:11]
    if cur:
        lines.append(cur)
    return tuple(lines[:2])


def overflowing_labels(lay: dict | None = None) -> list[str]:
    """Tile lines wider than the tile (the second line of a two-line split tile also clears the qualifier mark)."""
    lay = lay or layout()
    out = []
    for t in lay["tiles"]:
        lines = t["lines"]
        for i, line in enumerate(lines):
            avail = t["w"] - 12 - (QUAL_MARK_W if (t["row"].qualifiers and len(lines) == 2 and i == 1) else 0)
            width = K.text_width(line, TEXT, 500)
            if width > avail:
                out.append(f"label {line!r} of {t['row'].name!r} is {width:.0f} units wide; {avail:.0f} fit (shorten SHORT)")
    return out


def layout(path: Path | str | None = None) -> dict:
    """Tile geometry shared by the SVG and PNG renderers."""
    rows = matrix_parse.rows(path)
    domains = matrix_parse.domains(path)
    ncol = len(domains)
    col_w = (W - 2 * MARGIN - (ncol - 1) * COL_GAP) / ncol
    per = {d: [r for r in rows if r.domain == d] for d in domains}
    nrows = max(len(v) for v in per.values())
    height = HEADER_H + nrows * (TILE_H + ROW_GAP) + FOOTER_H
    tiles = []
    for ci, d in enumerate(domains):
        x = MARGIN + ci * (col_w + COL_GAP)
        for ri, r in enumerate(per[d]):
            y = HEADER_H + ri * (TILE_H + ROW_GAP)
            tiles.append(dict(row=r, x=x, y=y, w=col_w, h=TILE_H, col=ci, lines=_short(r.name)))
    return dict(rows=rows, domains=domains, col_w=col_w, tiles=tiles, height=height, counts=matrix_parse.counts(path),
                per=per)


def draw_svg(theme: str, lay: dict | None = None) -> str:
    lay = lay or layout()
    H = lay["height"]
    s = K.Svg(W, H, theme, "The capability matrix as a tile map",
              ALT, generator=GENERATOR)
    th = s.t
    # column headers
    for ci, d in enumerate(lay["domains"]):
        x = MARGIN + ci * (lay["col_w"] + COL_GAP)
        lines = DOMAIN_SHORT.get(d, (d,))
        for li, line in enumerate(lines):
            s.text(x + lay["col_w"] / 2, 16 + li * 15, line, size=TEXT, color="ink", weight=600)
        s.line(x, HEADER_H - 8, x + lay["col_w"], HEADER_H - 8, "axis", 1.2)
    for t in lay["tiles"]:
        r = t["row"]
        main = th["grades"][r.slug]
        x, y, w, h = t["x"], t["y"], t["w"], t["h"]
        with s.group("tile"):
            s.rect(x, y, w, h, fill=main, stroke="none", rx=5)
            if r.qualifiers:  # split grade: the qualifier's colour fills the bottom-right half
                q = th["grades"][matrix_parse.GLYPH_SLUGS[r.qualifiers[0][0]]]
                clip = s.clip_rect(x, y, w, h, rx=5)
                s.add(f'<path d="M{K.fmt(x + w)},{K.fmt(y + h * WEDGE_TOP)} L{K.fmt(x + w)},{K.fmt(y + h)} '
                      f'L{K.fmt(x + w * WEDGE_LEFT)},{K.fmt(y + h)} Z" fill="{q}" clip-path="{clip}"/>')
                K._glyph_mark(s, matrix_parse.GLYPH_SLUGS[r.qualifiers[0][0]], x + w - 9, y + h - 8, 9, tokens.text_on(q))
            ink = tokens.text_on(main)
            K._glyph_mark(s, r.slug, x + w - 16, y + 10, 11, ink)  # headline mark top-right
            lines = t["lines"]
            y0 = y + 31 if len(lines) == 1 else y + 27
            for li, line in enumerate(lines):
                s.text(x + 6, y0 + li * 14, line, size=TEXT, anchor="start", color=ink, weight=500)
    # legend + counts
    yl = H - FOOTER_H + 34
    xl = MARGIN
    s.eyebrow(xl, yl - 20, "Legend — the simulator's grades, one tile per matrix row", color="ink2")
    for slug in tokens.GRADE_ORDER:
        wch = K.chip(s, xl, yl + 6, slug, size=14)
        s.text(xl + wch + 6, yl + 11, f"· {lay['counts'][slug]}", size=TEXT, anchor="start", color="ink2", cls="count")
        xl += wch + 40
    s.text(xl + TRAILER_GAP, yl + 11, trailer(lay), size=TEXT, anchor="start", color="muted", cls="trailer")
    return s.render()


def trailer(lay: dict) -> str:
    """The legend's trailer: start-anchored after the last count so the two never run together."""
    return f"{len(lay['rows'])} rows · {len(lay['domains'])} domains · two-tone = split grade"


def draw_png(theme_name: str, lay: dict | None = None):
    """The same layout as a matplotlib figure (for the README PNG pair)."""
    import matplotlib.patches as mpatches  # noqa: PLC0415
    import matplotlib.pyplot as plt  # noqa: PLC0415

    lay = lay or layout()
    th = tokens.theme(theme_name)
    H = lay["height"]
    fig = plt.figure(figsize=(W / 100, H / 100), dpi=100)
    ax = fig.add_axes([0, 0, 1, 1])
    ax.set_xlim(0, W)
    ax.set_ylim(H, 0)
    ax.axis("off")
    for ci, d in enumerate(lay["domains"]):
        x = MARGIN + ci * (lay["col_w"] + COL_GAP)
        for li, line in enumerate(DOMAIN_SHORT.get(d, (d,))):
            ax.text(x + lay["col_w"] / 2, 16 + li * 15, line, ha="center", va="center", fontsize=9, color=th["ink"], weight="semibold")
        ax.plot([x, x + lay["col_w"]], [HEADER_H - 8] * 2, color=th["axis"], lw=0.8)
    marks = {"implemented": "✓", "simplified": "◆", "theoretical": "⚗", "planned": "▭"}
    for t in lay["tiles"]:
        r = t["row"]
        main = th["grades"][r.slug]
        x, y, w, h = t["x"], t["y"], t["w"], t["h"]
        ax.add_patch(mpatches.FancyBboxPatch((x, y), w, h, boxstyle="round,pad=0,rounding_size=5", fc=main, ec="none"))
        if r.qualifiers:
            q = th["grades"][matrix_parse.GLYPH_SLUGS[r.qualifiers[0][0]]]
            ax.add_patch(mpatches.Polygon([(x + w, y + h * WEDGE_TOP), (x + w, y + h - 5), (x + w - 5, y + h), (x + w * WEDGE_LEFT, y + h)],
                                          closed=True, fc=q, ec="none"))
            ax.text(x + w - 5, y + h - 8, marks[matrix_parse.GLYPH_SLUGS[r.qualifiers[0][0]]], ha="right", va="center", fontsize=6.5, color=tokens.text_on(q))
        ink = tokens.text_on(main)
        ax.text(x + w - 6, y + 10, marks[r.slug], ha="right", va="center", fontsize=7.5, color=ink, weight="bold")
        lines = t["lines"]
        y0 = y + 29 if len(lines) == 1 else y + 24
        for li, line in enumerate(lines):
            ax.text(x + 6, y0 + li * 14, line, ha="left", va="center", fontsize=8.6, color=ink)
    yl = H - FOOTER_H + 40
    xl = MARGIN
    ax.text(xl, yl - 24, "LEGEND — THE SIMULATOR'S GRADES, ONE TILE PER MATRIX ROW", ha="left", va="center", fontsize=8,
            color=th["ink2"], weight="semibold")
    for slug in tokens.GRADE_ORDER:
        tokens.chip(ax, slug, (xl + 6, yl), th, size_pt=8.5)
        ax.text(xl + 112, yl, f"· {lay['counts'][slug]}", ha="left", va="center", fontsize=8.6, color=th["ink2"])
        xl += 150
    ax.text(xl + TRAILER_GAP, yl, trailer(lay), ha="left", va="center", fontsize=8.6, color=th["muted"])
    return fig


ALT = ("Tile map of the capability matrix: one tile per row in eleven domain columns, coloured by headline grade "
       "(green implemented, amber simplified, purple theoretical, blue planned), split grades two-tone.")
CAPTION = ("<strong>One tile per matrix row; the row text is authoritative.</strong> Two-tone tiles are split grades "
           "(first colour = headline grade). <em>Drawn from the matrix itself.</em>")


def generate(svg_only: bool = False) -> list[Path]:
    lay = layout()
    paths = K.write_pair(NAME, lambda t: draw_svg(t, lay), OUT)
    if not svg_only:
        sys.path.insert(0, str(REPO / "docs" / "figures" / "sim"))
        import style  # noqa: PLC0415

        meta = {"Title": NAME, "Description": ALT, "Source": f"{GENERATOR} (README PNG pair)",
                "Software": "highuvlith figure scripts"}
        for th in style.THEMES:  # flat colours: an 8-bit palette with alpha keeps the 2x PNG small
            with style.use(th):
                fig = draw_png(th.name, lay)
            paths.append(style.save_figure(fig, OUT / f"{NAME}-{th.name}.png", meta, "png", palette=64))
    return paths


def check(strict: bool = False) -> tuple[list[str], list[str]]:
    errors: list[str] = []
    warnings: list[str] = []
    lay = layout()
    missing = [r.name for r in lay["rows"] if r.name not in SHORT]
    if missing:
        warnings.append(f"matrix-map: {len(missing)} row(s) have no SHORT label (auto-wrapped): {missing[:3]}…")
    for over in overflowing_labels(lay):
        errors.append(f"matrix-map: {over}")
    svg = OUT / f"{NAME}-light.svg"
    if not svg.exists():
        errors.append(f"matrix-map: {registry.rel_to_repo(svg)} missing (run make_matrix_map.py)")
        return errors, warnings
    text = svg.read_text(encoding="utf-8")
    tiles = len(re.findall(r'<g class="tile">', text))
    if tiles != len(lay["rows"]):
        errors.append(f"matrix-map: {tiles} tiles in the SVG but the matrix has {len(lay['rows'])} rows (regenerate)")
    counts = [int(m) for m in re.findall(r'class="count">· (\d+)</text>', text)]
    want = [lay["counts"][s] for s in tokens.GRADE_ORDER]
    if counts != want:
        errors.append(f"matrix-map: footer counts {counts} differ from the parsed matrix {want} (regenerate)")
    # the trailer sits on the counts' baseline: it must start past the last count and end inside the figure
    count_x = [float(x) for x in re.findall(r'<text x="([\d.]+)"[^>]*class="count">', text)]
    tm = re.search(r'<text x="([\d.]+)"[^>]*class="trailer">([^<]*)</text>', text)
    if not tm or not count_x:
        errors.append("matrix-map: legend counts or trailer missing from the SVG (regenerate)")
    else:
        tx, ttext = float(tm.group(1)), html.unescape(tm.group(2))
        last_end = count_x[-1] + K.text_width(f"· {counts[-1]}", TEXT)
        if tx < last_end + 4:
            errors.append(f"matrix-map: legend trailer at x={tx:.0f} runs into the last count (ends at {last_end:.0f})")
        if tx + K.text_width(ttext, TEXT) > W - MARGIN:
            errors.append(f"matrix-map: legend trailer overflows the figure ({tx + K.text_width(ttext, TEXT):.0f} > {W - MARGIN})")
    try:
        import make_legend  # noqa: PLC0415

        if make_legend.matrix_parse.counts() != lay["counts"]:
            errors.append("matrix-map: counts differ from make_legend.py")
    except ImportError:
        pass
    return errors, warnings


registry.register(registry.AssetSpec(
    name=NAME, kind="schematic", group="reference", subdir="reference", fmt="svg",
    alt=ALT, caption=CAPTION,
    pages=("capability-matrix.md", "reference/index.md", "README.md"),
    generator=generate, platform_independent=True,
    extra={"readme_fmt": "png", "width": {"README.md": 820}, "platform_dependent_variants": ("readme",)},
), module="make_matrix_map.py")


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if "--check" in argv:
        e, w = check(strict="--strict" in argv)
        for x in w:
            print("warning:", x)
        for x in e:
            print("error:", x)
        print(f"matrix-map check: {len(e)} error(s), {len(w)} warning(s)")
        return 1 if e else 0
    for p in generate(svg_only="--svg-only" in argv):
        print(f"wrote {registry.rel_to_repo(p)} ({p.stat().st_size / 1024:.1f} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
