"""Mask and process figures: Bossung curves + ED window, MEEF, and the v1 -> v2 L/S geometry fix."""

from __future__ import annotations

import numpy as np
from matplotlib import pyplot as plt
from matplotlib.patches import Rectangle

import style as S
from common import FigureSpec, native, register

# ---------------------------------------------------------------------------
# 1. Bossung curves and the exposure-defocus window
# ---------------------------------------------------------------------------

PW_CD, PW_PITCH, PW_TOL = 65.0, 180.0, 10.0
PW_DOSE_OFFSETS = (-4.0, -2.0, 0.0, 2.0, 4.0)  # mJ/cm^2 around the computed dose-to-size


def pw_compute(fast: bool):
    huv = native()
    src = huv.SourceConfig.f2_laser(0.7)
    optics = huv.OpticsConfig(numerical_aperture=0.75)  # default 2 % flare
    mask = huv.MaskConfig.line_space(PW_CD, PW_PITCH)
    grid = mask.commensurate_grid(128, 1.0)
    sim = huv.BatchSimulator(src, optics, mask, grid=grid, max_kernels=30)
    doses = list(np.linspace(18.0, 36.0, 10 if fast else 37))
    focuses = list(np.arange(-300.0, 300.1, 25.0 if fast else 10.0))
    pw = sim.process_window(doses, focuses, cd_threshold=0.3, cd_target_nm=PW_CD, cd_tolerance_pct=PW_TOL)
    summary = pw.summary()
    dts = summary["dose_to_size_mj_cm2"]
    fine = np.arange(-300.0, 300.1, 2.5)
    bossung = {}
    for off in PW_DOSE_OFFSETS:
        dose = dts + off
        bossung[dose] = np.array([np.nan if (c := pw.cd_at(dose, float(z))) is None else c for z in fine])
    dof, el = pw.el_vs_dof(101)
    return {
        "focus": np.array(focuses),
        "fine_focus": fine,
        "bossung": bossung,
        "dose_limits": [None if lim is None else tuple(lim) for lim in pw.dose_limits],
        "summary": summary,
        "max_area": pw.max_area_rectangle(),
        "el_dof": (np.asarray(dof), np.asarray(el)),
    }


