#!/usr/bin/env python3
"""P4b · P5b · P6 · P7a · P8b · P9a — the process-page mechanism schematics (DESIGN §5.3, V2c).

Scripted SVG through ``svgkit`` (standard library only — no plotting library is imported
anywhere in this module, so every asset is ``platform_independent=True`` and the nightly
can regenerate and diff it), light/dark pairs written to ``docs/assets/images/diagrams/``.
Each one is the *Mechanism* slot of its page's Physics Strip; the engine figure beside it
is drawn by ``fig_deep.py`` / ``fig_deep_b.py``. None of them carries a model badge: they
draw geometry and closed forms, so every caption ends in "Schematic, not to scale."

* ``thinfilm-layer-stack`` — superstrate / top coat / resist / BARC / substrate with the
  forward and backward waves, q_j, η_j, U₀ = 1 + r, V₀ = η₀(1 − r), the ordered matrix
  product and the standing-wave intensity of period λ/2n inside the resist
  (``processes/thin-film.md``);
* ``resist-chain-schematic`` — photon → PAC/PAG → acid → bake diffusion → deprotection →
  dissolution rate as one row, with the depth-averaged 2-D path and the z-resolved
  volumetric path branching at exposure and rejoining at development
  (``processes/resist-models.md``);
* ``grayscale-mechanism`` — transmittance → dose → contrast curve → height, one row
  (``processes/grayscale.md``);
* ``interference-beam-geometry`` — three x–z projections (two-beam, hexagonal, umbrella)
  with the in-plane E arrows and the transverse wavevector dimensioned on both sides of
  the air/resist interface so its conservation is visible
  (``processes/interference-volumetric.md``);
* ``talbot-schemes`` — fixed gap · DTL · ATL · two-grating EUV-IL (``processes/talbot.md``);
* ``liga-geometry`` — bending magnet → Be window → filters → Ti/Au mask → gap → thick
  PMMA, the scan arrow, "no optic: 1:1 shadow", d = g + z and the spectrum hardening
  station by station (``processes/liga-deep-xray.md``, ``future/beyond-euv.md``).

    python docs/figures/diagrams/make_process_schematics.py            # write all six pairs
    python docs/figures/diagrams/make_process_schematics.py talbot     # only the names matching
    python docs/figures/diagrams/make_process_schematics.py --check    # geometry self-checks

The angles drawn are computed (Snell's law for the interference panels, the projected
k-vectors for the hexagonal and umbrella geometries); ``--check`` re-derives them.
"""

from __future__ import annotations

import math
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import registry  # noqa: E402
import svgkit as K  # noqa: E402

OUT = registry.IMAGES / "diagrams"
GENERATOR = "docs/figures/diagrams/make_process_schematics.py"
MODULE = "make_process_schematics.py"

# Text sizes: the Physics Strip shows these at half the column width, so primary labels
# sit well above the 12.5-unit floor and strokes are 1.5–2.5 units.
T_TITLE = 16
T_LABEL = 14
T_SMALL = 13
T_MIN = 12.5

TRAILER = "<em>Schematic, not to scale.</em>"


# ---------------------------------------------------------------------------
# Rich text: ``_{sub}`` / ``^{sup}`` (or a single character) rendered as tspans
# ---------------------------------------------------------------------------

_SUBSUP = re.compile(r"([_^])(\{[^}]*\}|[^\s_^{}])")


def plain(text: str) -> str:
    """The markup stripped (for width estimates)."""
    return _SUBSUP.sub(lambda m: m.group(2).strip("{}"), text)


def rich(s: K.Svg, x: float, y: float, text: str, size: float = T_LABEL, anchor: str = "middle",
         color: str = "ink", weight: int = 400, italic: bool = False, opacity: float | None = None) -> None:
    """``s.text`` with real subscripts and superscripts: ``q_j``, ``D_{th}``, ``e^{−αd}``."""
    if size < K.MIN_TEXT:
        raise ValueError(f"text {text!r} at {size} units is below the {K.MIN_TEXT}-unit floor")
    parts: list[tuple[str, int]] = []
    pos = 0
    for m in _SUBSUP.finditer(text):
        if m.start() > pos:
            parts.append((text[pos:m.start()], 0))
        body = m.group(2)
        parts.append((body[1:-1] if body.startswith("{") else body, -1 if m.group(1) == "_" else 1))
        pos = m.end()
    if pos < len(text):
        parts.append((text[pos:], 0))
    small = size * 0.76
    shift = size * 0.27
    spans = []
    pending = 0.0
    for t, kind in parts:
        dy = pending
        if kind == 0:
            attrs = f' dy="{K.fmt(dy)}"' if dy else ""
            spans.append(f"<tspan{attrs}>{K.esc(t)}</tspan>")
            pending = 0.0
        else:
            d = (shift if kind < 0 else -shift * 1.3) + dy
            spans.append(f'<tspan dy="{K.fmt(d)}" font-size="{K.fmt(small)}">{K.esc(t)}</tspan>')
            pending = -(shift if kind < 0 else -shift * 1.3)
    style = ' font-style="italic"' if italic else ""
    style += f' opacity="{K.fmt(opacity)}"' if opacity is not None else ""
    s.add(f'<text x="{K.fmt(x)}" y="{K.fmt(y)}" font-size="{K.fmt(size)}" text-anchor="{anchor}" '
          f'fill="{s.c(color)}" font-weight="{weight}"{style}>{"".join(spans)}</text>')


# ---------------------------------------------------------------------------
# Shared drawing helpers
# ---------------------------------------------------------------------------


def _dot_te(s: K.Svg, x: float, y: float, color: str, r: float = 6.0) -> None:
    """E out of the drawing plane (TE, ∥ y): a circle with a centre dot."""
    s.circle(x, y, r, fill="surface", stroke=color, sw=1.6)
    s.circle(x, y, 1.9, fill=color, stroke="none")


def _inplane_e(s: K.Svg, x: float, y: float, ux: float, uy: float, color: str, half: float = 11.0) -> None:
    """E in the drawing plane: a double-headed arrow along the unit direction (ux, uy)."""
    s.line(x - ux * half, y - uy * half, x + ux * half, y + uy * half, color, 1.6, arrow="both")


def _beam(s: K.Svg, x1: float, y1: float, x2: float, y2: float, color: str, *, sw: float = 2.2,
          arrow: str | None = "end", dash: str | None = None, opacity: float | None = None) -> None:
    s.line(x1, y1, x2, y2, color, sw, dash=dash, arrow=arrow, opacity=opacity)


def _arc(s: K.Svg, cx: float, cy: float, r: float, a0_deg: float, a1_deg: float, color: str = "ink2",
         sw: float = 1.1) -> None:
    """A circular arc from a0 to a1 (degrees, SVG orientation: 0° = +x, 90° = +y)."""
    a0, a1 = math.radians(a0_deg), math.radians(a1_deg)
    x0, y0 = cx + r * math.cos(a0), cy + r * math.sin(a0)
    x1, y1 = cx + r * math.cos(a1), cy + r * math.sin(a1)
    large = 1 if abs(a1_deg - a0_deg) > 180 else 0
    sweep = 1 if a1_deg > a0_deg else 0
    s.path(f"M{K.fmt(x0)},{K.fmt(y0)} A{K.fmt(r)},{K.fmt(r)} 0 {large},{sweep} {K.fmt(x1)},{K.fmt(y1)}",
           stroke=color, sw=sw)


def _grating(s: K.Svg, x: float, y: float, w: float, period: float, duty: float = 0.5, h: float = 9.0,
             membrane: bool = True) -> list[float]:
    """Absorber bars on a membrane line; returns the bar centres (x)."""
    centres = []
    if membrane:
        s.line(x, y + h, x + w, y + h, "axis", 1.0)
    bar = period * duty
    xx = x + (period - bar) / 2
    while xx + bar <= x + w + 0.5:
        s.rect(xx, y, bar, h, "absorber", sw=0.6)
        centres.append(xx + bar / 2)
        xx += period
    return centres


def _spectrum_glyph(s: K.Svg, x: float, y: float, w: float, h: float, *, peak: float, cut: float,
                    color: str) -> None:
    """A qualitative bending-magnet spectrum (photons per ln E vs log E) that hardens as
    ``cut`` (the low-energy absorption edge, 0–1 of the width) and ``peak`` (0–1) move right."""
    s.line(x, y + h, x + w, y + h, "axis", 1.0, arrow="end")
    pts = []
    n = 40
    for i in range(n + 1):
        u = i / n
        env = math.exp(-((u - peak) ** 2) / (2 * 0.22 ** 2)) if u >= peak else (u / peak) ** 1.6
        cutoff = 1.0 / (1.0 + math.exp(-(u - cut) * 28))
        pts.append((x + u * w, y + h - env * cutoff * (h - 4)))
    s.polyline(pts, color, 2.0)


def _slab(s: K.Svg, x: float, y: float, w: float, h: float, *, stripes: int = 0) -> None:
    """A resist slab pictogram; ``stripes`` > 0 draws standing-wave bands inside it."""
    s.rect(x, y, w, h, "resist", sw=0.9)
    if stripes:
        band = h / (2 * stripes)
        for i in range(stripes):
            s.rect(x + 1, y + (2 * i + 0.5) * band, w - 2, band, fill=s.tint("orange", 0.55), stroke="none")
    s.rect(x, y + h, w, 6, "substrate", sw=0.8)


