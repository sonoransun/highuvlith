#!/usr/bin/env python3
"""Data charts for the "Process Nodes" site section.

Writes light and dark SVG variants into ``docs/assets/images/nodes/`` and registers each as
an :class:`registry.AssetSpec` so ``make_all.py --check`` verifies files, embeds and captions.

matplotlib charts (through ``sim/style.py``'s theme, text outlined; the figure machine renders
them, DESIGN §8.1):

* ``pitch-scaling``   -- contacted gate pitch (CPP) and minimum metal pitch (MMP) of leading-edge
  logic vs. year of volume production, with the node *label* for comparison and the "≈ ×8" gap
  between the name and the pitch annotated. Ranges span the values reported for different
  companies/sources (see ``PITCH_DATA``).
* ``k1-vs-pitch``     -- k1 = (pitch/2)·NA/λ needed to print a line/space pitch in ONE exposure
  on ArF immersion (NA 1.35), EUV (NA 0.33) and High-NA EUV (NA 0.55); the k1 = 0.25 floor
  marks where multipatterning becomes unavoidable.
* ``photons-vs-dose`` -- incident photons on a (20 nm)² area vs. dose at 193 nm and 13.5 nm,
  and the resulting 3σ Poisson dose noise.
* ``irds-roadmap``    -- IRDS More Moore gate/metal pitch targets, 2022 edition vs 2024 edition.

Scripted SVG (svgkit, standard library only, platform independent):

* ``node-ladder`` (N3) -- twenty years of nodes on one ruler: transistor pictograms, the
  tightest-metal-pitch range per generation, the node names and the critical-layer tool bands,
  2003–2033; ``node-ladder-<era>`` repeats it with one chapter's years highlighted.
* ``k1-heatmap`` (N14) -- the k1 each tool would need, node by node, as a matrix with a hard
  break at 0.25 (hatched: impossible in one exposure); IRDS rows marked as projections.

Every number is either arithmetic from physical constants (CODATA: h, c, e) or a value from a
public source cited next to it. Run with the project venv:

    /path/to/.venv/bin/python docs/figures/nodes/make_node_figures.py            # everything
    /path/to/.venv/bin/python docs/figures/nodes/make_node_figures.py ladder     # names containing "ladder"
    /path/to/.venv/bin/python docs/figures/nodes/make_node_figures.py --check    # heat-map cells == litho-math table
"""

from __future__ import annotations

import math
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / "sim"))

import matplotlib  # noqa: E402

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402

import registry  # noqa: E402
import style  # noqa: E402
import svgkit as K  # noqa: E402
import tokens  # noqa: E402

OUT = registry.IMAGES / "nodes"
GENERATOR = "docs/figures/nodes/make_node_figures.py"

# --- physical constants (CODATA 2018 exact / recommended values) -----------
HC_EV_NM = 1239.84193  # h·c in eV·nm
E_CHARGE = 1.602176634e-19  # J per eV


def photons_per_nm2_per_mj_cm2(wavelength_nm: float) -> float:
    """Photons per nm² carried by a dose of 1 mJ/cm² (= 10 J/m² = 1e-17 J/nm²)."""
    e_photon_j = HC_EV_NM / wavelength_nm * E_CHARGE
    return 1e-17 / e_photon_j


# --- exposure tools (ASML product pages, retrieved 2026-09-30) -------------
# https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt2000i
#   "1.35 NA 193 nm catadioptric projection lens ... production resolutions
#    down to 40 nm (C-quad) and 38 nm (dipole)"
# https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe3400c
#   13.5 nm, 0.33 NA, resolution 13 nm; throughput >= 170 wph at 20 mJ/cm²,
#   >= 135 wph at 30 mJ/cm²
# https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe-3800e
#   >= 220 wph at 30 mJ/cm²
# https://www.asml.com/en/products/euv-lithography-systems/twinscan-exe-5000
#   13.5 nm, 0.55 NA, resolution 8 nm; >= 110 wph at 50 mJ/cm²
TOOLS = [
    # label, wavelength nm, NA, vendor half-pitch resolution spec (nm), product family of the spec
    ("ArF immersion, NA 1.35", 193.0, 1.35, 38.0, "NXT:2000i"),
    ("EUV, NA 0.33", 13.5, 0.33, 13.0, "NXE:3400C"),
    ("High-NA EUV, NA 0.55", 13.5, 0.55, 8.0, "EXE:5000"),
]

# --- leading-edge pitch history ---------------------------------------------
# One row per generation: (label, representative year of volume production,
# CPP min, CPP max, MMP min, MMP max, status). None = no value we could verify.
# Ranges span the companies listed; values are rounded reported numbers, not
# measurements of ours. status: "hvm" (volume production) or "roadmap" (IRDS).
# Sources (retrieved 2026-09-30; "reported" = secondary source or teardown):
#   Intel 65 nm gate pitch 220: Bai et al., IEDM 2004, doi:10.1109/IEDM.2004.1419253.
#   Intel 45 nm gate and M1 pitch 160: Mistry et al., IEDM 2007,
#     doi:10.1109/IEDM.2007.4418914.
#   Intel 32 nm 112.5/112.5: Packan et al., IEDM 2009, doi:10.1109/IEDM.2009.5424253.
#   Intel 22 nm 90/80 (80 nm metal single-patterned): Auth et al., VLSI 2012,
#     doi:10.1109/VLSIT.2012.6242496.
#   16/14 nm: Intel 70/52 (Natarajan et al., IEDM 2014, doi:10.1109/IEDM.2014.7046976);
#     TSMC 16FF 90/64 (Chipworks on TSMC's IEDM 2014 paper, reported); Samsung 14LPE
#     78/64-67 (reported; sources differ).
#   10 nm: Intel 54/36 (IEDM 2017, doi:10.1109/IEDM.2017.8268472); Samsung 10LPE
#     64-68/48-51 and TSMC 10FF 64-66/42-44 (teardowns, reported ranges).
#   7 nm: TSMC N7 57/40, Samsung 7LPP 54/36 (reported).
#   5 nm: TSMC N5 50-51/28-30, Samsung 5LPE 57/36 (reported).
#   Intel 4 50/30: Intel, VLSI 2022.  3 nm: TSMC N3 gate pitch 45 and N3E 48/23
#     (TSMC, IEDM 2022, doi:10.1109/IEDM45625.2022.10019498).
#   Intel 18A 50/32: as reported from Intel's VLSI 2025 paper.
#   Roadmap: IEEE IRDS 2024 More Moore table MM01 (G48M22 2027, G46M20 2029,
#   G44M18 2031, G44M16 2033, G42M14 2035).
PITCH_DATA: list[tuple[str, float, float | None, float | None, float | None, float | None, str]] = [
    ("65 nm", 2006.0, 220.0, 220.0, None, None, "hvm"),
    ("45 nm", 2007.9, 160.0, 160.0, 160.0, 160.0, "hvm"),
    ("32 nm", 2010.0, 112.5, 112.5, 112.5, 112.5, "hvm"),
    ("22 nm", 2011.9, 90.0, 90.0, 80.0, 80.0, "hvm"),
    ("16/14 nm", 2014.7, 70.0, 90.0, 52.0, 67.0, "hvm"),
    ("10 nm", 2017.3, 54.0, 68.0, 36.0, 51.0, "hvm"),
    ("7 nm", 2018.6, 54.0, 57.0, 36.0, 40.0, "hvm"),
    ("5 nm", 2020.5, 50.0, 57.0, 28.0, 36.0, "hvm"),
    ("Intel 4", 2023.4, 50.0, 50.0, 30.0, 30.0, "hvm"),
    ("3 nm", 2022.9, 45.0, 48.0, 23.0, 23.0, "hvm"),
    ("Intel 18A", 2025.8, 50.0, 50.0, 32.0, 32.0, "hvm"),
    ("IRDS 2027", 2027.0, 48.0, 48.0, 22.0, 22.0, "roadmap"),
    ("IRDS 2029", 2029.0, 46.0, 46.0, 20.0, 20.0, "roadmap"),
    ("IRDS 2031", 2031.0, 44.0, 44.0, 18.0, 18.0, "roadmap"),
    ("IRDS 2033", 2033.0, 44.0, 44.0, 16.0, 16.0, "roadmap"),
]

