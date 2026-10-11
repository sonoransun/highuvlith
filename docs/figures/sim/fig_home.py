"""Home-page figure: "Five generations, one pattern" (DESIGN §5.2 I1).

Five engine-computed aerial images of equal lines and spaces, one per exposure
generation, all at the same k₁ = 0.35 under an x-dipole, so the tiles look alike while
the pitch they print shrinks from 217 nm (KrF, NA 0.80) to 17.2 nm (High-NA EUV, NA 0.55).
The wavelengths and numerical apertures are the published ones; nothing here is a model of
a particular tool. Every number printed on the figure comes from the engine call that drew it.
"""

from __future__ import annotations

import numpy as np
from matplotlib import patheffects
from matplotlib import pyplot as plt
from matplotlib.gridspec import GridSpec

import style as S
from common import FigureSpec, native, register

K1 = 0.35
#: x-dipole: pole centre at σ_c = 1/(4 k₁) (two-beam geometry: 0 and −1 orders symmetric about the axis), pole radius 0.15
DIPOLE_SIGMA_C = 1.0 / (4.0 * K1)
DIPOLE_RADIUS = 0.15
#: Published wavelength / NA pairs; the engine factories supply λ, the optics presets supply NA and n.
GENERATIONS = (
    # key, label, source factory, optics factory
    ("krf", "KrF", lambda huv: huv.SourceConfig.krf_laser(0.9), lambda huv: huv.OpticsConfig(numerical_aperture=0.80, flare_fraction=0.0)),
    ("arf", "ArF dry", lambda huv: huv.SourceConfig.arf_laser(0.9), lambda huv: huv.OpticsConfig(numerical_aperture=0.93, flare_fraction=0.0)),
    ("arfi", "ArF immersion", lambda huv: huv.SourceConfig.arf_laser(0.9),
     lambda huv: huv.OpticsConfig.immersion(numerical_aperture=1.35, immersion_index=1.437, flare_fraction=0.0)),
    ("euv", "EUV", lambda huv: huv.SourceConfig.lpp_sn_13nm5(0.9), lambda huv: huv.OpticsConfig.euv_nxe()),
    ("hna", "High-NA EUV", lambda huv: huv.SourceConfig.lpp_sn_13nm5(0.9), lambda huv: huv.OpticsConfig.euv_high_na()),
)
#: Largest allowed spread of the five contrasts (DESIGN §5.2 I1): the tiles must look alike.
CONTRAST_SPREAD_MAX = 0.15


def generations_compute(fast: bool):
    huv = native()
    size = 64 if fast else 128
    tiles = []
    for key, label, make_source, make_optics in GENERATIONS:
        src = make_source(huv).with_illumination("dipole", DIPOLE_SIGMA_C, DIPOLE_RADIUS, 0.0)
        optics = make_optics(huv)
        lam, na = src.wavelength_nm, optics.numerical_aperture
        pitch = 2.0 * K1 * lam / na  # half-pitch = k₁ λ / NA
        mask = huv.MaskConfig.line_space(pitch / 2, pitch)
        grid = mask.commensurate_grid(size, 2.0 * pitch / size)  # exactly two periods per tile
        if abs(grid.periods_in_field(pitch) - 2.0) > 1e-9:
            raise AssertionError(f"{key}: the field holds {grid.periods_in_field(pitch)} periods, not 2")
        eng = huv.SimulationEngine(src, optics, mask, grid=grid, max_kernels=64)
        img = eng.compute_aerial_image(0.0)
        intensity = np.array(img.intensity, dtype=float)
        tiles.append({
            "key": key, "label": label, "wavelength_nm": lam, "na": na, "index": optics.immersion_index,
            "obscuration": optics.central_obscuration, "pitch_nm": pitch, "half_pitch_nm": pitch / 2,
            "field_nm": grid.field_size_nm(), "pixel_nm": grid.pixel_nm, "intensity": intensity,
            "contrast": img.image_contrast(), "kernels": eng.num_kernels(), "source_points": len(eng.source_points()[0]),
        })
    contrasts = np.array([t["contrast"] for t in tiles])
    spread = float(contrasts.max() - contrasts.min())
    if spread > CONTRAST_SPREAD_MAX:
        raise AssertionError(f"five-generation contrasts spread {spread:.3f} > {CONTRAST_SPREAD_MAX} ({contrasts.round(3)})")
    ratio = tiles[0]["pitch_nm"] / tiles[-1]["pitch_nm"]
    if not 12.0 < ratio < 13.2:
        raise AssertionError(f"pitch ratio KrF/High-NA is {ratio:.2f}, expected ≈ 12.6")
    return {"tiles": tiles, "spread": spread, "ratio": ratio, "k1": K1, "sigma_c": DIPOLE_SIGMA_C}


def _fmt_nm(v: float) -> str:
    return f"{v:.0f}" if v >= 100 else f"{v:.1f}"


