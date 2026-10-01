"""Deep-lithography figures: LIGA, multilayer mirrors, development, CAR bake, Talbot, interference, grayscale."""

from __future__ import annotations

import numpy as np
from matplotlib import pyplot as plt
from matplotlib.lines import Line2D

import style as S
from common import FigureSpec, native, register

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
    caption="LIGA deep X-ray exposure of 500 µm PMMA through 20 µm Au on 2 µm Ti, every curve scaled so the "
            "bottom receives 3 kJ/cm³. A harder (filtered) beam flattens the depth dose and keeps the top below "
            "the 20 kJ/cm³ damage ceiling at the cost of exposure time; times are absolute, from the bending-magnet "
            "flux at 200 mA. Model: LIGA depth dose with NIST μ / μ<sub>en</sub> and the bending-magnet "
            "spectrum per log-energy bin ✅ (scan-averaged, collimated beam, no beamline mirrors).",
    pages=("processes/liga-deep-xray.md",),
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
    return {
        "x": np.asarray(fres.x_nm), "fresnel": np.asarray(fres.dose_kj_cm3)[0] / float(np.asarray(fres.open_dose_kj_cm3)[0]),
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
    ax2.set_title(f"(b) Bending-magnet beam: edge through 500 µm PMMA")
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
    alt="Left: dose across an absorber edge at 8 keV, where Fresnel diffraction gives the textbook 25 percent "
        "value at the edge and ringing up to 1.37 on the open side, unlike the smooth legacy Gaussian blur. "
        "Right: dose profiles across the edge at five depths for the polychromatic beam, with the developed edge "
        "moving by about 300 nm over 500 micrometres.",
    caption="LIGA proximity diffraction. (a) Monochromatic 8 keV straight edge, 100 µm gap: the default Fresnel "
            "(angular-spectrum) model gives the knife-edge pattern — 0.25 at the geometric edge, a 1.37 maximum "
            "and ringing — while the legacy <code>gaussian</code> option smooths the step with σ = ½√(λg). Both "
            "curves are simulator output. (b) Polychromatic bending-magnet beam (no filter): absorbed dose across "
            "the edge at five depths and the developed edge at a 2.5 kJ/cm³ threshold. Model: LIGA proximity "
            "diffraction ✅/🔶 (thin-screen absorber; no photo-/secondary-electron transport).",
    pages=("processes/liga-deep-xray.md", "README.md", "history/origins.md"),
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
    caption="EUV/BEUV multilayer mirrors from Parratt recursion with CXRO/Henke optical constants. (a) Mo/Si "
            "40 × 6.9 nm (Mo fraction 0.4, ideal interfaces) against the best reported measured reflectance. "
            "(b) La/B<sub>4</sub>C and La/B, periods tuned to 6.7 / 6.65 nm: the ideal model sits well above the "
            "64.1 % La/B record (Opt. Lett. 40, 3778, 2015) because interdiffusion and roughness are off. (c) Mo/Si "
            "at 13.5 nm vs angle of incidence, s and p. Model: multilayer mirrors ✅ (matches the CXRO calculator "
            "within 3·10⁻³; no interdiffusion layers by default).",
    pages=("materials.md", "future/beyond-euv.md"),
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
    caption="3D development of an exposed trench (PAC m = 1 − 0.9·exp(−x²/2·20²), uniform in depth; default "
            "Mack resist). The moving-boundary level set (thin line) and the static-rate fast-marching arrival "
            "time (thick line) agree to within a grid cell after 3 s; surface inhibition (top-surface rate × 0.1, "
            "10 nm decay) necks the opening just below the surface — the T-top. Models: fast marching ✅, level "
            "set ✅/🔶 (first order; phenomenological inhibition constants).",
    pages=("processes/volumetric-exposure.md",),
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
    src = huv.SourceConfig.f2_laser(0.7)
    optics = huv.OpticsConfig(numerical_aperture=0.75)
    mask = huv.MaskConfig.line_space(150.0, 300.0)
    grid = mask.commensurate_grid(64 if fast else 128, 4.6875)
    lat = huv.expose_volumetric(src, optics, mask, huv.FilmStackConfig(), huv.ResistConfig(), grid,
                                dose_mj_cm2=40.0, nz=32 if fast else 64, n_defocus_planes=4, dose_steps=5)
    car = huv.peb_car(lat)
    acid0 = 1 - np.asarray(lat.values)
    dep = 1 - np.asarray(car.protected.values)
    mid = acid0.shape[1] // 2
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
    caption="Chemically amplified resist (CAR) post-exposure bake: acid/quencher reaction–diffusion with "
            "deprotection, default rates (60 s, k<sub>amp</sub> 0.1 s⁻¹, D<sub>acid</sub> 2 nm²/s). (a) A fixed "
            "1-D acid profile baked with more quencher deprotects less and with a steeper edge. (b, c) A real "
            "volumetric latent image — F<sub>2</sub> 157.63 nm, NA 0.75, 150 nm lines on a 300 nm pitch, 150 nm "
            "resist on silicon, so standing waves band the acid — before and after the bake. Model: CAR PEB 🔶 "
            "(standard Mack/PROLITH-class equations; illustrative constants, no acid loss).",
    pages=("processes/resist-models.md",),
))


