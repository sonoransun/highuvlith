"""Mask and process figures: Bossung curves + ED window, MEEF, the v1 -> v2 L/S geometry fix,
the mask primitives with their exact spectra (I9) and commensurate vs incommensurate grids (I9b).

Captions follow the DESIGN §4.6 grammar (a ``<strong>`` lead, ≤ 3 sentences, an ``<em>Model: …</em>``
trailer whose labels resolve to capability-matrix rows through ``MASK_ROWS``).
"""

from __future__ import annotations

import warnings

import numpy as np
from matplotlib import pyplot as plt
from matplotlib.patches import Circle, Rectangle

import style as S
from common import FigureSpec, native, register

#: Caption labels of this file → the exact capability-matrix rows they abbreviate (DESIGN §5.0).
MASK_ROWS = {
    "Hopkins imaging": "Hopkins imaging (factorized TCC/SOCS)",
    "exact thin-mask spectra": "Exact thin-mask spectrum",
    "commensurate grids": "Commensurate periodic grids",
    "mask model": "Mask model and types",
    "CD metrics": "CD / ILS / NILS / MEEF metrics",
    "MEEF metrics": "CD / ILS / NILS / MEEF metrics",
}
_EXTRA = {"model_rows": MASK_ROWS}

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
    caption="<strong>The dose-aware process window of 65 nm lines on a 180 nm pitch.</strong> F<sub>2</sub> "
            "157.63 nm, NA 0.75, σ 0.7, 2 % flare, from <code>BatchSimulator.process_window</code>: printed CD per "
            "(dose, focus) with a constant-threshold resist, Bossung curves around the dose-to-size, the continuous "
            "dose limits, the largest-DOF window at 5 % exposure latitude, the max-area rectangle and the EL–DOF "
            "curve. <em>Model: process window / ED analysis 🔶 (constant threshold; no resist blur, diffusion or "
            "development; CD on the y = 0 cut).</em>",
    pages=("metrics-and-process-window.md", "python-api.md"),
    hero_on=("metrics-and-process-window.md",),
    extra=_EXTRA,
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
    caption="<strong>Mask error enhancement grows as k<sub>1</sub> falls.</strong> For 1:1 lines at three pitches "
            "(F<sub>2</sub> 157.63 nm, NA 0.75, σ 0.7, 2 % flare) the threshold is set so the nominal line prints on "
            "size; the mask line is then biased by ±25 nm (wafer scale) and the printed CD measured with the "
            "sub-pixel crossing finder. MEEF = dCD<sub>wafer</sub>/dCD<sub>mask</sub> rises well above 1 at 180 nm "
            "pitch. <em>Model: exact thin-mask spectra ✅ · Hopkins imaging ✅ · MEEF metrics ✅ · process window / "
            "ED analysis 🔶 (constant-threshold resist).</em>",
    pages=("metrics-and-process-window.md",),
    extra=_EXTRA,
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
    caption="<strong>What v1 got wrong and v2 fixed.</strong> Top: the intended 65 nm lines on a 180 nm pitch; "
            "middle: what v1 rasterized for <code>line_space(65, 180)</code> on the old 256 nm default field, re-drawn "
            "from the v1 source and not simulated — the absorber was the 115 nm space width and the FFT's one-period "
            "field made it a 256 nm pattern with a 12 nm sliver at the seam; bottom: v2 rasterizes one 180 nm unit "
            "cell on a commensurate grid (antialiased; imaging uses the exact analytic spectrum). <em>Model: exact "
            "thin-mask spectra ✅ · mask model 🔶 (Kirchhoff thin mask).</em>",
    pages=("masks.md",),
    extra=_EXTRA,
))


# ---------------------------------------------------------------------------
# 4. I9 — mask primitives and their exact spectra (the canonical F2 / NA 0.75 / σ 0.7 scene)
# ---------------------------------------------------------------------------

PRIM_LAMBDA, PRIM_NA, PRIM_SIGMA = 157.63, 0.75, 0.7
PRIM_N, PRIM_TARGET = 256, 1.5  # 256 px, pixel ≥ 1.5 nm -> two 180 nm periods (1.40625 nm, 360 nm field)
SERIF_N = 1024  # the non-periodic serif line sits on a 1440 nm field so its spectrum has 0.69 cycles/µm bins
SERIF_VIEW = 180.0  # the raster panels show ±180 nm of it
SERIF_LINE = (0.0, 0.0, 65.0, 260.0)  # centre x, y, width, height (nm)
SERIF_HEADS = [[(-52.0, 100.0), (52.0, 100.0), (52.0, 134.0), (-52.0, 134.0)],
               [(-52.0, -134.0), (52.0, -134.0), (52.0, -100.0), (-52.0, -100.0)]]


