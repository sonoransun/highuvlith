#!/usr/bin/env python3
"""Generate the hand-authored SVG schematics for the "Process Nodes" site section.

Every figure is written twice -- ``<name>-light.svg`` and ``<name>-dark.svg`` --
into ``docs/assets/images/nodes/``. The pages show the right one with MkDocs
Material's ``#only-light`` / ``#only-dark`` URL suffixes.

These are *schematics*: geometry is illustrative, not to scale, and carries no
measured data. Numbers that matter (pitches, years) live in the page text and
tables, next to their sources.

Run from anywhere (standard library only):

    python docs/figures/nodes/make_node_schematics.py
"""

from __future__ import annotations

import random
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "assets" / "images" / "nodes"

FONT = "system-ui, -apple-system, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif"

# Chrome / ink tokens and material colours, per theme. The categorical hues
# follow the site's data-viz reference palette (blue, orange, aqua, yellow,
# magenta, green, violet, red); large fills use tints so the data-free
# schematics stay quiet in both themes.
THEMES = {
    "light": {
        "bg": "#ffffff",
        "ink": "#0b0b0b",
        "ink2": "#52514e",
        "muted": "#898781",
        "hair": "#e1e0d9",
        "base": "#c3c2b7",
        "blue": "#2a78d6",
        "orange": "#eb6834",
        "aqua": "#1baf7a",
        "yellow": "#eda100",
        "magenta": "#e87ba4",
        "green": "#008300",
        "violet": "#4a3aa7",
        "red": "#e34948",
        "gray": "#898781",
    },
    "dark": {
        "bg": "#1e2129",
        "ink": "#ffffff",
        "ink2": "#c3c2b7",
        "muted": "#9a988f",
        "hair": "#34363d",
        "base": "#4a4c52",
        "blue": "#3987e5",
        "orange": "#d95926",
        "aqua": "#199e70",
        "yellow": "#c98500",
        "magenta": "#d55181",
        "green": "#0ca30c",
        "violet": "#9085e9",
        "red": "#e66767",
        "gray": "#8a8880",
    },
}

# material -> (hue, tint fraction for the fill). Tint = share of the hue mixed
# into the page background.
MATERIALS = {
    "si": ("blue", 0.30),
    "si2": ("violet", 0.30),  # second channel material (pFET sheets in CFET)
    "gate": ("orange", 0.45),
    "hk": ("magenta", 1.00),
    "ox": ("gray", 0.18),
    "spacer": ("aqua", 0.55),
    "spacer2": ("yellow", 0.55),
    "mandrel": ("violet", 0.40),
    "hm": ("gray", 0.40),
    "metal": ("yellow", 0.50),
    "power": ("red", 0.45),
    "maskA": ("blue", 0.55),
    "maskB": ("orange", 0.55),
    "resist": ("red", 0.35),
}


def _hex(c: str) -> tuple[int, int, int]:
    c = c.lstrip("#")
    return int(c[0:2], 16), int(c[2:4], 16), int(c[4:6], 16)


def mix(color: str, bg: str, frac: float) -> str:
    """Blend ``frac`` of ``color`` into ``bg`` (both #rrggbb)."""
    a, b = _hex(color), _hex(bg)
    return "#%02x%02x%02x" % tuple(
        round(frac * x + (1 - frac) * y) for x, y in zip(a, b)
    )


