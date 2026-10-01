"""Optimization figures: ILT mask evolution, SRAF depth-of-focus gain, fragment-OPC convergence."""

from __future__ import annotations

import numpy as np
from matplotlib import patheffects
from matplotlib import pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.patches import Polygon, Rectangle

import style as S
from common import FigureSpec, native, register

# ---------------------------------------------------------------------------
# 1. ILT for a 2 x 2 contact array
# ---------------------------------------------------------------------------

ILT_GRID = (64, 8.0)


def ilt_compute(fast: bool):
    huv = native()
    src = huv.SourceConfig.f2_laser(0.6)
    optics = huv.OpticsConfig(numerical_aperture=0.75)
    grid = huv.GridConfig(*ILT_GRID)
    target = np.asarray(huv.ilt_contact_target(100.0, 200.0, 200.0, 2, 2, grid))
    r = huv.optimize_ilt(src, optics, grid, target, cost="resist", threshold=0.25, steepness=50.0, max_iterations=40,
                         binarization_weight=0.1, binarization_after=20, snapshot_every=10, max_kernels=8)
    tgt90 = np.asarray(huv.ilt_contact_target(90.0, 180.0, 180.0, 2, 2, grid))
    hist = {}
    for grad in ("adjoint", "proxy"):
        rr = huv.optimize_ilt(src, optics, grid, tgt90, cost="resist", threshold=0.25, steepness=50.0,
                              max_iterations=25, gradient=grad, max_kernels=8)
        hist[grad] = {"cost": np.asarray(rr.cost_history), "termination": rr.termination,
                      "misprinted": rr.pattern_error}
    return {
        "target": target, "snapshots": [(it, np.asarray(m)) for it, m in r.snapshots],
        "binary": np.asarray(r.binary_mask), "binary_image": np.asarray(r.binary_aerial_image),
        "cost": np.asarray(r.cost_history), "stage_starts": list(r.stage_starts), "termination": r.termination,
        "binary_misprinted": r.binary_pattern_error, "hist90": hist,
    }


