"""Source figures: power chains (S1), coherence → pupil (S2), per-family physics (S3a–c), the power
landscape, dose-limited throughput, the X-ray spectra and the spectra + pupil gallery.

Simulator numbers come from the presets' ``derived_quantities()`` / ``average_power_w`` /
``wafer_throughput`` / ``spectrum`` / ``pupil_fill`` / ``spectral_flux_density``; published numbers
come from ``data/source_power_vs_wavelength.csv`` (one citation + URL per row). Every bar of a power
chain is a number the code reports (or a product of two it reports, named as such in the row label);
hatched, ringed and hollow bars follow ``tokens.PROVENANCE`` and every chain figure carries the legend.

Physical asserts live in ``compute()`` (``make_all.py --verify --group sources`` runs them without
drawing): every ``derived_quantity`` name a chain uses exists for its preset, the coherence → σ bridge
reproduces each preset's pupil, the 1 kW waterfall arithmetic equals the engine's, the LPA-FEL sweep
reproduces the preset's saturation length, the SASE rms equals 1/√M, and the bending-magnet
``xray_spectrum()`` probe decides panel (d) of the synchrotron figure (the caption names the branch).
"""

from __future__ import annotations

import csv
import math
import sys
import textwrap

import numpy as np
from matplotlib import pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.patches import Patch, Rectangle

import style as S
from common import HERE, FigureSpec, fmt_si, native, register

_REF = HERE.parent / "reference"
if str(_REF) not in sys.path:
    sys.path.insert(0, str(_REF))
import matrix_parse  # noqa: E402

CSV_PATH = HERE / "data" / "source_power_vs_wavelength.csv"
DODGE = 1.04  # landscape: published points at lambda / 1.04, presets at lambda * 1.04 (visibility only)
EV_J = 1.602176634e-19
HC_EV_NM = 1239.84193
TP_DOSE = 30.0
HVM_BAND_W = (250.0, 1000.0)  # production 13.5 nm need at IF: NXE:3400B → the stated 1 kW target

# Capability-matrix rows the captions name (resolved through extra["model_rows"]; the chips on the
# power-chain panels are read from the matrix through matrix_parse so a grade change reflows them).
ROW_LPP = "Laser-produced plasma (Sn 13.5; Gd 6.7; Tb 6.5 nm)"
ROW_DPP = "Discharge-produced plasma (DPP/LDP, Xe/Sn 13.5 nm)"
ROW_SYNC = "Synchrotron (bending magnet / undulator)"
ROW_HHG = "High-harmonic generation"
ROW_XFEL = "XFEL (SASE / seeded)"
ROW_SXRL = "Plasma soft-X-ray laser (Ne-like Ar 46.9, Ni-like Ag 13.9 / Cd 13.2 / Mo 18.9 nm)"
ROW_TUBE = "Hard X-ray tube (W / Mo / Cu / Rh, 5–300 kV)"
ROW_ICS = "Inverse Compton scattering"
ROW_BETATRON = "Laser-wakefield betatron X-rays"
ROW_SSMB = "SSMB storage ring"
ROW_SP = "Smith–Purcell free-electron grating"
ROW_NOON = "Entangled-photon (NOON) source"
ROW_LPA = "LPA-FEL"
ROW_VUV = "VUV excimer (F₂ 157.63 nm; Ar₂ 126 nm)"
ROW_DUV = "DUV/UV heritage (ArF 193.368, KrF 248.3, Hg g/h/i)"
ROW_TP = "Dose-limited wafer throughput"
ROW_FILLS = "Illumination pupil fills"
ROW_PHYSICS = "Machine-parameter physics layer"

# (key, factory, short label, family for the 13.5 nm panel, status of the preset's power)
#   status: "measured" = anchored to a demonstrated machine class; "projection" = design point / what-if
PRESETS = (
    ("krf", lambda S_: S_.krf_laser(), "KrF laser", None, "measured"),
    ("arf", lambda S_: S_.arf_laser(), "ArF laser", None, "measured"),
    ("f2", lambda S_: S_.f2_laser(), "F$_2$ laser", None, "measured"),
    ("ar2", lambda S_: S_.ar2_laser(), "Ar$_2$ (hypothetical)", None, "projection"),
    ("lpa_fel", lambda S_: S_.lpa_fel_bella_25nm(), "LPA-FEL 25 nm", None, "projection"),
    ("lpp_sn", lambda S_: S_.lpp_sn_13nm5(), "Sn LPP, CO$_2$ drive", "Sn LPP", "measured"),
    # NXE:3800E class: 500 W at IF is the sourced shipping figure (only the 43 kW drive is assumed)
    ("lpp_sn_500", lambda S_: S_.lpp_sn_13nm5_500w(), "Sn LPP, NXE:3800E class", "Sn LPP", "measured"),
    ("lpp_sn_1um", lambda S_: S_.lpp_sn_13nm5(drive_laser="solid_state_1um"), "Sn LPP, 1 µm drive", "Sn LPP",
     "projection"),
    ("lpp_sn_2um", lambda S_: S_.lpp_sn_13nm5(drive_laser="thulium_2um"), "Sn LPP, 2 µm drive", "Sn LPP",
     "projection"),
    ("lpp_gd", lambda S_: S_.lpp_gd_6nm7(), "Gd LPP 6.7 nm", None, "projection"),
    ("lpp_tb", lambda S_: S_.lpp_tb_6nm5(), "Tb LPP 6.5 nm", None, "projection"),
    ("dpp_sn", lambda S_: S_.dpp_sn_13nm5(), "Sn LDP", "Discharge plasma", "measured"),
    ("dpp_xe", lambda S_: S_.dpp_xe_13nm5(), "Xe DPP", "Discharge plasma", "measured"),
    ("sync_euv", lambda S_: S_.synchrotron_compact_euv(), "compact undulator", "Storage-ring undulator", "measured"),
    ("flash", lambda S_: S_.xfel_flash_13nm5(), "FLASH-class", "Linac FEL", "measured"),
    ("fermi", lambda S_: S_.xfel_fermi_seeded(), "FERMI-class", "Linac FEL", "measured"),
    ("cw_sc", lambda S_: S_.xfel_cw_sc_13nm5(), "CW-SC linac FEL", "Linac FEL", "projection"),
    ("erl", lambda S_: S_.xfel_erl_13nm5(), "ERL-FEL", "ERL FEL", "projection"),
    ("ssmb", lambda S_: S_.ssmb_euv_13nm5(), "SSMB", "SSMB", "projection"),
    ("ics", lambda S_: S_.ics_compact_euv_13nm5(), "ICS design", "Inverse Compton", "projection"),
    ("hhg_ne", lambda S_: S_.hhg_ne_13nm5(), "HHG Ne 13.56 nm", "HHG", "measured"),
    ("hhg_ar", lambda S_: S_.hhg_ar_30nm(), "HHG Ar 29.6 nm", None, "measured"),
    ("sxrl_ag", lambda S_: S_.sxrl_ag_13nm9(), "SXRL Ag 13.9 nm", "Soft-X-ray laser", "measured"),
    ("sxrl_cd", lambda S_: S_.sxrl("cd_13nm2"), "SXRL Cd 13.2 nm", "Soft-X-ray laser", "measured"),
    ("sxrl_mo", lambda S_: S_.sxrl("mo_18nm9"), "SXRL Mo 18.9 nm", None, "measured"),
    ("sxrl_ar", lambda S_: S_.sxrl_ar_46nm9(), "SXRL Ar 46.9 nm", None, "measured"),
    ("smith_purcell", lambda S_: S_.smith_purcell(), "Smith–Purcell", "Smith–Purcell", "projection"),
    ("tube_w", lambda S_: S_.xray_tube("W"), "X-ray tube W 60 kV (4π)", None, "measured"),
    ("tube_mo", lambda S_: S_.xray_tube("Mo"), "X-ray tube Mo", None, "measured"),
    ("tube_cu", lambda S_: S_.xray_tube("Cu"), "X-ray tube Cu", None, "measured"),
    ("betatron", lambda S_: S_.betatron(), "betatron", None, "projection"),
    ("noon", lambda S_: S_.entangled_noon(), "entangled NOON (photon power)", None, "projection"),
)

# categorical columns of the 13.5 nm panel, left to right
FAMILIES_13 = ("Sn LPP", "Discharge plasma", "Linac FEL", "ERL FEL", "SSMB", "Storage-ring undulator",
               "Inverse Compton", "HHG", "Soft-X-ray laser", "Smith–Purcell")


# ---------------------------------------------------------------------------
# Small helpers shared by the source figures
# ---------------------------------------------------------------------------


def _dq(src) -> dict[str, float]:
    """``derived_quantities()`` as {name: value}."""
    return {name: val for name, val, _u, _n in src.derived_quantities()}


def _need(dq: dict[str, float], who: str, *names: str) -> list[float]:
    """The named derived quantities, asserting every one exists (the chains depend on them)."""
    missing = [n for n in names if n not in dq]
    if missing:
        raise AssertionError(f"{who}: derived_quantities() lacks {missing}; the power chain needs them")
    return [float(dq[n]) for n in names]


def _fmt_w(value: float, unit: str = "W") -> str:
    """``fmt_si`` extended from femto to tera for the chain rows (NOON picowatts, megawatt beams)."""
    if value == 0 or not math.isfinite(value):
        return f"{value:g} {unit}"
    prefixes = [(1e12, "T"), (1e9, "G"), (1e6, "M"), (1e3, "k"), (1.0, ""), (1e-3, "m"), (1e-6, "µ"), (1e-9, "n"), (1e-12, "p"), (1e-15, "f")]
    v = float(f"{value:.3g}")
    for scale, pre in prefixes:
        if abs(v) >= scale:
            return f"{float(f'{v / scale:.3g}'):g} {pre}{unit}"
    return f"{v:.2g} {unit}"


def _grade(row: str) -> str:
    """Grade slug of a matrix row (read from capability-matrix.md; a grade change reflows the chip)."""
    r = matrix_parse.find(row)
    if r is None:
        raise AssertionError(f"capability-matrix row not found: {row!r}")
    return r.slug


def _sci(x: float, digits: int = 2) -> str:
    """5.32×10⁻³ with Unicode superscripts (plain text, no mathtext)."""
    if x == 0 or not math.isfinite(x):
        return f"{x:g}"
    exp = int(math.floor(math.log10(abs(x))))
    mant = x / 10 ** exp
    if round(mant, digits) >= 10:
        mant /= 10
        exp += 1
    sup = str(exp).translate(str.maketrans("-0123456789", "⁻⁰¹²³⁴⁵⁶⁷⁸⁹"))
    return f"{mant:.{digits}f}×10{sup}"


def _pct(x: float) -> str:
    """Compact percentage for factor labels: 6 %, 0.49 %, 4.5e-8."""
    if x >= 1e-3:
        s = f"{x * 100:.3g}"
        return f"{s} %"
    return _sci(x, 1)


# ===========================================================================
# S1. Power chains: one draw helper, one FigureSpec per family panel, the composite, the waterfall
# ===========================================================================

#: Provenance classes drawn (legend order). Every chain figure draws the legend.
PROV_LEGEND = (
    ("measured", "measured / shipping (demonstrated machine class)"),
    ("calibrated", "calibrated to a published pair"),
    ("reported", "reported order of magnitude"),
    ("assumed", "model assumption"),
    ("projection", "projection / design point / what-if"),
)

_CHAINS_CACHE: dict | None = None


def _wafer(src, preset: str) -> tuple[float, float, dict]:
    """(power at the wafer, IF→wafer transmission, the throughput dict) through the named optics preset."""
    r = src.wafer_throughput(TP_DOSE, preset=preset)
    assert r is not None, f"wafer_throughput returned None for a source with average power ({preset})"
    return float(r["power_at_wafer_w"]), float(r["optics_transmission"]) * float(r["mask_efficiency"]), r


def _chain(label: str, slot: int, values: list, prov: list, short: str | None = None) -> dict:
    assert len(values) == len(prov)
    return {"label": label, "short": short or label, "slot": slot, "values": values, "prov": prov}


def _panel(key: str, title: str, row: str, rows: list, chains: list, factors: list, *, hvm: bool,
           anchors: list | None = None, note: str = "") -> dict:
    assert len(factors) == len(rows) - 1, key
    for c in chains:
        assert len(c["values"]) == len(rows), (key, c["label"])
    return {"key": key, "title": title, "row": row, "rows": rows, "chains": chains, "factors": factors,
            "hvm": hvm, "anchors": anchors or [], "note": note}


