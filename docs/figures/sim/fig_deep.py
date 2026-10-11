"""Deep-lithography figures: LIGA, multilayer mirrors, the volumetric chain, CAR bake, Talbot, interference, grayscale.

Every spec here is engine output at the parameters its caption states. Physical asserts live in
``compute()`` (``make_all.py --verify --group deep`` runs them without drawing). The canonical
process scene shared by P1, P2, P3 and P12 is the CAR figure's: F₂ 157.63 nm, NA 0.75, σ 0.7,
150 nm lines on a 300 nm pitch, 150 nm of the default (illustrative, low-contrast) resist on Si.
"""

from __future__ import annotations

import base64
import struct
import zlib

import numpy as np
from matplotlib import pyplot as plt
from matplotlib.colors import ListedColormap
from matplotlib.lines import Line2D

import style as S
from common import FigureSpec, native, register

WAVELENGTH_NM = 157.63


def _scene(huv):
    """The canonical process scene: F₂ σ 0.7, NA 0.75, 150/300 nm lines."""
    return (huv.SourceConfig.f2_laser(0.7), huv.OpticsConfig(numerical_aperture=0.75),
            huv.MaskConfig.line_space(150.0, 300.0))


def _rms(a, b) -> float:
    """Relative rms difference of two profiles."""
    a, b = np.asarray(a, dtype=float), np.asarray(b, dtype=float)
    return float(np.sqrt(np.mean((a - b) ** 2)) / np.sqrt(np.mean(b ** 2)))


def _contrast(p) -> float:
    p = np.asarray(p, dtype=float)
    return float((p.max() - p.min()) / (p.max() + p.min()))


# ---------------------------------------------------------------------------
# 1. LIGA depth dose: beam filters trade top dose against exposure time
# ---------------------------------------------------------------------------

LIGA_FILTERS = ((None, "no filter"), ([("Be", 100.0)], "100 µm Be"), ([("kapton", 125.0)], "125 µm Kapton"),
                ([("Al", 20.0)], "20 µm Al"))


def liga_dose_compute(fast: bool):
    huv = native()
    bm = huv.SourceConfig.synchrotron_liga_bending_magnet()
    out = []
    for filt, label in LIGA_FILTERS:
        r = huv.simulate_liga(source=bm, source_distance_m=15.0, vertical_scan_mm=50.0, filters=filt, grid_size=32,
                              pixel_nm=200.0, nz=16, diffraction="gaussian")
        z, dose = r.depth_dose
        out.append({"label": label, "z": np.asarray(z), "dose": np.asarray(dose), "time": r.exposure_time_s,
                    "ratio": r.dose_ratio, "e_top": r.mean_energy_top_kev, "e_bot": r.mean_energy_bottom_kev,
                    "charge": r.exposure_charge_ma_h})
    # every curve is scaled to the 3 kJ/cm³ bottom target; a filtered beam always lowers the top dose
    assert all(abs(r["dose"][-1] - 3.0) < 0.05 for r in out), "bottom dose is not the 3 kJ/cm³ target"
    assert out[0]["dose"][0] > max(r["dose"][0] for r in out[1:]), "an unfiltered beam must give the highest top dose"
    return out


def liga_dose_draw(rows, th: S.Theme):
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 4.3))
    S.style_axes(ax, th, grid="both")
    ax.set_yscale("log")
    ends = []
    for slot, r in enumerate(rows, start=1):
        ax.plot(r["z"], r["dose"], color=th.s(slot), zorder=3)
        S.dot(ax, [r["z"][0]], [r["dose"][0]], th.s(slot), th)
        ends.append((r["z"][-1], r["dose"][-1] * 1.0,
                     f"{r['label']}: top {r['dose'][0]:.1f} kJ/cm³, {r['time']:.0f} s ({r['charge']:.1f} mA·h)"))
    S.limit_line(ax, 20.0, th, "damage ceiling 20 kJ/cm³", axis="y", where=0.99, side="left", va="bottom")
    S.limit_line(ax, 3.0, th, "bottom target 3 kJ/cm³", axis="y", where=0.99, side="left", va="top")
    ax.set_xlim(0, 500)
    ax.set_ylim(2, 100)
    ax.set_yticks([2, 3, 5, 10, 20, 50, 100])
    ax.set_yticklabels(["2", "3", "5", "10", "20", "50", "100"])
    ax.set_xlabel("Depth in PMMA (µm)")
    ax.set_ylabel("Absorbed dose (kJ/cm³)")
    ax.set_title("LIGA depth dose, 2.5 GeV / 1.5 T bending magnet, 15 m, 50 mm scan, 200 mA")
    handles = [Line2D([], [], color=th.s(i + 1)) for i in range(len(rows))]
    ax.legend(handles, [e[2] for e in ends], loc="upper right", fontsize=S.SMALL_PT)
    return fig


register(FigureSpec(
    name="liga-depth-dose",
    group="deep",
    compute=liga_dose_compute,
    draw=liga_dose_draw,
    alt="Log-scale depth-dose curves through 500 micrometres of PMMA for no filter and Be, Kapton and Al "
        "filters; unfiltered the top dose is about 62 kJ/cm3, far above the 20 kJ/cm3 damage ceiling, while the "
        "aluminium filter keeps it near 9 kJ/cm3 at the cost of a longer exposure.",
    caption="<strong>A harder beam flattens the depth dose.</strong> 500 µm PMMA exposed through 20 µm Au on "
            "2 µm Ti, every curve scaled so the bottom receives 3 kJ/cm³: a filtered beam keeps the top below the "
            "20 kJ/cm³ damage ceiling at the price of exposure time, which is absolute (bending-magnet flux at "
            "200 mA, 15 m, 50 mm scan). <em>Model: LIGA / deep X-ray depth dose and exposure time ✅ (NIST μ and "
            "μ<sub>en</sub>, bending-magnet photons per ln E; scan-averaged collimated beam, no beamline mirrors).</em>",
    pages=("processes/liga-deep-xray.md", "python-api-modules.md"),
))


# ---------------------------------------------------------------------------
# 2. LIGA edge: Fresnel diffraction vs the legacy Gaussian blur; sidewall with depth
# ---------------------------------------------------------------------------

EDGE_E_KEV, EDGE_GAP_UM = 8.0, 100.0


def liga_edge_compute(fast: bool):
    huv = native()
    mono = [(EDGE_E_KEV, 1.0)]
    common_kw = dict(spectrum_table=mono, absorber_thickness_um=200.0, membrane_thickness_um=0.0,
                     proximity_gap_um=EDGE_GAP_UM)
    fres = huv.liga_edge_profile(depths_um=[0.0], dx_nm=1.0, x_min_nm=-400.0, x_max_nm=400.0, **common_kw)
    # the legacy model, from the simulator's own Gaussian path: a periodic line whose left edge we cut out
    pitch, cd, n = 4000.0, 2000.0, 1024
    gauss = huv.simulate_liga(diffraction="gaussian", cd_nm=cd, pitch_nm=pitch, grid_size=n, pixel_nm=pitch / n,
                              nz=4, energy_bins=1, **common_kw)
    vol = np.asarray(gauss.volume.values)
    xg = np.asarray(gauss.volume.x_nm)
    row = vol[0, vol.shape[1] // 2]
    open_level = float(np.median(row[np.abs(xg) > 1500]))
    # absorber spans [-1000, 1000]; around its left edge the open side is x < -1000: flip so absorber is x < 0
    sel = np.abs(xg + cd / 2) <= 400
    xg_rel = -(xg[sel] + cd / 2)
    order = np.argsort(xg_rel)
    bm = huv.SourceConfig.synchrotron_liga_bending_magnet()
    depths = [0.0, 125.0, 250.0, 375.0, 500.0]
    poly = huv.liga_edge_profile(source=bm, depths_um=depths, dx_nm=2.0, x_min_nm=-600.0, x_max_nm=600.0)
    thr = 2.5
    fresnel = np.asarray(fres.dose_kj_cm3)[0] / float(np.asarray(fres.open_dose_kj_cm3)[0])
    x = np.asarray(fres.x_nm)
    # the textbook knife edge: 0.25 of the open field at the geometric edge, a 1.37 first maximum
    at_edge = float(np.interp(0.0, x, fresnel))
    assert abs(at_edge - 0.25) < 0.03, f"Fresnel edge value {at_edge:.3f} is not the knife-edge 0.25"
    assert abs(fresnel.max() - 1.37) < 0.03, f"Fresnel first maximum {fresnel.max():.3f} is not 1.37"
    return {
        "x": x, "fresnel": fresnel,
        "xg": xg_rel[order], "gauss": row[sel][order] / open_level,
        "poly_x": np.asarray(poly.x_nm), "poly": np.asarray(poly.dose_kj_cm3), "depths": depths, "thr": thr,
        "edges": poly.edge_positions_nm(thr), "sidewall": poly.sidewall_angle_deg(thr),
        "lam_nm": 1.23984193 / EDGE_E_KEV,
    }


def liga_edge_draw(d, th: S.Theme):
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 4.1), gridspec_kw={"wspace": 0.28})
    S.style_axes(ax1, th, grid="both")
    ax1.axvspan(-400, 0, color=th.band, zorder=0, lw=0)
    ax1.text(-390, 1.42, "Au absorber", color=th.ink2, fontsize=S.SMALL_PT, va="top")
    ax1.plot(d["xg"], d["gauss"], color=th.s(2), zorder=3)
    ax1.plot(d["x"], d["fresnel"], color=th.s(1), zorder=4)
    scale = np.sqrt(d["lam_nm"] * EDGE_GAP_UM * 1e3)
    ax1.set_xlim(-400, 400)
    ax1.set_ylim(0, 1.45)
    ax1.set_xlabel("x from the absorber edge (nm)")
    ax1.set_ylabel("Dose / open-field dose at the resist top")
    ax1.set_title(f"(a) {EDGE_E_KEV:.0f} keV, {EDGE_GAP_UM:.0f} µm gap: √(λg) = {scale:.0f} nm")
    ax1.legend([Line2D([], [], color=th.s(1)), Line2D([], [], color=th.s(2))],
               ["Fresnel (default)", "legacy Gaussian, σ = ½√(λg)"], loc="center left",
               bbox_to_anchor=(0.0, 0.62))
    ax2.set_title("(b) Bending-magnet beam: edge through 500 µm PMMA")
    S.style_axes(ax2, th, grid="both")
    cols = S.ordinal(th, len(d["depths"]))
    for col, z, prof in zip(cols, d["depths"], d["poly"]):
        ax2.plot(d["poly_x"], prof, color=col, zorder=3, linewidth=S.LINE_PT * 0.85)
    S.limit_line(ax2, d["thr"], th, f"development threshold {d['thr']:.1f} kJ/cm³", axis="y", where=0.02,
                 side="right", va="bottom")
    for col, e in zip(cols, d["edges"]):
        if e is not None:
            S.dot(ax2, [e], [d["thr"]], col, th, size_px=7)
    ax2.set_yscale("log")
    ax2.set_xlim(-600, 600)
    ax2.set_ylim(0.05, 150)
    ax2.set_xlabel("x from the absorber edge (nm)")
    ax2.set_ylabel("Absorbed dose (kJ/cm³)")
    leg = ax2.legend([Line2D([], [], color=c) for c in cols], [f"{z:.0f} µm deep" for z in d["depths"]],
                     loc="lower right", title=f"sidewall {d['sidewall']:.3f}° from vertical",
                     title_fontsize=S.SMALL_PT)
    leg.get_title().set_color(th.ink2)
    return fig


register(FigureSpec(
    name="liga-fresnel-edge",
    group="deep",
    compute=liga_edge_compute,
    draw=liga_edge_draw,
    alt="Left: dose across an absorber edge at 8 keV, where Fresnel diffraction gives the knife-edge 0.25 at the "
        "edge and ringing up to 1.37, unlike the smooth legacy Gaussian blur; right: dose across the edge at five "
        "depths of the polychromatic beam, the developed edge moving about 300 nm over 500 µm.",
    caption="<strong>Proximity diffraction at a LIGA absorber edge.</strong> (a) Monochromatic 8 keV straight edge, "
            "100 µm gap: the default Fresnel (angular-spectrum) model gives the knife-edge pattern — 0.25 at the "
            "geometric edge, a 1.37 maximum and ringing — while the legacy <code>gaussian</code> option smooths the "
            "step with σ = ½√(λg); both curves are simulator output. (b) Polychromatic bending-magnet beam, no "
            "filter: absorbed dose across the edge at five depths and the developed edge at a 2.5 kJ/cm³ "
            "threshold. <em>Model: LIGA proximity diffraction ✅ (🔶 thin-screen absorber; no photo- or "
            "secondary-electron transport).</em>",
    pages=("processes/liga-deep-xray.md", "README.md", "history/parallel-paths.md"),
    extra={"width": {"README.md": 820}},  # README <picture> width (DESIGN §6.2; INT late-binding edit)
))


# ---------------------------------------------------------------------------
# 3. Multilayer mirrors: Mo/Si at 13.5 nm, La/B4C and La/B at 6.6-6.7 nm
# ---------------------------------------------------------------------------