# node label vs. approximate year of first volume production (the label is a
# name, plotted at its nominal "nm" value to show that it tracks no pitch);
# A16 and A14 are announced plans (TSMC), drawn hollow.
NODE_LABELS: list[tuple[str, float, float, str]] = [
    ("90", 2004.0, 90.0, "hvm"),
    ("65", 2006.0, 65.0, "hvm"),
    ("45", 2007.9, 45.0, "hvm"),
    ("32", 2010.0, 32.0, "hvm"),
    ("22", 2011.9, 22.0, "hvm"),
    ("14", 2014.7, 14.0, "hvm"),
    ("10", 2017.3, 10.0, "hvm"),
    ("7", 2018.6, 7.0, "hvm"),
    ("5", 2020.5, 5.0, "hvm"),
    ("3", 2022.9, 3.0, "hvm"),
    ("2", 2025.8, 2.0, "hvm"),
    ("A16", 2026.8, 1.6, "announced"),
    ("A14", 2028.0, 1.4, "announced"),
]

# IRDS More Moore ground rules (table MM01), two editions:
# (year, contacted gate pitch nm, tightest metal pitch nm)
IRDS_2022 = [(2022, 48, 24), (2025, 45, 20), (2028, 42, 16), (2031, 40, 16), (2034, 38, 16), (2037, 38, 16)]
IRDS_2024 = [(2024, 48, 24), (2025, 48, 22), (2027, 48, 22), (2029, 46, 20), (2031, 44, 18), (2033, 44, 16),
             (2035, 42, 14), (2037, 42, 14), (2039, 42, 14)]

# --- the node ladder (N3): first-volume years from nodes/index.md's generations table -----
# transistor architecture: (label, first volume-production year, status)
#   planar: the 90 nm generation (2003–2004); FinFET: Intel 22 nm, manufacturing late 2011;
#   nanosheet: Samsung 3 nm, 2022; CFET: IRDS 2024 projects about 2031 (imec about 2033).
TRANSISTORS: list[tuple[str, float, str]] = [
    ("planar", 2003.5, "hvm"),
    ("FinFET", 2011.9, "hvm"),
    ("nanosheet", 2022.5, "hvm"),
    ("CFET", 2031.0, "roadmap"),
]
# critical-layer tool bands: (label, from, to, colour key, provenance) — the generations table's tool column:
#   dry ArF from the 90 nm generation (2003); water immersion from 45 nm (2007, IBM) / 32-28 nm single exposure;
#   double patterning from TSMC 20 nm (2012–2014) through SADP/SAQP at 14 and 10 nm and N7 (2018–2019);
#   first EUV processes 2018–2019 (Samsung 7LPP, TSMC N7+); EUV double patterning on TSMC's first N3 (2022);
#   High-NA on selected Intel 18A layers, reported in 2026 (hatched: reported, selected layers).
TOOL_BANDS: list[tuple[str, float, float, str, str]] = [
    ("dry ArF", 2003.0, 2007.5, "blue", "reported"),
    ("193i, 1 exposure", 2007.5, 2012.0, "aqua", "reported"),
    ("193i multipatterning", 2012.0, 2019.5, "orange", "reported"),
    ("EUV, single exposure", 2018.5, 2033.0, "violet", "reported"),
    ("EUV double patterning", 2022.5, 2033.0, "violet", "reported"),
    ("High-NA (selected layers)", 2026.0, 2033.0, "magenta", "hatched"),
]
# the five chapters of the section and the years each covers (the era strips, N3 highlights)
ERAS: dict[str, tuple[str, int, int]] = {
    "90-to-28nm": ("nodes/90-to-28nm.md", 2003, 2014),
    "22-to-10nm": ("nodes/22-to-10nm.md", 2011, 2019),
    "7-and-5nm": ("nodes/7-and-5nm.md", 2018, 2020),
    "3nm-and-2nm": ("nodes/3nm-and-2nm.md", 2022, 2026),
    "angstrom-era": ("nodes/angstrom-era.md", 2025, 2033),
}

