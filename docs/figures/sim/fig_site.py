"""Simulator figures for the narrative site sections (History, Process nodes, Future).

Requested by the site pages' authors (notes S1 §6b, S2 §6, S3 §5). Wherever a page
has a "Try it" example, the figure uses the same source, optics, mask and grid, so
the numbers printed by the example and the curves agree. Outputs go to
``docs/assets/images/sim/site/``.
"""

from __future__ import annotations

import math

import numpy as np
from matplotlib import pyplot as plt
from matplotlib.lines import Line2D

import style as S
from common import FigureSpec, native, register

HC = 1239.84193  # eV nm


def _engine(huv, src, opt, mask, grid, **kw):
    return huv.SimulationEngine(src, opt, mask, grid=grid, **kw)


def _profile(img):
    a = np.asarray(img.intensity)
    return np.asarray(img.x_nm), a[a.shape[0] // 2]


def _ls(huv, src, opt, pitch, cd=None, size=64, per=32, focus=0.0, **kw):
    """Contrast of a pitch/2 (or cd) line/space grating on a two-period commensurate field."""
    mask = huv.MaskConfig.line_space(pitch / 2 if cd is None else cd, pitch)
    eng = _engine(huv, src, opt, mask, huv.GridConfig(size, pitch / per), **kw)
    return eng.compute_aerial_image(focus).image_contrast()


def _site(name, compute, draw, alt, caption, pages, **kw):
    register(FigureSpec(name=name, group="site", subdir="site", compute=compute, draw=draw, alt=alt,
                        caption=caption, pages=pages, **kw))


def _legend(ax, th, labels, slots, **kw):
    ax.legend([Line2D([], [], color=th.s(s)) for s in slots], labels, **kw)


# ===========================================================================
# History
# ===========================================================================

ERAS = (("g-line, NA 0.45 (1987)", "hg_g_line", 0.45), ("i-line, NA 0.63 (1994)", "hg_i_line", 0.63),
        ("KrF, NA 0.68 (1998)", "krf_laser", 0.68), ("ArF, NA 0.92 (2004)", "arf_laser", 0.92))


def eras_compute(fast):
    huv = native()
    mask = huv.MaskConfig.line_space(200.0, 400.0)
    grid = huv.GridConfig(128, 6.25)
    out = []
    for label, factory, na in ERAS:
        src = getattr(huv.SourceConfig, factory)(0.5)
        img = _engine(huv, src, huv.OpticsConfig(numerical_aperture=na), mask, grid).compute_aerial_image(0.0)
        x, p = _profile(img)
        out.append({"label": label, "x": x, "p": p, "c": img.image_contrast(), "k1": 200.0 * na / src.wavelength_nm})
    return out


def _profiles_draw(rows, th, title, xlabel="x (nm)", width=S.FULL_WIDTH, height=4.0):
    fig, ax = plt.subplots(figsize=(width, height))
    S.style_axes(ax, th, grid="both")
    for slot, r in enumerate(rows, start=1):
        ax.plot(r["x"], r["p"], color=th.s(slot), zorder=3)
    ax.set_xlim(rows[0]["x"][0], rows[0]["x"][-1])
    ax.set_ylim(0, None)
    ax.set_xlabel(xlabel)
    ax.set_ylabel("Relative intensity (clear field = 1)")
    ax.set_title(title)
    return fig, ax


def eras_draw(rows, th):
    fig, ax = _profiles_draw(rows, th, "200 nm lines on a 400 nm pitch, four eras (σ 0.5)")
    _legend(ax, th, [f"{r['label']}: k₁ {r['k1']:.2f}, contrast {r['c']:.2f}".replace("₁", "$_1$") for r in rows],
            range(1, len(rows) + 1), loc="upper center", bbox_to_anchor=(0.5, -0.17), ncol=2,
            fontsize=S.SMALL_PT)
    fig.subplots_adjust(bottom=0.3)
    return fig


_site("site-eras-400nm-pitch", eras_compute, eras_draw,
      alt="Aerial-image cross-sections of 200 nm lines on a 400 nm pitch for a g-line, an i-line, a KrF and an ArF "
          "tool: the modulation deepens with each generation as k1 grows from 0.21 to 0.95.",
      caption="The page's “one pitch through four eras” example as images: 200 nm lines and spaces through a "
              "g-line (NA 0.45), i-line (NA 0.63), KrF (NA 0.68) and ArF (NA 0.92) tool, conventional σ 0.5, "
              "default 2 % flare; two periods of the simulated field. Model: scalar Hopkins imaging ✅, thin "
              "mask, no resist.",
      pages=("history/index.md",))


STEPPERS = (("GCA DSW 4800 (1978), g-line NA 0.28", "hg_g_line", 0.28),
            ("Nikon NSR-1505G4D (1987), g-line NA 0.45", "hg_g_line", 0.45),
            ("Nikon NSR-2205i11D (1994), i-line NA 0.63", "hg_i_line", 0.63))


def steppers_compute(fast):
    huv = native()
    mask = huv.MaskConfig.line_space(800.0, 1600.0)
    grid = huv.GridConfig(128, 25.0)
    out = []
    for label, factory, na in STEPPERS:
        src = getattr(huv.SourceConfig, factory)(0.5)
        img = _engine(huv, src, huv.OpticsConfig(numerical_aperture=na), mask, grid).compute_aerial_image(0.0)
        x, p = _profile(img)
        out.append({"label": label, "x": x, "p": p, "c": img.image_contrast()})
    return out


def steppers_draw(rows, th):
    fig, ax = _profiles_draw(rows, th, "800 nm lines on a 1.6 µm pitch, three stepper generations (σ 0.5)")
    _legend(ax, th, [f"{r['label']}: contrast {r['c']:.2f}" for r in rows], range(1, 4), loc="upper center",
            bbox_to_anchor=(0.5, -0.17), ncol=1, fontsize=S.SMALL_PT)
    fig.subplots_adjust(bottom=0.32)
    return fig


_site("site-steppers-1600nm-pitch", steppers_compute, steppers_draw,
      alt="Cross-sections of 800 nm lines on a 1.6 micrometre pitch through three stepper lenses; the edges sharpen "
          "from the 1978 g-line tool to the 1994 i-line tool.",
      caption="The projection-era example as images: 800 nm lines on a 1.6 µm pitch through the GCA DSW 4800 "
              "(g-line, NA 0.28), Nikon NSR-1505G4D (g-line, NA 0.45) and NSR-2205i11D (i-line, NA 0.63), "
              "σ 0.5, default 2 % flare. Model: scalar Hopkins imaging ✅ (ideal lenses: the tools' real "
              "aberrations and illuminators are not modelled).",
      pages=("history/projection-era.md",))


KRF_ONSET = 248.3 / (180.0 * 0.80) - 1  # first orders enter the pupil


def krf_sigma_compute(fast):
    huv = native()
    mask = huv.MaskConfig.line_space(90.0, 180.0)
    opt = huv.OpticsConfig(numerical_aperture=0.80)
    grid = huv.GridConfig(128, 2.8125)
    sig = np.round(np.arange(0.30, 0.951, 0.05 if fast else 0.01), 3)
    c = [_engine(huv, huv.SourceConfig.krf_laser(float(s)), opt, mask, grid)
         .compute_aerial_image(0.0).image_contrast() for s in sig]
    ex = (0.3, 0.5, 0.7, 0.9)
    cex = [_engine(huv, huv.SourceConfig.krf_laser(s), opt, mask, grid).compute_aerial_image(0.0).image_contrast()
           for s in ex]
    lam = huv.SourceConfig.krf_laser(0.5).wavelength_nm
    return {"sigma": sig, "c": np.array(c), "ex": ex, "cex": cex, "onset": lam / (180.0 * 0.80) - 1}


def krf_sigma_draw(d, th):
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 3.8))
    S.style_axes(ax, th, grid="both")
    ax.plot(d["sigma"], d["c"], color=th.s(1), zorder=3)
    S.dot(ax, d["ex"], d["cex"], th.s(2), th)
    ax.legend([Line2D([], [], color=th.s(1)), Line2D([], [], color=th.s(2), marker="o", linestyle="none")],
              ["simulated contrast", "values the page example prints"], loc="upper left")
    S.limit_line(ax, d["onset"], th, f"σ = λ/(p·NA) − 1 = {d['onset']:.3f}", where=0.97, side="left")
    ax.set_xlim(0.3, 0.95)
    ax.set_ylim(0, None)
    ax.set_xlabel("Outer σ of conventional illumination")
    ax.set_ylabel("Aerial-image contrast")
    ax.set_title("90 nm lines, 180 nm pitch, KrF 248.3 nm, NA 0.80 (k₁ ≈ 0.29)".replace("₁", "$_1$"))
    return fig


_site("site-krf-sigma-sweep", krf_sigma_compute, krf_sigma_draw,
      alt="Contrast of 90 nm lines on a 180 nm pitch at KrF NA 0.8 versus the illumination sigma: zero until sigma "
          "about 0.72, where the first diffraction orders start to enter the pupil, then rising.",
      caption="Why larger σ helps a k<sub>1</sub> ≈ 0.29 grating (the page's σ-sweep example; dots = the four "
              "values it prints; default adaptive source sampling throughout). Below σ = λ/(p·NA) − 1 ≈ 0.72 no source point can put a first diffraction "
              "order through the pupil and the image is flat; above it the off-axis part of the fill forms "
              "two-beam images. Conventional fill only; default 2 % flare. Model: scalar Hopkins imaging ✅, "
              "illumination pupil fills ✅.",
      pages=("history/excimer-duv.md",))


