"""Source figures: power landscape, dose-limited throughput, X-ray spectra, spectra + pupil fills.

Simulator numbers come from the presets' ``average_power_w`` / ``wafer_throughput`` /
``xray_spectrum`` / ``spectral_flux_density``; published numbers come from
``data/source_power_vs_wavelength.csv`` (one citation + URL per row).
"""

from __future__ import annotations

import csv
import math

import numpy as np
from matplotlib import pyplot as plt
from matplotlib.lines import Line2D

import style as S
from common import HERE, FigureSpec, fmt_si, native, register

CSV_PATH = HERE / "data" / "source_power_vs_wavelength.csv"
DODGE = 1.04  # panel (a): published points at lambda / 1.04, presets at lambda * 1.04 (visibility only)

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
            span = (1.23984193 / e.max(), 1.23984193 / e.min())
        if key == "betatron":
            ec = src.derived_quantity("critical_energy_kev")
            span = (1.23984193 / (8 * ec), 1.23984193 / (0.1 * ec))  # photon_spectrum window [0.1, 8] E_c
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
    # ---- (a) everything on log-log axes
    S.style_axes(ax, th, grid="both")
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlim(0.012, 700)
    ax.set_ylim(1e-12, 1e5)
    ax.add_patch(plt.Rectangle((6.0, 250.0), 20.0 - 6.0, 1000.0 - 250.0, facecolor=th.band, edgecolor=th.axis,
                               linewidth=S.HAIR_PT, zorder=1))
    ax.text(5.6, 600, "EUV/BEUV need at IF:\n250 W → 1 kW", color=th.ink2, fontsize=S.SMALL_PT, ha="right",
            va="center")
    for r in d["rows"]:  # published values drawn 4 % left of their wavelength, presets 4 % right
        _draw_csv_point(ax, r["lam"] / DODGE, r, th)
    for p in d["presets"]:
        if p["power"] is None:
            continue
        if p["span"]:
            ax.plot(p["span"], [p["power"]] * 2, color=th.s(2), linewidth=S.HAIR_PT * 1.3, zorder=3)
        _draw_preset(ax, p["lam"] * DODGE, p, th)
    notes = [
        (200, 600, "DUV/VUV excimer lasers", "center"),
        (126, 1.0, "Ar$_2$ 126 nm\n(hypothetical)", "center"),
        (0.13, 40, "X-ray tubes (4π emission, LIGA)", "center"),
        (0.45, 3e-7, "betatron", "center"),
        (80, 1e-3, "HHG, SXRL\n(26–57 nm)", "left"),
        (157.6, 3e-11, "entangled NOON\n(photon power)", "center"),
        (32, 1.5e-7, "LPA-FEL", "left"),
    ]
    for x, y, txt, ha in notes:
        if txt:
            ax.text(x, y, txt, color=th.ink2, fontsize=S.SMALL_PT, ha=ha, va="center")
    ax.annotate("13.5 nm: detailed in (b)", xy=(13.5, 2e-9), xytext=(2.0, 1e-10), color=th.ink2,
                fontsize=S.SMALL_PT, ha="center",
                arrowprops=dict(arrowstyle="-", color=th.axis, lw=S.HAIR_PT, shrinkA=1, shrinkB=2))
    ax.set_xlabel("Wavelength (nm)")
    ax.set_ylabel("Average power (W)")
    ax.set_title("(a) Average power versus wavelength")
    # ---- (b) the 13.5 nm column, by family
    S.style_axes(bx, th, grid="y")
    bx.set_yscale("log")
    bx.set_ylim(1e-10, 1e5)
    bx.set_xlim(-0.6, len(FAMILIES_13) - 0.4)
    bx.axhspan(250.0, 1000.0, color=th.band, zorder=0, lw=0)
    bx.text(len(FAMILIES_13) - 0.45, 230.0, "HVM need at IF: 250 W (NXE:3400B) → 1 kW (stated target)",
            color=th.ink2, fontsize=S.SMALL_PT, ha="right", va="top")
    pos = {f: i for i, f in enumerate(FAMILIES_13)}
    for r in d["rows"]:
        if r["family13"] in pos:
            _draw_csv_point(bx, pos[r["family13"]] - 0.2, r, th)
    for fam, i in pos.items():
        members = [p for p in d["presets"] if p["family13"] == fam and p["power"] is not None]
        members.sort(key=lambda p: -p["power"])
        items = []
        for k, p in enumerate(members):
            x = i + 0.12 + 0.1 * k
            _draw_preset(bx, x, p, th)
            items.append((x, p["power"], fmt_si(p["power"], "W")))
        if items:
            S.end_labels(bx, items, th, min_gap_px=11, dx_px=6, fontsize=S.SMALL_PT - 1)
    bx.set_xticks(range(len(FAMILIES_13)))
    bx.set_xticklabels([f.replace(" ", "\n", 1) if len(f) > 12 else f for f in FAMILIES_13], fontsize=S.SMALL_PT)
    bx.tick_params(axis="x", length=0)
    bx.set_ylabel("Average power (W)")
    bx.set_title("(b) At 13.5 ± 0.5 nm: published values (left) and simulator presets (right) per family")
    handles, labels = _legend_handles(th)
    fig.legend(handles, labels, loc="lower center", ncol=2, bbox_to_anchor=(0.5, 0.0), handletextpad=0.4)
    fig.subplots_adjust(bottom=0.12, top=0.96)
    return fig