def multilayer_compute(fast: bool):
    huv = native()
    mosi = huv.MultilayerMirror.mo_si(40, 6.9, 0.4)
    lam1 = np.linspace(12.8, 14.2, 141 if fast else 561)
    lab4c = huv.MultilayerMirror.la_b4c(200, huv.tune_multilayer_period("la_b4c", 6.7, 200), 0.4)
    lab = huv.MultilayerMirror.la_b(200, huv.tune_multilayer_period("la_b", 6.65, 200), 0.4)
    lam2 = np.linspace(6.45, 6.95, 201 if fast else 801)
    ang = np.linspace(0, 25, 101 if fast else 251)
    out = {
        "lam1": lam1, "mosi": np.asarray(mosi.reflectance_curve(lam1)), "mosi_peak": mosi.peak(12.8, 14.2),
        "mosi_fwhm": mosi.bandwidth_fwhm_nm(12.8, 14.2),
        "lam2": lam2, "lab4c": np.asarray(lab4c.reflectance_curve(lam2)), "lab": np.asarray(lab.reflectance_curve(lam2)),
        "lab4c_peak": lab4c.peak(6.45, 6.95), "lab_peak": lab.peak(6.45, 6.95),
        "lab4c_d": lab4c.period_nm, "lab_d": lab.period_nm,
        "ang": ang,
        "s": np.asarray(mosi.reflectance_vs_angle(13.5, ang, "te")),
        "p": np.asarray(mosi.reflectance_vs_angle(13.5, ang, "tm")),
    }
    # the matrix row's own numbers: ideal Mo/Si 73.0 % at 13.48 nm
    lp, rp = out["mosi_peak"]
    assert abs(rp - 0.730) < 0.005 and abs(lp - 13.48) < 0.02, f"Mo/Si peak {rp:.3f} at {lp:.2f} nm drifted"
    # at normal incidence s and p coincide; by 20° the Bragg peak has walked off (the page's 4.2 % s)
    assert abs(out["s"][0] - out["p"][0]) < 1e-9, "s and p must coincide at normal incidence"
    assert out["s"][-1] < 0.1 and out["p"][-1] < 0.1, "Mo/Si reflectance at 25° should be far below the Bragg peak"
    return out


def multilayer_draw(d, th: S.Theme):
    fig, axes = plt.subplots(1, 3, figsize=(S.WIDE_WIDTH + 0.6, 3.9), gridspec_kw={"wspace": 0.32,
                                                                                    "width_ratios": [1.1, 1.1, 1]})
    for ax in axes:
        S.style_axes(ax, th, grid="both")
        ax.set_ylim(0, 0.9)
    ax = axes[0]
    ax.plot(d["lam1"], d["mosi"], color=th.s(1), zorder=3)
    S.limit_line(ax, 0.7015, th, "record ≈ 70.15 %\n(2007)", axis="y", where=0.99, side="left", va="bottom")
    lp, rp = d["mosi_peak"]
    ax.annotate(f"model peak {rp * 100:.1f} % at {lp:.2f} nm\nFWHM {d['mosi_fwhm']:.2f} nm", xy=(lp, rp),
                xytext=(12.85, 0.84), color=th.ink2, fontsize=S.SMALL_PT, va="top",
                arrowprops=dict(arrowstyle="-", color=th.axis, lw=S.HAIR_PT, shrinkA=1, shrinkB=2))
    ax.set_xlim(12.8, 14.2)
    ax.set_xlabel("Wavelength (nm)")
    ax.set_ylabel("Reflectance (normal incidence)")
    ax.set_title("(a) Mo/Si, 40 × 6.9 nm")
    ax = axes[1]
    ax.plot(d["lam2"], d["lab4c"], color=th.s(1), zorder=3)
    ax.plot(d["lam2"], d["lab"], color=th.s(2), zorder=3)
    S.limit_line(ax, 0.641, th, "La/B record\n64.1 % (2015)", axis="y", where=0.01, side="right", va="bottom")
    ax.set_xlim(6.45, 6.95)
    ax.set_xlabel("Wavelength (nm)")
    ax.set_title("(b) 200-period La-based, ideal")
    ax.legend([Line2D([], [], color=th.s(1)), Line2D([], [], color=th.s(2))],
              [f"La/B₄C, d {d['lab4c_d']:.3f} nm: {d['lab4c_peak'][1] * 100:.1f} %".replace("₄", "$_4$"),
               f"La/B, d {d['lab_d']:.3f} nm: {d['lab_peak'][1] * 100:.1f} %"], loc="center right")
    ax = axes[2]
    ax.plot(d["ang"], d["s"], color=th.s(1), zorder=3)
    ax.plot(d["ang"], d["p"], color=th.s(2), zorder=3)
    ax.set_xlim(0, 25)
    ax.set_xlabel("Angle of incidence (°)")
    ax.set_title("(c) Mo/Si at 13.5 nm vs angle")
    ax.legend([Line2D([], [], color=th.s(1)), Line2D([], [], color=th.s(2))], ["s (TE)", "p (TM)"],
              loc="upper right")
    return fig


register(FigureSpec(
    name="materials-multilayer",
    group="deep",
    compute=multilayer_compute,
    draw=multilayer_draw,
    alt="Three reflectance charts: a Mo/Si multilayer peaking near 73 percent at 13.5 nm with a 0.6 nm "
        "bandwidth; La/B4C and La/B multilayers near 6.7 nm with much narrower peaks; and Mo/Si reflectance "
        "falling with angle of incidence, faster for p polarization.",
    caption="<strong>Multilayer mirrors from Parratt recursion with CXRO/Henke constants.</strong> (a) Mo/Si "
            "40 × 6.9 nm (Mo fraction 0.4, ideal interfaces) against the best reported measured reflectance; "
            "(b) La/B<sub>4</sub>C and La/B with periods tuned to 6.7 / 6.65 nm sit well above the 64.1 % La/B "
            "record (Opt. Lett. 40, 3778, 2015) because interdiffusion and roughness are off; (c) Mo/Si at 13.5 nm "
            "against the angle of incidence, s and p. <em>Model: EUV/BEUV multilayer mirrors ✅ (matches the CXRO "
            "calculator within 3·10⁻³; no interdiffusion layers by default).</em>",
    pages=("materials.md", "future/beyond-euv.md", "history/euv.md", "sources/lpp.md"),
))


# ---------------------------------------------------------------------------
# 4. Level-set vs fast-marching development, with and without surface inhibition
# ---------------------------------------------------------------------------

DEV_T = 3.0


def dev_compute(fast: bool):
    huv = native()
    dx, dz = (2.0, 4.0) if fast else (1.0, 2.0)
    nx, nz, ny = int(128 / dx), int(100 / dz), 2
    x = (np.arange(nx) + 0.5) * dx - nx * dx / 2
    m = 1 - 0.9 * np.exp(-x ** 2 / (2 * 20.0 ** 2))  # exposed Gaussian trench in the PAC field, uniform in depth
    vol = np.ascontiguousarray(np.broadcast_to(m[None, None, :], (nz, ny, nx)))
    V = huv.VolumetricResult.from_array(vol, (-nx * dx / 2, nx * dx / 2), (0.0, ny * dx), (0.0, nz * dz))
    res = huv.ResistConfig()
    out = {"x": x, "z": (np.arange(nz) + 0.5) * dz, "m": m, "cases": {}}
    for key, rs, depth in (("plain", 1.0, 0.0), ("inhibited", 0.1, 10.0)):
        ls = huv.develop_level_set(V, res, DEV_T, surface_rate_ratio=rs, inhibition_depth_nm=depth)
        fm = huv.develop_fast_marching(V, res, dx, dz, surface_rate_ratio=rs, inhibition_depth_nm=depth,
                                       lateral="periodic")
        out["cases"][key] = {
            "ls": np.asarray(ls.arrival_times.values)[:, 0, :],
            "fm": np.asarray(fm.values)[:, 0, :],
            "steps": ls.steps,
        }
    # the two solvers agree: the developed fractions at DEV_T differ by less than a few per cent
    for key, c in out["cases"].items():
        f_ls = np.mean(np.where(np.isfinite(c["ls"]), c["ls"], 1e9) <= DEV_T)
        f_fm = np.mean(np.where(np.isfinite(c["fm"]), c["fm"], 1e9) <= DEV_T)
        assert abs(f_ls - f_fm) < 0.05, f"{key}: level set and FMM developed fractions differ ({f_ls:.3f} vs {f_fm:.3f})"
    return out


def dev_draw(d, th: S.Theme):
    fig, axes = plt.subplots(1, 2, figsize=(S.FULL_WIDTH + 0.6, 3.8), sharey=True, gridspec_kw={"wspace": 0.08})
    x, z = d["x"], d["z"]
    cmap = S.field_cmap()
    field = np.broadcast_to(1 - d["m"][None, :], (len(z), len(x)))
    titles = {"plain": "(a) no surface inhibition", "inhibited": "(b) surface inhibition r = 0.1, δ = 10 nm"}
    im = None
    for ax, key in zip(axes, ("plain", "inhibited")):
        c = d["cases"][key]
        im = ax.imshow(field, origin="upper", extent=[x[0], x[-1], z[-1], z[0]], aspect="auto", cmap=cmap, vmin=0,
                       vmax=1, interpolation="bilinear")
        fm = np.where(np.isfinite(c["fm"]), c["fm"], 1e9)  # undeveloped voxels carry inf
        ls = np.where(np.isfinite(c["ls"]), c["ls"], 1e9)
        ax.contour(x, z, fm, levels=[DEV_T], colors=[th.s(2)], linewidths=S.LINE_PT * 1.8)
        ax.contour(x, z, ls, levels=[DEV_T], colors=[th.s(1)], linewidths=S.LINE_PT * 0.75)
        S.image_axes(ax, th)
        ax.set_title(titles[key])
        ax.set_xlabel("x (nm)")
        ax.set_xlim(-60, 60)
        ax.set_ylim(60, 0)
    axes[0].set_ylabel("Depth below the original surface (nm)")
    S.colorbar(fig, im, axes, th, "Exposed fraction 1 − m")
    fig.legend([Line2D([], [], color=th.s(1)), Line2D([], [], color=th.s(2), linewidth=S.LINE_PT * 1.4)],
               [f"level set, front at t = {DEV_T:.0f} s", f"fast marching, arrival time = {DEV_T:.0f} s"],
               loc="lower center", ncol=2, bbox_to_anchor=(0.45, 0.0))
    fig.subplots_adjust(bottom=0.25, right=0.86)
    return fig


register(FigureSpec(
    name="volumetric-levelset-vs-fmm",
    group="deep",
    compute=dev_compute,
    draw=dev_draw,
    alt="Two cross-sections of a developing resist trench over an exposure map: the level-set and fast-marching "
        "development fronts after 3 seconds coincide; with surface inhibition both show a narrowed neck a few "
        "nanometres below the top surface (T-top).",
    caption="<strong>Two solvers, one front.</strong> 3-D development of an exposed trench (PAC "
            "m = 1 − 0.9·exp(−x²/2·20²), uniform in depth; default Mack resist): the moving-boundary level set "
            "(thin line) and the static-rate fast-marching arrival time (thick line) agree to within a grid cell "
            "after 3 s, and surface inhibition (top-surface rate × 0.1, 10 nm decay) necks the opening just below "
            "the surface — the T-top. <em>Model: 3D development (fast marching) ✅ · 3D development (level set) ✅ "
            "(🔶 first order; phenomenological inhibition constants).</em>",
    pages=("processes/volumetric-exposure.md", "python-api-modules.md"),
))


# ---------------------------------------------------------------------------
# 5. Chemically amplified resist: quencher sweep and the x-z deprotection map
# ---------------------------------------------------------------------------

CAR_QUENCH = (0.0, 0.1, 0.2, 0.3)


def car_compute(fast: bool):
    huv = native()
    # (a) the 1-D acid profile used in the CAR tests: smooth periodic latent image, period 128 nm
    period = 128.0
    xs = (np.arange(64) + 0.5) * 2.0
    h0 = 0.6 * 0.5 * (1 + np.tanh(3 * np.cos(2 * np.pi * (xs / period - 0.5))))
    vol1 = np.ascontiguousarray(np.repeat((1 - h0)[None, None, :], 2, axis=1))
    V1 = huv.VolumetricResult.from_array(vol1, (0.0, period), (0.0, 4.0), (0.0, 2.0))
    sweep = {q: 1 - np.asarray(huv.peb_car(V1, quencher=q).protected.values)[0, 0] for q in CAR_QUENCH}
    # (b, c) a real volumetric latent image (F2, 150/300 nm L/S, 150 nm resist on Si) before and after the bake
    src, optics, mask = _scene(huv)
    grid = mask.commensurate_grid(64 if fast else 128, 4.6875)
    lat = huv.expose_volumetric(src, optics, mask, huv.FilmStackConfig(), huv.ResistConfig(), grid,
                                dose_mj_cm2=40.0, nz=32 if fast else 64, n_defocus_planes=4, dose_steps=5)
    car = huv.peb_car(lat)
    acid0 = 1 - np.asarray(lat.values)
    dep = 1 - np.asarray(car.protected.values)
    mid = acid0.shape[1] // 2
    # more quencher deprotects less, everywhere
    peaks = [float(sweep[q].max()) for q in CAR_QUENCH]
    assert all(a > b for a, b in zip(peaks, peaks[1:])), f"deprotection peaks not monotone in quencher: {peaks}"
    return {"xs": xs, "h0": h0, "sweep": sweep, "x": np.asarray(lat.x_nm), "z": np.asarray(lat.z_nm),
            "acid": acid0[:, mid, :], "dep": dep[:, mid, :]}


