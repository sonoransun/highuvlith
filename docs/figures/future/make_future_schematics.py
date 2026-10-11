#!/usr/bin/env python3
"""SVG schematics for "The Future" section (docs/future/) — svgkit, standard library only.

Every drawing is an :class:`registry.AssetSpec` (``kind="schematic"`` or ``"strip"``),
written as a light/dark pair into ``docs/assets/images/future/`` through
``svgkit.write_pair`` with the site's tokens (ink, series, materials), ``<title>`` +
``<desc>`` = the alt, no ``<a>``, no SMIL. Platform independent: the nightly regenerates
and diffs them byte for byte.

* ``anamorphic-why``   — incident / reflected cones at an EUV mask for 0.33 NA · 4×, 0.55 NA · 4×
  (overlap) and 0.55 NA · 8× in scan; half-angles ``asin(NA/M)`` are computed, every angle is
  drawn ``ANGLE_GAIN`` × larger, the 6° chief-ray angle is illustrative.
* ``ssmb-concept``     — the steady-state-microbunching storage ring.
* ``erl-fel-loop``     — the energy-recovery-linac FEL loop.
* ``ics-collision``    — inverse Compton scattering, head-on, before / after.
* ``five-limits`` (N15, strip + narrow twin) — the five constraints of ``future/index.md``
  as tiles with their equations as text; the page carries the link row under it.
* ``field-size`` (N19) — 26 × 33 mm at 4×, the 26 × 16.5 mm anamorphic half field with a
  stitching seam, the 6 × 12-inch mask restoring 26 × 33; press-reported wph; announced dates.

    python docs/figures/future/make_future_schematics.py            # write every pair
    python docs/figures/future/make_future_schematics.py --check    # the geometry asserts
    python docs/figures/future/make_future_schematics.py five-limits field-size   # a subset
"""

from __future__ import annotations

import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import registry  # noqa: E402
import svgkit as K  # noqa: E402

OUT = registry.IMAGES / "future"
GENERATOR = "docs/figures/future/make_future_schematics.py"
PAGES_FUTURE = "future/"


# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------


def euv_color(s: K.Svg) -> str:
    """EUV light: brand cyan, deep on the light page and bright on the dark one."""
    return K.tokens.BRAND["cyan700"] if s.theme == "light" else K.tokens.BRAND["cyan300"]


def mtext(s: K.Svg, x, y, text: str, size=K.LABEL, anchor="middle", color="ink", weight=400, lh=1.25, **kw) -> float:
    """Multi-line text (``\\n``); returns the y of the last line."""
    yy = y
    for i, line in enumerate(text.split("\n")):
        yy = y + i * size * lh
        s.text(x, yy, line, size=size, anchor=anchor, color=color, weight=weight, **kw)
    return yy


def undulator(s: K.Svg, x: float, y: float, length: float, periods: int, gap: float = 16.0, mh: float = 9.0) -> None:
    """Row of alternating magnet blocks above and below a beam axis at height y."""
    w = length / periods
    for i in range(periods):
        top, bot = ("ink2", "axis") if i % 2 == 0 else ("axis", "ink2")
        s.rect(x + i * w, y - gap / 2 - mh, w - 1.2, mh, fill=top, opacity=0.8 if top == "axis" else 0.75)
        s.rect(x + i * w, y + gap / 2, w - 1.2, mh, fill=bot, opacity=0.8 if bot == "axis" else 0.75)


def wavy(x1: float, y: float, x2: float, amp: float, wl: float) -> str:
    n = max(8, int(abs(x2 - x1) / 2))
    pts = []
    for i in range(n + 1):
        x = x1 + (x2 - x1) * i / n
        pts.append(f"{x:.1f},{y + amp * math.sin(2 * math.pi * (x - x1) / wl):.1f}")
    return "M" + " L".join(pts)


def packet(x_center: float, y: float, half_len: float, amp: float, wl: float) -> str:
    n = int(4 * half_len)
    pts = []
    for i in range(n + 1):
        x = x_center - half_len + 2 * half_len * i / n
        env = math.exp(-(((x - x_center) / (half_len * 0.5)) ** 2))
        pts.append(f"{x:.1f},{y + amp * env * math.sin(2 * math.pi * (x - x_center) / wl):.1f}")
    return "M" + " L".join(pts)