def generations_draw(d, th: S.Theme):
    import tokens  # noqa: PLC0415

    tiles = d["tiles"]
    fig = plt.figure(figsize=(S.WIDE_WIDTH, 3.3))
    gs = GridSpec(1, len(tiles), figure=fig, wspace=0.12, left=0.012, right=0.988, top=0.76, bottom=0.17)
    cmap = S.field_cmap()
    vmax = max(float(t["intensity"].max()) for t in tiles)
    for k, t in enumerate(tiles):
        ax = fig.add_subplot(gs[0, k])
        half = t["field_nm"] / 2
        ax.imshow(t["intensity"], origin="lower", extent=[-half, half, -half, half], cmap=cmap, vmin=0, vmax=vmax,
                  interpolation="bilinear")
        ax.set_xlim(-half, half)
        ax.set_ylim(-half, half)
        ax.set_aspect("equal")
        ax.set_xticks([])
        ax.set_yticks([])
        for side in ("top", "right", "left", "bottom"):
            ax.spines[side].set_visible(False)
        # spectrum-coloured rule above the tile, in the wavelength's colour
        colour = tokens.wavelength_color(t["wavelength_nm"])
        ax.plot([0, 1], [1.045, 1.045], transform=ax.transAxes, color=colour, linewidth=3.0, solid_capstyle="butt",
                clip_on=False)
        medium = f", n = {t['index']:.3f}" if t["index"] > 1.0 else ""
        ax.text(0.0, 1.235, t["label"], transform=ax.transAxes, ha="left", va="bottom", color=th.ink, fontsize=S.BASE_PT,
                fontweight="semibold")
        ax.text(0.0, 1.10, f"{_fmt_nm(t['wavelength_nm'])} nm · NA {t['na']:.2f}{medium}", transform=ax.transAxes,
                ha="left", va="bottom", color=th.ink2, fontsize=S.SMALL_PT)
        # scale bar = one pitch: the same drawn length in every tile, a label that shrinks 12.6x
        p = t["pitch_nm"]
        y_bar = -half + 0.075 * t["field_nm"]
        ax.plot([-p / 2, p / 2], [y_bar, y_bar], color="#ffffff", linewidth=2.2, solid_capstyle="butt", zorder=4)
        ax.plot([-p / 2, p / 2], [y_bar, y_bar], color="#0b0b0b", linewidth=4.0, solid_capstyle="butt", zorder=3, alpha=0.55)
        ax.text(0.0, y_bar + 0.035 * t["field_nm"], f"pitch {_fmt_nm(p)} nm", ha="center", va="bottom", color="#ffffff",
                fontsize=S.SMALL_PT, zorder=5,
                path_effects=[patheffects.withStroke(linewidth=2.0, foreground="#0b0b0b")])
        ax.text(0.0, -0.045, f"{_fmt_nm(t['half_pitch_nm'])} nm lines · contrast {t['contrast']:.2f}", transform=ax.transAxes,
                ha="left", va="top", color=th.ink2, fontsize=S.SMALL_PT)
    fig.text(0.012, 0.012, f"1:1 lines at $k_1$ = {d['k1']:.2f} under an x-dipole (σ$_c$ = {d['sigma_c']:.2f}, pole radius {DIPOLE_RADIUS}); "
             f"two periods per tile; the pitch shrinks {d['ratio']:.1f}× from the first tile to the last. Thin mask, no flare, no resist.",
             ha="left", va="bottom", color=th.muted, fontsize=S.SMALL_PT)
    return fig


register(FigureSpec(
    name="home-generations",
    group="home",
    compute=generations_compute,
    draw=generations_draw,
    alt="Five computed aerial images of equal lines and spaces that look alike: KrF 248 nm at NA 0.80 printing a 217 nm "
        "pitch, ArF 193 nm at NA 0.93 (145 nm), ArF immersion NA 1.35 (100 nm), EUV 13.5 nm at NA 0.33 (28.6 nm) and "
        "High-NA EUV at NA 0.55 (17.2 nm), all at k1 = 0.35 with contrast about 0.9.",
    caption="<strong>Five generations printing the same pattern at k<sub>1</sub> = 0.35.</strong> Each tile is a computed "
            "aerial image of equal lines and spaces under an x-dipole (σ<sub>c</sub> ≈ 0.71) at the half-pitch the published "
            "λ and NA allow — published wavelengths and numerical apertures, not models of the tools; the pitch bar shrinks "
            "12.6× while the image barely changes. Thin mask, no flare, no resist. "
            "<em>Model: Hopkins imaging ✅ · illumination pupil fills ✅ · refractive and immersion optics ✅ · "
            "EUV projection optics 🔶 (isotropic pupil, assumed 0.2·NA obscuration at NA 0.55, no mask 3-D).</em>",
    pages=("index.md", "README.md"),
    requires=("illumination",),
    extra={
        "width": {"README.md": 820},
        "model_rows": {
            "Hopkins imaging": "Hopkins imaging (factorized TCC/SOCS)",
            "refractive and immersion optics": ["Refractive projection optics (dry)", "Immersion lithography (193i)"],
            "EUV projection optics": "EUV scanner projection optics (NA 0.33 / High-NA 0.55)",
        },
    },
))
