"""Thin-film, resist, grayscale and two-photon engine figures (DESIGN §5.3 P4a, P5a, P6a, P7b; group ``deep_b``).

Every number drawn on these figures is computed by ``highuvlith._native`` in ``compute()``;
the closed forms (the Mack rate law, the contrast curve, the Gaussian focus) are plotted
beside the engine's own evaluation of the same quantity and asserted equal there, so
``make_all.py --verify`` proves the figure and the module agree without drawing anything.
"""

from __future__ import annotations

import numpy as np
from matplotlib import pyplot as plt
from matplotlib.lines import Line2D

import style as S
from common import FigureSpec, native, register

F2_NM = 157.63

# ---------------------------------------------------------------------------
# P4a. Thin film: standing wave, swing curve, Brewster zero, X-ray total external reflection
# ---------------------------------------------------------------------------

ARC = ("arc", 60.0, 1.8, 0.3)          # the interface-continuity test fixture's ARC, not a product BARC
SI_157 = (0.88, 2.10)                  # Si at 157 nm as FilmStack::default() carries it
RESIST_157 = (1.65, 0.015)
GLASS_N = 1.5                          # the lossless substrate of test_bare_substrate_reflectance / test_brewster_angle
AU_KEV = 8.0
#: DF-recorded values (processes/thin-film.md example; matrix audit row): the figure annotates the live ones.
R_DEFAULT_DOC, R_ARC_DOC = 0.2747, 0.0423
TER_DOC = {4.0: 0.9116, 9.77: 0.4156, 20.0: 0.0047}   # thinfilm.rs::test_xray_total_external_reflection


def _local_minima(z, p):
    idx = [i for i in range(1, len(p) - 1) if p[i] < p[i - 1] and p[i] <= p[i + 1]]
    return np.asarray([z[i] for i in idx])


def thinfilm_compute(fast: bool):
    huv = native()
    z = np.linspace(0.0, 150.0, 301 if fast else 1501)
    bare = huv.FilmStackConfig()
    arc = huv.FilmStackConfig()
    arc.add_layer(*ARC)
    arc.set_substrate(*SI_157)
    e_bare = np.asarray(bare.intensity_profile(F2_NM, list(z)))
    e_arc = np.asarray(arc.intensity_profile(F2_NM, list(z)))
    r_default = float(bare.reflectance(F2_NM))
    r_arc = float(arc.reflectance(F2_NM))
    minima = _local_minima(z, e_bare)
    period_model = F2_NM / (2 * RESIST_157[0])
    period = float(np.mean(np.diff(minima))) if len(minima) > 1 else float("nan")
    # (b) swing curve: a single resist layer of varying thickness on Si
    d = np.arange(50.0, 400.0 + 1e-9, 2.0 if fast else 0.5)
    swing = np.asarray([huv.FilmStackConfig(layers=[("resist", float(t), *RESIST_157)], substrate_n=SI_157[0],
                                            substrate_k=SI_157[1]).reflectance(F2_NM) for t in d])
    # (c) Brewster: TE / TM reflectance of a lossless n = 1.5 substrate against incidence
    glass = huv.FilmStackConfig(layers=[], substrate_n=GLASS_N, substrate_k=0.0)
    ang = np.linspace(0.0, 89.5, 180 if fast else 360)
    r_te = np.asarray([glass.reflectance(F2_NM, float(a), "te") for a in ang])
    r_tm = np.asarray([glass.reflectance(F2_NM, float(a), "tm") for a in ang])
    brewster = float(np.degrees(np.arctan(GLASS_N)))
    # (d) Au at 8 keV: total external reflection below the critical grazing angle sqrt(2 delta) (Henke constants)
    lam_au = 1.23984193 / AU_KEV
    delta, beta = huv.xray_optical_constants("Au", [lam_au])
    delta, beta = float(np.asarray(delta)[0]), float(np.asarray(beta)[0])
    au = huv.FilmStackConfig(layers=[], substrate_n=1.0 - delta, substrate_k=beta)
    mrad = np.linspace(0.5, 30.0, 120 if fast else 300)
    ter = np.asarray([au.reflectance(lam_au, 90.0 - np.degrees(g * 1e-3), "te") for g in mrad])
    ter_marks = {g: float(au.reflectance(lam_au, 90.0 - np.degrees(g * 1e-3), "te")) for g in TER_DOC}
    crit_mrad = float(np.sqrt(2 * delta) * 1e3)
    # --- physical asserts (run by make_all.py --verify) ---
    assert abs(r_default - R_DEFAULT_DOC) < 5e-4, f"default stack R {r_default:.4f} != documented {R_DEFAULT_DOC}"
    assert abs(r_arc - R_ARC_DOC) < 5e-4, f"ARC stack R {r_arc:.4f} != documented {R_ARC_DOC}"
    assert abs(period - period_model) / period_model < 0.01, f"standing-wave period {period:.2f} vs λ/2n {period_model:.2f}"
    assert abs(float(ang[int(np.argmin(r_tm))]) - brewster) < 1.0, "TM minimum is not at Brewster's angle"
    for g, want in TER_DOC.items():
        assert abs(ter_marks[g] - want) < 2e-3, f"TER at {g} mrad: {ter_marks[g]:.4f} vs {want}"
    return {
        "z": z, "e_bare": e_bare, "e_arc": e_arc, "minima": minima, "period": period, "period_model": period_model,
        "r_default": r_default, "r_arc": r_arc, "d": d, "swing": swing,
        "ang": ang, "r_te": r_te, "r_tm": r_tm, "brewster": brewster,
        "mrad": mrad, "ter": ter, "ter_marks": ter_marks, "crit_mrad": crit_mrad, "delta": delta, "beta": beta,
    }


