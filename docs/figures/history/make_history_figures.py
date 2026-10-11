#!/usr/bin/env python3
"""Data charts for the History section of the highuvlith site (docs/history/).

Run from the repository root (numpy + matplotlib for the charts; the two svgkit timelines
need only the standard library):

    python docs/figures/history/make_history_figures.py                 # write every pair
    python docs/figures/history/make_history_figures.py --only era-ribbon
    python docs/figures/history/make_history_figures.py --table          # print the tool table
    python docs/figures/history/make_history_figures.py --table --write  # INT / nightly only

Every chart is written twice into ``docs/assets/images/history/`` (``<name>-light.svg`` and
``<name>-dark.svg``; transparent background) and registered as an ``AssetSpec`` with its
pages, alt and caption, so ``make_all.py --check`` verifies it like every other asset
(DESIGN §5.0, §5.5 N2 · N6 · N20, §5.7).

Data
----
The exposure-tool data set is ``litho_tools.csv`` next to this script, loaded once by
``history_data.py`` (shared with ``make_history_animation.py``): one row per tool
generation with year, vendor, model, wavelength, NA and the vendor-stated resolution, each
value traceable to the sources in ``history_data.SOURCES`` (S1–S41). ``k1`` is derived as
R·NA/λ and is an indicator, not a measured process value. ``--table`` prints the data set as
a column-0 GFM table (the block ``docs/history/tool-data.md`` pastes between its
``<!-- tool-table:start -->`` / ``<!-- tool-table:end -->`` markers).

Two charts are models, not data: ``proximity-gap`` plots the first-Fresnel-zone blur
√(λg), and ``psm-principle`` is a 1-D coherent scalar image of two openings. Neither is
an engine run, and their captions say so.

Colours, fonts and marker vocabulary come from ``docs/figures/tokens.py``; the chart
chrome of the matplotlib charts is ``tokens.THEMES``, the three optics classes use the
categorical slots of ``tokens.SERIES`` (``history_data.CLASSES``).
"""

from __future__ import annotations

import argparse
import logging
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
FIGURES = HERE.parent
for _p in (str(FIGURES), str(HERE)):
    if _p not in sys.path:
        sys.path.insert(0, _p)

import registry  # noqa: E402
import svgkit as K  # noqa: E402
import tokens  # noqa: E402

import history_data as hd  # noqa: E402

OUT = registry.IMAGES / "history"
GENERATOR = "docs/figures/history/make_history_figures.py"

# The matplotlib charts are drawn 8 in wide and shown in a ~700 px column, so their type
# sits one step above tokens' matplotlib base size (10.5 / 9 pt instead of 9.5 / 8.5).
BASE_PT = tokens.TYPE["mpl_base_pt"] + 1.0
SMALL_PT = tokens.TYPE["mpl_small_pt"] + 0.5

# ---------------------------------------------------------------------------------------
# matplotlib chrome through tokens
# ---------------------------------------------------------------------------------------


def mpl_theme(name: str) -> dict:
    t = dict(tokens.THEMES[name])
    t["name"] = name
    t["series"] = tokens.SERIES[name]
    return t


def style(theme: dict) -> None:
    import matplotlib.pyplot as plt  # noqa: PLC0415

    plt.rcParams.update({
        "font.family": list(tokens.FONT_MPL),
        "font.size": BASE_PT,
        "svg.fonttype": "path",
        "svg.hashsalt": "highuvlith-history",
        "path.simplify": True,
        "path.simplify_threshold": 0.5,
        "axes.edgecolor": theme["axis"],
        "axes.labelcolor": theme["ink2"],
        "axes.titlecolor": theme["ink"],
        "axes.titlesize": BASE_PT + 1.5,
        "axes.titleweight": "bold",
        "axes.titlelocation": "left",
        "axes.linewidth": 0.8,
        "axes.spines.top": False,
        "axes.spines.right": False,
        "axes.grid": True,
        "axes.axisbelow": True,
        "grid.color": theme["grid"],
        "grid.linewidth": 0.7,
        "grid.linestyle": "-",
        "xtick.color": theme["muted"],
        "ytick.color": theme["muted"],
        "xtick.labelcolor": theme["ink2"],
        "ytick.labelcolor": theme["ink2"],
        "legend.frameon": False,
        "legend.labelcolor": theme["ink2"],
        "figure.facecolor": "none",
        "axes.facecolor": "none",
        "savefig.facecolor": "none",
        "savefig.transparent": True,
    })


def series_of(theme: dict, cls: str) -> str:
    slot, _marker, _label = hd.CLASSES[cls]
    return theme["series"][slot - 1]


def scatter_class(ax, theme, pts, cls, size=8.0, zorder=3, hollow=False):
    _slot, marker, _ = hd.CLASSES[cls]
    if not pts:
        return
    xs, ys = zip(*pts)
    color = series_of(theme, cls)
    if hollow:
        ax.plot(xs, ys, linestyle="none", marker=marker, markersize=size - 1,
                markerfacecolor="none", markeredgecolor=color, markeredgewidth=1.6, zorder=zorder)
    else:
        ax.plot(xs, ys, linestyle="none", marker=marker, markersize=size,
                markerfacecolor=color, markeredgecolor=theme["surface"], markeredgewidth=1.4, zorder=zorder)


def plot_tools(ax, theme, tools, value, size=8.0, highlight=None):
    """Scatter one value per tool, grouped by optics class; later dry tools hollow.

    ``highlight(tool) -> bool`` dims every tool it rejects (used by the projection-era crop).
    """
    groups: dict[tuple[str, bool, bool], list] = {}
    for t in tools:
        v = value(t)
        if v is not None:
            dim = highlight is not None and not highlight(t)
            groups.setdefault((t.optics, t.later_dry, dim), []).append((t.year, v))
    for (cls, hollow, dim), pts in sorted(groups.items(), key=lambda kv: (kv[0][2], kv[0][1]), reverse=True):
        if dim:
            xs, ys = zip(*pts)
            ax.plot(xs, ys, linestyle="none", marker=hd.CLASSES[cls][1], markersize=size - 1.5,
                    markerfacecolor="none", markeredgecolor=theme["axis"], markeredgewidth=1.2, zorder=2)
        else:
            scatter_class(ax, theme, pts, cls, size=size, hollow=hollow)


def class_legend(ax, theme, later_dry=True, top=True, **kw):
    """Legend for the optics classes; by default one row just above the axes."""
    from matplotlib.lines import Line2D  # noqa: PLC0415

    handles = []
    for cls in hd.CLASSES:
        _slot, marker, label = hd.CLASSES[cls]
        handles.append(Line2D([], [], linestyle="none", marker=marker, markersize=8,
                              markerfacecolor=series_of(theme, cls), markeredgecolor=theme["surface"],
                              markeredgewidth=1.4, label=label))
        if cls == "dry" and later_dry:
            handles.append(Line2D([], [], linestyle="none", marker=marker, markersize=7, markerfacecolor="none",
                                  markeredgecolor=series_of(theme, cls), markeredgewidth=1.6, label=hd.LATER_DRY_LABEL))
    if top:
        kw = {"loc": "lower left", "bbox_to_anchor": (0.0, 1.01), "ncols": len(handles), "fontsize": SMALL_PT,
              "handletextpad": 0.3, "columnspacing": 1.2, "borderaxespad": 0.0, **kw}
    return ax.legend(handles=handles, **kw)