class Svg:
    """Tiny SVG builder bound to one theme."""

    def __init__(self, width: int, height: int, theme: str, title: str, desc: str):
        self.w, self.h, self.t = width, height, THEMES[theme]
        self.theme = theme
        self.parts: list[str] = []
        self.title, self.desc = title, desc

    # -- colours -------------------------------------------------------
    def c(self, key: str) -> str:
        return self.t[key]

    def fill(self, mat: str) -> str:
        hue, frac = MATERIALS[mat]
        return mix(self.t[hue], self.t["bg"], frac)

    def stroke(self, mat: str) -> str:
        hue, _ = MATERIALS[mat]
        return self.t[hue]

    # -- primitives ----------------------------------------------------
    def rect(
        self,
        x,
        y,
        w,
        h,
        mat=None,
        *,
        fill=None,
        stroke=None,
        sw=1.2,
        rx=0,
        dash=None,
        opacity=None,
    ):
        f = fill if fill is not None else (self.fill(mat) if mat else "none")
        s = stroke if stroke is not None else (self.stroke(mat) if mat else "none")
        extra = f' stroke-dasharray="{dash}"' if dash else ""
        extra += f' opacity="{opacity}"' if opacity is not None else ""
        self.parts.append(
            f'<rect x="{x:.1f}" y="{y:.1f}" width="{w:.1f}" height="{h:.1f}" rx="{rx}" '
            f'fill="{f}" stroke="{s}" stroke-width="{sw}"{extra}/>'
        )

    def path(
        self, d, mat=None, *, fill=None, stroke=None, sw=1.2, dash=None, join="round"
    ):
        f = fill if fill is not None else (self.fill(mat) if mat else "none")
        s = stroke if stroke is not None else (self.stroke(mat) if mat else "none")
        extra = f' stroke-dasharray="{dash}"' if dash else ""
        self.parts.append(
            f'<path d="{d}" fill="{f}" stroke="{s}" stroke-width="{sw}" stroke-linejoin="{join}" '
            f'stroke-linecap="round"{extra}/>'
        )

    def line(self, x1, y1, x2, y2, color="muted", sw=1.0, dash=None, arrow=None):
        extra = f' stroke-dasharray="{dash}"' if dash else ""
        if arrow in ("end", "both"):
            extra += f' marker-end="url(#arr-{color})"'
        if arrow in ("start", "both"):
            extra += f' marker-start="url(#arr-{color})"'
        self.parts.append(
            f'<line x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" stroke="{self.c(color)}" '
            f'stroke-width="{sw}" stroke-linecap="round"{extra}/>'
        )

    def text(
        self, x, y, s, size=14, anchor="middle", color="ink", weight=400, italic=False
    ):
        style = ' font-style="italic"' if italic else ""
        s = s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
        self.parts.append(
            f'<text x="{x:.1f}" y="{y:.1f}" font-size="{size}" text-anchor="{anchor}" '
            f'fill="{self.c(color)}" font-weight="{weight}"{style}>{s}</text>'
        )

    def dim_h(self, x1, x2, y, label, color="ink2", size=13, above=True, ticks=True):
        """Horizontal dimension: arrowed line between x1 and x2 with a label."""
        self.line(x1, y, x2, y, color, 1.1, arrow="both")
        if ticks:
            self.line(x1, y - 6, x1, y + 6, color, 1.0)
            self.line(x2, y - 6, x2, y + 6, color, 1.0)
        ty = y - 8 if above else y + size + 4
        self.text((x1 + x2) / 2, ty, label, size=size, color=color)

    def dim_v(self, x, y1, y2, label, color="ink2", size=13, side="left"):
        self.line(x, y1, x, y2, color, 1.1, arrow="both")
        self.line(x - 6, y1, x + 6, y1, color, 1.0)
        self.line(x - 6, y2, x + 6, y2, color, 1.0)
        if side == "left":
            self.text(
                x - 10,
                (y1 + y2) / 2 + size / 3,
                label,
                size=size,
                anchor="end",
                color=color,
            )
        else:
            self.text(
                x + 10,
                (y1 + y2) / 2 + size / 3,
                label,
                size=size,
                anchor="start",
                color=color,
            )

    def swatch_legend(self, x, y, items, size=13, gap=26):
        """items: list of (material_or_None, label, kind) with kind in {'rect','line','dash'}."""
        cx = x
        for mat, label, kind in items:
            if kind == "rect":
                self.rect(cx, y - 11, 16, 12, mat, rx=2)
            elif kind == "outline":
                self.rect(
                    cx,
                    y - 11,
                    16,
                    12,
                    fill="none",
                    stroke=self.stroke(mat),
                    dash="3 2",
                    rx=2,
                )
            elif kind == "line":
                self.line(cx, y - 5, cx + 16, y - 5, mat, 2.2)
            self.text(cx + 22, y, label, size=size, anchor="start", color="ink2")
            cx += 22 + 7.2 * len(label) + gap

    # -- output --------------------------------------------------------
    def render(self) -> str:
        markers = []
        for key in (
            "muted",
            "ink2",
            "ink",
            "red",
            "blue",
            "orange",
            "aqua",
            "violet",
            "green",
        ):
            col = self.c(key)
            markers.append(
                f'<marker id="arr-{key}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" '
                f'markerHeight="7" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" fill="{col}"/></marker>'
            )
        head = (
            f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {self.w} {self.h}" '
            f'width="{self.w}" height="{self.h}" role="img" aria-labelledby="t d" '
            f'font-family="{FONT}">\n'
            f'<title id="t">{self.title}</title>\n<desc id="d">{self.desc}</desc>\n'
            f'<defs>{"".join(markers)}</defs>\n'
        )
        return head + "\n".join(self.parts) + "\n</svg>\n"


