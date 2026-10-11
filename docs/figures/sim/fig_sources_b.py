"""Source figures B (DESIGN §5.4 S3d–k): the engine slot of the family pages that
``fig_sources.py`` does not draw — HHG, SSMB, inverse Compton, the entangled NOON source,
plasma soft-X-ray lasers, Smith–Purcell, the X-ray tube, the betatron and the excimer
line shapes.

Every number drawn here is reported by the engine (``SourceConfig.<factory>()``,
``derived_quantities()``, ``spectrum()``, ``spectral_flux_density()``,
``SimulationEngine.compute_noon_ideal_image()``, ``simulate_liga()``,
``MultilayerMirror.mo_si()``) or is a closed form the page states and this module
re-evaluates and asserts against the engine inside ``compute()`` (so ``make_all.py
--verify --group sources_b`` runs every assert without drawing). The few cited
measurements (the HHG power records, the age of the Universe) live in named tuples
with their citation, exactly as the source pages cite them.

Ownership (DESIGN Appendix F): every spec name here starts with one of the nine
``sources-<family>-`` prefixes of group V3b; ``register()`` rejects any name already
claimed by ``fig_sources.py``.
"""

from __future__ import annotations

import math

import numpy as np
from matplotlib import pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.patches import Patch

import style as S
from common import FigureSpec, fmt_si, native, register

HC_EV_NM = 1239.84193  # h c in eV·nm (CODATA 2018)

#: Asset-name prefixes this module may write (Appendix F); checked at registration.
OWNED_PREFIXES = ("sources-hhg-", "sources-ssmb-", "sources-ics-", "sources-noon-", "sources-sxrl-",
                  "sources-smith-purcell-", "sources-xray-tube-", "sources-betatron-", "sources-line-shapes-")

GROUP = "sources_b"


def _register(spec: FigureSpec) -> FigureSpec:
    if not spec.name.startswith(OWNED_PREFIXES):
        raise ValueError(f"{spec.name}: not an asset name group V3b owns ({OWNED_PREFIXES})")
    return register(spec)


def _dq(src) -> dict[str, float]:
    return {name: value for name, value, *_ in src.derived_quantities()}


def _hours(seconds: float) -> str:
    return f"{seconds / 3600:.3g} h"


# ---------------------------------------------------------------------------
# S3d · HHG — the comb and the record curve
# ---------------------------------------------------------------------------

#: The HHG presets as the page defines them: (key, gas, driver intensity, selected harmonic, driver average
#: power W, the engine factory, the page's power-vs-record ratio, label). The driver power is what makes a
#: generic ``hhg(...)`` call the preset — without it the engine assumes its 5 W default — so every preset source
#: below passes it and ``compute()`` asserts the result against the factory and the page's ratio.
HHG_PRESETS = (
    ("ar", "argon", 2e14, 27, 3.0, "hhg_ar_30nm", 0.54, "Ar preset (3 W driver)"),
    ("ne", "neon", 4e14, 59, 20.0, "hhg_ne_13nm5", 0.86, "Ne preset (20 W driver)"),
)
HHG_DRIVER_NM = 800.0
HHG_PASSBAND_NM = (25.0, 35.0)  # the page's example passband (keeps Ar q 23–31)


def _comb_lines(src):
    """(harmonic orders, photon energies eV, relative power per line) of a full-comb HHG source."""
    sp = np.array(src.spectrum())
    e_driver = HC_EV_NM / HHG_DRIVER_NM
    energies = HC_EV_NM / sp[:, 0]
    # the sampler emits `spectral_samples` points per comb line, the centre sample on the exact harmonic
    n_lines = int(round(_dq(src)["comb_line_count"]))
    per = len(sp) // n_lines
    assert per * n_lines == len(sp) and per % 2 == 1, (len(sp), n_lines)
    centre = energies[per // 2::per] / e_driver
    orders = np.rint(centre).astype(int)
    assert np.allclose(centre, orders, atol=1e-6), "centre sample on an integer harmonic"
    power = sp[:, 1].reshape(n_lines, per).sum(axis=1)
    return orders, orders * e_driver, power


def _assert_is_preset(SC, src, factory: str, label: str) -> None:
    """``src`` carries the named factory preset's wavelength and derived power (not the engine's 5 W default)."""
    preset = getattr(SC, factory)()
    assert abs(src.wavelength_nm - preset.wavelength_nm) < 1e-9 * preset.wavelength_nm, (label, src.wavelength_nm)
    assert abs(src.average_power_w - preset.average_power_w) < 1e-9 * preset.average_power_w, \
        f"{label}: {src.average_power_w} W is not the {factory}() preset's {preset.average_power_w} W"


def hhg_comb_compute(fast: bool):
    huv = native()
    SC = huv.SourceConfig
    panels = []
    for key, gas, inten, q_pre, p_drv, factory, _ratio, label in HHG_PRESETS:
        comb = SC.hhg(driver_wavelength_nm=HHG_DRIVER_NM, gas=gas, driver_intensity_w_cm2=inten, harmonic=q_pre,
                      driver_average_power_w=p_drv, full_comb=True)
        mono = SC.hhg(driver_wavelength_nm=HHG_DRIVER_NM, gas=gas, driver_intensity_w_cm2=inten, harmonic=q_pre,
                      driver_average_power_w=p_drv)
        _assert_is_preset(SC, mono, factory, label)
        orders, e_q, power = _comb_lines(comb)
        dq = _dq(comb)
        u_p, e_max = dq["ponderomotive_energy"], dq["cutoff_energy"]
        i_p = e_max - 3.17 * u_p  # the gas's ionization potential, from the cutoff law the engine reports
        # --- physical asserts (the comb the engine samples is the textbook odd-harmonic comb)
        assert len(orders) == int(round(dq["comb_line_count"])), (len(orders), dq["comb_line_count"])
        assert np.all(orders % 2 == 1) and np.all(np.diff(orders) == 2), "odd harmonics spaced by 2ħω"
        assert e_q.min() > i_p and e_q.max() < e_max, "every comb line between I_p and the cutoff"
        plateau = power[e_q <= 0.9 * e_max]
        rel = power / plateau.max()
        envelope = np.where(e_q > 0.9 * e_max, np.exp(-(e_q - 0.9 * e_max) / (0.05 * e_max)), 1.0)
        assert np.allclose(rel, envelope, rtol=0.02), "relative line powers follow the plateau + roll-off envelope"
        e_grid = np.linspace(0.0, 110.0, 1101)
        env_grid = np.where(e_grid > 0.9 * e_max, np.exp(-(e_grid - 0.9 * e_max) / (0.05 * e_max)), 1.0)
        env_grid[e_grid < i_p] = np.nan
        env_grid[e_grid > e_max] = np.nan
        panel = dict(key=key, gas=gas, label=label, orders=orders, e=e_q, power=rel, i_p=i_p, e_max=e_max,
                     u_p=u_p, keldysh=dq["keldysh_parameter"], e_grid=e_grid, env=env_grid,
                     q_pre=q_pre, e_pre=_dq(mono)["photon_energy"], p_pre=mono.average_power_w,
                     lam_pre=mono.wavelength_nm)
        if key == "ar":
            band = SC.hhg(driver_wavelength_nm=HHG_DRIVER_NM, gas=gas, driver_intensity_w_cm2=inten, harmonic=q_pre,
                          driver_average_power_w=p_drv, full_comb=True, comb_passband_nm=HHG_PASSBAND_NM)
            b_orders, b_e, _ = _comb_lines(band)
            assert list(b_orders) == [23, 25, 27, 29, 31], list(b_orders)  # the page: "keeps the Ar harmonics 23–31"
            panel["passband_orders"] = b_orders
        panels.append(panel)
    assert panels[0]["orders"].min() == 11 and panels[0]["orders"].max() == 33  # Ar q 11–33 (page)
    return {"panels": panels, "e_driver": HC_EV_NM / HHG_DRIVER_NM}


def hhg_comb_draw(d, th: S.Theme):
    fig, axes = plt.subplots(2, 1, figsize=(S.FULL_WIDTH, 5.4), sharex=True, gridspec_kw={"hspace": 0.32})
    e_lo, e_hi = 8.0, 102.0
    for ax, p, slot in zip(axes, d["panels"], (1, 2)):
        S.style_axes(ax, th, grid="y")
        colour = th.s(slot)
        ax.plot(p["e_grid"], p["env"], color=th.ink2, linewidth=S.HAIR_PT * 1.4, linestyle=(0, (3, 2)), zorder=2)
        ax.vlines(p["e"], 0, p["power"], color=colour, linewidth=S.LINE_PT, zorder=3)
        S.dot(ax, p["e"], p["power"], colour, th, size_px=6.5, zorder=4)
        S.limit_line(ax, p["i_p"], th, f"I_p {p['i_p']:.1f} eV".replace("I_p", "$I_p$"), where=0.96, side="left")
        S.limit_line(ax, p["e_max"], th, f"cutoff {p['e_max']:.1f} eV", where=0.96, side="right")
        if "passband_orders" in p:
            lo, hi = HC_EV_NM / HHG_PASSBAND_NM[1], HC_EV_NM / HHG_PASSBAND_NM[0]
            ax.axvspan(lo, hi, color=th.band, zorder=0, lw=0)
            ax.text((lo + hi) / 2, 1.04, f"{HHG_PASSBAND_NM[0]:.0f}–{HHG_PASSBAND_NM[1]:.0f} nm passband: q "
                    f"{p['passband_orders'][0]}–{p['passband_orders'][-1]}", color=th.ink2, fontsize=S.SMALL_PT,
                    ha="center", va="bottom")
        # the monochromatized preset harmonic
        k = int(np.where(p["orders"] == p["q_pre"])[0][0])
        if key_is_ar := (p["key"] == "ar"):
            xytext, coords, ha, va = (100.0, 0.5), "data", "right", "center"
        else:
            xytext, coords, ha, va = (-10, 14), "offset points", "right", "bottom"
        ax.annotate(f"q = {p['q_pre']} selected: {p['lam_pre']:.2f} nm, {fmt_si(p['p_pre'], 'W')} (derived)",
                    (p["e"][k], p["power"][k]), xytext=xytext, textcoords=coords, ha=ha, va=va, color=th.ink2,
                    fontsize=S.SMALL_PT, arrowprops=dict(arrowstyle="-", color=th.axis, lw=S.HAIR_PT, shrinkA=0, shrinkB=3))
        if key_is_ar:
            ax.text(100.0, 0.8, "dashed: the plateau and roll-off\nenvelope the engine applies", color=th.ink2,
                    fontsize=S.SMALL_PT, ha="right", va="center")
        ax.set_ylim(0, 1.18)
        ax.set_yticks([0, 0.5, 1.0])
        ax.set_ylabel("relative line power")
        ax.set_title(f"({'ab'[slot - 1]}) {p['gas'].capitalize()}, 800 nm driver, "
                     f"{'2' if p['key'] == 'ar' else '4'}×10¹⁴ W/cm²: q {p['orders'][0]}–{p['orders'][-1]} "
                     f"({len(p['orders'])} lines), γ_K = {p['keldysh']:.2f}".replace("γ_K", "$\\gamma_K$"))
    axes[0].set_xlim(e_lo, e_hi)
    axes[1].set_xlabel("photon energy (eV)")
    sec = axes[0].secondary_xaxis("top", functions=(lambda e: HC_EV_NM / np.maximum(e, 1e-9),
                                                    lambda lam: HC_EV_NM / np.maximum(lam, 1e-9)))
    sec.set_xticks([100, 60, 40, 30, 20, 15, 13.5])
    sec.set_xticklabels(["100", "60", "40", "30", "20", "15", "13.5 nm"], fontsize=S.SMALL_PT)
    sec.tick_params(colors=th.axis, labelcolor=th.ink2, width=S.HAIR_PT)
    sec.spines["top"].set_color(th.axis)
    fig.subplots_adjust(left=0.09, right=0.98, top=0.9, bottom=0.1)
    return fig


_register(FigureSpec(
    name="sources-hhg-comb-spectrum",
    group=GROUP,
    compute=hhg_comb_compute,
    draw=hhg_comb_draw,
    alt="Two odd-harmonic combs on a photon-energy axis: argon from an 800 nm driver spans harmonics 11 to 33 "
        "(cutoff 53.6 eV) and neon spans 15 to 63 (cutoff 97.3 eV); each comb sits on a flat plateau that rolls off "
        "below the cutoff, with the 25 to 35 nm passband and the selected harmonics 27 and 59 marked.",
    caption="<strong>The harmonic comb the engine samples.</strong> Full-comb mode of the two HHG presets "
            "(<code>hhg(full_comb=True).spectrum()</code>, summed per harmonic): odd harmonics of the 800 nm driver "
            "from the first line above the ionization potential to the single-atom cutoff "
            "<i>I</i><sub>p</sub> + 3.17 <i>U</i><sub>p</sub>, flat on the plateau and rolling off above 0.9 of the "
            "cutoff. The band shows the 25–35 nm passband the page uses with the exact per-wavelength imaging path; "
            "the selected harmonics are the monochromatized presets with their derived powers. "
            "<em>Model: High-harmonic generation ✅ (🔶 intensities: line positions and the cutoff are exact, the "
            "relative intensities are an empirical envelope).</em>",
    pages=("sources/hhg.md",),
))


#: Measured single-harmonic HHG average powers the page cites (photon energy eV, W, author, year); the first
#: three are the anchors of the engine's record curve (see the ``measured_power_record`` note), the others are
#: the page's additional 13.5 nm reports.
HHG_RECORDS = (
    (26.5, 12.9e-3, "Klas et al. 2021", "engine anchor"),
    (30.0, 0.144e-3, "Hädrich et al. 2014", "engine anchor"),
    (92.0, 1.0e-6, "Hyogo / RIKEN 2012", "engine anchor"),
    (21.7, 0.83e-3, "Klas et al. 2016", "page"),
    (92.0, 0.43e-6, "KMLabs / JILA 2017", "page"),
    (92.0, 60e-9, "Drs et al. 2024", "page"),
)
HHG_RECORD_END_EV = 92.0


def _record_at(SC, energy_ev: float):
    """The engine's ``measured_power_record`` at one photon energy (helium at 1.5e15 W/cm² keeps every
    harmonic below the cutoff; the record curve depends on the photon energy only)."""
    best = None
    for q in range(5, 301, 2):
        lam = q * HC_EV_NM / energy_ev
        if 450.0 <= lam <= 1600.0 and (best is None or abs(lam - 800.0) < abs(best[1] - 800.0)):
            best = (q, lam)
    q, lam = best
    src = SC.hhg(driver_wavelength_nm=lam, gas="helium", driver_intensity_w_cm2=1.5e15, harmonic=q)
    e = _dq(src)["photon_energy"]
    assert abs(e - energy_ev) < 1e-6 * energy_ev
    return src.derived_quantity("measured_power_record")


def hhg_record_compute(fast: bool):
    huv = native()
    SC = huv.SourceConfig
    energies = np.round(np.arange(12.0, 110.0 + 1e-9, 1.0 if fast else 0.5), 3)
    curve = np.array([(_record_at(SC, e) or np.nan) for e in energies], dtype=float)
    above = energies > HHG_RECORD_END_EV + 0.25
    assert np.all(np.isnan(curve[above])), "no record above 92 eV: the curve is blank there"
    assert np.all(np.isfinite(curve[~above]))
    inter = (energies >= 30.0) & (energies <= HHG_RECORD_END_EV)
    assert np.all(np.diff(curve[inter]) <= 1e-12), "the 30–92 eV interpolation is monotone"
    for e_ev, p_w, _, kind in HHG_RECORDS:
        if kind == "engine anchor":
            r = _record_at(SC, e_ev - 1e-6)  # just below the step
            assert abs(r - p_w) / p_w < 0.02, (e_ev, r, p_w)
    presets = []
    for key, gas, inten, q_pre, p_drv, factory, ratio_page, label in HHG_PRESETS:
        src = SC.hhg(driver_wavelength_nm=HHG_DRIVER_NM, gas=gas, driver_intensity_w_cm2=inten, harmonic=q_pre,
                     driver_average_power_w=p_drv)
        _assert_is_preset(SC, src, factory, label)
        dq = _dq(src)
        assert dq["power_vs_measured_record"] < 1.0, "the presets sit inside the measured records"
        # the page's derived-quantities table and this figure's caption: 0.54 (Ar) and 0.86 (Ne) of the record
        assert abs(dq["power_vs_measured_record"] - ratio_page) < 0.01 * ratio_page, \
            (label, dq["power_vs_measured_record"], ratio_page)
        presets.append(dict(label=label, e=dq["photon_energy"], p=src.average_power_w, ratio=dq["power_vs_measured_record"],
                            record=dq["measured_power_record"], projection=False))
    big = SC.hhg(driver_wavelength_nm=HHG_DRIVER_NM, gas="argon", driver_intensity_w_cm2=2e14, harmonic=27,
                 driver_average_power_w=50.0)
    dq = _dq(big)
    assert dq["power_vs_measured_record"] > 1.0
    presets.append(dict(label="Ar q 27 with a 50 W driver", e=dq["photon_energy"], p=big.average_power_w,
                        ratio=dq["power_vs_measured_record"], record=dq["measured_power_record"], projection=True))
    return {"e": energies, "curve": curve, "presets": presets,
            "records": [dict(e=e, p=p, who=w, kind=k) for e, p, w, k in HHG_RECORDS]}


def hhg_record_draw(d, th: S.Theme):
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 4.6))
    S.style_axes(ax, th, grid="both")
    ax.set_yscale("log")
    e, c = d["e"], d["curve"]
    blue, orange = th.s(1), th.s(2)
    solid = e <= 30.0
    inter = (e >= 30.0) & (e <= HHG_RECORD_END_EV)
    ax.step(e[solid], c[solid], where="post", color=blue, linewidth=S.LINE_PT, zorder=3)
    ax.plot(e[inter], c[inter], color=blue, linewidth=S.LINE_PT, linestyle=(0, (4, 3)), zorder=3)
    ax.axvspan(HHG_RECORD_END_EV, 112, facecolor="none", edgecolor=th.axis, hatch="///", linewidth=0, zorder=0)
    ax.text(HHG_RECORD_END_EV + 1.5, 2e-2, "no record\nabove 92 eV:\nunanchored", color=th.ink2, fontsize=S.SMALL_PT,
            va="top")
    ax.text(54, 2.2e-6, "log-linear interpolation:\nno records between 30 and 92 eV", color=th.ink2,
            fontsize=S.SMALL_PT, ha="center", va="top")
    items = []
    for r in d["records"]:
        S.dot(ax, [r["e"]], [r["p"]], blue, th, size_px=8.4, zorder=5)
        items.append((r["e"], r["p"], f"{r['who']}  {fmt_si(r['p'], 'W')}"))
    for p in d["presets"]:
        S.dot(ax, [p["e"]], [p["p"]], orange, th, marker="D", filled=not p["projection"], size_px=8.0, zorder=6)
        tag = "projection, " if p["projection"] else "derived, "
        items.append((p["e"], p["p"], f"{p['label']}: {fmt_si(p['p'], 'W')} ({tag}{p['ratio']:.2f}× the record)"))
    S.end_labels(ax, items, th, min_gap_px=13, dx_px=9, fontsize=S.SMALL_PT - 0.5)
    ax.set_xlim(10, 112)
    ax.set_ylim(1e-8, 5e-2)
    ax.set_xlabel("photon energy (eV)")
    ax.set_ylabel("average power in one harmonic (W)")
    sec = ax.secondary_xaxis("top", functions=(lambda x: HC_EV_NM / np.maximum(x, 1e-9),
                                               lambda lam: HC_EV_NM / np.maximum(lam, 1e-9)))
    sec.set_xticks([100, 60, 40, 30, 20, 13.5])
    sec.set_xticklabels(["100", "60", "40", "30", "20", "13.5 nm"], fontsize=S.SMALL_PT)
    sec.tick_params(colors=th.axis, labelcolor=th.ink2, width=S.HAIR_PT)
    sec.spines["top"].set_color(th.axis)
    handles = [Line2D([], [], color=blue, linewidth=S.LINE_PT),
               Line2D([], [], linestyle="none", marker="o", markerfacecolor=blue, markeredgecolor=th.surface,
                      markersize=S.MARKER_PT),
               Line2D([], [], linestyle="none", marker="D", markerfacecolor=orange, markeredgecolor=th.surface,
                      markersize=S.MARKER_PT * 0.85),
               Line2D([], [], linestyle="none", marker="D", markerfacecolor=th.surface, markeredgecolor=orange,
                      markersize=S.MARKER_PT * 0.8)]
    ax.legend(handles, ["the engine's record curve (measured_power_record)", "measured record (cited)",
                        "preset: derived driver × efficiency", "projection: above the record"], loc="lower left",
              fontsize=S.SMALL_PT - 0.5)
    fig.subplots_adjust(left=0.1, right=0.72, top=0.88, bottom=0.12)
    return fig


