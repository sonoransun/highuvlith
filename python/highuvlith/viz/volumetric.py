"""Volumetric (z-resolved) and deep-layer visualization.

Matplotlib plots for cross-sections, depth dose, and height maps, plus an
interactive Plotly isosurface. Plotting libraries are imported lazily inside
each function so importing highuvlith never requires matplotlib or plotly.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import numpy as np

if TYPE_CHECKING:
    from highuvlith import HeightMapResult, VolumetricResult


def plot_xz_slice(
    volume: VolumetricResult,
    *,
    y_index: int | None = None,
    title: str | None = None,
    cmap: str = "viridis",
    show_colorbar: bool = True,
    ax=None,
):
    """Plot an x-z cross-section (depth downward) of a volumetric field.

    Args:
        volume: A VolumetricResult (PAC, dose, or arrival times).
        y_index: Row index of the slice. Defaults to the middle row.
        title: Optional plot title.
        cmap: Matplotlib colormap.
        show_colorbar: Whether to draw a colorbar.
        ax: Optional matplotlib axis to draw on.
    """
    import matplotlib.pyplot as plt

    if ax is None:
        _, ax = plt.subplots(1, 1, figsize=(8, 5))

    values = np.asarray(volume.values)  # (nz, ny, nx)
    ny = values.shape[1]
    row = ny // 2 if y_index is None else int(y_index)
    slice_xz = values[:, row, :]  # (nz, nx)

    x = np.asarray(volume.x_nm)
    z = np.asarray(volume.z_nm)

    im = ax.imshow(
        slice_xz,
        extent=[x[0], x[-1], z[-1], z[0]],
        cmap=cmap,
        aspect="auto",
    )
    ax.set_xlabel("x (nm)")
    ax.set_ylabel("depth z (nm)")
    ax.set_title(title or f"x-z slice (y index {row})")

    if show_colorbar:
        plt.colorbar(im, ax=ax, label="value")

    return ax


def plot_depth_dose(
    z,
    dose,
    *,
    z_label: str = "depth z (um)",
    dose_label: str = "absorbed dose (kJ/cm³)",
    title: str | None = None,
    ax=None,
):
    """Plot a 1D depth-dose profile (e.g. from ``LigaResult.depth_dose``)."""
    import matplotlib.pyplot as plt

    if ax is None:
        _, ax = plt.subplots(1, 1, figsize=(6, 4))

    ax.plot(np.asarray(z), np.asarray(dose), "b-", linewidth=1.5)
    ax.set_xlabel(z_label)
    ax.set_ylabel(dose_label)
    ax.set_title(title or "Depth Dose")
    ax.grid(True, alpha=0.3)
    return ax


def plot_height_map(
    height_map: HeightMapResult,
    *,
    title: str | None = None,
    cmap: str = "viridis",
    show_colorbar: bool = True,
    ax=None,
):
    """Plot a 2D remaining-height / development-depth map."""
    import matplotlib.pyplot as plt

    if ax is None:
        _, ax = plt.subplots(1, 1, figsize=(7, 6))

    values = np.asarray(height_map.values)
    x = np.asarray(height_map.x_nm)
    y = np.asarray(height_map.y_nm)

    im = ax.imshow(
        values,
        extent=[x[0], x[-1], y[-1], y[0]],
        cmap=cmap,
        aspect="equal",
    )
    ax.set_xlabel("x (nm)")
    ax.set_ylabel("y (nm)")
    ax.set_title(title or "Height Map (nm)")

    if show_colorbar:
        plt.colorbar(im, ax=ax, label="height (nm)")

    return ax


def plot_isosurface(
    volume: VolumetricResult,
    level: float,
    *,
    title: str | None = None,
    colorscale: str = "Viridis",
    opacity: float = 0.6,
):
    """Interactive 3D isosurface of a volumetric field at a given level (plotly)."""
    import plotly.graph_objects as go

    values = np.asarray(volume.values)  # (nz, ny, nx)
    x = np.asarray(volume.x_nm)
    y = np.asarray(volume.y_nm)
    z = np.asarray(volume.z_nm)

    # Build coordinate grids matching the (nz, ny, nx) data layout.
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