def thinfilm_draw(d, th: S.Theme):
    fig, axes = plt.subplots(2, 2, figsize=(S.WIDE_WIDTH, 6.6), gridspec_kw={"wspace": 0.24, "hspace": 0.42})
    (ax_a, ax_b), (ax_c, ax_d) = axes
    for ax in axes.flat:
        S.style_axes(ax, th, grid="both")
    # (a) standing wave through the resist
    ax = ax_a
    ax.plot(d["z"], d["e_bare"], color=th.s(1), zorder=3)
    ax.plot(d["z"], d["e_arc"], color=th.s(2), zorder=3)
    for zm in d["minima"]:
        ax.axvline(zm, color=th.axis, linewidth=S.HAIR_PT, linestyle=(0, (2, 3)), zorder=1)
    if len(d["minima"]) > 1:
        z0, z1 = d["minima"][0], d["minima"][1]
        yy = float(d["e_bare"].max()) * 0.92
        ax.annotate("", xy=(z1, yy), xytext=(z0, yy), arrowprops=dict(arrowstyle="<->", color=th.ink2, lw=S.HAIR_PT * 1.2))
        ax.text((z0 + z1) / 2, yy, f"λ/2n = {d['period']:.1f} nm", ha="center", va="bottom", color=th.ink2, fontsize=S.SMALL_PT)
    swing_bare = f"bare Si: {d['e_bare'].min():.2f} – {d['e_bare'].max():.2f}"
    swing_arc = f"60 nm ARC: {d['e_arc'].min():.2f} – {d['e_arc'].max():.2f}"
    ax.legend([Line2D([], [], color=th.s(1)), Line2D([], [], color=th.s(2))], [swing_bare, swing_arc], loc="upper right",
              fontsize=S.SMALL_PT, title="|E|² range", title_fontsize=S.SMALL_PT).get_title().set_color(th.ink2)
    ax.set_xlim(0, 150)
    ax.set_ylim(0, float(d["e_bare"].max()) * 1.45)
    ax.set_xlabel("Depth in the resist z (nm)")
    ax.set_ylabel("|E(z)|² (unit incident field)")
    ax.set_title(f"(a) Standing wave, 150 nm resist on Si at {F2_NM} nm")
    # (b) swing curve
    ax = ax_b
    ax.plot(d["d"], d["swing"], color=th.s(1), zorder=3)
    S.dot(ax, [150.0], [d["r_default"]], th.s(1), th)
    S.end_label(ax, 150.0, d["r_default"], f"default stack, 150 nm: R = {d['r_default']:.4f}", th, dx_px=8, dy_px=6)
    S.dot(ax, [150.0], [d["r_arc"]], th.s(2), th)
    S.end_label(ax, 150.0, d["r_arc"], f"150 nm resist + 60 nm ARC: R = {d['r_arc']:.4f}", th, dx_px=8, dy_px=-8)
    dmin = d["d"][1:-1][(d["swing"][1:-1] < d["swing"][:-2]) & (d["swing"][1:-1] <= d["swing"][2:])]
    for x0 in dmin:
        ax.axvline(x0, color=th.axis, linewidth=S.HAIR_PT, linestyle=(0, (2, 3)), zorder=1)
    if len(dmin) > 1:
        ax.text(395.0, float(d["swing"].max()) * 1.1, f"minima {np.mean(np.diff(dmin)):.1f} nm apart (λ/2n)",
                ha="right", va="top", color=th.ink2, fontsize=S.SMALL_PT)
    ax.set_xlim(50, 400)
    ax.set_ylim(0, float(d["swing"].max()) * 1.15)
    ax.set_xlabel("Resist thickness d (nm)")
    ax.set_ylabel("Reflectance R (unpolarized, normal)")
    ax.set_title("(b) Swing curve: resist on Si, bare")
    # (c) Brewster
    ax = ax_c
    ax.plot(d["ang"], d["r_te"], color=th.s(1), zorder=3)
    ax.plot(d["ang"], d["r_tm"], color=th.s(2), zorder=3)
    S.limit_line(ax, d["brewster"], th, f"Brewster θ_B = arctan {GLASS_N} = {d['brewster']:.1f}°", axis="x", where=0.97, side="left", va="top")
    ax.set_xlim(0, 90)
    ax.set_ylim(0, 1.0)
    ax.set_xlabel("Angle of incidence (°)")
    ax.set_ylabel("Reflectance")
    ax.set_title(f"(c) Lossless n = {GLASS_N} substrate: TE and TM")
    ax.legend([Line2D([], [], color=th.s(1)), Line2D([], [], color=th.s(2))], ["TE (s)", "TM (p): zero at θ_B"], loc="upper left")
    # (d) total external reflection
    ax = ax_d
    ax.plot(d["mrad"], d["ter"], color=th.s(1), zorder=3)
    # the critical-angle line stops short of the 4 mrad label; its label sits horizontally in the empty right half
    ax.plot([d["crit_mrad"]] * 2, [0.0, 0.8], color=th.ink2, linewidth=S.HAIR_PT * 1.2, linestyle=(0, (5, 3)), zorder=1)
    ax.text(d["crit_mrad"] + 0.8, 0.62, f"θ_c = √(2δ) = {d['crit_mrad']:.2f} mrad", color=th.ink2, fontsize=S.SMALL_PT, ha="left", va="center")
    for g, r in sorted(d["ter_marks"].items()):
        S.dot(ax, [g], [r], th.s(1), th)
        S.end_label(ax, g, r, f"{r:.4f} at {g:g} mrad", th, dx_px=10, dy_px=(-9 if abs(g - d["crit_mrad"]) < 1 else 5))
    ax.set_xlim(0, 30)
    ax.set_ylim(0, 1.02)
    ax.set_xlabel("Grazing angle (mrad)")
    ax.set_ylabel("Reflectance R_s")
    ax.set_title(f"(d) Au at {AU_KEV:.0f} keV: δ = {d['delta']:.2e}, β = {d['beta']:.2e} (Henke)")
    return fig