def panel(s: K.Svg, x, y, w, h, rx=10) -> None:
    s.rect(x, y, w, h, fill="band", rx=rx)


# ---------------------------------------------------------------------------
# 1. Why High-NA is anamorphic
# ---------------------------------------------------------------------------
ANGLE_GAIN = 3.0
#: Illumination (chief-ray) angle at the reticle: ~6° is the commonly quoted value for 0.33 NA
#: tools (SemiEngineering 2021; Levinson, JJAP 61, SD0803 (2022)). Illustrative in every panel.
CRA_DEG = 6.0
PANELS_ANAMORPHIC = [
    ("NXE: NA 0.33, 4×", 0.33, 4.0, "cones separate", False),
    ("NA 0.55 at 4× (not built)", 0.55, 4.0, "cones overlap: the mask would\nshadow its own light path", True),
    ("EXE: NA 0.55, 8× in scan", 0.55, 8.0, "cones separate again", False),
]


def half_angle_deg(na: float, mag: float) -> float:
    return math.degrees(math.asin(na / mag))


ANAMORPHIC_ALT = ("Incident and reflected light cones at an EUV mask in three panels: separate at NA 0.33 and 4x (mask-side "
                  "half-angle 4.7 degrees), overlapping around the normal at NA 0.55 and 4x (7.9 degrees), separate again with "
                  "8x in the scan direction (3.9 degrees); angles drawn three times larger than real.")
ANAMORPHIC_CAPTION = ("<strong>Why High-NA optics are anamorphic.</strong> The mask-side half-angles asin(NA/M) are computed — "
                      "4.7° (0.33 NA, 4×), 7.9° (0.55 NA, 4×), 3.9° (0.55 NA, 8× in scan); every panel uses the same "
                      "illustrative 6° chief-ray angle and every angle is drawn 3× larger than real. "
                      "<em>Schematic, not to scale.</em>")


def draw_anamorphic(theme: str) -> str:
    s = K.Svg(960, 560, theme, "Why High-NA EUV optics are anamorphic", ANAMORPHIC_ALT, generator=GENERATOR)
    euv = euv_color(s)
    pw = 300
    for i, (label, na, mag, verdict, bad) in enumerate(PANELS_ANAMORPHIC):
        x0 = 30 + i * (pw + 15)
        cx = x0 + pw / 2
        ym = 370
        panel(s, x0, 20, pw, 455)
        s.text(cx, 50, label, size=15, weight=600)
        alpha = half_angle_deg(na, mag)
        mtext(s, cx, 74, f"cone half-angle asin(NA/M) = {alpha:.1f}°\nillumination ≈ {CRA_DEG:g}° (illustrative)",
              size=K.SMALL, color="ink2")
        # mask
        s.rect(cx - 120, ym, 240, 16, "substrate", sw=1.2)
        for k in range(1, 4):
            s.line(cx - 120, ym + 4 * k, cx + 120, ym + 4 * k, "ink2", 0.6, opacity=0.5)
        s.text(cx, ym + 36, "reflective multilayer mask", size=K.SMALL, color="muted")
        s.line(cx, ym, cx, 118, "muted", 1.2, dash="4 4")
        s.text(cx + 4, 130, "normal", size=K.SMALL, color="muted", anchor="start")
        length = 225.0

        def ray(theta_deg: float) -> tuple[float, float]:
            th = math.radians(ANGLE_GAIN * theta_deg)
            return cx + length * math.sin(th), ym - length * math.cos(th)

        inc = [(cx, ym), ray(-CRA_DEG - alpha), ray(-CRA_DEG + alpha)]
        ref = [(cx, ym), ray(CRA_DEG - alpha), ray(CRA_DEG + alpha)]
        s.polyline(inc, "violet", 1.5, fill=s.tint("violet", 0.22), close=True)
        s.polyline(ref, euv, 1.5, fill=K.tokens.mix(euv, s.t["surface"], 0.22), close=True)
        xi, yi = ray(-CRA_DEG)
        xr, yr = ray(CRA_DEG)
        s.line(xi, yi, cx, ym, "violet", 2, arrow="end")
        s.line(cx, ym, xr, yr, euv, 2, arrow="end")
        mtext(s, cx - 58, 142, "from\nilluminator", size=K.SMALL, color="ink2", anchor="end")
        mtext(s, cx + 58, 142, "to projection\noptics", size=K.SMALL, color="ink2", anchor="start")
        if bad:
            ov = [(cx, ym), ray(-(alpha - CRA_DEG)), ray(alpha - CRA_DEG)]
            s.polyline(ov, "red", 2, fill=s.tint("red", 0.3), close=True)
        mtext(s, cx, 432, verdict, size=K.LABEL, color="red" if bad else "ink", weight=600)
    mtext(s, 480, 496,
          f"Plane of incidence = scan direction; angles drawn {ANGLE_GAIN:g}× larger than real.\n"
          "Across the slit the optics keep 4× (no overlap problem there), so the wafer field\n"
          "is 26 mm wide but only 16.5 mm long in the scan direction.", size=K.SMALL, color="muted")
    return s.render()