def _dim_v(s: K.Svg, x: float, y1: float, y2: float, label: str, side: str = "right", size: float = T_SMALL) -> None:
    """``svgkit.dim_v`` with a rich-text label."""
    s.dim_v(x, y1, y2, "", side=side, size=size)
    if side == "right":
        rich(s, x + 10, (y1 + y2) / 2 + size / 3, label, size=size, anchor="start", color="ink2")
    else:
        rich(s, x - 10, (y1 + y2) / 2 + size / 3, label, size=size, anchor="end", color="ink2")


def _dim_h(s: K.Svg, x1: float, x2: float, y: float, label: str, above: bool = True, size: float = T_SMALL) -> None:
    s.dim_h(x1, x2, y, "", size=size, above=above)
    ty = y - 8 if above else y + size + 4
    rich(s, (x1 + x2) / 2, ty, label, size=size, color="ink2")


# ---------------------------------------------------------------------------
# P4b — thinfilm-layer-stack
# ---------------------------------------------------------------------------

STACK_LAYERS = [  # (name, index/thickness label, height in units, material, admittance line)
    ("superstrate", "n_0 (air · water)", 90, None, "η_0"),
    ("top coat", "n_1, d_1", 36, "glass", "q_1, η_1 · δ_1 = k_0q_1d_1"),
    ("resist", "n_2, d_2", 176, "resist", "q_2, η_2 · δ_2 = k_0q_2d_2"),
    ("BARC", "n_3, d_3", 44, "mandrel", "q_3, η_3 · δ_3 = k_0q_3d_3"),
    ("substrate", "n_s", 96, "substrate", "η_s"),
]
#: Refraction angles (degrees from the normal) drawn in the five media — illustrative, decreasing
#: into denser media; the stack is not drawn to any particular index.
STACK_ANGLES = (40.0, 30.0, 24.0, 20.0, 14.0)


def draw_thinfilm(theme: str) -> str:
    W, H = 960, 540
    s = K.Svg(W, H, theme, "Thin-film stack: forward and backward waves, admittances and the standing wave",
              THINFILM_ALT, generator=GENERATOR)
    x0, x1 = 44, 500
    y = 56
    bounds = []
    for name, thick, h, mat, _adm in STACK_LAYERS:
        if mat:
            s.rect(x0, y, x1 - x0, h, mat, sw=0.0)
        else:
            s.rect(x0, y, x1 - x0, h, fill=s.tint("blue", 0.05), stroke="none")
        bounds.append((y, y + h))
        s.text(x0 + 10, y + 19, name, size=T_LABEL, anchor="start", color="ink", weight=600)
        if h < 48:  # too thin for a second line: the index label sits beside the name
            rich(s, x0 + 10 + K.text_width(name, T_LABEL, 600) + 12, y + 19, thick, size=T_SMALL, anchor="start", color="ink2")
        else:
            rich(s, x0 + 10, y + 36, thick, size=T_SMALL, anchor="start", color="ink2")
        y += h
    y_top, y_bot = bounds[0][0], bounds[-1][1]
    for yb, _ in bounds[1:]:
        s.line(x0, yb, x1, yb, "ink2", 1.3)
    s.rect(x0, y_top, x1 - x0, y_bot - y_top, fill="none", stroke="axis", sw=1.0)

    # the z axis beside the stack
    s.line(x1 + 8, bounds[1][0], x1 + 8, bounds[3][1], "ink2", 1.1, arrow="end")
    s.text(x1 + 14, bounds[3][1] - 2, "z", size=T_SMALL, anchor="start", color="ink2", italic=True)

    # the incident and reflected rays in the superstrate
    xi = 334
    yi = bounds[1][0]  # the stack top
    L = 82
    a0 = math.radians(STACK_ANGLES[0])
    s.line(xi, bounds[0][0] + 6, xi, yi, "axis", 0.9, dash="3 3")  # the normal
    _beam(s, xi - L * math.sin(a0), yi - L * math.cos(a0), xi, yi, "blue", sw=2.4)
    _beam(s, xi, yi, xi + L * math.sin(a0), yi - L * math.cos(a0), "orange", sw=2.0)
    _arc(s, xi, yi, 30, 270, 270 - STACK_ANGLES[0], "ink2")
    rich(s, xi - 44, yi - 40, "θ_0", size=T_SMALL, color="ink2")
    s.text(xi - L * math.sin(a0) - 6, yi - L * math.cos(a0) - 6, "incident (TE / TM)", size=T_SMALL, anchor="end", color="blue")
    s.text(xi + L * math.sin(a0) + 8, yi - L * math.cos(a0) - 6, "reflected r", size=T_SMALL, anchor="start", color="orange")
    rich(s, x0 + 10, bounds[0][0] + 56, "s = n_0 sin θ_0 is invariant", size=T_SMALL, anchor="start", color="ink2", italic=True)
    rich(s, x1 - 8, yi - 12, "U_0 = 1 + r   V_0 = η_0(1 − r)", size=T_LABEL, anchor="end", color="ink", weight=600)
    s.circle(xi, yi, 4.2, fill="ink", stroke="surface", sw=1.2)

    # forward (+) and backward (−) waves in the three film layers, then the transmitted wave
    xf = xi
    for j in (1, 2, 3):
        ya, yb = bounds[j]
        a = math.radians(STACK_ANGLES[j])
        dx = (yb - ya) * math.tan(a)
        _beam(s, xf, ya, xf + dx, yb, "blue", sw=2.2)
        xb = xf + dx + 26
        _beam(s, xb, yb, xb + dx, ya, "orange", sw=1.8, opacity=0.9)
        s.text(xf + dx / 2 - 10, (ya + yb) / 2 + 5, "+", size=T_LABEL, anchor="end", color="blue", weight=600)
        s.text(xb + dx / 2 + 10, (ya + yb) / 2 + 5, "−", size=T_LABEL, anchor="start", color="orange", weight=600)
        xf += dx
    ya, yb = bounds[4]
    a = math.radians(STACK_ANGLES[4])
    _beam(s, xf, ya, xf + (yb - ya - 18) * math.tan(a), yb - 18, "blue", sw=2.2, dash="7 5", opacity=0.75)
    s.text(xf + 30, yb - 24, "transmitted t (Beer–Lambert decay)", size=T_SMALL, anchor="start", color="blue")

    # per-layer admittance labels and the ordered matrix product
    xl = x1 + 30
    for i, ((_name, _thick, _h, _mat, adm), (ya, yb)) in enumerate(zip(STACK_LAYERS, bounds)):
        rich(s, xl, (yb - 8) if i == 0 else (ya + yb) / 2 + 5, adm, size=T_SMALL, anchor="start", color="ink2")
    rich(s, xl, bounds[0][0] + 20, "q_j = √(n_j² − s²), Im q_j ≥ 0", size=T_SMALL, anchor="start", color="ink2")
    rich(s, xl, bounds[0][0] + 38, "η_j = q_j (TE) · n_j²/q_j (TM)", size=T_SMALL, anchor="start", color="ink2")
    rich(s, xl, bounds[0][0] + 56, "M_j from δ_j and η_j", size=T_SMALL, anchor="start", color="ink2")
    bx = xl + 160
    s.line(bx, bounds[1][0] + 2, bx, bounds[3][1] - 2, "ink2", 1.2)
    s.line(bx, bounds[1][0] + 2, bx - 6, bounds[1][0] + 2, "ink2", 1.2)
    s.line(bx, bounds[3][1] - 2, bx - 6, bounds[3][1] - 2, "ink2", 1.2)
    rich(s, bx + 8, (bounds[1][0] + bounds[3][1]) / 2 - 2, "M = M_1M_2M_3", size=T_LABEL, anchor="start", color="ink", weight=600)
    s.text(bx + 8, (bounds[1][0] + bounds[3][1]) / 2 + 16, "top to bottom", size=T_SMALL, anchor="start", color="ink2")

    # right: the in-film intensity S(z) against depth, aligned with the stack
    px0, pw = 836, 86
    s.eyebrow(px0 - 30, bounds[0][0] + 20, "in-film intensity", size=T_MIN, color="hue:simulator")
    rich(s, px0 - 30, bounds[0][0] + 38, "S(z) = (n_r/n_0)·|E(z)|²", size=T_SMALL, anchor="start", color="ink2")
    s.line(px0, bounds[1][0], px0, bounds[3][1], "axis", 1.0)
    s.rect(px0, bounds[2][0], pw, bounds[2][1] - bounds[2][0], fill=s.tint("orange", 0.10), stroke="none")
    pts = []
    period = (bounds[2][1] - bounds[2][0]) / 3.2  # ≈ λ/2n: a 150 nm resist holds ~3.2 periods at 47.6 nm
    ya, yb = bounds[1][0], bounds[3][1]
    n = 160
    for i in range(n + 1):
        z = ya + (yb - ya) * i / n
        decay = math.exp(-(z - ya) / 420)
        val = 1.0 + 0.55 * decay * math.cos(2 * math.pi * (z - bounds[2][1]) / period)
        pts.append((px0 + 6 + val * 0.5 * pw, z))
    s.polyline(pts, "blue", 2.0)
    zm = bounds[2][1] - period * 2
    s.dim_v(px0 + pw + 12, zm, zm + period, "", side="right")
    s.text(px0 + pw + 12, zm + period + 17, "λ/2n", size=T_SMALL, color="ink2")
    s.text(px0 - 30, bounds[3][1] + 20, "standing wave;", size=T_SMALL, anchor="start", color="ink2")
    rich(s, px0 - 30, bounds[3][1] + 37, "its mean swings with d_2", size=T_SMALL, anchor="start", color="ink2")

    rich(s, x0, H - 14, "R = |r|²   T = Re η_s |t|² / Re η_0   ·   fields e^{i(kz − ωt)}, n_j = n + ik with k ≥ 0",
         size=T_SMALL, anchor="start", color="ink2")
    return s.render()


