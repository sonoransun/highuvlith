#!/usr/bin/env python3
"""SVG schematics for the History section of the highuvlith site (docs/history/).

Run from the repository root (standard library, plus numpy for the ``printing-modes``
intensity profiles):

    python docs/figures/history/make_history_schematics.py            # every pair
    python docs/figures/history/make_history_schematics.py --only mirror-staircase

Every figure is written twice into ``docs/assets/images/history/`` (``<name>-light.svg``,
``<name>-dark.svg``; transparent background, ink from ``tokens.theme``) through
``svgkit.write_pair`` and registered as an ``AssetSpec`` (DESIGN §5.0, §5.5 N5 · N7 · N8 ·
N9, §5.7). The pupils of ``illumination-oai`` use the D5 pupil grammar of ``svgkit`` so the
same iconography appears wherever a pupil is drawn.

All drawings are schematics and are not to scale; their captions say so. The only
computed content is the set of 1-D scalar intensity profiles in ``printing-modes``
(Fresnel propagation across a gap for proximity printing, a partially coherent low-pass
image for projection) — the shape of each effect in arbitrary units, not measured data and
not an engine run. The ``mirror-staircase`` numbers are arithmetic on the cited 70 % and
64.1 % reflectances (0.70¹² = 1.4 %).
"""

from __future__ import annotations

import argparse
import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
FIGURES = HERE.parent
for _p in (str(FIGURES), str(HERE)):
    if _p not in sys.path:
        sys.path.insert(0, _p)

import registry  # noqa: E402
import svgkit as K  # noqa: E402
import tokens  # noqa: E402

OUT = registry.IMAGES / "history"
GENERATOR = "docs/figures/history/make_history_schematics.py"
SMALL = K.SMALL  # 12.5 — the floor
LABEL = K.LABEL  # 13
UV = "brand:violet600"   # DUV light rays
EUV = "brand:cyan700"    # EUV light rays


def _uv(s: K.Svg) -> str:
    return s.c("brand:violet600") if s.theme == "light" else s.c("brand:violet300")


def _euv(s: K.Svg) -> str:
    return s.c("brand:cyan700") if s.theme == "light" else s.c("brand:cyan300")


def _ir(s: K.Svg) -> str:
    return s.c("red")


def lines(s: K.Svg, x, y, rows, size=LABEL, anchor="middle", color="ink2", dy=None, **kw) -> None:
    dy = dy or size * 1.3
    for i, row in enumerate(rows):
        s.text(x, y + i * dy, row, size=size, anchor=anchor, color=color, **kw)


def footer(s: K.Svg, text: str, *, anchor="end") -> None:
    x = s.w - 8 if anchor == "end" else s.w / 2
    s.text(x, s.h - 6, text, size=SMALL, anchor=anchor, color="muted")


# ---------------------------------------------------------------------------------------
# Alt texts and captions
# ---------------------------------------------------------------------------------------

PRINTING_ALT = ("Three panels: contact printing with the mask on the resist, proximity printing with a gap g that "
                "blurs the edges, and projection printing through a lens onto a reduced image field; under each, the "
                "intensity reaching the resist against the mask pattern.")
PRINTING_CAPTION = ("<strong>Three ways to transfer a mask pattern.</strong> Contact copies sharply but damages mask "
                    "and wafer; a gap protects the mask and diffraction blurs edges over roughly √(λg); a lens images "
                    "the mask at R = k₁λ/NA and nothing touches the wafer. The profiles are a 1-D scalar model in "
                    "arbitrary units (Fresnel propagation averaged over ±20 % of λg; a partially coherent band-limited "
                    "image). <em>Schematic, not to scale.</em>")

STEPPER_ALT = ("Left, a step-and-repeat stepper: the whole reticle is lit in one flash, one field printed, the wafer "
               "stepped to the next. Right, a step-and-scan system: a slit of light sweeps the reticle while reticle "
               "and wafer scan in opposite directions, the reticle four times faster for a 4x lens.")
STEPPER_CAPTION = ("<strong>Stepper and scanner.</strong> With an image-inverting 4× lens the reticle and wafer stages "
                   "scan in opposite directions, the reticle four times faster, so a lens corrected over the slit "
                   "alone prints a 26 × 33 mm field. <em>Schematic, not to scale.</em>")

THREE_WAYS_ALT = ("Three panels: the Micralign 1:1 scanning projection aligner with an Offner two-mirror relay, an arc "
                  "slit and a carriage moving mask and wafer together; a step-and-repeat stepper lighting a whole "
                  "reticle field; a step-and-scan system sweeping a slit while reticle and wafer scan oppositely.")
THREE_WAYS_CAPTION = ("<strong>Three ways to move the image.</strong> The Micralign scans mask and wafer together "
                      "through a 1:1 ring-field mirror relay; a stepper exposes a whole reduced field and steps; a "
                      "step-and-scan tool sweeps a slit while reticle and wafer scan at 4v and v. "
                      "<em>Schematic, not to scale.</em>")

IMMERSION_ALT = ("Two panels of the last lens element above a wafer: in air a steep ray is totally internally "
                 "reflected at the glass face because n sin theta cannot exceed 1, so NA stays below 1; with water "
                 "(n about 1.44 at 193 nm) in the gap the same ray reaches the wafer and NA = n sin theta can reach "
                 "about 1.35.")
IMMERSION_CAPTION = ("<strong>The immersion principle.</strong> Ray angles follow Snell's law with n ≈ 1.56 for the "
                     "fused-silica lens and 1.44 for water at 193 nm: the steep ray that air reflects back into the "
                     "glass, water lets through, and NA = n·sin θ reaches 1.35. <em>Schematic, not to scale.</em>")

EUV_PATH_ALT = ("Schematic EUV scanner: a CO2 laser hits tin droplets, an ellipsoidal collector focuses the 13.5 nm "
                "light to an intermediate focus, facet mirrors shape it onto a reflective mask at oblique incidence, "
                "and six projection mirrors demagnify the pattern four times onto the wafer, all in vacuum.")
EUV_PATH_CAPTION = ("<strong>An EUV scanner, schematically.</strong> Tin-plasma source and collector, faceted "
                    "illuminator, reflective mask at oblique incidence, six-mirror 4× projection optics and wafer, all "
                    "in vacuum; real tools differ in geometry and use more illuminator mirrors. "
                    "<em>Schematic, not to scale.</em>")
EUV_SOURCE_ALT = ("The source end of an EUV scanner: a CO2 laser pulse strikes a stream of tin droplets, the plasma "
                  "radiates 13.5 nm light, and an ellipsoidal multilayer collector focuses it to the intermediate "
                  "focus.")
EUV_SOURCE_CAPTION = ("<strong>The source problem, in one corner of the scanner.</strong> A CO₂ laser turns tin "
                      "droplets into plasma; only the in-band light the collector catches reaches the intermediate "
                      "focus, and every later mirror takes its 30 % cut. <em>Schematic, not to scale.</em>")
EUV_SOURCE_BOX = (26.0, 200.0, 480.0, 268.0)  # the crop's viewBox (x, y, w, h): laser → intermediate focus;
# the right edge (506) stops before the field facet mirror (x ≥ 500 only below y = 475) and the projection-optics
# label (x ≥ 559), the bottom edge (468) clears the mirror and the one-line collector label (VERIFY r1).

MULTIPATTERNING_ALT = ("Top row, litho-etch-litho-etch: two exposures shifted by half a pitch print lines at pitch "
                       "P/2 that depend on overlay. Bottom row, self-aligned double patterning: mandrels at pitch P, a "
                       "conformal film, etch-back leaving sidewall spacers, mandrel removal and transfer at pitch P/2.")
MULTIPATTERNING_CAPTION = ("<strong>Pitch splitting.</strong> LELE needs two exposures and ties the result to their "
                           "overlay; SADP needs one exposure plus deposition and etch steps and is self-aligned, with "
                           "the spacer pitch set by mandrel width and film thickness. "
                           "<em>Model: Multiple patterning (LELE, SADP, SAQP) 🔶 (SADP geometric; LELE as two "
                           "thresholded exposures).</em>")