def f2_bw_compute(fast):
    huv = native()
    opt = huv.OpticsConfig(numerical_aperture=0.85)
    mask = huv.MaskConfig.line_space(100.0, 200.0)
    grid = huv.GridConfig(128, 3.125)
    bws = np.round(np.arange(0.1, 5.01, 0.5 if fast else 0.1), 2)
    mono, poly, multi = [], [], []
    for bw in bws:
        src = huv.SourceConfig(wavelength_nm=157.63, sigma_outer=0.7, bandwidth_pm=float(bw), spectral_samples=7)
        eng = _engine(huv, src, opt, mask, grid)
        mono.append(eng.compute_aerial_image(0.0).image_contrast())
        poly.append(eng.compute_polychromatic(0.0).image_contrast())
        multi.append(eng.compute_multiwavelength(0.0).image_contrast())
    return {"bw": bws, "mono": np.array(mono), "poly": np.array(poly), "multi": np.array(multi)}


def f2_bw_draw(d, th):
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 3.8))
    S.style_axes(ax, th, grid="both")
    ax.plot(d["bw"], d["mono"], color=th.muted, linewidth=S.HAIR_PT * 1.4, zorder=2)
    ax.plot(d["bw"], d["multi"], color=th.s(1), linewidth=S.LINE_PT * 1.8, zorder=3)
    ax.plot(d["bw"], d["poly"], color=th.s(2), zorder=4, linestyle=(0, (4, 3)))
    ax.set_xlim(0, 5)
    lo = min(d["poly"].min(), d["multi"].min())
    ax.set_ylim(max(0.0, lo - 0.1), 1.0)
    ax.set_xlabel("F₂ laser bandwidth, FWHM (pm)".replace("₂", "$_2$"))
    ax.set_ylabel("Aerial-image contrast")
    ax.set_title("100 nm lines, 200 nm pitch, 157.63 nm, NA 0.85, σ 0.7")
    ax.legend([Line2D([], [], color=th.muted), Line2D([], [], color=th.s(1), linewidth=S.LINE_PT * 1.8),
               Line2D([], [], color=th.s(2), linestyle=(0, (4, 3)))],
              ["monochromatic", "exact: TCC rebuilt per spectral sample", "narrow-band: centre-λ TCC + focus shift"],
              loc="lower left")
    return fig


_site("site-f2-bandwidth", f2_bw_compute, f2_bw_draw,
      alt="Contrast of 100 nm lines at 157 nm versus laser bandwidth from 0.1 to 5 picometres: it falls as the "
          "bandwidth grows because the single-material lens focuses each wavelength at a different depth; the "
          "exact per-wavelength and the narrow-band calculations lie on top of each other.",
      caption="Why a single lens material makes bandwidth matter (the page's bandwidth example, swept). Each "
              "spectral sample (7 per line) is focused at a different depth by the default chromatic "
              "coefficient of a CaF<sub>2</sub> lens at 157 nm, so contrast falls with FWHM. The exact path "
              "(<code>compute_multiwavelength</code>) and the narrow-band approximation "
              "(<code>compute_polychromatic</code>) agree at Δλ/λ ~ 10<sup>−5</sup>. Default 2 % flare. Models: "
              "broadband imaging ✅, narrow-band approximation 🔶, refractive optics ✅ (linear axial "
              "chromatic coefficient).",
      pages=("history/157nm-detour.md",))


WHY_IMM = (("F₂ 157.63 nm, dry NA 0.85", "f2_laser", "dry", 0.85),
           ("ArF 193.37 nm, dry NA 0.93", "arf_laser", "dry", 0.93),
           ("ArF 193.37 nm, water NA 1.35", "arf_laser", "193i", 1.35))


def why_imm_compute(fast):
    huv = native()
    hp = np.arange(30.0, 100.01, 2.0 if fast else 0.5)
    out = {"hp": hp, "rows": []}
    for label, factory, kind, na in WHY_IMM:
        src = getattr(huv.SourceConfig, factory)(0.9)
        opt = huv.OpticsConfig.immersion_193i() if kind == "193i" else huv.OpticsConfig(numerical_aperture=na)
        c = np.array([_ls(huv, src, opt, 2 * h) for h in hp])
        out["rows"].append({"label": label, "c": c, "k1hp": 0.25 * src.wavelength_nm / na})
    return out


def why_imm_draw(d, th):
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 4.0))
    S.style_axes(ax, th, grid="both")
    for slot, r in enumerate(d["rows"], start=1):
        ax.plot(d["hp"], r["c"], color=th.s(slot), zorder=3)
        ax.axvline(r["k1hp"], color=th.s(slot), linewidth=S.HAIR_PT * 1.2, linestyle=(0, (3, 3)), zorder=2)
    S.limit_line(ax, 45.0, th, "45 nm (page example)", where=0.97, side="left")
    ax.set_xlim(30, 100)
    ax.set_ylim(0, 1)
    ax.set_xlabel("Half-pitch of 1:1 lines (nm)")
    ax.set_ylabel("Aerial-image contrast")
    ax.set_title("Why immersion beat 157 nm (σ 0.9; dotted: each tool's k₁ = 0.25)".replace("₁", "$_1$"))
    _legend(ax, th, [r["label"].replace("₂", "$_2$") for r in d["rows"]], (1, 2, 3), loc="lower right")
    return fig


_site("site-why-immersion", why_imm_compute, why_imm_draw,
      alt="Contrast versus half-pitch for a dry 157 nm lens at NA 0.85, a dry 193 nm lens at NA 0.93 and 193 nm "
          "water immersion at NA 1.35: immersion keeps imaging down to about 38 nm half-pitch, the dry 157 nm lens "
          "only to about 49 nm and dry 193 nm to about 55 nm.",
      caption="The comparison that ended the 157 nm programme, across half-pitch: conventional σ 0.9, default "
              "2 % flare, two-period commensurate fields. Dotted lines mark each tool's k<sub>1</sub> = 0.25 "
              "half-pitch; with σ 0.9 the image vanishes slightly above it, at λ/(2NA·1.9). The dashed line is "
              "the 45 nm example on the page. Models: scalar Hopkins imaging ✅, immersion optics ✅ (water "
              "n = 1.437; no polarization in this chart).",
      pages=("history/157nm-detour.md", "history/immersion-multipatterning.md"))


def pol193_compute(fast):
    huv = native()
    src = huv.SourceConfig.arf_laser(0.9)
    opt = huv.OpticsConfig.immersion_193i()
    pitch = np.arange(72.0, 160.1, 4.0 if fast else 1.0)
    out = {"pitch": pitch}
    out["scalar"] = np.array([_ls(huv, src, opt, p) for p in pitch])
    for key, pol in (("te", "y"), ("tm", "x"), ("un", "unpolarized")):
        vs = huv.VectorSettings(pol)
        out[key] = np.array([_ls(huv, src, opt, p, vector=vs) for p in pitch])
    return out


def pol193_draw(d, th):
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 4.0))
    S.style_axes(ax, th, grid="both")
    x = d["pitch"]
    ax.plot(x, d["scalar"], color=th.ink2, linewidth=S.HAIR_PT * 1.4, zorder=2)
    for slot, key in ((1, "te"), (2, "tm"), (3, "un")):
        ax.plot(x, d[key], color=th.s(slot), zorder=3)
    ax.set_xlim(x[0], x[-1])
    ax.set_ylim(0, 1)
    ax.set_xlabel("Pitch of 1:1 lines (nm)")
    ax.set_ylabel("Aerial-image contrast")
    ax.set_title("193i, NA 1.35, conventional σ 0.9: polarization decides the contrast")
    ax.legend([Line2D([], [], color=th.s(i)) for i in (1, 2, 3)] + [Line2D([], [], color=th.ink2)],
              ["TE (field along the lines)", "TM (field across the lines)", "unpolarized", "scalar model"],
              loc="lower right")
    return fig


_site("site-193i-polarization", pol193_compute, pol193_draw,
      alt="Contrast versus pitch for 193 nm water immersion at NA 1.35 with TE, TM and unpolarized light: TE keeps "
          "the most contrast, TM the least, unpolarized lies between; the scalar model overestimates all but TE.",
      caption="Polarization at NA 1.35 (water): dense 1:1 lines imaged with the vector model for TE, TM and "
              "unpolarized illumination, against the scalar model. Near the resolution limit the two first "
              "orders meet at steep angles, so TM light (field in the plane of incidence) interferes poorly; "
              "production 193i tools use polarized illumination. Thin mask, ideal lens, image in water (no "
              "resist film). Models: vector imaging ✅, immersion optics ✅.",
      pages=("history/immersion-multipatterning.md",))


LELE_T = 0.3  # nominal, replaced by the dose-to-size threshold below


def _dts_threshold(x, p, cd, centre, pitch):
    """Threshold at which the dark line centred at `centre` prints `cd` wide (periodic profile)."""
    period = x[-1] - x[0] + (x[1] - x[0])
    xx = (x - centre + period / 2) % period - period / 2
    keep = np.abs(xx) < pitch / 2  # this line's unit cell only
    xx, p = xx[keep], p[keep]
    order = np.argsort(xx)
    xs, ps = xx[order], p[order]
    lo, hi = float(ps.min()), float(ps.max())

    def width(t):
        dark = ps < t
        return dark.sum() * (xs[1] - xs[0])

    for _ in range(60):
        mid = (lo + hi) / 2
        lo, hi = (mid, hi) if width(mid) < cd else (lo, mid)
    return (lo + hi) / 2


