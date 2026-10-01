#!/usr/bin/env python3
"""Data charts for the "Process Nodes" site section.

Writes light and dark SVG variants into ``docs/assets/images/nodes/``:

* ``pitch-scaling-{light,dark}.svg``   -- contacted gate pitch (CPP) and minimum
  metal pitch (MMP) of leading-edge logic vs. year of volume production, with the
  node *label* for comparison. Ranges span the values reported for different
  companies/sources (see ``PITCH_DATA``).
* ``k1-vs-pitch-{light,dark}.svg``      -- k1 = (pitch/2)·NA/λ needed to print a
  line/space pitch in ONE exposure on ArF immersion (NA 1.35), EUV (NA 0.33) and
  High-NA EUV (NA 0.55); the k1 = 0.25 floor marks where multipatterning becomes
  unavoidable.
* ``photons-vs-dose-{light,dark}.svg``  -- incident photons on a (20 nm)² area vs.
  dose at 193 nm and 13.5 nm, and the resulting 3σ Poisson dose noise.
* ``irds-roadmap-{light,dark}.svg``     -- IRDS More Moore gate/metal pitch targets,
  2022 edition vs 2024 edition.

Every number is either arithmetic from physical constants (CODATA: h, c, e) or a
value from a public source cited next to it. Run with the project venv:

    /path/to/.venv/bin/python docs/figures/nodes/make_node_figures.py
"""

from __future__ import annotations

