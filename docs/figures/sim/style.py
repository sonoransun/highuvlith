"""One visual language for every simulator-driven figure (README, site, notebooks).

Every script in ``docs/figures/sim/`` draws through this module, and
``highuvlith.viz`` mirrors the same tokens, so a chart reads the same wherever it
appears. The values follow the data-viz method used across the site (the same
palette as the Process Nodes charts): a fixed categorical order, one-hue
sequential ramps, a blue <-> red diverging pair, thin marks and recessive chrome.

Surfaces
    Figures are drawn on a transparent background and validated against the two
    page surfaces they sit on: the site's light page ``#ffffff`` and its dark
    ("slate") page ``#201e29`` (Material's ``hsl(250 15% 14%)`` with the site's
    violet hue 250). They also read on GitHub's ``#ffffff`` / ``#0d1117``.

Categorical palette (identity; assign slots in order, never cycle)
    Light: ``#2a78d6 #eb6834 #1baf7a #eda100 #e87ba4 #008300 #4a3aa7 #e34948``
    Dark:  ``#3987e5 #d95926 #199e70 #c98500 #d55181 #008300 #9085e9 #e66767``
    Validated with the palette checker on the surfaces above: adjacent pairs pass
    in both modes (worst CVD Delta E 9.1 light / 8.4 dark, normal vision 19.6 /
    19.3); the first three slots also pass all-pairs (scatter). Light-mode aqua,
    yellow and magenta sit below 3:1 on white, so charts that use them carry
    direct labels (and captions/tables carry the numbers).

Field ramp (2D images of intensity, dose, height; mode-invariant)
    One blue hue, OKLab-interpolated through the documented blue ramp
    (steps 100-700) and extended at both ends to near-black and near-white.
    Dark means "no light / no dose", light means "full" in BOTH themes: these
    images depict light, so brightness carries the quantity. Every such image
    has a colour bar.

Diverging pair
    blue ``#2a78d6`` <-> red ``#e34948`` with a neutral grey midpoint
    (light ``#f0efec``, dark ``#3b3943``).

Chrome
    Light: ink ``#0b0b0b``, secondary ``#52514e`` (7.9:1), muted ``#6b6a65``
    (5.4:1), grid ``#e1e0d9``, axis ``#c3c2b7``.
    Dark:  ink ``#f4f3f8``, secondary ``#c3c2b7`` (9.2:1), muted ``#9b99a3``
    (5.9:1), grid ``#32303a``, axis ``#4d4b54``.
    Text never wears a series colour; identity comes from the mark beside it.

Type and marks
    System sans (Helvetica Neue / Helvetica / Arial / Liberation Sans / DejaVu
    Sans, first available). Base 9.5 pt at 100 dpi logical, rendered at 2x.
    Lines 2 px, markers >= 8 px with a 2 px surface ring, hairline solid grid,
    dashed lines only for limits, thresholds and projections.
"""

from __future__ import annotations

import io
import math
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Iterator, Sequence

import matplotlib

matplotlib.use("Agg")
import matplotlib.patches  # noqa: E402
import matplotlib.pyplot as plt  # noqa: E402
import matplotlib.transforms  # noqa: E402
from matplotlib.colors import LinearSegmentedColormap, to_rgb  # noqa: E402
from PIL import Image, PngImagePlugin  # noqa: E402

# ---------------------------------------------------------------------------
# Tokens
# ---------------------------------------------------------------------------

FONT_STACK = ["Helvetica Neue", "Helvetica", "Arial", "Liberation Sans", "DejaVu Sans"]


def font_families() -> list[str]:
    """The installed members of ``FONT_STACK``, in order, ending with DejaVu Sans.

    Used as an explicit ``font.family`` list so matplotlib falls back glyph by
    glyph (e.g. arrows and subscript digits missing from Helvetica Neue).
    """
    from matplotlib import font_manager

    installed = {f.name for f in font_manager.fontManager.ttflist}
    fams = [f for f in FONT_STACK if f in installed]
    return fams if "DejaVu Sans" in fams else fams + ["DejaVu Sans"]

