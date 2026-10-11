"""Materials figures (DESIGN §5.3 P10, P11; group ``materials``).

* ``materials-bragg-anim`` — a two-layer CSS-animated SVG (svgkit) whose curves are the engine's
  ``MultilayerMirror`` reflectance; the stack and phasor panels are schematic and say so;
* ``materials-optical-constants`` — δ, β from the embedded CXRO/Henke tables and n(λ) of the VUV
  crystals from the database's Sellmeier fits, with the derived crop ``materials-optical-constants-vuv``
  for ``history/157nm-detour.md``;
* ``materials-coverage-bands`` — the range policy *probed* from ``refractive_index`` (a band where it
  returns a value, a gap where it raises) and asserted against the page's range table in ``compute()``.

Every number is computed in ``compute()`` so ``make_all.py --verify`` runs the asserts without drawing.
"""

from __future__ import annotations

import math

import numpy as np
from matplotlib import pyplot as plt
from matplotlib.lines import Line2D

import style as S
import svgkit as K
from common import FigureSpec, native, register

EV_NM = 1239.84193          # hc in eV·nm
GENERATOR = "docs/figures/sim/fig_materials.py"

# ---------------------------------------------------------------------------
# P10. The Bragg animation: eight bilayers, their phasor sum, the engine's R(λ) and R(θ)
# ---------------------------------------------------------------------------

MOSI = dict(periods=40, period_nm=6.9, gamma=0.4)
SHOWN_BILAYERS = 8
BRAGG_ANGLES = (0.0, 5.0, 10.0, 15.0, 20.0)
#: materials.md "Multilayer mirrors" table — the page's own numbers, asserted against the engine in compute()
BRAGG_DOC = dict(peak_nm=13.48, peak_r=0.730, fwhm_nm=0.63,
                 s=(72.9, 72.2, 53.2, 15.8, 4.2), p=(72.9, 71.5, 44.0, 10.2, 1.5))
T_A, T_B, T_HOLD, T_BACK = 6.0, 5.0, 1.5, 0.5     # λ sweep · θ sweep · hold at 20° · return to the resting state
T_TOTAL = T_A + T_B + T_HOLD + T_BACK
DT = 0.25


def bragg_compute(fast: bool):
    huv = native()
    m40 = huv.MultilayerMirror.mo_si(MOSI["periods"], MOSI["period_nm"], MOSI["gamma"])
    m8 = huv.MultilayerMirror.mo_si(SHOWN_BILAYERS, MOSI["period_nm"], MOSI["gamma"])
    lam = np.linspace(12.5, 14.5, 81 if fast else 201)
    r_lam = {th: np.asarray(m40.reflectance_curve(lam, angle_deg=th, polarization="s")) for th in BRAGG_ANGLES}
    r_lam_8 = np.asarray(m8.reflectance_curve(lam, angle_deg=0.0, polarization="s"))
    peak_nm, peak_r = m40.peak(12.8, 14.2)
    fwhm = m40.bandwidth_fwhm_nm(12.8, 14.2)
    theta = np.linspace(0.0, 25.0, 51 if fast else 126)
    r_s = np.asarray(m40.reflectance_vs_angle(13.5, theta, "s"))
    r_p = np.asarray(m40.reflectance_vs_angle(13.5, theta, "p"))
    s_marks = [float(m40.reflectance(13.5, th, "s")) for th in BRAGG_ANGLES]
    p_marks = [float(m40.reflectance(13.5, th, "p")) for th in BRAGG_ANGLES]
    # the mean index of a bilayer at 13.5 nm (for the refraction term of the schematic phase step)
    d_mo, _ = huv.xray_optical_constants("Mo", [13.5])
    d_si, _ = huv.xray_optical_constants("Si", [13.5])
    n_bar = 1.0 - (MOSI["gamma"] * float(np.asarray(d_mo)[0]) + (1 - MOSI["gamma"]) * float(np.asarray(d_si)[0]))
    # --- physical asserts (make_all.py --verify): the engine reproduces the page's table ---
    assert abs(peak_nm - BRAGG_DOC["peak_nm"]) < 0.015, f"Mo/Si peak at {peak_nm:.3f} nm, page says {BRAGG_DOC['peak_nm']}"
    assert abs(peak_r - BRAGG_DOC["peak_r"]) < 0.002, f"Mo/Si peak {peak_r:.4f}, page says {BRAGG_DOC['peak_r']}"
    assert abs(fwhm - BRAGG_DOC["fwhm_nm"]) < 0.01, f"FWHM {fwhm:.3f} nm, page says {BRAGG_DOC['fwhm_nm']}"
    for got, want in zip(s_marks, BRAGG_DOC["s"]):
        assert abs(100 * got - want) < 0.15, f"R_s at 13.5 nm: {100 * got:.1f} % vs the page's {want}"
    for got, want in zip(p_marks, BRAGG_DOC["p"]):
        assert abs(100 * got - want) < 0.15, f"R_p at 13.5 nm: {100 * got:.1f} % vs the page's {want}"
    assert r_lam_8.max() < peak_r, "eight bilayers must reflect less than forty"
    return {"lam": lam, "r_lam": r_lam, "r_lam_8": r_lam_8, "peak": (float(peak_nm), float(peak_r)), "fwhm": float(fwhm),
            "theta": theta, "r_s": r_s, "r_p": r_p, "n_bar": n_bar, "peak_8": float(r_lam_8.max())}


def _timeline():
    """(t, λ, θ) samples of the 13 s timeline: λ 12.5 → 14.5 → 13.5 at θ = 0, then θ 0 → 20° at 13.5, hold, return."""
    out = []
    t = 0.0
    while t <= T_TOTAL + 1e-9:
        if t <= T_A - 0.5:
            lam, th = 12.5 + 2.0 * t / (T_A - 0.5), 0.0
        elif t <= T_A:
            lam, th = 14.5 - 1.0 * (t - (T_A - 0.5)) / 0.5, 0.0
        elif t <= T_A + T_B:
            lam, th = 13.5, 20.0 * (t - T_A) / T_B
        elif t <= T_A + T_B + T_HOLD:
            lam, th = 13.5, 20.0
        else:
            lam, th = 13.5, 20.0 * (1 - (t - T_A - T_B - T_HOLD) / T_BACK)
        out.append((round(t, 3), lam, th))
        t += DT
    return out