register(FigureSpec(
    name="thinfilm-standing-wave-swing",
    group="deep_b",
    compute=thinfilm_compute,
    draw=thinfilm_draw,
    alt="Four thin-film panels: the standing wave through 150 nm of resist on silicon, bare and under an "
        "antireflective layer; the swing curve of reflectance against resist thickness; TE and TM reflectance "
        "against angle with the TM zero at Brewster's angle; gold reflecting 8 keV X-rays below 10 mrad.",
    caption="<strong>Standing waves, the swing curve and the Brewster zero from the transfer-matrix stack.</strong> "
            "(a) |E(z)|² through 150 nm of resist on silicon at 157.63 nm, bare and under a 60 nm ARC (1.8 + 0.3i, the "
            "test fixture's constants, not a product BARC), with the λ/2n spacing of the minima; (b) reflectance "
            "against resist thickness, the default stack and the ARC stack annotated at their computed values; "
            "(c) TE and TM reflectance of a lossless n = 1.5 substrate against incidence, the TM zero at Brewster's "
            "angle; (d) gold at 8 keV, total external reflection below the critical grazing angle √(2δ). The "
            "oblique-TM in-film intensity is the documented |U|² approximation. "
            "<em>Model: thin-film transfer matrix ✅.</em>",
    pages=("processes/thin-film.md",),
))


# ---------------------------------------------------------------------------
# P5a. The depth-averaged 2-D resist chain: Dill exposure → Gaussian PEB → Mack rate → vertical development
# ---------------------------------------------------------------------------

CHAIN_DOSES = (10.0, 30.0, 50.0)
CHAIN_TIMES = (20.0, 40.0, 60.0)
MACK_N = (1.0, 3.0, 10.0)
MACK_DEFAULT = dict(r_max=100.0, r_min=0.1, m_th=0.5, n=3.0)   # ResistParams::vuv_fluoropolymer (resist-models.md)
THRESHOLD = dict(fast=1000.0, slow=0.01, m_th=0.5)              # DevelopmentModel::Threshold


def mack_rate(m, n: float, r_max=MACK_DEFAULT["r_max"], r_min=MACK_DEFAULT["r_min"], m_th=MACK_DEFAULT["m_th"]):
    """The Mack kinetic rate law R(m) in nm/s (resist.rs; n = 1 is the linear limit the code falls back to)."""
    m = np.clip(np.asarray(m, dtype=float), 0.0, 1.0)
    if abs(n - 1.0) < 1e-12:
        return r_max * (1 - m) + r_min
    a = (n + 1) / (n - 1) * (1 - m_th) ** n
    return r_max * (a + 1) * (1 - m) ** n / (a + (1 - m) ** n) + r_min


def resist_chain_compute(fast: bool):
    huv = native()
    n = 128 if fast else 256
    src = huv.SourceConfig.f2_laser(0.7)
    optics = huv.OpticsConfig(numerical_aperture=0.75)
    mask = huv.MaskConfig.line_space(150.0, 300.0)
    grid = mask.commensurate_grid(n, 600.0 / n)
    resist = huv.ResistConfig()
    engine = huv.SimulationEngine(src, optics, mask, resist=resist, grid=grid)
    aerial = engine.compute_aerial_image(0.0)
    I = np.asarray(aerial.intensity)
    x = np.asarray(aerial.x_nm)
    mid = I.shape[0] // 2
    thickness = resist.thickness_nm
    alpha = (resist.dill_a + resist.dill_b) * 1e-3            # 1/nm at m = 1 (bleaching does not evolve on the 2-D path)
    coupling = 1.0 if alpha * thickness < 0.01 else (1 - np.exp(-alpha * thickness)) / (alpha * thickness)

    def slab(arr):
        return huv.VolumetricResult.from_array(np.ascontiguousarray(arr[None, :, :]), (float(x[0]), float(x[-1])),
                                               (float(x[0]), float(x[-1])), (0.0, 1.0))

    latent, baked, rate = {}, {}, {}
    for dose in CHAIN_DOSES:
        m = np.exp(-resist.dill_c * dose * I * coupling)          # Dill m = exp(-C D I_eff), I_eff = I·coupling
        vol = huv.peb_gaussian(slab(m), resist.peb_diffusion_nm)   # the exact Gaussian bake of the latent image
        r = np.asarray(huv.development_rate(vol, resist).values)[0]
        latent[dose], baked[dose], rate[dose] = m[mid], np.asarray(vol.values)[0][mid], r
    profiles = {}
    for t in CHAIN_TIMES:
        p = engine.compute_resist_profile(dose_mj_cm2=30.0, focus_nm=0.0, dev_time_s=t)
        profiles[t] = (np.asarray(p.x_nm), np.asarray(p.height_nm))
    # the plotted rate law against the engine's own development_rate on a ramp of m
    mr = np.linspace(0.0, 1.0, 201)
    ramp = huv.VolumetricResult.from_array(np.ascontiguousarray(np.broadcast_to(mr[None, None, :], (1, 2, mr.size)).copy()),
                                           (0.0, 100.0), (0.0, 2.0), (0.0, 1.0))
    engine_rate = np.asarray(huv.development_rate(ramp, resist).values)[0, 0]
    # --- physical asserts (make_all.py --verify) ---
    assert np.abs(mack_rate(mr, MACK_DEFAULT["n"]) - engine_rate).max() < 1e-6, "Mack rate law differs from development_rate"
    for t in CHAIN_TIMES:
        mine = np.maximum(thickness - rate[30.0][mid] * t, 0.0)
        err = float(np.abs(mine - profiles[t][1]).max())
        # the 2-D path's peb_diffuse truncates its kernel at 3σ (replicate edges); the volumetric peb_gaussian is spectral
        assert err < 2.0, f"chain (Dill → Gaussian PEB → Mack → vertical etch) differs from compute_resist_profile by {err:.2f} nm at {t} s"
    for a, b in zip(CHAIN_DOSES, CHAIN_DOSES[1:]):
        assert np.all(latent[b] <= latent[a] + 1e-12), "more dose must consume more PAC"
    return {"x": x, "aerial": I[mid], "latent": latent, "baked": baked, "profiles": profiles, "thickness": thickness,
            "coupling": coupling, "mr": mr, "engine_rate": engine_rate, "peb_nm": resist.peb_diffusion_nm,
            "dill": (resist.dill_a, resist.dill_b, resist.dill_c)}