def ilt_draw(d, th: S.Theme):
    n, px = ILT_GRID
    half = n * px / 2
    ext = [-half, half, -half, half]
    snaps = d["snapshots"]
    picks = [snaps[0], snaps[min(1, len(snaps) - 1)], snaps[min(2, len(snaps) - 1)], snaps[-1]]
    fig = plt.figure(figsize=(S.WIDE_WIDTH + 0.8, 5.6))
    gs = fig.add_gridspec(2, 5, height_ratios=[1, 1.15], hspace=0.45, wspace=0.12)
    cmap = S.field_cmap()
    tiles = [("target", d["target"])] + [(f"mask, iteration {it}", m) for it, m in picks]
    for k, (title, img) in enumerate(tiles):
        ax = fig.add_subplot(gs[0, k])
        ax.imshow(img, origin="lower", extent=ext, cmap=cmap, vmin=0, vmax=1, interpolation="nearest")
        S.image_axes(ax, th)
        ax.set_title(title, fontsize=S.SMALL_PT + 0.5)
        ax.set_xticks([])
        ax.set_yticks([])
    # printed image of the final binary mask with the 0.25 contour and the target outline
    bottom = gs[1, :].subgridspec(1, 4, width_ratios=[1.0, 0.045, 0.42, 2.1], wspace=0.05)
    ax = fig.add_subplot(bottom[0, 0])
    img = d["binary_image"]
    im = ax.imshow(img, origin="lower", extent=ext, cmap=cmap, vmin=0, vmax=float(img.max()), interpolation="bilinear")
    xs = np.linspace(-half + px / 2, half - px / 2, n)
    ax.contour(xs, xs, img, levels=[0.25], colors=[th.s(2)], linewidths=S.LINE_PT)
    ax.contour(xs, xs, d["target"], levels=[0.5], colors=["#ffffff"], linewidths=S.HAIR_PT * 1.2,
               linestyles=[(0, (3, 2))])
    S.image_axes(ax, th)
    ax.set_title("final binary mask, printed:\n0.25 contour vs target (dashed)", fontsize=S.SMALL_PT + 0.5)
    ax.set_xlabel("x (nm)")
    ax.set_ylabel("y (nm)")
    cax = fig.add_subplot(bottom[0, 1])
    cb = fig.colorbar(im, cax=cax)
    cb.outline.set_edgecolor(th.axis)
    cb.ax.tick_params(colors=th.axis, labelcolor=th.ink2, labelsize=S.SMALL_PT)
    cb.set_label("Relative intensity", color=th.ink2, fontsize=S.SMALL_PT)
    # cost histories
    ax2 = fig.add_subplot(bottom[0, 3])
    S.style_axes(ax2, th, grid="both")
    ax2.set_yscale("log")
    h = d["hist90"]
    ax2.plot(np.arange(len(h["adjoint"]["cost"])), h["adjoint"]["cost"], color=th.s(1), zorder=3)
    ax2.plot(np.arange(len(h["proxy"]["cost"])), h["proxy"]["cost"], color=th.s(2), zorder=3)
    S.dot(ax2, [len(h["proxy"]["cost"]) - 1], [h["proxy"]["cost"][-1]], th.s(2), th)
    ax2.annotate(f"proxy stops: line search failed\n({h['proxy']['misprinted']} pixels misprinted)",
                 (len(h["proxy"]["cost"]) - 1, h["proxy"]["cost"][-1]), xytext=(8, 10), textcoords="offset points",
                 color=th.ink2, fontsize=S.SMALL_PT)
    ax2.annotate(f"exact adjoint ({h['adjoint']['misprinted']} pixels misprinted)",
                 (len(h["adjoint"]["cost"]) - 1, h["adjoint"]["cost"][-1]), xytext=(-6, 10),
                 textcoords="offset points", ha="right", color=th.ink2, fontsize=S.SMALL_PT)
    ax2.set_xlabel("Iteration")
    ax2.set_ylabel("Cost (sigmoid-resist mismatch)")
    ax2.set_title("90 nm contacts on 180 nm: exact adjoint vs legacy proxy gradient", fontsize=S.SMALL_PT + 0.5)
    ax2.legend([Line2D([], [], color=th.s(1)), Line2D([], [], color=th.s(2))],
               ["adjoint (default)", "local proxy (legacy)"], loc="upper right")
    return fig


register(FigureSpec(
    name="optim-ilt-contacts",
    group="optim",
    compute=ilt_compute,
    draw=ilt_draw,
    alt="Inverse lithography for a two-by-two contact array: the target, the continuous mask after 0, 10, 20 and "
        "35 iterations growing serifs and assist-like rings, the printed image of the final binary mask with its "
        "contour on the target, and cost histories where the exact adjoint keeps descending while the legacy "
        "proxy stalls.",
    caption="Inverse lithography (ILT) of 100 nm contacts on a 200 nm pitch (F<sub>2</sub> 157.63 nm, NA 0.75, σ "
            "0.6, 8 kernels, 64 × 8 nm grid): sigmoid-resist cost with the exact adjoint gradient through the "
            "SOCS kernels, conjugate gradients, binarization penalty switched on after iteration 20 (converged at "
            "35). Bottom right: "
            "on 90 nm contacts the legacy local proxy stops after a few iterations while the adjoint drives the "
            "cost down ~18×. Model: ILT ✅ (constant-threshold sigmoid resist; pixel mask, no MRC or polygon "
            "extraction).",
    pages=("research-modules.md", "README.md"),
))


# ---------------------------------------------------------------------------
# 2. SRAF: isolated-line depth of focus with and without assists
# ---------------------------------------------------------------------------

SRAF_CD, SRAF_ILLUM = 80.0, ("annular", 0.5, 0.8)