def pw_draw(d, th: S.Theme):
    fig, axes = plt.subplots(1, 3, figsize=(S.WIDE_WIDTH + 0.6, 4.0),
                             gridspec_kw={"wspace": 0.4, "width_ratios": [1.15, 1.15, 0.9]})
    s = d["summary"]
    dts = s["dose_to_size_mj_cm2"]
    # (a) Bossung curves: ordered doses -> the ordinal one-hue ramp
    ax = axes[0]
    S.style_axes(ax, th)
    ax.axhspan(PW_CD * (1 - PW_TOL / 100), PW_CD * (1 + PW_TOL / 100), color=th.band, zorder=0, lw=0)
    cols = S.ordinal(th, len(d["bossung"]))
    handles, labels = [], []
    for col, (dose, y) in zip(cols, sorted(d["bossung"].items())):
        ax.plot(d["fine_focus"], y, color=col, zorder=3)
        handles.append(plt.Line2D([], [], color=col))
        labels.append(f"{dose:.1f}" + ("  (dose-to-size)" if abs(dose - dts) < 1e-6 else ""))
    ax.set_xlim(-300, 300)
    ax.set_ylim(30, 100)
    ax.set_xlabel("Focus (nm)")
    ax.set_ylabel("Printed line CD (nm)")
    ax.set_title("(a) Bossung curves")
    ax.text(298, PW_CD * (1 + PW_TOL / 100) + 1, "±10 % CD", color=th.ink2, fontsize=S.SMALL_PT, ha="right",
            va="bottom")
    ax.legend(handles, labels, title="Dose (mJ/cm²)", title_fontsize=S.SMALL_PT, loc="upper center",
              bbox_to_anchor=(0.5, -0.2), ncol=2, columnspacing=1.0)
    # (b) process window in focus-dose space
    ax2 = axes[1]
    S.style_axes(ax2, th, grid="both")
    z = d["focus"]
    lo = np.array([np.nan if lim is None else lim[0] for lim in d["dose_limits"]])
    hi = np.array([np.nan if lim is None else lim[1] for lim in d["dose_limits"]])
    ax2.fill_between(z, lo, hi, color=th.s(1), alpha=0.18, lw=0, zorder=1)
    ax2.plot(z, lo, color=th.s(1), linewidth=S.LINE_PT * 0.75, zorder=2)
    ax2.plot(z, hi, color=th.s(1), linewidth=S.LINE_PT * 0.75, zorder=2)
    r5 = s["dof_at_5pct_el"]
    ma = d["max_area"]
    for rect, slot in ((r5, 2), (ma, 3)):
        ax2.add_patch(Rectangle((rect["focus_min_nm"], rect["dose_min_mj_cm2"]), rect["dof_nm"],
                                rect["dose_max_mj_cm2"] - rect["dose_min_mj_cm2"], fill=False,
                                edgecolor=th.s(slot), linewidth=S.LINE_PT, zorder=4))
    ax2.set_xlim(-300, 300)
    finite = np.isfinite(lo) & np.isfinite(hi)
    ax2.set_ylim(np.floor(np.nanmin(lo[finite]) - 0.5), np.ceil(np.nanmax(hi[finite]) + 0.5))
    ax2.set_xlabel("Focus (nm)")
    ax2.set_ylabel("Dose (mJ/cm²)")
    ax2.set_title("(b) Exposure-defocus window")
    ax2.legend([plt.Rectangle((0, 0), 1, 1, facecolor=th.s(1), alpha=0.35, edgecolor=th.s(1)),
                plt.Line2D([], [], color=th.s(2)), plt.Line2D([], [], color=th.s(3))],
               ["CD within ±10 %",
                f"largest DOF at 5 % EL: {r5['dof_nm']:.0f} nm",
                f"max DOF × EL: {ma['dof_nm']:.0f} nm, {ma['exposure_latitude_pct']:.1f} % EL"],
               loc="upper center", bbox_to_anchor=(0.5, -0.2), ncol=1)
    # (c) EL vs DOF
    ax3 = axes[2]
    S.style_axes(ax3, th)
    dof, el = d["el_dof"]
    ax3.plot(dof, el, color=th.s(1), zorder=3)
    ax3.set_xlim(0, float(dof[el > 0.05].max()) * 1.15 if np.any(el > 0.05) else float(dof.max()))
    ax3.set_ylim(0, float(np.ceil(el.max() / 2) * 2))
    ax3.set_xlabel("Depth of focus (nm)")
    ax3.set_ylabel("Exposure latitude (%)")
    ax3.set_title("(c) EL versus DOF")
    S.dot(ax3, [r5["dof_nm"]], [5.0], th.s(2), th)
    S.end_label(ax3, r5["dof_nm"], 5.0, f"{r5['dof_nm']:.0f} nm at 5 %", th, dy_px=-1)
    fig.subplots_adjust(bottom=0.33)
    return fig


register(FigureSpec(
    name="masks-process-window",
    group="masks",
    compute=pw_compute,
    draw=pw_draw,
    alt="Three panels for 65 nm lines on a 180 nm pitch: Bossung curves of printed CD versus focus for five "
        "doses; the focus-dose window where the CD stays within ten percent, with the 5 percent "
        "exposure-latitude and maximum-area rectangles; and exposure latitude versus depth of focus.",
    caption="Dose-aware process window of 65 nm lines on a 180 nm pitch (F<sub>2</sub> 157.63 nm, NA 0.75, σ 0.7, "
            "2 % flare), from <code>BatchSimulator.process_window</code>: printed CD per (dose, focus) with a "
            "constant-threshold resist, Bossung curves around the dose-to-size, continuous dose limits, the "
            "largest-DOF window at 5 % exposure latitude, the max-area rectangle and the EL–DOF curve. "
            "Model: process window / ED analysis 🔶 (constant threshold; no resist blur, diffusion or "
            "development; CD on the y = 0 cut).",
    pages=("masks-and-metrics.md",),
))