def _phase_step(lam: float, theta_deg: float, peak_nm: float, n_bar: float) -> float:
    """Schematic phase step per bilayer: 2π at the computed peak, scaled by λ_peak/λ and the refraction term q̄(θ)/q̄(0)."""
    q0 = n_bar
    q = math.sqrt(max(n_bar ** 2 - math.sin(math.radians(theta_deg)) ** 2, 1e-9))
    return 2 * math.pi * (peak_nm / lam) * (q / q0)


class _Chart:
    """A tiny linear chart frame inside the SVG (axes, ticks, gridlines, a data → px map)."""

    def __init__(self, svg: K.Svg, x0, y0, w, h, xlim, ylim):
        self.s, self.x0, self.y0, self.w, self.h = svg, x0, y0, w, h
        self.xlim, self.ylim = xlim, ylim

    def px(self, x, y):
        return (self.x0 + (x - self.xlim[0]) / (self.xlim[1] - self.xlim[0]) * self.w,
                self.y0 + self.h - (y - self.ylim[0]) / (self.ylim[1] - self.ylim[0]) * self.h)

    def frame(self, xticks, yticks, xlabel, ylabel, xfmt=lambda v: f"{v:g}", yfmt=lambda v: f"{v:g}"):
        s = self.s
        for v in yticks:
            _, y = self.px(self.xlim[0], v)
            s.line(self.x0, y, self.x0 + self.w, y, "grid", 1.0)
            s.text(self.x0 - 6, y + 4, yfmt(v), size=K.SMALL, anchor="end", color="ink2")
        for v in xticks:
            x, _ = self.px(v, self.ylim[0])
            s.line(x, self.y0 + self.h, x, self.y0 + self.h + 4, "axis", 1.0)
            s.text(x, self.y0 + self.h + 16, xfmt(v), size=K.SMALL, anchor="middle", color="ink2")
        s.line(self.x0, self.y0, self.x0, self.y0 + self.h, "axis", 1.0)
        s.line(self.x0, self.y0 + self.h, self.x0 + self.w, self.y0 + self.h, "axis", 1.0)
        s.text(self.x0 + self.w / 2, self.y0 + self.h + 30, xlabel, size=K.SMALL, anchor="middle", color="ink2")
        s.text(self.x0 - 34, self.y0 + self.h / 2, ylabel, size=K.SMALL, anchor="middle", color="ink2", rotate=-90)

    def curve(self, xs, ys, color, sw=2.0, cls=None, dash=None, opacity=None):
        pts = [self.px(float(x), float(y)) for x, y in zip(xs, ys)]
        self.s.polyline(pts, color=color, sw=sw, cls=cls, dash=dash, opacity=opacity)


