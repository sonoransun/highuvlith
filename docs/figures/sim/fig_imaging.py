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
    alt="Line chart of aerial-image contrast versus pitch for coherent, sigma 0.4 and sigma 0.8 illumination at "
        "157.63 nm and NA 0.75; each curve drops to zero at its cutoff pitch lambda/(NA(1+sigma)).",
    caption="Simulated contrast of 1:1 line/space gratings versus pitch (F<sub>2</sub> 157.63 nm, NA 0.75, "
            "conventional illumination, no flare). A coherent point source stops imaging at λ/NA = 210 nm; a "
            "partially coherent fill of radius σ keeps imaging down to λ/(NA(1+σ)) — the regime the v1 engine "
            "could not reach. Model: scalar Hopkins/SOCS imaging with an exactly factorized TCC ✅.",
    pages=("pipeline.md",),
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
    caption="Why the defocus fix matters. Simulated 90 nm lines on a 180 nm pitch (F<sub>2</sub> 157.63 nm, NA 0.75, "
            "σ 0.7, no flare), through ±400 nm of focus. The exact model applies the defocus phase inside the "
            "pupil for every source point (default, ✅); the legacy <code>kernel_phase</code> option multiplies each "
            "mask frequency by one on-axis phase and predicts Talbot-like contrast revivals that partially coherent "
            "illumination washes out (🔶, kept for comparison only).",
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
    caption="Polarization at high NA. (a) Fringe contrast of two diffraction orders at the pupil edge from the "
            "vector pupil model: TE stays at 1, TM follows |cos 2θ| and vanishes at NA = 1/√2, unpolarized "
            "light gives cos²θ; inside a resist film (n = 1.7) the refracted angle is smaller and TM recovers. "
            "(b) Full vector imaging of dense 1:1 lines (193.368 nm, pitch 1.1 λ/NA, conventional σ 0.3) shows "
            "the same ordering. Model: vector (TE/TM) imaging ✅ — thin mask, no mask-3D or lens polarization "
            "aberrations.",
    pages=("vector-imaging.md", "future/high-na-and-hyper-na.md"),
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
    caption="The vector pupil at NA 0.9 (image in air). (a) For x-polarized illumination the field of each "
            "order tilts with its propagation direction: near the pupil edge along x most of the energy is in "
            "E<sub>z</sub>, which cannot interfere with the opposite order. (b) The radiometric (obliquity) factor "
            "brightens the pupil edge. Computed by the vector pupil model (<code>vector_pupil_map</code>) ✅.",
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
    caption="Simulated contrast of 1:1 lines vs pitch for ArF (193.368 nm, conventional σ 0.9, no flare): dry "
            "optics at NA 0.93 against water immersion at NA 1.35 (n = 1.437, <code>OpticsConfig.immersion</code>). "
            "The scalar curves show the NA gain; the vector curves show why 193i needs polarization control — "
            "unpolarized light loses much of the gain, TE (y-) polarized light keeps it. Models: immersion optics "
            "✅, scalar and vector imaging ✅ (thin mask; no water absorption or in-resist focus).",
    pages=("optics.md",),
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
    caption="Honest comb imaging. A neon HHG source (800 nm driver) filtered to 12.8–14.4 nm keeps harmonics "
            "57, 59 and 61; each has its own cutoff pitch λ<sub>q</sub>/(NA(1+σ)) (grey: each harmonic alone). The "
            "exact path rebuilds the TCC at every spectral sample, so the comb image is the weighted sum of the "
            "three harmonics' images (q 57 carries most of the filtered flux) and its contrast lies between the "
            "single-harmonic curves; the narrow-band "
            "path keeps the centre-λ kernels and only shifts focus — with reflective optics (no chromatic focus "
            "shift) it is identical to the centre harmonic alone. EUV projection optics NA 0.33, unobscured, no "
            "flare; near-coherent Gaussian fill (σ ≈ 0.05). "
            "Models: per-wavelength imaging ✅; narrow-band approximation 🔶 (valid for Δλ/λ ≪ 1); HHG source ✅/🔶 (comb and cutoff exact; power assumed).",
    pages=("sources/hhg.md", "pipeline.md"),
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
    caption="Off-axis illumination below λ/NA. F<sub>2</sub> 157.63 nm, NA 0.75, 1:1 lines on a 130 nm pitch "
            "(k<sub>1</sub> ≈ 0.31; λ/NA = 210 nm). (a–c) Pupil fills of the source (circle = NA), all with outer "
            "σ ≤ 0.8 so they share the λ/(NA·1.8) ≈ 117 nm cutoff. (d) Aerial-image cross-sections at best focus: "
            "moving the light to the pupil edge in the direction of the grating (annular, and most of all the "
            "x-dipole) puts more of it into two-beam (0, ±1) imaging and raises the contrast. (e) The same fills "
            "across pitch: the dipole gives the highest contrast here, but only for gratings of this orientation — "
            "the same pattern rotated by 90° does not get its off-axis benefit. Model: "
            "scalar Hopkins/SOCS imaging ✅ with the pupil-fill shapes of the source module (two-pole dipole since "
            "the round-3 fix); no flare, thin mask, no resist.",
    pages=("pipeline.md", "README.md"),
    requires=("illumination", "pupil_fill"),
))
