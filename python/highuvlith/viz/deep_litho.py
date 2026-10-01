"""Deep-lithography visualization: LIGA Fresnel edge profile, Talbot carpet."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

import numpy as np

from highuvlith.viz.style import (
    axes_or_new,
    ink,
    ordinal_colors,
    resolve_theme,
    sequential_cmap,
    style_axes,
    style_colorbar,
    use_theme,
)

if TYPE_CHECKING:
    from highuvlith._native import LigaEdgeProfile, TalbotResult


def plot_liga_edge_profile(
    profile: LigaEdgeProfile,
    *,
    threshold_kj_cm3: float | None = None,
    ax: Any = None,
    theme: str | None = None,
) -> Any:
    """Absorbed dose (kJ/cm³) across a straight LIGA absorber edge.

    One curve per depth (ordinal ramp, top of the resist first): the
    analytic Fresnel straight-edge solution per energy bin, summed with the
    depth-dose weights (``liga_edge_profile``; scalar thin-screen absorber,
    no photoelectron transport). ``threshold_kj_cm3`` draws a developer
    threshold and marks the developed-edge position at each depth.
    x > 0 is the open side. Returns the Axes.
    """
    th = resolve_theme(theme)
    with use_theme(th):
        ax = axes_or_new(ax, (7.2, 4.4))
        x = np.asarray(profile.x_nm)
        depths = np.asarray(profile.z_um)
        dose = np.asarray(profile.dose_kj_cm3)
        colors = ordinal_colors(len(depths), th)
        for color, z, row in zip(colors, depths, dose):
            ax.plot(x, row, color=color, linewidth=1.5, label=f"{z:.4g}")
        if threshold_kj_cm3 is not None:
            ax.axhline(threshold_kj_cm3, color=ink("muted", th), linestyle="--", linewidth=1.0)
            ax.annotate(
                f"threshold {threshold_kj_cm3:.3g} kJ/cm³",
                xy=(0.0, threshold_kj_cm3),
                xycoords=("axes fraction", "data"),
                xytext=(4, 3),
                textcoords="offset points",
                fontsize=8,
                color=ink("secondary", th),
            )
            for color, edge in zip(colors, profile.edge_positions_nm(threshold_kj_cm3)):
                if edge is not None:
                    ax.plot([edge], [threshold_kj_cm3], "o", color=color, markersize=5)
        ax.axvline(0.0, color=ink("muted", th), linewidth=0.8)
        ax.set_xlabel("x from the geometric absorber edge (nm)")
        ax.set_ylabel("Absorbed dose (kJ/cm³)")
        ax.set_title("LIGA edge profile (Fresnel diffraction)")
        if len(depths) >= 2:
            ax.legend(title="Depth (µm)", loc="upper left")
        style_axes(ax, th)
    return ax


def plot_talbot_carpet(
    result: TalbotResult,
    *,
    mark_talbot_length: bool = True,
    ax: Any = None,
    theme: str | None = None,
) -> Any:
    """Coherent Talbot carpet I(x, z) behind a grating (scalar thin-mask
    model with angular-spectrum propagation, 🔶).

    Intensity is relative to the incident plane wave; z is the distance
    behind the grating (nm). Dashed lines mark z_T/2 and the paraxial Talbot
    length z_T = 2p²/λ when inside the plotted range. Returns the Axes.
    """
    th = resolve_theme(theme)
    with use_theme(th):
        ax = axes_or_new(ax, (6.4, 5.2))
        carpet = np.asarray(result.carpet)
        x = np.asarray(result.carpet_x_nm, dtype=float)
        z = np.asarray(result.carpet_z_nm, dtype=float)
        dx = float(x[1] - x[0]) if x.size > 1 else 1.0
        dz = float(z[1] - z[0]) if z.size > 1 else 1.0
        im = ax.imshow(
            carpet,
            extent=(
                float(x[0]) - dx / 2,
                float(x[-1]) + dx / 2,
                float(z[0]) - dz / 2,
                float(z[-1]) + dz / 2,
            ),
            origin="lower",
            cmap=sequential_cmap(th),
            aspect="auto",
            interpolation="nearest",
        )
        if mark_talbot_length:
            z_t = float(result.talbot_length_nm)
            for frac, label in ((0.5, "z_T/2"), (1.0, "z_T")):
                zz = frac * z_t
                if z[0] <= zz <= z[-1]:
                    ax.axhline(zz, color=ink("secondary", th), linestyle="--", linewidth=1.0)
                    ax.annotate(
                        label,
                        xy=(1.0, zz),
                        xycoords=("axes fraction", "data"),
                        xytext=(-4, 3),
                        textcoords="offset points",
                        ha="right",
                        fontsize=8,
                        color=ink("secondary", th),
                    )
        ax.set_xlabel("x (nm)")
        ax.set_ylabel("Distance behind the grating z (nm)")
        ax.set_title(
            f"Talbot carpet (p = {result.period_nm:g} nm, λ = {result.wavelength_nm:g} nm)"
        )
        style_colorbar(ax.figure.colorbar(im, ax=ax, label="Intensity (incident = 1)"), th)
        style_axes(ax, th)
        ax.grid(False)
    return ax
