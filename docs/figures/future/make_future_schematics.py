#!/usr/bin/env python3
"""SVG schematics for "The Future" section of the highuvlith site (docs/future/).

Run from the repository root (standard library only):

    python docs/figures/future/make_future_schematics.py

Every figure is written twice into ``docs/assets/images/future/``: ``<name>-light.svg``
and ``<name>-dark.svg`` (transparent background, ink chosen for Material's light and
"slate" schemes); pages select one with the ``#only-light`` / ``#only-dark`` suffixes.

These are schematics, not to scale. The only computed geometry is in
``anamorphic-why``: the mask-side cone half-angle asin(NA / M) and its position around
the chief-ray angle, drawn with every angle multiplied by ``ANGLE_GAIN`` (stated on
the figure) so that degree-scale gaps are visible.
"""

from __future__ import annotations

import math
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "assets" / "images" / "future"

FONT = "system-ui, -apple-system, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif"

# The site's content column is ~690 px wide, so a 960-unit-wide drawing is shown at ~0.72x.
# All text sizes below are multiplied by FONT_SCALE to stay legible at that width.
FONT_SCALE = 1.3

THEMES = {
    "light": {
        "ink": "#1d1c1a",
        "ink2": "#52514e",
        "muted": "#6b6a66",
        "line": "#52514e",
        "faint": "#d9d8d2",
        "panel": "#f5f4f0",
        "ebeam": "#2a78d6",
        "ebeam_fill": "#dbe9fa",
        "laser": "#d4521f",
        "laser_fill": "#fbe0d3",
        "euv": "#0e7490",
        "euv_fill": "#cdeef5",
        "duv": "#5b36c9",
        "duv_fill": "#e6dffb",
        "bad": "#c62828",
        "bad_fill": "#f8d7d7",
        "magnet_a": "#8a8780",
        "magnet_b": "#d9d8d2",
        "rf_fill": "#e3eefb",
        "rf": "#2a78d6",
        "mask_fill": "#c9c7bf",
        "mask": "#52514e",
    },
    "dark": {
        "ink": "#f1f0ee",
        "ink2": "#c3c2b7",
        "muted": "#a3a29b",
        "line": "#c3c2b7",
        "faint": "#4a4852",
        "panel": "#2a2834",
        "ebeam": "#6aa6ee",
        "ebeam_fill": "#1f3552",
        "laser": "#f08a5d",
        "laser_fill": "#5a2a16",
        "euv": "#22d3ee",
        "euv_fill": "#0f3d48",
        "duv": "#b5a1ff",
        "duv_fill": "#352a66",
        "bad": "#ff7b7b",
        "bad_fill": "#5a1f24",
        "magnet_a": "#a3a29b",
        "magnet_b": "#4a4852",
        "rf_fill": "#1f3552",
        "rf": "#6aa6ee",
        "mask_fill": "#4a4852",
        "mask": "#c3c2b7",
    },
}


def esc(s: str) -> str:
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