# --- the k1 table of nodes/litho-math.md (N14) -------------------------------------------------
# (generation, tightest metal pitch lo, hi, columns that apply (193i, EUV, High-NA), provenance)
# "—" in the page's table = the tool was not in production for that generation.
K1_ROWS: list[tuple[str, float, float, tuple[bool, bool, bool], str]] = [
    ("32 nm (Intel)", 112.5, 112.5, (True, False, False), "reported"),
    ("22 nm (Intel)", 80.0, 80.0, (True, False, False), "reported"),
    ("16/14 nm (TSMC, Samsung)", 64.0, 67.0, (True, False, False), "reported"),
    ("14 nm (Intel)", 52.0, 52.0, (True, False, False), "reported"),
    ("10 nm class", 36.0, 51.0, (True, False, False), "reported"),
    ("7 nm (TSMC N7)", 40.0, 40.0, (True, True, False), "reported"),
    ("5 nm (TSMC N5)", 28.0, 30.0, (True, True, False), "reported"),
    ("3 nm (TSMC N3E)", 23.0, 23.0, (True, True, True), "reported"),
    ('IRDS 2025 ("2 nm")', 20.0, 22.0, (True, True, True), "roadmap"),
    ("IRDS 2028–2033", 16.0, 16.0, (True, True, True), "roadmap"),
    ("IRDS 2035+ (2024 edition)", 14.0, 14.0, (True, True, True), "roadmap"),
]
K1_COLUMNS = [("193i, NA 1.35", 193.0, 1.35), ("EUV, NA 0.33", 13.5, 0.33), ("High-NA, NA 0.55", 13.5, 0.55)]
K1_FLOOR = 0.25


def k1_of(pitch_nm: float, wavelength_nm: float, na: float) -> float:
    return (pitch_nm / 2.0) * na / wavelength_nm


def k1_cell(lo: float, hi: float, wavelength_nm: float, na: float) -> tuple[float, float, str]:
    """(k1 lo, k1 hi, printed text) for a pitch range on one tool; equal rounded ends collapse."""
    a, b = k1_of(lo, wavelength_nm, na), k1_of(hi, wavelength_nm, na)
    sa, sb = f"{a:.2f}", f"{b:.2f}"
    return a, b, sa if sa == sb else f"{sa}–{sb}"


# ---------------------------------------------------------------------------
# matplotlib: shared save
# ---------------------------------------------------------------------------
def save_svg(fig, name: str, theme: style.Theme, title: str, desc: str) -> Path:
    """Write ``fig`` as an outlined-text SVG and add the ``<title>``/``<desc>`` the registry requires."""
    path = OUT / f"{name}-{theme.name}.svg"
    style.save_figure(fig, path, {"Title": title}, fmt="svg")
    text = path.read_text(encoding="utf-8")
    ins = f"\n <title>{K.esc(title)}</title>\n <desc>{K.esc(desc)}</desc>"
    text = re.sub(r"(<svg\b[^>]*>)", lambda m: m.group(1) + ins, text, count=1)
    path.write_text(text, encoding="utf-8")
    print("wrote", path.relative_to(registry.REPO))
    return path


def _finish(ax, theme: style.Theme) -> None:
    style.style_axes(ax, theme, grid="y")
    ax.tick_params(length=0, pad=6)


# ---------------------------------------------------------------------------
# (a) pitch scaling
# ---------------------------------------------------------------------------
PITCH_ALT = ("Log-scale chart against year, 2003 to 2033: contacted gate pitch falls from 220 nm (Intel 65 nm) to about "
             "45–50 nm and flattens; minimum metal pitch falls from 160 nm (2007) to 23 nm (2022), IRDS points reach "
             "16 nm by 2033; the node name falls from 90 to 1.4 (A14), about eight times below the pitch.")
PITCH_CAPTION = ("<strong>Gate and metal pitch against the node name.</strong> Before 2014 the points are Intel's "
                 "published pitches; afterwards bars span the values reported for different companies (Intel 4 and 18A "
                 "labelled separately), and hollow points are IRDS 2024 projections or announced names. The node name "
                 "sits about eight times below the metal pitch it is named after. "
                 "<em>Data chart from make_node_figures.py; no model badge.</em>")


def chart_pitch_scaling(theme: style.Theme) -> Path:
    with style.use(theme):
        fig, ax = plt.subplots(figsize=(8.6, 4.9))
        series = (("Contacted gate pitch (CPP)", theme.s(1), -0.16, 2, 3), ("Minimum metal pitch (MMP)", theme.s(2), 0.16, 4, 5))
        for lab, col, off, lo_i, hi_i in series:
            hvm_x, hvm_mid = [], []
            for r in PITCH_DATA:
                lo, hi = r[lo_i], r[hi_i]
                if lo is None or hi is None:
                    continue
                x, mid, roadmap = r[1] + off, math.sqrt(lo * hi), r[6] == "roadmap"
                if hi > lo:
                    ax.plot([x, x], [lo, hi], color=col, lw=2.0, solid_capstyle="round", zorder=3)
                ax.plot([x], [mid], marker="o", ms=8, ls="none", zorder=4, mfc=theme.surface if roadmap else col,
                        mec=col if roadmap else theme.surface, mew=2.0)
                if r[0].startswith("Intel"):  # company-specific point: label it, keep it off the trend line
                    if lo_i == 4:
                        ax.annotate(r[0], (x, mid), xytext=(6, 4), textcoords="offset points", fontsize=8.5, color=theme.ink2)
                elif not roadmap:
                    hvm_x.append(x)
                    hvm_mid.append(mid)
            order = np.argsort(hvm_x)
            ax.plot(np.array(hvm_x)[order], np.array(hvm_mid)[order], color=col, lw=1.2, alpha=0.45, zorder=2)
            ax.plot([], [], color=col, marker="o", ms=8, mec=theme.surface, mew=2.0, lw=2.0, label=lab)
        lx = [n[1] for n in NODE_LABELS]
        ly = [n[2] for n in NODE_LABELS]
        ax.plot(lx, ly, color=theme.muted, lw=1.4, ls=(0, (2, 2)), zorder=2)
        for name, x, y, status in NODE_LABELS:
            hollow = status != "hvm"
            ax.plot([x], [y], marker="s", ms=6, ls="none", zorder=3, mfc=theme.surface if hollow else theme.muted,
                    mec=theme.muted if hollow else theme.surface, mew=1.5)
            ax.annotate(name, (x, y), xytext=(0, -9), textcoords="offset points", ha="center", va="top", fontsize=9,
                        color=theme.ink2)
        ax.plot([], [], color=theme.muted, lw=1.4, ls=(0, (2, 2)), marker="s", ms=6, mfc=theme.muted, mec=theme.surface,
                label="Node name (the “nm” label)")
        # the gap between the name and the pitch it is named after (computed, never typed)
        row3 = next(r for r in PITCH_DATA if r[0] == "3 nm")
        node3 = next(n for n in NODE_LABELS if n[0] == "3")
        ratio = row3[4] / node3[2]
        xg = row3[1] + 0.16
        ax.annotate("", xy=(xg, row3[4] * 0.86), xytext=(xg, node3[2] * 1.25),
                    arrowprops=dict(arrowstyle="<->", color=theme.ink2, lw=1.0, shrinkA=0, shrinkB=0), zorder=5)
        ax.annotate(f"≈ ×{ratio:.0f}: the name\nis not a pitch", (xg, math.sqrt(row3[4] * node3[2])),
                    xytext=(10, 0), textcoords="offset points", ha="left", va="center", fontsize=9, color=theme.ink,
                    bbox=dict(boxstyle="round,pad=0.25", fc=theme.surface, ec="none", alpha=0.85))
        ax.set_yscale("log")
        ax.set_ylim(0.8, 300)
        ax.set_yticks([1, 2, 5, 10, 20, 50, 100, 200])
        ax.set_yticklabels(["1", "2", "5", "10", "20", "50", "100", "200"])
        ax.tick_params(which="minor", length=0)
        ax.set_xlim(2003, 2034)
        ax.set_xlabel("Year of volume production (hollow = IRDS roadmap or announced plan)")
        ax.set_ylabel("nm (log scale)")
        _finish(ax, theme)
        ax.legend(loc="upper right", fontsize=9.5, handlelength=2.2)
        return save_svg(fig, "pitch-scaling", theme, "Gate and metal pitch vs node name", PITCH_ALT)