_register(FigureSpec(
    name="sources-hhg-record-curve",
    group=GROUP,
    compute=hhg_record_compute,
    draw=hhg_record_draw,
    alt="Log-scale HHG average power versus photon energy: the engine's record curve steps from 12.9 mW below 26.5 eV "
        "to 0.14 mW at 30 eV and is interpolated down to 1 microwatt at 92 eV, with six cited measurements, the two "
        "presets below the curve and a 50 W-driver projection nine times above it.",
    caption="<strong>Where the presets sit against what HHG has delivered.</strong> The engine's "
            "<code>measured_power_record</code> curve is anchored on three cited records and log-interpolated between "
            "30 and 92 eV, where the curated set holds no measurement; above 92 eV it is blank, so a derived power there "
            "is unanchored. The Ar and Ne presets (assumed 3 W and 20 W drivers times an order-of-magnitude "
            "efficiency) fall at 0.54 and 0.86 of the record; the same Ar harmonic from a 50 W driver would be 9× "
            "above it and is labelled a projection. <em>Model: High-harmonic generation ✅ (🔶 intensities: the "
            "conversion efficiency is an order-of-magnitude estimate).</em>",
    pages=("sources/hhg.md",),
))


# ---------------------------------------------------------------------------
# S3e · SSMB — coherent power against the bunching factor
# ---------------------------------------------------------------------------

SSMB_TABLE_B = (0.01, 0.05, 0.073, 0.1, 0.146)  # the page's five rows
SSMB_HVM_W, SSMB_TARGET_W = 250.0, 1000.0


def ssmb_compute(fast: bool):
    huv = native()
    SC = huv.SourceConfig
    design = SC.ssmb_euv_13nm5()
    dq = _dq(design)
    p_full, p_inc, lam = dq["power_at_full_bunching"], dq["incoherent_power"], design.wavelength_nm
    b = np.logspace(-3, 0, 60 if fast else 181)
    power = np.array([SC.ssmb(bunching_factor=float(x)).average_power_w for x in b])
    sigma = np.array([SC.ssmb(bunching_factor=float(x)).derived_quantity("microbunch_rms_length") for x in b])
    assert np.allclose(power / b ** 2, p_full, rtol=1e-9), "P = P(b=1)·b² exactly"
    k = 2 * math.pi / lam
    assert np.allclose(sigma[:-1], np.sqrt(2 * np.log(1 / b[:-1])) / k, rtol=1e-9), "σ_z = √(2 ln(1/b)) / k"
    assert abs(power[-1] - p_full) < 1e-6 * p_full and sigma[-1] == 0.0
    table = []
    for x in SSMB_TABLE_B[:-1] + (dq["bunching_factor"],):  # the fifth row is the design point's own b (0.1458 → 1 kW)
        s = SC.ssmb(bunching_factor=x)
        table.append(dict(b=x, p=s.average_power_w, sigma=s.derived_quantity("microbunch_rms_length")))
    assert abs(table[-1]["b"] - 0.146) < 5e-4 and abs(table[-1]["p"] - 1000.0) < 1e-6, "the design b reproduces the 1 kW projection"
    assert abs(SC.ssmb(bunching_factor=0.146).average_power_w - 1000.0) / 1000.0 < 0.01, "b = 0.146 rounds to it"
    assert abs(dq["bunching_for_hvm"] - math.sqrt(SSMB_HVM_W / p_full)) < 1e-9
    return {"b": b, "power": power, "sigma": sigma, "p_full": p_full, "p_inc": p_inc, "lam": lam, "k": k,
            "b_hvm": dq["bunching_for_hvm"], "b_design": dq["bunching_factor"], "table": table,
            "enhancement": dq["coherent_enhancement"]}


def ssmb_draw(d, th: S.Theme):
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 4.8))
    S.style_axes(ax, th, grid="both")
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.axvspan(1e-3, 1.0, facecolor="none", edgecolor=th.axis, hatch="//", linewidth=0, zorder=0, alpha=0.6)
    ax.plot(d["b"], d["power"], color=th.s(1), linewidth=S.LINE_PT, zorder=3)
    S.limit_line(ax, SSMB_TARGET_W, th, "1 kW design projection", axis="y", where=0.02, side="right", va="bottom")
    S.limit_line(ax, SSMB_HVM_W, th, "250 W HVM benchmark", axis="y", where=0.02, side="right", va="top")
    ax.axhline(d["p_inc"], color=th.s(2), linewidth=S.HAIR_PT * 1.4, zorder=2)
    ax.text(1.05e-3, d["p_inc"] * 1.25, f"{d['p_inc']:.2f} W spontaneous floor (same radiator and current)",
            color=th.ink2, fontsize=S.SMALL_PT, va="bottom")
    items = []
    for row in d["table"]:
        S.dot(ax, [row["b"]], [row["p"]], th.s(1), th, size_px=8.0, zorder=5)
        watts = f"{row['p']:,.0f}" if row["p"] >= 100 else f"{row['p']:.3g}"
        y_lab = row["p"] * (1.45 if any(abs(math.log(row["p"] / ref)) < 0.12 for ref in (SSMB_HVM_W, SSMB_TARGET_W)) else 1.0)
        items.append((row["b"], y_lab, f"b = {row['b']:.3g}: {watts} W, σ_z {row['sigma']:.2f} nm"))
    S.end_labels(ax, items, th, min_gap_px=13, dx_px=9, fontsize=S.SMALL_PT - 0.5)
    ax.text(0.5, 0.97, "whole range undemonstrated at any EUV harmonic\n(one-turn microbunching shown at 1064 nm only)",
            transform=ax.transAxes, ha="center", va="top", color=th.ink2, fontsize=S.SMALL_PT)
    ax.text(0.95, 3e3, f"P = {d['p_full'] / 1e3:.1f} kW × b²", color=th.ink2, fontsize=S.SMALL_PT, ha="right", va="top")
    ax.set_xlim(1e-3, 1.0)
    ax.set_ylim(1e-3, 1e5)
    ax.set_xlabel("bunching factor |b| at harmonic 78 (13.5 nm)")
    ax.set_ylabel("coherent average power (W, into the radiator's central cone)")
    k = d["k"]
    sec = ax.secondary_xaxis("top", functions=(lambda b: np.sqrt(2 * np.log(1 / np.clip(b, 1e-12, 1 - 1e-12))) / k,
                                               lambda s: np.exp(-(k * np.asarray(s, dtype=float)) ** 2 / 2)))
    sec.set_xticks([8, 7, 6, 5, 4, 3, 2])
    sec.set_xticklabels(["8", "7", "6", "5", "4", "3", "2 nm rms"], fontsize=S.SMALL_PT)
    sec.set_xlabel("Gaussian microbunch length σ_z giving this b".replace("σ_z", "$\\sigma_z$"), fontsize=S.SMALL_PT,
                   color=th.ink2)
    sec.tick_params(colors=th.axis, labelcolor=th.ink2, width=S.HAIR_PT)
    sec.spines["top"].set_color(th.axis)
    fig.subplots_adjust(left=0.1, right=0.7, top=0.84, bottom=0.12)
    return fig


