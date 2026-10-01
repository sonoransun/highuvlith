#!/usr/bin/env python3
"""SVG schematics for the History section of the highuvlith site (docs/history/).

Run from the repository root (numpy is the only dependency):

    python docs/figures/history/make_history_schematics.py

Every figure is written twice into docs/assets/images/history/: ``<name>-light.svg`` and
``<name>-dark.svg``. Backgrounds are transparent and the ink is chosen for Material's
light ("default") and dark ("slate") schemes; pages pick the right file with the
``#only-light`` / ``#only-dark`` URL suffixes.

All drawings are schematics and are not to scale. The only computed content is the set of
1D scalar intensity profiles in ``printing-modes``: Fresnel propagation across a gap for
proximity printing and a partially coherent low-pass image for projection. They show the
shape of each effect in arbitrary units; they are not measured data.
"""

from __future__ import annotations

import math
from pathlib import Path

import numpy as np

OUT = Path(__file__).resolve().parents[2] / "assets" / "images" / "history"

FONT = "system-ui, -apple-system, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif"
# The figures are drawn 860-960 units wide but shown in a ~700 px content column, so all
# text is scaled up to stay legible at that size.
TEXT_SCALE = 1.12

# Ink and fills for each colour scheme. Series hues follow the site's chart palette
# (#2a78d6 / #eb6834 / #1baf7a in light, #3987e5 / #d95926 / #199e70 in dark); light
# rays use the brand violet (DUV) and cyan (EUV).
THEMES = {
    "light": {
        "ink": "#1d1c1a",
        "ink2": "#52514e",
        "muted": "#6b6a66",
        "line": "#52514e",
        "faint": "#d9d8d2",
        "uv": "#5b36c9",
        "euv": "#0e7490",
        "ir": "#c0392b",
        "glass_fill": "#e3eefb",
        "glass": "#2a78d6",
        "chrome": "#33322f",
        "resist_fill": "#fbd5bf",
        "resist": "#c8551f",
        "wafer_fill": "#d6d4cc",
        "wafer": "#8a8780",
        "water_fill": "#d4eef8",
        "water": "#0e7490",
        "mirror_fill": "#bcd3f2",
        "mirror": "#2a78d6",
        "spacer_fill": "#b9ead7",
        "spacer": "#128a5f",
        "mandrel_fill": "#ddd3fb",
        "mandrel": "#5b36c9",
        "accent": "#eb6834",
        "plot": "#2a78d6",
        "tin": "#6b6a66",
    },
    "dark": {
        "ink": "#f1f0ee",
        "ink2": "#c3c2b7",
        "muted": "#a3a29b",
        "line": "#c3c2b7",
        "faint": "#4a4852",
        "uv": "#b5a1ff",
        "euv": "#22d3ee",
        "ir": "#ff7b6b",
        "glass_fill": "#1f3552",
        "glass": "#6aa6ee",
        "chrome": "#0e0e10",
        "resist_fill": "#6b3219",
        "resist": "#f08a5d",
        "wafer_fill": "#4a4852",
        "wafer": "#a3a29b",
        "water_fill": "#123a4a",
        "water": "#22d3ee",
        "mirror_fill": "#27456e",
        "mirror": "#6aa6ee",
        "spacer_fill": "#135c43",
        "spacer": "#4fd1a1",
        "mandrel_fill": "#3b2d73",
        "mandrel": "#b5a1ff",
        "accent": "#f08a5d",
        "plot": "#3987e5",
        "tin": "#c3c2b7",
    },
}


def esc(s: str) -> str:
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


class Svg:
    """Minimal SVG writer: collects elements, emits arrow markers on demand."""

    def __init__(self, w: int, h: int, title: str, desc: str, theme: dict[str, str]):
        self.w, self.h, self.t = w, h, theme
        self.title, self.desc = title, desc
        self.parts: list[str] = []
        self.markers: dict[str, str] = {}

    # -- primitives ------------------------------------------------------------------
    def add(self, s: str) -> None:
        self.parts.append(s)

    def arrow(self, color: str) -> str:
        mid = "a" + color.lstrip("#")
        self.markers[mid] = color
        return f"url(#{mid})"

    def text(self, x, y, s, size=14, anchor="middle", color=None, weight="normal",
             italic=False) -> None:
        color = color or self.t["ink"]
        style = ' font-style="italic"' if italic else ""
        self.add(
            f'<text x="{x:.1f}" y="{y:.1f}" font-size="{size * TEXT_SCALE:.1f}" text-anchor="{anchor}" '
            f'fill="{color}" font-weight="{weight}"{style}>{esc(s)}</text>'
        )

    def lines(self, x, y, rows, size=13, anchor="middle", color=None, dy=None) -> None:
        dy = dy or size * 1.3
        for i, row in enumerate(rows):
            self.text(x, y + i * dy, row, size=size, anchor=anchor, color=color)

    def rect(self, x, y, w, h, fill, stroke="none", sw=1.5, rx=0, opacity=1.0) -> None:
        op = f' fill-opacity="{opacity}"' if opacity < 1 else ""
        self.add(
            f'<rect x="{x:.1f}" y="{y:.1f}" width="{w:.1f}" height="{h:.1f}" rx="{rx}" '
            f'fill="{fill}"{op} stroke="{stroke}" stroke-width="{sw}"/>'
        )

    def line(self, x1, y1, x2, y2, color, sw=2.0, dash=None, arrow=False,
             arrow_start=False) -> None:
        extra = f' stroke-dasharray="{dash}"' if dash else ""
        if arrow:
            extra += f' marker-end="{self.arrow(color)}"'
        if arrow_start:
            extra += f' marker-start="{self.arrow(color)}"'
        self.add(
            f'<line x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" stroke="{color}" '
            f'stroke-width="{sw}" stroke-linecap="round"{extra}/>'
        )

    def poly(self, pts, color, sw=2.0, fill="none", dash=None, arrow=False,
             close=False, opacity=1.0) -> None:
        d = " ".join(f"{x:.1f},{y:.1f}" for x, y in pts)
        tag = "polygon" if close else "polyline"
        extra = f' stroke-dasharray="{dash}"' if dash else ""
        if arrow:
            extra += f' marker-end="{self.arrow(color)}"'
        op = f' fill-opacity="{opacity}"' if opacity < 1 else ""
        self.add(
            f'<{tag} points="{d}" fill="{fill}"{op} stroke="{color}" stroke-width="{sw}" '
            f'stroke-linejoin="round" stroke-linecap="round"{extra}/>'
        )

    def path(self, d, fill="none", stroke="none", sw=1.5, opacity=1.0, dash=None,
             arrow=False) -> None:
        op = f' fill-opacity="{opacity}"' if opacity < 1 else ""
        extra = f' stroke-dasharray="{dash}"' if dash else ""
        if arrow:
            extra += f' marker-end="{self.arrow(stroke)}"'
        self.add(
            f'<path d="{d}" fill="{fill}"{op} stroke="{stroke}" stroke-width="{sw}" '
            f'stroke-linejoin="round" stroke-linecap="round"{extra}/>'
        )

    def circle(self, cx, cy, r, fill="none", stroke="none", sw=1.5, opacity=1.0) -> None:
        op = f' fill-opacity="{opacity}"' if opacity < 1 else ""
        self.add(
            f'<circle cx="{cx:.1f}" cy="{cy:.1f}" r="{r:.1f}" fill="{fill}"{op} '
            f'stroke="{stroke}" stroke-width="{sw}"/>'
        )

    def render(self) -> str:
        defs = "".join(
            f'<marker id="{mid}" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" '
            f'markerHeight="6" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" '
            f'fill="{c}"/></marker>'
            for mid, c in sorted(self.markers.items())
        )
        head = (
            f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {self.w} {self.h}" '
            f'width="{self.w}" height="{self.h}" role="img" aria-labelledby="t d" '
            f'font-family="{FONT}">\n<title id="t">{esc(self.title)}</title>\n'
            f'<desc id="d">{esc(self.desc)}</desc>\n<defs>{defs}</defs>\n'
        )
        return head + "\n".join(self.parts) + "\n</svg>\n"


