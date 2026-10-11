#!/usr/bin/env python3
"""R3 · the playground's static fallback figures (DESIGN §5.6).

Three scripted SVG pairs (svgkit, standard library only → platform independent) that stand
in for the calculators where JavaScript does not run (github.com, print, no-JS); the page
hides each one when its widget mounts. The physics is the same closed-form textbook toy
``docs/javascripts/huv-physics.js`` computes in the browser, re-derived here in Python and
asserted against the values ``docs/javascripts/tests/physics.test.mjs`` pins at three sample
points (``check()``, run before every write and by ``--check``). Nothing here is the engine;
every caption ends in the no-badge trailer.

* ``playground-rayleigh`` — half-pitch k₁λ/NA and depth of focus k₂λ/NA² (paraxial dashed,
  exact quarter-wave solid, ending at NA = n) against NA, presets as dots;
* ``playground-shotnoise`` — photons per feature and the Poisson 1σ against feature size;
* ``playground-aerial`` — the aerial toy's opening state: the D5 pupil with its orders and
  the 1-D partially coherent Abbe image over two periods.

    python docs/figures/playground/make_playground_figures.py           # check the pins, write all three pairs
    python docs/figures/playground/make_playground_figures.py --check   # pins only; writes nothing
"""

from __future__ import annotations

import cmath
import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import registry  # noqa: E402
import svgkit as K  # noqa: E402

OUT = registry.IMAGES / "playground"
GENERATOR = "docs/figures/playground/make_playground_figures.py"
TRAILER = "<em>Closed-form toy, same formulas as the calculator; not the engine. No model badge.</em>"

# ---------------------------------------------------------------------------
# The toy physics (a line-for-line port of huv-physics.js)
# ---------------------------------------------------------------------------

HC_EV_NM = 1239.84193  # h·c in eV·nm (CODATA)
EV_IN_J = 1.602176634e-19  # exact since the 2019 SI
MJ_PER_CM2_IN_J_PER_NM2 = 1e-17  # 1e-3 J / 1e14 nm²

#: The calculator's presets (huv-physics.js PRESETS): id, label, λ (nm), example NA, medium index, illustrative?
PRESETS = (
    ("g", "g-line 436", 436.0, 0.45, 1.0, True),
    ("i", "i-line 365", 365.0, 0.6, 1.0, True),
    ("krf", "KrF 248", 248.0, 0.8, 1.0, False),
    ("arf", "ArF 193", 193.0, 0.93, 1.0, False),
    ("arfi", "ArF 193 in water", 193.0, 1.35, 1.437, False),
    ("f2", "F₂ 157", 157.0, 0.85, 1.0, True),
    ("euv", "EUV 13.5", 13.5, 0.33, 1.0, False),
    ("hna", "High-NA EUV 13.5", 13.5, 0.55, 1.0, False),
    ("beuv", "Beyond-EUV 6.7", 6.7, 0.55, 1.0, True),
)


def rayleigh(wavelength: float, na: float, k1: float, k2: float, n: float = 1.0) -> dict:
    half_pitch = k1 * wavelength / na
    dof_paraxial = k2 * wavelength / (na * na)
    dof_exact = math.nan
    if na < n:
        s = na / n
        c = math.sqrt(1 - s * s)
        one_minus_cos = s * s / (1 + c)
        dof_exact = k2 * wavelength / (2 * n * one_minus_cos)
    return {"halfPitch": half_pitch, "pitch": 2 * half_pitch, "dofParaxial": dof_paraxial, "dofExact": dof_exact,
            "minHalfPitch": 0.25 * wavelength / na}


def photon_energy(wavelength_nm: float) -> dict:
    ev = HC_EV_NM / wavelength_nm
    return {"eV": ev, "J": ev * EV_IN_J}


def shot_noise(wavelength: float, dose: float, area: float, absorbed: float = 1.0) -> dict:
    energy = photon_energy(wavelength)
    per_nm2 = dose * MJ_PER_CM2_IN_J_PER_NM2 / energy["J"]
    count = per_nm2 * area * absorbed
    return {"perNm2": per_nm2, "count": count, "relSigma": 1 / math.sqrt(count) if count > 0 else math.inf}