THINFILM_ALT = ("Cross-section of a superstrate, top coat, resist, BARC and substrate stack with the incident, "
                "reflected and transmitted rays, a forward and a backward wave in every layer labelled by its "
                "q and η, the stack-top fields U₀ = 1 + r and V₀ = η₀(1 − r), and the standing wave of period "
                "λ/2n in the resist.")
THINFILM_CAPTION = (
    "<strong>Mechanism.</strong> Each layer j carries a forward and a backward wave; its normal wavevector "
    "q_j = √(n_j² − s²) and tilted admittance η_j fill one 2×2 matrix M_j, and the ordered product M = Π M_j gives r "
    "and t from the superstrate and substrate admittances. The stack-top fields U₀ = 1 + r and V₀ = η₀(1 − r) "
    "propagated downward give the in-film intensity S(z), whose standing wave has period λ/2n in the resist. "
    + TRAILER
)


# ---------------------------------------------------------------------------
# P5b — resist-chain-schematic
# ---------------------------------------------------------------------------

CHAIN_STEPS = [  # (title, line 1, line 2)
    ("Photon", "dose D × I(x, y)", "mJ/cm² × relative"),
    ("PAC / PAG", "m = exp(−C·D·I)", "Dill kinetics"),
    ("Acid", "h_0 = 1 − m (CAR)", "or PAC m (DNQ)"),
    ("Bake", "Gaussian σ = L_D", "or h, q react–diffuse"),
    ("Deprotection", "protected m after", "the bake"),
    ("Dissolution", "R(m): Mack or", "threshold"),
]


def _step_icon(s: K.Svg, i: int, cx: float, cy: float) -> None:
    """A small pictogram per chain step, drawn above the box text."""
    if i == 0:  # a photon: a short wave with an arrow
        pts = [(cx - 18 + k * 2, cy - 5 * math.sin(k * 0.9)) for k in range(0, 15)]
        s.polyline(pts, "blue", 1.8)
        s.line(cx + 10, cy, cx + 20, cy, "blue", 1.8, arrow="end")
    elif i == 1:  # molecules: filled dots turning hollow
        for k, hollow in enumerate((False, False, True, True)):
            x = cx - 16 + k * 11
            if hollow:
                s.circle(x, cy, 4, fill="none", stroke="orange", sw=1.5)
            else:
                s.circle(x, cy, 4, fill="orange", stroke="none")
    elif i == 2:  # acid: a cluster of small plus marks
        for dx, dy in ((-11, -4), (0, 3), (11, -3)):
            s.line(cx + dx - 4, cy + dy, cx + dx + 4, cy + dy, "orange", 1.6)
            s.line(cx + dx, cy + dy - 4, cx + dx, cy + dy + 4, "orange", 1.6)
    elif i == 3:  # diffusion: a sharp profile becoming a wide Gaussian
        pts = [(cx - 20 + k, cy + 6 - 11 * math.exp(-((k - 20) ** 2) / 60.0)) for k in range(0, 41)]
        s.polyline(pts, "green", 1.8)
        s.line(cx - 2, cy - 7, cx - 2, cy + 6, "green", 1.4, dash="2 2")
    elif i == 4:  # deprotection: a slab with a lighter window where the acid was
        s.rect(cx - 20, cy - 7, 40, 14, fill=s.tint("orange", 0.45), stroke="none")
        s.rect(cx - 8, cy - 7, 16, 14, fill=s.tint("orange", 0.12), stroke="none")
    else:  # dissolution: a rate curve R(m)
        pts = [(cx - 20 + k, cy + 7 - 14 / (1 + math.exp(-(k - 20) / 3.0))) for k in range(0, 41)]
        s.polyline(pts, "violet", 1.8)


def draw_resist_chain(theme: str) -> str:
    W, H = 960, 490
    s = K.Svg(W, H, theme, "The resist chain: one chemistry, a 2-D and a volumetric path",
              RESIST_ALT, generator=GENERATOR)
    n = len(CHAIN_STEPS)
    bw, bh, gap = 130, 100, 20
    x0 = (W - (n * bw + (n - 1) * gap)) / 2
    yc = H / 2
    boxes = []
    s.eyebrow(W / 2, yc - bh / 2 - 12, "shared chemistry — Dill · PEB · CAR · Mack", size=T_MIN, color="hue:simulator",
              anchor="middle")
    for i, (title, l1, l2) in enumerate(CHAIN_STEPS):
        x = x0 + i * (bw + gap)
        y = yc - bh / 2
        s.rect(x, y, bw, bh, fill=s.tint("hue:simulator", 0.06), stroke="axis", sw=1.0, rx=7)
        _step_icon(s, i, x + bw / 2, y + 20)
        s.text(x + bw / 2, y + 48, title, size=T_LABEL, color="ink", weight=600)
        rich(s, x + bw / 2, y + 66, l1, size=T_SMALL, color="ink2")
        s.text(x + bw / 2, y + 82, l2, size=T_SMALL, color="ink2")
        boxes.append((x, y))
        if i:
            s.line(x - gap + 2, yc, x - 2, yc, "ink2", 1.5, arrow="end")

    def lane_box(x, y, w, h, title, lines, hue):
        s.rect(x, y, w, h, fill=s.tint(hue, 0.05), stroke=hue, sw=1.2, rx=7, dash="5 4")
        s.text(x + 12, y + 20, title, size=T_LABEL, anchor="start", color="ink", weight=600)
        for k, ln in enumerate(lines):
            rich(s, x + 12, y + 40 + k * 17, ln, size=T_SMALL, anchor="start", color="ink2")

    lw, lh = 330, 96
    top_y, bot_y = 24, H - lh - 36
    lx_in, lx_out = x0 - 10, W - x0 - lw + 10
    # the 2-D path (above the chain)
    lane_box(lx_in, top_y, lw, lh, "2-D path — depth-averaged exposure",
             ["I_{eff} = I_{aer} · (1 − e^{−αd}) / αd", "one Beer–Lambert coupling scalar,", "bleaching frozen at m = 1"], "hue:nodes")
    _slab(s, lx_in + lw - 78, top_y + 36, 60, 30)
    s.text(lx_in + lw - 48, top_y + 86, "one scalar", size=T_MIN, color="ink2")
    lane_box(lx_out, top_y, lw, lh, "2-D develop() — centre row, vertical etch",
             ["h(x) = max(d − R(m(x, y_c))·t, 0)", "a 1-D cross-section: no lateral front,", "no sidewall, no depth variation"], "hue:nodes")
    px, py = lx_out + lw - 80, top_y + 44
    s.polyline([(px, py), (px + 16, py), (px + 16, py + 22), (px + 42, py + 22), (px + 42, py), (px + 60, py)], "hue:nodes", 1.8)
    s.rect(px, py + 24, 60, 6, "substrate", sw=0.8)
    # the volumetric path (below the chain)
    lane_box(lx_in, bot_y, lw, lh, "Volumetric path — z-resolved exposure",
             ["I(x, y, z) = I_{aer}(x, y; d_0 + z/n_r) · S(z)", "exact thin-film standing wave S(z),", "split-step bleaching"], "hue:future")
    _slab(s, lx_in + lw - 78, bot_y + 36, 60, 30, stripes=3)
    s.text(lx_in + lw - 48, bot_y + 86, "S(z) bands", size=T_MIN, color="ink2")
    lane_box(lx_out, bot_y, lw, lh, "3-D development — depth map · FMM · level set",
             ["|∇T| = 1/R  or  φ_t + R|∇φ| = 0", "sidewalls, undercut, surface inhibition,", "developer ageing and loading"], "hue:future")
    px, py = lx_out + lw - 80, bot_y + 40
    s.rect(px, py, 60, 26, "resist", sw=0.9)
    s.polyline([(px + 14, py), (px + 20, py + 26), (px + 40, py + 26), (px + 46, py)], "surface", 0.0, fill="surface", close=True)
    s.polyline([(px + 14, py), (px + 20, py + 26), (px + 40, py + 26), (px + 46, py)], "hue:future", 1.8)
    s.rect(px, py + 26, 60, 6, "substrate", sw=0.8)

    # connectors: inputs → the PAC box; the dissolution box → outputs
    pac_x, pac_y = boxes[1]
    dis_x, dis_y = boxes[5]
    s.path(f"M{K.fmt(lx_in + lw - 140)},{K.fmt(top_y + lh)} C{K.fmt(lx_in + lw - 140)},{K.fmt(top_y + lh + 40)} "
           f"{K.fmt(pac_x + bw / 2)},{K.fmt(pac_y - 40)} {K.fmt(pac_x + bw / 2)},{K.fmt(pac_y - 2)}",
           stroke="hue:nodes", sw=1.6, arrow="end")
    s.path(f"M{K.fmt(lx_in + lw - 140)},{K.fmt(bot_y)} C{K.fmt(lx_in + lw - 140)},{K.fmt(bot_y - 40)} "
           f"{K.fmt(pac_x + bw / 2)},{K.fmt(pac_y + bh + 40)} {K.fmt(pac_x + bw / 2)},{K.fmt(pac_y + bh + 2)}",
           stroke="hue:future", sw=1.6, arrow="end")
    s.path(f"M{K.fmt(dis_x + bw / 2)},{K.fmt(dis_y - 2)} C{K.fmt(dis_x + bw / 2)},{K.fmt(dis_y - 40)} "
           f"{K.fmt(lx_out + 140)},{K.fmt(top_y + lh + 40)} {K.fmt(lx_out + 140)},{K.fmt(top_y + lh + 2)}",
           stroke="hue:nodes", sw=1.6, arrow="end")
    s.path(f"M{K.fmt(dis_x + bw / 2)},{K.fmt(dis_y + bh + 2)} C{K.fmt(dis_x + bw / 2)},{K.fmt(dis_y + bh + 40)} "
           f"{K.fmt(lx_out + 140)},{K.fmt(bot_y - 40)} {K.fmt(lx_out + 140)},{K.fmt(bot_y - 2)}",
           stroke="hue:future", sw=1.6, arrow="end")
    s.text(W / 2, H - 10, "exposure enters the chain from above (2-D) or below (volumetric); development leaves it the same way",
           size=T_MIN, color="ink2", italic=True)
    return s.render()