IRDS_ALT = ("Two small line charts of IRDS roadmap targets against year, comparing the 2022 and 2024 editions. Gate "
            "pitch: the 2024 edition sits above the 2022 edition at every year. Metal pitch: the 2024 edition reaches "
            "16 nm in 2033 where the 2022 edition had it in 2028.")
IRDS_CAPTION = ("<strong>IRDS ground-rule targets, 2022 versus 2024 edition.</strong> The roadmap relaxed its pitches "
                "and leaned on stacking. <em>Data chart from the IRDS More Moore tables; no model badge.</em>")


def chart_irds(theme: style.Theme) -> Path:
    with style.use(theme):
        fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(8.8, 3.8), sharex=True)
        for data, col, lab in ((IRDS_2022, theme.s(1), "IRDS 2022 edition"), (IRDS_2024, theme.s(2), "IRDS 2024 edition")):
            yrs = [d[0] for d in data]
            for ax, idx in ((ax1, 1), (ax2, 2)):
                ax.plot(yrs, [d[idx] for d in data], color=col, lw=2.0, marker="o", ms=7, mec=theme.surface, mew=2.0, label=lab)
        ax1.set_ylabel("Contacted gate pitch (nm)")
        ax2.set_ylabel("Tightest metal pitch (nm)")
        ax1.set_ylim(34, 50)
        ax2.set_ylim(10, 26)
        for ax in (ax1, ax2):
            ax.set_xlim(2021, 2040)
            ax.set_xticks([2022, 2025, 2028, 2031, 2034, 2037, 2040])
            ax.set_xlabel("Roadmap year")
            _finish(ax, theme)
        ax2.legend(loc="upper right", fontsize=9.5)
        fig.tight_layout(w_pad=3.0)
        return save_svg(fig, "irds-roadmap", theme, "IRDS ground rules, 2022 vs 2024 edition", IRDS_ALT)


# ---------------------------------------------------------------------------
# (b) required k1 for single exposure
# ---------------------------------------------------------------------------
# critical pitches marked along the top (label, pitch nm); rounded reported
# minimum metal pitches -- see the litho-math page for sources.
NODE_PITCH_MARKS: list[tuple[str, float]] = [
    ("Intel 22 nm", 80.0),
    ("TSMC 16FF", 64.0),
    ("Intel 14 nm", 52.0),
    ("TSMC N7", 40.0),
    ("Intel 10 nm", 36.0),
    ("TSMC N5", 28.0),
    ("TSMC N3E", 23.0),
    ("IRDS 2033", 16.0),
]
K1_ALT = ("Required k1 against line/space pitch for ArF immersion (NA 1.35), EUV (NA 0.33) and High-NA EUV (NA 0.55); "
          "each line rises with pitch; the band below k1 = 0.25 marks pitches one exposure cannot print. Dots: vendor "
          "half-pitch specs (38, 13 and 8 nm); ticks: tightest metal pitches of named processes.")
K1_CAPTION = ("<strong>The k₁ a single exposure needs, pitch by pitch.</strong> Where a tool's line enters the shaded "
              "band below 0.25 that tool needs multipatterning; dots are the vendor half-pitch resolution specs (ASML "
              "product pages), ticks the tightest metal pitches of the processes named. "
              "<em>Closed form; not the engine. No model badge.</em>")


def chart_k1(theme: style.Theme) -> Path:
    with style.use(theme):
        fig, ax = plt.subplots(figsize=(8.6, 4.8))
        pitch = np.linspace(4.0, 100.0, 600)
        cols = [theme.s(2), theme.s(1), theme.s(3)]
        ax.axhspan(0.0, K1_FLOOR, color=theme.band, zorder=0, lw=0)
        ax.axhline(K1_FLOOR, color=theme.muted, lw=1.2, zorder=1)
        ax.text(99, 0.012, "k₁ < 0.25: a dense grating cannot be imaged in one exposure", ha="right", va="bottom",
                fontsize=9.5, color=theme.ink2)
        label_y = (0.33, 0.62, 0.58)  # where each direct label sits along its line
        for (label, lam, na, res_hp, family), col, ly in zip(TOOLS, cols, label_y):
            k1 = (pitch / 2.0) * na / lam
            ax.plot(pitch, k1, color=col, lw=2.0, solid_capstyle="round", zorder=3)
            pk, kk = 2 * res_hp, res_hp * na / lam  # vendor resolution spec (half-pitch) as a marker on the line
            ax.plot([pk], [kk], marker="o", ms=8, color=col, mec=theme.surface, mew=2.0, zorder=4)
            if na > 0.5 and lam < 20:  # High-NA: the dot sits by the axis; label it in the gap between the lines
                ax.annotate(f"{family} · {res_hp:g} nm HP", (pk, kk), xytext=(56, 0.46), textcoords="data",
                            fontsize=8.5, color=theme.ink2, va="center", ha="left",
                            arrowprops=dict(arrowstyle="-", color=theme.muted, lw=0.8, shrinkA=0, shrinkB=5))
            else:
                ax.annotate(f"{family} · {res_hp:g} nm HP", (pk, kk), xytext=(9, -5), textcoords="offset points",
                            fontsize=8.5, color=theme.ink2, va="top", ha="left")
            ax.annotate(label, (ly * 2 * lam / na, ly), xytext=(-9, 0), textcoords="offset points", ha="right",
                        va="center", fontsize=9.5, color=theme.ink, fontweight="semibold")
        # tightest metal pitches of named processes: hairlines with labels on two alternating rows
        for i, (name, pp) in enumerate(NODE_PITCH_MARKS):
            ax.axvline(pp, color=theme.grid, lw=1.0, zorder=0)
            ax.text(pp, 0.775 if i % 2 == 0 else 0.735, name, ha="center", va="center", fontsize=7.5, color=theme.muted,
                    bbox=dict(boxstyle="round,pad=0.15", fc=theme.surface, ec="none"), zorder=2)
        ax.set_xlim(4, 100)
        ax.set_ylim(0, 0.8)
        ax.set_xlabel("Line/space pitch to print (nm)")
        ax.set_ylabel("Required k₁ = (pitch/2)·NA/λ")
        _finish(ax, theme)
        return save_svg(fig, "k1-vs-pitch", theme, "k1 needed for single exposure vs pitch", K1_ALT)