def _spectrum_1d(spec, n):
    """|S(m)| / N² along f_x at f_y = 0 for bins m = -n … n (fft2 layout: row 0 is f_y = 0)."""
    size = spec.shape[1]
    row = np.abs(spec[0]) / float(size) ** 2
    m = np.arange(-n, n + 1)
    return m, row[np.mod(m, size)]


def primitives_compute(fast: bool):
    huv = native()
    out = {"lambda": PRIM_LAMBDA, "na": PRIM_NA, "sigma": PRIM_SIGMA, "panels": {}}
    # (a) the canonical grating on a two-period commensurate field
    ls = huv.MaskConfig.line_space(65.0, 180.0)
    grid = ls.commensurate_grid(PRIM_N, PRIM_TARGET)
    assert grid.is_commensurate_with(180.0) and round(grid.periods_in_field(180.0)) == 2, grid
    field = grid.field_size_nm()
    pixel = grid.pixel_nm
    out["field"] = field
    out["pixel"] = pixel
    t_ls = np.abs(ls.rasterize(grid))
    s_ls = ls.spectrum(grid)
    out["panels"]["ls"] = {"t": t_ls, "method": ls.spectrum_method(grid), "title": "line_space(65, 180)"}
    # physical asserts: the two-period field holds the grating orders at even bins only, and the DC term
    # is the clear-field fraction; the analytic spectrum makes the off-harmonic bins exactly zero.
    n2 = float(grid.size) ** 2
    dc = abs(s_ls[0, 0]) / n2
    assert abs(dc - (1.0 - 65.0 / 180.0)) < 1e-9, dc
    odd = np.abs(s_ls[0, 1::2])
    assert odd.max() == 0.0, f"off-harmonic orders are not exactly zero: max {odd.max():.3e}"
    assert np.abs(s_ls[0, 2]) > 0.1 * n2, "first grating order missing at bin 2"
    out["ls_spectrum"] = _spectrum_1d(s_ls, 12)
    # (b) a contact-hole array (dark field) on the same field
    ch = huv.MaskConfig.contact_hole(90.0, 180.0)
    gch = ch.commensurate_grid(PRIM_N, PRIM_TARGET)
    assert abs(gch.pixel_nm - pixel) < 1e-12
    out["panels"]["contacts"] = {"t": np.abs(ch.rasterize(gch)), "method": ch.spectrum_method(gch),
                                 "title": "contact_hole(90, 180)"}
    # (c) a line with hammerhead serifs: a Rect and two overlapping Polygons, painted in order
    gfree = huv.GridConfig(SERIF_N, pixel)  # a wide non-periodic field: fine bins in the 2-D spectrum
    out["serif_field"] = gfree.field_size_nm()
    features = [{"type": "rect", "x": SERIF_LINE[0], "y": SERIF_LINE[1], "w": SERIF_LINE[2], "h": SERIF_LINE[3]}]
    features += [{"type": "polygon", "vertices": v} for v in SERIF_HEADS]
    serif = huv.MaskConfig.from_features(features)
    t_serif = serif.rasterize(gfree)
    s_serif = serif.spectrum(gfree)
    out["panels"]["serif"] = {"t": np.abs(t_serif), "method": serif.spectrum_method(gfree),
                              "title": "Rect + Polygon serifs"}
    # (d, e) the same geometry as an attenuated PSM: amplitude √T and the 180° phase of the absorber
    att = huv.MaskConfig.from_features(features, mask_type="att_psm", transmission=0.06, phase_deg=180.0)
    t_att = att.rasterize(gfree)
    amp, phase = np.abs(t_att), np.degrees(np.angle(t_att))
    inside = amp < 0.5
    assert np.allclose(amp[inside], np.sqrt(0.06), atol=1e-12), "AttPSM absorber amplitude is not sqrt(T)"
    assert np.allclose(np.abs(phase[inside]), 180.0, atol=1e-9), "AttPSM absorber phase is not 180°"
    out["panels"]["att_amp"] = {"t": amp, "method": att.spectrum_method(gfree), "title": "AttPSM |t| (T = 6 %)"}
    out["panels"]["att_phase"] = {"t": phase, "method": att.spectrum_method(gfree), "title": "AttPSM arg t (°)"}
    # (g) the serif pattern's 2-D spectrum with the pupil circles
    f = np.fft.fftshift(np.fft.fftfreq(gfree.size, d=pixel)) * 1000.0  # cycles per µm
    out["serif_spectrum"] = {"f": f, "s": np.fft.fftshift(np.abs(s_serif)) / float(gfree.size) ** 2}
    out["f_na"] = PRIM_NA / PRIM_LAMBDA * 1000.0
    out["f_support"] = (1 + PRIM_SIGMA) * PRIM_NA / PRIM_LAMBDA * 1000.0
    return out