def bragg_draw(d, th: S.Theme) -> str:
    theme = th.name
    W, H = 960, 430
    s = K.Svg(W, H, theme, "Why a 13.5 nm mirror is a Bragg stack", BRAGG_ALT, generator=GENERATOR)
    anim = K.Anim(s, loops=3)
    peak_nm, peak_r = d["peak"]
    n_bar = d["n_bar"]
    timeline = _timeline()
    # ---------------------------------------------------------------- stack panel (schematic)
    sx, sy = 40, 60            # top-left of the stack
    stack_w = 220
    d_mo, d_si = 11.0, 17.0    # Γ = 0.4 of a 28-unit bilayer
    bilayer_h = d_mo + d_si
    x_hit = sx + 118           # where the incident ray meets the top interface
    y_top = sy + 90
    s.eyebrow(sx, sy - 24, "Eight of 40 Mo/Si bilayers")
    s.text(sx, sy - 6, f"d = {MOSI['period_nm']} nm, Γ(Mo) = {MOSI['gamma']}; schematic,", size=K.SMALL, anchor="start", color="ink2")
    s.text(sx, sy + 10, "one partial reflection per bilayer", size=K.SMALL, anchor="start", color="ink2")
    s.rect(sx, y_top - 60, stack_w, 60, fill="none", stroke="none")
    s.text(sx + 6, y_top - 10, "vacuum", size=K.SMALL, anchor="start", color="muted")
    for k in range(SHOWN_BILAYERS):
        y = y_top + k * bilayer_h
        s.rect(sx, y, stack_w, d_mo, "mirror", sw=0.8)
        s.rect(sx, y + d_mo, stack_w, d_si, "spacer", sw=0.8)
    y_bot = y_top + SHOWN_BILAYERS * bilayer_h
    s.rect(sx, y_bot, stack_w, 14, "substrate", sw=0.8, opacity=0.6)
    s.text(sx + stack_w / 2, y_bot + 30, "⋯ 32 more bilayers, SiO₂ substrate", size=K.SMALL, anchor="middle", color="muted")
    s.text(sx + stack_w + 8, y_top + d_mo / 2 + 4, "Mo", size=K.SMALL, anchor="start", color="ink2")
    s.text(sx + stack_w + 8, y_top + d_mo + d_si / 2 + 4, "Si", size=K.SMALL, anchor="start", color="ink2")
    ray_len = 80.0
    reflections = [(x_hit, y_top + k * bilayer_h) for k in range(SHOWN_BILAYERS)]

    def rays(cls_prefix: str | None):
        # the incident ray (comes from above-left when θ > 0), the transmitted path, one reflected ray per bilayer;
        # animated rays sit inside a static translate group at their pivot so the keyframes hold a bare rotate()
        if cls_prefix is None:
            s.line(x_hit, y_top - ray_len, x_hit, y_top, "brand:violet600", 2.2, arrow="end")
            s.line(x_hit, y_top, x_hit, y_bot + 10, "brand:violet600", 1.0, dash="3 3", opacity=0.6)
            for k, (px_, py_) in enumerate(reflections):
                s.line(px_, py_, px_, y_top - 70 + 4 * k, "brand:cyan700", 1.4, arrow="end", opacity=0.85)
            return
        # CSS transforms on SVG elements resolve transform-origin in viewport units (transform-box: view-box), so
        # every rotating group names its own pivot and the keyframes hold a bare rotate()
        pivot = lambda x, y: f'style="transform-origin:{K.fmt(x)}px {K.fmt(y)}px"'  # noqa: E731
        with s.group(f"{cls_prefix}in", extra=pivot(x_hit, y_top)):
            s.line(x_hit, y_top - ray_len, x_hit, y_top, "brand:violet600", 2.2, arrow="end")
        with s.group(f"{cls_prefix}tr", extra=pivot(x_hit, y_top)):
            s.line(x_hit, y_top, x_hit, y_bot + 10, "brand:violet600", 1.0, dash="3 3", opacity=0.6)
        for k, (px_, py_) in enumerate(reflections):
            with s.group(f"{cls_prefix}r", extra=pivot(px_, py_)):
                s.line(px_, py_, px_, y_top - 70 + 4 * k, "brand:cyan700", 1.4, arrow="end", opacity=0.85)

    # ---------------------------------------------------------------- phasor panel (schematic)
    cx0, cy0 = 365, 300        # start (tail of phasor 0) when aligned: a horizontal chain
    L = 21.0
    s.eyebrow(330, sy - 24, "Phasor sum of the reflections")
    s.text(330, sy - 6, "equal amplitudes, no absorption; the phase", size=K.SMALL, anchor="start", color="ink2")
    s.text(330, sy + 10, "step per bilayer is scaled to align at the", size=K.SMALL, anchor="start", color="ink2")
    s.text(330, sy + 26, f"computed peak, {peak_nm:.2f} nm", size=K.SMALL, anchor="start", color="ink2")
    s.circle(cx0 + 4 * L, cy0, 4 * L, fill="none", stroke="grid", sw=1.0)
    s.circle(cx0, cy0, 3, fill="ink2", stroke="none")

    def phasor_chain(dphi: float):
        """(tail, angle°) of each phasor and the tip of the chain for one phase step (math angle → SVG y down)."""
        tails, tx, ty = [], cx0, cy0
        for k in range(SHOWN_BILAYERS):
            ang = k * dphi
            tails.append((tx, ty, -math.degrees(ang)))
            tx += L * math.cos(ang)
            ty -= L * math.sin(ang)
        return tails, (tx, ty)

    def phasors(cls_prefix: str | None, dphi: float):
        """The chain of eight phasors and the resultant: static (still) or class-driven (animated)."""
        tails, tip = phasor_chain(dphi)
        for k, (tx, ty, ang) in enumerate(tails):
            cls = f"{cls_prefix}p{k}" if cls_prefix else None
            with s.group(cls, transform=None if cls_prefix else f"translate({K.fmt(tx)} {K.fmt(ty)}) rotate({K.fmt(ang)})"):
                s.line(0, 0, L - 3, 0, "brand:cyan700", 2.0, arrow="end")
        if not cls_prefix:
            s.line(cx0, cy0, tip[0], tip[1], "brand:violet600", 2.6)
            s.circle(tip[0], tip[1], 5, fill="brand:violet600", stroke="surface", sw=1.5)
        else:
            # a filled 1-unit bar (not a stroked line: round caps would scale with the length) stretched to the tip
            with s.group(f"{cls_prefix}res", extra=f'style="transform-origin:{K.fmt(cx0)}px {K.fmt(cy0)}px"'):
                s.rect(cx0, cy0 - 1.3, 1, 2.6, fill="brand:violet600", stroke="none")
            s.circle(0, 0, 5, fill="brand:violet600", stroke="surface", sw=1.5, cls=f"{cls_prefix}tip")

    # ---------------------------------------------------------------- charts (engine data)
    lam = d["lam"]
    c1 = _Chart(s, 650, 60, 270, 130, (12.5, 14.5), (0.0, 0.8))
    c1.frame([12.5, 13.0, 13.5, 14.0, 14.5], [0.0, 0.2, 0.4, 0.6, 0.8], "wavelength (nm) at θ", "R", yfmt=lambda v: f"{v:.1f}")
    s.eyebrow(650, sy - 24, "Computed reflectance")
    s.text(650, sy - 6, f"Mo/Si {MOSI['periods']} × {MOSI['period_nm']} nm, s-polarized, ideal interfaces", size=K.SMALL, anchor="start", color="ink2")
    c1.curve(lam, d["r_lam_8"], "muted", sw=1.3, dash="4 3")
    for th_ in BRAGG_ANGLES:
        c1.curve(lam, d["r_lam"][th_], "blue", sw=2.0, cls=f"lc{int(th_)}", opacity=(1.0 if th_ == 0 else 0.0))
    s.text(c1.x0 + 6, c1.px(0, 0.66)[1], f"{100 * peak_r:.1f} %", size=K.SMALL, anchor="start", color="ink2")
    s.text(c1.x0 + 6, c1.px(0, 0.56)[1], f"at {peak_nm:.2f} nm", size=K.SMALL, anchor="start", color="ink2")
    s.text(c1.x0 + 6, c1.px(0, 0.46)[1], f"FWHM {d['fwhm']:.2f} nm", size=K.SMALL, anchor="start", color="ink2")
    s.text(c1.px(14.45, d["peak_8"])[0], c1.px(14.45, d["peak_8"])[1] - 8, "8 bilayers alone", size=K.SMALL, anchor="end", color="muted")
    c2 = _Chart(s, 650, 270, 270, 110, (0.0, 25.0), (0.0, 0.8))
    c2.frame([0, 5, 10, 15, 20, 25], [0.0, 0.4, 0.8], "angle of incidence (°) at 13.5 nm", "R", yfmt=lambda v: f"{v:.1f}")
    c2.curve(d["theta"], d["r_s"], "blue", sw=2.0)
    c2.curve(d["theta"], d["r_p"], "orange", sw=2.0)
    s.swatch_legend(800, 290, [("blue", "s (TE)", "line"), ("orange", "p (TM)", "line")], size=K.SMALL, gap=18)
    # marker and readout helpers
    readout_x, readout_y = 330, 122

    # ---------------------------------------------------------------- the animation
    # phasor transforms, ray rotations, markers and readouts, all keyed on the same timeline
    kf_p = [anim.keyframes(f"ph{k}", T_TOTAL) for k in range(SHOWN_BILAYERS)]
    kf_tip = anim.keyframes("tip", T_TOTAL)
    kf_res = anim.keyframes("res", T_TOTAL)
    kf_m1 = anim.keyframes("m1", T_TOTAL)
    kf_ms = anim.keyframes("ms", T_TOTAL)
    kf_mp = anim.keyframes("mp", T_TOTAL)
    kf_in = anim.keyframes("rin", T_TOTAL)
    kf_tr = anim.keyframes("rtr", T_TOTAL)
    kf_r = anim.keyframes("rr", T_TOTAL)           # one rotate() track for all eight reflected rays
    r_s_at = lambda th_: float(np.interp(th_, d["theta"], d["r_s"]))  # noqa: E731
    r_p_at = lambda th_: float(np.interp(th_, d["theta"], d["r_p"]))  # noqa: E731
    nearest_angle = lambda th_: min(BRAGG_ANGLES, key=lambda a: abs(a - th_))  # noqa: E731
    tracks = [*kf_p, kf_tip, kf_res, kf_m1, kf_ms, kf_mp, kf_in, kf_tr, kf_r]
    samples: dict[int, list[tuple[float, str]]] = {id(kf): [] for kf in tracks}

    def put(kf, t, css):
        samples[id(kf)].append((t, css))
    for t, lam_t, th_t in timeline:
        dphi = _phase_step(lam_t, th_t, peak_nm, n_bar)
        tails, tip = phasor_chain(dphi)
        for k, (tx, ty, ang) in enumerate(tails):
            put(kf_p[k], t, f"transform:translate({K.fmt(tx)}px,{K.fmt(ty)}px) rotate({K.fmt(ang)}deg)")
        put(kf_tip, t, f"transform:translate({K.fmt(tip[0])}px,{K.fmt(tip[1])}px)")
        length = math.hypot(tip[0] - cx0, tip[1] - cy0)
        res_ang = math.degrees(math.atan2(tip[1] - cy0, tip[0] - cx0))
        put(kf_res, t, f"transform:rotate({K.fmt(res_ang)}deg) scale({K.fmt(max(length, 0.5))},1)")
        r_here = float(np.interp(lam_t, lam, d["r_lam"][nearest_angle(th_t)]))
        mx, my = c1.px(lam_t, r_here)
        put(kf_m1, t, f"transform:translate({K.fmt(mx)}px,{K.fmt(my)}px)")
        sx_, sy_ = c2.px(th_t, r_s_at(th_t))
        px_, py_ = c2.px(th_t, r_p_at(th_t))
        put(kf_ms, t, f"transform:translate({K.fmt(sx_)}px,{K.fmt(sy_)}px)")
        put(kf_mp, t, f"transform:translate({K.fmt(px_)}px,{K.fmt(py_)}px)")
        put(kf_in, t, f"transform:rotate({K.fmt(-th_t)}deg)")
        put(kf_tr, t, f"transform:rotate({K.fmt(th_t * 1.03)}deg)")
        put(kf_r, t, f"transform:rotate({K.fmt(th_t)}deg)")
    # a keyframe is emitted only where an element's state changes (the ends of every constant run are kept, so the
    # linear interpolation between kept frames reproduces the dense timeline exactly); this keeps the file under the cap
    for kf in tracks:
        seq = samples[id(kf)]
        for i, (t, css) in enumerate(seq):
            if i == 0 or i == len(seq) - 1 or css != seq[i - 1][1] or css != seq[i + 1][1]:
                kf.at(t, css)
    for k in range(SHOWN_BILAYERS):
        anim.animate(f".ap{k}", f"ph{k}", T_TOTAL)
    anim.animate(".ar", "rr", T_TOTAL)
    anim.animate(".atip", "tip", T_TOTAL)
    anim.animate(".ares", "res", T_TOTAL)
    anim.animate(".am1", "m1", T_TOTAL)
    anim.animate(".ams", "ms", T_TOTAL)
    anim.animate(".amp", "mp", T_TOTAL)
    anim.animate(".ain", "rin", T_TOTAL)
    anim.animate(".atr", "rtr", T_TOTAL)
    # the θ-dependent R(λ) curves cross-fade during the θ sweep; the readouts swap by opacity
    for th_ in BRAGG_ANGLES:
        kf = anim.keyframes(f"lc{int(th_)}", T_TOTAL)
        vis = []
        for t, _lam, th_t in timeline:
            vis.append((t, 1.0 if nearest_angle(th_t) == th_ else 0.0))
        for i, (t, v) in enumerate(vis):
            if i == 0 or v != vis[i - 1][1]:
                if i > 0:
                    kf.at(t - 0.05, f"opacity:{vis[i - 1][1]:g}")
                kf.at(t, f"opacity:{v:g}")
        kf.at(T_TOTAL, f"opacity:{vis[-1][1]:g}")
        anim.animate(f".lc{int(th_)}", f"lc{int(th_)}", T_TOTAL)
    readouts = [(j, lam_r, th_r) for j, (lam_r, th_r) in
                enumerate([(12.5 + 0.25 * i, 0.0) for i in range(9)] + [(13.5, a) for a in BRAGG_ANGLES])]

    def current(lam_t, th_t):
        """Index of the readout text that is visible at (λ, θ)."""
        if th_t > 0.01:
            return 9 + BRAGG_ANGLES.index(nearest_angle(th_t))
        return int(round((lam_t - 12.5) / 0.25))
    for j, lam_r, th_r in readouts:
        kf = anim.keyframes(f"ro{j}", T_TOTAL)
        seq = [(t, 1.0 if current(lam_t, th_t) == j else 0.0) for t, lam_t, th_t in timeline]
        for i, (t, v) in enumerate(seq):
            if i == 0 or v != seq[i - 1][1]:
                if i > 0:
                    kf.at(t - 0.05, f"opacity:{seq[i - 1][1]:g}")
                kf.at(t, f"opacity:{v:g}")
        kf.at(T_TOTAL, f"opacity:{seq[-1][1]:g}")
        anim.animate(f".ro{j}", f"ro{j}", T_TOTAL)
    with anim.layer("anim"):
        rays("a")
        phasors("a", _phase_step(12.5, 0.0, peak_nm, n_bar))     # draws the chain, the resultant bar and the tip
        s.circle(0, 0, 5, fill="blue", stroke="surface", sw=1.5, cls="am1")
        s.circle(0, 0, 5, fill="blue", stroke="surface", sw=1.5, cls="ams")
        s.circle(0, 0, 5, fill="orange", stroke="surface", sw=1.5, cls="amp")
        for j, lam_r, th_r in readouts:
            s.text(readout_x, readout_y, f"λ = {lam_r:.2f} nm · θ = {th_r:.0f}°", size=15, anchor="start", color="ink", weight=600,
                   cls=f"ro{j}", opacity=0.0)
    with anim.layer("still"):
        rays(None)
        phasors(None, _phase_step(13.5, 0.0, peak_nm, n_bar))
        mx, my = c1.px(13.5, float(np.interp(13.5, lam, d["r_lam"][0.0])))
        s.circle(mx, my, 5, fill="blue", stroke="surface", sw=1.5)
        px_, py_ = c2.px(0.0, r_p_at(0.0))
        s.circle(px_, py_, 5, fill="orange", stroke="surface", sw=1.5)
        sx_, sy_ = c2.px(0.0, r_s_at(0.0))
        s.circle(sx_, sy_, 5, fill="blue", stroke="surface", sw=1.5)
        s.text(readout_x, readout_y, "λ = 13.50 nm · θ = 0°", size=15, anchor="start", color="ink", weight=600)
    anim.install()
    return s.render()