def resist_chain_draw(d, th: S.Theme):
    fig, axes = plt.subplots(2, 2, figsize=(S.WIDE_WIDTH, 6.6), gridspec_kw={"wspace": 0.24, "hspace": 0.42})
    (ax_a, ax_b), (ax_c, ax_d) = axes
    for ax in axes.flat:
        S.style_axes(ax, th, grid="both")
    cols = S.ordinal(th, len(CHAIN_DOSES))
    x = d["x"]
    # (a) aerial image and the Dill latent image
    ax = ax_a
    ax.fill_between(x, 0, d["aerial"], color=th.band, zorder=1, lw=0)
    ax.plot(x, d["aerial"], color=th.ink2, linewidth=S.HAIR_PT * 1.4, linestyle=(0, (4, 3)), zorder=2)
    for c, dose in zip(cols, CHAIN_DOSES):
        ax.plot(x, d["latent"][dose], color=c, zorder=3)
    A, B, C = d["dill"]
    ax.set_title(f"(a) Dill m = exp(−C·D·I_eff), C = {C} cm²/mJ, coupling {d['coupling']:.3f}")
    ax.legend([Line2D([], [], color=th.ink2, linestyle=(0, (4, 3)))] + [Line2D([], [], color=c) for c in cols],
              ["aerial image I(x), clear-field units"] + [f"m(x) at {dose:.0f} mJ/cm²" for dose in CHAIN_DOSES],
              loc="lower center", fontsize=S.SMALL_PT - 0.5)
    # (b) after the bake
    ax = ax_b
    for c, dose in zip(cols, CHAIN_DOSES):
        ax.plot(x, d["latent"][dose], color=c, linewidth=S.HAIR_PT * 1.2, alpha=0.5, zorder=2)
        ax.plot(x, d["baked"][dose], color=c, zorder=3)
    ax.set_title(f"(b) After the Gaussian PEB, σ = {d['peb_nm']:.0f} nm (faint: before)")
    ax.legend([Line2D([], [], color=c) for c in cols], [f"{dose:.0f} mJ/cm²" for dose in CHAIN_DOSES], loc="lower right")
    for ax in (ax_a, ax_b):
        ax.set_xlim(x[0], x[-1])
        ax.set_ylim(0, 1.05)
        ax.set_xlabel("x (nm)")
        ax.set_ylabel("Fraction")
    # (c) the rate law
    ax = ax_c
    ncols = S.ordinal(th, len(MACK_N))
    for c, nn in zip(ncols, MACK_N):
        ax.plot(d["mr"], mack_rate(d["mr"], nn), color=c, zorder=3)
    step = np.where(d["mr"] < THRESHOLD["m_th"], THRESHOLD["fast"], THRESHOLD["slow"])
    ax.plot(d["mr"], step, color=th.ink2, linewidth=S.HAIR_PT * 1.4, linestyle=(0, (4, 3)), zorder=2, drawstyle="steps-post")
    S.dot(ax, d["mr"][::20], d["engine_rate"][::20], ncols[1], th, size_px=6.5, zorder=4)
    ax.set_yscale("log")
    ax.set_ylim(2e-3, 3e3)
    ax.set_xlim(0, 1)
    ax.set_xlabel("Protected / remaining PAC fraction m")
    ax.set_ylabel("Dissolution rate R(m) (nm/s)")
    md = MACK_DEFAULT
    ax.set_title(f"(c) Rate law, R_max {md['r_max']:.0f}, R_min {md['r_min']} nm/s, m_th {md['m_th']}")
    ax.legend([Line2D([], [], color=c) for c in ncols] + [Line2D([], [], color=th.ink2, linestyle=(0, (4, 3)))]
              + [Line2D([], [], color=ncols[1], marker="o", linestyle="none", markersize=5)],
              [f"Mack, n = {nn:g}" for nn in MACK_N] + ["threshold (10³ / 10⁻² nm/s)", "development_rate (n = 3)"],
              loc="lower left", bbox_to_anchor=(0.01, 0.02), fontsize=S.SMALL_PT - 1)
    # (d) the developed profile
    ax = ax_d
    tcols = S.ordinal(th, len(CHAIN_TIMES))
    for c, t in zip(tcols, CHAIN_TIMES):
        px, ph = d["profiles"][t]
        ax.plot(px, ph, color=c, zorder=3)
    ax.axhline(d["thickness"], color=th.axis, linewidth=S.HAIR_PT, linestyle=(0, (2, 3)))
    ax.set_xlim(x[0], x[-1])
    ax.set_ylim(0, d["thickness"] * 1.1)
    ax.set_xlabel("x (nm)")
    ax.set_ylabel("Remaining height h(x) (nm)")
    ax.set_title("(d) compute_resist_profile at 30 mJ/cm²: centre-row vertical etch")
    ax.legend([Line2D([], [], color=c) for c in tcols], [f"{t:.0f} s" for t in CHAIN_TIMES], loc="lower right")
    return fig


