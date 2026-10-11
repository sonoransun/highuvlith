"""Raster animations of the imaging engine: through focus (I5) and TE against TM (I7b).

Both are ports of ``examples/generate_demo.py`` sequences 1 and 2 into the figure registry:
``compute(fast)`` runs the engine once and keeps plain arrays (the physical asserts live
here, so ``make_all.py --verify --group anim`` exercises them without drawing); ``draw``
returns a list of matplotlib figures drawn at 1× and 800 px wide, which ``style.write_frames``
turns into the APNG (``-anim-<theme>.png``), the designed still (``-still-<theme>.png`` at 2×)
and, for the through-focus sequence, the README GIF.

* ``through-focus`` — 90 nm lines on a 180 nm pitch, F₂ 157.63 nm, NA 0.75 (k₁ ≈ 0.43),
  conventional σ 0.7 against an x-dipole at σ_c = λ/(2pNA) ≈ 0.58, 48 planes from
  0 → +500 → −500 → 0 nm. The engine construction is the one ``getting-started.md``'s
  "Through focus" example uses (:func:`through_focus_engines`), so the contrasts the frames
  print at −500, 0 and +500 nm are the numbers that example records.
* ``through-focus-rest`` — the resting frame (best focus) as a static 2× figure for the
  pages that show the still rather than the animation (``pipeline.md``).
* ``te-tm-anim`` — dense 1:1 lines under an ArF x-dipole (σ 0.8) at pitch λ/(1.6·NA), so the
  two orders meet at sin θ = 0.8·NA; the NA sweeps 0.5 → 0.95 and back; TE (y), TM (x) and
  unpolarized cross-sections from ``SimulationEngine(…, vector=VectorSettings(polarization))``
  with the two-beam closed forms overlaid.
"""

from __future__ import annotations

import numpy as np
from matplotlib import pyplot as plt
from matplotlib.gridspec import GridSpec

import style as S
from common import FigureSpec, native, register

F2_NM = 157.63
ARF_NM = 193.368

# ---------------------------------------------------------------------------
# I5 · through-focus
# ---------------------------------------------------------------------------

TF_CD, TF_PITCH, TF_NA, TF_SIGMA = 90.0, 180.0, 0.75, 0.7
TF_SIGMA_C = F2_NM / (2 * TF_PITCH * TF_NA)  # the x-dipole pole centre, λ/(2pNA) ≈ 0.584
TF_RECORDED = (-500.0, 0.0, 500.0)  # the planes the getting-started example prints
TF_FRAMES = 48
TF_CURVE_PLANES = 41


def through_focus_engines(huv):
    """The two engines of the getting-started "Through focus" example — byte for byte the same construction.

    ::

        optics = huv.OpticsConfig(numerical_aperture=0.75, flare_fraction=0.0)
        mask = huv.MaskConfig.line_space(90.0, 180.0)
        grid = huv.GridConfig(size=128, pixel_nm=360.0 / 128)          # two periods, commensurate
        sigma_c = 157.63 / (2 * 180.0 * 0.75)                            # λ / (2 · pitch · NA) ≈ 0.58
        conventional = huv.SourceConfig.f2_laser(0.7)
        dipole = huv.SourceConfig.f2_laser().with_illumination("dipole", sigma_c, 0.1, 0.0)
        engine = huv.SimulationEngine(source, optics, mask, grid=grid)
    """
    optics = huv.OpticsConfig(numerical_aperture=TF_NA, flare_fraction=0.0)
    mask = huv.MaskConfig.line_space(TF_CD, TF_PITCH)
    grid = huv.GridConfig(size=128, pixel_nm=2 * TF_PITCH / 128)
    conventional = huv.SourceConfig.f2_laser(TF_SIGMA)
    dipole = huv.SourceConfig.f2_laser().with_illumination("dipole", TF_SIGMA_C, 0.1, 0.0)
    return [
        ("conventional σ 0.7", huv.SimulationEngine(conventional, optics, mask, grid=grid)),
        (f"x-dipole σ_c {TF_SIGMA_C:.2f}", huv.SimulationEngine(dipole, optics, mask, grid=grid)),
    ]