RESIST_ALT = ("Six-step resist chain from photon to dissolution rate (photon, PAC or PAG decomposition, acid, bake "
              "diffusion, deprotection, Mack or threshold rate) with the depth-averaged 2-D path above it and the "
              "z-resolved volumetric path below it, branching at exposure and rejoining at development.")
RESIST_CAPTION = (
    "<strong>Mechanism.</strong> One chemistry, two dimensionalities: photons decompose the photo-active compound or "
    "photo-acid generator (m = exp(−C·D·I)), the bake diffuses the latent image (a Gaussian blur, or the acid–quencher "
    "reaction–diffusion of a chemically amplified resist), and the Mack or threshold rate R(m) turns the protected "
    "fraction into dissolution. The 2-D path feeds it one depth-averaged coupling scalar and etches the centre row "
    "vertically; the volumetric path feeds it I_aer(x, y; d₀ + z/n_r)·S(z) and develops in three dimensions. "
    + TRAILER
)


# ---------------------------------------------------------------------------
# P6 — grayscale-mechanism
# ---------------------------------------------------------------------------

GRAY_PERIODS = 3
GRAY_HARMONICS = 4  # the band-limited sawtooth keeps this many Fourier harmonics


def _sawtooth(u: float) -> float:
    return u - math.floor(u)


def _bandlimited_sawtooth(u: float, n_harm: int = GRAY_HARMONICS) -> float:
    """The sawtooth's Fourier series truncated at ``n_harm`` harmonics (the optics band-limit the mask's ramps)."""
    acc = 0.5
    for k in range(1, n_harm + 1):
        acc -= math.sin(2 * math.pi * k * u) / (math.pi * k)
    return acc


def draw_grayscale(theme: str) -> str:
    W, H = 960, 352
    s = K.Svg(W, H, theme, "Grayscale lithography: transmittance → dose → contrast curve → height",
              GRAYSCALE_ALT, generator=GENERATOR)
    m, gap = 16, 34
    pw = (W - 2 * m - 3 * gap) / 4
    xs = [m + i * (pw + gap) for i in range(4)]
    titles = ["Mask transmittance T(x)", "Local dose D(x) = D_{exp} · I(x)", "Contrast curve f(D)", "Height h = T_r·(1 − f(D))"]
    for x, t in zip(xs, titles):
        rich(s, x + pw / 2, 30, t, size=T_LABEL, color="ink", weight=600)
    for x in xs[1:]:
        s.line(x - gap + 5, 180, x - 5, 180, "ink2", 1.6, arrow="end")

    # (1) a continuous-tone strip: three sawtooth periods in ten gray levels each, then T(x)
    x = xs[0]
    strip_y, strip_h = 50, 42
    levels = 10
    cell = pw / (GRAY_PERIODS * levels)
    for p in range(GRAY_PERIODS):
        for l in range(levels):
            tnt = 0.12 + 0.78 * (l + 0.5) / levels
            s.rect(x + (p * levels + l) * cell, strip_y, cell + 0.4, strip_h, fill=s.tint("ink", tnt), stroke="none")
    s.rect(x, strip_y, pw, strip_h, fill="none", stroke="axis", sw=1.0)
    s.text(x + pw / 2, strip_y + strip_h + 16, "continuous-tone or pixelated", size=T_MIN, color="ink2")
    rich(s, x + pw / 2, strip_y + strip_h + 33, "T = D_{req} / D_{exp}, amplitude √T", size=T_MIN, color="ink2")
    ay0, ay1 = 300, 160
    s.line(x, ay0, x + pw, ay0, "axis", 1.0, arrow="end")
    s.line(x, ay0, x, ay1 - 6, "axis", 1.0, arrow="end")
    pts = [(x + pw * u, ay0 - (ay0 - ay1) * (0.15 + 0.8 * _sawtooth(u * GRAY_PERIODS))) for u in (i / 300 for i in range(301))]
    s.polyline(pts, "blue", 2.0)
    s.text(x + 6, ay1, "T", size=T_MIN, anchor="start", color="ink2", italic=True)
    s.text(x + pw / 2, 322, "x →", size=T_MIN, color="ink2")

    # (2) the dose after imaging: a band-limited sawtooth
    x = xs[1]
    ay0, ay1 = 300, 70
    s.line(x, ay0, x + pw, ay0, "axis", 1.0, arrow="end")
    s.line(x, ay0, x, ay1 - 6, "axis", 1.0, arrow="end")
    pts = [(x + pw * u, ay0 - (ay0 - ay1) * (0.15 + 0.75 * _bandlimited_sawtooth(u * GRAY_PERIODS)))
           for u in (i / 300 for i in range(301))]
    s.polyline(pts, "orange", 2.0)
    s.text(x + 6, ay1, "D", size=T_MIN, anchor="start", color="ink2", italic=True)
    s.text(x + pw / 2, 322, "x → · the optics round the cusps", size=T_MIN, color="ink2")

    # (3) the contrast curve on a log-dose axis
    x = xs[2]
    ay0, ay1 = 300, 70
    s.line(x, ay0, x + pw, ay0, "axis", 1.0, arrow="end")
    s.line(x, ay0, x, ay1 - 6, "axis", 1.0, arrow="end")
    xth, xcl = x + pw * 0.24, x + pw * 0.64
    ytop = ay0 - (ay0 - ay1) * 0.85
    s.polyline([(x, ay0), (xth, ay0), (xcl, ytop), (x + pw, ytop)], "green", 2.2)
    s.line(xth, ay0, xth, ay0 + 8, "ink2", 1.0)
    s.line(xcl, ay0, xcl, ay0 + 8, "ink2", 1.0)
    rich(s, xth, ay0 + 22, "D_{th}", size=T_MIN, color="ink2")
    rich(s, xcl, ay0 + 22, "D_{clear}", size=T_MIN, color="ink2")
    s.text(x + pw, 322, "log₁₀ D", size=T_MIN, anchor="end", color="ink2")
    rich(s, x + 6, ay1, "f = removed depth / T_r", size=T_MIN, anchor="start", color="ink2")
    s.line(x - 4, ytop, x + 4, ytop, "ink2", 1.0)
    s.text(x - 7, ytop + 4, "1", size=T_MIN, anchor="end", color="ink2")
    s.text(x + pw * 0.44 - 14, (ay0 + ytop) / 2 + 4, "slope γ", size=T_MIN, anchor="end", color="green")
    rich(s, x + pw, 270, "γ = 1/log₁₀(D_{clear}/D_{th})", size=T_MIN, anchor="end", color="ink2")
    s.text(x + pw, 287, "clamped to [0, 1]", size=T_MIN, anchor="end", color="ink2")

    # (4) the relief: a blazed grating sawtooth cut into the resist
    x = xs[3]
    top, base = 118, 262
    pts = [(x, top)]
    for p in range(GRAY_PERIODS):
        xa = x + pw * p / GRAY_PERIODS
        xb = x + pw * (p + 1) / GRAY_PERIODS
        pts += [(xa, top), (xb - 0.01, base - 20)]
    pts += [(x + pw, top), (x + pw, base), (x, base)]
    s.polyline(pts, s.stroke("resist"), 1.2, fill=s.fill("resist"), close=True)
    s.rect(x, base, pw, 14, "substrate", sw=0.8)
    _dim_v(s, x + pw + 10, top, base, "T_r", side="right")
    s.text(x + pw / 2, base + 32, "blazed grating in positive resist", size=T_MIN, color="ink2")
    rich(s, x + pw / 2, 322, "h = T_r − depth(D·I)", size=T_MIN, color="ink2")

    s.text(W / 2, H - 8, "2.5-D fast path, per pixel: no standing waves, no bake, no lateral development (those belong to the volumetric path)",
           size=T_MIN, color="ink2", italic=True)
    return s.render()