register(FigureSpec(
    name="resist-2d-chain",
    group="deep_b",
    compute=resist_chain_compute,
    draw=resist_chain_draw,
    alt="The depth-averaged resist chain for 150 nm lines on a 300 nm pitch: the aerial image and the Dill "
        "latent image at three doses, the same after a 30 nm Gaussian bake, the Mack rate law for three "
        "exponents with the threshold step, and the developed profile at 20, 40 and 60 s.",
    caption="<strong>The depth-averaged 2-D chain: exposure → bake → rate law → development.</strong> 150 nm lines "
            "on a 300 nm pitch at 157.63 nm, NA 0.75, σ 0.7, in the default 150 nm VUV resist: (a) the aerial image "
            "and the Dill latent image m(x) at 10, 30 and 50 mJ/cm²; (b) the same after the 30 nm Gaussian bake; "
            "(c) the Mack rate law with the default parameters for n = 1, 3 and 10, the threshold switch, and the "
            "engine's own rate on top of the n = 3 curve; (d) the developed centre-row profile at 20, 40 and 60 s. "
            "Panel (c) is a plotted rate law and the constants are illustrative, not a calibration of this code. "
            "<em>Model: 2-D exposure 🔶 (depth-averaged) · 2-D development 🔶 (centre row) · Gaussian PEB ✅.</em>",
    pages=("processes/resist-models.md",),
    extra={"model_rows": {
        "2-D exposure": "2D resist exposure (Dill ABC)",
        "2-D development": "2D development (Mack / threshold)",
        "Gaussian PEB": "Post-exposure bake (anisotropic Gaussian)",
    }},
))


# ---------------------------------------------------------------------------
# P6a. Grayscale: the contrast curve, the synthesized mask, the printed relief
# ---------------------------------------------------------------------------

GRAY = dict(thickness=500.0, depth=400.0, d_th=20.0, d_clear=80.0, dose=100.0, wavelength=365.0, sigma=0.7, pixel=100.0)
GRAY_NAS = (0.4, 0.75)
GAMMAS = (1.0, 2.0, 3.0)


def contrast_curve(dose, d_th: float, d_clear: float):
    """Removed-depth fraction f(D) of the log-linear contrast curve (grayscale.rs::ContrastCurve), clamped to [0, 1]."""
    gamma = 1.0 / np.log10(d_clear / d_th)
    return np.clip(gamma * np.log10(np.asarray(dose, dtype=float) / d_th), 0.0, 1.0), gamma


def grayscale_compute(fast: bool):
    huv = native()
    n = 64 if fast else 128
    period_px = 16 if fast else 32
    g = GRAY
    target = np.asarray(huv.blazed_grating(n, period_px, g["depth"], g["thickness"]))
    trans = np.asarray(huv.grayscale_transmittance_for_target(target, g["thickness"], g["d_th"], g["d_clear"], g["dose"]))
    # the exact inverse of the contrast curve, as grayscale.rs states it: D(f) = D_th·10^(f/γ), T = D_req / D_exp
    f_req = (g["thickness"] - target) / g["thickness"]
    gamma = 1.0 / np.log10(g["d_clear"] / g["d_th"])
    t_closed = g["d_th"] * 10 ** (f_req / gamma) / g["dose"]
    assert np.abs(trans - t_closed).max() < 1e-9, "grayscale_transmittance_for_target differs from the closed-form inverse"
    import highuvlith as api  # noqa: PLC0415 — simulate_grayscale is the api.py wrapper the page documents

    printed = {}
    for na in GRAY_NAS:
        r = api.simulate_grayscale(target, g["thickness"], d_th=g["d_th"], d_clear=g["d_clear"], exposure_dose_mj_cm2=g["dose"],
                                   wavelength_nm=g["wavelength"], na=na, sigma=g["sigma"], pixel_nm=g["pixel"])
        printed[na] = (np.asarray(r.height_map.x_nm), np.asarray(r.height_map.values))
    row = n // 2
    x = printed[GRAY_NAS[0]][0]
    dose_axis = np.logspace(0.0, 3.0, 400)
    curves = {}
    for gam in GAMMAS:
        d_clear = g["d_th"] * 10 ** (1.0 / gam)
        f, _ = contrast_curve(dose_axis, g["d_th"], d_clear)
        curves[gam] = (d_clear, 1.0 - f)
    f_fig, gamma_fig = contrast_curve(dose_axis, g["d_th"], g["d_clear"])
    for na in GRAY_NAS:  # the printed relief stays inside the film and follows the target's range
        h = printed[na][1][row]
        assert 0.0 <= h.min() and h.max() <= g["thickness"] + 1e-9, "printed height outside the film"
    return {"x": x, "target": target[row], "trans": trans[row], "printed": {na: printed[na][1][row] for na in GRAY_NAS},
            "dose_axis": dose_axis, "curves": curves, "fig_curve": (gamma_fig, 1.0 - f_fig), "period_nm": period_px * g["pixel"]}