# ---------------------------------------------------------------------------
# 2. SSMB storage ring concept
# ---------------------------------------------------------------------------
SSMB_ALT = ("A phase-locked laser co-propagates with the electrons through a modulator undulator and imprints an energy "
            "modulation; the ring lattice turns it into microbunches at a radiator undulator that emits coherent EUV at a laser "
            "harmonic on every turn; inset: microbunches spaced by the laser wavelength.")
SSMB_CAPTION = ("<strong>The SSMB concept.</strong> A phase-locked laser imprints an energy modulation in the modulator, the "
                "ring lattice turns it into microbunches at the radiator and re-forms them on every turn, so the beam "
                "radiates coherently at a laser harmonic (coherent power ∝ N²|bₙ|²). <em>Schematic, not to scale.</em>")


def draw_ssmb(theme: str) -> str:
    s = K.Svg(960, 540, theme, "Steady-state microbunching (SSMB) storage-ring concept", SSMB_ALT, generator=GENERATOR)
    euv = euv_color(s)
    x1, x2, yt, yb, r = 190, 590, 140, 380, 120
    d = f"M{x1},{yb} L{x2},{yb} A{r},{r} 0 0 0 {x2},{yt} L{x1},{yt} A{r},{r} 0 0 0 {x1},{yb} Z"
    s.path(d, stroke="blue", sw=3)
    s.line(250, yb, 280, yb, "blue", 3, arrow="end")
    s.line(560, yt, 530, yt, "blue", 3, arrow="end")
    s.rect(196, yb - 14, 44, 28, fill=s.tint("blue", 0.18), stroke="blue", sw=1.5, rx=6)
    s.text(218, yb + 36, "RF cavity", size=K.SMALL, color="ink2")
    undulator(s, 390, yb, 150, 10)
    s.text(465, yb - 30, "modulator", size=K.BASE, weight=600)
    s.rect(262, yb + 38, 96, 28, fill=s.tint("orange", 0.2), stroke="orange", sw=1.5, rx=5)
    s.text(310, yb + 57, "laser", size=K.SMALL)
    s.path(wavy(310, yb + 38, 310, 0, 10) + f" L310,{yb + 6}", stroke="orange", sw=2)
    s.line(300, yb + 16, 322, yb - 6, "ink2", 2.5)
    s.path(wavy(316, yb, 545, 3.5, 16), stroke="orange", sw=2)
    mtext(s, 372, yb + 60, "phase-locked, ≈ 1 µm: imprints\nan energy modulation", size=K.SMALL, anchor="start", color="ink2")
    undulator(s, 300, yt, 170, 12)
    s.text(385, yt + 44, "radiator", size=K.BASE, weight=600)
    s.path(wavy(300, yt - 32, 480, 3, 7), stroke=euv, sw=2)
    s.line(470, yt - 32, 930, yt - 32, euv, 3, arrow="end")
    mtext(s, 940, yt - 84, "coherent EUV at a laser harmonic\n(simulator preset: 1053 nm / 78 = 13.5 nm)", size=K.SMALL,
          anchor="end", color="ink2")
    mtext(s, 390, 250, "ring lattice (longitudinal focusing):\nenergy modulation → microbunches at the radiator,\n"
          "undone and re-formed on every turn", size=K.LABEL)
    ix, iy, iw, ih = 735, 250, 205, 150
    panel(s, ix, iy, iw, ih, rx=8)
    s.text(ix + iw / 2, iy + 22, "density at the radiator", size=K.SMALL, weight=600)
    base = iy + ih - 32
    s.line(ix + 12, base, ix + iw - 12, base, "muted", 1)
    pts = []
    n = 220
    for k in range(n + 1):
        xx = ix + 12 + (iw - 24) * k / n
        ph = 2 * math.pi * 4.3 * k / n
        val = math.exp(-(((ph % (2 * math.pi)) - math.pi) ** 2) / 0.02)
        pts.append((xx, base - 78 * val))
    s.polyline(pts, "blue", 1.8)
    s.text(ix + iw / 2, base + 20, "spacing = laser wavelength", size=K.SMALL, color="ink2")
    mtext(s, 30, 510, "The stored beam (hundreds of MeV) passes both undulators on every turn. Coherent power ∝ N²|bₙ|² "
          "(N electrons,\nbunching factor bₙ at harmonic n) versus ∝ N without microbunching. Schematic, not to scale.",
          size=K.SMALL, anchor="start", color="muted")
    return s.render()