_register(FigureSpec(
    name="sources-ssmb-bunching",
    group=GROUP,
    compute=ssmb_compute,
    draw=ssmb_draw,
    alt="Log-log coherent power versus bunching factor for the SSMB design point: a b-squared line from 47 kW at "
        "full bunching, crossing 250 W at b 0.073 and 1 kW at b 0.146 (4.2 nm rms microbunches), above a 1.67 W "
        "spontaneous floor, with the whole range marked undemonstrated.",
    caption="<strong>Every watt of the SSMB projection is a bunching factor nobody has sustained.</strong> "
            "<code>ssmb(bunching_factor=b).average_power_w</code> for the 400 MeV, 1 A design point with its "
            "100-period K = 1.6 radiator: the power is P(b = 1)·b² exactly, so the 1 kW projection is the statement "
            "b ≈ 0.146 — 4.2 nm rms microbunches at harmonic 78, every turn — and 250 W needs b ≈ 0.073. The top axis "
            "is the Gaussian microbunch length that gives each b; the hatched range has no demonstration at any EUV "
            "harmonic (one-turn microbunching has been shown at 1064 nm). <em>Model: SSMB storage ring 🧪.</em>",
    pages=("sources/ssmb.md",),
))


# ---------------------------------------------------------------------------
# S3f · Inverse Compton — collection fraction and red-shift
# ---------------------------------------------------------------------------

ICS_BAND = 0.02  # the Mo/Si 2 % band the page uses


def ics_compute(fast: bool):
    huv = native()
    SC = huv.SourceConfig
    design = SC.ics_compact_euv_13nm5()
    dq = _dq(design)
    gamma = dq["electron_gamma"]
    theta_band, f_band = dq["in_band_half_angle"], dq["in_band_fraction_max"]
    beta = math.sqrt(1 - 1 / gamma ** 2)
    a0 = 0.1  # the design point's laser a0 (the factory default the preset is built from)
    theta = np.logspace(-0.6, 3.2, 60 if fast else 200)  # mrad
    f = np.empty_like(theta)
    bw = np.empty_like(theta)
    for i, t in enumerate(theta):
        s = SC.ics(collection_half_angle_mrad=float(t))
        q = _dq(s)
        f[i], bw[i] = q["collected_fraction"], q["relative_bandwidth"]
        if i == 0:
            assert abs(q["electron_gamma"] - gamma) < 1e-12, "same kinematics as the design point"
    assert np.all(np.diff(f) > 0), "the collected fraction grows monotonically with the half-angle"
    # exact collection fraction of the boosted dipole pattern (the page's closed form)
    u = (np.cos(theta * 1e-3) - beta) / (1 - beta * np.cos(theta * 1e-3))
    f_closed = 3 / 8 * ((1 - u) + (1 - u ** 3) / 3)
    assert np.allclose(f, f_closed, rtol=1e-6), "engine collected_fraction = the exact boosted-dipole fraction"
    one_over_gamma = 1e3 / gamma  # mrad
    f_1g = _dq(SC.ics(collection_half_angle_mrad=one_over_gamma))["collected_fraction"]
    assert abs(f_1g - 0.5) < 0.01, f"half the photons inside 1/γ (γ = {gamma:.2f}): {f_1g}"
    f_design = _dq(SC.ics(collection_half_angle_mrad=theta_band))["collected_fraction"]
    assert abs(f_design - f_band) < 1e-9
    shift = gamma ** 2 * (theta * 1e-3) ** 2 / (1 + a0 ** 2 / 2)
    e_spread = math.sqrt(max(bw[0] ** 2 - shift[0] ** 2, 0.0))  # the 2ΔE/E term the engine adds in quadrature
    assert np.allclose(bw, np.sqrt(shift ** 2 + e_spread ** 2), rtol=1e-6), "quadrature bandwidth model"
    assert abs(shift[np.argmin(abs(theta - theta_band))] - ICS_BAND) < 0.05 * ICS_BAND
    f_legacy = _dq(SC.ics(collection_half_angle_mrad=1.0))["collected_fraction"]
    return {"theta": theta, "f": f, "bw": bw, "shift": shift, "gamma": gamma, "theta_band": theta_band,
            "f_band": f_band, "one_over_gamma": one_over_gamma, "f_1g": f_1g, "f_legacy": f_legacy,
            "e_spread": e_spread, "power_w": design.average_power_w, "gap": dq["hvm_power_gap"]}


def ics_draw(d, th: S.Theme):
    fig, (ax, bx) = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 4.2), gridspec_kw={"wspace": 0.38})
    for a in (ax, bx):
        S.style_axes(a, th, grid="both")
        a.set_xscale("log")
        a.set_yscale("log")
        a.set_xlim(0.3, 1500)
        a.set_xlabel("collection half-angle θ_c (mrad)".replace("θ_c", "$\\theta_c$"))
    ax.plot(d["theta"], d["f"], color=th.s(1), linewidth=S.LINE_PT, zorder=3)
    for t, lab, side, where in ((1.0, "1 mrad (legacy preset)", "left", 0.5), (d["theta_band"], f"{d['theta_band']:.1f} mrad: 2 % band", "right", 0.03),
                                (d["one_over_gamma"], f"1/γ = {d['one_over_gamma']:.0f} mrad", "right", 0.03)):
        S.limit_line(ax, t, th, lab, where=where, side=side, va="bottom")
    for t, f in ((1.0, d["f_legacy"]), (d["theta_band"], d["f_band"]), (d["one_over_gamma"], d["f_1g"])):
        S.dot(ax, [t], [f], th.s(1), th, size_px=8.0, zorder=5)
    S.end_labels(ax, [(1.0, d["f_legacy"], f"{d['f_legacy']:.1e}"), (d["theta_band"], d["f_band"], f"{d['f_band'] * 100:.2f} % in band"),
                      (d["one_over_gamma"], d["f_1g"], f"{d['f_1g']:.3f}: half the photons")], th, dx_px=8)
    ax.set_ylim(1e-5, 1.5)
    ax.set_ylabel("collected fraction f(θ_c)".replace("θ_c", "$\\theta_c$"))
    ax.set_title(f"(a) Thomson dipole pattern boosted to the lab, γ = {d['gamma']:.3f}")
    bx.plot(d["theta"], d["shift"], color=th.s(2), linewidth=S.LINE_PT, zorder=3)
    bx.plot(d["theta"], d["bw"], color=th.ink2, linewidth=S.HAIR_PT * 1.6, linestyle=(0, (4, 3)), zorder=3)
    bx.axhspan(1e-5, ICS_BAND, color=th.band, zorder=0, lw=0)
    bx.text(0.37, 1.2e-3, "inside the 2 % Mo/Si band", color=th.ink2, fontsize=S.SMALL_PT, va="bottom")
    S.limit_line(bx, d["theta_band"], th, f"{d['theta_band']:.1f} mrad", where=0.97, side="right")
    bx.set_ylim(1e-5, 30)
    bx.set_ylabel("relative red-shift γ²θ² / (1 + a₀²/2)".replace("γ²θ²", "$\\gamma^2\\theta^2$").replace("a₀²", "$a_0^2$"))
    bx.set_title("(b) Off-axis photons are redder")
    bx.legend([Line2D([], [], color=th.s(2)), Line2D([], [], color=th.ink2, linestyle=(0, (4, 3)))],
              ["angular red-shift (exact kinematics)", f"engine bandwidth: quadrature with 2ΔE/E = {d['e_spread']:.1%}"],
              loc="lower right", fontsize=S.SMALL_PT - 0.5)
    fig.subplots_adjust(left=0.08, right=0.98, top=0.9, bottom=0.14)
    return fig


_register(FigureSpec(
    name="sources-ics-collection",
    group=GROUP,
    compute=ics_compute,
    draw=ics_draw,
    alt="Two log-log panels for the 13.5 nm inverse-Compton design point at gamma 4.378: the collected photon "
        "fraction rises from 0.003 percent at 1 mrad to 2.8 percent at the 32.4 mrad band edge and half at 1 over "
        "gamma; the off-axis red-shift crosses the 2 percent Mo/Si band at the same 32.4 mrad.",
    caption="<strong>Why only 2.8 % of the scattered photons are usable.</strong> (a) The exact collection fraction "
            "of the Thomson dipole pattern boosted to the lab (<code>ics(collection_half_angle_mrad=θ)</code>) for the "
            "1.73 MeV electrons that reach 13.5 nm from a 1030 nm laser: half the photons lie inside 1/γ ≈ 228 mrad. "
            "(b) But a photon at angle θ is red-shifted by γ²θ², so staying inside the 2 % multilayer band caps the "
            "half-angle at 32.4 mrad and the fraction at 2.8 % — the number behind the design point's 71 µW. "
            "<em>Model: Inverse Compton scattering 🧪 (✅ exact kinematics; 🔶 quadrature bandwidth).</em>",
    pages=("sources/inverse-compton.md",),
))


# ---------------------------------------------------------------------------
# S3g · NOON — the exposure-time ladder and the honesty split
# ---------------------------------------------------------------------------

NOON_RATES = (1e6, 1e9)  # pairs/s: the SPDC-class default and a 1000× brighter hypothetical
#: (derived-quantity key, σ_E label, bound?) in the order the page lists them
NOON_SCENARIOS = (
    ("exposure_time_claims_optimistic", "largest disputed claim, σ_E = 10⁻¹⁷ cm²", False),
    ("exposure_time_claims_pessimistic", "smallest early claim, σ_E = 10⁻²¹ cm²", False),
    ("exposure_time_bound", "loosest independent upper bound, σ_E = 10⁻²³ cm² (default)", True),
    ("exposure_time_bound_tight", "tightest independent upper bound, σ_E = 10⁻²⁵ cm²", True),
)
#: Age of the Universe in seconds as the quantum page cites it (reference [35] there).
AGE_OF_UNIVERSE_S = 4.4e17
YEAR_S = 365.25 * 86400.0


def noon_ladder_compute(fast: bool):
    huv = native()
    SC = huv.SourceConfig
    rows = []
    for rate in NOON_RATES:
        src = SC.entangled_noon(pair_rate_hz=rate)
        dq = _dq(src)
        rows.append({"rate": rate, "times": [dq[key] for key, _, _ in NOON_SCENARIOS],
                     "photon_power": dq["photon_power"]})
    t6, t9 = np.array(rows[0]["times"]), np.array(rows[1]["times"])
    assert np.allclose(t9 * (NOON_RATES[1] / NOON_RATES[0]), t6, rtol=1e-9), "times scale inversely with the pair rate"
    assert np.all(np.diff(t6) > 0), "scenarios ordered from the optimistic claim to the tightest bound"
    dq = _dq(SC.entangled_noon())
    t_classical = dq["classical_photons_30mj"] / dq["hvm_wafer_photon_rate"]  # 1 cm² at HVM wafer power
    assert 0.01 < t_classical < 0.1
    assert t6[2] >= 1e16 and t6[3] >= AGE_OF_UNIVERSE_S, "the default bounds exceed 10¹⁶ s and the age of the Universe"
    return {"rows": rows, "t_classical": t_classical, "hvm_rate": dq["hvm_wafer_photon_rate"],
            "classical_photons": dq["classical_photons_30mj"], "ceiling": dq["entangled_flux_ceiling"]}