class Svg:
    """Minimal SVG writer with arrowheads and multi-line text."""

    def __init__(self, w: int, h: int, theme: str, title: str, desc: str):
        self.w, self.h = w, h
        self.t = THEMES[theme]
        self.title, self.desc = title, desc
        self.parts: list[str] = []
        self.markers: dict[str, str] = {}

    def add(self, s: str) -> None:
        self.parts.append(s)

    def arrow_marker(self, color: str) -> str:
        mid = "arr" + color.lstrip("#")
        self.markers[mid] = color
        return f"url(#{mid})"

    def line(self, x1, y1, x2, y2, color=None, width=2.0, arrow=False, dash=None, opacity=1.0) -> None:
        color = color or self.t["line"]
        extra = f' marker-end="{self.arrow_marker(color)}"' if arrow else ""
        if dash:
            extra += f' stroke-dasharray="{dash}"'
        if opacity != 1.0:
            extra += f' stroke-opacity="{opacity}"'
        self.add(
            f'<line x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" stroke="{color}" '
            f'stroke-width="{width}" stroke-linecap="round"{extra}/>'
        )

    def path(self, d, stroke=None, width=2.0, fill="none", arrow=False, dash=None, opacity=1.0, fill_opacity=1.0):
        stroke = stroke or self.t["line"]
        extra = f' marker-end="{self.arrow_marker(stroke)}"' if arrow else ""
        if dash:
            extra += f' stroke-dasharray="{dash}"'
        if opacity != 1.0:
            extra += f' stroke-opacity="{opacity}"'
        if fill_opacity != 1.0:
            extra += f' fill-opacity="{fill_opacity}"'
        self.add(
            f'<path d="{d}" stroke="{stroke}" stroke-width="{width}" fill="{fill}" '
            f'stroke-linecap="round" stroke-linejoin="round"{extra}/>'
        )

    def poly(self, pts, fill, stroke="none", width=1.5, fill_opacity=1.0) -> None:
        s = " ".join(f"{x:.1f},{y:.1f}" for x, y in pts)
        fo = f' fill-opacity="{fill_opacity}"' if fill_opacity != 1.0 else ""
        self.add(f'<polygon points="{s}" fill="{fill}" stroke="{stroke}" stroke-width="{width}"{fo} stroke-linejoin="round"/>')

    def rect(self, x, y, w, h, fill, stroke="none", width=1.5, rx=0.0) -> None:
        self.add(
            f'<rect x="{x:.1f}" y="{y:.1f}" width="{w:.1f}" height="{h:.1f}" rx="{rx}" '
            f'fill="{fill}" stroke="{stroke}" stroke-width="{width}"/>'
        )

    def circle(self, cx, cy, r, fill, stroke="none", width=1.5) -> None:
        self.add(f'<circle cx="{cx:.1f}" cy="{cy:.1f}" r="{r:.1f}" fill="{fill}" stroke="{stroke}" stroke-width="{width}"/>')

    def ellipse(self, cx, cy, rx, ry, fill, stroke="none", width=1.5) -> None:
        self.add(
            f'<ellipse cx="{cx:.1f}" cy="{cy:.1f}" rx="{rx:.1f}" ry="{ry:.1f}" fill="{fill}" '
            f'stroke="{stroke}" stroke-width="{width}"/>'
        )

    def text(self, x, y, s, size=14, anchor="middle", color=None, weight="normal", italic=False, lh=1.25):
        color = color or self.t["ink"]
        size = round(size * FONT_SCALE, 1)
        style = ' font-style="italic"' if italic else ""
        lines = s.split("\n")
        if len(lines) == 1:
            self.add(
                f'<text x="{x:.1f}" y="{y:.1f}" font-size="{size}" text-anchor="{anchor}" '
                f'fill="{color}" font-weight="{weight}"{style}>{esc(s)}</text>'
            )
            return
        tsp = "".join(
            f'<tspan x="{x:.1f}" dy="{0 if i == 0 else size * lh:.1f}">{esc(t)}</tspan>' for i, t in enumerate(lines)
        )
        self.add(
            f'<text x="{x:.1f}" y="{y:.1f}" font-size="{size}" text-anchor="{anchor}" '
            f'fill="{color}" font-weight="{weight}"{style}>{tsp}</text>'
        )

    def render(self) -> str:
        defs = "".join(
            f'<marker id="{mid}" viewBox="0 0 10 10" refX="8.5" refY="5" markerWidth="7" markerHeight="7" '
            f'orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" fill="{c}"/></marker>'
            for mid, c in self.markers.items()
        )
        return (
            f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {self.w} {self.h}" '
            f'width="{self.w}" height="{self.h}" role="img" aria-labelledby="t d" '
            f'font-family="{FONT}">'
            f'<title id="t">{esc(self.title)}</title><desc id="d">{esc(self.desc)}</desc>'
            f"<defs>{defs}</defs>" + "".join(self.parts) + "</svg>\n"
        )


# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------
def undulator(s: Svg, x: float, y: float, length: float, periods: int, gap: float = 16.0, mh: float = 9.0) -> None:
    """Row of alternating magnet blocks above and below a beam axis at height y."""
    w = length / periods
    for i in range(periods):
        top = s.t["magnet_a"] if i % 2 == 0 else s.t["magnet_b"]
        bot = s.t["magnet_b"] if i % 2 == 0 else s.t["magnet_a"]
        s.rect(x + i * w, y - gap / 2 - mh, w - 1.2, mh, top)
        s.rect(x + i * w, y + gap / 2, w - 1.2, mh, bot)


def wavy(x1: float, y: float, x2: float, amp: float, wl: float) -> str:
    """Path string of a sine wave along +x (a light wave or wiggle)."""
    n = max(8, int(abs(x2 - x1) / 2))
    pts = []
    for i in range(n + 1):
        x = x1 + (x2 - x1) * i / n
        pts.append(f"{x:.1f},{y + amp * math.sin(2 * math.pi * (x - x1) / wl):.1f}")
    return "M" + " L".join(pts)


def packet(x_center: float, y: float, half_len: float, amp: float, wl: float) -> str:
    """Gaussian-envelope wave packet (a short laser pulse)."""
    n = int(4 * half_len)
    pts = []
    for i in range(n + 1):
        x = x_center - half_len + 2 * half_len * i / n
        env = math.exp(-(((x - x_center) / (half_len * 0.5)) ** 2))
        pts.append(f"{x:.1f},{y + amp * env * math.sin(2 * math.pi * (x - x_center) / wl):.1f}")
    return "M" + " L".join(pts)


# ---------------------------------------------------------------------------
# 1. Why High-NA is anamorphic: incident and reflected cones at the mask
# ---------------------------------------------------------------------------
ANGLE_GAIN = 3.0
# Illumination (chief-ray) angle at the reticle. ~6 degrees is the commonly quoted value for
# 0.33 NA tools (SemiEngineering, "Gearing Up For High-NA EUV", 2021; Levinson, JJAP 61,
# SD0803 (2022)). We draw ALL panels at this illustrative 6 degrees: the EXE value is not
# verified here, and the point of the figure (cone overlap iff asin(NA/M) > chief-ray angle)
# does not depend on it.
CRA_DEG = 6.0