# ---------------------------------------------------------------------------
# 3. Energy-recovery-linac FEL loop
# ---------------------------------------------------------------------------
ERL_ALT = ("An injector feeds a superconducting linac through a merger; the accelerated beam is turned by an arc into a long "
           "undulator that emits EUV to the scanners, a return arc brings the spent beam back half an RF period late so the "
           "linac decelerates it and recovers its energy before a low-energy beam dump.")
ERL_CAPTION = ("<strong>Energy-recovery-linac FEL: accelerate, lase, then decelerate the same beam in the same cavities.</strong> "
               "The spent beam returns half an RF period late and hands its energy back before a low-energy dump, so the beam "
               "current — and the average FEL power — can far exceed what single-pass RF power would allow. "
               "<em>Schematic, not to scale.</em>")


def draw_erl(theme: str) -> str:
    s = K.Svg(960, 520, theme, "Energy-recovery linac (ERL) driven free-electron laser", ERL_ALT, generator=GENERATOR)
    euv = euv_color(s)
    yb, yt = 370, 140
    xr = 760
    s.rect(300, yb - 26, 330, 52, fill=s.tint("blue", 0.18), stroke="blue", sw=1.5, rx=10)
    for k in range(8):
        s.ellipse(330 + k * 41, yb, 14, 18, fill="none", stroke="blue", sw=1.2)
    mtext(s, 470, yb - 52, "superconducting linac: accelerates on pass 1,\ndecelerates on pass 2", size=K.SMALL)
    s.rect(40, yb + 46, 100, 40, fill="band", stroke="ink2", sw=1.2, rx=6)
    s.text(90, yb + 71, "injector", size=K.LABEL)
    s.path(f"M140,{yb + 66} C200,{yb + 66} 240,{yb + 6} 300,{yb + 6}", stroke="blue", sw=2.5, arrow="end")
    s.text(218, yb + 62, "merger", size=K.SMALL, color="ink2", anchor="start")
    s.line(630, yb - 6, xr, yb - 6, "blue", 2.5)
    rr = (yb - 6 - yt) / 2
    s.path(f"M{xr},{yb - 6} A{rr},{rr} 0 0 0 {xr},{yt}", stroke="blue", sw=2.5)
    s.line(xr, yt, 250, yt, "blue", 2.5)
    s.path(f"M250,{yt} A{rr},{rr} 0 0 0 250,{yb - 6}", stroke="blue", sw=2.5)
    s.line(250, yb - 6, 300, yb - 6, "blue", 2.5, arrow="end")
    s.line(700, yt, 670, yt, "blue", 2.5, arrow="end")
    s.text(xr - 14, 234, "arc", size=K.SMALL, color="ink2", anchor="end")
    mtext(s, 264, 226, "return arc:\narrives ½ RF period late", size=K.SMALL, color="ink2", anchor="start")
    undulator(s, 330, yt, 280, 18)
    s.text(470, yt + 44, "undulator (FEL)", size=K.BASE, weight=600)
    s.path(wavy(330, yt - 34, 610, 3, 7), stroke=euv, sw=2)
    s.line(600, yt - 34, 930, yt - 34, euv, 3, arrow="end")
    s.text(930, yt - 58, "EUV to one or several scanners", size=K.SMALL, anchor="end", color="ink2")
    s.line(630, yb + 6, 820, yb + 6, "blue", 2.5, arrow="end")
    s.rect(825, yb - 12, 100, 36, fill="band", stroke="ink2", sw=1.2, rx=6)
    s.text(875, yb + 11, "beam dump", size=K.SMALL)
    s.text(875, yb + 44, "(low energy)", size=K.SMALL, color="ink2")
    mtext(s, 30, 486, "Energy recovery lets the beam current (and so the average FEL power) be far higher\n"
          "than the RF power a single-pass linac could supply. Schematic, not to scale.", size=K.SMALL, anchor="start",
          color="muted")
    return s.render()