#: Logical resolution (px per inch) at which sizes are specified; PNGs are
#: written at 2x this for sharp rendering on high-density screens.
LOGICAL_DPI = 100
EXPORT_SCALE = 2

#: Figure widths in inches at ``LOGICAL_DPI`` (800 px logical = one text column).
FULL_WIDTH = 8.0
WIDE_WIDTH = 9.6


@dataclass(frozen=True)
class Theme:
    """Colour tokens for one page surface."""

    name: str
    surface: str
    ink: str
    ink2: str
    muted: str
    grid: str
    axis: str
    series: tuple[str, ...]
    diverging_mid: str
    band: str  # faint fill for tolerance bands / windows (mark-free areas)

    def s(self, slot: int) -> str:
        """Categorical colour for 1-based ``slot`` (fixed order, never cycled)."""
        if not 1 <= slot <= len(self.series):
            raise ValueError(f"categorical slot {slot} outside 1..{len(self.series)}")
        return self.series[slot - 1]


LIGHT = Theme(
    name="light",
    surface="#ffffff",
    ink="#0b0b0b",
    ink2="#52514e",
    muted="#6b6a65",
    grid="#e1e0d9",
    axis="#c3c2b7",
    series=("#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4", "#008300", "#4a3aa7", "#e34948"),
    diverging_mid="#f0efec",
    band="#f0efec",
)

DARK = Theme(
    name="dark",
    surface="#201e29",
    ink="#f4f3f8",
    ink2="#c3c2b7",
    muted="#9b99a3",
    grid="#32303a",
    axis="#4d4b54",
    series=("#3987e5", "#d95926", "#199e70", "#c98500", "#d55181", "#008300", "#9085e9", "#e66767"),
    diverging_mid="#3b3943",
    band="#2c2a35",
)

THEMES = (LIGHT, DARK)

#: Ordinal ramps (ordered classes such as doses): one hue, monotone lightness,
#: validated with the palette checker's --ordinal mode on each surface. Light
#: mode runs light -> dark (more = darker); dark mode runs dark -> light.
ORDINAL = {
    "light": ("#86b6ef", "#5598e7", "#2a78d6", "#1c5cab", "#104281"),
    "dark": ("#184f95", "#256abf", "#3987e5", "#6da7ec", "#9ec5f4"),
}


def ordinal(theme: "Theme", n: int) -> list[str]:
    """``n`` (<= 5) evenly spaced steps of the theme's ordinal ramp, low -> high."""
    ramp = ORDINAL[theme.name]
    if not 1 <= n <= len(ramp):
        raise ValueError(f"ordinal ramp has {len(ramp)} validated steps, asked for {n}")
    if n == 1:
        return [ramp[2]]
    return [ramp[round(i * (len(ramp) - 1) / (n - 1))] for i in range(n)]

#: Documented blue ramp (steps 100..700), light -> dark.
BLUE_RAMP = (
    "#cde2fb", "#b7d3f6", "#9ec5f4", "#86b6ef", "#6da7ec", "#5598e7", "#3987e5",
    "#2a78d6", "#256abf", "#1c5cab", "#184f95", "#104281", "#0d366b",
)

# Field-ramp anchors (OKLCH L, C, h), dark -> light. The middle thirteen are the
# documented blue steps (OKLCH measured from BLUE_RAMP; L falls 0.047 per step);
# the ramp continues that spacing to L 0.15 (chroma tapering) and up to L 0.985.
_FIELD_ANCHORS_OKLCH = (
    (0.150, 0.035, 257.5),
    (0.197, 0.055, 257.4),
    (0.244, 0.075, 257.2),
    (0.291, 0.090, 257.1),
    (0.338, 0.103, 256.9),
    (0.385, 0.118, 256.7),
    (0.433, 0.128, 256.5),
    (0.480, 0.142, 256.1),
    (0.527, 0.150, 255.9),
    (0.575, 0.163, 255.5),
    (0.622, 0.161, 255.1),
    (0.671, 0.136, 253.8),
    (0.717, 0.118, 253.6),
    (0.764, 0.097, 253.2),
    (0.812, 0.079, 253.4),
    (0.858, 0.057, 254.1),
    (0.905, 0.041, 252.8),
    (0.950, 0.022, 253.0),
    (0.985, 0.008, 253.0),
)