# ---------------------------------------------------------------------------
# 1. Transistor architectures: planar -> FinFET -> GAA nanosheet -> CFET
# ---------------------------------------------------------------------------
def fig_transistor_evolution(theme: str) -> Svg:
    s = Svg(
        900,
        400,
        theme,
        "Transistor architecture evolution",
        "Four schematic cross-sections cut across the channel: a planar MOSFET with the gate on "
        "top only; a FinFET whose gate wraps three sides of two silicon fins; a gate-all-around "
        "nanosheet transistor with three stacked sheets fully surrounded by the gate; and a CFET "
        "with nFET sheets stacked under pFET sheets inside one gate.",
    )
    pw, gap, x0, top = 200, 20, 20, 44
    names = ["Planar MOSFET", "FinFET", "GAA nanosheet", "CFET (stacked n/p)"]
    notes = [
        "gate on 1 side",
        "gate on 3 sides",
        "gate on all 4 sides",
        "nFET under pFET, one gate",
    ]
    for i in range(4):
        px = x0 + i * (pw + gap)
        s.text(px + pw / 2, 26, names[i], size=15, weight=600)
        s.rect(px, top, pw, 260, fill="none", stroke=s.c("hair"), sw=1, rx=6)
        if i == 0:  # planar
            s.rect(px + 12, top + 150, pw - 24, 96, "si")
            s.rect(px + 12, top + 150, 34, 56, "ox")
            s.rect(px + pw - 46, top + 150, 34, 56, "ox")
            s.rect(px + 50, top + 80, 100, 64, "gate")
            s.rect(px + 50, top + 143, 100, 7, "hk", sw=0.6)
            s.rect(
                px + 50,
                top + 150,
                100,
                10,
                fill=mix(s.c("blue"), s.c("bg"), 0.6),
                stroke="none",
            )
            s.text(px + 100, top + 117, "gate", size=13, color="ink")
            s.text(px + 100, top + 180, "channel", size=12, color="ink2")
            s.text(px + 100, top + 228, "Si substrate", size=12, color="ink2")
        elif i == 1:  # FinFET
            s.rect(px + 12, top + 200, pw - 24, 46, "si")
            s.rect(px + 30, top + 60, 140, 110, "gate")
            for fx in (px + 64, px + 114):
                s.rect(fx - 5, top + 85, 32, 90, "hk", sw=0.6)
                s.rect(fx, top + 90, 22, 110, "si")
            s.rect(px + 12, top + 170, pw - 24, 30, "ox")
            for fx in (px + 64, px + 114):
                s.rect(fx, top + 170, 22, 30, "si")
            s.text(px + 100, top + 78, "gate", size=13)
            s.text(px + 38, top + 190, "STI", size=11, color="ink2")
            s.text(px + 75, top + 142, "fin", size=11, color="ink2")
        elif i == 2:  # GAA nanosheet
            s.rect(px + 12, top + 200, pw - 24, 46, "si")
            s.rect(px + 12, top + 180, pw - 24, 20, "ox")
            s.rect(px + 30, top + 48, 140, 132, "gate")
            for k, sy in enumerate((top + 72, top + 108, top + 144)):
                s.rect(px + 50, sy - 5, 100, 22, "hk", sw=0.6)
                s.rect(px + 55, sy, 90, 12, "si")
            s.text(px + 100, top + 64, "gate", size=13)
            s.text(px + 100, top + 228, "Si substrate", size=12, color="ink2")
        else:  # CFET
            s.rect(px + 12, top + 214, pw - 24, 32, "si")
            s.rect(px + 12, top + 198, pw - 24, 16, "ox")
            s.rect(px + 30, top + 30, 132, 168, "gate")
            ys_p = (top + 50, top + 80)
            ys_n = (top + 140, top + 170)
            for sy in ys_p:
                s.rect(px + 45, sy - 5, 100, 22, "hk", sw=0.6)
                s.rect(px + 50, sy, 90, 12, "si2")
            s.rect(px + 38, top + 112, 116, 10, "ox", sw=0.8)
            for sy in ys_n:
                s.rect(px + 45, sy - 5, 100, 22, "hk", sw=0.6)
                s.rect(px + 50, sy, 90, 12, "si")
            s.text(px + 181, top + 76, "p", size=14, weight=600)
            s.text(px + 181, top + 166, "n", size=14, weight=600)
            s.text(px + 96, top + 44, "gate", size=12)
        s.text(px + pw / 2, top + 284, notes[i], size=13, color="ink2")
    # progression arrows between panels
    for i in range(3):
        ax = x0 + (i + 1) * (pw + gap) - gap + 3
        s.line(ax, top + 130, ax + gap - 6, top + 130, "muted", 1.6, arrow="end")
    s.swatch_legend(
        70,
        382,
        [
            ("si", "Si channel / substrate", "rect"),
            ("si2", "pFET channel", "rect"),
            ("gate", "metal gate", "rect"),
            ("hk", "high-k dielectric", "rect"),
            ("ox", "oxide", "rect"),
        ],
    )
    return s


# ---------------------------------------------------------------------------
# 2. Standard cell: CPP x MMP and track height (top view)
# ---------------------------------------------------------------------------
def fig_standard_cell(theme: str) -> Svg:
    s = Svg(
        860,
        445,
        theme,
        "Standard-cell anatomy: CPP, MMP and track height",
        "Top view of a schematic logic standard cell. Vertical gate lines repeat at the contacted "
        "poly pitch (CPP); horizontal routing tracks repeat at the minimum metal pitch (MMP). "
        "Power rails run along the top and bottom edges. The cell height equals the number of "
        "tracks times the metal pitch; the cell width is a whole number of CPPs.",
    )
    cpp, mmp, ntr = 96, 44, 6
    x0, y0 = 170, 60
    ncpp = 4
    cw, ch = ncpp * cpp, ntr * mmp
    # cell boundary
    s.rect(x0, y0, cw, ch, fill="none", stroke=s.c("muted"), sw=1.4, dash="6 4")
    # fins (active regions): p on top half, n on bottom half
    for k, fy in enumerate((y0 + 70, y0 + 92, y0 + ch - 104, y0 + ch - 82)):
        s.rect(x0 + 18, fy, cw - 36, 9, "si", sw=0.8)
    s.text(x0 + cw + 14, y0 + 90, "pFET fins", size=12, anchor="start", color="ink2")
    s.text(
        x0 + cw + 14, y0 + ch - 80, "nFET fins", size=12, anchor="start", color="ink2"
    )
    # gates at CPP (cell-edge gates are shared dummies)
    for g in range(ncpp + 1):
        gx = x0 + g * cpp
        dummy = g in (0, ncpp)
        s.rect(
            gx - 8,
            y0 + 16,
            16,
            ch - 32,
            "gate",
            sw=1.0,
            dash="4 3" if dummy else None,
            opacity=0.55 if dummy else None,
        )
    # routing tracks at MMP
    for t in range(1, ntr):
        ty = y0 + t * mmp
        s.line(x0 - 20, ty, x0 + cw + 6, ty, "hair", 1.0)
    # a few M0/M1 wire segments on tracks
    for t, a, b in (
        (1, 0.3, 2.6),
        (2, 1.2, 3.4),
        (3, 0.5, 1.6),
        (3, 2.4, 3.7),
        (4, 0.8, 3.1),
        (5, 1.6, 3.6),
    ):
        ty = y0 + t * mmp
        s.rect(
            x0 + a * cpp, ty - 9, (b - a) * cpp, 18, "metal", sw=1.0, rx=3, opacity=0.9
        )
    # power rails
    s.rect(x0 - 20, y0 - 12, cw + 40, 24, "power", rx=3)
    s.rect(x0 - 20, y0 + ch - 12, cw + 40, 24, "power", rx=3)
    s.text(x0 + cw + 30, y0 + 5, "VDD rail", size=12, anchor="start", color="ink2")
    s.text(x0 + cw + 30, y0 + ch + 5, "VSS rail", size=12, anchor="start", color="ink2")
    # dimensions
    s.dim_h(x0 + cpp, x0 + 2 * cpp, y0 - 26, "CPP (gate pitch)")
    s.dim_v(
        x0 + cw + 110, y0 + 2 * mmp, y0 + 3 * mmp, "MMP (metal pitch)", side="right"
    )
    s.dim_v(x0 - 44, y0, y0 + ch, "cell height", side="left")
    s.text(
        x0 - 54,
        y0 + ch / 2 + 22,
        f"= {ntr} tracks × MMP",
        size=12,
        anchor="end",
        color="ink2",
    )
    s.dim_h(x0, x0 + cw, y0 + ch + 38, f"cell width = {ncpp} × CPP", above=False)
    s.swatch_legend(
        120,
        432,
        [
            ("gate", "gate (poly/metal)", "rect"),
            ("si", "fins (active)", "rect"),
            ("metal", "M0/M1 wires", "rect"),
            ("power", "power rails", "rect"),
        ],
    )
    return s