def grayscale_draw(d, th: S.Theme):
    fig, (ax_a, ax_b, ax_c) = plt.subplots(1, 3, figsize=(S.WIDE_WIDTH + 0.6, 3.7), gridspec_kw={"wspace": 0.32})
    for ax in (ax_a, ax_b, ax_c):
        S.style_axes(ax, th, grid="both")
    g = GRAY
    # (a) the contrast curve family
    ax = ax_a
    ax.axvspan(1.0, g["d_th"], color=th.band, lw=0, zorder=0)
    gcols = S.ordinal(th, len(GAMMAS))
    for c, gam in zip(gcols, GAMMAS):
        d_clear, h = d["curves"][gam]
        ax.plot(d["dose_axis"], h, color=c, zorder=3)
        ax.axvline(d_clear, color=c, linewidth=S.HAIR_PT, linestyle=(0, (2, 3)), zorder=1)
    gamma_fig, h_fig = d["fig_curve"]
    ax.plot(d["dose_axis"], h_fig, color=th.ink2, linewidth=S.HAIR_PT * 1.5, linestyle=(0, (4, 3)), zorder=2)
    ax.set_xscale("log")
    ax.set_xlim(1.0, 1000.0)
    ax.set_ylim(0, 1.05)
    ax.set_xlabel("Dose D (mJ/cm²), log scale")
    ax.set_ylabel("Remaining height h / T_r")
    ax.set_title("(a) Log-linear model curve")
    ax.text(np.sqrt(g["d_th"]), 0.86, "below D_th\nnothing clears", ha="center", va="center", color=th.ink2, fontsize=S.SMALL_PT)
    ax.legend([Line2D([], [], color=c) for c in gcols] + [Line2D([], [], color=th.ink2, linestyle=(0, (4, 3)))],
              [f"γ = {gam:g}: D_clear {d['curves'][gam][0]:.0f}" for gam in GAMMAS] + [f"γ = {gamma_fig:.2f} (b, c)"],
              loc="lower left", bbox_to_anchor=(0.01, 0.02), fontsize=S.SMALL_PT - 1.5, handlelength=1.2,
              title=f"D_th = {g['d_th']:.0f} mJ/cm²", title_fontsize=S.SMALL_PT - 1.5).get_title().set_color(th.ink2)
    # (b) target relief and the synthesized mask
    ax = ax_b
    x_um = d["x"] / 1000.0
    ax.plot(x_um, d["target"] / g["thickness"], color=th.ink2, linewidth=S.HAIR_PT * 1.5, linestyle=(0, (4, 3)), zorder=2)
    ax.plot(x_um, d["trans"], color=th.s(1), zorder=3)
    ax.set_xlim(x_um[0], x_um[-1])
    ax.set_ylim(0, 1.42)
    ax.set_xlabel("x (µm)")
    ax.set_ylabel("Fraction")
    ax.set_title("(b) Blazed target and its mask")
    ax.legend([Line2D([], [], color=th.ink2, linestyle=(0, (4, 3))), Line2D([], [], color=th.s(1))],
              [f"target h*/T_r, period {d['period_nm'] / 1000:.1f} µm", f"required T(x), one {g['dose']:.0f} mJ/cm² exposure"],
              loc="upper center", fontsize=S.SMALL_PT - 0.5)
    # (c) the printed relief
    ax = ax_c
    ax.plot(x_um, d["target"], color=th.ink2, linewidth=S.HAIR_PT * 1.5, linestyle=(0, (4, 3)), zorder=2)
    for slot, na in zip((1, 2), GRAY_NAS):
        ax.plot(x_um, d["printed"][na], color=th.s(slot), zorder=3)
    ax.set_xlim(x_um[0], x_um[-1])
    ax.set_ylim(0, g["thickness"] * 1.36)
    ax.set_xlabel("x (µm)")
    ax.set_ylabel("Height (nm)")
    ax.set_title("(c) Printed through i-line optics")
    ax.legend([Line2D([], [], color=th.ink2, linestyle=(0, (4, 3)))] + [Line2D([], [], color=th.s(s)) for s in (1, 2)],
              ["target"] + [f"printed, NA {na}" for na in GRAY_NAS], loc="upper center", ncol=3, fontsize=S.SMALL_PT - 0.5,
              title=f"{g['wavelength']:.0f} nm, σ {g['sigma']}, {g['pixel']:.0f} nm pixels", title_fontsize=S.SMALL_PT - 0.5).get_title().set_color(th.ink2)
    return fig


register(FigureSpec(
    name="grayscale-contrast-curve",
    group="deep_b",
    compute=grayscale_compute,
    draw=grayscale_draw,
    alt="Three grayscale panels: remaining height against log dose for contrast exponents 1, 2 and 3 with the "
        "threshold and clearing doses; a sawtooth target with the continuous mask transmittance that prints it; "
        "and the relief printed through i-line optics at NA 0.4 and 0.75, rounded at the sawtooth edges.",
    caption="<strong>The contrast curve is the whole resist model.</strong> (a) Remaining height against log dose for "
            "γ = 1, 2 and 3 at a 20 mJ/cm² threshold: the log-linear model curve with its clamps, no development "
            "dynamics; (b) a blazed-grating target and the continuous transmittance mask that prints it in one "
            "100 mJ/cm² exposure (threshold 20, clearing 80 mJ/cm²); (c) the relief printed through i-line optics at "
            "NA 0.4 and 0.75, where the band-limited image rounds the sawtooth edges. The mask carries intensity "
            "transmittance only (zero phase). <em>Model: grayscale lithography 🔶 (log-linear contrast curve, "
            "2.5-D height map, no standing waves or bake).</em>",
    pages=("processes/grayscale.md",),
))