def _tf_sweep(n: int) -> np.ndarray:
    """0 → +500 → −500 → 0 nm without repeated end points (frame 0 = best focus, the designed still)."""
    a, b = n // 4, n // 2
    return np.concatenate([np.linspace(0, 500, a, endpoint=False), np.linspace(500, -500, b, endpoint=False),
                           np.linspace(-500, 0, n - a - b, endpoint=False)])


_TF_CACHE: dict = {}


def through_focus_compute(fast: bool):
    key = bool(fast)
    if key in _TF_CACHE:
        return _TF_CACHE[key]
    huv = native()
    sweep = _tf_sweep(12 if fast else TF_FRAMES)
    curve_z = np.linspace(-500.0, 500.0, 11 if fast else TF_CURVE_PLANES)
    out = {"focus": sweep, "curve_z": curve_z, "panels": {}, "recorded": {}, "sigma_c": TF_SIGMA_C}
    vmax = 0.0
    for name, eng in through_focus_engines(huv):
        planes = eng.compute_through_focus([float(z) for z in sweep])
        curve = np.array([p.image_contrast() for p in eng.compute_through_focus([float(z) for z in curve_z])])
        recorded = [p.image_contrast() for p in eng.compute_through_focus(list(TF_RECORDED))]
        images = [np.asarray(p.intensity, dtype=float).copy() for p in planes]
        cuts = [np.asarray(p.cross_section(0.0)[1], dtype=float).copy() for p in planes]
        out["panels"][name] = {
            "images": images, "cuts": cuts, "x": np.asarray(planes[0].x_nm, dtype=float).copy(),
            "contrast": np.array([p.image_contrast() for p in planes]), "curve": curve,
        }
        out["recorded"][name] = recorded
        vmax = max(vmax, max(float(im.max()) for im in images))
    out["vmax"] = vmax
    # physical asserts (DESIGN §5.2 I5): best focus is the maximum; symmetric in z; the dipole keeps its contrast
    conv, dip = list(out["recorded"].values())
    assert abs(conv[0] - conv[2]) < 0.01 and abs(dip[0] - dip[2]) < 0.01, "exact defocus is symmetric in z for a symmetric source"
    assert conv[1] > 0.5 and conv[0] < 0.05, f"conventional σ 0.7: best focus {conv[1]:.3f}, ±500 nm {conv[0]:.3f}"
    assert dip[1] > 0.85 and dip[0] > 0.5, f"dipole: best focus {dip[1]:.3f}, ±500 nm {dip[0]:.3f}"
    for name, p in out["panels"].items():
        assert abs(float(p["curve"].max()) - out["recorded"][name][1]) < 0.02, f"{name}: best focus is not the curve maximum"
    _TF_CACHE[key] = out
    return out


def _frame_figure():
    """An 8 × 4.5 in figure: 800 × 450 px at 1× (``style.ANIM_WIDTH_PX``), 1600 × 900 for the 2× still."""
    return plt.figure(figsize=(S.ANIM_WIDTH_PX / S.LOGICAL_DPI, 4.5))