# ---------------------------------------------------------------------------
# helpers for process-flow rows
# ---------------------------------------------------------------------------
def _row_base(s: Svg, x, y, w, label, step, sub=None, hm=True):
    s.text(20, y + 26, step, size=15, anchor="start", weight=600)
    s.text(46, y + 26, label, size=14, anchor="start")
    if sub:
        s.text(46, y + 46, sub, size=12, anchor="start", color="ink2")
    # target layer (hard mask / film to be patterned) + substrate
    if hm:
        s.rect(x, y + 70, w, 16, "hm", sw=1.0)
    s.rect(x, y + 86, w, 16, "si", sw=1.0)


def fig_sadp(theme: str) -> Svg:
    s = Svg(
        900,
        640,
        theme,
        "Self-aligned double patterning (SADP)",
        "Five-step SADP flow in cross-section. 1: mandrel lines printed at pitch P. 2: a conformal "
        "spacer film is deposited. 3: an anisotropic etch leaves spacers only on the mandrel "
        "sidewalls. 4: the mandrels are removed, leaving two spacer lines per mandrel at pitch P/2. "
        "5: the spacer pattern is etched into the target layer.",
    )
    X, W = 330, 540
    P, ts, h = 180.0, 40.0, 44.0
    M = (P - 2 * ts) / 2  # uniform-pitch condition: mandrel width = gap width
    xs = [X + 45 + k * P for k in range(3)]  # mandrel left edges
    rows = [
        ("1", "Litho + etch: mandrels", "one mask, printed at pitch P"),
        ("2", "Conformal spacer deposition", "film thickness t on every surface"),
        ("3", "Anisotropic spacer etch-back", "spacers survive on the sidewalls"),
        ("4", "Mandrel pull", "two spacer lines per mandrel"),
        ("5", "Transfer into the target layer", "lines at pitch P/2, no second mask"),
    ]
    for r, (step, label, sub) in enumerate(rows):
        y = 18 + r * 118
        _row_base(s, X, y, W, label, step, sub, hm=(r != 4))
        base = y + 70
        if r == 1:  # conformal film over mandrels and floor
            s.rect(X, base - ts * 0.5, W, ts * 0.5, "spacer", sw=0.8)
            for mx in xs:
                s.rect(
                    mx - ts,
                    base - h - ts * 0.5,
                    M + 2 * ts,
                    h + ts * 0.5,
                    "spacer",
                    sw=0.8,
                    rx=10,
                )
        if r in (0, 1, 2):
            for mx in xs:
                s.rect(mx, base - h, M, h, "mandrel", sw=1.0)
        if r in (2, 3):
            for mx in xs:
                s.rect(mx - ts, base - h, ts, h, "spacer", sw=0.9, rx=3)
                s.rect(mx + M, base - h, ts, h, "spacer", sw=0.9, rx=3)
        if r == 4:  # etched target: the hard-mask survives only under the spacers
            for mx in xs:
                for lx in (mx - ts, mx + M):
                    s.rect(lx, y + 70, ts, 16, "hm", sw=1.0)
                    s.rect(
                        lx,
                        base - h * 0.45,
                        ts,
                        h * 0.45,
                        "spacer",
                        sw=0.8,
                        opacity=0.35,
                    )
        if r == 0:
            s.dim_h(xs[0], xs[1], base - h - 12, "P")
        if r == 1:
            s.dim_h(xs[2] + M, xs[2] + M + ts, base - h - ts * 0.5 - 12, "t")
        if r == 3:
            s.dim_h(xs[0] - ts, xs[0] + M, base - h - 12, "P/2")
            s.text(xs[0] + M / 2, base - 8, "core", size=11, color="ink2")
            s.text(
                (xs[0] + M + ts + xs[1] - ts) / 2,
                base - 8,
                "gap",
                size=11,
                color="ink2",
            )
    s.swatch_legend(
        150,
        630,
        [
            ("mandrel", "mandrel", "rect"),
            ("spacer", "spacer", "rect"),
            ("hm", "target / hard mask", "rect"),
            ("si", "substrate", "rect"),
        ],
    )
    return s