# ---------------------------------------------------------------------------
# 2. MEEF: wafer CD vs mask CD at three pitches
# ---------------------------------------------------------------------------

MEEF_PITCHES = (180.0, 240.0, 360.0)


def _cd(huv, src, optics, cd_mask, pitch, threshold):
    mask = huv.MaskConfig.line_space(cd_mask, pitch)
    grid = mask.commensurate_grid(128, 1.0)
    img = huv.SimulationEngine(src, optics, mask, grid=grid, max_kernels=30).compute_aerial_image(0.0)
    return img.cd(threshold, "dark")


def meef_compute(fast: bool):
    huv = native()
    src = huv.SourceConfig.f2_laser(0.7)
    optics = huv.OpticsConfig(numerical_aperture=0.75)
    out = {}
    for pitch in MEEF_PITCHES:
        nominal = pitch / 2
        lo, hi = 0.05, 0.95  # bisection on the threshold that sizes the nominal line (dose-to-size)
        for _ in range(50):
            t = 0.5 * (lo + hi)
            cd = _cd(huv, src, optics, nominal, pitch, t)
            if cd is None or cd > nominal:
                hi = t
            else:
                lo = t
        t = 0.5 * (lo + hi)
        mask_cds = np.arange(nominal - 25.0, nominal + 25.01, 5.0 if fast else 1.0)
        wafer = np.array([np.nan if (c := _cd(huv, src, optics, m, pitch, t)) is None else c for m in mask_cds])
        out[pitch] = {"threshold": t, "mask": mask_cds, "wafer": wafer}
    return out


def meef_draw(d, th: S.Theme):
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 4.0), gridspec_kw={"wspace": 0.3})
    for ax in (ax1, ax2):
        S.style_axes(ax, th)
    handles, labels = [], []
    for slot, pitch in enumerate(MEEF_PITCHES, start=1):
        e = d[pitch]
        k1 = pitch / 2 * 0.75 / 157.63
        bias = e["mask"] - pitch / 2
        ax1.plot(bias, e["wafer"] - pitch / 2, color=th.s(slot), zorder=3)
        meef = np.gradient(e["wafer"], e["mask"])
        ax2.plot(bias, meef, color=th.s(slot), zorder=3)
        m0 = float(np.interp(0.0, bias, meef))
        S.dot(ax2, [0.0], [m0], th.s(slot), th, clip_on=False)
        handles.append(plt.Line2D([], [], color=th.s(slot)))
        labels.append(f"pitch {pitch:.0f} nm, 1:1 lines (" + r"$k_1$" + f" {k1:.2f}); MEEF at nominal {m0:.2f}")
    lim = 25
    ax1.plot([-lim, lim], [-lim, lim], color=th.ink2, linewidth=S.HAIR_PT * 1.3, linestyle=(0, (4, 3)), zorder=2)
    ax1.text(-lim + 1, 42, "dashed: MEEF = 1 (wafer follows mask)", color=th.ink2, fontsize=S.SMALL_PT, ha="left",
             va="top")
    ax1.set_xlim(-lim, lim)
    ax1.set_ylim(-45, 45)
    ax1.set_xlabel("Mask CD bias, wafer scale (nm)")
    ax1.set_ylabel("Printed CD change (nm)")
    ax1.set_title("(a) Printed CD vs mask bias")
    ax2.axhline(1.0, color=th.ink2, linewidth=S.HAIR_PT * 1.3, linestyle=(0, (4, 3)), zorder=2)
    ax2.set_xlim(-lim, lim)
    ax2.set_ylim(0, 4)
    ax2.set_xlabel("Mask CD bias, wafer scale (nm)")
    ax2.set_ylabel("MEEF = ∂CD(wafer) / ∂CD(mask)")
    ax2.set_title("(b) Mask error enhancement factor")
    ax2.annotate("180 nm: the space closes,\nMEEF climbs past 4", xy=(16.2, 3.5), xytext=(13, 3.5),
                 color=th.ink2, fontsize=S.SMALL_PT, ha="right", va="center",
                 arrowprops=dict(arrowstyle="-", color=th.axis, lw=S.HAIR_PT, shrinkA=1, shrinkB=1))
    fig.legend(handles, labels, loc="lower center", ncol=1, bbox_to_anchor=(0.5, 0.0))
    fig.subplots_adjust(bottom=0.3)
    return fig