def lele_compute(fast):
    huv = native()
    src = huv.SourceConfig.arf_laser(0.8)
    opt = huv.OpticsConfig(numerical_aperture=0.93)
    grid = huv.GridConfig(256, 1.1875)
    out = {}
    img = _engine(huv, src, opt, huv.MaskConfig.line_space(38.0, 76.0), grid).compute_aerial_image(0.0)
    out["x"], out["target"] = _profile(img)
    out["c_target"] = img.image_contrast()
    for key, off in (("a", 0.0), ("b", 76.0)):
        im = _engine(huv, src, opt, huv.MaskConfig.line_space(38.0, 152.0, offset_nm=off), grid).compute_aerial_image(0.0)
        _, p = _profile(im)
        out[key] = p
        out["c_" + key] = im.image_contrast()
        out["t_" + key] = _dts_threshold(out["x"], p, 38.0, off, 152.0)
    out["printed"] = (out["a"] < out["t_a"]) | (out["b"] < out["t_b"])
    return out


def lele_draw(d, th):
    fig, (ax, ax2) = plt.subplots(2, 1, figsize=(S.FULL_WIDTH, 4.8), sharex=True,
                                  gridspec_kw={"height_ratios": [3, 1], "hspace": 0.12})
    S.style_axes(ax, th, grid="both")
    x = d["x"]
    ax.plot(x, d["target"], color=th.ink2, linewidth=S.HAIR_PT * 1.6, zorder=2)
    ax.plot(x, d["a"], color=th.s(1), zorder=3)
    ax.plot(x, d["b"], color=th.s(2), zorder=3)
    for key, slot in (("t_a", 1), ("t_b", 2)):
        ax.axhline(d[key], color=th.s(slot), linewidth=S.HAIR_PT, linestyle=(0, (3, 3)))
    ax.set_ylim(0, None)
    ax.set_ylabel("Relative intensity")
    ax.set_title("Pitch splitting: 38 nm lines on a 76 nm pitch, ArF dry NA 0.93, σ 0.8")
    ax.legend([Line2D([], [], color=th.ink2), Line2D([], [], color=th.s(1)), Line2D([], [], color=th.s(2))],
              [f"single exposure at 76 nm pitch (contrast {d['c_target']:.2f})",
               f"LELE exposure A, 152 nm pitch (contrast {d['c_a']:.2f})",
               "LELE exposure B, shifted by 76 nm"], loc="upper center", bbox_to_anchor=(0.5, -0.62), ncol=1,
              fontsize=S.SMALL_PT)
    S.style_axes(ax2, th, grid=None)
    pr = d["printed"].astype(float)
    ax2.fill_between(x, 0, pr, step="mid", color=th.ink2, linewidth=0)
    ax2.set_ylim(0, 1.05)
    ax2.set_yticks([])
    ax2.set_xlim(x[0], x[-1])
    ax2.set_xlabel("x (nm)")
    ax2.set_title("printed union (each exposure thresholded at its dose-to-size)", fontsize=S.SMALL_PT + 0.5)
    fig.subplots_adjust(bottom=0.3)
    return fig


_site("site-lele-pitch-split", lele_compute, lele_draw,
      alt="A 76 nm pitch grating gives a flat image in one dry ArF exposure, while each of two LELE exposures at 152 "
          "nm pitch is well modulated; thresholding each and taking the union prints all lines at 76 nm pitch.",
      caption="Why pitch splitting works (the page's LELE example): one dry ArF exposure of a 76 nm pitch "
              "(k<sub>1</sub> = 0.18) has no modulation, but each LELE exposure carries every other line at 152 nm "
              "pitch. Each exposure is thresholded at the dose that prints 38 nm lines (dashed) and the two "
              "printed patterns are united (bottom). Default 2 % flare; perfect overlay. Models: scalar imaging ✅, "
              "multiple patterning 🔶 (constant-threshold resist, no etch).",
      pages=("history/immersion-multipatterning.md",))


def nxe_hna_compute(fast):
    huv = native()
    src = huv.SourceConfig.lpp_sn_13nm5(0.9)
    pitch = np.arange(12.0, 40.01, 0.5 if fast else 0.2)
    out = {"pitch": pitch, "lam": src.wavelength_nm}
    for key, opt in (("nxe", huv.OpticsConfig.euv_nxe()), ("hna", huv.OpticsConfig.euv_high_na())):
        out[key] = np.array([_ls(huv, src, opt, float(p)) for p in pitch])
    return out


def nxe_hna_draw(d, th):
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 3.9))
    S.style_axes(ax, th, grid="both")
    x = d["pitch"]
    ax.plot(x, d["nxe"], color=th.s(1), zorder=3)
    ax.plot(x, d["hna"], color=th.s(2), zorder=3)
    for na, slot in ((0.33, 1), (0.55, 2)):
        ax.axvline(0.5 * d["lam"] / na, color=th.s(slot), linewidth=S.HAIR_PT * 1.2, linestyle=(0, (3, 3)))
    S.limit_line(ax, 16.0, th, "16 nm (page example)", where=0.97, side="right")
    ax.set_xlim(x[0], x[-1])
    ax.set_ylim(0, 1)
    ax.set_xlabel("Pitch of 1:1 lines (nm)")
    ax.set_ylabel("Aerial-image contrast")
    ax.set_title("Sn LPP 13.5 nm, conventional σ 0.9 (dotted: k₁ = 0.25 pitch)".replace("₁", "$_1$"))
    _legend(ax, th, ["NXE, NA 0.33 (unobscured)", "High-NA, NA 0.55 (0.2·NA obscuration, assumed)"], (1, 2),
            loc="lower right")
    return fig


_site("site-euv-nxe-vs-high-na", nxe_hna_compute, nxe_hna_draw,
      alt="Contrast versus pitch from 12 to 40 nm for 0.33 and 0.55 NA EUV optics with a tin plasma source: the "
          "0.33 NA curve reaches zero near 22 nm pitch, the 0.55 NA curve near 13 nm.",
      caption="Resolution at 0.33 vs 0.55 NA through the simulator's EUV presets (Sn LPP, conventional σ 0.9, "
              "two-period commensurate fields). Isotropic pupil — the anamorphic 4×/8× magnification is not "
              "modelled — with a 0.2·NA central obscuration assumed for High-NA; thin mask (no mask-3D shadowing), "
              "no flare. Models: scalar Hopkins imaging ✅, EUV projection optics 🔶.",
      pages=("history/euv.md", "nodes/angstrom-era.md"))


def schw_compute(fast):
    huv = native()
    src = huv.SourceConfig.lpp_sn_13nm5(0.5)
    mask = huv.MaskConfig.line_space(30.0, 60.0)
    grid = huv.GridConfig(128, 1.875)
    pitch = np.arange(30.0, 120.1, 3.0 if fast else 1.0)
    out = {"rows": [], "pitch": pitch}
    for obs in (0.0, 0.25, 0.4):
        opt = huv.OpticsConfig.schwarzschild(numerical_aperture=0.3, obscuration_ratio=obs)
        img = _engine(huv, src, opt, mask, grid).compute_aerial_image(0.0)
        x, p = _profile(img)
        cvp = np.array([_ls(huv, src, opt, float(pp)) for pp in pitch])
        out["rows"].append({"obs": obs, "x": x, "p": p, "c": img.image_contrast(), "cvp": cvp})
    return out


def schw_draw(d, th):
    fig, (ax, ax2) = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 3.9), gridspec_kw={"wspace": 0.28})
    S.style_axes(ax, th, grid="both")
    S.style_axes(ax2, th, grid="both")
    for slot, r in enumerate(d["rows"], start=1):
        sel = np.abs(r["x"]) <= 60.0
        ax.plot(r["x"][sel], r["p"][sel], color=th.s(slot), zorder=3)
        ax2.plot(d["pitch"], r["cvp"], color=th.s(slot), zorder=3)
    ax.set_xlim(-60, 60)
    ax.set_ylim(0, None)
    ax.set_xlabel("x (nm)")
    ax.set_ylabel("Relative intensity")
    ax.set_title("(a) 30 nm lines, 60 nm pitch")
    ax2.set_xlim(d["pitch"][0], d["pitch"][-1])
    ax2.set_ylim(0, 1)
    ax2.set_xlabel("Pitch of 1:1 lines (nm)")
    ax2.set_ylabel("Aerial-image contrast")
    ax2.set_title("(b) Contrast vs pitch")
    fig.legend([Line2D([], [], color=th.s(i)) for i in (1, 2, 3)],
               [f"obscuration {r['obs']:.2f}: contrast at 60 nm {r['c']:.2f}" for r in d["rows"]],
               loc="lower center", ncol=3, fontsize=S.SMALL_PT)
    fig.subplots_adjust(bottom=0.27)
    return fig


_site("site-schwarzschild-obscuration", schw_compute, schw_draw,
      alt="Image cross-sections of 30 nm lines through a 0.3 NA Schwarzschild objective with central obscuration "
          "0, 0.25 and 0.4, and contrast versus pitch for the three, showing the mid-pitch contrast lost to the "
          "obscuration.",
      caption="13.5 nm through a Schwarzschild objective (the page's example: Sn LPP σ 0.5, NA 0.3, 30 nm lines "
              "on 60 nm) for three central obscurations. Blocking the pupil centre removes some zero-order/first-"
              "order pairs and changes contrast pitch by pitch; images are normalized to the clear field of the "
              "same pupil. Default 3 % flare; scalar mirror reflectance. Model: Schwarzschild objective ✅ "
              "(annular scalar pupil, no figure error).",
      pages=("history/euv.md",))