def primitives_draw(d, th: S.Theme):
    fig = plt.figure(figsize=(S.WIDE_WIDTH + 0.4, 6.0))
    gs = fig.add_gridspec(2, 5, height_ratios=[0.8, 1.45], hspace=0.55, wspace=0.42, top=0.95, bottom=0.08)
    cmap = S.field_cmap()
    half = d["field"] / 2
    ext = [-half, half, -half, half]
    shalf = d["serif_field"] / 2
    sext = [-shalf, shalf, -shalf, shalf]
    order = ("ls", "contacts", "serif", "att_amp", "att_phase")
    letters = "abcde"
    for k, key in enumerate(order):
        ax = fig.add_subplot(gs[0, k])
        p = d["panels"][key]
        e = ext if key in ("ls", "contacts") else sext
        if key == "att_phase":
            im = ax.imshow(p["t"], origin="lower", extent=e, cmap=S.diverging_cmap(th), vmin=-180, vmax=180,
                           interpolation="nearest")
        else:
            im = ax.imshow(p["t"], origin="lower", extent=e, cmap=cmap, vmin=0, vmax=1, interpolation="nearest")
        if key not in ("ls", "contacts"):
            ax.set_xlim(-SERIF_VIEW, SERIF_VIEW)
            ax.set_ylim(-SERIF_VIEW, SERIF_VIEW)
        S.image_axes(ax, th)
        ax.set_title(f"({letters[k]}) {p['title']}", fontsize=S.SMALL_PT + 0.5, loc="left")
        ax.set_xlabel("x (nm)")
        if k == 0:
            ax.set_ylabel("y (nm)")
        else:
            ax.tick_params(labelleft=False)
        ax.text(0.5, -0.42, f"spectrum: {p['method']}", transform=ax.transAxes, ha="center", va="top",
                color=th.ink2, fontsize=S.SMALL_PT - 0.5)
        if k in (3, 4):
            cb = S.colorbar(fig, im, ax, th, "", fraction=0.05, pad=0.04)
            if k == 4:
                cb.set_ticks([-180, 0, 180])
    # (f) the grating's exact spectrum on the two-period field
    axf = fig.add_subplot(gs[1, 0:3])
    S.style_axes(axf, th, grid="y")
    m, s = d["ls_spectrum"]
    nonzero = s > 0
    floor = 1e-4
    axf.set_yscale("log")
    axf.set_ylim(floor, 1.5)
    axf.vlines(m[nonzero], floor, s[nonzero], color=th.s(1), linewidth=S.LINE_PT, zorder=3)
    axf.plot(m[nonzero], s[nonzero], linestyle="none", marker="o", markersize=S.MARKER_PT * 0.75, color=th.s(1),
             zorder=4)
    axf.plot(m[~nonzero], np.full((~nonzero).sum(), floor * 1.25), linestyle="none", marker="o",
             markersize=S.MARKER_PT * 0.75, markerfacecolor="none", markeredgecolor=th.s(2), zorder=4)
    axf.text(-12.3, floor * 1.9, "odd m: exactly 0 (off-harmonic bins)", color=th.s(2), fontsize=S.SMALL_PT,
             ha="left", va="bottom")
    bins_per_um = d["field"] / 1000.0  # bin m ↔ m / field cycles per nm
    for f_, lab, side in ((d["f_na"], "NA/λ", "left"), (d["f_support"], "(1+σ)·NA/λ", "right")):
        mb = f_ * bins_per_um
        for sign in (1, -1):
            S.limit_line(axf, sign * mb, th, lab if sign > 0 else None, where=0.56, side=side)
    axf.set_xlim(-12.5, 12.5)
    axf.set_xticks(range(-12, 13, 2))
    axf.set_xlabel(f"FFT bin m  (f = m / {d['field']:.0f} nm; grating orders at even m = 2q)")
    axf.set_ylabel("|S(m)| / N²  (clear field = 1)")
    axf.set_title("(f) line_space(65, 180): |S| along f$_x$ on the two-period field", fontsize=S.SMALL_PT + 0.5,
                  loc="left")
    # (g) the serif pattern's 2-D spectrum inside the pupil circles
    axg = fig.add_subplot(gs[1, 3:5])
    sp = d["serif_spectrum"]
    f = sp["f"]
    logs = np.log10(np.maximum(sp["s"], 1e-7))
    lim = 1.9 * d["f_support"]
    im = axg.imshow(logs, origin="lower", extent=[f[0], f[-1], f[0], f[-1]], cmap=cmap, vmin=-6, vmax=0,
                    interpolation="nearest")
    for r, lab in ((d["f_na"], "NA/λ"), (d["f_support"], "(1+σ)NA/λ")):
        axg.add_patch(Circle((0, 0), r, fill=False, edgecolor=th.ink2, linewidth=S.HAIR_PT * 1.3,
                             linestyle=(0, (4, 3))))
        axg.text(0.0, r + 0.25, lab, color=th.ink2, fontsize=S.SMALL_PT - 0.5, ha="center", va="bottom",
                 bbox=dict(boxstyle="round,pad=0.1", facecolor=th.surface, edgecolor="none", alpha=0.8))
    axg.set_xlim(-lim, lim)
    axg.set_ylim(-lim, lim)
    axg.set_aspect("equal")
    S.image_axes(axg, th)
    axg.set_xlabel("f$_x$ (cycles/µm)")
    axg.set_ylabel("f$_y$ (cycles/µm)")
    axg.set_title("(g) serif line: log₁₀ |S| / N² and the pupil".replace("₁₀", "$_{10}$"),
                  fontsize=S.SMALL_PT + 0.5, loc="left")
    S.colorbar(fig, im, axg, th, "log₁₀ |S| / N²".replace("₁₀", "$_{10}$"), fraction=0.05, pad=0.04)
    axg.set_xlabel(f"f$_x$ (cycles/µm) — F$_2$ {d['lambda']:.2f} nm, NA {d['na']:.2f}, σ {d['sigma']:.1f}")
    return fig