def fig_anamorphic_why(theme: str) -> Svg:
    s = Svg(
        960,
        560,
        theme,
        "Why High-NA EUV optics are anamorphic",
        "Three panels show the cone of light that illuminates a reflective EUV mask and the cone "
        "it reflects, around the mask normal. Left: 0.33 NA at 4x reduction, the mask-side cone "
        "half-angle is 4.7 degrees and, tilted by a 6 degree chief-ray angle, the two cones do not "
        "overlap. Middle: 0.55 NA at 4x would need a 7.9 degree half-angle, so the incident and "
        "reflected cones overlap around the normal. Right: with 8x reduction in the scan direction "
        "the half-angle drops to 3.9 degrees and the cones separate again; the other direction keeps "
        "4x. All panels use an illustrative 6 degree illumination angle. Angles are drawn three times "
        "larger than real.",
    )
    t = s.t
    panels = [
        ("NXE: NA 0.33, 4×", 0.33, 4.0, CRA_DEG, "cones separate", False),
        ("NA 0.55 at 4× (not built)", 0.55, 4.0, CRA_DEG, "cones overlap: the mask would\nshadow its own light path", True),
        ("EXE: NA 0.55, 8× in scan", 0.55, 8.0, CRA_DEG, "cones separate again", False),
    ]
    pw = 300
    for i, (label, na, mag, cra, verdict, bad) in enumerate(panels):
        x0 = 30 + i * (pw + 15)
        cx = x0 + pw / 2
        ym = 370  # mask surface
        s.rect(x0, 20, pw, 455, t["panel"], rx=10)
        s.text(cx, 50, label, size=14.5, weight="bold")
        alpha = math.degrees(math.asin(na / mag))
        s.text(
            cx,
            74,
            f"cone half-angle asin(NA/M) = {alpha:.1f}°\nillumination ≈ {cra:g}° (illustrative)",
            size=11.5,
            color=t["ink2"],
        )
        # mask
        s.rect(cx - 120, ym, 240, 16, t["mask_fill"], stroke=t["mask"], width=1.2)
        for k in range(1, 4):
            s.line(cx - 120, ym + 4 * k, cx + 120, ym + 4 * k, color=t["mask"], width=0.6, opacity=0.6)
        s.text(cx, ym + 34, "reflective multilayer mask", size=11, color=t["muted"])
        # normal
        s.line(cx, ym, cx, 118, color=t["muted"], width=1.2, dash="4 4")
        s.text(cx + 4, 128, "normal", size=10, color=t["muted"], anchor="start")
        length = 225.0

        def ray(theta_deg: float) -> tuple[float, float]:
            th = math.radians(ANGLE_GAIN * theta_deg)
            return cx + length * math.sin(th), ym - length * math.cos(th)

        # incident cone (from upper left) and reflected cone (to upper right)
        inc = [(cx, ym), ray(-cra - alpha), ray(-cra + alpha)]
        ref = [(cx, ym), ray(cra - alpha), ray(cra + alpha)]
        s.poly(inc, t["duv_fill"], stroke=t["duv"], width=1.5, fill_opacity=0.85)
        s.poly(ref, t["euv_fill"], stroke=t["euv"], width=1.5, fill_opacity=0.85)
        # chief rays
        xi, yi = ray(-cra)
        xr, yr = ray(cra)
        s.line(xi, yi, cx, ym, color=t["duv"], width=2, arrow=True)
        s.line(cx, ym, xr, yr, color=t["euv"], width=2, arrow=True)
        s.text(cx - 58, 142, "from\nilluminator", size=11, color=t["ink2"], anchor="end")
        s.text(cx + 58, 142, "to projection\noptics", size=11, color=t["ink2"], anchor="start")
        if bad:
            ov = [(cx, ym), ray(-(alpha - cra)), ray(alpha - cra)]
            s.poly(ov, t["bad_fill"], stroke=t["bad"], width=2)
        vcol = t["bad"] if bad else t["ink"]
        s.text(cx, 430, verdict, size=12, color=vcol, weight="bold")
    s.text(
        480,
        494,
        f"Plane of incidence = scan direction; angles drawn {ANGLE_GAIN:g}× larger than real.\n"
        "Across the slit the optics keep 4× (no overlap problem there), so the wafer field\n"
        "is 26 mm wide but only 16.5 mm long in the scan direction.",
        size=11,
        color=t["muted"],
    )
    return s