LIGA_CASES = ((3.0, None, "E$_c$ 3 keV"), (6.23, None, "E$_c$ 6.23 keV"), (10.0, None, "E$_c$ 10 keV"),
              (6.23, [("Al", 20.0)], "E$_c$ 6.23 keV + 20 µm Al"))


def liga_ec_compute(fast):
    huv = native()
    import highuvlith.api as api

    out = []
    for ec, filt, label in LIGA_CASES:
        r = api.simulate_liga(critical_energy_kev=ec, resist_thickness_um=500.0, filters=filt,
                              grid_size=32 if fast else 128, pixel_nm=312.5 if fast else 78.125, nz=32, warn=False)
        z, dose = r.depth_dose
        out.append({"label": label, "z": np.asarray(z), "dose": np.asarray(dose), "ratio": r.dose_ratio,
                    "exceeds": r.exceeds_damage_ceiling})
    return out


def liga_ec_draw(rows, th):
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 4.0))
    S.style_axes(ax, th, grid="both")
    ax.set_yscale("log")
    for slot, r in enumerate(rows, start=1):
        ax.plot(r["z"], r["dose"], color=th.s(slot), zorder=3)
    S.limit_line(ax, 20.0, th, "damage ceiling 20 kJ/cm³", axis="y", where=0.99, side="left", va="bottom")
    S.limit_line(ax, 3.0, th, "clearing dose 3 kJ/cm³", axis="y", where=0.99, side="left", va="top")
    ax.set_xlim(0, 500)
    ax.set_ylim(2, 1000)
    ax.set_xlabel("Depth in PMMA (µm)")
    ax.set_ylabel("Absorbed dose (kJ/cm³)")
    ax.set_title("LIGA: harder spectra expose 500 µm of PMMA more evenly")
    _legend(ax, th, [f"{r['label']}: top/bottom {r['ratio']:.1f}" for r in rows], range(1, 5), loc="upper right",
            fontsize=S.SMALL_PT)
    return fig


_site("site-liga-critical-energy", liga_ec_compute, liga_ec_draw,
      alt="Depth-dose curves through 500 micrometres of PMMA for bending-magnet spectra with critical energies 3, "
          "6.23 and 10 keV and for 6.23 keV behind 20 micrometres of aluminium; softer spectra put far more dose "
          "at the top for the same 3 kJ/cm3 at the bottom.",
      caption="The page's LIGA example as depth-dose curves: bending-magnet spectra of critical energy 3, 6.23 "
              "and 10 keV (and 6.23 keV through a 20 µm Al filter) on 500 µm PMMA behind the default 20 µm Au / "
              "2 µm Ti mask, each scaled so the bottom receives the 3 kJ/cm³ clearing dose. Model: LIGA depth "
              "dose ✅ (NIST μ / μ<sub>en</sub>, collimated beam, no beamline mirrors).",
      pages=("history/parallel-paths.md",))


def interf_talbot_compute(fast):
    huv = native()
    import highuvlith.api as api

    v = api.simulate_interference("two_beam", wavelength_nm=193.0, n_medium=1.7, half_angle_deg=30.0, nx=128, ny=4,
                                  nz=48, x_span_nm=772.0, y_span_nm=100.0, z_span_nm=300.0)
    t = api.simulate_talbot(193.0, 1000.0, max_order=10, n_periods=2, nx=256, carpet_nz=200 if fast else 400)
    return {"pac": np.asarray(v.values)[:, 0, :], "x": np.asarray(v.x_nm), "z": np.asarray(v.z_nm),
            "gap": t.gap_nm,
            "carpet": np.asarray(t.carpet), "cx": np.asarray(t.carpet_x_nm), "cz": np.asarray(t.carpet_z_nm),
            "zt": t.talbot_length_nm, "xi": np.asarray(t.x_nm), "coh": np.asarray(t.coherent_image)[0],
            "dtl": np.asarray(t.stationary_image)[0]}


def interf_talbot_draw(d, th):
    fig = plt.figure(figsize=(S.WIDE_WIDTH + 0.4, 4.0))
    gs = fig.add_gridspec(1, 3, width_ratios=[1, 1.3, 1], wspace=0.55)
    cmap = S.field_cmap()
    ax = fig.add_subplot(gs[0, 0])
    im = ax.imshow(d["pac"], origin="upper", aspect="auto", cmap=cmap, vmin=float(d["pac"].min()),
                   vmax=float(d["pac"].max()),
                   extent=[d["x"][0], d["x"][-1], d["z"][-1], d["z"][0]], interpolation="bilinear")
    S.image_axes(ax, th)
    ax.set_xlabel("x (nm)")
    ax.set_ylabel("Depth in resist (nm)")
    ax.set_title("(a) two beams, 193 nm, 30°: PAC", fontsize=S.SMALL_PT + 0.5)
    S.colorbar(fig, im, ax, th, "PAC m")
    ax2 = fig.add_subplot(gs[0, 1])
    car = d["carpet"]
    ax2.imshow(car.T, origin="lower", aspect="auto", cmap=cmap, vmin=0, vmax=float(np.percentile(car, 99.5)),
               extent=[d["cz"][0] / 1e3, d["cz"][-1] / 1e3, d["cx"][0], d["cx"][-1]], interpolation="nearest")
    S.image_axes(ax2, th)
    ax2.set_xlabel("Distance behind the grating (µm)")
    ax2.set_ylabel("x (nm)")
    ax2.set_title(f"(b) Talbot carpet, 1 µm grating, z_T ≈ {d['zt'] / 1e3:.1f} µm", fontsize=S.SMALL_PT + 0.5)
    ax3 = fig.add_subplot(gs[0, 2])
    S.style_axes(ax3, th, grid="both")
    ax3.plot(d["xi"], d["coh"] / d["coh"].max(), color=th.muted, linewidth=S.HAIR_PT * 1.4)
    ax3.plot(d["xi"], d["dtl"] / d["dtl"].max(), color=th.s(1), zorder=3)
    ax3.set_xlabel("x (nm)")
    ax3.set_ylim(0, 1.45)
    ax3.set_title("(c) normalized images", fontsize=S.SMALL_PT + 0.5)
    ax3.legend([Line2D([], [], color=th.s(1)), Line2D([], [], color=th.muted)],
               ["DTL (gap-averaged)", f"coherent, gap {d['gap'] / 1e3:.1f} µm"], loc="upper center", ncol=1,
               fontsize=S.SMALL_PT - 0.5)
    return fig


_site("site-interference-and-talbot", interf_talbot_compute, interf_talbot_draw,
      alt="Left: a two-beam interference pattern recorded as a photoactive-compound map with a 193 nm fringe "
          "period. Middle: the Talbot carpet behind a 1 micrometre grating lit at 193 nm. Right: the displacement-"
          "Talbot image, which has half the grating period.",
      caption="Lensless patterning at 193 nm. (a) The page's two-beam example (30° half-angle in air, resist "
              "n = 1.7): fringe period λ/(2 sin θ) = 193 nm, shown as the Dill photoactive compound (PAC) through "
              "depth; at the example's default dose the PAC only falls from 1.00 to 0.92 (colour scale stretched). "
              "(b) Self-imaging behind a 1 µm binary amplitude grating, exact angular-spectrum propagation; "
              "(c) averaging over a scanned gap (DTL) gives a stationary image at half the period. Models: "
              "multi-beam interference ✅/🔶 (per-beam scalar absorption, no substrate reflection); Talbot/DTL 🔶 "
              "(scalar thin mask, coherent normal illumination).",
      pages=("history/parallel-paths.md",))


# ===========================================================================
# Process nodes
# ===========================================================================

OAI_TOOLS = (("193i, NA 1.35", "193i", np.arange(60.0, 120.01, 0.5)),
             ("EUV, NA 0.33", "nxe", np.arange(16.0, 40.01, 0.2)),
             ("High-NA EUV, NA 0.55", "hna", np.arange(10.0, 30.01, 0.1)))
OAI_DIPOLE = ("dipole", 0.75, 0.15, 0.0)  # poles on the x axis at sigma 0.75, radius 0.15 (outer 0.9)


def _tool(huv, key):
    if key == "193i":
        return huv.SourceConfig.arf_laser(0.9), huv.OpticsConfig.immersion_193i(), 1.35
    src = huv.SourceConfig.lpp_sn_13nm5(0.9)
    return (src, huv.OpticsConfig.euv_nxe(), 0.33) if key == "nxe" else (src, huv.OpticsConfig.euv_high_na(), 0.55)


def oai_compute(fast):
    huv = native()
    out = []
    for label, key, pitch in OAI_TOOLS:
        if fast:
            pitch = pitch[::4]
        src, opt, na = _tool(huv, key)
        dip = src.with_illumination(OAI_DIPOLE)
        out.append({"label": label, "pitch": pitch, "lam": src.wavelength_nm, "na": na,
                    "conv": np.array([_ls(huv, src, opt, float(p)) for p in pitch]),
                    "dip": np.array([_ls(huv, dip, opt, float(p)) for p in pitch])})
    return out


