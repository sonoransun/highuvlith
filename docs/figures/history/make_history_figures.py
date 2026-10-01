#!/usr/bin/env python3
"""Data charts for the History section of the highuvlith site (docs/history/).

Run from the repository root (numpy + matplotlib):

    python docs/figures/history/make_history_figures.py          # write the SVGs
    python docs/figures/history/make_history_figures.py --table  # print the data table

Every chart is written twice into docs/assets/images/history/: ``<name>-light.svg`` and
``<name>-dark.svg`` (transparent background, ink for Material's light and dark schemes).

Data
----
The exposure-tool data points live in ``litho_tools.csv`` next to this script: one row
per tool generation with year, vendor, model, wavelength, numerical aperture (NA) and the
vendor-stated resolution. The ``sources`` column lists keys into ``SOURCES`` below; every
value in a row was read from at least one of those sources, and a blank field means no
source was found (the chart then skips that point). ``k1`` is derived here as
resolution x NA / wavelength. Vendors define "resolution" in different ways (usually the
smallest dense line/space half-pitch the tool is specified for), so k1 is an indicator,
not a measured process value.

Two charts are models, not data: ``proximity-gap`` plots the first-Fresnel-zone blur
sqrt(lambda * g), and ``psm-principle`` is a 1D coherent scalar image of two openings.
"""

from __future__ import annotations

import argparse
import csv
from dataclasses import dataclass
from pathlib import Path

import logging

import matplotlib

matplotlib.use("Agg")
logging.getLogger("matplotlib.font_manager").setLevel(logging.ERROR)
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402
from matplotlib.lines import Line2D  # noqa: E402

HERE = Path(__file__).resolve().parent
OUT = HERE.parents[1] / "assets" / "images" / "history"
DATA = HERE / "litho_tools.csv"