# ---------------------------------------------------------------------------------------
# 1. Contact vs proximity vs projection printing
# ---------------------------------------------------------------------------------------

OPENINGS = (-60.0, 0.0, 60.0)  # mask openings (drawing units), 26 wide, 60 pitch
OPEN_W = 26.0


def printing_profiles() -> dict[str, tuple[np.ndarray, np.ndarray]]:
    """1D scalar intensity just above the resist for the three printing modes."""
    n, domain = 8192, 1600.0
    x = (np.arange(n) - n / 2) * (domain / n)
    u0 = np.zeros(n)
    for c in OPENINGS:
        u0[np.abs(x - c) <= OPEN_W / 2] = 1.0
    f = np.fft.fftfreq(n, d=domain / n)
    spec = np.fft.fft(u0)

    # Contact: the mask pattern itself, with a sub-pixel blur for drawing.
    k = np.exp(-0.5 * (x / 0.8) ** 2)
    contact = np.real(np.fft.ifft(np.fft.fft(u0) * np.fft.fft(np.fft.ifftshift(k / k.sum()))))

    # Proximity: paraxial Fresnel propagation over the gap, H(f) = exp(-i pi lambda g f^2),
    # with sqrt(lambda g) = 15 drawing units (Fresnel number w^2/(lambda g) ~ 3), averaged
    # over a +-20 % spread of lambda g as a broadband mercury lamp would do.
    prox = np.zeros(n)
    spread = np.linspace(0.8, 1.2, 9) * 15.0**2
    for lam_g in spread:
        prox += np.abs(np.fft.ifft(spec * np.exp(-1j * np.pi * lam_g * f**2))) ** 2
    prox /= len(spread)

    # Projection: partially coherent 1D image, pupil cutoff 1.25/pitch, sigma 0.5
    # (incoherent sum over 21 tilted plane waves).
    fc = 1.25 / 60.0
    proj = np.zeros(n)
    tilts = np.linspace(-0.5, 0.5, 21) * fc
    for s in tilts:
        proj += np.abs(np.fft.ifft(spec * (np.abs(f + s) <= fc))) ** 2
    proj /= len(tilts)

    keep = np.abs(x) <= 100
    return {
        "mask": (x[keep], u0[keep]),
        "contact": (x[keep], contact[keep]),
        "proximity": (x[keep], prox[keep]),
        "projection": (x[keep], proj[keep]),
    }


def draw_mask(s: Svg, cx: float, y_glass: float, glass_h: float = 24, chrome_h: float = 6,
              half: float = 100) -> float:
    """Glass plate with chrome on its underside; returns the y of the chrome bottom."""
    t = s.t
    s.rect(cx - half, y_glass, 2 * half, glass_h, t["glass_fill"], t["glass"], 1.5)
    y_c = y_glass + glass_h
    edges = [cx - half]
    for c in OPENINGS:
        edges += [cx + c - OPEN_W / 2, cx + c + OPEN_W / 2]
    edges.append(cx + half)
    for a, b in zip(edges[0::2], edges[1::2]):
        s.rect(a, y_c, b - a, chrome_h, t["chrome"], t["line"], 1)
    return y_c + chrome_h


def draw_wafer(s: Svg, cx: float, y_top: float, half: float = 110, resist_h: float = 12,
               wafer_h: float = 22) -> None:
    t = s.t
    s.rect(cx - half, y_top, 2 * half, resist_h, t["resist_fill"], t["resist"], 1.2)
    s.rect(cx - half, y_top + resist_h, 2 * half, wafer_h, t["wafer_fill"], t["wafer"], 1.2)


def draw_rays_down(s: Svg, cx: float, y0: float, y1: float, half: float = 90, n: int = 7) -> None:
    for i in range(n):
        x = cx - half + i * (2 * half) / (n - 1)
        s.line(x, y0, x, y1, s.t["uv"], sw=2, arrow=True)


