#!/usr/bin/env python3
"""I2 · I6a · I7a — the imaging-physics schematics (scripted SVG, the D5 pupil grammar).

Standard library only (svgkit), so every file here is *platform independent* and the
nightly can regenerate and diff it byte for byte. Written to ``docs/assets/images/diagrams/``:

* ``pupil-orders-anim`` (I2) — a two-layer CSS animation: the orders of a 1:1 grating enter
  and leave the pupil as the pitch shrinks (k₁ 1.0 → 0.28 under a conventional σ 0.7 fill,
  then an x-dipole at σ_c = 1/(4k₁) from k₁ 0.5 down to the 0.25 two-beam limit), with the
  1-D aerial cut and the image contrast of the closed-form partially coherent toy beneath.
  The toy is the same Abbe sum ``docs/javascripts/huv-physics.js`` uses
  (c₀ = 1 − w/p, c_m = −sin(πmw/p)/(πm), source points on a midpoint rule) and is asserted
  against the values its tests pin (:func:`check_toy`). The designed still
  (``pupil-orders-still``) shows three pupils side by side: σ 0.7 at k₁ 0.40 · the dipole
  at k₁ 0.30 · the 0.25 limit — and is also registered on its own for the pages that show
  the still (``pipeline.md``'s Physics Strip, the playground fallback).
* ``optics-pupil-cone`` (I6a) — the last lens surface, the image-side ray cone in air
  (NA 0.93 = sin 68.4°) and in water (NA 1.35 = 1.437·sin 70°), the pupil coordinate
  ρ = n sin θ / NA and the exact defocus phase Φ_z(ρ) as a labelled inset.
* ``vector-two-beam`` (I7a) — two plane waves at ±θ: TE fields stay parallel (full fringe),
  TM fields overlap as cos 2θ (zero at 45°, NA = 1/√2 in air, reversed beyond); refracted
  into n = 1.7 the angle shrinks and TM recovers.

    python docs/figures/diagrams/make_physics_schematics.py            # write every pair
    python docs/figures/diagrams/make_physics_schematics.py pupil      # names containing "pupil"
    python docs/figures/diagrams/make_physics_schematics.py --check    # the toy against the JS pins, sizes
"""

from __future__ import annotations

import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import registry  # noqa: E402
import svgkit as K  # noqa: E402

OUT = registry.IMAGES / "diagrams"
GENERATOR = "docs/figures/diagrams/make_physics_schematics.py"

# ---------------------------------------------------------------------------
# The closed-form partially coherent toy (the same model as huv-physics.js aerialModel)
# ---------------------------------------------------------------------------


def mask_coefficient(m: int, w_over_p: float = 0.5) -> float:
    """Fourier coefficient c_m of a bright-field grating: opaque lines of width w on pitch p."""
    if m == 0:
        return 1.0 - w_over_p
    return -math.sin(math.pi * m * w_over_p) / (math.pi * m)


def source_points(sigma_in: float, sigma_out: float, count: int = 41) -> list[float]:
    """1-D source: ``count`` midpoint-rule points over σ_in ≤ |s| ≤ σ_out (two poles when σ_in > 0)."""
    lo = max(0.0, min(sigma_in, sigma_out))
    hi = max(0.0, sigma_out)
    if hi == 0:
        return [0.0]
    segments = [(-hi, hi)] if lo == 0 else [(-hi, -lo), (lo, hi)]
    per = max(1, round(count / len(segments)))
    pts: list[float] = []
    for a, b in segments:
        width = b - a
        if width == 0:
            pts.append(a)
            continue
        for j in range(per):
            pts.append(a + (j + 0.5) * width / per)
    return pts


class Toy:
    """1-D aerial image of a line/space grating by Abbe's method, in pupil units.

    ``k1`` = (p/2)·NA/λ, so order m sits at u_m = m/(2k₁) (units of NA/λ) and passes for
    source point s when |u_m + s| ≤ 1. Scalar, thin mask, in focus; clear field = 1.
    """

    def __init__(self, k1: float, *, sigma_in: float = 0.0, sigma_out: float = 0.0, count: int = 41,
                 w_over_p: float = 0.5):
        self.k1 = k1
        self.spacing = 1.0 / (2.0 * k1)
        self.sources = source_points(sigma_in, sigma_out, count)
        smax = max(abs(sigma_in), abs(sigma_out))
        mmax = int((1.0 + smax) / self.spacing) + 1
        self.fields: list[list[tuple[int, float]]] = []
        for s in self.sources:
            orders = []
            for m in range(-mmax, mmax + 1):
                if abs(m * self.spacing + s) > 1.0 + 1e-12:
                    continue
                orders.append((m, mask_coefficient(m, w_over_p)))
            self.fields.append(orders)

    def at(self, x_over_p: float) -> float:
        total = 0.0
        for orders in self.fields:
            re = im = 0.0
            for m, c in orders:
                ph = 2.0 * math.pi * m * x_over_p
                re += c * math.cos(ph)
                im += c * math.sin(ph)
            total += re * re + im * im
        return total / len(self.fields)

    def contrast(self, samples: int = 401) -> float:
        vals = [self.at(-0.5 + i / (samples - 1)) for i in range(samples)]
        hi, lo = max(vals), min(vals)
        return (hi - lo) / (hi + lo) if hi + lo > 0 else 0.0

    def passes_first_order(self) -> bool:
        return any(m != 0 for orders in self.fields for m, _c in orders)


def check_toy() -> list[str]:
    """The toy against the values ``docs/javascripts/tests/physics.test.mjs`` pins (three k₁ points)."""
    errors: list[str] = []

    def close(actual, expected, rel, label):
        if abs(actual - expected) > rel * max(1.0, abs(expected)):
            errors.append(f"toy: {label}: expected {expected}, got {actual}")

    # (1) coherent three-beam image, λ 193 nm, NA 0.93, pitch 300 nm, cd 150 nm → k₁ = 0.7228
    t = Toy(150.0 * 0.93 / 193.0)
    close(t.at(0.0), 0.018664962201769754, 1e-12, "coherent at(0)")
    close(t.at(37.5 / 300), 0.0024842092061225034, 1e-9, "coherent at(37.5)")
    close(t.at(75 / 300), 0.25, 1e-12, "coherent at(75)")
    close(t.at(150 / 300), 1.2919045069369328, 1e-12, "coherent at(150)")
    # (2) below λ/((1+σ)NA): pitch 100 nm, σ 0.7 → k₁ = 0.241, no first order, flat at c₀² = 0.25
    t = Toy(50.0 * 0.93 / 193.0, sigma_out=0.7)
    if t.passes_first_order():
        errors.append("toy: a first order passes below λ/((1+σ)NA)")
    close(t.contrast(), 0.0, 1e-12, "sub-cutoff contrast")
    close(max(t.at(x / 100) for x in range(0, 100, 5)), 0.25, 1e-12, "sub-cutoff Imax")
    # (3) the I2 cutoffs: conventional σ 0.7 stops at k₁ = 1/(2(1+σ)); the matched dipole at 0.25
    kc = 1.0 / (2.0 * 1.7)
    if not Toy(kc + 0.004, sigma_out=0.7).contrast() > 0.02:
        errors.append("toy: σ 0.7 has no contrast just above its cutoff")
    close(Toy(kc - 0.002, sigma_out=0.7).contrast(), 0.0, 1e-12, "σ 0.7 contrast below the cutoff")
    for k1 in (0.5, 0.3, 0.251):
        sc = 1.0 / (4.0 * k1)
        if not Toy(k1, sigma_in=sc, sigma_out=sc).contrast() > 0.85:
            errors.append(f"toy: the matched dipole has no two-beam contrast at k₁ {k1}")
    sc = 1.0 / (4.0 * 0.249)
    close(Toy(0.249, sigma_in=sc, sigma_out=sc).contrast(), 0.0, 1e-12, "dipole contrast past the 0.25 limit")
    return errors