def _all_chains() -> dict:
    """Every family's power chain from the engine (computed once per process; cheap, no imaging)."""
    global _CHAINS_CACHE
    if _CHAINS_CACHE is not None:
        return _CHAINS_CACHE
    huv = native()
    S_ = huv.SourceConfig
    panels: dict[str, dict] = {}

    # ---- LPP: drive → in-band into 2π → at IF → on the wafer ------------------------------------
    rows = [("drive laser", "laser output"), ("in-band emission", "into 2π sr, at the source"),
            ("in-band power", "at intermediate focus"), ("power on the wafer", "model optics train")]
    lpp = (
        ("Sn, CO$_2$ 10.6 µm (NXE:3400B chain)", "Sn CO$_2$", S_.lpp_sn_13nm5(), "euv", "reported", "reported", "calibrated"),
        ("Sn, NXE:3800E class", "Sn 500 W", S_.lpp_sn_13nm5_500w(), "euv", "assumed", "assumed", "measured"),
        ("Sn, 1 µm solid-state drive", "Sn 1 µm", S_.lpp_sn_13nm5(drive_laser="solid_state_1um"), "euv", "assumed",
         "reported", "projection"),
        ("Sn, 2 µm thulium drive", "Sn 2 µm", S_.lpp_sn_13nm5(drive_laser="thulium_2um"), "euv", "assumed",
         "projection", "projection"),
        ("Gd 6.7 nm", "Gd", S_.lpp_gd_6nm7(), "beuv", None, "assumed", "projection"),
        ("Tb 6.5 nm", "Tb", S_.lpp_tb_6nm5(), "beuv", None, "assumed", "projection"),
    )
    chains, ce, eta, tw = [], {}, None, {}
    for j, (label, short, src, preset, p_drive, p_2pi, p_if) in enumerate(lpp, start=1):
        dq = _dq(src)
        p2pi, pif, fet = _need(dq, label, "in_band_emission_2pi", "in_band_power_at_if", "etendue_limited_fraction")
        drive = None
        if "conversion_efficiency_default" in dq:
            ce[short] = dq["conversion_efficiency_default"]
            drive = p2pi / ce[short]
        if short == "Sn CO$_2$":
            eta = pif / (p2pi * fet)
        pw, t, _r = _wafer(src, preset)
        tw[preset] = t
        chains.append(_chain(label, j, [drive, p2pi, pif, pw], [p_drive, p_2pi, p_if, "assumed"], short))
    assert eta is not None and abs(eta - 250.0 / 1290.0) < 1e-6, f"Sn chain η₂π→IF = {eta} (expected 250/1290)"
    panels["lpp"] = _panel(
        "lpp", "Laser-produced plasma", ROW_LPP, rows, chains,
        [f"× CE: {_pct(ce['Sn CO$_2$'])} CO$_2$ (reported, NXE:3400B) · {_pct(ce['Sn 1 µm'])} 1 µm (reported, laboratory) · "
         f"{_pct(ce['Sn 2 µm'])} 2 µm (projection); the Gd/Tb chains start at the in-band emission the code reports",
         f"× η$_{{2π→IF}}$ = {eta:.3f} (calibrated: 250 W / 1290 W on the Sn chain, reused for Gd/Tb) × étendue fraction {fet:g} "
         "(assumed NXE-like geometry)",
         f"× {_pct(tw['euv'])} Mo/Si column (0.70¹⁰ × 0.65, assumed) · {_pct(tw['beuv'])} La/B column for Gd/Tb (0.641¹¹)"],
        hvm=True)

    # ---- DPP: electrical → into 2π → collected → at IF → on the wafer ---------------------------
    rows = [("electrical input", "capacitor bank"), ("in-band emission", "into 2π sr, at the source"),
            ("collected", "collector and debris trap"), ("in-band power", "at intermediate focus"),
            ("power on the wafer", "model optics train")]
    chains, facts = [], {}
    for j, (label, short, src, p2) in enumerate((("Sn LDP (TRINITI anchor)", "Sn LDP", S_.dpp_sn_13nm5(), "reported"),
                                                 ("Xe DPP (Energetiq EQ-10 class)", "Xe DPP", S_.dpp_xe_13nm5(), "reported")),
                                                start=1):
        dq = _dq(src)
        p2pi, heat, coll, fg, pif = _need(dq, label, "in_band_power_2pi_w", "electrode_heat_load_w", "collected_power_w",
                                          "etendue_usable_fraction", "power_at_if_w")
        pelec = p2pi + heat  # the code reports P_2π and the electrical input not radiated in band
        pw, t, _r = _wafer(src, "euv")
        facts[short] = dict(ce=p2pi / pelec, coll=coll / p2pi, fg=fg, t=t)
        chains.append(_chain(label, j, [pelec, p2pi, coll, pif, pw], ["assumed", p2, "assumed", "assumed", "assumed"], short))
    panels["dpp"] = _panel(
        "dpp", "Discharge-produced plasma", ROW_DPP, rows, chains,
        [f"× CE = {_pct(facts['Sn LDP']['ce'])} Sn · {_pct(facts['Xe DPP']['ce'])} Xe (reported order of magnitude; electrical input implied)",
         f"× Ω$_c$/2π × η$_c$ = {facts['Sn LDP']['coll']:.3f} (1.5 sr collector, throughput 0.4: assumed)",
         f"× étendue fraction {facts['Sn LDP']['fg']:.2f} Sn · {facts['Xe DPP']['fg']:.2f} Xe (G$_{{ill}}$ = 3.3 mm²·sr assumed)",
         f"× {_pct(facts['Sn LDP']['t'])} Mo/Si column (assumed)"],
        hvm=True)

    # ---- HHG: driver → harmonic (vs the measured record) → driver a 250 W source would need -----
    rows = [("driver average power", "laser output, assumed"), ("single-harmonic power", "derived; diamond = measured record")]
    chains, anchors, eff = [], [], {}
    for j, (label, short, src) in enumerate((("Ne, harmonic 59 → 13.56 nm", "Ne 13.5 nm", S_.hhg_ne_13nm5()),
                                             ("Ar, harmonic 27 → 29.6 nm", "Ar 29.6 nm", S_.hhg_ar_30nm())), start=1):
        dq = _dq(src)
        p, ceff, rec = _need(dq, label, "derived_harmonic_power", "conversion_efficiency", "measured_power_record")
        driver = p / ceff
        eff[short] = ceff
        assert p <= rec, f"{label}: the derived power exceeds the measured record; the caption says it stays below"
        chains.append(_chain(label, j, [driver, p], ["assumed", "assumed"], short))
        anchors.append((1, j, rec, f"measured record {_fmt_w(rec)}", "measured"))
    panels["hhg"] = _panel(
        "hhg", "High-harmonic generation", ROW_HHG, rows, chains,
        [f"× η = {_pct(eff['Ne 13.5 nm'])} Ne · {_pct(eff['Ar 29.6 nm'])} Ar (order-of-magnitude conversion efficiency, ±1 decade)"],
        hvm=True, anchors=anchors)

    # ---- ICS: beam and laser inputs → collected in-band power --------------------------------------
    src = S_.ics_compact_euv_13nm5()
    dq = _dq(src)
    pbeam, plaser, pav, nph, fcol, ncol, eph = _need(dq, "ICS", "electron_beam_power", "laser_average_power", "average_power",
                                                       "photons_per_collision", "collected_fraction",
                                                       "collected_photons_per_collision", "photon_energy")
    rate = src.rep_rate_hz
    rows = [("electron-beam power", "accelerator, 10 mA at 1.73 MeV"), ("circulating laser power", "enhancement cavity"),
            ("in-band power", "2 % band, collected")]
    panels["ics"] = _panel(
        "ics", "Inverse Compton scattering", ROW_ICS, rows,
        [_chain("compact EUV design point", 1, [pbeam, plaser, pav], ["assumed", "assumed", "projection"], "ICS design")],
        ["collision of the two beams at 10 µm spots, 100 MHz (design point)",
         f"Thomson luminosity: {nph:.3g} photons per collision × {_pct(fcol)} collected = {ncol:.3g} × {eph:.1f} eV × {rate:.3g} Hz"],
        hvm=True)

    # ---- SSMB: full-bunching coherent power → × b² → stored projection; incoherent floor ---------
    src = S_.ssmb_euv_13nm5()
    dq = _dq(src)
    pfull, b, pcoh, pinc = _need(dq, "SSMB", "power_at_full_bunching", "bunching_factor", "coherent_power", "incoherent_power")
    assert abs(pfull * b * b - pcoh) < 1e-6 * pcoh, "SSMB coherent power ≠ P(b=1)·b²"
    rows = [("coherent power at b = 1", "radiator, full bunching"), (f"coherent power at b = {b:.3f}", "stored design projection"),
            ("incoherent power", "same radiator and current")]
    panels["ssmb"] = _panel(
        "ssmb", "SSMB storage ring", ROW_SSMB, rows,
        [_chain("Tsinghua-class design point (400 MeV, 1 A)", 1, [pfull, pcoh, pinc], ["projection", "projection", "projection"], "SSMB 1 kW")],
        [f"× b² = {b:.3f}² = {b * b:.4f} (assumed; no bunching factor has been demonstrated at any EUV harmonic)",
         f"÷ coherent enhancement N|b|²(I_pk/e)λ/c = {pcoh / pinc:.3g}"],
        hvm=True)

    # ---- Smith–Purcell: ideal coupling → × ε (assumed) → estimate; vdW-crystal anchor ------------
    euv = S_.smith_purcell()
    vdw = S_.smith_purcell(target_wavelength_nm=1.02, grating_period_nm=0.335)
    d_e, d_v = _dq(euv), _dq(vdw)
    e_ideal, e_est, e_rate = _need(d_e, "SP EUV", "ideal_coupling_power_w", "emitted_power_w", "photon_rate_per_na")
    v_ideal, v_est, v_lam, v_ev = _need(d_v, "SP vdW", "ideal_coupling_power_w", "emitted_power_w", "emitted_wavelength_nm", "photon_energy_ev")
    assert abs(v_lam - 1.02) < 0.02, f"vdW-period case emits {v_lam} nm (expected ≈ 1.02 nm)"
    eps = e_est / e_ideal
    vdw_anchor = 80.0 * 10.0 * v_ev * EV_J  # Huang et al. 2023: ~80 photons/s per nA, at 10 nA and this photon energy
    ratio = v_est / vdw_anchor
    assert 3e4 < ratio < 8e4, f"vdW ratio {ratio:.3g} — the caption says ~5×10⁴"
    rows = [("ideal coupling, ε = 1, h = 0", "heuristic scale, not a limit"), (f"estimate at ε = {eps:.0e}", "assumed coupling"),
            ("measured vdW-crystal yield", "Huang 2023: ~80 photons/s per nA")]
    panels["smith-purcell"] = _panel(
        "smith-purcell", "Smith–Purcell grating", ROW_SP, rows,
        [_chain("13.5 nm preset (4.43 nm grating, 30 keV, 10 nA)", 1, [e_ideal, e_est, None], ["projection", "assumed", "assumed"], "13.5 nm preset"),
         _chain("0.335 nm crystal period (1.02 nm, 1.2 keV)", 2, [v_ideal, v_est, vdw_anchor], ["projection", "assumed", "measured"], "crystal period")],
        [f"× ε = {eps:.0e} (assumed)", f"the estimate sits {ratio:.2g}× above the measured yield at the crystal period"],
        hvm=True)

    # ---- X-ray tube: beam power → bremsstrahlung → after the Be window → through 1 cm² at 100 mm --
    rows = [("electron-beam power", "V × I on the anode"), ("bremsstrahlung generated", "4π, before the window"),
            ("X-rays after the Be window", "4π-equivalent, E ≥ 1 keV, + lines"), ("through 1 cm² at 100 mm", "isotropic point source")]
    chains, etas = [], {}
    for j, (anode, label) in enumerate((("W", "W, 60 kV × 30 mA"), ("Mo", "Mo, 50 kV × 40 mA"), ("Cu", "Cu, 40 kV × 40 mA"),
                                        ("Rh", "Rh, 50 kV × 40 mA")), start=1):
        src = S_.xray_tube(anode)
        dq = _dq(src)
        pb, eta_b, pbr, px, pd = _need(dq, f"tube {anode}", "beam_power_w", "bremsstrahlung_efficiency", "bremsstrahlung_power_w",
                                       "xray_power_w", "power_density_at_100mm")
        etas[anode] = eta_b
        chains.append(_chain(label, j, [pb, pbr, px, pd * 1e-3], ["reported", "reported", "assumed", "assumed"], anode))
    panels["xray-tube"] = _panel(
        "xray-tube", "Hard X-ray tube", ROW_TUBE, rows, chains,
        [f"× η = 1.1×10⁻⁹ Z V: {_pct(etas['W'])} W · {_pct(etas['Mo'])} Mo · {_pct(etas['Cu'])} Cu · {_pct(etas['Rh'])} Rh (empirical thick-target law, ±20 %)",
         "+ characteristic lines (empirical (U − 1)^1.67 law, ±×2) − Be-window absorption; no anode self-absorption",
         "× 1 cm² / 4π (100 mm)² (isotropic; a LIGA mask plane)"],
        hvm=False)

    # ---- Betatron: X-ray average power → coherent part -------------------------------------------
    src = S_.betatron()
    dq = _dq(src)
    pav, zeta, nsh, emean = _need(dq, "betatron", "average_power_w", "coherent_fraction", "photons_per_shot", "mean_photon_energy_kev")
    rows = [("X-ray average power", "all angles and energies, 10 Hz"), ("coherent part", "× ζ, Kim's form, both planes")]
    panels["betatron"] = _panel(
        "betatron", "Laser-wakefield betatron", ROW_BETATRON, rows,
        [_chain("100 TW-class LWFA preset (200 MeV, 10¹⁹ cm⁻³)", 1, [pav, pav * zeta], ["projection", "projection"], "betatron")],
        [f"× ζ = {zeta:.2g} ({nsh:.2g} photons × {emean:.2f} keV per shot; the K = 8.3 wiggler regime is incoherent)"],
        hvm=False)

    # ---- SXRL: peak power → average power (reported anchors) ---------------------------------------
    rows = [("peak power", "pulse energy ÷ pulse duration"), ("average power", "pulse energy × repetition rate")]
    chains = []
    for j, (label, short, src, prov) in enumerate((("Ni-like Ag 13.9 nm (10 µJ × 10 Hz)", "Ag 13.9", S_.sxrl_ag_13nm9(), "reported"),
                                                   ("Ni-like Cd 13.2 nm (0.1 µJ × 10 Hz)", "Cd 13.2", S_.sxrl("cd_13nm2"), "reported"),
                                                   ("Ni-like Mo 18.9 nm (1 µJ × 100 Hz)", "Mo 18.9", S_.sxrl("mo_18nm9"), "reported"),
                                                   ("Ne-like Ar 46.9 nm (13 µJ × 12 Hz)", "Ar 46.9", S_.sxrl_ar_46nm9(), "reported")),
                                                  start=1):
        dq = _dq(src)
        ppk, pav = _need(dq, label, "peak_power_w", "average_power_w")
        chains.append(_chain(label, j, [ppk, pav], ["assumed", prov], short))
    panels["sxrl"] = _panel(
        "sxrl", "Plasma soft-X-ray laser", ROW_SXRL, rows, chains,
        ["× duty cycle τ × f (assumed ps / ns durations; Ar and Mo pulse–rate splits published, Ag and Cd assumed to match reported powers)"],
        hvm=True)

    # ---- NOON: entangled beam vs the classical laser; the pair rate a 1 s exposure would need ------
    src = S_.entangled_noon()
    dq = _dq(src)
    pph, need = _need(dq, "NOON", "photon_power", "pair_rate_for_1s_bound")
    pair_rate = 1e6  # the factory's default pair_rate_hz (the preset's own input)
    f2 = S_.f2_laser()
    rows = [("entangled beam, photon power", "10⁶ pairs/s × 2 × 7.87 eV"), ("pairs to clear 1 cm² in 1 s", "at the loosest σ$_E$ bound, 10²² pairs/s"),
            ("classical F$_2$ laser, for scale", "laser output")]
    panels["noon"] = _panel(
        "noon", "Entangled-photon NOON source", ROW_NOON, rows,
        [_chain("NOON N = 2 at 157.63 nm", 1, [pph, pph * need / pair_rate, None], ["assumed", "projection", "assumed"], "NOON"),
         _chain("F$_2$ excimer laser", 2, [None, None, float(f2.average_power_w)], ["assumed", "assumed", "reported"], "F$_2$")],
        [f"× {need / pair_rate:.0e}: the pair rate the ETPA cross-section bound (10⁻²³ cm²) demands for a 1 s exposure",
         "the classical dose is 2.4 × 10¹⁶ photons/cm² at 30 mJ/cm²"],
        hvm=False)

    # ---- Synchrotron: undulator total → central cone → coherent; LIGA bending magnet per mrad -----
    und, bm = S_.synchrotron_compact_euv(), S_.synchrotron_liga_bending_magnet()
    du, db = _dq(und), _dq(bm)
    ptot, pline, lfrac, zeta, pcoh = _need(du, "undulator", "total_undulator_power", "central_cone_power", "line_power_fraction",
                                           "coherent_fraction", "coherent_power")
    pmrad, pmono = _need(db, "bending magnet", "power_per_mrad", "mono_power_per_mrad")
    rows = [("total undulator power", "all harmonics, all angles"), ("central-cone line power", "n = 1 line = average_power_w"),
            ("coherent power", "× ζ, matched-beta upper bound"), ("white beam, per mrad", "KARA-class bend, 2.5 GeV, 1.5 T"),
            ("monochromatised, per mrad", "0.2 nm, 0.1 % band")]
    panels["synchrotron"] = _panel(
        "synchrotron", "Synchrotron", ROW_SYNC, rows,
        [_chain("compact EUV undulator (538 MeV, K = 1, N = 100, 200 mA)", 1, [ptot, pline, pcoh, None, None],
                ["assumed", "assumed", "assumed", "assumed", "assumed"], "compact undulator"),
         _chain("LIGA bending magnet, per mrad of horizontal acceptance", 2, [None, None, None, pmrad, pmono],
                ["reported", "reported", "reported", "reported", "reported"], "bending magnet")],
        [f"× line fraction {_pct(lfrac)} (πα Q$_1$ (I/e) E$_γ$: N cancels)", f"× ζ = {zeta:.3f} ($ζ_x$ × $ζ_y$; real lattices are not matched)",
         "the bending-magnet rows are a different machine (KARA-class white beam); units W per mrad",
         "× 0.1 % band at the 0.2 nm selection (S(E/E$_c$) at y = 0.99)"],
        hvm=True, anchors=[(1, 1, 11e-3, "PSI XIL-II: ≈ 11 mW delivered to the sample (page value)", "measured")])

    # ---- XFEL: beam power → FEL power (× efficiency) → on the wafer ----------------------------------
    rows = [("average electron-beam power", "to dump or energy-recover"), ("FEL average power", "pulse energy × repetition rate"),
            ("power on the wafer", "model optics train")]
    chains, effs = [], {}
    for j, (label, short, src, prov) in enumerate((("FLASH-class SASE (100 µJ × 1 kHz)", "FLASH", S_.xfel_flash_13nm5(), "reported"),
                                                   ("FERMI-class seeded (20 µJ × 50 Hz)", "FERMI", S_.xfel_fermi_seeded(), "reported"),
                                                   ("CW-SC linac (1 MHz) — projection", "CW-SC", S_.xfel_cw_sc_13nm5(), "projection"),
                                                   ("ERL at 162.5 MHz — projection", "ERL", S_.xfel_erl_13nm5(), "projection")),
                                                  start=1):
        dq = _dq(src)
        pb, pav, eff = _need(dq, label, "beam_power_average", "average_power", "fel_efficiency")
        pw, t, _r = _wafer(src, "euv")
        effs[short] = eff
        chains.append(_chain(label, j, [pb, pav, pw], ["assumed", prov, "assumed"], short))
    panels["xfel"] = _panel(
        "xfel", "X-ray free-electron laser", ROW_XFEL, rows, chains,
        [f"× FEL efficiency ≈ ρ at saturation: {_pct(effs['FLASH'])} FLASH · {_pct(effs['FERMI'])} FERMI · {_pct(effs['CW-SC'])} CW-SC · {_pct(effs['ERL'])} ERL",
         f"× {_pct(t)} Mo/Si column (assumed)"],
        hvm=True)

    # ---- LPA-FEL: peak beam power → Ming-Xie saturation → × duty → average; the stored pulse energy --
    und_kw = dict(undulator_period_mm=20.0, num_periods=200, peak_current_a=1000.0, norm_emittance_um=0.5, beta_m=1.0)
    p25 = S_.lpa_fel_bella_25nm()
    p420 = S_.lpa_fel(420.0, electron_energy_mev=100.0, undulator_k=1.118, energy_spread_rel=1e-2, rep_rate_hz=1.0,
                      pulse_energy_uj=1.0, **und_kw)
    chains, lam = [], {}
    rows = [("average electron-beam power", "peak beam power × τ × f$_{rep}$"),
            ("saturated FEL average power", "Ming-Xie saturation × τ × f$_{rep}$"),
            ("stored average power", "pulse energy × rate, as modelled")]
    for j, (label, short, src, prov_last) in enumerate((("25 nm / 500 MeV / 1 kHz preset (projection)", "25 nm", p25, "projection"),
                                                        ("420 nm / 100 MeV / 1 Hz (BELLA class; 1 µJ assumed)", "420 nm", p420, "assumed")), start=1):
        dq = _dq(src)
        pb, psat, esat, pav, mx, ratio = _need(dq, label, "beam_power_peak", "saturation_power_3d", "saturation_pulse_energy_3d",
                                               "average_power", "ming_xie_lambda", "energy_spread_over_rho")
        duty = float(src.pulse_duration_fs) * 1e-15 * float(src.rep_rate_hz)
        assert abs(psat * float(src.pulse_duration_fs) * 1e-15 - esat) < 1e-6 * esat, f"{label}: saturation energy ≠ power × τ"
        lam[short] = (mx, ratio, duty)
        chains.append(_chain(label, j, [pb * duty, esat * src.rep_rate_hz, pav], ["assumed", "projection", prov_last], short))
    panels["lpa-fel"] = _panel(
        "lpa-fel", "Laser-plasma-accelerator FEL", ROW_LPA, rows, chains,
        [f"× 1.6 ρ (L$_g$/L$_{{g,3D}}$)²: Λ = {lam['25 nm'][0]:.1f} at 25 nm ($σ_δ$ = {lam['25 nm'][1]:.2f} ρ, no practical gain) · "
         f"Λ = {lam['420 nm'][0]:.1f} at 420 nm; both beams illustrative, not BELLA's",
         "the stored 5 µJ (25 nm) and 1 µJ (420 nm) are assumptions, not outputs of the gain physics"],
        hvm=True)

    # ---- Excimer lasers: output → on the wafer (refractive column) ----------------------------------
    for key, title, row, members in (("duv", "DUV/UV heritage lasers", ROW_DUV, (("ArF 193.368 nm (15 mJ × 6 kHz)", "ArF", S_.arf_laser(), "reported"),
                                                                                  ("KrF 248.3 nm (10 mJ × 4 kHz)", "KrF", S_.krf_laser(), "reported"))),
                                     ("vuv", "VUV excimer lasers", ROW_VUV, (("F$_2$ 157.63 nm (10 mJ × 4 kHz)", "F$_2$", S_.f2_laser(), "reported"),
                                                                             ("Ar$_2$ 126 nm — hypothetical", "Ar$_2$", S_.ar2_laser(), "projection")))):
        rows = [("laser output", "pulse energy × repetition rate"), ("power on the wafer", "refractive column: optics 0.3, mask 0.9")]
        chains = []
        for j, (label, short, src, prov) in enumerate(members, start=1):
            dq = _dq(src)
            (pav,) = _need(dq, label, "average_power")
            pw, t, _r = _wafer(src, "refractive")
            chains.append(_chain(label, j, [pav, pw], [prov, "assumed"], short))
        panels[key] = _panel(key, title, row, rows, chains, [f"× {_pct(t)} (optics transmission 0.3 × mask 0.9, assumed; Hg lamps carry no power model)"],
                             hvm=False)

    _CHAINS_CACHE = panels
    return panels