# ---------------------------------------------------------------------------
# Colour maths (OKLab <-> sRGB) for the ramps
# ---------------------------------------------------------------------------


def _lin_to_srgb(c: float) -> float:
    c = min(1.0, max(0.0, c))
    return 12.92 * c if c <= 0.0031308 else 1.055 * c ** (1 / 2.4) - 0.055


def _srgb_to_lin(c: float) -> float:
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def oklab_to_rgb(lab: tuple[float, float, float]) -> tuple[float, float, float]:
    """OKLab -> gamma-encoded sRGB in [0, 1] (clipped)."""
    L, a, b = lab
    l_ = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3
    m_ = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3
    s_ = (L - 0.0894841775 * a - 1.2914855480 * b) ** 3
    r = 4.0767416621 * l_ - 3.3077115913 * m_ + 0.2309699292 * s_
    g = -1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_
    bb = -0.0041960863 * l_ - 0.7034186147 * m_ + 1.7076147010 * s_
    return (_lin_to_srgb(r), _lin_to_srgb(g), _lin_to_srgb(bb))


def rgb_to_oklab(rgb: tuple[float, float, float]) -> tuple[float, float, float]:
    """Gamma-encoded sRGB in [0, 1] -> OKLab."""
    r, g, b = (_srgb_to_lin(c) for c in rgb)
    l_ = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b) ** (1 / 3)
    m_ = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b) ** (1 / 3)
    s_ = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b) ** (1 / 3)
    return (
        0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
    )


def _oklch_to_oklab(lch: tuple[float, float, float]) -> tuple[float, float, float]:
    L, C, h = lch
    return (L, C * math.cos(math.radians(h)), C * math.sin(math.radians(h)))


def _ramp_from_oklab(anchors: Sequence[tuple[float, float, float]], name: str, n: int = 256) -> LinearSegmentedColormap:
    """Colormap interpolated linearly in OKLab through ``anchors`` (evenly spaced)."""
    k = len(anchors) - 1
    colours = []
    for i in range(n):
        t = i / (n - 1) * k
        j = min(int(t), k - 1)
        f = t - j
        lab = tuple(anchors[j][c] * (1 - f) + anchors[j + 1][c] * f for c in range(3))
        colours.append(oklab_to_rgb(lab))  # type: ignore[arg-type]
    return LinearSegmentedColormap.from_list(name, colours, N=n)


def field_cmap() -> LinearSegmentedColormap:
    """Mode-invariant single-hue ramp for 2D fields: dark = 0, light = maximum."""
    return _ramp_from_oklab([_oklch_to_oklab(a) for a in _FIELD_ANCHORS_OKLCH], "huv_field")


def diverging_cmap(theme: Theme) -> LinearSegmentedColormap:
    """blue <-> neutral <-> red, interpolated in OKLab (equal arms)."""
    blue = rgb_to_oklab(to_rgb(theme.s(1)))
    red = rgb_to_oklab(to_rgb(theme.s(8)))
    mid = rgb_to_oklab(to_rgb(theme.diverging_mid))
    return _ramp_from_oklab([blue, mid, red], f"huv_div_{theme.name}")


def mix(a: str, b: str, t: float) -> str:
    """sRGB mix of two hex colours (t = 0 -> a, 1 -> b)."""
    ra, rb = to_rgb(a), to_rgb(b)
    return "#%02x%02x%02x" % tuple(round(255 * (x * (1 - t) + y * t)) for x, y in zip(ra, rb))


