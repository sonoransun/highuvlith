"""Aerial image visualization (Hopkins/SOCS images from ``SimulationEngine``)."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from highuvlith.viz.style import (
    axes_or_new,
    categorical,
    ink,
    resolve_theme,
    sequential_cmap,
    style_axes,
    style_colorbar,
    use_theme,
)

if TYPE_CHECKING:
    from highuvlith import AerialImageResult


def plot_aerial(
    result: AerialImageResult,
    *,
    title: str | None = None,
    cmap: Any = None,
    show_colorbar: bool = True,
    ax: Any = None,
    theme: str | None = None,
) -> Any:
    """Plot a 2D aerial image.

    Intensity is relative to the clear field (the engine default; absolute
    when the engine used ``normalization="absolute"``); axes in nm, y up.
    ``cmap`` defaults to the theme's sequential blue ramp. Returns the Axes.
    """
    th = resolve_theme(theme)
    with use_theme(th):
        ax = axes_or_new(ax, (6.4, 5.2))
        x_min, x_max, y_min, y_max = result.extent_nm
        im = ax.imshow(
            result.intensity,
            extent=(x_min, x_max, y_min, y_max),
            origin="lower",
            cmap=sequential_cmap(th) if cmap is None else cmap,
            aspect="equal",
            interpolation="nearest",
        )
        ax.set_xlabel("x (nm)")
        ax.set_ylabel("y (nm)")
        ax.set_title(title or "Aerial image")
        ax.grid(False)
        if show_colorbar:
            cbar = ax.figure.colorbar(im, ax=ax, label="Relative intensity")
            style_colorbar(cbar, th)
        style_axes(ax, th)
        ax.grid(False)
    return ax


def plot_cross_section(
    result: AerialImageResult,
    *,
    y_nm: float = 0.0,
    threshold: float | None = None,
    title: str | None = None,
    ax: Any = None,
    theme: str | None = None,
) -> Any:
    """Plot the intensity along x at the row nearest ``y_nm`` (nm).

    ``threshold`` (relative intensity) draws the print threshold with a
    direct label. Returns the Axes.
    """
    th = resolve_theme(theme)
    with use_theme(th):
        ax = axes_or_new(ax, (8.0, 3.4))
        x, intensity = result.cross_section(y_nm=y_nm)
        ax.plot(x, intensity, color=categorical(0, th), linewidth=1.5)
        ax.set_xlabel("x (nm)")
        ax.set_ylabel("Relative intensity")
        ax.set_title(title or f"Aerial image cross-section (y = {y_nm:.1f} nm)")
        if threshold is not None:
            ax.axhline(threshold, color=ink("muted", th), linestyle="--", linewidth=1.0)
            ax.annotate(
                f"threshold {threshold:.2f}",
                xy=(1.0, threshold),
                xycoords=("axes fraction", "data"),
                xytext=(-4, 3),
                textcoords="offset points",
                ha="right",
                va="bottom",
                fontsize=8,
                color=ink("secondary", th),
            )
        style_axes(ax, th)
    return ax
