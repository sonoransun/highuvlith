#!/usr/bin/env python3
"""Scripted SVG schematics for the "Process Nodes" site section (svgkit, standard library only).

Every figure is written as a light/dark pair into ``docs/assets/images/nodes/`` and
registered as an :class:`registry.AssetSpec` so ``make_all.py --check`` verifies the files,
the pages that embed them and the caption trailers. Geometry is illustrative, not to scale,
and carries no measured data; numbers that matter (pitches, years) live in the page text and
tables next to their sources.

Static schematics (re-rendered through ``tokens``, no content change):
``transistor-evolution`` · ``standard-cell`` · ``sadp-flow`` · ``saqp-flow`` · ``pitch-walk`` ·
``lele-overlay`` · ``backside-power`` · ``anamorphic-field`` · ``pupil-orders`` (the D5 pupil
grammar) · ``stochastic-defects``.

New (DESIGN §5.5): ``sadp-flow-anim`` (N10, a two-layer CSS animation whose designed still is
the five-row ``sadp-flow`` drawing) · ``euv-mask-stack`` (N11) · ``nanosheet-flow`` (N12) ·
``cut-mask`` (N13).

``Svg`` is re-exported from :mod:`svgkit` (the builder was promoted out of this file by V0);
the local :class:`NodeSvg` subclass only adds this section's material names.

    python docs/figures/nodes/make_node_schematics.py            # write every pair
    python docs/figures/nodes/make_node_schematics.py sadp       # names containing "sadp"
    python docs/figures/nodes/make_node_schematics.py --check    # sizes and hygiene of the files
"""

from __future__ import annotations

import random
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import registry  # noqa: E402
import svgkit as K  # noqa: E402
import tokens  # noqa: E402
from svgkit import Svg  # noqa: E402  (re-export: ``from make_node_schematics import Svg`` keeps working)

mix = tokens.mix  # re-export (was defined here before tokens.py existed)

__all__ = ["Svg", "NodeSvg", "mix", "FIGURES", "main"]

OUT = registry.IMAGES / "nodes"
GENERATOR = "docs/figures/nodes/make_node_schematics.py"

#: This section's material fills beyond ``tokens.MATERIAL``: name -> (colour key, tint fraction).
MATERIALS: dict[str, tuple[str, float]] = {
    "si": ("blue", 0.30),
    "si2": ("violet", 0.30),  # second channel material (pFET sheets in CFET)
    "sige": ("magenta", 0.40),  # the sacrificial layers of a nanosheet superlattice
    "gate": ("orange", 0.45),
    "hk": ("magenta", 1.00),
    "ox": ("gray", 0.18),
    "spacer2": ("yellow", 0.55),
    "hm": ("gray", 0.40),
    "power": ("red", 0.45),
    "maskA": ("blue", 0.55),
    "maskB": ("orange", 0.55),
    "cut": ("red", 0.30),
    "via": ("yellow", 0.75),
    "mo": ("blue", 0.60),
    "ru": ("aqua", 0.60),
    "pellicle": ("brand:cyan700", 0.45),
}


class NodeSvg(Svg):
    """:class:`svgkit.Svg` with the nodes section's extra material names."""

    def __init__(self, width, height, theme, title, desc):
        super().__init__(width, height, theme, title, desc, generator=GENERATOR)

    def fill(self, mat: str) -> str:
        if mat in MATERIALS:
            key, frac = MATERIALS[mat]
            return self.tint(key, frac)
        return super().fill(mat)

    def stroke(self, mat: str) -> str:
        if mat in MATERIALS:
            return self.c(MATERIALS[mat][0])
        return super().stroke(mat)

    def swatch_legend(self, x, y, items, size=K.LABEL, gap=26):
        """items: (material_or_colour, label, kind) with kind in {'rect', 'outline', 'line', 'dash', 'hatch'}."""
        cx = x
        for mat, label, kind in items:
            if mat in MATERIALS or mat in self.t["material"]:
                col, fillc = self.stroke(mat), self.fill(mat)
            else:
                col, fillc = self.c(mat), self.tint(mat, 0.35)
            if kind == "rect":
                self.rect(cx, y - 11, 16, 12, fill=fillc, stroke=col, rx=2)
            elif kind == "outline":
                self.rect(cx, y - 11, 16, 12, fill="none", stroke=col, dash="3 2", rx=2)
            elif kind == "line":
                self.line(cx, y - 5, cx + 16, y - 5, col, 2.2)
            elif kind == "dash":
                self.line(cx, y - 5, cx + 16, y - 5, col, 1.6, dash="4 3")
            elif kind == "hatch":
                self.rect(cx, y - 11, 16, 12, fill=self.hatch(f"lg{int(cx)}", col, 4, 0.9), stroke=col, rx=2)
            elif kind == "dot":
                self.circle(cx + 8, y - 5, 5, fill=col, stroke="surface", sw=1.5)
            elif kind == "hollow":
                self.circle(cx + 8, y - 5, 5, fill="none", stroke=col, sw=1.6)
            self.text(cx + 22, y, label, size=size, anchor="start", color="ink2")
            cx += 22 + K.text_width(label, size) + gap


# ---------------------------------------------------------------------------
# 1. Transistor architectures: planar -> FinFET -> GAA nanosheet -> CFET
# ---------------------------------------------------------------------------
def _gaa_sheets(s: NodeSvg, x, ys, w=90, mat="si"):
    """Stacked nanosheets at ``x`` (left edge), one per y in ``ys``, each inside its high-k ring."""
    for sy in ys:
        s.rect(x - 5, sy - 5, w + 10, 22, "hk", sw=0.6)
        s.rect(x, sy, w, 12, mat)


