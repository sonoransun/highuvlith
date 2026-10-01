"""Shared visual language for every highuvlith figure (light and dark).

One palette, one set of chart-chrome colors and one font stack for the
plotting helpers in :mod:`highuvlith.viz`, the documentation figures and the
notebooks, so a figure reads the same on the README, the GitHub Pages site
(light and dark schemes) and in a notebook.

The palette is the validated reference instance of the data-visualization
method this project follows:

* **Categorical** (series identity) — eight hues in a FIXED order, assigned
  in sequence and never cycled. The order is the color-vision-deficiency
  (CVD) safety mechanism: adjacent slots clear ΔE ≥ 8 (OKLab ×100) under
  simulated protanopia/deuteranopia in both themes; for scatter-type charts
  (any two marks can sit side by side) only the first three slots are
  pairwise safe. Three light-theme slots (aqua, yellow, magenta) sit below
  3:1 contrast on the light surface, so charts using them carry visible
  labels or a legend.
* **Sequential** (magnitude) — one blue hue, steps 100 → 700; light theme:
  low → light, high → dark; dark theme flips the anchor (low recedes into the
  dark surface).
* **Ordinal** (ordered classes such as doses or depths) — evenly spaced blue
  steps 250 → 650 (light) or 600 → 200 (dark), so the lightest/darkest step
  still clears 2:1 against the surface.
* **Diverging** (sign) — blue ↔ red through a neutral gray midpoint.

Text always wears the ink colors, never a series color.
"""

from __future__ import annotations

import contextlib
import pathlib
from collections.abc import Callable, Iterator
from typing import Any

__all__ = [
    "THEMES",
    "BLUE_RAMP",
    "axes_or_new",
    "categorical",
    "current_theme",
    "diverging_cmap",
    "export_light_dark",
    "figure_of",
    "ink",
    "ordinal_colors",
    "resolve_theme",
    "sequential_cmap",
    "style_axes",
    "style_colorbar",
    "use_theme",
]

#: Blue ramp, steps 100 … 700 (the sequential / ordinal hue).
BLUE_RAMP: dict[int, str] = {
    100: "#cde2fb",
    150: "#b7d3f6",
    200: "#9ec5f4",
    250: "#86b6ef",
    300: "#6da7ec",
    350: "#5598e7",
    400: "#3987e5",
    450: "#2a78d6",
    500: "#256abf",
    550: "#1c5cab",
    600: "#184f95",
    650: "#104281",
    700: "#0d366b",
}

#: Chart chrome, ink and the categorical slots per theme.
THEMES: dict[str, dict[str, Any]] = {
    "light": {
        "surface": "#fcfcfb",
        "page": "#f9f9f7",
        "ink_primary": "#0b0b0b",
        "ink_secondary": "#52514e",
        "ink_muted": "#898781",
        "gridline": "#e1e0d9",
        "baseline": "#c3c2b7",
        "border": (11 / 255, 11 / 255, 11 / 255, 0.10),
        "diverging_mid": "#f0efec",
        "categorical": (
            "#2a78d6",  # 1 blue
            "#eb6834",  # 2 orange
            "#1baf7a",  # 3 aqua
            "#eda100",  # 4 yellow
            "#e87ba4",  # 5 magenta
            "#008300",  # 6 green
            "#4a3aa7",  # 7 violet
            "#e34948",  # 8 red
        ),
    },
    "dark": {
        "surface": "#1a1a19",
        "page": "#0d0d0d",
        "ink_primary": "#ffffff",
        "ink_secondary": "#c3c2b7",
        "ink_muted": "#898781",
        "gridline": "#2c2c2a",
        "baseline": "#383835",
        "border": (1.0, 1.0, 1.0, 0.10),
        "diverging_mid": "#383835",
        "categorical": (
            "#3987e5",
            "#d95926",
            "#199e70",
            "#c98500",
            "#d55181",
            "#008300",
            "#9085e9",
            "#e66767",
        ),
    },
}

_THEME_STACK: list[str] = ["light"]


def resolve_theme(theme: str | None) -> str:
    """The theme to use: ``theme`` if given, else the current one."""
    name = current_theme() if theme is None else theme
    if name not in THEMES:
        raise ValueError(f"theme must be 'light' or 'dark', got {name!r}")
    return name


def current_theme() -> str:
    """The active theme ("light" outside any :func:`use_theme` block)."""
    return _THEME_STACK[-1]