def mask_coefficient(m: int, cd: float, pitch: float) -> float:
    if m == 0:
        return 1 - cd / pitch
    return -math.sin(math.pi * m * cd / pitch) / (math.pi * m)


def source_points(sigma_in: float, sigma_out: float, count: int) -> list[float]:
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
        pts += [a + (j + 0.5) * width / per for j in range(per)]
    return pts


def aerial_model(wavelength: float, na: float, pitch: float, cd: float, *, sigma_in: float = 0.0, sigma_out: float = 0.0,
                 source_count: int = 41, defocus: float = 0.0, n: float = 1.0):
    """``at(x)`` of the partially coherent 1-D scalar Abbe image (clear field = 1)."""
    fc = na / wavelength
    sources = source_points(sigma_in, sigma_out, source_count)
    s_max = max(abs(sigma_in), abs(sigma_out))
    m_max = int(math.floor((1 + s_max) * fc * pitch)) + 1
    tol = 1e-12 * fc
    k_medium = n / wavelength
    fields = []
    for s in sources:
        shift = s * fc
        orders = []
        for m in range(-m_max, m_max + 1):
            f = m / pitch + shift
            if abs(f) > fc + tol:
                continue
            c = mask_coefficient(m, cd, pitch)
            phase = 0.0 if defocus == 0 else 2 * math.pi * defocus * (math.sqrt(k_medium * k_medium - f * f) - k_medium)
            orders.append((m, c * cmath.exp(1j * phase)))
        fields.append(orders)

    def at(x: float) -> float:
        total = 0.0
        for orders in fields:
            amp = sum(c * cmath.exp(2j * math.pi * m * x / pitch) for m, c in orders)
            total += abs(amp) ** 2
        return total / len(fields)

    on_axis = [m for m in range(-m_max, m_max + 1) if abs(m / pitch) <= fc + tol]
    return {"at": at, "ordersOnAxis": on_axis, "sourceCount": len(sources)}


def contrast_of(model, pitch: float, samples: int = 801) -> float:
    ys = [model["at"](-pitch / 2 + pitch * i / (samples - 1)) for i in range(samples)]
    imax, imin = max(ys), min(ys)
    return (imax - imin) / (imax + imin) if imax + imin > 0 else 0.0


#: The aerial toy's opening state (playground.js: preset arf → pitch round(2·0.45·λ/NA) = 187, cd 94, σ 0.7, threshold 0.3)
AERIAL_DEFAULT = dict(wavelength=193.0, na=0.93, pitch=187.0, cd=94.0, sigma_out=0.7)
AERIAL_THRESHOLD = 0.3


def check() -> list[str]:
    """The three sample points physics.test.mjs pins (plus the opening-state contrast G0b recorded)."""
    errs: list[str] = []

    def close(name, got, want, tol=1e-9):
        if not (abs(got - want) <= tol * max(1.0, abs(want))):
            errs.append(f"{name}: {got!r} != pinned {want!r}")

    close("rayleigh EUV NA 0.33 dofExact", rayleigh(13.5, 0.33, 0.3, 1)["dofExact"], 120.49468586843875)
    close("rayleigh EUV NA 0.33 dofParaxial", rayleigh(13.5, 0.33, 0.3, 1)["dofParaxial"], 123.96694214876032)
    close("rayleigh ArFi dofExact", rayleigh(193, 1.35, 0.3, 1, n=1.44)["dofExact"], 102.77971659323931)
    sn = shot_noise(13.5, 30, 400)
    close("shotnoise perNm2", sn["perNm2"], 20.388172991992686)
    close("shotnoise count", sn["count"], 8155.269196797074)
    close("shotnoise relSigma", sn["relSigma"], 0.011073396464743734)
    close("photonEnergy eV", photon_energy(13.5)["eV"], 91.84014296296297)
    m = aerial_model(193, 0.93, 300, 150)
    if m["ordersOnAxis"] != [-1, 0, 1]:
        errs.append(f"coherent three-beam orders: {m['ordersOnAxis']}")
    close("aerial at(0)", m["at"](0), 0.018664962201769754)
    close("aerial at(75)", m["at"](75), 0.25)
    close("aerial at(150)", m["at"](150), 1.2919045069369328)
    d = AERIAL_DEFAULT
    close("opening-state contrast", contrast_of(aerial_model(d["wavelength"], d["na"], d["pitch"], d["cd"], sigma_out=d["sigma_out"]), d["pitch"]),
          0.7923451, 2e-7)
    return errs


