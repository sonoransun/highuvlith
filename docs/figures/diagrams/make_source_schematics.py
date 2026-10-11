#!/usr/bin/env python3
"""S4 — the six source-cluster mechanism schematics (DESIGN §5.4, V3c).

One scripted SVG per nav cluster of ``Sources`` (standard library only through
``svgkit`` — no plotting library, so every asset is ``platform_independent=True`` and the
nightly can regenerate and diff it byte for byte). Every family page in a cluster embeds
its cluster's file as the *Mechanism* slot of its Physics Strip (``pages=`` lists the
families; the engine slot beside it is ``fig_sources.py`` / ``fig_sources_b.py``). Each
drawing is a set of lanes — one per family or per physical route — reading left to
right from the pump to the illuminator pupil, drawn in the D5 pupil grammar wherever a
beam meets a pupil (the fill is the preset's engine default: ``SourceConfig.<preset>()
.illumination()``), or ending on the shadow-printing mask/resist where no projection
pupil exists (the lab white-beam cluster).

* ``sources-cluster-excimer`` — the excimer-laser lane (pulsed discharge → KrF*/ArF*/F₂
  emission → line narrowing → E95 bandwidth) and the Hg-arc-lamp lane (arc → broadband
  → interference filter → g/h/i line) → top-hat pupil σ 0.7
  (``sources/duv-heritage.md``, ``sources/vuv-excimer.md``);
* ``sources-cluster-plasma`` — LPP (drive laser → Sn droplet → plasma → 2π emission →
  collector → IF) and DPP/LDP (capacitor bank → pinch → plasma column → 2π at the source
  → collector + debris trap → étendue cut → IF) → top-hat pupil σ 0.9
  (``sources/lpp.md``, ``sources/dpp.md``);
* ``sources-cluster-accelerator`` — synchrotron (ring → bending magnet / undulator →
  sinc² line → coherent fraction ζ → coherent-core pupil), LPA-FEL (drive laser →
  wakefield → bunch → undulator → gain criterion → SASE) and Smith–Purcell (keV beam →
  evanescent field → grating → angle-tuned line)
  (``sources/synchrotron.md``, ``sources/lpa-fel.md``, ``sources/smith-purcell.md``);
* ``sources-cluster-tabletop`` — HHG (fs driver → gas jet → three-step → odd comb →
  phase matching) and the plasma soft-X-ray laser (pump → plasma column → inversion →
  ASE → fixed line) → coherent-core pupils (``sources/hhg.md``, ``sources/soft-xray-laser.md``);
* ``sources-cluster-labxray`` — the X-ray tube (cathode → kVp → thick anode →
  bremsstrahlung + K/L lines → Be window → 1/4πd²) and the LWFA betatron (drive laser →
  bubble → trapped electrons → betatron oscillation → wiggler-regime burst) → shadow
  printing, no pupil (``sources/xray-tube.md``, ``sources/betatron.md``);
* ``sources-cluster-quantum`` — pump → SPDC crystal → NOON preparation → interferometric
  exposure → the hypothetical N-photon resist; imaged classically by the pipeline
  (``sources/entangled-photon.md``).

    python docs/figures/diagrams/make_source_schematics.py              # write all six pairs
    python docs/figures/diagrams/make_source_schematics.py plasma       # only the names matching
    python docs/figures/diagrams/make_source_schematics.py --check      # self-checks (+ engine pupils)
    python docs/figures/diagrams/make_source_schematics.py --highlight lpp plasma   # preview one lane lit

``draw(theme, highlight=<family>)`` dims every lane that is not the family's (the
committed cluster file is drawn with ``highlight=None``: every lane at full ink, so one
file serves every family page of the cluster, §5.4). ``--check`` renders every
``highlight`` of every cluster and, when the extension imports, verifies that the pupil
fill drawn for each lane equals the preset's ``illumination()``.
"""

from __future__ import annotations

import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import registry  # noqa: E402
import svgkit as K  # noqa: E402
import tokens  # noqa: E402

OUT = registry.IMAGES / "diagrams"
GENERATOR = "docs/figures/diagrams/make_source_schematics.py"
MODULE = "make_source_schematics.py"

W = 960
HEADER = 44          # cluster eyebrow row
LANE_H = 164         # one lane: eyebrow · icon row · title · two detail lines
NOTES_H = 90         # an optional row of note boxes under the lanes (the quantum cluster)
FOOTER = 50          # two footer lines: the cluster note · "Schematic, not to scale."
X0 = 20              # first station's left edge
PUPIL_X = 902        # pupil centre
PUPIL_R = 24
GAP = 16             # between stations
ICON = 46            # icon box side

T_TITLE = 14
T_LABEL = 13.5
T_SMALL = 12.5
DIM = 0.28           # opacity of a lane that is not highlighted

TRAILER = "<em>Schematic, not to scale; no model badge.</em>"


# ---------------------------------------------------------------------------
# Pupil facts: the engine default of each lane's preset (verified by --check)
# ---------------------------------------------------------------------------

#: lane key -> (preset factory, kwargs, shape, sigma, coherent fraction or None)
PUPILS = {
    "excimer": ("f2_laser", {}, "conventional", 0.7, None),
    "lamp": ("hg_i_line", {}, "conventional", 0.7, None),
    "lpp": ("lpp_sn_13nm5", {}, "conventional", 0.9, None),
    "dpp": ("dpp_sn_13nm5", {}, "conventional", 0.9, None),
    "synchrotron": ("synchrotron_compact_euv", {}, "coherent_gaussian", 0.1678, 0.0888),
    "lpa-fel": ("lpa_fel_bella_25nm", {}, "conventional", 0.7, 0.9),
    "smith-purcell": ("smith_purcell", {}, "conventional", 0.7, None),
    "hhg": ("hhg_ne_13nm5", {}, "coherent_gaussian", 0.0527, 0.9),
    "sxrl": ("sxrl_ag_13nm9", {}, "coherent_gaussian", 0.1118, 0.2),
    "xray-tube": ("xray_tube", {}, "conventional", 1.0, None),
    "betatron": ("betatron", {}, "conventional", 1.0, None),
    "noon": ("entangled_noon", {}, "coherent_gaussian", 0.05, 1.0),
}


def _pupil_label(key: str) -> tuple[str, str]:
    """Two short lines under the pupil: what the fill is and where it comes from."""
    _, _, shape, sigma, zeta = PUPILS[key]
    if shape == "conventional":
        return (f"top-hat σ {sigma:g}", "engine default")
    return (f"coherent σ {sigma:.2f}", f"from ζ {zeta:g}")


# ---------------------------------------------------------------------------
# Small pictograms (each fits a 46-unit box centred on (cx, cy))
# ---------------------------------------------------------------------------


def _beam(s: K.Svg, x1, y1, x2, y2, color, sw=2.0, arrow="end", dash=None, opacity=None):
    s.line(x1, y1, x2, y2, color, sw, dash=dash, arrow=arrow, opacity=opacity)


def _lamp(s, cx, cy, col):
    s.ellipse(cx, cy - 2, 13, 16, fill=s.tint(col, 0.25), stroke=col, sw=1.5)
    s.rect(cx - 5, cy + 13, 10, 8, fill=s.tint("ink", 0.5), stroke="none", rx=1)
    s.line(cx - 4, cy + 13, cx - 2, cy + 2, "ink2", 1.2)
    s.line(cx + 4, cy + 13, cx + 2, cy + 2, "ink2", 1.2)
    for a in (-60, -30, 0, 30, 60):
        r = math.radians(a - 90)
        s.line(cx + 16 * math.cos(r), cy - 2 + 18 * math.sin(r), cx + 22 * math.cos(r), cy - 2 + 24 * math.sin(r), col, 1.3)


def _filter(s, cx, cy, col):
    s.rect(cx - 5, cy - 20, 10, 40, fill=s.tint("blue", 0.18), stroke="blue", sw=1.3, rx=1)
    for i in range(5):
        s.line(cx - 22, cy - 12 + i * 6, cx - 6, cy - 12 + i * 6, "muted", 1.0, opacity=0.8)
    _beam(s, cx + 6, cy, cx + 24, cy, col, 2.2)


def _cavity(s, cx, cy, col):
    s.rect(cx - 22, cy - 13, 44, 26, fill=s.tint(col, 0.10), stroke="ink2", sw=1.3, rx=2)
    s.rect(cx - 18, cy - 17, 36, 4, fill=s.tint("ink", 0.55), stroke="none")
    s.rect(cx - 18, cy + 13, 36, 4, fill=s.tint("ink", 0.55), stroke="none")
    for i in range(3):
        x = cx - 12 + i * 12
        s.polyline([(x, cy - 12), (x - 3, cy - 5), (x + 3, cy + 1), (x - 2, cy + 7), (x, cy + 12)], col, 1.3)
    s.text(cx, cy + 4, "HV", size=T_SMALL, color="ink2", weight=600, halo=True)