def ink(role: str = "primary", theme: str | None = None) -> str:
    """Ink color: ``role`` is "primary", "secondary" or "muted"."""
    return str(THEMES[resolve_theme(theme)][f"ink_{role}"])


def categorical(index: int, theme: str | None = None) -> str:
    """Categorical slot ``index`` (0-based) in the fixed order.

    There are eight slots; a ninth series must fold into "other" or a
    separate panel rather than reuse a hue, so indices ≥ 8 raise.
    """
    slots = THEMES[resolve_theme(theme)]["categorical"]
    if not 0 <= index < len(slots):
        raise IndexError(
            f"categorical slot {index} out of range: fold extra series into 'other' "
            f"or facet instead of cycling the {len(slots)} hues"
        )
    return str(slots[index])


def ordinal_colors(n: int, theme: str | None = None) -> list[str]:
    """``n`` ordered colors from the blue ramp (low → high magnitude).

    Light theme: steps 250 → 650; dark theme: 600 → 200 (the end nearest the
    surface still clears 2:1 contrast). Steps are the documented ramp values
    (nearest documented step for ``n`` > 9).
    """
    if n < 1:
        return []
    light = resolve_theme(theme) == "light"
    lo, hi = (250, 650) if light else (600, 200)
    if n == 1:
        return [BLUE_RAMP[450 if light else 300]]
    steps = sorted(BLUE_RAMP)
    out = []
    for k in range(n):
        target = lo + (hi - lo) * k / (n - 1)
        nearest = min(steps, key=lambda s: abs(s - target))
        out.append(BLUE_RAMP[nearest])
    return out


def _register(name: str, colors: list[str]) -> Any:
    import matplotlib
    from matplotlib.colors import LinearSegmentedColormap

    try:
        return matplotlib.colormaps[name]
    except KeyError:
        cmap = LinearSegmentedColormap.from_list(name, colors)
        matplotlib.colormaps.register(cmap, name=name)
        return matplotlib.colormaps[name]


def sequential_cmap(theme: str | None = None) -> Any:
    """One-hue blue colormap for magnitudes (images, maps).

    Light theme: low = step 100 (light) → high = step 700 (dark). Dark theme:
    the anchor flips (low = step 700, high = step 100).
    """
    ramp = [BLUE_RAMP[s] for s in sorted(BLUE_RAMP)]
    if resolve_theme(theme) == "light":
        return _register("highuvlith-sequential-light", ramp)
    return _register("highuvlith-sequential-dark", ramp[::-1])


def diverging_cmap(theme: str | None = None) -> Any:
    """Blue ↔ red colormap through a neutral gray midpoint (signed values)."""
    name = resolve_theme(theme)
    t = THEMES[name]
    colors = [t["categorical"][0], t["diverging_mid"], t["categorical"][7]]
    return _register(f"highuvlith-diverging-{name}", colors)


def _rc(theme: str) -> dict[str, Any]:
    from cycler import cycler

    t = THEMES[theme]
    return {
        "figure.facecolor": t["surface"],
        "figure.edgecolor": t["surface"],
        "savefig.facecolor": t["surface"],
        "savefig.edgecolor": t["surface"],
        "axes.facecolor": t["surface"],
        "axes.edgecolor": t["baseline"],
        "axes.labelcolor": t["ink_secondary"],
        "axes.titlecolor": t["ink_primary"],
        "axes.titlesize": 11,
        "axes.labelsize": 9.5,
        "axes.linewidth": 0.8,
        "axes.grid": True,
        "axes.axisbelow": True,
        "axes.spines.top": False,
        "axes.spines.right": False,
        "axes.prop_cycle": cycler(color=list(t["categorical"])),
        "grid.color": t["gridline"],
        "grid.linewidth": 0.6,
        "grid.linestyle": "-",
        "text.color": t["ink_primary"],
        "xtick.color": t["baseline"],
        "ytick.color": t["baseline"],
        "xtick.labelcolor": t["ink_secondary"],
        "ytick.labelcolor": t["ink_secondary"],
        "xtick.labelsize": 8.5,
        "ytick.labelsize": 8.5,
        "legend.frameon": False,
        "legend.fontsize": 8.5,
        "legend.labelcolor": t["ink_secondary"],
        "legend.title_fontsize": 8.5,
        "lines.linewidth": 1.5,
        "lines.markersize": 6.0,
        "patch.linewidth": 0.8,
        "image.cmap": "highuvlith-sequential-" + theme,
        "font.family": "sans-serif",
        "font.sans-serif": [
            "Helvetica Neue",
            "Helvetica",
            "Arial",
            "Segoe UI",
            "Liberation Sans",
            "DejaVu Sans",
        ],
        "font.size": 9.5,
    }