ILLUMINATION_ALT = ("Top: conventional, annular, dipole and quadrupole illumination shapes inside the projection "
                    "pupil. Bottom: for a dense grating, on-axis light passes only the zero order and images nothing; "
                    "a tilted pole puts the zero and one first order inside the pupil and they image the grating.")
ILLUMINATION_CAPTION = ("<strong>Why off-axis illumination helps.</strong> Every shape shown is an engine illumination "
                        "fill (the graded CoherentGaussian fill as well); order spacing is λ/pitch in pupil units, "
                        "filled orders are transmitted, open ones blocked. "
                        "<em>Model: Illumination pupil fills ✅.</em>")

MIRROR_ALT = ("A staircase of twelve light bars shrinking by 30 percent at each multilayer reflection, from the "
              "collector through four illuminator mirrors, the mask and six projection mirrors, ending at 1.4 percent; "
              "a dotted line shows the 64 percent La/B record falling to 0.5 percent.")
MIRROR_CAPTION = ("<strong>Twelve reflections at 70 %: 1.4 % of the light left.</strong> 70 % is the cited best case "
                  "and twelve is the count the EUV chapter and this staircase use; the Beyond-EUV reflectance chart "
                  "and the throughput model's chain count eleven, ten mirrors and the mask (the throughput model also "
                  "takes the mask at 0.65, a stated assumption); the dotted line is the 64.1 % La/B record "
                  "at 6.7 nm. <em>Model: EUV/BEUV multilayer mirrors ✅.</em>")

TWO_LANES_ALT = ("Two timeline lanes joined by an arrow: printing and photography from Senefelder's 1796 lithography "
                 "through Niépce's 1827 bitumen image to Poitevin's 1855 carbon process, and silicon from the 1955 "
                 "oxide mask through 1957 photoresist patterning to the first planar integrated circuit in 1960.")
TWO_LANES_CAPTION = ("<strong>Two lanes to the planar integrated circuit.</strong> Lithography the printing craft and "
                     "light-hardened coatings on one side; oxide masking, photoresist and the planar process on the "
                     "other, joined by the loop every chip layer still runs: coat · expose · develop · etch. "
                     "<em>Dates as cited on this page; ranges where sources differ.</em>")

CHANGED_ALT = ("A four-column before-and-after strip for the move to 248 nm: light source from a mercury lamp to a "
               "KrF excimer laser, lens glass from BK7 and quartz to fused silica with calcium fluoride, resist from "
               "DNQ-novolac to chemically amplified, and masks from plain chrome to chrome with OPC and phase shift.")
CHANGED_CAPTION = ("<strong>What had to change at 248 nm.</strong> Leaving the mercury lamp meant a new source, a new "
                   "lens material, a new resist chemistry and, soon after, corrected masks — the four columns, each "
                   "summarising the paragraph it cites. "
                   "<em>Qualitative; each cell summarises the paragraph it cites. No model badge.</em>")


# ---------------------------------------------------------------------------------------
# 1. Contact vs proximity vs projection printing
# ---------------------------------------------------------------------------------------

OPENINGS = (-60.0, 0.0, 60.0)  # mask openings (drawing units), 26 wide, 60 pitch
OPEN_W = 26.0


def printing_profiles() -> dict[str, tuple[list[float], list[float]]]:
    """1-D scalar intensity just above the resist for the three printing modes (numpy)."""
    import numpy as np  # noqa: PLC0415

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
    # Proximity: paraxial Fresnel propagation over the gap, H(f) = exp(-iπλg f²), with √(λg) = 15 drawing
    # units (Fresnel number w²/(λg) ≈ 3), averaged over a ±20 % spread of λg as a broadband lamp would do.
    prox = np.zeros(n)
    spread = np.linspace(0.8, 1.2, 9) * 15.0**2
    for lam_g in spread:
        prox += np.abs(np.fft.ifft(spec * np.exp(-1j * np.pi * lam_g * f**2))) ** 2
    prox /= len(spread)
    # Projection: partially coherent 1-D image, pupil cutoff 1.25/pitch, σ 0.5 (21 tilted plane waves).
    fc = 1.25 / 60.0
    proj = np.zeros(n)
    for sh in np.linspace(-0.5, 0.5, 21) * fc:
        proj += np.abs(np.fft.ifft(spec * (np.abs(f + sh) <= fc))) ** 2
    proj /= len(spread) * 0 + 21
    keep = np.abs(x) <= 100
    out = {}
    for name, arr in (("mask", u0), ("contact", contact), ("proximity", prox), ("projection", proj)):
        out[name] = ([float(v) for v in x[keep]], [round(float(v), 4) for v in arr[keep]])
    return out


def draw_mask(s: K.Svg, cx: float, y_glass: float, glass_h: float = 24, chrome_h: float = 6, half: float = 100) -> float:
    """Glass plate with chrome on its underside; returns the y of the chrome bottom."""
    s.rect(cx - half, y_glass, 2 * half, glass_h, "glass", sw=1.5)
    y_c = y_glass + glass_h
    edges = [cx - half]
    for c in OPENINGS:
        edges += [cx + c - OPEN_W / 2, cx + c + OPEN_W / 2]
    edges.append(cx + half)
    for a, b in zip(edges[0::2], edges[1::2]):
        s.rect(a, y_c, b - a, chrome_h, "absorber", sw=0.8)
    return y_c + chrome_h


def draw_wafer(s: K.Svg, cx: float, y_top: float, half: float = 110, resist_h: float = 12, wafer_h: float = 22) -> None:
    s.rect(cx - half, y_top, 2 * half, resist_h, "resist", sw=1.2)
    s.rect(cx - half, y_top + resist_h, 2 * half, wafer_h, "substrate", sw=1.2)


def draw_rays_down(s: K.Svg, cx: float, y0: float, y1: float, half: float = 90, n: int = 7) -> None:
    for i in range(n):
        x = cx - half + i * (2 * half) / (n - 1)
        s.line(x, y0, x, y1, _uv(s), 2, arrow="end")


def draw_profile(s: K.Svg, cx: float, base: float, prof, ideal, scale: float = 38.0) -> None:
    s.line(cx - 104, base, cx + 104, base, "grid", 1)
    xi, ui = ideal
    s.polyline([(cx + x, base - scale * u) for x, u in zip(xi[::4], ui[::4])], "muted", 1.2, dash="3 3")
    xp, up = prof
    s.polyline([(cx + x, base - scale * u) for x, u in zip(xp[::2], up[::2])], "blue", 2)