register(FigureSpec(
    name="masks-meef",
    group="masks",
    compute=meef_compute,
    draw=meef_draw,
    alt="Two charts for 1:1 lines at pitches 180, 240 and 360 nm: printed CD change versus mask bias against "
        "the MEEF = 1 diagonal, and the mask error enhancement factor, which is about 1 at the larger pitches "
        "and well above 1 at 180 nm.",
    caption="Mask error enhancement. For 1:1 lines at three pitches (F<sub>2</sub> 157.63 nm, NA 0.75, σ 0.7, 2 % "
            "flare) the threshold is set so the nominal line prints on size; the mask line is then biased by ±25 nm "
            "(wafer scale) and the printed CD measured with the sub-pixel crossing finder. MEEF = dCD<sub>wafer"
            "</sub>/dCD<sub>mask</sub> rises well above 1 as k<sub>1</sub> falls. Models: exact thin-mask spectrum "
            "✅, scalar imaging ✅, CD metrics ✅; constant-threshold resist 🔶.",
    pages=("masks-and-metrics.md",),
))


# ---------------------------------------------------------------------------
# 3. What was fixed: the v1 L/S geometry vs the v2 periodic unit cell
# ---------------------------------------------------------------------------

GEO_CD, GEO_PITCH, GEO_FIELD = 65.0, 180.0, 256.0  # v1 simulate_line_space default: 256 px x 1 nm


def _v1_line_space_profile(cd: float, pitch: float, field: float, n: int) -> np.ndarray:
    """Re-implementation of v1 ``Mask::line_space`` + ``rasterize`` (commit 8f46982) on one row.

    v1 painted nine ABSORBER rectangles of width ``pitch - cd`` centred on x = k*pitch
    (k = -4..4) on a clear background, sampled at pixel centres (no antialiasing), on
    whatever field the grid had; the FFT then repeats that field periodically.
    """
    pixel = field / n
    x = -field / 2 + (np.arange(n) + 0.5) * pixel
    t = np.ones(n)
    width = pitch - cd
    for k in range(-4, 5):
        t[(x >= k * pitch - width / 2) & (x <= k * pitch + width / 2)] = 0.0
    return t


def geometry_compute(fast: bool):
    huv = native()
    n_v1 = 256
    v1 = _v1_line_space_profile(GEO_CD, GEO_PITCH, GEO_FIELD, n_v1)
    mask = huv.MaskConfig.line_space(GEO_CD, GEO_PITCH)
    grid = mask.commensurate_grid(1024, 0.2)  # fine pixels: antialiased edge pixels stay invisible at this scale
    v2 = np.real(mask.rasterize(grid))[0]
    return {"v1": v1, "v1_pixel": GEO_FIELD / n_v1, "v2": v2, "v2_pixel": grid.pixel_nm,
            "v2_field": grid.size * grid.pixel_nm}