def _finish_svg(text: str, alt: str) -> str:
    """Give a matplotlib SVG the accessibility head every asset carries: role, <title> and <desc>."""
    i = text.find("<svg")
    j = text.find(">", i)
    head = text[i:j]
    if 'role="img"' not in head:
        head += ' role="img" aria-labelledby="t d"'
    return (text[:i] + head + ">\n" + f"<title id=\"t\">{K.esc(alt)}</title>\n<desc id=\"d\">{K.esc(alt)}</desc>\n"
            + f"<!-- generated by {GENERATOR}; edit the script, not this file -->" + text[j + 1:])


def save(fig, name: str, scheme: str, alt: str) -> Path:
    import io  # noqa: PLC0415

    import matplotlib.pyplot as plt  # noqa: PLC0415

    OUT.mkdir(parents=True, exist_ok=True)
    path = OUT / f"{name}-{scheme}.svg"
    buf = io.StringIO()
    fig.savefig(buf, format="svg", bbox_inches="tight", pad_inches=0.08,
                metadata={"Date": None, "Creator": GENERATOR})
    plt.close(fig)
    path.write_text(_finish_svg(buf.getvalue(), alt), encoding="utf-8")
    return path


def year_axis(ax, lo=1971, hi=2027):
    ax.set_xlim(lo, hi)
    ax.set_xticks(range(1975, 2026, 5))


def annotate(ax, theme, text, xy, xytext, ha="left"):
    ax.annotate(text, xy=xy, xytext=xytext, textcoords="data", fontsize=SMALL_PT, ha=ha, va="center",
                color=theme["ink2"], arrowprops=dict(arrowstyle="-", color=theme["muted"], lw=0.8, shrinkA=2, shrinkB=5))


# ---------------------------------------------------------------------------------------
# Alt texts and captions (the registry's canonical text; pages paste --catalog output)
# ---------------------------------------------------------------------------------------

N_TOOLS = len(hd.load_tools())

RESOLUTION_ALT = ("Four stacked charts sharing a year axis from 1973 to 2027: exposure wavelength falling from 436 to "
                  "13.5 nm, numerical aperture rising to 1.35 in water, vendor-stated resolution falling from 1000 to "
                  "8 nm on a log scale, and the derived k1 falling from about 0.8 to 0.27 before EUV restarts near 0.5.")
RESOLUTION_CAPTION = (
    "<strong>Three levers, one dot per tool model.</strong> Squares are 193 nm water-immersion scanners, diamonds EUV "
    "systems, circles dry tools (open from 2007 on: newer models of older exposure technologies); the two Micrascan "
    "points near 250 nm used a filtered mercury-lamp band and the 157 nm point is the Micrascan VII prototype. "
    "k₁ = R·NA/λ from the vendor's stated resolution, which vendors define differently, so read the trend. "
    "<em>Data chart from litho_tools.csv; no model badge.</em>")

NA_ALT = ("Numerical aperture of each tool model against its year from 1973 to 2025: dry lenses climb from 0.17 to "
          "0.93, water immersion lifts the ceiling to 1.35 in 2007, EUV restarts at 0.25 to 0.33 and High-NA EUV "
          "reaches 0.55 in 2023.")
NA_CAPTION = ("<strong>Numerical aperture of the tool models in the data set.</strong> Dry NA climbed until it neared "
              "the physical limit of 1; water immersion moved the ceiling to 1.35; EUV started again at low NA with "
              "mirror optics. Open circles are dry tools introduced from 2007 on. "
              "<em>Data chart from litho_tools.csv; no model badge.</em>")

NA2000_ALT = ("Numerical aperture of each tool model from 1973 to 2000, the projection-era tools drawn in colour and "
              "later tools greyed out: the Micralign at 0.17, the GCA DSW 4800 stepper at 0.28, i-line steppers near "
              "0.63 and the first step-and-scan tools at 0.6 to 0.68.")
NA2000_CAPTION = ("<strong>NA through the projection era, 1973–2000.</strong> The same chart as the full NA history, "
                  "cut at 2000 with the projection-era tools in colour: every step is a larger lens, from the Micralign's "
                  "mirrors at 0.17 to 0.68 at the end of the century. "
                  "<em>Data chart from litho_tools.csv; no model badge.</em>")

K1_ALT = ("The derived process factor k1 for each tool model from 1976 to 2025: about 0.8 for g- and i-line steppers, "
          "falling below 0.5 with off-axis illumination and phase-shift masks around 1999, to 0.27 for NA 1.35 immersion "
          "scanners, and restarting near 0.5 with EUV in 2010.")
K1_CAPTION = ("<strong>k₁ = R·NA/λ, derived from each vendor's stated resolution.</strong> Vendors define "
              "resolution differently, so read the trend, not individual points: g/i-line steppers near 0.8, the "
              "resolution-enhancement decade taking it below 0.5, immersion at the 0.25 floor, EUV restarting high. "
              "<em>Data chart from litho_tools.csv; no model badge.</em>")

LANES_ALT = ("Timeline lanes, one per exposure technology, with a dot for each tool model introduced: g-line 1978 to "
             "1990, i-line 1984 to 2025, KrF 1988 to 2021, dry ArF 1998 to 2025, immersion ArF 2004 to 2023, EUV 2006 "
             "to 2024, High-NA EUV 2023 and 2025.")
LANES_CAPTION = ("<strong>Old wavelengths never die.</strong> When each exposure technology received new tool models, "
                 "from the same data set: a sample of Nikon, Canon, ASML, GCA and Perkin-Elmer/SVG models, not a "
                 "catalogue, so a gap means only that no model from those years was included. "
                 "<em>Data chart from litho_tools.csv; no model badge.</em>")

PROXIMITY_ALT = ("Edge blur of proximity printing, the square root of wavelength times gap, against gap from 1 to "
                 "1000 micrometres for 436 nm light, 1 nm soft X-rays and 0.2 nm hard X-rays: about 3 micrometres for "
                 "g-line at a 20 micrometre gap and sub-micrometre blur for X-rays at tens of micrometres.")
PROXIMITY_CAPTION = ("<strong>The proximity trade-off, R ≈ √(λg).</strong> Sub-micrometre shadow printing at a "
                     "workable gap needs a wavelength near 1 nm — the reasoning behind proximity X-ray lithography, "
                     "and why LIGA can print through hundreds of micrometres of resist. The lines are the formula; the "
                     "marked points are one sourced X-ray example and the simulator's LIGA preset. "
                     "<em>Closed form; not the engine.</em>")

PSM_ALT = ("Two columns of three panels: for a binary mask and an alternating phase-shift mask, the mask amplitude, "
           "the field at the wafer and the intensity at the wafer for two openings; the binary openings merge into one "
           "bright peak, the phase-shifted openings leave a dark null between two peaks.")
PSM_CAPTION = ("<strong>Levenson's idea in one dimension.</strong> The same lens (coherent illumination, pupil cutoff "
               "0.4 per opening width) images two openings; with opposite phases the field must pass through zero "
               "between them, and intensity is the square of the field. A 1-D scalar numpy model, not a highuvlith "
               "run: the engine treats an alternating PSM as binary. <em>Closed form; not the engine.</em>")

RIBBON_ALT = ("A timeline from 1796 to 2027 with the axis broken at 1960: dated events on a top rail from Senefelder's "
              "printing to High-NA EUV, one lane per exposure technology spanning its first to last tool model in the "
              "data set, and the seven history chapters as shaded spans beneath the year axis.")