def car_draw(d, th: S.Theme):
    fig = plt.figure(figsize=(S.WIDE_WIDTH + 0.6, 3.9))
    gs = fig.add_gridspec(1, 4, width_ratios=[1.15, 1, 1, 0.05], wspace=0.3)
    ax = fig.add_subplot(gs[0, 0])
    S.style_axes(ax, th, grid="both")
    ax.plot(d["xs"], d["h0"], color=th.ink2, linewidth=S.HAIR_PT * 1.5, linestyle=(0, (4, 3)), zorder=2)
    cols = S.ordinal(th, len(CAR_QUENCH))
    for col, q in zip(cols, CAR_QUENCH):
        ax.plot(d["xs"], d["sweep"][q], color=col, zorder=3)
    ax.set_xlim(0, 128)
    ax.set_ylim(0, 1.0)
    ax.set_xlabel("x (nm)")
    ax.set_ylabel("Fraction")
    ax.set_title("(a) Deprotection vs quencher loading")
    ax.legend([Line2D([], [], color=th.ink2, linestyle=(0, (4, 3)))] + [Line2D([], [], color=c) for c in cols],
              ["initial acid h₀".replace("₀", "$_0$")] + [f"deprotected, quencher {q:.1f}" for q in CAR_QUENCH],
              loc="upper center", fontsize=S.SMALL_PT - 0.5, bbox_to_anchor=(0.5, -0.2), ncol=2)
    cmap = S.field_cmap()
    x, z = d["x"], d["z"]
    im = None
    for k, (key, title) in enumerate((("acid", "(b) photo-acid after exposure"),
                                      ("dep", "(c) deprotection after the CAR bake"))):
        axm = fig.add_subplot(gs[0, 1 + k])
        im = axm.imshow(d[key], origin="upper", extent=[x[0], x[-1], z[-1], z[0]], aspect="auto", cmap=cmap,
                        vmin=0, vmax=1, interpolation="bilinear")
        S.image_axes(axm, th)
        axm.set_title(title)
        axm.set_xlabel("x (nm)")
        if k == 0:
            axm.set_ylabel("Depth in resist (nm)")
        else:
            axm.tick_params(labelleft=False)
    cax = fig.add_subplot(gs[0, 3])
    cb = fig.colorbar(im, cax=cax)
    cb.outline.set_edgecolor(th.axis)
    cb.ax.tick_params(colors=th.axis, labelcolor=th.ink2, labelsize=S.SMALL_PT)
    cb.set_label("Acid (b) / deprotected (c) fraction", color=th.ink2, fontsize=S.SMALL_PT)
    fig.subplots_adjust(bottom=0.3)
    return fig


register(FigureSpec(
    name="resist-car-deprotection",
    group="deep",
    compute=car_compute,
    draw=car_draw,
    alt="Left: deprotection profiles across a 128 nm period for quencher loadings 0 to 0.3, sharper and narrower "
        "with more quencher. Right: cross-sections through a 150 nm resist showing standing-wave bands in the "
        "photo-acid after exposure and their smoothing into a deprotected trench after the bake.",
    caption="<strong>The chemically amplified bake: acid, quencher, deprotection.</strong> Acid/quencher "
            "reaction–diffusion with default rates (60 s, k<sub>amp</sub> 0.1 s⁻¹, D<sub>acid</sub> 2 nm²/s): "
            "(a) a fixed 1-D acid profile baked with more quencher deprotects less and with a steeper edge; (b, c) a "
            "real volumetric latent image — F<sub>2</sub> 157.63 nm, NA 0.75, 150 nm lines on a 300 nm pitch, "
            "150 nm resist on silicon, so standing waves band the acid — before and after the bake. "
            "<em>Model: volumetric exposure 🔶 · chemically amplified bake 🔶 (standard Mack/PROLITH-class "
            "equations; illustrative constants, no acid loss).</em>",
    pages=("processes/resist-models.md",),
    extra={"model_rows": {"volumetric exposure": "Volumetric (z-resolved) exposure",
                          "chemically amplified bake": "Chemically amplified bake (acid/quencher reaction–diffusion)"}},
))


# ---------------------------------------------------------------------------
# 6. Talbot carpet and the DTL image
# ---------------------------------------------------------------------------

#: Open fraction of the binary amplitude grating the Talbot carpet and the fly-through draw. A 50 % grating has
#: no period-halved image at z_T/4: its paraxial field there is t(x) − i·t(x + p/2), so I = (t(x)² + t(x + p/2)²)/2
#: = 1/2 everywhere and only the ±10-order truncation ripple remains. The textbook fractional-Talbot image the
#: pages describe needs duty ≠ 0.5; ``_talbot_quarter`` checks both statements against the engine.
TALBOT_DUTY = 0.25


def _talbot_quarter(huv, duty: float, orders: int = 10):
    """The paraxial row at z_T/4 behind a 100 nm amplitude grating at 13.5 nm with open fraction ``duty``:
    (Michelson contrast, index of the dominant Fourier component on the two-period field, |p/2 component| / mean).
    The field holds two periods, so index 2 is the grating period p and index 4 is p/2."""
    zt = 2 * 100.0 ** 2 / 13.5
    r = huv.simulate_talbot(13.5, 100.0, grating="amplitude", duty_cycle=duty, max_order=orders,
                            propagation="paraxial", gap_nm=zt / 4, nx=128, carpet_nz=4)
    row = np.asarray(r.coherent_image)[0]
    f = np.abs(np.fft.rfft(row))
    dominant = int(np.argmax(f[1:])) + 1
    return (row.max() - row.min()) / (row.max() + row.min()), dominant, f[4] / f[0]


def _assert_quarter_talbot(huv) -> None:
    c, k, _ = _talbot_quarter(huv, TALBOT_DUTY)
    assert c > 0.9 and k == 4, f"no period-halved image at z_T/4 for a {TALBOT_DUTY:.0%} grating: contrast {c:.3f}, index {k}"
    _, k50, a50 = _talbot_quarter(huv, 0.5)
    assert k50 != 4 and a50 < 0.05, f"a 50 % grating should have no p/2 image at z_T/4 (index {k50}, p/2 amplitude {a50:.3f})"


def talbot_compute(fast: bool):
    huv = native()
    _assert_quarter_talbot(huv)
    r = huv.simulate_talbot(13.5, 100.0, grating="amplitude", duty_cycle=TALBOT_DUTY, max_order=10,
                            carpet_nz=128 if fast else 512, nx=128 if fast else 256)
    dtl = np.asarray(r.stationary_image)
    mid = dtl.shape[0] // 2
    # the stationary DTL image has half the grating period: shifting it by p/2 reproduces it
    half = dtl.shape[1] // 4  # the field holds two periods
    assert _rms(np.roll(dtl[mid], half), dtl[mid]) < 1e-6, "the DTL image is not p/2-periodic"
    assert abs(r.talbot_length_nm - 2 * 100.0 ** 2 / 13.5) < 1e-6, "paraxial Talbot length drifted from 2p²/λ"
    return {"carpet": np.asarray(r.carpet), "x": np.asarray(r.carpet_x_nm), "z": np.asarray(r.carpet_z_nm),
            "zt": r.talbot_length_nm, "zt_exact": r.talbot_length_exact_nm, "coh": np.asarray(r.coherent_image),
            "dtl": dtl, "xi": np.asarray(r.x_nm)}


def talbot_draw(d, th: S.Theme):
    fig = plt.figure(figsize=(S.WIDE_WIDTH, 4.6))
    gs = fig.add_gridspec(1, 4, width_ratios=[1.5, 0.04, 0.22, 1.0], wspace=0.06)
    ax = fig.add_subplot(gs[0, 0])
    cmap = S.field_cmap()
    car = d["carpet"]
    zt = d["zt_exact"] or d["zt"]
    im = ax.imshow(car.T, origin="lower", aspect="auto", cmap=cmap, vmin=0, vmax=float(np.percentile(car, 99.5)),
                   extent=[d["z"][0] / zt, d["z"][-1] / zt, d["x"][0], d["x"][-1]], interpolation="nearest")
    S.image_axes(ax, th)
    for k in (0.5, 1.0, 1.5, 2.0):
        ax.axvline(k, color=th.axis, linewidth=S.HAIR_PT, linestyle=(0, (3, 3)))
    ax.set_xlabel(f"Distance from the grating / exact Talbot length ({zt:.1f} nm)")
    ax.set_ylabel("x (nm)")
    ax.set_title(f"(a) Talbot carpet, 100 nm amplitude grating ({TALBOT_DUTY * 100:.0f} % open) at 13.5 nm")
    cax = fig.add_subplot(gs[0, 1])
    cb = fig.colorbar(im, cax=cax)
    cb.outline.set_edgecolor(th.axis)
    cb.ax.tick_params(colors=th.axis, labelcolor=th.ink2, labelsize=S.SMALL_PT)
    cb.set_label("Intensity", color=th.ink2, fontsize=S.SMALL_PT)
    ax2 = fig.add_subplot(gs[0, 3])
    S.style_axes(ax2, th, grid="both")
    xi = d["xi"]
    mid = d["coh"].shape[0] // 2
    ax2.plot(xi, d["dtl"][mid], color=th.s(1), zorder=3)
    ax2.set_xlabel("x (nm)")
    ax2.set_title("(b) DTL image: period p/2 = 50 nm")
    return fig


register(FigureSpec(
    name="talbot-carpet",
    group="deep",
    compute=talbot_compute,
    draw=talbot_draw,
    alt="A Talbot carpet: the intensity behind a 100 nm grating, 25 percent open, lit at 13.5 nm repeats itself at "
        "the Talbot length and shows shifted and frequency-doubled images in between; beside it, the displacement "
        "Talbot image averaged over the gap has half the grating period.",
    caption="<strong>Talbot self-imaging at 13.5 nm.</strong> (a) Intensity behind a 100 nm-period binary "
            "amplitude grating, 25 % open (±10 orders, exact angular-spectrum propagation): the grating re-images "
            "at the Talbot length (2p²/λ ≈ 1481 nm paraxially, 1475 nm exactly for the ±1 orders) with the "
            "half-shifted and doubled images in between; a 50 % open grating is uniform at z<sub>T</sub>/4 and "
            "has no doubled image. (b) Averaging over a scanned gap (displacement Talbot lithography) gives a "
            "stationary image with half the period. <em>Model: Talbot / DTL / ATL 🔶 (scalar thin mask, "
            "coherent normal illumination, wavelength-independent grating coefficients).</em>",
    pages=("processes/talbot.md", "README.md", "python-api-modules.md", "future/alternative-patterning.md"),
    extra={"model_rows": {"Talbot / DTL / ATL": "Talbot / DTL / ATL proximity lithography"},
           "width": {"README.md": 280}},
))


# ---------------------------------------------------------------------------
# 7. Multi-beam interference lattices
# ---------------------------------------------------------------------------

UMBRELLA_DEG = 60.0


def interference_compute(fast: bool):
    huv = native()
    n_xy = 96 if fast else 160
    out = {}
    for preset in ("two_beam", "three_beam_hex"):
        v = huv.simulate_interference(preset, 200.0, 1.6, 30.0, n_xy, n_xy, 8, 1000.0, 1000.0, 200.0,
                                      dose_scale=20.0)
        vals = np.asarray(v.values)
        out[preset] = {"img": 1 - vals[0], "x": np.asarray(v.x_nm), "y": np.asarray(v.y_nm)}
    v = huv.simulate_interference("four_beam_umbrella", 200.0, 1.6, UMBRELLA_DEG, n_xy, 4, n_xy, 1000.0, 40.0,
                                  2000.0, dose_scale=20.0)
    vals = np.asarray(v.values)
    out["four_beam_umbrella"] = {"img": 1 - vals[:, 2, :], "x": np.asarray(v.x_nm), "z": np.asarray(v.z_nm)}
    # two beams at ±30° in air write lines of period λ/(2 sin θ) = 200 nm, independent of the resist index
    row = out["two_beam"]["img"][0]
    spec = np.abs(np.fft.rfft(row - row.mean()))
    k = int(np.argmax(spec[1:]) + 1)
    period = 1000.0 / k
    assert abs(period - 200.0) < 1000.0 / len(row), f"two-beam period {period:.1f} nm is not λ/(2 sin 30°) = 200 nm"
    return out


def interference_draw(d, th: S.Theme):
    fig, axes = plt.subplots(1, 3, figsize=(S.WIDE_WIDTH, 3.6), gridspec_kw={"wspace": 0.3,
                                                                              "width_ratios": [1, 1, 1]})
    cmap = S.field_cmap()
    panels = (("two_beam", "(a) two beams, 30°: lines (x–y)"), ("three_beam_hex", "(b) three beams, 30°: hexagonal (x–y)"),
              ("four_beam_umbrella", f"(c) four beams, {UMBRELLA_DEG:.0f}°: x–z slice"))
    vmax = max(float(d[p]["img"].max()) for p, _ in panels)
    im = None
    for ax, (preset, title) in zip(axes, panels):
        e = d[preset]
        if preset != "four_beam_umbrella":
            ext = [e["x"][0], e["x"][-1], e["y"][0], e["y"][-1]]
            im = ax.imshow(e["img"], origin="lower", extent=ext, cmap=cmap, vmin=0, vmax=vmax, interpolation="nearest")
            ax.set_ylabel("y (nm)", fontsize=S.SMALL_PT)
        else:
            ext = [e["x"][0], e["x"][-1], e["z"][-1], e["z"][0]]
            im = ax.imshow(e["img"], origin="upper", extent=ext, cmap=cmap, vmin=0, vmax=vmax, interpolation="nearest")
            ax.set_ylabel("depth (nm)", fontsize=S.SMALL_PT)
        ax.set_aspect("auto")
        S.image_axes(ax, th)
        ax.set_title(title, fontsize=S.SMALL_PT + 0.5)
        ax.set_xlabel("x (nm)", fontsize=S.SMALL_PT)
        ax.tick_params(labelsize=S.SMALL_PT - 1)
    S.colorbar(fig, im, list(axes), th, "Exposed fraction 1 − m", fraction=0.02, pad=0.02)
    return fig


register(FigureSpec(
    name="interference-lattice",
    group="deep",
    compute=interference_compute,
    draw=interference_draw,
    alt="Three exposure maps from multi-beam interference: parallel lines from two beams, a hexagonal dot lattice "
        "from three beams, and a vertical slice through the three-dimensional lattice written by four beams.",
    caption="<strong>Lattices written by interfering plane waves.</strong> Coherent 200 nm beams recorded in a "
            "resist of index 1.6 as a Dill latent image (exposed fraction shown): two beams write lines and three "
            "a hexagonal lattice (30° air-side half-angle, top slice); four beams in an umbrella (60°) write a 3-D "
            "lattice, here cut in x–z. <em>Model: multi-beam interference ✅ (🔶 per-beam scalar absorption; "
            "single-exposure Dill kinetics).</em>",
    pages=("processes/interference-volumetric.md",),
    extra={"model_rows": {"multi-beam interference": "Multi-beam interference / two-photon"}},
))


# ---------------------------------------------------------------------------
# 8. Grayscale lithography: microlens target vs printed relief
# ---------------------------------------------------------------------------