def draw_printing_modes(theme: str) -> str:
    s = K.Svg(900, 470, theme, "Contact, proximity and projection printing", PRINTING_ALT, generator=GENERATOR)
    prof = printing_profiles()
    wafer_y = 246
    for i, (name, title) in enumerate([("contact", "Contact printing"), ("proximity", "Proximity printing"),
                                       ("projection", "Projection printing")]):
        cx = 190 + 280 * i
        s.text(cx, 30, title, size=17, weight=700)
        if name == "contact":
            draw_rays_down(s, cx, 96, 190)
            y_c = draw_mask(s, cx, wafer_y - 30)
            draw_wafer(s, cx, y_c)
            s.text(cx + 108, wafer_y - 34, "mask", size=SMALL, anchor="start", color="ink2")
        elif name == "proximity":
            draw_rays_down(s, cx, 96, 160)
            gap = 34
            y_c = draw_mask(s, cx, wafer_y - 30 - gap)
            draw_wafer(s, cx, wafer_y)
            gx = cx - 122
            s.line(gx, y_c + 2, gx, wafer_y - 2, "ink2", 1.5, arrow="both")
            s.text(gx - 6, (y_c + wafer_y) / 2 + 5, "g", size=15, anchor="end", italic=True)
            s.text(cx + 108, y_c - 34, "mask", size=SMALL, anchor="start", color="ink2")
        else:
            draw_rays_down(s, cx, 44, 76, half=80, n=6)
            y_c = draw_mask(s, cx, 80)
            ly = 166
            s.path(f"M{cx - 96},{ly} Q{cx},{ly - 34} {cx + 96},{ly} Q{cx},{ly + 34} {cx - 96},{ly} Z", "glass", sw=1.5)
            s.text(cx + 104, ly + 5, "lens", size=SMALL, anchor="start", color="ink2")
            s.polyline([(cx - 100, y_c), (cx + 100, y_c), (cx + 92, ly), (cx + 45, wafer_y), (cx - 45, wafer_y),
                        (cx - 92, ly)], _uv(s), 1.2, fill=_uv(s), close=True, opacity=0.12)
            draw_wafer(s, cx, wafer_y, half=70)
            s.line(cx - 45, wafer_y - 4, cx + 45, wafer_y - 4, _uv(s), 3)
            s.text(cx + 108, 90, "mask", size=SMALL, anchor="start", color="ink2")
            lines(s, cx + 78, wafer_y + 9, ["image field", "(reduced)"], size=SMALL, anchor="start", dy=15)
        if i == 0:
            s.text(cx - 116, wafer_y + 10, "resist", size=SMALL, anchor="end", color="ink2")
            s.text(cx - 116, wafer_y + 28, "wafer", size=SMALL, anchor="end", color="ink2")
        base = 360
        draw_profile(s, cx, base, prof[name], prof["mask"])
        if i == 0:
            s.text(cx - 104, base - 62, "intensity at the resist", size=SMALL, anchor="start", color="ink2")
    captions = {
        0: ["Mask pressed onto the resist:", "sharp copy, but every contact", "damages mask and wafer."],
        1: ["A gap g protects the mask;", "diffraction blurs edges", "over roughly √(λg)."],
        2: ["A lens images the mask;", "R = k₁λ/NA, and nothing", "touches the wafer."],
    }
    for i, rows in captions.items():
        lines(s, 190 + 280 * i, 392, rows, size=LABEL)
    footer(s, "Schematic, not to scale. Dashed: mask pattern; solid: 1-D scalar model (arbitrary units).")
    return s.render()


# ---------------------------------------------------------------------------------------
# 2. Stepper vs scanner (and N8: the Micralign ring-field panel beside them)
# ---------------------------------------------------------------------------------------