# ---------------------------------------------------------------------------
# 4. Inverse Compton scattering, head-on
# ---------------------------------------------------------------------------
ICS_LASER_NM = 1030.0
ICS_E_MEV = 1.72
ICS_GAMMA = 1.0 + ICS_E_MEV / 0.51099895
ICS_ALT = ("Inverse Compton scattering, before and after: an electron bunch meets a counter-propagating laser pulse; scattered "
           "photons leave in a forward cone of half-angle 1 over gamma, 4 gamma squared shorter in wavelength — 1030 nm on "
           "1.72 MeV electrons gives 13.5 nm in a 13 degree cone.")
ICS_CAPTION = ("<strong>Inverse Compton scattering, head-on.</strong> A 1030 nm laser on 1.72 MeV electrons (γ ≈ 4.4) gives "
               "13.5 nm on axis inside a cone of half-angle 1/γ ≈ 13°, and the wavelength changes across the cone. "
               "<em>Schematic, not to scale.</em>")


def ics_on_axis_nm(laser_nm: float = ICS_LASER_NM, gamma: float = ICS_GAMMA) -> float:
    return laser_nm / (4 * gamma * gamma)


def draw_ics(theme: str) -> str:
    s = K.Svg(960, 470, theme, "Inverse Compton scattering, head-on geometry", ICS_ALT, generator=GENERATOR)
    euv = euv_color(s)
    y1 = 95
    s.text(30, 36, "before", size=K.LABEL, anchor="start", color="muted", weight=600)
    s.ellipse(230, y1, 60, 22, fill=s.tint("blue", 0.18), stroke="blue", sw=2)
    for k in range(14):
        s.circle(190 + (k * 37) % 80, y1 - 12 + (k * 23) % 24, 3, fill="blue", stroke="none")
    s.line(300, y1, 360, y1, "blue", 3, arrow="end")
    s.text(230, y1 + 45, "electron bunch, Lorentz factor γ", size=K.LABEL)
    s.path(packet(680, y1, 70, 18, 16), stroke="orange", sw=2.2)
    s.line(600, y1, 540, y1, "orange", 3, arrow="end")
    s.text(680, y1 + 45, "laser pulse, wavelength λₗ", size=K.LABEL)
    s.circle(460, y1, 6, fill="none", stroke="ink", sw=1.5)
    s.text(460, y1 - 22, "interaction point", size=K.SMALL, color="ink2")
    s.line(30, 172, 930, 172, "grid", 1)
    y2 = 280
    s.text(30, 200, "after", size=K.LABEL, anchor="start", color="muted", weight=600)
    half = math.radians(math.degrees(1 / ICS_GAMMA))
    L = 400
    cx = 460
    s.polyline([(cx, y2), (cx + L * math.cos(half), y2 - L * math.sin(half)), (cx + L * math.cos(half), y2 + L * math.sin(half))],
               euv, 1.5, fill=K.tokens.mix(euv, s.t["surface"], 0.2), close=True)
    s.line(cx, y2, cx + L, y2, euv, 1.2, dash="5 4")
    s.ellipse(cx + 250, y2, 40, 16, fill=s.tint("blue", 0.18), stroke="blue", sw=1.5)
    s.text(cx + 250, y2 + 36, "electrons (slightly slower)", size=K.SMALL, color="ink2")
    mtext(s, cx + L - 12, y2 - 60, "scattered photons,\ncone half-angle ≈ 1/γ", size=K.LABEL, anchor="end")
    s.path(packet(300, y2, 60, 14, 16), stroke="orange", sw=2, opacity=0.5)
    s.text(300, y2 + 40, "unscattered laser light", size=K.SMALL, color="ink2")
    s.text(480, 390, "λₓ ≈ λₗ (1 + a₀²/2 + γ²θ²) / 4γ²", size=17, weight=600)
    s.text(480, 418, f"e.g. {ICS_LASER_NM:.0f} nm laser, {ICS_E_MEV:g} MeV electrons (γ ≈ {ICS_GAMMA:.2f}) → "
           f"{ics_on_axis_nm():.1f} nm on axis; 1/γ ≈ {1 / ICS_GAMMA:.2f} rad ≈ {math.degrees(1 / ICS_GAMMA):.0f}°",
           size=K.LABEL, color="ink2")
    s.text(480, 452, "Head-on, single scattering; a₀ = laser strength parameter. Schematic, not to scale.", size=K.SMALL,
           color="muted")
    return s.render()