# ---------------------------------------------------------------------------
# A small chart grammar on top of svgkit
# ---------------------------------------------------------------------------

SERIES = {"436": "yellow", "365": "magenta", "248": "orange", "193": "blue", "193i": "aqua", "157": "green", "13.5": "violet",
          "6.7": "red"}


class Chart:
    def __init__(self, svg: K.Svg, x: float, y: float, w: float, h: float, xlim, ylim, *, xlog=False, ylog=False):
        self.s, self.x, self.y, self.w, self.h = svg, x, y, w, h
        self.xlim, self.ylim, self.xlog, self.ylog = xlim, ylim, xlog, ylog

    def _t(self, v, lim, log):
        a, b = (math.log10(lim[0]), math.log10(lim[1])) if log else lim
        vv = math.log10(v) if log else v
        return (vv - a) / (b - a)

    def px(self, v):
        return self.x + self._t(v, self.xlim, self.xlog) * self.w

    def py(self, v):
        return self.y + self.h - self._t(v, self.ylim, self.ylog) * self.h

    def inside(self, xv, yv):
        return self.xlim[0] <= xv <= self.xlim[1] and self.ylim[0] <= yv <= self.ylim[1]

    def frame(self, xticks, yticks, xlabel, ylabel, *, xfmt=str, yfmt=str, title=None):
        s = self.s
        for v in yticks:
            yy = self.py(v)
            s.line(self.x, yy, self.x + self.w, yy, "grid", 1.0)
            s.text(self.x - 6, yy + 4, yfmt(v), size=K.SMALL, anchor="end", color="ink2")
        for v in xticks:
            xx = self.px(v)
            s.line(xx, self.y, xx, self.y + self.h, "grid", 1.0)
            s.text(xx, self.y + self.h + 16, xfmt(v), size=K.SMALL, color="ink2")
        s.rect(self.x, self.y, self.w, self.h, fill="none", stroke="axis", sw=1.2)
        s.text(self.x + self.w / 2, self.y + self.h + 34, xlabel, size=K.LABEL, color="ink2")
        s.text(self.x - 44, self.y + self.h / 2, ylabel, size=K.LABEL, color="ink2", rotate=-90)
        if title:
            s.text(self.x, self.y - 10, title, size=K.BASE, anchor="start", color="ink", weight=600)

    def curve(self, pts, color, *, dash=None, sw=2.0, opacity=None):
        pts = [(self.px(a), self.py(b)) for a, b in pts if self.inside(a, b)]
        if len(pts) > 1:
            self.s.polyline(pts, color, sw, dash=dash, opacity=opacity)

    def dot(self, xv, yv, color, *, hollow=False, r=4.5):
        if not self.inside(xv, yv):
            return
        if hollow:
            self.s.circle(self.px(xv), self.py(yv), r, fill="surface", stroke=color, sw=1.8)
        else:
            self.s.circle(self.px(xv), self.py(yv), r, fill=color, stroke="surface", sw=1.5)


def _nm(v):
    return f"{v:g}"


def _frange(a, b, n):
    return [a + (b - a) * i / (n - 1) for i in range(n)]


def _lrange(a, b, n):
    return [10 ** v for v in _frange(math.log10(a), math.log10(b), n)]


# ---------------------------------------------------------------------------
# R3a · Rayleigh resolution and depth of focus
# ---------------------------------------------------------------------------