def sraf_compute(fast: bool):
    huv = native()
    src = huv.SourceConfig.f2_laser(0.5)
    optics = huv.OpticsConfig(numerical_aperture=0.75)
    grid = huv.GridConfig(64, 12.0)
    field = 64 * 12.0
    bare = huv.mask_from_features(rects=[(0.0, 0.0, SRAF_CD, field)])
    thr = huv.dose_to_size_threshold(src, optics, bare, grid, SRAF_CD, max_kernels=16, illumination=SRAF_ILLUM)
    sraf = huv.insert_srafs(src, optics, bare, grid, SRAF_CD, 0.65, thr, defocus_nm=150.0, dose_excursion=0.08,
                            max_kernels=16, illumination=SRAF_ILLUM)
    dof = huv.compare_sraf_dof(src, optics, bare, sraf.mask, grid, SRAF_CD, focus_range_nm=300.0,
                               focus_steps=13 if fast else 25, max_kernels=16, illumination=SRAF_ILLUM)
    return {"field": field, "assists": list(sraf.assists), "margin": sraf.worst_margin, "focus": np.asarray(dof.focus_nm),
            "cd0": np.asarray(dof.cd_without_nm), "cd1": np.asarray(dof.cd_with_nm), "dof0": dof.dof_without_nm,
            "dof1": dof.dof_with_nm, "gain": dof.gain, "windows": dof.windows_nm}


def sraf_draw(d, th: S.Theme):
    fig, (ax0, ax1) = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 3.9), gridspec_kw={"width_ratios": [1, 1.3],
                                                                                  "wspace": 0.25})
    half = d["field"] / 2
    ax0.add_patch(Rectangle((-SRAF_CD / 2, -1), SRAF_CD, 2, facecolor=th.ink2, edgecolor="none"))
    for (x, _y, w, _h) in d["assists"]:
        for xx in {x, x - d["field"], x + d["field"]}:
            if -half - w <= xx <= half + w:
                ax0.add_patch(Rectangle((xx - w / 2, -1), w, 2, facecolor=th.s(2), edgecolor="none"))
    ax0.set_xlim(-half, half)
    ax0.set_ylim(-1.2, 1.2)
    ax0.set_yticks([])
    for side in ("left", "right", "top"):
        ax0.spines[side].set_visible(False)
    ax0.spines["bottom"].set_color(th.axis)
    ax0.set_xlabel("x (nm), one 768 nm periodic field")
    ax0.set_title("(a) mask: 80 nm line (grey) + assists (orange)")
    w = d["assists"][0][2] if d["assists"] else 0.0
    ax0.text(0, 1.12, f"assist width {w:.0f} nm, worst print margin {d['margin']:.2f}", ha="center",
             va="bottom", color=th.ink2, fontsize=S.SMALL_PT)
    S.style_axes(ax1, th, grid="both")
    ax1.axhspan(SRAF_CD * 0.9, SRAF_CD * 1.1, color=th.band, lw=0, zorder=0)
    ax1.plot(d["focus"], d["cd0"], color=th.s(1), zorder=3, marker="o", markersize=S.MARKER_PT * 0.6,
             markeredgecolor=th.surface, markeredgewidth=S.RING_PT * 0.6)
    ax1.plot(d["focus"], d["cd1"], color=th.s(2), zorder=3, marker="o", markersize=S.MARKER_PT * 0.6,
             markeredgecolor=th.surface, markeredgewidth=S.RING_PT * 0.6)
    for (lo, hi), slot, y in zip(d["windows"], (1, 2), (61.0, 57.0)):
        ax1.annotate("", xy=(lo, y), xytext=(hi, y),
                     arrowprops=dict(arrowstyle="<->", color=th.s(slot), lw=S.LINE_PT * 0.8, shrinkA=0, shrinkB=0))
    ax1.text(0, 61.8, f"DOF {d['dof0']:.0f} nm", ha="center", va="bottom", color=th.ink2, fontsize=S.SMALL_PT)
    ax1.text(0, 54.0, f"DOF {d['dof1']:.0f} nm (×{d['gain']:.2f})", ha="center", va="top", color=th.ink2,
             fontsize=S.SMALL_PT)
    ax1.set_xlim(d["focus"][0], d["focus"][-1])
    ax1.set_ylim(40, 90)
    ax1.set_xlabel("Focus (nm)")
    ax1.set_ylabel("Printed line CD (nm)")
    ax1.set_title("(b) CD through focus, each at its dose-to-size")
    ax1.legend([Line2D([], [], color=th.s(1)), Line2D([], [], color=th.s(2))], ["without assists", "with assists"],
               loc="lower center", bbox_to_anchor=(0.5, 0.0))
    return fig


