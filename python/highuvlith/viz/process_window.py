"""Process-window visualization: Bossung curves, ED window, EL vs DOF.

The data come from :class:`highuvlith.ProcessWindowResult` — a 🔶 simplified
analysis (constant-threshold resist, CD on the y = 0 cut, rectangular
windows) — or from a tabulated focus–exposure matrix.
"""

from __future__ import annotations

import math
from typing import TYPE_CHECKING, Any

import numpy as np

from highuvlith.viz.style import (
    axes_or_new,
    categorical,
    ink,
    ordinal_colors,
    resolve_theme,
    sequential_cmap,
    style_axes,
    style_colorbar,
    use_theme,
)

if TYPE_CHECKING:
    from highuvlith import ProcessWindowResult


def _spec_band(
    pw: Any, cd_target_nm: float | None, cd_tolerance_pct: float | None
) -> tuple[float, float, float, float]:
    """(target, tolerance %, cd_min, cd_max) from the arguments or the result."""
    target = float(pw.cd_target_nm if cd_target_nm is None else cd_target_nm)
    tol = float(pw.cd_tolerance_pct if cd_tolerance_pct is None else cd_tolerance_pct)
    return target, tol, target * (1.0 - tol / 100.0), target * (1.0 + tol / 100.0)


def plot_bossung(
    pw: ProcessWindowResult,
    *,
    title: str | None = None,
    ax: Any = None,
    theme: str | None = None,
) -> Any:
    """Bossung curves: printed CD (nm) vs focus (nm), one curve per dose.

    Doses are ordered, so they take a one-hue ordinal ramp (lighter = lower
    dose in the light theme). The CD spec band ``pw.spec_limits_nm()`` is
    shaded; CDs that do not print (NaN) leave gaps. Returns the Axes.
    """
    th = resolve_theme(theme)
    with use_theme(th):
        ax = axes_or_new(ax, (7.5, 4.6))
        doses = np.asarray(pw.doses)
        focuses = np.asarray(pw.focuses)
        cd_matrix = np.asarray(pw.cd_matrix)
        lo, hi = pw.spec_limits_nm()
        ax.axhspan(lo, hi, color=ink("muted", th), alpha=0.15, linewidth=0)
        ax.annotate(
            f"spec {pw.cd_target_nm:.4g} nm ± {pw.cd_tolerance_pct:.4g} %",
            xy=(0.0, hi),
            xycoords=("axes fraction", "data"),
            xytext=(4, 2),
            textcoords="offset points",
            fontsize=8,
            color=ink("secondary", th),
        )
        for color, dose, cds in zip(ordinal_colors(len(doses), th), doses, cd_matrix):
            ax.plot(focuses, cds, "-o", color=color, markersize=3.5, label=f"{dose:.4g}")
        ax.set_xlabel("Focus (nm)")
        ax.set_ylabel("CD (nm)")
        ax.set_title(title or "Bossung curves")
        if len(doses) >= 2:
            ax.legend(
                title="Dose (mJ/cm²)",
                ncol=2 if len(doses) > 6 else 1,
                loc="best",
            )
        style_axes(ax, th)
    return ax


def _rectangle(ax: Any, rect: dict[str, float], color: str, linestyle: str, label: str) -> None:
    from matplotlib.patches import Rectangle

    ax.add_patch(
        Rectangle(
            (rect["focus_min_nm"], rect["dose_min_mj_cm2"]),
            rect["focus_max_nm"] - rect["focus_min_nm"],
            rect["dose_max_mj_cm2"] - rect["dose_min_mj_cm2"],
            fill=False,
            edgecolor=color,
            linewidth=1.5,
            linestyle=linestyle,
            label=label,
        )
    )