import math
import re
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "assets" / "images" / "nodes"

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
    # label, wavelength nm, NA, vendor half-pitch resolution spec (nm)
    ("ArF immersion, NA 1.35", 193.0, 1.35, 38.0),
    ("EUV, NA 0.33", 13.5, 0.33, 13.0),
    ("High-NA EUV, NA 0.55", 13.5, 0.55, 8.0),
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
PITCH_DATA: list[
    tuple[str, float, float | None, float | None, float | None, float | None, str]
] = [
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
IRDS_2022 = [
    (2022, 48, 24),
    (2025, 45, 20),
    (2028, 42, 16),
    (2031, 40, 16),
    (2034, 38, 16),
    (2037, 38, 16),
]
IRDS_2024 = [
    (2024, 48, 24),
    (2025, 48, 22),
    (2027, 48, 22),
    (2029, 46, 20),
    (2031, 44, 18),
    (2033, 44, 16),
    (2035, 42, 14),
    (2037, 42, 14),
    (2039, 42, 14),
]


# --- style -----------------------------------------------------------------
PALETTE = {
    # dataviz reference palette, slots 1-3 (validated all-pairs for these
    # surfaces with the palette validator: light #ffffff, dark #1e2029)
    "light": {
        "bg": "#ffffff",
        "s1": "#2a78d6",
        "s2": "#eb6834",
        "s3": "#1baf7a",
        "ink": "#0b0b0b",
        "ink2": "#52514e",
        "muted": "#898781",
        "grid": "#e1e0d9",
        "axis": "#c3c2b7",
        "band": "#f0efec",
    },
    "dark": {
        "bg": "#1e2029",
        "s1": "#3987e5",
        "s2": "#d95926",
        "s3": "#199e70",
        "ink": "#ffffff",
        "ink2": "#c3c2b7",
        "muted": "#9a988f",
        "grid": "#2f313a",
        "axis": "#4a4c55",
        "band": "#2a2c35",
    },
}

CSS_FONT = "system-ui, -apple-system, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif"


def setup(theme: str):
    p = PALETTE[theme]
    plt.rcParams.update(
        {
            "svg.fonttype": "none",
            "svg.hashsalt": "highuvlith-nodes",  # stable element ids -> reproducible files
            "font.family": ["sans-serif"],
            "font.sans-serif": ["Helvetica", "Arial", "DejaVu Sans"],
            "font.size": 12,
            "axes.edgecolor": p["axis"],
            "axes.linewidth": 1.0,
            "axes.labelcolor": p["ink2"],
            "axes.titlecolor": p["ink"],
            "xtick.color": p["ink2"],
            "ytick.color": p["ink2"],
            "xtick.labelcolor": p["ink2"],
            "ytick.labelcolor": p["ink2"],
            "grid.color": p["grid"],
            "grid.linewidth": 1.0,
            "grid.linestyle": "-",
            "legend.frameon": False,
            "legend.labelcolor": p["ink2"],
            "figure.facecolor": "none",
            "axes.facecolor": "none",
            "savefig.facecolor": "none",
            "savefig.transparent": True,
        }
    )
    return p


def finish(ax, p):
    ax.grid(True, which="major", axis="y")
    ax.set_axisbelow(True)
    for side in ("top", "right"):
        ax.spines[side].set_visible(False)
    ax.tick_params(length=0, pad=6)


def save(fig, name: str, theme: str, title: str):
    path = OUT / f"{name}-{theme}.svg"
    fig.savefig(
        path,
        format="svg",
        bbox_inches="tight",
        pad_inches=0.12,
        metadata={"Title": title, "Date": None},
    )
    plt.close(fig)
    svg = path.read_text(encoding="utf-8")
    # use a proper CSS font stack (matplotlib quotes every family name)
    svg = re.sub(r"font-family: [^;\"]*", f"font-family: {CSS_FONT}", svg)
    path.write_text(svg, encoding="utf-8")
    print("wrote", path.relative_to(OUT.parents[2]))


# ---------------------------------------------------------------------------
# (a) pitch scaling
# ---------------------------------------------------------------------------
def chart_pitch_scaling(theme: str):
    p = setup(theme)
    fig, ax = plt.subplots(figsize=(8.6, 4.9))
    series = (
        ("Contacted gate pitch (CPP)", p["s1"], -0.16, 2, 3),
        ("Minimum metal pitch (MMP)", p["s2"], 0.16, 4, 5),
    )
    for lab, col, off, lo_i, hi_i in series:
        hvm_x, hvm_mid = [], []
        for r in PITCH_DATA:
            lo, hi = r[lo_i], r[hi_i]
            if lo is None or hi is None:
                continue
            x, mid, roadmap = r[1] + off, math.sqrt(lo * hi), r[6] == "roadmap"
            if hi > lo:
                ax.plot(
                    [x, x],
                    [lo, hi],
                    color=col,
                    lw=2.0,
                    solid_capstyle="round",
                    zorder=3,
                )
            ax.plot(
                [x],
                [mid],
                marker="o",
                ms=8,
                ls="none",
                zorder=4,
                mfc=p["bg"] if roadmap else col,
                mec=col if roadmap else p["bg"],
                mew=2.0,
            )
            if r[0].startswith(
                "Intel"
            ):  # company-specific point: label it, keep it off the trend line
                if lo_i == 4:
                    ax.annotate(
                        r[0],
                        (x, mid),
                        xytext=(6, 4),
                        textcoords="offset points",
                        fontsize=8.5,
                        color=p["ink2"],
                    )
            elif not roadmap:
                hvm_x.append(x)
                hvm_mid.append(mid)
        order = np.argsort(hvm_x)
        ax.plot(
            np.array(hvm_x)[order],
            np.array(hvm_mid)[order],
            color=col,
            lw=1.2,
            alpha=0.45,
            zorder=2,
        )
        ax.plot(
            [], [], color=col, marker="o", ms=8, mec=p["bg"], mew=2.0, lw=2.0, label=lab
        )
    lx = [n[1] for n in NODE_LABELS]
    ly = [n[2] for n in NODE_LABELS]
    ax.plot(lx, ly, color=p["muted"], lw=1.4, ls=(0, (2, 2)), zorder=2)
    for name, x, y, status in NODE_LABELS:
        hollow = status != "hvm"
        ax.plot(
            [x],
            [y],
            marker="s",
            ms=6,
            ls="none",
            zorder=3,
            mfc=p["bg"] if hollow else p["muted"],
            mec=p["muted"] if hollow else p["bg"],
            mew=1.5,
        )
        ax.annotate(
            name,
            (x, y),
            xytext=(0, -9),
            textcoords="offset points",
            ha="center",
            va="top",
            fontsize=9,
            color=p["ink2"],
        )
    ax.plot(
        [],
        [],
        color=p["muted"],
        lw=1.4,
        ls=(0, (2, 2)),
        marker="s",
        ms=6,
        mfc=p["muted"],
        mec=p["bg"],
        label="Node name (the \u201cnm\u201d label)",
    )
    ax.set_yscale("log")
    ax.set_ylim(0.8, 300)
    ax.set_yticks([1, 2, 5, 10, 20, 50, 100, 200])
    ax.set_yticklabels(["1", "2", "5", "10", "20", "50", "100", "200"])
    ax.tick_params(which="minor", length=0)
    ax.set_xlim(2003, 2034)
    ax.set_xlabel("Year of volume production (hollow = IRDS roadmap or announced plan)")
    ax.set_ylabel("nm (log scale)")
    finish(ax, p)
    ax.legend(loc="upper right", fontsize=10.5, handlelength=2.2)
    save(fig, "pitch-scaling", theme, "Gate and metal pitch vs node name")


def chart_irds(theme: str):
    p = setup(theme)
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(8.8, 3.8), sharex=True)
    for data, col, lab in (
        (IRDS_2022, p["s1"], "IRDS 2022 edition"),
        (IRDS_2024, p["s2"], "IRDS 2024 edition"),
    ):
        yrs = [d[0] for d in data]
        for ax, idx in ((ax1, 1), (ax2, 2)):
            ax.plot(
                yrs,
                [d[idx] for d in data],
                color=col,
                lw=2.0,
                marker="o",
                ms=7,
                mec=p["bg"],
                mew=2.0,
                label=lab,
            )
    ax1.set_ylabel("Contacted gate pitch (nm)")
    ax2.set_ylabel("Tightest metal pitch (nm)")
    ax1.set_ylim(34, 50)
    ax2.set_ylim(10, 26)
    for ax in (ax1, ax2):
        ax.set_xlim(2021, 2040)
        ax.set_xlabel("Roadmap year")
        finish(ax, p)
    ax2.legend(loc="upper right", fontsize=10)
    fig.tight_layout(w_pad=3.0)
    save(fig, "irds-roadmap", theme, "IRDS ground rules, 2022 vs 2024 edition")


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