def through_focus_frame(d, th: S.Theme, i: int):
    fig = _frame_figure()
    gs = GridSpec(2, 4, figure=fig, width_ratios=[0.045, 1.0, 1.1, 1.5], left=0.045, right=0.985, top=0.80, bottom=0.13,
                  wspace=0.42, hspace=0.55)
    focus = float(d["focus"][i])
    names = list(d["panels"])
    fig.text(0.045, 0.95, "Through focus: conventional σ 0.7 against an x-dipole at σ$_c$ ≈ 0.58", fontsize=S.TITLE_PT,
             fontweight="semibold", color=th.ink, ha="left", va="top")
    fig.text(0.045, 0.885, "90 nm lines / 180 nm pitch · F$_2$ 157.63 nm · NA 0.75 ($k_1$ ≈ 0.43) · exact defocus per source point · "
             "scalar, thin mask, no flare", fontsize=S.SMALL_PT, color=th.ink2, ha="left", va="top")
    cmap = S.field_cmap()
    vmax = float(d["vmax"])
    ax_c = fig.add_subplot(gs[:, 3])
    S.style_axes(ax_c, th)
    im = None
    for row, (name, slot) in enumerate(zip(names, (1, 2))):
        p = d["panels"][name]
        x = p["x"]
        ax_img = fig.add_subplot(gs[row, 1])
        im = ax_img.imshow(p["images"][i], cmap=cmap, vmin=0, vmax=vmax, origin="lower",
                           extent=[x[0], x[-1], x[0], x[-1]], aspect="equal", interpolation="nearest")
        S.image_axes(ax_img, th)
        ax_img.set_xticks([])
        ax_img.set_yticks([])
        ax_img.set_title(name.replace("σ_c", "σ$_c$"), fontsize=S.BASE_PT, x=0.13)
        ax_img.plot([0.0, 0.09], [1.075, 1.075], transform=ax_img.transAxes, color=th.s(slot), linewidth=3.2, clip_on=False,
                    solid_capstyle="butt")
        ax_img.plot([x[0] + 12, x[0] + 12 + 90], [x[0] + 14, x[0] + 14], color="#ffffff", linewidth=S.LINE_PT, solid_capstyle="butt")
        ax_img.text(x[0] + 12, x[0] + 24, "90 nm", color="#ffffff", fontsize=S.SMALL_PT, ha="left", va="bottom")
        ax_cut = fig.add_subplot(gs[row, 2])
        S.style_axes(ax_cut, th)
        ax_cut.plot(x, p["cuts"][i], color=th.s(slot), zorder=3)
        ax_cut.fill_between(x, 0, p["cuts"][i], color=th.s(slot), alpha=0.15, linewidth=0)
        ax_cut.set_xlim(x[0], x[-1])
        ax_cut.set_ylim(0, 1.3)
        ax_cut.set_yticks([0, 0.5, 1.0])
        ax_cut.set_xlabel("x (nm)" if row == 1 else "")
        ax_cut.set_ylabel("I / clear field")
        if row == 0:
            ax_cut.tick_params(labelbottom=False)
        ax_cut.text(0.97, 0.92, f"C = {p['contrast'][i]:.2f}", transform=ax_cut.transAxes, ha="right", va="top",
                    color=th.ink, fontsize=S.BASE_PT, fontweight="semibold")
        ax_cut.set_title("cut at y = 0", fontsize=S.BASE_PT)
        ax_c.plot(d["curve_z"], p["curve"], color=th.s(slot), zorder=3)
        S.dot(ax_c, [focus], [p["contrast"][i]], th.s(slot), th, zorder=5)
        for z, c in zip(TF_RECORDED, d["recorded"][name]):
            S.dot(ax_c, [z], [c], th.s(slot), th, size_px=6.5, zorder=4, filled=False)
            if z == 0.0 and slot == 1:
                ax_c.annotate(f"{c:.2f}", (z, c), xytext=(-9, 0), textcoords="offset points", ha="right", va="center",
                              color=th.ink2, fontsize=S.SMALL_PT)
            else:
                ax_c.annotate(f"{c:.2f}", (z, c), xytext=(0, 7), textcoords="offset points", ha="center", va="bottom",
                              color=th.ink2, fontsize=S.SMALL_PT)
    cax = fig.add_subplot(gs[:, 0])
    cb = fig.colorbar(im, cax=cax)
    cb.outline.set_edgecolor(th.axis)
    cb.outline.set_linewidth(S.HAIR_PT)
    cb.ax.yaxis.set_ticks_position("left")
    cb.ax.tick_params(colors=th.axis, labelcolor=th.ink2, labelsize=S.SMALL_PT, width=S.HAIR_PT, length=2.5)
    cb.ax.set_title("I / clear", fontsize=S.SMALL_PT, color=th.ink2, loc="left", pad=6)
    ax_c.axvline(focus, color=th.ink2, linewidth=S.HAIR_PT * 1.3, zorder=2)
    ax_c.set_xlim(-500, 500)
    ax_c.set_ylim(0, 1.0)
    ax_c.set_xlabel("defocus (nm)")
    ax_c.set_ylabel("image contrast")
    ax_c.set_title("contrast against focus, 41 planes", fontsize=S.BASE_PT)
    ztxt = "0 nm" if abs(focus) < 0.5 else f"{focus:+.0f} nm"
    ax_c.text(0.97, 0.96, f"focus {ztxt}", transform=ax_c.transAxes, ha="right", va="top", color=th.ink,
              fontsize=S.TITLE_PT, fontweight="semibold")
    return fig