# ---------------------------------------------------------------------------
# 6. Talbot carpet and the DTL image
# ---------------------------------------------------------------------------


def talbot_compute(fast: bool):
    huv = native()
    r = huv.simulate_talbot(13.5, 100.0, grating="amplitude", max_order=10, carpet_nz=128 if fast else 512,
                            nx=128 if fast else 256)
    return {"carpet": np.asarray(r.carpet), "x": np.asarray(r.carpet_x_nm), "z": np.asarray(r.carpet_z_nm),
            "zt": r.talbot_length_nm, "zt_exact": r.talbot_length_exact_nm, "coh": np.asarray(r.coherent_image),
            "dtl": np.asarray(r.stationary_image), "xi": np.asarray(r.x_nm)}


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
    ax.set_title("(a) Talbot carpet, 100 nm amplitude grating at 13.5 nm")
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
    alt="A Talbot carpet: the intensity behind a 100 nm grating lit at 13.5 nm repeats itself at the Talbot "
        "length and shows shifted and frequency-doubled images in between; beside it, the displacement Talbot "
        "image averaged over the gap has half the grating period.",
    caption="Talbot self-imaging at 13.5 nm. (a) Intensity behind a 100 nm-period binary amplitude grating "
            "(±10 orders, exact angular-spectrum propagation): the grating re-images at the Talbot length "
            "(2p²/λ ≈ 1481 nm paraxially, 1475 nm exactly) and shows the half-shifted and doubled images in "
            "between. (b) Averaging over a scanned gap (displacement Talbot lithography) gives a stationary "
            "image with half the period. Model: Talbot / DTL 🔶 (scalar thin mask, coherent normal illumination).",
    pages=("processes/talbot.md", "README.md"),
))


# ---------------------------------------------------------------------------
# 7. Multi-beam interference lattices
# ---------------------------------------------------------------------------


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
    return out


UMBRELLA_DEG = 60.0


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
    caption="Multi-beam interference (holographic) lithography: coherent 200 nm plane waves recorded in a resist "
            "of index 1.6 as a Dill latent image (exposed fraction shown). Two beams write lines and three a "
            "hexagonal lattice (30° air-side half-angle, top slice); four beams in an umbrella (60°) write a 3D "
            "lattice, here cut in x–z. Model: multi-beam "
            "interference ✅/🔶 (analytic vector-sum field; per-beam scalar absorption; single-exposure Dill "
            "kinetics).",
    pages=("processes/interference-volumetric.md",),
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
    src = huv.SourceConfig(wavelength_nm=157.63, sigma_outer=0.7)
    hm = huv.grayscale_height_map(src, huv.OpticsConfig(numerical_aperture=0.75), huv.GridConfig(n, pixel), trans,
                                  dose, d_th, d_clear, thickness, 20)
    return {"target": target, "trans": trans, "printed": np.asarray(hm.values), "x": np.asarray(hm.x_nm),
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
    caption="Grayscale lithography: a 2 × 2 microlens array (300 nm sag in 500 nm resist) designed through the "
            "log-linear contrast curve (threshold 20, clearing 80 mJ/cm², 100 mJ/cm² exposure), imaged with F<sub>2"
            "</sub> 157.63 nm, NA 0.75, σ 0.7 and mapped back to remaining height. The optics band-limit the relief, "
            "rounding the cusps between lenses. Model: grayscale 🔶 (thin-film contrast-curve resist, no "
            "development dynamics).",
    pages=("processes/grayscale.md",),
))