register(FigureSpec(
    name="optim-sraf-dof",
    group="optim",
    compute=sraf_compute,
    draw=sraf_draw,
    alt="Left: an 80 nm isolated line with sub-resolution assist bars on each side. Right: printed line width "
        "versus focus without and with the assists; with assists the width stays within ten percent over a wider "
        "focus range.",
    caption="Sub-resolution assist features for an isolated 80 nm line (F<sub>2</sub> 157.63 nm, NA 0.75, annular "
            "σ 0.5–0.8): rule-placed scattering bars are shrunk or removed until none prints at ±150 nm focus and "
            "±8 % dose, then the depth of focus (CD within ±10 %, each mask at its own dose-to-size) is compared. "
            "Model: SRAF insertion 🔶 (heuristic rule deck + model print check; constant-threshold resist).",
    pages=("research-modules.md",),
))


# ---------------------------------------------------------------------------
# 3. Fragment-based model OPC: a line end pulled back and corrected
# ---------------------------------------------------------------------------


def opc_compute(fast: bool):
    huv = native()
    src = huv.SourceConfig.f2_laser(0.6)
    optics = huv.OpticsConfig(numerical_aperture=0.75)
    grid = huv.GridConfig(64, 12.0)
    line = huv.mask_from_features(rects=[(0.0, 0.0, 90.0, 500.0)])
    thr = huv.dose_to_size_threshold(src, optics, line, grid, 90.0, max_kernels=12)
    o = huv.fragment_opc(src, optics, line, grid, thr, tolerance_nm=0.5, max_epe_tolerance_nm=1.0, max_kernels=12)
    o0 = huv.fragment_opc(src, optics, line, grid, thr, max_iterations=0, max_kernels=12)
    return {"thr": thr, "rms": np.asarray(o.epe_rms_history), "max": np.asarray(o.epe_max_history),
            "polygons": [list(p) for p in o.polygons], "image": np.asarray(o.final_image),
            "image0": np.asarray(o0.final_image), "converged": o.converged, "n": 64, "px": 12.0}


