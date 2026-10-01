"""Resist profile visualization (2D Dill + Mack/threshold development, 🔶)."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from highuvlith.viz.style import (
    axes_or_new,
    categorical,
    ink,
    resolve_theme,
    style_axes,
    use_theme,
)

if TYPE_CHECKING:
    from highuvlith import ResistProfileResult


def plot_resist_profile(
    result: ResistProfileResult,
    *,
    title: str | None = None,
    ax: Any = None,
    theme: str | None = None,
) -> Any:
    """Plot a developed resist profile: remaining height (nm) vs x (nm).

    The 2D resist path is simplified (🔶: depth-averaged Dill exposure,
    centre-row vertical development). The original thickness is drawn as a
    dashed reference with a direct label. Returns the Axes.
    """
    th = resolve_theme(theme)
    with use_theme(th):
        ax = axes_or_new(ax, (8.0, 3.4))
        x = result.x_nm
        height = result.height_nm
        thickness = result.thickness_nm
        color = categorical(0, th)
        ax.fill_between(x, 0.0, height, color=color, alpha=0.30, linewidth=0)
        ax.plot(x, height, color=color, linewidth=1.5)
        ax.axhline(thickness, color=ink("muted", th), linestyle="--", linewidth=1.0)
        ax.annotate(
            f"original thickness {thickness:.0f} nm",
            xy=(1.0, thickness),
            xycoords=("axes fraction", "data"),
            xytext=(-4, 3),
            textcoords="offset points",
            ha="right",
            va="bottom",
            fontsize=8,
            color=ink("secondary", th),
        )
        ax.set_xlabel("x (nm)")
        ax.set_ylabel("Resist height (nm)")
        ax.set_ylim(0.0, thickness * 1.15)
        ax.set_title(title or "Developed resist profile")
        style_axes(ax, th)
    return ax