# ---- the draw helper ------------------------------------------------------------------------------

_ROW_H_IN = (0.38, 0.085)  # family panel: row pitch = 0.38 + 0.085 × chains (inches); bars take 62 % of it
_ROW_H_COMPACT = (0.21, 0.040)


def _row_pitch(nchains: int, compact: bool) -> float:
    a, b = _ROW_H_COMPACT if compact else _ROW_H_IN
    return a + b * nchains


def draw_waterfall(ax, panel: dict, th: S.Theme, *, compact: bool = False, xlim: tuple[float, float] | None = None,
                   chip: bool = True) -> None:
    """One family's power chain as a horizontal log-power waterfall on ``ax``.

    Rows are the stages (top → bottom), one bar per chain and row; bar styles follow
    ``tokens.PROVENANCE``; the value label at every bar end is the number the code reports;
    ``compact`` (the composite) drops the factor texts and labels only the last two rows.
    """
    rows, chains = panel["rows"], panel["chains"]
    n, nrows = len(chains), len(rows)
    S.style_axes(ax, th, grid="x")
    ax.set_xscale("log")
    vals = [v for c in chains for v in c["values"] if v] + [a[2] for a in panel["anchors"]]
    lo, hi = (min(vals), max(vals))
    if panel["hvm"]:
        hi = max(hi, HVM_BAND_W[1])
    if xlim is None:
        xlim = (10 ** (math.floor(math.log10(lo)) - 1), 10 ** (math.ceil(math.log10(hi)) + 2.6))
    base = xlim[0]
    ax.set_xlim(*xlim)
    top = 0.58
    ax.set_ylim(-(nrows - 1) - 0.62, top)
    ax.tick_params(axis="y", length=0)
    if panel["hvm"]:
        ax.axvspan(*HVM_BAND_W, color=th.band, zorder=0, lw=0)
        if not compact:
            ax.text(math.sqrt(HVM_BAND_W[0] * HVM_BAND_W[1]), -(nrows - 1) - 0.58, "HVM need at IF\n250 W → 1 kW", ha="center",
                    va="bottom", fontsize=S.SMALL_PT - 1, color=th.ink2, zorder=1)
    span = 0.72 if compact else 0.62
    pitch = span / n
    hbar = pitch * (0.82 if n > 1 else 0.6)
    for i, (label, plane) in enumerate(rows):
        y0 = -i
        items = []
        for j, ch in enumerate(chains):
            v = ch["values"][i]
            if v is None or not math.isfinite(v) or v <= 0:
                continue
            yc = y0 - (j - (n - 1) / 2) * pitch
            colour = th.s(ch["slot"])
            style = S.provenance_style(ch["prov"][i], colour, th)
            if ch["prov"][i] == "measured":
                style.update(edgecolor=colour)
            ax.add_patch(Rectangle((base, yc - hbar / 2), v - base, hbar, zorder=3, **style))
            text = _fmt_w(v)
            x_label = v
            for (row_i, slot, value, label, prov) in panel["anchors"]:
                if row_i == i and slot == ch["slot"]:
                    S.dot(ax, [value], [yc], colour, th, marker="D", filled=(prov == "measured"), size_px=7.5, zorder=5)
                    if value > x_label:  # the label starts past the diamond
                        x_label, text = value, "  " + text
                    if not compact:
                        text += f" · ◆ {label}"
            if i == nrows - 1 and (n > 1 or not compact):
                text += f"  {ch['short'] if compact else ch['label']}"
            if not compact or i == nrows - 1 or n == 1:
                items.append((x_label, yc, text))
        if items:
            S.end_labels(ax, items, th, min_gap_px=(8.5 if compact else 10.5), dx_px=4,
                         fontsize=(S.SMALL_PT - 1.5 if compact else S.SMALL_PT - 0.5))
        if not compact:
            ax.text(0.0, y0 - 0.33, plane, transform=ax.get_yaxis_transform(), ha="right", va="center",
                    fontsize=S.SMALL_PT - 1.5, color=th.muted, clip_on=False)
    if not compact:
        for i, text in enumerate(panel["factors"]):
            arrow = r"$\downarrow$ " if "$" in text else "↓ "  # the plain arrow has no glyph in the mathtext font set
            wrapped = "\n".join(textwrap.wrap(arrow + text, width=118, subsequent_indent="   "))
            ax.text(base * 1.25, -i - 0.5, wrapped, fontsize=S.SMALL_PT - 1.5, color=th.muted, va="center", ha="left",
                    zorder=4, clip_on=False, linespacing=1.15)
    ax.set_yticks([-i for i in range(nrows)])
    ax.set_yticklabels([r[0] for r in rows], fontsize=(S.SMALL_PT - 1 if compact else S.SMALL_PT))
    for lab in ax.get_yticklabels():
        lab.set_va("center" if compact else "bottom")
        if not compact:
            lab.set_y(lab.get_position()[1])
    if not compact:
        # the stage name sits above its plane label: nudge the tick labels up by a few points
        for lab in ax.get_yticklabels():
            lab.set_transform(S._offset(ax, 0, 3.5, lab.get_transform()))
    ax.set_title(panel["title"], fontsize=(S.SMALL_PT + 0.5 if compact else S.TITLE_PT), pad=(5 if compact else 9))
    if chip:  # in the title line, right-aligned, never over a bar: a few points above the axes' top edge
        ax.figure.canvas.draw()
        px = ax.transAxes.transform((1.0, 1.0)) + np.array([0.0, (9.0 if compact else 11.0) * ax.figure.dpi / 72])
        _x, y_chip = ax.transData.inverted().transform(px)
        S.chip(ax, _grade(panel["row"]), (xlim[1], y_chip), th, ha="right", va="center", size_pt=S.SMALL_PT * (0.85 if compact else 0.95))


def _prov_handles(th: S.Theme):
    handles, labels = [], []
    for kind, text in PROV_LEGEND:
        st = S.provenance_style(kind, th.s(1), th)
        if kind == "measured":
            st.update(edgecolor=th.s(1))
        handles.append(Patch(**st))
        labels.append(text)
    handles.append(Line2D([], [], linestyle="none", marker="D", markerfacecolor=th.s(1), markeredgecolor=th.s(1), markersize=S.MARKER_PT * 0.8))
    labels.append("measured anchor (diamond)")
    return handles, labels


def _panel_figure(panel: dict, th: S.Theme):
    n = len(panel["chains"])
    rows_h = len(panel["rows"]) * _row_pitch(n, False)
    fig = plt.figure(figsize=(S.FULL_WIDTH, rows_h + 1.75))
    ax = fig.add_axes([0.27, 0.95 / (rows_h + 1.75), 0.69, rows_h / (rows_h + 1.75)])
    draw_waterfall(ax, panel, th)
    ax.set_xlabel("power (W) — the plane is named on each row", fontsize=S.SMALL_PT)
    handles, labels = _prov_handles(th)
    fig.legend(handles, labels, loc="lower center", ncol=3, fontsize=S.SMALL_PT - 1, bbox_to_anchor=(0.5, 0.0),
               handlelength=1.4, columnspacing=1.2)
    return fig


def _panel_compute(key: str):
    def compute(fast: bool):  # noqa: ARG001 - the chains need no sampling
        return _all_chains()[key]

    compute.__name__ = f"chain_{key.replace('-', '_')}_compute"
    return compute


def _panel_draw(panel: dict, th: S.Theme):
    return _panel_figure(panel, th)


