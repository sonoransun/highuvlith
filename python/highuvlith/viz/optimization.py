"""Optimization visualization: ILT mask evolution and SRAF depth of focus."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

import numpy as np

from highuvlith.viz.style import (
    axes_or_new,
    categorical,
    ink,
    resolve_theme,
    sequential_cmap,
    style_axes,
    use_theme,
)

if TYPE_CHECKING:
    from highuvlith._native import IltResult


def plot_ilt_evolution(
    result: IltResult,
    *,
    target: np.ndarray | None = None,
    threshold: float | None = None,
    max_snapshots: int = 6,
    fig: Any = None,
    theme: str | None = None,
) -> Any:
    """ILT mask evolution: optional target, mask snapshots, final image.

    Panels show the optional ``target`` (1 = printed), up to
    ``max_snapshots`` evenly spaced ``result.snapshots`` (continuous mask
    transmission 0–1, pixel units), and the aerial image of the binarized
    mask with the ``threshold`` contour (constant-threshold resist). The
    optimizer is the exact-adjoint ILT through the SOCS kernels. Returns the
    Figure.
    """
    import matplotlib.pyplot as plt

    th = resolve_theme(theme)
    snaps = list(result.snapshots)
    if len(snaps) > max_snapshots >= 2:
        picks = np.linspace(0, len(snaps) - 1, max_snapshots).round().astype(int)
        snaps = [snaps[i] for i in sorted(set(picks.tolist()))]
    panels: list[tuple[str, np.ndarray, bool]] = []
    if target is not None:
        panels.append(("target", np.asarray(target), False))
    panels.extend((f"iteration {it}", np.asarray(mask), False) for it, mask in snaps)
    panels.append(("binary-mask image", np.asarray(result.binary_aerial_image), True))
    with use_theme(th):
        if fig is None:
            fig = plt.figure(figsize=(2.3 * len(panels), 2.7))
        axes = np.atleast_1d(fig.subplots(1, len(panels)))
        for ax, (label, data, is_image) in zip(axes, panels):
            ax.imshow(
                data,
                origin="lower",
                cmap=sequential_cmap(th),
                vmin=None if is_image else 0.0,
                vmax=None if is_image else 1.0,
                interpolation="nearest",
            )
            if is_image and threshold is not None and data.min() < threshold < data.max():
                ax.contour(data, levels=[threshold], colors=[categorical(1, th)], linewidths=1.2)
            ax.set_title(label, fontsize=9)
            ax.set_xticks([])
            ax.set_yticks([])
            style_axes(ax, th)
            ax.grid(False)
        fig.suptitle(
            f"ILT: final cost {result.final_cost:.3g} after {result.iterations} iterations "
            f"({result.termination})",
            color=ink("primary", th),
            fontsize=10,
        )
    return fig


def plot_sraf_dof(
    study: Any,
    *,
    target_cd_nm: float | None = None,
    cd_tolerance_pct: float = 10.0,
    ax: Any = None,
    theme: str | None = None,
) -> Any:
    """Printed line CD (nm) vs focus (nm) with and without assist features.

    ``study`` is an ``api.SrafStudy`` (its ``dof`` comparison and ``cd_nm``
    target are used) or a bare ``DofComparison`` (then give
    ``target_cd_nm``). Each mask is exposed at its own dose-to-size threshold
    (constant-threshold resist); NaN CDs (line not printed) leave gaps. The
    ±``cd_tolerance_pct`` band around the target is shaded and the legend
    reports each depth of focus. Returns the Axes.
    """
    dof = getattr(study, "dof", study)
    target = target_cd_nm if target_cd_nm is not None else getattr(study, "cd_nm", None)
    if target is None:
        raise ValueError("give target_cd_nm (or an SrafStudy, which carries cd_nm)")
    th = resolve_theme(theme)
    with use_theme(th):
        ax = axes_or_new(ax, (6.8, 4.2))
        lo = target * (1.0 - cd_tolerance_pct / 100.0)
        hi = target * (1.0 + cd_tolerance_pct / 100.0)
        ax.axhspan(lo, hi, color=ink("muted", th), alpha=0.15, linewidth=0)
        ax.annotate(
            f"target {target:.4g} nm ± {cd_tolerance_pct:g} %",
            xy=(0.0, hi),
            xycoords=("axes fraction", "data"),
            xytext=(4, 2),
            textcoords="offset points",
            fontsize=8,
            color=ink("secondary", th),
        )
        focus = np.asarray(dof.focus_nm)
        ax.plot(
            focus,
            np.asarray(dof.cd_without_nm),
            "-o",
            color=categorical(0, th),
            markersize=4,
            label=f"without assists (DOF {dof.dof_without_nm:.0f} nm)",
        )
        ax.plot(
            focus,
            np.asarray(dof.cd_with_nm),
            "-s",
            color=categorical(1, th),
            markersize=4,
            label=f"with assists (DOF {dof.dof_with_nm:.0f} nm)",
        )
        ax.set_xlabel("Focus (nm)")
        ax.set_ylabel("Printed CD (nm)")
        ax.set_title(f"SRAF depth of focus (gain ×{dof.gain:.2f})")
        ax.legend(loc="best")
        style_axes(ax, th)
    return ax