def noon_ladder_draw(d, th: S.Theme):
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 4.7))
    S.style_axes(ax, th, grid="x")
    ax.set_xscale("log")
    ys = np.arange(len(NOON_SCENARIOS))[::-1] * 1.0
    height = 0.3
    x0 = 1e-2
    for j, row in enumerate(d["rows"]):
        colour = th.s(1 + j)
        off = (0.5 - j) * height * 1.1
        for i, (key, label, is_bound) in enumerate(NOON_SCENARIOS):
            t = row["times"][i]
            y = ys[i] + off
            kw = S.provenance_style("measured" if is_bound else "projection", colour, th)
            ax.barh(y, t - x0, left=x0, height=height, zorder=3, **kw)
            if is_bound:
                ax.annotate("", (t * 2.2, y), (t, y), arrowprops=dict(arrowstyle="-|>", color=colour, lw=S.LINE_PT * 0.8,
                                                                       shrinkA=0, shrinkB=0), zorder=4)
            yrs = t / YEAR_S
            txt = f"{'≥ ' if is_bound else ''}{t:.0e} s ≈ {yrs:.3g} yr".replace("e+", "e")
            ax.text(t * (5.0 if is_bound else 1.6), y, txt, color=th.ink2, fontsize=S.SMALL_PT - 0.5, va="center")
    ax.set_yticks(ys)
    ax.set_yticklabels([lab for _, lab, _ in NOON_SCENARIOS], fontsize=S.SMALL_PT)
    ax.tick_params(axis="y", length=0)
    ax.axvline(d["t_classical"], color=th.ink2, linewidth=S.HAIR_PT * 1.4, linestyle=(0, (4, 3)), zorder=2)
    ax.text(d["t_classical"] * 1.4, ys[0] + 0.62, f"classical 30 mJ/cm² at HVM wafer power: {d['t_classical'] * 1e3:.0f} ms per cm²",
            color=th.ink2, fontsize=S.SMALL_PT, va="bottom")
    S.limit_line(ax, AGE_OF_UNIVERSE_S, th, "age of the Universe", where=0.98, side="left", va="top")
    for t, lab in ((3600.0, "1 h"), (86400.0, "1 day"), (YEAR_S, "1 yr"), (1e3 * YEAR_S, "10³ yr"), (1e6 * YEAR_S, "10⁶ yr"),
                   (1e9 * YEAR_S, "10⁹ yr")):
        ax.axvline(t, color=th.grid, linewidth=S.HAIR_PT, zorder=1)
        ax.text(t, ys[-1] - 0.75, lab, color=th.ink2, fontsize=S.SMALL_PT - 1, ha="center", va="top")
    ax.set_xlim(x0, 1e21)
    ax.set_ylim(ys[-1] - 0.95, ys[0] + 0.95)
    ax.set_xlabel("time to clear 1 cm² of resist (s), p_clear = 0.1 assumed".replace("p_clear", "$p_\\mathrm{clear}$"))
    handles = [Patch(**S.provenance_style("measured", th.s(1), th)), Patch(**S.provenance_style("measured", th.s(2), th)),
               Patch(**S.provenance_style("projection", th.ink2, th))]
    fig.legend(handles, ["10⁶ pairs/s (SPDC class, the default)", "10⁹ pairs/s (hypothetical)",
                         "hollow: disputed early claims; arrow: a lower bound on the time"], loc="lower center",
               fontsize=S.SMALL_PT - 0.5, ncol=3, frameon=False, bbox_to_anchor=(0.5, 0.0), columnspacing=1.4)
    fig.subplots_adjust(left=0.36, right=0.98, top=0.95, bottom=0.24)
    return fig


_register(FigureSpec(
    name="sources-noon-exposure-ladder",
    group=GROUP,
    compute=noon_ladder_compute,
    draw=noon_ladder_draw,
    alt="Log-time bars for clearing 1 cm² with entangled pairs under four cross-section scenarios at 10⁶ and 10⁹ pairs/s: 320 years for the largest disputed claim, at least 10¹⁶ s for the independent bound and beyond the age of the Universe for the tightest, against 45 ms classically.",
    caption="<strong>Exposure time from the pair rate: years at best, longer than the Universe at the bounds.</strong> "
            "<code>entangled_noon(pair_rate_hz=…).derived_quantities()</code> turns the pair rate into the time to "
            "clear 1 cm² with an assumed 10 % of molecules reacting; the two bound scenarios use independent "
            "<i>upper</i> bounds on the entangled two-photon cross-section, so their bars are lower bounds on the time "
            "(arrows), while the hollow bars rest on disputed early claims. The dashed line is the classical reference: "
            "a 30 mJ/cm² dose at the derived 0.67 W wafer power. <em>Model: Entangled-photon (NOON) source 🧪.</em>",
    pages=("sources/entangled-photon.md", "future/quantum-and-exotic.md"),
))


NOON_PITCH_NM, NOON_NA, NOON_SIGMA, NOON_LAM = 100.0, 0.75, 0.5, 157.63


def noon_split_compute(fast: bool):
    huv = native()
    SC = huv.SourceConfig
    src = SC.f2_laser(NOON_SIGMA)
    assert abs(src.wavelength_nm - NOON_LAM) < 1e-9
    optics = huv.OpticsConfig(numerical_aperture=NOON_NA)
    mask = huv.MaskConfig.line_space(NOON_PITCH_NM / 2, NOON_PITCH_NM)
    grid = mask.commensurate_grid(128, 1.5)
    eng = huv.SimulationEngine(src, optics, mask, grid=grid, max_kernels=64)
    classical = eng.compute_aerial_image()
    cl = np.asarray(classical.intensity)
    sq = huv.n_photon_absorption_image(cl, 2)
    res = eng.compute_noon_ideal_image(2, 1.0)
    noon = np.asarray(res.noon_limit.intensity)
    assert np.allclose(np.asarray(res.classical.intensity), cl, atol=1e-9), "the NOON call images the same classical case"
    assert np.allclose(np.asarray(res.n_photon_absorption.intensity), sq, atol=1e-9), "I² = n_photon_absorption_image"
    cutoff_pitch = NOON_LAM / (NOON_NA * (1 + NOON_SIGMA))
    assert cutoff_pitch > NOON_PITCH_NM, "the classical optics does not pass the ±1 orders of this pitch"
    assert classical.image_contrast() < 1e-9 and S.__name__ and np.ptp(sq) < 1e-9, "classical and I² images are flat"
    c_noon = res.noon_limit.image_contrast()
    assert c_noon > 0.5, c_noon
    x = np.asarray(classical.x_nm)
    row_n = noon[noon.shape[0] // 2]
    # the NOON-limit image's dominant period is the mask period (not λ/2 fringes of some other pitch)
    spec = np.abs(np.fft.rfft(row_n - row_n.mean()))
    freqs = np.fft.rfftfreq(row_n.size, d=float(x[1] - x[0]))
    assert abs(1 / freqs[spec.argmax()] - NOON_PITCH_NM) < 1e-6, 1 / freqs[spec.argmax()]
    return {"x": x, "cl": cl[cl.shape[0] // 2], "sq": sq[sq.shape[0] // 2], "noon": row_n,
            "img_cl": cl[::4], "img_sq": sq[::4], "img_noon": noon[::4],
            "c_cl": classical.image_contrast(), "c_sq": float(np.ptp(sq) / (sq.max() + sq.min()) if sq.max() + sq.min() > 0 else 0.0),
            "c_noon": c_noon, "cutoff_pitch": cutoff_pitch, "lam_eff": res.effective_wavelength_nm,
            "r_cl": res.classical_resolution_nm, "r_noon": res.noon_limit_resolution_nm, "status": res.status,
            "pixel": float(x[1] - x[0])}


def noon_split_draw(d, th: S.Theme):
    fig = plt.figure(figsize=(S.WIDE_WIDTH, 4.3))
    gs = fig.add_gridspec(2, 3, height_ratios=[0.55, 1.0], hspace=0.12, wspace=0.3)
    cmap = S.field_cmap()
    x = d["x"]
    panels = (("(a) classical image I", d["img_cl"], d["cl"], d["c_cl"], th.s(1)),
              ("(b) classical two-photon absorption I²", d["img_sq"], d["sq"], d["c_sq"], th.s(2)),
              (f"(c) ideal N00N limit imaged at λ/2 = {d['lam_eff']:.1f} nm", d["img_noon"], d["noon"], d["c_noon"], th.s(3)))
    vmax = max(float(np.max(p[1])) for p in panels)
    for k, (title, img, row, contrast, colour) in enumerate(panels):
        ax = fig.add_subplot(gs[0, k])
        ax.imshow(img, extent=[x[0], x[-1], 0, img.shape[0]], aspect="auto", cmap=cmap, vmin=0, vmax=vmax,
                  interpolation="nearest")
        S.image_axes(ax, th)
        ax.set_xticks([])
        ax.set_yticks([])
        ax.set_title(title, fontsize=S.SMALL_PT + 0.5)
        bx = fig.add_subplot(gs[1, k])
        S.style_axes(bx, th, grid="y")
        bx.plot(x, row, color=colour, linewidth=S.LINE_PT, zorder=3)
        bx.set_xlim(x[0], x[-1])
        bx.set_ylim(0, vmax * 1.1)
        bx.set_xlabel("x (nm)")
        bx.text(0.03, 0.95, f"contrast {contrast:.2f}", transform=bx.transAxes, color=th.ink2, fontsize=S.SMALL_PT,
                va="top")
        if k == 0:
            bx.set_ylabel("exposure (clear field = 1)")
            bx.text(0.03, 0.82, f"cutoff pitch λ/(NA(1+σ)) = {d['cutoff_pitch']:.0f} nm > {NOON_PITCH_NM:.0f} nm:\nno ±1 orders reach the wafer",
                    transform=bx.transAxes, color=th.ink2, fontsize=S.SMALL_PT - 0.5, va="top")
        elif k == 1:
            bx.text(0.03, 0.82, "sharper where there is structure,\nbut nothing to sharpen here", transform=bx.transAxes,
                    color=th.ink2, fontsize=S.SMALL_PT - 0.5, va="top")
        else:
            bx.text(0.03, 0.82, "presumes an undemonstrated N00N state\nand a two-photon resist",
                    transform=bx.transAxes, color=th.ink2, fontsize=S.SMALL_PT - 0.5, va="top")
    fig.subplots_adjust(left=0.07, right=0.98, top=0.9, bottom=0.14)
    return fig


_register(FigureSpec(
    name="sources-noon-honesty-split",
    group=GROUP,
    compute=noon_split_compute,
    draw=noon_split_draw,
    alt="Three image panels of a 100 nm pitch grating at 157.63 nm, NA 0.75, sigma 0.5: the classical image and its "
        "square are flat at zero contrast because the pitch lies below the 140 nm cutoff, while the ideal N00N limit "
        "imaged at 78.8 nm resolves the grating.",
    caption="<strong>Three exposures the engine keeps apart.</strong> A 50 nm / 100 nm grating under the F₂ preset "
            "(σ 0.5) through NA 0.75: the classical image (<code>compute_aerial_image</code>) is flat because the "
            "optics passes no ±1 order below the 140 nm cutoff, and squaring it (<code>n_photon_absorption_image</code>, "
            "classical two-photon absorption) sharpens nothing; only the idealized N00N limit "
            "(<code>compute_noon_ideal_image</code>: the same system imaged at λ/2) prints the pitch, and it presumes "
            "a state preparation and a resist that do not exist. <em>Model: Entangled-photon (NOON) source 🧪 · "
            "Quantum lithography (N-photon / N00N) 🧪.</em>",
    pages=("sources/entangled-photon.md",),
))


# ---------------------------------------------------------------------------
# S3h · Plasma soft-X-ray lasers — lines and coherence length
# ---------------------------------------------------------------------------

SXRL_SCHEMES = (("ar_46nm9", "Ne-like Ar"), ("mo_18nm9", "Ni-like Mo"), ("ag_13nm9", "Ni-like Ag"), ("cd_13nm2", "Ni-like Cd"))
SXRL_REL_LINEWIDTH_ASSUMED = 1e-4


def sxrl_compute(fast: bool):
    huv = native()
    SC = huv.SourceConfig
    lines = []
    rel_grid = np.logspace(-6, -2, 17 if fast else 41)
    for scheme, label in SXRL_SCHEMES:
        src = SC.sxrl(scheme)
        dq = _dq(src)
        lam = src.wavelength_nm
        assert abs(dq["interference_min_half_pitch_nm"] - lam / 4) < 1e-9, "λ/4 two-beam limit"
        assert abs(src.bandwidth_pm - SXRL_REL_LINEWIDTH_ASSUMED * lam * 1e3) < 1e-9
        lc = np.array([SC.sxrl(scheme, rel_linewidth=float(r)).derived_quantity("coherence_length_um") for r in rel_grid])
        assert np.allclose(lc * rel_grid, lam * 1e-3, rtol=1e-9), "l_c = λ / (Δλ/λ)"
        sp = np.array(src.spectrum())
        assert abs(sp[sp[:, 1].argmax(), 0] - lam) < 1e-9
        lines.append(dict(scheme=scheme, label=label, lam=lam, power=src.average_power_w, e_ev=dq["photon_energy_ev"],
                          lc_preset=dq["coherence_length_um"], quarter=lam / 4, lc=lc, mode_count=dq["coherent_mode_count"]))
    # the Mo/Si multilayer band, computed from the materials module (not typed)
    mirror = huv.MultilayerMirror.mo_si(40, 6.9, 0.4)
    lam_grid = np.linspace(12.0, 15.5, 141 if fast else 701)
    refl = np.asarray(mirror.reflectance_curve(lam_grid.tolist()))
    peak_lam, peak_r = mirror.peak(12.5, 14.5)
    fwhm = mirror.bandwidth_fwhm_nm(12.5, 14.5)
    half = refl >= peak_r / 2
    band = (float(lam_grid[half].min()), float(lam_grid[half].max()))
    assert abs((band[1] - band[0]) - fwhm) < 2 * (lam_grid[1] - lam_grid[0]), (band, fwhm)
    lam_by = {l["scheme"]: l["lam"] for l in lines}
    assert band[0] < lam_by["cd_13nm2"] < band[1], "Cd 13.2 nm lies inside the Mo/Si reflectance FWHM"
    assert lam_by["ag_13nm9"] > band[1], "Ag 13.9 nm lies just outside it"
    # HHG neon comb lines as faint ticks (the other table-top route)
    comb = SC.hhg(driver_wavelength_nm=HHG_DRIVER_NM, gas="neon", driver_intensity_w_cm2=4e14, harmonic=59, full_comb=True)
    _, e_q, _ = _comb_lines(comb)
    return {"lines": lines, "rel": rel_grid, "band": band, "peak": (peak_lam, peak_r), "fwhm": fwhm,
            "mirror_lam": lam_grid, "mirror_r": refl, "hhg_lam": HC_EV_NM / e_q}


def sxrl_draw(d, th: S.Theme):
    fig, (ax, bx) = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 4.3), gridspec_kw={"wspace": 0.36, "width_ratios": [1.25, 1]})
    S.style_axes(ax, th, grid="y")
    ax.set_yscale("log")
    ax.axvspan(*d["band"], color=th.band, zorder=0, lw=0)
    ax.text(d["band"][1] + 0.4, 2.2e-3, f"Mo/Si 40 × 6.9 nm:\nreflectance FWHM {d['fwhm']:.2f} nm\naround {d['peak'][0]:.2f} nm "
            f"({d['peak'][1] * 100:.0f} %, computed)", color=th.ink2, fontsize=S.SMALL_PT - 0.5, va="top")
    for lam in d["hhg_lam"]:
        ax.plot([lam, lam], [3.5e-7, 5e-7], color=th.axis, linewidth=S.HAIR_PT, zorder=1)
    ax.text(45.9, 5.5e-7, "ticks: neon HHG harmonics (the other table-top route)", color=th.ink2, fontsize=S.SMALL_PT - 1,
            va="bottom", ha="right")
    items = []
    for k, line in enumerate(d["lines"]):
        colour = th.s(1 + k)
        ax.vlines(line["lam"], 3e-7, line["power"], color=colour, linewidth=S.LINE_PT, zorder=3)
        S.dot(ax, [line["lam"]], [line["power"]], colour, th, size_px=8.0, zorder=4)
        text = f"{line['label']} {line['lam']:.1f} nm: {fmt_si(line['power'], 'W')}"
        if line["lam"] > 40:  # Ne-like Ar sits at the right edge: label to its left
            ax.text(line["lam"] - 0.7, line["power"], text, color=th.ink2, fontsize=S.SMALL_PT - 0.5, ha="right", va="center")
        else:
            items.append((line["lam"], line["power"], text))
    S.end_labels(ax, items, th, min_gap_px=13, dx_px=8, fontsize=S.SMALL_PT - 0.5)
    ax.set_xlim(10, 50)
    ax.set_ylim(3e-7, 3e-3)
    ax.set_xlabel("wavelength (nm)")
    ax.set_ylabel("preset average power (W, reported anchor)")
    ax.set_title("(a) Four fixed atomic lines, one multilayer band")
    S.style_axes(bx, th, grid="both")
    bx.set_xscale("log")
    bx.set_yscale("log")
    for k, line in enumerate(d["lines"]):
        colour = th.s(1 + k)
        bx.plot(d["rel"], line["lc"], color=colour, linewidth=S.LINE_PT, zorder=3)
        S.dot(bx, [SXRL_REL_LINEWIDTH_ASSUMED], [line["lc_preset"]], colour, th, filled=False, size_px=8.0, zorder=5)
        bx.plot([d["rel"][0], d["rel"][-1]], [line["quarter"] * 1e-3] * 2, color=colour, linewidth=S.HAIR_PT * 1.4,
                linestyle=(0, (2, 2)), zorder=2)
    bx.text(1.2e-6, d["lines"][0]["quarter"] * 1e-3 * 1.5, "dotted: λ/4 two-beam interference half-pitch (3.3–11.7 nm)",
            color=th.ink2, fontsize=S.SMALL_PT - 0.5, va="bottom")
    bx.text(SXRL_REL_LINEWIDTH_ASSUMED * 1.3, d["lines"][0]["lc_preset"] * 1.6, "open: the presets' assumed\nΔλ/λ = 10⁻⁴",
            color=th.ink2, fontsize=S.SMALL_PT - 0.5, va="bottom")
    bx.set_xlim(d["rel"][0], d["rel"][-1])
    bx.set_ylim(1e-3, 1e5)
    bx.set_xlabel("relative linewidth Δλ/λ")
    bx.set_ylabel("longitudinal coherence length λ²/Δλ (µm)")
    bx.set_title("(b) Coherence length against linewidth")
    fig.subplots_adjust(left=0.08, right=0.98, top=0.9, bottom=0.14)
    return fig