# ---------------------------------------------------------------------------
# 2. SSMB storage ring concept
# ---------------------------------------------------------------------------
def fig_ssmb_concept(theme: str) -> Svg:
    s = Svg(
        960,
        540,
        theme,
        "Steady-state microbunching (SSMB) storage-ring concept",
        "A racetrack storage ring. In the lower straight a phase-locked laser, steered onto the beam "
        "axis by a mirror, co-propagates with the electrons through a modulator undulator and imprints "
        "an energy modulation at the laser wavelength. The ring lattice turns that energy modulation into "
        "density microbunches, much shorter than the laser wavelength, at a radiator undulator in the "
        "upper straight, where the beam radiates coherently at a harmonic of the laser; the lattice then "
        "undoes and re-forms the bunching on every turn. An RF cavity replaces the radiated energy. An "
        "inset shows electron density versus position with sharp microbunches spaced by the laser wavelength.",
    )
    t = s.t
    x1, x2, yt, yb, r = 190, 590, 140, 380, 120
    d = f"M{x1},{yb} L{x2},{yb} A{r},{r} 0 0 0 {x2},{yt} L{x1},{yt} A{r},{r} 0 0 0 {x1},{yb} Z"
    s.path(d, stroke=t["ebeam"], width=3)
    s.line(250, yb, 280, yb, color=t["ebeam"], width=3, arrow=True)
    s.line(560, yt, 530, yt, color=t["ebeam"], width=3, arrow=True)
    
    # RF cavity (lower-left straight)
    s.rect(196, yb - 14, 44, 28, t["rf_fill"], stroke=t["rf"], width=1.5, rx=6)
    s.text(218, yb + 36, "RF cavity", size=12, color=t["ink2"])
    # modulator in lower straight
    undulator(s, 390, yb, 150, 10)
    s.text(465, yb - 30, "modulator", size=14, weight="bold")
    # laser: source box below, mirror, then collinear through the modulator
    s.rect(262, yb + 38, 96, 28, t["laser_fill"], stroke=t["laser"], width=1.5, rx=5)
    s.text(310, yb + 57, "laser", size=12.5)
    s.path(wavy(310, yb + 38, 310, 0, 10) + f" L310,{yb + 6}", stroke=t["laser"], width=2)
    s.line(300, yb + 16, 322, yb - 6, color=t["ink2"], width=2.5)  # mirror
    s.path(wavy(316, yb, 545, 3.5, 16), stroke=t["laser"], width=2)
    s.text(
        372,
        yb + 60,
        "phase-locked, ≈ 1 µm: imprints\nan energy modulation",
        size=12,
        anchor="start",
        color=t["ink2"],
    )
    # radiator in upper straight
    undulator(s, 300, yt, 170, 12)
    s.text(385, yt + 44, "radiator", size=14, weight="bold")
    s.path(wavy(300, yt - 32, 480, 3, 7), stroke=t["euv"], width=2)
    s.line(470, yt - 32, 930, yt - 32, color=t["euv"], width=3, arrow=True)
    s.text(940, yt - 84, "coherent EUV at a laser harmonic\n(simulator preset: 1053 nm / 78 = 13.5 nm)", size=12, anchor="end", color=t["ink2"])
    s.text(
        390,
        250,
        "ring lattice (longitudinal focusing):\nenergy modulation → microbunches at the radiator,\nundone and re-formed on every turn",
        size=13,
    )
    # inset: microbunched density
    ix, iy, iw, ih = 735, 250, 205, 150
    s.rect(ix, iy, iw, ih, t["panel"], rx=8)
    s.text(ix + iw / 2, iy + 22, "density at the radiator", size=12.5, weight="bold")
    base = iy + ih - 32
    s.line(ix + 12, base, ix + iw - 12, base, color=t["muted"], width=1)
    pts = []
    n = 220
    for k in range(n + 1):
        xx = ix + 12 + (iw - 24) * k / n
        ph = 2 * math.pi * 4.3 * k / n
        val = math.exp(-(((ph % (2 * math.pi)) - math.pi) ** 2) / 0.02)
        pts.append(f"{xx:.1f},{base - 78 * val:.1f}")
    s.path("M" + " L".join(pts), stroke=t["ebeam"], width=1.8)
    s.text(ix + iw / 2, base + 20, "spacing = laser wavelength", size=11.5, color=t["ink2"])
    s.text(
        30,
        510,
        "The stored beam (hundreds of MeV) passes both undulators on every turn. Coherent power ∝ N²|bₙ|² (N electrons,\n"
        "bunching factor bₙ at harmonic n) versus ∝ N without microbunching. Schematic, not to scale.",
        size=12,
        anchor="start",
        color=t["muted"],
    )
    return s