# ---------------------------------------------------------------------------
# (c) photons per (20 nm)^2
# ---------------------------------------------------------------------------
PHOTONS_ALT = ("Two panels. Left: incident photons on a 20 nm by 20 nm area against dose, log scale; the ArF line sits "
               "about 14 times above the EUV line; at 30 mJ/cm² EUV gives 8,155 photons, at 50 mJ/cm² 13,592. Right: "
               "3-sigma photon-count noise, 3.3 and 2.6 percent for EUV at those doses, under 1 percent for ArF.")
PHOTONS_CAPTION = ("<strong>Photon shot noise at ArF and EUV doses.</strong> Incident photons on a (20 nm)² area from "
                   "E = hc/λ and Poisson statistics; dots mark the doses at which ASML quotes NXE and EXE throughput. "
                   "<em>Closed form; not the engine. No model badge.</em>")


def chart_photons(theme: style.Theme) -> Path:
    with style.use(theme):
        fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(8.8, 4.0))
        dose = np.linspace(10, 100, 400)
        area = 20.0 * 20.0
        series = [("EUV 13.5 nm", 13.5, theme.s(1)), ("ArF 193 nm", 193.0, theme.s(2))]
        for label, lam, col in series:
            n = dose * photons_per_nm2_per_mj_cm2(lam) * area
            ax1.plot(dose, n, color=col, lw=2.0, label=label)
            ax2.plot(dose, 300.0 / np.sqrt(n), color=col, lw=2.0, label=label)
            ax1.annotate(label, (100, n[-1]), xytext=(-4, 7), textcoords="offset points", ha="right", fontsize=9.5, color=theme.ink)
        for d, tag in ((30.0, "NXE spec dose"), (50.0, "EXE spec dose")):  # vendor throughput-spec doses (ASML)
            n = d * photons_per_nm2_per_mj_cm2(13.5) * area
            for ax, y in ((ax1, n), (ax2, 300.0 / math.sqrt(n))):
                ax.plot([d], [y], marker="o", ms=8, color=theme.s(1), mec=theme.surface, mew=2.0, zorder=4)
            ax1.annotate(f"{tag}\n{d:g} mJ/cm²: {n:,.0f}", (d, n), xytext=(6, -8), textcoords="offset points",
                         fontsize=8.5, color=theme.ink2, va="top")
            ax2.annotate(f"{300.0 / math.sqrt(n):.1f} %", (d, 300.0 / math.sqrt(n)), xytext=(6, 4),
                         textcoords="offset points", fontsize=8.5, color=theme.ink2)
        ax1.set_yscale("log")
        ax1.set_ylim(1e3, 1e6)
        ax1.set_yticks([1e3, 1e4, 1e5, 1e6])
        ax1.set_yticklabels(["1,000", "10,000", "100,000", "1,000,000"])
        ax1.tick_params(which="minor", length=0)
        ax1.set_xlabel("Dose (mJ/cm²)")
        ax1.set_ylabel("Incident photons on a (20 nm)² area")
        ax2.set_ylim(0, 6)
        ax2.set_xlabel("Dose (mJ/cm²)")
        ax2.set_ylabel("3σ photon-count noise (%)")
        for ax in (ax1, ax2):
            ax.set_xlim(10, 100)
            _finish(ax, theme)
        ax2.legend(loc="upper right", fontsize=9.5)
        fig.tight_layout(w_pad=3.0)
        return save_svg(fig, "photons-vs-dose", theme, "Photon shot noise at ArF and EUV doses", PHOTONS_ALT)


# ---------------------------------------------------------------------------
# N3 · the node ladder (svgkit)
# ---------------------------------------------------------------------------
LADDER_W, LADDER_H = 960, 500
X0, X1 = 130, 930
Y0, Y1 = 2003.0, 2033.0


def _xy(year: float) -> float:
    return X0 + (year - Y0) / (Y1 - Y0) * (X1 - X0)


def _mark(s: K.Svg, x: float, y: float, r: float, color: str, provenance: str, shape: str = "dot") -> None:
    """A data mark in the tokens.PROVENANCE vocabulary: filled (reported), hollow (announced), hollow dashed (roadmap)."""
    hollow = provenance in ("announced", "roadmap", "projection")
    dash = "3 2" if provenance in ("roadmap", "projection") else None
    if shape == "dot":
        if hollow:
            s.circle(x, y, r, fill="surface", stroke=color, sw=1.6, dash=dash)
        else:
            s.circle(x, y, r, fill=color, stroke="surface", sw=1.4)
    else:
        if hollow:
            s.rect(x - r, y - r, 2 * r, 2 * r, fill="surface", stroke=color, sw=1.4, dash=dash, rx=1.5)
        else:
            s.rect(x - r, y - r, 2 * r, 2 * r, fill=color, stroke="surface", sw=1.2, rx=1.5)


