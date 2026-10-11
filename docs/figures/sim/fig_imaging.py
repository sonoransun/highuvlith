"""Imaging-physics figures: resolution, defocus model, polarization, immersion, comb imaging.

All numbers come from ``highuvlith._native`` (scalar/vector Hopkins-SOCS engine,
exact per-source-point defocus, per-wavelength TCC).
"""

from __future__ import annotations

import numpy as np
from matplotlib import pyplot as plt
from matplotlib.gridspec import GridSpec

import style as S
from common import FigureSpec, native, register

F2_NM = 157.63
ARF_NM = 193.368


def _ls_contrast(huv, source, optics, pitch, *, cd=None, focus=0.0, pixel=1.0, size=128, **engine_kw):
    """Contrast of a line/space grating (dark lines of width cd, default pitch/2)."""
    mask = huv.MaskConfig.line_space(pitch / 2 if cd is None else cd, pitch)
    grid = mask.commensurate_grid(size, pixel)
    eng = huv.SimulationEngine(source, optics, mask, grid=grid, max_kernels=64, **engine_kw)
    return eng.compute_aerial_image(focus).image_contrast()


# ---------------------------------------------------------------------------
# 1. Resolution vs pitch (partial coherence extends the cutoff)
# ---------------------------------------------------------------------------

RES_NA = 0.75
RES_SIGMAS = (None, 0.4, 0.8)  # None = one coherent source point


def res_compute(fast: bool):
    huv = native()
    optics = huv.OpticsConfig(numerical_aperture=RES_NA, flare_fraction=0.0)
    pitches = np.arange(100.0, 330.1, 6.0 if fast else 2.0)
    curves = {}
    for sigma in RES_SIGMAS:
        src = huv.SourceConfig.f2_laser(0.7 if sigma is None else sigma)
        kw = {"source_points_per_axis": 1} if sigma is None else {}
        curves[sigma] = np.array([_ls_contrast(huv, src, optics, p, pixel=2.0, **kw) for p in pitches])
    return {"pitch": pitches, "curves": curves}


def res_draw(d, th: S.Theme):
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 4.1))
    S.style_axes(ax, th)
    labels = {None: "coherent (one source point)", 0.4: "σ = 0.4", 0.8: "σ = 0.8"}
    x = d["pitch"]
    for slot, sigma in enumerate(RES_SIGMAS, start=1):
        ax.plot(x, d["curves"][sigma], color=th.s(slot), zorder=3)
    S.limit_line(ax, F2_NM / RES_NA, th, "λ/NA", where=0.03, va="bottom", side="right")
    for sigma in (0.4, 0.8):
        S.limit_line(ax, F2_NM / (RES_NA * (1 + sigma)), th, f"λ/(NA(1+{sigma}))", where=0.97, va="top",
                     side="left")
    ax.set_xlim(x[0], x[-1])
    ax.set_ylim(0, 1.0)
    ax.set_xlabel("Pitch of 1:1 lines and spaces (nm)")
    ax.set_ylabel("Aerial-image contrast")
    top = ax.secondary_xaxis("top", functions=(lambda p: p / 2 * RES_NA / F2_NM, lambda k: 2 * k * F2_NM / RES_NA))
    top.set_xlabel(r"$k_1$ = (pitch/2)·NA/λ", color=th.ink2, fontsize=S.SMALL_PT)
    top.tick_params(colors=th.axis, labelcolor=th.ink2, labelsize=S.SMALL_PT)
    top.spines["top"].set_color(th.axis)
    ax.set_title("Partial coherence moves the resolution limit from λ/NA to λ/(NA(1+σ))", pad=26)
    ax.legend([plt.Line2D([], [], color=th.s(i)) for i in (1, 2, 3)], [labels[sg] for sg in RES_SIGMAS],
              loc="lower right")
    return fig


register(FigureSpec(
    name="imaging-resolution-vs-pitch",
    group="imaging",
    compute=res_compute,
    draw=res_draw,
    alt="Aerial-image contrast versus pitch for coherent, sigma 0.4 and sigma 0.8 illumination at 157.63 nm and "
        "NA 0.75; each curve drops to zero at its cutoff pitch lambda/(NA(1+sigma)).",
    caption="<strong>Partial coherence moves the resolution limit from λ/NA to λ/(NA(1+σ)).</strong> Simulated "
            "contrast of 1:1 line/space gratings versus pitch (F<sub>2</sub> 157.63 nm, NA 0.75, conventional "
            "illumination, no flare): a coherent point source stops imaging at λ/NA = 210 nm, a partially coherent fill "
            "of radius σ keeps imaging down to λ/(NA(1+σ)). <em>Model: Hopkins imaging (factorized TCC/SOCS) ✅; thin "
            "mask, no flare.</em>",
    pages=("pipeline.md", "nodes/litho-math.md", "README.md"),
    extra={"width": {"README.md": 820}},  # README <picture> width (DESIGN §6.2; INT late-binding edit)
))


# ---------------------------------------------------------------------------
# 2. Exact per-source-point defocus vs the legacy kernel-phase approximation
# ---------------------------------------------------------------------------

DEF_PITCH, DEF_CD, DEF_SIGMA = 180.0, 90.0, 0.7