register(FigureSpec(
    name="masks-primitives-spectra",
    group="masks",
    compute=primitives_compute,
    draw=primitives_draw,
    alt="Top: rasterized transmittance of a 65/180 nm grating, a 90 nm contact array, a serif line, and the "
        "same line as an attenuated PSM (amplitude 0.24, phase 180 degrees). Bottom: the grating's spectrum "
        "with exactly zero off-harmonic orders, and the serif pattern's 2-D spectrum inside the pupil circles."
    ,
    caption="<strong>Mask primitives and their exact spectra.</strong> Rasterized transmittance from "
            "<code>MaskConfig.rasterize</code> for the canonical 65/180 nm grating on a two-period commensurate "
            "field, a 90/180 nm contact-hole array, a line with two overlapping hammerhead serifs (features painted "
            "in order) and the same serif line as an attenuated PSM (|t| = √0.06, phase 180°); each panel names the "
            "<code>spectrum_method</code> the engine uses. Below, |S| from <code>MaskConfig.spectrum</code>: the "
            "grating's off-harmonic orders are exactly zero and its first orders sit outside NA/λ but inside the "
            "(1+σ)NA/λ support of F<sub>2</sub> 157.63 nm, NA 0.75, σ 0.7, which is why the canonical run images "
            "them only through off-axis source points. <em>Model: exact thin-mask spectra ✅ · antialiased "
            "rasterization ✅ · mask model 🔶 (Kirchhoff thin mask; alternating-PSM phase regions not modelled).</em>",
    pages=("masks.md", "python-api.md"),
    extra=_EXTRA,
))


# ---------------------------------------------------------------------------
# 5. I9b — commensurate versus incommensurate grids on the canonical run
# ---------------------------------------------------------------------------