def _spectrum(s, cx, cy, kind, col, w=44, h=30):
    """A tiny spectrum: baseline + shape. kinds: broad · narrow · doublet · comb · continuum · bm · line."""
    x0, y0 = cx - w / 2, cy + h / 2
    s.line(x0, y0, x0 + w, y0, "axis", 1.0)
    pts = []
    n = 40
    if kind == "broad":
        pts = [(x0 + i * w / n, y0 - h * math.exp(-((i / n - 0.5) / 0.22) ** 2)) for i in range(n + 1)]
    elif kind == "narrow":
        pts = [(x0 + i * w / n, y0 - h * math.exp(-((i / n - 0.5) / 0.045) ** 2)) for i in range(n + 1)]
    elif kind == "doublet":
        pts = [(x0 + i * w / n, y0 - h * (math.exp(-((i / n - 0.42) / 0.04) ** 2) + 0.7 * math.exp(-((i / n - 0.58) / 0.04) ** 2))) for i in range(n + 1)]
    elif kind == "continuum":  # Kramers: linear fall to the Duane–Hunt limit, hardened by a window
        pts = [(x0 + i * w / n, y0 - h * max(0.0, (1 - i / n) * min(1.0, (i / n) / 0.25))) for i in range(n + 1)]
    elif kind == "bm":  # bending-magnet S(E/E_c): broad, long tail
        pts = [(x0 + i * w / n, y0 - h * ((i / n) / 0.3) * math.exp(1 - (i / n) / 0.3)) for i in range(n + 1)]
    elif kind == "sinc":
        pts = [(x0 + i * w / n, y0 - h * (math.sin(math.pi * (i / n - 0.5) * 7) / (math.pi * (i / n - 0.5) * 7) if i != n / 2 else 1.0) ** 2) for i in range(n + 1)]
    if pts:
        s.polyline(pts, col, 1.6)
    if kind == "comb":
        for i in range(7):
            x = x0 + 5 + i * 5.5
            hh = h * (0.95 if i < 4 else 0.95 - (i - 3) * 0.25)
            s.line(x, y0, x, y0 - hh, col, 1.6)
    if kind in ("continuum", "lines"):
        for x, hh in ((x0 + w * 0.42, h * 0.95), (x0 + w * 0.48, h * 0.6)):
            s.line(x, y0, x, y0 - hh, "orange", 1.6)


def _grating(s, cx, cy, col):
    """A line-narrowing module: beam expander prisms + grating; the beam leaves narrower."""
    for i in range(3):
        x = cx - 20 + i * 9
        s.polyline([(x, cy + 12), (x + 7, cy - 12), (x + 7, cy + 12)], "blue", 1.1, fill=s.tint("blue", 0.15), close=True)
    s.rect(cx + 10, cy - 14, 6, 28, fill=s.tint("ink", 0.4), stroke="none")
    for i in range(7):
        s.line(cx + 10, cy - 12 + i * 4, cx + 16, cy - 12 + i * 4, "surface", 0.8)
    _beam(s, cx + 22, cy, cx + 24, cy, col, 1.0, arrow=None)


def _laser(s, cx, cy, col, label="laser"):
    s.rect(cx - 22, cy - 9, 30, 18, fill=s.tint("ink", 0.12), stroke="ink2", sw=1.3, rx=2)
    s.text(cx - 7, cy + 4, label, size=T_SMALL, color="ink", weight=600)
    _beam(s, cx + 9, cy, cx + 24, cy, col, 2.4)


def _droplet(s, cx, cy, col):
    for y in (cy - 20, cy - 10, cy + 10, cy + 20):
        s.circle(cx, y, 2.6, fill=s.fill("metal"), stroke="ink2", sw=0.8)
    s.circle(cx, cy, 5, fill=s.fill("metal"), stroke=s.stroke("metal"), sw=1.0)
    _beam(s, cx - 24, cy, cx - 7, cy, col, 2.4)
    s.text(cx + 9, cy + 4, "Sn", size=T_SMALL, color="ink2", anchor="start", weight=600)


def _plasma(s, cx, cy, col, r=14):
    s.circle(cx, cy, r, fill=s.tint(col, 0.22), stroke="none")
    s.circle(cx, cy, r * 0.62, fill=s.tint(col, 0.55), stroke="none")
    s.circle(cx, cy, r * 0.3, fill=col, stroke="none")
    for a in range(0, 360, 45):
        rr = math.radians(a)
        s.line(cx + (r + 2) * math.cos(rr), cy + (r + 2) * math.sin(rr), cx + (r + 8) * math.cos(rr), cy + (r + 8) * math.sin(rr), col, 1.2, opacity=0.8)


def _collector(s, cx, cy, col, grazing=False):
    """Normal-incidence ellipsoidal collector (LPP) or nested grazing shells (DPP) focusing on IF."""
    if grazing:
        for k in (0, 1, 2):
            d = f"M{K.fmt(cx - 20)},{K.fmt(cy - 14 + k * 5)} C{K.fmt(cx - 4)},{K.fmt(cy - 12 + k * 4)} {K.fmt(cx + 8)},{K.fmt(cy - 4 + k * 2)} {K.fmt(cx + 20)},{K.fmt(cy)}"
            s.path(d, stroke="blue", sw=1.5)
            d2 = f"M{K.fmt(cx - 20)},{K.fmt(cy + 14 - k * 5)} C{K.fmt(cx - 4)},{K.fmt(cy + 12 - k * 4)} {K.fmt(cx + 8)},{K.fmt(cy + 4 - k * 2)} {K.fmt(cx + 20)},{K.fmt(cy)}"
            s.path(d2, stroke="blue", sw=1.5)
        s.rect(cx - 23, cy - 18, 4, 36, fill=s.tint("ink", 0.45), stroke="none")  # the debris trap foil stack
    else:
        d = f"M{K.fmt(cx - 20)},{K.fmt(cy - 20)} A26,26 0 0,0 {K.fmt(cx - 20)},{K.fmt(cy + 20)}"
        s.path(d, stroke="blue", sw=2.4)
        for dy in (-12, 0, 12):
            s.line(cx - 10, cy, cx - 18 + abs(dy) * 0.15, cy + dy, col, 1.0, opacity=0.6)
            s.line(cx - 18 + abs(dy) * 0.15, cy + dy, cx + 20, cy, col, 1.0, opacity=0.6)
    s.circle(cx + 20, cy, 2.4, fill="ink", stroke="none")


def _focus(s, cx, cy, col):
    for dy in (-16, -8, 8, 16):
        s.line(cx - 22, cy + dy, cx, cy, col, 1.0, opacity=0.6)
        s.line(cx, cy, cx + 22, cy + dy * 0.8, col, 1.0, opacity=0.6)
    s.circle(cx, cy, 3, fill="ink", stroke="none")
    s.text(cx, cy - 20, "IF", size=T_SMALL, color="ink2", weight=600, halo=True)


def _capacitor(s, cx, cy, col):
    s.rect(cx - 20, cy - 16, 8, 32, fill=s.tint("ink", 0.5), stroke="none")
    s.rect(cx - 6, cy - 16, 8, 32, fill=s.tint("ink", 0.5), stroke="none")
    s.polyline([(cx + 8, cy - 18), (cx + 14, cy - 4), (cx + 9, cy - 2), (cx + 18, cy + 16)], col, 1.8)
    s.text(cx - 9, cy + 30, "+ −", size=T_SMALL, color="ink2")


def _pinch(s, cx, cy, col):
    s.polyline([(cx - 16, cy - 20), (cx - 5, cy - 8), (cx - 5, cy + 8), (cx - 16, cy + 20)], "ink2", 1.3)
    s.polyline([(cx + 16, cy - 20), (cx + 5, cy - 8), (cx + 5, cy + 8), (cx + 16, cy + 20)], "ink2", 1.3)
    s.rect(cx - 5, cy - 8, 10, 16, fill=s.tint(col, 0.5), stroke="none", rx=3)
    s.line(cx - 22, cy - 8, cx - 8, cy - 2, col, 1.2, arrow="end")
    s.line(cx + 22, cy - 8, cx + 8, cy - 2, col, 1.2, arrow="end")
    s.text(cx, cy + 32, "J × B", size=T_SMALL, color="ink2")