# Family panels: (key, page, alt, caption). Captions state the provenance of every hatched / hollow bar
# and end in the family's matrix grade (Appendix C form) resolved through model_rows.
_PANEL_PAGES = {
    "lpp": ("sources/lpp.md",
            "Log-power waterfall of six laser-produced-plasma chains from drive laser through in-band emission into 2π and "
            "power at intermediate focus to the wafer: the tin CO₂ chain lands on 250 W at IF, the NXE:3800E class on 500 W, "
            "the Gd and Tb 6.x nm projections near 12 W.",
            "<strong>From the drive laser to the wafer, six LPP chains.</strong> Each bar is a number the code reports: drive power × "
            "conversion efficiency gives the in-band emission into 2π sr, × η₂π→IF × f_étendue the power at intermediate focus, × the model's "
            "Mo/Si (or La/B) column the power on the wafer. η = 250/1290 is calibrated so the NXE:3400B chain lands on its published 250 W; "
            "the 2 µm conversion efficiency and the Gd/Tb powers are projections, the 500 W chain assumes a doubled drive. "
            "<em>Model: Laser-produced plasma ✅ (🔶 CE defaults, étendue; 🧪 Gd/Tb power) · Dose-limited wafer throughput 🔶.</em>"),
    "dpp": ("sources/dpp.md",
            "Log-power waterfall of the Sn LDP and Xe DPP chains from electrical input through in-band emission into 2π, collected power "
            "and power at intermediate focus to the wafer: 360 W into 2π becomes 34 W at IF for tin, 10 W becomes 0.7 W for xenon.",
            "<strong>Where a discharge plasma's watts go.</strong> The two planes the literature quotes are both on the chart: the "
            "in-band emission into 2π sr at the source (the DPP convention, anchored on the TRINITI 360 W and Energetiq 10 W reports) "
            "and the power at intermediate focus after the collector and the étendue cut. The pinch size, collector solid angle and "
            "throughput and the illuminator étendue are assumptions, as is the optics train to the wafer. "
            "<em>Model: Discharge-produced plasma 🔶 · Dose-limited wafer throughput 🔶.</em>"),
    "hhg": ("sources/hhg.md",
            "Log-power waterfall for the neon 13.56 nm and argon 29.6 nm HHG presets: an assumed driver power times an "
            "order-of-magnitude conversion efficiency gives 0.90 µW and 30 µW, each below the measured record at its photon "
            "energy and far below the 250 W band.",
            "<strong>An assumed driver, an order-of-magnitude efficiency, a measured record beside it.</strong> The harmonic power is "
            "driver power × conversion efficiency (±1 decade); the diamond is the best measured single-harmonic power at that photon "
            "energy, which both presets stay below; the band marks the 250 W a production source delivers at IF, nine decades up. "
            "<em>Model: High-harmonic generation ✅ (🔶 intensities).</em>"),
    "ics": ("sources/inverse-compton.md",
            "Log-power waterfall of the inverse-Compton design point: 17 kW of electron-beam power and 1 MW of circulating laser power "
            "produce 71 µW in the 2 % band, 3.5 million times short of the 250 W to 1 kW band drawn behind it.",
            "<strong>Two large inputs, one small output.</strong> The Thomson luminosity of the design point yields 1.7 × 10⁶ photons per "
            "collision, 2.8 % of them inside the 2 % Mo/Si band; at 100 MHz that is 71 µW. The beam and cavity numbers are the design "
            "point's own assumptions; no operating Compton source has produced EUV. "
            "<em>Model: Inverse Compton scattering 🧪.</em>"),
    "ssmb": ("sources/ssmb.md",
             "Log-power waterfall for the SSMB design point: 47 kW of coherent power at full bunching, times an assumed bunching factor "
             "squared, gives the stored 1 kW projection, with the 1.7 W incoherent floor of the same radiator below it.",
             "<strong>1 kW rests on b² = 0.021.</strong> The radiator and current give 47 kW at full bunching; the stored 1 kW projection "
             "needs a bunching factor of 0.146 sustained every turn, which no SSMB experiment has shown at any EUV harmonic (the two "
             "one-turn tests were at 1064 nm). Without bunching the same radiator radiates 1.7 W incoherently. "
             "<em>Model: SSMB storage ring 🧪.</em>"),
    "smith-purcell": ("sources/smith-purcell.md",
                      "Log-power waterfall for the Smith–Purcell preset and a crystal-period case: ideal-coupling scale, the estimate at an "
                      "assumed coupling of 0.001, and the measured van-der-Waals-crystal yield, which the crystal-period estimate exceeds "
                      "by about fifty thousand times.",
                      "<strong>Sub-nanowatts, and the measured anchor is fifty thousand times lower still.</strong> The power estimate is "
                      "photons per electron α·ε·N·exp(−h/h_int) × beam current × photon energy with ε = 10⁻³ assumed; the measured "
                      "van-der-Waals-crystal X-ray yield (~80 photons/s per nA) sits about 5 × 10⁴ below the estimate at the crystal period. "
                      "Smith–Purcell emission has been demonstrated only to ~230 nm. "
                      "<em>Model: Smith–Purcell free-electron grating 🧪.</em>"),
    "xray-tube": ("sources/xray-tube.md",
                  "Log-power waterfall for the W, Mo, Cu and Rh tube presets from electron-beam power through generated "
                  "bremsstrahlung and the X-rays after the beryllium window to the power through 1 cm² at 100 mm: 1.8 kW of "
                  "beam power becomes 8.9 W of X-rays and 7 mW per cm² at the mask plane.",
                  "<strong>Ninety-nine percent of the beam heats the anode.</strong> The thick-target efficiency η = 1.1 × 10⁻⁹ Z·V is "
                  "an empirical law (±20 %), the characteristic lines follow the empirical (U − 1)^1.67 law (±×2), and the model "
                  "neglects anode self-absorption, the heel effect and backscatter; the last row is the isotropic flux through 1 cm² "
                  "at 100 mm, the plane LIGA exposure times are computed in. "
                  "<em>Model: Hard X-ray tube 🔶 (✅ lines, edges, Duane–Hunt, Be window).</em>"),
    "betatron": ("sources/betatron.md",
                 "Log-power bars for the laser-wakefield betatron preset: 3.5 µW of average X-ray power at 10 Hz and its coherent part "
                 "near ten picowatts.",
                 "<strong>Microwatts of keV X-rays, incoherent.</strong> 8.7 × 10⁸ photons per shot at a mean energy of 2.48 keV, "
                 "ten times a second, give 3.5 µW; the K = 8.3 wiggler regime leaves a coherent fraction of 3.5 × 10⁻⁶. The preset "
                 "is illustrative within published ranges, not a specific machine; it feeds LIGA as a synchrotron-like spectrum, not "
                 "projection imaging. "
                 "<em>Model: Laser-wakefield betatron X-rays 🧪.</em>"),
    "sxrl": ("sources/soft-xray-laser.md",
             "Log-power waterfall for four plasma soft-X-ray-laser presets from peak power to average power: megawatt-class "
             "pulses become 1 µW to 0.16 mW averages at 10 to 100 Hz.",
             "<strong>Peak power is not average power.</strong> Pulse energy ÷ duration gives the kW–MW peaks; pulse energy × repetition "
             "rate the averages, which reproduce the reported orders of magnitude (the Ar and Mo pulse–rate splits are published; the Ag "
             "and Cd splits and all durations are assumptions). "
             "<em>Model: Plasma soft-X-ray laser ✅ (🔶 pulse splits, linewidth, no gain model).</em>"),
    "noon": ("sources/entangled-photon.md",
             "Log-power bars for the entangled NOON source: 2.5 picowatts of photon power at a million pairs per second, the "
             "25 kW-equivalent pair rate a one-second exposure would need at the cross-section bound, and a 40 W F₂ laser for scale.",
             "<strong>Picowatts against a 40 W laser.</strong> A million pairs per second of N = 2 photons at 157.63 nm is 2.5 pW; "
             "clearing 1 cm² in one second at the loosest independent ETPA cross-section bound (10⁻²³ cm²) would need 10²² pairs per "
             "second, drawn hollow because no such source exists. "
             "<em>Model: Entangled-photon (NOON) source 🧪.</em>"),
    "synchrotron": ("sources/synchrotron.md",
                    "Log-power waterfall for the compact EUV undulator from 21 W of total radiated power to 0.23 W in the central-cone "
                    "line and 21 mW of coherent power, with the LIGA bending magnet's 20 W per mrad of white beam and 8 mW per mrad "
                    "after the monochromator below it.",
                    "<strong>A storage ring spends its watts across harmonics, angles and modes.</strong> The compact undulator preset "
                    "radiates 21 W in all; 1.1 % sits in the natural line of the first harmonic and the matched-beta coherent fraction "
                    "(an upper bound) keeps 21 mW; the diamond is PSI's XIL-II beamline, ≈ 11 mW delivered to a 5 × 5 mm² sample. "
                    "The bending-magnet rows are per mrad of horizontal acceptance on a KARA-class ring. "
                    "<em>Model: Synchrotron (bending magnet / undulator) ✅ (🔶 coherence bound).</em>"),
    "xfel": ("sources/xfel.md",
             "Log-power waterfall for four FEL presets from average electron-beam power through FEL output to the wafer: FLASH-class "
             "0.1 W and FERMI-class 1 mW are measured-class values, the CW-SC 135 W and ERL 10.5 kW are design projections.",
             "<strong>The beam power that a kilowatt FEL would need.</strong> FEL efficiency ≈ ρ at saturation turns 51 W of average "
             "beam power into FLASH's 0.1 W and would turn 7.8 MW into the ERL design's 10.5 kW; the CW-SC and ERL presets are "
             "projections drawn hollow, their machine parameters illustrative of each class, not specifications. "
             "<em>Model: XFEL (SASE / seeded) ✅ (🧪 CW-SC and ERL presets) · Dose-limited wafer throughput 🔶.</em>"),
    "lpa-fel": ("sources/lpa-fel.md",
                "Log-power bars for the 25 nm projection and the 420 nm BELLA-class LPA-FEL: average electron-beam power, the "
                "Ming-Xie saturated FEL average power and the stored average power; the 25 nm beam's 1 % energy spread would "
                "saturate at 17 µW average while the stored 5 mW is an assumption.",
                "<strong>The gain physics and the stored number disagree at 25 nm.</strong> With σ_δ = 1.9 ρ the Ming-Xie fit gives "
                "Λ = 49 (no practical gain): saturation would come at 1.7 MW, i.e. 17 µW average at 10 fs × 1 kHz, while the preset "
                "stores 5 µJ × 1 kHz = 5 mW by assumption; the 420 nm beam (Λ = 6) is marginal but lasing-capable, consistent with "
                "BELLA's demonstration. Both beams are illustrative LPA-class values, not BELLA's. "
                "<em>Model: LPA-FEL 🔶.</em>"),
    "duv": ("sources/duv-heritage.md",
            "Log-power bars for the ArF and KrF excimer presets: 90 W and 40 W of laser output become 24 W and 11 W on the wafer "
            "through the refractive column's transmission.",
            "<strong>Representative excimer outputs, one transmission factor.</strong> Pulse energy × repetition rate gives the laser "
            "output; the refractive column (optics 0.3 × mask 0.9, an assumption) the power on the wafer. The Hg g/h/i lamps carry no "
            "power model and are not drawn. "
            "<em>Model: DUV/UV heritage ✅ (🔶 parameters) · Dose-limited wafer throughput 🔶.</em>"),
    "vuv": ("sources/vuv-excimer.md",
            "Log-power bars for the F₂ preset and the hypothetical Ar₂ preset: 40 W and 5 W of laser output become 11 W and 1.4 W on "
            "the wafer through the refractive column's transmission.",
            "<strong>The F₂ chain, and a laser that does not exist.</strong> Pulse energy × repetition rate gives the output; the "
            "refractive column (optics 0.3 × mask 0.9, an assumption) the power on the wafer. The Ar₂ preset is hollow: a "
            "line-narrowed Ar₂ laser has never been built. "
            "<em>Model: VUV excimer ✅ (🧪 Ar₂) · Dose-limited wafer throughput 🔶.</em>"),
}

_PANEL_ROWS = {
    "lpp": {"Laser-produced plasma": ROW_LPP, "Dose-limited wafer throughput": ROW_TP},
    "dpp": {"Discharge-produced plasma": ROW_DPP},
    "hhg": {}, "ics": {}, "ssmb": {}, "smith-purcell": {}, "betatron": {}, "noon": {},
    "xray-tube": {"Hard X-ray tube": ROW_TUBE},
    "sxrl": {"Plasma soft-X-ray laser": ROW_SXRL},
    "synchrotron": {},
    "xfel": {},
    "lpa-fel": {},
    "duv": {"DUV/UV heritage": ROW_DUV},
    "vuv": {"VUV excimer": ROW_VUV},
}

for _key, (_page, _alt, _caption) in _PANEL_PAGES.items():
    register(FigureSpec(
        name=f"sources-power-chain-{_key}",
        group="sources",
        compute=_panel_compute(_key),
        draw=_panel_draw,
        alt=_alt,
        caption=_caption,
        pages=(_page,),
        extra={"model_rows": _PANEL_ROWS[_key]},
    ))


# ---- the composite (sources/index.md) -------------------------------------------------------------

_COMPOSITE_ORDER = (
    ("duv", "vuv", "lpp", "dpp", "synchrotron", "xfel", "lpa-fel"),
    ("ssmb", "ics", "smith-purcell", "hhg", "sxrl", "xray-tube", "betatron", "noon"),
)


def composite_compute(fast: bool):  # noqa: ARG001
    panels = _all_chains()
    assert set(panels) == {k for col in _COMPOSITE_ORDER for k in col}, "composite order ≠ panel set"
    return panels


def composite_draw(panels: dict, th: S.Theme):
    title_in, gap_in = 0.34, 0.16
    heights = {k: title_in + len(p["rows"]) * _row_pitch(len(p["chains"]), True) for k, p in panels.items()}
    col_h = [sum(heights[k] + gap_in for k in col) for col in _COMPOSITE_ORDER]
    top_in, bottom_in = 0.25, 0.95
    fig_h = max(col_h) + top_in + bottom_in
    fig_w = S.WIDE_WIDTH + 0.8
    fig = plt.figure(figsize=(fig_w, fig_h))
    left = (0.165, 0.665)
    width = 0.30
    xlim = (1e-14, 1e8)
    for c, col in enumerate(_COMPOSITE_ORDER):
        y = fig_h - top_in
        for k in col:
            h = heights[k]
            y -= h
            ax = fig.add_axes([left[c], y / fig_h, width, (h - title_in) / fig_h])
            draw_waterfall(ax, panels[k], th, compact=True, xlim=xlim)
            ax.set_xticks([1e-12, 1e-9, 1e-6, 1e-3, 1, 1e3, 1e6])
            ax.tick_params(axis="x", labelsize=S.SMALL_PT - 1.5)
            if k == col[-1]:
                ax.set_xlabel("power (W); the plane of each row is on the family page", fontsize=S.SMALL_PT - 1)
            y -= gap_in
    handles, labels = _prov_handles(th)
    fig.legend(handles, labels, loc="lower center", ncol=3, fontsize=S.SMALL_PT - 1, bbox_to_anchor=(0.5, 0.004),
               handlelength=1.4, columnspacing=1.2)
    fig.text(0.5, (bottom_in - 0.35) / fig_h, "shaded band: HVM need at intermediate focus, 250 W → 1 kW (13.5 nm families); "
             "chips: the family's capability-matrix grade", ha="center", fontsize=S.SMALL_PT - 1, color=th.ink2)
    return fig


register(FigureSpec(
    name="sources-power-chain",
    group="sources",
    kind="sheet",
    compute=composite_compute,
    draw=composite_draw,
    alt="Fifteen small log-power waterfalls, one per source family, from machine input to the power the code reports, with "
        "the 250 W to 1 kW band behind the 13.5 nm families: only tin LPP reaches it, accelerator design points are hollow "
        "projections, and table-top sources sit at microwatts to picowatts.",
    caption="<strong>Every source family's watts, from the machine to the plane the code reports.</strong> One waterfall per "
            "family, one bar per multiplicative factor of its <code>derived_quantities()</code>, filled where a value is anchored "
            "to a demonstrated machine, ringed where calibrated, hatched where assumed and hollow where projected; the family pages "
            "carry the same chain with every factor and plane labelled. Powers are stated at different planes (laser output, "
            "into 2π sr, at intermediate focus, on the wafer), so compare orders of magnitude, not percent. "
            "<em>Chips copied from the matrix.</em>",
    pages=("sources/index.md",),
    extra={"png_palette": 64},
))


# ---- site-watts-waterfall: the IF → wafer → wafers-per-hour stage -------------------------------------


def watts_compute(fast: bool):  # noqa: ARG001
    huv = native()
    S_ = huv.SourceConfig
    src = S_.lpp_sn_13nm5()
    r = src.wafer_throughput(TP_DOSE)
    assert r is not None
    p_if = float(r["source_power_w"])
    t_opt, t_mask = float(r["optics_transmission"]), float(r["mask_efficiency"])
    p_w = float(r["power_at_wafer_w"])
    assert abs(p_if * t_opt * t_mask - p_w) < 1e-9 * p_w, "power at wafer ≠ IF × optics × mask"
    assert abs(t_opt - 0.70 ** 10) < 1e-9, "the EUV preset is ten mirrors at 0.70"
    area = float(r["exposed_area_cm2"])
    energy_j = TP_DOSE * 1e-3 * area
    t_exp, t_tot = float(r["exposure_time_per_wafer_s"]), float(r["total_time_per_wafer_s"])
    overhead = t_tot - t_exp
    fields = int(r["fields_per_wafer"])
    wph = float(r["wafers_per_hour"])
    assert abs(3600.0 / t_tot - wph) < 1e-6 * wph
    # the same chain for the stated 1 kW target: the model's factors applied to 1000 W, checked against the engine
    p1k = HVM_BAND_W[1]
    chain1k = dict(p_if=p1k, p_w=p1k * t_opt * t_mask, t_exp=t_exp * p_if / p1k)
    chain1k["t_tot"] = chain1k["t_exp"] + overhead
    chain1k["wph"] = 3600.0 / chain1k["t_tot"]
    probe = S_.ssmb(average_power_w=p1k).wafer_throughput(TP_DOSE)
    assert probe is not None and abs(probe["wafers_per_hour"] - chain1k["wph"]) < 1e-6 * chain1k["wph"], \
        "the 1 kW arithmetic differs from the engine's wafer_throughput at 1 kW"
    assert abs(probe["power_at_wafer_w"] - chain1k["p_w"]) < 1e-9 * chain1k["p_w"]
    return {
        "p_if": p_if, "p_mirrors": p_if * t_opt, "p_w": p_w, "t_opt": t_opt, "t_mask": t_mask, "area": area,
        "energy_j": energy_j, "t_exp": t_exp, "t_field": 0.1 * fields, "t_wafer": overhead - 0.1 * fields, "t_tot": t_tot,
        "wph": wph, "fields": fields, "dose": TP_DOSE, "scan_mm_s": float(r["dose_limited_scan_speed_mm_s"]),
        "k": chain1k,
    }