# ---------------------------------------------------------------------------
# 5. N15 — the five limits strip
# ---------------------------------------------------------------------------
LIMITS = [
    ("Diffraction", "HP = k₁ λ / NA", "k₁ ≥ 0.25 single exposure"),
    ("Photons", "N ∝ λ³ / NA²", "at fixed dose and k₁"),
    ("Resist trade-off", "Z = HP³ · LER² · D", "pick two of three"),
    ("Electron blur", "σ ≈ 2 nm", "set by transport, not λ"),
    ("Throughput", "t = t₀ + D·A / P", "dose buys output"),
]
LIMITS_ALT = ("Five tiles, one per constraint: diffraction HP = k1 lambda over NA with k1 at least 0.25; photons per feature "
              "proportional to lambda cubed over NA squared; the resist trade-off Z = HP cubed times LER squared times dose; "
              "electron blur of about 2 nm; throughput t = t0 plus dose times area over power.")
LIMITS_CAPTION = ("<strong>Five constraints frame every page of this section.</strong> Diffraction (k₁ ≥ 0.25), photons per "
                  "feature ∝ λ³/NA², the resolution–roughness–dose trade-off Z = HP³·LER²·D, an electron blur of about 2 nm "
                  "that does not shrink with λ, and a throughput t = t₀ + D·A/P that dose buys. "
                  "<em>Equations only; no model badge.</em>")


def draw_limits(theme: str, narrow: bool = False) -> str:
    if not narrow:
        W, H, cols = 960, 180, 5
        tw, gap, m, y0, th = 176, 12, 20, 28, 124
    else:
        W, H, cols = 600, 360, 3
        tw, gap, m, y0, th = 186, 11, 10, 20, 150
    s = K.Svg(W, H, theme, "The five limits that frame the future of lithography", LIMITS_ALT, generator=GENERATOR)
    hue = s.c("hue:future")
    for i, (title, eq, note) in enumerate(LIMITS):
        row, col = divmod(i, cols)
        if narrow and row == 1:
            x = m + (col + 0.5) * (tw + gap)  # the last two tiles centred on the second row
        else:
            x = m + col * (tw + gap)
        y = y0 + row * (th + gap)
        s.rect(x, y, tw, th, fill="band", rx=8)
        s.rect(x, y, tw, 3, fill=hue, rx=0)
        s.text(x + 12, y + 28, f"{i + 1}", size=K.SMALL, anchor="start", color="muted", weight=600)
        s.text(x + 30, y + 28, title, size=K.LABEL, anchor="start", weight=600)
        s.text(x + tw / 2, y + 68, eq, size=17 if not narrow else 18, weight=600, color="ink")
        s.text(x + tw / 2, y + 96, note, size=K.SMALL, color="ink2")
    return s.render()


# ---------------------------------------------------------------------------
# 6. N19 — field size
# ---------------------------------------------------------------------------
FIELD_FULL_MM = (26.0, 33.0)
FIELD_HALF_MM = (26.0, 16.5)
WPH_UNSTITCHED, WPH_STITCHED = 175, 125  # press-reported (Tom's Hardware), see high-na-and-hyper-na.md
FIELD_ALT = ("Three wafer fields to scale: 26 by 33 mm from a 6-inch mask at 4x, a 26 by 16.5 mm anamorphic half field with a "
             "stitching seam for larger dies, and 26 by 33 mm restored by a 6 by 12-inch mask; stitching is press-reported to "
             "cut 175 wafers per hour to about 125; the dates are announced plans.")
FIELD_CAPTION = ("<strong>Half the field, then a new mask to get it back.</strong> A 6-inch mask exposes 26 × 33 mm at 4×, "
                 "anamorphic optics 26 × 16.5 mm (dies larger than a half field are stitched across a seam), and a 6 × 12-inch "
                 "mask restores 26 × 33 mm; 175 → ~125 wafers per hour with stitching is press-reported and TSMC's 2030–33 "
                 "dates are announced plans. <em>Stitching throughput is press-reported; dates are plans.</em>")