K1, K2 = 0.3, 1.0
RAY_FAMILIES = (  # (key, label, λ, n)
    ("248", "KrF 248 nm", 248.0, 1.0), ("193", "ArF 193 nm, dry", 193.0, 1.0), ("193i", "ArF 193 nm, water n 1.437", 193.0, 1.437),
    ("13.5", "EUV 13.5 nm", 13.5, 1.0),
)
RAY_ALT = ("Two charts against numerical aperture from 0.2 to 1.45: the half-pitch k₁λ/NA at k₁ = 0.3 and the depth of "
           "focus k₂λ/NA² at k₂ = 1 (paraxial dashed, exact solid) for KrF 248 nm, ArF 193 nm dry and in water and EUV "
           "13.5 nm, the calculator's presets as dots; EUV at NA 0.33 gives 12.3 nm and 120 nm.")
RAY_CAPTION = ("<strong>Resolution and depth of focus against NA.</strong> Half-pitch k₁λ/NA at k₁ = 0.3 (left) and depth of "
               "focus at k₂ = 1 (right: paraxial k₂λ/NA² dashed, the exact quarter-wave form solid, ending where NA reaches "
               "the index n) for KrF 248 nm, ArF 193 nm dry and in water (n 1.437) and EUV 13.5 nm; dots are the "
               "calculator's presets, hollow where the NA is illustrative. " + TRAILER)


def draw_rayleigh(theme: str) -> str:
    s = K.Svg(900, 420, theme, "Resolution and depth of focus against NA", RAY_ALT, generator=GENERATOR)
    nas = _frange(0.2, 1.45, 126)
    left = Chart(s, 70, 44, 350, 280, (0.2, 1.45), (3, 500), ylog=True)
    right = Chart(s, 520, 44, 350, 280, (0.2, 1.45), (10, 20000), ylog=True)
    left.frame([0.2, 0.5, 0.8, 1.1, 1.4], [3, 10, 30, 100, 300], "numerical aperture NA", "half-pitch k₁λ/NA (nm), k₁ = 0.3",
               title="Resolution")
    right.frame([0.2, 0.5, 0.8, 1.1, 1.4], [10, 100, 1000, 10000], "numerical aperture NA", "depth of focus (nm), k₂ = 1",
                title="Depth of focus")
    seen = set()
    for key, _label, lam, n in RAY_FAMILIES:
        col = SERIES[key]
        if lam not in seen:  # R does not depend on n: one resolution curve per wavelength
            seen.add(lam)
            left.curve([(na, rayleigh(lam, na, K1, K2, n)["halfPitch"]) for na in nas], col)
        right.curve([(na, rayleigh(lam, na, K1, K2, n)["dofParaxial"]) for na in nas], col, dash="5 4", sw=1.4, opacity=0.8)
        exact = [(na, rayleigh(lam, na, K1, K2, n)["dofExact"]) for na in nas if na < n]
        right.curve(exact, col)
        if exact:
            right.dot(exact[-1][0], exact[-1][1], col, hollow=True, r=3)
    plotted = {k for k, _l, _lam, _n in RAY_FAMILIES}
    for pid, label, lam, na, n, illustrative in PRESETS:
        key = "193i" if pid == "arfi" else f"{lam:g}"
        col = SERIES[key] if key in plotted else "muted"  # a preset outside the drawn families: grey
        r = rayleigh(lam, na, K1, K2, n)
        left.dot(na, r["halfPitch"], col, hollow=illustrative)
        if not math.isnan(r["dofExact"]):
            right.dot(na, r["dofExact"], col, hollow=illustrative)
    # the EUV preset, annotated with the calculator's own numbers
    euv = rayleigh(13.5, 0.33, K1, K2)
    s.text(left.px(0.33) + 10, left.py(euv["halfPitch"]) + 4, f"EUV NA 0.33: {euv['halfPitch']:.1f} nm", size=K.SMALL,
           anchor="start", color="ink2", halo=True)
    s.text(right.px(0.33) + 10, right.py(euv["dofExact"]) + 4, f"EUV NA 0.33: {euv['dofExact']:.0f} nm", size=K.SMALL,
           anchor="start", color="ink2", halo=True)
    s.text(right.px(1.437) - 6, right.py(rayleigh(193, 1.3, K1, K2, 1.437)["dofExact"]) - 10, "ends at NA = n", size=K.SMALL,
           anchor="end", color="muted", halo=True)
    items = [(SERIES[k], lab, "line") for k, lab, _l, _n in RAY_FAMILIES]
    s.swatch_legend(70, 372, items, size=K.SMALL, gap=18)
    s.swatch_legend(70, 400, [("ink2", "paraxial k₂λ/NA²", "dash"), ("ink2", "exact (quarter-wave), ends at NA = n", "line"),
                              ("ink2", "preset", "dot"), ("ink2", "preset with an illustrative NA (grey: other λ)", "hollow")], size=K.SMALL, gap=18)
    return s.render()