# ---------------------------------------------------------------------------
# I2 — pupil-orders-anim: the timeline
# ---------------------------------------------------------------------------

W2, H2 = 900, 470
PCX, PCY, PR = 215, 240, 100  # the animated pupil: centre and radius (1 pupil unit = PR px)
SIGMA = 0.7
INV_LO, INV_HI = 1.0, 4.2  # the k₁ ruler is linear in 1/k₁ (the order spacing), 1.0 → 0.238
RX0, RX1 = 450, 860  # right column
CUT_Y0, CUT_Y1 = 60, 196  # the 1-D cut panel
CH_Y0, CH_Y1 = 300, 400  # the contrast chart
CH_TOP = 1.2  # the chart's contrast range: the strip above c = 1 holds the dipole's label, clear of every line
T_A = 5.0  # phase A ends (k₁ 1.0 → 0.28)
T_LIMIT = 8.5  # the dipole reaches the 0.25 limit
T_PAST = 9.0  # a hair past it (k₁ 0.24)
T_BACK = 9.5  # back on the limit; hold until T_END
T_END = 12.0
INV_A_END = 1.0 / 0.28
SIGC_PAST = 1.04


def scene(t: float) -> dict:
    """The toy's parameters at time ``t`` of the 12-s timeline (``T_A`` itself is the first instant of phase B)."""
    if t < T_A:
        inv = INV_LO + (INV_A_END - INV_LO) * t / T_A
        return dict(phase="A", inv=inv, k1=1.0 / inv, sigma_in=0.0, sigma_out=SIGMA, count=201)
    if t <= T_LIMIT:
        sc = 0.5 + 0.5 * (t - T_A) / (T_LIMIT - T_A)
    elif t <= T_PAST:
        sc = 1.0 + (SIGC_PAST - 1.0) * (t - T_LIMIT) / (T_PAST - T_LIMIT)
    elif t <= T_BACK:
        sc = SIGC_PAST - (SIGC_PAST - 1.0) * (t - T_PAST) / (T_BACK - T_PAST)
    else:
        sc = 1.0
    return dict(phase="B", inv=4.0 * sc, k1=1.0 / (4.0 * sc), sigma_in=sc, sigma_out=sc, count=2)


def toy_at(t: float) -> Toy:
    sc = scene(t)
    return Toy(sc["k1"], sigma_in=sc["sigma_in"], sigma_out=sc["sigma_out"], count=sc["count"])


def ruler_x(inv: float) -> float:
    return RX0 + (inv - INV_LO) / (INV_HI - INV_LO) * (RX1 - RX0)


def chart_y(c: float) -> float:
    return CH_Y1 - c / CH_TOP * (CH_Y1 - CH_Y0)


def cut_points(toy: Toy, x0: float, x1: float, y_base: float, height: float, n: int = 17,
               imax: float = 1.3) -> list[tuple[int, int]]:
    """One period of the cut (x/p from −1 to 0, drawn from x0 to x1) as integer points; a ``<use>`` repeats it."""
    pts = []
    for i in range(n):
        xp = -1.0 + i / (n - 1)
        val = toy.at(xp)
        pts.append((round(x0 + (xp + 1) * (x1 - x0)), round(y_base - min(val, imax) / imax * height)))
    return pts


def cut_element(s: K.Svg, uid: str, pts: list[tuple[int, int]], half: float) -> None:
    """The one-period polyline plus its shifted copy (two periods for the price of one)."""
    s.add(f'<polyline id="{uid}" points="{" ".join(f"{x},{y}" for x, y in pts)}" fill="none" '
          f'stroke="{s.c("brand:violet600")}" stroke-width="2"/>')
    s.add(f'<use href="#{uid}" x="{K.fmt(half)}"/>')


def snapshot_times(step: float = 0.1, tol: float = 0.04, max_gap: float = 1.5) -> list[float]:
    """Times whose cut differs noticeably from the previous kept one (or a second apart)."""
    kept = [0.0]
    last = [toy_at(0.0).at(-1 + 2 * i / 32) for i in range(33)]
    t = step
    while t <= T_BACK + 1e-9:
        cur = [toy_at(t).at(-1 + 2 * i / 32) for i in range(33)]
        if max(abs(a - b) for a, b in zip(cur, last)) > tol or t - kept[-1] >= max_gap - 1e-9:
            kept.append(round(t, 2))
            last = cur
        t = round(t + step, 6)
    if kept[-1] < T_BACK:
        kept.append(T_BACK)
    return kept


def _order_group(s: K.Svg, cls: str, m: int, u0: float, color: str, *, label: str, stem: bool = True,
                 hollow_cls: str | None = None, dot: float = 4.5, side: int = 1) -> None:
    """One order at ``u0`` pupil units (local origin = the pupil centre): stem (scaled) + dot (translated)."""
    if stem:
        s.add(f'<line x1="0" y1="0" x2="{K.fmt(side * PR)}" y2="0" stroke="{color}" stroke-width="1.2" '
              f'stroke-linecap="round" opacity="0.5" class="{cls}s"/>')
    with s.group(cls + "d", transform=f"translate({K.fmt(u0 * PR)} 0)"):
        s.circle(0, 0, dot, fill=color, stroke="surface", sw=1.2, cls=cls + "f")
        if hollow_cls:
            s.circle(0, 0, dot, fill="none", stroke=s.t["pupil"]["outside"], sw=1.4, cls=hollow_cls)
        s.text(0, -dot - 4, label, size=K.SMALL, color="ink2")