def oai_draw(rows, th):
    fig, axes = plt.subplots(1, 3, figsize=(S.WIDE_WIDTH + 0.4, 3.7), gridspec_kw={"wspace": 0.25})
    for ax, r in zip(axes, rows):
        S.style_axes(ax, th, grid="both")
        ax.plot(r["pitch"], r["conv"], color=th.s(1), zorder=3)
        ax.plot(r["pitch"], r["dip"], color=th.s(2), zorder=3)
        S.limit_line(ax, 0.5 * r["lam"] / r["na"], th, "k₁ = 0.25".replace("₁", "$_1$"), where=0.97, side="right")
        ax.set_xlim(r["pitch"][0], r["pitch"][-1])
        ax.set_ylim(0, 1)
        ax.set_title(r["label"])
        ax.set_xlabel("Pitch (nm)")
    axes[0].set_ylabel("Aerial-image contrast")
    fig.legend([Line2D([], [], color=th.s(1)), Line2D([], [], color=th.s(2))],
               ["conventional σ 0.9", "x-dipole, poles at σ 0.75 (radius 0.15)"], loc="lower center", ncol=2)
    fig.subplots_adjust(bottom=0.26)
    return fig


_site("site-k1-contrast-vs-pitch-oai", oai_compute, oai_draw,
      alt="Contrast versus pitch on 193 nm immersion, 0.33 NA EUV and 0.55 NA EUV for conventional and dipole "
          "illumination; the dipole keeps contrast down to close to the k1 = 0.25 pitch on each tool, the "
          "conventional source loses it well before.",
      caption="Companion to the k<sub>1</sub>-versus-pitch chart: dense 1:1 lines on the page's three tools "
              "(193i: 193 nm, water NA 1.35; EUV: Sn LPP through the NA 0.33 and 0.55 presets), conventional "
              "σ 0.9 against an x-dipole whose poles sit at σ 0.75. The dipole puts all its light where a two-beam "
              "image forms and keeps contrast to just above the k<sub>1</sub> = 0.25 pitch (dashed); the "
              "conventional disc fades out near λ/(NA·1.9). On the High-NA preset the dipole's contrast falls again above "
              "≈ 22 nm pitch, where first orders from the poles start to land in the assumed 0.2·NA central "
              "obscuration. Thin mask, isotropic pupils (High-NA obscuration "
              "0.2·NA assumed, no anamorphic optics), no resist. Models: scalar Hopkins imaging ✅, illumination "
              "pupil fills ✅ (two-pole dipole), EUV projection optics 🔶, immersion ✅.",
      pages=("nodes/litho-math.md",), requires=("illumination",))


def node_xs_compute(fast):
    huv = native()
    out = {"duv": [], "euv": []}
    src193 = huv.SourceConfig.arf_laser(0.9)
    for pitch in (80.0, 64.0):
        mask = huv.MaskConfig.line_space(pitch / 2, pitch)
        img = _engine(huv, src193, huv.OpticsConfig.immersion_193i(), mask,
                      huv.GridConfig(64, pitch / 32)).compute_aerial_image(0.0)
        x, p = _profile(img)
        out["duv"].append({"label": f"193i NA 1.35, {pitch:.0f} nm pitch", "x": x / pitch, "p": p,
                           "c": img.image_contrast()})
    sn = huv.SourceConfig.lpp_sn_13nm5(0.9)
    for label, opt in (("EUV NA 0.33, 24 nm pitch", huv.OpticsConfig.euv_nxe()),
                       ("EUV NA 0.55, 24 nm pitch", huv.OpticsConfig.euv_high_na())):
        mask = huv.MaskConfig.line_space(12.0, 24.0)
        img = _engine(huv, sn, opt, mask, huv.GridConfig(64, 24.0 / 32)).compute_aerial_image(0.0)
        x, p = _profile(img)
        out["euv"].append({"label": label, "x": x / 24.0, "p": p, "c": img.image_contrast()})
    return out


def node_xs_draw(d, th):
    fig, axes = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 3.9), sharey=True, gridspec_kw={"wspace": 0.08})
    for ax, key, title in ((axes[0], "duv", "(a) 193i, conventional σ 0.9"),
                           (axes[1], "euv", "(b) Sn LPP 13.5 nm, σ 0.9, 24 nm pitch")):
        S.style_axes(ax, th, grid="both")
        for slot, r in enumerate(d[key], start=1):
            ax.plot(r["x"], r["p"], color=th.s(slot), zorder=3)
        ax.set_xlim(d[key][0]["x"][0], d[key][0]["x"][-1])
        ax.set_xlabel("x / pitch")
        ax.set_title(title)
        _legend(ax, th, [f"{r['label']}: contrast {r['c']:.2f}" for r in d[key]], (1, 2), loc="upper center",
                bbox_to_anchor=(0.5, -0.17), fontsize=S.SMALL_PT)
    axes[0].set_ylim(0, None)
    axes[0].set_ylabel("Relative intensity")
    fig.subplots_adjust(bottom=0.32)
    return fig


_site("site-node-cross-sections", node_xs_compute, node_xs_draw,
      alt="Aerial-image cross-sections: on 193 nm immersion an 80 nm pitch keeps only a faint modulation and a 64 nm "
          "pitch none; at 24 nm pitch the 0.33 NA EUV image is weak while the 0.55 NA image is strongly modulated.",
      caption="Where single exposure runs out, as images (two periods each, x in units of the pitch). (a) 193i "
              "with a conventional σ 0.9 source: the 80 nm pitch is just above the λ/(NA·1.9) cut-off and keeps "
              "little contrast, 64 nm has none. (b) 24 nm pitch on the 0.33 and 0.55 NA EUV presets. Production "
              "uses dipole-like illumination near these limits (see the companion chart). Thin mask, no resist. "
              "Models: scalar Hopkins imaging ✅, immersion ✅, EUV projection optics 🔶.",
      pages=("nodes/3nm-and-2nm.md",))


def dof_compute(fast):
    huv = native()
    sn = huv.SourceConfig.lpp_sn_13nm5(0.9)
    focus = np.arange(-120.0, 120.1, 10.0 if fast else 2.0)
    out = {"focus": focus, "a": [], "b": []}
    mask = huv.MaskConfig.line_space(16.0, 32.0)
    for label, opt in (("NA 0.33 (NXE preset)", huv.OpticsConfig.euv_nxe()),
                       ("NA 0.55 (High-NA preset)", huv.OpticsConfig.euv_high_na())):
        eng = _engine(huv, sn, opt, mask, huv.GridConfig(64, 2.0))
        out["a"].append({"label": label, "c": np.array([im.image_contrast() for im in eng.compute_through_focus(list(focus))])})
    for na, pitch in ((0.33, 32.0), (0.55, 19.2)):
        opt = huv.OpticsConfig.euv_projection(numerical_aperture=na)
        m = huv.MaskConfig.line_space(pitch / 2, pitch)
        eng = _engine(huv, sn, opt, m, huv.GridConfig(128, pitch / 16))
        out["b"].append({"label": f"NA {na:.2f}, {pitch:g} nm pitch",
                         "c": np.array([im.image_contrast() for im in eng.compute_through_focus(list(focus))])})
    return out


def dof_draw(d, th):
    fig, axes = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 3.9), sharey=True, gridspec_kw={"wspace": 0.08})
    z = d["focus"]
    for ax, key, title in ((axes[0], "a", "(a) 32 nm pitch on both presets"),
                           (axes[1], "b", "(b) equal k₁ ≈ 0.39, unobscured pupils".replace("₁", "$_1$"))):
        S.style_axes(ax, th, grid="both")
        for slot, r in enumerate(d[key], start=1):
            ax.plot(z, r["c"], color=th.s(slot), zorder=3)
        ax.set_xlim(z[0], z[-1])
        ax.set_xlabel("Focus (nm)")
        ax.set_title(title)
        _legend(ax, th, [r["label"] for r in d[key]], (1, 2), loc="upper center", bbox_to_anchor=(0.5, -0.17),
                ncol=2, fontsize=S.SMALL_PT)
    axes[0].set_ylim(0, 1)
    axes[0].set_ylabel("Aerial-image contrast")
    fig.subplots_adjust(bottom=0.26)
    return fig


_site("site-euv-depth-of-focus", dof_compute, dof_draw,
      alt="Contrast versus focus for 0.33 and 0.55 NA EUV: at the same 32 nm pitch the 0.55 NA image starts higher "
          "but fades faster with defocus; at equal k1 both start near 0.5 and the 0.55 NA image fades about three "
          "times faster.",
      caption="The depth-of-focus price of 0.55 NA (Sn LPP, conventional σ 0.9). (a) The angstrom-era page's "
              "example: a 32 nm pitch through the NXE and High-NA presets. (b) The High-NA page's example: "
              "equal k<sub>1</sub> (32 nm at 0.33, 19.2 nm at 0.55) through ideal unobscured pupils. The engine "
              "applies the exact non-paraxial defocus phase per source point; image in air, no resist, no mask-3D "
              "best-focus shifts. Models: defocus / through-focus imaging ✅, EUV projection optics 🔶.",
      pages=("nodes/angstrom-era.md", "future/high-na-and-hyper-na.md"))