_register(FigureSpec(
    name="sources-sxrl-lines",
    group=GROUP,
    compute=sxrl_compute,
    draw=sxrl_draw,
    alt="Left, the four soft-X-ray laser presets as stems at 46.9, 18.9, 13.9 and 13.2 nm with reported powers from 1 µW to 0.16 mW and the computed Mo/Si reflectance band near 13.48 nm; right, coherence length against relative linewidth with the assumed 10⁻⁴ marked and the λ/4 limit.",
    caption="<strong>Fixed lines, assumed widths.</strong> (a) The lasing wavelengths are atomic transitions "
            "(<code>sxrl(scheme).wavelength_nm</code>) and the stem heights the reported average powers the presets "
            "reproduce; the band is the reflectance FWHM of a 40-bilayer Mo/Si mirror computed by the materials module "
            "— Cd 13.2 nm falls inside it, Ag 13.9 nm just outside. (b) <code>coherence_length_um</code> is λ²/Δλ "
            "at any relative linewidth; the presets assume 10⁻⁴ (open markers), an order-of-magnitude input, while "
            "the λ/4 two-beam limit is exact. <em>Model: Plasma soft-X-ray laser ✅ (🔶 output parameters: powers "
            "reproduce reports, linewidth and durations assumed) · EUV/BEUV multilayer mirrors ✅.</em>",
    pages=("sources/soft-xray-laser.md",),
    extra={"model_rows": {"Plasma soft-X-ray laser": "Plasma soft-X-ray laser (Ne-like Ar 46.9, Ni-like Ag 13.9 / Cd 13.2 / Mo 18.9 nm)"}},
))


# ---------------------------------------------------------------------------
# S3i · Smith–Purcell — angular tuning and the evanescent reach
# ---------------------------------------------------------------------------

SP_ENERGIES_KEV = (30.0, 100.0)
SP_VDW_PERIOD_NM = 0.335  # graphite interlayer spacing, the page's second row


def sp_compute(fast: bool):
    huv = native()
    SC = huv.SourceConfig
    preset = SC.smith_purcell()
    dq = _dq(preset)
    period = (dq["tuning_max_wavelength_nm"] - dq["tuning_min_wavelength_nm"]) / 2  # (a/m)(1/β+1) − (a/m)(1/β−1) = 2a/m
    assert abs(period * (1 / dq["electron_beta"]) - (dq["tuning_max_wavelength_nm"] + dq["tuning_min_wavelength_nm"]) / 2) < 1e-9
    assert abs(preset.wavelength_nm - 13.5) < 1e-9
    theta = np.linspace(0.5, 179.5, 61 if fast else 359)  # the engine's open interval (0, 180)°
    curves = []
    for t_kev in SP_ENERGIES_KEV:
        lam = np.array([SC.smith_purcell(electron_energy_kev=t_kev, observation_angle_deg=float(a), grating_period_nm=period).wavelength_nm
                        for a in theta])
        s90 = SC.smith_purcell(electron_energy_kev=t_kev, observation_angle_deg=90.0, grating_period_nm=period)
        q = _dq(s90)
        beta = q["electron_beta"]
        assert np.allclose(lam, period * (1 / beta - np.cos(np.radians(theta))), rtol=1e-9), "λ = (a/m)(1/β − cos θ)"
        assert abs(period * (1 / beta - 1) - q["tuning_min_wavelength_nm"]) < 1e-9, "tuning range = (a/m)(1/β ∓ 1)"
        assert abs(period * (1 / beta + 1) - q["tuning_max_wavelength_nm"]) < 1e-9
        assert lam.min() > q["tuning_min_wavelength_nm"] and lam.max() < q["tuning_max_wavelength_nm"]
        curves.append(dict(t_kev=t_kev, beta=beta, lam=lam, lam90=s90.wavelength_nm, h_int=q["interaction_height_nm"],
                           slope=q["tuning_slope_nm_per_deg"]))
    assert abs(curves[0]["lam90"] - 13.5) < 1e-6
    h = np.linspace(0.0, 5.0, 51 if fast else 251)
    reach = []
    for label, kw in (("13.5 nm from 30 keV (a = 4.43 nm)", {}), (f"1.02 nm from 30 keV (a = {SP_VDW_PERIOD_NM} nm, vdW crystal)", {"grating_period_nm": SP_VDW_PERIOD_NM})):
        base = SC.smith_purcell(**kw)
        h_int = _dq(base)["interaction_height_nm"]
        fac = np.array([_dq(SC.smith_purcell(impact_height_nm=float(x), **kw))["evanescent_coupling_factor"] for x in h])
        assert np.allclose(fac, np.exp(-h / h_int), rtol=1e-9), "exp(−h/h_int)"
        reach.append(dict(label=label, lam=base.wavelength_nm, h_int=h_int, fac=fac, at5=float(np.exp(-5.0 / h_int))))
    assert 1e-6 < reach[0]["at5"] < 2e-6, reach[0]["at5"]
    return {"theta": theta, "curves": curves, "period": period, "h": h, "reach": reach,
            "power_w": preset.average_power_w, "gap": dq["hvm_power_gap"]}


def sp_draw(d, th: S.Theme):
    fig, (ax, bx) = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 4.2), gridspec_kw={"wspace": 0.34})
    S.style_axes(ax, th, grid="both")
    items = []
    for k, c in enumerate(d["curves"]):
        colour = th.s(1 + k)
        ax.plot(d["theta"], c["lam"], color=colour, linewidth=S.LINE_PT, zorder=3)
        ax.text(176.0, c["lam"][-1] + 0.35, f"{c['t_kev']:.0f} keV, β = {c['beta']:.3f}", color=colour, fontsize=S.SMALL_PT,
                ha="right", va="bottom")
        S.dot(ax, [90.0], [c["lam90"]], colour, th, size_px=8.0, zorder=5)
    S.limit_line(ax, 90.0, th, "90° design point", where=0.97, side="left")
    c0 = d["curves"][0]
    ax.annotate(f"13.5 nm: slope {c0['slope']:.3f} nm/deg", (90.0, c0["lam90"]), xytext=(84.0, 9.0), textcoords="data",
                ha="right", va="center", color=th.ink2, fontsize=S.SMALL_PT,
                arrowprops=dict(arrowstyle="-", color=th.axis, lw=S.HAIR_PT, shrinkA=0, shrinkB=3))
    ax.set_xlim(0, 180)
    ax.set_xticks([0, 45, 90, 135, 180])
    ax.set_ylim(0, 20.5)
    ax.set_xlabel("observation angle θ from the beam (°)")
    ax.set_ylabel("emitted wavelength (nm)")
    ax.set_title(f"(a) λ = (a/m)(1/β − cos θ), period a = {d['period']:.3f} nm, m = 1")
    S.style_axes(bx, th, grid="both")
    bx.set_yscale("log")
    items = []
    for k, r in enumerate(d["reach"]):
        colour = th.s(1 + k)
        bx.plot(d["h"], r["fac"], color=colour, linewidth=S.LINE_PT, zorder=3)
        items.append((d["h"][-1], r["fac"][-1], f"{r['label']}: h_int = {r['h_int']:.3f} nm".replace("h_int", "$h_\\mathrm{int}$")))
    S.end_labels(bx, items, th, dx_px=6, fontsize=S.SMALL_PT - 0.5)
    S.dot(bx, [5.0], [d["reach"][0]["at5"]], th.s(1), th, size_px=8.0, zorder=5)
    bx.annotate(f"h = 5 nm above the grating:\n×{d['reach'][0]['at5']:.1e}", (5.0, d["reach"][0]["at5"]), xytext=(-12, -30),
                textcoords="offset points", ha="right", va="top", color=th.ink2, fontsize=S.SMALL_PT,
                arrowprops=dict(arrowstyle="-", color=th.axis, lw=S.HAIR_PT, shrinkA=0, shrinkB=3))
    bx.set_xlim(0, 5)
    bx.set_ylim(1e-80, 3)
    bx.set_yticks([1, 1e-20, 1e-40, 1e-60, 1e-80])
    bx.set_xlabel("impact height h above the structure (nm)")
    bx.set_ylabel("evanescent coupling exp(−h / h_int)".replace("h_int", "$h_\\mathrm{int}$"))
    bx.set_title("(b) The electron's field reaches a fraction of a nanometre")
    fig.subplots_adjust(left=0.08, right=0.8, top=0.9, bottom=0.14)
    return fig


_register(FigureSpec(
    name="sources-smith-purcell-tuning",
    group=GROUP,
    compute=sp_compute,
    draw=sp_draw,
    alt="Left, Smith–Purcell wavelength against observation angle for a 4.43 nm period at 30 and 100 keV, 13.5 nm at the 90° design point; right, the evanescent coupling exp(−h/h_int) falling to about 10⁻⁶ at 5 nm for the preset's 0.37 nm reach and far faster for a 0.028 nm reach.",
    caption="<strong>The kinematics are exact; the reach is the problem.</strong> (a) <code>smith_purcell(…)."
            "wavelength_nm</code> over the observation angle for the preset's derived 4.433 nm period: 13.5 nm at 90° "
            "from 30 keV electrons, tunable from 9.1 to 17.9 nm, and 8.1 nm at 90° from 100 keV. (b) The emitted "
            "intensity falls as exp(−h/h<sub>int</sub>) with the electron's distance from the structure, "
            "h<sub>int</sub> = βγλ/4π = 0.37 nm here (0.028 nm for the 1.02 nm van-der-Waals case), so a beam 5 nm "
            "above the grating loses a factor 10⁶. <em>Model: Smith–Purcell free-electron grating 🧪 (✅ kinematics; "
            "the sub-nW power estimate with its assumed coupling is not drawn).</em>",
    pages=("sources/smith-purcell.md",),
))


# ---------------------------------------------------------------------------
# S3j · X-ray tube and betatron (the split of the former shared tube/betatron spectra spec) + tube LIGA
# ---------------------------------------------------------------------------

TUBE_ANODES = ("W", "Mo", "Cu", "Rh")
TUBE_DISTANCE_MM = 100.0
BE_WINDOWS_UM = (100.0, 250.0, 500.0)