def draw_pupil_orders(theme: str) -> str:
    s = K.Svg(W2, H2, theme, "Where the k₁ limits come from", PUPIL_ALT, generator=GENERATOR)
    anim = K.Anim(s, loops=3)
    pf = s.t["pupil"]
    c0, c1, c3 = pf["order0"], pf["order1"], pf["order2"]
    times = snapshot_times()
    grid = [round(0.5 * i, 2) for i in range(int(T_A / 0.5))]
    conv_curve = [(inv, Toy(1.0 / inv, sigma_out=SIGMA, count=201).contrast()) for inv in [INV_LO + (INV_A_END - INV_LO) * i / 36 for i in range(37)]]
    dip_c = Toy(0.3, sigma_in=1 / 1.2, sigma_out=1 / 1.2, count=2).contrast()

    # ---- shared chrome (both layers): the legend -------------------------------------------
    ly = H2 - 14
    s.circle(40, ly - 5, 5, fill=c0, stroke="surface", sw=1.2)
    s.text(50, ly, "order 0", size=K.LABEL, anchor="start", color="ink2")
    s.circle(122, ly - 5, 5, fill=c1, stroke="surface", sw=1.2)
    s.text(132, ly, "±1", size=K.LABEL, anchor="start", color="ink2")
    s.circle(174, ly - 5, 5, fill=c3, stroke="surface", sw=1.2)
    s.text(184, ly, "±3 (even orders of a 1:1 grating vanish)", size=K.LABEL, anchor="start", color="ink2")
    s.circle(458, ly - 5, 5, fill="none", stroke=pf["outside"], sw=1.4)
    s.text(468, ly, "outside the (1+σ)·NA/λ support: no contribution", size=K.LABEL, anchor="start", color="ink2")
    s.rect(776, ly - 11, 14, 12, fill=pf["fill"], stroke=s.t["pupil_stroke"]["fill"], sw=1.0, rx=2)
    s.text(796, ly, "source fill", size=K.LABEL, anchor="start", color="ink2")

    # ======================= the animated layer ==========================================
    with anim.layer("anim"):
        # -- the pupil scene (left) --
        s.eyebrow(24, 30, "Pupil, radius = NA · orders at m/(2k₁)", size=K.SMALL)
        K.support_circle(s, PCX, PCY, PR, SIGMA, label=False, cls="supA")
        s.text(PCX + PR * 1.7 * 0.72 + 2, PCY - PR * 1.7 * 0.72 - 2, "(1+σ)·NA/λ", size=K.SMALL, color="ink2",
               anchor="start", cls="supA")
        K.pupil(s, PCX, PCY, PR, fill=None)
        clip = s.clip_rect(-PCX, 40 - PCY, 432, H2 - 70)  # in the groups' local coordinates: orders that leave the scene never cross into the right column
        with s.group("phA", transform=f"translate({PCX} {PCY})", extra=f'clip-path="{clip}"'):
            s.circle(0, 0, SIGMA * PR, fill=pf["fill"], stroke=s.t["pupil_stroke"]["fill"], sw=1.0)
            s.text(0, -PR - 14, "conventional source, σ 0.7", size=K.BASE, color="ink", weight=600)
            s.circle(0, 0, 4.5, fill=c0, stroke="surface", sw=1.2)
            s.text(0, -9, "0", size=K.SMALL, color="ink2")
            _order_group(s, "p1", 1, 0.5, c1, label="+1", hollow_cls="p1h")
            _order_group(s, "m1", -1, -0.5, c1, label="−1", hollow_cls="m1h", side=-1)
            _order_group(s, "p3", 3, 1.5, c3, label="+3", hollow_cls="p3h", dot=3.5)
            _order_group(s, "m3", -3, -1.5, c3, label="−3", hollow_cls="m3h", dot=3.5, side=-1)
        with s.group("phB", transform=f"translate({PCX} {PCY})", extra=f'clip-path="{clip}"'):
            s.text(0, -PR - 14, "x-dipole, poles at σ_c = 1/(4k₁)", size=K.BASE, color="ink", weight=600)
            s.text(0, PR + 22, "each pole sends its 0 order and one first order", size=K.SMALL, color="ink2")
            s.text(0, PR + 38, "to mirror positions: two-beam imaging", size=K.SMALL, color="ink2")
            s.add(f'<line x1="{-PR}" y1="0" x2="{PR}" y2="0" stroke="{c1}" stroke-width="1.2" stroke-linecap="round" '
                  f'opacity="0.5" class="bstem"/>')
            for sign, cls in ((-1, "bl"), (1, "br")):
                with s.group(cls, transform=f"translate({K.fmt(sign * 0.5 * PR)} 0)"):
                    s.circle(0, 0, 0.1 * PR, fill=pf["fill"], stroke=s.t["pupil_stroke"]["fill"], sw=1.0)
                    if sign < 0:
                        s.circle(0, 0, 4.5, fill=c0, stroke="surface", sw=1.2)
                        s.text(0, -15, "0", size=K.SMALL, color="ink2")
                        s.text(0, 24, "−σ_c", size=K.SMALL, color="ink2")
                    else:
                        s.circle(0, 0, 4.5, fill=c1, stroke="surface", sw=1.2, cls="b1f")
                        s.circle(0, 0, 4.5, fill="none", stroke=pf["outside"], sw=1.4, cls="b1h")
                        s.text(0, -15, "+1", size=K.SMALL, color="ink2")
                        s.text(0, 24, "+σ_c", size=K.SMALL, color="ink2")
        # -- the right column: the cut, the chart, the ruler --
        s.eyebrow(RX0, 30, "Aerial image of the closed-form toy · two periods", size=K.SMALL)
        s.rect(RX0, CUT_Y0, RX1 - RX0, CUT_Y1 - CUT_Y0, fill="none", stroke="axis", sw=1.0)
        for xp in (-1.0, 0.0, 1.0):  # the opaque lines of the mask under the axis
            xa = RX0 + (xp + 1) / 2 * (RX1 - RX0)
            s.rect(max(RX0, xa - 0.25 * (RX1 - RX0) / 2), CUT_Y1 + 4, min(RX1, xa + 0.25 * (RX1 - RX0) / 2) - max(RX0, xa - 0.25 * (RX1 - RX0) / 2), 5,
                   fill=s.t["material"]["absorber"], stroke="none")
        s.text(RX0, CUT_Y1 + 22, "mask lines (w = p/2)", size=K.SMALL, anchor="start", color="ink2")
        s.text(RX1, CUT_Y1 + 22, "x: −p … +p", size=K.SMALL, anchor="end", color="ink2")
        s.text(RX0 - 6, CUT_Y0 + 5, "1.3", size=K.SMALL, anchor="end", color="ink2")
        s.text(RX0 - 6, CUT_Y1 - (1.0 / 1.3) * (CUT_Y1 - CUT_Y0) + 4, "1", size=K.SMALL, anchor="end", color="ink2")
        s.text(RX0 - 6, CUT_Y1 + 4, "0", size=K.SMALL, anchor="end", color="ink2")
        s.line(RX0, CUT_Y1 - (1.0 / 1.3) * (CUT_Y1 - CUT_Y0), RX1, CUT_Y1 - (1.0 / 1.3) * (CUT_Y1 - CUT_Y0), "grid", 1.0)
        s.text(RX0 + 4, CUT_Y0 + 14, "I(x) / clear field", size=K.SMALL, anchor="start", color="ink2")
        half = (RX1 - RX0) / 2
        for i, t in enumerate(times):
            with s.group(f"c{i}", opacity=1.0 if i == 0 else 0.0):
                cut_element(s, f"q{i}", cut_points(toy_at(t), RX0, RX0 + half, CUT_Y1, CUT_Y1 - CUT_Y0), half)
        s.eyebrow(RX0, CH_Y0 - 56, "Image contrast of the toy as the pitch shrinks →", size=K.SMALL)
        # the chart box and the ruler
        s.line(RX0, CH_Y1, RX1, CH_Y1, "axis", 1.0)
        s.line(RX0, CH_Y0, RX0, CH_Y1, "axis", 1.0)
        for c in (0.5, 1.0):
            s.line(RX0, chart_y(c), RX1, chart_y(c), "grid", 1.0)
            s.text(RX0 - 6, chart_y(c) + 4, f"{c:g}", size=K.SMALL, anchor="end", color="ink2")
        s.text(RX0 - 6, CH_Y1 + 4, "0", size=K.SMALL, anchor="end", color="ink2")
        for k1 in (1.0, 0.7, 0.5, 0.4, 0.33, 0.3, 0.25):
            x = ruler_x(1 / k1)
            s.line(x, CH_Y1, x, CH_Y1 + 5, "axis", 1.0)
            s.text(x, CH_Y1 + 18, f"{k1:g}", size=K.SMALL, color="ink2")
        s.text(RX1, CH_Y1 + 34, "k₁ = (p/2)·NA/λ, axis linear in λ/p", size=K.SMALL, anchor="end", color="ink2")
        # the three limits: a dashed line each from the c = 1 gridline down to the axis, and one short caption
        # each above the chart, centred on its line (the 0.25 one a row higher, with a leader tick down to the chart)
        y_data_top = chart_y(1.0)
        for k1, label, anchor, dx, y in ((0.5, "coherent cutoff 0.5", "middle", 0, CH_Y0 - 6),
                                         (1 / 3.4, "σ 0.7 cutoff 0.29", "middle", 0, CH_Y0 - 6),
                                         (0.25, "two-beam limit 0.25", "end", RX1 - 4 - ruler_x(4.0), CH_Y0 - 22)):
            x = ruler_x(1 / k1)
            s.line(x, y_data_top, x, CH_Y1, "ink2", 1.0, dash="4 3", opacity=0.7)
            s.text(x + dx, y, label, size=K.SMALL, anchor=anchor, color="ink2")
            if y < CH_Y0 - 6:
                s.line(x, y + 4, x, y_data_top, "ink2", 1.0, opacity=0.7)
        # the two curves; the dipole's label sits in the strip above c = 1, right over its flat line and left of
        # the 0.25 leader, so it crosses neither curve nor cutoff line
        s.polyline([(round(ruler_x(inv)), round(chart_y(c))) for inv, c in conv_curve], "ink2", 1.8)
        s.text(ruler_x(1.04), chart_y(0.66), "conventional σ 0.7", size=K.SMALL, anchor="start", color="ink2")
        s.polyline([(ruler_x(2.0), chart_y(dip_c)), (ruler_x(4.0), chart_y(dip_c)), (ruler_x(4.0), chart_y(0)),
                    (ruler_x(4.0 * SIGC_PAST), chart_y(0))], "brand:violet600", 1.8)
        s.text(ruler_x(4.0) - 7, y_data_top - 5, "matched dipole: two beams, contrast stays", size=K.SMALL, anchor="end",
               color="brand:violet600")
        # the cursor (over the data range only, so it never sweeps through the label strip) and the dot
        s.add(f'<line x1="{K.fmt(ruler_x(INV_LO))}" y1="{K.fmt(y_data_top)}" x2="{K.fmt(ruler_x(INV_LO))}" y2="{CH_Y1}" '
              f'stroke="{s.c("brand:cyan700")}" stroke-width="1.2" class="cur"/>')
        s.add(f'<circle cx="0" cy="0" r="6" fill="{s.c("brand:cyan700")}" stroke="{s.c("surface")}" stroke-width="1.5" class="dot"/>')

    # ======================= the designed still ==========================================
    with anim.layer("still"):
        panels = ((150, "Conventional σ 0.7 at k₁ = 0.40", 0.40, None, "±1 inside the support: imaging"),
                  (450, "x-dipole at k₁ = 0.30", 0.30, 1 / 1.2, "0 and +1 land symmetrically"),
                  (750, "The limit: k₁ = 0.25", 0.25, 1.0, "pole and order on opposite rims"))
        R, cy = 72, 212
        for cx, title, k1, sc, note in panels:
            s.text(cx, 34, title, size=K.BASE, color="ink", weight=600)
            s.text(cx, 52, note, size=K.SMALL, color="ink2")
            if sc is None:
                K.support_circle(s, cx, cy, R, SIGMA, label=False)
                s.text(cx + R * 1.7 * 0.7 + 2, cy - R * 1.7 * 0.72, "(1+σ)·NA/λ", size=K.SMALL, color="ink2", anchor="start")
                K.pupil(s, cx, cy, R, fill="conventional", sigma=SIGMA)
                u = 1 / (2 * k1)
                K.orders(s, cx, cy, R, [(0, 0.0, 0.0), (1, u, 0.0), (-1, -u, 0.0), (3, 3 * u, 0.0), (-3, -3 * u, 0.0)],
                         support=1 + SIGMA)
                toy = Toy(k1, sigma_out=SIGMA)
            else:
                K.pupil(s, cx, cy, R, fill="dipole", poles=((sc, 0.0, 0.1), (-sc, 0.0, 0.1)))
                s.line(cx - sc * R, cy, cx + sc * R, cy, K.order_color(s, 1), 1.2, opacity=0.5)
                s.circle(cx - sc * R, cy, 4.5, fill=K.order_color(s, 0), stroke="surface", sw=1.2)
                s.text(cx - sc * R, cy - 10, "0", size=K.SMALL, color="ink2")
                s.circle(cx + sc * R, cy, 4.5, fill=K.order_color(s, 1), stroke="surface", sw=1.2)
                s.text(cx + sc * R, cy - 10, "+1", size=K.SMALL, color="ink2")
                s.text(cx - sc * R, cy + 22, "−σ_c", size=K.SMALL, color="ink2")
                s.text(cx + sc * R, cy + 22, "+σ_c", size=K.SMALL, color="ink2")
                if sc >= 1.0:
                    s.dim_h(cx - R, cx + R, cy + R + 30, "λ/p = 2·NA  ⇒  p = λ/(2·NA)", size=K.SMALL)
                toy = Toy(k1, sigma_in=sc, sigma_out=sc, count=2)
            x0, x1, yb, hh = cx - 100, cx + 100, 410, 54
            s.rect(x0, yb - hh, x1 - x0, hh, fill="none", stroke="axis", sw=1.0)
            cut_element(s, f"qs{cx}", cut_points(toy, x0, (x0 + x1) / 2, yb, hh), (x1 - x0) / 2)
            s.text(x1, yb + 16, f"contrast {toy.contrast():.2f}", size=K.LABEL, anchor="end", color="ink", weight=600)
            s.text(x0, yb + 16, "I(x), two periods", size=K.SMALL, anchor="start", color="ink2")

    # ======================= keyframes ===================================================
    T = T_END
    # phase switch
    kf = anim.keyframes("phA", T).at(0, "opacity:1").at(T_A - 0.02, "opacity:1").at(T_A, "opacity:0").at(T, "opacity:0")
    anim.animate(".phA,.supA", "phA", T)
    kf = anim.keyframes("phB", T).at(0, "opacity:0").at(T_A - 0.02, "opacity:0").at(T_A, "opacity:1").at(T, "opacity:1")
    anim.animate(".phB", "phB", T)
    # phase A: the ±1 and ±3 orders move linearly in λ/p (stem = scaleX, dot = translateX)
    u_end = INV_A_END / 2
    for cls, m in (("p1", 1), ("m1", -1), ("p3", 3), ("m3", -3)):
        a = abs(m) * 0.5
        b = abs(m) * u_end
        sgn = 1 if m > 0 else -1
        anim.keyframes(cls + "s", T).at(0, f"transform:scaleX({K.fmt(a)})").at(T_A, f"transform:scaleX({K.fmt(b)})").at(T, f"transform:scaleX({K.fmt(b)})")
        anim.animate(f".{cls}s", cls + "s", T)
        anim.keyframes(cls + "d", T).at(0, f"transform:translateX({K.fmt(sgn * a * PR)}px)").at(T_A, f"transform:translateX({K.fmt(sgn * b * PR)}px)").at(T, f"transform:translateX({K.fmt(sgn * b * PR)}px)")
        anim.animate(f".{cls}d", cls + "d", T)
        # hollow once outside the support circle (contribution gone)
        t_out = T_A * ((1 + SIGMA) / abs(m) - 0.5) / (u_end - 0.5)
        if 0 <= t_out <= T_A:
            anim.keyframes(cls + "f", T).at(0, "opacity:1").at(max(0.0, t_out - 0.02), "opacity:1").at(t_out, "opacity:0").at(T, "opacity:0")
            anim.animate(f".{cls}f", cls + "f", T)
            anim.keyframes(cls + "h", T).at(0, "opacity:0").at(max(0.0, t_out - 0.02), "opacity:0").at(t_out, "opacity:1").at(T, "opacity:1")
            anim.animate(f".{cls}h", cls + "h", T)
    # phase B: the poles move apart (σ_c 0.5 → 1.0 → 1.04 → 1.0); the +1 order goes hollow past the rim
    def sc_at(t: float) -> float:
        return scene(t)["sigma_out"]

    for cls, sgn in (("bl", -1), ("br", 1)):
        kfb = anim.keyframes(cls, T)
        for t in (0.0, T_A, T_LIMIT, T_PAST, T_BACK, T):
            kfb.at(t, f"transform:translateX({K.fmt(sgn * sc_at(max(t, T_A)) * PR)}px)")
        anim.animate(f".{cls}", cls, T)
    kfb = anim.keyframes("bstem", T)
    for t in (0.0, T_A, T_LIMIT, T_PAST, T_BACK, T):
        kfb.at(t, f"transform:scaleX({K.fmt(sc_at(max(t, T_A)))})")
    anim.animate(".bstem", "bstem", T)
    anim.keyframes("b1f", T).at(0, "opacity:1").at(T_LIMIT, "opacity:1").at(T_LIMIT + 0.03, "opacity:0").at(T_BACK - 0.03, "opacity:0").at(T_BACK, "opacity:1").at(T, "opacity:1")
    anim.animate(".b1f", "b1f", T)
    anim.keyframes("b1h", T).at(0, "opacity:0").at(T_LIMIT, "opacity:0").at(T_LIMIT + 0.03, "opacity:1").at(T_BACK - 0.03, "opacity:1").at(T_BACK, "opacity:0").at(T, "opacity:0")
    anim.animate(".b1h", "b1h", T)
    # the cut snapshots cross-fade
    for i, t in enumerate(times):
        kfc = anim.keyframes(f"c{i}", T)
        prev = times[i - 1] if i > 0 else None
        nxt = times[i + 1] if i + 1 < len(times) else None
        if prev is None:
            kfc.at(0, "opacity:1")
        else:
            kfc.at(0, "opacity:0").at(prev, "opacity:0").at(t, "opacity:1")
        if nxt is None:
            kfc.at(T, "opacity:1")
        else:
            kfc.at(nxt, "opacity:0").at(T, "opacity:0")
        anim.animate(f".c{i}", f"c{i}", T)
    # the cursor (linear per phase) and the dot (on the active curve)
    kfx = anim.keyframes("cur", T)
    for t in (0.0, T_A - 0.02, T_A, T_LIMIT, T_PAST, T_BACK, T):
        inv = scene(min(t, T_A - 0.02) if t < T_A else t)["inv"]
        kfx.at(t, f"transform:translateX({K.fmt(ruler_x(inv) - ruler_x(INV_LO))}px)")
    anim.animate(".cur", "cur", T)
    kfd = anim.keyframes("dot", T)
    for t in grid + [T_A - 0.02, T_A, T_LIMIT, T_LIMIT + 0.03, T_PAST, T_BACK - 0.03, T_BACK, T]:
        tt = min(t, T_A - 0.02) if t < T_A else t
        sc = scene(tt)
        c = toy_at(tt).contrast() if sc["phase"] == "A" else (dip_c if sc["sigma_out"] <= 1.0 + 1e-9 else 0.0)
        kfd.at(t, f"transform:translate({K.fmt(round(ruler_x(sc['inv']), 1))}px,{K.fmt(round(chart_y(c), 1))}px)")
    anim.animate(".dot", "dot", T)
    anim.install()
    return s.render()