def _transistor_icon(s: K.Svg, kind: str, x: float, y: float, hollow: bool = False) -> None:
    """A 44 x 36 cross-section pictogram centred at ``x`` with its baseline at ``y``."""
    si = s.tint("blue", 0.30)
    gate = s.tint("orange", 0.45)
    hk = s.c("magenta")
    if hollow:
        si = gate = "none"
    stroke_si, stroke_g = s.c("blue"), s.c("orange")
    dash = "3 2" if hollow else None
    s.rect(x - 22, y - 8, 44, 8, fill=si, stroke=stroke_si, sw=0.8, dash=dash)  # substrate
    if kind == "planar":
        s.rect(x - 11, y - 24, 22, 15, fill=gate, stroke=stroke_g, sw=0.8, rx=1)
        s.line(x - 11, y - 9, x + 11, y - 9, hk, 1.6)
    elif kind == "FinFET":
        s.rect(x - 16, y - 30, 32, 22, fill=gate, stroke=stroke_g, sw=0.8, rx=1)
        for fx in (x - 9, x + 3):
            s.rect(fx, y - 26, 6, 18, fill=si, stroke=stroke_si, sw=0.8)
    elif kind == "nanosheet":
        s.rect(x - 16, y - 34, 32, 26, fill=gate, stroke=stroke_g, sw=0.8, rx=1)
        for sy in (y - 29, y - 21, y - 13):
            s.rect(x - 9, sy, 18, 4, fill=si, stroke=stroke_si, sw=0.7)
    else:  # CFET: two tiers in one gate
        s.rect(x - 16, y - 40, 32, 32, fill=gate, stroke=stroke_g, sw=0.8, rx=1, dash=dash)
        for sy in (y - 36, y - 30):
            s.rect(x - 9, sy, 18, 3.5, fill="none" if hollow else s.tint("violet", 0.3), stroke="violet", sw=0.7, dash=dash)
        for sy in (y - 20, y - 14):
            s.rect(x - 9, sy, 18, 3.5, fill=si, stroke=stroke_si, sw=0.7, dash=dash)


def draw_node_ladder(theme: str, highlight: tuple[int, int] | None = None) -> str:
    title = "Twenty years of nodes on one ruler"
    desc = LADDER_ALT if highlight is None else f"The node ruler with the years {highlight[0]}–{highlight[1]} highlighted and the rest dimmed."
    s = K.Svg(LADDER_W, LADDER_H, theme, title, desc, generator=GENERATOR)
    lanes = [("transistor", 34, 110), ("tightest metal pitch", 124, 258), ("node name", 272, 334), ("critical-layer tool", 348, 452)]
    for name, ya, yb in lanes:
        s.line(X0 - 6, yb + 4, X1, yb + 4, "grid", 1.0)
        s.eyebrow(14, ya + 12, name, size=K.SMALL, color="muted")
    # year grid
    for yr in range(2005, 2034, 5):
        x = _xy(yr)
        s.line(x, 34, x, 452, "grid", 0.8, dash="2 4")
    # lane 1: transistor pictograms at first-volume year
    for label, yr, prov in TRANSISTORS:
        x = _xy(yr)
        _transistor_icon(s, label, x, 84, hollow=(prov == "roadmap"))
        txt = f"{label} {int(yr)}" if label != "planar" else "planar"
        if prov == "roadmap":
            txt = f"{label} {int(yr)}+ (roadmap)"
        s.text(x, 104, txt, size=K.SMALL, color="ink2", halo=True)
    # lane 2: tightest metal pitch, log scale 12–200 nm
    la, lb = 134, 246

    def ypitch(nm: float) -> float:
        return lb - (math.log10(nm) - math.log10(12)) / (math.log10(200) - math.log10(12)) * (lb - la)

    for nm in (20, 50, 100):
        s.line(X0 - 6, ypitch(nm), X1, ypitch(nm), "grid", 0.8)
        s.text(X0 - 10, ypitch(nm) + 4, f"{nm} nm", size=K.SMALL, anchor="end", color="muted")
    col = s.c("orange")
    for label, yr, _c0, _c1, lo, hi, status in PITCH_DATA:
        if lo is None or hi is None:
            continue
        x = _xy(yr)
        prov = "roadmap" if status == "roadmap" else "reported"
        if hi > lo:
            s.line(x, ypitch(hi), x, ypitch(lo), col, 2.2, opacity=0.6)
        _mark(s, x, ypitch(math.sqrt(lo * hi)), 4.2, col, prov)
        if label.startswith("IRDS"):
            continue
        anchor, dx = ("end", -8) if label in ("3 nm", "10 nm") else ("start", 8)
        s.text(x + dx, ypitch(math.sqrt(lo * hi)) + 4, label, size=K.SMALL, anchor=anchor, color="ink2", halo=True)
    s.text(_xy(2030), ypitch(12.6), "IRDS 2024 roadmap", size=K.SMALL, color="ink2", halo=True)
    # lane 3: node names on their own log scale (100 → 1 nm) as a descending dotted line of squares
    na_, nb_ = 282, 322

    def yname(nm: float) -> float:
        return nb_ - (math.log10(nm) - 0) / 2 * (nb_ - na_)

    pts = [(_xy(yr), yname(v)) for _n, yr, v, _st in NODE_LABELS]
    s.polyline(pts, "muted", 1.2, dash="2 3")
    for name, yr, v, status in NODE_LABELS:
        x, y = _xy(yr), yname(v)
        _mark(s, x, y, 4, s.c("muted"), "announced" if status != "hvm" else "reported", shape="square")
        s.text(x, y + 15 if v > 3.5 else y - 8, name, size=K.SMALL, color="ink2", halo=True)
    s.text(X0 - 10, na_ + 8, "100", size=K.SMALL, anchor="end", color="muted")
    s.text(X0 - 10, nb_ + 4, "1", size=K.SMALL, anchor="end", color="muted")
    # lane 4: critical-layer tool bands
    rows = [366, 388, 410, 432]
    for i, (label, a, b, ck, prov) in enumerate(TOOL_BANDS):
        row = [0, 0, 0, 1, 2, 3][i]
        y = rows[row]
        xa, xb = _xy(a), _xy(b)
        if prov == "hatched":
            fill = s.hatch("hna", s.c(ck), 5, 1.0)
            s.rect(xa, y, xb - xa, 18, fill=fill, stroke=ck, sw=1.0, rx=4)
        else:
            s.rect(xa, y, xb - xa, 18, fill=s.tint(ck, 0.28), stroke=ck, sw=1.0, rx=4)
        tx = (xa + xb) / 2
        s.text(tx, y + 13, label, size=K.SMALL, color="ink", halo=True)
    # the year axis
    s.line(X0 - 6, 456, X1, 456, "axis", 1.2)
    for yr in range(2003, 2034, 2):
        x = _xy(yr)
        s.line(x, 456, x, 460 if yr % 10 else 463, "axis", 1.0)
        if yr % 5 == 0 or yr == 2033:
            s.text(x, 476, str(yr), size=K.SMALL, color="ink2")
    # legend
    lx, ly = X0 - 6, 494
    s.circle(lx + 6, ly - 4, 4.2, fill=col, stroke="surface", sw=1.2)
    s.text(lx + 16, ly, "reported pitch (rounded; bars span companies)", size=K.SMALL, anchor="start", color="ink2")
    lx += 16 + K.text_width("reported pitch (rounded; bars span companies)", K.SMALL) + 18
    s.circle(lx + 6, ly - 4, 4.2, fill="surface", stroke=col, sw=1.4, dash="3 2")
    s.text(lx + 16, ly, "IRDS projection", size=K.SMALL, anchor="start", color="ink2")
    lx += 16 + K.text_width("IRDS projection", K.SMALL) + 18
    s.rect(lx, ly - 10, 12, 12, fill="surface", stroke="muted", sw=1.2, rx=1.5)
    s.text(lx + 18, ly, "announced name", size=K.SMALL, anchor="start", color="ink2")
    lx += 18 + K.text_width("announced name", K.SMALL) + 18
    s.rect(lx, ly - 10, 14, 12, fill=s.hatch("hna2", s.c("magenta"), 4, 0.9), stroke="magenta", sw=0.9, rx=2)
    s.text(lx + 20, ly, "selected layers, reported", size=K.SMALL, anchor="start", color="ink2")
    # the era highlight: dim everything outside the span, bracket it at the top
    if highlight is not None:
        ya, yb = highlight
        xa, xb = _xy(ya), _xy(yb)
        s.rect(0, 0, xa, LADDER_H, fill="surface", stroke="none", opacity=0.62)
        s.rect(xb, 0, LADDER_W - xb, LADDER_H, fill="surface", stroke="none", opacity=0.62)
        s.rect(xa, 30, xb - xa, 428, fill="none", stroke="hue:nodes", sw=1.2, rx=4)
        s.line(xa, 22, xb, 22, "hue:nodes", 2.0)
        s.text((xa + xb) / 2, 14, f"You are here: {ya}–{yb}", size=K.LABEL, weight=600, color="ink", halo=True)
    return s.render()