BRAGG_ALT = ("Animated cross-section of eight Mo/Si bilayers with one reflected ray each, the eight reflections "
             "summed as rotating phasors that line up at the Bragg peak, and the computed reflectance against "
             "wavelength near 13.5 nm and against incidence angle for s and p polarization.")

register(FigureSpec(
    name="materials-bragg-anim",
    group="materials",
    compute=bragg_compute,
    draw=bragg_draw,
    fmt="svg",
    subdir="anim",
    alt=BRAGG_ALT,
    caption="<strong>Why a 13.5 nm mirror is a Bragg stack.</strong> Eight of the 40 Mo/Si bilayers (d = 6.9 nm, "
            "Γ = 0.4) with one partial reflection per bilayer and their phasor sum (schematic: equal amplitudes, no "
            "absorption, the phase step per bilayer scaled to align at the computed peak); right, the computed "
            "reflectance against wavelength at normal incidence (ideal interfaces: 73.0 % at 13.48 nm, FWHM 0.63 nm; "
            "the dashed curve is the eight-bilayer stack alone) and against angle at 13.5 nm for s and p. The sweep "
            "runs λ 12.5 → 14.5 nm at θ = 0, then θ 0 → 20° at 13.5 nm, where the peak walks to shorter wavelengths "
            "and s and p separate. <em>Model: EUV/BEUV multilayer mirrors ✅ (Parratt recursion, CXRO/Henke constants; "
            "measured Mo/Si mirrors reach ~70 %).</em>",
    pages=("materials.md",),
    extra={"loops": 3, "duration_s": T_TOTAL},
))