def _column(s, cx, cy, col, label=None):
    s.rect(cx - 6, cy - 22, 12, 44, fill=s.tint(col, 0.45), stroke=col, sw=1.0, rx=6)
    s.rect(cx - 2.5, cy - 18, 5, 36, fill=col, stroke="none", rx=2.5)
    if label:
        s.text(cx + 11, cy + 4, label, size=T_SMALL, color="ink2", anchor="start")


def _etendue(s, cx, cy, col):
    s.rect(cx - 3, cy - 20, 6, 40, fill=s.tint("ink", 0.45), stroke="none")
    s.rect(cx - 3, cy - 7, 6, 14, fill="surface", stroke="none")
    for dy in (-16, -9, -2, 5, 12):
        inside = abs(dy + 2) <= 7
        s.line(cx - 22, cy + dy, cx - 3, cy + dy, col, 1.1, opacity=0.9 if inside else 0.4)
        if inside:
            s.line(cx + 3, cy + dy, cx + 22, cy + dy, col, 1.1)


def _ring(s, cx, cy, col):
    s.circle(cx, cy, 18, fill="none", stroke="ink2", sw=1.6)
    for a in (20, 140, 260):
        rr = math.radians(a)
        s.circle(cx + 18 * math.cos(rr), cy + 18 * math.sin(rr), 2.6, fill=col, stroke="none")
    s.text(cx, cy + 4, "e⁻", size=T_SMALL, color="ink2", weight=600)
    s.line(cx + 18, cy, cx + 24, cy, col, 1.4, arrow="end")


def _bend(s, cx, cy, col):
    d = f"M{K.fmt(cx - 22)},{K.fmt(cy + 14)} Q{K.fmt(cx - 6)},{K.fmt(cy - 16)} {K.fmt(cx + 12)},{K.fmt(cy - 18)}"
    s.path(d, stroke="ink2", sw=1.8)
    s.rect(cx - 16, cy - 4, 14, 8, fill=s.tint("violet", 0.5), stroke="none", rx=1)
    s.text(cx - 9, cy + 2.5, "B", size=T_SMALL, color="ink", weight=600)
    for k, dy in enumerate((0, -4, 4)):
        s.line(cx - 6, cy - 13, cx + 24, cy - 13 + dy * 2, col, 1.1, opacity=0.9 - 0.25 * k)


def _undulator(s, cx, cy, col, short=False):
    n = 5
    w = 7
    x0 = cx - n * w / 2
    for i in range(n):
        up = i % 2 == 0
        s.rect(x0 + i * w, cy - 18, w - 1, 8, fill=s.tint("red" if up else "blue", 0.6), stroke="none")
        s.rect(x0 + i * w, cy + 10, w - 1, 8, fill=s.tint("blue" if up else "red", 0.6), stroke="none")
    pts = [(x0 + t * n * w / 40, cy + 5 * math.sin(2 * math.pi * t / 40 * n / 2)) for t in range(41)]
    s.polyline(pts, "ink", 1.5)
    s.line(x0 + n * w, cy, cx + 24, cy, col, 1.8, arrow="end")
    s.text(cx, cy + 32, "λ_u, K, N", size=T_SMALL, color="ink2")


def _cone(s, cx, cy, col, kind="sinc"):
    s.polyline([(cx - 22, cy), (cx + 6, cy - 12), (cx + 6, cy + 12)], col, 1.0, fill=s.tint(col, 0.18), close=True)
    _spectrum(s, cx + 10, cy, kind, col, w=28, h=20)


def _bunch(s, cx, cy, col):
    s.ellipse(cx, cy, 14, 7, fill=s.tint(col, 0.45), stroke=col, sw=1.2)
    s.text(cx, cy + 4, "e⁻", size=T_SMALL, color="ink", weight=600)
    s.line(cx + 15, cy, cx + 24, cy, col, 1.6, arrow="end")


def _jet(s, cx, cy, col, plume=True):
    s.polyline([(cx - 8, cy + 22), (cx - 8, cy + 8), (cx - 3, cy + 2), (cx + 3, cy + 2), (cx + 8, cy + 8), (cx + 8, cy + 22)], "ink2", 1.4,
               fill=s.tint("ink", 0.2), close=True)
    if plume:
        s.ellipse(cx, cy - 8, 9, 10, fill=s.tint("aqua", 0.3), stroke="none")
        s.ellipse(cx, cy - 8, 4, 5, fill=s.tint("aqua", 0.6), stroke="none")
    _beam(s, cx - 24, cy - 8, cx + 24, cy - 8, col, 2.2)


def _three_step(s, cx, cy, col):
    s.circle(cx - 12, cy + 6, 4, fill=s.tint("ink", 0.6), stroke="none")  # the ion
    s.circle(cx - 12, cy + 6, 11, fill="none", stroke="axis", sw=1.0, dash="2 2")
    d = f"M{K.fmt(cx - 4)},{K.fmt(cy + 2)} C{K.fmt(cx + 12)},{K.fmt(cy - 24)} {K.fmt(cx + 22)},{K.fmt(cy - 6)} {K.fmt(cx - 2)},{K.fmt(cy + 8)}"
    s.path(d, stroke=col, sw=1.4)
    s.circle(cx - 4, cy + 2, 2.2, fill=col, stroke="none")
    s.line(cx - 12, cy + 6, cx - 24, cy - 14, "violet", 1.6, arrow="end")
    s.text(cx + 10, cy + 24, "1·2·3", size=T_SMALL, color="ink2")


def _levels(s, cx, cy, col):
    s.line(cx - 16, cy - 14, cx + 4, cy - 14, "ink", 1.8)
    s.line(cx - 16, cy + 10, cx + 4, cy + 10, "ink", 1.8)
    s.line(cx - 16, cy + 20, cx + 4, cy + 20, "ink", 1.2)
    s.text(cx + 8, cy - 10, "J=0", size=T_SMALL, color="ink2", anchor="start")
    s.text(cx + 8, cy + 14, "J=1", size=T_SMALL, color="ink2", anchor="start")
    s.line(cx - 10, cy + 19, cx - 10, cy - 12, "ink2", 1.1, arrow="end")
    s.line(cx - 2, cy - 12, cx - 2, cy + 8, col, 1.8, arrow="end")
    for k in range(4):
        s.circle(cx - 14 + k * 4.5, cy - 17, 1.6, fill="ink", stroke="none")


def _ase(s, cx, cy, col):
    s.rect(cx - 22, cy - 6, 44, 12, fill=s.tint(col, 0.35), stroke=col, sw=1.0, rx=6)
    s.polyline([(cx - 18, cy), (cx - 6, cy - 1), (cx + 6, cy - 3), (cx + 16, cy - 7)], col, 2.2)
    s.polyline([(cx - 18, cy), (cx - 6, cy + 1), (cx + 6, cy + 3), (cx + 16, cy + 7)], col, 2.2)
    s.line(cx + 16, cy - 7, cx + 24, cy - 9, col, 2.2, arrow="end")
    s.line(cx + 16, cy + 7, cx + 24, cy + 9, col, 2.2, arrow="end")


def _ebeam_grating(s, cx, cy, col):
    pts = [(cx - 22 + i * 4, cy + 6 + (0 if i % 2 else 5)) for i in range(12)]
    s.polyline(pts, "ink2", 1.4)
    s.rect(cx - 22, cy + 11, 44, 8, fill=s.tint("ink", 0.2), stroke="none")
    s.line(cx - 24, cy - 2, cx + 24, cy - 2, "ink", 1.8, arrow="end")
    s.line(cx - 22, cy + 3, cx + 22, cy + 3, "axis", 0.8, dash="2 2")
    s.text(cx - 2, cy + 32, "e⁻ ∥ grating", size=T_SMALL, color="ink2")
    s.line(cx - 4, cy - 2, cx + 14, cy - 22, col, 1.6, arrow="end")


def _cathode_anode(s, cx, cy, col):
    s.polyline([(cx - 22, cy - 10), (cx - 16, cy - 2), (cx - 22, cy + 6)], "orange", 1.6)
    for dy in (-6, 0, 6):
        s.line(cx - 12, cy + dy * 0.6, cx + 4, cy + dy * 0.4, "ink2", 1.0, arrow="end")
    s.polyline([(cx + 6, cy - 20), (cx + 22, cy - 20), (cx + 22, cy + 20), (cx + 6, cy + 4)], "ink", 1.2,
               fill=s.tint("ink", 0.35), close=True)
    s.line(cx + 10, cy - 2, cx + 2, cy - 24, col, 1.6, arrow="end")
    s.text(cx + 14, cy + 32, "W · Mo · Cu · Rh", size=T_SMALL, color="ink2")