PUPIL_ALT = ("Animated pupil diagram: the orders of a 1:1 grating slide outward as k1 falls from 1.0 to 0.28 under a "
             "sigma 0.7 fill and the image contrast beneath drops to zero at k1 = 0.29; a matched dipole then keeps "
             "two-beam contrast down to the 0.25 limit, where pole and order sit on opposite rims.")
PUPIL_CAPTION = ("<strong>Where the k₁ limits come from.</strong> Orders of a 1:1 grating sit at m/(2k₁) in pupil "
                 "units; a conventional fill of radius σ keeps imaging until the ±1 orders leave the support circle at "
                 "k₁ = 1/(2(1+σ)); a dipole at σ_c = λ/(2pNA) keeps two-beam imaging down to k₁ = 0.25, where pole "
                 "and order sit on opposite rims. The contrast strip is the closed-form 1-D toy; the support circle "
                 "is the engine's (1+σ)·NA/λ rule. <em>Closed form and geometry; not the engine. No model badge.</em>")
PUPIL_STILL_ALT = ("Three pupil diagrams: a sigma 0.7 fill at k1 0.40 with the first orders inside the support circle, "
                   "a matched dipole at k1 0.30 with its zeroth and first order placed symmetrically, and the k1 = 0.25 "
                   "limit with pole and order on opposite rims; the toy's image cut and contrast sit under each.")