RIBBON_CAPTION = ("<strong>Nine exposure technologies, first to last tool model in the data set.</strong> Band ends are "
                  "sampling artefacts of an 86-tool sample, not retirement dates; the contact/proximity lane and the "
                  "events on the top rail are dated in the chapters, and the 157 nm lane is dashed because no production "
                  "tool shipped. <em>Data chart from litho_tools.csv; no model badge.</em>")

PARALLEL_ALT = ("Nine timeline lanes from 1960 to 2025 for proximity X-ray, LIGA, electron-beam writing, electron "
                "projection, ion beams, multi-beam writers, nanoimprint, directed self-assembly and interference "
                "lithography, with dated first-report, milestone, abandoned and still-used markers and reference numbers.")
PARALLEL_CAPTION = ("<strong>Nine paths beside projection, 1960–2025.</strong> Filled dots are first reports, small "
                    "dots milestones, rings an abandoned or shelved line, arrows a path still in use; every marker "
                    "carries its reference number from this chapter, and 'abandoned' appears only where a decision is "
                    "cited. <em>Data chart from the chapter's references; no model badge.</em>")


# ---------------------------------------------------------------------------------------
# matplotlib charts
# ---------------------------------------------------------------------------------------


def fig_resolution_story(tools, theme, scheme):
    import matplotlib.pyplot as plt  # noqa: PLC0415

    fig, axes = plt.subplots(4, 1, figsize=(8, 10.6), sharex=True, gridspec_kw={"hspace": 0.36})
    ax_l, ax_na, ax_r, ax_k = axes
    ink2, muted = theme["ink2"], theme["muted"]
    ref = dict(color=muted, lw=1.1, ls=(0, (4, 3)), zorder=2)

    # 1. Wavelength (linear axis: the lanes stay apart and the EUV drop reads as a drop)
    plot_tools(ax_l, theme, tools, lambda t: t.wavelength_nm)
    ax_l.set_ylim(0, 480)
    ax_l.set_yticks([13.5, 193, 248, 365, 436])
    ax_l.set_yticklabels(["EUV 13.5", "ArF 193", "KrF 248", "i-line 365", "g-line 436"])
    annotate(ax_l, theme, "F₂ 157 nm prototype (2003)", (2003, 157), (1985.5, 95))
    ax_l.set_ylabel("wavelength λ (nm)")
    ax_l.set_title("1 · Shorter wavelength λ", pad=30)
    class_legend(ax_l, theme)

    # 2. Numerical aperture
    plot_tools(ax_na, theme, tools, lambda t: t.na)
    ax_na.set_ylim(0, 1.6)
    ax_na.axhline(1.0, **ref)
    ax_na.axhline(1.44, **ref)
    ax_na.text(1972, 1.035, "NA = 1: limit for a dry lens", color=ink2, fontsize=SMALL_PT, va="bottom")
    ax_na.text(1972, 1.475, "water at 193 nm (n ≈ 1.44)", color=ink2, fontsize=SMALL_PT, va="bottom")
    ax_na.set_ylabel("numerical aperture NA")
    ax_na.set_title("2 · Larger numerical aperture NA", pad=8)

    # 3. Resolution
    plot_tools(ax_r, theme, tools, lambda t: t.resolution_nm)
    ax_r.set_yscale("log")
    ax_r.set_ylim(5, 2000)
    ax_r.set_yticks([8, 13, 38, 100, 250, 500, 1000])
    ax_r.set_yticklabels(["8", "13", "38", "100", "250", "500", "1000"])
    ax_r.minorticks_off()
    ax_r.set_ylabel("resolution R (nm)")
    ax_r.set_title("3 · Vendor-stated resolution R (log scale)", pad=8)

    # 4. k1
    plot_tools(ax_k, theme, tools, lambda t: t.k1)
    ax_k.set_ylim(0.15, 0.95)
    ax_k.axhline(0.25, **ref)
    ax_k.text(1972, 0.235, "k₁ = 0.25: single-exposure limit for dense lines", color=ink2, fontsize=SMALL_PT, va="top")
    ax_k.set_ylabel("k₁ = R · NA / λ")
    ax_k.set_title("4 · Smaller process factor k₁ (derived from 1–3)", pad=8)
    year_axis(ax_k)
    ax_k.set_xlabel("year of introduction or first shipment")
    for ax in axes:
        ax.tick_params(axis="x", labelbottom=True)
    fig.align_ylabels(axes)
    return save(fig, "resolution-story", scheme, RESOLUTION_ALT)


def fig_na(tools, theme, scheme, through: int | None = None):
    """NA vs year; ``through=2000`` is the projection-era crop (N20) with later tools greyed out."""
    import matplotlib.pyplot as plt  # noqa: PLC0415

    fig, ax = plt.subplots(figsize=(8, 4.8))
    muted, ink2 = theme["muted"], theme["ink2"]
    ref = dict(color=muted, lw=1.1, ls=(0, (4, 3)), zorder=2)
    if through is None:
        plot_tools(ax, theme, tools, lambda t: t.na, size=8.5)
        ax.axhline(1.0, **ref)
        ax.axhline(1.44, **ref)
        ax.text(2026.5, 1.035, "NA = 1: dry-lens limit", color=ink2, fontsize=SMALL_PT, ha="right", va="bottom")
        ax.text(2026.5, 1.475, "water at 193 nm, n ≈ 1.44", color=ink2, fontsize=SMALL_PT, ha="right", va="bottom")
        annotate(ax, theme, "Micralign 1:1 scanner, 0.167", (1973, 0.167), (1975.5, 0.07))
        annotate(ax, theme, "GCA DSW 4800 stepper, 0.28", (1978, 0.28), (1981, 0.19))
        annotate(ax, theme, "Nikon NSR-S201A scanner, 0.60", (1995, 0.60), (1993.5, 0.86), "right")
        annotate(ax, theme, "Nikon NSR-S609B: first NA > 1 (1.07)", (2005, 1.07), (2003.2, 1.13), "right")
        annotate(ax, theme, "ASML XT:1900i: 1.35", (2007, 1.35), (2003.2, 1.31), "right")
        annotate(ax, theme, "EUV NXE:3300B: 0.33", (2013, 0.33), (2015.3, 0.19))
        annotate(ax, theme, "High-NA EUV EXE:5000: 0.55", (2023, 0.55), (2005.5, 0.49))
        ax.set_ylim(0, 1.6)
        year_axis(ax)
        class_legend(ax, theme)
        name, alt = "na-vs-year", NA_ALT
    else:
        sel = [t for t in tools if t.year <= through]
        plot_tools(ax, theme, sel, lambda t: t.na, size=8.5)
        ax.axhline(1.0, **ref)
        ax.text(through + 0.3, 1.035, "NA = 1: dry-lens limit", color=ink2, fontsize=SMALL_PT, ha="right", va="bottom")
        annotate(ax, theme, "Micralign 1:1 mirror scanner, 0.167", (1973, 0.167), (1975.5, 0.06))
        annotate(ax, theme, "GCA DSW 4800 stepper, 0.28", (1978, 0.28), (1980.5, 0.17))
        annotate(ax, theme, "Nikon NSR-2205i11D i-line, 0.63", (1994, 0.63), (1980.5, 0.72))
        annotate(ax, theme, "NSR-S201A KrF scanner, 0.60", (1995, 0.60), (1986.5, 0.50))
        annotate(ax, theme, "NSR-S203B / S204B, 0.68", (1998.5, 0.68), (1987.5, 0.86))
        annotate(ax, theme, "Micrascan I, 0.35", (1990, 0.35), (1983, 0.30))
        ax.set_ylim(0, 1.1)
        ax.set_xlim(1971, through + 0.5)
        ax.set_xticks(range(1975, through + 1, 5))
        class_legend(ax, theme, later_dry=False)
        name, alt = "na-vs-year-to-2000", NA2000_ALT
    ax.set_ylabel("numerical aperture NA")
    ax.set_xlabel("year of introduction or first shipment")
    return save(fig, name, scheme, alt)