def sadp_compute(fast):
    huv = native()
    import highuvlith.api as api

    w = np.arange(26.0, 38.01, 0.25)
    sadp = [api.sadp(128.0, float(x), 32.0) for x in w]
    dl = np.arange(-4.0, 4.01, 0.25)
    saqp = [api.saqp(128.0, 48.0 + float(x), 16.0, 16.0) for x in dl]
    cart = {f"SADP, W = {x:g} nm": api.sadp(128.0, x, 32.0).lines for x in (32.0, 36.0)}
    cart.update({f"SAQP, W = {48 + x:g} nm": api.saqp(128.0, 48.0 + x, 16.0, 16.0).lines for x in (0.0, 2.0)})
    return {"w": w, "walk": np.array([r.pitch_walk_nm for r in sadp]), "dl": dl,
            "spaces": np.array([sorted(r.spaces_nm) for r in saqp]), "saqp_walk": np.array([r.pitch_walk_nm for r in saqp]),
            "cart": {k: [tuple(map(float, l)) for l in v] for k, v in cart.items()}}


def sadp_draw(d, th):
    fig = plt.figure(figsize=(S.WIDE_WIDTH, 4.6))
    gs = fig.add_gridspec(2, 2, height_ratios=[1.4, 1], hspace=0.6, wspace=0.28)
    ax = fig.add_subplot(gs[0, 0])
    S.style_axes(ax, th, grid="both")
    ax.plot(d["w"], d["walk"], color=th.s(1), zorder=3)
    ax.set_xlabel("Mandrel CD W (nm)")
    ax.set_ylabel("Pitch walk (nm)")
    ax.set_title("(a) SADP, P = 128 nm, t = 32 nm")
    ax2 = fig.add_subplot(gs[0, 1])
    S.style_axes(ax2, th, grid="both")
    sp = d["spaces"]
    for k in range(sp.shape[1]):
        ax2.plot(48 + d["dl"], sp[:, k], color=th.s(2), linewidth=S.LINE_PT * 0.8, zorder=3)
    ax2.set_xlabel("Mandrel CD W (nm)")
    ax2.set_ylabel("Spaces (nm)")
    ax2.set_title("(b) SAQP, P = 128 nm, t₁ = t₂ = 16 nm".replace("₁", "$_1$").replace("₂", "$_2$"))
    ax3 = fig.add_subplot(gs[1, :])
    names = list(d["cart"])
    for i, name in enumerate(names):
        for a, b in d["cart"][name]:
            ax3.add_patch(plt.Rectangle((a, i - 0.35), b - a, 0.7, color=th.s(1 if "SADP" in name else 2), lw=0))
    ax3.set_yticks(range(len(names)))
    ax3.set_yticklabels(names, fontsize=S.SMALL_PT)
    ax3.set_xlim(0, 128)
    ax3.set_ylim(len(names) - 0.5, -0.5)
    ax3.set_xlabel("x across one mandrel period (nm)")
    S.style_axes(ax3, th, grid=None)
    ax3.set_title("(c) printed lines over one 128 nm mandrel period")
    return fig


_site("site-sadp-saqp-pitch-walk", sadp_compute, sadp_draw,
      alt="Self-aligned double and quadruple patterning geometry: SADP pitch walk grows linearly with mandrel-width "
          "error, SAQP spaces split into different values when the mandrel width departs from 48 nm, and cartoons "
          "of the resulting lines.",
      caption="Pitch walk from the page's SADP/SAQP example: <code>api.sadp(128, W, 32)</code> walk = |2W + 2t − P| "
              "(zero at W = 32 nm) and the SAQP spaces from <code>api.saqp(128, W, 16, 16)</code>, uniform 16 nm at "
              "W = 48 nm. (c) The line positions over one mandrel period. Model: multiple patterning 🔶 — SADP/SAQP "
              "purely geometric (ideal conformal spacers, no deposition or etch physics).",
      pages=("nodes/22-to-10nm.md",))


# ===========================================================================
# Future
# ===========================================================================

CONES = (("NXE 0.33, 4×", 0.33, 4), ("0.55 at 4×", 0.55, 4), ("EXE 0.55, 8× direction", 0.55, 8))


def cone_compute(fast):
    huv = native()
    ang = np.linspace(0.0, 20.0, 81 if fast else 401)
    out = {"ang": ang, "cones": [], "mirrors": {}}
    for label, na, mag in CONES:
        half = math.degrees(math.asin(na / mag))
        cra = max(6.0, half)
        key = f"{cra:.2f}"
        if key not in out["mirrors"]:
            d = huv.tune_multilayer_period("mo_si", 13.5, 40, angle_deg=cra)
            ml = huv.MultilayerMirror.mo_si(period_nm=d)
            out["mirrors"][key] = {"cra": cra, "d": d,
                                   "r": np.array([ml.reflectance(13.5, angle_deg=float(a)) for a in ang])}
            mirror = ml
        else:
            mirror = huv.MultilayerMirror.mo_si(period_nm=out["mirrors"][key]["d"])
        lo, hi = (mirror.reflectance(13.5, angle_deg=a) for a in (cra - half, cra + half))
        out["cones"].append({"label": label, "half": half, "cra": cra, "key": key, "lo": lo, "hi": hi})
    return out


def cone_draw(d, th):
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 4.2))
    S.style_axes(ax, th, grid="both")
    keys = list(d["mirrors"])
    for slot, k in enumerate(keys, start=1):
        m = d["mirrors"][k]
        ax.plot(d["ang"], m["r"], color=th.s(slot), zorder=3)
    for i, c in enumerate(d["cones"]):
        slot = keys.index(c["key"]) + 1
        y = 0.05 + 0.1 * i
        a0, a1 = max(0.0, c["cra"] - c["half"]), c["cra"] + c["half"]
        ax.plot([a0, a1], [y, y], color=th.s(slot), linewidth=S.LINE_PT * 2.2, solid_capstyle="butt")
        ax.text(a0, y + 0.018, f"{c['label']}: R {c['lo']:.2f} … {c['hi']:.2f}", va="bottom", color=th.ink2,
                fontsize=S.SMALL_PT)
    ax.set_xlim(0, 20)
    ax.set_ylim(0, 0.8)
    ax.set_xlabel("Angle of incidence on the mask (°)")
    ax.set_ylabel("Reflectance at 13.5 nm (unpolarized)")
    ax.set_title("Ideal Mo/Si mask blank vs the mask-side cone of light")
    _legend(ax, th, [f"period tuned for a {d['mirrors'][k]['cra']:.1f}° chief ray" for k in keys],
            range(1, len(keys) + 1), loc="upper right")
    return fig


_site("site-mask-cone-reflectance", cone_compute, cone_draw,
      alt="Reflectance of an ideal molybdenum-silicon mask blank versus angle of incidence, with bars marking the "
          "range of angles each scanner design sends to the mask; at 0.55 NA with 4x reduction the range runs into "
          "the steep fall-off, with 8x it stays on the plateau.",
      caption="Why the High-NA mask wants 8× in one direction (the High-NA page's example): reflectance of an "
              "ideal 40-period Mo/Si blank at 13.5 nm, period tuned for the chief-ray angle (max(6°, cone "
              "half-angle)), against the mask-side cone half-angle asin(NA/M). Bars show each design's cone and "
              "the reflectance at its two edges. Ideal multilayer: no absorber, capping or roughness, and the "
              "single worst ray — not a mask-3D calculation. Model: multilayer mirrors ✅.",
      pages=("future/high-na-and-hyper-na.md",))


def beuv_compute(fast):
    huv = native()
    opt = huv.OpticsConfig.euv_projection(numerical_aperture=0.33)
    pitch = np.arange(8.0, 40.01, 0.5 if fast else 0.2)
    out = {"pitch": pitch, "rows": []}
    for factory in ("lpp_sn_13nm5", "lpp_gd_6nm7"):
        src = getattr(huv.SourceConfig, factory)(0.9)
        mask = huv.MaskConfig.line_space(10.0, 20.0)
        img = _engine(huv, src, opt, mask, huv.GridConfig(128, 20.0 / 16)).compute_aerial_image(0.0)
        x, p = _profile(img)
        out["rows"].append({"lam": src.wavelength_nm, "x": x, "p": p, "c": img.image_contrast(),
                            "cvp": np.array([_ls(huv, src, opt, float(pp)) for pp in pitch])})
    return out


def beuv_draw(d, th):
    fig, (ax, ax2) = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 3.8), gridspec_kw={"wspace": 0.28})
    S.style_axes(ax, th, grid="both")
    S.style_axes(ax2, th, grid="both")
    for slot, r in enumerate(d["rows"], start=1):
        sel = np.abs(r["x"]) <= 20.0
        ax.plot(r["x"][sel], r["p"][sel], color=th.s(slot), zorder=3)
        ax2.plot(d["pitch"], r["cvp"], color=th.s(slot), zorder=3)
        ax2.axvline(r["lam"] / (0.33 * 1.9), color=th.s(slot), linewidth=S.HAIR_PT * 1.2, linestyle=(0, (3, 3)))
    ax.set_xlim(-20, 20)
    ax.set_ylim(0, None)
    ax.set_xlabel("x (nm)")
    ax.set_ylabel("Relative intensity")
    ax.set_title("(a) 10 nm lines, 20 nm pitch")
    ax2.set_xlim(d["pitch"][0], d["pitch"][-1])
    ax2.set_ylim(0, 1)
    ax2.set_xlabel("Pitch (nm)")
    ax2.set_ylabel("Aerial-image contrast")
    ax2.set_title("(b) contrast vs pitch (dotted: λ/(NA·1.9))")
    fig.legend([Line2D([], [], color=th.s(i)) for i in (1, 2)],
               [f"{r['lam']:.1f} nm (contrast at 20 nm pitch {r['c']:.2f})" for r in d["rows"]],
               loc="lower center", ncol=2)
    fig.subplots_adjust(bottom=0.25)
    return fig