GRAYSCALE_ALT = ("Four panels in a row: a continuous-tone mask transmittance T(x), the band-limited local dose D(x) it "
                 "images to, the log-linear contrast curve rising from D_th to D_clear with slope γ, and the resulting "
                 "blazed resist relief h = T_r·(1 − f(D)) on its substrate.")
GRAYSCALE_CAPTION = (
    "<strong>Mechanism.</strong> A continuous-tone mask sets the local dose through the same diffraction engine that "
    "images binary masks; the log-linear contrast curve f(D) = clamp(γ·log₁₀(D/D_th), 0, 1) converts dose to removed "
    "depth, and the height map is h = T_r·(1 − f(D)). The 2.5-D path is per pixel: standing waves, bake and lateral "
    "development belong to the volumetric path. "
    + TRAILER
)


# ---------------------------------------------------------------------------
# P7a — interference-beam-geometry
# ---------------------------------------------------------------------------

N_RESIST = 1.6                  # the engine figure's resist index
THETA_AIR = (30.0, 30.0, 60.0)  # air-side half-angles of the three presets drawn (two-beam, hex, umbrella)


def snell_med(theta_air_deg: float, n: float = N_RESIST) -> float:
    """In-medium half-angle (degrees) for an air-side half-angle."""
    return math.degrees(math.asin(math.sin(math.radians(theta_air_deg)) / n))


def projected_angle(theta_deg: float, azimuth_deg: float) -> float:
    """Angle from the normal (degrees) of a k-vector at polar θ and azimuth φ projected on the x–z plane."""
    th, ph = math.radians(theta_deg), math.radians(azimuth_deg)
    return math.degrees(math.atan2(math.sin(th) * math.cos(ph), math.cos(th)))


def _inset_topview(s: K.Svg, x: float, y: float, size: float, *, centre: bool) -> None:
    """Top view (x–y): the transverse k-vectors of the three umbrella beams at 0°, 120°, 240°."""
    s.rect(x, y, size, size, fill="surface", stroke="axis", sw=0.9, rx=4)
    cx, cy, r = x + size / 2, y + size / 2 + 4, size * 0.34
    s.eyebrow(x + size / 2, y + 12, "top view", size=T_MIN, anchor="middle")
    for az, col in ((0.0, "blue"), (120.0, "orange"), (240.0, "orange")):
        a = math.radians(az)
        s.line(cx, cy, cx + r * math.cos(a), cy - r * math.sin(a), col, 1.6, arrow="end")
    if centre:
        s.circle(cx, cy, 4, fill="green", stroke="surface", sw=1.0)
    rich(s, x + size - 4, y + size - 4, "k_⊥", size=T_MIN, anchor="end", color="ink2")


def _panel_interference(s: K.Svg, x: float, w: float, which: int) -> None:
    """One x–z projection panel: air above the interface at y_i, resist below."""
    y_top, y_i, y_bot = 56, 262, 428
    xc = x + w / 2
    th_air = THETA_AIR[which]
    th_med = snell_med(th_air)
    s.rect(x, y_i, w, y_bot - y_i, "resist", sw=0.0)
    s.line(x, y_i, x + w, y_i, "ink2", 1.4)
    s.text(x + w - 8, y_i - 8, "air, n = 1", size=T_MIN, anchor="end", color="ink2")
    s.text(x + w - 8, y_i + 18, f"resist, n = {N_RESIST:g}", size=T_MIN, anchor="end", color="ink2")
    s.line(xc, y_top + 30, xc, y_bot - 8, "axis", 0.9, dash="3 3")  # the normal
    s.line(x + w - 52, y_bot - 10, x + w - 22, y_bot - 10, "ink2", 1.0, arrow="end")
    s.line(x + w - 52, y_bot - 10, x + w - 52, y_bot - 40, "ink2", 1.0, arrow="end")
    s.text(x + w - 14, y_bot - 6, "x", size=T_MIN, color="ink2", italic=True)
    s.text(x + w - 52, y_bot - 46, "−z", size=T_MIN, color="ink2", italic=True)

    L_air, L_med = 140.0, 150.0

    def air_beam(angle_deg: float, color: str, doubled: bool = False) -> None:
        a = math.radians(angle_deg)
        for o in ((3, -3) if doubled else (0,)):
            _beam(s, xc - L_air * math.sin(a) + o, y_i - L_air * math.cos(a), xc + o, y_i, color, sw=2.2 if not doubled else 1.8)

    def med_beam(angle_deg: float, color: str, doubled: bool = False) -> None:
        a = math.radians(angle_deg)
        for o in ((3, -3) if doubled else (0,)):
            _beam(s, xc + o, y_i, xc + L_med * math.sin(a) + o, y_i + L_med * math.cos(a), color, sw=2.2 if not doubled else 1.8)

    def mid_air(angle_deg: float, f: float = 0.5) -> tuple[float, float]:
        a = math.radians(angle_deg)
        return xc - L_air * math.sin(a) * f, y_i - L_air * math.cos(a) * f

    def mid_med(angle_deg: float, f: float = 0.55) -> tuple[float, float]:
        a = math.radians(angle_deg)
        return xc + L_med * math.sin(a) * f, y_i + L_med * math.cos(a) * f

    if which == 0:
        air_beam(th_air, "blue")
        air_beam(-th_air, "orange")
        med_beam(th_med, "blue")
        med_beam(-th_med, "orange")
        for ang, col in ((th_air, "blue"), (-th_air, "orange")):
            mx, my = mid_air(ang, 0.62)
            _dot_te(s, mx, my, col)
        for ang, col in ((th_med, "blue"), (-th_med, "orange")):
            mx, my = mid_med(ang, 0.5)
            _dot_te(s, mx, my, col)
        _arc(s, xc, y_i, 46, 270, 270 - th_air, "ink2")
        rich(s, xc - 22, y_i - 52, "θ_{air}", size=T_MIN, anchor="end", color="ink2")
        _arc(s, xc, y_i, 56, 90, 90 - th_med, "ink2")
        rich(s, xc + 24, y_i + 70, "θ_{res}", size=T_MIN, anchor="start", color="ink2")
        # the transverse wavevector of beam 1 on both sides of the interface: equal lengths
        kx0, kx1 = xc - 126, xc - 64
        s.line(kx0, y_i - 18, kx1, y_i - 18, "blue", 1.9, arrow="end")
        s.line(kx0, y_i + 18, kx1, y_i + 18, "blue", 1.9, arrow="end")
        s.line(kx0, y_i - 26, kx0, y_i + 26, "ink2", 0.9, dash="2 2")
        s.line(kx1, y_i - 26, kx1, y_i + 26, "ink2", 0.9, dash="2 2")
        rich(s, (kx0 + kx1) / 2, y_i - 26, "k_⊥ in air", size=T_MIN, color="ink")
        rich(s, (kx0 + kx1) / 2, y_i + 36, "k_⊥ in resist", size=T_MIN, color="ink")
        s.text((kx0 + kx1) / 2, y_i + 51, "equal", size=T_MIN, color="ink2", italic=True)
        lam = 34
        for k in range(-4, 5):
            fx = xc + k * lam
            if x + 8 < fx < x + w - 8:
                s.line(fx, y_i + 96, fx, y_bot - 46, "orange", 1.3, dash="4 3", opacity=0.9)
        _dim_h(s, xc, xc + lam, y_bot - 36, "Λ = λ / (2 sin θ_{air})", above=False)
        s.text(xc, y_top + 2, "(a) two beams — 1-D grating; n drops out", size=T_LABEL, color="ink", weight=600)
        rich(s, xc, y_bot + 18, "k_⊥ = (2π/λ) sin θ_{air} = (2πn/λ) sin θ_{res}", size=T_MIN, color="ink2")
        s.text(xc, y_bot + 33, f"air-side half-angle {th_air:g}° → {th_med:.1f}° in n = {N_RESIST:g}", size=T_MIN, color="ink2")
        return

    a1 = projected_angle(th_air, 0.0)
    a23 = projected_angle(th_air, 120.0)
    m1 = projected_angle(th_med, 0.0)
    m23 = projected_angle(th_med, 120.0)
    if which == 2:  # the central beam along +z, polarized along x
        _beam(s, xc, y_i - L_air, xc, y_i, "green", sw=2.2)
        _beam(s, xc, y_i, xc, y_i + L_med * 0.98, "green", sw=2.2)
        _inplane_e(s, xc, y_i - 92, 1.0, 0.0, "green")
        _inplane_e(s, xc, y_i + 72, 1.0, 0.0, "green")
        s.text(xc - 10, y_top + 26, "central beam", size=T_MIN, anchor="end", color="green")
        s.text(xc - 10, y_top + 41, "k ∥ z, E ∥ x", size=T_MIN, anchor="end", color="green")
    air_beam(a1, "blue")
    air_beam(a23, "orange", doubled=True)
    med_beam(m1, "blue")
    med_beam(m23, "orange", doubled=True)
    mx, my = mid_air(a1, 0.62)
    _dot_te(s, mx, my, "blue")
    mx, my = mid_med(m1, 0.5)
    _dot_te(s, mx, my, "blue")
    # beams 2 and 3: the azimuthal TE polarization has an in-plane (x) projection of |sin φ| = 0.87
    mx, my = mid_air(a23, 0.62)
    _inplane_e(s, mx, my, 1.0, 0.0, "orange")
    mx, my = mid_med(m23, 0.5)
    _inplane_e(s, mx, my, 1.0, 0.0, "orange")
    # beam 1 enters from the upper left (it travels towards +x); beams 2, 3 from the upper right
    s.text(x + 8, y_i - 100, "beam 1", size=T_MIN, anchor="start", color="blue")
    s.text(x + 8, y_i - 85, "φ = 0°", size=T_MIN, anchor="start", color="blue")
    s.text(x + w - 8, y_i - 156, "beams 2, 3", size=T_MIN, anchor="end", color="orange")
    s.text(x + w - 8, y_i - 141, "φ = 120°, 240°", size=T_MIN, anchor="end", color="orange")
    s.text(x + w - 8, y_i - 126, "overlap in projection", size=T_MIN, anchor="end", color="ink2", italic=True)
    inset = 84
    _inset_topview(s, x + 8, y_i + 44, inset, centre=(which == 2))
    x_min = x + 8 + inset + 14
    if which == 1:
        for k in range(-3, 4):  # columns of maxima: the 2-D lattice is invariant along z
            fx = xc + k * 40
            if x_min < fx < x + w - 8:
                for yy in range(int(y_i + 92), int(y_bot - 50), 10):
                    s.circle(fx, yy, 1.4, fill="orange", stroke="none")
        s.text(x + 8, y_bot - 22, "hexagonal in x–y,", size=T_MIN, anchor="start", color="ink2")
        s.text(x + 8, y_bot - 7, "invariant in z", size=T_MIN, anchor="start", color="ink2")
        s.text(xc, y_top + 2, "(b) three beams — 2-D hexagonal lattice", size=T_LABEL, color="ink", weight=600)
    else:
        for row, yy in enumerate(range(int(y_i + 92), int(y_bot - 50), 22)):  # an FCC-like lattice: staggered rows
            for k in range(-3, 4):
                fx = xc + k * 40 + (20 if row % 2 else 0)
                if x_min < fx < x + w - 8:
                    s.circle(fx, yy, 3.2, fill="orange", stroke="none", opacity=0.9)
        s.text(x + 8, y_bot - 22, "3-D lattice,", size=T_MIN, anchor="start", color="ink2")
        s.text(x + 8, y_bot - 7, "staggered in x–z", size=T_MIN, anchor="start", color="ink2")
        s.text(xc, y_top + 2, "(c) central beam + umbrella — 3-D lattice", size=T_LABEL, color="ink", weight=600)
    s.text(xc, y_bot + 18, f"air-side half-angle {th_air:g}° → {th_med:.1f}° in n = {N_RESIST:g}", size=T_MIN, color="ink2")