def plot_ed_window(
    pw: ProcessWindowResult,
    *,
    cd_target_nm: float | None = None,
    cd_tolerance_pct: float | None = None,
    title: str | None = None,
    ax: Any = None,
    theme: str | None = None,
) -> Any:
    """Exposure–defocus window: in-spec dose range (mJ/cm²) vs focus (nm).

    The band is the per-focus in-spec dose interval ``pw.dose_limits``
    (constant-threshold resist, 🔶); the largest-area process rectangle
    (``max_area_rectangle``) and the largest-DOF rectangle at 5 % exposure
    latitude (``dof_at_el(5.0)``) are outlined. Results without dose limits
    (tabulated focus–exposure matrices) fall back to CD contours with the
    spec limits from ``cd_target_nm`` / ``cd_tolerance_pct`` (default: the
    result's). Returns the Axes.
    """
    th = resolve_theme(theme)
    target, tol, cd_min, cd_max = _spec_band(pw, cd_target_nm, cd_tolerance_pct)
    with use_theme(th):
        ax = axes_or_new(ax, (7.0, 4.8))
        focuses = np.asarray(pw.focuses, dtype=float)
        limits = list(pw.dose_limits)
        has_limits = any(lim is not None for lim in limits)
        if has_limits:
            lower = np.array([math.nan if lim is None else lim[0] for lim in limits])
            upper = np.array([math.nan if lim is None else lim[1] for lim in limits])
            color = categorical(0, th)
            ax.fill_between(
                focuses,
                lower,
                upper,
                where=np.isfinite(lower) & np.isfinite(upper),
                color=color,
                alpha=0.22,
                linewidth=0,
                label="in-spec dose range",
            )
            ax.plot(focuses, lower, color=color, linewidth=1.2)
            ax.plot(focuses, upper, color=color, linewidth=1.2)
            best = pw.max_area_rectangle()
            if best is not None:
                _rectangle(
                    ax,
                    best,
                    categorical(1, th),
                    "-",
                    f"max-area window: DOF {best['dof_nm']:.0f} nm, "
                    f"EL {best['exposure_latitude_pct']:.1f} %",
                )
            at5 = pw.dof_at_el(5.0)
            if at5 is not None:
                _rectangle(
                    ax,
                    at5,
                    categorical(2, th),
                    "--",
                    f"DOF at 5 % EL: {at5['dof_nm']:.0f} nm",
                )
            ax.legend(loc="best")
        else:
            doses = np.asarray(pw.doses, dtype=float)
            cd_matrix = np.asarray(pw.cd_matrix, dtype=float)
            grid_f, grid_d = np.meshgrid(focuses, doses)
            filled = ax.contourf(grid_f, grid_d, cd_matrix, levels=16, cmap=sequential_cmap(th))
            cbar = ax.figure.colorbar(filled, ax=ax, label="CD (nm)")
            style_colorbar(cbar, th)
            finite = cd_matrix[np.isfinite(cd_matrix)]
            levels = [
                lv for lv in (cd_min, cd_max) if finite.size and finite.min() < lv < finite.max()
            ]
            if levels:
                ax.contour(
                    grid_f,
                    grid_d,
                    cd_matrix,
                    levels=levels,
                    colors=[ink("primary", th)],
                    linewidths=1.5,
                )
        ax.set_xlabel("Focus (nm)")
        ax.set_ylabel("Dose (mJ/cm²)")
        ax.set_title(title or f"ED window (target {target:.4g} nm ± {tol:.4g} %)")
        style_axes(ax, th)
    return ax


def plot_el_vs_dof(
    pw: ProcessWindowResult,
    *,
    ax: Any = None,
    theme: str | None = None,
) -> Any:
    """Exposure latitude (%) vs depth of focus (nm) from ``pw.el_vs_dof()``,
    with the DOF at 5 % EL marked when it exists. Returns the Axes."""
    th = resolve_theme(theme)
    with use_theme(th):
        ax = axes_or_new(ax, (6.0, 4.0))
        dof, el = pw.el_vs_dof()
        ax.plot(dof, el, color=categorical(0, th), linewidth=1.5)
        at5 = pw.dof_at_el(5.0)
        if at5 is not None:
            ax.plot([at5["dof_nm"]], [5.0], "o", color=categorical(0, th), markersize=6)
            ax.annotate(
                f"DOF at 5 % EL = {at5['dof_nm']:.0f} nm",
                xy=(at5["dof_nm"], 5.0),
                xytext=(6, 6),
                textcoords="offset points",
                fontsize=8,
                color=ink("secondary", th),
            )
        ax.set_xlabel("Depth of focus (nm)")
        ax.set_ylabel("Exposure latitude (%)")
        ax.set_title("Exposure latitude vs depth of focus")
        ax.set_xlim(left=0.0)
        ax.set_ylim(bottom=0.0)
        style_axes(ax, th)
    return ax