# Source keys used in litho_tools.csv -> (short description, URL).
SOURCES: dict[str, tuple[str, str]] = {
    "KATO": ("A. Kato, 'Chronology of Lithography Milestones', v0.9, May 2007",
             "https://www.lithoguru.com/scientist/litho_history/Kato_Litho_History.pdf"),
    "MACK": ("C. A. Mack, 'Milestones in Optical Lithography Tool Suppliers', 2005",
             "https://lithoguru.com/scientist/litho_history/milestones_tools.pdf"),
    "BRUNING2007": ("J. H. Bruning, 'Optical lithography ... 40 years and holding', Proc. SPIE 6520, "
                    "652004 (2007)", "https://doi.org/10.1117/12.720631"),
    "SHMJ": ("Semiconductor History Museum of Japan, stepper exhibit",
             "https://www.shmj.or.jp/museum2010/exhibi2446.html"),
    "NIKON_HIST": ("Nikon, history of semiconductor lithography systems",
                   "https://www.nikon.com/business/semi/history/"),
    "NIKON_ARCH": ("Nikon, lithography system archive (NA, resolution per model)",
                   "https://www.nikon.com/business/semi/lineup/archives/"),
    "NIKON_LINEUP": ("Nikon, current lithography line-up",
                     "https://www.nikon.com/business/semi/lineup/"),
    "CANON_IP_HIST": ("Canon, history of industrial equipment (IP history pages)",
                      "https://global.canon/en/intellectual-property/history/industrial.html"),
    "CANON_EX6": ("Canon, FPA-3030EX6 product page",
                  "https://global.canon/en/product/indtech/semicon/fpa3030ex6.html"),
    "CANON_EX6_NEWS": ("Photonics Online, 'Canon introduces the first 0.65-NA DUV stepper'",
                       "https://www.photonicsonline.com/doc/canon-introduces-the-first-065-na-duv-stepper-0001"),
    "CANON_ES6A": ("Canon, FPA-6300ES6a product page",
                   "https://global.canon/en/product/indtech/semicon/fpa6300es6a.html"),
    "CANON_ES6A_NEWS": ("Canon news release, 5 Apr 2012",
                        "https://global.canon/en/news/2012/apr05e.html"),
    "CANON_IZ2": ("Canon, FPA-5550iZ2 product page",
                  "https://global.canon/en/product/indtech/semicon/fpa5550iz2.html"),
    "CANON_IZ2_NEWS": ("Canon news release, 11 Dec 2017",
                       "https://global.canon/en/news/2017/20171211.html"),
    "ASML_HIST": ("ASML, company history", "https://www.asml.com/en/company/about-asml/history"),
    "ASML_TWINSCAN20": ("ASML, 'TWINSCAN: 20 years of lithography innovation' (2021)",
                        "https://www.asml.com/en/company/stories/2021/twinscan-20-years-innovation"),
    "ASML_IMM_STORY": ("ASML, 'How immersion lithography saved Moore's Law' (2023)",
                       "https://www.asml.com/en/company/stories/2023/how-immersion-lithography-saved-moores-law"),
    "ASML_EUV_STORY": ("ASML, 'Making EUV: from lab to fab' (2022)",
                       "https://www.asml.com/en/company/stories/2022/making-euv-lab-to-fab"),
    "ASML_20F_2004": ("ASML Form 20-F, fiscal 2004",
                      "https://www.sec.gov/Archives/edgar/data/937966/000115697305000129/u48286e20vf.htm"),
    "ASML_20F_2006": ("ASML Form 20-F, fiscal 2006",
                      "https://www.sec.gov/Archives/edgar/data/937966/000095012307000854/u51076e20vf.htm"),
    "ASML_20F_2010": ("ASML Form 20-F, fiscal 2010",
                      "https://www.sec.gov/Archives/edgar/data/937966/000095012311013996/u09689e20vf.htm"),
    "ASML_20F_2013": ("ASML Form 20-F, fiscal 2013",
                      "https://www.sec.gov/Archives/edgar/data/937966/000119312514046822/d546896d20f.htm"),
    "ASML_20F_2015": ("ASML Form 20-F, fiscal 2015",
                      "https://www.sec.gov/Archives/edgar/data/937966/000093796616000017/a20-fasml2015.htm"),
    "ASML_20F_2017": ("ASML Form 20-F / integrated report, fiscal 2017",
                      "https://www.sec.gov/Archives/edgar/data/937966/000093796618000007/a2017integratedreportbased.htm"),
    "ASML_Q2_2021": ("ASML Q2 2021 results press release",
                     "https://www.sec.gov/Archives/edgar/data/937966/000093796621000016/pressreleasequarterlyresul.htm"),
    "ASML_Q4_2021": ("ASML Q4 2021 results press release",
                     "https://www.sec.gov/Archives/edgar/data/937966/000093796622000004/pressreleasequarterlyresul.htm"),
    "ASML_Q4_2023": ("ASML Q4 2023 results press release",
                     "https://www.sec.gov/Archives/edgar/data/937966/000093796624000003/pressreleasequarterlyresul.htm"),
    "ASML_Q1_2024": ("ASML Q1 2024 investor presentation",
                     "https://www.sec.gov/Archives/edgar/data/937966/000093796624000013/presentationinvestorrela.htm"),
    "ASML_Q2_2025": ("ASML Q2 2025 results press release",
                     "https://www.sec.gov/Archives/edgar/data/937966/000162828025034992/pressreleasequarterlyresul.htm"),
    "ASML_1980DI": ("ASML, TWINSCAN NXT:1980Di product page",
                    "https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt1980di"),
    "ASML_2100I": ("ASML, TWINSCAN NXT:2100i product page",
                   "https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt2100i"),
    "ASML_1470": ("ASML, TWINSCAN NXT:1470 product page",
                  "https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt1470"),
    "ASML_860N": ("ASML, TWINSCAN XT:860N product page",
                  "https://www.asml.com/en/products/duv-lithography-systems/twinscan-xt860n"),
    "ASML_XT260": ("ASML, TWINSCAN XT:260 product page",
                   "https://www.asml.com/en/products/duv-lithography-systems/twinscan-xt-260"),
    "ASML_3400B": ("ASML, TWINSCAN NXE:3400B product page",
                   "https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe3400b"),
    "ASML_3400C": ("ASML, TWINSCAN NXE:3400C product page",
                   "https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe3400c"),
    "ASML_3600D": ("ASML, TWINSCAN NXE:3600D product page",
                   "https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe-3600d"),
    "ASML_3800E": ("ASML, TWINSCAN NXE:3800E product page",
                   "https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe-3800e"),
    "ASML_EXE5000": ("ASML, TWINSCAN EXE:5000 product page",
                     "https://www.asml.com/en/products/euv-lithography-systems/twinscan-exe-5000"),
    "ASML_EXE5200B": ("ASML, TWINSCAN EXE:5200B product page",
                      "https://www.asml.com/en/products/euv-lithography-systems/twinscan-exe-5200b"),
    "ASML_5500_900": ("Semiconductor Online, 'PAS 5500/900 193 nm Step and Scan System' (ASML product "
                      "description: NA 0.45 to <0.6, linewidths 150-130 nm)",
                      "https://www.semiconductoronline.com/doc/pas-5500900-193-nm-step-and-scan-system-0001"),
}