def draw_wafer_map(s: K.Svg, cx: float, cy: float, r: float, fw: float, fh: float, current, exposed) -> dict:
    s.circle(cx, cy, r, fill=s.fill("substrate"), stroke=s.stroke("substrate"), sw=1.5)
    cells = {}
    nx, ny = int(2 * r // fw) + 2, int(2 * r // fh) + 2
    for i in range(-nx // 2, nx // 2 + 1):
        for j in range(-ny // 2, ny // 2 + 1):
            x0, y0 = cx + (i - 0.5) * fw, cy + (j - 0.5) * fh
            corners = [(x0, y0), (x0 + fw, y0), (x0, y0 + fh), (x0 + fw, y0 + fh)]
            if all(math.hypot(px - cx, py - cy) <= r - 2 for px, py in corners):
                s.rect(x0, y0, fw, fh, fill=s.fill("resist") if (i, j) in exposed else "none",
                       stroke=s.stroke("substrate"), sw=1)
                cells[(i, j)] = (x0, y0)
    if current is not None:
        x0, y0 = cells[current]
        s.rect(x0, y0, fw, fh, fill="none", stroke="orange", sw=3)
    return cells


def _stepper_panel(s: K.Svg, cx: float) -> None:
    uv = _uv(s)
    s.text(cx, 30, "Stepper (step-and-repeat)", size=17, weight=700)
    s.rect(cx - 60, 52, 120, 70, "glass", sw=1.5)
    s.rect(cx - 60, 52, 120, 70, fill=uv, stroke="none", opacity=0.22)
    for k in range(4):
        s.rect(cx - 44 + 24 * k, 64, 10, 46, "absorber", sw=0)
    lines(s, cx + 70, 80, ["reticle:", "whole field lit", "in one flash"], size=SMALL, anchor="start", dy=16)
    s.path(f"M{cx - 70},150 L{cx + 70},150 L{cx + 40},196 L{cx - 40},196 Z", "glass", sw=1.5)
    s.text(cx, 178, "projection lens", size=SMALL, color="ink2")
    s.line(cx - 60, 124, cx - 66, 150, uv, 1.2)
    s.line(cx + 60, 124, cx + 66, 150, uv, 1.2)
    cells = draw_wafer_map(s, cx, 330, 118, 34, 34, (0, 0),
                           {(-2, -1), (-1, -1), (0, -1), (1, -1), (2, -1), (-2, -2), (-1, -2), (0, -2), (1, -2),
                            (2, -2), (-3, 0), (-2, 0), (-1, 0)})
    x0, y0 = cells[(0, 0)]
    s.line(cx - 40, 196, x0 + 2, y0 + 2, uv, 1.2)
    s.line(cx + 40, 196, x0 + 32, y0 + 2, uv, 1.2)
    s.line(x0 + 36, y0 + 17, x0 + 62, y0 + 17, "orange", 2.5, arrow="end")
    lines(s, cx + 128, 312, ["expose a field,", "then step the", "wafer to the next"], size=SMALL, anchor="start", dy=16)


def _scanner_panel(s: K.Svg, cx: float) -> None:
    uv = _uv(s)
    s.text(cx, 30, "Scanner (step-and-scan)", size=17, weight=700)
    s.rect(cx - 60, 52, 120, 70, "glass", sw=1.5)
    for k in range(4):
        s.rect(cx - 44 + 24 * k, 64, 10, 46, "absorber", sw=0)
    s.rect(cx - 60, 80, 120, 16, fill=uv, stroke="none", opacity=0.45)
    # The reticle moves along the scan axis (perpendicular to the slit), 4× faster than the wafer and,
    # for an image-inverting lens, in the opposite direction.
    s.line(cx - 76, 110, cx - 76, 66, "orange", 2.5, arrow="end")
    lines(s, cx - 84, 84, ["reticle", "scans at 4v"], size=SMALL, anchor="end", dy=16)
    lines(s, cx + 70, 84, ["slit of light", "across the field"], size=SMALL, anchor="start", dy=16)
    s.path(f"M{cx - 70},150 L{cx + 70},150 L{cx + 40},196 L{cx - 40},196 Z", "glass", sw=1.5)
    s.text(cx, 178, "projection lens", size=SMALL, color="ink2")
    s.line(cx - 60, 96, cx - 66, 150, uv, 1.2)
    s.line(cx + 60, 96, cx + 66, 150, uv, 1.2)
    cells = draw_wafer_map(s, cx, 330, 118, 26, 33, (0, 0),
                           {(i, j) for i in range(-4, 5) for j in (-3, -2, -1)} | {(-4, 0), (-3, 0), (-2, 0), (-1, 0)})
    x0, y0 = cells[(0, 0)]
    s.rect(x0, y0 + 12, 26, 8, fill=uv, stroke="none", opacity=0.6)
    s.line(cx - 40, 196, x0 + 1, y0 + 12, uv, 1.2)
    s.line(cx + 40, 196, x0 + 25, y0 + 12, uv, 1.2)
    s.line(cx + 132, 356, cx + 132, 396, "orange", 2.5, arrow="end")
    lines(s, cx + 142, 372, ["wafer scans", "at v"], size=SMALL, anchor="start", dy=16)
    lines(s, cx + 128, 290, ["26 × 33 mm field", "printed by sweeping", "the slit across it"], size=SMALL, anchor="start", dy=16)


def _micralign_panel(s: K.Svg, cx: float) -> None:
    """The Micralign: a 1:1 Offner ring-field relay scanning mask and wafer together on one carriage."""
    uv = _uv(s)
    s.text(cx, 30, "Scanning projection (Micralign, 1:1)", size=17, weight=700)
    # mask (left) and wafer (right) on one carriage, face to face through the relay
    mask_x, wafer_x, y_plane = cx - 92, cx + 92, 96
    s.rect(mask_x - 44, y_plane - 12, 88, 12, "glass", sw=1.5)
    for k in range(4):
        s.rect(mask_x - 36 + 20 * k, y_plane, 9, 4, "absorber", sw=0)
    s.text(mask_x, y_plane - 20, "mask", size=SMALL, color="ink2")
    s.rect(wafer_x - 44, y_plane - 12, 88, 6, "resist", sw=1)
    s.rect(wafer_x - 44, y_plane - 6, 88, 6, "substrate", sw=1)
    s.text(wafer_x, y_plane - 20, "wafer", size=SMALL, color="ink2")
    # the carriage carrying both, scanning across the arc slit
    s.rect(cx - 150, y_plane + 4, 300, 8, "substrate", sw=1, rx=2)
    s.line(cx - 160, y_plane + 8, cx - 186, y_plane + 8, "orange", 2.5, arrow="end")
    lines(s, cx - 150, y_plane + 30, ["one carriage moves mask", "and wafer together (1:1)"], size=SMALL, anchor="start", dy=15)
    # Offner relay: a large concave primary (bottom arc) and a small convex secondary
    s.path(f"M{cx - 120},{330} A130,130 0 0 0 {cx + 120},{330}", fill="none", stroke=s.stroke("mirror"), sw=7)
    s.path(f"M{cx - 120},{330} A130,130 0 0 0 {cx + 120},{330}", fill="none", stroke=s.fill("mirror"), sw=3)
    s.path(f"M{cx - 24},{244} A30,30 0 0 1 {cx + 24},{244}", fill="none", stroke=s.stroke("mirror"), sw=6)
    s.path(f"M{cx - 24},{244} A30,30 0 0 1 {cx + 24},{244}", fill="none", stroke=s.fill("mirror"), sw=2.5)
    s.text(cx, 400, "concave primary", size=SMALL, color="ink2")
    s.text(cx + 32, 236, "convex secondary", size=SMALL, anchor="start", color="ink2")
    lines(s, cx, 420, ["Offner relay: two concentric", "spherical mirrors, well corrected", "only in a thin ring"],
          size=SMALL, dy=15)
    # rays: mask → primary → secondary → primary → wafer (1:1, inverted)
    pts = [(mask_x, y_plane + 12), (cx - 100, 372), (cx - 10, 258), (cx + 100, 372), (wafer_x, y_plane + 12)]
    for a, b in zip(pts, pts[1:]):
        s.line(a[0], a[1], b[0], b[1], uv, 1.6)
    s.line(mask_x - 30, 40, mask_x - 30, y_plane - 16, uv, 2, arrow="end")
    s.text(mask_x - 24, 50, "arc lamp", size=SMALL, anchor="start", color="ink2")
    # the illuminated arc (ring field) drawn on the mask
    s.path(f"M{mask_x - 30},{y_plane - 4} Q{mask_x},{y_plane - 16} {mask_x + 30},{y_plane - 4}", fill="none",
           stroke=uv, sw=3, opacity=0.7)
    s.text(mask_x + 48, y_plane - 2, "arc slit", size=SMALL, anchor="start", color="ink2")


def draw_stepper_scanner(theme: str) -> str:
    s = K.Svg(900, 500, theme, "Stepper versus scanner", STEPPER_ALT, generator=GENERATOR)
    _stepper_panel(s, 215)
    _scanner_panel(s, 630)
    footer(s, "Schematic, not to scale. Orange frame: field being exposed; shaded fields: already exposed.", anchor="middle")
    return s.render()


def draw_three_ways(theme: str) -> str:
    s = K.Svg(960, 500, theme, "Three ways to move the image", THREE_WAYS_ALT, generator=GENERATOR)
    with s.group(transform="translate(0,0)"):
        _micralign_panel(s, 176)
    with s.group(transform="translate(60,0) scale(0.86)"):
        _stepper_panel(s, 430)
    with s.group(transform="translate(90,0) scale(0.86)"):
        _scanner_panel(s, 760)
    s.line(345, 44, 345, 470, "grid", 1)
    s.line(660, 44, 660, 470, "grid", 1)
    footer(s, "Schematic, not to scale. Orange: motion; shaded fields already exposed.", anchor="middle")
    return s.render()


# ---------------------------------------------------------------------------------------
# 3. Immersion principle
# ---------------------------------------------------------------------------------------

N_GLASS = 1.56  # fused silica near 193 nm
N_WATER = 1.44  # water at 193 nm


def draw_immersion(theme: str) -> str:
    s = K.Svg(860, 420, theme, "Why water immersion raises the numerical aperture", IMMERSION_ALT, generator=GENERATOR)
    uv = _uv(s)
    half, sag = 170.0, 55.0  # lens half-width; sagitta of the curved top above its rim
    for panel, (title, medium) in enumerate([("Dry: air gap", None), ("Immersion: water", N_WATER)]):
        cx = 215 + 430 * panel
        s.text(cx, 30, title, size=17, weight=700)
        y_lb, y_w = 222, 272  # lens bottom, wafer top
        y_rim = y_lb - 40

        def top_y(x: float) -> float:  # curved top surface (parabolic cap)
            u = (x - cx) / half
            return y_rim - sag * (1 - u * u)

        s.path(f"M{cx - half},{y_lb} L{cx - half},{y_rim} Q{cx},{y_rim - 2 * sag} {cx + half},{y_rim} L{cx + half},{y_lb} Z",
               "glass", sw=1.5)
        s.text(cx, y_rim - sag - 14, "last lens element (n ≈ 1.56)", size=SMALL, color="ink2")
        if medium:
            s.rect(cx - 150, y_lb, 300, y_w - y_lb, "water", sw=1.2)
            s.text(cx + 158, y_lb + 30, "water", size=SMALL, anchor="start", color="ink2")
        else:
            s.text(cx + 178, y_lb + 30, "air", size=SMALL, anchor="start", color="ink2")
        s.rect(cx - 190, y_w, 380, 10, "resist", sw=1.2)
        s.rect(cx - 190, y_w + 10, 380, 22, "substrate", sw=1.2)
        n_med = medium or 1.0
        gap = y_w - y_lb
        for na_ray in (0.9, 1.25):
            steep = na_ray > 1.0
            color = s.c("orange") if steep else uv
            n_aim = n_med if na_ray < n_med else N_WATER
            x_hit = gap * math.tan(math.asin(na_ray / n_aim))
            th_g = math.asin(na_ray / N_GLASS)
            for sgn in (-1, 1):
                xh = cx + sgn * x_hit
                k = 0.0
                while True:
                    xa, ya = xh + sgn * (k + 1) * math.sin(th_g), y_lb - (k + 1) * math.cos(th_g)
                    if ya <= top_y(xa) or abs(xa - cx) >= half:
                        break
                    k += 1
                x0, y0 = xh + sgn * k * math.sin(th_g), y_lb - k * math.cos(th_g)
                s.line(x0, y0, xh, y_lb, color, 2)
                if na_ray / n_med <= 1.0:
                    th_m = math.asin(na_ray / n_med)
                    xf = xh - sgn * gap * math.tan(th_m)
                    s.line(xh, y_lb, xf, y_w, color, 2, arrow="end")
                else:  # total internal reflection at the flat glass face
                    L = 0.6 * k
                    s.line(xh, y_lb, xh - sgn * L * math.sin(th_g), y_lb - L * math.cos(th_g), color, 2, dash="5 4",
                           arrow="end")
        if medium:
            lines(s, cx, 330, ["Both rays reach the wafer: NA = n·sinθ", "up to ≈ 1.35 with water (n ≈ 1.44 at 193 nm);",
                               "inside the water λ/n ≈ 134 nm."], size=LABEL)
        else:
            lines(s, cx, 330, ["The steep ray (n·sinθ > 1) is totally", "internally reflected: in air NA < 1",
                               "(dry ArF lenses stopped near 0.93)."], size=LABEL)
    footer(s, "Schematic, not to scale: the real gap is thin and only the last element is drawn. Ray angles follow Snell's law.")
    return s.render()


# ---------------------------------------------------------------------------------------
# 4. EUV reflective optical path (+ the source crop)
# ---------------------------------------------------------------------------------------


def _sub(a, b):
    return (a[0] - b[0], a[1] - b[1])


def _hyp(v):
    return math.hypot(v[0], v[1])


def fold_mirror(s: K.Svg, c, a, b, length: float = 46.0, thick: float = 8.0) -> None:
    """Flat multilayer mirror at point c reflecting light arriving from a toward b."""
    ua = _sub(a, c)
    ua = (ua[0] / _hyp(ua), ua[1] / _hyp(ua))
    ub = _sub(b, c)
    ub = (ub[0] / _hyp(ub), ub[1] / _hyp(ub))
    n = (ua[0] + ub[0], ua[1] + ub[1])
    n = (n[0] / _hyp(n), n[1] / _hyp(n))
    tan = (-n[1], n[0])
    p1 = (c[0] + tan[0] * length / 2, c[1] + tan[1] * length / 2)
    p2 = (c[0] - tan[0] * length / 2, c[1] - tan[1] * length / 2)
    q1 = (p1[0] - n[0] * thick, p1[1] - n[1] * thick)
    q2 = (p2[0] - n[0] * thick, p2[1] - n[1] * thick)
    s.polyline([p1, p2, q2, q1], s.stroke("mirror"), 1.2, fill=s.fill("mirror"), close=True)
    for f in (0.35, 0.65):
        r1 = (p1[0] - n[0] * thick * f, p1[1] - n[1] * thick * f)
        r2 = (p2[0] - n[0] * thick * f, p2[1] - n[1] * thick * f)
        s.line(r1[0], r1[1], r2[0], r2[1], s.stroke("mirror"), 0.8)


def draw_euv_path(theme: str) -> str:
    s = K.Svg(960, 580, theme, "EUV scanner light path", EUV_PATH_ALT, generator=GENERATOR)
    euv = _euv(s)
    s.rect(20, 46, 920, 486, fill="none", stroke="grid", sw=1.5, rx=14)
    s.text(36, 72, "vacuum: 13.5 nm light is absorbed by air", size=SMALL, anchor="start", color="muted")
    # --- source: collector (parabolic section with a central hole), droplets, laser
    D = (240.0, 330.0)
    xv, a = 155.0, 45.0 / 105.0**2

    def col_x(y: float) -> float:
        return xv + a * (y - D[1]) ** 2

    for lo, hi in ((225, 318), (342, 435)):
        pts = [(col_x(lo + (hi - lo) * i / 23), lo + (hi - lo) * i / 23) for i in range(24)]
        s.polyline(pts, s.stroke("mirror"), 9)
        s.polyline(pts, s.fill("mirror"), 5)
    s.text(172, 456, "collector (ellipsoidal mirror)", size=SMALL, color="ink2")  # one line: the source crop ends at y = 468
    for k in range(6):
        s.circle(D[0], D[1] - 96 + 16 * k, 3.2, fill="muted", stroke="none")
    s.text(D[0] + 12, D[1] - 90, "tin droplets", size=SMALL, anchor="start", color="ink2")
    s.line(40, D[1], D[0] - 11, D[1], _ir(s), 3, arrow="end")
    lines(s, 40, D[1] - 34, ["CO₂ laser", "10.6 µm"], size=SMALL, anchor="start", dy=15)
    s.circle(D[0], D[1], 7, fill="orange", stroke="none")
    s.text(D[0] + 12, D[1] + 24, "tin plasma", size=SMALL, anchor="start", color="ink2")
    IF = (400.0, 330.0)
    for yc in (250.0, 410.0):
        cpt = (col_x(yc) + 4, yc)
        s.line(D[0], D[1], cpt[0], cpt[1], euv, 1.2)
        s.line(cpt[0], cpt[1], IF[0], IF[1], euv, 2)
    s.circle(IF[0], IF[1], 3.5, fill=euv, stroke="none")
    s.text(IF[0], IF[1] - 30, "intermediate focus", size=SMALL, color="ink2")  # centred above the focus so the source crop can end at x = 506
    # --- illuminator: field facet mirror, pupil facet mirror
    FFM, PFM, MASK = (535.0, 470.0), (650.0, 222.0), (765.0, 104.0)
    s.line(IF[0], IF[1], FFM[0], FFM[1], euv, 2)
    s.line(FFM[0], FFM[1], PFM[0], PFM[1], euv, 2)
    s.line(PFM[0], PFM[1], MASK[0], MASK[1], euv, 2)
    fold_mirror(s, FFM, IF, PFM, length=70)
    fold_mirror(s, PFM, FFM, MASK, length=60)
    s.text(FFM[0] + 44, FFM[1] + 34, "field facet mirror", size=SMALL, color="ink2")
    lines(s, PFM[0] - 22, PFM[1] - 12, ["pupil facet", "mirror"], size=SMALL, anchor="end", dy=16)
    s.text(FFM[0] + 44, FFM[1] + 52, "illuminator", size=LABEL, weight=700, color="ink2")
    # --- reflective mask, face down
    mx0, mx1, my = 680.0, 860.0, MASK[1]
    s.rect(mx0, my - 30, mx1 - mx0, 24, "glass", sw=1.5)
    s.rect(mx0, my - 6, mx1 - mx0, 6, "mirror", sw=1.2)
    for k in range(5):
        s.rect(mx0 + 12 + 34 * k, my, 16, 4, "absorber", sw=0)
    s.text((mx0 + mx1) / 2, my - 38, "reflective mask", size=SMALL, color="ink2")
    lines(s, mx0 - 10, my - 20, ["Mo/Si multilayer", "+ absorber pattern"], size=SMALL, anchor="end", dy=16)
    # --- projection optics: six mirrors
    P = [MASK, (830.0, 178.0), (728.0, 232.0), (840.0, 290.0), (722.0, 346.0), (836.0, 398.0), (764.0, 448.0), (800.0, 496.0)]
    for p, q in zip(P[:-1], P[1:]):
        s.line(p[0], p[1], q[0], q[1], euv, 2)
    for k in range(1, 7):
        fold_mirror(s, P[k], P[k - 1], P[k + 1], length=40, thick=7)
        right = P[k][0] > 790 or (k == 6)
        s.text(P[k][0] + (26 if right else -26), P[k][1] + 5, f"M{k}", size=SMALL, anchor="start" if right else "end",
               color="ink2")
    s.rect(684, 150, 214, 322, fill="none", stroke="ink2", sw=1.2, rx=8)
    s.text(614, 420, "projection optics", size=LABEL, weight=700, color="ink2")  # 8 right of the old 606: clear of the FFM→PFM ray
    lines(s, 614, 437, ["6 mirrors,", "4× reduction"], size=SMALL, dy=17)
    # --- wafer
    s.rect(730, 496, 140, 8, "resist", sw=1.2)
    s.rect(730, 504, 140, 14, "substrate", sw=1.2)
    s.text(884, 512, "wafer", size=SMALL, anchor="start", color="ink2")
    footer(s, "Schematic, not to scale: every optic is a multilayer mirror; real tools differ in geometry and use more illuminator mirrors.")
    return s.render()


# ---------------------------------------------------------------------------------------
# 5. Multiple patterning: LELE vs SADP
# ---------------------------------------------------------------------------------------


def draw_multipatterning(theme: str) -> str:
    s = K.Svg(900, 470, theme, "Pitch splitting: LELE versus SADP", MULTIPATTERNING_ALT, generator=GENERATOR)
    W, P = 150, 76.0  # panel width, litho pitch in drawing units
    q = P / 4

    def base(x0, y0):
        s.rect(x0, y0 + 40, W, 18, "substrate", sw=1)

    def hm_full(x0, y0):
        s.rect(x0, y0 + 30, W, 10, "mirror", sw=1)

    def hm_lines(x0, y0, xs):
        for x in xs:
            s.rect(x0 + x, y0 + 30, q, 10, "mirror", sw=1)

    def resist_lines(x0, y0, xs, h=22):
        for x in xs:
            s.rect(x0 + x, y0 + 30 - h, q, h, "resist", sw=1)

    first = [12.0, 12.0 + P]
    second = [x + P / 2 for x in first]

    def step_label(x0, y0, n, label):
        s.text(x0 + W / 2, y0 - 12, f"{n}. {label}", size=LABEL, color="ink2")

    # --- LELE row
    y0 = 92
    s.text(20, 52, "LELE: litho–etch–litho–etch (two exposures)", size=16, anchor="start", weight=700)
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
        s.line(a + W + 14, y0 + 36, a + W + 56, y0 + 36, "ink2", 2, arrow="end")
    s.line(xs[0] + first[0] + q / 2, y0 + 70, xs[0] + first[1] + q / 2, y0 + 70, "ink2", 1.3, arrow="both")
    s.text(xs[0] + first[0] + q / 2 + P / 2, y0 + 86, "P", size=LABEL, italic=True)
    pts = sorted(first + second)
    s.line(xs[3] + pts[0] + q / 2, y0 + 70, xs[3] + pts[1] + q / 2, y0 + 70, "ink2", 1.3, arrow="both")
    s.text(xs[3] + pts[0] + q / 2 + P / 4, y0 + 86, "P/2", size=LABEL, italic=True)
    s.text(xs[2] + W / 2, y0 + 86, "overlay error shifts these lines", size=SMALL, color="orange")

    # --- SADP row
    y1 = 300
    s.text(20, 258, "SADP: self-aligned (spacer) double patterning (one exposure)", size=16, anchor="start", weight=700)
    xs = [20, 196, 372, 548, 724]
    W2 = 150
    film = 12.0
    wm = P / 2 - film  # uniform spacer pitch P/2 needs mandrel width + film thickness = P/2
    mand = [24.0, 24.0 + P]

    def base2(x0):
        s.rect(x0, y1 + 40, W2, 18, "substrate", sw=1)
        s.rect(x0, y1 + 30, W2, 10, "mirror", sw=1)

    def mandrels(x0):
        for x in mand:
            s.rect(x0 + x, y1 + 8, wm, 22, "mandrel", sw=1)

    base2(xs[0]); mandrels(xs[0])
    s.text(xs[0] + W2 / 2, y1 - 12, "1. mandrels at pitch P", size=LABEL, color="ink2")
    base2(xs[1]); mandrels(xs[1])
    x0 = xs[1]
    d = f"M{x0},{y1 + 30 - film} "
    for x in mand:
        d += (f"L{x0 + x - film},{y1 + 30 - film} L{x0 + x - film},{y1 + 8 - film} "
              f"L{x0 + x + wm + film},{y1 + 8 - film} L{x0 + x + wm + film},{y1 + 30 - film} ")
    d += f"L{x0 + W2},{y1 + 30 - film} L{x0 + W2},{y1 + 30} L{x0},{y1 + 30} Z"
    s.path(d, "spacer", sw=1, opacity=0.9)
    mandrels(xs[1])
    s.text(xs[1] + W2 / 2, y1 - 12, "2. deposit film", size=LABEL, color="ink2")
    base2(xs[2]); mandrels(xs[2])
    for x in mand:
        s.rect(xs[2] + x - film, y1 + 8, film, 22, "spacer", sw=1)
        s.rect(xs[2] + x + wm, y1 + 8, film, 22, "spacer", sw=1)
    s.text(xs[2] + W2 / 2, y1 - 12, "3. etch back", size=LABEL, color="ink2")
    base2(xs[3])
    sp = []
    for x in mand:
        sp += [x - film, x + wm]
    for x in sp:
        s.rect(xs[3] + x, y1 + 8, film, 22, "spacer", sw=1)
    s.text(xs[3] + W2 / 2, y1 - 12, "4. remove mandrels", size=LABEL, color="ink2")
    s.rect(xs[4], y1 + 40, W2, 18, "substrate", sw=1)
    for x in sp:
        s.rect(xs[4] + x, y1 + 30, film, 10, "mirror", sw=1)
    s.text(xs[4] + W2 / 2, y1 - 12, "5. transfer", size=LABEL, color="ink2")
    for a in xs[:-1]:
        s.line(a + W2 + 4, y1 + 36, a + W2 + 22, y1 + 36, "ink2", 2, arrow="end")
    s.line(xs[0] + mand[0] + wm / 2, y1 + 70, xs[0] + mand[1] + wm / 2, y1 + 70, "ink2", 1.3, arrow="both")
    s.text(xs[0] + mand[0] + wm / 2 + P / 2, y1 + 86, "P", size=LABEL, italic=True)
    lines(s, xs[3] + W2 / 2 + 88, y1 + 82, ["spacer pitch ≈ P/2, set by the mandrel width", "and the deposited film thickness"],
          size=SMALL)
    lines(s, 450, 420, ["Repeating the spacer step on the spacers (SAQP) quarters the pitch.",
                        "LELE needs no extra films but ties the result to overlay; SADP is self-aligned but only makes regular lines."],
          size=LABEL)
    footer(s, "Schematic cross-sections, not to scale.")
    return s.render()


# ---------------------------------------------------------------------------------------
# 6. Illumination pupils and off-axis imaging (D5 pupil grammar)
# ---------------------------------------------------------------------------------------


def draw_illumination(theme: str) -> str:
    s = K.Svg(880, 500, theme, "Illumination shapes and why off-axis illumination helps", ILLUMINATION_ALT,
              generator=GENERATOR)
    s.text(20, 34, "Illumination shapes (pupil view; circle = lens NA)", size=16, anchor="start", weight=700)
    shapes = [("conventional", dict(fill="conventional", sigma=0.6)),
              ("annular", dict(fill="annular", sigma=0.85, sigma_inner=0.55)),
              ("dipole", dict(fill="dipole", poles=((0.62, 0.0, 0.22), (-0.62, 0.0, 0.22)))),
              ("quadrupole", dict(fill="quadrupole", poles=tuple((0.62 * math.cos(math.radians(a)), 0.62 * math.sin(math.radians(a)), 0.2)
                                                                for a in (45, 135, 225, 315))))]
    for k, (label, kw) in enumerate(shapes):
        cx = 110 + 220 * k
        s.line(cx - 68, 125, cx + 68, 125, "grid", 1)
        s.line(cx, 57, cx, 193, "grid", 1)
        K.pupil(s, cx, 125, 62, label=label, **kw)
    s.text(20, 264, "A dense grating: which diffraction orders get through the pupil?", size=16, anchor="start", weight=700)
    r = 62
    spacing = 1.3  # first-order offset in pupil units, larger than the pupil radius
    for k, (title, src) in enumerate([("on-axis illumination", 0.0), ("off-axis (dipole pole)", -0.65)]):
        cx, cy = 230 + 420 * k, 368
        s.circle(cx, cy, r, fill="surface", stroke="none")
        pos = [(m, src + m * spacing, 0.0) for m in (-1, 0, 1) if abs(src + m * spacing) <= 2.3]
        K.orders(s, cx, cy, r, pos, stem=False, dot=7, support=1.0)
        s.circle(cx, cy, r, fill="none", stroke="ink2", sw=1.6)
        s.text(cx, cy + r + 24, title, size=14, color="ink2")
        msg = "only the 0 order passes: no image" if k == 0 else "0 and +1 pass: two-beam interference"
        s.text(cx, cy + r + 42, msg, size=LABEL, color="orange" if k == 0 else "ink2")
    footer(s, "Order spacing = λ/(pitch) in pupil units; filled = transmitted, open = blocked.")
    return s.render()


# ---------------------------------------------------------------------------------------
# N5 · the mirror staircase: twelve reflections at 70 % → 1.4 %
# ---------------------------------------------------------------------------------------

MIRRORS = ["collector", "illum. 1", "illum. 2", "illum. 3", "illum. 4", "mask", "M1", "M2", "M3", "M4", "M5", "M6"]
R_MOSI, R_LAB = 0.70, 0.641


def mirror_fractions(r: float, n: int = 12) -> list[float]:
    """Light left after each of ``n`` reflections at reflectance ``r`` (arithmetic, not the engine)."""
    return [r ** (k + 1) for k in range(n)]


def draw_mirror_staircase(theme: str) -> str:
    fr = mirror_fractions(R_MOSI)
    fr_lab = mirror_fractions(R_LAB)
    assert abs(fr[-1] - 0.0138) < 5e-4, fr[-1]
    s = K.Svg(900, 420, theme, "Twelve reflections at 70 percent", MIRROR_ALT, generator=GENERATOR)
    euv = _euv(s)
    x0, x1, y_base, y_top = 120, 880, 330, 60
    n = len(MIRRORS)
    slot = (x1 - x0) / n
    bar_w = slot * 0.56
    s.eyebrow(x0, 30, "Light left after each multilayer reflection (70 % each)", size=SMALL)
    # y axis (linear): 100 % at the top
    for pct in (100, 75, 50, 25, 0):
        y = y_base - (y_base - y_top) * pct / 100
        s.line(x0 - 6, y, x1, y, "grid", 0.8)
        s.text(x0 - 12, y + 4.5, f"{pct} %", size=SMALL, anchor="end", color="ink2")
    # the 100 % source bar
    s.rect(x0 + 8, y_top, bar_w * 0.6, y_base - y_top, fill=s.tint("brand:cyan700", 0.5), stroke="none", rx=2)
    s.text(x0 + 8 + bar_w * 0.3, y_base + 18, "source", size=SMALL, color="ink2")
    s.text(x0 + 8 + bar_w * 0.3, y_top - 8, "100 %", size=SMALL, color="ink2")
    labels_y = y_base + 18
    for k, (name, f) in enumerate(zip(MIRRORS, fr)):
        x = x0 + 8 + bar_w * 0.6 + 12 + k * slot
        h = (y_base - y_top) * f
        colour = euv
        s.rect(x, y_base - h, bar_w, h, fill=tokens.mix(colour, s.t["surface"], 0.85 - 0.04 * k), stroke="none", rx=2)
        # a mirror glyph under the bar
        s.rect(x, y_base + 3, bar_w, 4, "mirror", sw=0.8)
        s.text(x + bar_w / 2, labels_y + 10, name, size=SMALL, color="ink2", rotate=None)
        pct = f"{100 * f:.0f} %" if f >= 0.095 else f"{100 * f:.1f} %"
        s.text(x + bar_w / 2, y_base - h - 6, pct, size=SMALL, color="ink", halo=True)
    # section braces
    def brace(k0, k1, text):
        xa = x0 + 8 + bar_w * 0.6 + 12 + k0 * slot
        xb = x0 + 8 + bar_w * 0.6 + 12 + k1 * slot + bar_w
        y = labels_y + 34
        s.line(xa, y, xb, y, "ink2", 1.1)
        s.line(xa, y - 5, xa, y, "ink2", 1.1)
        s.line(xb, y - 5, xb, y, "ink2", 1.1)
        s.text((xa + xb) / 2, y + 15, text, size=SMALL, color="ink2")

    brace(1, 4, "four illuminator mirrors")
    brace(6, 11, "six projection mirrors")
    # La/B record comparison as a dotted line through the bar tops
    pts = []
    for k, f in enumerate(fr_lab):
        x = x0 + 8 + bar_w * 0.6 + 12 + k * slot + bar_w / 2
        pts.append((x, y_base - (y_base - y_top) * f))
    s.polyline(pts, "violet", 1.4, dash="2 4")
    s.text(pts[2][0] + 34, 206, "dotted: La/B record, 64.1 % per mirror (6.7 nm) → 0.5 % after twelve",
           size=SMALL, anchor="start", color="violet", halo=True)
    s.line(pts[3][0] + 2, pts[3][1] - 4, pts[3][0] + 14, 212, "violet", 0.8)
    # the headline
    xe = pts[-1][0]
    s.text(xe, y_top + 26, "1.4 %", size=26, anchor="end", color="ink", weight=700)
    s.text(xe, y_top + 44, "of the light reaches the wafer", size=SMALL, anchor="end", color="ink2")
    s.line(xe - 2, y_top + 50, xe - 2, y_base - (y_base - y_top) * fr[-1] - 12, "muted", 0.8, dash="2 3")
    footer(s, "Arithmetic on the cited reflectances: 0.70¹² = 1.4 %, 0.641¹² = 0.5 %. Twelve is a representative count.")
    return s.render()


# ---------------------------------------------------------------------------------------
# N7 · two lanes to the planar integrated circuit (origins)
# ---------------------------------------------------------------------------------------

PRINT_LANE = [  # (year, year label, text, ref)
    (1796, "c. 1796", "Senefelder invents lithographic printing", "[1, 2]"),
    (1827, "1826–27", "Niépce's light-hardened bitumen image", "[3, 4]"),
    (1839, "1839", "Ponton: dichromate is light-sensitive", "[4]"),
    (1852, "1852", "Talbot patents dichromated gelatin", "[4]"),
    (1855, "1855", "Poitevin's carbon process", "[5]"),
]
SILICON_LANE = [
    (1955, "1955", "Frosch & Derick: oxide as a diffusion mask", "[6, 7]"),
    (1957, "1957", "Lathrop & Nall pattern metal with photoresist", "[8, 9]"),
    (1959, "1959", "Hoerni's planar process (filed May 1959)", "[10]"),
    (1960, "1960", "first planar integrated circuits (26 May)", "[11]"),
]


def draw_two_lanes(theme: str) -> str:
    s = K.Svg(960, 410, theme, "Two lanes to the planar integrated circuit", TWO_LANES_ALT, generator=GENERATOR)
    hist = s.c("hue:history")
    # lane 1: printing and photography, 1790–1860
    def lane(y, title, span, events, colour, slots, x0=70, x1=890):
        a, b = span
        s.eyebrow(x0, y - 58, title, size=SMALL)
        s.line(x0, y, x1, y, colour, 2.4, opacity=0.6)
        for (yr, ylab, text, ref), slot in zip(events, slots):
            x = x0 + (yr - a) / (b - a) * (x1 - x0)
            s.circle(x, y, 5.5, fill=colour, stroke="surface", sw=1.4)
            ty = {"up": y - 18, "down": y + 24, "up2": y - 46}[slot]
            anchor = "start" if x < x0 + 90 else ("end" if x > x1 - 90 else "middle")
            tx = x - 6 if anchor == "start" else (x + 6 if anchor == "end" else x)
            if slot != "down":
                s.line(x, y - 7, x, ty + 4, "muted", 0.7)
            s.text(tx, ty, ylab, size=LABEL, anchor=anchor, color="ink", weight=600, halo=True)
            s.text(tx, ty + (-14 if slot != "down" else 15), f"{text} {ref}", size=SMALL, anchor=anchor, color="ink2",
                   halo=True)
        return x0, x1

    lane(118, "Printing and photography, 1796–1855", (1790, 1862), PRINT_LANE, hist, ["up", "down", "up", "down", "up2"])
    lane(318, "Silicon, 1955–1960", (1954.3, 1960.7), SILICON_LANE, s.c("blue"), ["up", "down", "up", "up2"])
    # the bridge: coat · expose · develop · etch
    bx, by = 480, 212
    s.path(f"M{850},{150} C{850},{232} {150},{196} {80},{300}", fill="none", stroke="ink2", sw=1.6, dash="6 4", arrow="end")
    w = K.text_width("the loop every layer still runs: coat · expose · develop · etch", LABEL, 600) + 24
    s.rect(bx - w / 2, by - 13, w, 26, fill="surface", stroke="axis", sw=1, rx=13)
    s.text(bx, by + 4.5, "the loop every layer still runs: coat · expose · develop · etch", size=LABEL, color="ink", weight=600)
    footer(s, "Dates as cited on the page; ranges where sources differ (Niépce: 1826 or 1827).")
    return s.render()


# ---------------------------------------------------------------------------------------
# N9 · what had to change at 248 nm (four columns, before → after; + narrow twin)
# ---------------------------------------------------------------------------------------

CHANGED_COLUMNS = [  # (title, before, after, one cited sentence, ref)
    ("Light source", "mercury lamp", "KrF excimer laser",
     ["A lamp emits ~1/30 as much at 248 nm", "as in its other UV bands; the laser", "is line-narrowed to picometres."], "[2, 3, 8]"),
    ("Lens glass", "BK7, quartz", "fused silica + CaF₂",
     ["Below 300 nm ordinary glass absorbs:", "every element had to be fused silica,", "with CaF₂ for colour correction."], "[1]"),
    ("Resist", "DNQ / novolac", "chemically amplified",
     ["DNQ/novolac absorbs too strongly at", "248 nm; one photon releases an acid", "that catalyses many reactions."], "[2, 10, 17]"),
    ("Mask", "chrome on quartz", "chrome + OPC, PSM",
     ["Near the limit the print no longer", "follows the mask: phase shift (1982)", "and OPC pre-distort it."], "[3, 15, 16]"),
]


def _changed_cell(s: K.Svg, x, y, w, col, narrow: bool) -> None:
    title, before, after, sentence, ref = col
    uv = _uv(s)
    s.text(x + w / 2, y, title, size=15 if not narrow else 14, weight=700)
    by = y + 28
    bw = w - 16
    s.rect(x + 8, by, bw, 24, fill=s.tint("muted", 0.12), stroke="axis", sw=0.9, rx=5)
    s.text(x + w / 2, by + 16.5, before, size=LABEL, color="ink2")
    ay = by + 24 + 22
    s.line(x + w / 2, by + 26, x + w / 2, ay - 3, uv, 1.6, arrow="end")
    s.rect(x + 8, ay, bw, 24, fill=s.tint("brand:violet600", 0.14), stroke=uv, sw=1.0, rx=5)
    s.text(x + w / 2, ay + 16.5, after, size=LABEL, color="ink", weight=600)
    if not narrow:
        lines(s, x + w / 2, ay + 46, sentence, size=SMALL, dy=15)
        s.text(x + w / 2, ay + 46 + 3 * 15 + 2, ref, size=SMALL, color="muted")


def draw_changed_248(theme: str, *, narrow: bool = False) -> str:
    if narrow:
        s = K.Svg(600, 360, theme, "What had to change at 248 nm", CHANGED_ALT, generator=GENERATOR)
        s.eyebrow(12, 22, "Mercury lamp → KrF 248 nm: what had to change", size=SMALL)
        w = 290
        for k, col in enumerate(CHANGED_COLUMNS):
            x = 10 + (k % 2) * (w + 6)
            y = 70 + (k // 2) * 150
            _changed_cell(s, x, y, w, col, narrow=True)
        s.text(300, 346, "Before (grey) → after (violet); the sentences and references are on the full-width figure.",
               size=SMALL, color="muted")
        return s.render()
    s = K.Svg(960, 250, theme, "What had to change at 248 nm", CHANGED_ALT, generator=GENERATOR)
    s.eyebrow(16, 22, "Mercury lamp → KrF 248 nm: what had to change at once", size=SMALL)
    w = 232
    for k, col in enumerate(CHANGED_COLUMNS):
        x = 10 + k * (w + 4)
        if k:
            s.line(x - 2, 40, x - 2, 232, "grid", 1)
        _changed_cell(s, x, 54, w, col, narrow=False)
    footer(s, "Before (grey) → after (violet). Qualitative; [n] = reference on this page.")
    return s.render()


# ---------------------------------------------------------------------------------------
# Generators and the registry
# ---------------------------------------------------------------------------------------


def gen_printing_modes() -> list[Path]:
    return K.write_pair("printing-modes", draw_printing_modes, OUT)


def gen_stepper_scanner() -> list[Path]:
    return K.write_pair("stepper-vs-scanner", draw_stepper_scanner, OUT)


def gen_three_ways() -> list[Path]:
    return K.write_pair("three-ways-to-move", draw_three_ways, OUT)


def gen_immersion() -> list[Path]:
    return K.write_pair("immersion-principle", draw_immersion, OUT)


def gen_euv_path() -> list[Path]:
    return K.write_pair("euv-optical-path", draw_euv_path, OUT, crops={"source": EUV_SOURCE_BOX})


def gen_multipatterning() -> list[Path]:
    return K.write_pair("multipatterning-lele-sadp", draw_multipatterning, OUT)


def gen_illumination() -> list[Path]:
    return K.write_pair("illumination-oai", draw_illumination, OUT)


def gen_mirror_staircase() -> list[Path]:
    return K.write_pair("mirror-staircase", draw_mirror_staircase, OUT)


def gen_two_lanes() -> list[Path]:
    return K.write_pair("two-lanes-timeline", draw_two_lanes, OUT)


def gen_changed_248() -> list[Path]:
    return K.write_pair("what-changed-at-248", draw_changed_248, OUT, narrow=lambda t: draw_changed_248(t, narrow=True))


GENERATORS = {
    "printing-modes": gen_printing_modes,
    "stepper-vs-scanner": gen_stepper_scanner,
    "three-ways-to-move": gen_three_ways,
    "immersion-principle": gen_immersion,
    "euv-optical-path": gen_euv_path,
    "multipatterning-lele-sadp": gen_multipatterning,
    "illumination-oai": gen_illumination,
    "mirror-staircase": gen_mirror_staircase,
    "two-lanes-timeline": gen_two_lanes,
    "what-changed-at-248": gen_changed_248,
}

_MODULE = "make_history_schematics.py"


def _reg(**kw) -> registry.AssetSpec:
    base = dict(group="history", subdir="history", fmt="svg", kind="schematic", platform_independent=True)
    base.update(kw)
    return registry.register(registry.AssetSpec(**base), module=_MODULE)


_reg(name="printing-modes", alt=PRINTING_ALT, caption=PRINTING_CAPTION, pages=("history/origins.md",),
     generator=gen_printing_modes)
_reg(name="stepper-vs-scanner", alt=STEPPER_ALT, caption=STEPPER_CAPTION, pages=("history/projection-era.md",),
     generator=gen_stepper_scanner)
_reg(name="three-ways-to-move", alt=THREE_WAYS_ALT, caption=THREE_WAYS_CAPTION, pages=("history/projection-era.md",),
     hero_on=("history/projection-era.md",), generator=gen_three_ways)
_reg(name="immersion-principle", alt=IMMERSION_ALT, caption=IMMERSION_CAPTION,
     pages=("history/immersion-multipatterning.md", "nodes/90-to-28nm.md"),
     hero_on=("history/immersion-multipatterning.md",), generator=gen_immersion)
_reg(name="euv-optical-path", alt=EUV_PATH_ALT, caption=EUV_PATH_CAPTION, pages=("history/euv.md",),
     hero_on=("history/euv.md",), generator=gen_euv_path)
# The source crop is a derived spec of euv-optical-path (DESIGN §5.0): registry.derive_crops() would reject
# a crop that shares its parent's page, and DESIGN places both on history/euv.md, so the crop is registered
# beside its parent with the same crop_of / box fields derive_crops() would set; gen_euv_path writes it.
_reg(name="euv-optical-path-source", alt=EUV_SOURCE_ALT, caption=EUV_SOURCE_CAPTION, pages=("history/euv.md",),
     generator=gen_euv_path, extra={"crop_of": "euv-optical-path", "box": EUV_SOURCE_BOX})
_reg(name="multipatterning-lele-sadp", alt=MULTIPATTERNING_ALT, caption=MULTIPATTERNING_CAPTION,
     pages=("history/immersion-multipatterning.md",), generator=gen_multipatterning)
_reg(name="illumination-oai", alt=ILLUMINATION_ALT, caption=ILLUMINATION_CAPTION, pages=("history/excimer-duv.md",),
     generator=gen_illumination)
_reg(name="mirror-staircase", alt=MIRROR_ALT, caption=MIRROR_CAPTION, pages=("history/euv.md", "future/beyond-euv.md"),
     generator=gen_mirror_staircase)
_reg(name="two-lanes-timeline", alt=TWO_LANES_ALT, caption=TWO_LANES_CAPTION, pages=("history/origins.md",),
     hero_on=("history/origins.md",), generator=gen_two_lanes)
_reg(name="what-changed-at-248", kind="strip", narrow=True, alt=CHANGED_ALT, caption=CHANGED_CAPTION,
     pages=("history/excimer-duv.md",), hero_on=("history/excimer-duv.md",), generator=gen_changed_248)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--only", nargs="+", metavar="NAME", help="render only these figures")
    ap.add_argument("--list", action="store_true")
    args = ap.parse_args(argv)
    if args.list:
        for name in GENERATORS:
            print(name)
        return 0
    for name in args.only or list(GENERATORS):
        if name not in GENERATORS:
            print(f"unknown figure {name!r}; one of {', '.join(GENERATORS)}")
            return 2
        for p in GENERATORS[name]():
            print(f"wrote {registry.rel_to_repo(p)} ({K.size_kb(p):.0f} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