def watts_draw(d, th: S.Theme):
    fig = plt.figure(figsize=(S.FULL_WIDTH, 4.6))
    ax = fig.add_axes([0.22, 0.30, 0.42, 0.60])
    bx = fig.add_axes([0.70, 0.30, 0.27, 0.60])
    # ---- left: the power waterfall, 250 W filled, 1 kW hollow
    S.style_axes(ax, th, grid="x")
    ax.set_xscale("log")
    rows = [("in-band power at IF", "what every datasheet quotes"), ("after ten mirrors", "× 0.70¹⁰ (assumed)"),
            ("on the wafer", "× 0.65 reflective mask (assumed)")]
    base, hi = 0.3, 1e5
    ax.set_xlim(base, hi)
    ax.set_ylim(-2.62, 0.85)
    ax.axvspan(*HVM_BAND_W, color=th.band, zorder=0, lw=0)
    ax.text(math.sqrt(HVM_BAND_W[0] * HVM_BAND_W[1]), 0.82, "HVM need\n250 W → 1 kW", ha="center", va="top", fontsize=S.SMALL_PT - 1, color=th.ink2)
    chains = [("NXE:3400B Sn LPP", 1, "measured", [d["p_if"], d["p_mirrors"], d["p_w"]]),
              ("1 kW target", 2, "projection", [d["k"]["p_if"], d["k"]["p_if"] * d["t_opt"], d["k"]["p_w"]])]
    pitch, hbar = 0.34, 0.27
    for i, (label, plane) in enumerate(rows):
        items = []
        for j, (name, slot, prov, vals) in enumerate(chains):
            yc = -i - (j - 0.5) * pitch
            st = S.provenance_style(prov if i == 0 else ("assumed" if j == 0 else "projection"), th.s(slot), th)
            if prov == "measured" and i == 0:
                st.update(edgecolor=th.s(slot))
            ax.add_patch(Rectangle((base, yc - hbar / 2), vals[i] - base, hbar, zorder=3, **st))
            items.append((vals[i], yc, _fmt_w(vals[i]) + (f"  {name}" if i == 0 else "")))
        S.end_labels(ax, items, th, min_gap_px=10, dx_px=4, fontsize=S.SMALL_PT - 0.5)
        ax.text(0.0, -i - 0.30, plane, transform=ax.get_yaxis_transform(), ha="right", va="center", fontsize=S.SMALL_PT - 1.5,
                color=th.muted, clip_on=False)
    ax.set_yticks([0, -1, -2])
    ax.set_yticklabels([r[0] for r in rows], fontsize=S.SMALL_PT)
    for lab in ax.get_yticklabels():
        lab.set_va("bottom")
        lab.set_transform(S._offset(ax, 0, 3.5, lab.get_transform()))
    ax.tick_params(axis="y", length=0)
    ax.set_xlabel("in-band power (W)", fontsize=S.SMALL_PT)
    ax.set_title("From intermediate focus to the wafer", fontsize=S.TITLE_PT)
    # ---- right: the time budget per wafer → wafers per hour
    S.style_axes(bx, th, grid="x")
    parts = [("exposure", th.s(1)), ("field overheads", th.axis), ("wafer overhead", th.grid)]
    for j, (name, vals, hollow) in enumerate((("250 W", (d["t_exp"], d["t_field"], d["t_wafer"]), False),
                                               ("1 kW", (d["k"]["t_exp"], d["t_field"], d["t_wafer"]), True))):
        y = -j
        x0 = 0.0
        for (pname, colour), v in zip(parts, vals):
            if hollow and pname == "exposure":
                st = dict(facecolor="none", edgecolor=th.s(2), linewidth=1.6 * 72 / S.LOGICAL_DPI, linestyle=(0, (4, 3)))
            elif pname == "exposure":
                st = dict(facecolor=colour, edgecolor=th.surface, linewidth=S.RING_PT)
            else:
                st = dict(facecolor=colour, edgecolor=th.surface, linewidth=S.RING_PT)
            bx.add_patch(Rectangle((x0, y - 0.27), v, 0.54, zorder=3, **st))
            x0 += v
        total = sum(vals)
        wph = 3600.0 / total
        bx.annotate(f"{total:.1f} s → {wph:.0f} wph", (total, y), xytext=(4, 0), textcoords="offset points", va="center",
                    fontsize=S.SMALL_PT - 0.5, color=th.ink2, annotation_clip=False)
        bx.text(-0.4, y, name, ha="right", va="center", fontsize=S.SMALL_PT, color=th.ink2)
    bx.set_xlim(0, 32)
    bx.set_ylim(-1.62, 1.05)
    bx.set_yticks([])
    bx.spines["left"].set_visible(False)
    bx.set_xlabel("seconds per wafer", fontsize=S.SMALL_PT)
    bx.set_title("Time per wafer at 30 mJ/cm²", fontsize=S.TITLE_PT)
    bx.legend([Patch(facecolor=c, edgecolor="none") for _n, c in parts], [p[0] for p in parts], loc="upper center",
              bbox_to_anchor=(0.5, 1.0), ncol=3, fontsize=S.SMALL_PT - 1.5, frameon=False, handlelength=1.2, columnspacing=0.8)
    lines = [
        f"{d['dose']:.0f} mJ/cm² × {d['area']:.0f} cm² ({d['fields']} fields of 26 × 33 mm) = {d['energy_j']:.1f} J per wafer; at {_fmt_w(d['p_w'])} the "
        f"dose-limited scan runs at {d['scan_mm_s']:.0f} mm/s and takes {d['t_exp']:.1f} s (2 mm slit overscan included);",
        f"+ {d['t_field']:.1f} s of field and {d['t_wafer']:.0f} s of wafer overheads (illustrative, not vendor data) → {d['wph']:.0f} wph. "
        f"A 1 kW source through the same factors scans in {d['k']['t_exp']:.2f} s → {d['k']['wph']:.0f} wph:",
        "overheads, not watts, cap the model near 196 wph.",
    ]
    fig.text(0.22, 0.115, "\n".join(lines), fontsize=S.SMALL_PT - 1, color=th.ink2, va="top", linespacing=1.3)
    return fig


register(FigureSpec(
    name="site-watts-waterfall",
    group="sources",
    compute=watts_compute,
    draw=watts_draw,
    alt="Log-power waterfall from 250 W at intermediate focus through ten mirrors at 0.70 and a 0.65 mask to 4.6 W on the "
        "wafer, the 1 kW target drawn hollow beside it, and the time budget per wafer at 30 mJ/cm²: 5.0 s of exposure plus "
        "18.4 s of overheads give 154 wafers per hour, 183 at 1 kW.",
    caption="<strong>From the source to wafers per hour.</strong> 250 W at intermediate focus × 0.70¹⁰ × 0.65 leaves 4.6 W on the "
            "wafer; a 300 mm wafer at 30 mJ/cm² needs 21.6 J, so the dose-limited scan of 84 fields takes 5.0 s, and the model's "
            "0.1 s per field and 10 s per wafer overheads make it 154 wafers per hour. Model assumptions, not a vendor optical budget; "
            "1 kW is the stated target, drawn hollow: it buys 183 wph, because the overheads cap the model near 196. "
            "<em>Model: Dose-limited wafer throughput 🔶.</em>",
    pages=("future/accelerator-light-sources.md", "future/stochastic-frontier.md"),
))


# ===========================================================================
# S2. Coherence → pupil fill
# ===========================================================================

SIGMA_CORE = 0.05
COH_NA = 0.33
COH_LAM = 13.5


def _bridge(zeta: np.ndarray) -> np.ndarray:
    """``sigma_from_coherence``: clamp(σ_core / √ζ, σ_core, 1); ζ ≤ 0 → 1."""
    z = np.asarray(zeta, dtype=float)
    out = np.where(z > 0, np.clip(SIGMA_CORE / np.sqrt(np.where(z > 0, z, 1.0)), SIGMA_CORE, 1.0), 1.0)
    return out


def coherence_compute(fast: bool):
    huv = native()
    S_ = huv.SourceConfig
    # (a) the bridge and the presets that use it
    coherent = (("undulator", S_.synchrotron_compact_euv()), ("HHG Ne", S_.hhg_ne_13nm5()), ("XFEL FLASH", S_.xfel_flash_13nm5()),
                ("ICS", S_.ics_compact_euv_13nm5()), ("SSMB", S_.ssmb_euv_13nm5()), ("SXRL Ag", S_.sxrl_ag_13nm9()))
    points = []
    for label, src in coherent:
        zeta = src.transverse_coherence_fraction
        shape, sigma = src.illumination[0], float(src.illumination[1])
        assert shape == "coherent_gaussian", f"{label}: expected a coherent_gaussian pupil, got {shape}"
        assert abs(float(_bridge(np.array([zeta]))[0]) - sigma) < 1e-9, f"{label}: bridge σ ≠ the preset's pupil σ"
        points.append({"label": label, "zeta": float(zeta), "sigma": sigma})
    tophat = []
    for label, src in (("F$_2$ excimer", S_.f2_laser()), ("Sn LPP", S_.lpp_sn_13nm5())):
        shape, sigma = src.illumination[0], float(src.illumination[1])
        assert shape == "conventional", f"{label}: expected a conventional pupil"
        tophat.append({"label": label, "sigma": sigma})
    # (b) the fills the engine receives
    n = 61 if fast else 101
    ext = 1.0
    fills = []
    for label, src in (("F$_2$ excimer", S_.f2_laser()), ("Sn LPP", S_.lpp_sn_13nm5()), ("undulator", S_.synchrotron_compact_euv()),
                       ("HHG Ne", S_.hhg_ne_13nm5()), ("XFEL FLASH", S_.xfel_flash_13nm5())):
        fills.append({"label": label, "fill": np.asarray(src.pupil_fill(n, ext)), "sigma": float(src.illumination[1])})
    # (c) contrast vs pitch at 13.5 nm, NA 0.33, for a σ 0.05 Gaussian fill and conventional σ 0.7 / 0.9
    optics = huv.OpticsConfig(numerical_aperture=COH_NA, flare_fraction=0.0)
    pitches = np.arange(20.0, 64.1, 3.0 if fast else 1.0)
    sources = {
        "Gaussian σ 0.05": S_.lpp_sn_13nm5().with_illumination("coherent_gaussian", SIGMA_CORE),
        "conventional σ 0.7": S_.lpp_sn_13nm5(0.7),
        "conventional σ 0.9": S_.lpp_sn_13nm5(0.9),
    }
    curves = {}
    for label, src in sources.items():
        vals = []
        for p in pitches:
            mask = huv.MaskConfig.line_space(p / 2, p)
            grid = mask.commensurate_grid(128, 0.5)
            eng = huv.SimulationEngine(src, optics, mask, grid=grid, max_kernels=64)
            vals.append(eng.compute_aerial_image(0.0).image_contrast())
        curves[label] = np.array(vals)
    coh_cut = COH_LAM / COH_NA
    # physics: the σ 0.9 fill images past the σ 0.7 cutoff, and nothing images below λ/(NA(1+σ))
    c07, c09 = curves["conventional σ 0.7"], curves["conventional σ 0.9"]
    cut07, cut09 = COH_LAM / (COH_NA * 1.7), COH_LAM / (COH_NA * 1.9)
    assert c09[(pitches > cut09 + 1.0) & (pitches < cut07 - 0.5)].max() > 0.05, "σ 0.9 should still image between its cutoff and σ 0.7's"
    assert c07[pitches < cut07 - 1.0].max() < 0.05, "no contrast below λ/(NA(1+σ)) for σ 0.7"
    return {"points": points, "tophat": tophat, "fills": fills, "ext": ext, "pitches": pitches, "curves": curves,
            "coh_cut": coh_cut, "cut07": cut07, "cut09": cut09}


def coherence_draw(d, th: S.Theme):
    fig = plt.figure(figsize=(S.WIDE_WIDTH, 6.3))
    ax = fig.add_axes([0.07, 0.52, 0.40, 0.40])
    cx = fig.add_axes([0.57, 0.52, 0.40, 0.40])
    # (a) the bridge
    S.style_axes(ax, th, grid="both")
    ax.set_xscale("log")
    z = np.logspace(-3, 0, 300)
    ax.plot(z, _bridge(z), color=th.s(1), zorder=3)
    S.limit_line(ax, SIGMA_CORE, th, f"σ$_{{core}}$ = {SIGMA_CORE}", axis="y", where=0.02, side="right", va="bottom")
    lines, groups = [], []
    for k, p in enumerate(sorted(d["points"], key=lambda q: q["zeta"]), start=1):
        S.dot(ax, [p["zeta"]], [p["sigma"]], th.s(2), th, size_px=7.5, zorder=4)
        lines.append(f"{k}  {p['label']}: ζ {p['zeta']:.2g} → σ {p['sigma']:.3f}")
        if groups and math.log10(p["zeta"] / groups[-1][-1][1]) < 0.09:  # crowded points share one number label
            groups[-1].append((k, p["zeta"], p["sigma"]))
        else:
            groups.append([(k, p["zeta"], p["sigma"])])
    for g in groups:
        text = str(g[0][0]) if len(g) == 1 else f"{g[0][0]}–{g[-1][0]}"
        zx = math.sqrt(g[0][1] * g[-1][1])
        ax.annotate(text, (zx, max(q[2] for q in g)), xytext=(0, 8), textcoords="offset points", ha="center", va="bottom",
                    fontsize=S.SMALL_PT - 1, color=th.ink2)
    ax.text(0.03, 0.62, "\n".join(lines), fontsize=S.SMALL_PT - 1, color=th.ink2, va="top", ha="left", linespacing=1.35)
    for k, t in enumerate(d["tophat"]):
        ax.axhline(t["sigma"], color=th.s(3), linewidth=S.LINE_PT * 0.8, zorder=2)
        ax.text(1.1e-3, t["sigma"] + 0.02, f"{t['label']}: top-hat σ {t['sigma']:.1f} (incoherent plasma or laser; no ζ)", fontsize=S.SMALL_PT - 1,
                color=th.ink2, va="bottom")
    ax.set_xlim(1e-3, 1.0)
    ax.set_ylim(0, 1.0)
    ax.set_xlabel("transverse coherence fraction ζ")
    ax.set_ylabel("pupil σ handed to the engine")
    ax.set_title("(a) σ = clamp(σ$_{core}$/√ζ, σ$_{core}$, 1), a bookkeeping bridge")
    # (c) contrast vs pitch
    S.style_axes(cx, th)
    for slot, (label, curve) in enumerate(d["curves"].items(), start=1):
        cx.plot(d["pitches"], curve, color=th.s(slot), zorder=3, label=label)
    S.limit_line(cx, d["coh_cut"], th, "λ/NA", where=0.45, side="right", va="top")
    S.limit_line(cx, d["cut07"], th, "λ/(NA·1.7)", where=0.97, side="left", va="top")
    S.limit_line(cx, d["cut09"], th, "λ/(NA·1.9)", where=0.60, side="left", va="top")
    cx.set_xlim(d["pitches"][0], d["pitches"][-1])
    cx.set_ylim(0, 1.0)
    cx.set_xlabel("pitch of 1:1 lines (nm)")
    cx.set_ylabel("aerial-image contrast")
    cx.set_title("(c) Contrast against pitch at 13.5 nm, NA 0.33")
    cx.legend(loc="lower right", fontsize=S.SMALL_PT - 0.5)
    # (b) the fills
    cmap = S.field_cmap()
    n = len(d["fills"])
    w = 0.86 / n
    for k, f in enumerate(d["fills"]):
        bx = fig.add_axes([0.07 + k * (w + 0.015), 0.06, w, 0.34])
        e = d["ext"]
        bx.imshow(f["fill"], origin="lower", extent=[-e, e, -e, e], cmap=cmap, vmin=0, vmax=max(float(np.max(f["fill"])), 1e-12),
                  interpolation="bilinear")
        bx.add_patch(plt.Circle((0, 0), 1.0, fill=False, edgecolor=th.axis, linewidth=S.HAIR_PT * 1.3))
        bx.set_xticks([])
        bx.set_yticks([])
        for side in ("top", "right", "left", "bottom"):
            bx.spines[side].set_visible(False)
        bx.set_title(f"{f['label']}\nσ = {f['sigma']:.3f}", fontsize=S.SMALL_PT, pad=4)
        if k == 0:
            bx.set_ylabel("(b) pupil_fill(): circle = NA", fontsize=S.SMALL_PT)
    return fig