def fig_k1(tools, theme, scheme):
    import matplotlib.pyplot as plt  # noqa: PLC0415

    fig, ax = plt.subplots(figsize=(8, 4.6))
    muted, ink2 = theme["muted"], theme["ink2"]
    plot_tools(ax, theme, tools, lambda t: t.k1, size=8.5)
    ax.axhline(0.25, color=muted, lw=1.1, ls=(0, (4, 3)), zorder=2)
    ax.text(1977, 0.235, "k₁ = 0.25: single-exposure limit for dense line/space", color=ink2, fontsize=SMALL_PT, va="top")
    annotate(ax, theme, "g/i-line steppers: k₁ ≈ 0.8", (1986, 0.79), (1989.5, 0.86))
    annotate(ax, theme, "off-axis illumination, phase-shift\nmasks and OPC take k₁ below 0.5", (1999, 0.41), (1980.5, 0.44))
    annotate(ax, theme, "immersion at NA 1.35: k₁ ≈ 0.27", (2012, 0.266), (2005.5, 0.19))
    annotate(ax, theme, "EUV restarts near k₁ ≈ 0.5", (2013, 0.538), (2014.5, 0.69))
    ax.set_ylim(0.15, 0.95)
    year_axis(ax, 1976, 2027)
    ax.set_xticks(range(1980, 2026, 5))
    ax.set_ylabel("k₁ = R · NA / λ")
    ax.set_xlabel("year of introduction or first shipment")
    class_legend(ax, theme)
    return save(fig, "k1-vs-year", scheme, K1_ALT)


def fig_lanes(tools, theme, scheme):
    import matplotlib.pyplot as plt  # noqa: PLC0415

    fig, ax = plt.subplots(figsize=(8, 3.9))
    ink2 = theme["ink2"]
    n = len(hd.LANES)
    for i, (label, _short, test) in enumerate(hd.LANES):
        y = n - 1 - i
        sel = [t for t in tools if test(t)]
        if not sel:
            continue
        cls = sel[0].optics
        years = hd.lane_years(tools, test)
        if len(years) > 1:
            ax.plot([years[0], years[-1]], [y, y], color=series_of(theme, cls), lw=2, alpha=0.35,
                    solid_capstyle="round", zorder=2)
        scatter_class(ax, theme, [(yr, y) for yr in years], cls, size=8)
        ax.text(years[0] - 0.9, y, label, ha="right", va="center", color=ink2, fontsize=SMALL_PT + 0.5)
    ax.set_ylim(-0.7, n - 0.3)
    ax.set_yticks([])
    ax.spines["left"].set_visible(False)
    ax.grid(axis="y", visible=False)
    year_axis(ax, 1958, 2027)
    ax.set_xticks(range(1975, 2026, 5))
    ax.set_xlabel("year in which a tool model in the data set was introduced or first shipped")
    return save(fig, "tool-lanes", scheme, LANES_ALT)


def fig_proximity(theme, scheme):
    """First-Fresnel-zone blur √(λg) for three wavelengths (a closed form, not an engine run).

    Marked points: 100 nm features at a 15 µm gap with ~1 nm X-rays (H. I. Smith and
    M. L. Schattenburg, IBM J. Res. Dev. 37, 319 (1993)); the 100 µm gap of highuvlith's
    LIGA preset (DeepXrayConfig::pmma_default); an illustrative 20 µm g-line gap.
    """
    import matplotlib.pyplot as plt  # noqa: PLC0415
    import numpy as np  # noqa: PLC0415

    fig, ax = plt.subplots(figsize=(8, 4.6))
    ink2 = theme["ink2"]
    g_um = np.logspace(0, 3, 200)  # 1 µm .. 1 mm
    cases = [
        (0.436, "g-line lamp, 436 nm", 0),
        (0.001, "soft X-ray (proximity X-ray lithography), 1 nm", 1),
        (0.0002, "hard X-ray (LIGA), 0.2 nm ≈ 6 keV", 2),
    ]
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlim(1, 1000)
    ax.set_ylim(0.008, 40)
    handles = []
    for lam_um, label, slot in cases:
        (line,) = ax.plot(g_um, np.sqrt(lam_um * g_um), color=theme["series"][slot], lw=2, solid_capstyle="round",
                          label=label)
        handles.append(line)
    fig.canvas.draw()  # fix the transforms before measuring the on-screen slope
    for lam_um, label, _ in cases:
        x0, x1 = 1.3, 10.0
        p0 = ax.transData.transform((x0, np.sqrt(lam_um * x0)))
        p1 = ax.transData.transform((x1, np.sqrt(lam_um * x1)))
        angle = np.degrees(np.arctan2(p1[1] - p0[1], p1[0] - p0[0]))
        ax.text(x0, np.sqrt(lam_um * x0) * 1.35, label, color=ink2, fontsize=SMALL_PT + 0.5, rotation=angle,
                rotation_mode="anchor", ha="left", va="bottom")
    marks = [  # (gap µm, wavelength µm, slot, text, text position)
        (20.0, 0.436, 0, "20 µm gap: ≈ 3 µm", (32.0, 1.6)),
        (15.0, 0.001, 1, "100 nm features at a 15 µm gap\n(Smith & Schattenburg, 1993)", (1.6, 0.36)),
        (100.0, 0.0002, 2, "100 µm gap of highuvlith's\nLIGA preset", (140.0, 0.04)),
    ]
    for g, lam_um, slot, text, (tx, ty) in marks:
        y = np.sqrt(lam_um * g)
        ax.plot([g], [y], marker="o", markersize=8.5, linestyle="none", zorder=4, markerfacecolor=theme["series"][slot],
                markeredgecolor=theme["surface"], markeredgewidth=1.4)
        ax.annotate(text, xy=(g, y), xytext=(tx, ty), fontsize=SMALL_PT, color=ink2, va="center",
                    arrowprops=dict(arrowstyle="-", color=theme["muted"], lw=0.8, shrinkA=2, shrinkB=5))
    ax.set_xticks([1, 10, 100, 1000])
    ax.set_xticklabels(["1", "10", "100", "1000"])
    ax.set_yticks([0.01, 0.1, 1, 10])
    ax.set_yticklabels(["0.01", "0.1", "1", "10"])
    ax.minorticks_off()
    ax.set_xlabel("mask–resist gap g (µm)")
    ax.set_ylabel("edge blur √(λg) (µm)")
    ax.legend(handles=handles, loc="lower left", bbox_to_anchor=(0.0, 1.01), ncols=3, fontsize=SMALL_PT,
              handlelength=1.6, columnspacing=1.2, borderaxespad=0.0)
    return save(fig, "proximity-gap", scheme, PROXIMITY_ALT)


