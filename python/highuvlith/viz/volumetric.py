"""Volumetric (z-resolved) and deep-layer visualization.

Matplotlib plots for x–z cross-sections, depth dose, height maps and
development fronts, plus an interactive Plotly isosurface. Plotting libraries
are imported lazily inside each function so importing highuvlith never
requires matplotlib or plotly.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

import numpy as np

from highuvlith.viz.style import (
    axes_or_new,
    categorical,
    resolve_theme,
    sequential_cmap,
    style_axes,
    style_colorbar,
    use_theme,
)

if TYPE_CHECKING:
    from highuvlith import HeightMapResult, VolumetricResult


def _edges(centres: np.ndarray, fallback: float = 1.0) -> tuple[float, float]:
    """(first edge, last edge) of uniformly spaced cell centres."""
    c = np.asarray(centres, dtype=float)
    step = float(c[1] - c[0]) if c.size > 1 else fallback
    return float(c[0]) - step / 2.0, float(c[-1]) + step / 2.0


def plot_xz_slice(
    volume: VolumetricResult,
    *,
    y_index: int | None = None,
    title: str | None = None,
    cmap: Any = None,
    show_colorbar: bool = True,
    colorbar_label: str = "value",
    ax: Any = None,
    theme: str | None = None,
) -> Any:
    """Plot an x–z cross-section (depth increasing downward) of a volume.

    Args:
        volume: A VolumetricResult (PAC, dose, or arrival times).
        y_index: Row index of the slice. Defaults to the middle row.
        title: Optional plot title.
        cmap: Colormap; defaults to the theme's sequential ramp.
        show_colorbar: Whether to draw a colorbar.
        colorbar_label: Colorbar label (state the quantity and unit).
        ax: Optional matplotlib Axes to draw on.
        theme: "light" or "dark" (default: current theme).
    """
    th = resolve_theme(theme)
    with use_theme(th):
        ax = axes_or_new(ax, (7.0, 4.4))
        values = np.asarray(volume.values)  # (nz, ny, nx)
        row = values.shape[1] // 2 if y_index is None else int(y_index)
        x0, x1 = _edges(volume.x_nm)
        z0, z1 = _edges(volume.z_nm)
        im = ax.imshow(
            values[:, row, :],
            extent=(x0, x1, z1, z0),
            cmap=sequential_cmap(th) if cmap is None else cmap,
            aspect="auto",
            interpolation="nearest",
        )
        ax.set_xlabel("x (nm)")
        ax.set_ylabel("Depth z (nm)")
        ax.set_title(title or f"x–z slice (y index {row})")
        if show_colorbar:
            style_colorbar(ax.figure.colorbar(im, ax=ax, label=colorbar_label), th)
        style_axes(ax, th)
        ax.grid(False)
    return ax


def plot_depth_dose(
    z: Any,
    dose: Any = None,
    *,
    z_label: str = "Depth z (µm)",
    dose_label: str = "Absorbed dose (kJ/cm³)",
    title: str | None = None,
    log: bool = False,
    ax: Any = None,
    theme: str | None = None,
) -> Any:
    """Plot a 1D depth-dose profile.

    Pass either ``(z_um, dose_kj_cm3)`` arrays or a ``LigaResult`` (its
    ``depth_dose`` is used). ``log=True`` uses a logarithmic dose axis.
    Returns the Axes.
    """
    if dose is None:
        if not hasattr(z, "depth_dose"):
            raise TypeError("give (z, dose) arrays or a LigaResult")
        z, dose = z.depth_dose
    th = resolve_theme(theme)
    with use_theme(th):
        ax = axes_or_new(ax, (6.0, 4.0))
        ax.plot(np.asarray(z), np.asarray(dose), color=categorical(0, th), linewidth=1.5)
        if log:
            ax.set_yscale("log")
        ax.set_xlabel(z_label)
        ax.set_ylabel(dose_label)
        ax.set_title(title or "Depth dose")
        style_axes(ax, th)
    return ax


def plot_height_map(
    height_map: HeightMapResult,
    *,
    title: str | None = None,
    cmap: Any = None,
    show_colorbar: bool = True,
    ax: Any = None,
    theme: str | None = None,
) -> Any:
    """Plot a 2D remaining-height / development-depth map (nm), y up."""
    th = resolve_theme(theme)
    with use_theme(th):
        ax = axes_or_new(ax, (6.4, 5.4))
        x0, x1 = _edges(height_map.x_nm)
        y0, y1 = _edges(height_map.y_nm)
        im = ax.imshow(
            np.asarray(height_map.values),
            extent=(x0, x1, y0, y1),
            origin="lower",
            cmap=sequential_cmap(th) if cmap is None else cmap,
            aspect="equal",
            interpolation="nearest",
        )
        ax.set_xlabel("x (nm)")
        ax.set_ylabel("y (nm)")
        ax.set_title(title or "Height map")
        if show_colorbar:
            style_colorbar(ax.figure.colorbar(im, ax=ax, label="Height (nm)"), th)
        style_axes(ax, th)
        ax.grid(False)
    return ax


def plot_development_front(
    fronts: dict[str, VolumetricResult],
    dev_time_s: float,
    *,
    y_index: int | None = None,
    title: str | None = None,
    ax: Any = None,
    theme: str | None = None,
) -> Any:
    """Developed-resist boundary at ``dev_time_s`` (s) on the x–z plane.

    ``fronts`` maps a label to a volume of development-front arrival times
    (s) — e.g. ``{"level set": ls.arrival_times, "fast marching": fmm}`` —
    and each is drawn as the contour ``t = dev_time_s`` (depth downward).
    This is the level-set vs fast-marching comparison; both solvers are
    first order (≤ one cell of front position). Returns the Axes.
    """
    from matplotlib.lines import Line2D

    th = resolve_theme(theme)
    with use_theme(th):
        ax = axes_or_new(ax, (7.0, 4.4))
        handles = []
        z_range: tuple[float, float] | None = None
        for i, (label, times) in enumerate(fronts.items()):
            values = np.asarray(times.values, dtype=float)
            row = values.shape[1] // 2 if y_index is None else int(y_index)
            plane = values[:, row, :]
            finite = plane[np.isfinite(plane)]
            ceiling = 10.0 * max(float(finite.max()) if finite.size else 0.0, dev_time_s, 1.0)
            plane = np.where(np.isfinite(plane), plane, ceiling)
            x = np.asarray(times.x_nm, dtype=float)
            z = np.asarray(times.z_nm, dtype=float)
            color = categorical(i, th)
            if plane.min() < dev_time_s < plane.max() and plane.shape[0] > 1 and plane.shape[1] > 1:
                ax.contour(x, z, plane, levels=[dev_time_s], colors=[color], linewidths=1.5)
            handles.append(Line2D([], [], color=color, linewidth=1.5, label=label))
            edges = _edges(z)
            z_range = edges if z_range is None else (min(z_range[0], edges[0]), max(z_range[1], edges[1]))
        if z_range is not None:
            ax.set_ylim(z_range[1], z_range[0])
        ax.set_xlabel("x (nm)")
        ax.set_ylabel("Depth z (nm)")
        ax.set_title(title or f"Development front at t = {dev_time_s:g} s")
        if handles:
            ax.legend(handles=handles, loc="best")
        style_axes(ax, th)
    return ax


def plot_isosurface(
    volume: VolumetricResult,
    level: float,
    *,
    title: str | None = None,
    colorscale: str = "Blues",
    opacity: float = 0.6,
) -> Any:
    """Interactive 3D isosurface of a volumetric field at ``level`` (plotly).

    Requires the optional ``plotly`` package. Returns the plotly Figure.
    """
    import plotly.graph_objects as go

    values = np.asarray(volume.values)  # (nz, ny, nx)
    x = np.asarray(volume.x_nm)
    y = np.asarray(volume.y_nm)
    z = np.asarray(volume.z_nm)

    # Coordinate grids matching the (nz, ny, nx) data layout.
    zz, yy, xx = np.meshgrid(z, y, x, indexing="ij")

    fig = go.Figure(
        data=go.Isosurface(
            x=xx.ravel(),
            y=yy.ravel(),
            z=zz.ravel(),
            value=values.ravel(),
            isomin=level,
            isomax=level,
            surface_count=1,
            colorscale=colorscale,
            opacity=opacity,
            caps=dict(x_show=False, y_show=False, z_show=False),
        )
    )
    fig.update_layout(
        title=title or f"Isosurface (level={level:g})",
        scene=dict(
            xaxis_title="x (nm)",
            yaxis_title="y (nm)",
            zaxis_title="depth z (nm)",
            zaxis=dict(autorange="reversed"),
        ),
        width=700,
        height=600,
    )
    return fig