def draw_field(theme: str) -> str:
    s = K.Svg(960, 470, theme, "High-NA field size: full field, anamorphic half field, 6 × 12-inch mask", FIELD_ALT,
              generator=GENERATOR)
    scale = 7.0  # px per mm
    fw, fh = FIELD_FULL_MM[0] * scale, FIELD_FULL_MM[1] * scale
    hh = FIELD_HALF_MM[1] * scale
    cols = [
        ("NXE · 0.33 NA · 4×", "6-inch mask", f"{FIELD_FULL_MM[0]:g} × {FIELD_FULL_MM[1]:g} mm", "full", "In production", ""),
        ("EXE · 0.55 NA · 4× / 8×", "6-inch mask", f"{FIELD_HALF_MM[0]:g} × {FIELD_HALF_MM[1]:g} mm", "half", "In production", "selected layers"),
        ("EXE · 0.55 NA · 4× / 8×", "6 × 12-inch mask", f"{FIELD_FULL_MM[0]:g} × {FIELD_FULL_MM[1]:g} mm", "restored", "Announced", "TSMC: pilot line 2031, production 2033"),
    ]
    for i, (tool, mask, size, kind, rung, qual) in enumerate(cols):
        cx = 170 + i * 310
        s.text(cx, 36, tool, size=K.BASE, weight=600)
        s.text(cx, 56, mask, size=K.SMALL, color="ink2")
        x, y = cx - fw / 2, 80
        # the wafer field outline (full) and what one exposure covers
        s.rect(x, y, fw, fh, fill="none", stroke="axis", sw=1.2, dash="4 3" if kind == "half" else None)
        if kind == "half":
            s.rect(x, y, fw, hh, fill=s.tint("violet", 0.22), stroke="violet", sw=1.5)
            s.rect(x, y + hh, fw, hh, fill=s.tint("violet", 0.1), stroke="violet", sw=1.5, dash="3 3")
            s.line(x - 8, y + hh, x + fw + 8, y + hh, "red", 2)
            s.text(x + fw / 2, y + hh + 15, "stitching seam", size=K.SMALL, anchor="middle", color="red", halo=True)
            s.text(cx, y + hh / 2 + 5, "exposure 1", size=K.SMALL, color="ink2")
            s.text(cx, y + hh + hh / 2 + 12, "exposure 2 (stitched)", size=K.SMALL, color="ink2")
            s.dim_v(x - 18, y, y + hh, f"{FIELD_HALF_MM[1]:g} mm", size=K.SMALL)
        else:
            fill = s.tint("violet", 0.22) if kind == "full" else s.tint("aqua", 0.3)
            s.rect(x, y, fw, fh, fill=fill, stroke="violet" if kind == "full" else "aqua", sw=1.5)
            s.text(cx, y + fh / 2 + 5, "one exposure", size=K.SMALL, color="ink2")
            s.dim_v(x - 18, y, y + fh, f"{FIELD_FULL_MM[1]:g} mm", size=K.SMALL)
        s.dim_h(x, x + fw, y + fh + 16, f"{FIELD_FULL_MM[0]:g} mm", size=K.SMALL, above=False)
        s.text(cx, y + fh + 52, size + " per exposure", size=K.LABEL, weight=600)
        K.chip(s, cx, y + fh + 76, rung, outlined=True, anchor="middle", size=K.SMALL)
        if qual:
            s.text(cx, y + fh + 100, qual, size=K.SMALL, color="muted")
    mtext(s, 480, 426, f"Stitched dies: ≥ {WPH_UNSTITCHED} wph (ASML spec, unstitched half fields) → about {WPH_STITCHED} wph "
          "with stitching, press-reported; no ASML publication found.\nThe 6 × 12-inch format doubles the mask along the 8× "
          "direction. TSMC: 6 × 6-inch High-NA volume in 2030,\n6 × 12-inch pilot line 2031, production 2033 — announced "
          "plans, not delivered results.", size=K.SMALL, color="muted", lh=1.35)
    return s.render()


# ---------------------------------------------------------------------------
# generators + registry
# ---------------------------------------------------------------------------


def gen_anamorphic() -> list[Path]:
    return K.write_pair("anamorphic-why", draw_anamorphic, OUT)


def gen_ssmb() -> list[Path]:
    return K.write_pair("ssmb-concept", draw_ssmb, OUT)