def _window(s, cx, cy, col):
    s.rect(cx - 4, cy - 20, 8, 40, fill=s.tint("aqua", 0.3), stroke="aqua", sw=1.2, rx=1)
    for dy, through in ((-12, True), (-4, True), (4, False), (12, False)):
        s.line(cx - 22, cy + dy, cx - 4, cy + dy, col, 1.1 if through else 0.9, opacity=1.0 if through else 0.45)
        if through:
            s.line(cx + 4, cy + dy, cx + 22, cy + dy, col, 1.1, arrow="end")
    s.text(cx, cy + 32, "Be", size=T_SMALL, color="ink2", weight=600)


def _isotropic(s, cx, cy, col):
    s.circle(cx - 8, cy, 3, fill="ink", stroke="none")
    for a in range(-60, 61, 30):
        rr = math.radians(a)
        s.line(cx - 8 + 6 * math.cos(rr), cy + 6 * math.sin(rr), cx - 8 + 28 * math.cos(rr), cy + 28 * math.sin(rr), col, 1.1, opacity=0.7)
    s.text(cx + 2, cy + 34, "1/(4πd²)", size=T_SMALL, color="ink2")


def _bubble(s, cx, cy, col):
    s.ellipse(cx + 2, cy, 18, 11, fill=s.tint("aqua", 0.25), stroke="aqua", sw=1.3)
    for k in range(3):
        s.circle(cx - 8 + k * 3, cy + 5 - k * 2, 1.8, fill="ink", stroke="none")
    _beam(s, cx - 24, cy, cx - 2, cy, col, 2.2, arrow=None)
    s.rect(cx - 6, cy - 7, 5, 14, fill=col, stroke="none", rx=2)
    s.text(cx, cy + 28, "ion cavity", size=T_SMALL, color="ink2")


def _wiggle(s, cx, cy, col):
    pts = [(cx - 22 + t * 1.1, cy + 7 * math.sin(t * 0.55)) for t in range(41)]
    s.polyline(pts, "ink", 1.6)
    s.polyline([(cx + 2, cy), (cx + 24, cy - 8), (cx + 24, cy + 8)], col, 1.0, fill=s.tint(col, 0.2), close=True)
    s.text(cx, cy + 28, "r_β ≈ 1 µm", size=T_SMALL, color="ink2")


def _spdc(s, cx, cy, col):
    s.rect(cx - 10, cy - 14, 20, 28, fill=s.tint("blue", 0.2), stroke="blue", sw=1.3, rx=2)
    s.text(cx, cy + 4, "χ⁽²⁾", size=T_SMALL, color="ink", weight=600)
    _beam(s, cx - 24, cy, cx - 10, cy, "violet", 2.2, arrow=None)
    _beam(s, cx + 10, cy - 3, cx + 24, cy - 10, col, 1.8)
    _beam(s, cx + 10, cy + 3, cx + 24, cy + 10, col, 1.8)


def _splitter(s, cx, cy, col):
    s.line(cx - 10, cy - 10, cx + 10, cy + 10, "ink", 1.8)
    _beam(s, cx - 24, cy, cx - 2, cy, col, 1.8, arrow=None)
    _beam(s, cx, cy, cx + 22, cy, col, 1.8)
    _beam(s, cx, cy, cx, cy - 20, col, 1.8)
    s.text(cx + 2, cy + 30, "|N,0⟩+|0,N⟩", size=T_SMALL, color="ink2")


def _interference(s, cx, cy, col):
    _beam(s, cx - 22, cy - 18, cx, cy + 8, col, 1.8, arrow=None)
    _beam(s, cx + 22, cy - 18, cx, cy + 8, col, 1.8, arrow=None)
    K.wafer(s, cx - 20, cy + 12, 40, 6, label=None, resist=6)
    for k in range(7):
        s.line(cx - 18 + k * 6, cy + 6, cx - 18 + k * 6, cy + 12, col, 1.2, opacity=0.9)


def _resist_hypo(s, cx, cy, col):
    s.rect(cx - 20, cy - 10, 40, 20, fill="none", stroke="ink2", sw=1.2, dash="4 3", rx=2)
    s.text(cx, cy + 4, "I^N resist?", size=T_SMALL, color="ink2", italic=True)
    s.text(cx, cy + 30, "does not exist", size=T_SMALL, color="ink2", italic=True)


def _shadow_print(s, cx, cy, col):
    """The lab white-beam end point: mask + gap + thick resist, no projection pupil."""
    for dx in (-14, -4, 6, 16):
        s.line(cx + dx, cy - 26, cx + dx, cy - 6, col, 1.1, opacity=0.8)
    s.rect(cx - 22, cy - 6, 44, 3, fill=s.tint("ink", 0.3), stroke="none")
    for ax in (-22, -4, 12):
        s.rect(cx + ax, cy - 10, 8, 7, fill=s.fill("absorber"), stroke="none")
    s.rect(cx - 22, cy + 2, 44, 18, "resist", sw=0.8)
    s.rect(cx - 22, cy + 20, 44, 5, "substrate", sw=0.8)
    for dx in (-14, 6):
        s.line(cx + dx, cy + 2, cx + dx, cy + 16, col, 1.1, opacity=0.5)


# ---------------------------------------------------------------------------
# The lane machinery
# ---------------------------------------------------------------------------


class Station:
    def __init__(self, title: str, lines: tuple[str, ...], icon, *, color: str | None = None):
        self.title, self.lines, self.icon, self.color = title, lines, icon, color


class Lane:
    def __init__(self, key: str, eyebrow: str, stations: list[Station], end: str, *, wavelength_nm: float,
                 end_lines: tuple[str, str] | None = None):
        self.key, self.eyebrow, self.stations, self.end = key, eyebrow, stations, end
        self.wavelength_nm = wavelength_nm
        self.end_lines = end_lines


def _beam_color(s: K.Svg, wavelength_nm: float) -> str:
    return tokens.wavelength_color(wavelength_nm, muted=s.t["ink2"])


def _draw_lane(s: K.Svg, lane: Lane, top: float, lit: bool) -> None:
    col = _beam_color(s, lane.wavelength_nm)
    yc = top + 74                      # icon row centre
    y_title = top + 120
    n = len(lane.stations)
    w = _station_width(n)
    with s.group("lane", opacity=None if lit else DIM):
        s.rect(X0 - 8, top + 4, W - X0 - 4, LANE_H - 10, fill=s.tint("ink", 0.03), stroke="none", rx=6)
        s.text(X0, top + 24, lane.eyebrow, size=T_LABEL, color="ink", weight=600, anchor="start")
        for i, st in enumerate(lane.stations):
            x = X0 + i * (w + GAP)
            cx = x + w / 2
            st.icon(s, cx, yc, st.color or col)
            s.text(cx, y_title, st.title, size=T_LABEL, color="ink", weight=600, halo=True)
            for k, line in enumerate(st.lines):
                s.text(cx, y_title + 15 + k * 14, line, size=T_SMALL, color="ink2", halo=True)
            if i < n - 1:
                s.line(x + w + 1, yc, x + w + GAP - 1, yc, "ink2", 1.3, arrow="end", opacity=0.8)
        # the end of the chain: a D5 pupil, or the shadow-printing stack where there is no pupil
        x_last = X0 + (n - 1) * (w + GAP) + w
        s.line(x_last + 1, yc, PUPIL_X - PUPIL_R - 4, yc, col, 2.0, arrow="end")
        lines = lane.end_lines or _pupil_label(lane.key)
        if lane.end == "pupil":
            factory, _, shape, sigma, zeta = PUPILS[lane.key]
            if shape == "conventional":
                K.pupil(s, PUPIL_X, yc, PUPIL_R, fill="conventional", sigma=sigma)
            else:
                K.pupil(s, PUPIL_X, yc, PUPIL_R, fill=None, coherent=max(sigma, 0.08))
            s.text(PUPIL_X, top + 24, "pupil", size=T_SMALL, color="ink2")
        else:
            _shadow_print(s, PUPIL_X, yc, col)
            s.text(PUPIL_X, top + 24, "no pupil", size=T_SMALL, color="ink2")
        s.text(PUPIL_X, y_title, lines[0], size=T_SMALL, color="ink", weight=600, halo=True)
        s.text(PUPIL_X, y_title + 15, lines[1], size=T_SMALL, color="ink2", halo=True)