# ---------------------------------------------------------------------------
# P11a. Optical constants across the bands (Henke δ, β; Sellmeier n(λ) in the VUV)
# ---------------------------------------------------------------------------

HENKE_MATERIALS = ("Si", "Mo", "Ru", "Ta", "La", "B4C", "Au")
HENKE_LABELS = {"B4C": "B$_4$C"}                                   # mathtext: Helvetica Neue has no subscript glyphs
EDGES_EV = {"Si L$_{2,3}$": 99.8, "B K": 188.0, "Mo M$_{4,5}$": 228.0}      # X-ray Data Booklet values, for orientation only
LINES_NM = {"EUV 13.5": 13.5, "BEUV 6.7": 6.7}
#: materials.md "Range policy" + "Sellmeier models" tables: accepted range and where the page calls the fit checked.
SELLMEIER = {
    "CaF2": dict(label="CaF$_2$", lo=125.0, checked_from=230.0),
    "MgF2": dict(label="MgF$_2$", lo=115.0, checked_from=200.0),
    "LiF": dict(label="LiF", lo=105.0, checked_from=200.0),
    "BaF2": dict(label="BaF$_2$", lo=135.0, checked_from=200.0),
    "SiO2": dict(label="SiO$_2$ (fused silica)", lo=165.0, checked_from=165.0),
}
VUV_TABLE = {"AlF3": "AlF₃", "Na3AlF6": "Na₃AlF₆", "LaF3": "LaF₃", "GdF3": "GdF₃", "VUV_resist": "VUV_resist (stand-in)"}
VUV_LINES = {"Ar$_2$ 126": 126.0, "F$_2$ 157.63": 157.63, "ArF 193.4": 193.4, "KrF 248": 248.0}
#: capability-matrix.md "Materials database" row: Si at 13.5 nm through the Henke fallback
SI_EUV_DOC = (0.999001, 0.001826)


def optical_constants_compute(fast: bool):
    huv = native()
    energy = np.logspace(np.log10(30.5), np.log10(29900.0), 160 if fast else 420)
    lam = EV_NM / energy
    henke = {}
    for f in HENKE_MATERIALS:
        delta, beta = huv.xray_optical_constants(f, lam)
        henke[f] = (np.asarray(delta), np.asarray(beta))
    d_si, b_si = huv.xray_optical_constants("Si", [13.5])
    n_si = huv.refractive_index("Si", 13.5)          # the fallback the matrix row describes
    assert abs((1 - float(np.asarray(d_si)[0])) - SI_EUV_DOC[0]) < 1e-5 and abs(float(np.asarray(b_si)[0]) - SI_EUV_DOC[1]) < 1e-5
    assert abs(n_si.real - SI_EUV_DOC[0]) < 1e-5 and abs(n_si.imag - SI_EUV_DOC[1]) < 1e-5, "Si fallback differs from the matrix row"
    _, b_b4c = huv.xray_optical_constants("B4C", [6.5, 6.7])
    assert float(np.asarray(b_b4c)[1]) < float(np.asarray(b_b4c)[0]), "B4C must be more transparent at 6.7 nm than above the B K edge"
    # the VUV crystals: n(λ) inside each accepted range, probed point by point (no extrapolation exists to draw)
    lam_v = np.arange(100.0, 300.0 + 1e-9, 1.0 if fast else 0.5)
    sell = {}
    for name, info in SELLMEIER.items():
        vals = []
        for l in lam_v:
            try:
                vals.append(huv.refractive_index(name, float(l)).real)
            except ValueError:
                vals.append(np.nan)
        vals = np.asarray(vals)
        inside = lam_v >= info["lo"]
        assert np.all(np.isfinite(vals[inside])), f"{name}: a lookup inside its accepted range failed"
        assert np.all(~np.isfinite(vals[~inside])), f"{name}: a lookup below its stated range returned a value"
        try:
            huv.refractive_index(name, info["lo"] - 1.0)
            raise AssertionError(f"{name}: no error 1 nm below its accepted range")
        except ValueError:
            pass
        sell[name] = vals
    dots = {name: [(l, huv.refractive_index(name, l).real) for l in (126.0, 157.0)] for name in VUV_TABLE}
    return {"energy": energy, "henke": henke, "lam_v": lam_v, "sell": sell, "dots": dots}


def _split_sign(x, y):
    """Segments of (x, |y|) where y keeps one sign: [(xs, ys, negative?), …]."""
    out, start = [], 0
    sign = np.sign(y)
    for i in range(1, len(y) + 1):
        if i == len(y) or sign[i] != sign[start]:
            out.append((x[start:i], np.abs(y[start:i]), bool(sign[start] < 0)))
            start = i
    return out