def gen_erl() -> list[Path]:
    return K.write_pair("erl-fel-loop", draw_erl, OUT)


def gen_ics() -> list[Path]:
    return K.write_pair("ics-collision", draw_ics, OUT)


def gen_limits() -> list[Path]:
    return K.write_pair("five-limits", draw_limits, OUT, narrow=lambda t: draw_limits(t, narrow=True))


def gen_field() -> list[Path]:
    return K.write_pair("field-size", draw_field, OUT)


def _spec(name, kind, alt, caption, pages, gen, **kw) -> registry.AssetSpec:
    return registry.AssetSpec(name=name, kind=kind, group="future", subdir="future", fmt="svg", alt=alt, caption=caption,
                              pages=pages, generator=gen, platform_independent=True, **kw)


registry.register(_spec("anamorphic-why", "schematic", ANAMORPHIC_ALT, ANAMORPHIC_CAPTION,
                        ("future/high-na-and-hyper-na.md",), gen_anamorphic), module="make_future_schematics.py")
registry.register(_spec("ssmb-concept", "schematic", SSMB_ALT, SSMB_CAPTION,
                        ("future/accelerator-light-sources.md", "sources/ssmb.md"), gen_ssmb), module="make_future_schematics.py")
registry.register(_spec("erl-fel-loop", "schematic", ERL_ALT, ERL_CAPTION,
                        ("future/accelerator-light-sources.md", "sources/xfel.md"), gen_erl), module="make_future_schematics.py")
registry.register(_spec("ics-collision", "schematic", ICS_ALT, ICS_CAPTION,
                        ("future/accelerator-light-sources.md", "sources/inverse-compton.md"), gen_ics), module="make_future_schematics.py")
registry.register(_spec("five-limits", "strip", LIMITS_ALT, LIMITS_CAPTION, ("future/index.md",), gen_limits, narrow=True),
                  module="make_future_schematics.py")
registry.register(_spec("field-size", "schematic", FIELD_ALT, FIELD_CAPTION, ("future/high-na-and-hyper-na.md",), gen_field),
                  module="make_future_schematics.py")

GENERATORS = {
    "anamorphic-why": gen_anamorphic, "ssmb-concept": gen_ssmb, "erl-fel-loop": gen_erl, "ics-collision": gen_ics,
    "five-limits": gen_limits, "field-size": gen_field,
}


def check() -> tuple[list[str], list[str]]:
    """Geometry asserts: the computed half-angles and the ICS example match the captions."""
    errors: list[str] = []
    warnings: list[str] = []
    want = {(0.33, 4.0): 4.7, (0.55, 4.0): 7.9, (0.55, 8.0): 3.9}
    for (na, mag), deg in want.items():
        got = round(half_angle_deg(na, mag), 1)
        if got != deg:
            errors.append(f"anamorphic-why: asin({na}/{mag:g}) = {got}°, caption says {deg}°")
    if abs(ics_on_axis_nm() - 13.5) > 0.2:
        errors.append(f"ics-collision: {ICS_LASER_NM} nm / 4γ² = {ics_on_axis_nm():.2f} nm, not 13.5")
    if round(math.degrees(1 / ICS_GAMMA)) != 13:
        errors.append(f"ics-collision: 1/γ = {math.degrees(1 / ICS_GAMMA):.1f}°, caption says 13°")
    if FIELD_HALF_MM[1] * 2 != FIELD_FULL_MM[1]:
        errors.append("field-size: the half field is not half the full field")
    page = registry.DOCS / "future" / "index.md"
    if page.exists():
        text = page.read_text(encoding="utf-8")
        for needle in ("k_1 \\ge 0.25", "LER", "2 nm", "t_0 + D A"):
            if needle not in text:
                warnings.append(f"five-limits: future/index.md no longer states {needle!r}")
    return errors, warnings


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if "--check" in argv:
        e, w = check()
        for x in w:
            print("warning:", x)
        for x in e:
            print("error:", x)
        print(f"future schematics check: {len(e)} error(s), {len(w)} warning(s)")
        return 1 if e else 0
    only = [a for a in argv if not a.startswith("--")]
    for name, fn in GENERATORS.items():
        if only and name not in only:
            continue
        for p in fn():
            print(f"wrote {registry.rel_to_repo(p)} ({p.stat().st_size / 1024:.1f} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