PUPIL_STILL_CAPTION = ("<strong>Three pupils, one rule.</strong> A conventional σ 0.7 fill still images at k₁ = 0.40 "
                       "(the ±1 orders are inside the (1+σ)·NA/λ support circle); a dipole matched to the pitch images at "
                       "k₁ = 0.30 with two beams; at k₁ = 0.25 pole and order sit on opposite rims and nothing below it "
                       "prints in one exposure. The resting frame of the animation on the lithography-arithmetic page. "
                       "<em>Closed form and geometry; not the engine. No model badge.</em>")


# ---------------------------------------------------------------------------
# I6a — optics-pupil-cone
# ---------------------------------------------------------------------------

W6, H6 = 900, 440
N_WATER = 1.437


def _cone_panel(s: K.Svg, x0: float, w: float, *, na: float, n: float, title: str, sub: str) -> None:
    cx = x0 + w / 2
    theta = math.asin(na / n)
    y_pupil, y_focus = 232, 302
    half = (y_focus - y_pupil) * math.tan(theta)
    s.text(x0, 30, title, size=K.BASE, anchor="start", color="ink", weight=600)
    s.text(x0, 48, sub, size=K.SMALL, anchor="start", color="ink2")
    # medium between the last surface and the wafer
    if n > 1:
        s.rect(x0 + 10, y_pupil, w - 20, y_focus - y_pupil, "water", sw=0.8)
        s.text(x0 + 16, y_focus - 8, f"water, n = {n}", size=K.SMALL, anchor="start", color="ink2")
    else:
        s.text(x0 + 16, y_focus - 8, "air, n = 1", size=K.SMALL, anchor="start", color="ink2")
    # the last lens element: a plano-convex shape above the pupil plane
    lw = min(half + 24, w / 2 - 14)
    s.path(f"M{K.fmt(cx - lw)},{K.fmt(y_pupil - 48)} L{K.fmt(cx + lw)},{K.fmt(y_pupil - 48)} "
           f"Q{K.fmt(cx + lw)},{K.fmt(y_pupil - 2)} {K.fmt(cx)},{K.fmt(y_pupil - 2)} "
           f"Q{K.fmt(cx - lw)},{K.fmt(y_pupil - 2)} {K.fmt(cx - lw)},{K.fmt(y_pupil - 48)} Z", "glass", sw=1.0)
    s.text(cx, y_pupil - 56, "last lens surface", size=K.SMALL, color="ink2")
    # the pupil coordinate ρ along the exit pupil
    yp = y_pupil - 6
    s.line(cx - half, yp, cx + half, yp, "ink2", 1.2)
    for rho in (-1.0, -0.5, 0.0, 0.5, 1.0):
        x = cx + rho * half
        s.line(x, yp - 4, x, yp + 4, "ink2", 1.0)
    s.text(cx - half, yp - 9, "ρ = −1", size=K.SMALL, color="ink2")
    s.text(cx, yp - 9, "ρ = 0", size=K.SMALL, color="ink2")
    s.text(cx + half, yp - 9, "ρ = 1", size=K.SMALL, color="ink2")
    # the ray cone
    for rho in (-1.0, -0.6, -0.3, 0.0, 0.3, 0.6, 1.0):
        s.line(cx + rho * half, yp, cx, y_focus, "brand:cyan700" if abs(rho) == 1 else "brand:cyan300",
               1.8 if abs(rho) == 1 else 1.0, opacity=1.0 if abs(rho) == 1 else 0.8)
    # the axis and the half-angle
    s.line(cx, y_pupil - 60, cx, y_focus + 14, "muted", 0.8, dash="3 3")
    r_arc = 34
    s.path(f"M{K.fmt(cx)},{K.fmt(y_focus - r_arc)} A{r_arc},{r_arc} 0 0,1 {K.fmt(cx + r_arc * math.sin(theta))},{K.fmt(y_focus - r_arc * math.cos(theta))}",
           stroke="ink", sw=1.2)
    s.text(cx + r_arc * math.sin(theta / 2) + 16, y_focus - r_arc * math.cos(theta / 2) - 2, f"θ = {math.degrees(theta):.1f}°",
           size=K.LABEL, anchor="start", color="ink", weight=600)
    # the wafer and the defocus plane
    K.wafer(s, x0 + 10, y_focus, w - 20, 8, label=None)
    s.text(x0 + 16, y_focus + 20, "wafer (best focus)", size=K.SMALL, anchor="start", color="ink2")
    yz = y_focus + 36
    s.line(x0 + 10, yz, x0 + w - 10, yz, "ink2", 1.0, dash="5 4")
    s.text(x0 + 16, yz + 15, "defocused plane", size=K.SMALL, anchor="start", color="ink2")
    s.dim_v(x0 + w - 28, y_focus, yz, "z", size=K.SMALL, side="left")