def optical_constants_draw(d, th: S.Theme):
    fig = plt.figure(figsize=(S.WIDE_WIDTH + 0.8, 5.0))
    gs = fig.add_gridspec(2, 2, width_ratios=[1.15, 1.0], wspace=0.3, hspace=0.12)
    ax_d = fig.add_subplot(gs[0, 0])
    ax_b = fig.add_subplot(gs[1, 0], sharex=ax_d)
    ax_n = fig.add_subplot(gs[:, 1])
    for ax in (ax_d, ax_b, ax_n):
        S.style_axes(ax, th, grid="both")
    energy = d["energy"]
    ends_d, ends_b = [], []
    for slot, f in enumerate(HENKE_MATERIALS, start=1):
        delta, beta = d["henke"][f]
        col = th.s(slot)
        for xs, ys, neg in _split_sign(energy, delta):
            ax_d.plot(xs, ys, color=col, linestyle=(0, (2, 2)) if neg else "-", linewidth=S.LINE_PT * (0.7 if neg else 1.0), zorder=3)
        ax_b.plot(energy, beta, color=col, zorder=3)
        label = HENKE_LABELS.get(f, f)
        ends_d.append((energy[-1], max(abs(delta[-1]), 1.3e-6), label))     # clamped into the axes: B₄C's β at
        ends_b.append((energy[-1], max(beta[-1], 1.6e-9), label))            # 30 keV falls below the 1e-9 floor
    for ax in (ax_d, ax_b):
        ax.set_xscale("log")
        ax.set_yscale("log")
        ax.set_xlim(30, 3e4)
        for name, ev in EDGES_EV.items():
            ax.axvline(ev, color=th.axis, linewidth=S.HAIR_PT, linestyle=(0, (2, 3)), zorder=1)
        for name, nm in LINES_NM.items():
            ax.axvline(EV_NM / nm, color=th.ink2, linewidth=S.HAIR_PT * 1.2, linestyle=(0, (5, 3)), zorder=1)
    ax_d.set_ylim(1e-6, 1.0)
    ax_b.set_ylim(1e-9, 1.0)
    ax_d.tick_params(labelbottom=False)
    ax_d.set_ylabel("|δ| (dashed: δ < 0)")
    ax_b.set_ylabel("β")
    ax_b.set_xlabel("Photon energy (eV)   ·   n = 1 − δ + iβ")
    ax_d.set_title("(a) CXRO/Henke optical constants, 30 eV – 30 keV")
    for name, ev in EDGES_EV.items():
        ax_d.text(ev * 1.06, 0.55, name, color=th.ink2, fontsize=S.SMALL_PT - 1, rotation=90, va="top", ha="left")
    for name, nm in LINES_NM.items():
        ax_b.text(EV_NM / nm * 1.06, 2e-9, name, color=th.ink2, fontsize=S.SMALL_PT - 1, rotation=90, va="bottom", ha="left")
    S.end_labels(ax_d, ends_d, th, min_gap_px=11, dx_px=5, fontsize=S.SMALL_PT - 1)
    S.end_labels(ax_b, ends_b, th, min_gap_px=11, dx_px=5, fontsize=S.SMALL_PT - 1)
    # (b) the VUV crystals
    lam_v = d["lam_v"]
    ends_n = []
    for slot, (name, info) in enumerate(SELLMEIER.items(), start=1):
        vals = d["sell"][name]
        col = th.s(slot)
        extrap = (lam_v < info["checked_from"]) & np.isfinite(vals)
        checked = (lam_v >= info["checked_from"]) & np.isfinite(vals)
        ax_n.plot(np.where(extrap, lam_v, np.nan), vals, color=col, linestyle=(0, (3, 2)), linewidth=S.LINE_PT * 0.85, zorder=3)
        ax_n.plot(np.where(checked, lam_v, np.nan), vals, color=col, zorder=3)
        first = int(np.argmax(np.isfinite(vals)))
        S.dot(ax_n, [lam_v[first]], [vals[first]], col, th, size_px=6, filled=False)
        ends_n.append((300.0, vals[-1], info["label"]))
    S.end_labels(ax_n, ends_n, th, min_gap_px=12, dx_px=5, fontsize=S.SMALL_PT - 1)
    for name, pts in d["dots"].items():
        xs, ys = zip(*pts)
        ax_n.plot(xs, ys, linestyle="none", marker="o", markersize=5.5, markerfacecolor=th.ink2, markeredgecolor=th.surface,
                  markeredgewidth=S.RING_PT, zorder=4)
    ax_n.text(0.97, 0.64, "dots: VUV tables at 126 / 157 nm\n(AlF$_3$, Na$_3$AlF$_6$, LaF$_3$, GdF$_3$, resist stand-in)",
              color=th.ink2, fontsize=S.SMALL_PT - 1, ha="right", va="top", transform=ax_n.transAxes, zorder=6,
              bbox=dict(boxstyle="square,pad=0.25", facecolor=th.surface, edgecolor="none", alpha=0.92))
    for name, nm in VUV_LINES.items():
        ax_n.axvline(nm, color=th.ink2, linewidth=S.HAIR_PT * 1.2, linestyle=(0, (5, 3)), zorder=1)
        ax_n.text(nm + 2, 2.07, name, color=th.ink2, fontsize=S.SMALL_PT - 1, rotation=90, va="top", ha="left")
    ax_n.set_xlim(100, 300)
    ax_n.set_ylim(1.25, 2.1)
    ax_n.set_xlabel("Wavelength (nm)")
    ax_n.set_ylabel("Refractive index n (Sellmeier fit, k = 0)")
    ax_n.set_title("(b) VUV crystals: n(λ) inside each entry's range")
    ax_n.legend([Line2D([], [], color=th.ink2), Line2D([], [], color=th.ink2, linestyle=(0, (3, 2)), linewidth=S.LINE_PT * 0.85),
                 Line2D([], [], color=th.ink2, marker="o", markerfacecolor=th.surface, linestyle="none", markersize=5)],
                ["fit checked (page: CaF$_2$ ≥ 230 nm, others ≥ 200 nm)", "fit extrapolated into the VUV", "accepted-range edge (error below it)"],
                loc="lower right", fontsize=S.SMALL_PT - 1, frameon=True, facecolor=th.surface, edgecolor="none", framealpha=0.92)
    return fig


OPTICAL_ALT = ("Two panels: Henke optical constants delta and beta of seven materials on log axes from 30 eV to 30 "
               "keV with the B, Si and Mo edges and the 13.5 and 6.7 nm lines, and the refractive index of five VUV "
               "crystals from 100 to 300 nm, drawn only inside each entry's range and dashed where extrapolated.")
VUV_ALT = ("Refractive index of calcium fluoride, magnesium fluoride, lithium fluoride, barium fluoride and fused silica "
           "between 100 and 300 nm, each curve starting at its accepted-range edge and dashed where the Sellmeier fit is "
           "extrapolated, with the 126, 157.63, 193.4 and 248 nm lines marked.")