# ---------------------------------------------------------------------------
# P7b. Interference fill fraction against dose, and the two-photon voxel of a Gaussian focus (closed form)
# ---------------------------------------------------------------------------

FILL_DOSES = (0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0)
FILL_THRESHOLD = 0.5
UMBRELLA = dict(wavelength_nm=355.0, n_medium=1.67, half_angle_deg=40.0, dill_c=0.5)   # the page's Campbell-style example
FOCUS = dict(wavelength_nm=800.0, n=1.5, waists=(200.0, 800.0))                             # GaussianFocus is Rust-only


def gaussian_focus(r, z, w0: float, wavelength_nm: float, n: float):
    """Scalar paraxial Gaussian beam: z_R, w(z) and the normalised intensity I(r, z) (interference.rs::GaussianFocus)."""
    z_r = np.pi * w0 ** 2 * n / wavelength_nm
    w = w0 * np.sqrt(1 + (z / z_r) ** 2)
    return z_r, w, (w0 / w) ** 2 * np.exp(-2 * r ** 2 / w ** 2)


def interference_compute(fast: bool):
    huv = native()
    nxy, nz = (32, 16) if fast else (48, 24)
    fill = {}
    consumed = {}
    for two_photon in (False, True):
        fs, cs = [], []
        for scale in FILL_DOSES:
            pac = huv.simulate_interference(
                "four_beam_umbrella", wavelength_nm=UMBRELLA["wavelength_nm"], n_medium=UMBRELLA["n_medium"],
                half_angle_deg=UMBRELLA["half_angle_deg"], nx=nxy, ny=nxy, nz=nz, x_span_nm=2000.0, y_span_nm=2000.0,
                z_span_nm=2000.0, dose_scale=float(scale), dill_c=UMBRELLA["dill_c"], two_photon=two_photon)
            m = np.asarray(pac.values)
            fs.append(float((m < FILL_THRESHOLD).mean()))
            cs.append(float((1 - m).mean()))
        fill[two_photon], consumed[two_photon] = np.asarray(fs), np.asarray(cs)
        assert np.all(np.diff(fill[two_photon]) >= -1e-12), "fill fraction must not fall with dose"
    # the closed-form focus and the Rust test's pins: w(z_R) = w0·√2, I(0, z_R) = 1/2, I² falls 4× at z_R
    z_norm = np.linspace(-3.0, 3.0, 601)
    out_focus = {}
    for w0 in FOCUS["waists"]:
        z_r, w, _ = gaussian_focus(0.0, z_norm * 1.0, w0, FOCUS["wavelength_nm"], FOCUS["n"])
        z_r, w_r, i_r = gaussian_focus(0.0, z_r, w0, FOCUS["wavelength_nm"], FOCUS["n"])
        assert abs(w_r - w0 * np.sqrt(2)) < 1e-9 and abs(i_r - 0.5) < 1e-12 and abs(i_r ** 2 - 0.25) < 1e-12
        out_focus[w0] = z_r
    axial = 1.0 / (1.0 + z_norm ** 2)             # I(0, z) in units of z_R, identical for every waist
    # the r–z map for the 200 nm waist
    w0 = FOCUS["waists"][0]
    z_r0 = out_focus[w0]
    rr = np.linspace(-600.0, 600.0, 121 if fast else 241)
    zz = np.linspace(-1000.0, 1000.0, 161 if fast else 321)
    R, Z = np.meshgrid(rr, zz)
    _, w_env, i_map = gaussian_focus(R, Z, w0, FOCUS["wavelength_nm"], FOCUS["n"])
    fwhm_i = 2.0                                   # in z_R: I = 1/2 at z = z_R
    fwhm_i2 = 2.0 * np.sqrt(np.sqrt(2.0) - 1.0)    # I² = 1/2 at z = z_R·√(√2 − 1)
    return {"doses": np.asarray(FILL_DOSES), "fill": fill, "consumed": consumed, "z_norm": z_norm, "axial": axial,
            "z_r": out_focus, "rr": rr, "zz": zz, "i2_map": i_map ** 2, "w_env": w0 * np.sqrt(1 + (zz / z_r0) ** 2),
            "fwhm": (fwhm_i, fwhm_i2)}