def sci(value: float, digits: int = 2) -> str:
    """Readable number for labels: plain for 0.01..1e4, mathtext ``a×10^b`` otherwise."""
    if value == 0 or not math.isfinite(value):
        return f"{value:g}"
    if 1e-2 <= abs(value) < 1e4:
        return f"{float(f'{value:.3g}'):g}"
    exp = int(math.floor(math.log10(abs(value))))
    mant = value / 10 ** exp
    if round(mant, digits - 1) >= 10:
        mant /= 10
        exp += 1
    return rf"${mant:.{digits - 1}f}\times10^{{{exp}}}$"


def text_on(fill_rgb: Sequence[float], theme: Theme) -> str:
    """Ink for text set inside a coloured fill: white or near-black by luminance."""
    lum = 0.2126 * _srgb_to_lin(fill_rgb[0]) + 0.7152 * _srgb_to_lin(fill_rgb[1]) + 0.0722 * _srgb_to_lin(fill_rgb[2])
    return "#0b0b0b" if lum > 0.25 else "#ffffff"


# ---------------------------------------------------------------------------
# matplotlib setup
# ---------------------------------------------------------------------------

BASE_PT = 9.5
SMALL_PT = 8.5
TITLE_PT = 10.5
LINE_PT = 2.0 * 72 / LOGICAL_DPI  # 2 px
HAIR_PT = 1.0 * 72 / LOGICAL_DPI  # 1 px
MARKER_PT = 8.4 * 72 / LOGICAL_DPI  # >= 8 px diameter
RING_PT = 2.0 * 72 / LOGICAL_DPI  # 2 px surface ring


def rc(theme: Theme) -> dict[str, object]:
    """rcParams for ``theme`` (transparent background, recessive chrome)."""
    return {
        "font.family": font_families(),
        "font.sans-serif": FONT_STACK,
        "font.size": BASE_PT,
        "axes.titlesize": TITLE_PT,
        "axes.titleweight": "semibold",
        "axes.titlelocation": "left",
        "axes.titlepad": 8.0,
        "axes.titlecolor": theme.ink,
        "axes.labelsize": BASE_PT,
        "axes.labelcolor": theme.ink2,
        "axes.edgecolor": theme.axis,
        "axes.linewidth": HAIR_PT,
        "axes.facecolor": "none",
        "axes.grid": False,
        "axes.axisbelow": True,
        "axes.spines.top": False,
        "axes.spines.right": False,
        "axes.prop_cycle": matplotlib.cycler(color=list(theme.series)),
        "grid.color": theme.grid,
        "grid.linewidth": HAIR_PT,
        "grid.linestyle": "-",
        "xtick.color": theme.axis,
        "ytick.color": theme.axis,
        "xtick.labelcolor": theme.ink2,
        "ytick.labelcolor": theme.ink2,
        "xtick.labelsize": SMALL_PT,
        "ytick.labelsize": SMALL_PT,
        "xtick.major.size": 3.0,
        "ytick.major.size": 3.0,
        "xtick.major.width": HAIR_PT,
        "ytick.major.width": HAIR_PT,
        "xtick.minor.size": 1.8,
        "ytick.minor.size": 1.8,
        "xtick.minor.width": HAIR_PT,
        "ytick.minor.width": HAIR_PT,
        "lines.linewidth": LINE_PT,
        "lines.solid_capstyle": "round",
        "lines.solid_joinstyle": "round",
        "lines.markersize": MARKER_PT,
        "lines.markeredgewidth": RING_PT,
        "legend.frameon": False,
        "legend.fontsize": SMALL_PT,
        "legend.labelcolor": theme.ink2,
        "legend.handlelength": 1.6,
        "legend.borderaxespad": 0.2,
        "figure.facecolor": "none",
        "figure.edgecolor": "none",
        "figure.dpi": LOGICAL_DPI,
        "savefig.facecolor": "none",
        "savefig.edgecolor": "none",
        "savefig.transparent": True,
        "image.interpolation": "nearest",
        "mathtext.default": "regular",
        "text.color": theme.ink,
        "svg.fonttype": "path",
        "svg.hashsalt": "highuvlith-sim",
    }