def through_focus_draw(d, th: S.Theme):
    return [through_focus_frame(d, th, i) for i in range(len(d["focus"]))]


def through_focus_rest_draw(d, th: S.Theme):
    return through_focus_frame(d, th, 0)


TF_CAPTION = ("<strong>Through focus, conventional versus dipole illumination.</strong> 90 nm lines on a 180 nm pitch at "
              "157.63 nm, NA 0.75; defocus −500 to +500 nm; the marker tracks image contrast and the hollow dots are the "
              "three planes the getting-started example prints. <em>Model: defocus / through-focus imaging ✅ (exact "
              "non-paraxial phase in the pupil per source point) · scalar, thin mask, no flare, no resist.</em>")

register(FigureSpec(
    name="through-focus",
    group="anim",
    compute=through_focus_compute,
    draw=through_focus_draw,
    alt="Animated focus sweep of 90 nm lines at 180 nm pitch, F2 157.63 nm, NA 0.75: the aerial image and its cut under "
        "conventional sigma 0.7 lose nearly all contrast by 500 nm of defocus while an x-dipole keeps most of it; a third "
        "panel traces contrast against focus for both with a moving marker.",
    caption=TF_CAPTION,
    pages=("getting-started.md", "README.md"),
    subdir="anim",
    fmt="apng",
    still_frame=0,
    extra={"gif": True, "duration_ms": 100},
))

register(FigureSpec(
    name="through-focus-rest",
    group="anim",
    compute=through_focus_compute,
    draw=through_focus_rest_draw,
    alt="Best focus of 90 nm lines at 180 nm pitch, F2 157.63 nm, NA 0.75: the aerial image and its cut under conventional "
        "sigma 0.7 (contrast 0.59) and under an x-dipole (0.90), with the contrast-against-focus curves of both beside them.",
    caption=("<strong>Best focus, the resting frame of the through-focus animation.</strong> 90 nm lines on a 180 nm "
             "pitch at 157.63 nm, NA 0.75: conventional σ 0.7 above, an x-dipole at σ_c ≈ 0.58 below; the curves at right "
             "show how each loses contrast with defocus (the animation plays on the getting-started page). <em>Model: "
             "defocus / through-focus imaging ✅ · scalar, thin mask, no flare, no resist.</em>"),
    pages=("pipeline.md",),
    subdir="anim",
    fmt="png",
))


# ---------------------------------------------------------------------------
# I7b · te-tm-anim
# ---------------------------------------------------------------------------

TT_SIGMA_C, TT_POLE = 0.8, 0.08
TT_NA_LO, TT_NA_HI = 0.5, 0.95
TT_NA_ZERO = 1.0 / (TT_SIGMA_C * np.sqrt(2.0))  # sin θ = 0.8·NA = 1/√2: TM fringe contrast zero
TT_POLS = (("y", "TE (y, along the lines)"), ("x", "TM (x, across the lines)"), ("unpolarized", "unpolarized"))