def defocus_compute(fast: bool):
    huv = native()
    src = huv.SourceConfig.f2_laser(DEF_SIGMA)
    optics = huv.OpticsConfig(numerical_aperture=0.75, flare_fraction=0.0)
    mask = huv.MaskConfig.line_space(DEF_CD, DEF_PITCH)
    grid = mask.commensurate_grid(128, 1.5)
    focus = np.arange(-400.0, 400.1, 20.0 if fast else 5.0)
    out = {"focus": focus}
    for model in ("exact", "kernel_phase"):
        eng = huv.SimulationEngine(src, optics, mask, grid=grid, max_kernels=64, defocus_model=model)
        imgs = eng.compute_through_focus(list(focus))
        out[model] = {
            "contrast": np.array([im.image_contrast() for im in imgs]),
            "xz": np.array([im.intensity[im.intensity.shape[0] // 2] for im in imgs]),
        }
        out["x"] = np.asarray(imgs[0].x_nm)
    return out


def defocus_draw(d, th: S.Theme):
    fig = plt.figure(figsize=(S.WIDE_WIDTH, 4.2))
    gs = GridSpec(1, 4, figure=fig, width_ratios=[1.5, 1, 1, 0.06], wspace=0.3)
    ax = fig.add_subplot(gs[0, 0])
    S.style_axes(ax, th)
    z = d["focus"]
    for slot, model in ((1, "exact"), (2, "kernel_phase")):
        ax.plot(z, d[model]["contrast"], color=th.s(slot), zorder=3)
    ax.set_xlim(z[0], z[-1])
    ax.set_ylim(0, 0.7)
    ax.set_xlabel("Focus (nm)")
    ax.set_ylabel("Aerial-image contrast")
    ax.set_title("(a) Contrast through focus")
    ax.legend([plt.Line2D([], [], color=th.s(1)), plt.Line2D([], [], color=th.s(2))],
              ["exact: defocus inside the pupil, per source point", "legacy: one phase per mask frequency"],
              loc="upper center", bbox_to_anchor=(0.5, -0.17), ncol=1)
    cmap = S.field_cmap()
    x = d["x"]
    period = float(len(x) * (x[1] - x[0]))
    vmax = max(d["exact"]["xz"].max(), d["kernel_phase"]["xz"].max())
    im = None
    for k, (model, title) in enumerate((("exact", "(b) exact: I(x, focus)"), ("kernel_phase", "(c) legacy: I(x, focus)"))):
        axm = fig.add_subplot(gs[0, 1 + k])
        # the field is one period centred on x = 0; show two periods, [-p, p)
        rows = d[model]["xz"]
        img = np.tile(np.roll(rows, rows.shape[1] // 2, axis=1), (1, 2))
        im = axm.imshow(img, origin="lower", aspect="auto", extent=[-period, period, z[0], z[-1]], cmap=cmap,
                        vmin=0, vmax=vmax, interpolation="bilinear")
        S.image_axes(axm, th)
        axm.set_title(title)
        axm.set_xlabel("x (nm)")
        if k == 0:
            axm.set_ylabel("Focus (nm)")
        else:
            axm.tick_params(labelleft=False)
    cax = fig.add_subplot(gs[0, 3])
    cb = fig.colorbar(im, cax=cax)
    cb.outline.set_edgecolor(th.axis)
    cb.outline.set_linewidth(S.HAIR_PT)
    cb.ax.tick_params(colors=th.axis, labelcolor=th.ink2, labelsize=S.SMALL_PT, width=S.HAIR_PT, length=2.5)
    cb.set_label("Relative intensity", color=th.ink2, fontsize=S.SMALL_PT)
    fig.subplots_adjust(bottom=0.27, right=0.93)
    return fig


register(FigureSpec(
    name="imaging-defocus-model",
    group="imaging",
    compute=defocus_compute,
    draw=defocus_draw,
    alt="Contrast versus focus for 90 nm lines at 180 nm pitch: the exact defocus model decays smoothly, the legacy "
        "kernel-phase model shows false contrast revivals; two intensity maps versus focus show the same difference.",
    caption="<strong>Why the defocus fix matters.</strong> Simulated 90 nm lines on a 180 nm pitch (F<sub>2</sub> "
            "157.63 nm, NA 0.75, σ 0.7, no flare) through ±400 nm of focus: the exact model applies the defocus phase "
            "inside the pupil for every source point, while the legacy <code>kernel_phase</code> option multiplies each "
            "mask frequency by one on-axis phase and predicts Talbot-like contrast revivals that partially coherent "
            "illumination washes out. <em>Model: defocus / through-focus imaging ✅ (exact, the default; 🔶 legacy "
            "KernelPhase, kept for comparison only).</em>",
    pages=("pipeline.md",),
))


# ---------------------------------------------------------------------------
# 3. TE / TM / unpolarized: two-beam pupil physics and full L/S imaging
# ---------------------------------------------------------------------------

VEC_POLS = (("y", "TE (field along the lines)"), ("x", "TM (field across the lines)"), ("unpolarized", "unpolarized"))


def _two_beam_contrast(huv, na, pol, film_n=None):
    """Fringe contrast of the (+1, 0) and (-1, 0) orders at the pupil edge (vector pupil module)."""
    vs = huv.VectorSettings(pol, film_n=film_n)
    e1 = huv.vector_pupil_field(vs, na, 1.0, 0.0)
    e2 = huv.vector_pupil_field(vs, na, -1.0, 0.0)
    cross = np.sum(e1 * np.conj(e2))
    power = np.sum(np.abs(e1) ** 2 + np.abs(e2) ** 2)
    return float(2 * abs(cross) / power)


def vector_compute(fast: bool):
    huv = native()
    na_a = np.linspace(0.05, 0.98, 25 if fast else 94)
    two_beam = {pol: np.array([_two_beam_contrast(huv, na, pol) for na in na_a]) for pol, _ in VEC_POLS}
    two_beam["x_film"] = np.array([_two_beam_contrast(huv, na, "x", film_n=1.7) for na in na_a])
    na_b = np.linspace(0.40, 0.95, 6 if fast else 23)
    src = huv.SourceConfig(wavelength_nm=ARF_NM, sigma_outer=0.3, bandwidth_pm=0.0, spectral_samples=1)
    engine = {pol: [] for pol in ("scalar",) + tuple(p for p, _ in VEC_POLS)}
    for na in na_b:
        optics = huv.OpticsConfig(numerical_aperture=float(na), flare_fraction=0.0)
        pitch = round(1.1 * ARF_NM / na, 2)
        engine["scalar"].append(_ls_contrast(huv, src, optics, pitch))
        for pol, _ in VEC_POLS:
            engine[pol].append(_ls_contrast(huv, src, optics, pitch, vector=huv.VectorSettings(pol)))
    return {"na_a": na_a, "two_beam": two_beam, "na_b": na_b, "engine": {k: np.array(v) for k, v in engine.items()}}


def vector_draw(d, th: S.Theme):
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 4.1), gridspec_kw={"wspace": 0.3})
    for ax in (ax1, ax2):
        S.style_axes(ax, th)
        ax.set_ylim(0, 1.03)
        ax.set_xlabel("Numerical aperture")
    na = d["na_a"]
    for slot, (pol, label) in enumerate(VEC_POLS, start=1):
        ax1.plot(na, d["two_beam"][pol], color=th.s(slot), zorder=3)
    ax1.plot(na, d["two_beam"]["x_film"], color=th.s(4), zorder=3)
    S.limit_line(ax1, 1 / np.sqrt(2), th, "NA = 1/√2", where=0.9, va="top", side="left")
    ax1.set_xlim(0, 1.0)
    ax1.set_ylabel("Two-beam fringe contrast")
    ax1.set_title("(a) Two orders at the pupil edge")
    nb = d["na_b"]
    ax2.plot(nb, d["engine"]["scalar"], color=th.ink2, linewidth=S.HAIR_PT * 1.4, zorder=2)
    for slot, (pol, label) in enumerate(VEC_POLS, start=1):
        ax2.plot(nb, d["engine"][pol], color=th.s(slot), zorder=3)
        S.dot(ax2, [nb[-1]], [d["engine"][pol][-1]], th.s(slot), th)
    ax2.set_xlim(0.38, 0.95)
    ax2.set_ylabel("Aerial-image contrast")
    ax2.set_title("(b) Dense 1:1 lines, pitch 1.1 λ/NA, σ 0.3")
    ax2.annotate("scalar", xy=(0.6, float(np.interp(0.6, nb, d["engine"]["scalar"]))), xytext=(0.6, 0.99),
                 color=th.ink2, fontsize=S.SMALL_PT, ha="center", va="bottom",
                 arrowprops=dict(arrowstyle="-", color=th.axis, lw=S.HAIR_PT, shrinkA=0, shrinkB=1))
    handles = [plt.Line2D([], [], color=th.s(i)) for i in (1, 2, 3, 4)]
    fig.legend(handles, [lab for _, lab in VEC_POLS] + ["TM inside a resist film, n = 1.7 (a)"],
               loc="lower center", ncol=4, bbox_to_anchor=(0.5, 0.0))
    fig.subplots_adjust(bottom=0.2)
    return fig


register(FigureSpec(
    name="imaging-vector-te-tm",
    group="imaging",
    compute=vector_compute,
    draw=vector_draw,
    alt="Two charts versus numerical aperture: TE contrast stays at one while TM contrast falls to zero at NA 0.71 "
        "and reverses; unpolarized sits in between; the same ordering appears in simulated dense line images.",
    caption="<strong>Polarization at high NA.</strong> (a) Fringe contrast of two diffraction orders at the pupil edge "
            "from the vector pupil model: TE stays at 1, TM follows |cos 2θ| and vanishes at NA = 1/√2, unpolarized "
            "light gives cos²θ; inside a resist film (n = 1.7) the refracted angle is smaller and TM recovers. "
            "(b) Full vector imaging of dense 1:1 lines (193.368 nm, pitch 1.1 λ/NA, conventional σ 0.3) shows "
            "the same ordering. <em>Model: vector (polarized) high-NA imaging ✅; thin mask, no mask-3D or lens "
            "polarization aberrations.</em>",
    pages=("vector-imaging.md", "future/high-na-and-hyper-na.md", "README.md"),
    extra={"width": {"README.md": 820}},  # README <picture> width (DESIGN §6.2; INT late-binding edit)
))


def pupil_compute(fast: bool):
    huv = native()
    n = 41 if fast else 121
    out = {"n": n, "na": 0.9}
    for pol in ("x", "unpolarized"):
        out[pol] = huv.vector_pupil_map(huv.VectorSettings(pol), 0.9, n=n)
    return out


def pupil_draw(d, th: S.Theme):
    from matplotlib.patches import Circle

    n = d["n"]
    p = np.linspace(-1, 1, n)
    px, py = np.meshgrid(p, p)
    inside = px ** 2 + py ** 2 <= 1.0
    fig, axes = plt.subplots(1, 2, figsize=(S.FULL_WIDTH, 3.9), gridspec_kw={"wspace": 0.42})
    cmap = S.field_cmap()
    field_x = d["x"][0]  # (3, n, n): the single x-polarized state
    tot = np.sum(np.abs(field_x) ** 2, axis=0)
    safe = np.where(tot > 0, tot, 1.0)
    long_frac = np.where(inside & (tot > 0), np.abs(field_x[2]) ** 2 / safe, np.nan)
    un = d["unpolarized"]  # (2 states, 3, n, n)
    tot_u = np.sum(np.abs(un) ** 2, axis=(0, 1))
    rel_u = np.where(inside, tot_u / np.nanmax(np.where(inside, tot_u, np.nan)), np.nan)
    # extend each map past the rim with its rim value so the circular clip path, not
    # the pixel grid, draws the pupil edge
    r = np.hypot(px, py)
    j = np.clip(np.round((px / np.maximum(r, 1e-12) * np.minimum(r, 0.985) + 1) / 2 * (n - 1)).astype(int), 0, n - 1)
    i = np.clip(np.round((py / np.maximum(r, 1e-12) * np.minimum(r, 0.985) + 1) / 2 * (n - 1)).astype(int), 0, n - 1)

    def extend(a):
        return np.where(inside, a, a[i, j])

    panels = ((axes[0], extend(long_frac), r"$|E_z|^2\,/\,|E|^2$", "(a) x-polarized: longitudinal share"),
              (axes[1], extend(rel_u), r"$|E|^2$ / max", "(b) unpolarized: intensity incl. obliquity"))
    for ax, data, clabel, title in panels:
        im = ax.imshow(data, origin="lower", extent=[-1, 1, -1, 1], cmap=cmap, vmin=0, vmax=float(np.nanmax(data)),
                       interpolation="bilinear")
        im.set_clip_path(Circle((0, 0), 1.0, transform=ax.transData))
        ax.add_patch(Circle((0, 0), 1.0, fill=False, edgecolor=th.axis, linewidth=S.HAIR_PT))
        S.colorbar(fig, im, ax, th, clabel)
        ax.set_title(title)
        ax.set_xlim(-1.05, 1.05)
        ax.set_ylim(-1.05, 1.05)
        ax.set_aspect("equal")
        ax.set_xlabel("pupil x (units of NA)")
        for side in ("top", "right", "left", "bottom"):
            ax.spines[side].set_visible(False)
        ax.tick_params(colors=th.axis, labelcolor=th.ink2)
    axes[0].set_ylabel("pupil y (units of NA)")
    # transverse-field arrows on a coarse sub-grid of the x-polarized map
    step = max(1, n // 12)
    sl = (slice(step // 2, None, step), slice(step // 2, None, step))
    ex, ey = np.real(field_x[0]), np.real(field_x[1])
    keep = inside[sl] & ((px[sl] ** 2 + py[sl] ** 2) <= 0.92)
    scale = np.nanmax(np.hypot(ex, ey)[inside])
    from matplotlib import patheffects

    q = axes[0].quiver(px[sl][keep], py[sl][keep], ex[sl][keep] / scale, ey[sl][keep] / scale, color="#ffffff",
                       angles="xy", scale_units="xy", scale=9.0, width=0.007, headwidth=3.4, headlength=3.8,
                       pivot="middle", zorder=3)
    q.set_path_effects([patheffects.withStroke(linewidth=1.6, foreground="#0b0b0b")])
    return fig


register(FigureSpec(
    name="imaging-vector-pupil",
    group="imaging",
    compute=pupil_compute,
    draw=pupil_draw,
    alt="Two pupil maps at NA 0.9: arrows of the transverse field for x-polarized light over a map of the "
        "longitudinal field share, which grows towards the pupil edge along x; and the total field intensity for "
        "unpolarized light, which rises towards the edge.",
    caption="<strong>The vector pupil at NA 0.9 (image in air).</strong> (a) For x-polarized illumination the field of "
            "each order tilts with its propagation direction: near the pupil edge along x most of the energy is in "
            "E<sub>z</sub>, which cannot interfere with the opposite order. (b) The radiometric (obliquity) factor "
            "brightens the pupil edge. <em>Model: vector (polarized) high-NA imaging ✅ (the vector pupil model, "
            "<code>vector_pupil_map</code>).</em>",
    pages=("vector-imaging.md",),
))


# ---------------------------------------------------------------------------
# 4. Water immersion: 193i NA 1.35 vs dry NA 0.93
# ---------------------------------------------------------------------------

IMM_SIGMA = 0.9


def immersion_compute(fast: bool):
    huv = native()
    src = huv.SourceConfig.arf_laser(IMM_SIGMA)
    dry = huv.OpticsConfig(numerical_aperture=0.93, flare_fraction=0.0)
    wet = huv.OpticsConfig.immersion(numerical_aperture=1.35, immersion_index=1.437, flare_fraction=0.0)
    pitches = np.arange(60.0, 170.1, 5.0 if fast else 1.0)
    series = {
        "dry": [_ls_contrast(huv, src, dry, p) for p in pitches],
        "wet_scalar": [_ls_contrast(huv, src, wet, p) for p in pitches],
        "wet_y": [_ls_contrast(huv, src, wet, p, vector=huv.VectorSettings("y")) for p in pitches],
        "wet_unpol": [_ls_contrast(huv, src, wet, p, vector=huv.VectorSettings()) for p in pitches],
    }
    return {"pitch": pitches, **{k: np.array(v) for k, v in series.items()}}


def immersion_draw(d, th: S.Theme):
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 4.3))
    S.style_axes(ax, th)
    x = d["pitch"]
    spec = (("dry", 1, "dry ArF, NA 0.93 (scalar)"), ("wet_scalar", 2, "193i, NA 1.35 (scalar)"),
            ("wet_y", 3, "193i, TE-polarized (vector)"), ("wet_unpol", 4, "193i, unpolarized (vector)"))
    ends = []
    for key, slot, label in spec:
        ax.plot(x, d[key], color=th.s(slot), zorder=3)
        S.dot(ax, [x[-1]], [d[key][-1]], th.s(slot), th)
        ends.append((x[-1], d[key][-1], label))
    S.limit_line(ax, ARF_NM / (0.93 * (1 + IMM_SIGMA)), th, "dry cutoff ≈ 109 nm", where=0.97, side="left")
    S.limit_line(ax, ARF_NM / (1.35 * (1 + IMM_SIGMA)), th, "193i cutoff ≈ 75 nm", where=0.97, side="left")
    ax.set_xlim(x[0], x[-1])
    ax.set_ylim(0, 1.0)
    ax.set_xlabel("Pitch of 1:1 lines and spaces (nm)")
    ax.set_ylabel("Aerial-image contrast")
    ax.set_title("Water immersion: NA 1.35 prints pitches that dry NA 0.93 cannot (193.368 nm, σ 0.9)")
    fig.subplots_adjust(right=0.73)
    S.end_labels(ax, ends, th)
    return fig


register(FigureSpec(
    name="imaging-immersion-193i",
    group="imaging",
    compute=immersion_compute,
    draw=immersion_draw,
    alt="Contrast versus pitch at 193 nm: dry NA 0.93 stops imaging near 109 nm pitch, water immersion at NA 1.35 "
        "keeps imaging down to about 75 nm; vector imaging shows unpolarized light losing contrast and TE "
        "polarization keeping it.",
    caption="<strong>Water immersion: NA 1.35 prints pitches that dry NA 0.93 cannot.</strong> Simulated contrast of "
            "1:1 lines versus pitch for ArF (193.368 nm, conventional σ 0.9, no flare): dry optics at NA 0.93 against "
            "water immersion at NA 1.35 (n = 1.437, <code>OpticsConfig.immersion</code>). The scalar curves show the NA "
            "gain; the vector curves show why 193i needs polarization control — unpolarized light loses much of the "
            "gain, TE (y-) polarized light keeps it. <em>Model: immersion lithography (193i) ✅ · Hopkins imaging "
            "(factorized TCC/SOCS) ✅ · vector (polarized) high-NA imaging ✅; thin mask, no water absorption or "
            "in-resist focus.</em>",
    pages=("optics.md", "README.md"),
    extra={"width": {"README.md": 820}},  # README <picture> width (DESIGN §6.2; INT late-binding edit)
))


# ---------------------------------------------------------------------------
# 5. HHG harmonic comb: exact per-wavelength imaging vs the centre-wavelength TCC
# ---------------------------------------------------------------------------

HHG_BAND = (12.8, 14.4)
HHG_Q = (57, 59, 61)  # odd harmonics of 800 nm inside the passband


def hhg_compute(fast: bool):
    huv = native()
    comb = huv.SourceConfig.hhg(full_comb=True, comb_passband_nm=HHG_BAND)
    optics = huv.OpticsConfig.euv_projection(numerical_aperture=0.33, central_obscuration=0.0, flare=0.0)
    pitches = np.arange(34.0, 48.01, 0.5 if fast else 0.1)
    singles = {q: huv.SourceConfig.hhg(harmonic=q) for q in HHG_Q}
    out = {"pitch": pitches, "mono": [], "poly": [], "multi": [], "sigma": comb.sigma_outer,
           "single": {q: [] for q in HHG_Q}, "lam": {q: singles[q].wavelength_nm for q in HHG_Q}}
    for p in pitches:
        mask = huv.MaskConfig.line_space(p / 2, p)
        grid = mask.commensurate_grid(128, 0.5)
        eng = huv.SimulationEngine(comb, optics, mask, grid=grid, max_kernels=64)
        out["mono"].append(eng.compute_aerial_image(0.0).image_contrast())
        out["poly"].append(eng.compute_polychromatic(0.0).image_contrast())
        out["multi"].append(eng.compute_multiwavelength(0.0).image_contrast())
        for q, src in singles.items():
            e1 = huv.SimulationEngine(src, optics, mask, grid=grid, max_kernels=64)
            out["single"][q].append(e1.compute_aerial_image(0.0).image_contrast())
    for k in ("mono", "poly", "multi"):
        out[k] = np.array(out[k])
    out["single"] = {q: np.array(v) for q, v in out["single"].items()}
    return out


def hhg_draw(d, th: S.Theme):
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 4.3))
    S.style_axes(ax, th)
    x = d["pitch"]
    if not np.allclose(d["mono"], d["poly"]):
        raise AssertionError("narrow-band path should equal the monochromatic image for reflective optics")
    for q in HHG_Q:
        ax.plot(x, d["single"][q], color=th.muted, linewidth=S.HAIR_PT * 1.2, zorder=2)
    ax.plot(x, d["poly"], color=th.s(2), zorder=3)
    ax.plot(x, d["multi"], color=th.s(1), zorder=4)
    lab = d["lam"]
    y61, y57 = d["single"][61], d["single"][57]
    i61 = int(np.argmax(y61 > 0.85))
    ax.annotate(f"q = 61 alone ({lab[61]:.2f} nm)", xy=(x[i61], y61[i61]), xytext=(x[i61] - 0.6, 0.97),
                color=th.ink2, fontsize=S.SMALL_PT, ha="right", va="center",
                arrowprops=dict(arrowstyle="-", color=th.axis, lw=S.HAIR_PT, shrinkA=1, shrinkB=1))
    i57 = int(np.argmax(y57 > 0.55))
    ax.annotate(f"q = 57 alone ({lab[57]:.2f} nm)", xy=(x[i57] + 0.3, y57[i57]), xytext=(x[i57] + 1.6, 0.45),
                color=th.ink2, fontsize=S.SMALL_PT, ha="left", va="center",
                arrowprops=dict(arrowstyle="-", color=th.axis, lw=S.HAIR_PT, shrinkA=1, shrinkB=1))
    ax.set_xlim(x[0], x[-1])
    ax.set_ylim(0, 1.0)
    ax.set_xlabel("Pitch of 1:1 lines and spaces (nm)")
    ax.set_ylabel("Aerial-image contrast")
    ax.set_title("Three-harmonic HHG comb through NA 0.33 EUV projection optics")
    ax.legend([plt.Line2D([], [], color=th.s(1)), plt.Line2D([], [], color=th.s(2)),
               plt.Line2D([], [], color=th.muted, linewidth=S.HAIR_PT * 1.2)],
              ["comb, exact: TCC rebuilt per harmonic",
               "comb, narrow-band: centre-λ TCC + focus shift (= q 59 alone)",
               "one harmonic alone (monochromatic)"],
              loc="lower right")
    return fig


register(FigureSpec(
    name="imaging-hhg-comb",
    group="imaging",
    compute=hhg_compute,
    draw=hhg_draw,
    alt="Contrast versus pitch near the resolution limit for a three-harmonic HHG comb: the exact per-wavelength "
        "curve lies between the single-harmonic curves, while the narrow-band approximation is identical to the "
        "centre harmonic alone.",
    caption="<strong>Honest comb imaging: the exact path images every harmonic, the narrow-band path only the centre "
            "one.</strong> A neon HHG source (800 nm driver) filtered to 12.8–14.4 nm keeps harmonics 57, 59 and 61, each "
            "with its own cutoff pitch λ<sub>q</sub>/(NA(1+σ)) (grey: each harmonic alone); the exact path rebuilds the TCC "
            "at every spectral sample, so the comb image is the weighted sum of the three harmonics' images (q 57 carries "
            "most of the filtered flux) and its contrast lies between the single-harmonic curves. The narrow-band path "
            "keeps the centre-λ kernels and only shifts focus — with reflective optics (no chromatic focus shift) it is "
            "identical to the centre harmonic alone; EUV projection optics NA 0.33, unobscured, no flare, near-coherent "
            "Gaussian fill (σ ≈ 0.05). <em>Model: broadband imaging (<code>compute_multiwavelength</code>) ✅ · narrow-band "
            "polychromatic (<code>compute_polychromatic</code>) 🔶 (valid for Δλ/λ ≪ 1) · high-harmonic generation ✅ "
            "(🔶 power: comb and cutoff exact, driver power assumed).</em>",
    pages=("sources/hhg.md", "pipeline.md"),
    extra={"model_rows": {
        "broadband imaging": "Broadband imaging (`compute_multiwavelength`)",
        "narrow-band polychromatic": "Narrow-band polychromatic (`compute_polychromatic`)",
    }},
))


# ---------------------------------------------------------------------------
# 6. Off-axis illumination: conventional vs annular vs dipole below lambda/NA
# ---------------------------------------------------------------------------

OAI_NA, OAI_PITCH = 0.75, 130.0
OAI_SHAPES = (
    ("conventional", ("conventional", 0.8), "conventional σ 0.8"),
    ("annular", ("annular", 0.5, 0.8), "annular σ 0.5–0.8"),
    ("dipole", ("dipole", 0.65, 0.15, 0.0), "x-dipole, poles at σ 0.65 (r 0.15)"),
)


def oai_compute(fast: bool):
    huv = native()
    optics = huv.OpticsConfig(numerical_aperture=OAI_NA, flare_fraction=0.0)
    mask = huv.MaskConfig.line_space(OAI_PITCH / 2, OAI_PITCH)
    grid = mask.commensurate_grid(128, 1.0)
    pitches = np.arange(110.0, 230.1, 4.0 if fast else 1.0)
    n_fill = 61 if fast else 121
    out = {"pitch": pitches, "shapes": {}}
    for key, spec, _ in OAI_SHAPES:
        src = huv.SourceConfig.f2_laser(0.8).with_illumination(spec)
        eng = huv.SimulationEngine(src, optics, mask, grid=grid, max_kernels=64)
        img = eng.compute_aerial_image(0.0)
        out["x"] = np.asarray(img.x_nm)
        out["shapes"][key] = {
            "fill": np.asarray(src.pupil_fill(n_fill, 1.0)),
            "profile": np.asarray(img.intensity)[np.asarray(img.intensity).shape[0] // 2],
            "contrast": img.image_contrast(),
            "vs_pitch": np.array([_ls_contrast(huv, src, optics, p, pixel=2.0) for p in pitches]),
        }
    return out


def oai_draw(d, th: S.Theme):
    from matplotlib.patches import Circle

    fig = plt.figure(figsize=(S.WIDE_WIDTH, 6.4))
    gs = GridSpec(2, 3, figure=fig, height_ratios=[1, 1.25], hspace=0.42, wspace=0.3)
    cmap = S.field_cmap()
    for k, (key, _spec, label) in enumerate(OAI_SHAPES):
        ax = fig.add_subplot(gs[0, k])
        fill = d["shapes"][key]["fill"]
        im = ax.imshow(fill, origin="lower", extent=[-1, 1, -1, 1], cmap=cmap, vmin=0,
                       vmax=max(float(fill.max()), 1e-12), interpolation="nearest")
        im.set_clip_path(Circle((0, 0), 1.0, transform=ax.transData))
        ax.add_patch(Circle((0, 0), 1.0, fill=False, edgecolor=th.axis, linewidth=S.HAIR_PT))
        ax.set_xlim(-1.05, 1.05)
        ax.set_ylim(-1.05, 1.05)
        ax.set_aspect("equal")
        ax.set_xticks([])
        ax.set_yticks([])
        for side in ("top", "right", "left", "bottom"):
            ax.spines[side].set_visible(False)
        ax.set_title(f"({'abc'[k]}) {label}", fontsize=S.SMALL_PT + 0.5)
    # cross-sections at the 130 nm pitch: two periods
    ax = fig.add_subplot(gs[1, 0:2])
    S.style_axes(ax, th, grid="both")
    x = d["x"]
    period = float(len(x) * (x[1] - x[0]))
    xx = np.concatenate([x - period / 2, x + period / 2])
    for slot, (key, _spec, label) in enumerate(OAI_SHAPES, start=1):
        prof = np.roll(d["shapes"][key]["profile"], len(x) // 2)
        ax.plot(xx, np.tile(prof, 2), color=th.s(slot), zorder=3,
                label=f"{label}: contrast {d['shapes'][key]['contrast']:.2f}")
    ax.set_xlim(xx[0], xx[-1])
    ax.set_ylim(0, None)
    ax.set_xlabel("x (nm)")
    ax.set_ylabel("Relative intensity")
    ax.set_title(f"(d) Aerial image of {OAI_PITCH / 2:.0f} nm lines, {OAI_PITCH:.0f} nm pitch (k₁ = "
                 f"{OAI_PITCH / 2 * OAI_NA / F2_NM:.2f})".replace("₁", "$_1$"))
    ax.legend(loc="upper center", bbox_to_anchor=(0.5, -0.17), ncol=1, fontsize=S.SMALL_PT)
    ax2 = fig.add_subplot(gs[1, 2])
    S.style_axes(ax2, th, grid="both")
    for slot, (key, _spec, _label) in enumerate(OAI_SHAPES, start=1):
        ax2.plot(d["pitch"], d["shapes"][key]["vs_pitch"], color=th.s(slot), zorder=3)
    S.limit_line(ax2, F2_NM / OAI_NA, th, "λ/NA", where=0.03, va="bottom", side="right")
    S.limit_line(ax2, F2_NM / (OAI_NA * 1.8), th, "λ/(NA·1.8)", where=0.97, va="top", side="right")
    S.limit_line(ax2, OAI_PITCH, th, None)
    ax2.set_xlim(d["pitch"][0], d["pitch"][-1])
    ax2.set_ylim(0, 1.0)
    ax2.set_xlabel("Pitch (nm)")
    ax2.set_ylabel("Contrast")
    ax2.set_title("(e) Contrast vs pitch")
    fig.subplots_adjust(bottom=0.2)
    return fig


register(FigureSpec(
    name="imaging-illumination-shapes",
    group="imaging",
    compute=oai_compute,
    draw=oai_draw,
    alt="Three pupil fills (a filled disk, a ring and two poles on the x axis), the aerial-image cross-sections "
        "they give for 65 nm lines on a 130 nm pitch, below the coherent limit lambda/NA of 210 nm, with the "
        "dipole giving the deepest modulation, and contrast versus pitch for the three fills.",
    caption="<strong>Off-axis illumination below λ/NA.</strong> F<sub>2</sub> 157.63 nm, NA 0.75, 1:1 lines on a "
            "130 nm pitch (k<sub>1</sub> ≈ 0.31; λ/NA = 210 nm): (a–c) pupil fills of the source (circle = NA), all with "
            "outer σ ≤ 0.8 so they share the λ/(NA·1.8) ≈ 117 nm cutoff; (d) aerial-image cross-sections at best focus — "
            "moving the light to the pupil edge in the direction of the grating (annular, and most of all the x-dipole) "
            "puts more of it into two-beam (0, ±1) imaging and raises the contrast. (e) The same fills across pitch: the "
            "dipole gives the highest contrast here, but only for gratings of this orientation — the same pattern rotated "
            "by 90° does not get its off-axis benefit. <em>Model: Hopkins imaging (factorized TCC/SOCS) ✅ · illumination "
            "pupil fills ✅; no flare, thin mask, no resist.</em>",
    pages=("pipeline.md", "playground/index.md", "README.md"),
    requires=("illumination", "pupil_fill"),
    extra={"width": {"README.md": 280}},
))


# ===========================================================================
# DESIGN §5.2 — the Physics-Strip engine figures (I3, I4, I6b, I6c, I8)
# ===========================================================================

#: The canonical run's scene (examples/sim.toml, getting-started example 1): 65 nm lines on a 180 nm pitch,
#: F₂ 157.63 nm, NA 0.75, σ 0.7. The anatomy and focus figures use it without flare (stated in their captions).
CANON_CD, CANON_PITCH, CANON_NA, CANON_SIGMA = 65.0, 180.0, 0.75, 0.7
#: x-dipole for the canonical pitch: pole centre at σ_c = λ/(2 p NA) so the 0 and −1 orders sit symmetrically.
CANON_DIPOLE_SIGMA_C = round(F2_NM / (2 * CANON_PITCH * CANON_NA), 3)
CANON_DIPOLE_RADIUS = 0.15


def _pupil_axes(ax, th: S.Theme, *, sigma: float | None = None, support: float | None = None, extent: float = 2.0,
                ticks: bool = True) -> dict:
    """The D5 pupil grammar on a matplotlib axes: pupil circle (radius 1 = NA/λ), the conventional source disc
    of radius σ, the dashed (1+σ)·NA/λ support circle. Returns the theme's pupil palette."""
    from matplotlib.patches import Circle  # noqa: PLC0415

    import tokens  # noqa: PLC0415

    pal = tokens.theme(th.name)["pupil"]
    stroke = tokens.theme(th.name)["pupil_stroke"]
    ax.add_patch(Circle((0, 0), 1.0, facecolor=pal["fill"], edgecolor=stroke["fill"], linewidth=S.HAIR_PT * 1.4, zorder=1))
    if sigma:
        ax.add_patch(Circle((0, 0), sigma, facecolor=pal["coherent_core"], edgecolor="none", zorder=1.2))
    if support:
        ax.add_patch(Circle((0, 0), support, facecolor="none", edgecolor=pal["support"], linestyle=(0, (4, 3)),
                            linewidth=S.HAIR_PT * 1.3, zorder=1.1))
    ax.set_aspect("equal")
    ax.set_xlim(-extent, extent)
    ax.set_ylim(-extent, extent)
    for side in ("top", "right", "left", "bottom"):
        ax.spines[side].set_visible(False)
    if ticks:
        ax.set_xticks([-2, -1, 0, 1, 2])
        ax.set_yticks([-2, -1, 0, 1, 2])
        ax.tick_params(colors=th.axis, labelcolor=th.ink2, labelsize=S.SMALL_PT, length=2.5)
    else:
        ax.set_xticks([])
        ax.set_yticks([])
    return pal


def _crop_centre(a: np.ndarray, half: int) -> np.ndarray:
    """fftshift ``a`` and keep ``half`` samples either side of the zero frequency."""
    c = a.shape[0] // 2
    s = np.fft.fftshift(a)
    return s[c - half: c + half + 1, c - half: c + half + 1]


def _socs_engines(huv, *, fast: bool, dipole: bool = False, model: str = "exact"):
    """The canonical scene on a multi-period field (a dense frequency grid, so the kernels are smooth maps)."""
    periods, size, max_kernels = (8, 256, 24) if fast else (16, 512, 64)
    src = huv.SourceConfig.f2_laser(CANON_SIGMA)
    if dipole:
        src = src.with_illumination("dipole", CANON_DIPOLE_SIGMA_C, CANON_DIPOLE_RADIUS, 0.0)
    optics = huv.OpticsConfig(numerical_aperture=CANON_NA, flare_fraction=0.0)
    mask = huv.MaskConfig.line_space(CANON_CD, CANON_PITCH)
    grid = mask.commensurate_grid(size, periods * CANON_PITCH / size)
    if abs(grid.periods_in_field(CANON_PITCH) - periods) > 1e-9:
        raise AssertionError(f"field holds {grid.periods_in_field(CANON_PITCH)} periods, expected {periods}")
    eng = huv.SimulationEngine(src, optics, mask, grid=grid, max_kernels=max_kernels, defocus_model=model)
    return src, optics, mask, grid, eng


def _partial_sums(lam, ker, M, cf, levels):
    """Cross-sections (row y = 0) of Σ_{k≤K} λ_k |F⁻¹[u_k M]|² / clear field for each K in ``levels``."""
    out = {}
    acc = np.zeros(M.shape)
    for k in range(len(lam)):
        acc += lam[k] * np.abs(np.fft.ifft2(ker[k] * M)) ** 2
        if (k + 1) in levels:
            out[k + 1] = acc[acc.shape[0] // 2] / cf
    return out, acc / cf


# ---------------------------------------------------------------------------
# 7. I3 — SOCS anatomy: source points, mask orders, kernels, partial sums
# ---------------------------------------------------------------------------


def socs_compute(fast: bool):
    huv = native()
    src, optics, mask, grid, eng = _socs_engines(huv, fast=fast)
    src_d, _o, _m, _g, eng_d = _socs_engines(huv, fast=fast, dipole=True)
    n = grid.size
    unit = F2_NM / CANON_NA  # one pupil unit in nm⁻¹ terms: f (cycles/nm) × λ/NA
    step = unit / grid.field_size_nm()  # frequency-grid step in pupil units
    # (a) the engine's own source sampling
    points = {}
    for key, e in (("conventional", eng), ("dipole", eng_d)):
        sx, sy, w = e.source_points()
        if abs(float(np.sum(w)) - 1.0) > 1e-9:
            raise AssertionError(f"{key}: source weights sum to {np.sum(w)}, not 1")
        points[key] = {"sx": np.asarray(sx), "sy": np.asarray(sy), "w": np.asarray(w)}
    # (b) the exact mask spectrum: harmonics at m/p, every other sample exactly zero
    M = mask.spectrum(grid)
    fx = np.fft.fftfreq(n, d=grid.pixel_nm) * unit  # pupil units
    row = np.abs(M[0])  # fy = 0
    harm = int(round(grid.periods_in_field(CANON_PITCH)))  # the m-th order sits at sample m·harm
    orders = {}
    for m in range(-6, 7):
        orders[m] = float(row[(m * harm) % n])
    off = np.ones(n, dtype=bool)
    off[::harm] = False
    if np.abs(row[off]).max() != 0.0 or np.abs(M[1:]).max() != 0.0:
        raise AssertionError("off-harmonic samples of the exact line/space spectrum are not exactly zero")
    # (c) kernels and the captured-energy staircase
    lam, ker = eng.kernels(0.0)
    diag = eng.imaging_diagnostics(0.0)
    cf = eng.clear_field_intensity(0.0)
    for k in range(min(4, len(lam))):
        norm = float(np.sum(np.abs(ker[k]) ** 2))
        if abs(norm - 1.0) > 1e-9:
            raise AssertionError(f"kernel {k + 1} is not unit-norm in the frequency domain ({norm})")
    total = float(np.sum(lam)) / diag["captured_energy_fraction"]
    staircase = np.cumsum(lam) / total
    half = int(np.ceil(2.2 / step))
    tiles = [np.abs(_crop_centre(ker[k], half)).astype(np.float32) for k in range(min(4, len(lam)))]
    # (d) partial sums; the full sum must reproduce the engine's image
    levels = (1, 2, 4, len(lam))
    cuts, full = _partial_sums(lam, ker, M, cf, levels)
    img = eng.compute_aerial_image(0.0)
    err = float(np.abs(full - np.asarray(img.intensity)).max())
    if err > 1e-9:
        raise AssertionError(f"all-kernel partial sum differs from compute_aerial_image by {err:.2e} (> 1e-9)")
    x = np.asarray(img.x_nm)
    keep = np.abs(x) <= CANON_PITCH
    return {
        "k1": CANON_CD * CANON_NA / F2_NM, "sigma": CANON_SIGMA, "sigma_c": CANON_DIPOLE_SIGMA_C, "step": step,
        "points": points, "orders": orders, "order_unit": unit / CANON_PITCH, "support": 1 + CANON_SIGMA,
        "eig": np.asarray(lam), "staircase": staircase, "captured": diag["captured_energy_fraction"],
        "num_kernels": int(diag["num_kernels"]), "num_frequencies": int(diag["num_frequencies"]), "method": diag["method"],
        "tiles": tiles, "tile_extent": half * step,
        "x": x[keep], "cuts": {K: v[keep] for K, v in cuts.items()}, "levels": levels, "contrast": img.image_contrast(),
        "recon_err": err, "field_nm": grid.field_size_nm(), "pixel_nm": grid.pixel_nm, "size": n,
    }


def socs_draw(d, th: S.Theme):
    from matplotlib.patches import Rectangle  # noqa: PLC0415

    fig = plt.figure(figsize=(S.FULL_WIDTH, 8.2))
    gs = GridSpec(3, 2, figure=fig, height_ratios=[1.0, 0.8, 0.9], hspace=0.55, wspace=0.22, left=0.07, right=0.985,
                  top=0.95, bottom=0.1)
    cmap = S.field_cmap()
    # --- (a) source points -------------------------------------------------------------------------------
    ga = gs[0, 0].subgridspec(1, 2, wspace=0.1)
    for k, (key, title) in enumerate((("conventional", f"conventional σ {d['sigma']}"), ("dipole", f"x-dipole, poles at σ {d['sigma_c']} (r {CANON_DIPOLE_RADIUS})"))):
        ax = fig.add_subplot(ga[0, k])
        pal = _pupil_axes(ax, th, extent=1.25, ticks=False)
        pts = d["points"][key]
        w = pts["w"] / pts["w"].max()
        ax.scatter(pts["sx"], pts["sy"], s=1.0 + 6.0 * w, c=th.s(1), edgecolors="none", zorder=3, antialiased=False)
        ax.text(0.5, -0.02, f"{title}\n{len(w):,} points".replace(",", " "), transform=ax.transAxes, ha="center", va="top",
                color=th.ink2, fontsize=S.SMALL_PT, linespacing=1.4)
        if k == 0:
            ax.text(0.0, 1.04, "(a) the source points the engine integrates over", color=th.ink, fontsize=S.TITLE_PT,
                    fontweight="semibold", ha="left", va="bottom", transform=ax.transAxes)
            ax.text(0.0, -0.3, "circle: pupil (radius NA/λ); marker size ∝ weight", color=th.muted, fontsize=S.SMALL_PT,
                    ha="left", va="top", transform=ax.transAxes)
    # --- (b) mask orders in the pupil ------------------------------------------------------------------------
    ax = fig.add_subplot(gs[0, 1])
    pal = _pupil_axes(ax, th, sigma=d["sigma"], support=d["support"], extent=2.6, ticks=False)
    o = d["orders"]
    scale = 1.3 / o[0]
    colours = {0: pal["order0"], 1: pal["order1"], 2: pal["order2"]}
    for m, amp in sorted(o.items()):
        x = m * d["order_unit"]
        if abs(x) > 2.55:
            continue
        inside = abs(x) <= d["support"]
        col = colours.get(abs(m), pal["outside"]) if inside else pal["outside"]
        ax.plot([x, x], [0, amp * scale], color=col, linewidth=S.LINE_PT * 1.2, solid_capstyle="butt", zorder=3)
        ax.plot([x], [amp * scale], marker="o", markersize=S.MARKER_PT * 0.75, color=col, markeredgecolor=th.surface,
                markeredgewidth=S.RING_PT, linestyle="none", zorder=4)
        if abs(m) <= 2:
            ax.text(x, amp * scale + 0.1, f"m = {m:+d}" if m else "m = 0", ha="center", va="bottom", color=th.ink2,
                    fontsize=S.SMALL_PT)
    # the off-harmonic samples: every grid frequency between the orders is exactly zero
    xs = np.arange(-2.5, 2.5001, d["step"])
    ax.plot(xs, np.zeros_like(xs), linestyle="none", marker="|", markersize=4, color=th.axis, zorder=2)
    ax.axhline(0, color=th.axis, linewidth=S.HAIR_PT, zorder=1.5)
    ax.text(0, -1.9, "every sample between the orders: exactly 0", ha="center", va="top", color=th.muted, fontsize=S.SMALL_PT)
    ax.text(1.0, -0.3, "pupil\nNA/λ", ha="center", va="top", color=pal["support"], fontsize=S.SMALL_PT, linespacing=1.3)
    ax.text(1.25, 1.3, "support\n(1+σ)·NA/λ", ha="left", va="bottom", color=pal["support"], fontsize=S.SMALL_PT, linespacing=1.3)
    ax.text(-2.6, 2.62, "(b) the exact mask spectrum |M(f)|: orders at m/p", color=th.ink, fontsize=S.TITLE_PT,
            fontweight="semibold", ha="left", va="bottom")
    ax.text(-2.6, -2.3, f"{CANON_CD:.0f} nm lines, {CANON_PITCH:.0f} nm pitch; |m| ≥ 2 lies outside the support and cannot image",
            color=th.muted, fontsize=S.SMALL_PT, ha="left", va="top")
    # --- (c) kernels + staircase ----------------------------------------------------------------------------------
    gc = gs[1, :].subgridspec(1, 6, wspace=0.18, width_ratios=[1, 1, 1, 1, 0.3, 1.9])
    ext = d["tile_extent"]
    for k, tile in enumerate(d["tiles"]):
        ax = fig.add_subplot(gc[0, k])
        ax.imshow(tile / tile.max(), origin="lower", extent=[-ext, ext, -ext, ext], cmap=cmap, vmin=0, vmax=1,
                  interpolation="bilinear")
        from matplotlib.patches import Circle  # noqa: PLC0415

        ax.add_patch(Circle((0, 0), 1.0, fill=False, edgecolor="#ffffff", linewidth=S.HAIR_PT, alpha=0.55))
        ax.set_xlim(-ext, ext)
        ax.set_ylim(-ext, ext)
        ax.set_aspect("equal")
        ax.set_xticks([])
        ax.set_yticks([])
        S.image_axes(ax, th)
        frac = d["eig"][k] / np.sum(d["eig"]) * d["captured"]
        ax.set_title(rf"$|u_{k + 1}|$   λ$_{k + 1}$ = {frac:.3f}", fontsize=S.SMALL_PT + 0.5)
        if k == 0:
            ax.text(0, 1.42, "(c) SOCS kernels (frequency domain, unit norm) and the energy they capture", color=th.ink,
                    fontsize=S.TITLE_PT, fontweight="semibold", ha="left", va="bottom", transform=ax.transAxes)
    ax = fig.add_subplot(gc[0, 5])
    S.style_axes(ax, th)
    kk = np.arange(1, len(d["staircase"]) + 1)
    ax.step(np.concatenate([[0], kk]), np.concatenate([[0], d["staircase"]]), where="post", color=th.s(1), zorder=3)
    for K in d["levels"][:3]:
        ax.plot([K], [d["staircase"][K - 1]], marker="o", markersize=S.MARKER_PT * 0.8, color=th.s(1),
                markeredgecolor=th.surface, markeredgewidth=S.RING_PT, linestyle="none", zorder=4)
        ax.text(K + 0.6, d["staircase"][K - 1] - 0.02, f"K = {K}: {d['staircase'][K - 1]:.1%}", ha="left", va="top", color=th.ink2,
                fontsize=S.SMALL_PT)
    ax.axhline(1.0, color=th.axis, linewidth=S.HAIR_PT, linestyle=(0, (4, 3)))
    ax.set_xlim(0, len(kk))
    ax.set_ylim(0, 1.02)
    ax.set_xlabel("kernels kept, K")
    ax.set_title(f"captured TCC energy — truncated at K = {d['num_kernels']}:\n{d['captured']:.1%}, from {d['num_frequencies']} "
                 f"support frequencies ({d['method']} eigensolver)", fontsize=S.SMALL_PT + 0.5, linespacing=1.3)
    # --- (d) partial sums -----------------------------------------------------------------------------------------
    ax = fig.add_subplot(gs[2, :])
    S.style_axes(ax, th, grid="both")
    ramp = S.ordinal(th, 3)
    x = d["x"]
    labels = []
    for j, K in enumerate(d["levels"][:3]):
        ax.plot(x, d["cuts"][K], color=ramp[j], zorder=3)
        labels.append(f"K = {K}  ({d['staircase'][K - 1]:.0%} of the energy)")
    K_all = d["levels"][3]
    ax.plot(x, d["cuts"][K_all], color=th.ink, linewidth=S.LINE_PT * 1.1, zorder=4)
    labels.append(f"all {K_all} kernels = compute_aerial_image (max difference {d['recon_err']:.0e})")
    ax.set_xlim(x[0], x[-1])
    ax.set_ylim(0, 0.8)
    ax.set_xlabel("x (nm)")
    ax.set_ylabel("relative intensity")
    ax.set_title(f"(d) the aerial image as a partial sum over kernels — contrast {d['contrast']:.3f}")
    ax.legend([plt.Line2D([], [], color=c) for c in ramp] + [plt.Line2D([], [], color=th.ink)], labels, loc="upper center",
              bbox_to_anchor=(0.5, -0.2), ncol=2, fontsize=S.SMALL_PT)
    ax.add_patch(Rectangle((-CANON_CD / 2, 0), CANON_CD, 10, facecolor=th.band, edgecolor="none", zorder=0.5))
    ax.text(0, 0.97, f"mask line, {CANON_CD:.0f} nm", ha="center", va="top", color=th.muted, fontsize=S.SMALL_PT,
            transform=ax.get_xaxis_transform())
    return fig


register(FigureSpec(
    name="imaging-socs-anatomy",
    group="imaging",
    compute=socs_compute,
    draw=socs_draw,
    alt="Four panels: the engine's source-point lattices for a conventional and a dipole fill, the exact mask orders of a "
        "65 nm / 180 nm grating inside the pupil, the first four SOCS kernels with their captured-energy staircase, and "
        "the aerial cross-section converging as kernels are added.",
    caption="<strong>How one image is built: source points, mask orders, SOCS kernels, partial sums.</strong> 65 nm "
            "lines on a 180 nm pitch, F<sub>2</sub> 157.63 nm, NA 0.75, σ 0.7 — the canonical run's scene without flare; "
            "kernels are unit-norm in the frequency domain and the staircase shows the truncation; the full sum reproduces "
            "<code>compute_aerial_image</code> to 1e-9. <em>Model: Hopkins imaging (factorized TCC/SOCS) ✅ · exact thin-mask "
            "spectrum ✅ · illumination pupil fills ✅; thin mask, no flare, no resist.</em>",
    pages=("pipeline.md", "architecture.md", "README.md"),
    requires=("illumination",),
    extra={"width": {"README.md": 820}},  # README <picture> width (DESIGN §6.2; INT late-binding edit)
))


# ---------------------------------------------------------------------------
# 8. I4 — focus changes the kernels: exact model vs the legacy KernelPhase, animated
# ---------------------------------------------------------------------------

FOCUS_PLANES = 19  # −400 … +400 nm in 44 nm steps; the yoyo sequence has 2·19 − 2 = 36 frames (the APNG budget, §5.2 I4)
FOCUS_RANGE_NM = 400.0


def socs_focus_frames_compute(fast: bool):
    huv = native()
    import common  # noqa: PLC0415

    planes = FOCUS_PLANES
    if fast:
        planes = 7
    if common.FRAME_LIMIT:
        planes = min(planes, max(2, (common.FRAME_LIMIT + 2) // 2))
    z = np.linspace(-FOCUS_RANGE_NM, FOCUS_RANGE_NM, planes)
    z_curve = np.linspace(-FOCUS_RANGE_NM, FOCUS_RANGE_NM, 41 if not fast else 11)
    out = {"z": z, "z_curve": z_curve, "models": {}, "sigma_c": CANON_DIPOLE_SIGMA_C, "k1": CANON_CD * CANON_NA / F2_NM}
    ref = {}
    for model in ("exact", "kernel_phase"):
        _s, _o, mask, grid, eng = _socs_engines(huv, fast=True, dipole=True, model=model)  # the dense-but-small grid
        _s, _o, _m, _g, eng_c = _socs_engines(huv, fast=True, dipole=False, model=model)
        n = grid.size
        unit = F2_NM / CANON_NA
        step = unit / grid.field_size_nm()
        half = int(np.ceil(2.0 / step))
        x = np.asarray(eng.compute_aerial_image(0.0).x_nm)
        win = np.abs(x) <= CANON_PITCH
        frames = []
        k0 = None
        for zi in z:
            lam, ker = eng.kernels(float(zi))
            tiles = [np.abs(_crop_centre(ker[k], half)).astype(np.float32) for k in range(min(4, len(lam)))]
            if model == "kernel_phase":
                if k0 is None:
                    k0 = tiles
                elif max(float(np.abs(a - b).max()) for a, b in zip(tiles, k0)) > 1e-6:
                    raise AssertionError("KernelPhase changed a kernel magnitude with focus; it should only add a phase")
            img = eng.compute_aerial_image(float(zi))
            inten = np.asarray(img.intensity)
            frames.append({
                "z": float(zi), "tiles": tiles, "eig": np.asarray(lam[:4]) / float(np.sum(lam)),
                "num_kernels": len(lam), "image": inten[np.ix_(win, win)].astype(np.float32), "contrast": img.image_contrast(),
            })
        ims = eng.compute_through_focus([float(v) for v in z_curve])
        ims_c = eng_c.compute_through_focus([float(v) for v in z_curve])
        out["models"][model] = {
            "frames": frames, "tile_extent": half * step, "window_nm": CANON_PITCH,
            "curve_dipole": np.array([i.image_contrast() for i in ims]),
            "curve_conventional": np.array([i.image_contrast() for i in ims_c]),
        }
        ref[model] = np.asarray(eng.compute_aerial_image(0.0).intensity)
    diff0 = float(np.abs(ref["exact"] - ref["kernel_phase"]).max())
    if diff0 > 1e-9:
        raise AssertionError(f"the two defocus models differ at z = 0 by {diff0:.2e}; they must agree in focus")
    out["diff_at_focus"] = diff0
    return out


def _focus_frame(d, th: S.Theme, i: int):
    fig = plt.figure(figsize=(8.0, 4.0))
    gs = GridSpec(2, 7, figure=fig, width_ratios=[0.58, 0.58, 0.58, 0.58, 1.05, 0.12, 1.75], wspace=0.12, hspace=0.46,
                  left=0.012, right=0.93, top=0.85, bottom=0.12)
    cmap = S.field_cmap()
    zc = d["z"][i]
    rows = (("exact", "exact defocus (default): kernels rebuilt at every plane", th.s(1)),
            ("kernel_phase", r"legacy kernel_phase: in-focus kernels × $e^{\,i\pi z\lambda|f|^2}$", th.s(2)))
    for r, (model, title, colour) in enumerate(rows):
        m = d["models"][model]
        fr = m["frames"][i]
        ext = m["tile_extent"]
        for k in range(4):
            ax = fig.add_subplot(gs[r, k])
            if k < len(fr["tiles"]):
                tile = fr["tiles"][k]
                ax.imshow(tile / max(float(tile.max()), 1e-12), origin="lower", extent=[-ext, ext, -ext, ext], cmap=cmap,
                          vmin=0, vmax=1, interpolation="bilinear")
                ax.text(0.5, -0.06, rf"λ$_{k + 1}$ = {fr['eig'][k]:.2f}", transform=ax.transAxes, ha="center", va="top",
                        color=th.ink2, fontsize=S.SMALL_PT)
            ax.set_xlim(-ext, ext)
            ax.set_ylim(-ext, ext)
            ax.set_aspect("equal")
            ax.set_xticks([])
            ax.set_yticks([])
            S.image_axes(ax, th)
            if k == 0:
                ax.text(0, 1.06, title, transform=ax.transAxes, ha="left", va="bottom", color=th.ink, fontsize=S.BASE_PT,
                        fontweight="semibold")
        ax = fig.add_subplot(gs[r, 4])
        w = m["window_nm"]
        ax.imshow(fr["image"], origin="lower", extent=[-w, w, -w, w], cmap=cmap, vmin=0, vmax=1.0, interpolation="bilinear")
        ax.set_xticks([])
        ax.set_yticks([])
        S.image_axes(ax, th)
        ax.text(0.5, -0.06, f"image · contrast {fr['contrast']:.2f}", transform=ax.transAxes, ha="center", va="top",
                color=th.ink2, fontsize=S.SMALL_PT)
        ax = fig.add_subplot(gs[r, 6])
        S.style_axes(ax, th)
        ax.plot(d["z_curve"], m["curve_dipole"], color=colour, zorder=3)
        ax.plot(d["z_curve"], m["curve_conventional"], color=th.muted, linewidth=S.HAIR_PT * 1.4, zorder=2)
        ax.axvline(zc, color=th.axis, linewidth=S.HAIR_PT, zorder=1.5)
        cz = float(np.interp(zc, d["z_curve"], m["curve_dipole"]))
        ax.plot([zc], [cz], marker="o", markersize=S.MARKER_PT, color=colour, markeredgecolor=th.surface,
                markeredgewidth=S.RING_PT, linestyle="none", zorder=4, clip_on=False)
        cc = float(np.interp(zc, d["z_curve"], m["curve_conventional"]))
        ax.plot([zc], [cc], marker="o", markersize=S.MARKER_PT * 0.7, color=th.muted, markeredgecolor=th.surface,
                markeredgewidth=S.RING_PT, linestyle="none", zorder=4, clip_on=False)
        ax.set_xlim(-FOCUS_RANGE_NM, FOCUS_RANGE_NM)
        ax.set_ylim(0, 1.0)
        ax.set_yticks([0, 0.5, 1.0])
        ax.tick_params(labelsize=S.SMALL_PT)
        if r == 1:
            ax.set_xlabel("focus z (nm)", fontsize=S.SMALL_PT)
        else:
            ax.tick_params(labelbottom=False)
        ax.set_title("contrast vs focus", fontsize=S.SMALL_PT, loc="left", pad=3)
        S.end_label(ax, FOCUS_RANGE_NM, float(m["curve_dipole"][-1]), "x-dipole", th, dx_px=5, color=colour)
        S.end_label(ax, FOCUS_RANGE_NM, float(m["curve_conventional"][-1]), f"σ {CANON_SIGMA}", th, dx_px=5)
    fig.text(0.985, 0.975, f"z = {zc:+.0f} nm", ha="right", va="top", color=th.ink, fontsize=S.TITLE_PT, fontweight="semibold")
    fig.text(0.012, 0.975, f"{CANON_CD:.0f} nm lines, {CANON_PITCH:.0f} nm pitch · F₂ 157.63 nm · NA {CANON_NA} · x-dipole σ$_c$ {d['sigma_c']}"
             .replace("F₂", "F$_2$"), ha="left", va="top", color=th.ink2, fontsize=S.SMALL_PT)
    return fig


def socs_focus_draw(d, th: S.Theme):
    n = len(d["z"])
    order = list(range(n)) + list(range(n - 2, 0, -1))  # yoyo: 0 … n−1 … 1
    return [_focus_frame(d, th, i) for i in order]


register(FigureSpec(
    name="socs-focus-anim",
    group="imaging",
    subdir="anim",
    fmt="apng",
    still_frame=FOCUS_PLANES // 2,
    compute=socs_focus_frames_compute,
    draw=socs_focus_draw,
    alt="Animation through focus from -400 to +400 nm: the exact model's SOCS kernels change shape at every plane and the "
        "dipole image keeps its contrast, while the legacy kernel-phase model keeps its in-focus kernels and its contrast "
        "collapses and revives; a marker tracks contrast versus focus for both.",
    caption="<strong>Focus changes the kernels, not just the image.</strong> 65 nm lines on a 180 nm pitch, F<sub>2</sub> "
            "157.63 nm, NA 0.75, x-dipole (σ<sub>c</sub> ≈ 0.58), −400 to +400 nm: the exact model rebuilds the kernel set "
            "at every plane and the dipole keeps most of its contrast; the legacy KernelPhase model multiplies the in-focus "
            "kernels by one on-axis phase per mask frequency and its contrast collapses and revives (thin grey: conventional "
            "σ 0.7). <em>Model: defocus / through-focus imaging ✅ (exact phase in the pupil per source point; 🔶 legacy "
            "KernelPhase, kept for comparison).</em>",
    pages=("pipeline.md",),
    requires=("illumination",),
    extra={"duration_ms": 120, "max_kb": 800, "max_kb_reason": "DESIGN §5.2 I4 budgets this APNG at 800 KB, below the 900 KB kind cap"},
))


# ---------------------------------------------------------------------------
# 9. I6b — four pupils at one Rayleigh unit of defocus (+ an opt-in multilayer pupil)
# ---------------------------------------------------------------------------

PUPIL_N = 129
Z9_WAVES = 0.05
#: Illustrative incidence-angle maps for the six mirrors of an NXE-class projection optic (centre, tilt, azimuth,
#: radial — degrees). User assumptions, not a ray trace of any design: the matrix grades the multilayer pupil 🔶.
MULTILAYER_MIRRORS = ((4.0, 1.5, 0.0, 0.8), (6.0, 2.0, 90.0, 1.0), (8.0, 1.5, 0.0, 0.8), (5.0, 1.5, 90.0, 0.8),
                      (7.0, 2.0, 0.0, 1.0), (6.0, 1.5, 90.0, 0.8))
#: Crop box (x, y, w, h) in pixels of the 2× PNG for the High-NA column (measured; draw() re-measures and refuses to
#: write if the layout moved by more than FOUR_PUPILS_CROP_TOL px).
FOUR_PUPILS_CROP_BOX = (399, 92, 338, 823)
FOUR_PUPILS_CROP_TOL = 6


def _defocus_phase_exact(na: float, index: float, wavelength_nm: float, z_nm: float, rho: np.ndarray) -> np.ndarray:
    s = np.clip(na * rho / index, -1.0, 1.0)
    return 2 * np.pi * index / wavelength_nm * z_nm * (1 - np.sqrt(1 - s * s))


def _pupil_entry(key, title, optics, wavelength_nm, n, *, note=""):
    na = optics.numerical_aperture
    z = wavelength_nm / na ** 2
    P = np.asarray(optics.pupil_map(wavelength_nm, n=n, focus_nm=z))
    rho = np.linspace(-1, 1, n)
    RX, RY = np.meshgrid(rho, rho)
    R = np.hypot(RX, RY)
    inside = np.abs(P) > 0
    # a piston (the phase at the pupil centre, or at the innermost sample of an obscured pupil) changes no image
    ref = P[inside][np.argmin(R[inside])]
    rel = np.where(inside, np.angle(P * np.conj(ref) / abs(ref)), np.nan)
    return {"key": key, "title": title, "na": na, "index": optics.immersion_index, "obscuration": optics.central_obscuration,
            "wavelength_nm": wavelength_nm, "z_nm": z, "amp": np.abs(P), "phase": np.where(inside, np.angle(P), np.nan),
            "phase_rel": rel, "piston": float(np.angle(ref)), "inside": inside, "R": R, "note": note, "kind": optics.kind}


def four_pupils_compute(fast: bool):
    huv = native()
    n = 65 if fast else PUPIL_N
    rho = np.linspace(-1, 1, n)
    RX, RY = np.meshgrid(rho, rho)
    R = np.hypot(RX, RY)
    # (1) refractive NA 0.75 with 0.05 waves of Fringe Z9 (primary spherical); verify the convention against the plain pupil
    plain = huv.OpticsConfig(numerical_aperture=0.75, flare_fraction=0.0)
    ab = huv.OpticsConfig(numerical_aperture=0.75, flare_fraction=0.0)
    ab.add_aberration(9, Z9_WAVES)
    P0 = np.asarray(plain.pupil_map(F2_NM, n=n, focus_nm=0.0))
    P9 = np.asarray(ab.pupil_map(F2_NM, n=n, focus_nm=0.0))
    inside = np.abs(P0) > 0
    extra = np.angle(P9[inside] / P0[inside])
    z9 = (6 * R ** 4 - 6 * R ** 2 + 1)[inside]  # Fringe Z9, University of Arizona normalisation
    coeff = float(np.sum(extra * z9) / np.sum(z9 * z9))
    resid = float(np.abs(extra - coeff * z9).max())
    if abs(coeff - 2 * np.pi * Z9_WAVES) > 1e-9 or resid > 1e-9:
        raise AssertionError(f"add_aberration(9, {Z9_WAVES}) is not 2π·{Z9_WAVES}·(6ρ⁴−6ρ²+1): coeff {coeff:.6f}, residual {resid:.1e}")
    entries = [
        _pupil_entry("refractive", f"refractive, NA 0.75\n+ Z9 spherical {Z9_WAVES} λ", ab, F2_NM, n),
        _pupil_entry("high_na", "High-NA EUV, NA 0.55\n0.2·NA obscuration (assumed)", huv.OpticsConfig.euv_high_na(), 13.5, n),
        _pupil_entry("schwarzschild", "Schwarzschild, NA 0.33\n0.25 obscuration, R 0.67", huv.OpticsConfig.schwarzschild(numerical_aperture=0.33, obscuration_ratio=0.25), 13.5, n),
        _pupil_entry("zone_plate", "zone plate, NA 0.34\nΔr = 20 nm, first order", huv.OpticsConfig.zone_plate(20.0, 13.5), 13.5, n),
    ]
    # the refractive pupil's phase must be Z9 + the exact defocus law; the others carry the exact defocus law alone
    for e in entries:
        law = _defocus_phase_exact(e["na"], e["index"], e["wavelength_nm"], e["z_nm"], e["R"])
        if e["key"] == "refractive":
            law = law + 2 * np.pi * Z9_WAVES * (6 * e["R"] ** 4 - 6 * e["R"] ** 2 + 1)
        d = np.angle(np.exp(1j * (e["phase"][e["inside"]] - law[e["inside"]])))
        if np.abs(d).max() > 1e-6:
            raise AssertionError(f"{e['key']}: pupil phase differs from the exact defocus law by {np.abs(d).max():.1e} rad")
        amps = np.unique(np.round(e["amp"][e["inside"]], 6))
        if len(amps) != 1:
            raise AssertionError(f"{e['key']}: expected one amplitude level inside the pupil, found {amps[:4]}")
        e["amp_level"] = float(amps[0])
        if e["obscuration"] > 0:
            hole = e["R"] < e["obscuration"] - 2.0 / n
            if np.any(e["amp"][hole] > 0):
                raise AssertionError(f"{e['key']}: amplitude inside the {e['obscuration']}·NA obscuration is not zero")
    # (5) opt-in multilayer pupil under an illustrative (assumed) angle map
    try:
        ml = huv.OpticsConfig.euv_nxe().with_multilayer_pupil([tuple(m) for m in MULTILAYER_MIRRORS])
        e = _pupil_entry("multilayer", "EUV NA 0.33 + multilayer pupil\nsix Mo/Si mirrors (assumed)", ml, 13.5, n,
                         note="assumed")
        if not np.all(np.isfinite(e["amp"])) or e["amp"].max() <= 0:
            raise ValueError("multilayer pupil amplitude is not finite")
        e["amp_level"] = float(e["amp"].max())
        entries.append(e)
    except Exception as exc:  # noqa: BLE001 - the fifth column is optional (DESIGN §5.2 I6b)
        print(f"  note  optics-four-pupils: multilayer column skipped ({exc})")
    return {"entries": entries, "n": n, "z9_waves": Z9_WAVES, "z9_coeff": coeff, "z9_resid": resid}


def _measure_crop(fig, ax_list, pad_in: float = 0.08) -> tuple[int, int, int, int]:
    """Pixel box of ``ax_list`` in the saved 2× PNG (tight bbox, the pad save_figure uses)."""
    fig.canvas.draw()
    rend = fig.canvas.get_renderer()
    tb = fig.get_tightbbox(rend)  # inches
    boxes = [a.get_tightbbox(rend) for a in ax_list]
    x0 = min(b.x0 for b in boxes) / fig.dpi
    x1 = max(b.x1 for b in boxes) / fig.dpi
    y0 = min(b.y0 for b in boxes) / fig.dpi
    y1 = max(b.y1 for b in boxes) / fig.dpi
    scale = S.LOGICAL_DPI * S.EXPORT_SCALE
    margin = 0.05  # inches of air around the column
    left = (x0 - tb.x0 + pad_in - margin) * scale
    top = (tb.y1 - y1 + pad_in - 2 * margin) * scale
    w = (x1 - x0 + 2 * margin) * scale
    h = (y1 - y0 + 3 * margin) * scale
    return tuple(int(round(v)) for v in (left, top, w, h))


def four_pupils_draw(d, th: S.Theme):
    entries = d["entries"]
    ncol = len(entries)
    from matplotlib.colors import LinearSegmentedColormap  # noqa: PLC0415

    fig = plt.figure(figsize=(S.WIDE_WIDTH, 4.9))
    gs = GridSpec(2, ncol + 1, figure=fig, width_ratios=[1] * ncol + [0.05], wspace=0.3, hspace=0.3, left=0.045,
                  right=0.95, top=0.84, bottom=0.07)
    # the field ramp stops short of white so a uniform pupil still reads as a disc on the light page
    cmap = LinearSegmentedColormap.from_list("huv_field_amp", S.field_cmap()(np.linspace(0.0, 0.8, 256)))
    dmap = S.diverging_cmap(th)
    ax_hna = []
    im_a = im_p = None
    for c, e in enumerate(entries):
        amp = np.where(e["inside"], e["amp"] / e["amp_level"], np.nan)
        for r, (data, mp, vmin, vmax) in enumerate(((amp, cmap, 0.0, 1.0), (e["phase_rel"], dmap, -np.pi, np.pi))):
            ax = fig.add_subplot(gs[r, c])
            im = ax.imshow(data, origin="lower", extent=[-1, 1, -1, 1], cmap=mp, vmin=vmin, vmax=vmax, interpolation="nearest")
            from matplotlib.patches import Circle  # noqa: PLC0415

            ax.add_patch(Circle((0, 0), 1.0, fill=False, edgecolor=th.axis, linewidth=S.HAIR_PT))
            if e["obscuration"] > 0:
                ax.add_patch(Circle((0, 0), e["obscuration"], fill=False, edgecolor=th.axis, linewidth=S.HAIR_PT, linestyle=(0, (3, 2))))
            ax.set_xlim(-1.06, 1.06)
            ax.set_ylim(-1.06, 1.06)
            ax.set_aspect("equal")
            ax.set_xticks([])
            ax.set_yticks([])
            for side in ("top", "right", "left", "bottom"):
                ax.spines[side].set_visible(False)
            if r == 0:
                ax.set_title(e["title"], fontsize=S.SMALL_PT, pad=8, linespacing=1.3)
                im_a = im
                ax.text(0.5, -0.03, f"|P| = {e['amp_level']:.2f}" if e["key"] != "multilayer" else "|P| relative to its maximum",
                        transform=ax.transAxes, ha="center", va="top", color=th.muted, fontsize=S.SMALL_PT)
            else:
                im_p = im
                ax.text(0.5, -0.03, f"z = λ/NA² = {e['z_nm']:.0f} nm", transform=ax.transAxes, ha="center", va="top",
                        color=th.muted, fontsize=S.SMALL_PT)
            if c == 0:
                ax.text(-0.08, 0.5, "amplitude |P|" if r == 0 else "phase arg P", transform=ax.transAxes, rotation=90,
                        ha="right", va="center", color=th.ink2, fontsize=S.BASE_PT)
            if e["key"] == "high_na":
                ax_hna.append(ax)
    cax = fig.add_subplot(gs[0, ncol])
    cb = fig.colorbar(im_a, cax=cax)
    cb.outline.set_edgecolor(th.axis)
    cb.outline.set_linewidth(S.HAIR_PT)
    cb.ax.tick_params(colors=th.axis, labelcolor=th.ink2, labelsize=S.SMALL_PT, width=S.HAIR_PT, length=2.5)
    cb.set_ticks([0, 0.5, 1.0])
    cax = fig.add_subplot(gs[1, ncol])
    cb = fig.colorbar(im_p, cax=cax)
    cb.outline.set_edgecolor(th.axis)
    cb.outline.set_linewidth(S.HAIR_PT)
    cb.ax.tick_params(colors=th.axis, labelcolor=th.ink2, labelsize=S.SMALL_PT, width=S.HAIR_PT, length=2.5)
    cb.set_ticks([-np.pi, 0, np.pi])
    cb.set_ticklabels(["−π", "0", "π"])
    fig.text(0.045, 0.975, "Pupil amplitude and phase at one Rayleigh unit of defocus (OpticsConfig.pupil_map)",
             ha="left", va="top", color=th.ink, fontsize=S.TITLE_PT, fontweight="semibold")
    fig.text(0.045, 0.935, "wafer-side pupil, radius NA/λ; dashed: central obscuration; phase relative to the pupil centre, wrapped to "
             f"(−π, π]; the refractive column adds Z9 = 2π·{d['z9_waves']}·(6ρ⁴ − 6ρ² + 1)", ha="left", va="top", color=th.muted,
             fontsize=S.SMALL_PT)
    box = _measure_crop(fig, ax_hna)
    if any(abs(a - b) > FOUR_PUPILS_CROP_TOL for a, b in zip(box, FOUR_PUPILS_CROP_BOX)):
        raise AssertionError(f"optics-four-pupils: the High-NA column sits at {box} in the 2x PNG; FOUR_PUPILS_CROP_BOX is "
                             f"{FOUR_PUPILS_CROP_BOX} — update the constant (and re-render) so the crop stays on the column")
    return fig


register(FigureSpec(
    name="optics-four-pupils",
    group="imaging",
    compute=four_pupils_compute,
    draw=four_pupils_draw,
    alt="Amplitude and phase maps of five pupils at one Rayleigh unit of defocus: a refractive NA 0.75 lens with Z9 "
        "spherical, the High-NA EUV ring with its assumed 0.2 NA obscuration, a Schwarzschild annulus, a uniform "
        "zone-plate pupil and an EUV pupil shaded by an assumed multilayer angle map.",
    caption="<strong>In the engine: four pupils at one Rayleigh unit of defocus.</strong> Amplitude above, phase below, each "
            "at z = λ/NA²: a refractive NA 0.75 lens with 0.05 λ of Fringe Z9 spherical, the High-NA EUV preset with its "
            "assumed 0.2·NA obscuration, a Schwarzschild objective and a first-order zone plate; the fifth column adds the "
            "opt-in Mo/Si multilayer pupil under an assumed six-mirror angle map. <em>Model: refractive projection optics "
            "(dry) ✅ · Zernike aberrations ✅ · EUV scanner projection optics (NA 0.33 / High-NA 0.55) 🔶 (obscuration "
            "assumed) · Schwarzschild reflective objective ✅ · Fresnel zone plate ✅ · EUV/BEUV multilayer pupil (opt-in) 🔶 "
            "(user-assumed angle map).</em>",
    pages=("optics.md",),
    extra={
        "crops": {
            "high-na": dict(
                pages=("future/high-na-and-hyper-na.md",),
                kind="figure",
                box=FOUR_PUPILS_CROP_BOX,
                alt="Amplitude and phase of the High-NA EUV pupil at NA 0.55 at one Rayleigh unit of defocus: a ring with "
                    "the assumed 0.2 NA central obscuration above, the exact defocus phase wrapping towards the rim below.",
                caption="<strong>The High-NA pupil the engine images through.</strong> Isotropic wafer-side pupil at NA 0.55 "
                        "with the assumed 0.2·NA central obscuration (not a published figure); amplitude above, the exact "
                        "defocus phase at z = λ/NA² ≈ 45 nm below, wrapped to (−π, π]. <em>Model: EUV scanner projection "
                        "optics (NA 0.33 / High-NA 0.55) 🔶 (obscuration assumed; no anamorphic magnification, no mask "
                        "3-D).</em>",
            ),
        },
    },
))


# ---------------------------------------------------------------------------
# 10. I6c — the exact defocus phase and its paraxial limit
# ---------------------------------------------------------------------------

DEFOCUS_Z_NM = 100.0


def defocus_phase_compute(fast: bool):
    huv = native()
    n = 65 if fast else 257
    rho = np.linspace(0, 1, n)
    cases = (("na010", "NA 0.10, air", 0.10, 1.0, huv.OpticsConfig(numerical_aperture=0.10, flare_fraction=0.0)),
             ("na080", "NA 0.80, air", 0.80, 1.0, huv.OpticsConfig(numerical_aperture=0.80, flare_fraction=0.0)),
             ("na135", "NA 1.35, water n = 1.437", 1.35, 1.437, huv.OpticsConfig.immersion(numerical_aperture=1.35, immersion_index=1.437, flare_fraction=0.0)))
    curves = []
    m = 2 * n - 1  # pupil_map grid: odd, so ρ = 0 … 1 is exactly the right half of the centre row
    for key, label, na, index, optics in cases:
        P = np.asarray(optics.pupil_map(ARF_NM, n=m, focus_nm=DEFOCUS_Z_NM))
        row = P[m // 2, m // 2:]
        exact = _defocus_phase_exact(na, index, ARF_NM, DEFOCUS_Z_NM, rho)
        par = np.pi * DEFOCUS_Z_NM * na ** 2 * rho ** 2 / (index * ARF_NM)
        inside = np.abs(row) > 0
        d = np.angle(np.exp(1j * (np.angle(row)[inside] - exact[inside])))
        if np.abs(d).max() > 1e-6:
            raise AssertionError(f"{key}: pupil_map phase differs from the exact law by {np.abs(d).max():.1e} rad")
        curves.append({"key": key, "label": label, "na": na, "index": index, "rho": rho, "exact": exact, "paraxial": par,
                       "rim_error": float(1 - par[-1] / exact[-1]), "rim_exact": float(exact[-1]), "engine_max_diff": float(np.abs(d).max())})
    # relative rim error (1 − cosθ)/2 ≈ sin²θ/4 against NA, in air and in water
    na_air = np.linspace(0.05, 0.95, 91)
    na_water = np.linspace(0.05, 1.35, 131)
    err_air = (1 - np.sqrt(1 - na_air ** 2)) / 2
    err_water = (1 - np.sqrt(1 - (na_water / 1.437) ** 2)) / 2
    marks = {c["key"]: (c["na"], c["rim_error"]) for c in curves}
    if not (0.002 < marks["na010"][1] < 0.003 and 0.19 < marks["na080"][1] < 0.21):
        raise AssertionError(f"rim errors {marks} are not 0.25 % at NA 0.1 and 20 % at NA 0.8")
    s_dry, s_wet = 0.9, 0.9 / 1.437
    phi_dry = 2 * np.pi / ARF_NM * DEFOCUS_Z_NM * (1 - np.sqrt(1 - s_dry ** 2))
    phi_wet = 2 * np.pi * 1.437 / ARF_NM * DEFOCUS_Z_NM * (1 - np.sqrt(1 - s_wet ** 2))
    # the caption quotes these two numbers (1.83 → 1.03 rad at NA·ρ = 0.9); pin them so the text cannot drift from the figure
    assert (round(float(phi_dry), 2), round(float(phi_wet), 2)) == (1.83, 1.03), (phi_dry, phi_wet)
    return {"curves": curves, "z_nm": DEFOCUS_Z_NM, "wavelength_nm": ARF_NM, "na_air": na_air, "err_air": err_air,
            "na_water": na_water, "err_water": err_water, "marks": marks, "phi_09": (phi_dry, phi_wet)}


def defocus_phase_draw(d, th: S.Theme):
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 4.2), gridspec_kw={"wspace": 0.34, "width_ratios": [1.3, 1]})
    for ax in (ax1, ax2):
        S.style_axes(ax, th, grid="both")
    slots = {"na135": 1, "na080": 2, "na010": 3}
    curves = {c["key"]: c for c in d["curves"]}
    for c in d["curves"]:
        slot = slots[c["key"]]
        x = c["na"] * c["rho"]  # NA·ρ = n sin θ, one axis for all three optics
        ax1.plot(x, c["exact"], color=th.s(slot), zorder=3)
        ax1.plot(x, c["paraxial"], color=th.s(slot), linestyle=(0, (4, 3)), linewidth=S.HAIR_PT * 1.6, zorder=2)
    c = curves["na135"]
    S.end_label(ax1, c["na"], c["rim_exact"], f"NA 1.35 in water (n = 1.437):\n{c['rim_exact']:.2f} rad at the rim", th, dx_px=6, dy_px=-4,
                va="top", linespacing=1.3)
    c = curves["na080"]
    S.end_label(ax1, c["na"], c["rim_exact"], f"NA 0.80 in air: {c['rim_exact']:.2f} rad at the rim", th, dx_px=-6, dy_px=10, ha="right")
    c = curves["na010"]
    ax1.text(0.52, 0.04, f"NA 0.10 in air: its rim is at NA·ρ = 0.1, {c['rim_exact']:.3f} rad", color=th.ink2, fontsize=S.SMALL_PT,
             ha="left", va="bottom")
    phi_dry, phi_wet = d["phi_09"]
    ax1.axvline(0.9, color=th.axis, linewidth=S.HAIR_PT, zorder=1)
    for y, lab, dx, dy, ha in ((phi_dry, f"{phi_dry:.2f} rad in air", -8, 8, "right"), (phi_wet, f"{phi_wet:.2f} rad in water", 8, -8, "left")):
        ax1.plot([0.9], [y], marker="o", markersize=S.MARKER_PT * 0.9, color=th.ink, markeredgecolor=th.surface,
                 markeredgewidth=S.RING_PT, linestyle="none", zorder=5, clip_on=False)
        S.end_label(ax1, 0.9, y, lab, th, dx_px=dx, dy_px=dy, ha=ha)
    ax1.set_xlim(0, 1.45)
    ax1.set_ylim(0, 3.3)
    ax1.set_xlabel("pupil position NA·ρ = n sin θ")
    ax1.set_ylabel(f"defocus phase Φ at z = {d['z_nm']:.0f} nm (rad)")
    ax1.set_title(f"(a) exact (solid) vs paraxial (dashed), λ = {d['wavelength_nm']} nm")
    ax1.legend([plt.Line2D([], [], color=th.ink2), plt.Line2D([], [], color=th.ink2, linestyle=(0, (4, 3)), linewidth=S.HAIR_PT * 1.6)],
               ["exact: (2πn/λ)·z·(1 − √(1 − (NAρ/n)²))", "paraxial: πzNA²ρ²/(nλ)"], loc="upper left")
    fig.subplots_adjust(left=0.07, right=0.98, bottom=0.15, top=0.9)
    ax2.plot(d["na_air"], 100 * d["err_air"], color=th.s(2), zorder=3)
    ax2.plot(d["na_water"], 100 * d["err_water"], color=th.s(1), zorder=3)
    for key, slot in (("na010", 2), ("na080", 2), ("na135", 1)):
        na, err = d["marks"][key]
        ax2.plot([na], [100 * err], marker="o", markersize=S.MARKER_PT, color=th.s(slot), markeredgecolor=th.surface,
                 markeredgewidth=S.RING_PT, linestyle="none", zorder=4, clip_on=False)
        S.end_label(ax2, na, 100 * err, f"NA {na:.2f}: {100 * err:.2g} %", th, dx_px=8 if key != "na135" else -8,
                    dy_px=6, ha="left" if key != "na135" else "right")
    ax2.set_xlim(0, 1.4)
    ax2.set_ylim(0, 36)
    ax2.set_xlabel("numerical aperture")
    ax2.set_ylabel("paraxial error at the rim (%)")
    ax2.set_title("(b) how far off the paraxial phase is: (1 − cos θ)/2 ≈ sin²θ/4")
    ax2.legend([plt.Line2D([], [], color=th.s(2)), plt.Line2D([], [], color=th.s(1))], ["in air (n = 1)", "in water (n = 1.437)"],
               loc="upper left")
    return fig


register(FigureSpec(
    name="optics-defocus-phase",
    group="imaging",
    compute=defocus_phase_compute,
    draw=defocus_phase_draw,
    alt="Defocus phase across the pupil at 100 nm defocus and 193.368 nm for NA 0.1 and 0.8 in air and NA 1.35 in water, "
        "exact against paraxial, 1.83 rad in air falling to 1.03 rad in water at pupil position 0.9; a second panel shows "
        "the paraxial rim error rising from 0.25 % at NA 0.1 to 20 % at NA 0.8.",
    caption="<strong>The exact defocus phase and its paraxial limit.</strong> Φ<sub>z</sub>(ρ) at z = 100 nm and "
            "193.368 nm for NA 0.1 and 0.8 in air and NA 1.35 in water: the paraxial πzNA²ρ²/(nλ) is 0.25 % low at the rim "
            "for NA 0.1 and 20 % low at NA 0.8, and in water at the same pupil position NA·ρ = 0.9 the phase is smaller "
            "(1.83 → 1.03 rad) — immersion buys depth of focus. The curves agree with <code>OpticsConfig.pupil_map</code> "
            "to 1e-6 rad. <em>Model: defocus / through-focus imaging ✅ · immersion lithography (193i) ✅.</em>",
    pages=("optics.md",),
))


# ---------------------------------------------------------------------------
# 11. I8 — your first image: the canonical run, drawn
# ---------------------------------------------------------------------------

FIRST_THRESHOLD = 0.3


def first_image_compute(fast: bool):
    import highuvlith as H  # noqa: PLC0415 - the public one-liner, exactly as getting-started example 1 calls it

    r = H.simulate_line_space(CANON_CD, CANON_PITCH, na=CANON_NA)
    aerial = r.aerial
    x = np.asarray(aerial.x_nm)
    inten = np.asarray(aerial.intensity, dtype=float)
    cut_x, cut = aerial.cross_section(0.0)
    cut_x, cut = np.asarray(cut_x), np.asarray(cut)
    feats = aerial.features(FIRST_THRESHOLD, "dark")
    if not feats:
        raise AssertionError("no dark line prints at the 0.3 threshold")
    left, right, width, centre = min(feats, key=lambda f: abs(f[3]))
    if r.cd_nm is None or abs(width - r.cd_nm) > 1e-9:
        raise AssertionError(f"printed CD {r.cd_nm} does not match the feature width {width}")
    c_direct = (inten.max() - inten.min()) / (inten.max() + inten.min())
    if abs(c_direct - r.contrast) > 1e-12:
        raise AssertionError(f"contrast {r.contrast} is not (Imax−Imin)/(Imax+Imin) = {c_direct}")
    return {
        "x": x, "y": np.asarray(aerial.y_nm), "intensity": inten, "cut_x": cut_x, "cut": cut,
        "contrast": float(r.contrast), "nils": float(r.nils), "cd_nm": float(r.cd_nm), "edges": (float(left), float(right)),
        "threshold": FIRST_THRESHOLD, "config": {k: r.config[k] for k in ("wavelength_nm", "na", "sigma", "pixel_nm", "grid_size", "field_nm")},
        "flare": 0.02, "pitch": CANON_PITCH, "cd": CANON_CD,
    }


def first_image_draw(d, th: S.Theme):
    from matplotlib.patches import Rectangle  # noqa: PLC0415

    fig = plt.figure(figsize=(S.WIDE_WIDTH, 4.1))
    gs = GridSpec(1, 2, figure=fig, width_ratios=[1.0, 1.4], wspace=0.3, left=0.05, right=0.985, top=0.86, bottom=0.16)
    cmap = S.field_cmap()
    p = d["pitch"]
    # the field holds one period (a square of one pitch); show 2 × 2 periods, so the line sits between two spaces
    img = np.tile(np.roll(d["intensity"], d["intensity"].shape[1] // 2, axis=1), (2, 2))
    ax = fig.add_subplot(gs[0, 0])
    im = ax.imshow(img, origin="lower", extent=[-p, p, -p, p], cmap=cmap, vmin=0, vmax=max(1.0, float(img.max())),
                   interpolation="bilinear", aspect="equal")
    S.image_axes(ax, th)
    ax.set_xlabel("x (nm)")
    ax.set_ylabel("y (nm)")
    ax.set_title("(a) aerial image, 2 × 2 periods")
    S.colorbar(fig, im, ax, th, "intensity", fraction=0.04, pad=0.04)
    # (b) the y = 0 cut with the threshold, the printed CD, contrast and NILS
    ax = fig.add_subplot(gs[0, 1])
    S.style_axes(ax, th, grid="both")
    cx, cut = d["cut_x"], d["cut"]
    xx = np.concatenate([cx - p / 2, cx + p / 2])
    yy = np.tile(np.roll(cut, len(cut) // 2), 2)
    ax.plot(xx, yy, color=th.s(1), zorder=3)
    ax.axhline(d["threshold"], color=th.ink2, linewidth=S.HAIR_PT * 1.3, linestyle=(0, (4, 3)), zorder=2)
    ax.text(xx[0] + 4, d["threshold"] + 0.015, f"threshold {d['threshold']}", color=th.ink2, fontsize=S.SMALL_PT, ha="left", va="bottom")
    l, r = d["edges"]
    ax.add_patch(Rectangle((l, 0), r - l, d["threshold"], facecolor=th.band, edgecolor="none", zorder=1))
    y_dim = d["threshold"] * 0.5
    ax.annotate("", xy=(l, y_dim), xytext=(r, y_dim), arrowprops=dict(arrowstyle="<->", color=th.ink2, lw=S.HAIR_PT * 1.3, shrinkA=0, shrinkB=0))
    ax.text((l + r) / 2, y_dim + 0.012, f"printed CD {d['cd_nm']:.1f} nm", ha="center", va="bottom", color=th.ink, fontsize=S.SMALL_PT)
    ax.set_xlim(xx[0], xx[-1])
    ax.set_ylim(0, max(1.0, float(yy.max()) * 1.12))
    ax.set_xlabel("x (nm)")
    ax.set_ylabel("relative intensity")
    ax.set_title("(b) cross-section at y = 0")
    text = (f"contrast = {d['contrast']:.3f}\nNILS = {d['nils']:.2f}\n{d['cd']:.0f} nm mask line → {d['cd_nm']:.1f} nm printed at {d['threshold']}")
    ax.text(0.985, 0.97, text, transform=ax.transAxes, ha="right", va="top", color=th.ink, fontsize=S.SMALL_PT,
            bbox=dict(boxstyle="round,pad=0.4", facecolor=th.surface, edgecolor=th.axis, linewidth=S.HAIR_PT))
    cfg = d["config"]
    fig.text(0.045, 0.975, f"simulate_line_space({d['cd']:.1f}, {d['pitch']:.1f}, na={cfg['na']})  ·  F$_2$ {cfg['wavelength_nm']} nm, "
             f"σ {cfg['sigma']}, grid {cfg['grid_size']} × {cfg['pixel_nm']:.4g} nm (one period), {d['flare']:.0%} flare",
             ha="left", va="top", color=th.ink2, fontsize=S.SMALL_PT)
    return fig


register(FigureSpec(
    name="start-first-image",
    group="imaging",
    compute=first_image_compute,
    draw=first_image_draw,
    alt="The canonical first run drawn: a two-period aerial image of 65 nm lines on a 180 nm pitch at 157.63 nm, NA 0.75, "
        "and its cross-section with the 0.3 threshold, a 41.2 nm printed line, contrast 0.454 and NILS 0.65.",
    caption="<strong>Your first image: the numbers printed above, drawn.</strong> 65 nm lines at 180 nm pitch, "
            "F<sub>2</sub> 157.63 nm, NA 0.75, σ 0.7 — the same run as <code>examples/sim.toml</code>; contrast, NILS and "
            "the printed CD at the 0.3 threshold are computed at figure time by the same call. <em>Model: Hopkins imaging "
            "(factorized TCC/SOCS) ✅; thin mask, 2 % uniform flare, no resist.</em>",
    pages=("getting-started.md", "python-api.md", "simulator/index.md", "README.md"),
    extra={"width": {"README.md": 820}},  # README <picture> width (DESIGN §6.2; INT late-binding edit)
))