@contextmanager
def use(theme: Theme) -> Iterator[Theme]:
    """Context manager applying ``theme``'s rcParams."""
    with matplotlib.rc_context(rc(theme)):
        yield theme


def style_axes(ax, theme: Theme, grid: str | None = "y") -> None:
    """Recessive axes: hairline solid grid behind the data, no top/right spines."""
    if grid:
        ax.grid(True, axis=grid, color=theme.grid, linewidth=HAIR_PT, linestyle="-")
    ax.set_axisbelow(True)
    for side in ("top", "right"):
        ax.spines[side].set_visible(False)
    for side in ("left", "bottom"):
        ax.spines[side].set_color(theme.axis)


def image_axes(ax, theme: Theme) -> None:
    """Axes that frame a 2D field: thin axis-coloured frame, ticks outside."""
    for side in ("top", "right", "left", "bottom"):
        ax.spines[side].set_visible(True)
        ax.spines[side].set_color(theme.axis)
        ax.spines[side].set_linewidth(HAIR_PT)


def colorbar(fig, mappable, ax, theme: Theme, label: str, **kw):
    """Slim colour bar with theme chrome."""
    cb = fig.colorbar(mappable, ax=ax, fraction=kw.pop("fraction", 0.046), pad=kw.pop("pad", 0.03), **kw)
    cb.outline.set_edgecolor(theme.axis)
    cb.outline.set_linewidth(HAIR_PT)
    cb.ax.tick_params(colors=theme.axis, labelcolor=theme.ink2, labelsize=SMALL_PT, width=HAIR_PT, length=2.5)
    cb.set_label(label, color=theme.ink2, fontsize=SMALL_PT)
    return cb


def _offset(ax, dx_px: float = 0.0, dy_px: float = 0.0, base=None):
    """``base`` transform shifted by a fixed screen offset in logical pixels."""
    from matplotlib.transforms import offset_copy

    return offset_copy(base if base is not None else ax.transData, fig=ax.figure,
                       x=dx_px * 72 / LOGICAL_DPI, y=dy_px * 72 / LOGICAL_DPI, units="points")


def limit_line(ax, value: float, theme: Theme, label: str | None = None, axis: str = "x", where: float = 0.97,
               side: str = "left", va: str = "top", gap_px: float = 3.0, **kw):
    """Dashed reference line (a limit / threshold / projection) with a muted label beside it.

    For a vertical line (``axis="x"``) the label is rotated and placed on ``side``
    of the line, starting at ``where`` (axes fraction) and running towards
    ``va``; for a horizontal line it sits above (``va="bottom"``) or below.
    """
    line_kw = dict(color=theme.ink2, linewidth=HAIR_PT * 1.3, linestyle=(0, (4, 3)), zorder=1.5)
    line_kw.update(kw)
    if axis == "x":
        ax.axvline(value, **line_kw)
        if label:
            dx = -gap_px if side == "left" else gap_px
            ax.text(value, where, label, transform=_offset(ax, dx, 0, ax.get_xaxis_transform()),
                    ha="right" if side == "left" else "left", va=va, color=theme.ink2, fontsize=SMALL_PT,
                    rotation=90)
    else:
        ax.axhline(value, **line_kw)
        if label:
            dy = gap_px if va == "bottom" else -gap_px
            ax.text(where, value, label, transform=_offset(ax, 0, dy, ax.get_yaxis_transform()),
                    ha="right" if side == "left" else "left", va=va, color=theme.ink2, fontsize=SMALL_PT)