def draw_profile(s: Svg, cx: float, base: float, prof, ideal, scale: float = 38.0) -> None:
    t = s.t
    s.line(cx - 104, base, cx + 104, base, t["faint"], sw=1)
    xi, ui = ideal
    pts_i = [(cx + x, base - scale * u) for x, u in zip(xi[::4], ui[::4])]
    s.poly(pts_i, t["muted"], sw=1.2, dash="3 3")
    xp, up = prof
    pts = [(cx + x, base - scale * u) for x, u in zip(xp[::2], up[::2])]
    s.poly(pts, t["plot"], sw=2)


def fig_printing_modes(theme: dict[str, str]) -> Svg:
    s = Svg(
        900, 470,
        "Contact, proximity and projection printing",
        "Three panels. Contact: the mask sits on the resist and prints a sharp copy but is "
        "damaged by the contact. Proximity: a gap g separates mask and resist; diffraction "
        "blurs the edges over roughly the square root of wavelength times gap. Projection: "
        "a lens images the mask onto the wafer with resolution k1 lambda over NA. Below each "
        "panel, the light intensity reaching the resist is plotted against the mask pattern.",
        theme,
    )
    t = theme
    prof = printing_profiles()
    wafer_y = 246
    for i, (name, title) in enumerate(
        [("contact", "Contact printing"), ("proximity", "Proximity printing"),
         ("projection", "Projection printing")]
    ):
        cx = 190 + 280 * i
        s.text(cx, 30, title, size=17, weight="bold")
        if name == "contact":
            draw_rays_down(s, cx, 96, 190)
            y_c = draw_mask(s, cx, wafer_y - 30)
            draw_wafer(s, cx, y_c)
            s.text(cx + 108, wafer_y - 34, "mask", size=12, anchor="start", color=t["ink2"])
        elif name == "proximity":
            draw_rays_down(s, cx, 96, 160)
            gap = 34
            y_c = draw_mask(s, cx, wafer_y - 30 - gap)
            draw_wafer(s, cx, wafer_y)
            gx = cx - 122
            s.line(gx, y_c + 2, gx, wafer_y - 2, t["ink2"], sw=1.5, arrow=True, arrow_start=True)
            s.text(gx - 6, (y_c + wafer_y) / 2 + 5, "g", size=15, anchor="end", italic=True)
            s.text(cx + 108, y_c - 34, "mask", size=12, anchor="start", color=t["ink2"])
        else:
            draw_rays_down(s, cx, 44, 76, half=80, n=6)
            y_c = draw_mask(s, cx, 80)
            ly = 166
            # Lens: a biconvex element.
            s.path(
                f"M{cx - 96},{ly} Q{cx},{ly - 34} {cx + 96},{ly} Q{cx},{ly + 34} {cx - 96},{ly} Z",
                fill=t["glass_fill"], stroke=t["glass"], sw=1.5,
            )
            s.text(cx + 104, ly + 5, "lens", size=12, anchor="start", color=t["ink2"])
            # Light from the whole mask field is collected by the lens and converges onto a
            # smaller image field on the wafer (reduction imaging).
            s.poly([(cx - 100, y_c), (cx + 100, y_c), (cx + 92, ly), (cx + 45, wafer_y),
                    (cx - 45, wafer_y), (cx - 92, ly)], t["uv"], sw=1.2, fill=t["uv"],
                   close=True, opacity=0.12)
            draw_wafer(s, cx, wafer_y, half=70)
            s.line(cx - 45, wafer_y - 4, cx + 45, wafer_y - 4, t["uv"], sw=3)
            s.text(cx + 108, 90, "mask", size=12, anchor="start", color=t["ink2"])
            s.text(cx + 78, wafer_y + 9, "image field", size=12, anchor="start",
                   color=t["ink2"])
            s.text(cx + 78, wafer_y + 24, "(reduced)", size=12, anchor="start",
                   color=t["ink2"])
        if i == 0:
            s.text(cx - 116, wafer_y + 10, "resist", size=12, anchor="end", color=t["ink2"])
            s.text(cx - 116, wafer_y + 28, "wafer", size=12, anchor="end", color=t["ink2"])
        base = 360
        draw_profile(s, cx, base, prof[name], prof["mask"])
        if i == 0:
            s.text(cx - 104, base - 62, "intensity at the resist", size=12, anchor="start",
                   color=t["ink2"])
    captions = {
        0: ["Mask pressed onto the resist:", "sharp copy, but every contact", "damages mask and wafer."],
        1: ["A gap g protects the mask;", "diffraction blurs edges", "over roughly √(λg)."],
        2: ["A lens images the mask;", "R = k₁λ/NA, and nothing", "touches the wafer."],
    }
    for i, rows in captions.items():
        s.lines(190 + 280 * i, 392, rows, size=13, color=t["ink2"])
    s.text(896, 464, "Schematic, not to scale. Dashed: mask pattern; solid: 1D scalar model "
           "(arbitrary units).", size=11, anchor="end", color=t["muted"])
    return s


# ---------------------------------------------------------------------------------------
# 2. Stepper vs scanner
# ---------------------------------------------------------------------------------------