def fig_psm(theme, scheme):
    """Binary mask vs alternating phase-shift mask, 1-D coherent scalar imaging (a closed form)."""
    import matplotlib.pyplot as plt  # noqa: PLC0415
    import numpy as np  # noqa: PLC0415

    n, L = 4096, 64.0
    x = (np.arange(n) - n / 2) * (L / n)
    f = np.fft.fftfreq(n, d=L / n)
    w, d, fc = 1.0, 2.0, 0.4  # opening width, centre spacing, pupil cutoff NA/λ
    left = (np.abs(x + d / 2) <= w / 2).astype(float)
    right = (np.abs(x - d / 2) <= w / 2).astype(float)
    masks = {"Binary mask": left + right, "Alternating phase-shift mask": left - right}
    fig, axes = plt.subplots(3, 2, figsize=(8, 5.6), sharex=True, gridspec_kw={"hspace": 0.5, "wspace": 0.22})
    c0 = theme["series"][0]
    muted, ink2 = theme["muted"], theme["ink2"]
    keep = np.abs(x) <= 3.2
    for col, (title, t_mask) in enumerate(masks.items()):
        field = np.fft.ifft(np.fft.fft(t_mask) * (np.abs(f) <= fc)).real
        inten = field**2
        rows = [(t_mask, "mask amplitude"), (field, "field at wafer"), (inten, "intensity at wafer")]
        for row, (y, ylab) in enumerate(rows):
            ax = axes[row, col]
            ax.axhline(0, color=theme["axis"], lw=0.8)
            ax.plot(x[keep], y[keep], color=c0, lw=2, solid_capstyle="round")
            ax.set_ylim(-1.3, 1.3) if row < 2 else ax.set_ylim(-0.05, 1.3)
            ax.set_yticks([-1, 0, 1] if row < 2 else [0, 1])
            if col == 0:
                ax.set_ylabel(ylab)
            ax.grid(axis="x", visible=False)
        axes[0, col].set_title(title, pad=8)
        if col == 1:
            axes[0, col].text(-1.0, 1.08, "0°", ha="center", color=ink2, fontsize=SMALL_PT)
            axes[0, col].text(1.0, -1.25, "180°", ha="center", color=ink2, fontsize=SMALL_PT, va="bottom")
    binary_inten = np.fft.ifft(np.fft.fft(left + right) * (np.abs(f) <= fc)).real ** 2
    axes[2, 0].annotate("openings merge", xy=(0, float(np.interp(0, x, binary_inten))), xytext=(0, 1.2), ha="center",
                        fontsize=SMALL_PT, color=ink2, arrowprops=dict(arrowstyle="-", color=muted, lw=0.8))
    axes[2, 1].annotate("dark null between openings", xy=(0, 0.0), xytext=(0, 1.2), ha="center", fontsize=SMALL_PT,
                        color=ink2, arrowprops=dict(arrowstyle="-", color=muted, lw=0.8))
    for ax in axes[2]:
        ax.set_xlabel("position (units of opening width)")
        ax.set_xlim(-3.2, 3.2)
    return save(fig, "psm-principle", scheme, PSM_ALT)


# ---------------------------------------------------------------------------------------
# N2 · the era ribbon (svgkit): one lane per exposure technology on a broken 1796–2027 axis
# ---------------------------------------------------------------------------------------

#: The dated events of the old index timeline (year, label) — dated in the chapters.
RIBBON_EVENTS: list[tuple[float, str]] = [
    (1796, "Senefelder: lithographic printing"),
    (1827, "Niépce: light-hardened bitumen image"),
    (1957.5, "1955–60: resist on oxide, planar process and IC"),
    (1973, "Micralign scanning projection"),
    (1978, "GCA DSW 4800 wafer stepper"),
    (1985, "1982–88: excimers, CA resists, first KrF stepper"),
    (1993.5, "1990–97: step-and-scan"),
    (1998.5, "1998–99: first ArF scanners"),
    (2003.5, "2003–04: 157 nm abandoned"),
    (2005.5, "2004–07: immersion → NA 1.35"),
    (2009.5, "2006–13: EUV demo tools"),
    (2018.5, "2018–19: EUV in volume"),
    (2024, "2023–25: High-NA EUV"),
]

#: Lanes of the ribbon: (label, wavelength for the colour or None, test over the tools or a fixed
#: (first, last) span with a note, style).
RIBBON_LANES = [
    ("Contact / proximity", None, (1960, 1975), "chapter dates: aligners of the 1960s–70s", "hatched"),
    ("Micralign 1:1 scanning", 400.0, lambda t: t.model == "Micralign", None, "solid"),
    ("g / i-line steppers", 436.0, lambda t: t.wavelength_nm in (436, 365), None, "solid"),
    ("KrF 248 nm", 248.0, lambda t: t.wavelength_nm in (248, 250), None, "solid"),
    ("ArF dry 193 nm", 193.0, lambda t: t.wavelength_nm == 193 and t.optics == "dry", None, "solid"),
    ("F₂ 157 nm (prototype only)", 157.0, lambda t: t.wavelength_nm == 157, None, "dashed"),
    ("ArF immersion 193 nm", 193.0, lambda t: t.optics == "immersion", None, "solid"),
    ("EUV 13.5 nm, NA 0.25–0.33", 13.5, lambda t: t.optics == "euv" and t.na is not None and t.na < 0.5, None, "solid"),
    ("High-NA EUV, NA 0.55", 13.5, lambda t: t.optics == "euv" and t.na is not None and t.na >= 0.5, None, "solid"),
]

#: The seven chapters as (title, first year, last year) spans; dates from the chapters' leads.
CHAPTERS = [
    ("From stone to silicon", 1796, 1972),
    ("The projection era", 1973, 1997),
    ("Excimer lasers & deep UV", 1982, 2003),
    ("The 157 nm detour", 1997, 2004),
    ("Immersion & multipatterning", 2004, 2016),
    ("The road to EUV", 1984, 2025),
    ("Parallel paths", 1960, 2025),
]


class BrokenAxis:
    """x(year) on an axis broken at ``brk``: the left segment is compressed."""

    def __init__(self, y0: float, brk: float, y1: float, x0: float, xb0: float, xb1: float, x1: float):
        self.y0, self.brk, self.y1 = y0, brk, y1
        self.x0, self.xb0, self.xb1, self.x1 = x0, xb0, xb1, x1

    def x(self, year: float) -> float:
        if year <= self.brk:
            return self.x0 + (year - self.y0) / (self.brk - self.y0) * (self.xb0 - self.x0)
        return self.xb1 + (year - self.brk) / (self.y1 - self.brk) * (self.x1 - self.xb1)


def _lane_span(tools, lane) -> tuple[float, float, list[int], str | None]:
    label, lam, test, note, _style = lane
    if isinstance(test, tuple):
        return test[0], test[1], [], note
    years = hd.lane_years(tools, test)
    return years[0], years[-1], years, None