@contextlib.contextmanager
def use_theme(theme: str = "light") -> Iterator[str]:
    """Context manager applying the light or dark highuvlith style.

    Matplotlib rc settings (surface, ink, grid, fonts, the categorical color
    cycle and the default image colormap) apply to figures created inside
    the block; the previous rc settings and theme are restored on exit.
    """
    import matplotlib

    name = resolve_theme(theme)
    sequential_cmap(name)  # make sure the default image colormap is registered
    _THEME_STACK.append(name)
    try:
        with matplotlib.rc_context(_rc(name)):
            yield name
    finally:
        _THEME_STACK.pop()


def style_axes(ax: Any, theme: str | None = None) -> Any:
    """Apply the theme to an existing Axes (surface, spines, ticks, grid, text).

    Plot helpers call it so an Axes created outside :func:`use_theme` still
    gets the theme's chrome.
    """
    t = THEMES[resolve_theme(theme)]
    ax.set_facecolor(t["surface"])
    for side, spine in ax.spines.items():
        spine.set_color(t["baseline"])
        spine.set_linewidth(0.8)
        if side in ("top", "right"):
            spine.set_visible(False)
    ax.tick_params(colors=t["baseline"], labelcolor=t["ink_secondary"])
    ax.xaxis.label.set_color(t["ink_secondary"])
    ax.yaxis.label.set_color(t["ink_secondary"])
    ax.title.set_color(t["ink_primary"])
    ax.grid(True, color=t["gridline"], linewidth=0.6)
    ax.set_axisbelow(True)
    legend = ax.get_legend()
    if legend is not None:
        for text in legend.get_texts():
            text.set_color(t["ink_secondary"])
        title = legend.get_title()
        if title is not None:
            title.set_color(t["ink_secondary"])
    return ax


def style_colorbar(colorbar: Any, theme: str | None = None) -> Any:
    """Theme a colorbar's outline, ticks and label."""
    t = THEMES[resolve_theme(theme)]
    colorbar.outline.set_edgecolor(t["baseline"])
    colorbar.ax.tick_params(colors=t["baseline"], labelcolor=t["ink_secondary"])
    colorbar.ax.yaxis.label.set_color(t["ink_secondary"])
    colorbar.ax.xaxis.label.set_color(t["ink_secondary"])
    return colorbar


def axes_or_new(ax: Any, figsize: tuple[float, float]) -> Any:
    """``ax`` if given, else the Axes of a new figure of size ``figsize`` (in)."""
    if ax is not None:
        return ax
    import matplotlib.pyplot as plt

    _, new_ax = plt.subplots(1, 1, figsize=figsize)
    return new_ax


def figure_of(obj: Any) -> Any:
    """The Figure behind a plot helper's return value (Axes or Figure)."""
    from matplotlib.figure import Figure

    if isinstance(obj, Figure):
        return obj
    figure = getattr(obj, "figure", None)
    if figure is None:
        raise TypeError(f"cannot find a matplotlib Figure in {type(obj).__name__}")
    return figure


def export_light_dark(
    plot_fn: Callable[..., Any],
    stem: str | pathlib.Path,
    *args: Any,
    formats: tuple[str, ...] = ("png",),
    dpi: int = 200,
    **kwargs: Any,
) -> list[pathlib.Path]:
    """Render ``plot_fn(*args, theme=..., **kwargs)`` in both themes and save
    ``<stem>-light.<fmt>`` and ``<stem>-dark.<fmt>`` for every format.

    ``plot_fn`` is any helper of :mod:`highuvlith.viz` (or any callable that
    accepts ``theme=`` and returns an Axes or Figure). The figures are closed
    after saving. Returns the written paths (light first).
    """
    import matplotlib.pyplot as plt

    stem = pathlib.Path(stem)
    stem.parent.mkdir(parents=True, exist_ok=True)
    written: list[pathlib.Path] = []
    for name in ("light", "dark"):
        with use_theme(name):
            fig = figure_of(plot_fn(*args, theme=name, **kwargs))
            try:
                for fmt in formats:
                    path = stem.with_name(f"{stem.name}-{name}.{fmt}")
                    fig.savefig(
                        path,
                        dpi=dpi,
                        facecolor=THEMES[name]["surface"],
                        bbox_inches="tight",
                    )
                    written.append(path)
            finally:
                plt.close(fig)
    return written