def _line_groups(e, flux):
    """Index groups of bins that stand above the continuum (characteristic lines)."""
    logf = np.log(np.maximum(flux, 1e-300))
    base = np.array([np.median(logf[max(0, i - 6): i + 7]) for i in range(len(logf))])
    peaks = np.where(logf - base > math.log(2.5))[0]
    groups, cur = [], []
    for i in peaks:
        if cur and i - cur[-1] > 2:
            groups.append(cur)
            cur = []
        cur.append(int(i))
    if cur:
        groups.append(cur)
    return groups


def tube_compute(fast: bool):
    huv = native()
    SC = huv.SourceConfig
    n_bins = 200 if fast else 600
    tubes = []
    for anode in TUBE_ANODES:
        src = SC.xray_tube(anode)
        dq = _dq(src)
        fd = np.array(src.spectral_flux_density(TUBE_DISTANCE_MM, n_bins))
        e, flux = fd[:, 0], fd[:, 1]
        e0 = HC_EV_NM / dq["duane_hunt_min_wavelength_nm"] / 1e3  # kVp in keV
        assert np.all(e <= e0 + 1e-9), "no photon above the Duane–Hunt limit"
        assert flux[e > 0.98 * e0].max() < 0.01 * flux.max(), "the continuum vanishes at the Duane–Hunt limit"
        groups = _line_groups(e, flux)
        u_k = dq["k_shell_overvoltage"]
        e_k = e0 / u_k
        if u_k > 1:
            assert any(e[g[0]] > 0.7 * e_k for g in groups), f"{anode}: K lines excited (U = {u_k:.2f})"
            series = "K lines"
        else:
            assert not any(e[g[0]] > 0.7 * e_k for g in groups), f"{anode}: no K lines below the K edge (U = {u_k:.2f})"
            series = "L lines"
        tubes.append(dict(anode=anode, e=e, flux=flux, e0=e0, kvp=e0, line_groups=groups, series=series, u_k=u_k,
                          e_k=e_k, power=src.average_power_w, mean_kev=dq["mean_photon_energy_kev"],
                          line_fraction=dq["characteristic_photon_fraction"], label=repr(src)))
    # Be-window transmission from the engine: the filtered spectrum over the unfiltered one, same bins
    w_src = SC.xray_tube("W")
    bare = np.array(SC.xray_tube("W", be_window_um=0.0).spectral_flux_density(TUBE_DISTANCE_MM, n_bins))
    windows = []
    for t_um in BE_WINDOWS_UM:
        fd = np.array(SC.xray_tube("W", be_window_um=t_um).spectral_flux_density(TUBE_DISTANCE_MM, n_bins))
        assert np.allclose(fd[:, 0], bare[:, 0])
        t = fd[:, 1] / np.maximum(bare[:, 1], 1e-300)
        cont = np.array([i for i in range(len(t)) if not any(i in g for g in _line_groups(bare[:, 0], bare[:, 1]))])
        tc = t[cont]
        assert np.all(np.diff(tc) >= -1e-9) and tc.max() <= 1 + 1e-9, "Be transmission rises monotonically with energy"
        windows.append(dict(t_um=t_um, e=fd[:, 0], t=t, cont=cont))
    t250 = windows[[w["t_um"] for w in windows].index(250.0)]
    e_half = float(np.interp(0.5, t250["t"][t250["cont"]], t250["e"][t250["cont"]]))
    return {"tubes": tubes, "windows": windows, "e_half_250": e_half, "distance_mm": TUBE_DISTANCE_MM,
            "preset_repr": repr(w_src)}


def tube_draw(d, th: S.Theme):
    fig, (ax, bx) = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 4.3), gridspec_kw={"wspace": 0.34, "width_ratios": [1.7, 1]})
    S.style_axes(ax, th, grid="y")
    ax.set_yscale("log")
    handles, labels = [], []
    for k, t in enumerate(d["tubes"]):
        colour = th.s(1 + k)
        ax.plot(t["e"], t["flux"], color=colour, linewidth=S.LINE_PT * 0.85, zorder=3)
        handles.append(Line2D([], [], color=colour))
        labels.append(f"{t['anode']} anode, {t['kvp']:.0f} kV: {t['series']}, mean {t['mean_kev']:.1f} keV, {fmt_si(t['power'], 'W')} into 4π")
        ax.plot([t["e0"], t["e0"]], [1e5, 2.2e5], color=colour, linewidth=S.LINE_PT, zorder=4)
        ax.text(t["e0"], 2.5e5, f"{t['kvp']:.0f} kV", color=colour, fontsize=S.SMALL_PT - 1, ha="center", va="bottom")
        if t["line_groups"]:
            g = max(t["line_groups"], key=lambda g: t["flux"][g].max())
            i = g[int(np.argmax(t["flux"][g]))]
            ax.annotate(f"{t['anode']} {t['series'][0]}", (t["e"][i], t["flux"][i]), xytext=(0, 9), textcoords="offset points",
                        ha="center", color=colour, fontsize=S.SMALL_PT - 0.5)
    ax.text(1.2, 1.6e5, "Be window\nroll-off", color=th.ink2, fontsize=S.SMALL_PT - 0.5, va="bottom")
    ax.text(45, 1.3e5, "Duane–Hunt limits ↑", color=th.ink2, fontsize=S.SMALL_PT - 0.5, va="bottom", ha="center")
    ax.legend(handles, labels, loc="upper right", fontsize=S.SMALL_PT - 1)
    ax.set_xlim(0, 63)
    ax.set_ylim(1e5, 2e11)
    ax.set_xlabel("photon energy (keV)")
    ax.set_ylabel(f"flux density at {d['distance_mm']:.0f} mm (photons s⁻¹ mm⁻² keV⁻¹)")
    ax.set_title("(a) Kramers continuum + characteristic lines, 250 µm Be")
    S.style_axes(bx, th, grid="both")
    w_handles, w_labels = [], []
    for k, w in enumerate(d["windows"]):
        main = w["t_um"] == 250.0
        style = dict(color=th.s(1) if main else th.ink2, linewidth=S.LINE_PT if main else S.HAIR_PT * 1.4,
                     linestyle="-" if main else (0, (4, 3)))
        bx.plot(w["e"][w["cont"]], w["t"][w["cont"]], zorder=3 if main else 2, **style)
        w_handles.append(Line2D([], [], **style))
        w_labels.append(f"{w['t_um']:.0f} µm Be" + (" (the presets)" if main else ""))
    bx.legend(w_handles, w_labels, loc="center right", fontsize=S.SMALL_PT - 0.5)
    S.limit_line(bx, d["e_half_250"], th)
    bx.text(d["e_half_250"] + 0.7, 0.14, f"T = ½ at {d['e_half_250']:.1f} keV (250 µm)", color=th.ink2, fontsize=S.SMALL_PT,
            va="center")
    bx.set_xlim(0, 30)
    bx.set_ylim(0, 1.02)
    bx.set_xlabel("photon energy (keV)")
    bx.set_ylabel("Be window transmission T(E)")
    bx.set_title("(b) The exit window removes the soft photons")
    fig.subplots_adjust(left=0.08, right=0.98, top=0.9, bottom=0.14)
    return fig


_register(FigureSpec(
    name="sources-xray-tube-spectra",
    group=GROUP,
    compute=tube_compute,
    draw=tube_draw,
    alt="Left, log-scale tube spectra for W 60 kV, Mo 50 kV, Cu 40 kV and Rh 50 kV anodes: bremsstrahlung continua ending at each tube voltage, K lines for Mo, Cu and Rh and L lines for W; right, the Be window transmission for 100, 250 and 500 µm, reaching one half near 3.4 keV for 250 µm.",
    caption="<strong>What a laboratory tube offers LIGA.</strong> (a) <code>spectral_flux_density(100 mm)</code> "
            "of the four anode presets: Kramers bremsstrahlung ending at each Duane–Hunt limit, K lines where the tube "
            "voltage exceeds the K edge (Mo, Cu, Rh) and only L lines for tungsten at 60 kV (K edge 69.5 keV), the "
            "soft end cut by the 250 µm Be window. (b) The window transmission, obtained from the engine as the "
            "filtered over the unfiltered spectrum. <em>Model: Hard X-ray tube (W / Mo / Cu / Rh, 5–300 kV) 🔶 "
            "(✅ line energies, edges, Duane–Hunt limit and Be filtration; the Kramers continuum, line yields and "
            "absolute flux are simplified: no anode self-absorption, heel effect or backscatter).</em>",
    pages=("sources/xray-tube.md", "processes/liga-deep-xray.md"),
))


K53_HEAD_A = 0.5 * math.gamma(5 / 3) * 2 ** (5 / 3)  # K_{5/3}(x) → A x^{−5/3} as x → 0
K53_X0 = 1e-6


def _k53_tail(y: np.ndarray) -> np.ndarray:
    """∫_y^∞ K_{5/3}(x) dx in plain numpy: the integral representation of K_ν on [K53_X0, 80] plus the
    analytic small-x head (3/2)·A·y^{−2/3} below K53_X0."""
    t = np.linspace(0.0, 20.0, 8001)
    x = np.logspace(math.log10(K53_X0), math.log10(80.0), 1600)
    k53 = np.trapezoid(np.exp(-x[:, None] * np.cosh(t)[None, :]) * np.cosh(5 / 3 * t)[None, :], t, axis=1)
    tail = np.concatenate([np.cumsum((0.5 * (k53[1:] + k53[:-1]) * np.diff(x))[::-1])[::-1], [0.0]])  # ∫_x^∞ K
    y = np.asarray(y, dtype=float)
    out = np.interp(y, x, tail)
    low = y < K53_X0
    out[low] = tail[0] + 1.5 * K53_HEAD_A * (y[low] ** (-2 / 3) - K53_X0 ** (-2 / 3))
    return out


def bm_universal_shape(y: np.ndarray) -> np.ndarray:
    """S(y) = y ∫_y^∞ K_{5/3}(x) dx (the universal bending-magnet shape; the engine's ``bm_universal_flux``).

    Evaluated here in numpy; the betatron figure checks it against the engine's photon bins and the
    synchrotron figure against the engine's flux and power per mrad of the LIGA bending magnet.
    """
    return y * _k53_tail(y)


def _k53_head_integral(y0: float) -> float:
    """∫_0^{y0} S(y)/y dy = ∫_0^{y0} (3/2)·A·y^{−2/3} dy = (9/2)·A·y0^{1/3} for y0 ≪ 1 (the y^{−2/3} head)."""
    return 4.5 * K53_HEAD_A * y0 ** (1 / 3)


def betatron_spectrum_compute(fast: bool):
    huv = native()
    src = huv.SourceConfig.betatron()
    dq = _dq(src)
    ec = dq["critical_energy_kev"]
    sp = np.array(src.xray_spectrum(120 if fast else 400))
    y_bins, w = sp[:, 0] / ec, sp[:, 1]
    assert abs(w.sum() - 1) < 1e-9
    ratio = y_bins[1:] / y_bins[:-1]
    assert np.allclose(ratio, ratio[0], rtol=1e-9), "log-spaced bins"
    assert 0.09 < y_bins[0] < 0.11 and 7.5 < y_bins[-1] < 8.2, "the [0.1, 8] E_c LIGA window"
    s_bins = bm_universal_shape(y_bins)
    assert np.allclose(w / w.sum(), s_bins / s_bins.sum(), rtol=3e-3), "engine photons per log bin ∝ S(y)"
    y = np.logspace(-6, math.log10(30.0), 900)
    s = bm_universal_shape(y)
    head = _k53_head_integral(y[0])
    total = head + np.trapezoid(s / y, y)
    assert abs(total - 5 * math.pi / 3) < 0.01, total  # ∫_0^∞ S/y dy = 5π/3
    window = (y >= 0.1) & (y <= 8.0)
    frac_window = np.trapezoid((s / y)[window], y[window]) / (5 * math.pi / 3)
    frac_below = (head + np.trapezoid((s / y)[y <= 0.1], y[y <= 0.1])) / (5 * math.pi / 3)
    assert abs(frac_window - 0.46) < 0.01 and abs(frac_below - 0.53) < 0.01, (frac_window, frac_below)
    mean_ratio = np.trapezoid(s, y) / total
    assert abs(mean_ratio - 8 / (15 * math.sqrt(3))) < 2e-3 and abs(dq["mean_photon_energy_kev"] / ec - mean_ratio) < 2e-3
    fd = np.array(src.spectral_flux_density(1000.0, 120 if fast else 400))
    e, flux = fd[:, 0], fd[:, 1]
    # scale the closed form to the engine's absolute on-axis flux density (same photon count in the window)
    de = np.diff(e).mean()
    n_win = (flux * de).sum()
    s_curve = s / y / ec  # per keV
    scale = n_win / np.trapezoid(s_curve[window], y[window] * ec)
    return {"ec": ec, "y": y, "closed": s_curve * scale, "e": e, "flux": flux, "frac_window": frac_window,
            "frac_below": frac_below, "mean_kev": dq["mean_photon_energy_kev"], "mean_ratio": dq["mean_photon_energy_kev"] / ec,
            "photons_shot": dq["photons_per_shot"], "power_w": src.average_power_w, "divergence": dq["divergence_half_angle_mrad"],
            "k": dq["betatron_strength_k"]}