register(FigureSpec(
    name="sources-landscape",
    group="sources",
    compute=landscape_compute,
    draw=landscape_draw,
    alt="Log-log chart of average source power versus wavelength combining 49 published values and the "
        "simulator's source presets, with the 250 W to 1 kW EUV requirement band; a second panel compares "
        "families at 13.5 nm, where only tin LPP sources have demonstrated HVM power and accelerator sources "
        "are design projections.",
    caption="The source landscape. Blue: 49 published average powers (measured or shipping = filled; "
            "projection or design = open; discharge plasmas measured into 2π at the source = squares; vertical "
            "bars = ranges), each cited in <code>docs/figures/sim/data/source_power_vs_wavelength.csv</code>. "
            "Orange diamonds: what the simulator's presets report as <code>average_power_w</code> (open = "
            "projection or what-if; X-ray tubes: total 4π emission, horizontal bars = spectral span). In (a) "
            "published points sit 4 % left and presets 4 % right of their wavelength so both stay visible. "
            "Powers are defined at different planes (IF, laser output, source), so compare orders of magnitude, "
            "not percent. Source models range from ✅ to 🧪 — see each family page.",
    pages=("sources/index.md", "README.md"),
))


# ---------------------------------------------------------------------------
# 2. Dose-limited wafer throughput per preset
# ---------------------------------------------------------------------------

TP_DOSE = 30.0
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
    caption="Dose-limited throughput from each preset's <code>average_power_w</code> through "
            "<code>SourceConfig.wafer_throughput</code> at 30 mJ/cm² on 300 mm wafers (84 fields of 26 × 33 mm, "
            "0.1 s per field and 10 s per wafer overhead, which caps the model near 196 wph). Columns: EUV "
            "10 mirrors × 0.70 and mask 0.65 (1.8 % IF→wafer); BEUV 11 × 0.641 (record La/B); refractive "
            "transmission 0.3, mask 0.9. X-ray tubes, betatron and the NOON source are not projection-scanner "
            "sources and are omitted. Model: dose-limited throughput 🔶 (illustrative overheads; no "
            "stage-acceleration or vendor calibration).",
    pages=("sources/index.md",),
))


# ---------------------------------------------------------------------------
# 3. Hard X-ray spectra: tube anodes and betatron
# ---------------------------------------------------------------------------


def xray_compute(fast: bool):
    huv = native()
    n_bins = 150 if fast else 600
    tubes = {}
    for anode in ("W", "Mo", "Cu"):
        src = huv.SourceConfig.xray_tube(anode)
        fd = np.array(src.spectral_flux_density(100.0, n_bins))
        tubes[anode] = {"e": fd[:, 0], "flux": fd[:, 1], "lam": src.wavelength_nm,
                        "power": src.average_power_w, "repr": repr(src)}
    bet = huv.SourceConfig.betatron()
    fd = np.array(bet.spectral_flux_density(100.0, 200))
    return {"tubes": tubes, "betatron": {"e": fd[:, 0], "flux": fd[:, 1],
                                         "ec": bet.derived_quantity("critical_energy_kev")}}