def _run_grid(huv, src, opt, mask, grid):
    """Raster row, spectrum row, aerial cut and metrics of ``mask`` on ``grid``; the engine's warnings are kept."""
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        eng = huv.SimulationEngine(src, opt, mask, grid=grid)
        img = eng.compute_aerial_image(0.0)
    x, prof = np.asarray(img.x_nm), np.asarray(img.intensity)[img.shape[0] // 2]
    t = np.abs(mask.rasterize(grid))[0]
    s = np.abs(mask.spectrum(grid))[0] / float(grid.size) ** 2
    return {
        "pixel": grid.pixel_nm, "field": grid.field_size_nm(), "periods": grid.periods_in_field(180.0),
        "commensurate": grid.is_commensurate_with(180.0),
        "t": t, "s": s, "f": np.fft.fftfreq(grid.size, d=grid.pixel_nm) * 1000.0,
        "x": x, "i": prof, "contrast": img.image_contrast(), "cd": img.cd(0.3), "nils": img.nils_periodic(0.3),
        "warnings": [str(w.message) for w in caught if issubclass(w.category, UserWarning)],
    }


def commensurate_compute(fast: bool):
    huv = native()
    src = huv.SourceConfig.f2_laser(0.7)
    opt = huv.OpticsConfig(numerical_aperture=0.75)
    mask = huv.MaskConfig.line_space(65.0, 180.0)
    inc = _run_grid(huv, src, opt, mask, huv.GridConfig(256, 1.0))
    com = _run_grid(huv, src, opt, mask, mask.commensurate_grid(256, 1.0))
    # behavioural asserts: the engine warns on the incommensurate field and is silent on the commensurate one;
    # the commensurate field holds a whole number of periods, the default field does not.
    assert any("incommensurate" in w for w in inc["warnings"]), inc["warnings"]
    assert not com["warnings"], com["warnings"]
    assert not inc["commensurate"] and abs(inc["periods"] - round(inc["periods"])) > 0.1
    assert com["commensurate"] and round(com["periods"]) == 1
    # the truncated grating leaks into every FFT bin; the commensurate one has orders at multiples of 1/180 nm only
    inc_bins = int((inc["s"] > 1e-6).sum())
    assert inc_bins > 0.9 * inc["s"].size, inc_bins
    assert com["cd"] is not None and inc["cd"] is not None
    return {"inc": inc, "com": com, "f_na": PRIM_NA / PRIM_LAMBDA * 1000.0,
            "f_support": (1 + PRIM_SIGMA) * PRIM_NA / PRIM_LAMBDA * 1000.0}


def _tile_periodic(x, y, field, reps=(-1, 0, 1)):
    xs = np.concatenate([x + k * field for k in reps])
    ys = np.concatenate([y for _ in reps])
    return xs, ys


def commensurate_draw(d, th: S.Theme):
    fig, axes = plt.subplots(3, 2, figsize=(S.FULL_WIDTH, 7.0), gridspec_kw={"hspace": 0.78, "wspace": 0.26,
                                                                             "height_ratios": [0.7, 1.1, 1.1],
                                                                             "top": 0.9, "bottom": 0.085})
    x_lo, x_hi = -312.0, 312.0  # both windows end in clear regions of every tiled field
    cols = (("inc", f"(a) incommensurate: 256 px × 1 nm = 256 nm field\n({d['inc']['periods']:.2f} periods)", 2),
            ("com", f"(b) commensurate: 256 px × {d['com']['pixel']:.4g} nm = 180 nm field\n(1 period)", 1))
    for j, (key, title, slot) in enumerate(cols):
        r = d[key]
        field = r["field"]
        # row 1: the raster, repeated as the FFT sees it
        ax = axes[0, j]
        xs = np.concatenate([(-field / 2 + k * field) + np.arange(r["t"].size) * r["pixel"] for k in (-2, -1, 0, 1, 2)])
        ys = np.concatenate([r["t"] for _ in range(5)])
        keep = (xs >= x_lo) & (xs <= x_hi)
        xs, ys = xs[keep], ys[keep]
        ax.fill_between(np.append(xs, xs[-1] + r["pixel"]), 0, 1 - np.append(ys, ys[-1]), step="post",
                        color=th.ink2, lw=0, edgecolor="none", antialiased=False)
        for k in range(-3, 4):
            xb = -field / 2 + k * field
            if x_lo < xb < x_hi:
                ax.axvline(xb, color=th.s(slot), linewidth=S.HAIR_PT * 1.3, linestyle=(0, (3, 2)))
        ax.set_xlim(x_lo, x_hi)
        ax.set_ylim(0, 1)
        ax.set_yticks([])
        for side in ("left", "top", "right"):
            ax.spines[side].set_visible(False)
        ax.spines["bottom"].set_color(th.axis)
        ax.set_title(title, fontsize=S.SMALL_PT + 0.5, loc="left", color=th.ink)
        ax.set_xlabel("x (nm); filled = absorber, dashed = field edge", fontsize=S.SMALL_PT)
        # row 2: the spectrum
        ax2 = axes[1, j]
        S.style_axes(ax2, th, grid="y")
        ax2.set_yscale("log")
        floor = 1e-5
        sel = r["s"] > floor
        order = np.argsort(r["f"])
        f, s = r["f"][order], r["s"][order]
        sel = sel[order]
        ax2.vlines(f[sel], floor, s[sel], color=th.s(slot), linewidth=S.LINE_PT * 0.8, zorder=3)
        ax2.plot(f[sel], s[sel], linestyle="none", marker="o", markersize=S.MARKER_PT * 0.55, color=th.s(slot),
                 zorder=4)
        for f_, lab, side in ((d["f_na"], "NA/λ", "left"), (d["f_support"], "(1+σ)NA/λ", "right")):
            S.limit_line(ax2, f_, th, lab, where=0.95, side=side)
            S.limit_line(ax2, -f_, th, None)
        for q in range(-2, 3):  # where the grating's orders belong: multiples of 1/180 nm
            if q:
                ax2.plot([q * 1000 / 180.0], [floor * 1.6], marker="^", markersize=S.MARKER_PT * 0.7,
                         color=th.ink2, linestyle="none", clip_on=False, zorder=5)
        ax2.set_xlim(-14, 14)
        ax2.set_ylim(floor, 1.5)
        ax2.set_xlabel("spatial frequency (cycles/µm); ▲ = grating orders q/180 nm" if j == 0 else "spatial frequency (cycles/µm)")
        if j == 0:
            ax2.set_ylabel("|S| / N²")
        n_bins = int((r["s"] > 1e-6).sum())
        ax2.set_title((f"spectrum: all {r['s'].size} bins non-zero,\nnone of them on a grating order" if key == "inc"
                       else f"spectrum: every bin is a grating order\n({n_bins} of {r['s'].size} non-zero)"),
                      fontsize=S.SMALL_PT + 0.5, loc="left")
        # row 3: the aerial image
        ax3 = axes[2, j]
        S.style_axes(ax3, th, grid="both")
        xs3, ys3 = _tile_periodic(r["x"], r["i"], field, reps=(-2, -1, 0, 1, 2))
        o = np.argsort(xs3)
        ax3.plot(xs3[o], ys3[o], color=th.s(slot), zorder=3)
        ax3.axhline(0.3, color=th.ink2, linewidth=S.HAIR_PT * 1.3, linestyle=(0, (4, 3)), zorder=2)
        for k in range(-3, 4):
            xb = -field / 2 + k * field
            if x_lo < xb < x_hi:
                ax3.axvline(xb, color=th.s(slot), linewidth=S.HAIR_PT * 1.3, linestyle=(0, (3, 2)), zorder=1)
        ax3.set_xlim(x_lo, x_hi)
        ax3.set_ylim(0, 1.0)
        ax3.set_xlabel("x (nm)")
        if j == 0:
            ax3.set_ylabel("Relative intensity")
        note = "wrong period and contrast" if key == "inc" else "the canonical run"
        ax3.set_title(f"image: contrast {r['contrast']:.3f}, NILS {r['nils']:.2f}, CD {r['cd']:.1f} nm\n{note}",
                      fontsize=S.SMALL_PT + 0.5, loc="left")
    fig.suptitle("65 nm lines on a 180 nm pitch — F$_2$ 157.63 nm, NA 0.75, σ 0.7: one grating, two fields",
                 fontsize=S.TITLE_PT, y=0.99)
    return fig


register(FigureSpec(
    name="masks-commensurate",
    group="masks",
    compute=commensurate_compute,
    draw=commensurate_draw,
    alt="Two columns for 65 nm lines on a 180 nm pitch: on a 256 nm field at 1 nm the FFT cuts the grating at "
        "1.42 periods, the spectrum leaks into every bin and the image has the wrong period and contrast; on "
        "the commensurate 0.703 nm pixel only the harmonics remain and the contrast is 0.454."
    ,
    caption="<strong>Commensurate versus incommensurate.</strong> Left, the canonical grating on the old 256 nm "
            "default field (1.42 periods): the FFT's periodic copy inserts a defect at the field edge, the spectrum "
            "leaks into every bin and the aerial image has the wrong period and contrast; right, the field "
            "<code>commensurate_grid(256, 1.0)</code> picks (0.703 nm pixel, one period) gives the harmonics only "
            "and the canonical run's contrast. The engine warns on the left-hand case and the CLI auto-adjusts, so it "
            "occurs only when a user opts out. <em>Model: commensurate grids ✅ · exact thin-mask spectra ✅ · "
            "Hopkins imaging ✅.</em>",
    pages=("masks.md",),
    extra=_EXTRA,
))