def betatron_spectrum_draw(d, th: S.Theme):
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 4.4))
    S.style_axes(ax, th, grid="both")
    ax.set_xscale("log")
    ax.set_yscale("log")
    ec = d["ec"]
    ax.axvspan(0.1, 8.0, color=th.band, zorder=0, lw=0)
    inside = (d["y"] >= 0.1) & (d["y"] <= 8.0)
    ax.plot(d["y"][~inside & (d["y"] < 0.1)], d["closed"][~inside & (d["y"] < 0.1)], color=th.ink2, linewidth=S.HAIR_PT * 1.4,
            linestyle=(0, (4, 3)), zorder=2)
    ax.plot(d["y"][~inside & (d["y"] > 8.0)], d["closed"][~inside & (d["y"] > 8.0)], color=th.ink2, linewidth=S.HAIR_PT * 1.4,
            linestyle=(0, (4, 3)), zorder=2)
    ax.plot(d["e"] / ec, d["flux"], color=th.s(1), linewidth=S.LINE_PT, zorder=3)
    S.limit_line(ax, 1.0, th, f"E_c = {ec:.2f} keV".replace("E_c", "$E_c$"), where=0.97, side="right")
    S.limit_line(ax, d["mean_ratio"], th, f"$\\langle E \\rangle$ = {d['mean_ratio']:.3f} $E_c$ = {d['mean_kev']:.2f} keV",
                 where=0.97, side="left")
    ax.text(0.97, 0.96, f"[0.1, 8] $E_c$ window the LIGA module samples:\n{d['frac_window'] * 100:.0f} % of the photons "
            f"({d['frac_below'] * 100:.0f} % lie below 0.1 $E_c$)", transform=ax.transAxes, ha="right",
            va="top", color=th.ink2, fontsize=S.SMALL_PT)
    ax.legend([Line2D([], [], color=th.s(1)), Line2D([], [], color=th.ink2, linestyle=(0, (4, 3)))],
              [f"engine spectral_flux_density(1 m): {d['photons_shot']:.2g} photons/shot, {fmt_si(d['power_w'], 'W')} at 10 Hz",
               "universal S(y)/y, closed form outside the window"], loc="lower left", fontsize=S.SMALL_PT - 0.5)
    ax.set_xlim(1e-2, 30)
    ax.set_ylim(d["closed"][inside].min() * 0.1, d["closed"].max() * 3)
    ax.set_xlabel("photon energy E / E_c".replace("E_c", "$E_c$"))
    ax.set_ylabel("on-axis flux density at 1 m (photons s⁻¹ mm⁻² keV⁻¹)")
    ax.set_title(f"Betatron preset (200 MeV, 10¹⁹ cm⁻³, K = {d['k']:.1f}): a synchrotron-like spectrum")
    fig.subplots_adjust(left=0.1, right=0.98, top=0.9, bottom=0.13)
    return fig


_register(FigureSpec(
    name="sources-betatron-spectrum",
    group=GROUP,
    compute=betatron_spectrum_compute,
    draw=betatron_spectrum_draw,
    alt="Log-log betatron flux density against E/E_c for E_c = 8.07 keV: the engine's spectrum inside the 0.1–8 E_c window the LIGA module samples, holding 46 % of the photons, the universal synchrotron shape dashed outside it and the mean photon energy at 0.308 E_c.",
    caption="<strong>A synchrotron spectrum from a plasma wiggler.</strong> The preset's "
            "<code>spectral_flux_density(1000 mm)</code> on the log-energy bins of the [0.1, 8] E<sub>c</sub> window "
            "the LIGA module samples (E<sub>c</sub> = 8.07 keV, K = 8.3, 10 periods), with the universal S(y)/y "
            "shape continued outside it: the window holds 46 % of the photons and the mean photon energy is "
            "0.308 E<sub>c</sub>, both evaluated from the K<sub>5/3</sub> integral and checked against the engine's "
            "bins. One electron energy, one amplitude, a flat-top cone of half-angle K/γ. "
            "<em>Model: Laser-wakefield betatron X-rays 🧪.</em>",
    pages=("sources/betatron.md",),
))


BETATRON_DENSITIES = (1e18, 1e19)
BETATRON_PRESET = (200.0, 1e19)


def betatron_scaling_compute(fast: bool):
    huv = native()
    SC = huv.SourceConfig
    energies = np.logspace(1.0, 3.0, 41 if fast else 121)  # MeV
    m_e_mev = 0.51099895
    curves = []
    for n_e in BETATRON_DENSITIES:
        ec = np.full(energies.shape, np.nan)
        kk = np.full(energies.shape, np.nan)
        for i, e in enumerate(energies):
            try:
                q = _dq(SC.betatron(electron_energy_mev=float(e), plasma_density_cm3=n_e))
            except Exception:  # noqa: BLE001 — the model rejects K < 1 (undulator regime)
                continue
            ec[i], kk[i] = q["critical_energy_kev"], q["betatron_strength_k"]
        ok = np.isfinite(ec)
        gamma = 1 + energies / m_e_mev
        # the page's practical forms, checked against the engine where it accepts the point
        k_form = 1.33e-10 * np.sqrt(gamma * n_e) * 1.0
        ec_form = 5.24e-24 * gamma ** 2 * n_e * 1.0
        assert np.allclose(kk[ok], k_form[ok], rtol=5e-3), "K = 1.33e-10 √(γ n_e) r_β"
        assert np.allclose(ec[ok], ec_form[ok], rtol=5e-3), "E_c = 5.24e-24 γ² n_e r_β keV"
        assert np.all(kk[ok] >= 1.0) and np.all(k_form[~ok] < 1.0 + 1e-3), "rejected points are exactly the K < 1 ones"
        e_k1 = float(np.interp(1.0, k_form, energies))  # electron energy where K = 1
        curves.append(dict(n_e=n_e, e=energies, ec=ec, k=kk, ec_form=ec_form, k_form=k_form, e_k1=e_k1, ok=ok))
    e200 = _dq(SC.betatron(electron_energy_mev=200.0, plasma_density_cm3=1e19))["critical_energy_kev"]
    e400 = _dq(SC.betatron(electron_energy_mev=400.0, plasma_density_cm3=1e19))["critical_energy_kev"]
    g200, g400 = 1 + 200 / m_e_mev, 1 + 400 / m_e_mev
    assert abs(e400 / e200 - (g400 / g200) ** 2) < 1e-6, "E_c ∝ γ² at fixed density"
    preset = _dq(SC.betatron())
    return {"curves": curves, "preset": (BETATRON_PRESET[0], preset["critical_energy_kev"], preset["betatron_strength_k"]),
            "ratio_400_200": e400 / e200}


def betatron_scaling_draw(d, th: S.Theme):
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 4.4))
    S.style_axes(ax, th, grid="both")
    ax.set_xscale("log")
    ax.set_yscale("log")
    items = []
    for k, c in enumerate(d["curves"]):
        colour = th.s(1 + k)
        ax.plot(c["e"][c["ok"]], c["ec"][c["ok"]], color=colour, linewidth=S.LINE_PT, zorder=3)
        ax.plot(c["e"][~c["ok"]], c["ec_form"][~c["ok"]], color=colour, linewidth=S.HAIR_PT * 1.4, linestyle=(0, (2, 2)), zorder=2)
        exp = int(round(math.log10(c["n_e"])))
        items.append((c["e"][-1], c["ec"][-1], f"n_e = 10^{exp} cm⁻³".replace("n_e", "$n_e$").replace(f"10^{exp}", f"$10^{{{exp}}}$")))
        if (~c["ok"]).any():  # the K = 1 boundary lies inside the drawn range
            ec_k1 = float(np.interp(c["e_k1"], c["e"], c["ec_form"]))
            S.dot(ax, [c["e_k1"]], [ec_k1], colour, th, filled=False, size_px=7.5, zorder=4)
            ax.annotate(f"K = 1 at {c['e_k1']:.0f} MeV", (c["e_k1"], ec_k1), xytext=(8, -12), textcoords="offset points",
                        color=th.ink2, fontsize=S.SMALL_PT - 0.5)
    S.end_labels(ax, items, th, dx_px=6)
    ax.text(0.03, 0.97, "dotted: K < 1 (undulator regime) — the model rejects these points;\nthe narrow-harmonic spectrum there is not described",
            transform=ax.transAxes, va="top", color=th.ink2, fontsize=S.SMALL_PT)
    e_p, ec_p, k_p = d["preset"]
    S.dot(ax, [e_p], [ec_p], th.s(2), th, marker="D", size_px=8.5, zorder=5)
    ax.annotate(f"preset: {e_p:.0f} MeV $\\rightarrow$ $E_c$ = {ec_p:.2f} keV, K = {k_p:.1f}", (e_p, ec_p), xytext=(-12, 10),
                textcoords="offset points", ha="right", color=th.ink2, fontsize=S.SMALL_PT,
                arrowprops=dict(arrowstyle="-", color=th.axis, lw=S.HAIR_PT, shrinkA=0, shrinkB=3))
    ax.text(0.97, 0.04, f"$E_c \\propto \\gamma^2$ at fixed density and amplitude: 200 $\\rightarrow$ 400 MeV is ×{d['ratio_400_200']:.2f}",
            transform=ax.transAxes, ha="right", va="bottom", color=th.ink2, fontsize=S.SMALL_PT)
    ax.set_xlim(10, 1000)
    ax.set_ylim(1.5e-3, 500)
    ax.set_xlabel("electron energy (MeV)")
    ax.set_ylabel("critical energy E_c (keV)".replace("E_c", "$E_c$"))
    ax.set_title("Critical energy against electron energy, r_β = 1 µm".replace("r_β", "$r_\\beta$"))
    fig.subplots_adjust(left=0.1, right=0.86, top=0.9, bottom=0.13)
    return fig


_register(FigureSpec(
    name="sources-betatron-scaling",
    group=GROUP,
    compute=betatron_scaling_compute,
    draw=betatron_scaling_draw,
    alt="Log-log critical energy against electron energy from 10 to 1000 MeV at plasma densities of 10¹⁸ and 10¹⁹ cm⁻³ and 1 µm amplitude: two γ² lines, the preset at 200 MeV and 8.07 keV, and dotted low-energy ends where the strength parameter K drops below one.",
    caption="<strong>Doubling the electron energy quadruples the critical energy.</strong> "
            "<code>betatron(electron_energy_mev=…, plasma_density_cm3=…)</code> at the preset's 1 µm amplitude: "
            "E<sub>c</sub> = (3/2) K γ² ħω<sub>β</sub> ∝ γ² n<sub>e</sub> r<sub>β</sub>, so the two densities are "
            "parallel lines a decade apart; the model accepts a point only in the wiggler regime K ≥ 1 and the dotted "
            "ends are the closed form where it refuses. <em>Model: Laser-wakefield betatron X-rays 🧪.</em>",
    pages=("sources/betatron.md",),
))


TUBE_LIGA_THICKNESS_UM = 200.0
TUBE_LIGA_DISTANCES_MM = (25.0, 35.0, 50.0, 70.0, 100.0, 140.0, 200.0, 280.0, 400.0)


def tube_liga_compute(fast: bool):
    huv = native()
    import highuvlith.api as api  # noqa: PLC0415

    src = huv.SourceConfig.xray_tube("W")
    n_bins = 100 if fast else 200
    kw = dict(resist_thickness_um=TUBE_LIGA_THICKNESS_UM, absorber="Au", absorber_thickness_um=20.0, membrane="Ti",
              membrane_thickness_um=2.0, target_bottom_dose_kj_cm3=3.0, grid_size=32, nz=128 if not fast else 48, warn=False)
    ref = api.simulate_liga(flux_density=src.spectral_flux_density(100.0, n_bins), **kw)
    z, dose = ref.depth_dose
    z, dose = np.asarray(z, dtype=float), np.asarray(dose, dtype=float)
    assert abs(dose[-1] - 3.0) < 1e-6 and abs(dose[0] - ref.top_dose_kj_cm3) < 1e-9
    assert ref.mean_energy_bottom_kev > ref.mean_energy_top_kev, "spectral hardening with depth"
    assert not ref.exceeds_damage_ceiling
    assert 50.0 < ref.exposure_time_s / 3600 < 55.0, ref.exposure_time_s / 3600  # the page's ≈ 52 h at 100 mm
    times = []
    for dist in TUBE_LIGA_DISTANCES_MM:
        r = api.simulate_liga(flux_density=src.spectral_flux_density(dist, n_bins), **kw)
        times.append(r.exposure_time_s)
    times = np.array(times)
    dist = np.array(TUBE_LIGA_DISTANCES_MM)
    assert np.allclose(times / dist ** 2, times[0] / dist[0] ** 2, rtol=1e-6), "inverse square from an isotropic point focus"
    return {"z": z, "dose": dose, "top": ref.top_dose_kj_cm3, "ratio": ref.dose_ratio, "e_top": ref.mean_energy_top_kev,
            "e_bottom": ref.mean_energy_bottom_kev, "t100": ref.exposure_time_s, "dist": dist, "times": times,
            "ceiling": 20.0, "spectrum": ref.spectrum, "power_density": _dq(src)["power_density_at_100mm"]}