def draw_interference(theme: str) -> str:
    W, H = 960, 476
    s = K.Svg(W, H, theme, "Interference-lithography beam geometries projected on the x–z plane",
              INTERFERENCE_ALT, generator=GENERATOR)
    m, gap = 14, 10
    pw = (W - 2 * m - 2 * gap) / 3
    for i in range(3):
        _panel_interference(s, m + i * (pw + gap), pw, i)
    lx, ly = m + 4, 28
    _dot_te(s, lx + 6, ly - 4, "ink2")
    s.text(lx + 18, ly, "E out of the plane (TE, ∥ y)", size=T_MIN, anchor="start", color="ink2")
    _inplane_e(s, lx + 236, ly - 4, 1.0, 0.0, "ink2")
    s.text(lx + 254, ly, "in-plane projection of E", size=T_MIN, anchor="start", color="ink2")
    s.text(W - m - 4, ly, "ideal infinite plane waves; no Fresnel coefficients, no substrate reflection",
           size=T_MIN, anchor="end", color="ink2", italic=True)
    return s.render()


INTERFERENCE_ALT = ("Three x–z projections of interference-lithography beams: two beams at ±30° refracting into a "
                    "resist of index 1.6 with the transverse wavevector equal on both sides of the interface, three "
                    "beams at azimuths 0°, 120° and 240° (hexagonal lattice), and a central beam plus a 60° umbrella "
                    "(3-D lattice).")
INTERFERENCE_CAPTION = (
    "<strong>Mechanism.</strong> Each beam is an infinite plane wave polarized perpendicular to its k-vector; two beams "
    "at ±θ write a 1-D grating whose period λ/(2·sin θ_air) survives refraction because the transverse wavevector is "
    "conserved across the interface, three beams at azimuths 0°, 120° and 240° write a hexagonal lattice, and a central "
    "beam plus a three-beam umbrella writes a 3-D lattice. Air-side half-angles 30°, 30° and 60° into n = 1.6, the engine "
    "figure's values. "
    + TRAILER
)


# ---------------------------------------------------------------------------
# P8b — talbot-schemes
# ---------------------------------------------------------------------------


def _self_image_row(s: K.Svg, x: float, y: float, w: float, period: float, *, shift: float = 0.0,
                    duty: float = 0.5, h: float = 7.0, opacity: float = 1.0, color: str = "blue") -> None:
    bar = period * duty
    xx = x + (period - bar) / 2 + shift
    while xx < x + w - 1:
        if xx >= x:
            s.rect(xx, y, min(bar, x + w - xx), h, fill=s.tint(color, 0.55), stroke="none", opacity=opacity)
        xx += period


def _wafer_bar(s: K.Svg, x: float, y: float, w: float) -> None:
    s.rect(x, y, w, 7, "substrate", sw=0.8)