_site("site-beuv-20nm-pitch", beuv_compute, beuv_draw,
      alt="A 20 nm pitch grating through the same ideal NA 0.33 optics at 13.5 nm and at 6.7 nm: flat at 13.5 nm, "
          "well modulated at 6.7 nm; contrast versus pitch shows the cut-off halving with the wavelength.",
      caption="The same optics at 13.5 nm and 6.7 nm (the Beyond-EUV page's example): Sn and Gd plasma presets "
              "(conventional σ 0.9) through an ideal, unobscured NA 0.33 <code>euv_projection</code> pupil. No "
              "BEUV projection lens exists; mirror reflectance, flare and mask-3D are not included, and the Gd "
              "source power is a projection. Models: scalar Hopkins imaging ✅, EUV projection optics 🔶, Gd LPP "
              "source 🧪 (power).",
      pages=("future/beyond-euv.md",))


POWER_PRESETS = (
    ("Sn LPP, NXE:3400B class", "lpp_sn_13nm5", (0.9,), "shipping"),
    ("Sn LPP, NXE:3800E class", "lpp_sn_13nm5_500w", (0.9,), "shipping"),
    ("Sn discharge plasma (at IF)", "dpp_sn_13nm5", (), "demonstrated"),
    ("FLASH-like SASE FEL", "xfel_flash_13nm5", (), "demonstrated"),
    ("FERMI-like seeded FEL", "xfel_fermi_seeded", (), "demonstrated"),
    ("CW-SC linac FEL", "xfel_cw_sc_13nm5", (), "projection"),
    ("ERL-FEL", "xfel_erl_13nm5", (), "projection"),
    ("SSMB ring", "ssmb_euv_13nm5", (), "projection"),
    ("compact undulator", "synchrotron_compact_euv", (), "demonstrated"),
    ("ICS design point", "ics_compact_euv_13nm5", (), "projection"),
    ("LPA-FEL target", "lpa_fel_bella_25nm", (0.7,), "projection"),
    ("HHG, neon", "hhg_ne_13nm5", (), "demonstrated"),
    ("soft-X-ray laser, Ag", "sxrl_ag_13nm9", (), "demonstrated"),
    ("Smith–Purcell", "smith_purcell", (), "projection"),
)


def power_compute(fast):
    huv = native()
    rows = []
    for label, factory, args, status in POWER_PRESETS:
        src = getattr(huv.SourceConfig, factory)(*args)
        rows.append({"label": label, "p": float(src.average_power_w), "lam": src.wavelength_nm, "status": status})
    return rows


def power_draw(rows, th):
    from common import fmt_si

    rows = sorted(rows, key=lambda r: r["p"])
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 5.4))
    S.style_axes(ax, th, grid="x")
    y = np.arange(len(rows))
    for i, r in enumerate(rows):
        filled = r["status"] != "projection"
        col = th.s(1) if filled else th.s(2)
        ax.barh(i, r["p"], left=0, height=0.6, color=col if filled else "none", edgecolor=col,
                linewidth=S.HAIR_PT * 1.5)
        ax.text(r["p"] * 1.4, i, fmt_si(r["p"], "W"), va="center", color=th.ink2, fontsize=S.SMALL_PT)
    ax.set_xscale("log")
    ax.set_xlim(1e-10, 1e6)
    ax.set_yticks(y)
    ax.set_yticklabels([f"{r['label']} ({r['lam']:.1f} nm)" for r in rows], fontsize=S.SMALL_PT)
    ax.axvspan(250.0, 1000.0, color=th.band, zorder=0, lw=0)
    ax.set_xlabel("average_power_w of the preset (W); shaded: 250 W – 1 kW")
    ax.set_title("Simulator source presets against the 250 W – 1 kW HVM band")
    ax.legend([plt.Rectangle((0, 0), 1, 1, color=th.s(1)),
               plt.Rectangle((0, 0), 1, 1, facecolor="none", edgecolor=th.s(2))],
              ["anchored to a shipping or demonstrated machine", "projection / design point"], loc="upper center",
              bbox_to_anchor=(0.4, -0.12), ncol=2, fontsize=S.SMALL_PT)
    fig.subplots_adjust(bottom=0.18)
    return fig


_site("site-preset-power-bars", power_compute, power_draw,
      alt="Horizontal log-scale bars of the average power reported by each EUV-range source preset, from below a "
          "nanowatt to above ten kilowatts, with the 250 W and 1 kW high-volume-manufacturing lines; only the tin "
          "plasma presets and the accelerator projections reach the band.",
      caption="Every EUV-range source preset's <code>average_power_w</code> against the 250 W – 1 kW band "
              "(250 W at IF on the NXE:3400B chain; 1 kW the stated target). Filled bars are anchored to a "
              "shipping or demonstrated machine class, open bars are projections or design points (as labelled "
              "in each preset's documentation). Powers are defined at different planes (IF, undulator exit, laser "
              "output), so compare orders of magnitude. Source models range from ✅ to 🧪 — see each family page.",
      pages=("future/accelerator-light-sources.md",))


def ssmb_compute(fast):
    huv = native()
    b = np.linspace(0.005, 0.2, 40 if fast else 196)
    p, sig = [], []
    for x in b:
        s = huv.SourceConfig.ssmb(bunching_factor=float(x))
        p.append(s.average_power_w)
        sig.append(s.derived_quantity("microbunch_rms_length"))
    pre = huv.SourceConfig.ssmb_euv_13nm5()
    return {"b": b, "p": np.array(p), "sigma": np.array(sig), "b_hvm": pre.derived_quantity("bunching_for_hvm"),
            "b_proj": pre.derived_quantity("bunching_for_projection"), "p_proj": pre.average_power_w}


def ssmb_draw(d, th):
    fig, (ax, ax2) = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 3.8), gridspec_kw={"wspace": 0.3})
    for a in (ax, ax2):
        S.style_axes(a, th, grid="both")
        a.set_xlim(0, 0.2)
        a.set_xlabel("Bunching factor b at 13.5 nm")
    ax.plot(d["b"], d["p"], color=th.s(1), zorder=3)
    for v, lab in ((250.0, "250 W"), (1000.0, "1 kW")):
        S.limit_line(ax, v, th, lab, axis="y", where=0.02, side="right", va="bottom")
    S.dot(ax, [d["b_hvm"], d["b_proj"]], [250.0, d["p_proj"]], th.s(2), th)
    ax.annotate(f"b ≈ {d['b_hvm']:.3f}", (d["b_hvm"], 250.0), xytext=(6, -14), textcoords="offset points",
                color=th.ink2, fontsize=S.SMALL_PT)
    ax.annotate(f"preset: b ≈ {d['b_proj']:.3f}", (d["b_proj"], d["p_proj"]), xytext=(-8, 8),
                textcoords="offset points", ha="right", color=th.ink2, fontsize=S.SMALL_PT)
    ax.set_ylim(0, 2000)
    ax.set_ylabel("Coherent power (W)")
    ax.set_title("(a) P ∝ b² (1 A, 100-period K = 1.6 radiator)")
    ax2.plot(d["b"], d["sigma"], color=th.s(1), zorder=3)
    ax2.set_ylabel("Microbunch rms length (nm)")
    ax2.set_title("(b) bunch length needed for b")
    return fig


_site("site-ssmb-bunching", ssmb_compute, ssmb_draw,
      alt="Steady-state microbunching: coherent EUV power rising as the square of the bunching factor, reaching "
          "250 W near b = 0.073 and 1 kW near 0.146, and the microbunch length that bunching factor implies, a "
          "few nanometres.",
      caption="What SSMB's kilowatt assumes (the accelerator page's example): coherent power of the "
              "<code>ssmb()</code> model versus the bunching factor b at 13.5 nm, P ∝ b² for the 1 A, 100-period, "
              "K = 1.6 radiator, and the rms microbunch length that b implies for a Gaussian bunch, "
              "b = exp(−k²σ²/2). The 1 kW preset needs b ≈ 0.146 — nanometre microbunches on every turn, which "
              "has not been demonstrated. Model: SSMB 🧪.",
      pages=("future/accelerator-light-sources.md",))


WPH_SOURCES = (("Sn LPP ~250 W", "lpp_sn_13nm5", (0.9,), 1), ("SSMB 1 kW (projection)", "ssmb_euv_13nm5", (), 1),
               ("ERL-FEL shared by 16 scanners", "xfel_erl_13nm5", (), 16))


def wph_compute(fast):
    huv = native()
    dose = np.arange(10.0, 100.1, 5.0 if fast else 1.0)
    out = {"dose": dose, "rows": []}
    for label, factory, args, share in WPH_SOURCES:
        src = getattr(huv.SourceConfig, factory)(*args)
        w = []
        for dd in dose:
            t = src.wafer_throughput(dose_mj_cm2=float(dd))
            w.append(3600 / (t["total_time_per_wafer_s"] + t["exposure_time_per_wafer_s"] * (share - 1)))
        out["rows"].append({"label": label, "wph": np.array(w)})
    return out