# Chart palette: the site's validated categorical slots (dataviz reference palette,
# checked with validate_palette.js against #ffffff and #201e29, all pairs). Marker shape
# is a second, colour-independent encoding of the same class.
THEMES = {
    "light": {
        "ink": "#1d1c1a", "ink2": "#52514e", "muted": "#6b6a66", "grid": "#e1e0d9",
        "axis": "#c3c2b7", "surface": "#ffffff",
        "series": ("#2a78d6", "#eb6834", "#1baf7a"),
    },
    "dark": {
        "ink": "#f1f0ee", "ink2": "#c3c2b7", "muted": "#a3a29b", "grid": "#34323d",
        "axis": "#4a4852", "surface": "#201e29",
        "series": ("#3987e5", "#d95926", "#199e70"),
    },
}

CLASSES = {  # optics class -> (series slot, marker, legend label)
    "dry": (0, "o", "Dry projection (air)"),
    "immersion": (1, "s", "ArF water immersion"),
    "euv": (2, "D", "EUV (mirrors, vacuum)"),
}
# Immersion scanners entered volume production in 2005-2006 (Nikon NSR-S609B, ASML
# XT:1700i). Dry tools introduced from 2007 on are newer models of older exposure
# technologies; the charts draw them as open markers so the leading edge stays readable.
LATER_DRY_FROM = 2007
LATER_DRY_LABEL = "Dry, introduced 2007 or later"


@dataclass
class Tool:
    year: int
    vendor: str
    model: str
    wavelength_nm: float | None
    na: float | None
    resolution_nm: float | None
    optics: str
    year_note: str
    sources: list[str]

    @property
    def later_dry(self) -> bool:
        return self.optics == "dry" and self.year >= LATER_DRY_FROM

    @property
    def k1(self) -> float | None:
        if None in (self.wavelength_nm, self.na, self.resolution_nm):
            return None
        return self.resolution_nm * self.na / self.wavelength_nm


def load_tools() -> list[Tool]:
    def num(s: str) -> float | None:
        return float(s) if s.strip() else None

    tools = []
    with DATA.open(newline="", encoding="utf-8") as fh:
        for r in csv.DictReader(fh):
            keys = [k.strip() for k in r["sources"].split(";") if k.strip()]
            unknown = [k for k in keys if k not in SOURCES]
            if unknown or not keys:
                raise ValueError(f"{r['model']}: unknown or missing sources {unknown}")
            if r["optics"] not in CLASSES:
                raise ValueError(f"{r['model']}: unknown optics class {r['optics']}")
            tools.append(Tool(int(r["year"]), r["vendor"], r["model"], num(r["wavelength_nm"]),
                              num(r["na"]), num(r["resolution_nm"]), r["optics"],
                              r["year_note"], keys))
    return sorted(tools, key=lambda t: (t.year, t.model))


# ---------------------------------------------------------------------------------------
# Styling helpers
# ---------------------------------------------------------------------------------------