def _panel_talbot(s: K.Svg, x: float, w: float, which: int) -> None:
    y_g = 96
    gw = w - 80
    gx = x + 10
    period = 22.0
    y_zt = y_g + 222  # one Talbot length below the grating
    zt = y_zt - y_g
    lab_x = gx + gw + 8  # the label column at the right of the propagation region
    titles = ["(a) fixed gap", "(b) DTL — gap scanned", "(c) ATL — beyond z_A", "(d) two gratings — EUV-IL"]
    rich(s, x + w / 2, 26, titles[which], size=T_LABEL, color="ink", weight=600)
    if which < 3:
        _grating(s, gx, y_g, gw, period)
    if which == 0:
        _dim_h(s, gx + period / 4, gx + period / 4 + period, y_g + 20, "p", above=False)
    if which in (0, 1):
        for k in range(3):
            xx = gx + gw * (0.2 + 0.3 * k)
            s.line(xx, y_g - 40, xx, y_g - 18, "ink2", 1.2, arrow="end")
    if which == 0:
        _self_image_row(s, gx, y_g + zt * 0.25, gw, period / 2, duty=0.5, h=6, opacity=0.8)
        _self_image_row(s, gx, y_g + zt * 0.5, gw, period, shift=period / 2, h=7)
        _self_image_row(s, gx, y_g + zt * 0.75, gw, period / 2, duty=0.5, h=6, opacity=0.8)
        _self_image_row(s, gx, y_zt, gw, period, h=7)
        rich(s, lab_x, y_g + zt * 0.25 + 5, "z_T/4", size=T_MIN, anchor="start", color="ink2")
        rich(s, lab_x, y_g + zt * 0.5 + 5, "z_T/2", size=T_MIN, anchor="start", color="ink2")
        rich(s, lab_x, y_g + zt * 0.75 + 5, "3z_T/4", size=T_MIN, anchor="start", color="ink2")
        rich(s, lab_x, y_zt + 5, "z_T", size=T_MIN, anchor="start", color="ink2")
        yw = y_g + zt * 0.40
        _wafer_bar(s, gx, yw, gw)
        rich(s, lab_x, yw + 6, "wafer", size=T_MIN, anchor="start", color="ink2")
        _dim_v(s, gx - 6, y_g + 9, yw, "g", side="left")
        rich(s, x + w / 2, y_zt + 44, "z_T = 2p²/λ: self-image; z_T/2 shifted", size=T_MIN, color="ink2")
        rich(s, x + w / 2, y_zt + 59, "by p/2; z_T/4 period halved", size=T_MIN, color="ink2")
        s.text(x + w / 2, y_zt + 76, "fragile: the image changes within a", size=T_MIN, color="ink2", italic=True)
        rich(s, x + w / 2, y_zt + 91, "fraction of z_T", size=T_MIN, color="ink2", italic=True)
    elif which == 1:
        y1 = y_g + 34
        y2 = y1 + zt - 10
        _wafer_bar(s, gx, y1, gw)
        s.rect(gx, y2, gw, 7, fill="none", stroke="axis", sw=0.9, dash="3 2")
        _dim_v(s, lab_x + 2, y1, y2, "scan z_T", side="right")
        _self_image_row(s, gx, y1 - 9, gw, period / 2, duty=0.5, h=8, color="orange")
        s.text(gx + gw / 2, y1 + 21, "wafer, moving", size=T_MIN, color="ink2")
        s.text(x + w / 2, y_zt + 44, "the gap sweeps one Talbot length", size=T_MIN, color="ink2")
        s.text(x + w / 2, y_zt + 59, "during the exposure", size=T_MIN, color="ink2")
        s.text(x + w / 2, y_zt + 76, "time-integrated image: period p/2,", size=T_MIN, color="ink2", italic=True)
        s.text(x + w / 2, y_zt + 91, "independent of the gap", size=T_MIN, color="ink2", italic=True)
    elif which == 2:
        _spectrum_glyph(s, gx + gw * 0.3, y_g - 64, gw * 0.4, 26, peak=0.5, cut=0.0, color="violet")
        s.text(gx + gw * 0.72, y_g - 44, "Δλ", size=T_MIN, anchor="start", color="violet")
        s.line(gx + gw * 0.5, y_g - 34, gx + gw * 0.5, y_g - 18, "ink2", 1.2, arrow="end")
        for k, f in enumerate((0.22, 0.44, 0.66)):
            _self_image_row(s, gx, y_g + zt * f, gw, period, shift=(period / 2 if k % 2 == 0 else 0), h=7,
                            opacity=0.9 - 0.3 * k)
        ya = y_g + zt * 0.84
        s.line(gx - 6, ya, gx + gw + 6, ya, "violet", 1.2, dash="5 4")
        _dim_v(s, lab_x + 2, y_g + 9, ya, "z_A", side="right")
        yw = y_zt + 12
        _wafer_bar(s, gx, yw, gw)
        _self_image_row(s, gx, yw - 9, gw, period / 2, duty=0.5, h=8, color="orange")
        rich(s, lab_x, yw + 6, "wafer", size=T_MIN, anchor="start", color="ink2")
        rich(s, x + w / 2, y_zt + 44, "z_A = 2p²/Δλ: beyond it the", size=T_MIN, color="ink2")
        s.text(x + w / 2, y_zt + 59, "spectral average washes out", size=T_MIN, color="ink2")
        s.text(x + w / 2, y_zt + 76, "the revivals → stationary", size=T_MIN, color="ink2", italic=True)
        s.text(x + w / 2, y_zt + 91, "image of period p/2", size=T_MIN, color="ink2", italic=True)
    else:
        g_w = (gw - 24) / 2
        _grating(s, gx, y_g, g_w, period)
        _grating(s, gx + g_w + 24, y_g, g_w, period)
        for xx in (gx + g_w * 0.5, gx + g_w + 24 + g_w * 0.5):
            s.line(xx, y_g - 40, xx, y_g - 18, "ink2", 1.2, arrow="end")
        theta = 24.0
        a = math.radians(theta)
        xl, xr = gx + g_w * 0.5, gx + g_w + 24 + g_w * 0.5
        yw = y_zt + 12
        Lb = (yw - y_g - 12) / math.cos(a)
        _beam(s, xl, y_g + 12, xl + Lb * math.sin(a), yw, "blue", sw=2.2)
        _beam(s, xr, y_g + 12, xr - Lb * math.sin(a), yw, "orange", sw=2.2)
        s.text(xl - 6, y_g + 60, "+m", size=T_MIN, anchor="end", color="blue")
        s.text(xr + 6, y_g + 60, "−m", size=T_MIN, anchor="start", color="orange")
        xcross = (xl + xr) / 2
        ycross = y_g + 12 + ((xr - xl) / 2) / math.tan(a)
        _wafer_bar(s, gx, yw, gw)
        rich(s, lab_x, yw + 6, "wafer", size=T_MIN, anchor="start", color="ink2")
        fp = period / 2
        for k in range(-3, 4):
            fx = xcross + k * fp
            s.line(fx, yw - 44, fx, yw - 2, "ink", 1.3, opacity=0.85)
        _dim_h(s, xcross, xcross + fp, yw - 52, "p/(2m)", above=True)
        _arc(s, xl, y_g + 12, 36, 90, 90 - theta, "ink2")
        s.text(xl + 22, y_g + 64, "θ", size=T_MIN, anchor="start", color="ink2")
        rich(s, x + w / 2, y_zt + 44, "sin θ = mλ/p: the +m order of one", size=T_MIN, color="ink2")
        s.text(x + w / 2, y_zt + 59, "grating meets the −m of the other", size=T_MIN, color="ink2")
        s.text(x + w / 2, y_zt + 76, "fringes of period p/(2m),", size=T_MIN, color="ink2", italic=True)
        s.text(x + w / 2, y_zt + 91, "independent of λ", size=T_MIN, color="ink2", italic=True)


def draw_talbot(theme: str) -> str:
    W, H = 960, 440
    s = K.Svg(W, H, theme, "Four Talbot-family schemes behind a transmission grating",
              TALBOT_ALT, generator=GENERATOR)
    m, gap = 14, 12
    pw = (W - 2 * m - 3 * gap) / 4
    for i in range(4):
        _panel_talbot(s, m + i * (pw + gap), pw, i)
    s.text(W / 2, H - 8, "one scalar thin-mask grating model, coherent normal illumination; (a)–(c) share the same orders, (d) converts into a two-beam setup",
           size=T_MIN, color="ink2", italic=True)
    return s.render()


TALBOT_ALT = ("Four schemes behind a transmission grating: a fixed gap inside the Talbot carpet, displacement Talbot "
              "with the gap scanned over one Talbot length, achromatic Talbot beyond z_A = 2p²/Δλ with a broadband "
              "source, and two gratings whose +m and −m orders cross at ±θ to print fringes of period p/2m.")
TALBOT_CAPTION = (
    "<strong>Mechanism.</strong> A grating re-images itself every Talbot length z_T = 2p²/λ with a half-period-shifted "
    "copy in between, so a fixed gap is fragile; displacement Talbot scans the gap over z_T and achromatic Talbot uses a "
    "band Δλ beyond z_A = 2p²/Δλ, both leaving a stationary image of period p/2. Two gratings whose +m and −m orders "
    "cross at ±θ print fringes of period p/2m whatever the wavelength. "
    + TRAILER
)


# ---------------------------------------------------------------------------
# P9a — liga-geometry
# ---------------------------------------------------------------------------