def chart_k1(theme: str):
    p = setup(theme)
    fig, ax = plt.subplots(figsize=(8.6, 4.8))
    pitch = np.linspace(4.0, 100.0, 600)
    cols = [p["s2"], p["s1"], p["s3"]]
    ax.axhspan(0.0, 0.25, color=p["band"], zorder=0, lw=0)
    ax.axhline(0.25, color=p["muted"], lw=1.2, zorder=1)
    ax.text(
        99,
        0.012,
        "k\u2081 < 0.25: a dense grating cannot be imaged in one exposure",
        ha="right",
        va="bottom",
        fontsize=10,
        color=p["ink2"],
    )
    label_y = (0.33, 0.72, 0.72)  # where each direct label sits along its line
    for (label, lam, na, res_hp), col, ly in zip(TOOLS, cols, label_y):
        k1 = (pitch / 2.0) * na / lam
        ax.plot(pitch, k1, color=col, lw=2.0, solid_capstyle="round", zorder=3)
        # vendor resolution spec (half-pitch) as a marker on the line
        pk, kk = 2 * res_hp, res_hp * na / lam
        ax.plot([pk], [kk], marker="o", ms=8, color=col, mec=p["bg"], mew=2.0, zorder=4)
        left = (
            na > 0.5 and lam < 20
        )  # High-NA: label left of its dot, clear of the NA 0.33 line
        ax.annotate(
            f"{res_hp:g} nm HP",
            (pk, kk),
            xytext=(-9, 3) if left else (9, -5),
            textcoords="offset points",
            fontsize=9.5,
            color=p["ink2"],
            va="bottom" if left else "top",
            ha="right" if left else "left",
        )
        ax.annotate(
            label,
            (ly * 2 * lam / na, ly),
            xytext=(-9, 0),
            textcoords="offset points",
            ha="right",
            va="center",
            fontsize=10.5,
            color=p["ink"],
        )
        ax.plot([], [], color=col, lw=2.0, label=label)
    for name, pp in NODE_PITCH_MARKS:
        ax.axvline(pp, color=p["grid"], lw=1.0, zorder=0)
        ax.text(
            pp,
            0.805,
            name,
            rotation=90,
            ha="center",
            va="bottom",
            fontsize=8.5,
            color=p["muted"],
        )
    ax.set_xlim(4, 100)
    ax.set_ylim(0, 0.8)
    ax.set_xlabel("Line/space pitch to print (nm)")
    ax.set_ylabel("Required k₁ = (pitch/2)·NA/λ")
    finish(ax, p)
    ax.legend(
        loc="upper left",
        bbox_to_anchor=(0.0, -0.14),
        ncol=3,
        fontsize=10.5,
        handlelength=2.0,
    )
    save(fig, "k1-vs-pitch", theme, "k1 needed for single exposure vs pitch")