#: pixel box of panel (b) in the rendered 2× PNG (measured on the figure machine; the crop test in the notes re-checks it)
VUV_CROP_BOX = (987, 0, 931, 925)        # 1918 × 925 render; the transparent gap between the panels is x = 915–991

register(FigureSpec(
    name="materials-optical-constants",
    group="materials",
    compute=optical_constants_compute,
    draw=optical_constants_draw,
    alt=OPTICAL_ALT,
    caption="<strong>What the materials database knows, across the bands.</strong> (a) δ and β of Si, Mo, Ru, Ta, La, "
            "B₄C and Au from the embedded CXRO/Henke tables, 30 eV to 30 keV, with the Si L, B K and Mo M edges and the "
            "13.5 and 6.7 nm lines; dashed where δ is negative. (b) n(λ) of the VUV crystals from the Sellmeier fits, "
            "drawn only inside each entry's accepted range and dashed where the page calls the fit extrapolated; dots are "
            "the tabulated VUV coatings and the resist stand-in, approximate transcriptions and never measured resist "
            "data. <em>Model: X-ray/EUV/BEUV optical constants (CXRO/Henke) ✅ · materials database 🔶 (Sellmeier fits "
            "extrapolated into the VUV; VUV tables approximate).</em>",
    pages=("materials.md",),
    extra={"crops": {"vuv": dict(
        pages=("history/157nm-detour.md",),
        alt=VUV_ALT,
        caption="<strong>Where 157 nm sits on the transmission edge.</strong> Refractive index of the VUV crystals from "
                "the Sellmeier fits in the materials database, each drawn only inside its accepted range (CaF₂ from "
                "125 nm, MgF₂ 115, LiF 105, BaF₂ 135, fused silica 165) and dashed where the page calls the fit "
                "extrapolated; the F₂ line at 157.63 nm lies where only the fluorides still transmit, and the fit gives "
                "CaF₂ 1.557 against about 1.559 measured. <em>Model: materials database 🔶 (Sellmeier fits extrapolated "
                "into the VUV; VUV tables approximate).</em>",
        kind="figure",
        box=VUV_CROP_BOX,
    )}},
))


# ---------------------------------------------------------------------------
# P11b. Coverage bands: the range policy probed from refractive_index
# ---------------------------------------------------------------------------

HENKE_LO, HENKE_HI = 0.0413, 41.3
#: materials.md "Range policy (no silent extrapolation)" table, transcribed: entry → [(lo, hi, source), …]
PAGE_RANGES = {
    "CaF2": [(125.0, 2000.0, "sellmeier")], "MgF2": [(115.0, 2000.0, "sellmeier")], "LiF": [(105.0, 2000.0, "sellmeier")],
    "BaF2": [(135.0, 2000.0, "sellmeier")],
    "SiO2": [(HENKE_LO, HENKE_HI, "fallback"), (165.0, 2000.0, "sellmeier")],
    "Cr": [(HENKE_LO, HENKE_HI, "fallback"), (125.0, 161.0, "vuv")], "Si": [(HENKE_LO, HENKE_HI, "fallback"), (125.0, 161.0, "vuv")],
    "AlF3": [(125.0, 158.0, "vuv")], "Na3AlF6": [(125.0, 158.0, "vuv")], "LaF3": [(125.0, 158.0, "vuv")], "GdF3": [(125.0, 158.0, "vuv")],
    "VUV_resist": [(125.0, 158.0, "vuv")], "VUV_BARC": [(125.0, 158.0, "vuv")],
    "Si_euv": [(HENKE_LO, HENKE_HI, "henke")], "Mo_euv": [(HENKE_LO, HENKE_HI, "henke")], "Ru_euv": [(HENKE_LO, HENKE_HI, "henke")],
    "Ta_euv": [(HENKE_LO, HENKE_HI, "henke")], "La_beuv": [(HENKE_LO, HENKE_HI, "henke")], "B4C_beuv": [(HENKE_LO, HENKE_HI, "henke")],
    "EUV_resist": [(HENKE_LO, HENKE_HI, "standin")], "MOx_resist": [(HENKE_LO, HENKE_HI, "standin")], "BEUV_resist": [(HENKE_LO, HENKE_HI, "standin")],
    "henke:B4C@2.52": [(HENKE_LO, HENKE_HI, "henke")],
}
SELLMEIER_CHECKED = {"CaF2": 230.0, "MgF2": 200.0, "LiF": 200.0, "BaF2": 200.0, "SiO2": 165.0}
SOURCES = {  # legend order: (colour key, label, grade slug)
    "sellmeier": ("blue", "Sellmeier fit, checked range", "implemented"),
    "sellmeier_vuv": ("blue", "Sellmeier fit extrapolated into the VUV", "simplified"),
    "vuv": ("orange", "VUV table (126–160 nm), approximate", "simplified"),
    "henke": ("green", "Henke/CXRO computed at λ", "implemented"),
    "fallback": ("green", "Henke fallback for a VUV name", "implemented"),
    "standin": ("magenta", "resist stand-in (formula + density)", "simplified"),
}
COVERAGE_LINES = {"6.7": (6.7, 0), "13.5": (13.5, 0), "126": (126.0, 0), "158": (157.63, 1), "193": (193.4, 0), "248": (248.0, 1), "365": (365.0, 0)}


def coverage_compute(fast: bool):
    huv = native()
    lams = np.logspace(np.log10(0.03), np.log10(3000.0), 350 if fast else 900)
    names = list(huv.material_names()) + ["henke:B4C@2.52"]
    probed = {}
    for name in names:
        ok = []
        for l in lams:
            try:
                huv.refractive_index(name, float(l))
                ok.append(True)
            except ValueError:
                ok.append(False)
        ok = np.asarray(ok)
        segs, start = [], None
        for i, v in enumerate(ok):
            if v and start is None:
                start = i
            if not v and start is not None:
                segs.append((float(lams[start]), float(lams[i - 1])))
                start = None
        if start is not None:
            segs.append((float(lams[start]), float(lams[-1])))
        probed[name] = (ok, segs)
    # --- the test: the probed behaviour equals the page's range table (grid resolution ≈ 1.3 %) ---
    step = lams[1] / lams[0]
    tol = 2 * np.log(step) + 1e-9
    for name, rows in PAGE_RANGES.items():
        ok, segs = probed[name]
        assert len(segs) == len(rows), f"{name}: probed {len(segs)} range(s), the page lists {len(rows)}"
        for (lo, hi, _src), (plo, phi) in zip(rows, segs):
            inside = (lams >= lo * step) & (lams <= hi / step)
            assert np.all(ok[inside]), f"{name}: a lookup inside the page's range {lo}–{hi} nm failed"
            assert abs(np.log(plo / lo)) < tol, f"{name}: probed start {plo:.4f} nm vs the page's {lo}"
            assert abs(np.log(phi / hi)) < tol, f"{name}: probed end {phi:.1f} nm vs the page's {hi}"
    extra = sorted(set(names) - set(PAGE_RANGES))
    assert not extra, f"database entries missing from the page's range table: {extra}"
    return {"lams": lams, "names": names, "probed": {n: segs for n, (_ok, segs) in probed.items()}}