# ---------------------------------------------------------------------------
# R3b · photon shot noise
# ---------------------------------------------------------------------------

SN_DOSE = 30.0  # mJ/cm², the calculator's default
SN_FAMILIES = (("436", "g-line 436 nm", 436.0), ("248", "KrF 248 nm", 248.0), ("193", "ArF 193 nm", 193.0),
               ("13.5", "EUV 13.5 nm", 13.5), ("6.7", "Beyond-EUV 6.7 nm", 6.7))
_SN_EUV20 = shot_noise(13.5, SN_DOSE, 400.0)
SN_ALT = (f"Two log–log charts against feature size from 5 to 100 nm at {SN_DOSE:g} mJ/cm²: incident photons in a square "
          "feature and the Poisson 1σ fluctuation 1/√N in percent for g-line 436, KrF 248, ArF 193, EUV 13.5 and 6.7 nm; "
          f"a 20 nm square at 13.5 nm holds {_SN_EUV20['count']:.0f} photons, a 1σ of {100 * _SN_EUV20['relSigma']:.2f} %.")
SN_CAPTION = (f"<strong>Photons per feature and shot noise.</strong> Incident photons × f<sub>abs</sub> (here 1) in a square "
              f"feature at {SN_DOSE:g} mJ/cm² (left) and the Poisson 1σ fluctuation 1/√N (right) against feature size: the "
              f"count falls with λ at fixed dose, so a 20 nm square holds {_SN_EUV20['count']:.0f} photons at 13.5 nm, a 1σ of "
              f"{100 * _SN_EUV20['relSigma']:.2f} %. " + TRAILER)


def draw_shotnoise(theme: str) -> str:
    s = K.Svg(900, 420, theme, "Photons per feature and shot noise", SN_ALT, generator=GENERATOR)
    sizes = _lrange(5, 100, 61)
    left = Chart(s, 70, 44, 350, 280, (5, 100), (10, 1e7), xlog=True, ylog=True)
    right = Chart(s, 520, 44, 350, 280, (5, 100), (0.03, 30), xlog=True, ylog=True)

    def pow10(v):
        e = round(math.log10(v))
        return "1" if e == 0 else ("10" if e == 1 else f"10^{e}")

    left.frame([5, 10, 20, 50, 100], [10, 100, 1000, 1e4, 1e5, 1e6, 1e7], "feature size (nm, square)",
               f"incident photons at {SN_DOSE:g} mJ/cm²", yfmt=pow10, title="Photons per feature")
    right.frame([5, 10, 20, 50, 100], [0.03, 0.1, 0.3, 1, 3, 10, 30], "feature size (nm, square)", "1σ fluctuation 1/√N (%)",
                yfmt=_nm, title="Shot noise")
    for key, _label, lam in SN_FAMILIES:
        col = SERIES[key]
        pts = [(d, shot_noise(lam, SN_DOSE, d * d)) for d in sizes]
        left.curve([(d, r["count"]) for d, r in pts], col)
        right.curve([(d, 100 * r["relSigma"]) for d, r in pts], col)
    for pct in (1.0, 5.0):
        s.line(right.x, right.py(pct), right.x + right.w, right.py(pct), "muted", 1.0, dash="3 3")
        s.text(right.x + right.w - 4, right.py(pct) - 5, f"{pct:g} %", size=K.SMALL, anchor="end", color="muted", halo=True)
    e = _SN_EUV20
    left.dot(20, e["count"], SERIES["13.5"])
    right.dot(20, 100 * e["relSigma"], SERIES["13.5"])
    s.text(left.px(20) + 10, left.py(e["count"]) + 14, f"20 nm at 13.5 nm: {e['count']:.0f} photons", size=K.SMALL,
           anchor="start", color="ink2", halo=True)
    s.text(right.px(20) + 10, right.py(100 * e["relSigma"]) - 8, f"1σ = {100 * e['relSigma']:.2f} %", size=K.SMALL,
           anchor="start", color="ink2", halo=True)
    s.swatch_legend(70, 372, [(SERIES[k], lab, "line") for k, lab, _l in SN_FAMILIES], size=K.SMALL, gap=18)
    s.text(70, 400, "Poisson statistics of the incident photons only (f_abs = 1); no resist chemistry, no blur, no acid shot noise.",
           size=K.SMALL, anchor="start", color="muted")
    return s.render()