def interference_draw(d, th: S.Theme):
    fig = plt.figure(figsize=(S.WIDE_WIDTH + 0.6, 3.7))
    gs = fig.add_gridspec(1, 3, width_ratios=[1.15, 0.85, 1.05], wspace=0.5)
    pale = "#f4f3f8"  # ink over the mode-invariant field ramp (dark = 0 in both themes)
    # (a) fill fraction against dose
    ax = fig.add_subplot(gs[0, 0])
    S.style_axes(ax, th, grid="both")
    for slot, two_photon in zip((1, 2), (False, True)):
        ax.plot(d["doses"], d["fill"][two_photon], color=th.s(slot), zorder=3)
        S.dot(ax, d["doses"], d["fill"][two_photon], th.s(slot), th, size_px=7)
    ax.set_xscale("log", base=2)
    ax.set_xticks(d["doses"])
    ax.set_xticklabels([f"{v:g}" for v in d["doses"]])
    ax.set_xlim(d["doses"][0] / 1.3, d["doses"][-1] * 1.3)
    ax.set_ylim(0, 1.05)
    ax.set_xlabel("Dose scale (× the preset dose)")
    ax.set_ylabel(f"Fill fraction: voxels with m < {FILL_THRESHOLD}")
    u = UMBRELLA
    ax.set_title("(a) Four-beam umbrella: fill against dose")
    ax.legend([Line2D([], [], color=th.s(1)), Line2D([], [], color=th.s(2))],
              ["one-photon, m = exp(−C D I)", "two-photon, m = exp(−C D I²)"], loc="lower right",
              title=f"{u['wavelength_nm']:.0f} nm in n = {u['n_medium']}, {u['half_angle_deg']:.0f}° air-side, C = {u['dill_c']}",
              title_fontsize=S.SMALL_PT).get_title().set_color(th.ink2)
    # (b) the I² map of the 200 nm focus
    ax = fig.add_subplot(gs[0, 1])
    w0 = FOCUS["waists"][0]
    im = ax.imshow(d["i2_map"], origin="lower", extent=[d["rr"][0] / 1000, d["rr"][-1] / 1000, d["zz"][0] / 1000, d["zz"][-1] / 1000],
                   aspect="auto", cmap=S.field_cmap(), vmin=0, vmax=1, interpolation="bilinear")
    ax.plot(d["w_env"] / 1000, d["zz"] / 1000, color=th.s(2), linewidth=S.HAIR_PT * 1.6, linestyle=(0, (4, 3)))
    ax.plot(-d["w_env"] / 1000, d["zz"] / 1000, color=th.s(2), linewidth=S.HAIR_PT * 1.6, linestyle=(0, (4, 3)))
    z_r = d["z_r"][w0]
    for sgn in (-1, 1):
        ax.axhline(sgn * z_r / 1000, color=pale, linewidth=S.HAIR_PT, linestyle=(0, (2, 3)))
    ax.text(0.55, z_r / 1000, f"z_R = {z_r / 1000:.2f} µm", color=pale, fontsize=S.SMALL_PT, va="bottom", ha="right")
    S.image_axes(ax, th)
    ax.set_xlabel("r (µm)")
    ax.set_ylabel("z (µm)")
    ax.set_title(f"(b) I² of a w$_0$ = {w0:.0f} nm focus")
    S.colorbar(fig, im, ax, th, "I² / I²(0, 0); dashed: w(z)", fraction=0.06, pad=0.02)
    # (c) axial profiles in units of z_R
    ax = fig.add_subplot(gs[0, 2])
    S.style_axes(ax, th, grid="both")
    ax.plot(d["z_norm"], d["axial"], color=th.s(1), zorder=3)
    ax.plot(d["z_norm"], d["axial"] ** 2, color=th.s(2), zorder=3)
    fi, fi2 = d["fwhm"]
    for y_, fw, slot in ((0.5, fi, 1), (0.5, fi2, 2)):
        ax.plot([-fw / 2, fw / 2], [y_, y_], color=th.s(slot), linewidth=S.HAIR_PT * 1.6, linestyle=(0, (3, 2)), zorder=2)
    ax.text(1.45, 0.57, f"FWHM {fi:.2f} z_R (I)\n{fi2:.2f} z_R (I²)", color=th.ink2, fontsize=S.SMALL_PT, va="center")
    ax.set_xlim(-3, 3)
    ax.set_ylim(0, 1.05)
    ax.set_xlabel("z / z_R")
    ax.set_ylabel("On-axis I(0, z) = 1 / (1 + (z/z_R)²)")
    ax.set_title("(c) I and I² along the axis (closed form)")
    zr = {w: d["z_r"][w] for w in FOCUS["waists"]}
    ax.text(2.95, 1.02, "z_R\n" + "\n".join(f"{zr[w] / 1000:.2f} µm (w$_0$ {w:.0f} nm)" for w in FOCUS["waists"]), ha="right",
            va="top", color=th.ink2, fontsize=S.SMALL_PT - 1)
    ax.legend([Line2D([], [], color=th.s(1)), Line2D([], [], color=th.s(2))],
              ["I(0, z)", "I² (two-photon)"], loc="upper left", fontsize=S.SMALL_PT - 1, handlelength=1.2)
    return fig


register(FigureSpec(
    name="interference-fill-voxel",
    group="deep_b",
    compute=interference_compute,
    draw=interference_draw,
    alt="Three panels: the fill fraction of a four-beam interference lattice rising with dose for one- and "
        "two-photon kinetics; the squared intensity of a 200 nm Gaussian focus in the r-z plane with its beam "
        "envelope; on-axis intensity and intensity squared against z, the latter narrower.",
    caption="<strong>Dose sets the fill; I² kinetics shrink the voxel.</strong> (a) Iso-surface fill fraction "
            "(m &lt; 0.5) of the four-beam umbrella lattice against the dose scale for one-photon and two-photon "
            "kinetics, each point a <code>simulate_interference</code> run (355 nm in n = 1.67, 40° air-side); "
            "(b) the scalar Gaussian focus I² for w₀ = 200 nm at 800 nm in n = 1.5 with its w(z) envelope; "
            "(c) on-axis I and I² in units of the Rayleigh length, the I² voxel shorter by √(√2 − 1). Panels (b) "
            "and (c) are closed-form Gaussian-beam formulas, no high-NA vector focusing, and the voxel writer "
            "itself is Rust-only. <em>Model: multi-beam interference ✅ (🔶 two-photon kinetics, per-beam scalar "
            "absorption).</em>",
    pages=("processes/interference-volumetric.md",),
    extra={"model_rows": {"multi-beam interference": "Multi-beam interference / two-photon"}},
))