def coverage_draw(d, th: S.Theme) -> str:
    theme = th.name
    names = [n for n in PAGE_RANGES]  # the page's order: crystals, Henke fallbacks, VUV tables, EUV/BEUV, stand-ins, generic
    row_h, x0, x1, y0 = 15.5, 150, 920, 72
    W, H = 960, int(y0 + row_h * len(names) + 136)
    s = K.Svg(W, H, theme, "Every entry has a stated range", COVERAGE_ALT, generator=GENERATOR)
    lo_nm, hi_nm = 0.03, 3000.0

    def xpx(nm):
        return x0 + (math.log10(nm) - math.log10(lo_nm)) / (math.log10(hi_nm) - math.log10(lo_nm)) * (x1 - x0)

    s.eyebrow(x0, 22, "Where refractive_index(name, λ) returns a value")
    s.text(x0, 40, "probed on a log grid of the compiled module; white = WavelengthOutOfRange", size=K.SMALL, anchor="start", color="ink2")
    hatch = s.hatch("fallback-hatch", s.c("green"), spacing=5.0, sw=1.0)
    for dec in (0.1, 1.0, 10.0, 100.0, 1000.0):
        x = xpx(dec)
        s.line(x, y0 - 6, x, y0 + row_h * len(names), "grid", 1.0)
        s.text(x, y0 - 10, f"{dec:g} nm", size=K.SMALL, anchor="middle", color="ink2")
    for label, (nm, row) in COVERAGE_LINES.items():
        x = xpx(nm)
        s.line(x, y0, x, y0 + row_h * len(names), "ink2", 0.8, dash="2 3", opacity=0.6)
        s.text(x, y0 + row_h * len(names) + 14 + 14 * row, label, size=K.SMALL, anchor="middle", color="ink2")
    s.text((x0 + x1) / 2, y0 + row_h * len(names) + 44, "wavelength (nm), log axis · dotted: the lines the presets use", size=K.SMALL, anchor="middle", color="ink2")
    for i, name in enumerate(names):
        y = y0 + i * row_h
        if i % 2 == 0:
            s.rect(x0, y, x1 - x0, row_h, fill="band", stroke="none", opacity=0.5)
        s.text(x0 - 8, y + row_h - 4, name, size=K.SMALL, anchor="end", color="ink", mono=True)
        for (lo, hi, src), (plo, phi) in zip(PAGE_RANGES[name], d["probed"][name]):
            xa, xb = xpx(plo), xpx(phi)
            if src == "sellmeier":
                xc = xpx(SELLMEIER_CHECKED[name])
                if xc > xa + 1:
                    s.rect(xa, y + 2.5, xc - xa, row_h - 5, fill=s.tint("blue", 0.45), stroke=s.c("blue"), sw=0.8, dash="3 2", rx=2)
                s.rect(max(xc, xa), y + 2.5, xb - max(xc, xa), row_h - 5, fill="blue", stroke="none", rx=2, opacity=0.85)
            elif src == "fallback":
                s.rect(xa, y + 2.5, xb - xa, row_h - 5, fill=hatch, stroke=s.c("green"), sw=0.8, rx=2)
            else:
                col = SOURCES[src][0]
                s.rect(xa, y + 2.5, xb - xa, row_h - 5, fill=col, stroke="none", rx=2, opacity=0.85)
    # legend with the grade of each source copied from the matrix row it belongs to
    rows_ = [[("sellmeier", "rect"), ("sellmeier_vuv", "outline")], [("vuv", "rect"), ("henke", "rect")],
             [("fallback", "hatch"), ("standin", "rect")]]
    ly = y0 + row_h * len(names) + 70
    for row in rows_:
        for col_i, (key, kind) in enumerate(row):
            lx = x0 + col_i * 400
            col, label, grade = SOURCES[key]
            if kind == "rect":
                s.rect(lx, ly - 10, 16, 11, fill=col, stroke="none", rx=2, opacity=0.85)
            elif kind == "outline":
                s.rect(lx, ly - 10, 16, 11, fill=s.tint("blue", 0.45), stroke=s.c("blue"), sw=0.8, dash="3 2", rx=2)
            else:
                s.rect(lx, ly - 10, 16, 11, fill=hatch, stroke=s.c("green"), sw=0.8, rx=2)
            s.text(lx + 22, ly, label, size=K.SMALL, anchor="start", color="ink2")
            tw = K.text_width(label, K.SMALL)
            K.chip(s, lx + 22 + tw + 8, ly - 4, grade, size=K.SMALL)
        ly += 22
    return s.render()


COVERAGE_ALT = ("One row per materials-database entry on a log wavelength axis from 0.03 to 3000 nm: coloured bands "
                "where a refractive-index lookup returns a value, gaps where it raises an out-of-range error, hatched "
                "bands for the Henke fallback of Si, Cr and SiO2, and a legend with each source's grade.")

register(FigureSpec(
    name="materials-coverage-bands",
    group="materials",
    compute=coverage_compute,
    draw=coverage_draw,
    fmt="svg",
    kind="schematic",
    alt=COVERAGE_ALT,
    caption="<strong>Every entry has a stated range; outside it the lookup fails.</strong> One row per database entry "
            "on a log wavelength axis: bands are where <code>refractive_index</code> returns a value, coloured by the "
            "source behind it, white where it raises <code>WavelengthOutOfRange</code>; the hatched bands are the "
            "Henke fallback of Si, Cr and SiO₂ and the dashed outlines the Sellmeier fits extrapolated into the VUV. "
            "Probed from the behaviour of the compiled module and asserted against the page's range table, not "
            "transcribed from it. <em>Model: materials database 🔶 (no extrapolation; VUV tables approximate; resist "
            "entries are stand-ins) · X-ray/EUV/BEUV optical constants (CXRO/Henke) ✅.</em>",
    pages=("materials.md",),
))