def _station_width(n: int) -> float:
    end_w = 2 * PUPIL_R + 26
    avail = (PUPIL_X - end_w / 2 - GAP) - X0
    return (avail - (n - 1) * GAP) / n


def _height(lanes: list[Lane], notes) -> float:
    return HEADER + len(lanes) * LANE_H + (NOTES_H if notes else 0) + FOOTER


def _draw_notes(s: K.Svg, top: float, notes: list[tuple[str, str, str]]) -> None:
    """A row of short note boxes (title · two lines) under the lanes."""
    n = len(notes)
    w = (W - 2 * X0 - (n - 1) * GAP) / n
    for i, (title, l1, l2) in enumerate(notes):
        x = X0 + i * (w + GAP)
        s.rect(x, top + 6, w, NOTES_H - 18, fill="none", stroke="axis", sw=1.0, rx=5)
        s.text(x + 12, top + 30, title, size=T_LABEL, color="ink", weight=600, anchor="start")
        s.text(x + 12, top + 48, l1, size=T_SMALL, color="ink2", anchor="start")
        s.text(x + 12, top + 63, l2, size=T_SMALL, color="ink2", anchor="start")


def _cluster_svg(theme: str, *, name: str, title: str, alt: str, lanes: list[Lane], highlight: str | None,
                 families: dict[str, tuple[str, ...]], footer: str, header_note: str, notes=None) -> str:
    h = _height(lanes, notes)
    s = K.Svg(W, h, theme, title, alt, generator=GENERATOR)
    lit_keys = None
    if highlight is not None:
        if highlight not in families:
            raise KeyError(f"{name}: unknown family {highlight!r} (one of {sorted(families)})")
        lit_keys = set(families[highlight])
    s.eyebrow(X0, 24, title, size=T_LABEL, color="ink", anchor="start")
    s.text(W - 14, 24, header_note, size=T_SMALL, color="ink2", anchor="end")
    for i, lane in enumerate(lanes):
        lit = lit_keys is None or lane.key in lit_keys
        _draw_lane(s, lane, HEADER + i * LANE_H, lit)
    if notes:
        _draw_notes(s, HEADER + len(lanes) * LANE_H, notes)
    s.text(W / 2, h - 27, footer, size=T_SMALL, color="ink2", italic=True)
    s.text(W / 2, h - 11, "Schematic, not to scale.", size=T_SMALL, color="ink2", italic=True)
    return s.render()


# ---------------------------------------------------------------------------
# The six clusters
# ---------------------------------------------------------------------------


def lanes_excimer() -> list[Lane]:
    return [
        Lane("excimer", "Excimer laser · KrF 248.3 · ArF 193.4 · F₂ 157.63 nm (Ar₂ 126 nm: hypothetical)", [
            Station("Pulsed discharge", ("Kr / Ar / F₂ + buffer,", "~ns pulses, ~1 bar"), _cavity),
            Station("Excimer forms", ("KrF* · ArF* bound–free,", "F₂ D′→A′ bound–bound"), lambda s, x, y, c: _spectrum(s, x, y, "broad", c)),
            Station("Line narrowing", ("prisms + grating:", "free-running → pm"), _grating),
            Station("E95 bandwidth", ("Gaussian / Lorentzian /", "sinc² at equal FWHM"), lambda s, x, y, c: _spectrum(s, x, y, "narrow", c)),
            Station("Silica · CaF₂ lens", ("few-material refractive", "column, N₂ purge"), lambda s, x, y, c: _lens(s, x, y, c)),
        ], "pupil", wavelength_nm=193.4),
        Lane("lamp", "Mercury arc lamp · g 435.83 · h 404.66 · i 365.02 nm", [
            Station("Hg arc lamp", ("high-pressure plasma,", "CW"), _lamp),
            Station("Broadband", ("many Hg lines +", "continuum"), lambda s, x, y, c: _spectrum(s, x, y, "lines", c)),
            Station("Interference filter", ("isolates one line,", "few-nm passband"), _filter),
            Station("One line, 3 nm FWHM", ("Gaussian, 1 spectral", "sample in the model"), lambda s, x, y, c: _spectrum(s, x, y, "narrow", c)),
            Station("Multi-glass lens", ("colour-corrected", "refractive column"), lambda s, x, y, c: _lens(s, x, y, c)),
        ], "pupil", wavelength_nm=365.0),
    ]


def _lens(s, cx, cy, col):
    d = f"M{K.fmt(cx - 4)},{K.fmt(cy - 20)} Q{K.fmt(cx + 12)},{K.fmt(cy)} {K.fmt(cx - 4)},{K.fmt(cy + 20)} Q{K.fmt(cx - 20)},{K.fmt(cy)} {K.fmt(cx - 4)},{K.fmt(cy - 20)} Z"
    s.path(d, fill=s.fill("glass"), stroke=s.stroke("glass"), sw=1.2)
    for dy in (-10, 0, 10):
        s.line(cx - 24, cy + dy, cx - 6, cy + dy, col, 1.0, opacity=0.7)
        s.line(cx - 2, cy + dy, cx + 24, cy + dy * 0.2, col, 1.0, opacity=0.7)


def lanes_plasma() -> list[Lane]:
    return [
        Lane("lpp", "Laser-produced plasma (LPP) · Sn 13.5 nm · Gd 6.7 / Tb 6.5 nm beyond-EUV", [
            Station("Drive laser", ("CO₂ ~10 µm (HVM);", "1–2 µm solid state"), lambda s, x, y, c: _laser(s, x, y, c, "CO₂")),
            Station("Sn droplet", ("25–30 µm, 70 m/s,", "pre-pulse, then main"), _droplet),
            Station("Dense plasma", ("Sn⁸⁺–Sn¹⁴⁺ UTA;", "Gd / Tb for 6.x nm"), lambda s, x, y, c: _plasma(s, x, y, c)),
            Station("In-band into 2π sr", ("P₂π = P_drive · CE", "(2 % band)"), lambda s, x, y, c: _spectrum(s, x, y, "narrow", c)),
            Station("Collector", ("Mo/Si ellipsoid,", "η₂π→IF lumped"), lambda s, x, y, c: _collector(s, x, y, c)),
            Station("Intermediate focus", ("average_power_w()", "250 W NXE:3400B"), _focus),
        ], "pupil", wavelength_nm=13.5),
        Lane("dpp", "Discharge-produced plasma (DPP / LDP) · Xe or Sn · 13.5 nm", [
            Station("Capacitor bank", ("P_elec: tens of kW,", "electrode heat limit"), _capacitor),
            Station("Discharge · pinch", ("Xe gas or Sn vapour", "(laser-assisted: LDP)"), _pinch),
            Station("Plasma column", ("d 0.4–1, l 1–3 mm;", "Xe¹⁰⁺ or Sn ions"), lambda s, x, y, c: _column(s, x, y, c)),
            Station("2π at the source", ("P₂π = CE · P_elec", "(at the source)"), lambda s, x, y, c: _spectrum(s, x, y, "narrow", c)),
            Station("Collector + trap", ("grazing shells, Ω_c,", "throughput η_c"), lambda s, x, y, c: _collector(s, x, y, c, grazing=True)),
            Station("Étendue cut → IF", ("étendue f_G ≤ 1;", "P_IF (scanner plane)"), _etendue),
        ], "pupil", wavelength_nm=13.5),
    ]