def fig_transistor_evolution(theme: str) -> NodeSvg:
    s = NodeSvg(
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
    notes = ["gate on 1 side", "gate on 3 sides", "gate on all 4 sides", "nFET under pFET, one gate"]
    for i in range(4):
        px = x0 + i * (pw + gap)
        s.text(px + pw / 2, 26, names[i], size=15, weight=600)
        s.rect(px, top, pw, 260, fill="none", stroke="grid", sw=1, rx=6)
        if i == 0:  # planar
            s.rect(px + 12, top + 150, pw - 24, 96, "si")
            s.rect(px + 12, top + 150, 34, 56, "ox")
            s.rect(px + pw - 46, top + 150, 34, 56, "ox")
            s.rect(px + 50, top + 80, 100, 64, "gate")
            s.rect(px + 50, top + 143, 100, 7, "hk", sw=0.6)
            s.rect(px + 50, top + 150, 100, 10, fill=s.tint("blue", 0.6), stroke="none")
            s.text(px + 100, top + 117, "gate", size=K.LABEL, color="ink")
            s.text(px + 100, top + 180, "channel", size=K.SMALL, color="ink2")
            s.text(px + 100, top + 228, "Si substrate", size=K.SMALL, color="ink2")
        elif i == 1:  # FinFET
            s.rect(px + 12, top + 200, pw - 24, 46, "si")
            s.rect(px + 30, top + 60, 140, 110, "gate")
            for fx in (px + 64, px + 114):
                s.rect(fx - 5, top + 85, 32, 90, "hk", sw=0.6)
                s.rect(fx, top + 90, 22, 110, "si")
            s.rect(px + 12, top + 170, pw - 24, 30, "ox")
            for fx in (px + 64, px + 114):
                s.rect(fx, top + 170, 22, 30, "si")
            s.text(px + 100, top + 78, "gate", size=K.LABEL)
            s.text(px + 38, top + 190, "STI", size=K.SMALL, color="ink2")
            s.text(px + 75, top + 142, "fin", size=K.SMALL, color="ink2")
        elif i == 2:  # GAA nanosheet
            s.rect(px + 12, top + 200, pw - 24, 46, "si")
            s.rect(px + 12, top + 180, pw - 24, 20, "ox")
            s.rect(px + 30, top + 48, 140, 132, "gate")
            _gaa_sheets(s, px + 55, (top + 72, top + 108, top + 144))
            s.text(px + 100, top + 64, "gate", size=K.LABEL)
            s.text(px + 100, top + 228, "Si substrate", size=K.SMALL, color="ink2")
        else:  # CFET
            s.rect(px + 12, top + 214, pw - 24, 32, "si")
            s.rect(px + 12, top + 198, pw - 24, 16, "ox")
            s.rect(px + 30, top + 30, 132, 168, "gate")
            _gaa_sheets(s, px + 50, (top + 50, top + 80), mat="si2")
            s.rect(px + 38, top + 112, 116, 10, "ox", sw=0.8)
            _gaa_sheets(s, px + 50, (top + 140, top + 170))
            s.text(px + 181, top + 76, "p", size=K.BASE, weight=600)
            s.text(px + 181, top + 166, "n", size=K.BASE, weight=600)
            s.text(px + 96, top + 44, "gate", size=K.SMALL)
        s.text(px + pw / 2, top + 284, notes[i], size=K.LABEL, color="ink2")
    for i in range(3):  # progression arrows between panels
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
def fig_standard_cell(theme: str) -> NodeSvg:
    s = NodeSvg(
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
    s.rect(x0, y0, cw, ch, fill="none", stroke="muted", sw=1.4, dash="6 4")
    for fy in (y0 + 70, y0 + 92, y0 + ch - 104, y0 + ch - 82):
        s.rect(x0 + 18, fy, cw - 36, 9, "si", sw=0.8)
    s.text(x0 + cw + 14, y0 + 90, "pFET fins", size=K.SMALL, anchor="start", color="ink2")
    s.text(x0 + cw + 14, y0 + ch - 80, "nFET fins", size=K.SMALL, anchor="start", color="ink2")
    for g in range(ncpp + 1):
        gx = x0 + g * cpp
        dummy = g in (0, ncpp)
        s.rect(gx - 8, y0 + 16, 16, ch - 32, "gate", sw=1.0, dash="4 3" if dummy else None,
               opacity=0.55 if dummy else None)
    for t in range(1, ntr):
        ty = y0 + t * mmp
        s.line(x0 - 20, ty, x0 + cw + 6, ty, "grid", 1.0)
    for t, a, b in ((1, 0.3, 2.6), (2, 1.2, 3.4), (3, 0.5, 1.6), (3, 2.4, 3.7), (4, 0.8, 3.1), (5, 1.6, 3.6)):
        ty = y0 + t * mmp
        s.rect(x0 + a * cpp, ty - 9, (b - a) * cpp, 18, "metal", sw=1.0, rx=3, opacity=0.9)
    s.rect(x0 - 20, y0 - 12, cw + 40, 24, "power", rx=3)
    s.rect(x0 - 20, y0 + ch - 12, cw + 40, 24, "power", rx=3)
    s.text(x0 + cw + 30, y0 + 5, "VDD rail", size=K.SMALL, anchor="start", color="ink2")
    s.text(x0 + cw + 30, y0 + ch + 5, "VSS rail", size=K.SMALL, anchor="start", color="ink2")
    s.dim_h(x0 + cpp, x0 + 2 * cpp, y0 - 26, "CPP (gate pitch)")
    s.dim_v(x0 + cw + 110, y0 + 2 * mmp, y0 + 3 * mmp, "MMP (metal pitch)", side="right")
    s.dim_v(x0 - 44, y0, y0 + ch, "cell height", side="left")
    s.text(x0 - 54, y0 + ch / 2 + 22, f"= {ntr} tracks × MMP", size=K.SMALL, anchor="end", color="ink2")
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
# helpers for process-flow rows and the SADP geometry shared by the still and the animation
# ---------------------------------------------------------------------------
def _row_base(s: NodeSvg, x, y, w, label, step, sub=None, hm=True):
    s.text(20, y + 26, step, size=15, anchor="start", weight=600)
    s.text(46, y + 26, label, size=K.BASE, anchor="start")
    if sub:
        s.text(46, y + 46, sub, size=K.SMALL, anchor="start", color="ink2")
    _target(s, x, y + 70, w, hm)


def _target(s: NodeSvg, x, base, w, hm=True, k=1.0):
    """The target layer (hard mask) at ``base`` and the substrate under it."""
    if hm:
        s.rect(x, base, w, 16 * k, "hm", sw=1.0)
    s.rect(x, base + 16 * k, w, 16 * k, "si", sw=1.0)


SADP_STEPS = [
    ("1", "Litho + etch: mandrels", "one mask, printed at pitch P"),
    ("2", "Conformal spacer deposition", "film thickness t on every surface"),
    ("3", "Anisotropic spacer etch-back", "spacers survive on the sidewalls"),
    ("4", "Mandrel pull", "two spacer lines per mandrel"),
    ("5", "Transfer into the target layer", "lines at pitch P/2, no second mask"),
]


def _sadp_state(s: NodeSvg, r: int, X, base, P, ts, h, xs, *, W=540.0, k=1.0, dims=True):
    """Cross-section of SADP state ``r`` (0..4) with its features standing on ``base``."""
    M = (P - 2 * ts) / 2  # uniform-pitch condition: mandrel width = gap width
    if r == 1:  # conformal film over mandrels and floor
        s.rect(X, base - ts * 0.5, W, ts * 0.5, "spacer", sw=0.8)
        for mx in xs:
            s.rect(mx - ts, base - h - ts * 0.5, M + 2 * ts, h + ts * 0.5, "spacer", sw=0.8, rx=10 * k)
    if r in (0, 1, 2):
        for mx in xs:
            s.rect(mx, base - h, M, h, "mandrel", sw=1.0)
    if r in (2, 3):
        for mx in xs:
            s.rect(mx - ts, base - h, ts, h, "spacer", sw=0.9, rx=3)
            s.rect(mx + M, base - h, ts, h, "spacer", sw=0.9, rx=3)
    if r == 4:  # etched target: the hard mask survives only under the spacers
        for mx in xs:
            for lx in (mx - ts, mx + M):
                s.rect(lx, base, ts, 16 * k, "hm", sw=1.0)
                s.rect(lx, base - h * 0.45, ts, h * 0.45, "spacer", sw=0.8, opacity=0.35)
    if not dims:
        return
    if r == 0:
        s.dim_h(xs[0], xs[1], base - h - 12, "P")
    if r == 1:
        s.dim_h(xs[-1] + M, xs[-1] + M + ts, base - h - ts * 0.5 - 12, "t")
    if r == 3:
        s.dim_h(xs[0] - ts, xs[0] + M, base - h - 12, "P/2")
        s.text(xs[0] + M / 2, base - 8, "core", size=K.SMALL, color="ink2")
        s.text((xs[0] + M + ts + xs[1] - ts) / 2, base - 8, "gap", size=K.SMALL, color="ink2")
    if r == 4:
        s.dim_h(xs[0] - ts, xs[0] + M, base - h * 0.45 - 12, "P/2")


def _sadp_rows(s: NodeSvg, legend_y: float = 630) -> None:
    """The five-row SADP drawing (the static figure and the animation's designed still)."""
    X, W = 330, 540
    P, ts, h = 180.0, 40.0, 44.0
    xs = [X + 45 + k * P for k in range(3)]
    for r, (step, label, sub) in enumerate(SADP_STEPS):
        y = 18 + r * 118
        _row_base(s, X, y, W, label, step, sub, hm=(r != 4))
        _sadp_state(s, r, X, y + 70, P, ts, h, xs, W=W)
    s.swatch_legend(
        150,
        legend_y,
        [
            ("mandrel", "mandrel", "rect"),
            ("spacer", "spacer", "rect"),
            ("hm", "target / hard mask", "rect"),
            ("si", "substrate", "rect"),
        ],
    )


SADP_TITLE = "Self-aligned double patterning (SADP)"
SADP_DESC = (
    "Five-step SADP flow in cross-section. 1: mandrel lines printed at pitch P. 2: a conformal "
    "spacer film is deposited. 3: an anisotropic etch leaves spacers only on the mandrel "
    "sidewalls. 4: the mandrels are removed, leaving two spacer lines per mandrel at pitch P/2. "
    "5: the spacer pattern is etched into the target layer."
)


def fig_sadp(theme: str) -> NodeSvg:
    s = NodeSvg(900, 640, theme, SADP_TITLE, SADP_DESC)
    _sadp_rows(s)
    return s


def fig_sadp_anim(theme: str) -> NodeSvg:
    """N10: the five SADP states cross-fade in one large panel (2 s holds, 3 loops, rest on the last)."""
    s = NodeSvg(900, 640, theme, SADP_TITLE + ", step by step",
                SADP_DESC + " Animated: the five states cross-fade in one panel, two seconds each; "
                "with reduced motion the five rows are shown side by side.")
    anim = K.Anim(s, loops=3)
    hold, fade = 2.0, 0.6
    step = hold + fade
    T = 5 * step  # 13 s per loop
    with anim.layer("anim"):
        s.text(20, 36, "Self-aligned double patterning", size=17, anchor="start", weight=600)
        s.text(20, 58, "one lithography step, two lines per mandrel", size=K.LABEL, anchor="start", color="ink2")
        # the step list (left): static text, an animated marker behind the active number
        for k, (num, label, sub) in enumerate(SADP_STEPS):
            y = 110 + k * 78
            s.rect(16, y - 24, 30, 30, fill="axis", stroke="none", rx=15, cls=f"mk{k}")
            s.text(31, y - 4, num, size=15, weight=600)
            s.text(60, y - 6, label, size=K.BASE, anchor="start")
            s.text(60, y + 14, sub, size=K.SMALL, anchor="start", color="ink2")
        # the cross-section (right): one state group per step
        k_ = 1.5
        X, base = 360, 420.0
        W = 520
        P, ts, h = 180.0 * k_, 40.0 * k_, 44.0 * k_
        M = (P - 2 * ts) / 2
        xs = [X + 70 + i * P for i in range(2)]
        for r in range(5):
            with s.group(f"st{r}", opacity=1.0 if r == 0 else 0.0):
                _target(s, X, base, W, hm=(r != 4), k=k_)
                _sadp_state(s, r, X, base, P, ts, h, xs, W=W, k=k_)
                s.text(X + W / 2, base + 32 * k_ + 26, SADP_STEPS[r][1], size=K.BASE, color="ink")
        s.text(X + W / 2, 92, "cross-section, features standing on the target layer", size=K.SMALL, color="ink2")
        # a progress bar under the panel: one segment per step, filled while that step holds
        bx, by, bw = X, 560, W
        seg = bw / 5
        for r in range(5):
            s.rect(bx + r * seg + 2, by, seg - 4, 6, fill="grid", stroke="none", rx=3)
            s.rect(bx + r * seg + 2, by, seg - 4, 6, fill="axis", stroke="none", rx=3, cls=f"pb{r}")
        s.swatch_legend(
            X,
            612,
            [
                ("mandrel", "mandrel", "rect"),
                ("spacer", "spacer", "rect"),
                ("hm", "target / hard mask", "rect"),
                ("si", "substrate", "rect"),
            ],
        )
        s.text(20, 612, "Ideal conformal spacers; no deposition or etch physics.", size=K.SMALL, anchor="start",
               color="muted")
        # timelines
        for r in range(5):
            t0 = r * step
            kf = anim.keyframes(f"st{r}", T)
            kf.at(0, "opacity:1" if r == 0 else "opacity:0")
            if r > 0:
                kf.at(t0 - fade, "opacity:0")
                kf.at(t0, "opacity:1")
            kf.at(t0 + hold, "opacity:1")
            if r < 4:
                kf.at(t0 + hold + fade, "opacity:0")
            kf.at(T, "opacity:1" if r == 4 else "opacity:0")
            anim.animate(f".st{r}", f"st{r}", T)
            # marker: 0.25 when idle, 1 while active (same timing, no fade)
            mk = anim.keyframes(f"mk{r}", T)
            mk.at(0, "opacity:1" if r == 0 else "opacity:0.25")
            if r > 0:
                mk.at(t0 - 0.01, "opacity:0.25")
                mk.at(t0, "opacity:1")
            mk.at(t0 + hold + fade - 0.01, "opacity:1")
            if r < 4:
                mk.at(t0 + hold + fade, "opacity:0.25")
            mk.at(T, "opacity:1" if r == 4 else "opacity:0.25")
            anim.animate(f".mk{r}", f"mk{r}", T)
            # progress segment: grows from 0 to full width while the step holds (scaleX on the group origin)
            pb = anim.keyframes(f"pb{r}", T)
            pb.at(0, "opacity:1" if r == 0 else "opacity:0")
            if r > 0:
                pb.at(t0 - 0.01, "opacity:0")
                pb.at(t0, "opacity:1")
            pb.at(T, "opacity:1")
            anim.animate(f".pb{r}", f"pb{r}", T)
    with anim.layer("still"):
        _sadp_rows(s)
    anim.install()
    return s


def fig_saqp(theme: str) -> NodeSvg:
    s = NodeSvg(
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


def fig_pitch_walk(theme: str) -> NodeSvg:
    s = NodeSvg(
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
        s.text(20, y + 62, sub, size=K.LABEL, anchor="start", color="ink2")
        base = y + 96
        s.rect(X - 10, base, 3 * P + 20, 14, "hm", sw=0.8)
        tt = t + dt
        for k in range(3):
            mx = X + 50 + k * P - dM / 2
            MM = M + dM
            s.rect(mx, base - 58, MM, 58, fill="none", stroke=s.stroke("mandrel"), sw=1.0, dash="4 3")
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


def fig_lele_overlay(theme: str) -> NodeSvg:
    s = NodeSvg(
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
        s.text(20, y + 66, sub, size=K.LABEL, anchor="start", color="ink2")
        base = y + 110
        s.rect(X - 10, base, 6 * p, 14, "hm", sw=0.8)
        for k in range(6):
            isB = k % 2 == 1
            lx = X + k * p + (d if isB else 0.0)
            s.rect(lx, base - 70, w, 70, "maskB" if isB else "maskA", rx=2)
            s.text(lx + w / 2, base - 30, "B" if isB else "A", size=K.LABEL, weight=600)
        s.dim_h(X + w, X + p + d, base - 82, "s − δ" if d else "s")
        s.dim_h(X + p + d + w, X + 2 * p, base - 82, "s + δ" if d else "s")
        if d:
            s.line(X + p, base + 24, X + p + d, base + 24, "red", 1.4, arrow="end")
            s.text(X + p + d / 2, base + 42, "δ", size=K.LABEL, color="ink")
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
def fig_backside_power(theme: str) -> NodeSvg:
    s = NodeSvg(
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
        s.text(px + 200, 26, ["Front-side power (FSPDN)", "Backside power (BSPDN)"][panel], size=15, weight=600)
        if panel == 0:
            s.rect(px + 20, 330, 360, 80, "si")
            s.text(px + 200, 376, "silicon substrate", size=K.SMALL, color="ink2")
            s.rect(px + 20, 306, 360, 24, "gate", sw=1.0)
            s.text(px + 200, 323, "transistors", size=K.SMALL)
            widths = [8, 8, 10, 10, 12, 14, 18, 22]
            yy = 300
            for k, wth in enumerate(widths):
                yy -= wth + 10
                for j in range(0, 360, 40 + k * 6):
                    power = (j // (40 + k * 6)) % 3 == 0
                    s.rect(px + 22 + j, yy, 22 + k * 3, wth, "power" if power else "metal", sw=0.7, rx=1)
            s.rect(px + 60, 44, 40, 14, "power", rx=7)
            s.rect(px + 300, 44, 40, 14, "power", rx=7)
            s.line(px + 80, 60, px + 80, 300, "red", 2.0, arrow="end")
            s.text(px + 110, 74, "supply current crosses", size=K.SMALL, anchor="start", color="ink2")
            s.text(px + 110, 90, "the whole metal stack", size=K.SMALL, anchor="start", color="ink2")
        else:
            s.rect(px + 20, 44, 360, 40, "ox")
            s.text(px + 200, 70, "bonded carrier wafer", size=K.SMALL, color="ink2")
            widths = [16, 12, 10, 10, 8, 8]
            yy = 88
            for k, wth in enumerate(widths):
                for j in range(0, 360, 44):
                    s.rect(px + 22 + j, yy, 26, wth, "metal", sw=0.7, rx=1)
                yy += wth + 10
            s.text(px + 200, yy + 4, "front metal: signals only", size=K.SMALL, color="ink2")
            s.rect(px + 20, 250, 360, 24, "gate", sw=1.0)
            s.text(px + 200, 267, "transistors", size=K.SMALL)
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
            s.text(px + 100, 290, "nano-TSVs", size=K.SMALL, color="ink", halo=True)
            s.text(px + 200, 432, "backside power network (thinned wafer)", size=K.SMALL, color="ink2")
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
def fig_anamorphic_field(theme: str) -> NodeSvg:
    s = NodeSvg(
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
    s.text(rx + 52 * k, 40, "reticle image area", size=K.BASE, weight=600)
    s.rect(rx, ry, 104 * k, 132 * k, fill=s.tint("violet", 0.18), stroke="violet", sw=1.4, rx=4)
    s.dim_h(rx, rx + 104 * k, ry + 132 * k + 22, "104 mm", above=False)
    s.dim_v(rx - 14, ry, ry + 132 * k, "132 mm", side="left")
    ax = rx + 104 * k + 30
    s.line(ax, ry + 66 * k, ax + 90, ry + 66 * k, "muted", 1.8, arrow="end")
    s.text(ax + 45, ry + 66 * k - 30, "projection", size=K.SMALL, color="ink2")
    s.text(ax + 45, ry + 66 * k - 14, "optics", size=K.SMALL, color="ink2")
    w = 5.2  # px per mm on the wafer drawing
    fx, fy = ax + 160, ry + 10
    s.text(fx + 13 * w, 40, "0.33 NA: 4× / 4×", size=K.BASE, weight=600)
    s.rect(fx, fy, 26 * w, 33 * w, fill=s.tint("blue", 0.18), stroke="blue", sw=1.4, rx=3)
    s.text(fx + 13 * w, fy + 16.5 * w + 5, "full field", size=K.LABEL)
    s.dim_h(fx, fx + 26 * w, fy + 33 * w + 22, "26 mm", above=False)
    s.dim_v(fx - 12, fy, fy + 33 * w, "33 mm", side="left")
    gx = fx + 26 * w + 70
    s.text(gx + 13 * w, 40, "0.55 NA: 4× / 8×", size=K.BASE, weight=600)
    s.rect(gx, fy, 26 * w, 16.5 * w, fill=s.tint("orange", 0.25), stroke="orange", sw=1.4, rx=3)
    s.rect(gx, fy + 16.5 * w, 26 * w, 16.5 * w, fill="none", stroke="orange", sw=1.2, rx=3, dash="5 4")
    s.text(gx + 13 * w, fy + 8.25 * w + 5, "half field", size=K.LABEL)
    s.text(gx + 13 * w, fy + 24.75 * w - 4, "2nd exposure", size=K.SMALL, color="ink2")
    s.text(gx + 13 * w, fy + 24.75 * w + 12, "(stitched)", size=K.SMALL, color="ink2")
    s.dim_h(gx, gx + 26 * w, fy + 33 * w + 22, "26 mm", above=False)
    s.dim_v(gx + 26 * w + 12, fy, fy + 16.5 * w, "16.5 mm", side="right")
    s.text(450, 415, "x = across the scan · y = scan direction (vertical here)", size=K.SMALL, color="ink2")
    return s


# ---------------------------------------------------------------------------
# Pupil / diffraction-order picture behind k1 = 0.25 (the D5 pupil grammar)
# ---------------------------------------------------------------------------
def fig_pupil_orders(theme: str) -> NodeSvg:
    s = NodeSvg(
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
        ("On-axis source", "orders outside → no image", [(0, 0.0), (1, 1.35), (-1, -1.35)], 0.0),
        ("Dipole (off-axis) source", "0 and −1 inside → two-beam image", [(0, 0.68), (-1, -0.68), (1, 2.03)], 0.68),
        ("The limit: k₁ = 0.25", "orders at the rim, spacing 2·NA", [(0, 1.0), (-1, -1.0), (1, 3.0)], 1.0),
    ]
    for i, (title, sub, ords, src) in enumerate(panels):
        cx, cy = 150 + i * 300, 190
        s.text(cx, 36, title, size=15, weight=600)
        # the pupil with its (point-like) source: on-axis = a small coherent core; off-axis = one pole
        if src == 0.0:
            K.pupil(s, cx, cy, R, fill=None, coherent=0.06)
        else:
            K.pupil(s, cx, cy, R, fill="poles", poles=((src, 0.0, 0.06),))
        s.line(cx - R - 20, cy, cx + R + 20, cy, "grid", 1.0)
        shown, far = [], []
        for m, u in ords:
            (far if abs(u) > 1.0 + 60 / R else shown).append((m, u))
        K.orders(s, cx, cy, R, [(m, u, 0.0) for m, u in shown], stem=False, dot=7, label=False)
        for m, u in shown:
            lab = {0: "0", 1: "+1", -1: "−1"}[m]
            ox = cx + u * R
            if abs(abs(u) - 1.0) < 1e-6:  # order on the rim: label outside the circle
                s.text(ox + 12 * (1 if u > 0 else -1), cy - 14, lab, size=K.SMALL, color="ink2",
                       anchor="start" if u > 0 else "end")
            else:
                s.text(ox, cy - 20, lab, size=K.SMALL, color="ink2")
        for m, u in far:
            lab = {0: "0", 1: "+1", -1: "−1"}[m]
            s.text(cx + (R + 30) * (1 if u > 0 else -1), cy + 28, f"{lab} →" if u > 0 else f"← {lab}",
                   size=K.SMALL, color="muted")
        s.text(cx, cy + R + 36, sub, size=K.LABEL, color="ink2")
        if i == 1:
            s.text(cx, cy + R + 54, "(one pole shown; its mirror pole works the same way)", size=K.SMALL, color="muted")
        if i == 2:
            s.dim_h(cx - R, cx + R, cy + R + 60, "λ/p = 2·NA  ⇒  p = λ/(2·NA)")
    s.swatch_legend(
        250,
        408,
        [
            ("pupil:order0", "zeroth order", "dot"),
            ("pupil:order1", "first order inside the pupil", "dot"),
            ("muted", "blocked order", "hollow"),
        ],
    )
    return s


# ---------------------------------------------------------------------------
# EUV stochastic failure modes (illustrative, seeded)
# ---------------------------------------------------------------------------
def fig_stochastic_defects(theme: str) -> NodeSvg:
    s = NodeSvg(
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
                s.circle(cx, cy, 13, fill="none", stroke="red", sw=1.4, dash="4 3")
                continue
            if (i, j) in ((4, 3), (5, 3)):
                continue
            s.circle(cx, cy, r, fill=s.fill("si"), stroke=s.stroke("si"), sw=1.0)
    cx1, cx2, cy = x0 + 4 * pc + 26, x0 + 5 * pc + 26, y0 + 3 * pc + 26
    s.path(
        f"M{cx1},{cy - 15} L{cx2},{cy - 14} A14,14 0 0,1 {cx2},{cy + 14} L{cx1},{cy + 15} A15,15 0 0,1 {cx1},{cy - 15} Z",
        "si",
        sw=1.0,
    )
    s.text(x0 + 2 * pc + 26, y0 + pc + 52, "missing", size=K.SMALL, color="ink", halo=True)
    s.text(x0 + 4.5 * pc + 26, y0 + 3 * pc + 58, "merged", size=K.SMALL, color="ink", halo=True)
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
                d += " L" + " L".join(f"{px:.1f},{py:.1f}" for px, py in reversed(b)) + " Z"
                s.path(d, "si", sw=1.0)
            continue
        d = "M" + " L".join(f"{px:.1f},{py:.1f}" for px, py in pts_l)
        d += " L" + " L".join(f"{px:.1f},{py:.1f}" for px, py in reversed(pts_r)) + " Z"
        s.path(d, "si", sw=1.0)
    bx = lx0 + 1 * lp + lw - 1
    s.path(
        f"M{bx},{ly0 + 196} C{bx + 10},{ly0 + 202} {bx + 16},{ly0 + 202} {bx + 27},{ly0 + 194} "
        f"L{bx + 27},{ly0 + 210} C{bx + 16},{ly0 + 214} {bx + 10},{ly0 + 214} {bx},{ly0 + 212} Z",
        "si",
        sw=1.0,
    )
    s.line(bx + 13, ly0 + 216, bx + 13, ly0 + L + 12, "ink2", 1.0)
    s.text(bx + 13, ly0 + L + 28, "microbridge", size=K.SMALL)
    s.text(lx0 + 5 * lp + lw / 2, ly0 + 136, "break", size=K.SMALL, halo=True)
    s.text(lx0 + 5 * lp, ly0 + L + 28, "rough edges everywhere: LER", size=K.SMALL, color="ink2")
    s.text(450, 378, "Schematic, not simulated: sizes and positions are illustrative.", size=K.SMALL, color="muted")
    return s


# ---------------------------------------------------------------------------
# N11 · the EUV mask stack
# ---------------------------------------------------------------------------
def fig_euv_mask_stack(theme: str) -> NodeSvg:
    s = NodeSvg(
        900,
        440,
        theme,
        "An EUV mask: Mo/Si multilayer mirror, Ru cap, absorber, 6-degree chief ray, pellicle",
        "Cross-section of an EUV reticle. A low-expansion substrate carries 40 to 50 Mo/Si bilayers "
        "(eight drawn) capped with ruthenium; a tantalum-based absorber tens of nanometres thick is "
        "patterned on top. The chief ray arrives 6 degrees off normal, so the absorber casts a "
        "shadow (hatched) on the mirror beside it, and a thin pellicle above the mask is crossed "
        "twice. The simulator's mask is a thin transmittance map with none of these effects.",
    )
    X, W = 120, 520  # mask extent
    y_cap, cap_h = 250, 7
    ml_top, ml_h = y_cap + cap_h, 110
    sub_top, sub_h = ml_top + ml_h, 46
    # substrate and multilayer
    s.rect(X, sub_top, W, sub_h, "glass", sw=1.0)
    s.text(X + W / 2, sub_top + 29, "low-thermal-expansion substrate", size=K.LABEL, color="ink2")
    n_pairs = 8
    bil = ml_h / n_pairs
    for k in range(n_pairs):
        y = ml_top + k * bil
        s.rect(X, y, W, bil * 0.4, "mo", sw=0.4)
        s.rect(X, y + bil * 0.4, W, bil * 0.6, "si", sw=0.4)
    # a break mark through the stack: "40–50 bilayers, 8 drawn"
    bx = X + W - 70
    s.path(f"M{bx},{ml_top - 2} l6,10 l-12,12 l12,12 l-12,12 l12,12 l-12,12 l12,12 l-12,12 l12,12 l-6,10",
           fill="none", stroke="surface", sw=5)
    s.path(f"M{bx},{ml_top - 2} l6,10 l-12,12 l12,12 l-12,12 l12,12 l-12,12 l12,12 l-12,12 l12,12 l-6,10",
           fill="none", stroke="ink2", sw=1.2)
    # cap and absorber
    s.rect(X, y_cap, W, cap_h, "ru", sw=0.6)
    ab_h, ab_w = 46, 64
    abs_x = [X + 80, X + 230, X + 390]
    for ax_ in abs_x:
        s.rect(ax_, y_cap - ab_h, ab_w, ab_h, "absorber", sw=0.8, rx=2)
    # the chief ray (angle exaggerated for legibility) hitting the mirror between absorbers
    import math  # noqa: PLC0415

    ang = math.radians(22)  # drawn angle; the real chief-ray angle is 6°
    hit_x, hit_y = X + 330, y_cap
    L = 215
    ix, iy = hit_x - L * math.sin(ang), hit_y - L * math.cos(ang)
    rx_, ry_ = hit_x + L * math.sin(ang), hit_y - L * math.cos(ang)
    # pellicle: a thin membrane on a frame, crossed by both rays
    pel_y = 118
    s.rect(X + 30, pel_y - 1.5, W - 60, 3, "pellicle", sw=0.6)
    for fx in (X + 30, X + W - 36):
        s.rect(fx, pel_y, 6, y_cap - ab_h - pel_y, "hm", sw=0.6)
    s.text(X + 44, pel_y - 8, "pellicle (crossed twice: T counts as T²)", size=K.SMALL, anchor="start",
           color="ink2", halo=True)
    s.line(ix, iy, hit_x, hit_y, "orange", 2.0, arrow="end")
    s.line(hit_x, hit_y, rx_, ry_, "orange", 2.0, arrow="end")
    # the normal and the angle label
    s.line(hit_x, hit_y - 150, hit_x, hit_y, "muted", 1.0, dash="3 3")
    s.text(ix - 8, iy + 4, "incident", size=K.SMALL, anchor="end", color="ink2", halo=True)
    s.text(rx_ + 8, ry_ + 4, "reflected (≈ 70 %)", size=K.SMALL, anchor="start", color="ink2", halo=True)
    s.text(hit_x + 8, hit_y - 112, "6° chief ray (angle exaggerated here)", size=K.SMALL, anchor="start",
           color="ink", halo=True)
    s.path(f"M{hit_x},{hit_y - 60} A60,60 0 0,0 {hit_x - 60 * math.sin(ang):.1f},{hit_y - 60 * math.cos(ang):.1f}",
           fill="none", stroke="ink2", sw=1.0)
    # the absorber shadow on the cap, beside the absorber that the ray grazes
    sh_x0 = abs_x[2]
    sh_w = ab_h * math.tan(ang)
    hatch = s.hatch("shadow", s.c("red"), 5, 1.0)
    s.path(f"M{sh_x0 - sh_w},{y_cap} L{sh_x0},{y_cap - ab_h} L{sh_x0},{y_cap} Z", fill=hatch, stroke="red", sw=0.8)
    s.line(sh_x0 - sh_w / 2, y_cap - ab_h / 2, sh_x0 + ab_w + 10, y_cap - ab_h - 24, "red", 1.0)
    s.text(sh_x0 + ab_w + 14, y_cap - ab_h - 28, "shadow beside the absorber", size=K.SMALL, anchor="start",
           color="ink", halo=True)
    s.text(sh_x0 + ab_w + 14, y_cap - ab_h - 13, "(mask 3-D, orientation-dependent)", size=K.SMALL, anchor="start",
           color="ink2", halo=True)
    # right-hand labels with leaders
    lx = X + W + 14
    for yy, text, col in (
        (y_cap - ab_h / 2, "Ta-based absorber (tens of nm)", "ink"),
        (y_cap + cap_h / 2, "Ru capping layer", "ink"),
        (ml_top + ml_h / 2 - 8, "40–50 Mo/Si bilayers (8 drawn):", "ink"),
        (ml_top + ml_h / 2 + 8, "a Bragg mirror, ≈ 70 % at 13.5 nm", "ink2"),
    ):
        if "Bragg" not in text:
            s.line(X + W + 2, yy, lx - 4, yy, "muted", 1.0)
        s.text(lx, yy + 4.5, text, size=K.SMALL, anchor="start", color=col)
    s.dim_v(X - 16, y_cap - ab_h, y_cap, "tens of nm", side="left")
    s.dim_v(X - 16, ml_top, ml_top + ml_h, "≈ 0.3 µm", side="left")
    s.text(X + 30, 30, "EUV mask (reticle), cross-section", size=15, anchor="start", weight=600)
    s.text(450, 428, "highuvlith: the mask is a thin transmittance map — no shadowing, no phase, no pellicle.",
           size=K.SMALL, color="muted")
    return s


# ---------------------------------------------------------------------------
# N12 · the nanosheet build flow
# ---------------------------------------------------------------------------
def fig_nanosheet_flow(theme: str) -> NodeSvg:
    s = NodeSvg(
        900,
        420,
        theme,
        "How a nanosheet transistor is built",
        "Four cross-section steps: a silicon / silicon-germanium superlattice is grown; the stack is "
        "patterned into a narrow ribbon; the silicon-germanium is etched away selectively, leaving "
        "suspended silicon sheets; the gate dielectric and metal wrap every sheet. An inset "
        "contrasts a FinFET, whose drive is quantized in whole fins, with nanosheets whose width is "
        "a design variable.",
    )
    pw, gap, x0, top = 160, 14, 20, 50
    names = ["1 · Superlattice", "2 · Ribbon", "3 · SiGe release", "4 · Gate wrap"]
    notes = ["Si / SiGe epitaxy", "pattern into a fin-like stack", "etch SiGe selectively", "gate on all four sides"]
    sheet_ys = (top + 96, top + 128, top + 160)
    for i in range(4):
        px = x0 + i * (pw + gap)
        s.text(px + pw / 2, 30, names[i], size=15, weight=600)
        s.rect(px, top, pw, 240, fill="none", stroke="grid", sw=1, rx=6)
        s.rect(px + 10, top + 196, pw - 20, 36, "si")  # substrate
        s.text(px + pw / 2, top + 220, "Si substrate", size=K.SMALL, color="ink2")
        if i == 0:
            for k, sy in enumerate(sheet_ys):
                s.rect(px + 10, sy - 16, pw - 20, 16, "sige", sw=0.8)
                s.rect(px + 10, sy, pw - 20, 14, "si", sw=0.8)
            s.rect(px + 10, sheet_ys[-1] + 14, pw - 20, 16, "sige", sw=0.8)
            s.text(px + pw - 18, sheet_ys[0] - 4, "SiGe", size=K.SMALL, anchor="end", color="ink", halo=True)
            s.text(px + pw - 18, sheet_ys[0] + 11, "Si", size=K.SMALL, anchor="end", color="ink", halo=True)
        else:
            rx0, rw = px + pw / 2 - 30, 60
            s.rect(px + 10, top + 180, pw - 20, 16, "ox", sw=0.8)  # STI
            if i == 1:
                for sy in sheet_ys:
                    s.rect(rx0, sy - 16, rw, 16, "sige", sw=0.8)
                    s.rect(rx0, sy, rw, 14, "si", sw=0.8)
                s.rect(rx0, sheet_ys[-1] + 14, rw, 16, "sige", sw=0.8)
                s.rect(rx0, sheet_ys[0] - 30, rw, 14, "hm", sw=0.8)
                s.text(rx0 + rw / 2, sheet_ys[0] - 18.5, "hard mask", size=K.SMALL, color="ink", halo=True)
                s.dim_h(rx0, rx0 + rw, top + 56, "sheet width")
            elif i == 2:
                for sy in sheet_ys:
                    s.rect(rx0, sy, rw, 14, "si", sw=0.8)
                    s.rect(rx0 - 8, sy - 16, 8, 16, "ox", sw=0.6)  # inner spacers
                    s.rect(rx0 + rw, sy - 16, 8, 16, "ox", sw=0.6)
                s.text(rx0 + rw / 2, sheet_ys[0] - 22, "suspended Si sheets", size=K.SMALL, color="ink2")
            else:
                s.rect(rx0 - 26, sheet_ys[0] - 30, rw + 52, 110, "gate")
                _gaa_sheets(s, rx0, sheet_ys, w=rw)
                s.text(rx0 + rw / 2, sheet_ys[0] - 36, "metal gate + high-k", size=K.SMALL, color="ink2")
        s.text(px + pw / 2, top + 262, notes[i], size=K.LABEL, color="ink2")
    for i in range(3):
        ax = x0 + (i + 1) * (pw + gap) - gap + 2
        s.line(ax, top + 120, ax + gap - 4, top + 120, "muted", 1.6, arrow="end")
    # inset: whole fins vs variable sheet width
    ix = x0 + 4 * (pw + gap) + 6
    iw = 900 - ix - 16
    s.rect(ix, top, iw, 240, fill="none", stroke="grid", sw=1, rx=6)
    s.text(ix + iw / 2, 30, "Why it matters", size=15, weight=600)
    s.text(ix + iw / 2, top + 22, "FinFET: whole fins", size=K.SMALL, color="ink2")
    fb = top + 96
    for fx in (ix + 30, ix + 62):
        s.rect(fx - 4, fb - 60, 26, 60, "hk", sw=0.5)
        s.rect(fx, fb - 54, 18, 54, "si", sw=0.8)
    s.rect(ix + 14, fb, iw - 28, 10, "ox", sw=0.6)
    s.text(ix + iw / 2, fb + 24, "1, 2, 3 … fins", size=K.SMALL, color="ink2")
    s.text(ix + iw / 2, top + 150, "nanosheet: any width", size=K.SMALL, color="ink2")
    nb = top + 196
    _gaa_sheets(s, ix + 24, (nb - 40, nb - 20), w=36)
    _gaa_sheets(s, ix + 84, (nb - 40, nb - 20), w=18)
    s.rect(ix + 14, nb + 2, iw - 28, 10, "ox", sw=0.6)
    s.text(ix + iw / 2, nb + 26, "wide or narrow sheets", size=K.SMALL, color="ink2")
    s.swatch_legend(
        120,
        406,
        [
            ("si", "silicon", "rect"),
            ("sige", "SiGe (sacrificial)", "rect"),
            ("gate", "metal gate", "rect"),
            ("hk", "high-k", "rect"),
            ("ox", "oxide / spacer", "rect"),
            ("hm", "hard mask", "rect"),
        ],
    )
    return s


# ---------------------------------------------------------------------------
# N13 · cut, block and via masks over a spacer grating
# ---------------------------------------------------------------------------
def fig_cut_mask(theme: str) -> NodeSvg:
    s = NodeSvg(
        900,
        400,
        theme,
        "Cut and via masks carve a circuit out of a spacer grating",
        "Three top views. Left: the regular line grating that spacer patterning produces. Middle: "
        "cut masks remove line segments to make line ends and via masks connect the survivors to "
        "the level above. Right: a cut that lands shifted by a few nanometres clips the neighbouring "
        "line and leaves a sliver on the intended one; the shift is dimensioned as the "
        "edge-placement error.",
    )
    pw, gap, x0, top, ph = 270, 25, 20, 50, 270
    names = ["1 · Spacer grating", "2 · Cut + via masks", "3 · A cut lands off target"]
    notes = ["regular lines only (SADP / SAQP)", "cuts make line ends, vias connect up", "edge-placement error → clipped neighbour"]
    lp, lw = 36, 14  # line pitch and width
    cm = 12  # cut-mask opening margin beyond each line edge (the opening is lw + 2·cm wide, lp − lw − cm from a neighbour)
    n_lines = 7

    def lines(px, segments=None, notch=None):
        for k in range(n_lines):
            lx = px + 18 + k * lp
            segs = (segments or {}).get(k, [(top + 16, top + ph - 16)])
            for y1, y2 in segs:
                s.rect(lx, y1, lw, y2 - y1, "spacer", sw=0.9, rx=2)
        if notch:
            k, y1, y2, dx = notch
            lx = px + 18 + k * lp
            s.rect(lx, y1, dx, y2 - y1, fill="surface", stroke="none")
            s.rect(lx, y1, dx, y2 - y1, fill="none", stroke="red", sw=1.0, dash="3 2")

    for i in range(3):
        px = x0 + i * (pw + gap)
        s.text(px + pw / 2, 30, names[i], size=15, weight=600)
        s.rect(px, top, pw, ph, fill="none", stroke="grid", sw=1, rx=6)
        if i == 0:
            lines(px)
            s.dim_h(px + 18, px + 18 + lp, top + 8, "pitch")
        elif i == 1:
            cuts = [(1, top + 80, top + 110), (4, top + 150, top + 180), (2, top + 190, top + 220)]
            segs = {}
            for k, y1, y2 in cuts:
                segs[k] = [(top + 16, y1), (y2, top + ph - 16)]
            lines(px, segs)
            for k, y1, y2 in cuts:
                lx = px + 18 + k * lp
                s.rect(lx - cm, y1, lw + 2 * cm, y2 - y1, "cut", sw=1.2, rx=3)
            for k, y in ((0, top + 60), (3, top + 120), (5, top + 230), (6, top + 90)):
                lx = px + 18 + k * lp
                s.rect(lx - 2, y - 2, lw + 4, lw + 4, "via", sw=1.0, rx=2)
            s.text(px + 18 + 1 * lp + lw / 2 + 24, top + 100, "cut", size=K.SMALL, anchor="start", color="ink", halo=True)
            s.text(px + 18 + 3 * lp + lw / 2 + 14, top + 132, "via", size=K.SMALL, anchor="start", color="ink", halo=True)
        else:
            k = 3
            y1, y2 = top + 110, top + 140
            # the shift: past the cut margin (a sliver of the intended line survives) and past the space to the
            # neighbour (it is clipped) — both consequences the panel is labelled with must actually be drawn
            d = 17
            assert cm < d < lp - lw, (cm, d, lp - lw)
            lx = px + 18 + k * lp
            nx = lx + lp
            segs = {k: [(top + 16, y1), (y2, top + ph - 16)]}
            lines(px, segs)
            # intended cut (dashed) and the actual, shifted cut
            s.rect(lx - cm, y1, lw + 2 * cm, y2 - y1, fill="none", stroke="red", sw=1.0, dash="4 3", rx=3)
            cut_x0, cut_x1 = lx - cm + d, lx - cm + d + lw + 2 * cm
            s.rect(cut_x0, y1, cut_x1 - cut_x0, y2 - y1, "cut", sw=1.2, rx=3)
            # consequences: the sliver of the intended line the shifted cut misses, the notch it bites from the neighbour
            sliver = min(cut_x0 - lx, lw)
            notch = max(0, min(cut_x1, nx + lw) - nx)
            assert sliver > 0 and notch > 0, (sliver, notch)
            s.rect(lx, y1, sliver, y2 - y1, "spacer", sw=0.9, rx=1)
            if notch > 0:
                s.rect(nx, y1, notch, y2 - y1, fill="surface", stroke="none")
                s.rect(nx, y1, notch, y2 - y1, fill="none", stroke="red", sw=1.0, dash="3 2")
            s.dim_h(lx - cm, lx - cm + d, y1 - 14, "Δ")
            s.text(lx + lw / 2 + d, y1 - 30, "edge-placement error Δ", size=K.SMALL, color="ink", halo=True)
            s.line(lx + 2, y1 + 15, lx - 30, y1 + 60, "ink2", 1.0)
            s.text(lx - 30, y1 + 74, "sliver left", size=K.SMALL, color="ink2", halo=True)
            s.line(nx + 3, y2 - 4, nx + 40, y2 + 40, "ink2", 1.0)
            s.text(nx + 40, y2 + 54, "neighbour clipped", size=K.SMALL, color="ink2", halo=True)
        s.text(px + pw / 2, top + ph + 22, notes[i], size=K.LABEL, color="ink2")
    s.swatch_legend(
        150,
        390,
        [
            ("spacer", "spacer-defined line", "rect"),
            ("cut", "cut / block mask opening", "rect"),
            ("via", "via mask opening", "rect"),
            ("red", "intended position", "dash"),
        ],
    )
    return s


# ---------------------------------------------------------------------------
# Registry
# ---------------------------------------------------------------------------
FIGURES = {
    "transistor-evolution": fig_transistor_evolution,
    "standard-cell": fig_standard_cell,
    "sadp-flow": fig_sadp,
    "sadp-flow-anim": fig_sadp_anim,
    "saqp-flow": fig_saqp,
    "pitch-walk": fig_pitch_walk,
    "lele-overlay": fig_lele_overlay,
    "backside-power": fig_backside_power,
    "anamorphic-field": fig_anamorphic_field,
    "pupil-orders": fig_pupil_orders,
    "stochastic-defects": fig_stochastic_defects,
    "euv-mask-stack": fig_euv_mask_stack,
    "nanosheet-flow": fig_nanosheet_flow,
    "cut-mask": fig_cut_mask,
}


def write(name: str) -> list[Path]:
    fn = FIGURES[name]
    paths = K.write_pair(name, lambda t: fn(t).render(), OUT)
    for p in paths:
        print("wrote", p.relative_to(registry.REPO))
    return paths


def _gen(name: str):
    def gen() -> list[Path]:
        return write(name)

    gen.__name__ = f"gen_{name.replace('-', '_')}"
    return gen


MP_ROWS = {"multiple patterning (SADP/SAQP)": "Multiple patterning (LELE, SADP, SAQP)",
           "multiple patterning (LELE)": "Multiple patterning (LELE, SADP, SAQP)"}
MP_TRAILER = "<em>Model: multiple patterning (SADP/SAQP) 🔶 (geometric).</em>"

SPECS = [
    dict(
        name="transistor-evolution", kind="schematic", pages=("nodes/index.md",),
        alt="Four schematic transistor cross-sections cut across the channel: planar MOSFET with the gate on top; "
            "FinFET with the gate over three sides of two fins; gate-all-around nanosheet with three stacked sheets "
            "fully surrounded by gate; CFET with nFET sheets stacked under pFET sheets inside one gate.",
        caption="<strong>Each architecture gives the gate more control over the channel.</strong> Planar → FinFET → "
                "nanosheet, then CFET stacks the two transistor types to save area. "
                "<em>highuvlith does not model transistors. Schematic, not to scale.</em>",
    ),
    dict(
        name="standard-cell", kind="schematic", pages=("nodes/index.md",),
        alt="Top view of a schematic standard cell: vertical gate lines at the contacted poly pitch, horizontal "
            "routing tracks at the minimum metal pitch, power rails top and bottom, pFET and nFET fins; dimension "
            "arrows mark CPP, MMP, a six-track cell height and a four-CPP cell width.",
        caption="<strong>A standard cell is a whole number of gate pitches wide and routing tracks tall.</strong> "
                "CPP and MMP set logic density; the cell height is the track count times the metal pitch. "
                "<em>Schematic, not to scale.</em>",
    ),
    dict(
        name="sadp-flow", kind="schematic", pages=("nodes/22-to-10nm.md",),
        alt="Five cross-section steps of self-aligned double patterning: mandrels at pitch P; conformal spacer film; "
            "etch-back leaves spacers on the mandrel sidewalls; mandrel pull leaves two spacer lines per mandrel at "
            "pitch P/2; transfer into the target layer.",
        caption="<strong>Self-aligned double patterning.</strong> One lithography step prints mandrels at pitch P; a "
                "conformal spacer, an etch-back and a mandrel pull leave lines at P/2. " + MP_TRAILER,
        extra={"model_rows": MP_ROWS},
    ),
    dict(
        name="sadp-flow-anim", kind="anim", animated="svg",
        pages=("nodes/22-to-10nm.md", "history/immersion-multipatterning.md"),
        alt="Animated cross-section of self-aligned double patterning: mandrels printed at pitch P, a conformal "
            "spacer film, an anisotropic etch-back, the mandrel pull and the transfer into the target layer, one "
            "state fading into the next; the still shows the five steps as rows.",
        caption="<strong>Self-aligned double patterning, step by step.</strong> Ideal conformal spacers; no deposition "
                "or etch physics. " + MP_TRAILER,
        extra={"model_rows": MP_ROWS, "duration_ms": 13000, "loops": 3},
    ),
    dict(
        name="saqp-flow", kind="schematic", pages=("nodes/22-to-10nm.md",),
        alt="Four cross-section steps of self-aligned quadruple patterning: mandrels at pitch P; first spacers at P/2 "
            "after the mandrel pull; second spacers formed on the first spacers; first spacers removed, leaving lines "
            "at P/4.",
        caption="<strong>Self-aligned quadruple patterning.</strong> The spacer step repeated on the first spacers "
                "divides the lithographic pitch by four. " + MP_TRAILER,
        extra={"model_rows": MP_ROWS},
    ),
    dict(
        name="pitch-walk", kind="schematic", pages=("nodes/22-to-10nm.md",),
        alt="Three rows of SADP spacer lines. Nominal: core and gap spaces equal. Mandrel CD larger by delta: core "
            "spaces widen by delta and gap spaces shrink by delta. Spacer thicker by delta: lines widen and gap spaces "
            "shrink by two delta while core spaces stay the same.",
        caption="<strong>Pitch walk.</strong> A mandrel CD error widens the core spaces and narrows the gaps by the "
                "same amount; a spacer-thickness error narrows only the gaps, by twice the error. " + MP_TRAILER,
        extra={"model_rows": MP_ROWS},
    ),
    dict(
        name="lele-overlay", kind="schematic", pages=("nodes/22-to-10nm.md",),
        alt="Two rows of interleaved lines labelled A and B. With perfect overlay the spaces between A and B lines are "
            "equal; with an overlay error delta on mask B the spaces alternate between s minus delta and s plus delta.",
        caption="<strong>LELE double patterning.</strong> Each exposure prints twice the final pitch, but the second "
                "mask's overlay error goes straight into the spaces. "
                "<em>Model: multiple patterning (LELE) 🔶 (two exposures, sub-pixel overlay).</em>",
        extra={"model_rows": MP_ROWS},
    ),
    dict(
        name="backside-power", kind="schematic", pages=("nodes/3nm-and-2nm.md",),
        alt="Two schematic cross-sections. Front-side power delivery: power and signal wires share one metal stack "
            "above the transistors. Backside power delivery: the wafer is thinned, a power network on the back "
            "connects through nano through-silicon vias and the front metal stack carries only signals.",
        caption="<strong>Front-side versus backside power delivery.</strong> With the power network on the back of a "
                "thinned wafer, the front metal stack — and its lithography — carries only signals. "
                "<em>Schematic, not to scale.</em>",
    ),
    dict(
        name="anamorphic-field", kind="schematic", pages=("nodes/angstrom-era.md",),
        alt="A 104 by 132 mm reticle image area maps to a 26 by 33 mm full field at 4x/4x reduction for 0.33 NA, and "
            "to a 26 by 16.5 mm half field at 4x/8x reduction for 0.55 NA, where a larger die needs a second, "
            "stitched exposure.",
        caption="<strong>Anamorphic reduction halves the field.</strong> 4× across the scan and 8× along it lets "
                "High-NA optics keep today's reticles at the price of a 26 × 16.5 mm half field; field sizes from the "
                "IRDS 2023 lithography chapter. <em>Schematic, not to scale.</em>",
    ),
    dict(
        name="pupil-orders", kind="schematic", pages=("nodes/litho-math.md",),
        alt="Three pupil diagrams: on-axis illumination leaves the first diffraction orders outside the pupil; dipole "
            "illumination puts the zeroth and minus-first orders inside; at the limit the two orders sit at opposite "
            "pupil edges, spaced 2 NA, which gives k1 = 0.25.",
        caption="<strong>Where k₁ = 0.25 comes from.</strong> Two-beam imaging with the zeroth and a first diffraction "
                "order at opposite edges of the pupil; pupil radius = NA, orders spaced λ/p. "
                "<em>Closed form and geometry; not the engine. No model badge.</em>",
    ),
    dict(
        name="stochastic-defects", kind="schematic", pages=("nodes/7-and-5nm.md",),
        alt="Illustrative sketch of stochastic printing failures: a contact-hole array with varying hole sizes, a "
            "missing contact and two merged contacts; dense lines with rough edges, a microbridge between two lines "
            "and a break in one line.",
        caption="<strong>Stochastic failure modes.</strong> Size variation, missing and merged contacts, rough edges, "
                "microbridges and breaks. <em>Schematic, not simulated; sizes and positions are illustrative.</em>",
    ),
    dict(
        name="euv-mask-stack", kind="schematic", pages=("nodes/7-and-5nm.md", "history/euv.md"),
        alt="Cross-section of an EUV mask: a low-expansion substrate, 40 to 50 Mo/Si bilayers under a Ru cap, a "
            "tantalum-based absorber on top, the 6-degree chief ray with the absorber's shadow hatched beside it, and "
            "a pellicle membrane above that the light crosses twice.",
        caption="<strong>An EUV mask is a mirror with a patterned absorber on top.</strong> 40–50 Mo/Si bilayers under "
                "a Ru cap reflect about 70 % at 13.5 nm; the 6° chief ray makes the tens-of-nanometre absorber cast a "
                "shadow, and a pellicle's transmission counts twice. <em>Schematic, not to scale. The engine's mask is "
                "a thin transmittance map: no mask 3-D, pellicles not modelled.</em>",
    ),
    dict(
        name="nanosheet-flow", kind="schematic", pages=("nodes/3nm-and-2nm.md",),
        alt="Four cross-section steps of a nanosheet transistor: a Si/SiGe superlattice, the stack patterned into a "
            "ribbon, the SiGe released to leave suspended Si sheets, and the gate wrapped around every sheet; an inset "
            "contrasts whole fins with sheets of any width.",
        caption="<strong>How a nanosheet transistor is built.</strong> A Si/SiGe superlattice is patterned into "
                "ribbons, the SiGe is released and the gate wraps every sheet; unlike whole fins, the sheet width is a "
                "design variable. <em>highuvlith does not model transistors. Schematic, not to scale.</em>",
    ),
    dict(
        name="cut-mask", kind="schematic", pages=("nodes/22-to-10nm.md",),
        alt="Three top views: a regular spacer grating; cut masks removing line segments and via masks connecting "
            "lines to the level above; a cut shifted by a few nanometres that clips the neighbouring line and leaves "
            "a sliver, with the edge-placement error dimensioned.",
        caption="<strong>Cut, block and via masks carve the circuit out of a spacer grating.</strong> A cut that lands "
                "a few nanometres off clips the neighbouring line or leaves a sliver: cut overlay is part of the "
                "edge-placement budget. <em>Schematic, not to scale.</em>",
    ),
]

for _spec in SPECS:
    registry.register(registry.AssetSpec(
        group="nodes", subdir="nodes", fmt="svg", generator=_gen(_spec["name"]), platform_independent=True,
        **_spec,
    ), module="make_node_schematics.py")


def check() -> list[str]:
    """Sizes and hygiene of every file this module writes (the registry's rules, applied locally)."""
    errors: list[str] = []
    for name in FIGURES:
        for theme in ("light", "dark"):
            p = OUT / f"{name}-{theme}.svg"
            if not p.exists():
                errors.append(f"{p.name}: missing")
                continue
            kb = p.stat().st_size / 1024
            cap = 120 if name.endswith("-anim") else 60
            if kb > cap:
                errors.append(f"{p.name}: {kb:.0f} KB over the {cap} KB cap")
            text = p.read_text(encoding="utf-8")
            if "<a " in text or "<script" in text:
                errors.append(f"{p.name}: contains <a> or <script>")
            if "@keyframes" in text and "prefers-reduced-motion" not in text:
                errors.append(f"{p.name}: animated without the reduced-motion rule")
    return errors


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    if "--check" in argv:
        errs = check()
        for e in errs:
            print("error:", e)
        print(f"make_node_schematics --check: {len(errs)} error(s)")
        return 1 if errs else 0
    names = [n for n in FIGURES if not argv or any(a in n for a in argv)]
    OUT.mkdir(parents=True, exist_ok=True)
    for name in names:
        write(name)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