def draw_era_ribbon(theme: str, *, narrow: bool = False) -> str:
    tools = hd.load_tools()
    W, H = (600, 360) if narrow else (960, 466)
    s = K.Svg(W, H, theme, "The whole arc on one timeline", RIBBON_ALT, generator=GENERATOR)
    hist = s.c("hue:history")
    if narrow:
        ax = BrokenAxis(1955, 1960, 2027, 20, 20, 20, 586)  # no compressed left segment on the phone twin
        y_axis = 332
        s.eyebrow(12, 20, "Exposure technologies, first to last tool model", size=12.5)
        s.text(12, 38, "1796 Senefelder · 1827 Niépce · 1955–60 planar silicon: see the chapters", size=12.5,
               anchor="start", color="muted")
        lane_y0, lane_h = 58, 29
    else:
        ax = BrokenAxis(1796, 1960, 2027, 74, 196, 214, 906)
        y_axis = 354
        lane_y0, lane_h = 160, 20
        s.eyebrow(12, 20, "The whole arc on one timeline, 1796–2027", size=12.5)
    # --- year axis with the break
    s.line(ax.x(ax.y0), y_axis, ax.x(ax.y1), y_axis, "axis", 1.2)
    if not narrow:
        for dx in (0, 6):
            s.line(ax.xb0 + 2 + dx, y_axis - 7, ax.xb0 + 8 + dx, y_axis + 7, "surface", 4)
            s.line(ax.xb0 + 2 + dx, y_axis - 7, ax.xb0 + 8 + dx, y_axis + 7, "axis", 1.2)
        for yr in (1796, 1850, 1900, 1950):
            s.line(ax.x(yr), y_axis, ax.x(yr), y_axis + 5, "axis", 1.0)
            s.text(ax.x(yr), y_axis + 20, str(yr), size=12.5, color="ink2")
    for yr in range(1960, 2030, 10):
        s.line(ax.x(yr), y_axis, ax.x(yr), y_axis + 5, "axis", 1.0)
        s.text(ax.x(yr), y_axis + 20, str(yr), size=12.5, color="ink2")
        if yr > 1960:
            s.line(ax.x(yr), lane_y0 - 6, ax.x(yr), y_axis, "grid", 0.8)
    # --- the event rail (desktop only): ticks and labels in five staggered rows, either side of the tick
    if not narrow:
        rail_y = 36
        rows = [rail_y + 14.5 * i for i in range(7)]
        taken: list[list[tuple[float, float]]] = [[] for _ in rows]
        for year, label in RIBBON_EVENTS:
            x = ax.x(year)
            w = K.text_width(label, 12.5) * 0.92
            choice = None
            for i in range(len(rows)):
                for anchor in ("start", "end"):
                    xa = x + 5 if anchor == "start" else x - 5 - w
                    xb = xa + w
                    if xa < 4 or xb > W - 4:
                        continue
                    if all(xb < a - 6 or xa > b + 6 for a, b in taken[i]):
                        choice = (i, anchor, xa, xb)
                        break
                if choice:
                    break
            i, anchor, xa, xb = choice or (len(rows) - 1, "start", x + 5, x + 5 + w)
            taken[i].append((xa, xb))
            s.line(x, rows[i] + 4, x, lane_y0 - 10, "muted", 0.8, dash="2 3", opacity=0.9)
            s.circle(x, rows[i] + 4, 2.6, fill=hist, stroke="none")
            s.text(xa if anchor == "start" else xb, rows[i] + 8, label, size=12.5, anchor=anchor, color="ink2", halo=True)
    # --- lanes
    for i, lane in enumerate(RIBBON_LANES):
        label, lam, _test, _note, style = lane
        y = lane_y0 + i * lane_h + lane_h / 2
        a, b, years, note = _lane_span(tools, lane)
        xa, xb = ax.x(a), ax.x(max(b, a + 0.6))
        colour = tokens.wavelength_color(lam) if lam else s.c("muted")
        bar_h = 10 if not narrow else 9
        if style == "hatched":
            s.rect(xa, y - bar_h / 2, xb - xa, bar_h, fill=s.hatch(f"h{i}{theme[0]}", colour, 5, 1), stroke=colour,
                   sw=0.8, rx=bar_h / 2)
        elif style == "dashed":
            s.rect(xa - 9, y - bar_h / 2, max(xb - xa, 1) + 18, bar_h, fill="none", stroke=colour, sw=1.2,
                   rx=bar_h / 2, dash="4 3")
        else:
            s.rect(xa - 4, y - bar_h / 2, (xb - xa) + 8, bar_h, fill=tokens.mix(colour, s.t["surface"], 0.35),
                   stroke="none", rx=bar_h / 2)
        for yr in years:
            s.circle(ax.x(yr), y, 2.6 if not narrow else 2.2, fill=colour, stroke="surface", sw=1)
        if narrow:
            s.text(12, y - bar_h / 2 - 3, label, size=12.5, anchor="start", color="ink2", halo=True)
        else:
            s.text(ax.xb1 - 24, y + 4.5, label, size=12.5, anchor="end", color="ink2")
            if note:
                s.text(xb + 10, y + 4.5, note, size=12.5, anchor="start", color="muted", italic=True)
    if narrow:
        s.text(586, y_axis + 20, "year", size=12.5, anchor="end", color="muted")
        return s.render()
    # --- chapter spans under the axis, packed into rows
    cy0 = y_axis + 36
    row_h = 17
    rows_end: list[float] = []
    for title, a, b in CHAPTERS:
        xa, xb = ax.x(a), ax.x(b)
        tw = K.text_width(title, 12.5)
        xe = max(xb, xa + 6 + tw + 4)
        row = next((i for i, e in enumerate(rows_end) if xa > e + 8), None)
        if row is None:
            rows_end.append(xe)
            row = len(rows_end) - 1
        else:
            rows_end[row] = xe
        y = cy0 + row * row_h
        s.rect(xa, y - 6, xb - xa, 12, fill=s.tint("hue:history", 0.18), stroke="none", rx=3)
        s.line(xa, y - 6, xa, y + 6, hist, 1.4)
        s.text(xa + 6, y + 4.5, title, size=12.5, anchor="start", color="ink", halo=True)
    s.text(W - 8, H - 6, "Chapters as shaded spans; lane ends = first and last tool model in the data set.",
           size=12.5, anchor="end", color="muted")
    return s.render()


# ---------------------------------------------------------------------------------------
# N6 · parallel lanes (svgkit): the paths beside projection, with reference numbers
# ---------------------------------------------------------------------------------------

