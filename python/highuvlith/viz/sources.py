"""Light-source visualization: spectrum + pupil fill, and the power landscape.

All numbers come from the source models (``SourceConfig``): spectral samples
and pupil fill are what the imaging engine uses; average powers are each
model's ``average_power_w`` (different definitions per family — plasma
sources in band at intermediate focus, lasers at the output, X-ray tubes as
4π X-ray power — see ``SourcePreset.power_definition``).
"""

from __future__ import annotations

from collections.abc import Sequence
from typing import TYPE_CHECKING, Any

import numpy as np

from highuvlith.viz.style import (
    THEMES,
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
    from highuvlith import SourceConfig

#: Maturity classes of the landscape, in categorical slot order (the three
#: slots that stay distinguishable pairwise in a scatter), with marker shapes
#: as the secondary encoding.
MATURITY_CLASSES: tuple[tuple[str, str], ...] = (
    ("demonstrated", "o"),
    ("projection", "s"),
    ("theoretical", "^"),
)


def plot_source(
    source: SourceConfig,
    *,
    n: int = 101,
    extent: float = 1.0,
    theme: str | None = None,
    fig: Any = None,
) -> Any:
    """Source spectrum (left) and pupil fill (right) as the imaging engine
    samples them.

    Left: the spectral samples ``source.spectrum()`` as stems (weight vs
    wavelength; narrow lines are shown as Δλ in pm from the center
    wavelength). Right: ``source.pupil_fill(n, extent)`` over σ (units of
    NA/λ) with the σ = 1 pupil edge. Returns the Figure.
    """
    import matplotlib.pyplot as plt
    from matplotlib.patches import Circle

    th = resolve_theme(theme)
    with use_theme(th):
        if fig is None:
            fig = plt.figure(figsize=(9.6, 3.9))
        ax_spec, ax_fill = fig.subplots(1, 2, gridspec_kw={"width_ratios": [1.25, 1.0]})
        samples = np.asarray(source.spectrum(), dtype=float).reshape(-1, 2)
        center = float(source.wavelength_nm)
        wl, weight = samples[:, 0], samples[:, 1]
        span = float(wl.max() - wl.min()) if wl.size > 1 else 0.0
        if span < 1.0:
            x = (wl - center) * 1e3
            ax_spec.set_xlabel(f"Δλ from {center:.4g} nm (pm)")
        else:
            x = wl
            ax_spec.set_xlabel("Wavelength (nm)")
        color = categorical(0, th)
        markerline, stemlines, baseline = ax_spec.stem(x, weight, basefmt=" ")
        markerline.set_color(color)
        markerline.set_markersize(5)
        stemlines.set_color(color)
        stemlines.set_linewidth(1.5)
        ax_spec.set_ylabel("Spectral weight")
        ax_spec.set_ylim(bottom=0.0)
        ax_spec.set_title(f"Spectrum ({len(wl)} sample{'s' if len(wl) != 1 else ''})")
        style_axes(ax_spec, th)

        fill = np.asarray(source.pupil_fill(n, extent))
        im = ax_fill.imshow(
            fill,
            extent=(-extent, extent, -extent, extent),
            origin="lower",
            cmap=sequential_cmap(th),
            aspect="equal",
            interpolation="nearest",
        )
        ax_fill.add_patch(
            Circle((0.0, 0.0), 1.0, fill=False, edgecolor=ink("secondary", th), linestyle="--", linewidth=1.0)
        )
        ax_fill.set_xlabel("σx (NA/λ)")
        ax_fill.set_ylabel("σy (NA/λ)")
        ax_fill.set_title("Pupil fill")
        style_colorbar(fig.colorbar(im, ax=ax_fill, label="Relative source intensity"), th)
        style_axes(ax_fill, th)
        ax_fill.grid(False)
        fig.suptitle(f"{source.kind} source, λ = {center:.4g} nm", color=ink("primary", th))
    return fig


def plot_source_landscape(
    presets: Sequence[Any] | None = None,
    *,
    hvm_band_w: tuple[float, float] = (250.0, 1000.0),
    annotate: bool = True,
    ax: Any = None,
    theme: str | None = None,
) -> Any:
    """Average power (W) vs wavelength (nm) of the simulator's source presets.

    ``presets`` defaults to :func:`highuvlith.api.source_landscape`; each item
    needs ``label``, ``wavelength_nm``, ``average_power_w`` (None = not
    modeled; skipped and counted in the note), ``maturity``
    ("demonstrated" | "projection" | "theoretical") and ``broadband``. Color
    and marker encode maturity; broadband (X-ray tube, betatron: mean
    wavelength) presets are hollow. The EUV HVM requirement band at
    intermediate focus (``hvm_band_w``, default 250 W – 1 kW) is shaded over
    5–20 nm only — it is not a requirement at other wavelengths. Powers are
    each model's own ``average_power_w`` definition (see
    ``power_definition``), so they are not all measured at the same plane.
    Returns the Axes.
    """
    if presets is None:
        from highuvlith import api

        landscape = getattr(api, "source_landscape", None)
        if landscape is None:
            raise RuntimeError("highuvlith.api.source_landscape is not available; pass presets")
        presets = landscape()
    th = resolve_theme(theme)
    surface = THEMES[th]["surface"]
    with use_theme(th):
        ax = axes_or_new(ax, (8.6, 5.6))
        lo, hi = hvm_band_w
        ax.fill_between([5.0, 20.0], [lo, lo], [hi, hi], color=ink("muted", th), alpha=0.18, linewidth=0)
        ax.annotate(
            f"EUV HVM at IF ({lo:.0f} W – {hi / 1000:.0f} kW)" if hi >= 1000 else f"EUV HVM at IF ({lo:.0f} W – {hi:.0f} W)",
            xy=(5.0, hi),
            xytext=(0, 3),
            textcoords="offset points",
            fontsize=8,
            color=ink("secondary", th),
        )
        skipped = []
        for slot, (maturity, marker) in enumerate(MATURITY_CLASSES):
            color = categorical(slot, th)
            group = [
                p
                for p in presets
                if p.maturity == maturity and p.average_power_w is not None and p.average_power_w > 0
            ]
            if not group:
                continue
            for broadband in (False, True):
                pts = [p for p in group if bool(p.broadband) == broadband]
                if not pts:
                    continue
                ax.scatter(
                    [p.wavelength_nm for p in pts],
                    [p.average_power_w for p in pts],
                    marker=marker,
                    s=46,
                    facecolors="none" if broadband else color,
                    edgecolors=color if broadband else surface,
                    linewidths=1.5 if broadband else 0.9,
                    label=maturity if not broadband else f"{maturity} (broadband, mean λ)",
                    zorder=3,
                )
        for p in presets:
            if p.average_power_w is None or p.average_power_w <= 0:
                skipped.append(p.label)
            elif annotate:
                ax.annotate(
                    p.label,
                    xy=(p.wavelength_nm, p.average_power_w),
                    xytext=(4, 3),
                    textcoords="offset points",
                    fontsize=6.5,
                    color=ink("secondary", th),
                )
        ax.set_xscale("log")
        ax.set_yscale("log")
        ax.set_xlabel("Wavelength (nm)")
        ax.set_ylabel("Average power (W)")
        ax.set_title("Light-source landscape: simulator presets")
        ax.legend(loc="lower right", title="Maturity")
        if skipped:
            ax.text(
                0.0,
                -0.16,
                f"Not shown (no average power modeled): {', '.join(skipped)}",
                transform=ax.transAxes,
                fontsize=7,
                color=ink("muted", th),
                va="top",
                wrap=True,
            )
        style_axes(ax, th)
    return ax