def lanes_accelerator() -> list[Lane]:
    return [
        Lane("synchrotron", "Synchrotron · bending magnet (LIGA white beam) or undulator (13.5 nm)", [
            Station("Storage ring", ("E, γ, current I,", "emittances ε_x, ε_y"), _ring),
            Station("Bending magnet", ("field B: broad S(E/E_c),", "white beam → LIGA"), _bend),
            Station("Undulator", ("λ_n = λ_u(1+K²/2)/2nγ²,", "odd harmonics"), _undulator),
            Station("sinc² line", ("zeros at ±λ/(nN);", "cone ∝ I · Q_n(K)"), lambda s, x, y, c: _cone(s, x, y, c, "sinc")),
            Station("Coherent fraction", ("ζ_x ζ_y from ε vs λ/4π", "→ CoherentGaussian"), _zeta),
        ], "pupil", wavelength_nm=13.5),
        Lane("lpa-fel", "LPA-FEL · LPA + undulator · lased at 420 nm (BELLA) and 27 nm (SIOM), ≤ 1 Hz · 25 nm / 1 kHz preset: projection", [
            Station("Drive laser", ("~100 TW, ~30 fs,", "800 nm Ti:sapphire"), lambda s, x, y, c: _laser(s, x, y, c, "TW")),
            Station("Plasma cell", ("wakefield ~100 GV/m", "n_e ≈ 10¹⁸ cm⁻³"), lambda s, x, y, c: _jet(s, x, y, c)),
            Station("Electron bunch", ("100–500 MeV, kA peak,", "σ_δ ≈ 1 %"), _bunch),
            Station("Undulator", ("λ_u ≈ cm, K tunable;", "gain only if σ_δ < ρ"), _undulator),
            Station("SASE radiation", ("bandwidth 2ρλ;", "or no gain (25 nm)"), lambda s, x, y, c: _cone(s, x, y, c, "narrow")),
        ], "pupil", wavelength_nm=25.0, end_lines=("top-hat σ 0.7", "λ a set-point")),
        Lane("smith-purcell", "Smith–Purcell · electron over a grating · 13.5 nm preset (demonstrated only to ~230 nm)", [
            Station("Electron beam", ("SEM / TEM class,", "1–300 keV, nA–µA"), _bunch),
            Station("Evanescent field", ("reach h_int = βγλ/4π,", "sub-nm at EUV"), _ebeam_grating),
            Station("Periodic structure", ("period a, N periods:", "grating · vdW planes"), lambda s, x, y, c: _comb_structure(s, x, y, c)),
            Station("In-phase emission", ("λ = (a/m)(1/β − cos θ)", "angle-tuned"), lambda s, x, y, c: _cone(s, x, y, c, "narrow")),
            Station("Line width", ("Δλ/λ ≥ 1/(mN);", "power: estimate only"), lambda s, x, y, c: _spectrum(s, x, y, "narrow", c)),
        ], "pupil", wavelength_nm=13.5),
    ]


def _zeta(s, cx, cy, col):
    s.circle(cx - 6, cy, 18, fill="none", stroke="axis", sw=1.2, dash="3 2")
    s.circle(cx - 6, cy, 7, fill=s.tint("aqua", 0.6), stroke="aqua", sw=1.0)
    s.text(cx - 6, cy + 32, "ζ = ζ_x ζ_y", size=T_SMALL, color="ink2")
    s.line(cx + 14, cy, cx + 24, cy, col, 1.6, arrow="end")


def _comb_structure(s, cx, cy, col):
    for i in range(8):
        s.rect(cx - 22 + i * 5.6, cy, 3, 12, fill=s.tint("ink", 0.5), stroke="none")
    s.line(cx - 24, cy - 8, cx + 24, cy - 8, "ink", 1.6, arrow="end")
    for i in range(4):
        x = cx - 16 + i * 10
        s.line(x, cy - 8, x + 8, cy - 26, col, 1.1, opacity=0.8)
    s.text(cx, cy + 30, "a · N", size=T_SMALL, color="ink2")


def lanes_tabletop() -> list[Lane]:
    return [
        Lane("hhg", "High-harmonic generation (HHG) · Ne 13.56 nm · Ar 29.6 nm", [
            Station("fs driver", ("800 nm, ~10¹⁴ W/cm²,", "Keldysh γ_K < 1"), lambda s, x, y, c: _laser(s, x, y, "red", "fs")),
            Station("Gas jet", ("Ne or Ar;", "tunnel ionization"), lambda s, x, y, c: _jet(s, x, y, "red")),
            Station("Three-step", ("ionize · accelerate ·", "recombine: I_p + KE"), _three_step),
            Station("Odd-harmonic comb", ("spacing 2ħω, cutoff", "E_max = I_p + 3.17 U_p"), lambda s, x, y, c: _spectrum(s, x, y, "comb", c)),
            Station("Phase matching", ("ionization < η_cr;", "absorption-limited η_q"), lambda s, x, y, c: _passband(s, x, y, c)),
        ], "pupil", wavelength_nm=13.5),
        Lane("sxrl", "Plasma soft-X-ray laser · Ne-like Ar 46.9 nm · Ni-like Ag 13.9 / Cd 13.2 / Mo 18.9 nm", [
            Station("Pump", ("capillary discharge (Ar)", "or ps laser (Ag, Cd, Mo)"), lambda s, x, y, c: _laser(s, x, y, "red", "ps")),
            Station("Plasma column", ("Ne-like Ar⁸⁺ or", "Ni-like Ag¹⁹⁺, Cd, Mo"), lambda s, x, y, c: _column(s, x, y, c)),
            Station("Collisional pumping", ("e⁻ excite ground → J=0;", "J=1 decays radiatively"), _levels),
            Station("ASE along the column", ("amplified spontaneous", "emission, single pass"), _ase),
            Station("Fixed atomic line", ("narrow line, partially", "coherent beam"), lambda s, x, y, c: _spectrum(s, x, y, "narrow", c)),
        ], "pupil", wavelength_nm=13.9),
    ]


def _passband(s, cx, cy, col):
    _spectrum(s, cx, cy, "comb", s.t["muted"], w=44, h=26)
    s.rect(cx - 2, cy - 16, 14, 32, fill=s.tint(col, 0.22), stroke=col, sw=1.0, rx=2)
    s.text(cx, cy + 30, "filter + Mo/Si", size=T_SMALL, color="ink2")


def lanes_labxray() -> list[Lane]:
    return [
        Lane("xray-tube", "X-ray tube · W / Mo / Cu / Rh anode · bremsstrahlung + K/L lines (LIGA)", [
            Station("Cathode", ("thermionic electrons,", "tube current I"), lambda s, x, y, c: _cathode(s, x, y, c)),
            Station("Acceleration", ("V = kVp: each e⁻", "arrives with E₀ = eV"), lambda s, x, y, c: _accel(s, x, y, c)),
            Station("Thick anode", ("~99 % of eVI → heat;", "η = 1.1×10⁻⁹ Z V"), _cathode_anode),
            Station("Continuum + lines", ("Kramers 0 < E ≤ E₀;", "K / L if E₀ > edge"), lambda s, x, y, c: _spectrum(s, x, y, "continuum", c)),
            Station("Be window", ("exp(−μ_Be t) removes", "soft photons"), _window),
            Station("Isotropic", ("flux ∝ 1/(4πd²)", "at the mask"), _isotropic),
        ], "shadow", wavelength_nm=0.15, end_lines=("shadow print", "mask → PMMA")),
        Lane("betatron", "LWFA betatron · fs keV burst, ~10 Hz · imaging, not lithography (0.5 nm preset)", [
            Station("Drive laser", ("~100 TW, fs,", "~10 Hz"), lambda s, x, y, c: _laser(s, x, y, "red", "TW")),
            Station("Plasma bubble", ("n_e ≈ 10¹⁸ cm⁻³,", "plasma frequency ω_p"), _bubble),
            Station("Trapped electrons", ("accelerated to", "γ ≈ 400 (~200 MeV)"), _bunch),
            Station("Betatron oscillation", ("ω_β = ω_p/√(2γ),", "amplitude r_β"), _wiggle),
            Station("Wiggler regime", ("K = γ k_β r_β ≫ 1;", "cone θ ≈ K/γ"), lambda s, x, y, c: _cone(s, x, y, c, "bm")),
            Station("keV burst", ("S(E/E_c) spectrum,", "µW average, ~10 Hz"), lambda s, x, y, c: _spectrum(s, x, y, "bm", c)),
        ], "shadow", wavelength_nm=0.5, end_lines=("shadow print", "no pupil")),
    ]


def _cathode(s, cx, cy, col):
    s.polyline([(cx - 10, cy - 14), (cx - 4, cy - 6), (cx - 10, cy + 2), (cx - 4, cy + 10)], "orange", 1.8)
    s.circle(cx - 7, cy - 2, 14, fill="none", stroke="axis", sw=1.0, dash="2 2")
    for dy in (-8, 0, 8):
        s.line(cx + 2, cy + dy, cx + 22, cy + dy, "ink2", 1.0, arrow="end", opacity=0.8)
    s.text(cx - 7, cy + 30, "heated filament", size=T_SMALL, color="ink2")


def _accel(s, cx, cy, col):
    s.rect(cx - 22, cy - 16, 6, 32, fill=s.tint("ink", 0.45), stroke="none")
    s.rect(cx + 16, cy - 16, 6, 32, fill=s.tint("ink", 0.45), stroke="none")
    s.text(cx - 19, cy + 30, "−", size=T_SMALL, color="ink2")
    s.text(cx + 19, cy + 30, "+", size=T_SMALL, color="ink2")
    s.line(cx - 14, cy, cx + 12, cy, "ink", 2.0, arrow="end")
    s.text(cx, cy - 8, "eV", size=T_SMALL, color="ink2", halo=True)