def draw_pupil_cone(theme: str) -> str:
    s = K.Svg(W6, H6, theme, "The image-side cone and the pupil coordinate", CONE_ALT, generator=GENERATOR)
    _cone_panel(s, 20, 400, na=0.93, n=1.0, title="Dry: NA = sin θ = 0.93 in air",
                sub="the marginal ray leaves the pupil rim at 68.4°; air caps NA below 1")
    _cone_panel(s, 480, 400, na=1.35, n=N_WATER, title="Immersion: NA = n·sin θ = 1.437 × 0.94 = 1.35",
                sub="a 70° cone in water carries an NA that no cone in air can")
    s.line(450, 24, 450, 350, "grid", 1.0)
    # the defocus phase: formula + inset
    y = 384
    s.eyebrow(20, y, "Defocus phase across the pupil, the engine's rule", size=K.SMALL)
    s.text(20, y + 22, "Φ_z(ρ) = (2πn/λ) · z · (1 − √(1 − (NA·ρ/n)²))", size=K.BASE, anchor="start", color="ink", weight=600)
    s.text(20, y + 42, "ρ = n·sin θ / NA: ρ = 1 is the marginal ray, where Φ is steepest", size=K.SMALL, anchor="start", color="ink2")
    ix0, iy0, iw, ih = 650, 380, 210, 46
    s.rect(ix0, iy0, iw, ih, fill="none", stroke="axis", sw=1.0)
    for na, n, col in ((0.93, 1.0, "brand:cyan300"), (1.35, N_WATER, "brand:cyan700")):
        pts = []
        for i in range(31):
            rho = i / 30
            phi = n * (1 - math.sqrt(1 - (na * rho / n) ** 2))  # Φ in units of 2πz/λ
            pts.append((ix0 + rho * iw, iy0 + ih - phi / 1.0 * ih))
        s.polyline(pts, col, 1.8)
    s.text(ix0 + iw + 6, iy0 + ih - 0.632 * ih + 4, "0.63", size=K.SMALL, anchor="start", color="ink2")
    s.text(ix0 + iw + 6, iy0 + ih - 0.945 * ih + 4, "0.95", size=K.SMALL, anchor="start", color="ink2")
    s.text(ix0, iy0 - 8, "Φ_z(ρ) / (2πz/λ) against ρ: dry 0.93 · water 1.35", size=K.SMALL, anchor="start", color="ink2")
    s.text(ix0 + 4, iy0 + ih - 4, "0", size=K.SMALL, anchor="start", color="ink2")
    s.text(ix0 + iw - 4, iy0 + ih - 4, "ρ = 1", size=K.SMALL, anchor="end", color="ink2")
    return s.render()