def _periodic_steps(profile, pixel, field, x_lo, x_hi):
    """Pixel-step outline of a periodically repeated 1-D raster over [x_lo, x_hi]."""
    n = len(profile)
    k0 = int(np.floor((x_lo + field / 2) / field)) - 1
    k1 = int(np.ceil((x_hi + field / 2) / field)) + 1
    xs, ys = [], []
    for k in range(k0, k1 + 1):
        x0 = -field / 2 + k * field
        xs.append(x0 + np.arange(n) * pixel)
        ys.append(profile)
    xs = np.concatenate(xs)
    ys = np.concatenate(ys)
    return np.append(xs, xs[-1] + pixel), np.append(ys, ys[-1])


def geometry_draw(d, th: S.Theme):
    x_lo, x_hi = -400.0, 400.0
    fig, axes = plt.subplots(3, 1, figsize=(S.FULL_WIDTH, 4.6), sharex=True, gridspec_kw={"hspace": 0.95})
    rows = [
        ("Intended: 65 nm opaque lines on a 180 nm pitch", None),
        ("v1 (re-drawn from the v1 source, commit 8f46982): absorber = pitch − CD, laid on the 256 nm default "
         "field; the FFT repeats that field every 256 nm, leaving a 12 nm sliver at the seam", "v1"),
        ("v2 (rasterized by today's MaskConfig.line_space): one 180 nm unit cell, commensurate grid, "
         "antialiased edges", "v2"),
    ]
    for ax, (title, key) in zip(axes, rows):
        if key is None:
            xs = np.arange(x_lo - 200, x_hi + 200, 0.25)
            phase = np.mod(xs + GEO_PITCH / 2, GEO_PITCH) - GEO_PITCH / 2
            ys = np.where(np.abs(phase) <= GEO_CD / 2, 0.0, 1.0)
            ax.fill_between(xs, 0, 1 - ys, step=None, color=th.ink2, lw=0)
        else:
            field = GEO_FIELD if key == "v1" else d["v2_field"]
            xs, ys = _periodic_steps(d[key], d[f"{key}_pixel"], field, x_lo, x_hi)
            ax.fill_between(xs, 0, 1 - ys, step="post", color=th.ink2, lw=0)
            for k in range(-3, 4):  # field (FFT period) boundaries
                xb = -field / 2 + k * field
                if x_lo < xb < x_hi:
                    ax.axvline(xb, color=th.s(2), linewidth=S.HAIR_PT * 1.3, linestyle=(0, (3, 2)))
        ax.set_ylim(0, 1)
        ax.set_xlim(x_lo, x_hi)
        ax.set_yticks([])
        for side in ("left", "top", "right"):
            ax.spines[side].set_visible(False)
        ax.spines["bottom"].set_color(th.axis)
        ax.set_title(title, fontsize=S.SMALL_PT, fontweight="normal", color=th.ink2, loc="left", wrap=True)
    axes[-1].set_xlabel("x (nm). Filled = absorber; dashed = edge of the simulated field (the FFT period)")
    return fig


register(FigureSpec(
    name="masks-ls-geometry-fix",
    group="masks",
    compute=geometry_compute,
    draw=geometry_draw,
    alt="Three mask transmission strips for 65 nm lines on a 180 nm pitch: the intended pattern; the v1 raster "
        "with 115 nm absorbers repeating every 256 nm and a thin sliver at the field seam; and the v2 raster "
        "that repeats the 180 nm unit cell exactly.",
    caption="What was fixed in the mask model. Top: the intended 65 nm lines on a 180 nm pitch. Middle: what v1 "
            "rasterized for <code>line_space(65, 180)</code> on the old 256 nm default field — re-drawn from the v1 "
            "source, not simulated: the absorber was the 115 nm space width, and because the FFT treats the field "
            "as one period, the image was that of a 256 nm-period pattern with a 12 nm absorber sliver at the seam. "
            "Bottom: v2 rasterizes one 180 nm unit cell on a commensurate grid (antialiased; the exact analytic "
            "spectrum is used for imaging). Exact thin-mask spectrum ✅; Kirchhoff thin-mask model 🔶.",
    pages=("masks-and-metrics.md",),
))