LADDER_ALT = ("Four lanes against the years 2003 to 2033: transistor type (planar, FinFET 2011, nanosheet 2022, CFET "
              "projected 2031), tightest metal pitch per generation from 160 nm (2007) to 23 nm (2022) and IRDS "
              "projections to 16 nm, node names 90 to A14, and the critical-layer tool from dry ArF to High-NA EUV.")
LADDER_CAPTION = ("<strong>Twenty years of nodes on one ruler.</strong> A node name is a label, the pitch is the "
                  "dimension, and the tool follows the pitch; rounded reported values for the tightest layers, "
                  "company-specific where sources differ; hollow and hatched marks are announced or roadmap items. "
                  "Sources in make_node_figures.py. <em>Data chart; no model badge.</em>")


def gen_node_ladder() -> list[Path]:
    paths = K.write_pair("node-ladder", lambda t: draw_node_ladder(t), OUT)
    for era, (_page, ya, yb) in ERAS.items():
        paths += K.write_pair(f"node-ladder-{era}", lambda t, ya=ya, yb=yb: draw_node_ladder(t, (ya, yb)), OUT)
    for p in paths:
        print("wrote", p.relative_to(registry.REPO))
    return paths


# ---------------------------------------------------------------------------
# N14 · the k1 heat-map (svgkit)
# ---------------------------------------------------------------------------
HEAT_ALT = ("A matrix of the k1 each tool would need for the tightest metal pitch of eleven generations, 32 nm to the "
            "IRDS 2035 row, on 193i at NA 1.35, EUV at NA 0.33 and High-NA EUV at NA 0.55; cells below 0.25 are "
            "hatched (impossible in one exposure), the three IRDS rows are marked as projections.")
HEAT_CAPTION = ("<strong>The k₁ each tool would need, node by node.</strong> k₁ = (pitch/2)·NA/λ for the tightest "
                "reported metal pitch on 193i (NA 1.35), EUV (NA 0.33) and High-NA EUV (NA 0.55); hatched cells sit "
                "below the 0.25 floor, where one exposure cannot print the pitch. "
                "<em>Reported, rounded pitches; IRDS rows are projections. Data chart; no model badge.</em>")
HEAT_BINS = (0.50, 0.40, 0.33, 0.28, K1_FLOOR)  # k1 at or above the i-th value -> ramp step i (0 = lightest)


def _heat_step(k1: float) -> int:
    for i, edge in enumerate(HEAT_BINS):
        if k1 >= edge:
            return i
    return -1


def draw_k1_heatmap(theme: str) -> str:
    s = K.Svg(960, 470, theme, "The k1 each tool would need, node by node", HEAT_ALT, generator=GENERATOR)
    ramp = tokens.ORDINAL[theme]
    x_label, x_cols, cw, rh = 20, (360, 555, 750), 190, 27
    y_head = 50
    s.text(x_label, 24, "Required k₁ for the tightest metal pitch, one exposure", size=15, anchor="start", weight=600)
    s.text(x_label, y_head + 18, "Generation (tightest metal pitch)", size=K.SMALL, anchor="start", color="muted", weight=600)
    for (label, _lam, _na), x in zip(K1_COLUMNS, x_cols):
        s.text(x + cw / 2, y_head + 18, label, size=K.LABEL, weight=600, color="ink")
    y = y_head + 30
    hatch = s.hatch("floor", s.c("muted"), 5, 0.9)
    for gen, lo, hi, cols, prov in K1_ROWS:
        roadmap = prov == "roadmap"
        if roadmap:
            s.rect(x_label - 6, y, x_cols[-1] + cw - x_label + 12, rh, fill="none", stroke="muted", sw=0.9, dash="4 3", rx=3)
        ptxt = f"{lo:g} nm" if lo == hi else f"{lo:g}–{hi:g} nm"
        s.text(x_label, y + 18, f"{gen}  ·  {ptxt}", size=K.LABEL, anchor="start", color="ink2" if roadmap else "ink",
               italic=roadmap)
        for (_label, lam, na), x, applies in zip(K1_COLUMNS, x_cols, cols):
            if not applies:
                s.rect(x + 2, y + 2, cw - 4, rh - 4, fill="none", stroke="grid", sw=0.8, rx=3)
                s.text(x + cw / 2, y + 18, "—", size=K.LABEL, color="muted")
                continue
            a, b, txt = k1_cell(lo, hi, lam, na)
            if a < K1_FLOOR:
                s.rect(x + 2, y + 2, cw - 4, rh - 4, fill=hatch, stroke="axis", sw=0.8, rx=3)
                s.text(x + cw / 2, y + 18, txt, size=K.LABEL, color="ink2", halo=True)
            else:
                step = _heat_step(a)
                fill = ramp[step]
                s.rect(x + 2, y + 2, cw - 4, rh - 4, fill=fill, stroke="none", rx=3)
                s.text(x + cw / 2, y + 18, txt, size=K.LABEL, color=tokens.text_on(fill), weight=600)
        y += rh
    # legend
    ly = y + 28
    s.text(x_label, ly, "k₁ needed", size=K.SMALL, anchor="start", color="ink2")
    lx = x_label + K.text_width("k₁ needed", K.SMALL) + 12
    labels = ("≥ 0.50", "0.40", "0.33", "0.28", "0.25")
    for i, lab in enumerate(labels):
        s.rect(lx, ly - 11, 46, 14, fill=ramp[i], stroke="none", rx=2)
        s.text(lx + 23, ly, lab, size=K.SMALL, color=tokens.text_on(ramp[i]))
        lx += 48
    lx += 14
    s.rect(lx, ly - 11, 30, 14, fill=hatch, stroke="axis", sw=0.8, rx=2)
    s.text(lx + 38, ly, "below 0.25: impossible in one exposure", size=K.SMALL, anchor="start", color="ink2")
    lx += 38 + K.text_width("below 0.25: impossible in one exposure", K.SMALL) + 18
    s.rect(lx, ly - 11, 30, 14, fill="none", stroke="muted", sw=0.9, dash="4 3", rx=2)
    s.text(lx + 38, ly, "IRDS rows: projections, not announcements", size=K.SMALL, anchor="start", color="ink2")
    s.text(x_label, ly + 24, "— : the tool was not in production for that generation. "
           "Pitches are rounded reported values; sources on the litho-math page.", size=K.SMALL, anchor="start", color="muted")
    return s.render()