def _tt_nas(fast: bool) -> np.ndarray:
    base = np.linspace(TT_NA_LO, TT_NA_HI, 6 if fast else 23)
    return np.array(sorted(set(np.round(base, 4)) | {round(float(TT_NA_ZERO), 4)}))


def _tt_two_beam_visibility() -> float:
    """Point-dipole two-beam contrast of a 1:1 grating: orders 0 and +1 with amplitudes c₀ = 1/2, |c₁| = 1/π."""
    c0, c1 = 0.5, 1.0 / np.pi
    return float(2 * c0 * c1 / (c0 * c0 + c1 * c1))


def te_tm_compute(fast: bool):
    huv = native()
    nas = _tt_nas(fast)
    src = huv.SourceConfig.arf_laser().with_illumination("dipole", TT_SIGMA_C, TT_POLE, 0.0)
    out = {"na": nas, "pol": {pol: {"contrast": [], "x": [], "cut": []} for pol, _l in TT_POLS}, "pitch": []}
    for na in nas:
        pitch = ARF_NM / (2 * TT_SIGMA_C * float(na))  # the ±1 orders of the pole meet at sin θ = 0.8·NA
        out["pitch"].append(pitch)
        optics = huv.OpticsConfig(numerical_aperture=float(na), flare_fraction=0.0)
        mask = huv.MaskConfig.line_space(pitch / 2, pitch)
        grid = huv.GridConfig(size=128, pixel_nm=2 * pitch / 128)
        for pol, _label in TT_POLS:
            eng = huv.SimulationEngine(src, optics, mask, grid=grid, max_kernels=40, source_points_per_axis=41,
                                       vector=huv.VectorSettings(polarization=pol))
            img = eng.compute_aerial_image()
            x, cut = img.cross_section(0.0)
            out["pol"][pol]["contrast"].append(img.image_contrast())
            out["pol"][pol]["x"].append(np.asarray(x, dtype=float) / pitch)
            out["pol"][pol]["cut"].append(np.asarray(cut, dtype=float).copy())
    for pol, _l in TT_POLS:
        out["pol"][pol]["contrast"] = np.array(out["pol"][pol]["contrast"])
    # the two-beam closed forms on a fine NA grid (point dipole: sin θ = 0.8·NA)
    fine = np.linspace(TT_NA_LO, TT_NA_HI, 91)
    v_te = _tt_two_beam_visibility()
    cos2t = 1 - 2 * (TT_SIGMA_C * fine) ** 2
    out["closed"] = {"na": fine, "y": np.full_like(fine, v_te), "x": v_te * np.abs(cos2t), "unpolarized": v_te * np.abs(1 + cos2t) / 2}
    out["na_zero"] = float(TT_NA_ZERO)
    # physical asserts (DESIGN §5.2 I7b)
    te, tm, un = (out["pol"][p]["contrast"] for p in ("y", "x", "unpolarized"))
    assert te.min() > 0.8, f"TE contrast should stay high: min {te.min():.3f}"
    iz = int(np.argmin(np.abs(nas - TT_NA_ZERO)))
    assert tm[iz] < 0.08, f"TM contrast at NA {nas[iz]:.3f} should vanish: {tm[iz]:.3f}"
    assert abs(float(nas[int(np.argmin(tm))]) - TT_NA_ZERO) < 0.03, "the TM zero should sit at sin θ = 1/√2"
    below = nas < TT_NA_ZERO - 0.02
    assert np.all(un[below] < te[below]) and np.all(un[below] > tm[below]), "unpolarized light sits between TE and TM"
    assert abs(te.mean() - v_te) < 0.06, f"engine TE {te.mean():.3f} vs the two-beam closed form {v_te:.3f}"
    return out