CONE_ALT = ("Two ray cones converging on a wafer: in air the marginal ray of NA 0.93 leaves the pupil rim at 68.4 degrees, "
            "in water (n 1.437) a 70-degree cone carries NA 1.35; rho runs along the exit pupil, a dashed plane marks a "
            "defocus z, and an inset plots the defocus phase against rho.")
CONE_CAPTION = ("<strong>Mechanism.</strong> The numerical aperture is n·sin θ of the marginal ray, so air caps it below 1 "
                "while water (n = 1.437) carries NA 1.35 at a 70° half-angle; the pupil coordinate ρ = n·sin θ/NA puts "
                "that ray on the rim, and a defocus z adds the exact phase Φ_z(ρ) = (2πn/λ)·z·(1 − √(1 − (NAρ/n)²)) "
                "across the pupil, steepest at the rim. <em>Schematic, not to scale.</em>")


# ---------------------------------------------------------------------------
# I7a — vector-two-beam
# ---------------------------------------------------------------------------

W7, H7 = 900, 456
THETA_DEG = 35.0
N_RESIST = 1.7


def _wave_panel(s: K.Svg, x0: float, w: float, *, tm: bool) -> None:
    cx = x0 + w / 2
    th = math.radians(THETA_DEG)
    y_meet, L = 212, 150
    title = "TM — E in the plane of the rays" if tm else "TE — E along the lines"
    s.text(x0, 30, title, size=K.BASE, anchor="start", color="ink", weight=600)
    sub = "E across the lines; E₁·E₂* = cos 2θ, the fields tilt" if tm else "E out of the page; E₁·E₂* = 1, the fields stay parallel"
    s.text(x0, 48, sub, size=K.SMALL, anchor="start", color="ink2")
    col = s.c("brand:cyan700")
    for sign in (-1, 1):
        # k vector from the upper corner down to the meeting point
        xs, ys = cx + sign * L * math.sin(th), y_meet - L * math.cos(th)
        s.line(xs, ys, cx, y_meet, "brand:cyan700", 2.0, arrow="end")
        s.text(xs + sign * 8, ys - 8, f"k{'₂' if sign > 0 else '₁'}", size=K.LABEL, anchor="start" if sign > 0 else "end", color="ink")
        # the field marks at three points along the ray
        for f in (0.3, 0.55, 0.8):
            px, py = xs + (cx - xs) * f, ys + (y_meet - ys) * f
            if tm:
                # E ⊥ k in the plane: (cos θ, −sin θ) for the ray coming from the left (sign −1), mirrored for the right
                ex, ey = math.cos(th) * 11, -sign * math.sin(th) * 11
                s.line(px - ex, py - ey, px + ex, py + ey, "orange", 2.0, arrow="both")
            else:
                s.circle(px, py, 5, fill="surface", stroke="orange", sw=1.6)
                s.circle(px, py, 1.6, fill="orange", stroke="none")
    # the half-angle
    r_arc = 40
    s.line(cx, y_meet - L * math.cos(th) - 10, cx, y_meet + 4, "muted", 0.8, dash="3 3")
    s.path(f"M{K.fmt(cx)},{K.fmt(y_meet - r_arc)} A{r_arc},{r_arc} 0 0,1 {K.fmt(cx + r_arc * math.sin(th))},{K.fmt(y_meet - r_arc * math.cos(th))}",
           stroke="ink", sw=1.1)
    s.text(cx + r_arc + 6, y_meet - 4, f"θ = {THETA_DEG:g}°", size=K.SMALL, anchor="start", color="ink")
    # the E-field legend of this panel
    if tm:
        s.text(x0 + w / 2, y_meet + 24, "E₁ = (cos θ, 0, −sin θ) · E₂ = (cos θ, 0, +sin θ)", size=K.SMALL, color="ink2")
    else:
        s.text(x0 + w / 2, y_meet + 24, "E₁ = E₂ = (0, 1, 0): the dot marks E out of the page", size=K.SMALL, color="ink2")
    # the fringe below: I = 1 + V cos(2k x sin θ)
    V = abs(math.cos(2 * th)) if tm else 1.0
    fx0, fx1, fy, fh = x0 + 10, x0 + w - 10, 330, 66
    s.rect(fx0, fy - fh, fx1 - fx0, fh, fill="none", stroke="axis", sw=1.0)
    pts = []
    for i in range(121):
        xx = i / 120
        val = 1 - V * math.cos(2 * math.pi * 3 * xx)
        pts.append((fx0 + xx * (fx1 - fx0), fy - val / 2 * fh))
    s.polyline(pts, "brand:violet600", 2.0)
    s.text(fx0, fy + 16, "fringe I(x) = 1 + V·cos(2kx·sin θ)", size=K.SMALL, anchor="start", color="ink2")
    s.text(fx1, fy - fh - 6, f"V = {'|cos 2θ| = ' if tm else ''}{V:.2f}", size=K.LABEL, anchor="end", color="ink", weight=600)
    s.text(fx0 + 4, fy - fh + 14, "2", size=K.SMALL, anchor="start", color="ink2")
    s.text(fx0 + 4, fy - 4, "0", size=K.SMALL, anchor="start", color="ink2")