#: (lane, year, kind, label, refs). kind ∈ first · note · end (abandoned/shelved) · alive (still used).
#: ``dashed`` years are unconfirmed in the chapter (drawn hollow-dashed). Every entry cites the
#: reference numbers of docs/history/parallel-paths.md.
EVENTS: list[dict] = [
    dict(lane="Proximity X-ray", year=1972, kind="first", label="Spears & Smith, soft X-rays", refs="2"),
    dict(lane="Proximity X-ray", year=1980, kind="note", label="IBM synchrotron programme", refs="4"),
    dict(lane="Proximity X-ray", year=1991, kind="note", label="Helios ring, IBM", refs="4"),
    dict(lane="Proximity X-ray", year=1998, kind="note", label="SEMATECH narrows NGL", refs="6"),
    dict(lane="Proximity X-ray", year=2000, kind="end", label="never mainstream", refs="40", dashed=True),
    dict(lane="LIGA", year=1982, kind="first", label="Becker, Ehrfeld: X-ray + plating", refs="7"),
    dict(lane="LIGA", year=1986, kind="note", label="the LIGA process", refs="8"),
    dict(lane="LIGA", year=2025, kind="alive", label="KIT beamlines (KARA)", refs="9"),
    dict(lane="Electron-beam writing", year=1960, kind="first", label="Möllenstedt micro-writer", refs="10"),
    dict(lane="Electron-beam writing", year=1968, kind="note", label="PMMA", refs="11"),
    dict(lane="Electron-beam writing", year=1975, kind="note", label="EBES", refs="12"),
    dict(lane="Electron-beam writing", year=1978, kind="note", label="VSB", refs="13"),
    dict(lane="Electron-beam writing", year=2025, kind="alive", label="mask writing", refs="1, 13"),
    dict(lane="Electron projection", year=1990, kind="first", label="SCALPEL", refs="15"),
    dict(lane="Electron projection", year=1999, kind="note", label="PREVAIL; eLITH", refs="16, 17"),
    dict(lane="Electron projection", year=2000, kind="note", label="eLITH closed", refs="17, 44"),
    dict(lane="Electron projection", year=2005, kind="end", label="Nikon stops EPL", refs="17"),
    dict(lane="Ion beams", year=1987, kind="first", label="FIB mask repair", refs="26"),
    dict(lane="Ion beams", year=1998, kind="note", label="IPL (MEDEA)", refs="25"),
    dict(lane="Ion beams", year=2001, kind="end", label="IPL ends; IMS pivots", refs="20, 25", dashed=True),
    dict(lane="Ion beams", year=2006, kind="note", label="He-ion microscope", refs="27"),
    dict(lane="Ion beams", year=2025, kind="alive", label="FIB repair, edit", refs="26"),
    dict(lane="Multi-beam writers", year=2000, kind="first", label="MAPPER", refs="18"),
    dict(lane="Multi-beam writers", year=2016, kind="note", label="IMS alpha (2014), MBMW-101", refs="20, 22"),
    dict(lane="Multi-beam writers", year=2018, kind="end", label="MAPPER bankrupt", refs="19"),
    dict(lane="Multi-beam writers", year=2025, kind="alive", label="IMS mask writers", refs="21, 23"),
    dict(lane="Nanoimprint", year=1995, kind="first", label="Chou, sub-25 nm", refs="29, 30"),
    dict(lane="Nanoimprint", year=1999, kind="note", label="SFIL", refs="31"),
    dict(lane="Nanoimprint", year=2014, kind="note", label="Canon + MII", refs="32"),
    dict(lane="Nanoimprint", year=2017, kind="note", label="NAND-fab tool", refs="45"),
    dict(lane="Nanoimprint", year=2023, kind="note", label="launch; volume not confirmed", refs="33, 34"),
    dict(lane="Nanoimprint", year=2025, kind="alive", label="", refs="33, 34"),
    dict(lane="Directed self-assembly", year=2001, kind="first", label="graphoepitaxy", refs="36"),
    dict(lane="Directed self-assembly", year=2003, kind="note", label="chemo-epitaxy", refs="37"),
    dict(lane="Directed self-assembly", year=2018, kind="end", label="shelved", refs="38, 39"),
    dict(lane="Directed self-assembly", year=2023, kind="alive", label="returning (EUV)", refs="39"),
    dict(lane="Interference / Talbot", year=1996, kind="first", label="achromatic IL", refs="41"),
    dict(lane="Interference / Talbot", year=2011, kind="note", label="DTL", refs="43"),
    dict(lane="Interference / Talbot", year=2025, kind="alive", label="research use", refs="43"),
]
PARALLEL_LANES = ["Proximity X-ray", "LIGA", "Electron-beam writing", "Electron projection", "Ion beams",
                  "Multi-beam writers", "Nanoimprint", "Directed self-assembly", "Interference / Talbot"]


def _lane_colour(s: K.Svg, i: int) -> str:
    names = ["blue", "aqua", "violet", "violet", "orange", "orange", "yellow", "magenta", "red"]
    return s.c(names[i % len(names)])


def draw_parallel_lanes(theme: str, *, narrow: bool = False) -> str:
    W, H = (600, 360) if narrow else (960, 820)
    s = K.Svg(W, H, theme, "Parallel paths beside projection", PARALLEL_ALT, generator=GENERATOR)
    y_lo, y_hi = 1960, 2026
    if narrow:
        x0, x1 = 30, 584
        lane_y0, lane_h = 44, 33
        y_axis = 342
    else:
        x0, x1 = 236, 896
        lane_y0, lane_h = 50, 78
        y_axis = 774

    def x(year: float) -> float:
        return x0 + (year - y_lo) / (y_hi - y_lo) * (x1 - x0)

    s.eyebrow(12, 20, "Paths beside projection, 1960–2025", size=12.5)
    for yr in range(1960, 2030, 10):
        s.line(x(yr), lane_y0 - 8, x(yr), y_axis, "grid", 0.8)
        s.line(x(yr), y_axis, x(yr), y_axis + 5, "axis", 1.0)
        s.text(x(yr), y_axis + 18, str(yr), size=12.5, color="ink2")
    s.line(x0, y_axis, x1, y_axis, "axis", 1.2)
    boxes: list[tuple[float, float, float, float]] = []  # every placed label, across lanes
    if not narrow:  # the lane lines themselves, so no label is pushed onto a neighbouring lane
        for i, lane in enumerate(PARALLEL_LANES):
            y = lane_y0 + i * lane_h + lane_h / 2
            x_first = x(min(e["year"] for e in EVENTS if e["lane"] == lane))
            boxes.append((x_first - 8, y - 3, x1 + 8, y + 3))
    for i, lane in enumerate(PARALLEL_LANES):
        y = lane_y0 + i * lane_h + (lane_h / 2 if not narrow else 19)
        col = _lane_colour(s, i)
        evs = sorted((e for e in EVENTS if e["lane"] == lane), key=lambda e: e["year"])
        first_x = x(evs[0]["year"])
        alive = [e for e in evs if e["kind"] == "alive"]
        last = x(max(e["year"] for e in evs))
        s.line(first_x, y, last, y, col, 2.2, opacity=0.55)
        if alive:
            s.line(last, y, x1 + 4, y, col, 2.2, opacity=0.55, arrow="end")
        if narrow:
            s.text(12, y - 9, lane, size=12.5, anchor="start", color="ink2", weight=600, halo=True)
        else:
            s.text(x0 - 16, y + 4.5, lane, size=13, anchor="end", color="ink", weight=600)
        for e in evs:
            ex = x(e["year"])
            kind = e["kind"]
            if kind == "first":
                s.circle(ex, y, 5.5, fill=col, stroke="surface", sw=1.4)
            elif kind == "note":
                s.circle(ex, y, 3.2, fill=col, stroke="surface", sw=1.0)
            elif kind == "end":
                s.circle(ex, y, 5.5, fill="surface", stroke=col, sw=1.8, dash="3 2" if e.get("dashed") else None)
                s.line(ex - 2.5, y - 2.5, ex + 2.5, y + 2.5, col, 1.4)
                s.line(ex - 2.5, y + 2.5, ex + 2.5, y - 2.5, col, 1.4)
            else:
                s.circle(ex, y, 3.2, fill=col, stroke="surface", sw=1.0)
            if narrow:
                if kind == "end":
                    s.text(ex, y + 14, str(e["year"]), size=12.5, color="ink2", halo=True)
                continue
            if not e["label"]:  # an unlabelled still-in-use arrow: the previous marker carries the reference
                continue
            text = (f"{e['year']} " if kind != "alive" else "") + f"{e['label']} [{e['refs']}]"
            w = K.text_width(text, 12.5) * 0.92
            dys = {0: -9, 1: 17, 2: -24, 3: 32, 4: -39, 5: 47, 6: -54, 7: 62, 8: -69, 9: 77}
            best = None
            for slot, dy in dys.items():
                for side in ("start", "end"):
                    xa = ex + 7 if side == "start" else ex - 7 - w
                    xb = xa + w
                    if xa < x0 - 190 or xb > W - 4:
                        continue
                    ya, yb = y + dy - 11, y + dy + 3
                    if all(xb < a - 6 or xa > b + 6 or yb < c - 2 or ya > d + 2 for a, c, b, d in boxes):
                        best = (slot, side, xa, xb, dy)
                        break
                if best:
                    break
            if best is None:
                print(f"warning: parallel-lanes: no free label slot for {text!r}")
            slot, side, xa, xb, dy = best or (5, "end", ex - 7 - w, ex - 7, dys[5])
            boxes.append((xa, y + dy - 11, xb, y + dy + 3))
            if slot >= 2:
                up = dy < 0
                s.line(ex, y + (-7 if up else 7), ex, y + dy + (3 if up else -10), "muted", 0.7)
            s.text(xa if side == "start" else xb, y + dy, text, size=12.5, anchor=side, color="ink2" if kind != "end" else "ink",
                   halo=True)
    if not narrow:
        lx, ly = x0, H - 8
        s.circle(lx + 6, ly - 4, 5, fill="ink2", stroke="surface", sw=1.2)
        s.text(lx + 16, ly, "first report", size=12.5, anchor="start", color="ink2")
        s.circle(lx + 104, ly - 4, 3, fill="ink2", stroke="surface", sw=1)
        s.text(lx + 112, ly, "milestone", size=12.5, anchor="start", color="ink2")
        s.circle(lx + 190, ly - 4, 5, fill="surface", stroke="ink2", sw=1.6)
        s.text(lx + 200, ly, "abandoned or shelved (decision cited)", size=12.5, anchor="start", color="ink2")
        s.line(lx + 440, ly - 4, lx + 462, ly - 4, "ink2", 2, arrow="end")
        s.text(lx + 470, ly, "still in use", size=12.5, anchor="start", color="ink2")
        s.text(lx + 560, ly, "[n] = reference on this page; dashed ring = date not cited exactly", size=12.5,
               anchor="start", color="muted")
    return s.render()