register(FigureSpec(
    name="sources-coherence-to-pupil",
    group="sources",
    compute=coherence_compute,
    draw=coherence_draw,
    alt="The coherence-to-sigma bridge curve with six coherent presets on it, five pupil-fill maps from a wide excimer disk to "
        "the near-coherent FEL core, and contrast versus pitch at 13.5 nm and NA 0.33, where the sigma 0.05 core stops at "
        "lambda over NA and the sigma 0.9 disk images down to 21.5 nm.",
    caption="<strong>From coherence to pupil fill.</strong> (a) The Gaussian–Schell mode-count heuristic σ = σ_core/√ζ — a bookkeeping "
            "bridge, not a coherent-mode decomposition — with every preset that uses it at its derived or stored ζ; (b) the fills the "
            "engine receives, from the F₂ and tin top-hats to the near-coherent cores; (c) what they do to imaging at 13.5 nm, "
            "NA 0.33: the coherent core cuts off at λ/NA, the disks at λ/(NA(1+σ)). "
            "<em>Model: Illumination pupil fills ✅ · Synchrotron (bending magnet / undulator) ✅ (🔶 coherence bound) · "
            "High-harmonic generation ✅ (🔶 intensities) · XFEL (SASE / seeded) ✅ (🧪 CW-SC and ERL presets) · "
            "Inverse Compton scattering 🧪 · SSMB storage ring 🧪 · Hopkins imaging (factorized TCC/SOCS) ✅.</em>",
    pages=("sources/index.md",),
    requires=("spectrum", "pupil_fill"),
))


# ===========================================================================
# S3a. Synchrotron physics
# ===========================================================================


def synchrotron_compute(fast: bool):
    huv = native()
    S_ = huv.SourceConfig
    und = S_.synchrotron_compact_euv()
    dq = _dq(und)
    lam0 = float(und.wavelength_nm)
    fwhm_pm = float(und.bandwidth_pm)
    n_periods = 100
    assert abs(fwhm_pm - 0.8859 * lam0 * 1e3 / n_periods) < 1e-3 * fwhm_pm, "sinc² FWHM = 0.8859 λ/(nN)"
    samples = np.array(und.spectrum())
    # (b) F_n(K), Q_n(K) from the engine for n = 1, 3, 5
    ks = np.linspace(0.1, 3.0, 15 if fast else 59)
    harmonics = {}
    for n in (1, 3, 5):
        fn, qn = [], []
        for k in ks:
            d_ = _dq(S_.synchrotron_undulator(k=float(k), harmonic=n))
            fn.append(d_["harmonic_factor_fn"])
            qn.append(d_["harmonic_factor_qn"])
        harmonics[n] = (np.array(fn), np.array(qn))
    assert abs(harmonics[1][0][np.argmin(abs(ks - 1.0))] - dq["harmonic_factor_fn"]) < 1e-6 or fast
    # (c) ζ per plane vs emittance, and the compact-ring point
    eps = np.logspace(-2, 2, 12 if fast else 41)
    zeta_plane = np.array([_dq(S_.synchrotron_undulator(emittance_x_nm_rad=float(e), emittance_y_nm_rad=float(e)))["coherent_fraction_x"]
                           for e in eps])
    ring = {"ex": 10.0, "ey": 0.1, "zx": dq["coherent_fraction_x"], "zy": dq["coherent_fraction_y"], "z": dq["coherent_fraction"]}
    assert abs(ring["zx"] * ring["zy"] - ring["z"]) < 1e-9
    photon_emit = lam0 / (4 * math.pi)
    assert abs(zeta_plane[np.argmin(abs(eps - 1.0))] - photon_emit / (1.0 + photon_emit)) < 2e-2, "ζ(ε) = (λ/4π)/(ε + λ/4π)"
    # (d) the bending-magnet probe decides the panel: xray_spectrum() → S(E/E_c); None → central-cone flux per harmonic
    bm = S_.synchrotron_liga_bending_magnet()
    spec = bm.xray_spectrum(50 if fast else 200)
    assert spec is None, ("synchrotron_liga_bending_magnet().xray_spectrum() now returns data: draw the S(E/E_c) panel and "
                          "update the caption, which says the harmonic fallback ran")
    harm = []
    for n in (1, 3, 5, 7, 9):
        s = S_.synchrotron_undulator(harmonic=n)
        d_ = _dq(s)
        harm.append((n, float(s.wavelength_nm), d_["central_cone_flux"], d_["central_cone_power"]))
    assert abs(harm[1][2] / harm[0][2] - dq["harmonic3_over_1"]) < 1e-9
    return {"lam0": lam0, "fwhm_pm": fwhm_pm, "samples": samples, "ks": ks, "harmonics": harmonics, "eps": eps, "zeta": zeta_plane,
            "ring": ring, "photon_emit": photon_emit, "harm": harm, "ec_kev": _dq(bm)["critical_energy"], "n_periods": n_periods,
            "line_power": dq["central_cone_power"], "total_power": dq["total_undulator_power"]}


def synchrotron_draw(d, th: S.Theme):
    fig, axes = plt.subplots(2, 2, figsize=(S.WIDE_WIDTH, 7.0), gridspec_kw={"hspace": 0.55, "wspace": 0.32})
    (ax, bx), (cx, dx) = axes
    # (a) the sampled line over the analytic sinc²
    S.style_axes(ax, th)
    dl = np.linspace(-300, 300, 1201)  # pm
    x = d["n_periods"] * dl * 1e-3 / d["lam0"]
    sinc2 = np.sinc(x) ** 2
    gauss = np.exp(-4 * math.log(2) * (dl / (d["lam0"] * 1e3 / d["n_periods"])) ** 2)
    ax.plot(dl, sinc2, color=th.s(1), zorder=3, label=f"sinc²: FWHM {d['fwhm_pm']:.2f} pm")
    ax.plot(dl, gauss, color=th.ink2, linewidth=S.HAIR_PT * 1.3, linestyle=(0, (4, 3)), zorder=2, label="former Gaussian, FWHM λ/(nN) = 135 pm")
    w = d["samples"]
    ax.vlines((w[:, 0] - d["lam0"]) * 1e3, 0, w[:, 1] / w[:, 1].max(), color=th.s(2), linewidth=S.LINE_PT * 1.2, zorder=4,
              label="the five samples spectrum() hands the engine")
    outer = sorted(w[:, 1] / w[:, 1].sum())[:-1]
    ax.text(0.02, 0.34, f"the four outer samples sit at ±1.25 and\n±2.5 FWHM and carry {100 * min(outer):.1f}–{100 * max(outer):.1f} % each",
            transform=ax.transAxes, fontsize=S.SMALL_PT - 1.5, color=th.muted, va="top")
    ax.set_xlim(-300, 300)
    ax.set_ylim(0, 1.08)
    ax.set_xlabel("λ − λ$_1$ (pm)")
    ax.set_ylabel("relative intensity")
    ax.set_title(f"(a) The natural line at λ$_1$ = {d['lam0']:.3f} nm, N = {d['n_periods']}")
    ax.legend(loc="upper right", fontsize=S.SMALL_PT - 1)
    # (b) F_n, Q_n
    S.style_axes(bx, th)
    for slot, n in enumerate((1, 3, 5), start=1):
        fn, qn = d["harmonics"][n]
        bx.plot(d["ks"], fn, color=th.s(slot), zorder=3, label=f"$F_{n}(K)$")
        bx.plot(d["ks"], qn, color=th.s(slot), linewidth=S.LINE_PT * 0.7, linestyle=(0, (2, 2)), zorder=3, label=f"$Q_{n}(K)$")
    S.limit_line(bx, 1.0, th, "K = 1 (preset)", where=0.03, side="right", va="bottom")
    bx.set_xlim(0, 3)
    bx.set_ylim(0, 1.0)
    bx.set_xlabel("undulator strength K")
    bx.set_ylabel("harmonic factor")
    bx.set_title("(b) $F_n(K)$ brightness and $Q_n(K)$ flux factors, from the engine")
    bx.legend(loc="upper left", ncol=2, fontsize=S.SMALL_PT - 1)
    # (c) ζ vs emittance
    S.style_axes(cx, th, grid="both")
    cx.set_xscale("log")
    cx.plot(d["eps"], d["zeta"], color=th.s(1), zorder=3, label="ζ per plane = (λ/4π)/(ε + λ/4π)")
    r = d["ring"]
    S.dot(cx, [r["ex"]], [r["zx"]], th.s(2), th, zorder=4)
    S.dot(cx, [r["ey"]], [r["zy"]], th.s(2), th, zorder=4)
    leader = dict(arrowstyle="-", color=th.axis, lw=S.HAIR_PT, shrinkA=2, shrinkB=4)
    cx.annotate(f"$ζ_x$ = {r['zx']:.3f} at $ε_x$ = {r['ex']:g} nm·rad", (r["ex"], r["zx"]), xytext=(0.013, 0.30), textcoords="data",
                fontsize=S.SMALL_PT, color=th.ink2, ha="left", va="center", arrowprops=leader)
    cx.annotate(f"$ζ_y$ = {r['zy']:.3f} at $ε_y$ = {r['ey']:g} nm·rad", (r["ey"], r["zy"]), xytext=(1.45, 0.955), textcoords="data",
                fontsize=S.SMALL_PT, color=th.ink2, ha="left", va="center", arrowprops=leader)
    cx.axhline(r["z"], color=th.s(2), linewidth=S.HAIR_PT * 1.3, linestyle=(0, (4, 3)), zorder=2)
    cx.text(0.012, r["z"] - 0.025, f"ζ = $ζ_x ζ_y$ = {r['z']:.4f} (compact ring, upper bound)", fontsize=S.SMALL_PT - 1, color=th.ink2, va="top")
    S.limit_line(cx, d["photon_emit"], th, f"λ/4π = {d['photon_emit']:.2f} nm·rad", where=0.98, side="right", va="top")
    cx.set_xlim(1e-2, 1e2)
    cx.set_ylim(0, 1.0)
    cx.set_xlabel("geometric emittance ε (nm·rad)")
    cx.set_ylabel("coherent fraction")
    cx.set_title(f"(c) Coherent fraction at {d['lam0']:.1f} nm, matched beta")
    # (d) central-cone flux per harmonic (the fallback panel)
    S.style_axes(dx, th, grid="y")
    dx.set_yscale("log")
    ns = [h[0] for h in d["harm"]]
    flux = np.array([h[2] for h in d["harm"]])
    dx.vlines(ns, flux.min() / 30, flux, color=th.s(1), linewidth=S.LINE_PT * 1.6, zorder=3)
    S.dot(dx, ns, flux, th.s(1), th, zorder=4)
    for n, lam, f, p in d["harm"]:
        dx.annotate(f"{lam:.2f} nm\n{f / flux[0]:.3g}", (n, f), xytext=(0, 9), textcoords="offset points", ha="center", va="bottom",
                    fontsize=S.SMALL_PT - 1.5, color=th.ink2)
    dx.set_xticks(ns)
    dx.set_xlim(0, 10)
    dx.set_ylim(flux.min() / 30, flux.max() * 40)
    dx.set_xlabel("harmonic n at K = 1")
    dx.set_ylabel("central-cone flux (photons/s/0.1 % BW)")
    dx.set_title("(d) Central-cone flux per harmonic, 200 mA (fallback panel)")
    dx.text(0.98, 0.97, f"bending magnet: xray_spectrum() returns None\nfor the LIGA preset ($E_c$ = {d['ec_kev']:.2f} keV), so this\npanel draws the undulator harmonics instead",
            transform=dx.transAxes, ha="right", va="top", fontsize=S.SMALL_PT - 1.5, color=th.muted)
    return fig


register(FigureSpec(
    name="sources-synchrotron-physics",
    group="sources",
    compute=synchrotron_compute,
    draw=synchrotron_draw,
    alt="Four synchrotron panels: the sampled weights on the sinc-squared undulator line, the harmonic factors F and Q "
        "against K for harmonics 1, 3 and 5, the matched-beta coherent fraction against emittance with the compact-ring point "
        "at 0.089, and the central-cone flux per odd harmonic at K = 1.",
    caption="<strong>What the undulator model derives from the machine.</strong> (a) The on-axis sinc² line of the compact preset "
            "(FWHM 119.65 pm, formerly a 135 pm Gaussian) and the five samples the engine images with; (b) F_n(K) and Q_n(K) "
            "for n = 1, 3, 5 with the preset's K = 1 marked; (c) the per-plane coherent fraction (λ/4π)/(ε + λ/4π), an upper bound "
            "with matched beta functions, giving ζ = 0.089 for the illustrative compact ring; (d) the bending-magnet "
            "<code>xray_spectrum()</code> probe returned no data for the LIGA preset, so the fallback panel shows the central-cone "
            "flux per odd harmonic at K = 1 (3rd and 5th at 16 % and 3 % of the fundamental). Flux values are filament-beam figures. "
            "<em>Model: Synchrotron (bending magnet / undulator) ✅ (🔶 coherence bound) · Machine-parameter physics layer ✅.</em>",
    pages=("sources/synchrotron.md",),
))


# ===========================================================================
# S3b. XFEL SASE statistics
# ===========================================================================


def _gamma_pdf(x: np.ndarray, m: float) -> np.ndarray:
    """Gamma(M) pulse-energy pdf in units of ⟨E⟩: M^M/Γ(M) x^(M−1) e^(−Mx)."""
    x = np.asarray(x, dtype=float)
    out = np.zeros_like(x)
    pos = x > 0
    out[pos] = np.exp(m * math.log(m) - math.lgamma(m) + (m - 1) * np.log(x[pos]) - m * x[pos])
    return out