def gen_k1_heatmap() -> list[Path]:
    paths = K.write_pair("k1-heatmap", draw_k1_heatmap, OUT)
    for p in paths:
        print("wrote", p.relative_to(registry.REPO))
    return paths


# ---------------------------------------------------------------------------
# Registry
# ---------------------------------------------------------------------------
def _gen_chart(fn):
    def gen() -> list[Path]:
        return [fn(t) for t in style.THEMES]

    gen.__name__ = f"gen_{fn.__name__}"
    return gen


for _name, _fn, _alt, _cap, _pages, _extra in (
    ("pitch-scaling", chart_pitch_scaling, PITCH_ALT, PITCH_CAPTION, ("nodes/index.md",), {}),
    ("k1-vs-pitch", chart_k1, K1_ALT, K1_CAPTION, ("nodes/litho-math.md",), {}),
    ("photons-vs-dose", chart_photons, PHOTONS_ALT, PHOTONS_CAPTION, ("nodes/litho-math.md",), {}),
    ("irds-roadmap", chart_irds, IRDS_ALT, IRDS_CAPTION, ("nodes/angstrom-era.md",), {}),
):
    registry.register(registry.AssetSpec(
        name=_name, kind="figure", group="nodes", subdir="nodes", fmt="svg", alt=_alt, caption=_cap, pages=_pages,
        generator=_gen_chart(_fn), platform_independent=False, extra=_extra,
    ), module="make_node_figures.py")

registry.register(registry.AssetSpec(
    name="node-ladder", kind="schematic", group="nodes", subdir="nodes", fmt="svg", alt=LADDER_ALT, caption=LADDER_CAPTION,
    pages=("nodes/index.md", "README.md"), hero_on=("nodes/index.md",), generator=gen_node_ladder,
    platform_independent=True, extra={"width": {"README.md": 300}},
), module="make_node_figures.py")

for _era, (_page, _ya, _yb) in ERAS.items():
    registry.register(registry.AssetSpec(
        name=f"node-ladder-{_era}", kind="schematic", group="nodes", subdir="nodes", fmt="svg",
        alt=f"The node ruler of the section index with the years {_ya}–{_yb} highlighted and the rest dimmed: "
            "transistor type, tightest metal pitch, node name and critical-layer tool against the year.",
        caption=f"<strong>You are here: {_ya}–{_yb}.</strong> This chapter's years on the node ruler of the "
                "section index. <em>Data chart; no model badge.</em>",
        pages=(_page,), generator=gen_node_ladder, platform_independent=True,
    ), module="make_node_figures.py")

registry.register(registry.AssetSpec(
    name="k1-heatmap", kind="schematic", group="nodes", subdir="nodes", fmt="svg", alt=HEAT_ALT, caption=HEAT_CAPTION,
    pages=("nodes/litho-math.md",), generator=gen_k1_heatmap, platform_independent=True,
), module="make_node_figures.py")


# ---------------------------------------------------------------------------
# --check: the heat-map's printed cells equal the k1 table on nodes/litho-math.md
# ---------------------------------------------------------------------------
def check() -> list[str]:
    errors: list[str] = []
    page = registry.DOCS / "nodes" / "litho-math.md"
    if not page.exists():
        return [f"{page} missing"]
    rows: dict[str, list[str]] = {}
    for line in page.read_text(encoding="utf-8").splitlines():
        if not line.startswith("| ") or "|---" in line:
            continue
        cells = [c.strip().replace("**", "") for c in line.strip().strip("|").split("|")]
        if len(cells) == 6:
            rows[cells[0]] = cells[1:5]
    for gen, lo, hi, cols, _prov in K1_ROWS:
        if gen not in rows:
            errors.append(f"k1-heatmap: row {gen!r} not found in the litho-math table")
            continue
        want = rows[gen][1:4]
        for (label, lam, na), applies, w in zip(K1_COLUMNS, cols, want):
            got = k1_cell(lo, hi, lam, na)[2] if applies else "—"
            if got.replace(" ", "") != w.replace(" ", ""):
                errors.append(f"k1-heatmap: {gen!r} / {label}: figure prints {got!r}, the page table says {w!r}")
    return errors


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    if "--check" in argv:
        errs = check()
        for e in errs:
            print("error:", e)
        print(f"make_node_figures --check: {len(errs)} error(s)")
        return 1 if errs else 0
    OUT.mkdir(parents=True, exist_ok=True)
    jobs = {
        "pitch-scaling": lambda: [chart_pitch_scaling(t) for t in style.THEMES],
        "k1-vs-pitch": lambda: [chart_k1(t) for t in style.THEMES],
        "photons-vs-dose": lambda: [chart_photons(t) for t in style.THEMES],
        "irds-roadmap": lambda: [chart_irds(t) for t in style.THEMES],
        "node-ladder": gen_node_ladder,
        "k1-heatmap": gen_k1_heatmap,
    }
    for name, job in jobs.items():
        if not argv or any(a in name for a in argv):
            job()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