def draw_wafer_map(s: Svg, cx: float, cy: float, r: float, fw: float, fh: float,
                   current: tuple[int, int], exposed: set[tuple[int, int]]) -> dict:
    t = s.t
    s.circle(cx, cy, r, fill=t["wafer_fill"], stroke=t["wafer"], sw=1.5)
    cells = {}
    nx, ny = int(2 * r // fw) + 2, int(2 * r // fh) + 2
    for i in range(-nx // 2, nx // 2 + 1):
        for j in range(-ny // 2, ny // 2 + 1):
            x0, y0 = cx + (i - 0.5) * fw, cy + (j - 0.5) * fh
            corners = [(x0, y0), (x0 + fw, y0), (x0, y0 + fh), (x0 + fw, y0 + fh)]
            if all(math.hypot(px - cx, py - cy) <= r - 2 for px, py in corners):
                fill = t["resist_fill"] if (i, j) in exposed else "none"
                s.rect(x0, y0, fw, fh, fill, t["wafer"], 1)
                cells[(i, j)] = (x0, y0)
    x0, y0 = cells[current]
    s.rect(x0, y0, fw, fh, "none", t["accent"], 3)
    return cells


def fig_stepper_scanner(theme: dict[str, str]) -> Svg:
    s = Svg(
        900, 500,
        "Stepper versus scanner",
        "Left: a step-and-repeat stepper illuminates the whole reticle field at once, prints "
        "one field on the wafer, then steps the wafer to the next field. Right: a "
        "step-and-scan system illuminates only a slit; reticle and wafer scan through it in "
        "opposite directions, the reticle four times faster for a 4x lens, so a lens corrected "
        "over the slit alone prints a large field.",
        theme,
    )
    t = theme
    # ---- stepper
    cx = 215
    s.text(cx, 30, "Stepper (step-and-repeat)", size=17, weight="bold")
    s.rect(cx - 60, 52, 120, 70, t["glass_fill"], t["glass"], 1.5)
    s.rect(cx - 60, 52, 120, 70, t["uv"], "none", 0, opacity=0.22)
    for k in range(4):
        s.rect(cx - 44 + 24 * k, 64, 10, 46, t["chrome"], "none", 0)
    s.text(cx + 70, 80, "reticle:", size=12, anchor="start", color=t["ink2"])
    s.text(cx + 70, 96, "whole field lit", size=12, anchor="start", color=t["ink2"])
    s.text(cx + 70, 112, "in one flash", size=12, anchor="start", color=t["ink2"])
    s.path(f"M{cx - 70},{150} L{cx + 70},{150} L{cx + 40},{196} L{cx - 40},{196} Z",
           fill=t["glass_fill"], stroke=t["glass"], sw=1.5)
    s.text(cx, 178, "projection lens", size=12, color=t["ink2"])
    s.line(cx - 60, 124, cx - 66, 150, t["uv"], sw=1.2)
    s.line(cx + 60, 124, cx + 66, 150, t["uv"], sw=1.2)
    cells = draw_wafer_map(s, cx, 330, 118, 34, 34, (0, 0),
                           {(-2, -1), (-1, -1), (0, -1), (1, -1), (2, -1), (-2, -2), (-1, -2),
                            (0, -2), (1, -2), (2, -2), (-3, 0), (-2, 0), (-1, 0)})
    x0, y0 = cells[(0, 0)]
    s.line(cx - 40, 196, x0 + 2, y0 + 2, t["uv"], sw=1.2)
    s.line(cx + 40, 196, x0 + 32, y0 + 2, t["uv"], sw=1.2)
    # step arrows along the row
    y_row = y0 + 17
    s.line(x0 + 36, y_row, x0 + 62, y_row, t["accent"], sw=2.5, arrow=True)
    s.text(cx + 128, 312, "expose a field,", size=12, anchor="start", color=t["ink2"])
    s.text(cx + 128, 328, "then step the", size=12, anchor="start", color=t["ink2"])
    s.text(cx + 128, 344, "wafer to the next", size=12, anchor="start", color=t["ink2"])
    # ---- scanner
    cx = 630
    s.text(cx, 30, "Scanner (step-and-scan)", size=17, weight="bold")
    s.rect(cx - 60, 52, 120, 70, t["glass_fill"], t["glass"], 1.5)
    for k in range(4):
        s.rect(cx - 44 + 24 * k, 64, 10, 46, t["chrome"], "none", 0)
    s.rect(cx - 60, 80, 120, 16, t["uv"], "none", 0, opacity=0.45)
    # The reticle moves along the scan axis (perpendicular to the slit), 4x faster than
    # the wafer and, for an image-inverting lens, in the opposite direction.
    s.line(cx - 76, 110, cx - 76, 66, t["accent"], sw=2.5, arrow=True)
    s.text(cx - 84, 84, "reticle", size=12, anchor="end", color=t["ink2"])
    s.text(cx - 84, 100, "scans at 4v", size=12, anchor="end", color=t["ink2"])
    s.text(cx + 70, 84, "slit of light", size=12, anchor="start", color=t["ink2"])
    s.text(cx + 70, 100, "across the field", size=12, anchor="start", color=t["ink2"])
    s.path(f"M{cx - 70},{150} L{cx + 70},{150} L{cx + 40},{196} L{cx - 40},{196} Z",
           fill=t["glass_fill"], stroke=t["glass"], sw=1.5)
    s.text(cx, 178, "projection lens", size=12, color=t["ink2"])
    s.line(cx - 60, 96, cx - 66, 150, t["uv"], sw=1.2)
    s.line(cx + 60, 96, cx + 66, 150, t["uv"], sw=1.2)
    cells = draw_wafer_map(s, cx, 330, 118, 26, 33, (0, 0),
                           {(i, j) for i in range(-4, 5) for j in (-3, -2, -1)}
                           | {(-4, 0), (-3, 0), (-2, 0), (-1, 0)})
    x0, y0 = cells[(0, 0)]
    s.rect(x0, y0 + 12, 26, 8, t["uv"], "none", 0, opacity=0.6)
    s.line(cx - 40, 196, x0 + 1, y0 + 12, t["uv"], sw=1.2)
    s.line(cx + 40, 196, x0 + 25, y0 + 12, t["uv"], sw=1.2)
    s.line(cx + 132, 356, cx + 132, 396, t["accent"], sw=2.5, arrow=True)
    s.text(cx + 142, 372, "wafer scans", size=12, anchor="start", color=t["ink2"])
    s.text(cx + 142, 388, "at v", size=12, anchor="start", color=t["ink2"])
    s.text(cx + 128, 290, "26 × 33 mm field", size=12, anchor="start", color=t["ink2"])
    s.text(cx + 128, 306, "printed by sweeping", size=12, anchor="start", color=t["ink2"])
    s.text(cx + 128, 322, "the slit across it", size=12, anchor="start", color=t["ink2"])
    s.text(450, 490, "Schematic, not to scale. Orange frame: field being exposed; shaded "
           "fields: already exposed.", size=11, color=t["muted"])
    return s


# ---------------------------------------------------------------------------------------
# 3. Immersion principle
# ---------------------------------------------------------------------------------------

N_GLASS = 1.56  # fused silica near 193 nm
N_WATER = 1.44  # water at 193 nm


def fig_immersion(theme: dict[str, str]) -> Svg:
    s = Svg(
        860, 420,
        "Why water immersion raises the numerical aperture",
        "Two panels showing the last lens element above a wafer. In the dry case a steep ray "
        "inside the glass is totally internally reflected at the glass-air surface, because "
        "n sin theta cannot exceed 1 in air, so NA stays below 1. With water (n about 1.44 "
        "at 193 nm) filling the gap, the same steep ray leaves the lens and reaches the "
        "wafer, so NA = n sin theta can reach about 1.35.",
        theme,
    )
    t = theme
    half, sag = 170.0, 55.0  # lens half-width; sagitta of the curved top above its rim
    for panel, (title, medium) in enumerate([("Dry: air gap", None), ("Immersion: water", N_WATER)]):
        cx = 215 + 430 * panel
        s.text(cx, 30, title, size=17, weight="bold")
        y_lb, y_w = 222, 272  # lens bottom, wafer top
        y_rim = y_lb - 40

        def top_y(x: float) -> float:  # curved top surface (parabolic cap)
            u = (x - cx) / half
            return y_rim - sag * (1 - u * u)

        # Last lens element: plano-convex, flat face down.
        s.path(f"M{cx - half},{y_lb} L{cx - half},{y_rim} Q{cx},{y_rim - 2 * sag} "
               f"{cx + half},{y_rim} L{cx + half},{y_lb} Z",
               fill=t["glass_fill"], stroke=t["glass"], sw=1.5)
        s.text(cx, y_rim - sag - 14, "last lens element (n ≈ 1.56)", size=12,
               color=t["ink2"])
        if medium:
            s.rect(cx - 150, y_lb, 300, y_w - y_lb, t["water_fill"], t["water"], 1.2)
            s.text(cx + 158, y_lb + 30, "water", size=12, anchor="start", color=t["ink2"])
        else:
            s.text(cx + 178, y_lb + 30, "air", size=12, anchor="start", color=t["ink2"])
        s.rect(cx - 190, y_w, 380, 10, t["resist_fill"], t["resist"], 1.2)
        s.rect(cx - 190, y_w + 10, 380, 22, t["wafer_fill"], t["wafer"], 1.2)
        n_med = medium or 1.0
        gap = y_w - y_lb
        for na_ray in (0.9, 1.25):
            steep = na_ray > 1.0
            color = t["accent"] if steep else t["uv"]
            # Where the ray meets the flat face: aimed at the focus on the wafer axis. A ray
            # that cannot leave the glass keeps the landing point of the water case.
            n_aim = n_med if na_ray < n_med else N_WATER
            x_hit = gap * math.tan(math.asin(na_ray / n_aim))
            th_g = math.asin(na_ray / N_GLASS)
            for sgn in (-1, 1):
                xh = cx + sgn * x_hit
                # Trace back up inside the glass until the curved top surface.
                k = 0.0
                while True:
                    xa, ya = xh + sgn * (k + 1) * math.sin(th_g), y_lb - (k + 1) * math.cos(th_g)
                    if ya <= top_y(xa) or abs(xa - cx) >= half:
                        break
                    k += 1
                x0, y0 = xh + sgn * k * math.sin(th_g), y_lb - k * math.cos(th_g)
                s.line(x0, y0, xh, y_lb, color, sw=2)
                if na_ray / n_med <= 1.0:
                    th_m = math.asin(na_ray / n_med)
                    xf = xh - sgn * gap * math.tan(th_m)
                    s.line(xh, y_lb, xf, y_w, color, sw=2, arrow=True)
                else:  # total internal reflection at the flat glass face
                    L = 0.6 * k
                    s.line(xh, y_lb, xh - sgn * L * math.sin(th_g), y_lb - L * math.cos(th_g),
                           color, sw=2, dash="5 4", arrow=True)
        if medium:
            s.lines(cx, 330, ["Both rays reach the wafer: NA = n·sinθ",
                              "up to ≈ 1.35 with water (n ≈ 1.44 at 193 nm);",
                              "inside the water λ/n ≈ 134 nm."], size=13, color=t["ink2"])
        else:
            s.lines(cx, 330, ["The steep ray (n·sinθ > 1) is totally",
                              "internally reflected: in air NA < 1",
                              "(dry ArF lenses stopped near 0.93)."], size=13, color=t["ink2"])
    s.text(856, 412, "Schematic, not to scale: the real gap is thin and only the last element "
           "is drawn. Ray angles follow Snell's law.", size=11, anchor="end", color=t["muted"])
    return s


# ---------------------------------------------------------------------------------------
# 4. EUV reflective optical path
# ---------------------------------------------------------------------------------------


def fold_mirror(s: Svg, c, a, b, length: float = 46.0, thick: float = 8.0) -> None:
    """Flat multilayer mirror at point c that reflects light arriving from a toward b.

    The surface normal bisects the directions to a and b; the mirror body is drawn
    behind the reflecting surface, with thin lines hinting at the multilayer stack.
    """
    t = s.t
    ua = np.subtract(a, c) / np.hypot(*np.subtract(a, c))
    ub = np.subtract(b, c) / np.hypot(*np.subtract(b, c))
    n = (ua + ub) / np.hypot(*(ua + ub))
    tan = np.array([-n[1], n[0]])
    p1, p2 = np.add(c, tan * length / 2), np.subtract(c, tan * length / 2)
    q1, q2 = p1 - n * thick, p2 - n * thick
    s.poly([tuple(p1), tuple(p2), tuple(q2), tuple(q1)], t["mirror"], sw=1.2,
           fill=t["mirror_fill"], close=True)
    for f in (0.35, 0.65):
        r1, r2 = p1 - n * thick * f, p2 - n * thick * f
        s.line(r1[0], r1[1], r2[0], r2[1], t["mirror"], sw=0.8)


def fig_euv_path(theme: dict[str, str]) -> Svg:
    s = Svg(
        960, 580,
        "EUV scanner light path",
        "Schematic of an EUV exposure tool. A CO2 laser pulse hits a tin droplet; the plasma "
        "emits 13.5 nm light that an ellipsoidal collector mirror focuses to an intermediate "
        "focus. Faceted illuminator mirrors shape the beam and send it to a reflective mask "
        "at oblique incidence; six projection mirrors demagnify the mask pattern four times "
        "onto the wafer. Every optic is a multilayer mirror and the whole path is kept in "
        "vacuum.",
        theme,
    )
    t = theme
    euv = t["euv"]
    s.rect(20, 46, 920, 486, "none", t["faint"], 1.5, rx=14)
    s.text(36, 72, "vacuum: 13.5 nm light is absorbed by air", size=12, anchor="start",
           color=t["muted"])
    # --- source: collector (parabolic section with a central hole), droplets, laser
    D = (240.0, 330.0)
    xv, a = 155.0, 45.0 / 105.0**2

    def col_x(y: float) -> float:
        return xv + a * (y - D[1]) ** 2

    for ys in (np.linspace(225, 318, 24), np.linspace(342, 435, 24)):
        pts = [(col_x(y), y) for y in ys]
        s.poly(pts, t["mirror"], sw=9)
        s.poly(pts, t["mirror_fill"], sw=5)
    s.text(172, 470, "collector", size=12, color=t["ink2"])
    s.text(172, 486, "(ellipsoidal mirror)", size=12, color=t["ink2"])
    for k in range(6):
        s.circle(D[0], D[1] - 96 + 16 * k, 3.2, fill=t["tin"])
    s.text(D[0] + 12, D[1] - 90, "tin droplets", size=12, anchor="start", color=t["ink2"])
    s.line(40, D[1], D[0] - 11, D[1], t["ir"], sw=3, arrow=True)
    s.text(40, D[1] - 34, "CO₂ laser", size=12, anchor="start", color=t["ink2"])
    s.text(40, D[1] - 19, "10.6 µm", size=12, anchor="start", color=t["ink2"])
    s.circle(D[0], D[1], 7, fill=t["accent"])
    s.text(D[0] + 12, D[1] + 24, "tin plasma", size=12, anchor="start", color=t["ink2"])
    IF = (400.0, 330.0)
    for yc in (250.0, 410.0):
        cpt = (col_x(yc) + 4, yc)
        s.line(D[0], D[1], cpt[0], cpt[1], euv, sw=1.2)
        s.line(cpt[0], cpt[1], IF[0], IF[1], euv, sw=2)
    s.circle(*IF, 3.5, fill=euv)
    s.text(IF[0] + 10, IF[1] - 12, "intermediate focus", size=12, anchor="start",
           color=t["ink2"])
    # --- illuminator: field facet mirror, pupil facet mirror
    FFM, PFM, MASK = (535.0, 470.0), (650.0, 222.0), (765.0, 104.0)
    s.line(*IF, *FFM, euv, sw=2)
    s.line(*FFM, *PFM, euv, sw=2)
    s.line(*PFM, *MASK, euv, sw=2)
    fold_mirror(s, FFM, IF, PFM, length=70)
    fold_mirror(s, PFM, FFM, MASK, length=60)
    s.text(FFM[0] + 44, FFM[1] + 34, "field facet mirror", size=12, anchor="middle",
           color=t["ink2"])
    s.text(PFM[0] - 22, PFM[1] - 12, "pupil facet", size=12, anchor="end", color=t["ink2"])
    s.text(PFM[0] - 22, PFM[1] + 4, "mirror", size=12, anchor="end", color=t["ink2"])
    s.text(FFM[0] + 44, FFM[1] + 52, "illuminator", size=13, weight="bold", color=t["ink2"])
    # --- reflective mask, face down
    mx0, mx1, my = 680.0, 860.0, MASK[1]
    s.rect(mx0, my - 30, mx1 - mx0, 24, t["glass_fill"], t["glass"], 1.5)
    s.rect(mx0, my - 6, mx1 - mx0, 6, t["mirror_fill"], t["mirror"], 1.2)
    for k in range(5):
        s.rect(mx0 + 12 + 34 * k, my, 16, 4, t["chrome"], "none", 0)
    s.text((mx0 + mx1) / 2, my - 38, "reflective mask", size=12, color=t["ink2"])
    s.text(mx0 - 10, my - 20, "Mo/Si multilayer", size=12, anchor="end", color=t["ink2"])
    s.text(mx0 - 10, my - 4, "+ absorber pattern", size=12, anchor="end", color=t["ink2"])
    # --- projection optics: six mirrors
    P = [MASK, (830.0, 178.0), (728.0, 232.0), (840.0, 290.0), (722.0, 346.0),
         (836.0, 398.0), (764.0, 448.0), (800.0, 496.0)]
    for p, q in zip(P[:-1], P[1:]):
        s.line(*p, *q, euv, sw=2)
    for k in range(1, 7):
        fold_mirror(s, P[k], P[k - 1], P[k + 1], length=40, thick=7)
        right = P[k][0] > 790 or (k == 6)
        s.text(P[k][0] + (26 if right else -26), P[k][1] + 5, f"M{k}", size=12,
               anchor="start" if right else "end", color=t["ink2"])
    s.rect(684, 150, 214, 322, "none", t["line"], 1.2, rx=8)
    s.text(606, 420, "projection optics", size=13, weight="bold", color=t["ink2"])
    s.text(606, 437, "6 mirrors,", size=12, color=t["ink2"])
    s.text(606, 454, "4× reduction", size=12, color=t["ink2"])
    # --- wafer
    s.rect(730, 496, 140, 8, t["resist_fill"], t["resist"], 1.2)
    s.rect(730, 504, 140, 14, t["wafer_fill"], t["wafer"], 1.2)
    s.text(884, 512, "wafer", size=12, anchor="start", color=t["ink2"])
    s.text(940, 566, "Schematic, not to scale: every optic is a multilayer mirror; real tools "
           "differ in geometry and use more illuminator mirrors.", size=11, anchor="end",
           color=t["muted"])
    return s


# ---------------------------------------------------------------------------------------
# 5. Multiple patterning: LELE vs SADP
# ---------------------------------------------------------------------------------------


def fig_multipatterning(theme: dict[str, str]) -> Svg:
    s = Svg(
        900, 470,
        "Pitch splitting: LELE versus SADP",
        "Top row, litho-etch-litho-etch: a first exposure and etch print every other line at "
        "pitch P, a second exposure shifted by P/2 prints the lines in between, giving pitch "
        "P/2 but making the result depend on overlay between the two exposures. Bottom row, "
        "self-aligned double patterning: one exposure prints mandrels at pitch P; a conformal "
        "film is deposited and etched back so that spacers remain on the mandrel sidewalls; "
        "removing the mandrels leaves spacers at pitch P/2, with widths set by deposition.",
        theme,
    )
    t = theme
    W, P = 150, 76.0  # panel width, litho pitch in drawing units
    q = P / 4

    def base(x0, y0):
        s.rect(x0, y0 + 40, W, 18, t["wafer_fill"], t["wafer"], 1)

    def hm_full(x0, y0):
        s.rect(x0, y0 + 30, W, 10, t["mirror_fill"], t["mirror"], 1)

    def hm_lines(x0, y0, xs):
        for x in xs:
            s.rect(x0 + x, y0 + 30, q, 10, t["mirror_fill"], t["mirror"], 1)

    def resist_lines(x0, y0, xs, h=22):
        for x in xs:
            s.rect(x0 + x, y0 + 30 - h, q, h, t["resist_fill"], t["resist"], 1)

    first = [12.0, 12.0 + P]  # left edges of the first-exposure lines (width q)
    second = [x + P / 2 for x in first]

    def step_label(x0, y0, n, label):
        s.text(x0 + W / 2, y0 - 12, f"{n}. {label}", size=13, color=t["ink2"])

    # --- LELE row
    y0 = 92
    s.text(20, 52, "LELE: litho–etch–litho–etch (two exposures)", size=16, anchor="start",
           weight="bold")
    xs = [20, 240, 460, 680]
    base(xs[0], y0); hm_full(xs[0], y0); resist_lines(xs[0], y0, first)
    step_label(xs[0], y0, 1, "expose + develop #1")
    base(xs[1], y0); hm_lines(xs[1], y0, first)
    step_label(xs[1], y0, 2, "etch #1, strip")
    base(xs[2], y0); hm_lines(xs[2], y0, first); resist_lines(xs[2], y0, second)
    step_label(xs[2], y0, 3, "expose + develop #2")
    base(xs[3], y0); hm_lines(xs[3], y0, sorted(first + second))
    step_label(xs[3], y0, 4, "etch #2: pitch P/2")
    for a in xs[:-1]:
        s.line(a + W + 14, y0 + 36, a + W + 56, y0 + 36, t["line"], sw=2, arrow=True)
    # pitch annotations
    s.line(xs[0] + first[0] + q / 2, y0 + 70, xs[0] + first[1] + q / 2, y0 + 70, t["ink2"],
           sw=1.3, arrow=True, arrow_start=True)
    s.text(xs[0] + first[0] + q / 2 + P / 2, y0 + 86, "P", size=13, italic=True)
    pts = sorted(first + second)
    s.line(xs[3] + pts[0] + q / 2, y0 + 70, xs[3] + pts[1] + q / 2, y0 + 70, t["ink2"], sw=1.3,
           arrow=True, arrow_start=True)
    s.text(xs[3] + pts[0] + q / 2 + P / 4, y0 + 86, "P/2", size=13, italic=True)
    s.text(xs[2] + W / 2, y0 + 86, "overlay error shifts these lines", size=12,
           color=t["accent"])

    # --- SADP row
    y1 = 300
    s.text(20, 258, "SADP: self-aligned (spacer) double patterning (one exposure)", size=16,
           anchor="start", weight="bold")
    xs = [20, 196, 372, 548, 724]
    W2 = 150
    # Uniform spacer pitch P/2 needs mandrel width + film thickness = P/2.
    film = 12.0
    wm = P / 2 - film
    mand = [24.0, 24.0 + P]  # mandrel left edges

    def base2(x0):
        s.rect(x0, y1 + 40, W2, 18, t["wafer_fill"], t["wafer"], 1)
        s.rect(x0, y1 + 30, W2, 10, t["mirror_fill"], t["mirror"], 1)

    def mandrels(x0):
        for x in mand:
            s.rect(x0 + x, y1 + 8, wm, 22, t["mandrel_fill"], t["mandrel"], 1)

    base2(xs[0]); mandrels(xs[0])
    s.text(xs[0] + W2 / 2, y1 - 12, "1. mandrels at pitch P", size=13, color=t["ink2"])
    base2(xs[1]); mandrels(xs[1])
    # conformal film: outline around mandrels plus a floor layer
    x0 = xs[1]
    d = f"M{x0},{y1 + 30 - film} "
    for x in mand:
        d += (f"L{x0 + x - film},{y1 + 30 - film} L{x0 + x - film},{y1 + 8 - film} "
              f"L{x0 + x + wm + film},{y1 + 8 - film} L{x0 + x + wm + film},{y1 + 30 - film} ")
    d += f"L{x0 + W2},{y1 + 30 - film} L{x0 + W2},{y1 + 30} L{x0},{y1 + 30} Z"
    s.path(d, fill=t["spacer_fill"], stroke=t["spacer"], sw=1, opacity=0.9)
    mandrels(xs[1])
    s.text(xs[1] + W2 / 2, y1 - 12, "2. deposit film", size=13, color=t["ink2"])
    base2(xs[2]); mandrels(xs[2])
    for x in mand:
        s.rect(xs[2] + x - film, y1 + 8, film, 22, t["spacer_fill"], t["spacer"], 1)
        s.rect(xs[2] + x + wm, y1 + 8, film, 22, t["spacer_fill"], t["spacer"], 1)
    s.text(xs[2] + W2 / 2, y1 - 12, "3. etch back", size=13, color=t["ink2"])
    base2(xs[3])
    sp = []
    for x in mand:
        sp += [x - film, x + wm]
    for x in sp:
        s.rect(xs[3] + x, y1 + 8, film, 22, t["spacer_fill"], t["spacer"], 1)
    s.text(xs[3] + W2 / 2, y1 - 12, "4. remove mandrels", size=13, color=t["ink2"])
    s.rect(xs[4], y1 + 40, W2, 18, t["wafer_fill"], t["wafer"], 1)
    for x in sp:
        s.rect(xs[4] + x, y1 + 30, film, 10, t["mirror_fill"], t["mirror"], 1)
    s.text(xs[4] + W2 / 2, y1 - 12, "5. transfer", size=13, color=t["ink2"])
    for a in xs[:-1]:
        s.line(a + W2 + 4, y1 + 36, a + W2 + 22, y1 + 36, t["line"], sw=2, arrow=True)
    s.line(xs[0] + mand[0] + wm / 2, y1 + 70, xs[0] + mand[1] + wm / 2, y1 + 70, t["ink2"],
           sw=1.3, arrow=True, arrow_start=True)
    s.text(xs[0] + mand[0] + wm / 2 + P / 2, y1 + 86, "P", size=13, italic=True)
    s.lines(xs[3] + W2 / 2 + 88, y1 + 82, ["spacer pitch ≈ P/2, set by the mandrel width",
                                            "and the deposited film thickness"], size=12,
            color=t["ink2"])
    s.lines(450, 420, ["Repeating the spacer step on the spacers (SAQP) quarters the pitch.",
                       "LELE needs no extra films but ties the result to overlay; "
                       "SADP is self-aligned but only makes regular lines."],
            size=13, color=t["ink2"])
    s.text(896, 464, "Schematic cross-sections, not to scale.", size=11, anchor="end",
           color=t["muted"])
    return s


# ---------------------------------------------------------------------------------------
# 6. Illumination pupils and off-axis imaging
# ---------------------------------------------------------------------------------------


def pupil(s: Svg, cx, cy, r, shape: str) -> None:
    t = s.t
    s.circle(cx, cy, r, fill="none", stroke=t["line"], sw=1.5)
    fill, op = t["uv"], 0.55
    if shape == "conventional":
        s.circle(cx, cy, 0.6 * r, fill=fill, opacity=op)
    elif shape == "annular":
        ro, ri = 0.85 * r, 0.55 * r
        s.path(
            f"M{cx - ro},{cy} A{ro},{ro} 0 1,0 {cx + ro},{cy} A{ro},{ro} 0 1,0 {cx - ro},{cy} Z "
            f"M{cx - ri},{cy} A{ri},{ri} 0 1,1 {cx + ri},{cy} A{ri},{ri} 0 1,1 {cx - ri},{cy} Z",
            fill=fill, opacity=op,
        )
    elif shape == "dipole":
        for sgn in (-1, 1):
            s.circle(cx + sgn * 0.62 * r, cy, 0.22 * r, fill=fill, opacity=op)
    elif shape == "quadrupole":
        for a in (45, 135, 225, 315):
            s.circle(cx + 0.62 * r * math.cos(math.radians(a)),
                     cy + 0.62 * r * math.sin(math.radians(a)), 0.2 * r, fill=fill, opacity=op)
    s.line(cx - r - 6, cy, cx + r + 6, cy, t["faint"], sw=1)
    s.line(cx, cy - r - 6, cx, cy + r + 6, t["faint"], sw=1)


def fig_illumination(theme: dict[str, str]) -> Svg:
    s = Svg(
        880, 500,
        "Illumination shapes and why off-axis illumination helps",
        "Top: four illumination shapes seen in the projection-lens pupil: conventional "
        "disc, annular ring, dipole and quadrupole. Bottom: for a dense grating whose "
        "first diffraction orders fall outside the pupil, on-axis light passes only the zero "
        "order and no pattern is imaged; tilting the illumination puts the zero order and "
        "one first order inside the pupil, and their interference images the grating.",
        theme,
    )
    t = theme
    s.text(20, 34, "Illumination shapes (pupil view; circle = lens NA)", size=16, anchor="start",
           weight="bold")
    for k, (shape, label) in enumerate([("conventional", "conventional"), ("annular", "annular"),
                                        ("dipole", "dipole"), ("quadrupole", "quadrupole")]):
        cx = 110 + 220 * k
        pupil(s, cx, 125, 62, shape)
        s.text(cx, 212, label, size=14, color=t["ink2"])
    s.text(20, 264, "A dense grating: which diffraction orders get through the pupil?",
           size=16, anchor="start", weight="bold")
    r = 62
    spacing = 1.3 * r  # first-order offset larger than the pupil radius
    for k, (title, src) in enumerate([("on-axis illumination", 0.0),
                                      ("off-axis (dipole pole)", -0.65 * r)]):
        cx, cy = 230 + 420 * k, 368
        s.circle(cx, cy, r, fill="none", stroke=t["line"], sw=1.5)
        for m in (-1, 0, 1):
            ox = cx + src + m * spacing
            inside = abs(ox - cx) < r
            if abs(ox - cx) > 2.3 * r:
                continue
            s.circle(ox, cy, 7, fill=t["uv"] if inside else "none",
                     stroke=t["uv"] if inside else t["muted"], sw=1.8)
            lab = {-1: "−1", 0: "0", 1: "+1"}[m]
            s.text(ox, cy - 14, lab, size=13, color=t["ink"] if inside else t["muted"])
        s.text(cx, cy + r + 24, title, size=14, color=t["ink2"])
        msg = ("only the 0 order passes: no image" if k == 0
               else "0 and +1 pass: two-beam interference")
        s.text(cx, cy + r + 42, msg, size=13, color=t["accent"] if k == 0 else t["ink2"])
    s.text(876, 494, "Order spacing = λ/(pitch) in pupil units; filled = transmitted, "
           "open = blocked.", size=11, anchor="end", color=t["muted"])
    return s


FIGURES = {
    "printing-modes": fig_printing_modes,
    "stepper-vs-scanner": fig_stepper_scanner,
    "immersion-principle": fig_immersion,
    "euv-optical-path": fig_euv_path,
    "multipatterning-lele-sadp": fig_multipatterning,
    "illumination-oai": fig_illumination,
}


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, build in FIGURES.items():
        for scheme, theme in THEMES.items():
            path = OUT / f"{name}-{scheme}.svg"
            path.write_text(build(theme).render(), encoding="utf-8")
            print(f"wrote {path.relative_to(OUT.parents[3])}")


if __name__ == "__main__":
    main()