def style(theme: dict) -> None:
    plt.rcParams.update({
        "font.family": ["Helvetica Neue", "Helvetica", "Arial", "DejaVu Sans"],
        "font.size": 10.5,
        "svg.fonttype": "path",
        "svg.hashsalt": "highuvlith-history",
        "axes.edgecolor": theme["axis"],
        "axes.labelcolor": theme["ink2"],
        "axes.titlecolor": theme["ink"],
        "axes.titlesize": 12,
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


def scatter_class(ax, theme, pts, cls, size=8.0, zorder=3, hollow=False):
    slot, marker, _ = CLASSES[cls]
    if not pts:
        return
    xs, ys = zip(*pts)
    color = theme["series"][slot]
    if hollow:
        ax.plot(xs, ys, linestyle="none", marker=marker, markersize=size - 1,
                markerfacecolor="none", markeredgecolor=color, markeredgewidth=1.6,
                zorder=zorder)
    else:
        ax.plot(xs, ys, linestyle="none", marker=marker, markersize=size,
                markerfacecolor=color, markeredgecolor=theme["surface"],
                markeredgewidth=1.4, zorder=zorder)


def plot_tools(ax, theme, tools, value, size=8.0):
    """Scatter one value per tool, grouped by optics class; later dry tools hollow."""
    groups: dict[tuple[str, bool], list] = {}
    for t in tools:
        v = value(t)
        if v is not None:
            groups.setdefault((t.optics, t.later_dry), []).append((t.year, v))
    for (cls, hollow), pts in sorted(groups.items(), key=lambda kv: kv[0][1], reverse=True):
        scatter_class(ax, theme, pts, cls, size=size, hollow=hollow)


def class_legend(ax, theme, later_dry=True, top=True, **kw):
    """Legend for the optics classes; by default one row just above the axes."""
    handles = []
    for cls in CLASSES:
        slot, marker, label = CLASSES[cls]
        handles.append(Line2D([], [], linestyle="none", marker=marker, markersize=8,
                              markerfacecolor=theme["series"][slot],
                              markeredgecolor=theme["surface"], markeredgewidth=1.4,
                              label=label))
        if cls == "dry" and later_dry:
            handles.append(Line2D([], [], linestyle="none", marker=marker, markersize=7,
                                  markerfacecolor="none",
                                  markeredgecolor=theme["series"][slot],
                                  markeredgewidth=1.6, label=LATER_DRY_LABEL))
    if top:
        kw = {"loc": "lower left", "bbox_to_anchor": (0.0, 1.01), "ncols": len(handles),
              "fontsize": 9, "handletextpad": 0.3, "columnspacing": 1.2,
              "borderaxespad": 0.0, **kw}
    return ax.legend(handles=handles, **kw)


def save(fig, name: str, scheme: str) -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    path = OUT / f"{name}-{scheme}.svg"
    fig.savefig(path, format="svg", bbox_inches="tight", pad_inches=0.08,
                metadata={"Date": None, "Creator": "docs/figures/history/make_history_figures.py"})
    plt.close(fig)
    print(f"wrote {path.relative_to(HERE.parents[2])}")


def year_axis(ax, lo=1971, hi=2027):
    ax.set_xlim(lo, hi)
    ax.set_xticks(range(1975, 2026, 5))


# ---------------------------------------------------------------------------------------
# Charts
# ---------------------------------------------------------------------------------------


def annotate(ax, theme, text, xy, xytext, ha="left"):
    ax.annotate(text, xy=xy, xytext=xytext, textcoords="data", fontsize=9, ha=ha,
                va="center", color=theme["ink2"],
                arrowprops=dict(arrowstyle="-", color=theme["muted"], lw=0.8,
                                shrinkA=2, shrinkB=5))


def fig_resolution_story(tools, theme, scheme):
    fig, axes = plt.subplots(4, 1, figsize=(8, 10.6), sharex=True,
                             gridspec_kw={"hspace": 0.36})
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
    ax_na.text(1972, 1.035, "NA = 1: limit for a dry lens", color=ink2, fontsize=9,
               va="bottom")
    ax_na.text(1972, 1.475, "water at 193 nm (n ≈ 1.44)", color=ink2, fontsize=9,
               va="bottom")
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
    ax_k.text(1972, 0.235, "k₁ = 0.25: single-exposure limit for dense lines", color=ink2,
              fontsize=9, va="top")
    ax_k.set_ylabel("k₁ = R · NA / λ")
    ax_k.set_title("4 · Smaller process factor k₁ (derived from 1–3)", pad=8)
    year_axis(ax_k)
    ax_k.set_xlabel("year of introduction or first shipment")
    for ax in axes:
        ax.tick_params(axis="x", labelbottom=True)
    fig.align_ylabels(axes)
    save(fig, "resolution-story", scheme)


def fig_na(tools, theme, scheme):
    fig, ax = plt.subplots(figsize=(8, 4.8))
    muted, ink2 = theme["muted"], theme["ink2"]
    plot_tools(ax, theme, tools, lambda t: t.na, size=8.5)
    ref = dict(color=muted, lw=1.1, ls=(0, (4, 3)), zorder=2)
    ax.axhline(1.0, **ref)
    ax.axhline(1.44, **ref)
    ax.text(2026.5, 1.035, "NA = 1: dry-lens limit", color=ink2, fontsize=9, ha="right",
            va="bottom")
    ax.text(2026.5, 1.475, "water at 193 nm, n ≈ 1.44", color=ink2, fontsize=9, ha="right",
            va="bottom")
    annotate(ax, theme, "Micralign 1:1 scanner, 0.167", (1973, 0.167), (1975.5, 0.07))
    annotate(ax, theme, "GCA DSW 4800 stepper, 0.28", (1978, 0.28), (1981, 0.19))
    annotate(ax, theme, "Nikon NSR-S201A scanner, 0.60", (1995, 0.60), (1993.5, 0.86), "right")
    annotate(ax, theme, "Nikon NSR-S609B: first NA > 1 (1.07)", (2005, 1.07), (2003.2, 1.13),
             "right")
    annotate(ax, theme, "ASML XT:1900i: 1.35", (2007, 1.35), (2003.2, 1.31), "right")
    annotate(ax, theme, "EUV NXE:3300B: 0.33", (2013, 0.33), (2015.3, 0.19))
    annotate(ax, theme, "High-NA EUV EXE:5000: 0.55", (2023, 0.55), (2005.5, 0.49))
    ax.set_ylim(0, 1.6)
    year_axis(ax)
    ax.set_ylabel("numerical aperture NA")
    ax.set_xlabel("year of introduction or first shipment")
    class_legend(ax, theme)
    save(fig, "na-vs-year", scheme)


def fig_k1(tools, theme, scheme):
    fig, ax = plt.subplots(figsize=(8, 4.6))
    muted, ink2 = theme["muted"], theme["ink2"]
    plot_tools(ax, theme, tools, lambda t: t.k1, size=8.5)
    ax.axhline(0.25, color=muted, lw=1.1, ls=(0, (4, 3)), zorder=2)
    ax.text(1977, 0.235, "k₁ = 0.25: single-exposure limit for dense line/space",
            color=ink2, fontsize=9, va="top")
    annotate(ax, theme, "g/i-line steppers: k₁ ≈ 0.8", (1986, 0.79), (1989.5, 0.86))
    annotate(ax, theme, "off-axis illumination, phase-shift\nmasks and OPC take k₁ below 0.5",
             (1999, 0.41), (1980.5, 0.44))
    annotate(ax, theme, "immersion at NA 1.35: k₁ ≈ 0.27", (2012, 0.266), (2005.5, 0.19))
    annotate(ax, theme, "EUV restarts near k₁ ≈ 0.5", (2013, 0.538), (2014.5, 0.69))
    ax.set_ylim(0.15, 0.95)
    year_axis(ax, 1976, 2027)
    ax.set_xticks(range(1980, 2026, 5))
    ax.set_ylabel("k₁ = R · NA / λ")
    ax.set_xlabel("year of introduction or first shipment")
    class_legend(ax, theme)
    save(fig, "k1-vs-year", scheme)


LANES = [  # (label, test)
    ("g-line  436 nm", lambda t: t.wavelength_nm == 436),
    ("i-line  365 nm", lambda t: t.wavelength_nm == 365),
    ("KrF  248 nm", lambda t: t.wavelength_nm == 248),
    ("ArF dry  193 nm", lambda t: t.wavelength_nm == 193 and t.optics == "dry"),
    ("ArF immersion  193 nm", lambda t: t.optics == "immersion"),
    ("EUV  13.5 nm, NA 0.25–0.33", lambda t: t.optics == "euv" and t.na is not None and t.na < 0.5),
    ("High-NA EUV  13.5 nm, NA 0.55", lambda t: t.optics == "euv" and t.na is not None and t.na >= 0.5),
]


def fig_lanes(tools, theme, scheme):
    fig, ax = plt.subplots(figsize=(8, 3.9))
    ink2 = theme["ink2"]
    n = len(LANES)
    for i, (label, test) in enumerate(LANES):
        y = n - 1 - i
        sel = [t for t in tools if test(t)]
        if not sel:
            continue
        cls = sel[0].optics
        years = sorted({t.year for t in sel})
        if len(years) > 1:
            ax.plot([years[0], years[-1]], [y, y], color=theme["series"][CLASSES[cls][0]],
                    lw=2, alpha=0.35, solid_capstyle="round", zorder=2)
        scatter_class(ax, theme, [(yr, y) for yr in years], cls, size=8)
        ax.text(years[0] - 0.9, y, label, ha="right", va="center", color=ink2, fontsize=9.5)
    ax.set_ylim(-0.7, n - 0.3)
    ax.set_yticks([])
    ax.spines["left"].set_visible(False)
    ax.grid(axis="y", visible=False)
    year_axis(ax, 1958, 2027)
    ax.set_xticks(range(1975, 2026, 5))
    ax.set_xlabel("year in which a tool model in the data set was introduced or first shipped")
    save(fig, "tool-lanes", scheme)


def fig_proximity(theme, scheme):
    """First-Fresnel-zone blur sqrt(lambda*g) for three wavelengths (model, not data).

    Marked points: 100 nm features at a 15 um gap with ~1 nm X-rays (H. I. Smith and
    M. L. Schattenburg, IBM J. Res. Dev. 37, 319 (1993)); the 100 um gap of highuvlith's
    LIGA preset (DeepXrayConfig::pmma_default); an illustrative 20 um g-line gap.
    """
    fig, ax = plt.subplots(figsize=(8, 4.6))
    ink2 = theme["ink2"]
    g_um = np.logspace(0, 3, 200)  # 1 um .. 1 mm
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
        (line,) = ax.plot(g_um, np.sqrt(lam_um * g_um), color=theme["series"][slot], lw=2,
                          solid_capstyle="round", label=label)
        handles.append(line)
    fig.canvas.draw()  # fix the transforms before measuring the on-screen slope
    for lam_um, label, _ in cases:
        x0, x1 = 1.3, 10.0
        p0 = ax.transData.transform((x0, np.sqrt(lam_um * x0)))
        p1 = ax.transData.transform((x1, np.sqrt(lam_um * x1)))
        angle = np.degrees(np.arctan2(p1[1] - p0[1], p1[0] - p0[0]))
        ax.text(x0, np.sqrt(lam_um * x0) * 1.35, label, color=ink2, fontsize=9.5,
                rotation=angle, rotation_mode="anchor", ha="left", va="bottom")
    marks = [  # (gap um, wavelength um, slot, text, text position)
        (20.0, 0.436, 0, "20 µm gap: ≈ 3 µm", (32.0, 1.6)),
        (15.0, 0.001, 1, "100 nm features at a 15 µm gap\n(Smith & Schattenburg, 1993)",
         (1.6, 0.36)),
        (100.0, 0.0002, 2, "100 µm gap of highuvlith's\nLIGA preset", (140.0, 0.04)),
    ]
    for g, lam_um, slot, text, (tx, ty) in marks:
        y = np.sqrt(lam_um * g)
        scatter = dict(marker="o", markersize=8.5, linestyle="none", zorder=4,
                       markerfacecolor=theme["series"][slot],
                       markeredgecolor=theme["surface"], markeredgewidth=1.4)
        ax.plot([g], [y], **scatter)
        ax.annotate(text, xy=(g, y), xytext=(tx, ty), fontsize=9, color=ink2, va="center",
                    arrowprops=dict(arrowstyle="-", color=theme["muted"], lw=0.8,
                                    shrinkA=2, shrinkB=5))
    ax.set_xticks([1, 10, 100, 1000])
    ax.set_xticklabels(["1", "10", "100", "1000"])
    ax.set_yticks([0.01, 0.1, 1, 10])
    ax.set_yticklabels(["0.01", "0.1", "1", "10"])
    ax.minorticks_off()
    ax.set_xlabel("mask–resist gap g (µm)")
    ax.set_ylabel("edge blur √(λg) (µm)")
    ax.legend(handles=handles, loc="lower left", bbox_to_anchor=(0.0, 1.01), ncols=3,
              fontsize=9, handlelength=1.6, columnspacing=1.2, borderaxespad=0.0)
    save(fig, "proximity-gap", scheme)


def fig_psm(theme, scheme):
    """Binary mask vs alternating phase-shift mask, 1D coherent scalar imaging."""
    n, L = 4096, 64.0
    x = (np.arange(n) - n / 2) * (L / n)
    f = np.fft.fftfreq(n, d=L / n)
    w, d, fc = 1.0, 2.0, 0.4  # opening width, centre spacing, pupil cutoff NA/lambda
    left = (np.abs(x + d / 2) <= w / 2).astype(float)
    right = (np.abs(x - d / 2) <= w / 2).astype(float)
    masks = {"Binary mask": left + right, "Alternating phase-shift mask": left - right}
    fig, axes = plt.subplots(3, 2, figsize=(8, 5.6), sharex=True,
                             gridspec_kw={"hspace": 0.5, "wspace": 0.22})
    c0 = theme["series"][0]
    muted, ink2 = theme["muted"], theme["ink2"]
    keep = np.abs(x) <= 3.2
    for col, (title, t_mask) in enumerate(masks.items()):
        field = np.fft.ifft(np.fft.fft(t_mask) * (np.abs(f) <= fc)).real
        inten = field**2
        rows = [(t_mask, "mask amplitude"), (field, "field at wafer"),
                (inten, "intensity at wafer")]
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
            axes[0, col].text(-1.0, 1.08, "0°", ha="center", color=ink2, fontsize=9)
            axes[0, col].text(1.0, -1.25, "180°", ha="center", color=ink2, fontsize=9,
                              va="bottom")
    axes[2, 0].annotate("openings merge", xy=(0, float(np.interp(0, x, (np.fft.ifft(
        np.fft.fft(left + right) * (np.abs(f) <= fc)).real) ** 2))), xytext=(0, 1.2),
        ha="center", fontsize=9, color=ink2,
        arrowprops=dict(arrowstyle="-", color=muted, lw=0.8))
    axes[2, 1].annotate("dark null between openings", xy=(0, 0.0), xytext=(0, 1.2),
                        ha="center", fontsize=9, color=ink2,
                        arrowprops=dict(arrowstyle="-", color=muted, lw=0.8))
    for ax in axes[2]:
        ax.set_xlabel("position (units of opening width)")
        ax.set_xlim(-3.2, 3.2)
    save(fig, "psm-principle", scheme)


def print_table(tools) -> None:
    print("| Year | Vendor | Model | λ (nm) | NA | R (nm) | k₁ | Sources |")
    print("|---:|---|---|---:|---:|---:|---:|---|")
    for t in tools:
        k1 = f"{t.k1:.2f}" if t.k1 is not None else "–"
        lam = ("13.5" if t.wavelength_nm == 13.5 else f"{t.wavelength_nm:.0f}") \
            if t.wavelength_nm else "–"
        na = f"{t.na:.3g}" if t.na is not None else "–"
        res = f"{t.resolution_nm:.0f}" if t.resolution_nm is not None else "–"
        refs = ", ".join(f"S{i}" for i in sorted(list(SOURCES).index(k) + 1 for k in t.sources))
        print(f"| {t.year} | {t.vendor} | {t.model} | {lam} | {na} | {res} | {k1} | {refs} |")
    print()
    for i, (desc, url) in enumerate(SOURCES.values(), 1):
        print(f"- **S{i}** [{desc}]({url})")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--table", action="store_true", help="print the Markdown data table")
    args = parser.parse_args()
    tools = load_tools()
    if args.table:
        print_table(tools)
        return
    for scheme, theme in THEMES.items():
        style(theme)
        fig_resolution_story(tools, theme, scheme)
        fig_na(tools, theme, scheme)
        fig_k1(tools, theme, scheme)
        fig_lanes(tools, theme, scheme)
        fig_proximity(theme, scheme)
        fig_psm(theme, scheme)


if __name__ == "__main__":
    main()