def xray_draw(d, th: S.Theme):
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 4.0), gridspec_kw={"wspace": 0.42,
                                                                                  "width_ratios": [1.5, 1]})
    for ax in (ax1, ax2):
        S.style_axes(ax, th)
        ax.set_yscale("log")
        ax.set_xlabel("Photon energy (keV)")
    kv = {"W": 60, "Mo": 50, "Cu": 40}
    handles, labels = [], []
    for slot, anode in enumerate(("W", "Mo", "Cu"), start=1):
        t = d["tubes"][anode]
        ax1.plot(t["e"], t["flux"], color=th.s(slot), linewidth=S.LINE_PT * 0.8, zorder=3)
        handles.append(Line2D([], [], color=th.s(slot)))
        labels.append(f"{anode} anode, {kv[anode]} kV")
    ax1.legend(handles, labels, loc="upper right")
    ax1.set_xlim(0, 62)
    ax1.set_ylim(1e5, 1e11)
    ax1.set_ylabel("Flux density at 100 mm (photons s⁻¹ mm⁻² keV⁻¹)")
    ax1.set_title("(a) X-ray tube presets: Kramers continuum + K/L lines")
    b = d["betatron"]
    ax2.plot(b["e"], b["flux"], color=th.s(1), zorder=3)
    S.limit_line(ax2, b["ec"], th, f"E_c = {b['ec']:.2f} keV".replace("E_c", "$E_c$"), where=0.97, side="right")
    ax2.set_xlim(0, float(b["e"].max()))
    ax2.set_ylabel("Flux density at 100 mm (photons s⁻¹ mm⁻² keV⁻¹)")
    ax2.set_title("(b) Betatron preset (10 Hz LWFA)")
    fig.subplots_adjust(right=0.97)
    return fig


register(FigureSpec(
    name="sources-xray-spectra",
    group="sources",
    compute=xray_compute,
    draw=xray_draw,
    alt="Two log-scale spectra: tungsten, molybdenum and copper X-ray tube presets show a bremsstrahlung "
        "continuum ending at the tube voltage with sharp characteristic lines; the betatron preset shows a "
        "smooth synchrotron-like spectrum around its critical energy.",
    caption="Hard X-ray spectra of the LIGA-class presets from <code>spectral_flux_density(100 mm)</code>. (a) "
            "X-ray tubes: Kramers bremsstrahlung ending at the Duane–Hunt limit (eV<sub>tube</sub>), anode "
            "K/L lines gated by their edges, 250 µm Be window — line energies, edges, Duane–Hunt limit and "
            "Be filtration ✅; continuum shape and absolute flux 🔶 (Kramers thick target, no self-absorption, "
            "heel effect or backscatter; empirical line yields ±×2). (b) Laser-wakefield betatron: synchrotron-like spectrum from the critical "
            "energy E<sub>c</sub> of a single-energy, single-amplitude bunch, flat-top cone — 🧪.",
    pages=("sources/xray-tube.md", "sources/betatron.md"),
))


# ---------------------------------------------------------------------------
# 4. Spectrum + pupil fill per family (needs SourceConfig.spectrum() / pupil_fill())
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
    caption="What each source preset hands to the imaging engine: the spectral samples and weights (top; "
            "offset from the weighted mean wavelength) and the pupil fill (bottom; the circle is the NA). The "
            "excimer laser and the plasma source fill a disk (conventional σ); the undulator, FEL, HHG and ICS "
            "presets get a near-coherent Gaussian fill with σ derived from their transverse coherence; the "
            "filtered HHG comb is three harmonics (q 57, 59, 61), each sampled across its own linewidth. From "
            "each preset's <code>spectrum()</code> and <code>pupil_fill()</code>; the presets' own status "
            "(✅/🔶/🧪) is given on each family page.",
    pages=("sources/index.md",),
    requires=("spectrum", "pupil_fill"),
))