def fig_saqp(theme: str) -> Svg:
    s = Svg(
        900,
        530,
        theme,
        "Self-aligned quadruple patterning (SAQP)",
        "SAQP repeats the spacer trick twice. 1: mandrels at pitch P. 2: first spacers remain after "
        "the mandrel pull, at pitch P/2. 3: second spacers are formed on the sidewalls of the first "
        "spacers. 4: the first spacers are removed, leaving lines at pitch P/4.",
    )
    X, W = 330, 540
    P = 240.0
    t2 = P / 8  # final line width (equal lines and spaces at P/4)
    t1 = P / 8
    M1 = (P - 2 * t1) / 2
    xs = [X + 70 + k * P for k in range(2)]
    rows = [
        ("1", "Mandrels (mask 1 only)", "printed at pitch P"),
        ("2", "Spacer-1 after mandrel pull", "pitch P/2; they act as the new mandrels"),
        ("3", "Spacer-2 on spacer-1 sidewalls", "deposit + etch-back again"),
        ("4", "Remove spacer-1", "lines at pitch P/4 (then cut masks)"),
    ]
    for r, (step, label, sub) in enumerate(rows):
        y = 18 + r * 118
        _row_base(s, X, y, W, label, step, sub)
        base = y + 70
        h = 44
        sp1 = []
        for mx in xs:
            sp1 += [mx - t1, mx + M1]
        if r == 0:
            for mx in xs:
                s.rect(mx, base - h, M1, h, "mandrel")
            s.dim_h(xs[0], xs[1], base - h - 12, "P")
        if r in (1, 2):
            for lx in sp1:
                s.rect(lx, base - h, t1, h, "spacer", rx=2)
        if r in (2, 3):
            for lx in sp1:
                s.rect(lx - t2, base - h, t2, h, "spacer2", rx=2)
                s.rect(lx + t1, base - h, t2, h, "spacer2", rx=2)
        if r == 1:
            s.dim_h(sp1[0], sp1[1], base - h - 12, "P/2")
        if r == 3:
            s.dim_h(sp1[0] - t2, sp1[0] + t1, base - h - 12, "P/4")
    s.swatch_legend(
        150,
        520,
        [
            ("mandrel", "mandrel", "rect"),
            ("spacer", "spacer 1", "rect"),
            ("spacer2", "spacer 2", "rect"),
            ("hm", "target layer", "rect"),
        ],
    )
    return s


def fig_pitch_walk(theme: str) -> Svg:
    s = Svg(
        900,
        440,
        theme,
        "Pitch walk in SADP",
        "Three rows of SADP lines. Nominal: core spaces (where mandrels were) equal gap spaces, so "
        "the pitch is uniform. Mandrel CD error: core spaces widen by delta and gap spaces shrink "
        "by delta, so line pairs alternate and the pitch walks. Spacer-thickness error: lines "
        "widen and only the gap spaces shrink, by twice the error.",
    )
    X = 280
    P, t = 190.0, 50.0
    M = (P - 2 * t) / 2
    rows = [
        ("Nominal", "core = gap = s, uniform pitch", 0.0, 0.0),
        ("Mandrel CD + Δ", "core = s + Δ, gap = s − Δ", 16.0, 0.0),
        ("Spacer t + δ", "core = s, gap = s − 2δ", 0.0, 10.0),
    ]
    for r, (label, sub, dM, dt) in enumerate(rows):
        y = 20 + r * 134
        s.text(20, y + 40, label, size=15, anchor="start", weight=600)
        s.text(20, y + 62, sub, size=13, anchor="start", color="ink2")
        base = y + 96
        s.rect(X - 10, base, 3 * P + 20, 14, "hm", sw=0.8)
        tt = t + dt
        for k in range(3):
            mx = X + 50 + k * P - dM / 2
            MM = M + dM
            s.rect(
                mx,
                base - 58,
                MM,
                58,
                fill="none",
                stroke=s.stroke("mandrel"),
                sw=1.0,
                dash="4 3",
            )
            s.rect(mx - tt, base - 58, tt, 58, "spacer", rx=2)
            s.rect(mx + MM, base - 58, tt, 58, "spacer", rx=2)
            if k == 0:
                s.dim_h(mx, mx + MM, base - 70, "core")
                nx = X + 50 + P - dM / 2 - tt
                s.dim_h(mx + MM + tt, nx, base - 70, "gap")
    s.swatch_legend(
        300,
        430,
        [
            ("spacer", "spacer line", "rect"),
            ("mandrel", "former mandrel", "outline"),
            ("hm", "target layer", "rect"),
        ],
    )
    return s


def fig_lele_overlay(theme: str) -> Svg:
    s = Svg(
        900,
        400,
        theme,
        "LELE double patterning and overlay",
        "Litho-etch-litho-etch splits a dense line array between two masks, A and B, each at twice "
        "the target pitch. With perfect overlay the combined pattern has equal spaces. If mask B "
        "lands shifted by an overlay error delta, the spaces alternate between s minus delta and "
        "s plus delta: overlay error becomes space-CD error one-to-one.",
    )
    X = 300
    p, w = 100.0, 44.0
    rows = [
        ("Perfect overlay", "spaces s, s, s …", 0.0),
        ("Overlay error δ on mask B", "spaces s − δ, s + δ …", 18.0),
    ]
    for r, (label, sub, d) in enumerate(rows):
        y = 30 + r * 160
        s.text(20, y + 44, label, size=15, anchor="start", weight=600)
        s.text(20, y + 66, sub, size=13, anchor="start", color="ink2")
        base = y + 110
        s.rect(X - 10, base, 6 * p, 14, "hm", sw=0.8)
        for k in range(6):
            isB = k % 2 == 1
            lx = X + k * p + (d if isB else 0.0)
            s.rect(lx, base - 70, w, 70, "maskB" if isB else "maskA", rx=2)
            s.text(lx + w / 2, base - 30, "B" if isB else "A", size=13, weight=600)
        s.dim_h(X + w, X + p + d, base - 82, "s − δ" if d else "s")
        s.dim_h(X + p + d + w, X + 2 * p, base - 82, "s + δ" if d else "s")
        if d:
            s.line(X + p, base + 24, X + p + d, base + 24, "red", 1.4, arrow="end")
            s.text(X + p + d / 2, base + 42, "δ", size=13, color="ink")
    s.swatch_legend(
        300,
        388,
        [
            ("maskA", "exposure A (mask 1)", "rect"),
            ("maskB", "exposure B (mask 2)", "rect"),
        ],
    )
    return s