def draw_two_beam(theme: str) -> str:
    s = K.Svg(W7, H7, theme, "Two-beam interference: TE against TM", TWOBEAM_ALT, generator=GENERATOR)
    _wave_panel(s, 20, 280, tm=False)
    _wave_panel(s, 320, 280, tm=True)
    s.line(310, 24, 310, 356, "grid", 1.0)
    s.line(610, 24, 610, 356, "grid", 1.0)
    # the closed-form chart: |cos 2θ| against θ in air and inside a resist
    x0, w = 640, 240
    s.text(x0, 30, "TM contrast against the angle", size=K.BASE, anchor="start", color="ink", weight=600)
    s.text(x0, 48, "|cos 2θ| in air; refracted into n = 1.7", size=K.SMALL, anchor="start", color="ink2")
    cx0, cy0, cw, ch = x0, 80, w, 220
    cy1 = cy0 + ch
    s.line(cx0, cy1, cx0 + cw, cy1, "axis", 1.0)
    s.line(cx0, cy0, cx0, cy1, "axis", 1.0)
    for c in (0.5, 1.0):
        s.line(cx0, cy1 - c * ch, cx0 + cw, cy1 - c * ch, "grid", 1.0)
        s.text(cx0 - 6, cy1 - c * ch + 4, f"{c:g}", size=K.SMALL, anchor="end", color="ink2")
    s.text(cx0 - 6, cy1 + 4, "0", size=K.SMALL, anchor="end", color="ink2")
    for deg in (0, 30, 45, 60, 90):
        x = cx0 + deg / 90 * cw
        s.line(x, cy1, x, cy1 + 5, "axis", 1.0)
        s.text(x, cy1 + 18, f"{deg}°", size=K.SMALL, color="ink2")
    s.text(cx0 + cw / 2, cy1 + 36, "half-angle θ between the beams (air)", size=K.SMALL, color="ink2")
    # reversed-fringe region beyond 45°
    s.rect(cx0 + cw / 2, cy0, cw / 2, ch, fill=s.tint("ink", 0.05), stroke="none")
    s.text(cx0 + cw - 4, cy1 - 8, "fringe reversed →", size=K.SMALL, anchor="end", color="ink2")
    air, resist = [], []
    for i in range(91):
        th = math.radians(i)
        air.append((cx0 + i / 90 * cw, cy1 - abs(math.cos(2 * th)) * ch))
        thr = math.asin(math.sin(th) / N_RESIST)
        resist.append((cx0 + i / 90 * cw, cy1 - abs(math.cos(2 * thr)) * ch))
    s.polyline(resist, "aqua", 2.0)
    s.polyline(air, "orange", 2.0)
    s.label_box(cx0 + cw * 0.72, cy1 - 0.17 * ch, "zero at 45° (NA = 1/√2 in air)", size=K.SMALL, color="ink2", fill="surface", stroke="axis")
    s.label_box(cx0 + cw * 0.24, cy1 - 0.60 * ch, "TM in air: |cos 2θ|", size=K.SMALL, color="orange", fill="surface", stroke="axis")
    s.label_box(cx0 + cw * 0.70, cy1 - 0.44 * ch, "in resist, n = 1.7", size=K.SMALL, color="aqua", fill="surface", stroke="axis")
    # the TE reference and the panel angle marker
    s.line(cx0, cy0, cx0 + cw, cy0, "ink2", 1.2, dash="4 3")
    s.text(cx0 + 4, cy0 - 6, "TE: contrast 1 at every angle", size=K.SMALL, anchor="start", color="ink2")
    xm = cx0 + THETA_DEG / 90 * cw
    s.circle(xm, cy1 - abs(math.cos(2 * math.radians(THETA_DEG))) * ch, 5, fill="orange", stroke="surface", sw=1.5)
    s.text(xm - 8, cy1 - abs(math.cos(2 * math.radians(THETA_DEG))) * ch + 4, f"{THETA_DEG:g}°", size=K.SMALL, anchor="end", color="ink2")
    # footer
    s.text(20, H7 - 30, "Two plane waves at ±θ meet on the wafer; the fringe visibility is the normalised product of their field vectors:",
           size=K.SMALL, anchor="start", color="ink2")
    s.text(20, H7 - 12, "1 for TE, cos 2θ for TM, (1 + cos 2θ)/2 for unpolarized light.", size=K.SMALL, anchor="start", color="ink2")
    return s.render()


TWOBEAM_ALT = ("Three panels: two TE plane waves at plus and minus 35 degrees with dot-marked fields and a full-contrast "
               "fringe; the same TM waves with in-plane field arrows and a fringe of contrast 0.34; and TM contrast "
               "cos 2 theta against the angle, zero at 45 degrees in air and higher in a resist of index 1.7.")
TWOBEAM_CAPTION = ("<strong>Mechanism.</strong> Two plane waves at ±θ interfere with the product of their field vectors: TE "
                   "fields (along the lines) stay parallel and give full fringe contrast; TM fields tilt with the rays and "
                   "overlap only as cos 2θ — zero at θ = 45° (NA = 1/√2 in air), reversed beyond. Inside a resist "
                   "(n = 1.7) the rays refract to a smaller angle and TM recovers. <em>Closed form; not the engine.</em>")


# ---------------------------------------------------------------------------
# Registration, generation, check
# ---------------------------------------------------------------------------


def gen_pupil_orders() -> list[Path]:
    errs = check_toy()
    if errs:
        raise SystemExit("\n".join(errs))
    return K.write_pair("pupil-orders-anim", draw_pupil_orders, OUT)


def gen_pupil_cone() -> list[Path]:
    return K.write_pair("optics-pupil-cone", draw_pupil_cone, OUT)


def gen_two_beam() -> list[Path]:
    return K.write_pair("vector-two-beam", draw_two_beam, OUT)


registry.register(registry.AssetSpec(
    name="pupil-orders-anim", kind="anim", group="diagrams", subdir="diagrams", fmt="svg", animated="svg",
    alt=PUPIL_ALT, caption=PUPIL_CAPTION,
    pages=("nodes/litho-math.md", "history/excimer-duv.md", "index.md"),
    generator=gen_pupil_orders, platform_independent=True,
), module="make_physics_schematics.py")

#: The designed still on its own, for the pages that show the resting frame rather than the animation
#: (DESIGN §5.2 I2: pipeline.md's mechanism slot, the playground's aerial-toy fallback); the files are the
#: ``pupil-orders-still-*.svg`` that ``gen_pupil_orders`` derives from the animation.
registry.register(registry.AssetSpec(
    name="pupil-orders-still", kind="schematic", group="diagrams", subdir="diagrams", fmt="svg",
    alt=PUPIL_STILL_ALT, caption=PUPIL_STILL_CAPTION,
    pages=("pipeline.md", "playground/index.md", "masks.md"),
    generator=gen_pupil_orders, platform_independent=True,
), module="make_physics_schematics.py")

registry.register(registry.AssetSpec(
    name="optics-pupil-cone", kind="schematic", group="diagrams", subdir="diagrams", fmt="svg",
    alt=CONE_ALT, caption=CONE_CAPTION, pages=("optics.md",),
    generator=gen_pupil_cone, platform_independent=True,
), module="make_physics_schematics.py")

registry.register(registry.AssetSpec(
    name="vector-two-beam", kind="schematic", group="diagrams", subdir="diagrams", fmt="svg",
    alt=TWOBEAM_ALT, caption=TWOBEAM_CAPTION, pages=("vector-imaging.md",),
    generator=gen_two_beam, platform_independent=True,
), module="make_physics_schematics.py")


def check() -> tuple[list[str], list[str]]:
    errors = check_toy()
    warnings: list[str] = []
    for name, cap in (("pupil-orders-anim", 35), ("pupil-orders-still", 60), ("optics-pupil-cone", 20), ("vector-two-beam", 20)):
        for theme in ("light", "dark"):
            p = OUT / f"{name}-{theme}.svg"
            if not p.exists():
                warnings.append(f"{name}: {p.name} not rendered yet")
            elif K.size_kb(p) > cap:
                errors.append(f"{name}: {p.name} is {K.size_kb(p):.0f} KB, over the {cap} KB budget of DESIGN §5.2")
    return errors, warnings


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if "--check" in argv:
        e, w = check()
        for x in w:
            print("warning:", x)
        for x in e:
            print("error:", x)
        print(f"physics schematics check: {len(e)} error(s), {len(w)} warning(s)")
        return 1 if e else 0
    only = [a for a in argv if not a.startswith("--")]
    for name, fn in (("pupil-orders-anim", gen_pupil_orders), ("optics-pupil-cone", gen_pupil_cone), ("vector-two-beam", gen_two_beam)):
        if only and not any(o in name for o in only):
            continue
        for p in fn():
            print(f"wrote {registry.rel_to_repo(p)} ({p.stat().st_size / 1024:.1f} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