def wph_draw(d, th):
    fig, ax = plt.subplots(figsize=(S.FULL_WIDTH, 3.9))
    S.style_axes(ax, th, grid="both")
    for slot, r in enumerate(d["rows"], start=1):
        ax.plot(d["dose"], r["wph"], color=th.s(slot), zorder=3)
    ax.set_xlim(d["dose"][0], d["dose"][-1])
    ax.set_ylim(0, 200)
    ax.set_xlabel("Resist dose (mJ/cm²)")
    ax.set_ylabel("Wafers per hour per scanner")
    ax.set_title("Dose-limited throughput (10 mirrors × 0.70, mask 0.65, 84 fields)")
    _legend(ax, th, [r["label"] for r in d["rows"]], (1, 2, 3), loc="lower left")
    return fig


_site("site-wph-vs-dose", wph_compute, wph_draw,
      alt="Wafers per hour versus resist dose for a 250 W tin source, a 1 kW SSMB projection and an ERL free-"
          "electron laser shared by 16 scanners; the tin source falls from about 154 to 103 wafers per hour "
          "between 30 and 100 mJ/cm2 while the others stay near the overhead-limited ceiling.",
      caption="Power becomes throughput (the accelerator page's example, swept over dose): "
              "<code>wafer_throughput()</code> with its defaults (ten mirrors at 0.70, mask 0.65, 84 fields, 0.1 s "
              "per field and 10 s per wafer). The ERL curve spreads one source over 16 scanners by multiplying "
              "each wafer's exposure time by 16 — the page's back-of-envelope sharing, not a feature of the code. "
              "Model: dose-limited throughput 🔶; SSMB and ERL powers are projections.",
      pages=("future/accelerator-light-sources.md",))


def quantum_compute(fast):
    import highuvlith.api as api

    native()
    r = api.simulate_quantum_line_space(120.0, 240.0, n=2, fidelity=1.0, grid_size=128, pixel_nm=3.75)
    cl, qu = np.asarray(r.classical), np.asarray(r.quantum)
    row_c, row_q = cl[cl.shape[0] // 2], qu[qu.shape[0] // 2]
    x = (np.arange(row_c.size) - row_c.size / 2) * 3.75
    spec = {k: np.abs(np.fft.rfft(v - v.mean())) for k, v in (("c", row_c), ("q", row_q))}
    return {"x": x, "c": row_c, "q": row_q, "cc": r.classical_contrast, "qc": r.quantum_contrast, "spec": spec,
            "field": row_c.size * 3.75}


def quantum_draw(d, th):
    fig, (ax, ax2) = plt.subplots(1, 2, figsize=(S.WIDE_WIDTH, 3.8), gridspec_kw={"width_ratios": [1.6, 1],
                                                                                    "wspace": 0.3})
    S.style_axes(ax, th, grid="both")
    ax.plot(d["x"], d["c"] / d["c"].max(), color=th.s(1), zorder=3)
    ax.plot(d["x"], d["q"] / d["q"].max(), color=th.s(2), zorder=3)
    ax.set_xlim(d["x"][0], d["x"][-1])
    ax.set_ylim(0, 1.05)
    ax.set_xlabel("x (nm)")
    ax.set_ylabel("Normalized exposure")
    ax.set_title("(a) 120 nm lines, 240 nm pitch, 157.63 nm, NA 0.75")
    _legend(ax, th, [f"classical I (contrast {d['cc']:.3f})", f"I² model (contrast {d['qc']:.3f})"], (1, 2),
            loc="upper center", fontsize=S.SMALL_PT)
    S.style_axes(ax2, th, grid="y")
    k = np.arange(1, 7)
    w = 0.38
    for off, key, slot in ((-w / 2, "c", 1), (w / 2, "q", 2)):
        sp = d["spec"][key]
        ax2.bar(k + off, sp[k] / sp[1:].max(), width=w, color=th.s(slot))
    ax2.set_xticks(k)
    ax2.set_xticklabels([f"{d['field'] / kk:.0f}" for kk in k], fontsize=S.SMALL_PT - 0.5)
    ax2.set_xlabel("Period of the Fourier component (nm)")
    ax2.set_ylabel("Relative amplitude")
    ax2.set_title("(b) spectrum: strongest at 240 nm for both")
    return fig


_site("site-quantum-i2-period", quantum_compute, quantum_draw,
      alt="Classical and two-photon (I squared) exposure profiles of a 240 nm pitch grating: the I squared profile "
          "is sharper, but its Fourier spectrum still peaks at the 240 nm period; no finer period appears.",
      caption="Sharpening without a finer period (the quantum page's example): the classical aerial image "
              "(F<sub>2</sub> 157.63 nm, NA 0.75, σ 0.7) and the N = 2 post-step I² at fidelity 1. Contrast rises, "
              "but the dominant Fourier component of both images is the 240 nm mask period — the I² model is a "
              "sharpening proxy, not λ/N resolution. Model: quantum lithography 🧪 (classical N-photon "
              "absorption; the ideal N00N limit is a separate model).",
      pages=("future/quantum-and-exotic.md",))


def talbot_par_compute(fast):
    import highuvlith.api as api

    native()
    out = {}
    for prop in ("exact", "paraxial"):
        r = api.simulate_talbot(13.5, 100.0, max_order=10, n_periods=2, nx=128, propagation=prop,
                                carpet_nz=128 if fast else 400, carpet_z_max_nm=2 * 2 * 100.0 ** 2 / 13.5)
        out[prop] = {"carpet": np.asarray(r.carpet), "x": np.asarray(r.carpet_x_nm), "z": np.asarray(r.carpet_z_nm),
                     "zt": r.talbot_length_nm, "zte": r.talbot_length_exact_nm,
                     "coh": np.asarray(r.coherent_image)[0], "dtl": np.asarray(r.stationary_image)[0],
                     "xi": np.asarray(r.x_nm)}
    return out


def talbot_par_draw(d, th):
    fig = plt.figure(figsize=(S.WIDE_WIDTH + 0.4, 4.0))
    gs = fig.add_gridspec(1, 3, width_ratios=[1.2, 1.2, 1], wspace=0.45)
    e, p = d["exact"], d["paraxial"]
    zt = e["zt"]
    ext = [e["z"][0] / zt, e["z"][-1] / zt, e["x"][0], e["x"][-1]]
    ax = fig.add_subplot(gs[0, 0])
    im = ax.imshow(e["carpet"].T, origin="lower", aspect="auto", cmap=S.field_cmap(), vmin=0,
                   vmax=float(np.percentile(e["carpet"], 99.5)), interpolation="nearest", extent=ext)
    S.colorbar(fig, im, ax, th, "Intensity")
    ax.set_ylabel("x (nm)")
    ax.set_title("(a) exact angular spectrum", fontsize=S.SMALL_PT + 0.5)
    ax2 = fig.add_subplot(gs[0, 1])
    diff = e["carpet"] - p["carpet"]
    lim = float(np.percentile(np.abs(diff), 99.5))
    im2 = ax2.imshow(diff.T, origin="lower", aspect="auto", cmap=S.diverging_cmap(th), vmin=-lim, vmax=lim,
                     interpolation="nearest", extent=ext)
    S.colorbar(fig, im2, ax2, th, "exact − paraxial")
    ax2.tick_params(labelleft=False)
    ax2.set_title("(b) difference from the paraxial carpet", fontsize=S.SMALL_PT + 0.5)
    for a_ in (ax, ax2):
        S.image_axes(a_, th)
        for m in (1.0, 2.0):
            a_.axvline(m, color=th.axis, linewidth=S.HAIR_PT, linestyle=(0, (3, 3)))
        a_.set_xlabel("z / (2p²/λ)")
    ax3 = fig.add_subplot(gs[0, 2])
    S.style_axes(ax3, th, grid="both")
    e = d["exact"]
    ax3.plot(e["xi"], e["coh"] / e["coh"].max(), color=th.muted, linewidth=S.HAIR_PT * 1.4)
    ax3.plot(e["xi"], e["dtl"] / e["dtl"].max(), color=th.s(1), zorder=3)
    ax3.set_ylim(0, 1.45)
    ax3.set_xlabel("x (nm)")
    ax3.set_title("(c) DTL vs coherent row", fontsize=S.SMALL_PT + 0.5)
    ax3.legend([Line2D([], [], color=th.s(1)), Line2D([], [], color=th.muted)],
               ["DTL (stationary)", "coherent, at the gap"], loc="upper center", fontsize=S.SMALL_PT - 0.5)
    return fig


_site("site-talbot-exact-vs-paraxial", talbot_par_compute, talbot_par_draw,
      alt="The Talbot carpet behind a 100 nm grating at 13.5 nm from exact propagation and its difference from "
          "the paraxial carpet, which is large in the fine structure carried by the "
          "higher orders; beside them the DTL image with half the period and the coherent row.",
      caption="Talbot carpets of a 100 nm binary amplitude grating at 13.5 nm (the alternative-patterning page's "
              "example), ±10 orders: (a) exact angular-spectrum propagation, (b) its difference from the paraxial "
              "(Fresnel) carpet, both in units of the paraxial Talbot length 2p²/λ ≈ 1481 nm (dashed). With "
              "p/λ ≈ 7 the higher orders dephase in the exact propagation, so the fine structure differs strongly "
              "even though the 0/±1 self-image only moves to ≈ 1475 nm. (c) The "
              "stationary DTL image at half the period. Model: Talbot / DTL 🔶 (scalar thin mask, coherent normal "
              "illumination).",
      pages=("future/alternative-patterning.md",))