def end_label(ax, x: float, y: float, text: str, theme: Theme, dx_px: float = 6.0, dy_px: float = 0.0, **kw):
    """Direct label at a line end, in secondary ink (never the series colour)."""
    kw.setdefault("color", theme.ink2)
    kw.setdefault("fontsize", SMALL_PT)
    kw.setdefault("va", "center")
    kw.setdefault("ha", "left")
    return ax.annotate(text, (x, y), xytext=(dx_px * 72 / LOGICAL_DPI, dy_px * 72 / LOGICAL_DPI),
                       textcoords="offset points", annotation_clip=False, **kw)


def end_labels(ax, items, theme: Theme, min_gap_px: float = 14.0, dx_px: float = 8.0, **kw):
    """Direct labels for several line ends without collisions.

    ``items`` is a list of ``(x, y, text)`` in data coordinates. Labels keep their
    natural heights where possible; overlapping labels are spread apart (clusters
    are centred on the mean of their members' heights) and get a hairline leader
    back to their line end. Call after the axes limits and layout are final.
    """
    ax.figure.canvas.draw()
    scale = ax.figure.dpi / LOGICAL_DPI
    pts = [(x, y, t, ax.transData.transform((x, y))[1] / scale) for x, y, t in items]
    order = sorted(range(len(pts)), key=lambda i: pts[i][3])
    clusters = [[i] for i in order]  # each cluster: member indices (ascending desired y)

    def centre(cl):
        return sum(pts[i][3] for i in cl) / len(cl)

    def bounds(cl):
        c = centre(cl)
        span = (len(cl) - 1) * min_gap_px
        return c - span / 2, c + span / 2

    merged = True
    while merged:
        merged = False
        for k in range(len(clusters) - 1):
            lo_hi = bounds(clusters[k])[1]
            hi_lo = bounds(clusters[k + 1])[0]
            if hi_lo - lo_hi < min_gap_px:
                clusters[k] = clusters[k] + clusters[k + 1]
                del clusters[k + 1]
                merged = True
                break
    placed = {}
    for cl in clusters:
        lo, _ = bounds(cl)
        for j, i in enumerate(cl):
            placed[i] = lo + j * min_gap_px
    out = []
    for i, (x, y, text, ypx) in enumerate(pts):
        dy = placed[i] - ypx
        props = dict(color=theme.ink2, fontsize=SMALL_PT, va="center", ha="left")
        props.update(kw)
        arrow = None
        if abs(dy) > 2.5:
            arrow = dict(arrowstyle="-", color=theme.axis, lw=HAIR_PT, shrinkA=0, shrinkB=2)
        out.append(ax.annotate(text, (x, y), xytext=(dx_px * 72 / LOGICAL_DPI, dy * 72 / LOGICAL_DPI),
                               textcoords="offset points", annotation_clip=False, arrowprops=arrow, **props))
    return out


def dot(ax, x, y, color: str, theme: Theme, size_px: float = 8.4, zorder: float = 3, marker: str = "o", filled: bool = True, **kw):
    """Marker(s) with a 2 px surface ring so they stay legible over lines."""
    size_pt = size_px * 72 / LOGICAL_DPI
    kw.setdefault("clip_on", False)
    return ax.plot(x, y, linestyle="none", marker=marker, markersize=size_pt,
                   markerfacecolor=color if filled else theme.surface, markeredgecolor=theme.surface if filled else color,
                   markeredgewidth=RING_PT if filled else 1.6 * 72 / LOGICAL_DPI, zorder=zorder, **kw)