def xfel_compute(fast: bool):
    huv = native()
    S_ = huv.SourceConfig
    presets = (("FLASH-class SASE", S_.xfel_flash_13nm5(), "sase"), ("CW-SC linac SASE (projection)", S_.xfel_cw_sc_13nm5(), "sase"),
               ("ERL SASE (projection)", S_.xfel_erl_13nm5(), "sase"), ("FERMI-class seeded", S_.xfel_fermi_seeded(), "seeded"))
    rows = []
    for label, src, mode in presets:
        dq = _dq(src)
        (m,) = _need(dq, label, "longitudinal_modes")
        rms = float(src.shot_to_shot_rms)
        if mode == "sase":
            assert abs(rms - 1 / math.sqrt(m)) < 1e-9, f"{label}: shot_to_shot_rms ≠ 1/√M"
        else:
            assert m == 1.0 and abs(rms - 0.02) < 1e-9, f"{label}: seeded preset should report M = 1 and the residual 2 %"
        tp = src.wafer_throughput(TP_DOSE)
        assert tp is not None
        rows.append({"label": label, "mode": mode, "m": float(m), "rms": rms, "ppp": float(tp["pulses_per_point"]),
                     "wph": float(tp["wafers_per_hour"])})
    x = np.linspace(0, 3.0, 61 if fast else 601)
    return {"rows": rows, "x": x, "n": np.logspace(0, 5, 200)}


def xfel_draw(d, th: S.Theme):
    fig, (ax, bx) = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 4.2), gridspec_kw={"wspace": 0.35})
    S.style_axes(ax, th)
    x = d["x"]
    for slot, r in enumerate(d["rows"], start=1):
        if r["mode"] == "sase":
            ax.plot(x, _gamma_pdf(x, r["m"]), color=th.s(slot), zorder=3,
                    label=f"{r['label']}: M = {r['m']:.2g}, rms = 1/√M = {r['rms'] * 100:.0f} %")
        else:
            ax.axvline(1.0, color=th.s(slot), linewidth=S.LINE_PT * 0.9, zorder=3,
                       label=f"{r['label']}: residual {r['rms'] * 100:.0f} % (a line at ⟨E⟩)")
    ax.set_xlim(0, 3)
    ax.set_ylim(0, 1.8)
    ax.set_xlabel("pulse energy E / ⟨E⟩")
    ax.set_ylabel("probability density")
    ax.set_title("(a) Gamma(M) pulse-energy statistics of the presets")
    ax.legend(loc="upper right", fontsize=S.SMALL_PT - 1)
    S.style_axes(bx, th, grid="both")
    bx.set_xscale("log")
    bx.set_yscale("log")
    n = d["n"]
    offsets = (9.0, -10.0, -10.0, 0.0)
    for slot, (r, dy) in enumerate(zip(d["rows"], offsets), start=1):
        bx.plot(n, r["rms"] / np.sqrt(n), color=th.s(slot), zorder=3)
        eff = r["rms"] / math.sqrt(r["ppp"])
        S.dot(bx, [r["ppp"]], [eff], th.s(slot), th, zorder=4, filled=("projection" not in r["label"]))
        S.end_label(bx, r["ppp"], eff, f"{r['ppp']:.0f} pulses → {eff * 100:.2g} %", th, dx_px=8, dy_px=dy, fontsize=S.SMALL_PT - 1)
    bx.set_xlim(1, 1e5)
    bx.set_ylim(1e-5, 1.0)
    bx.set_xlabel("pulses integrated per exposure point N")
    bx.set_ylabel("effective dose jitter σ/√N")
    bx.set_title("(b) Jitter averages down with the throughput model's pulses per point")
    return fig


register(FigureSpec(
    name="sources-xfel-sase-statistics",
    group="sources",
    compute=xfel_compute,
    draw=xfel_draw,
    alt="Gamma-distribution pulse-energy densities for the FLASH-class, CW-SC and ERL SASE presets with 3, 6 and 16 "
        "longitudinal modes and the seeded FERMI preset as a line, then the effective dose jitter falling as one over root N "
        "with each preset's pulses per exposure point marked between 6,000 and 42,000.",
    caption="<strong>SASE jitter, and how the scanner averages it away.</strong> (a) A pulse of M longitudinal modes has a "
            "Gamma(M) energy distribution with σ_E/⟨E⟩ = 1/√M — 57 % for the FLASH-class preset, 42 % and 25 % for the CW-SC and "
            "ERL projections; the seeded FERMI-class preset keeps a residual 2 %. (b) The throughput model's pulses per exposure "
            "point divide that jitter by √N to the 0.01–0.6 % level. These are the model's statistics, not measured FLASH shot "
            "statistics. <em>Model: XFEL (SASE / seeded) ✅ (🧪 CW-SC and ERL presets) · Dose-limited wafer throughput 🔶.</em>",
    pages=("sources/xfel.md",),
))


# ===========================================================================
# S3c. LPA-FEL: can it lase?
# ===========================================================================

LPA_UNDULATOR = dict(undulator_period_mm=20.0, num_periods=200, peak_current_a=1000.0, norm_emittance_um=0.5, beta_m=1.0)
LPA_MACHINES = (("25 nm / 500 MeV (projection)", 25.0, 500.0, 1.672, 1000.0, 5.0), ("420 nm / 100 MeV (BELLA class)", 420.0, 100.0, 1.118, 1.0, 1.0))


def lpa_compute(fast: bool):
    huv = native()
    S_ = huv.SourceConfig
    spreads = np.logspace(-4, math.log10(3e-2), 10 if fast else 37)
    machines = []
    for label, lam, e_mev, k, rep, euj in LPA_MACHINES:
        lsat, lam_mx, rho, lu = [], [], None, None
        for sd in spreads:
            src = S_.lpa_fel(lam, electron_energy_mev=e_mev, undulator_k=k, energy_spread_rel=float(sd), rep_rate_hz=rep,
                             pulse_energy_uj=euj, **LPA_UNDULATOR)
            dq = _dq(src)
            a, b, rho, lu = _need(dq, label, "saturation_length_3d", "ming_xie_lambda", "pierce_parameter_1d", "undulator_length")
            lsat.append(a)
            lam_mx.append(b)
        lsat, lam_mx = np.array(lsat), np.array(lam_mx)
        assert np.all(np.diff(lsat) > 0), f"{label}: L_sat,3D must grow with the energy spread"
        machines.append({"label": label, "lam": lam, "lsat": lsat, "lambda": lam_mx, "rho": rho, "lu": lu})
    preset = _dq(S_.lpa_fel_bella_25nm())
    pts = []
    for label, lam, e_mev, k, rep, euj, sd in ((f"1 % → {preset['saturation_length_3d']:.0f} m", 25.0, 500.0, 1.672, 1000.0, 5.0, 1e-2),
                                               ("0.1 %", 25.0, 500.0, 1.672, 1000.0, 5.0, 1e-3),
                                               ("420 nm at 1 %", 420.0, 100.0, 1.118, 1.0, 1.0, 1e-2)):
        dq = _dq(S_.lpa_fel(lam, electron_energy_mev=e_mev, undulator_k=k, energy_spread_rel=sd, rep_rate_hz=rep, pulse_energy_uj=euj,
                            **LPA_UNDULATOR))
        pts.append({"label": label if "→" in label else f"{label} → {dq['saturation_length_3d']:.1f} m", "sd": sd,
                    "lsat": dq["saturation_length_3d"], "lambda": dq["ming_xie_lambda"], "machine": 0 if lam == 25.0 else 1})
    assert abs(pts[0]["lsat"] - preset["saturation_length_3d"]) < 1e-3 * preset["saturation_length_3d"], \
        "the 25 nm sweep at 1 % does not reproduce the preset's saturation_length_3d"
    assert abs(preset["energy_spread_over_rho"] - 1e-2 / machines[0]["rho"]) < 1e-3 * preset["energy_spread_over_rho"], \
        "the sweep's ρ differs from the preset's"
    return {"spreads": spreads, "machines": machines, "points": pts, "preset_ratio": preset["energy_spread_over_rho"]}


def lpa_draw(d, th: S.Theme):
    fig, (ax, bx) = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 4.3), gridspec_kw={"wspace": 0.32})
    for axis in (ax, bx):
        S.style_axes(axis, th, grid="both")
        axis.set_xscale("log")
        axis.set_yscale("log")
        axis.set_xlim(1e-4, 3e-2)
        axis.set_xlabel("relative energy spread $σ_δ$")
    for slot, m in enumerate(d["machines"], start=1):
        tag = f"{m['label']}, ρ = {_sci(m['rho'])}"
        ax.plot(d["spreads"], m["lsat"], color=th.s(slot), zorder=3, label=tag)
        bx.plot(d["spreads"], m["lambda"], color=th.s(slot), zorder=3, label=tag)
        for axis in (ax, bx):
            S.limit_line(axis, m["rho"], th, f"$σ_δ$ = ρ ({m['lam']:g} nm)", where=0.97, side=("left" if slot == 1 else "right"),
                         va="top", color=th.s(slot))
    lu = d["machines"][0]["lu"]
    S.limit_line(ax, lu, th, f"{lu:g} m undulator", axis="y", where=0.02, side="right", va="top")
    S.limit_line(bx, 5.0, th, "Λ ≈ 5: no practical gain", axis="y", where=0.02, side="right", va="bottom")
    for p in d["points"]:
        colour = th.s(p["machine"] + 1)
        S.dot(ax, [p["sd"]], [p["lsat"]], colour, th, zorder=4, filled=(p["machine"] == 1))
        S.dot(bx, [p["sd"]], [p["lambda"]], colour, th, zorder=4, filled=(p["machine"] == 1))
        if p["machine"] == 1:  # the 420 nm point: label to the left, under the orange guide line
            S.end_label(ax, p["sd"], p["lsat"], "420 nm, 1 %: 11.3 m", th, dx_px=-9, dy_px=-11, ha="right", fontsize=S.SMALL_PT - 1)
            S.end_label(bx, p["sd"], p["lambda"], f"Λ = {p['lambda']:.2g}", th, dx_px=-9, dy_px=-13, ha="right", fontsize=S.SMALL_PT - 1)
        else:
            S.end_label(ax, p["sd"], p["lsat"], p["label"], th, dx_px=8, dy_px=0, fontsize=S.SMALL_PT - 1)
            S.end_label(bx, p["sd"], p["lambda"], f"Λ = {p['lambda']:.2g}", th, dx_px=8, dy_px=0, fontsize=S.SMALL_PT - 1)
    ax.set_ylim(1, 1e4)
    ax.set_ylabel("saturation length $L_{sat,3D}$ (m)")
    ax.set_title("(a) Undulator length needed to saturate")
    ax.legend(loc="lower right", fontsize=S.SMALL_PT - 1)
    bx.set_ylim(0.1, 2e3)
    bx.set_ylabel("Ming-Xie Λ (3-D gain-length degradation)")
    bx.set_title("(b) Can it lase? Λ against the energy spread")
    return fig


register(FigureSpec(
    name="sources-lpa-fel-energy-spread",
    group="sources",
    compute=lpa_compute,
    draw=lpa_draw,
    alt="Two log-log panels against relative energy spread for the 25 nm and 420 nm LPA-FEL beams: the Ming-Xie saturation "
        "length rising past the 4 m undulator to 186 m at 1 percent for 25 nm, and the degradation parameter crossing the "
        "no-gain line near 5, with the preset points marked.",
    caption="<strong>Can it lase?</strong> 1-D theory plus the Ming-Xie fit for the two illustrative beams: at 25 nm the preset's "
            "1 % energy spread is 1.9 ρ, Λ = 49 and saturation would need 186 m of undulator instead of 4 m — no practical gain — "
            "while 0.1 % restores 5.5 m; the 100 MeV / 420 nm beam at 1 % (Λ = 6, 11 m) is marginal but lasing-capable. "
            "Λ ≳ 5 only means no practical gain; the beams are illustrative LPA-class values, not BELLA's. "
            "<em>Model: LPA-FEL 🔶.</em>",
    pages=("sources/lpa-fel.md",),
))


# ===========================================================================
# The power landscape (published values + presets), panel order: 13.5 nm first
# ===========================================================================


def _csv_family(row) -> str | None:
    cls = row["class"]
    lam = float(row["wavelength_nm"])
    if abs(lam - 13.5) > 0.5:
        return None
    if "LPP" in cls:
        return "Sn LPP"
    if "DPP" in cls or "LDP" in cls:
        return "Discharge plasma"
    if "ERL" in cls:
        return "ERL FEL"
    if "SSMB" in cls or "microbunching" in cls:
        return "SSMB"
    if "Synchrotron" in cls:
        return "Storage-ring undulator"
    if "FEL" in cls or "linac" in cls.lower():
        return "Linac FEL"
    if "HHG" in cls:
        return "HHG"
    if "ICS" in cls or "Compton" in cls:
        return "Inverse Compton"
    if "X-ray laser" in cls:
        return "Soft-X-ray laser"
    return None


def landscape_compute(fast: bool):
    huv = native()
    rows = []
    with open(CSV_PATH, encoding="utf-8") as fh:
        for r in csv.DictReader(fh):
            lo, hi = float(r["power_w_low"]), float(r["power_w_high"])
            rows.append({
                "id": int(r["id"]), "label": r["machine_or_paper"], "cls": r["class"],
                "lam": float(r["wavelength_nm"]), "lo": lo, "hi": hi,
                "projected": r["status"].upper().startswith("PROJECTED"),
                "at_source": "AT THE SOURCE" in r["plane_or_definition"].upper(),
                "family13": _csv_family(r),
            })
    assert len(rows) == 51, f"the dataset has {len(rows)} rows; the caption says 51"
    presets = []
    for key, factory, label, fam, status in PRESETS:
        src = factory(huv.SourceConfig)
        power = src.average_power_w
        if power is None and key == "noon":
            power = src.derived_quantity("photon_power")
        span = None
        if key.startswith("tube"):
            spec = src.xray_spectrum(200)
            e = np.array([p[0] for p in spec])
            span = (HC_EV_NM / 1e3 / e.max(), HC_EV_NM / 1e3 / e.min())
        if key == "betatron":
            ec = src.derived_quantity("critical_energy_kev")
            span = (HC_EV_NM / 1e3 / (8 * ec), HC_EV_NM / 1e3 / (0.1 * ec))  # photon_spectrum window [0.1, 8] E_c
        presets.append({"key": key, "label": label, "family13": fam, "status": status,
                        "lam": src.wavelength_nm, "power": power, "span": span})
    return {"rows": rows, "presets": presets}


def _legend_handles(th):
    blue, orange = th.s(1), th.s(2)
    ms = S.MARKER_PT
    return [
        Line2D([], [], linestyle="none", marker="o", markersize=ms, markerfacecolor=blue, markeredgecolor=th.surface,
               markeredgewidth=S.RING_PT),
        Line2D([], [], linestyle="none", marker="o", markersize=ms * 0.85, markerfacecolor=th.surface,
               markeredgecolor=blue, markeredgewidth=1.4),
        Line2D([], [], linestyle="none", marker="s", markersize=ms * 0.9, markerfacecolor=blue,
               markeredgecolor=th.surface, markeredgewidth=S.RING_PT),
        Line2D([], [], linestyle="none", marker="D", markersize=ms * 0.8, markerfacecolor=orange,
               markeredgecolor=th.surface, markeredgewidth=S.RING_PT),
        Line2D([], [], linestyle="none", marker="D", markersize=ms * 0.7, markerfacecolor=th.surface,
               markeredgecolor=orange, markeredgewidth=1.4),
    ], [
        "published: measured / shipping",
        "published: projection or design",
        "published: discharge plasma, into 2π at the source",
        "simulator preset (average_power_w)",
        "simulator preset: projection / what-if",
    ]