def grayscale_compute(fast: bool):
    huv = native()
    n = 64 if fast else 128
    pixel = 8.0 * 128 / n
    thickness, d_th, d_clear, dose = 500.0, 20.0, 80.0, 100.0
    target = np.asarray(huv.microlens_array(n, n // 2, 300.0, thickness))
    trans = np.asarray(huv.grayscale_transmittance_for_target(target, thickness, d_th, d_clear, dose))
    src = huv.SourceConfig(wavelength_nm=WAVELENGTH_NM, sigma_outer=0.7)
    hm = huv.grayscale_height_map(src, huv.OpticsConfig(numerical_aperture=0.75), huv.GridConfig(n, pixel), trans,
                                  dose, d_th, d_clear, thickness, 20)
    printed = np.asarray(hm.values)
    # the printed relief follows the target: the band-limited image rounds the cusps but keeps the sag
    assert 0.0 <= printed.min() and printed.max() <= thickness + 1e-6, "printed height outside the resist"
    assert abs(float(printed.max() - printed.min()) - 300.0) < 60.0, "printed sag is not the 300 nm target"
    return {"target": target, "trans": trans, "printed": printed, "x": np.asarray(hm.x_nm),
            "thickness": thickness}


def grayscale_draw(d, th: S.Theme):
    fig = plt.figure(figsize=(S.WIDE_WIDTH, 3.2))
    gs = fig.add_gridspec(1, 3, width_ratios=[1, 1, 1.3], wspace=0.62)
    cmap = S.field_cmap()
    x = d["x"]
    ext = [x[0], x[-1], x[0], x[-1]]
    ax0 = fig.add_subplot(gs[0, 0])
    im0 = ax0.imshow(d["trans"], origin="lower", extent=ext, cmap=cmap, vmin=0, vmax=1, interpolation="nearest")
    S.image_axes(ax0, th)
    ax0.set_title("(a) designed mask transmission")
    S.colorbar(fig, im0, ax0, th, "Intensity transmission")
    ax1 = fig.add_subplot(gs[0, 1])
    im1 = ax1.imshow(d["printed"], origin="lower", extent=ext, cmap=cmap, vmin=0, vmax=d["thickness"],
                     interpolation="bilinear")
    S.image_axes(ax1, th)
    ax1.set_title("(b) printed resist height")
    S.colorbar(fig, im1, ax1, th, "Height (nm)")
    for a in (ax0, ax1):
        a.set_xlabel("x (nm)")
        a.tick_params(labelsize=S.SMALL_PT - 1)
    ax0.set_ylabel("y (nm)")
    ax2 = fig.add_subplot(gs[0, 2])
    S.style_axes(ax2, th, grid="both")
    mid = d["printed"].shape[0] // 4  # through the lens centres of the first row
    ax2.plot(x, d["target"][mid], color=th.ink2, linewidth=S.HAIR_PT * 1.5, linestyle=(0, (4, 3)), zorder=2)
    ax2.plot(x, d["printed"][mid], color=th.s(1), zorder=3)
    ax2.set_xlim(x[0], x[-1])
    ax2.set_ylim(0, d["thickness"] * 1.05)
    ax2.set_xlabel("x (nm)")
    ax2.set_ylabel("Height (nm)")
    ax2.set_title("(c) cut through a lens row")
    ax2.legend([Line2D([], [], color=th.ink2, linestyle=(0, (4, 3))), Line2D([], [], color=th.s(1))],
               ["target microlens profile", "printed (imaged + contrast curve)"], loc="lower center")
    return fig


register(FigureSpec(
    name="grayscale-relief",
    group="deep",
    compute=grayscale_compute,
    draw=grayscale_draw,
    alt="Grayscale lithography of a microlens array: the designed continuous-tone mask, the printed resist "
        "height map with domed lenses, and a cut comparing the printed profile with the target, which it follows "
        "except for the rounding of sharp transitions by the optics.",
    caption="<strong>A continuous-tone mask printed as relief.</strong> A 2 × 2 microlens array (300 nm sag in "
            "500 nm resist) designed through the log-linear contrast curve (threshold 20, clearing 80 mJ/cm², "
            "100 mJ/cm² exposure), imaged with F<sub>2</sub> 157.63 nm, NA 0.75, σ 0.7 and mapped back to "
            "remaining height; the optics band-limit the relief and round the cusps between lenses. "
            "<em>Model: grayscale lithography 🔶 (thin-film contrast-curve resist; no standing waves, bake or "
            "development dynamics).</em>",
    pages=("processes/grayscale.md",),
))


# ---------------------------------------------------------------------------
# 9. P1 · process-pipeline-strip: one exposure, five stages (the Processes signature)
# ---------------------------------------------------------------------------

STRIP_DOSE, STRIP_PEB_XY, STRIP_PEB_Z, STRIP_T_DEV = 24.0, 20.0, 10.0, 30.0
STRIP_RATE_RATIO, STRIP_INHIBITION_NM = 0.3, 15.0


def _process_scene(huv, fast: bool, nz: int):
    """Expose, bake and return (grid, latent, baked) of the canonical scene."""
    src, optics, mask = _scene(huv)
    n = 64 if fast else 128
    grid = mask.commensurate_grid(n, 600.0 / n)
    res = huv.ResistConfig()
    lat = huv.expose_volumetric(src, optics, mask, huv.FilmStackConfig(), res, grid, dose_mj_cm2=STRIP_DOSE,
                                nz=nz, n_defocus_planes=4, dose_steps=5)
    baked = huv.peb_gaussian(lat, STRIP_PEB_XY, vertical_nm=STRIP_PEB_Z)
    return src, optics, mask, grid, res, lat, baked


def strip_compute(fast: bool):
    huv = native()
    nz = 24 if fast else 48
    src, optics, mask, grid, res, lat, baked = _process_scene(huv, fast, nz)
    # (a) the aerial image, clear-field normalised
    eng = huv.SimulationEngine(src, optics, mask, grid=grid, max_kernels=20)
    img = eng.compute_aerial_image(0.0)
    I = np.asarray(img.intensity)
    aerial = I[I.shape[0] // 2] / eng.clear_field_intensity()
    xi = np.asarray(img.x_nm)
    # (b) the exposing intensity S(z) = (n_r/n_0)|E(z)|² of the default 150 nm resist on Si
    fs = huv.FilmStackConfig()
    n_r, n_0 = fs.layers[0][2], fs.superstrate.real
    zs = np.linspace(0.0, res.thickness_nm, 601)
    S_z = (n_r / n_0) * np.asarray(fs.intensity_profile(WAVELENGTH_NM, list(zs)))
    period = WAVELENGTH_NM / (2 * n_r)
    maxima = zs[1:-1][(S_z[1:-1] > S_z[:-2]) & (S_z[1:-1] >= S_z[2:])]
    spacing = float(np.mean(np.diff(maxima)))
    assert abs(spacing - period) < 0.03 * period, f"standing-wave period {spacing:.1f} nm ≠ λ/2n = {period:.1f} nm"
    # (e) development: FMM arrival times (static rate) and the level-set front at 30 s
    x, z = np.asarray(baked.x_nm), np.asarray(baked.z_nm)
    dx, dz = float(x[1] - x[0]), float(z[1] - z[0])
    fm = huv.develop_fast_marching(baked, res, dx, dz, surface_rate_ratio=STRIP_RATE_RATIO,
                                   inhibition_depth_nm=STRIP_INHIBITION_NM, lateral="periodic")
    ls = huv.develop_level_set(baked, res, STRIP_T_DEV, surface_rate_ratio=STRIP_RATE_RATIO,
                               inhibition_depth_nm=STRIP_INHIBITION_NM)
    mid = baked.values.shape[1] // 2
    fmt = np.asarray(fm.values)[:, mid, :]
    lst = np.asarray(ls.arrival_times.values)[:, mid, :]
    fmt = np.where(np.isfinite(fmt), fmt, 1e9)
    lst = np.where(np.isfinite(lst), lst, 1e9)
    f_fm, f_ls = float(np.mean(fmt <= STRIP_T_DEV)), float(np.mean(lst <= STRIP_T_DEV))
    assert abs(f_fm - f_ls) < 0.05, f"FMM and level-set developed fractions at 30 s differ: {f_fm:.3f} vs {f_ls:.3f}"
    return {"xi": xi, "aerial": aerial, "contrast": img.image_contrast(),
            "zs": zs, "S": S_z, "period": period, "swing": (float(S_z.min()), float(S_z.max())), "maxima": maxima,
            "x": x, "z": z, "latent": 1 - np.asarray(lat.values)[:, mid, :],
            "baked": 1 - np.asarray(baked.values)[:, mid, :], "fm": fmt, "ls": lst}


STRIP_ISO_STYLES = (":", "--", "-")  # fast-marching isochrones 10 / 20 / 30 s


def strip_draw(d, th: S.Theme):
    fig = plt.figure(figsize=(S.WIDE_WIDTH + 1.0, 3.7))
    gs = fig.add_gridspec(1, 6, width_ratios=[1.0, 0.62, 1.0, 1.0, 1.0, 0.045], wspace=0.42)
    x, z = d["x"], d["z"]
    cmap = S.field_cmap()
    # (a) aerial image
    ax = fig.add_subplot(gs[0, 0])
    S.style_axes(ax, th, grid="both")
    ax.fill_between(d["xi"], 0, d["aerial"], color=th.s(1), alpha=0.12, lw=0)
    ax.plot(d["xi"], d["aerial"], color=th.s(1), zorder=3)
    ax.set_xlim(x[0], x[-1])
    ax.set_ylim(0, 1.05)
    ax.set_xlabel("x (nm)")
    ax.set_ylabel("I / clear field")
    ax.set_title("(a) aerial image")
    ax.text(0.03, 0.06, f"contrast {d['contrast']:.2f}", transform=ax.transAxes, color=th.ink2, fontsize=S.SMALL_PT)
    # (b) S(z): depth vertical, shared with the x–z panels
    ax = fig.add_subplot(gs[0, 1])
    S.style_axes(ax, th, grid="both")
    ax.plot(d["S"], d["zs"], color=th.s(2), zorder=3)
    ax.set_ylim(z[-1] + (z[1] - z[0]) / 2, 0)
    ax.set_xlim(0, float(d["S"].max()) * 1.15)
    ax.set_xlabel("S(z)")
    ax.set_ylabel("depth z (nm)")
    ax.set_title("(b) in-resist S(z)")
    mx = d["maxima"]
    if len(mx) >= 2:
        z1, z2 = float(mx[0]), float(mx[1])
        xv = float(d["S"].max()) * 1.05
        ax.annotate("", xy=(xv, z1), xytext=(xv, z2), arrowprops=dict(arrowstyle="<->", color=th.ink2, lw=S.HAIR_PT))
        ax.text(xv - 0.06 * xv, (z1 + z2) / 2, f"λ/2n = {d['period']:.1f} nm", color=th.ink2, fontsize=S.SMALL_PT - 1,
                rotation=90, ha="right", va="center")
    lo, hi = d["swing"]
    ax.text(0.04, 0.98, f"swing {lo:.2f}–{hi:.2f}", transform=ax.transAxes, color=th.ink2, fontsize=S.SMALL_PT - 1,
            va="top")
    # (c) latent, (d) baked, (e) developed
    ext = [x[0], x[-1], z[-1], z[0]]
    im = None
    for k, (key, title) in enumerate((("latent", "(c) latent 1 − m(x, z)"), ("baked", f"(d) baked σ {STRIP_PEB_XY:.0f}/{STRIP_PEB_Z:.0f} nm"),
                                      ("baked", f"(e) developed, {STRIP_T_DEV:.0f} s"))):
        ax = fig.add_subplot(gs[0, 2 + k])
        im = ax.imshow(d[key], origin="upper", extent=ext, aspect="auto", cmap=cmap, vmin=0, vmax=1,
                       interpolation="bilinear")
        if k == 2:
            removed = np.ma.masked_where(d["ls"] > STRIP_T_DEV, np.ones_like(d["ls"]))
            ax.imshow(removed, origin="upper", extent=ext, aspect="auto", cmap=ListedColormap([th.band]), vmin=0,
                      vmax=1, interpolation="nearest", zorder=2)
            for ls, t in zip(STRIP_ISO_STYLES, (10.0, 20.0, 30.0)):
                ax.contour(x, z, d["fm"], levels=[t], colors=[th.s(2)], linestyles=[ls], linewidths=S.HAIR_PT * 1.6,
                           zorder=3)
            ax.contour(x, z, d["ls"], levels=[STRIP_T_DEV], colors=[th.s(1)], linewidths=S.LINE_PT, zorder=4)
        S.image_axes(ax, th)
        ax.set_title(title)
        ax.set_xlabel("x (nm)")
        ax.tick_params(labelleft=False)
        ax.set_xlim(x[0], x[-1])
        ax.set_ylim(z[-1] + (z[1] - z[0]) / 2, 0)
    cax = fig.add_subplot(gs[0, 5])
    cb = fig.colorbar(im, cax=cax)
    cb.outline.set_edgecolor(th.axis)
    cb.ax.tick_params(colors=th.axis, labelcolor=th.ink2, labelsize=S.SMALL_PT)
    cb.set_label("exposed fraction 1 − m", color=th.ink2, fontsize=S.SMALL_PT)
    fig.legend([Line2D([], [], color=th.s(2), linestyle=ls, linewidth=S.HAIR_PT * 1.6) for ls in STRIP_ISO_STYLES]
               + [Line2D([], [], color=th.s(1))],
               [f"fast-marching isochrone {t:.0f} s" for t in (10, 20, 30)] + ["level-set front at 30 s"],
               loc="lower center", ncol=4, bbox_to_anchor=(0.62, -0.02), fontsize=S.SMALL_PT - 0.5)
    fig.subplots_adjust(bottom=0.3, left=0.06, right=0.95, top=0.88)
    return fig


register(FigureSpec(
    name="process-pipeline-strip",
    group="deep",
    compute=strip_compute,
    draw=strip_draw,
    alt="Five panels along the process pipeline for one exposure: the aerial image of 150 nm lines, the standing-"
        "wave exposing intensity through the resist, the banded latent image, the same image smoothed by the bake, "
        "and the developed profile at 30 s with fast-marching isochrones and the level-set front.",
    caption="<strong>One exposure, five stages.</strong> 150 nm lines on a 300 nm pitch at 157.63 nm, NA 0.75, "
            "σ 0.7 in 150 nm of resist on Si, 24 mJ/cm²: (a) the aerial image; (b) the exposing intensity "
            "S(z) = (n<sub>r</sub>/n<sub>0</sub>)|E(z)|² with its λ/2n standing wave; (c) the banded latent image; "
            "(d) after the 20/10 nm Gaussian bake; (e) development at 30 s with surface inhibition (rate ratio 0.3, "
            "15 nm) — fast-marching isochrones and the level-set front. The default resist is illustrative and "
            "low-contrast. <em>Model: Hopkins imaging ✅ · thin-film transfer matrix ✅ · volumetric exposure 🔶 "
            "· post-exposure bake ✅ · 3D development (fast marching) ✅ · 3D development (level set) ✅ "
            "(🔶 first-order front).</em>",
    pages=("processes/index.md", "pipeline.md", "README.md"),
    hero_on=("processes/index.md",),
    extra={"model_rows": {"Hopkins imaging": "Hopkins imaging (factorized TCC/SOCS)",
                          "volumetric exposure": "Volumetric (z-resolved) exposure",
                          "post-exposure bake": "Post-exposure bake (anisotropic Gaussian)"},
           "width": {"README.md": 820}},
))


# ---------------------------------------------------------------------------
# 10. P2 · levelset-development: the moving front, t = 0 … 30 s (APNG + still + README GIF)
# ---------------------------------------------------------------------------

LS_FRAMES, LS_HOLD = 30, 6


def levelset_compute(fast: bool):
    huv = native()
    nz = 24 if fast else 48
    src, optics, mask, grid, res, lat, baked = _process_scene(huv, fast, nz)
    ls = huv.develop_level_set(baked, res, STRIP_T_DEV, surface_rate_ratio=STRIP_RATE_RATIO,
                               inhibition_depth_nm=STRIP_INHIBITION_NM)
    times = list(np.linspace(0.0, STRIP_T_DEV, LS_FRAMES)) + [STRIP_T_DEV] * LS_HOLD
    # the developed region can only grow: developed(t) is monotone in t
    areas = [float(np.asarray(ls.developed(float(t)).values).sum()) for t in times[:LS_FRAMES]]
    assert all(a <= b + 1e-9 for a, b in zip(areas, areas[1:])), "developed(t) is not monotone in t"
    assert areas[0] == 0.0 and areas[-1] > 0.0, "nothing developed, or something developed at t = 0"
    mid = baked.values.shape[1] // 2
    plane_t = np.asarray(ls.arrival_times.values)[:, mid, :]
    plane_t = np.where(np.isfinite(plane_t), plane_t, 1e9)
    x, z = np.asarray(baked.x_nm), np.asarray(baked.z_nm)
    return {"x": x, "z": z, "field": 1 - np.asarray(baked.values)[:, mid, :], "t": plane_t, "times": times,
            "areas": areas, "thickness": res.thickness_nm, "steps": ls.steps}


def levelset_draw(d, th: S.Theme):
    x, z, field, plane_t = d["x"], d["z"], d["field"], d["t"]
    dz = float(z[1] - z[0])
    ext = [x[0], x[-1], z[-1] + dz / 2, z[0] - dz / 2]
    cmap = S.field_cmap()
    frames = []
    plt.rcParams["figure.max_open_warning"] = 0  # the frame writer closes the 36 figures after saving
    for t in d["times"]:
        fig = plt.figure(figsize=(S.ANIM_WIDTH_PX / S.LOGICAL_DPI, 4.0))
        gs = fig.add_gridspec(1, 2, width_ratios=[1.35, 1], wspace=0.3)
        ax0 = fig.add_subplot(gs[0, 0])
        ax0.imshow(field, origin="upper", extent=ext, aspect="auto", cmap=cmap, vmin=0, vmax=1,
                   interpolation="bilinear")
        removed = np.ma.masked_where(plane_t > t, np.ones_like(plane_t))
        ax0.imshow(removed, origin="upper", extent=ext, aspect="auto", cmap=ListedColormap([th.band]), vmin=0, vmax=1,
                   interpolation="nearest", zorder=2)
        if t > 0 and plane_t.min() < t:
            ax0.contour(x, z, plane_t, levels=[t], colors=[th.s(1)], linewidths=S.LINE_PT, zorder=3)
        S.image_axes(ax0, th)
        ax0.set_xlabel("x (nm)")
        ax0.set_ylabel("depth z (nm)")
        ax0.set_title(f"x–z slice: baked latent image, developed region at t = {t:4.1f} s")
        ax1 = fig.add_subplot(gs[0, 1])
        S.style_axes(ax1, th, grid="y")
        remaining = (plane_t > t).sum(axis=0) * dz
        ax1.fill_between(x, 0, remaining, color=th.s(1), alpha=0.25, step="mid", lw=0)
        ax1.step(x, remaining, where="mid", color=th.s(1), linewidth=S.LINE_PT * 0.9)
        ax1.set_xlim(x[0], x[-1])
        ax1.set_ylim(0, d["thickness"] * 1.04)
        ax1.set_xlabel("x (nm)")
        ax1.set_ylabel("resist remaining per column (nm)")
        ax1.set_title("resist remaining")
        ax1.text(0.97, 0.95, f"t = {t:4.1f} s of {STRIP_T_DEV:.0f} s", transform=ax1.transAxes, color=th.ink2,
                 fontsize=S.SMALL_PT, ha="right", va="top")
        fig.subplots_adjust(left=0.075, right=0.985, bottom=0.17, top=0.88)
        frames.append(fig)
    return frames


register(FigureSpec(
    name="levelset-development",
    group="deep",
    compute=levelset_compute,
    draw=levelset_draw,
    alt="Animation: a level-set development front advancing through the baked latent image of 150 nm lines in "
        "150 nm of resist over 30 seconds, the developed region opening from the top and the remaining resist "
        "height per column shrinking beside it.",
    caption="<strong>Development as a moving front.</strong> x–z slice of the baked latent image with the "
            "developed region and the level-set front at t = 0 … 30 s (F<sub>2</sub> σ 0.7, NA 0.75, 150/300 nm "
            "lines, 150 nm resist on Si, 24 mJ/cm², Gaussian bake 20/10 nm, surface inhibition 15 nm, surface-rate "
            "ratio 0.3); right, resist remaining per column. The default resist is illustrative and low-contrast. "
            "<em>Model: volumetric exposure 🔶 (separable I<sub>aer</sub>·S(z)) · anisotropic Gaussian bake ✅ · "
            "level-set development ✅ (🔶 first-order front, rate re-evaluated at the front).</em>",
    pages=("processes/volumetric-exposure.md", "README.md", "processes/index.md"),
    subdir="anim",
    fmt="apng",
    still_frame=LS_FRAMES + LS_HOLD - 1,
    extra={"gif": True, "duration_ms": 180,
           "model_rows": {"volumetric exposure": "Volumetric (z-resolved) exposure",
                          "anisotropic Gaussian bake": "Post-exposure bake (anisotropic Gaussian)",
                          "level-set development": "3D development (level set)"}},
))


# ---------------------------------------------------------------------------
# 11. P3 · volumetric-latent-xz: the separable model made visible
# ---------------------------------------------------------------------------

LATENT_THRESHOLD = 0.7
ARC_NM, ARC_N, ARC_K = 60.0, 1.8, 0.3


def _sw_amplitude(col, z, period):
    """Amplitude of the standing-wave harmonic (period ``period``) in a depth profile, interior only."""
    sel = (z > 15.0) & (z < z[-1] - 15.0)
    zz = z[sel]
    c = col[sel] - np.polyval(np.polyfit(zz, col[sel], 2), zz)
    w = 2 * np.pi / period
    return 2 * float(np.hypot(np.mean(c * np.cos(w * zz)), np.mean(c * np.sin(w * zz))))


def latent_compute(fast: bool):
    huv = native()
    src, optics, mask = _scene(huv)
    n = 64 if fast else 128
    grid = mask.commensurate_grid(n, 600.0 / n)
    res = huv.ResistConfig()
    nz = 32 if fast else 64
    bare = huv.FilmStackConfig()
    arc = huv.FilmStackConfig()
    arc.add_layer("ARC", ARC_NM, ARC_N, ARC_K)
    n_r = bare.layers[0][2]
    period = WAVELENGTH_NM / (2 * n_r)
    kw = dict(dose_mj_cm2=STRIP_DOSE, nz=nz, n_defocus_planes=4, dose_steps=5)
    lat_bare = huv.expose_volumetric(src, optics, mask, bare, res, grid, **kw)
    lat_arc = huv.expose_volumetric(src, optics, mask, arc, res, grid, **kw)
    b5 = huv.peb_gaussian(lat_bare, STRIP_PEB_XY, vertical_nm=5.0)
    b20 = huv.peb_gaussian(lat_bare, STRIP_PEB_XY, vertical_nm=20.0)
    z = np.asarray(lat_bare.z_nm)
    mid = lat_bare.values.shape[1] // 2
    m_bare, m_arc = np.asarray(lat_bare.values)[:, mid, :], np.asarray(lat_arc.values)[:, mid, :]
    m5, m20 = np.asarray(b5.values)[:, mid, :], np.asarray(b20.values)[:, mid, :]
    a0 = _sw_amplitude(m_bare.mean(axis=1), z, period)
    damp = {5.0: _sw_amplitude(m5.mean(axis=1), z, period) / a0, 20.0: _sw_amplitude(m20.mean(axis=1), z, period) / a0}
    closed = {s: float(np.exp(-2 * np.pi ** 2 * s ** 2 / period ** 2)) for s in (5.0, 20.0)}
    arc_ratio = _sw_amplitude(m_arc.mean(axis=1), z, period) / a0
    # the exact spectral Gaussian damps the standing wave by exp(-2π²σ_z²/P²); the ARC suppresses it at the source
    assert abs(damp[5.0] - closed[5.0]) < 0.05, f"5 nm bake damping {damp[5.0]:.3f} ≠ closed form {closed[5.0]:.3f}"
    assert damp[20.0] < 0.1, f"20 nm bake leaves {damp[20.0]:.3f} of the standing wave"
    assert arc_ratio < 0.2, f"the ARC leaves {arc_ratio:.3f} of the bare-Si standing wave"
    cds = {}
    for key, vol in (("bare", lat_bare), ("5 nm", b5), ("20 nm", b20)):
        cds[key] = [None if c is None else float(c) for c in vol.cd_at_z(LATENT_THRESHOLD)]
    assert all(c is not None for c in cds["20 nm"]), "the 20 nm-baked latent CD is undefined in some slice"
    R = {"bare": bare.reflectance(WAVELENGTH_NM), "arc": arc.reflectance(WAVELENGTH_NM)}
    return {"x": np.asarray(lat_bare.x_nm), "z": z, "period": period,
            "bare": 1 - m_bare, "arc": 1 - m_arc, "b5": 1 - m5, "b20": 1 - m20,
            "damp": damp, "closed": closed, "arc_ratio": arc_ratio, "cds": cds, "R": R}


def latent_draw(d, th: S.Theme):
    fig = plt.figure(figsize=(S.WIDE_WIDTH + 0.4, 3.3))
    gs = fig.add_gridspec(1, 7, width_ratios=[1, 1, 1, 1, 0.045, 0.18, 1.25], wspace=0.3)
    x, z = d["x"], d["z"]
    dz = float(z[1] - z[0])
    ext = [x[0], x[-1], z[-1] + dz / 2, z[0] - dz / 2]
    cmap = S.field_cmap()
    panels = (("bare", "(a) bare Si"), ("arc", f"(a′) {ARC_NM:.0f} nm ARC, n {ARC_N} + {ARC_K}i"),
              ("b5", "(b) bare, bake σ$_z$ 5 nm"), ("b20", "(b′) bare, bake σ$_z$ 20 nm"))
    im = None
    for k, (key, title) in enumerate(panels):
        ax = fig.add_subplot(gs[0, k])
        im = ax.imshow(d[key], origin="upper", extent=ext, aspect="auto", cmap=cmap, vmin=0, vmax=1,
                       interpolation="bilinear")
        S.image_axes(ax, th)
        ax.set_title(title, fontsize=S.SMALL_PT + 0.5)
        ax.set_xlabel("x (nm)")
        if k == 0:
            ax.set_ylabel("depth z (nm)")
        else:
            ax.tick_params(labelleft=False)
        note = {"bare": f"R = {d['R']['bare']:.2f}", "arc": f"R = {d['R']['arc']:.3f}",
                "b5": f"damping {d['damp'][5.0]:.2f} (closed form {d['closed'][5.0]:.2f})",
                "b20": f"damping {d['damp'][20.0]:.2f} (closed form {d['closed'][20.0]:.2f})"}[key]
        ax.text(0.03, 0.04, note, transform=ax.transAxes, color="#ffffff", fontsize=S.SMALL_PT - 1.2,
                bbox=dict(boxstyle="round,pad=0.25", facecolor="#0b0b0b", alpha=0.55, lw=0))
    cax = fig.add_subplot(gs[0, 4])
    cb = fig.colorbar(im, cax=cax)
    cb.outline.set_edgecolor(th.axis)
    cb.ax.tick_params(colors=th.axis, labelcolor=th.ink2, labelsize=S.SMALL_PT)
    cb.set_label("exposed fraction 1 − m", color=th.ink2, fontsize=S.SMALL_PT)
    ax = fig.add_subplot(gs[0, 6])
    S.style_axes(ax, th, grid="both")
    cols = {"bare": th.ink2, "5 nm": th.s(2), "20 nm": th.s(1)}
    for key, cd in d["cds"].items():
        zz = np.array([zi for zi, c in zip(z, cd) if c is not None])
        cc = np.array([c for c in cd if c is not None])
        if key == "bare":
            S.dot(ax, cc, zz, cols[key], th, size_px=5.5)
        else:
            ax.plot(cc, zz, color=cols[key], zorder=3)
    ax.set_ylim(z[-1] + dz / 2, 0)
    cd_all = np.array([c for cd in d["cds"].values() for c in cd if c is not None])
    ax.set_xlim(cd_all.min() - 40.0, cd_all.max() + 10.0)  # air at the left for the legend
    ax.set_xlabel(f"latent CD at m = {LATENT_THRESHOLD} (nm)")
    ax.tick_params(labelleft=False)  # the depth axis is shared with (a)–(b′)
    ax.set_title("(c) latent CD vs depth", fontsize=S.SMALL_PT + 0.5)
    ax.legend([Line2D([], [], marker="o", linestyle="none", color=cols["bare"], markersize=5),
               Line2D([], [], color=cols["5 nm"]), Line2D([], [], color=cols["20 nm"])],
              ["unbaked (gaps at the nodes)", "bake σ$_z$ 5 nm", "bake σ$_z$ 20 nm"], loc="upper left",
              fontsize=S.SMALL_PT - 1, framealpha=0.92, handlelength=1.4)
    fig.subplots_adjust(left=0.05, right=0.985, bottom=0.17, top=0.88)
    return fig


register(FigureSpec(
    name="volumetric-latent-xz",
    group="deep",
    compute=latent_compute,
    draw=latent_draw,
    alt="Four x–z latent images of 150 nm lines in 150 nm resist: bare silicon with strong standing-wave bands, a "
        "60 nm anti-reflection layer that suppresses them, and the bare case after 5 and 20 nm vertical bakes; "
        "beside them, latent linewidth against depth, undefined at the nodes until a bake smooths it.",
    caption="<strong>The separable model, seen.</strong> Latent image 1 − m(x, z) of 150 nm lines on a 300 nm "
            "pitch (157.63 nm, NA 0.75, σ 0.7, 24 mJ/cm²): (a) 150 nm resist on bare Si bands with the λ/2n "
            "standing wave, (a′) a 60 nm ARC (1.8 + 0.3i, the test fixture's constants) suppresses it; (b, b′) the "
            "bare-Si image after a 5 nm and a 20 nm vertical Gaussian bake, damped by exp(−2π²σ<sub>z</sub>²/P²) "
            "= 0.81 and 0.03; (c) the latent linewidth per slice, undefined at the nodes until the bake smooths "
            "them. The default resist is illustrative. <em>Model: volumetric exposure 🔶 (separable I<sub>aer</sub>"
            "·S(z)) · post-exposure bake ✅ · thin-film transfer matrix ✅.</em>",
    pages=("processes/volumetric-exposure.md",),
    extra={"model_rows": {"volumetric exposure": "Volumetric (z-resolved) exposure",
                          "post-exposure bake": "Post-exposure bake (anisotropic Gaussian)"}},
))


# ---------------------------------------------------------------------------
# 12. P8a · talbot-flythrough-anim: flying through the Talbot carpet (two-layer SVG)
# ---------------------------------------------------------------------------

FLY_LAMBDA, FLY_PERIOD, FLY_ORDERS = 13.5, 100.0, 10
FLY_ROWS = 24  # live profiles per 2 z_T (one every z_T/12)


def _palette_png(index_rows: np.ndarray, palette) -> bytes:
    """An 8-bit indexed PNG (one byte per pixel) — a third of the RGB form svgkit writes."""
    h, w = index_rows.shape

    def chunk(tag: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    raw = b"".join(b"\x00" + index_rows[i].astype(np.uint8).tobytes() for i in range(h))
    plte = b"".join(bytes(c) for c in palette)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 3, 0, 0, 0))
            + chunk(b"PLTE", plte) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def talbot_fly_compute(fast: bool):
    huv = native()
    _assert_quarter_talbot(huv)
    kw = dict(grating="amplitude", duty_cycle=TALBOT_DUTY, max_order=FLY_ORDERS, nx=128)

    def coherent(gap, prop="exact", orders=FLY_ORDERS):
        r = huv.simulate_talbot(FLY_LAMBDA, FLY_PERIOD, grating="amplitude", duty_cycle=TALBOT_DUTY, max_order=orders,
                                propagation=prop, gap_nm=float(gap), nx=128, carpet_nz=4)
        return np.asarray(r.coherent_image)[0], r

    _, r0 = coherent(0.0)
    zt, zte = r0.talbot_length_nm, r0.talbot_length_exact_nm
    assert abs(zt - 2 * FLY_PERIOD ** 2 / FLY_LAMBDA) < 1e-6, "paraxial Talbot length drifted from 2p²/λ"
    nz = 120 if fast else 240
    r = huv.simulate_talbot(FLY_LAMBDA, FLY_PERIOD, carpet_nz=nz, carpet_z_max_nm=2 * zte, **kw)
    carpet, cz, cx = np.asarray(r.carpet), np.asarray(r.carpet_z_nm), np.asarray(r.carpet_x_nm)
    rows = [coherent(k * zte / (FLY_ROWS / 2))[0] for k in range(FLY_ROWS + 1)]
    # the self-image: exact for the paraxial carpet and for the ±1 orders at the exact Talbot length;
    # at λ/p = 0.135 the higher orders are non-paraxial, so the exact ±10-order revival is only approximate
    par0, _ = coherent(0.0, "paraxial")
    par_t, _ = coherent(zt, "paraxial")
    assert _rms(par_t, par0) < 0.02, f"paraxial self-image at z_T off by {_rms(par_t, par0):.3f} rms"
    par_h, _ = coherent(zt / 2, "paraxial")
    assert _rms(par_h, np.roll(par0, len(par0) // 4)) < 0.02, "paraxial image at z_T/2 is not the p/2-shifted grating"
    one0, _ = coherent(0.0, "exact", 1)
    one_t, _ = coherent(zte, "exact", 1)
    assert _rms(one_t, one0) < 0.02, f"±1-order exact self-image at z_T off by {_rms(one_t, one0):.3f} rms"
    exact_dev = _rms(rows[FLY_ROWS // 2], rows[0])
    # the z_T/4 pause: the exact row the cursor stops on is the period-halved image (p/2 = index 4 on two periods)
    q = rows[FLY_ROWS // 8]  # rows are one per z_T/12, so z_T/4 is row 3
    fq = np.abs(np.fft.rfft(q))
    assert (q.max() - q.min()) / (q.max() + q.min()) > 0.9 and int(np.argmax(fq[1:])) + 1 == 4, \
        "the z_T/4 row the animation pauses on is not the period-halved image"
    # the running gap average over [z_T, 2 z_T] is the engine's DTL image (half the period)
    rd = huv.simulate_talbot(FLY_LAMBDA, FLY_PERIOD, gap_nm=zte, scan_length_nm=zte, carpet_nz=4, **kw)
    dtl = np.asarray(rd.dtl_image)[0]
    stationary = np.asarray(rd.stationary_image)[0]
    averages = {}
    for k in range(FLY_ROWS * 3 // 4, FLY_ROWS + 1):
        sel = (cz >= zte) & (cz <= k * zte / (FLY_ROWS / 2))
        averages[k] = carpet[sel].mean(axis=0)
    assert _rms(averages[FLY_ROWS], dtl) < 0.02, f"running average over one z_T ≠ dtl_image ({_rms(averages[FLY_ROWS], dtl):.3f})"
    # the engine's stationary (infinite-scan) image is exactly p/2-periodic; the one-z_T scan average is within 2 %
    assert _rms(np.roll(stationary, len(stationary) // 4), stationary) < 1e-6, "the stationary image is not p/2-periodic"
    assert _rms(dtl, stationary) < 0.02, f"dtl_image deviates {_rms(dtl, stationary):.3f} rms from the stationary image"
    return {"carpet": carpet, "cz": cz, "cx": cx, "x": np.asarray(r.x_nm), "zt": zt, "zte": zte,
            "rows": np.array(rows), "averages": averages, "dtl": dtl, "exact_dev": exact_dev,
            "vmax": float(np.percentile(carpet, 99.5))}


def talbot_fly_draw(d, theme: S.Theme):
    import svgkit as K

    W, H = 960, 454
    cx0, cy0, cw, ch = 64, 72, 600, 300          # the carpet box: z →, x ↑ (14 px of air above it for the station labels)
    px0, pw = 724, 206                            # the live-profile panel, sharing the x axis
    s = K.Svg(W, H, theme.name, "Flying through a Talbot carpet", FLY_ALT, generator="docs/figures/sim/fig_deep.py (talbot-flythrough-anim)")
    zte, carpet = d["zte"], d["carpet"]
    z_max = 2 * zte
    xs = d["x"]
    x_min, x_max = float(d["cx"][0]), float(d["cx"][-1])

    def X(z):  # carpet z → svg x
        return cx0 + cw * z / z_max

    def Y(x):  # carpet x → svg y (x increases upward)
        return cy0 + ch * (1 - (x - x_min) / (x_max - x_min))

    # --- the carpet as an indexed PNG through the OKLab field ramp (rows = x from the top, columns = z)
    ramp = K.field_ramp(256)
    norm = np.clip(carpet / d["vmax"], 0, 1)
    idx = (norm.T[::-1] * 255).round().astype(np.uint8)   # (nx, nz): first row = largest x
    uri = "data:image/png;base64," + base64.b64encode(_palette_png(idx, ramp)).decode("ascii")
    s.add(f'<image x="{K.fmt(cx0)}" y="{K.fmt(cy0)}" width="{K.fmt(cw)}" height="{K.fmt(ch)}" '
          f'preserveAspectRatio="none" href="{uri}"/>')
    s.rect(cx0, cy0, cw, ch, fill="none", stroke="axis", sw=1.0)
    # --- axes and labels
    s.eyebrow(cx0, 24, f"Talbot carpet · {FLY_PERIOD:.0f} nm amplitude grating · {FLY_LAMBDA} nm · ±{FLY_ORDERS} orders, exact propagation")
    s.text(cx0, 44, f"I(x, z) behind the grating ({TALBOT_DUTY * 100:.0f} % open), y = 0; brightness = intensity", size=K.SMALL, anchor="start", color="ink2")
    ticks = [(0, "0"), (0.25, "z_T/4"), (0.5, "z_T/2"), (0.75, "3z_T/4"), (1.0, "z_T"), (1.25, "5z_T/4"), (1.5, "3z_T/2"), (1.75, "7z_T/4"), (2.0, "2z_T")]
    for f, label in ticks:
        xx = X(f * zte)
        s.line(xx, cy0 + ch, xx, cy0 + ch + 6, "axis", 1.0)
        s.text(xx, cy0 + ch + 22, label.replace("z_T", "zᵀ"), size=K.SMALL, color="ink2")
    s.text(cx0 + cw / 2, cy0 + ch + 44, f"distance behind the grating z (zᵀ = {zte:.0f} nm exact for the ±1 orders; 2p²/λ = {d['zt']:.0f} nm paraxial)", size=K.SMALL, color="ink2")
    for xv in (0, 50, 100, 150, 200):
        yy = Y(xv)
        s.line(cx0 - 6, yy, cx0, yy, "axis", 1.0)
        s.text(cx0 - 10, yy + 4.5, str(xv), size=K.SMALL, anchor="end", color="ink2")
    s.text(18, cy0 + ch / 2, "x (nm)", size=K.SMALL, color="ink2", rotate=-90)
    # the three stations on the carpet (shared by both layers)
    stations = ((0.25, "zᵀ/4 · period halved"), (0.5, "zᵀ/2 · shifted by p/2"), (1.0, "zᵀ · self-image"))
    for f, _ in stations:
        s.line(X(f * zte), cy0, X(f * zte), cy0 + ch, "surface", 1.2, dash="4 4", opacity=0.75)
    # --- the right panel frame
    s.eyebrow(px0, 24, "I(x) at the cursor")
    s.rect(px0, cy0, pw, ch, fill="none", stroke="axis", sw=1.0)
    i_max = float(d["rows"].max()) * 1.05
    for iv in (0.0, 1.0, 2.0):
        if iv <= i_max:
            xx = px0 + pw * iv / i_max
            s.line(xx, cy0 + ch, xx, cy0 + ch + 6, "axis", 1.0)
            s.text(xx, cy0 + ch + 22, f"{iv:.0f}", size=K.SMALL, color="ink2")
    s.text(px0 + pw / 2, cy0 + ch + 44, "intensity (clear field = 1)", size=K.SMALL, color="ink2")

    def profile(row, x0, w, y_top, h, lo, hi):
        pts = [(x0 + w * min(max(v, 0.0), hi) / hi, y_top + h * (1 - (xx - lo) / (x_max - x_min))) for xx, v in zip(xs, row)]
        return " ".join(f"{int(round(px))},{int(round(py))}" for px, py in pts)

    def polyline(row, cls, color, *, dash=None, sw=1.6):
        pts = profile(row, px0, pw, cy0, ch, x_min, i_max)
        extra = f' stroke-dasharray="{dash}"' if dash else ""
        s.add(f'<polyline class="{cls}" points="{pts}" fill="none" stroke="{s.c(color)}" stroke-width="{sw}" '
              f'stroke-linejoin="round"{extra}/>')

    # --- timeline: 10 s sweep at 60 px/s, 1 s pauses at the three stations, 2 s hold at the end
    V, PAUSE, HOLD = cw / 10.0, 1.0, 2.0
    pause_x = [X(f * zte) - cx0 for f, _ in stations]

    def t_of(xpx: float) -> float:
        return xpx / V + PAUSE * sum(1 for p in pause_x if xpx > p + 1e-9)

    T = t_of(cw) + HOLD
    anim = K.Anim(s, loops=3)
    kf = anim.keyframes("cursor", T)
    kf.at(0, "transform:translateX(0px)")
    for p in pause_x:
        t_in = t_of(p)
        kf.at(t_in, f"transform:translateX({K.fmt(p)}px)").at(t_in + PAUSE, f"transform:translateX({K.fmt(p)}px)")
    kf.at(t_of(cw), f"transform:translateX({K.fmt(cw)}px)").at(T, f"transform:translateX({K.fmt(cw)}px)")
    anim.animate(".cursor", "cursor", T)
    step = cw / FLY_ROWS
    fade = 0.12

    def window(k: int, kmin: int = 0) -> tuple[float, float]:
        lo = max(kmin * step - step / 2, (k - 0.5) * step)
        hi = min(cw, (k + 0.5) * step)
        t0 = 0.0 if k == kmin and kmin == 0 else t_of(max(lo, 0.0))
        t1 = T if k == FLY_ROWS else t_of(hi)
        return t0, t1

    def visible(name: str, t0: float, t1: float) -> None:
        k = anim.keyframes(name, T)
        k.at(0, "opacity:0")
        if t0 > 0:
            k.at(max(t0 - fade, 0), "opacity:0")
        k.at(t0, "opacity:1").at(t1, "opacity:1")
        if t1 < T:
            k.at(min(t1 + fade, T), "opacity:0").at(T, "opacity:0")
        anim.animate(f".{name}", name, T)

    k_avg0 = FLY_ROWS * 3 // 4
    with anim.layer("anim"):
        for k in range(FLY_ROWS + 1):
            t0, t1 = window(k)
            polyline(d["rows"][k], f"p{k}", "blue")
            visible(f"p{k}", t0, t1)
        for k in sorted(d["averages"]):
            t0, t1 = window(k)
            polyline(d["averages"][k], f"a{k}", "orange", sw=2.0)
            visible(f"a{k}", t0, t1)
        polyline(d["dtl"], "dtl", "ink2", dash="5 4", sw=1.3)
        t_avg = t_of(max(k_avg0 * step - step / 2, 0.0))
        visible("dtl", t_avg, T)
        s.text(px0 + pw - 4, cy0 + 16, "running gap average from zᵀ (orange)", size=K.SMALL, anchor="end", color="ink2", cls="dtl")
        s.text(px0 + pw - 4, cy0 + 32, "→ DTL image, period p/2 (dashed)", size=K.SMALL, anchor="end", color="ink2", cls="dtl")
        for i, (f, label) in enumerate(stations):
            t_in = t_of(pause_x[i])
            visible(f"st{i}", t_in, t_in + PAUSE)
            s.label_box(X(f * zte), cy0 - 12, label, size=K.SMALL, color="ink", fill="surface", stroke="axis", opacity=None)
            s.parts[-1] = s.parts[-1]  # (label_box emits two parts; class the pair below)
            s.parts[-2] = s.parts[-2].replace("<rect ", f'<rect class="st{i}" ', 1)
            s.parts[-1] = s.parts[-1].replace("<text ", f'<text class="st{i}" ', 1)
        s.line(cx0, cy0 - 4, cx0, cy0 + ch + 4, "brand:cyan300", 2.2, cls="cursor")
        s.add(f'<polygon class="cursor" points="{K.fmt(cx0 - 5)},{K.fmt(cy0 - 12)} {K.fmt(cx0 + 5)},{K.fmt(cy0 - 12)} {K.fmt(cx0)},{K.fmt(cy0 - 4)}" fill="{s.c("brand:cyan300")}"/>')
    # --- the designed still: the three stations' profiles stacked at right
    with anim.layer("still"):
        sub_h = (ch - 2 * 8) / 3
        for i, (f, label) in enumerate(stations):
            y_top = cy0 + i * (sub_h + 8)
            row = d["rows"][int(round(f * FLY_ROWS / 2))]
            pts = profile(row, px0, pw, y_top, sub_h, x_min, i_max)
            s.add(f'<polyline points="{pts}" fill="none" stroke="{s.c("blue")}" stroke-width="1.6" stroke-linejoin="round"/>')
            if i:
                s.line(px0, y_top - 4, px0 + pw, y_top - 4, "axis", 1.0)
            s.label_box(px0 + pw - 4, y_top + 12, label, size=K.SMALL, color="ink2", fill="surface", anchor="end", pad=3)
        for f, label in stations:
            s.label_box(X(f * zte), cy0 - 12, label.split(" · ")[0], size=K.SMALL, color="ink", fill="surface", stroke="axis")
    s.style(".still text,.anim text{font-family:inherit}")
    anim.install()
    return s.render()


FLY_ALT = ("Animation: a cursor sweeps through the Talbot carpet behind a 100 nm grating (25 percent open) at 13.5 nm, "
           "its intensity profile drawn at right; it pauses where the period halves, where the image shifts by half a "
           "period and at the Talbot length, then a gap average becomes the half-period DTL image.")

register(FigureSpec(
    name="talbot-flythrough-anim",
    group="deep",
    compute=talbot_fly_compute,
    draw=talbot_fly_draw,
    alt=FLY_ALT,
    caption="<strong>Flying through a Talbot carpet.</strong> Intensity behind a 100 nm binary amplitude grating, "
            "25 % open, at 13.5 nm (±10 orders, exact angular-spectrum propagation; 128 × 240 engine samples); the "
            "cursor's row is drawn at right and pauses at z<sub>T</sub>/4, z<sub>T</sub>/2 and z<sub>T</sub>, where "
            "the paraxial carpet halves its period (a 50 % open grating would be uniform there), shifts by p/2 and "
            "re-images the grating (z<sub>T</sub> = 1475 nm exact for the ±1 orders, 1481 nm paraxial) — at "
            "λ/p = 0.135 the higher orders are non-paraxial, so the exact revivals are approximate; past "
            "3z<sub>T</sub>/2 the running average over the scanned gap converges to the displacement-Talbot image "
            "with half the period. <em>Model: Talbot / DTL / ATL 🔶 "
            "(scalar thin mask, coherent normal illumination, wavelength-independent grating coefficients); "
            "computed, not measured.</em>",
    pages=("processes/talbot.md", "future/alternative-patterning.md"),
    subdir="anim",
    fmt="svg",
    extra={"model_rows": {"Talbot / DTL / ATL": "Talbot / DTL / ATL proximity lithography"}},
))


# ---------------------------------------------------------------------------
# 13. P8c · talbot-atl-euvil: achromatic Talbot convergence and two-grating EUV-IL fringes
# ---------------------------------------------------------------------------

ATL_BAND_NM = 0.3


def talbot_atl_compute(fast: bool):
    huv = native()
    kw = dict(grating="amplitude", max_order=FLY_ORDERS, nx=128, carpet_nz=4)
    probe = huv.simulate_talbot(FLY_LAMBDA, FLY_PERIOD, gap_nm=1.0, bandwidth_nm=ATL_BAND_NM, spectrum="gaussian", **kw)
    z_a = probe.achromatic_distance_nm
    assert z_a is not None and abs(z_a - 2 * FLY_PERIOD ** 2 / ATL_BAND_NM) < 1e-6, "z_A drifted from 2p²/Δλ"
    gaps = np.linspace(0.0, 3.0 * z_a, 46 if fast else 121)
    atl_c, coh_c, dev = [], [], []
    rd = huv.simulate_talbot(FLY_LAMBDA, FLY_PERIOD, gap_nm=probe.talbot_length_exact_nm,
                             scan_length_nm=probe.talbot_length_exact_nm, **kw)
    stationary = np.asarray(rd.stationary_image)[0]
    for g in gaps:
        r = huv.simulate_talbot(FLY_LAMBDA, FLY_PERIOD, gap_nm=float(g), bandwidth_nm=ATL_BAND_NM, spectrum="gaussian",
                                spectral_bins=128 if fast else 256, **kw)
        a, c = np.asarray(r.atl_image)[0], np.asarray(r.coherent_image)[0]
        atl_c.append(_contrast(a))
        coh_c.append(_contrast(c))
        dev.append(_rms(a, stationary))
    atl_c, coh_c, dev = np.array(atl_c), np.array(coh_c), np.array(dev)
    stat_c = _contrast(stationary)
    # beyond a few achromatic distances the spectral average is the stationary (DTL) image
    assert dev[gaps >= 2.5 * z_a].max() < 0.02, f"ATL image at ≥ 2.5 z_A still deviates {dev[gaps >= 2.5 * z_a].max():.3f} rms from the stationary image"
    assert abs(atl_c[-1] - stat_c) < 0.01, "ATL contrast at 3 z_A ≠ stationary contrast"
    # two-grating EUV-IL: fringe period p/(2m), wavelength-independent
    il = {}
    for lam, m in ((13.5, 1), (13.5, 2), (6.7, 1)):
        e = huv.simulate_euv_il(FLY_PERIOD, lam, order=m, nx=256)
        assert abs(e.fringe_period_nm - FLY_PERIOD / (2 * m)) < 1e-9, f"EUV-IL period at order {m} ≠ p/2m"
        il[(lam, m)] = {"x": np.asarray(e.intensity.x_nm), "I": np.asarray(e.intensity.values)[0, 0, :],
                        "period": e.fringe_period_nm, "visibility": e.visibility, "angle": e.diffraction_angle_deg}
    assert il[(13.5, 1)]["period"] == il[(6.7, 1)]["period"], "the EUV-IL period depends on wavelength"
    return {"gaps": gaps, "z_a": z_a, "atl": atl_c, "coh": coh_c, "dev": dev, "stat_c": stat_c,
            "zte": probe.talbot_length_exact_nm, "il": il}


def talbot_atl_draw(d, th: S.Theme):
    fig, (ax, ax2) = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 3.9), gridspec_kw={"wspace": 0.3, "width_ratios": [1.15, 1]})
    S.style_axes(ax, th, grid="both")
    g = d["gaps"] / d["z_a"]
    ax.plot(g, d["coh"], color=th.muted, linewidth=S.HAIR_PT * 1.3, zorder=2)
    ax.plot(g, d["atl"], color=th.s(1), zorder=3)
    S.limit_line(ax, d["stat_c"], th, f"stationary DTL image, contrast {d['stat_c']:.2f}", axis="y", where=0.99,
                 side="left", va="bottom")
    S.limit_line(ax, 1.0, th, "z_A = 2p²/Δλ".replace("z_A", "z$_A$"), axis="x", where=0.55, side="right", va="top")
    ax.set_xlim(0, 3)
    ax.set_ylim(0.3, 1.05)
    ax.set_xlabel(f"gap / achromatic distance ({d['z_a'] / 1e3:.1f} µm)")
    ax.set_ylabel("image contrast")
    ax.set_title(f"(a) achromatic Talbot: {ATL_BAND_NM} nm FWHM Gaussian band")
    ax.legend([Line2D([], [], color=th.s(1)), Line2D([], [], color=th.muted, linewidth=S.HAIR_PT * 1.3)],
              ["spectrally averaged (ATL) image", "monochromatic carpet row"], loc="center right")
    S.style_axes(ax2, th, grid="both")
    styles = {(13.5, 1): (th.s(1), "-", 1.0), (13.5, 2): (th.s(2), "-", 1.0), (6.7, 1): (th.s(3), (0, (4, 3)), 0.8)}
    labels = []
    for key, e in d["il"].items():
        col, ls, wf = styles[key]
        ax2.plot(e["x"], e["I"] / e["I"].max(), color=col, linestyle=ls, linewidth=S.LINE_PT * wf, zorder=3)
        labels.append(f"λ {key[0]} nm, order {key[1]}: Λ = {e['period']:.0f} nm, V = {e['visibility']:.2f}")
    ax2.set_xlim(0, 200)
    ax2.set_ylim(0, 1.3)
    ax2.set_xlabel("x (nm)")
    ax2.set_ylabel("intensity (normalised)")
    ax2.set_title("(b) two-grating EUV-IL fringes, p = 100 nm")
    ax2.legend([Line2D([], [], color=styles[k][0], linestyle=styles[k][1]) for k in d["il"]], labels,
               loc="upper center", ncol=1, fontsize=S.SMALL_PT - 0.5)
    return fig


register(FigureSpec(
    name="talbot-atl-euvil",
    group="deep",
    compute=talbot_atl_compute,
    draw=talbot_atl_draw,
    alt="Left: image contrast against the mask–wafer gap for a 0.3 nm band, settling on the stationary DTL value "
        "beyond the achromatic distance; right: two-grating EUV-IL fringes of period 50 and 25 nm from the first "
        "and second orders at 13.5 nm, the 6.7 nm first-order fringes coinciding with the 50 nm ones.",
    caption="<strong>Washing out the revivals, and fringes that ignore the wavelength.</strong> (a) Contrast of the "
            "spectrally averaged image behind a 100 nm grating for a 0.3 nm FWHM Gaussian band: beyond the "
            "achromatic distance z<sub>A</sub> = 2p²/Δλ it settles on the stationary DTL image, which the scanned "
            "gap reaches by averaging instead; (b) two-grating EUV-IL fringes from the ±1 and ±2 orders have "
            "periods p/2 and p/4 at 13.5 nm, and the ±1 fringes at 6.7 nm have the same 50 nm period. "
            "<em>Model: Talbot / DTL / ATL 🔶 · two-grating EUV-IL 🔶 (ideal gratings, no zero-order background); "
            "computed, not measured.</em>",
    pages=("processes/talbot.md",),
    extra={"model_rows": {"Talbot / DTL / ATL": "Talbot / DTL / ATL proximity lithography",
                          "two-grating EUV-IL": "Two-grating EUV interference lithography"}},
))


# ---------------------------------------------------------------------------
# 14. P9b · liga-hardening: harder beam, flatter dose, longer exposure
# ---------------------------------------------------------------------------

HARDENING_AL_UM = (0.0, 20.0, 50.0)


def liga_hardening_compute(fast: bool):
    huv = native()
    bm = huv.SourceConfig.synchrotron_liga_bending_magnet()
    rows = []
    for t in HARDENING_AL_UM:
        r = huv.simulate_liga(resist_thickness_um=500.0, filters=([("Al", t)] if t else None), absorber="Au",
                              absorber_thickness_um=20.0, membrane="Ti", membrane_thickness_um=2.0,
                              target_bottom_dose_kj_cm3=3.0, source=bm, source_distance_m=15.0, vertical_scan_mm=50.0,
                              grid_size=32, pixel_nm=200.0, nz=16, diffraction="gaussian")
        z, dose = r.depth_dose
        rows.append({"al": t, "z": np.asarray(z), "dose": np.asarray(dose), "top": r.top_dose_kj_cm3,
                     "ratio": r.dose_ratio, "time": r.exposure_time_s, "e_top": r.mean_energy_top_kev,
                     "e_bot": r.mean_energy_bottom_kev, "ceiling": r.exceeds_damage_ceiling})
    # the soft photons go first: mean photon energy rises with depth and with filter thickness; so does the time
    for r in rows:
        assert r["e_bot"] > r["e_top"], "mean photon energy must rise with depth"
        assert abs(r["dose"][-1] - 3.0) < 0.05, "bottom dose is not the 3 kJ/cm³ target"
    tops = [r["e_top"] for r in rows]
    bots = [r["e_bot"] for r in rows]
    times = [r["time"] for r in rows]
    assert tops == sorted(tops) and bots == sorted(bots), "mean energies are not monotone with filter thickness"
    assert times == sorted(times), "exposure time must grow with filter thickness"
    assert rows[0]["ceiling"] and not rows[-1]["ceiling"], "the unfiltered beam exceeds the ceiling; 50 µm Al must not"
    return rows


def liga_hardening_draw(rows, th: S.Theme):
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 4.3))
    S.style_axes(ax, th, grid="both")
    ax.set_yscale("log")
    cols = S.ordinal(th, len(rows))
    labels = []
    for col, r in zip(cols, rows):
        ax.plot(r["z"], r["dose"], color=col, zorder=3)
        S.dot(ax, [r["z"][0]], [r["dose"][0]], col, th)
        labels.append(f"Al {r['al']:.0f} µm: top {r['top']:.1f} kJ/cm³ (×{r['ratio']:.1f}), {r['time']:.0f} s, "
                      f"⟨E⟩ {r['e_top']:.1f} → {r['e_bot']:.1f} keV")
    S.limit_line(ax, 20.0, th, "damage ceiling 20 kJ/cm³", axis="y", where=0.99, side="left", va="bottom")
    S.limit_line(ax, 3.0, th, "bottom target 3 kJ/cm³", axis="y", where=0.99, side="left", va="top")
    ax.set_xlim(0, 500)
    ax.set_ylim(2, 100)
    ax.set_yticks([2, 3, 5, 10, 20, 50, 100])
    ax.set_yticklabels(["2", "3", "5", "10", "20", "50", "100"])
    ax.set_xlabel("Depth in PMMA (µm)")
    ax.set_ylabel("Absorbed dose (kJ/cm³)")
    ax.set_title("Beam hardening by aluminium filters: 500 µm PMMA, 20 µm Au on 2 µm Ti, bending magnet at 200 mA")
    leg = ax.legend([Line2D([], [], color=c) for c in cols], labels, loc="upper right", fontsize=S.SMALL_PT,
                    title="top dose · exposure time · mean photon energy top → bottom", title_fontsize=S.SMALL_PT)
    leg.get_title().set_color(th.ink2)
    return fig


register(FigureSpec(
    name="liga-hardening",
    group="deep",
    compute=liga_hardening_compute,
    draw=liga_hardening_draw,
    alt="Depth-dose curves through 500 micrometres of PMMA for 0, 20 and 50 micrometres of aluminium filter, "
        "each scaled to 3 kJ/cm3 at the bottom: thicker filters flatten the curve below the 20 kJ/cm3 ceiling "
        "while the exposure time and the mean photon energy rise.",
    caption="<strong>Harder beam, flatter dose, longer exposure.</strong> The soft photons are absorbed first, so "
            "the mean photon energy rises with depth and with filtering; the filter buys a top dose under the "
            "ceiling at the price of exposure time (every curve scaled so the bottom receives 3 kJ/cm³; times "
            "absolute from the bending-magnet flux at 200 mA, 15 m, 50 mm scan). <em>Model: LIGA depth dose and "
            "exposure time ✅ (NIST μ/μ<sub>en</sub>, scan-averaged collimated beam, no beamline mirrors).</em>",
    pages=("processes/liga-deep-xray.md",),
    extra={"model_rows": {"LIGA depth dose and exposure time": "LIGA / deep X-ray depth dose and exposure time"}},
))


# ---------------------------------------------------------------------------
# 15. P12 · config-volumetric-dose-window: the documented default stack against dose (+ the dose table)
# ---------------------------------------------------------------------------

DOSE_TABLE_ROWS = (6.0, 10.0, 12.0, 15.0, 30.0)
DOSE_DEV_T, DOSE_NZ, DOSE_GRID = 60.0, 16, (64, 9.375)


def _dose_run(api, huv, src, optics, mask, grid, res, dose: float, dev_t: float = DOSE_DEV_T):
    p = api.simulate_volumetric(src, optics, mask, huv.FilmStackConfig(), res, grid, dose_mj_cm2=float(dose),
                                nz=DOSE_NZ, peb="gaussian", develop="fmm", dev_time_s=dev_t)
    cd = p.developed_cd_nm
    h = np.asarray(p.height_map.values)
    return {"dose": float(dose), "mid": cd[len(cd) // 2], "bottom": cd[-1], "mean_h": float(h.mean()),
            "min_h": float(h.min()), "max_h": float(h.max())}


def dose_window_compute(fast: bool):
    huv = native()
    import highuvlith.api as api  # noqa: PLC0415

    src, optics, mask = _scene(huv)
    res = huv.ResistConfig()
    grid = huv.GridConfig(size=DOSE_GRID[0], pixel_nm=DOSE_GRID[1])
    doses = sorted(set(np.round(np.arange(4.0, 40.01, 1.0 if fast else 0.5), 3).tolist()) | set(DOSE_TABLE_ROWS))
    sweep = [_dose_run(api, huv, src, optics, mask, grid, res, d) for d in doses]
    # the open-frame dose-to-clear: bisect the dose at which an unpatterned field clears to the substrate
    open_mask = huv.MaskConfig.from_features([], dark_field=False)
    small = huv.GridConfig(size=16, pixel_nm=DOSE_GRID[1])

    def clears(dose: float, dev_t: float) -> bool:
        p = api.simulate_volumetric(src, optics, open_mask, huv.FilmStackConfig(), res, small, dose_mj_cm2=dose,
                                    nz=DOSE_NZ, peb="gaussian", develop="fmm", dev_time_s=dev_t)
        return float(np.asarray(p.height_map.values).max()) <= 1e-9

    def dose_to_clear(dev_t: float) -> float:
        lo, hi = 1.0, 40.0
        for _ in range(24):
            mid = 0.5 * (lo + hi)
            lo, hi = (lo, mid) if clears(mid, dev_t) else (mid, hi)
        return hi

    d2c = {60.0: dose_to_clear(60.0), 30.0: dose_to_clear(30.0)}
    assert d2c[30.0] > d2c[60.0], "a shorter development must need a higher dose to clear"
    # the window: below the dose-to-clear nothing clears; above it the CD shrinks and the height falls monotonically
    heights = [r["mean_h"] for r in sweep]
    assert all(a >= b - 1e-6 for a, b in zip(heights, heights[1:])), "mean remaining height is not monotone in dose"
    cleared = [r for r in sweep if r["mid"] is not None]
    assert cleared and all(r["dose"] > d2c[60.0] for r in cleared), "a space cleared below the open-frame dose-to-clear"
    mids = [r["mid"] for r in cleared]
    assert all(a >= b - 1e-6 for a, b in zip(mids, mids[1:])), "developed mid-height CD is not monotone in dose"
    return {"sweep": sweep, "d2c": d2c, "table_doses": DOSE_TABLE_ROWS, "fallback": 30.0, "thickness": res.thickness_nm}


def _fmt_cd(v) -> str:
    return "n/a" if v is None else f"{v:.0f}"


def dose_window_table(d) -> str:
    """The column-0 GFM table configuration.md carries between the ``dose-table`` markers."""
    by_dose = {r["dose"]: r for r in d["sweep"]}
    lines = ["| Dose (mJ/cm²) | Developed CD mid / bottom (nm) | Mean remaining height (nm) |", "|---|---|---|"]
    for dose in d["table_doses"]:
        r = by_dose[dose]
        label = f"{dose:.0f} (fallback)" if dose == d["fallback"] else f"{dose:.0f}"
        cd = f"{_fmt_cd(r['mid'])} / {_fmt_cd(r['bottom'])}" + (" (spaces do not clear)" if r["mid"] is None else "")
        lines.append(f"| {label} | {cd} | {r['mean_h']:.0f} |")
    return "\n".join(lines)


def dose_window_draw(d, th: S.Theme):
    fig, (ax, ax2) = plt.subplots(1, 2, figsize=(S.FULL_WIDTH + 0.6, 3.7), gridspec_kw={"wspace": 0.3})
    doses = np.array([r["dose"] for r in d["sweep"]])
    mid = np.array([np.nan if r["mid"] is None else r["mid"] for r in d["sweep"]])
    bot = np.array([np.nan if r["bottom"] is None else r["bottom"] for r in d["sweep"]])
    mean_h = np.array([r["mean_h"] for r in d["sweep"]])
    lo_h = np.array([r["min_h"] for r in d["sweep"]])
    hi_h = np.array([r["max_h"] for r in d["sweep"]])
    d2c = d["d2c"][DOSE_DEV_T]
    for a in (ax, ax2):
        S.style_axes(a, th, grid="both")
        a.axvspan(doses[0], d2c, color=th.band, lw=0, zorder=0)
        S.limit_line(a, d2c, th, f"open-frame dose-to-clear {d2c:.1f} mJ/cm² ({DOSE_DEV_T:.0f} s)", axis="x", where=0.97,
                     side="right", va="top")
        S.limit_line(a, d["fallback"], th, f"{d['fallback']:.0f} mJ/cm² fallback", axis="x", where=0.40, side="right", va="bottom")
        a.set_xlim(doses[0], doses[-1])
        a.set_xlabel("dose (mJ/cm²)")
    ax.plot(doses, mid, color=th.s(1), zorder=3)
    ax.plot(doses, bot, color=th.s(2), zorder=3)
    for dose in d["table_doses"]:
        r = next(r for r in d["sweep"] if r["dose"] == dose)
        if r["mid"] is not None:
            S.dot(ax, [dose], [r["mid"]], th.s(1), th, size_px=7)
            S.dot(ax, [dose], [r["bottom"]], th.s(2), th, size_px=7)
    ax.set_ylim(0, 300)
    # cd_at_z measures the feature straddling the field centre, which for the 150/300 nm line/space mask is the
    # resist line (metrics.rs measure_cd: "whichever tone that feature has"): the remaining resist, not the space
    ax.set_ylabel("developed line CD, remaining resist (nm)")
    ax.set_title("(a) developed line CD at mid-height and at the substrate")
    ax.text(0.02, 0.04, "spaces do not clear", transform=ax.transAxes, color=th.ink2, fontsize=S.SMALL_PT - 1)
    ax.legend([Line2D([], [], color=th.s(1)), Line2D([], [], color=th.s(2))], ["mid-height", "bottom (substrate)"],
              loc="upper right")
    ax2.fill_between(doses, lo_h, hi_h, color=th.s(1), alpha=0.15, lw=0, zorder=2)
    ax2.plot(doses, mean_h, color=th.s(1), zorder=3)
    for dose in d["table_doses"]:
        r = next(r for r in d["sweep"] if r["dose"] == dose)
        S.dot(ax2, [dose], [r["mean_h"]], th.s(1), th, size_px=7)
    ax2.set_ylim(0, d["thickness"] * 1.04)
    ax2.set_ylabel("resist remaining (nm)")
    ax2.set_title("(b) mean remaining height (band: thinnest to thickest column)")
    ax2.legend([Line2D([], [], color=th.s(1)), Line2D([], [], marker="o", linestyle="none", color=th.s(1), markersize=5)],
               ["mean over the field", "the table's doses"], loc="upper right")
    fig.subplots_adjust(bottom=0.17, top=0.88, left=0.08, right=0.98)
    return fig


register(FigureSpec(
    name="config-volumetric-dose-window",
    group="deep",
    compute=dose_window_compute,
    draw=dose_window_draw,
    alt="Developed resist-line width at mid-height and at the substrate, and the mean remaining resist height, "
        "against dose for the default volumetric stack; the spaces clear just above the open-frame dose-to-clear, "
        "the line then narrows with dose and the 30 mJ/cm2 fallback leaves under 10 nm of resist.",
    caption="<strong>Why no single volumetric dose fits.</strong> The documented default stack (F<sub>2</sub> "
            "157.63 nm, NA 0.75, 150/300 nm lines, 150 nm resist on Si, Gaussian bake 30 nm, fast-marching "
            "development for 60 s, 64 × 9.375 nm grid, nz 16) against dose: the spaces clear only above the "
            "open-frame dose-to-clear, the resist line then narrows with dose and the 30 mJ/cm² fallback leaves "
            "under 10 nm of resist; the dots are the rows of the dose table. <em>Model: volumetric exposure 🔶 · "
            "post-exposure bake ✅ · 3D development (fast marching) ✅; illustrative low-contrast resist; computed, "
            "not transcribed.</em>",
    pages=("configuration.md", "processes/volumetric-exposure.md"),
    extra={"model_rows": {"volumetric exposure": "Volumetric (z-resolved) exposure",
                          "post-exposure bake": "Post-exposure bake (anisotropic Gaussian)"},
           "table": {"page": "configuration.md", "marker": "dose-table", "render": dose_window_table}},
))