def lanes_quantum() -> list[Lane]:
    return [
        Lane("noon", "Entangled-photon NOON source · degenerate pairs at 157.63 nm · 10⁶–10⁷ pairs/s", [
            Station("Pump laser", ("λ_p; pump photon →", "two at 2λ_p"), lambda s, x, y, c: _laser(s, x, y, "violet", "pump")),
            Station("SPDC crystal", ("χ⁽²⁾ down-conversion,", "~10⁶–10⁷ pairs/s"), _spdc),
            Station("NOON preparation", ("interferometer and", "beam splitters"), _splitter),
            Station("Two-beam exposure", ("N-photon phase e^{iNφ}:", "fringe λ/(2N sin θ)"), _interference),
            Station("N-photon resist", ("hypothetical; ETPA", "cross-section disputed"), _resist_hypo),
        ], "pupil", wavelength_nm=157.63, end_lines=("coherent σ 0.05", "from ζ 1")),
    ]


CLUSTERS = {
    "excimer": dict(
        title="Excimer & lamp lines",
        lanes=lanes_excimer,
        families={"duv-heritage": ("excimer", "lamp"), "vuv-excimer": ("excimer",)},
        pages=("sources/duv-heritage.md", "sources/vuv-excimer.md"),
        footer="Hg lines and excimer lines reach the same refractive column; the pupil fill is a user choice (engine default: top-hat σ 0.7).",
        alt=("Two lanes to a top-hat pupil of sigma 0.7: a pulsed Kr/Ar/F2 discharge forms the excimer, a prism-and-grating "
             "module narrows the line to the E95 bandwidth and a silica or CaF2 lens follows; an Hg arc lamp and an "
             "interference filter isolate the g, h or i line."),
        caption=("<strong>Mechanism.</strong> The excimer-laser lane: a pulsed discharge in Kr, Ar or F₂ with a buffer gas "
                 "forms the excimer, whose free-running band a prism-and-grating module narrows to the picometre-class "
                 "E95 bandwidth the model parameterises before a silica or CaF₂ lens column. The mercury-lamp lane: a "
                 "high-pressure arc and an interference filter isolate the g, h or i line as one 3 nm Gaussian sample. "
                 "Both lanes end in the engine's default top-hat pupil (σ 0.7). " + TRAILER),
    ),
    "plasma": dict(
        title="Plasma EUV",
        lanes=lanes_plasma,
        families={"lpp": ("lpp",), "dpp": ("dpp",)},
        pages=("sources/lpp.md", "sources/dpp.md"),
        footer="Powers are stated at the intermediate focus; η₂π→IF is a lumped LPP factor, the DPP collector and étendue factors are assumptions.",
        alt=("Two lanes to a top-hat pupil of sigma 0.9: a CO2 laser hits 25 to 30 micrometre Sn droplets, the plasma emits "
             "into 2 pi sr and a collector focuses it at the intermediate focus; a capacitor bank pinches Xe or Sn, grazing "
             "shells collect it and an etendue cut sets the IF power."),
        caption=("<strong>Mechanism.</strong> LPP: a CO₂ (or 1–2 µm solid-state) drive laser hits 25–30 µm tin droplets "
                 "50 000 times a second, the Sn⁸⁺–Sn¹⁴⁺ plasma emits in band into 2π sr, and a normal-incidence Mo/Si "
                 "collector brings the light to the intermediate focus where the model states its power. DPP/LDP: a "
                 "capacitor bank pinches Xe gas or Sn vapour into a millimetre column, grazing shells with a debris trap "
                 "collect it, and the étendue cut f_G = min(1, G_ill/G_src) decides what reaches IF. Both default to the "
                 "spatially incoherent top-hat pupil (σ 0.9). " + TRAILER),
    ),
    "accelerator": dict(
        title="Accelerator EUV concepts",
        lanes=lanes_accelerator,
        families={"synchrotron": ("synchrotron",), "lpa-fel": ("lpa-fel",), "smith-purcell": ("smith-purcell",)},
        pages=("sources/synchrotron.md", "sources/lpa-fel.md", "sources/smith-purcell.md"),
        footer="XFEL, SSMB and inverse Compton keep their Future-page schematics; Smith–Purcell at EUV is undemonstrated and its power an estimate.",
        alt=("Three lanes: a storage ring feeds a bending magnet or an undulator whose sinc-squared line and coherent "
             "fraction set a coherent-core pupil; a 100 TW laser drives a wakefield bunch that lases in an undulator only "
             "if its energy spread is below rho; a keV electron skims a grating (Smith-Purcell)."),
        caption=("<strong>Mechanism.</strong> Synchrotron: the ring's electrons radiate a broad bending-magnet spectrum "
                 "(the LIGA white beam) or, in an undulator of N periods, the sinc² line at λ_n with first zeros at "
                 "±λ/(nN); the emittances against λ/4π give the coherent fraction that becomes a coherent-core pupil. "
                 "LPA-FEL: a 100 TW-class pulse drives a wakefield, the 100–500 MeV bunch enters an undulator and lases "
                 "(SASE) only while σ_δ &lt; ρ. Smith–Purcell: an electron skimming a grating of period a radiates in phase "
                 "at λ = (a/m)(1/β − cos θ). " + TRAILER),
    ),
    "tabletop": dict(
        title="Table-top coherent EUV",
        lanes=lanes_tabletop,
        families={"hhg": ("hhg",), "soft-xray-laser": ("sxrl",)},
        pages=("sources/hhg.md", "sources/soft-xray-laser.md"),
        footer="Both are laser-like sources mapped to a coherent-core pupil through the Gaussian–Schell heuristic σ = σ_core/√ζ.",
        alt=("Two lanes to coherent-core pupils: a femtosecond driver in a Ne or Ar jet drives the three-step process that "
             "emits an odd-harmonic comb up to Ip plus 3.17 Up; a capillary discharge or picosecond laser pumps a plasma "
             "column that lases by amplified spontaneous emission on a fixed atomic line."),
        caption=("<strong>Mechanism.</strong> HHG: a femtosecond driver at ~10¹⁴ W/cm² tunnel-ionises the gas near each "
                 "field crest, the freed electron gains up to 3.17 U_p and recombines, emitting one photon of I_p + KE "
                 "every half-cycle, so the spectrum is an odd-harmonic comb with spacing 2ħω and cutoff I_p + 3.17 U_p "
                 "that phase matching and absorption limit. Soft-X-ray laser: a capillary discharge or picosecond pump "
                 "makes a hot column of closed-shell-like ions, collisions invert a J = 0 level, and amplified "
                 "spontaneous emission along the column yields a fixed atomic line. Both reach the engine as coherent-core "
                 "pupils. " + TRAILER),
    ),
    "labxray": dict(
        title="Lab white-beam (LIGA)",
        lanes=lanes_labxray,
        families={"xray-tube": ("xray-tube",), "betatron": ("betatron",)},
        pages=("sources/xray-tube.md", "sources/betatron.md"),
        footer="Hard X-rays are shadow-printed through a mask into thick PMMA: no projection optic, no pupil, no TCC.",
        header_note="how the light is made → the mask (shadow printing)",
        alt=("Two lanes ending in shadow printing with no pupil: electrons accelerated to the kVp hit a thick W, Mo, Cu or "
             "Rh anode, giving the Kramers continuum and K and L lines that a Be window hardens; a 100 TW laser drives a "
             "plasma bubble whose electrons oscillate and emit a femtosecond keV burst."),
        caption=("<strong>Mechanism.</strong> X-ray tube: thermionic electrons accelerated through the kVp strike a thick "
                 "anode where about 99 % of the power becomes heat and the rest the Kramers continuum up to E₀ = eV plus "
                 "the K and L lines the anode's edges allow; the Be window removes the soft photons and the flux spreads "
                 "as 1/(4πd²) to the LIGA mask. Betatron: a 100 TW pulse blows out a plasma bubble, trapped electrons "
                 "reach γ ≈ 400 and oscillate in the ion cavity at ω_β = ω_p/√(2γ), radiating a synchrotron-like "
                 "femtosecond keV burst. Both are shadow-printed: no projection pupil. " + TRAILER),
    ),
    "quantum": dict(
        title="Quantum",
        lanes=lanes_quantum,
        families={"entangled-photon": ("noon",)},
        pages=("sources/entangled-photon.md",),
        footer="The pipeline images this source classically; the opt-in quantum module keeps classical Iᴺ absorption apart from the idealised N00N fringe.",
        notes=[
            ("Never recorded in a material", "every sub-Rayleigh demonstration used", "coincidence detection, not a resist"),
            ("No N-photon resist exists", "the λ/N fringe needs N photons absorbed", "coherently as one event"),
            ("Cross-section disputed", "σ_E bounds span 10⁻²⁵–10⁻¹⁷ cm², so the", "exposure times are lower bounds"),
        ],
        alt=("One lane to a coherent pupil of sigma 0.05: a pump laser drives a chi-2 crystal emitting degenerate photon "
             "pairs at 157.63 nm, an interferometer prepares the NOON state, the N-photon phase exposes a fringe of "
             "period lambda over 2N sin theta, and the N-photon resist is marked hypothetical."),
        caption=("<strong>Mechanism.</strong> A pump laser drives spontaneous parametric down-conversion in a χ⁽²⁾ crystal, "
                 "emitting 10⁶–10⁷ degenerate pairs per second at 157.63 nm; an interferometer prepares the NOON state "
                 "(|N,0⟩ + |0,N⟩)/√2, whose N-photon phase e^{iNφ} would write a fringe of period λ/(2N sin θ) into a resist "
                 "that absorbs N photons as one event — a resist that does not exist. The pipeline images the source "
                 "classically; the quantum module is an explicit opt-in. " + TRAILER),
    ),
}