def _draw_csv_point(ax, x, r, th):
    blue = th.s(1)
    marker = "s" if r["at_source"] else "o"
    if r["hi"] > r["lo"]:
        ax.plot([x, x], [r["lo"], r["hi"]], color=blue, linewidth=S.LINE_PT * 0.8, zorder=2, clip_on=False)
    y = math.sqrt(r["lo"] * r["hi"])
    S.dot(ax, [x], [y], blue, th, marker=marker, filled=not r["projected"], size_px=8.4 if marker == "o" else 8.0)


def _draw_preset(ax, x, p, th, size_px=8.0):
    S.dot(ax, [x], [p["power"]], th.s(2), th, marker="D", filled=p["status"] == "measured", size_px=size_px, zorder=4)


def landscape_draw(d, th: S.Theme):
    fig, (ax, bx) = plt.subplots(2, 1, figsize=(S.FULL_WIDTH + 0.6, 9.4), gridspec_kw={"hspace": 0.42,
                                                                                     "height_ratios": [1.0, 1.0]})
    # ---- (a) the 13.5 nm column, by family (first: the question every reader brings)
    S.style_axes(ax, th, grid="y")
    ax.set_yscale("log")
    ax.set_ylim(1e-10, 1e5)
    ax.set_xlim(-0.6, len(FAMILIES_13) - 0.4)
    ax.axhspan(*HVM_BAND_W, color=th.band, zorder=0, lw=0)
    ax.text(len(FAMILIES_13) - 0.45, 230.0, "HVM need at IF: 250 W (NXE:3400B) → 1 kW (stated target)",
            color=th.ink2, fontsize=S.SMALL_PT, ha="right", va="top")
    pos = {f: i for i, f in enumerate(FAMILIES_13)}
    for r in d["rows"]:
        if r["family13"] in pos:
            _draw_csv_point(ax, pos[r["family13"]] - 0.2, r, th)
    for fam, i in pos.items():
        members = [p for p in d["presets"] if p["family13"] == fam and p["power"] is not None]
        members.sort(key=lambda p: -p["power"])
        items = []
        for k, p in enumerate(members):
            x = i + 0.12 + 0.1 * k
            _draw_preset(ax, x, p, th)
            items.append((x, p["power"], fmt_si(p["power"], "W")))
        if items:
            S.end_labels(ax, items, th, min_gap_px=11, dx_px=6, fontsize=S.SMALL_PT - 1)
    ax.set_xticks(range(len(FAMILIES_13)))
    ax.set_xticklabels([f.replace(" ", "\n", 1) if len(f) > 12 else f for f in FAMILIES_13], fontsize=S.SMALL_PT)
    ax.tick_params(axis="x", length=0)
    ax.set_ylabel("Average power (W)")
    ax.set_title("(a) At 13.5 ± 0.5 nm: published values (left) and simulator presets (right) per family")
    # ---- (b) everything on log-log axes
    S.style_axes(bx, th, grid="both")
    bx.set_xscale("log")
    bx.set_yscale("log")
    bx.set_xlim(0.012, 700)
    bx.set_ylim(1e-12, 1e5)
    bx.add_patch(plt.Rectangle((6.0, HVM_BAND_W[0]), 20.0 - 6.0, HVM_BAND_W[1] - HVM_BAND_W[0], facecolor=th.band,
                               edgecolor=th.axis, linewidth=S.HAIR_PT, zorder=1))
    bx.text(22.0, 500, "EUV/BEUV need at IF:\n250 W → 1 kW", color=th.ink2, fontsize=S.SMALL_PT, ha="left", va="center")
    for r in d["rows"]:  # published values drawn 4 % left of their wavelength, presets 4 % right
        _draw_csv_point(bx, r["lam"] / DODGE, r, th)
    for p in d["presets"]:
        if p["power"] is None:
            continue
        if p["span"]:
            bx.plot(p["span"], [p["power"]] * 2, color=th.s(2), linewidth=S.HAIR_PT * 1.3, zorder=3)
        _draw_preset(bx, p["lam"] * DODGE, p, th)
    notes = [
        (400, 2.5e4, "DUV/VUV excimer lasers", "right"),
        (126, 1.0, "Ar$_2$ 126 nm\n(hypothetical)", "center"),
        (0.13, 40, "X-ray tubes (4π emission, LIGA)", "center"),
        (0.45, 3e-7, "betatron", "center"),
        (80, 1e-3, "HHG, SXRL\n(26–57 nm)", "left"),
        (157.6, 3e-11, "entangled NOON\n(photon power)", "center"),
        (32, 1.5e-7, "LPA-FEL", "left"),
        (5.6, 12, "Gd / Tb 6.x nm\n(projections)", "right"),
    ]
    for x, y, txt, ha in notes:
        bx.text(x, y, txt, color=th.ink2, fontsize=S.SMALL_PT, ha=ha, va="center")
    bx.annotate("the 13.5 nm column: panel (a)", xy=(13.5, 2e-10), xytext=(1.6, 2e-10), color=th.ink2,
                fontsize=S.SMALL_PT, ha="center", va="center",
                arrowprops=dict(arrowstyle="->", color=th.axis, lw=S.HAIR_PT, shrinkA=2, shrinkB=2))
    bx.set_xlabel("Wavelength (nm)")
    bx.set_ylabel("Average power (W)")
    bx.set_title("(b) Average power versus wavelength")
    handles, labels = _legend_handles(th)
    fig.legend(handles, labels, loc="lower center", ncol=2, bbox_to_anchor=(0.5, 0.0), handletextpad=0.4)
    fig.subplots_adjust(bottom=0.12, top=0.96)
    return fig


register(FigureSpec(
    name="sources-landscape",
    group="sources",
    compute=landscape_compute,
    draw=landscape_draw,
    alt="Average source power at 13.5 nm per family, where only tin LPP reaches the 250 W to 1 kW band and accelerator "
        "sources are hollow projections, then on log-log axes against wavelength from X-ray tubes to excimer lasers with 51 "
        "published values beside the simulator's presets.",
    caption="<strong>The source landscape: 51 published values beside every preset.</strong> Blue: measured or shipping = filled, "
            "projection or design = open, discharge plasmas into 2π at the source = squares, ranges as bars — each cited in "
            "<code>docs/figures/sim/data/source_power_vs_wavelength.csv</code>; orange diamonds: what the presets report as "
            "<code>average_power_w</code> (open = projection or what-if; X-ray tubes: 4π emission with their spectral span). In (b) "
            "published points sit 4 % left and presets 4 % right of their wavelength so both stay visible. Powers are defined at "
            "different planes (IF, laser output, source), so compare orders of magnitude, not percent. "
            "<em>Data chart from source_power_vs_wavelength.csv and the presets; grades per family on each family page; no model badge.</em>",
    pages=("sources/index.md", "python-api-modules.md"),
))


# ---------------------------------------------------------------------------
# Dose-limited wafer throughput per preset
# ---------------------------------------------------------------------------

TP_REFRACTIVE = dict(optics_transmission=0.3, mask_efficiency=0.9, slit_height_mm=8.0)
TP_BEUV = dict(n_mirrors=11, mirror_reflectivity=0.641)


def throughput_compute(fast: bool):
    huv = native()
    out = []
    for key, factory, label, _fam, status in PRESETS:
        if key.startswith("tube") or key in ("betatron", "noon"):
            continue  # proximity/LIGA sources and the photon-power-only NOON source: no scanner throughput
        src = factory(huv.SourceConfig)
        lam = src.wavelength_nm
        if lam > 100:
            kw, column = TP_REFRACTIVE, "refractive"
        elif key in ("lpp_gd", "lpp_tb"):
            kw, column = TP_BEUV, "beuv"
        else:
            kw, column = {}, "euv"
        res = src.wafer_throughput(TP_DOSE, **kw)
        if res is None:
            continue
        out.append({"label": label, "key": key, "status": status, "column": column, "lam": lam,
                    "wph": res["wafers_per_hour"], "power": res["source_power_w"],
                    "at_wafer": res["power_at_wafer_w"], "pulses": res["pulses_per_point"]})
    out.sort(key=lambda r: r["wph"])
    return out


def throughput_draw(rows, th: S.Theme):
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 0.27 * len(rows) + 1.4))
    S.style_axes(ax, th, grid="x")
    ax.set_xscale("log")
    ys = np.arange(len(rows))
    lo = 1e-11
    markers = {"refractive": "o", "euv": "D", "beuv": "s"}
    for y, r in zip(ys, rows):
        ax.plot([lo, r["wph"]], [y, y], color=th.grid, linewidth=S.HAIR_PT * 1.5, zorder=1)
        S.dot(ax, [r["wph"]], [y], th.s(1), th, marker=markers[r["column"]], filled=r["status"] == "measured",
              size_px=8.0)
        ax.annotate(S.sci(r["wph"]), (r["wph"], y), xytext=(7 * 72 / S.LOGICAL_DPI, 0), textcoords="offset points",
                    color=th.ink2, fontsize=S.SMALL_PT - 1, va="center")
    ax.set_yticks(ys)
    ax.set_yticklabels([r["label"] for r in rows], fontsize=S.SMALL_PT)
    ax.tick_params(axis="y", length=0)
    ax.set_xlim(lo, 3e3)
    ax.set_ylim(-0.8, len(rows) - 0.2)
    ax.set_xlabel(f"Wafers per hour at {TP_DOSE:.0f} mJ/cm² (300 mm, dose-limited model)")
    ax.set_title("Dose-limited throughput of every source preset")
    handles = [
        Line2D([], [], linestyle="none", marker="o", markerfacecolor=th.s(1), markeredgecolor=th.surface,
               markersize=S.MARKER_PT),
        Line2D([], [], linestyle="none", marker="D", markerfacecolor=th.s(1), markeredgecolor=th.surface,
               markersize=S.MARKER_PT * 0.8),
        Line2D([], [], linestyle="none", marker="s", markerfacecolor=th.s(1), markeredgecolor=th.surface,
               markersize=S.MARKER_PT * 0.8),
        Line2D([], [], linestyle="none", marker="o", markerfacecolor=th.surface, markeredgecolor=th.s(1),
               markersize=S.MARKER_PT * 0.85, markeredgewidth=1.4),
    ]
    ax.legend(handles, ["refractive column (T 0.3, mask 0.9)", "EUV column (10 mirrors × 0.70)",
                        "BEUV column (11 × 0.641 La/B)", "open: projection / what-if preset"],
              loc="lower right", fontsize=S.SMALL_PT)
    return fig


register(FigureSpec(
    name="sources-throughput",
    group="sources",
    compute=throughput_compute,
    draw=throughput_draw,
    alt="Dot plot on a logarithmic axis of wafers per hour for every source preset at 30 mJ/cm2: excimer lasers "
        "and tin LPP reach about 150 to 185 wafers per hour, accelerator design points are similar, and "
        "laboratory sources fall between millions and trillions of times short.",
    caption="<strong>Dose-limited throughput of every preset.</strong> Each preset's <code>average_power_w</code> through "
            "<code>SourceConfig.wafer_throughput</code> at 30 mJ/cm² on 300 mm wafers (84 fields of 26 × 33 mm, 0.1 s per field and "
            "10 s per wafer overhead, which caps the model near 196 wph). Columns: EUV 10 mirrors × 0.70 and mask 0.65 (1.8 % "
            "IF→wafer); BEUV 11 × 0.641 (record La/B); refractive transmission 0.3, mask 0.9. X-ray tubes, betatron and the NOON "
            "source are not projection-scanner sources and are omitted. <em>Model: Dose-limited wafer throughput 🔶 (illustrative "
            "overheads; no stage-acceleration or vendor calibration).</em>",
    pages=("sources/index.md", "README.md"),
    extra={"width": {"README.md": 560}},
))


# ---------------------------------------------------------------------------
# Spectrum + pupil fill per family (needs SourceConfig.spectrum() / pupil_fill())
# ---------------------------------------------------------------------------

GALLERY = (
    ("ArF laser", lambda S_: S_.arf_laser(0.7)),
    ("Sn LPP 13.5 nm", lambda S_: S_.lpp_sn_13nm5()),
    ("compact undulator", lambda S_: S_.synchrotron_compact_euv()),
    ("FLASH-class SASE FEL", lambda S_: S_.xfel_flash_13nm5()),
    ("HHG Ne comb", lambda S_: S_.hhg(full_comb=True, comb_passband_nm=(12.8, 14.4))),
    ("ICS design", lambda S_: S_.ics_compact_euv_13nm5()),
)


def gallery_compute(fast: bool):
    huv = native()
    out = []
    n = 61 if fast else 121
    p = np.linspace(-1.2, 1.2, n)
    for label, factory in GALLERY:
        src = factory(huv.SourceConfig)
        weights = np.array(src.spectrum())
        fill = np.asarray(src.pupil_fill(n, float(p[-1])))
        out.append({"label": label, "weights": weights, "fill": fill, "lam": src.wavelength_nm})
    return {"rows": out, "p": p}


def gallery_draw(d, th: S.Theme):
    rows = d["rows"]
    fig, axes = plt.subplots(2, len(rows), figsize=(S.WIDE_WIDTH + 1.6, 4.4),
                             gridspec_kw={"hspace": 0.5, "wspace": 0.35, "height_ratios": [1, 1.1]})
    cmap = S.field_cmap()
    for k, r in enumerate(rows):
        ax = axes[0, k]
        S.style_axes(ax, th, grid=None)
        w = r["weights"]
        lam0 = float(np.sum(w[:, 0] * w[:, 1]) / np.sum(w[:, 1]))
        ax.vlines((w[:, 0] - lam0) * 1e3, 0, w[:, 1] / w[:, 1].max(), color=th.s(1), linewidth=S.LINE_PT)
        ax.set_title(f"{r['label']}\n{lam0:.2f} nm", fontsize=S.SMALL_PT)
        ax.set_yticks([])
        ax.tick_params(axis="x", labelsize=S.SMALL_PT - 1)
        ax.set_xlabel("λ − λ̄ (pm)".replace("λ̄", r"$\bar\lambda$"), fontsize=S.SMALL_PT)
        ax2 = axes[1, k]
        ax2.imshow(r["fill"], origin="lower", extent=[d["p"][0], d["p"][-1]] * 2, cmap=cmap, vmin=0,
                   vmax=max(float(np.max(r["fill"])), 1e-12), interpolation="bilinear")
        ax2.add_patch(plt.Circle((0, 0), 1.0, fill=False, edgecolor=th.axis, linewidth=S.HAIR_PT))
        ax2.set_xticks([])
        ax2.set_yticks([])
        for side in ("top", "right", "left", "bottom"):
            ax2.spines[side].set_visible(False)
    axes[1, 0].set_ylabel("pupil fill (σ)", fontsize=S.SMALL_PT)
    return fig


register(FigureSpec(
    name="sources-spectra-pupils",
    group="sources",
    compute=gallery_compute,
    draw=gallery_draw,
    alt="Six source presets, each with its sampled spectrum (spectral weights used by the imaging engine) above "
        "its pupil fill (illumination intensity over the pupil, unit circle = NA).",
    caption="<strong>What each source preset hands to the imaging engine.</strong> The spectral samples and "
            "weights (top; offset from the weighted mean wavelength) and the pupil fill (bottom; the circle is "
            "the NA). The excimer laser and the plasma source fill a disk (conventional σ); the undulator, FEL, "
            "HHG and ICS presets get a near-coherent Gaussian fill with σ derived from their transverse "
            "coherence; the filtered HHG comb is three harmonics (q 57, 59, 61), each sampled across its own "
            "linewidth. <em>From each preset's <code>spectrum()</code> and <code>pupil_fill()</code>; grades "
            "per family on each family page; no model badge.</em>",
    pages=("sources/index.md",),
    requires=("spectrum", "pupil_fill"),
))