# ---------------------------------------------------------------------------
# (c) photons per (20 nm)^2
# ---------------------------------------------------------------------------
def chart_photons(theme: str):
    p = setup(theme)
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(8.8, 4.0))
    dose = np.linspace(10, 100, 400)
    area = 20.0 * 20.0
    series = [("EUV 13.5 nm", 13.5, p["s1"]), ("ArF 193 nm", 193.0, p["s2"])]
    for label, lam, col in series:
        n = dose * photons_per_nm2_per_mj_cm2(lam) * area
        ax1.plot(dose, n, color=col, lw=2.0, label=label)
        ax2.plot(dose, 300.0 / np.sqrt(n), color=col, lw=2.0, label=label)
        ax1.annotate(
            label,
            (100, n[-1]),
            xytext=(-4, 7),
            textcoords="offset points",
            ha="right",
            fontsize=10.5,
            color=p["ink"],
        )
    # vendor throughput-spec doses on the EUV curve (ASML product pages)
    for d, tag in ((30.0, "NXE spec dose"), (50.0, "EXE spec dose")):
        n = d * photons_per_nm2_per_mj_cm2(13.5) * area
        for ax, y in ((ax1, n), (ax2, 300.0 / math.sqrt(n))):
            ax.plot(
                [d],
                [y],
                marker="o",
                ms=8,
                color=p["s1"],
                mec=p["bg"],
                mew=2.0,
                zorder=4,
            )
        ax1.annotate(
            f"{tag}\n{d:g} mJ/cm²: {n:,.0f}",
            (d, n),
            xytext=(6, -8),
            textcoords="offset points",
            fontsize=9,
            color=p["ink2"],
            va="top",
        )
        ax2.annotate(
            f"{300.0 / math.sqrt(n):.1f} %",
            (d, 300.0 / math.sqrt(n)),
            xytext=(6, 4),
            textcoords="offset points",
            fontsize=9,
            color=p["ink2"],
        )
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
        finish(ax, p)
    ax2.legend(loc="upper right", fontsize=10.5)
    fig.tight_layout(w_pad=3.0)
    save(fig, "photons-vs-dose", theme, "Photon shot noise at ArF and EUV doses")


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for theme in ("light", "dark"):
        chart_pitch_scaling(theme)
        chart_k1(theme)
        chart_photons(theme)
        chart_irds(theme)


if __name__ == "__main__":
    main()