def opc_draw(d, th: S.Theme):
    fig, (ax0, ax1) = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 4.3), gridspec_kw={"width_ratios": [1, 1.15],
                                                                                  "wspace": 0.32})
    n, px = d["n"], d["px"]
    half = n * px / 2
    xs = np.linspace(-half + px / 2, half - px / 2, n)
    cmap = S.field_cmap()
    im = ax0.imshow(d["image"], origin="lower", extent=[-half, half, -half, half], cmap=cmap, vmin=0,
                    vmax=float(d["image"].max()), interpolation="bilinear")
    halo = [patheffects.withStroke(linewidth=S.HAIR_PT * 3.2, foreground="#0b0b0b")]
    ax0.add_patch(Rectangle((-45, -250), 90, 500, fill=False, edgecolor="#ffffff", linewidth=S.HAIR_PT * 1.3,
                            linestyle=(0, (3, 2)), path_effects=halo))
    for poly in d["polygons"]:
        ax0.add_patch(Polygon(poly, closed=True, fill=False, edgecolor=th.s(4), linewidth=S.LINE_PT))
    ax0.contour(xs, xs, d["image0"], levels=[d["thr"]], colors=[th.s(2)], linewidths=S.LINE_PT * 0.9)
    ax0.contour(xs, xs, d["image"], levels=[d["thr"]], colors=[th.s(1)], linewidths=S.LINE_PT * 0.9)
    S.image_axes(ax0, th)
    ax0.set_xlim(-160, 160)
    ax0.set_ylim(-330, 330)
    ax0.set_xlabel("x (nm)")
    ax0.set_ylabel("y (nm)")
    ax0.set_title("(a) 90 × 500 nm line, image after OPC")
    S.colorbar(fig, im, ax0, th, "Relative intensity")
    ax0.legend([Line2D([], [], color="#ffffff", linestyle=(0, (3, 2)), linewidth=S.HAIR_PT * 1.3, path_effects=halo),
                Line2D([], [], color=th.s(4)), Line2D([], [], color=th.s(2)), Line2D([], [], color=th.s(1))],
               ["drawn line (target)", "corrected mask polygon", "printed, uncorrected", "printed, after OPC"],
               loc="upper center", bbox_to_anchor=(0.5, -0.14), ncol=2, fontsize=S.SMALL_PT - 0.5)
    S.style_axes(ax1, th, grid="both")
    ax1.set_yscale("log")
    it = np.arange(len(d["rms"]))
    ax1.plot(it, d["max"], color=th.s(2), zorder=3, marker="o", markersize=S.MARKER_PT * 0.7,
             markeredgecolor=th.surface, markeredgewidth=S.RING_PT * 0.6)
    ax1.plot(it, d["rms"], color=th.s(1), zorder=3, marker="o", markersize=S.MARKER_PT * 0.7,
             markeredgecolor=th.surface, markeredgewidth=S.RING_PT * 0.6)
    S.end_labels(ax1, [(it[-1], d["max"][-1], f"max |EPE| {d['max'][-1]:.2f} nm"),
                       (it[-1], d["rms"][-1], f"rms EPE {d['rms'][-1]:.2f} nm")], th)
    ax1.annotate(f"uncorrected: max {d['max'][0]:.1f} nm,\nrms {d['rms'][0]:.1f} nm (line-end pull-back)",
                 (0, d["max"][0]), xytext=(1.6, d["max"][0] * 1.05), textcoords="data", color=th.ink2,
                 fontsize=S.SMALL_PT, va="top",
                 arrowprops=dict(arrowstyle="-", color=th.axis, lw=S.HAIR_PT, shrinkA=2, shrinkB=4))
    ax1.set_xlim(-0.3, len(it) - 0.4)
    ax1.set_xlabel("OPC iteration")
    ax1.set_ylabel("Edge placement error (nm)")
    ax1.set_title("(b) Convergence of the fragment feedback")
    fig.subplots_adjust(bottom=0.22, right=0.86)
    return fig


register(FigureSpec(
    name="optim-opc-epe",
    group="optim",
    compute=opc_compute,
    draw=opc_draw,
    alt="Left: aerial image of a short line after optical proximity correction, with the drawn line, the "
        "corrected mask polygon extended at the line ends, and the printed contours before and after correction. "
        "Right: maximum and rms edge-placement error falling from about 27 and 9 nm to below 1 nm in six "
        "iterations.",
    caption="Fragment-based model OPC of a 90 × 500 nm line (F<sub>2</sub> 157.63 nm, NA 0.75, σ 0.6, threshold at "
            "the dose-to-size of the line centre): edges are cut into fragments, each moved by damped feedback on "
            "its edge-placement error measured on the aerial image. The uncorrected line ends pull back by tens "
            "of nanometres; the corrected polygon extends them and the residual falls below 1 nm. Model: OPC "
            "(fragment model-based) ✅ (aerial-image threshold; corners slaved, no MRC beyond bias clamp, jog "
            "cleanup and grid snap).",
    pages=("research-modules.md",),
))