def tube_liga_draw(d, th: S.Theme):
    fig, (ax, bx) = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 4.2), gridspec_kw={"wspace": 0.36})
    S.style_axes(ax, th, grid="both")
    ax.set_yscale("log")
    ax.plot(d["z"], d["dose"], color=th.s(1), linewidth=S.LINE_PT, zorder=3)
    S.limit_line(ax, d["ceiling"], th, f"{d['ceiling']:.0f} kJ/cm³ damage ceiling", axis="y", where=0.97, side="left", va="bottom")
    S.limit_line(ax, 3.0, th, "3 kJ/cm³ clearing dose at the bottom", axis="y", where=0.97, side="left", va="top")
    S.dot(ax, [d["z"][0], d["z"][-1]], [d["dose"][0], d["dose"][-1]], th.s(1), th, size_px=8.0, zorder=5)
    S.end_labels(ax, [(d["z"][0], d["dose"][0], f"top {d['top']:.2f} kJ/cm³, ⟨E⟩ {d['e_top']:.2f} keV"),
                      ], th, dx_px=8, fontsize=S.SMALL_PT - 0.5)
    ax.text(d["z"][-1] - 3, 4.4, f"bottom 3 kJ/cm³, ⟨E⟩ {d['e_bottom']:.2f} keV", color=th.ink2, fontsize=S.SMALL_PT - 0.5,
            ha="right", va="bottom")
    ax.text(0.03, 0.06, f"top/bottom dose ratio {d['ratio']:.2f}; the beam hardens with depth", transform=ax.transAxes,
            color=th.ink2, fontsize=S.SMALL_PT, va="bottom")
    ax.set_xlim(0, TUBE_LIGA_THICKNESS_UM)
    ax.set_ylim(1.0, 40.0)
    ax.set_xlabel("depth in PMMA (µm)")
    ax.set_ylabel("dose (kJ/cm³)")
    ax.set_title(f"(a) W 60 kV, 30 mA at 100 mm: depth dose in {TUBE_LIGA_THICKNESS_UM:.0f} µm PMMA")
    S.style_axes(bx, th, grid="both")
    bx.set_xscale("log")
    bx.set_yscale("log")
    hours = d["times"] / 3600.0
    bx.plot(d["dist"], hours, color=th.s(2), linewidth=S.LINE_PT, zorder=3)
    S.dot(bx, d["dist"], hours, th.s(2), th, size_px=7.0, zorder=4)
    i100 = int(np.argmin(abs(d["dist"] - 100.0)))
    bx.annotate(f"{hours[i100]:.0f} h at 100 mm", (100.0, hours[i100]), xytext=(-12, 14), textcoords="offset points", ha="right",
                color=th.ink2, fontsize=S.SMALL_PT, arrowprops=dict(arrowstyle="-", color=th.axis, lw=S.HAIR_PT, shrinkA=0, shrinkB=3))
    for hrs, lab in ((1.0, "1 h"), (24.0, "1 day"), (168.0, "1 week")):
        bx.axhline(hrs, color=th.grid, linewidth=S.HAIR_PT, zorder=1)
        bx.text(bx.get_xlim()[0] if False else 25.5, hrs * 1.1, lab, color=th.ink2, fontsize=S.SMALL_PT - 1, va="bottom")
    bx.text(0.97, 0.04, "t ∝ d²: an isotropic point focus", transform=bx.transAxes, ha="right", va="bottom", color=th.ink2,
            fontsize=S.SMALL_PT)
    bx.set_xlim(22, 450)
    bx.set_ylim(0.5, 2000)
    bx.set_xlabel("focal spot to mask distance (mm)")
    bx.set_ylabel("exposure time to the clearing dose (h)")
    bx.set_title("(b) Exposure time against distance")
    fig.subplots_adjust(left=0.08, right=0.98, top=0.9, bottom=0.14)
    return fig


_register(FigureSpec(
    name="sources-xray-tube-liga",
    group=GROUP,
    compute=tube_liga_compute,
    draw=tube_liga_draw,
    alt="Left, log dose against depth through 200 µm PMMA from the W 60 kV tube at 100 mm, falling from 6.65 to 3 kJ/cm³ while the mean photon energy rises from 5.8 to 8.1 keV; right, exposure time against source distance on log axes: 52 h at 100 mm, rising as the distance squared.",
    caption="<strong>Days, not hours: the tube's flux sets the LIGA exposure time.</strong> "
            "<code>simulate_liga(flux_density=tube.spectral_flux_density(d))</code> for the W 60 kV / 30 mA preset "
            "through 20 µm Au on 2 µm Ti into 200 µm PMMA, scaled to 3 kJ/cm³ at the bottom: (a) the depth dose "
            "hardens with depth (top/bottom ratio 2.22) and stays under the 20 kJ/cm³ ceiling; (b) the time to "
            "reach it is 52 h at 100 mm and grows with the square of the distance. The continuum and absolute flux "
            "are simplified, so the time is model output and likely optimistic. <em>Model: LIGA / deep X-ray depth "
            "dose and exposure time ✅ · Hard X-ray tube (W / Mo / Cu / Rh, 5–300 kV) 🔶 (✅ lines, edges, "
            "Duane–Hunt limit, Be window).</em>",
    pages=("sources/xray-tube.md",),
))


# ---------------------------------------------------------------------------
# S3k · Excimer line shapes and the E95 convention
# ---------------------------------------------------------------------------

E95_RATIOS = {"Gaussian": 1.6646, "Lorentzian": 12.706, "sinc²": 4.680}  # the page's table (duv-heritage.md)


def _shape(kind: str, x: np.ndarray) -> np.ndarray:
    """Unit-FWHM line shapes at offset x (in FWHM units), peak 1."""
    if kind == "Gaussian":
        return np.exp(-4 * math.log(2) * x ** 2)
    if kind == "Lorentzian":
        return 1 / (1 + (2 * x) ** 2)
    u0 = 1.3915573  # sinc²(u0) = 1/2
    u = 2 * u0 * x
    return np.where(abs(u) < 1e-12, 1.0, (np.sin(u) / np.maximum(abs(u), 1e-300)) ** 2)


def _e95_half_width(kind: str) -> float:
    """Half-width of the interval holding 95 % of the line energy, in FWHM units (numerical)."""
    x = np.linspace(0.0, 4000.0, 4_000_001)
    y = _shape(kind, x)
    cum = np.concatenate([[0.0], np.cumsum(0.5 * (y[1:] + y[:-1]) * np.diff(x))])
    total = {"Gaussian": 0.5 * math.sqrt(math.pi / (4 * math.log(2))), "Lorentzian": math.pi / 4,
             "sinc²": math.pi / (2 * 2 * 1.3915573)}[kind]  # analytic ∫_0^∞ of each unit-FWHM shape
    assert abs(cum[-1] - total) / total < 2e-3, (kind, cum[-1], total)
    return float(np.interp(0.95 * total, cum, x))


def line_shapes_compute(fast: bool):
    huv = native()
    SC = huv.SourceConfig
    e95 = {k: 2 * _e95_half_width(k) for k in E95_RATIOS}
    for k, v in E95_RATIOS.items():
        assert abs(e95[k] - v) / v < 5e-3, (k, e95[k], v)
    arf, f2 = SC.arf_laser(), SC.f2_laser()
    assert abs(_dq(arf)["e95_bandwidth"] / arf.bandwidth_pm - e95["Gaussian"]) < 1e-3, "ArF: Gaussian E95 convention"
    assert abs(_dq(f2)["e95_bandwidth"] / f2.bandwidth_pm - e95["Lorentzian"]) < 1e-2, "F₂: Lorentzian E95 convention"
    samples = []
    for src, kind, label in ((arf, "Gaussian", "ArF preset"), (f2, "Lorentzian", "F₂ preset")):
        sp = np.array(src.spectrum())
        off = (sp[:, 0] - src.wavelength_nm) * 1e3  # pm
        w = sp[:, 1]
        assert abs(w.sum() - 1) < 1e-9
        fwhm = src.bandwidth_pm
        assert np.allclose(off, np.linspace(-2.5, 2.5, len(off)) * fwhm, atol=1e-6), "samples span ±2.5 FWHM"
        # Both samplers sample the line at its stated FWHM (before 2026-10-10 the Gaussian sampler used
        # σ = FWHM/(2√ln 2) and sampled every Gaussian line √2 wider than stated; the pin below guards that).
        assert np.allclose(w / w.max(), _shape(kind, off / fwhm), rtol=1e-6), f"{kind} samples at the stated FWHM"
        if kind == "Gaussian":
            assert not np.allclose(w / w.max(), _shape(kind, off / (fwhm * math.sqrt(2))), rtol=0.2)
        samples.append(dict(label=label, kind=kind, lam=src.wavelength_nm, fwhm_pm=fwhm,
                            off=off, w=w, e95_pm=_dq(src)["e95_bandwidth"]))
    x = np.linspace(-8.0, 8.0, 1601)
    return {"x": x, "shapes": {k: _shape(k, x) for k in E95_RATIOS}, "e95": e95, "samples": samples}


def line_shapes_draw(d, th: S.Theme):
    fig = plt.figure(figsize=(S.WIDE_WIDTH, 4.3))
    gs = fig.add_gridspec(1, 3, width_ratios=[1.6, 1, 1], wspace=0.4)
    ax = fig.add_subplot(gs[0, 0])
    S.style_axes(ax, th, grid="y")
    items = []
    for k, (kind, y) in enumerate(d["shapes"].items()):
        colour = th.s(1 + k)
        half = d["e95"][kind] / 2
        ax.fill_between(d["x"], 0, y, where=abs(d["x"]) <= half, color=colour, alpha=0.14, lw=0, zorder=1)
        ax.plot(d["x"], y, color=colour, linewidth=S.LINE_PT, zorder=3)
        ax.plot([-half, half], [0.02 + 0.045 * k] * 2, color=colour, linewidth=S.LINE_PT, zorder=4, solid_capstyle="butt")
        items.append((min(half, 7.8), 0.02 + 0.045 * k, f"{kind}: E95 = {d['e95'][kind]:.2f} × FWHM"))
    S.end_labels(ax, items, th, dx_px=6, min_gap_px=12, fontsize=S.SMALL_PT - 0.5)
    ax.plot([-0.5, 0.5], [0.5, 0.5], color=th.ink2, linewidth=S.HAIR_PT * 1.4, zorder=4)
    ax.text(0.6, 0.5, "FWHM", color=th.ink2, fontsize=S.SMALL_PT, va="center")
    ax.set_xlim(-8, 8)
    ax.set_ylim(0, 1.08)
    ax.set_xlabel("offset from the line centre (FWHM units)")
    ax.set_ylabel("spectral density (peak 1)")
    ax.set_title("(a) Equal FWHM; bars = the 95 %-energy interval")
    for k, s in enumerate(d["samples"]):
        bx = fig.add_subplot(gs[0, 1 + k])
        S.style_axes(bx, th, grid="y")
        colour = th.s(1 if s["kind"] == "Gaussian" else 2)
        xx = np.linspace(-2.8, 2.8, 561) * s["fwhm_pm"]
        bx.plot(xx, _shape(s["kind"], xx / s["fwhm_pm"]) * s["w"].max(), color=colour, linewidth=S.LINE_PT * 0.9, zorder=2)
        # the note sits over the centre stem: a surface-coloured box keeps it legible
        note_kw = dict(transform=bx.transAxes, va="top", color=th.ink2, fontsize=S.SMALL_PT - 1, zorder=5,
                       bbox=dict(boxstyle="square,pad=0.25", facecolor=th.surface, edgecolor="none"))
        if s["kind"] == "Gaussian":
            bx.text(0.03, 0.96, f"Gaussian at the stated\n{s['fwhm_pm']:.1f} pm FWHM (σ = FWHM/2√(2 ln 2)):\n"
                    f"E95 {s['e95_pm']:.2f} pm", **note_kw)
        else:
            bx.text(0.03, 0.96, f"Lorentzian at the stated\n{s['fwhm_pm']:.1f} pm FWHM: heavy tails,\nE95 {s['e95_pm']:.1f} pm",
                    **note_kw)
        bx.vlines(s["off"], 0, s["w"], color=colour, linewidth=S.LINE_PT, zorder=3)
        S.dot(bx, s["off"], s["w"], colour, th, size_px=7.0, zorder=4)
        bx.set_xlim(xx[0], xx[-1])
        bx.set_ylim(0, 1.0)
        bx.set_xlabel("λ − λ$_0$ (pm)")
        if k == 0:
            bx.set_ylabel("sampled weight (sum 1)")
        bx.set_title(f"({'bc'[k]}) {s['label'].replace('F₂', 'F$_2$')}, {s['lam']:.2f} nm, {len(s['w'])} samples", fontsize=S.SMALL_PT + 0.5)
    fig.subplots_adjust(left=0.07, right=0.98, top=0.9, bottom=0.14)
    return fig


_register(FigureSpec(
    name="sources-line-shapes-e95",
    group=GROUP,
    compute=line_shapes_compute,
    draw=line_shapes_draw,
    alt="Left, Gaussian, Lorentzian and sinc-squared lines at equal full width at half maximum with their 95 "
        "percent-energy intervals of 1.66, 12.7 and 4.68 widths; right, the five sampled spectral weights of the ArF "
        "preset over a Gaussian and of the F2 preset over a Lorentzian, in picometre offsets.",
    caption="<strong>Why a vendor E95 is not the model's FWHM.</strong> (a) At equal FWHM the 95 %-energy interval "
            "is 1.66 FWHM for a Gaussian, 4.68 for the undulator sinc² and 12.7 for a Lorentzian — the ratios the "
            "derived quantity <code>e95_bandwidth</code> reports. (b, c) The spectral weights <code>spectrum()</code> "
            "hands the imaging engine: five samples over ±2.5 FWHM, each line sampled at its stated FWHM — the ArF "
            "preset's 0.2 pm Gaussian (σ = FWHM/2√(2 ln 2)) and the F₂ preset's 1.1 pm Lorentzian. Line shapes as "
            "parameterized in the model, not measured laser spectra. <em>Model: excimer sources ✅.</em>",
    pages=("sources/duv-heritage.md", "sources/vuv-excimer.md"),
    extra={"model_rows": {"excimer sources": ["DUV/UV heritage (ArF 193.368, KrF 248.3, Hg g/h/i)",
                                              "VUV excimer (F₂ 157.63 nm; Ar₂ 126 nm)"]}},
))