def draw_liga(theme: str) -> str:
    W, H = 960, 440
    s = K.Svg(W, H, theme, "LIGA shadow printing: bending magnet, window, filters, mask, gap and thick resist",
              LIGA_ALT, generator=GENERATOR)
    yb = 236  # the beam axis
    bh = 44   # the beam band height
    # the storage ring arc and the bending magnet
    s.path(f"M{K.fmt(40)},{K.fmt(yb + 150)} A 230,230 0 0,1 {K.fmt(150)},{K.fmt(yb)}", stroke="axis", sw=2.0, dash="6 5")
    s.rect(96, yb - 34, 58, 68, "metal", sw=1.0, rx=4)
    s.text(125, yb - 6, "bending", size=T_MIN, color="ink", weight=600)
    s.text(125, yb + 10, "magnet", size=T_MIN, color="ink", weight=600)
    s.circle(150, yb, 4.5, fill="blue", stroke="surface", sw=1.2)
    s.text(60, yb + 150 + 20, "storage ring: E, B, I", size=T_MIN, anchor="start", color="ink2")
    s.text(125, yb - 46, "e⁻", size=T_MIN, color="blue")
    # the white beam (tangent)
    grad = s.gradient_h("liga-beam", [(0.0, s.tint("violet", 0.55)), (1.0, s.tint("violet", 0.18))])
    s.rect(154, yb - bh / 2, 428, bh, fill=grad, stroke="none")
    s.line(154, yb, 582, yb, "violet", 1.4, arrow="end", opacity=0.9)
    s.text(160, yb + 54, "white beam: multi-keV, broadband, almost no diffraction", size=T_MIN, anchor="start", color="ink2")
    # stations along the beam: Be window, filter(s)
    for xx, mat, lab1, lab2 in ((300, "glass", "Be window", ""), (392, "metal", "filter(s)", "Al · Kapton")):
        s.rect(xx - 5, yb - bh / 2 - 10, 10, bh + 20, mat, sw=1.0)
        s.text(xx, yb + 74, lab1, size=T_MIN, color="ink", weight=600)
        if lab2:
            s.text(xx, yb + 90, lab2, size=T_MIN, color="ink2")
    # the spectrum, hardening station by station
    s.text(380, 36, "the beam hardens: soft photons go first", size=T_SMALL, color="ink", weight=600)
    for xx, pk, cut, lab in ((222, 0.40, 0.05, "at the ring: ∝ G_1(E/E_c)"), (330, 0.46, 0.22, "after Be"), (450, 0.56, 0.40, "after the filter")):
        _spectrum_glyph(s, xx - 36, 62, 72, 44, peak=pk, cut=cut, color="violet")
        rich(s, xx, 124, lab, size=T_MIN, color="ink2")
    s.text(186, 58, "photons / ln E", size=T_MIN, anchor="start", color="ink2", italic=True)
    s.text(500, 110, "E →", size=T_MIN, anchor="start", color="ink2")
    # the mask: a Ti membrane with Au absorbers on its downstream face
    mx = 586
    s.rect(mx, yb - 110, 6, 220, fill=s.tint("aqua", 0.45), stroke="aqua", sw=0.8)
    openings = [(yb - 110, yb - 72), (yb - 30, yb + 12), (yb + 58, yb + 110)]
    y_prev = yb - 110
    for (ya, yb2) in openings:
        if ya > y_prev:
            s.rect(mx + 6, y_prev, 16, ya - y_prev, "absorber", sw=0.6)
        y_prev = yb2
    s.text(mx + 12, yb - 150, "mask", size=T_SMALL, color="ink", weight=600)
    s.text(mx + 12, yb - 134, "Ti membrane 2 µm", size=T_MIN, color="ink2")
    s.text(mx + 12, yb - 120, "Au absorber 20 µm", size=T_MIN, color="ink2")
    # the gap and the resist on its substrate; depth z runs along the beam
    rx = mx + 22 + 44
    rw, rh = 150, 220
    s.rect(rx, yb - 110, rw, rh, "resist", sw=0.9)
    s.rect(rx + rw, yb - 110, 26, rh, "substrate", sw=0.8)
    s.text(rx + rw / 2, yb - 122, "PMMA 10–1000 µm", size=T_SMALL, color="ink", weight=600)
    s.text(rx + rw + 13, yb + 128, "substrate", size=T_MIN, color="ink2")
    gdose = s.gradient_h("liga-dose", [(0.0, s.tint("violet", 0.70)), (1.0, s.tint("violet", 0.12))])
    for (ya, yb2) in openings:
        s.rect(rx, ya, rw, yb2 - ya, fill=gdose, stroke="none")
        s.line(mx + 22, (ya + yb2) / 2, rx - 2, (ya + yb2) / 2, "violet", 1.2, arrow="end", opacity=0.8)
    _dim_h(s, mx + 22, rx, yb + 122, "gap g", above=False)
    zx = rx + rw * 0.6
    _dim_h(s, mx + 22, zx, yb + 36, "d = g + z", above=False)
    s.line(zx, yb + 30, zx, yb + 44, "ink2", 1.0)
    s.text(rx + 6, yb - 92, "1:1 shadow — no optic", size=T_MIN, anchor="start", color="ink", weight=600)
    s.text(rx + 6, yb - 60, "dose D(z) falls with", size=T_MIN, anchor="start", color="ink2")
    s.text(rx + 6, yb - 46, "depth, more slowly", size=T_MIN, anchor="start", color="ink2")
    s.text(rx + 6, yb - 32, "once hardened", size=T_MIN, anchor="start", color="ink2")
    # the scan
    sx = rx + rw + 26 + 22
    s.line(sx, yb - 100, sx, yb + 100, "ink2", 1.4, arrow="both")
    s.text(sx + 8, yb - 6, "scan mask +", size=T_MIN, anchor="start", color="ink2")
    s.text(sx + 8, yb + 10, "substrate", size=T_MIN, anchor="start", color="ink2")
    s.text(sx + 8, yb + 26, "through the fan", size=T_MIN, anchor="start", color="ink2")
    s.text(W / 2, H - 8, "shadow printing, not projection: no pupil, no TCC; simulation ends at the developed resist (electroforming and molding are not modelled)",
           size=T_MIN, color="ink2", italic=True)
    return s.render()


LIGA_ALT = ("Beam path of LIGA shadow printing from a storage-ring bending magnet through a Be window and filters to "
            "a Ti-membrane mask with 20 µm Au absorbers, a gap g and a 10–1000 µm PMMA resist on its substrate, with "
            "the spectrum hardening at each station and the scan direction marked.")
LIGA_CAPTION = (
    "<strong>Mechanism.</strong> A bending-magnet white beam passes a Be window and filters that harden its spectrum, "
    "then a Ti-membrane mask with Au absorbers casts a 1:1 shadow across a gap g into 10–1000 µm of PMMA while mask and "
    "substrate scan through the fan; there is no optic, and the dose at depth z is set by the hardened spectrum at "
    "d = g + z. Simulation ends at the developed resist: electroforming and molding are not modelled. "
    + TRAILER
)


# ---------------------------------------------------------------------------
# Registration, generation, check
# ---------------------------------------------------------------------------

DRAWERS = {
    "thinfilm-layer-stack": draw_thinfilm,
    "resist-chain-schematic": draw_resist_chain,
    "grayscale-mechanism": draw_grayscale,
    "interference-beam-geometry": draw_interference,
    "talbot-schemes": draw_talbot,
    "liga-geometry": draw_liga,
}


def _gen(name: str):
    def gen() -> list[Path]:
        return K.write_pair(name, DRAWERS[name], OUT)

    gen.__name__ = f"gen_{name.replace('-', '_')}"
    return gen


def _spec(name: str, alt: str, caption: str, pages: tuple[str, ...], hero_on: tuple[str, ...] = ()) -> registry.AssetSpec:
    return registry.AssetSpec(
        name=name, kind="schematic", group="diagrams", subdir="diagrams", fmt="svg",
        alt=alt, caption=caption, pages=pages, hero_on=hero_on,
        generator=_gen(name), platform_independent=True,
    )


SPECS = [
    _spec("thinfilm-layer-stack", THINFILM_ALT, THINFILM_CAPTION, ("processes/thin-film.md",), ("processes/thin-film.md",)),
    _spec("resist-chain-schematic", RESIST_ALT, RESIST_CAPTION, ("processes/resist-models.md",), ("processes/resist-models.md",)),
    _spec("grayscale-mechanism", GRAYSCALE_ALT, GRAYSCALE_CAPTION, ("processes/grayscale.md",), ("processes/grayscale.md",)),
    _spec("interference-beam-geometry", INTERFERENCE_ALT, INTERFERENCE_CAPTION, ("processes/interference-volumetric.md",),
          ("processes/interference-volumetric.md",)),
    _spec("talbot-schemes", TALBOT_ALT, TALBOT_CAPTION, ("processes/talbot.md",), ("processes/talbot.md",)),
    _spec("liga-geometry", LIGA_ALT, LIGA_CAPTION, ("processes/liga-deep-xray.md", "future/beyond-euv.md"),
          ("processes/liga-deep-xray.md",)),
]
for _s in SPECS:
    registry.register(_s, module=MODULE)


def check() -> tuple[list[str], list[str]]:
    """Geometry self-checks: the angles the interference panels draw are Snell's and the projections' own."""
    errors: list[str] = []
    warnings: list[str] = []
    for th in THETA_AIR:
        tm = snell_med(th)
        if abs(math.sin(math.radians(th)) - N_RESIST * math.sin(math.radians(tm))) > 1e-12:
            errors.append(f"interference-beam-geometry: Snell violated for θ_air = {th}°")
        k_air = math.sin(math.radians(th))
        k_med = N_RESIST * math.sin(math.radians(tm))
        if abs(k_air - k_med) > 1e-12:
            errors.append("interference-beam-geometry: k_⊥ not conserved across the interface")
        if abs(projected_angle(tm, 120.0) - projected_angle(tm, 240.0)) > 1e-9:
            errors.append("interference-beam-geometry: beams at φ = 120° and 240° must coincide in the x–z projection")
    # the band-limited sawtooth of the grayscale panel keeps the sawtooth's mean and peak-to-peak ordering
    mean = sum(_bandlimited_sawtooth(i / 200) for i in range(200)) / 200
    if abs(mean - 0.5) > 1e-6:
        errors.append("grayscale-mechanism: the band-limited sawtooth must keep the sawtooth's mean of 0.5")
    for spec in SPECS:
        if not spec.caption.rstrip().endswith(TRAILER):
            errors.append(f"{spec.name}: caption must end with the no-badge trailer {TRAILER!r}")
        if any(g in spec.caption or g in spec.alt for g in ("✅", "🔶", "🧪", "🗺")):
            errors.append(f"{spec.name}: a schematic carries no grade glyph")
        if len(spec.alt) > 300:
            errors.append(f"{spec.name}: alt is {len(spec.alt)} characters (≤ 300)")
    return errors, warnings


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if "--check" in argv:
        e, w = check()
        for x in w:
            print("warning:", x)
        for x in e:
            print("error:", x)
        print(f"process schematics check: {len(e)} error(s), {len(w)} warning(s)")
        return 1 if e else 0
    only = [a for a in argv if not a.startswith("--")]
    for spec in SPECS:
        if only and not any(o in spec.name for o in only):
            continue
        for p in spec.generator():
            print(f"wrote {registry.rel_to_repo(p)} ({p.stat().st_size / 1024:.1f} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