HEADER_NOTE = "how the light is made → the illuminator pupil"


def draw(cluster: str, theme: str, highlight: str | None = None) -> str:
    c = CLUSTERS[cluster]
    return _cluster_svg(theme, name=f"sources-cluster-{cluster}", title=c["title"], alt=c["alt"], lanes=c["lanes"](),
                        highlight=highlight, families=c["families"], footer=c["footer"],
                        header_note=c.get("header_note", HEADER_NOTE), notes=c.get("notes"))


# ---------------------------------------------------------------------------
# Registration, generation, check
# ---------------------------------------------------------------------------


def _gen(cluster: str):
    name = f"sources-cluster-{cluster}"

    def gen() -> list[Path]:
        return K.write_pair(name, lambda theme: draw(cluster, theme), OUT)

    gen.__name__ = f"gen_{name.replace('-', '_')}"
    return gen


SPECS = [
    registry.AssetSpec(
        name=f"sources-cluster-{key}", kind="schematic", group="diagrams", subdir="diagrams", fmt="svg",
        alt=c["alt"], caption=c["caption"], pages=c["pages"], hero_on=c["pages"],
        generator=_gen(key), platform_independent=True,
        extra={"families": sorted(c["families"])},
    )
    for key, c in CLUSTERS.items()
]
for _s in SPECS:
    registry.register(_s, module=MODULE)


def _engine_pupils() -> dict[str, tuple[str, float, float | None]] | None:
    """{lane key: (shape, sigma, coherence)} from the extension, or None when it does not import."""
    try:
        import highuvlith as huv  # noqa: WPS433
    except Exception:
        return None
    out = {}
    for key, (factory, kwargs, _, _, _) in PUPILS.items():
        src = getattr(huv.SourceConfig, factory)(**kwargs)
        ill = src.illumination
        ill = ill() if callable(ill) else ill
        tc = src.transverse_coherence_fraction
        tc = tc() if callable(tc) else tc
        out[key] = (str(ill[0]), float(ill[1]), None if tc is None else float(tc))
    return out


def check() -> tuple[list[str], list[str]]:
    errors: list[str] = []
    warnings: list[str] = []
    # every cluster renders for every family highlight and with none
    for key, c in CLUSTERS.items():
        for fam in [None, *sorted(c["families"])]:
            for theme in ("light", "dark"):
                try:
                    text = draw(key, theme, fam)
                except Exception as e:  # pragma: no cover
                    errors.append(f"sources-cluster-{key}: draw({theme!r}, highlight={fam!r}) failed: {e}")
                    continue
                if "<a " in text or "<script" in text or "<animate" in text:
                    errors.append(f"sources-cluster-{key}: forbidden element in the SVG")
        lanes = c["lanes"]()
        keys = {ln.key for ln in lanes}
        for fam, lane_keys in c["families"].items():
            missing = set(lane_keys) - keys
            if missing:
                errors.append(f"sources-cluster-{key}: family {fam!r} names unknown lanes {sorted(missing)}")
        # the family names are the pages' stems
        stems = {p.split("/")[-1][:-3] for p in c["pages"]}
        if stems != set(c["families"]):
            errors.append(f"sources-cluster-{key}: families {sorted(c['families'])} != page stems {sorted(stems)}")
        h = _height(lanes, c.get("notes"))
        if W / h > 3.0:
            errors.append(f"sources-cluster-{key}: aspect {W / h:.2f} > 3 needs a narrow twin")
        if K.text_width(c["footer"], T_SMALL) > W - 2 * X0:
            errors.append(f"sources-cluster-{key}: footer note is {K.text_width(c['footer'], T_SMALL):.0f} wide (limit {W - 2 * X0})")
        for ln in lanes:
            if ln.key not in PUPILS:
                errors.append(f"sources-cluster-{key}: lane {ln.key!r} has no PUPILS entry")
            limit = _station_width(len(ln.stations)) + GAP - 4   # a label may use the gap but never its neighbour
            for st in ln.stations:
                for line, size, weight in ((st.title, T_LABEL, 600), *((l, T_SMALL, 400) for l in st.lines)):
                    if K.text_width(line, size, weight) > limit:
                        errors.append(f"sources-cluster-{key}/{ln.key}: label {line!r} is {K.text_width(line, size, weight):.0f} wide (limit {limit:.0f})")
            if K.text_width(ln.eyebrow, T_LABEL, 600) > PUPIL_X - PUPIL_R - 10 - X0:
                errors.append(f"sources-cluster-{key}/{ln.key}: lane title too wide")
            for line in (ln.end_lines or _pupil_label(ln.key)):
                if K.text_width(line, T_SMALL, 600) > 2 * (W - PUPIL_X) - 4:
                    errors.append(f"sources-cluster-{key}/{ln.key}: pupil label {line!r} too wide")
    for spec in SPECS:
        if not spec.caption.rstrip().endswith(TRAILER):
            errors.append(f"{spec.name}: caption must end with the no-badge trailer {TRAILER!r}")
        if any(g in spec.caption or g in spec.alt for g in ("✅", "🔶", "🧪", "🗺")):
            errors.append(f"{spec.name}: a schematic carries no grade glyph")
        if len(spec.alt) > 300:
            errors.append(f"{spec.name}: alt is {len(spec.alt)} characters (≤ 300)")
    # the pupil each lane draws is the preset's engine default
    engine = _engine_pupils()
    if engine is None:
        warnings.append("extension not importable: the engine-default pupil check was skipped")
    else:
        for key, (factory, _, shape, sigma, zeta) in PUPILS.items():
            e_shape, e_sigma, e_zeta = engine[key]
            if e_shape != shape or abs(e_sigma - sigma) > 5e-4:
                errors.append(f"PUPILS[{key!r}] = ({shape}, {sigma}) but SourceConfig.{factory}().illumination() = ({e_shape}, {e_sigma:.4f})")
            if zeta is not None and (e_zeta is None or abs(e_zeta - zeta) > 5e-4):
                errors.append(f"PUPILS[{key!r}] coherence {zeta} but the engine reports {e_zeta}")
    return errors, warnings


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if "--check" in argv:
        e, w = check()
        for x in w:
            print("warning:", x)
        for x in e:
            print("error:", x)
        print(f"source schematics check: {len(e)} error(s), {len(w)} warning(s)")
        return 1 if e else 0
    if "--highlight" in argv:  # preview: write <name>-<family>-<theme>.svg next to the scratch dir given
        i = argv.index("--highlight")
        fam, cluster = argv[i + 1], argv[i + 2]
        out = Path(argv[i + 3]) if len(argv) > i + 3 else Path(".")
        out.mkdir(parents=True, exist_ok=True)
        for theme in ("light", "dark"):
            p = out / f"sources-cluster-{cluster}-{fam}-{theme}.svg"
            p.write_text(draw(cluster, theme, fam), encoding="utf-8")
            print("wrote", p)
        return 0
    only = [a for a in argv if not a.startswith("--")]
    for spec in SPECS:
        if only and not any(o in spec.name for o in only):
            continue
        for p in spec.generator():
            print(f"wrote {registry.rel_to_repo(p)} ({p.stat().st_size / 1024:.1f} KB)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