# ---------------------------------------------------------------------------
# R3c · the aerial toy's opening state
# ---------------------------------------------------------------------------

_AD = AERIAL_DEFAULT
_AERIAL_MODEL = aerial_model(_AD["wavelength"], _AD["na"], _AD["pitch"], _AD["cd"], sigma_out=_AD["sigma_out"])
_AERIAL_CONTRAST = contrast_of(_AERIAL_MODEL, _AD["pitch"])
_ORDER_POS = _AD["wavelength"] / (_AD["pitch"] * _AD["na"])  # first orders at m·λ/(p·NA) in pupil units
AERIAL_ALT = (f"The aerial toy's opening state, ArF 193 nm, NA 0.93, σ 0.7, 94 nm lines on a 187 nm pitch: left the pupil "
              f"with the ±1 orders at ±{_ORDER_POS:.2f} pupil units, outside the pupil on axis but inside the (1+σ) support; "
              f"right the 1-D partially coherent image over two periods, contrast {_AERIAL_CONTRAST:.3f}, with the 0.3 threshold.")
AERIAL_CAPTION = (f"<strong>The aerial toy's opening state.</strong> ArF 193 nm, NA 0.93, σ 0.7, 94 nm lines on a 187 nm pitch: "
                  f"the pupil with the orders at m·λ/(p·NA) = ±{_ORDER_POS:.2f} — outside the pupil for the on-axis source "
                  "point, inside the (1+σ) support, so they pass for part of the source — and the 1-D scalar Abbe image over "
                  f"two periods, contrast {_AERIAL_CONTRAST:.3f}, with the 0.3 resist threshold. " + TRAILER)