# ---------------------------------------------------------------------------------------
# Generators (one per registered name) and the registry
# ---------------------------------------------------------------------------------------


def _mpl_pair(fn, *args, **kw) -> list[Path]:
    paths = []
    for scheme in registry.THEMES:
        theme = mpl_theme(scheme)
        style(theme)
        paths.append(fn(*args, theme, scheme, **kw))
    return paths


def gen_resolution_story() -> list[Path]:
    return _mpl_pair(fig_resolution_story, hd.load_tools())


def gen_na() -> list[Path]:
    return _mpl_pair(fig_na, hd.load_tools())


def gen_na_to_2000() -> list[Path]:
    return _mpl_pair(fig_na, hd.load_tools(), through=2000)


def gen_k1() -> list[Path]:
    return _mpl_pair(fig_k1, hd.load_tools())


def gen_lanes() -> list[Path]:
    return _mpl_pair(fig_lanes, hd.load_tools())


def gen_proximity() -> list[Path]:
    return _mpl_pair(fig_proximity)


def gen_psm() -> list[Path]:
    return _mpl_pair(fig_psm)


def gen_era_ribbon() -> list[Path]:
    return K.write_pair("era-ribbon", draw_era_ribbon, OUT, narrow=lambda t: draw_era_ribbon(t, narrow=True))


def gen_parallel_lanes() -> list[Path]:
    return K.write_pair("parallel-lanes", draw_parallel_lanes, OUT, narrow=lambda t: draw_parallel_lanes(t, narrow=True))


GENERATORS = {
    "resolution-story": gen_resolution_story,
    "na-vs-year": gen_na,
    "na-vs-year-to-2000": gen_na_to_2000,
    "k1-vs-year": gen_k1,
    "tool-lanes": gen_lanes,
    "proximity-gap": gen_proximity,
    "psm-principle": gen_psm,
    "era-ribbon": gen_era_ribbon,
    "parallel-lanes": gen_parallel_lanes,
}

_MODULE = "make_history_figures.py"
_DATA_TABLE = {"page": "history/tool-data.md", "marker": hd.TABLE_MARKER}


def _reg(**kw) -> registry.AssetSpec:
    base = dict(group="history", subdir="history", fmt="svg")
    base.update(kw)
    return registry.register(registry.AssetSpec(**base), module=_MODULE)


_reg(name="resolution-story", kind="figure", alt=RESOLUTION_ALT, caption=RESOLUTION_CAPTION,
     pages=("history/index.md",), generator=gen_resolution_story,
     extra={"max_kb": 200, "max_kb_reason": "86-tool four-panel chart, outlined text"})
_reg(name="na-vs-year", kind="figure", alt=NA_ALT, caption=NA_CAPTION,
     pages=("history/index.md", "nodes/90-to-28nm.md"), generator=gen_na)
_reg(name="na-vs-year-to-2000", kind="figure", alt=NA2000_ALT, caption=NA2000_CAPTION,
     pages=("history/projection-era.md",), generator=gen_na_to_2000)
_reg(name="k1-vs-year", kind="figure", alt=K1_ALT, caption=K1_CAPTION,
     pages=("history/excimer-duv.md",), generator=gen_k1)
_reg(name="tool-lanes", kind="figure", alt=LANES_ALT, caption=LANES_CAPTION,
     pages=("history/index.md",), generator=gen_lanes)
_reg(name="proximity-gap", kind="figure", alt=PROXIMITY_ALT, caption=PROXIMITY_CAPTION,
     pages=("history/origins.md",), generator=gen_proximity)
_reg(name="psm-principle", kind="figure", alt=PSM_ALT, caption=PSM_CAPTION,
     pages=("history/excimer-duv.md",), generator=gen_psm)
_reg(name="era-ribbon", kind="strip", narrow=True, alt=RIBBON_ALT, caption=RIBBON_CAPTION,
     pages=("history/index.md",), generator=gen_era_ribbon, platform_independent=True)
_reg(name="parallel-lanes", kind="strip", narrow=True, alt=PARALLEL_ALT, caption=PARALLEL_CAPTION,
     pages=("history/parallel-paths.md",), hero_on=("history/parallel-paths.md",), generator=gen_parallel_lanes,
     platform_independent=True)


# ---------------------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------------------


def main(argv: list[str] | None = None) -> int:
    logging.getLogger("matplotlib.font_manager").setLevel(logging.ERROR)
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--table", action="store_true", help="print the tool table (column-0 GFM) with its source list")
    ap.add_argument("--write", action="store_true",
                    help="with --table: replace the block between the tool-table markers in docs/history/tool-data.md "
                         "(integrator / nightly only)")
    ap.add_argument("--only", nargs="+", metavar="NAME", help="render only these figures")
    ap.add_argument("--list", action="store_true", help="list the registered figures")
    args = ap.parse_args(argv)
    if args.table:
        text = hd.render_table(hd.load_tools())
        if args.write:
            hd.write_table(text)
        else:
            sys.stdout.write(text)
        return 0
    if args.list:
        for name in GENERATORS:
            print(name)
        return 0
    import matplotlib  # noqa: PLC0415

    matplotlib.use("Agg")
    names = args.only or list(GENERATORS)
    for name in names:
        if name not in GENERATORS:
            print(f"unknown figure {name!r}; one of {', '.join(GENERATORS)}")
            return 2
        for p in GENERATORS[name]():
            print(f"wrote {registry.rel_to_repo(p)} ({K.size_kb(p):.0f} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