def te_tm_frame(d, th: S.Theme, i: int):
    fig = _frame_figure()
    gs = GridSpec(1, 2, figure=fig, width_ratios=[1.15, 1.0], left=0.07, right=0.985, top=0.80, bottom=0.14, wspace=0.3)
    na = float(d["na"][i])
    fig.text(0.07, 0.95, "TE keeps its contrast, TM collapses as the NA grows", fontsize=S.TITLE_PT, fontweight="semibold",
             color=th.ink, ha="left", va="top")
    fig.text(0.07, 0.885, "ArF 193.368 nm · x-dipole σ$_c$ 0.8 · pitch λ/(1.6·NA), so the two orders meet at sin θ = 0.8·NA · "
             "vector imaging, thin mask, image in air", fontsize=S.SMALL_PT, color=th.ink2, ha="left", va="top")
    ax0 = fig.add_subplot(gs[0, 0])
    ax1 = fig.add_subplot(gs[0, 1])
    for ax in (ax0, ax1):
        S.style_axes(ax, th)
    handles, labels = [], []
    for slot, (pol, label) in enumerate(TT_POLS, start=1):
        p = d["pol"][pol]
        (line,) = ax0.plot(p["x"][i], p["cut"][i], color=th.s(slot), zorder=3)
        handles.append(line)
        labels.append(f"{label}: C = {p['contrast'][i]:.2f}")
        ax1.plot(d["na"][: i + 1], p["contrast"][: i + 1], color=th.s(slot), zorder=3)
        ax1.plot(d["closed"]["na"], d["closed"][pol], color=th.ink2, linewidth=S.HAIR_PT * 1.2, linestyle=(0, (1.5, 2.5)), zorder=2)
        S.dot(ax1, [na], [p["contrast"][i]], th.s(slot), th, zorder=5)
    ax0.set_xlim(-1, 1)
    ax0.set_ylim(0, 1.3)
    ax0.set_xlabel("x / pitch")
    ax0.set_ylabel("I / clear field")
    ax0.set_title("aerial image, two periods", fontsize=S.BASE_PT)
    ax0.legend(handles, labels, loc="upper center", ncol=1)
    S.limit_line(ax1, d["na_zero"], th, "TM zero: 2θ = 90°", where=0.97, va="top", side="left")
    ax1.set_xlim(TT_NA_LO, TT_NA_HI)
    ax1.set_ylim(0, 1.0)
    ax1.set_xlabel("numerical aperture (dry)")
    ax1.set_ylabel("image contrast")
    ax1.set_title("contrast vs NA · dotted = two-beam closed forms", fontsize=S.BASE_PT)
    sin_t = TT_SIGMA_C * na
    ax1.text(0.03, 0.05, f"NA {na:.2f} · sin θ = {sin_t:.2f} · 2θ = {2 * np.degrees(np.arcsin(sin_t)):.0f}°",
             transform=ax1.transAxes, ha="left", va="bottom", color=th.ink, fontsize=S.BASE_PT, fontweight="semibold")
    return fig


def te_tm_draw(d, th: S.Theme):
    n = len(d["na"])
    order = list(range(n)) + list(range(n - 2, 0, -1))  # up, then back down without repeating the ends (yoyo)
    return [te_tm_frame(d, th, i) for i in order]


register(FigureSpec(
    name="te-tm-anim",
    group="anim",
    compute=te_tm_compute,
    draw=te_tm_draw,
    alt="Animated NA sweep from 0.5 to 0.95 for dense 1:1 lines under an ArF x-dipole: the TE cut keeps contrast 0.90, "
        "the TM cut falls to zero near NA 0.88 and recovers reversed, unpolarized sits between; a second panel traces "
        "contrast against NA with the two-beam closed forms dotted.",
    caption=("<strong>TE against TM as the NA grows.</strong> Dense 1:1 lines under an ArF x-dipole (σ 0.8): TE keeps its "
             "contrast, TM follows |cos 2θ| and vanishes near NA 0.88, unpolarized light sits between; dotted lines are the "
             "two-beam closed forms. <em>Model: vector (polarized) high-NA imaging ✅ — thin mask, image in air, no "
             "film.</em>"),
    pages=("vector-imaging.md",),
    subdir="anim",
    fmt="apng",
    still_frame=23,
    extra={"duration_ms": 100},
))
