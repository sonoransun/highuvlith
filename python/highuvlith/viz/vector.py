"""Vector (polarized) imaging visualization: two-beam contrast vs NA."""

from __future__ import annotations

from collections.abc import Sequence
from typing import Any

import numpy as np

from highuvlith.viz.style import (
    axes_or_new,
    categorical,
    ink,
    resolve_theme,
    style_axes,
    use_theme,
)

_POLARIZATION_LABELS = {
    "y": "TE (y-polarized)",
    "x": "TM (x-polarized)",
    "te": "TE (azimuthal)",
    "tm": "TM (radial)",
    "unpolarized": "unpolarized",
}


def plot_polarization_contrast(
    na: Sequence[float] | np.ndarray | None = None,
    *,
    polarizations: Sequence[str] = ("y", "x", "unpolarized"),
    film_n: float | None = None,
    ax: Any = None,
    theme: str | None = None,
) -> Any:
    """Two-beam fringe contrast vs numerical aperture under the vector model.

    Two orders at the pupil edge (sin θ = NA) interfere; with the orders
    along x, y-polarized light is TE (contrast 1), x-polarized is TM
    (|cos 2θ|, a null at NA = 1/√2 in air) and unpolarized light gives
    cos²θ — the closed forms ``api.vector_two_beam_contrast`` evaluates
    (ideal TE/TM, thin mask, no aberrations). ``film_n`` evaluates the
    contrast just inside a film of that index (refracted angle). ``na``
    defaults to 40 values over 0.2–0.95. Returns the Axes.
    """
    from highuvlith import api

    values = np.linspace(0.2, 0.95, 40) if na is None else np.asarray(na, dtype=float)
    th = resolve_theme(theme)
    with use_theme(th):
        ax = axes_or_new(ax, (6.4, 4.2))
        for i, pol in enumerate(polarizations):
            contrast = [
                api.vector_two_beam_contrast(float(a), pol, film_n=film_n) for a in values
            ]
            label = _POLARIZATION_LABELS.get(pol, pol)
            ax.plot(values, contrast, color=categorical(i, th), linewidth=1.5, label=label)
            if len(polarizations) <= 4:
                ax.annotate(
                    label,
                    xy=(values[-1], contrast[-1]),
                    xytext=(4, 0),
                    textcoords="offset points",
                    va="center",
                    fontsize=8,
                    color=ink("secondary", th),
                )
        null_na = 1.0 / np.sqrt(2.0)
        if film_n is None and values.min() < null_na < values.max():
            ax.axvline(null_na, color=ink("muted", th), linestyle=":", linewidth=1.0)
            ax.annotate(
                "TM null (2θ = 90°)",
                xy=(null_na, 1.0),
                xytext=(3, -10),
                textcoords="offset points",
                fontsize=8,
                color=ink("secondary", th),
            )
        ax.set_xlabel("Numerical aperture (orders at the pupil edge)")
        ax.set_ylabel("Two-beam fringe contrast")
        where = f" in a film of n = {film_n:g}" if film_n is not None else ""
        ax.set_title(f"Two-beam contrast vs NA (vector model{where})")
        ax.set_ylim(0.0, 1.05)
        ax.set_xlim(float(values.min()), float(values.max()) + 0.12 * float(np.ptp(values)))
        if len(polarizations) >= 2:
            ax.legend(loc="lower left")
        style_axes(ax, th)
    return ax