def draw_aerial(theme: str) -> str:
    s = K.Svg(900, 400, theme, "The aerial toy's opening state", AERIAL_ALT, generator=GENERATOR)
    d = AERIAL_DEFAULT
    # left: the D5 pupil
    cx, cy, r = 190, 200, 80
    s.text(40, 34, "Pupil and diffraction orders", size=K.BASE, anchor="start", color="ink", weight=600)
    K.support_circle(s, cx, cy, r, d["sigma_out"], label=True)
    K.pupil(s, cx, cy, r, fill="conventional", sigma=d["sigma_out"], label=None)
    K.orders(s, cx, cy, r, [(0, 0.0, 0.0), (1, _ORDER_POS, 0.0), (-1, -_ORDER_POS, 0.0)], support=1 + d["sigma_out"])
    s.text(cx, cy - r - 8, "NA/λ", size=K.SMALL, color="ink2", halo=True)
    s.text(cx, cy + d["sigma_out"] * r - 8, "σ 0.7", size=K.SMALL, color="ink2", halo=True)
    s.text(40, 362, f"λ {d['wavelength']:g} nm · NA {d['na']:g} · σ {d['sigma_out']:g} (conventional)", size=K.SMALL, anchor="start", color="ink2")
    s.text(40, 382, f"p {d['pitch']:g} nm, w {d['cd']:g} nm → orders at ±λ/(p·NA) = ±{_ORDER_POS:.2f}", size=K.SMALL,
           anchor="start", color="ink2")
    # right: the 1-D image over two periods
    p = d["pitch"]
    ch = Chart(s, 470, 44, 400, 270, (-p, p), (0.0, 1.3))
    ch.frame([-p, -p / 2, 0, p / 2, p], [0, 0.25, 0.5, 0.75, 1.0, 1.25], "x (nm)", "intensity (clear field = 1)",
             xfmt=lambda v: f"{v:g}", yfmt=lambda v: f"{v:g}", title="1-D aerial image, in focus")
    for centre in (-p, 0.0, p):  # the opaque lines of the mask
        x0, x1 = max(-p, centre - d["cd"] / 2), min(p, centre + d["cd"] / 2)
        s.rect(ch.px(x0), ch.y, ch.px(x1) - ch.px(x0), ch.h, fill=s.tint("ink", 0.07), stroke="none")
    s.line(ch.x, ch.py(1.0), ch.x + ch.w, ch.py(1.0), "muted", 1.0, dash="3 3")
    s.line(ch.x, ch.py(AERIAL_THRESHOLD), ch.x + ch.w, ch.py(AERIAL_THRESHOLD), "orange", 1.2, dash="5 3")
    s.text(ch.x + ch.w - 4, ch.py(AERIAL_THRESHOLD) - 5, f"threshold {AERIAL_THRESHOLD:g}", size=K.SMALL, anchor="end",
           color="orange", halo=True)
    xs = _frange(-p, p, 241)
    ch.curve([(x, _AERIAL_MODEL["at"](x)) for x in xs], "blue", sw=2.2)
    s.text(ch.x + 6, ch.y + 18, f"contrast {_AERIAL_CONTRAST:.3f} · {_AERIAL_MODEL['sourceCount']} source points", size=K.SMALL,
           anchor="start", color="ink2", halo=True)
    s.text(ch.x + ch.w / 2, 362, "shaded: the opaque lines of the mask · dashed: clear field", size=K.SMALL, color="muted")
    s.text(ch.x + ch.w / 2, 382, "scalar, thin mask, 41 midpoint source samples — the calculator's own sum", size=K.SMALL,
           color="muted")
    return s.render()


# ---------------------------------------------------------------------------
# Registration (at import time; drawing only under __main__ / through the generators)
# ---------------------------------------------------------------------------


def _gen(name, draw):
    def run():
        errs = check()
        if errs:
            raise AssertionError("playground pins: " + "; ".join(errs))
        return K.write_pair(name, draw, OUT)
    return run


gen_rayleigh = _gen("playground-rayleigh", draw_rayleigh)
gen_shotnoise = _gen("playground-shotnoise", draw_shotnoise)
gen_aerial = _gen("playground-aerial", draw_aerial)

registry.register(registry.AssetSpec(
    name="playground-rayleigh", kind="schematic", group="playground", subdir="playground", fmt="svg",
    alt=RAY_ALT, caption=RAY_CAPTION, pages=("playground/index.md", "nodes/litho-math.md"),
    generator=gen_rayleigh, platform_independent=True,
), module="make_playground_figures.py")
registry.register(registry.AssetSpec(
    name="playground-shotnoise", kind="schematic", group="playground", subdir="playground", fmt="svg",
    alt=SN_ALT, caption=SN_CAPTION, pages=("playground/index.md", "nodes/litho-math.md", "nodes/7-and-5nm.md"),
    generator=gen_shotnoise, platform_independent=True,
), module="make_playground_figures.py")
registry.register(registry.AssetSpec(
    name="playground-aerial", kind="schematic", group="playground", subdir="playground", fmt="svg",
    alt=AERIAL_ALT, caption=AERIAL_CAPTION, pages=("playground/index.md",),
    generator=gen_aerial, platform_independent=True,
), module="make_playground_figures.py")

GENERATORS = (("playground-rayleigh", gen_rayleigh), ("playground-shotnoise", gen_shotnoise), ("playground-aerial", gen_aerial))


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    errs = check()
    for e in errs:
        print("error:", e)
    print(f"playground pins: {len(errs)} error(s)")
    if "--check" in argv or errs:
        return 1 if errs else 0
    only = [a for a in argv if not a.startswith("--")]
    for name, fn in GENERATORS:
        if only and not any(o in name for o in only):
            continue
        for p in fn():
            print(f"wrote {registry.rel_to_repo(p)} ({p.stat().st_size / 1024:.1f} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