# ---------------------------------------------------------------------------
# 3. Energy-recovery-linac FEL loop
# ---------------------------------------------------------------------------
def fig_erl_fel_loop(theme: str) -> Svg:
    s = Svg(
        960,
        520,
        theme,
        "Energy-recovery linac (ERL) driven free-electron laser",
        "An electron injector feeds a superconducting linac through a merger. The beam is accelerated, "
        "turned by an arc, and lased in a long undulator that emits EUV. A return arc brings the spent "
        "beam back to the linac half an RF period late, so it is decelerated and hands its energy back to "
        "the RF field before going to a low-energy beam dump.",
    )
    t = s.t
    yb, yt = 370, 140
    xr = 760
    s.rect(300, yb - 26, 330, 52, t["rf_fill"], stroke=t["rf"], width=1.5, rx=10)
    for k in range(8):
        s.ellipse(330 + k * 41, yb, 14, 18, "none", stroke=t["rf"], width=1.2)
    s.text(470, yb - 52, "superconducting linac: accelerates on pass 1,\ndecelerates on pass 2", size=12.5)
    s.rect(40, yb + 46, 100, 40, t["panel"], stroke=t["line"], width=1.2, rx=6)
    s.text(90, yb + 71, "injector", size=13)
    s.path(f"M140,{yb + 66} C200,{yb + 66} 240,{yb + 6} 300,{yb + 6}", stroke=t["ebeam"], width=2.5, arrow=True)
    s.text(218, yb + 62, "merger", size=11.5, color=t["ink2"], anchor="start")
    s.line(630, yb - 6, xr, yb - 6, color=t["ebeam"], width=2.5)
    s.path(f"M{xr},{yb - 6} A{(yb - 6 - yt) / 2},{(yb - 6 - yt) / 2} 0 0 0 {xr},{yt}", stroke=t["ebeam"], width=2.5)
    s.line(xr, yt, 250, yt, color=t["ebeam"], width=2.5)
    s.path(f"M250,{yt} A{(yb - 6 - yt) / 2},{(yb - 6 - yt) / 2} 0 0 0 250,{yb - 6}", stroke=t["ebeam"], width=2.5)
    s.line(250, yb - 6, 300, yb - 6, color=t["ebeam"], width=2.5, arrow=True)
    s.line(700, yt, 670, yt, color=t["ebeam"], width=2.5, arrow=True)
    s.text(xr - 14, 234, "arc", size=12, color=t["ink2"], anchor="end")
    s.text(264, 226, "return arc:\narrives ½ RF period late", size=12, color=t["ink2"], anchor="start")
    undulator(s, 330, yt, 280, 18)
    s.text(470, yt + 44, "undulator (FEL)", size=14, weight="bold")
    s.path(wavy(330, yt - 34, 610, 3, 7), stroke=t["euv"], width=2)
    s.line(600, yt - 34, 930, yt - 34, color=t["euv"], width=3, arrow=True)
    s.text(930, yt - 58, "EUV to one or several scanners", size=12.5, anchor="end", color=t["ink2"])
    s.line(630, yb + 6, 820, yb + 6, color=t["ebeam"], width=2.5, arrow=True)
    s.rect(825, yb - 12, 100, 36, t["panel"], stroke=t["line"], width=1.2, rx=6)
    s.text(875, yb + 11, "beam dump", size=12.5)
    s.text(875, yb + 44, "(low energy)", size=11.5, color=t["ink2"])
    s.text(
        30,
        486,
        "Energy recovery lets the beam current (and so the average FEL power) be far higher\n"
        "than the RF power a single-pass linac could supply. Schematic, not to scale.",
        size=12,
        anchor="start",
        color=t["muted"],
    )
    return s