# ---------------------------------------------------------------------------
# Backside power delivery
# ---------------------------------------------------------------------------
def fig_backside_power(theme: str) -> Svg:
    s = Svg(
        900,
        470,
        theme,
        "Front-side versus backside power delivery",
        "Left: conventional front-side power delivery, where power and signal wires share one "
        "metal stack above the transistors and supply current travels down through every layer. "
        "Right: backside power delivery, where the wafer is thinned and a power network on the back "
        "connects to the transistors through nano through-silicon vias, leaving the front metal "
        "stack for signals.",
    )
    for panel in range(2):
        px = 30 + panel * 440
        s.text(
            px + 200,
            26,
            ["Front-side power (FSPDN)", "Backside power (BSPDN)"][panel],
            size=15,
            weight=600,
        )
        if panel == 0:
            s.rect(px + 20, 330, 360, 80, "si")
            s.text(px + 200, 376, "silicon substrate", size=12, color="ink2")
            s.rect(px + 20, 306, 360, 24, "gate", sw=1.0)
            s.text(px + 200, 323, "transistors", size=12)
            widths = [8, 8, 10, 10, 12, 14, 18, 22]
            yy = 300
            for k, wth in enumerate(widths):
                yy -= wth + 10
                for j in range(0, 360, 40 + k * 6):
                    power = (j // (40 + k * 6)) % 3 == 0
                    s.rect(
                        px + 22 + j,
                        yy,
                        22 + k * 3,
                        wth,
                        "power" if power else "metal",
                        sw=0.7,
                        rx=1,
                    )
            s.rect(px + 60, 44, 40, 14, "power", rx=7)
            s.rect(px + 300, 44, 40, 14, "power", rx=7)
            s.line(px + 80, 60, px + 80, 300, "red", 2.0, arrow="end")
            s.text(
                px + 110,
                74,
                "supply current crosses",
                size=12,
                anchor="start",
                color="ink2",
            )
            s.text(
                px + 110,
                90,
                "the whole metal stack",
                size=12,
                anchor="start",
                color="ink2",
            )
        else:
            s.rect(px + 20, 44, 360, 40, "ox")
            s.text(px + 200, 70, "bonded carrier wafer", size=12, color="ink2")
            widths = [16, 12, 10, 10, 8, 8]
            yy = 88
            for k, wth in enumerate(widths):
                for j in range(0, 360, 44):
                    s.rect(px + 22 + j, yy, 26, wth, "metal", sw=0.7, rx=1)
                yy += wth + 10
            s.text(px + 200, yy + 4, "front metal: signals only", size=12, color="ink2")
            s.rect(px + 20, 250, 360, 24, "gate", sw=1.0)
            s.text(px + 200, 267, "transistors", size=12)
            s.rect(px + 20, 274, 360, 22, "si", sw=0.8)
            for j in range(40, 360, 80):
                s.rect(px + 20 + j, 274, 8, 36, "power", sw=0.8)
            yy = 312
            for k, wth in enumerate((10, 14, 20)):
                for j in range(0, 360, 60 + 20 * k):
                    s.rect(px + 22 + j, yy, 40 + 10 * k, wth, "power", sw=0.7, rx=1)
                yy += wth + 10
            s.rect(px + 60, 400, 40, 14, "power", rx=7)
            s.rect(px + 300, 400, 40, 14, "power", rx=7)
            s.line(px + 320, 398, px + 320, 282, "red", 2.0, arrow="end")
            s.text(px + 100, 290, "nano-TSVs", size=11, color="ink")
            s.text(
                px + 200,
                432,
                "backside power network (thinned wafer)",
                size=12,
                color="ink2",
            )
    s.swatch_legend(
        230,
        462,
        [
            ("metal", "signal wires", "rect"),
            ("power", "power wires / vias", "rect"),
            ("gate", "device layer", "rect"),
        ],
    )
    return s


# ---------------------------------------------------------------------------
# High-NA anamorphic field
# ---------------------------------------------------------------------------
def fig_anamorphic_field(theme: str) -> Svg:
    s = Svg(
        900,
        430,
        theme,
        "Anamorphic High-NA imaging halves the exposure field",
        "A 6-inch reticle's 104 by 132 mm image area maps to a 26 by 33 mm full field at 4x "
        "reduction in both directions (0.33 NA systems). High-NA optics reduce 4x across the scan "
        "and 8x along the scan, so the same reticle area prints a 26 by 16.5 mm half field; larger "
        "dies must be stitched from two exposures.",
    )
    k = 1.7  # px per mm on the reticle drawing
    rx, ry = 90, 70
    s.text(rx + 52 * k, 40, "reticle image area", size=14, weight=600)
    s.rect(
        rx,
        ry,
        104 * k,
        132 * k,
        fill=mix(s.c("violet"), s.c("bg"), 0.18),
        stroke=s.c("violet"),
        sw=1.4,
        rx=4,
    )
    s.dim_h(rx, rx + 104 * k, ry + 132 * k + 22, "104 mm", above=False)
    s.dim_v(rx - 14, ry, ry + 132 * k, "132 mm", side="left")
    # arrow
    ax = rx + 104 * k + 30
    s.line(ax, ry + 66 * k, ax + 90, ry + 66 * k, "muted", 1.8, arrow="end")
    s.text(ax + 45, ry + 66 * k - 30, "projection", size=12, color="ink2")
    s.text(ax + 45, ry + 66 * k - 14, "optics", size=12, color="ink2")
    # wafer fields
    w = 5.2  # px per mm on the wafer drawing
    fx, fy = ax + 160, ry + 10
    s.text(fx + 13 * w, 40, "0.33 NA: 4× / 4×", size=14, weight=600)
    s.rect(
        fx,
        fy,
        26 * w,
        33 * w,
        fill=mix(s.c("blue"), s.c("bg"), 0.18),
        stroke=s.c("blue"),
        sw=1.4,
        rx=3,
    )
    s.text(fx + 13 * w, fy + 16.5 * w + 5, "full field", size=13)
    s.dim_h(fx, fx + 26 * w, fy + 33 * w + 22, "26 mm", above=False)
    s.dim_v(fx - 12, fy, fy + 33 * w, "33 mm", side="left")
    gx = fx + 26 * w + 70
    s.text(gx + 13 * w, 40, "0.55 NA: 4× / 8×", size=14, weight=600)
    s.rect(
        gx,
        fy,
        26 * w,
        16.5 * w,
        fill=mix(s.c("orange"), s.c("bg"), 0.25),
        stroke=s.c("orange"),
        sw=1.4,
        rx=3,
    )
    s.rect(
        gx,
        fy + 16.5 * w,
        26 * w,
        16.5 * w,
        fill="none",
        stroke=s.c("orange"),
        sw=1.2,
        rx=3,
        dash="5 4",
    )
    s.text(gx + 13 * w, fy + 8.25 * w + 5, "half field", size=13)
    s.text(gx + 13 * w, fy + 24.75 * w - 4, "2nd exposure", size=12, color="ink2")
    s.text(gx + 13 * w, fy + 24.75 * w + 12, "(stitched)", size=12, color="ink2")
    s.dim_h(gx, gx + 26 * w, fy + 33 * w + 22, "26 mm", above=False)
    s.dim_v(gx + 26 * w + 12, fy, fy + 16.5 * w, "16.5 mm", side="right")
    s.text(
        450,
        415,
        "x = across the scan · y = scan direction (vertical here)",
        size=12,
        color="ink2",
    )
    return s


# ---------------------------------------------------------------------------
# Pupil / diffraction-order picture behind k1 = 0.25
# ---------------------------------------------------------------------------
def fig_pupil_orders(theme: str) -> Svg:
    s = Svg(
        900,
        420,
        theme,
        "Why single-exposure k1 cannot go below 0.25",
        "Three pupil diagrams in normalized coordinates (pupil radius = NA). A line grating of pitch "
        "p splits light into diffraction orders spaced lambda over p apart. An image forms only if "
        "the zeroth order and at least one first order pass the pupil together. Left: on-axis "
        "illumination, orders fall outside and nothing is imaged. Middle: tilted (dipole) "
        "illumination puts the zeroth and minus-first orders symmetrically inside the pupil. "
        "Right: the limit, where the two orders sit at opposite pupil edges, spaced 2 NA, giving "
        "p = lambda over 2 NA, that is k1 = 0.25.",
    )
    R = 92
    panels = [
        (
            "On-axis source",
            "orders outside → no image",
            [(0.0, "0"), (1.35, "+1"), (-1.35, "−1")],
            [0.0],
        ),
        (
            "Dipole (off-axis) source",
            "0 and −1 inside → two-beam image",
            [(0.68, "0"), (-0.68, "−1"), (2.03, "+1")],
            [0.68],
        ),
        (
            "The limit: k₁ = 0.25",
            "orders at the rim, spacing 2·NA",
            [(1.0, "0"), (-1.0, "−1"), (3.0, "+1")],
            [1.0],
        ),
    ]
    for i, (title, sub, orders, src) in enumerate(panels):
        cx, cy = 150 + i * 300, 190
        s.text(cx, 36, title, size=15, weight=600)
        s.path(
            f"M{cx - R},{cy} a{R},{R} 0 1,0 {2 * R},0 a{R},{R} 0 1,0 {-2 * R},0",
            fill=mix(s.c("blue"), s.c("bg"), 0.10),
            stroke=s.c("blue"),
            sw=1.6,
        )
        s.line(cx - R - 20, cy, cx + R + 20, cy, "hair", 1.0)
        for u, lab in orders:
            ox = cx + u * R
            inside = abs(u) <= 1.0 + 1e-9
            if ox < cx - R - 60 or ox > cx + R + 60:
                s.text(
                    cx + (R + 30) * (1 if u > 0 else -1),
                    cy + 28,
                    f"{lab} →" if u > 0 else f"← {lab}",
                    size=12,
                    color="muted",
                )
                continue
            col = s.c("orange") if inside else s.c("muted")
            s.path(
                f"M{ox - 7},{cy} a7,7 0 1,0 14,0 a7,7 0 1,0 -14,0",
                fill=col if inside else "none",
                stroke=col,
                sw=1.6,
            )
            if abs(abs(u) - 1.0) < 1e-6:  # order on the rim: label outside the circle
                s.text(
                    ox + 12 * (1 if u > 0 else -1),
                    cy - 14,
                    lab,
                    size=12,
                    color="ink2",
                    anchor="start" if u > 0 else "end",
                )
            else:
                s.text(ox, cy - 20, lab, size=12, color="ink2")
        s.text(cx, cy + R + 36, sub, size=13, color="ink2")
        if i == 1:
            s.text(
                cx,
                cy + R + 54,
                "(one pole shown; its mirror pole works the same way)",
                size=11,
                color="muted",
            )
        if i == 2:
            s.dim_h(cx - R, cx + R, cy + R + 60, "λ/p = 2·NA  ⇒  p = λ/(2·NA)")
    s.swatch_legend(
        290,
        408,
        [
            ("maskB", "order inside the pupil", "rect"),
            ("hm", "blocked order", "outline"),
        ],
    )
    return s


# ---------------------------------------------------------------------------
# EUV stochastic failure modes (illustrative, seeded)
# ---------------------------------------------------------------------------
def fig_stochastic_defects(theme: str) -> Svg:
    s = Svg(
        900,
        390,
        theme,
        "Stochastic failure modes",
        "Illustrative sketch of stochastic printing failures. Left: a contact-hole array with "
        "slightly different hole sizes, one missing contact and two merged (kissing) contacts. "
        "Right: dense lines with rough edges (line-edge roughness), a microbridge joining two "
        "lines and a break in one line.",
    )
    rng = random.Random(7)
    s.text(210, 30, "Contact holes", size=15, weight=600)
    x0, y0, pc = 60, 60, 52
    for i in range(6):
        for j in range(5):
            cx, cy = x0 + i * pc + 26, y0 + j * pc + 26
            r = 13 + rng.uniform(-2.6, 2.6)
            if (i, j) == (2, 1):
                s.path(
                    f"M{cx - 13},{cy} a13,13 0 1,0 26,0 a13,13 0 1,0 -26,0",
                    fill="none",
                    stroke=s.c("red"),
                    sw=1.4,
                    dash="4 3",
                )
                continue
            if (i, j) in ((4, 3), (5, 3)):
                continue
            s.path(
                f"M{cx - r},{cy} a{r},{r} 0 1,0 {2 * r},0 a{r},{r} 0 1,0 {-2 * r},0",
                "si",
                sw=1.0,
            )
    # merged pair
    cx1, cx2, cy = x0 + 4 * pc + 26, x0 + 5 * pc + 26, y0 + 3 * pc + 26
    s.path(
        f"M{cx1},{cy - 15} L{cx2},{cy - 14} A14,14 0 0,1 {cx2},{cy + 14} L{cx1},{cy + 15} A15,15 0 0,1 {cx1},{cy - 15} Z",
        "si",
        sw=1.0,
    )
    s.text(x0 + 2 * pc + 26, y0 + pc + 52, "missing", size=12, color="ink")
    s.text(x0 + 4.5 * pc + 26, y0 + 3 * pc + 58, "merged", size=12, color="ink")
    # lines with LER, a bridge and a break
    s.text(640, 30, "Dense lines", size=15, weight=600)
    lx0, ly0, lp, lw, L = 470, 58, 48, 22, 262
    for k in range(7):
        x = lx0 + k * lp
        pts_l, pts_r = [], []
        for n in range(0, 27):
            yy = ly0 + n * (L / 26)
            pts_l.append((x + rng.gauss(0, 1.6), yy))
            pts_r.append((x + lw + rng.gauss(0, 1.6), yy))
        if k == 5:  # break in the middle
            top_l = [p for p in pts_l if p[1] < ly0 + 118]
            top_r = [p for p in pts_r if p[1] < ly0 + 118]
            bot_l = [p for p in pts_l if p[1] > ly0 + 142]
            bot_r = [p for p in pts_r if p[1] > ly0 + 142]
            for a, b in ((top_l, top_r), (bot_l, bot_r)):
                d = "M" + " L".join(f"{px:.1f},{py:.1f}" for px, py in a)
                d += (
                    " L"
                    + " L".join(f"{px:.1f},{py:.1f}" for px, py in reversed(b))
                    + " Z"
                )
                s.path(d, "si", sw=1.0)
            continue
        d = "M" + " L".join(f"{px:.1f},{py:.1f}" for px, py in pts_l)
        d += " L" + " L".join(f"{px:.1f},{py:.1f}" for px, py in reversed(pts_r)) + " Z"
        s.path(d, "si", sw=1.0)
    # bridge between lines 1 and 2
    bx = lx0 + 1 * lp + lw - 1
    s.path(
        f"M{bx},{ly0 + 196} C{bx + 10},{ly0 + 202} {bx + 16},{ly0 + 202} {bx + 27},{ly0 + 194} "
        f"L{bx + 27},{ly0 + 210} C{bx + 16},{ly0 + 214} {bx + 10},{ly0 + 214} {bx},{ly0 + 212} Z",
        "si",
        sw=1.0,
    )
    s.line(bx + 13, ly0 + 216, bx + 13, ly0 + L + 12, "ink2", 1.0)
    s.text(bx + 13, ly0 + L + 28, "microbridge", size=12)
    s.text(lx0 + 5 * lp + lw / 2, ly0 + 136, "break", size=12)
    s.text(
        lx0 + 5 * lp, ly0 + L + 28, "rough edges everywhere: LER", size=12, color="ink2"
    )
    s.text(
        450,
        378,
        "Schematic, not simulated: sizes and positions are illustrative.",
        size=12,
        color="muted",
    )
    return s


FIGURES = {
    "transistor-evolution": fig_transistor_evolution,
    "standard-cell": fig_standard_cell,
    "sadp-flow": fig_sadp,
    "saqp-flow": fig_saqp,
    "pitch-walk": fig_pitch_walk,
    "lele-overlay": fig_lele_overlay,
    "backside-power": fig_backside_power,
    "anamorphic-field": fig_anamorphic_field,
    "pupil-orders": fig_pupil_orders,
    "stochastic-defects": fig_stochastic_defects,
}


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, fn in FIGURES.items():
        for theme in ("light", "dark"):
            path = OUT / f"{name}-{theme}.svg"
            path.write_text(fn(theme).render(), encoding="utf-8")
            print("wrote", path.relative_to(OUT.parents[2]))


if __name__ == "__main__":
    main()