class RoundedBar(matplotlib.patches.Patch):
    """A thin bar from ``base`` to ``value`` with a 4 px rounded data end.

    Geometry is computed in display space at draw time (so it survives any
    layout change, log axes, and the 2x export): thickness and corner radius
    are in logical pixels, the baseline end stays square.
    """

    def __init__(self, pos: float, value: float, base: float, thickness_px: float, radius_px: float,
                 horizontal: bool, **kw):
        super().__init__(**kw)
        self._geom = (pos, value, base, thickness_px, radius_px, horizontal)

    def get_transform(self):
        return matplotlib.transforms.IdentityTransform()

    def get_path(self):
        from matplotlib.path import Path as MPath

        pos, value, base, thickness_px, radius_px, horizontal = self._geom
        scale = self.figure.dpi / LOGICAL_DPI if self.figure is not None else 1.0
        trans = self.axes.transData
        half = thickness_px * scale / 2
        if horizontal:
            (b, c) = trans.transform((base, pos))
            (v, _) = trans.transform((value, pos))
        else:
            (c, b) = trans.transform((pos, base))
            (_, v) = trans.transform((pos, value))
        sgn = 1.0 if v >= b else -1.0
        r = min(radius_px * scale, half, abs(v - b))
        # (along, across) outline: baseline square, rounded corners at the data end
        pts = [(b, c - half), (v - sgn * r, c - half), (v, c - half), (v, c - half + r),
               (v, c + half - r), (v, c + half), (v - sgn * r, c + half), (b, c + half), (b, c - half)]
        codes = [MPath.MOVETO, MPath.LINETO, MPath.CURVE3, MPath.CURVE3, MPath.LINETO, MPath.CURVE3,
                 MPath.CURVE3, MPath.LINETO, MPath.CLOSEPOLY]
        verts = pts if horizontal else [(a, bb) for (bb, a) in pts]
        return MPath(verts, codes)


def rounded_bars(ax, positions, values, color: str, theme: Theme, thickness_px: float = 18.0,
                 horizontal: bool = True, base: float = 0.0, radius_px: float = 4.0, zorder: float = 2):
    """Thin bars (<= 24 px) growing from one baseline with a 4 px rounded data end.

    On a log value axis pass a positive ``base``. Non-finite / non-positive-on-log
    values are skipped (the caption or table carries them). Axis limits are not
    updated: set them explicitly.
    """
    bars = []
    for pos, val in zip(positions, values):
        if val is None or not math.isfinite(val):
            continue
        bar = RoundedBar(pos, val, base, thickness_px, radius_px, horizontal, facecolor=color,
                         edgecolor="none", linewidth=0, zorder=zorder)
        ax.add_patch(bar)
        bars.append(bar)
    return bars


# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------


def _optimize_png(buf: bytes, meta: dict[str, str]) -> bytes:
    """Re-encode a matplotlib PNG losslessly with maximum zlib effort and text metadata.

    (Palette quantization was tried and rejected: the fast octree quantizer
    collapses a smooth field ramp to ~40 colours, i.e. visible banding.)
    """
    img = Image.open(io.BytesIO(buf))
    img.load()
    info = PngImagePlugin.PngInfo()
    for k, v in meta.items():
        info.add_text(k, v)
    out = io.BytesIO()
    img.save(out, format="PNG", optimize=True, pnginfo=info)
    return out.getvalue()


def save_figure(fig, path: Path, meta: dict[str, str] | None = None, fmt: str = "png") -> Path:
    """Write ``fig`` to ``path`` (2x PNG, palette-optimized; or SVG) and close it."""
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    meta = dict(meta or {})
    if fmt == "svg":
        fig.savefig(path, format="svg", bbox_inches="tight", pad_inches=0.08,
                    metadata={"Title": meta.get("Title", path.stem), "Date": None})
    else:
        buf = io.BytesIO()
        fig.savefig(buf, format="png", dpi=LOGICAL_DPI * EXPORT_SCALE, bbox_inches="tight", pad_inches=0.08)
        path.write_bytes(_optimize_png(buf.getvalue(), meta))
    plt.close(fig)
    return path


def render_variants(draw: Callable[[object, Theme], object], data: object, out_stem: Path,
                    meta: dict[str, str] | None = None, fmt: str = "png") -> list[Path]:
    """Draw ``data`` once per theme and write ``<stem>-light`` / ``<stem>-dark``."""
    paths = []
    for theme in THEMES:
        with use(theme):
            fig = draw(data, theme)
            paths.append(save_figure(fig, Path(f"{out_stem}-{theme.name}.{fmt}"), meta, fmt))
    return paths