# ---------------------------------------------------------------------------
# 4. Inverse Compton scattering, head-on
# ---------------------------------------------------------------------------
def fig_ics_collision(theme: str) -> Svg:
    s = Svg(
        960,
        470,
        theme,
        "Inverse Compton scattering, head-on geometry",
        "Top row, before the collision: an electron bunch travelling to the right and a laser pulse "
        "travelling to the left approach the interaction point. Bottom row, after: photons scattered "
        "off the relativistic electrons leave in a forward cone of half-angle about one over gamma, their "
        "wavelength shortened by about four gamma squared. Example: a 1030 nm laser and 1.72 MeV electrons "
        "give 13.5 nm on axis, in a cone of about 13 degrees.",
    )
    t = s.t
    # --- before
    y1 = 95
    s.text(30, 36, "before", size=13, anchor="start", color=t["muted"], weight="bold")
    s.ellipse(230, y1, 60, 22, t["ebeam_fill"], stroke=t["ebeam"], width=2)
    for k in range(14):
        s.circle(190 + (k * 37) % 80, y1 - 12 + (k * 23) % 24, 3, t["ebeam"])
    s.line(300, y1, 360, y1, color=t["ebeam"], width=3, arrow=True)
    s.text(230, y1 + 45, "electron bunch, Lorentz factor γ", size=13.5)
    s.path(packet(680, y1, 70, 18, 16), stroke=t["laser"], width=2.2)
    s.line(600, y1, 540, y1, color=t["laser"], width=3, arrow=True)
    s.text(680, y1 + 45, "laser pulse, wavelength λₗ", size=13.5)
    s.circle(460, y1, 6, "none", stroke=t["ink"], width=1.5)
    s.text(460, y1 - 22, "interaction point", size=12.5, color=t["ink2"])
    s.line(30, 172, 930, 172, color=t["faint"], width=1)
    # --- after
    y2 = 280
    s.text(30, 200, "after", size=13, anchor="start", color=t["muted"], weight="bold")
    half = math.radians(13.0)
    L = 400
    cx = 460
    s.poly(
        [(cx, y2), (cx + L * math.cos(half), y2 - L * math.sin(half)), (cx + L * math.cos(half), y2 + L * math.sin(half) * 0.001 + L * math.sin(half))],
        t["euv_fill"],
        stroke=t["euv"],
        width=1.5,
        fill_opacity=0.6,
    )
    s.line(cx, y2, cx + L, y2, color=t["euv"], width=1.2, dash="5 4")
    s.ellipse(cx + 250, y2, 40, 16, t["ebeam_fill"], stroke=t["ebeam"], width=1.5)
    s.text(cx + 250, y2 + 36, "electrons (slightly slower)", size=12, color=t["ink2"])
    s.text(cx + L - 12, y2 - 60, "scattered photons,\ncone half-angle ≈ 1/γ", size=13, anchor="end")
    s.path(packet(300, y2, 60, 14, 16), stroke=t["laser"], width=2, opacity=0.5)
    s.text(300, y2 + 40, "unscattered laser light", size=12, color=t["ink2"])
    s.text(
        480,
        390,
        "λₓ ≈ λₗ (1 + a₀²/2 + γ²θ²) / 4γ²",
        size=17,
        weight="bold",
    )
    s.text(
        480,
        418,
        "e.g. 1030 nm laser, 1.72 MeV electrons (γ ≈ 4.37) → 13.5 nm on axis; 1/γ ≈ 0.23 rad ≈ 13°",
        size=13,
        color=t["ink2"],
    )
    s.text(480, 452, "Head-on, single scattering; a₀ = laser strength parameter. Schematic, not to scale.", size=12, color=t["muted"])
    return s


FIGURES = {
    "anamorphic-why": fig_anamorphic_why,
    "ssmb-concept": fig_ssmb_concept,
    "erl-fel-loop": fig_erl_fel_loop,
    "ics-collision": fig_ics_collision,
}


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, fn in FIGURES.items():
        for theme in ("light", "dark"):
            path = OUT / f"{name}-{theme}.svg"
            path.write_text(fn(theme).render())
            print("wrote", path.relative_to(OUT.parents[2]))


if __name__ == "__main__":
    main()
