#!/usr/bin/env python3
"""Data charts for "The Future" section of the highuvlith site (docs/future/).

Writes light and dark SVG variants into ``docs/assets/images/future/``; pages pick
the right one with the ``#only-light`` / ``#only-dark`` URL suffixes. Backgrounds
are transparent; ink and series colours are chosen for Material's light (white)
and dark ("slate", hsl(250 15% 14%) = #201e29) schemes. Series colours are the
site's chart palette (slots 1-3 of the dataviz reference palette), validated
all-pairs for both surfaces with the palette validator.

Charts:

* ``photons-per-feature``   -- photons that land in a (half-pitch)^2 square at a
  fixed dose, vs half-pitch, at 193 / 13.5 / 6.7 nm. Pure arithmetic from CODATA
  constants plus ASML's published resolution specs (cited in ``TOOLS``).
* ``polarization-contrast`` -- two-beam interference contrast for TE, TM and
  unpolarized light vs the sine of the beam half-angle in the recording medium.
  Closed-form optics (the same closed forms WP-A2 pins in tests/vector_pupil.rs).
* ``noon-fringes``          -- one-photon fringe, classical two-photon exposure of
  the same fringe, and an ideal N = 2 NOON fringe. Closed forms.
* ``talbot-carpet``         -- scalar paraxial Fresnel self-imaging behind a
  binary grating with 25 % open fraction (Fourier-series evaluation, no measured data).
* ``source-landscape``      -- reported / projected average power vs wavelength for
  every sourced row of ``source_power_vs_wavelength.csv`` (production / demonstrated /
  projected marked differently; EUV HVM band).
* ``source-landscape-euv``  -- the 13.5 nm rows as a horizontal dot plot by source class.
* ``multilayer-reflectance`` -- best measured normal-incidence multilayer peak
  reflectance per wavelength class, and what ~11 reflections leave.

Run with the project venv (numpy + matplotlib):

    /path/to/.venv/bin/python docs/figures/future/make_future_figures.py

Pass ``--preview DIR`` to also write PNG previews (needs ``rsvg-convert``).
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402
from matplotlib.colors import LinearSegmentedColormap  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "assets" / "images" / "future"

# --- physical constants (CODATA 2018) ---------------------------------------
HC_EV_NM = 1239.84193  # h*c in eV*nm
E_CHARGE = 1.602176634e-19  # J per eV


def photon_energy_ev(wavelength_nm: float) -> float:
    return HC_EV_NM / wavelength_nm


def photons_per_nm2_per_mj_cm2(wavelength_nm: float) -> float:
    """Photons per nm^2 carried by 1 mJ/cm^2 (= 10 J/m^2 = 1e-17 J/nm^2)."""
    return 1e-17 / (photon_energy_ev(wavelength_nm) * E_CHARGE)


# --- style ------------------------------------------------------------------
# Slots 1-3 of the dataviz reference palette (light / dark steps). Validated with
# validate_palette.js --pairs all: light on #ffffff (aqua 2.82:1 -> every chart
# carries direct labels and the page carries a data table), dark on #201e29.
PALETTE = {
    "light": {
        "s1": "#2a78d6",
        "s2": "#eb6834",
        "s3": "#1baf7a",
        "ink": "#0b0b0b",
        "ink2": "#52514e",
        "muted": "#6b6a66",
        "grid": "#e1e0d9",
        "axis": "#c3c2b7",
        "band": "#efeeea",
        "context": "#b3b2aa",  # de-emphasis gray for "emphasis" charts (values are direct-labeled)
        "seq": ["#ffffff", "#cde2fb", "#86b6ef", "#3987e5", "#256abf", "#184f95", "#0d366b"],
    },
    "dark": {
        "s1": "#3987e5",
        "s2": "#d95926",
        "s3": "#199e70",
        "ink": "#ffffff",
        "ink2": "#c3c2b7",
        "muted": "#a3a29b",
        "grid": "#34323f",
        "axis": "#4d4b58",
        "band": "#2c2a36",
        "context": "#5a5866",
        "seq": ["#201e29", "#104281", "#1c5cab", "#2a78d6", "#5598e7", "#9ec5f4", "#e6f0fd"],
    },
}


def setup(theme: str) -> dict:
    p = PALETTE[theme]
    plt.rcParams.update(
        {
            "svg.fonttype": "none",
            "svg.hashsalt": "highuvlith-future",  # stable element ids -> reproducible files
            "font.family": ["sans-serif"],
            "font.sans-serif": ["Helvetica", "Arial", "DejaVu Sans"],
            "font.size": 12,
            "axes.edgecolor": p["axis"],
            "axes.linewidth": 1.0,
            "axes.labelcolor": p["ink2"],
            "axes.titlecolor": p["ink"],
            "axes.titlesize": 13,
            "axes.titleweight": "bold",
            "axes.titlelocation": "left",
            "xtick.color": p["axis"],
            "ytick.color": p["axis"],
            "xtick.labelcolor": p["ink2"],
            "ytick.labelcolor": p["ink2"],
            "grid.color": p["grid"],
            "grid.linewidth": 1.0,
            "grid.linestyle": "-",
            "legend.frameon": False,
            "legend.labelcolor": p["ink2"],
            "lines.solid_capstyle": "round",
            "lines.solid_joinstyle": "round",
            "figure.facecolor": "none",
            "axes.facecolor": "none",
            "savefig.facecolor": "none",
            "savefig.transparent": True,
        }
    )
    return p


def style_axes(ax, p) -> None:
    ax.grid(True, which="major")
    ax.set_axisbelow(True)
    for side in ("top", "right"):
        ax.spines[side].set_visible(False)
    ax.tick_params(which="both", length=3)


def save(fig, name: str, theme: str, preview: Path | None) -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    path = OUT / f"{name}-{theme}.svg"
    fig.savefig(path, format="svg", bbox_inches="tight", metadata={"Date": None})
    plt.close(fig)
    if preview is not None and shutil.which("rsvg-convert"):
        preview.mkdir(parents=True, exist_ok=True)
        bg = "#ffffff" if theme == "light" else "#201e29"
        subprocess.run(
            ["rsvg-convert", "-b", bg, "-z", "1.5", "-o", str(preview / f"{name}-{theme}.png"), str(path)],
            check=True,
        )
    print("wrote", path.relative_to(OUT.parents[2]))


# ============================================================================
# 1. Photons per feature: the lambda^3 squeeze
# ============================================================================
DOSE_MJ_CM2 = 30.0

# (label, wavelength nm, half-pitch nm, kind, source)
# kind "spec": vendor resolution spec from the product page (retrieved 2026-09-30):
#   NXT:2000i  https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt2000i
#              ("production resolutions down to 40 nm (C-quad) and 38 nm (dipole)")
#   NXE:3400C  https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe3400c (13 nm)
#   EXE:5000   https://www.asml.com/en/products/euv-lithography-systems/twinscan-exe-5000 (8 nm)
# kind "extrap": NOT a product -- k1 = 0.32 (the k1 implied by the two EUV specs:
#   13*0.33/13.5 = 0.318, 8*0.55/13.5 = 0.326) applied to a hypothetical NA.
TOOLS = [
    ("ArF immersion\nNA 1.35 (38 nm)", 193.0, 38.0, "spec"),
    ("EUV NA 0.33\n(13 nm)", 13.5, 13.0, "spec"),
    ("High-NA 0.55\n(8 nm)", 13.5, 8.0, "spec"),
    ("Hyper-NA 0.75?\n(k₁ 0.32: 5.8 nm)", 13.5, 0.32 * 13.5 / 0.75, "extrap"),
    ("BEUV 6.7 nm, NA 0.55?\n(k₁ 0.32: 3.9 nm)", 6.7, 0.32 * 6.7 / 0.55, "extrap"),
]


def fig_photons_per_feature(theme: str, preview: Path | None) -> None:
    p = setup(theme)
    fig, ax = plt.subplots(figsize=(8.6, 5.4))
    style_axes(ax, p)
    hp = np.logspace(np.log10(2.0), np.log10(100.0), 200)
    series = [(193.0, "193 nm (ArF)", p["s1"]), (13.5, "13.5 nm (EUV)", p["s2"]), (6.7, "6.7 nm (BEUV)", p["s3"])]
    for lam, label, color in series:
        n = photons_per_nm2_per_mj_cm2(lam) * DOSE_MJ_CM2 * hp**2
        ax.plot(hp, n, color=color, lw=2, label=label, zorder=3)
        # direct label at the right end
        ax.annotate(
            label,
            xy=(hp[-1], n[-1]),
            xytext=(6, 0),
            textcoords="offset points",
            va="center",
            fontsize=11,
            color=p["ink"],
        )
    # Poisson reference: 3 sigma / mean = 10 %  <=>  N = 900
    ax.axhline(900, color=p["muted"], lw=1, zorder=2)
    ax.text(98, 900 / 1.3, "N = 900: 3σ photon noise = 10 %", fontsize=10, color=p["ink2"], ha="right", va="top")
    colors = {193.0: p["s1"], 13.5: p["s2"], 6.7: p["s3"]}
    offsets = {
        "ArF": (-10, 12, "right"),
        "EUV NA": (-12, 30, "right"),
        "High": (10, -30, "left"),
        "Hyper": (-12, 34, "right"),
        "BEUV": (10, -32, "left"),
    }
    for label, lam, h, kind in TOOLS:
        n = photons_per_nm2_per_mj_cm2(lam) * DOSE_MJ_CM2 * h**2
        c = colors[lam]
        if kind == "spec":
            ax.plot(h, n, "o", ms=9, color=c, mec=p["seq"][0], mew=2, zorder=4)
        else:
            ax.plot(h, n, "o", ms=9, mfc="none", mec=c, mew=2, zorder=4)
        key = next(k for k in offsets if label.startswith(k))
        dx, dy, ha = offsets[key]
        ax.annotate(
            f"{label}\n≈ {n:,.0f} photons",
            xy=(h, n),
            xytext=(dx, dy),
            textcoords="offset points",
            ha=ha,
            va="center",
            fontsize=9.5,
            color=p["ink2"],
            linespacing=1.15,
        )
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlim(2, 100)
    ax.set_ylim(10, 5e6)
    ax.set_xticks([2, 3, 5, 10, 20, 30, 50, 100])
    ax.set_xticklabels(["2", "3", "5", "10", "20", "30", "50", "100"])
    ax.set_xlabel("half-pitch (nm)")
    ax.set_ylabel(f"photons incident on a (half-pitch)² square\nat {DOSE_MJ_CM2:.0f} mJ/cm²")
    ax.set_title("At fixed dose and k₁, photons per feature fall as λ³", pad=12)
    ax.text(
        0.0,
        -0.2,
        "Filled: vendor resolution specs (ASML product pages). Hollow: hypothetical, k₁ = 0.32 extrapolation.\n"
        "Incident photons only; the absorbed fraction depends on the resist.",
        transform=ax.transAxes,
        fontsize=9,
        color=p["muted"],
        va="top",
    )
    save(fig, "photons-per-feature", theme, preview)


# ============================================================================
# 2. Polarization: two-beam contrast vs angle
# ============================================================================
# Two plane waves at +/- theta in the recording medium, equal amplitude:
#   TE (s, E perpendicular to the plane of incidence):  V = 1
#   TM (p, E in the plane of incidence):                 V = |cos 2 theta|
#   unpolarized (incoherent TE + TM, equal weight):      V = (1 + cos 2 theta)/2 = cos^2 theta
MARKERS = [
    (0.33, "EUV\nNA 0.33"),
    (0.55, "High-NA\n0.55"),
    (0.75, "Hyper-NA\n0.75"),
    (1.35 / 1.7, "ArF-i NA 1.35\nin resist n≈1.7"),
]


def fig_polarization_contrast(theme: str, preview: Path | None) -> None:
    p = setup(theme)
    fig, ax = plt.subplots(figsize=(8.6, 5.0))
    style_axes(ax, p)
    s = np.linspace(0.0, 0.999, 400)
    cos2t = 1.0 - 2.0 * s**2
    ax.plot(s, np.ones_like(s), color=p["s1"], lw=2, zorder=3, label="TE (s-polarized): 1")
    ax.plot(s, np.abs(cos2t), color=p["s2"], lw=2, zorder=3, label="TM (p-polarized): |cos 2\u03b8|")
    ax.plot(s, 1.0 - s**2, color=p["s3"], lw=2, zorder=3, label="unpolarized: cos\u00b2\u03b8")
    ax.axvline(1 / np.sqrt(2), color=p["axis"], lw=1, zorder=1)
    ax.text(
        1 / np.sqrt(2) - 0.008,
        0.03,
        "TM fringes reverse\nbeyond 45\u00b0",
        fontsize=9,
        color=p["muted"],
        va="bottom",
        ha="right",
    )
    align = {0.33: ("center", 0.0), 0.55: ("center", 0.0), 0.75: ("right", -0.006)}
    for x, label in MARKERS:
        ax.plot([x, x], [0, 1], color=p["muted"], lw=1, zorder=1)
        ax.plot(x, abs(1 - 2 * x**2), "o", ms=8, color=p["s2"], mec=p["seq"][0], mew=2, zorder=4)
        ax.plot(x, 1 - x**2, "o", ms=8, color=p["s3"], mec=p["seq"][0], mew=2, zorder=4)
        ha, dx = align.get(round(x, 2), ("left", 0.006))
        ax.text(x + dx, 1.05, label, ha=ha, va="bottom", fontsize=9.5, color=p["ink2"], linespacing=1.1)
    ax.legend(loc="lower left", fontsize=10.5, handlelength=1.6)
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1.3)
    ax.set_yticks([0, 0.25, 0.5, 0.75, 1.0])
    ax.set_xlabel("sin \u03b8 of the two interfering orders in the recording medium (= NA/n at the pitch limit)")
    ax.set_ylabel("fringe contrast (visibility)")
    ax.set_title("Why high NA needs polarization control: two-beam contrast", pad=14)
    save(fig, "polarization-contrast", theme, preview)


# ============================================================================
# 3. NOON fringes vs classical two-photon exposure
# ============================================================================
def fig_noon_fringes(theme: str, preview: Path | None) -> None:
    p = setup(theme)
    fig, ax = plt.subplots(figsize=(8.6, 4.6))
    style_axes(ax, p)
    x = np.linspace(0, 2, 800)  # in units of the classical fringe period
    one = 0.5 * (1 + np.cos(2 * np.pi * x))
    two_classical = one**2
    noon2 = 0.5 * (1 + np.cos(4 * np.pi * x))
    ax.plot(x, one, color=p["s1"], lw=2, zorder=3, label="classical light, one-photon resist: I")
    ax.plot(
        x,
        two_classical,
        color=p["s2"],
        lw=2,
        zorder=3,
        label="classical light, two-photon resist: I\u00b2  (= highuvlith quantum_aerial_image, N = 2, F = 1)",
    )
    ax.plot(x, noon2, color=p["s3"], lw=2, zorder=4, label="ideal N = 2 NOON state: 1 + cos 2\u03c6  (period halved)")
    ax.legend(loc="upper left", fontsize=10, handlelength=1.6, borderaxespad=0.2)
    ax.set_xlim(0, 2)
    ax.set_ylim(0, 1.55)
    ax.set_yticks([0, 0.5, 1.0])
    ax.set_xticks([0, 0.5, 1, 1.5, 2])
    ax.set_xlabel("position (units of the classical fringe period \u039b = \u03bb / 2 sin \u03b8)")
    ax.set_ylabel("normalized exposure")
    ax.set_title("Sharper is not finer: I\u00b2 narrows lines, only NOON halves the period", pad=12)
    save(fig, "noon-fringes", theme, preview)


# ============================================================================
# 4. Talbot carpet
# ============================================================================
def fig_talbot_carpet(theme: str, preview: Path | None) -> None:
    """Paraxial Fresnel self-imaging of a binary amplitude grating with 25 % open fraction.

    Field behind the grating: E(x, z) = sum_m c_m exp(i 2 pi m x / p) exp(-i pi lambda z m^2 / p^2),
    with c_0 = d, c_m = sin(pi m d) / (pi m), open fraction d = 0.25. In the scaled coordinates
    x/p and z/z_T (z_T = 2 p^2 / lambda) the result is independent of lambda and p.
    The open fraction matters at z_T/4: there the field is c_0 + (t(x) - c_0) * exp(-i pi m^2 / 2)
    summed over orders, i.e. two half-period-shifted copies of the grating; with d = 0.5 they
    add to a UNIFORM intensity, with d = 0.25 they do not overlap and give period p/2.
    """
    p = setup(theme)
    nx, nz, mmax = 360, 480, 60  # about the displayed resolution
    xs = np.linspace(-1.0, 1.0, nx)  # x / p
    zs = np.linspace(0.0, 1.0, nz)  # z / z_T
    m = np.arange(-mmax, mmax + 1)
    duty = 0.25
    c = np.where(m == 0, duty, np.sin(np.pi * m * duty) / (np.pi * np.where(m == 0, 1, m)))
    # exp(-i pi lambda z m^2 / p^2) with z = zs * 2 p^2 / lambda -> exp(-i 2 pi zs m^2)
    phase_x = np.exp(1j * 2 * np.pi * np.outer(m, xs))  # (M, nx)
    phase_z = np.exp(-1j * 2 * np.pi * np.outer(zs, m**2))  # (nz, M)
    field = (phase_z * c) @ phase_x  # (nz, nx)
    inten = np.abs(field) ** 2
    inten = np.clip(inten / np.percentile(inten, 99.5), 0, 1)
    cmap = LinearSegmentedColormap.from_list("seq", p["seq"])
    fig, ax = plt.subplots(figsize=(8.6, 4.6))
    ax.imshow(
        inten.T,
        origin="lower",
        extent=(0, 1, -1, 1),
        aspect="auto",
        cmap=cmap,
        interpolation="bilinear",
        rasterized=True,
    )
    for side in ("top", "right"):
        ax.spines[side].set_visible(False)
    for zf, label in [(0.25, r"$z_\mathrm{T}/4$: period halved"), (0.5, r"$z_\mathrm{T}/2$: shifted self-image"), (1.0, r"$z_\mathrm{T}$: self-image")]:
        ax.axvline(zf, color=p["ink2"], lw=1)
        ax.text(zf - 0.005, 1.04, label, ha="right" if zf == 1.0 else "center", va="bottom", fontsize=10, color=p["ink2"])
    ax.set_xlabel(r"distance behind the mask, $z\,/\,z_\mathrm{T}$   (Talbot length $z_\mathrm{T} = 2p^2/\lambda$)")
    ax.set_ylabel("x / p")
    ax.set_yticks([-1, -0.5, 0, 0.5, 1])
    ax.set_xticks([0, 0.25, 0.5, 0.75, 1.0])
    ax.set_title("Talbot carpet: a grating re-images itself without a lens", pad=22)
    save(fig, "talbot-carpet", theme, preview)


# ============================================================================
# 5. Source landscape: reported / projected average power vs wavelength
# ============================================================================
# Every point is a row of source_power_vs_wavelength.csv (next to this script). Rows 1-49
# were compiled and checked by a research pass (each number seen in the cited source;
# E_photon = 1239.84/lambda eV used for photon-rate conversions, noted per row); rows 50-51
# (Socol et al. 2011 ERL-FEL study; xLight's SPIE 2026 public design) were added from the
# references of docs/future/accelerator-light-sources.md. "Plane" matters: EUV numbers are
# in-band power at intermediate focus (IF) unless the row says "AT THE SOURCE" (discharge
# plasmas, into 2 pi sr, several times more than reaches IF); DUV numbers are laser output.
DATA_CSV = Path(__file__).resolve().parent / "source_power_vs_wavelength.csv"

HVM_BAND_W = (250.0, 1000.0)  # 13.5 nm at IF: 250 W (NXE:3400B, 2017) -> ~1 kW (ASML, ~2030)


def _status(raw: str) -> str:
    raw = raw.upper()
    if raw.startswith("PRODUCTION"):
        return "production"
    if raw.startswith("DEMONSTRATED"):
        return "demonstrated"
    return "projected"


def _load_sources() -> list[dict]:
    import csv

    rows = []
    with DATA_CSV.open(newline="") as f:
        for r in csv.DictReader(f):
            lo, hi = float(r["power_w_low"]), float(r["power_w_high"])
            rows.append(
                {
                    "id": int(r["id"]),
                    "cls": r["class"],
                    "name": r["machine_or_paper"],
                    "lam": float(r["wavelength_nm"]),
                    "lo": lo,
                    "hi": hi,
                    "mid": float(np.sqrt(lo * hi)),
                    "status": _status(r["status"]),
                    "at_source": "AT THE SOURCE" in r["plane_or_definition"].upper(),
                }
            )
    return rows


def _group_135(r: dict) -> str | None:
    """Source class for the 13.5 nm chart (rows at 13.2-13.8 nm only)."""
    if not 13.2 <= r["lam"] <= 13.8:
        return None
    c = r["cls"]
    if "LPP" in c:
        return GROUPS_135[0]
    if "DPP" in c or "LDP" in c:
        return GROUPS_135[1]
    if "FEL" in c:
        return GROUPS_135[2]
    if "SSMB" in c:
        return GROUPS_135[3]
    if "Synchrotron" in c:
        return GROUPS_135[4]
    if "HHG" in c:
        return GROUPS_135[5]
    return None


GROUPS_135 = [
    "tin laser-produced\nplasma",
    "discharge plasma",
    "linac / ERL\nfree-electron laser",
    "steady-state\nmicrobunching ring",
    "synchrotron\nundulator beamline",
    "high-harmonic\ngeneration",
]
SUBROW = {"production": 0.28, "demonstrated": 0.0, "projected": -0.28}

# direct labels on the 13.5 nm chart: row id -> (text, dx pt, dy pt, ha)
LABELS_135 = {
    8: ("production: 250 W (2017) … 600 W (2026)", -12, 0, "right"),
    17: ("demonstrated: 125 W … 1 kW (2025)", -12, 0, "right"),
    15: ("roadmap\n1.5–2 kW", 10, 0, "left"),
    24: ("production: Xe metrology lamps 10–40 W\ninto 2π; ~20 W at IF on NXE:3100 (2011)", -14, 0, "right"),
    21: ("demonstrated into 2π\nat the source:\n200 W … 1 kW", 12, 0, "left"),
    32: ("FLASH: 20 mW at 13.7 nm (2007)", 10, 0, "left"),
    37: ("designs: 1.7 kW (DESY), ~5 kW (Socol), 9–22 kW\n(KEK), 38 kW for 16 scanners (xLight)", -12, 0, "right"),
    40: ("designs: 1 kW (Tsinghua target),\n1.1 kW (SLAC 2016), 4 kW (2015 example)", -12, 0, "right"),
    31: ("PSI XIL-II beamline: 11 mW", 10, 0, "left"),
    46: ("0.4–1 µW (2017, 2012)", 10, 0, "left"),
}

# group labels on the full-spectrum chart: (text, x nm, y W, ha)
LABELS_WIDE = [
    ("ArF lasers\n60–120 W", 193.0, 900.0, "center"),
    ("KrF lasers\n30–60 W", 248.0, 6.0, "center"),
    ("Gd LPP concept, 1 kW", 6.35, 1000.0, "right"),
    ("KEK FEL design", 6.35, 1.8e4, "right"),
    ("ICS designs", 6.35, 1.2e-4, "right"),
    ("13.5 nm: tin plasma, discharge\nplasma and accelerator designs\n(next chart)", 15.0, 6e4, "left"),
    ("EUV HVM need at IF:\n250 W → ~1 kW", 17.5, 500.0, "left"),
    ("synchrotron beamline", 12.8, 1.1e-2, "right"),
    ("FLASH (13.7 nm)", 14.4, 2.8e-2, "left"),
    ("FERMI (15 nm)", 16.0, 3.9e-4, "left"),
    ("Ni-like Mo laser\n(18.9 nm)", 20.0, 5.0e-5, "left"),
    ("HHG at 13.5 nm", 12.8, 7.0e-7, "right"),
    ("FLASH (18.2 nm)", 19.5, 0.35, "left"),
    ("LPA-FEL\n(SIOM, 27 nm)", 29.0, 1.5e-7, "left"),
    ("HHG, mW class\nat 21–30 eV", 60.0, 3.0e-3, "left"),
    ("soft-X-ray lasers\n(46.9 nm)", 52.0, 3.0e-5, "left"),
]


def _marker(ax, x, y, r, p, color):
    marker = "^" if r["at_source"] else {"production": "o", "demonstrated": "D", "projected": "s"}[r["status"]]
    ms = 7 if marker == "D" else 8
    if r["status"] == "projected":
        ax.plot(x, y, marker, ms=ms, mfc="none", mec=color, mew=2, zorder=4)
    else:
        ax.plot(x, y, marker, ms=ms, color=color, mec=p["seq"][0], mew=1.5, zorder=4)


def _legend_handles(p):
    from matplotlib.lines import Line2D

    return [
        Line2D([], [], ls="none", marker="o", ms=8, color=p["s1"], label="in production"),
        Line2D([], [], ls="none", marker="D", ms=7, color=p["s2"], label="demonstrated"),
        Line2D([], [], ls="none", marker="s", ms=8, mfc="none", mec=p["s3"], mew=2, label="projected / design study"),
        Line2D([], [], ls="none", marker="^", ms=8, color=p["ink2"], label="into 2π at the source (not at IF)"),
        Line2D([], [], color=p["ink2"], lw=1.6, label="reported range"),
    ]


def fig_source_landscape(theme: str, preview: Path | None) -> None:
    """All sourced rows: average power vs wavelength (log-log)."""
    p = setup(theme)
    rows = _load_sources()
    color = {"production": p["s1"], "demonstrated": p["s2"], "projected": p["s3"]}
    fig, ax = plt.subplots(figsize=(8.6, 6.6))
    style_axes(ax, p)
    ax.set_yscale("log")
    ax.set_xscale("log")
    ax.fill_between([11.0, 16.5], *HVM_BAND_W, color=p["band"], zorder=1, lw=0)
    for r in rows:
        c = color[r["status"]]
        if r["hi"] > r["lo"] * 1.01:
            ax.plot([r["lam"], r["lam"]], [r["lo"], r["hi"]], color=c, lw=1.6, zorder=3, solid_capstyle="butt")
        _marker(ax, r["lam"], r["mid"], r, p, c)
    for text, x, y, ha in LABELS_WIDE:
        ax.text(x, y, text, ha=ha, va="center", fontsize=10, color=p["ink2"], linespacing=1.1)
    ax.set_xlim(3.2, 330)
    ax.set_ylim(5e-8, 3e5)
    ax.set_xticks([5, 6.7, 10, 13.5, 20, 30, 50, 100, 193, 248])
    ax.set_xticklabels(["5", "6.7", "10", "13.5", "20", "30", "50", "100", "193", "248"])
    ax.minorticks_off()
    ax.set_xlabel("wavelength (nm)")
    ax.set_ylabel("average power (W)")
    ax.legend(handles=_legend_handles(p), loc="upper left", bbox_to_anchor=(0.0, 1.13), ncol=3, fontsize=10,
              handletextpad=0.4, columnspacing=1.2, frameon=False)
    ax.set_title("Reported and projected average power\nof lithography-relevant light sources", pad=52)
    fig.text(0.125, 0.0, "EUV: in-band power at intermediate focus (IF) unless marked; DUV: laser output.\n"
             "Every point is a sourced row of docs/figures/future/source_power_vs_wavelength.csv.",
             fontsize=9, color=p["muted"], va="top")
    save(fig, "source-landscape", theme, preview)


def fig_source_landscape_euv(theme: str, preview: Path | None) -> None:
    """The 13.5 nm rows as a horizontal dot plot by source class."""
    p = setup(theme)
    rows = _load_sources()
    color = {"production": p["s1"], "demonstrated": p["s2"], "projected": p["s3"]}
    fig, ax = plt.subplots(figsize=(8.6, 7.4))
    style_axes(ax, p)
    ax.set_xscale("log")
    ax.grid(False, axis="y")
    n = len(GROUPS_135)
    ax.axvspan(*HVM_BAND_W, color=p["band"], zorder=1, lw=0)
    ax.text(np.sqrt(HVM_BAND_W[0] * HVM_BAND_W[1]), n - 0.35, "HVM need at IF\n250 W → ~1 kW",
            ha="center", va="bottom", fontsize=9.5, color=p["ink"], linespacing=1.1)
    for r in rows:
        g = _group_135(r)
        if g is None:
            continue
        y = (n - 1 - GROUPS_135.index(g)) + SUBROW[r["status"]]
        c = color[r["status"]]
        if r["hi"] > r["lo"] * 1.01:
            ax.plot([r["lo"], r["hi"]], [y, y], color=c, lw=1.6, zorder=3, solid_capstyle="butt")
        _marker(ax, r["mid"], y, r, p, c)
        if r["id"] in LABELS_135:
            text, dx, dy, ha = LABELS_135[r["id"]]
            ax.annotate(text, xy=(r["mid"], y), xytext=(dx, dy), textcoords="offset points", ha=ha, va="center",
                        fontsize=10, color=p["ink2"], linespacing=1.1)
    ssmb_y = n - 1 - 3  # the SSMB row (GROUPS_135[3])
    ax.text(2e-7, ssmb_y, "no EUV power demonstrated yet\n(2021, 2024 mechanism tests at 1064 nm)",
            ha="left", va="center", fontsize=10, color=p["ink2"], style="italic")
    ax.set_yticks(range(n))
    ax.set_yticklabels(list(reversed(GROUPS_135)), fontsize=11, linespacing=1.1)
    ax.tick_params(axis="y", length=0)
    ax.set_ylim(-0.6, n - 0.1)
    ax.set_xlim(1e-7, 2e5)
    ax.set_xticks([1e-6, 1e-4, 1e-2, 1, 1e2, 1e4])
    ax.set_xticklabels(["1 µW", "100 µW", "10 mW", "1 W", "100 W", "10 kW"])
    ax.set_xlabel("average power at 13.5 nm (13.2–13.8 nm)")
    ax.legend(handles=_legend_handles(p), loc="upper left", bbox_to_anchor=(0.0, 1.125), ncol=3, fontsize=10,
              handletextpad=0.4, columnspacing=1.2, frameon=False)
    ax.set_title("At 13.5 nm only tin plasma delivers HVM power;\naccelerator kilowatts are still designs", pad=62)
    fig.text(0.02, 0.0, "Within a class: upper row in production, middle demonstrated, lower projected.\n"
             "EUV power at IF unless marked. Every point is a sourced row of\n"
             "docs/figures/future/source_power_vs_wavelength.csv.",
             fontsize=9.5, color=p["muted"], va="top")
    save(fig, "source-landscape-euv", theme, preview)


# ============================================================================
# 6. Multilayer mirrors: best measured peak reflectance, and what ~11 reflections leave
# ============================================================================
# (label, wavelength class, measured peak R, BEUV?, source). Every value checked in the
# paper's abstract (retrieved via OpenAlex/Crossref, 2026-09-30):
#   Mo/Si:  Yakshin et al., Proc. SPIE 6517, 65170I (2007), doi:10.1117/12.711796
#           ("70.5% at 13.3 nm and 70.15% at 13.5 nm", B4C interface barriers, 50 periods)
#   Ru/Be:  Smertin et al., Opt. Express 30, 46749 (2022), doi:10.1364/OE.475079 ("record values of
#           R = 72.2%" at 11.4 nm, Mo interlayers; the 2024 Opt. Lett. 49, 3690 follow-up,
#           doi:10.1364/OL.528271, reports "R > 71%" with near-zero stress)
#   Mo/Be:  Skulina et al., Appl. Opt. 34, 3727 (1995), doi:10.1364/AO.34.003727 (68.7 +/- 0.2 % at 11.3 nm)
#   La/B:   Kuznetsov et al., Opt. Lett. 40, 3778 (2015), doi:10.1364/OL.40.003778
#           (64.1 % at 6.65 nm, 1.5 deg off normal, nitridated La)
#   LaN/B:  Makhotkin et al., Opt. Express 21, 29894 (2013), doi:10.1364/OE.21.029894 (57.3 % at 6.6 nm)
MIRRORS = [
    ("Mo/Si (B4C barriers)\n13.5 nm, 2007", 70.15, False),
    ("Ru/Be + Mo interlayers\n11.4 nm, 2022", 72.2, False),
    ("Mo/Be\n11.3 nm, 1995", 68.7, False),
    ("La/B (nitridated)\n6.65 nm, 2015", 64.1, True),
    ("LaN/B\n6.6 nm, 2013", 57.3, True),
]
N_REFLECTIONS = 11  # representative count: "about a dozen" reflections between source and wafer


def fig_multilayer_reflectance(theme: str, preview: Path | None) -> None:
    p = setup(theme)
    fig, (ax1, ax2) = plt.subplots(
        1, 2, figsize=(8.8, 4.6), sharey=True, gridspec_kw={"width_ratios": [1.25, 1.0], "wspace": 0.08}
    )
    labels = [m[0] for m in MIRRORS]
    y = np.arange(len(MIRRORS))[::-1]
    for ax in (ax1, ax2):
        style_axes(ax, p)
        ax.grid(False, axis="y")
        ax.tick_params(axis="y", length=0)
    for yi, (label, r, beuv) in zip(y, MIRRORS):
        c = p["s1"] if beuv else p["context"]
        ax1.barh(yi, r, height=0.5, color=c, zorder=3)
        ax1.text(r + 1.2, yi, f"{r:g} %", va="center", fontsize=11, color=p["ink"])
        t = (r / 100.0) ** N_REFLECTIONS * 100.0
        ax2.barh(yi, t, height=0.5, color=c, zorder=3)
        ax2.text(t + 0.05, yi, f"{t:.2f} %", va="center", fontsize=11, color=p["ink"])
    ax1.set_yticks(y)
    ax1.set_yticklabels(labels, fontsize=11, linespacing=1.1)
    ax1.set_xlim(0, 100)
    ax1.set_xticks([0, 25, 50, 75, 100])
    ax1.set_xlabel("best measured peak reflectance, one mirror (%)")
    ax1.set_title("One mirror", fontsize=12)
    ax2.set_xlim(0, 3.2)
    ax2.set_xticks([0, 1, 2, 3])
    ax2.set_xlabel(f"R¹¹: fraction left after {N_REFLECTIONS} reflections (%)")
    ax2.set_title(f"After {N_REFLECTIONS} reflections", fontsize=12)
    fig.suptitle(
        "Beyond-EUV mirrors: a few points less per mirror\nbecomes ~3–9× less light at the wafer",
        x=0.02, y=1.08, ha="left", fontsize=13, fontweight="bold", color=p["ink"],
    )
    fig.text(
        0.02, -0.06,
        "Blue: 6.x nm (La/B family); gray: 11–13.5 nm. Records as reported in each paper's\n"
        "abstract (sources in the script and on the page). The 11-reflection column is simple\n"
        "arithmetic with a representative mirror count, not a model of a specific scanner.",
        fontsize=9, color=p["muted"], va="top",
    )
    save(fig, "multilayer-reflectance", theme, preview)


FIGURES = {
    "photons-per-feature": fig_photons_per_feature,
    "multilayer-reflectance": fig_multilayer_reflectance,
    "source-landscape": fig_source_landscape,
    "source-landscape-euv": fig_source_landscape_euv,
    "polarization-contrast": fig_polarization_contrast,
    "noon-fringes": fig_noon_fringes,
    "talbot-carpet": fig_talbot_carpet,
}


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--preview", type=Path, default=None, help="also write PNG previews here")
    ap.add_argument("only", nargs="*", help="figure names to build (default: all)")
    args = ap.parse_args()
    names = args.only or list(FIGURES)
    for name in names:
        for theme in ("light", "dark"):
            FIGURES[name](theme, args.preview)


if __name__ == "__main__":
    main()
