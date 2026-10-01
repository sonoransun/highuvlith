#!/usr/bin/env python3
"""Render the highuvlith demo video and the documentation stills.

Five sequences, all computed live by the simulator:

1. through focus: conventional vs dipole illumination (exact defocus model);
2. vector imaging: TE vs TM contrast as the numerical aperture grows to 0.95;
3. the light-source landscape, one preset at a time;
4. level-set development of a volumetric latent image;
5. a scan through a Talbot carpet with the displacement-Talbot average.

Usage (from a checkout with the extension built, e.g. ``maturin develop --release``):

    python examples/generate_demo.py                # video + stills
    python examples/generate_demo.py --quick        # fewer frames, for a smoke test
    python examples/generate_demo.py --stills-only  # only docs/assets/images/demo/*

Outputs (the videos are git-ignored; the stills are tracked):

    examples/demo_vp9.webm, examples/demo_mpeg4.mp4
    docs/assets/images/demo/demo-through-focus.gif
    docs/assets/images/demo/demo-te-tm.png
    docs/assets/images/demo/demo-talbot.png

Needs matplotlib and ffmpeg (with libvpx-vp9) on PATH.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import tempfile
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402

import highuvlith as huv  # noqa: E402

# --- Constants ---
FPS = 24
DPI = 150
FIG_W, FIG_H = 12.8, 7.2  # inches -> 1920x1080 at 150 DPI
BG_COLOR = "#1a1a2e"
PANEL_COLOR = "#16213e"
TEXT_COLOR = "#e0e0e0"
MUTED_COLOR = "#8a8aa0"
ACCENT_COLOR = "#f0a030"
BLUE = "#4fc3f7"
GREEN = "#76ff03"
PINK = "#ff6f91"
CMAP = "inferno"

REPO_URL = "github.com/sonoransun/highuvlith"
BANNER = "highuvlith  ·  VUV → X-ray lithography simulator  ·  " + REPO_URL

ROOT = Path(__file__).resolve().parent.parent
STILLS_DIR = ROOT / "docs" / "assets" / "images" / "demo"


def setup_style():
    plt.rcParams.update({
        "figure.facecolor": BG_COLOR,
        "axes.facecolor": PANEL_COLOR,
        "axes.edgecolor": "#555555",
        "axes.labelcolor": TEXT_COLOR,
        "text.color": TEXT_COLOR,
        "xtick.color": TEXT_COLOR,
        "ytick.color": TEXT_COLOR,
        "grid.color": "#333333",
        "legend.facecolor": PANEL_COLOR,
        "legend.edgecolor": "#555555",
        "font.size": 12,
        "axes.titlesize": 14,
        "figure.titlesize": 16,
    })


class FrameWriter:
    """Numbered PNG frames for ffmpeg."""

    def __init__(self, frames_dir: Path):
        self.dir = frames_dir
        self.count = 0

    def save(self, fig):
        fig.savefig(self.dir / f"{self.count:05d}.png", dpi=DPI, facecolor=fig.get_facecolor())
        plt.close(fig)
        self.count += 1


def add_banner(fig, text: str = BANNER):
    fig.text(0.5, 0.01, text, ha="center", va="bottom", fontsize=9, color="#888888", family="monospace")


def add_badge(fig, text: str):
    """Model-status note under the title, right-aligned (plain words: emoji glyphs are not in the default font)."""
    fig.text(0.99, 0.92, "model: " + text, ha="right", va="bottom", fontsize=10, color=MUTED_COLOR)


# --- Title & transition cards ---

def render_title_card(w: FrameWriter, text: str, subtitle: str = "", footer: str = "",
                      num_frames: int = 48):
    for i in range(num_frames):
        fig = plt.figure(figsize=(FIG_W, FIG_H))
        alpha = min(1.0, i / 12.0)
        if i > num_frames - 12:
            alpha = max(0.0, (num_frames - i) / 12.0)
        fig.text(0.5, 0.55, text, ha="center", va="center", fontsize=40, fontweight="bold",
                 color=TEXT_COLOR, alpha=alpha)
        if subtitle:
            fig.text(0.5, 0.43, subtitle, ha="center", va="center", fontsize=18, color=ACCENT_COLOR, alpha=alpha)
        if footer:
            fig.text(0.5, 0.33, footer, ha="center", va="center", fontsize=14, color=MUTED_COLOR,
                     alpha=alpha, family="monospace")
        add_banner(fig)
        w.save(fig)


def render_section_title(w: FrameWriter, title: str, subtitle: str = "", num_frames: int = 36):
    for i in range(num_frames):
        fig = plt.figure(figsize=(FIG_W, FIG_H))
        alpha = min(1.0, i / 8.0)
        if i > num_frames - 8:
            alpha = max(0.0, (num_frames - i) / 8.0)
        fig.text(0.5, 0.53, title, ha="center", va="center", fontsize=30, color=ACCENT_COLOR, alpha=alpha)
        if subtitle:
            fig.text(0.5, 0.44, subtitle, ha="center", va="center", fontsize=15, color=TEXT_COLOR, alpha=alpha)
        add_banner(fig)
        w.save(fig)


def render_blank(w: FrameWriter, num_frames: int = 8):
    for _ in range(num_frames):
        fig = plt.figure(figsize=(FIG_W, FIG_H))
        add_banner(fig)
        w.save(fig)


def line_space_engine(source, optics, cd, pitch, *, periods=2, px_per_period=64, spp=41, **kw):
    """Engine for periodic lines/spaces on a commensurate grid (``periods`` pitches in the field)."""
    mask = huv.MaskConfig.line_space(cd, pitch)
    grid = huv.GridConfig(size=periods * px_per_period, pixel_nm=pitch / px_per_period)
    return huv.SimulationEngine(source, optics, mask, grid=grid, max_kernels=40 * periods,
                                source_points_per_axis=spp, **kw)


# --- Sequence 1: through focus, conventional vs dipole ---

def through_focus_data(n_frames: int):
    wl, na, cd, pitch = 157.63, 0.75, 90.0, 180.0
    sigma_c = wl / (2 * pitch * na)
    optics = huv.OpticsConfig(numerical_aperture=na)
    sources = {
        "conventional σ 0.7": huv.SourceConfig.f2_laser(sigma=0.7),
        f"dipole σc {sigma_c:.2f}": huv.SourceConfig.f2_laser().with_illumination("dipole", sigma_c, 0.1, 0.0),
    }
    sweep = np.concatenate([np.linspace(0, 500, n_frames // 4), np.linspace(500, -500, n_frames // 2),
                            np.linspace(-500, 0, n_frames - n_frames // 4 - n_frames // 2)])
    grid_focus = np.linspace(-500, 500, 41)
    out = {"focus": sweep, "grid_focus": grid_focus, "panels": {}}
    for name, src in sources.items():
        eng = line_space_engine(src, optics, cd, pitch)
        planes = eng.compute_through_focus(list(sweep))
        curve = [p.image_contrast() for p in eng.compute_through_focus(list(grid_focus))]
        out["panels"][name] = {
            "images": [np.asarray(p.intensity).copy() for p in planes],
            "cuts": [np.asarray(p.cross_section(0.0)[1]) for p in planes],
            "x": np.asarray(planes[0].x_nm),
            "contrast": [p.image_contrast() for p in planes],
            "curve": np.asarray(curve),
        }
    return out


def draw_through_focus(data, i: int, *, banner: bool = True):
    fig = plt.figure(figsize=(FIG_W, FIG_H))
    gs = fig.add_gridspec(2, 3, width_ratios=[1, 1, 1.25], hspace=0.38, wspace=0.32)
    fig.suptitle("Through focus: conventional vs dipole illumination", fontsize=18, fontweight="bold")
    focus = data["focus"][i]
    colors = (BLUE, GREEN)
    ax_c = fig.add_subplot(gs[:, 2])
    for row, ((name, p), color) in enumerate(zip(data["panels"].items(), colors)):
        x = p["x"]
        ax_img = fig.add_subplot(gs[row, 0])
        ax_img.imshow(p["images"][i], cmap=CMAP, vmin=0, vmax=1.1, origin="lower",
                      extent=[x[0], x[-1], x[0], x[-1]], aspect="equal")
        ax_img.set_title(name, color=color, fontsize=12)
        ax_img.set_xticks([])
        ax_img.set_yticks([])
        ax_cut = fig.add_subplot(gs[row, 1])
        ax_cut.plot(x, p["cuts"][i], color=color, lw=2)
        ax_cut.fill_between(x, 0, p["cuts"][i], color=color, alpha=0.15)
        ax_cut.set(xlim=(x[0], x[-1]), ylim=(0, 1.15), xlabel="x (nm)", ylabel="intensity")
        ax_cut.grid(alpha=0.2)
        ax_cut.text(0.97, 0.93, f"C = {p['contrast'][i]:.2f}", transform=ax_cut.transAxes, ha="right",
                    va="top", color=color, fontsize=12, fontweight="bold")
        ax_c.plot(data["grid_focus"], p["curve"], color=color, lw=2, label=name)
        ax_c.plot([focus], [p["contrast"][i]], "o", color=color, ms=9)
    ax_c.axvline(focus, color=ACCENT_COLOR, lw=1, alpha=0.6)
    ax_c.set(xlabel="defocus (nm)", ylabel="image contrast", ylim=(0, 1), title="Contrast vs focus")
    ax_c.grid(alpha=0.2)
    ax_c.legend(loc="lower center")
    ax_c.text(0.03, 0.97, f"focus {focus:+.0f} nm", transform=ax_c.transAxes, va="top", fontsize=15,
              fontweight="bold", color=ACCENT_COLOR)
    fig.text(0.02, 0.92, "90 nm lines / 180 nm pitch · F2 157.63 nm · NA 0.75 (k1 ≈ 0.43)", fontsize=11,
             color=MUTED_COLOR)
    add_badge(fig, "exact non-paraxial defocus per source point (implemented)")
    if banner:
        add_banner(fig)
    return fig


def render_through_focus(w: FrameWriter, n_frames: int):
    print("  Sequence 1: through focus (conventional vs dipole)...")
    data = through_focus_data(n_frames)
    for i in range(len(data["focus"])):
        w.save(draw_through_focus(data, i))
    return data


# --- Sequence 2: TE vs TM as NA grows ---

def te_tm_data(n_frames: int):
    wl, sigma_c = 193.368, 0.8
    nas = np.linspace(0.5, 0.95, n_frames)
    curve_na = np.linspace(0.5, 0.95, 46)
    out = {"na": nas, "curve_na": curve_na, "pol": {}}

    def run(na, pol):
        pitch = wl / (2 * sigma_c * na)        # x-dipole poles at ±0.8: the orders meet at sin θ = 0.8 NA
        src = huv.SourceConfig.arf_laser().with_illumination("dipole", sigma_c, 0.08, 0.0)
        optics = huv.OpticsConfig(numerical_aperture=float(na))
        mask = huv.MaskConfig.line_space(pitch / 2, pitch)
        grid = huv.GridConfig(size=128, pixel_nm=2 * pitch / 128)
        vec = huv.VectorSettings(polarization=pol)
        eng = huv.SimulationEngine(src, optics, mask, grid=grid, max_kernels=40, vector=vec,
                                   source_points_per_axis=41)
        img = eng.compute_aerial_image()
        x, cut = img.cross_section(0.0)
        return img.image_contrast(), np.asarray(x) / pitch, np.asarray(cut)

    for pol, label in (("y", "TE (y-polarized)"), ("x", "TM (x-polarized)"), ("unpolarized", "unpolarized")):
        frames = [run(na, pol) for na in nas]
        out["pol"][label] = {
            "contrast": [f[0] for f in frames],
            "x": [f[1] for f in frames],
            "cut": [f[2] for f in frames],
        }
    # Two-beam closed form relative to the TE image: the TM fringe is weighted by cos 2θ and the
    # unpolarized fringe is the mean of TE and TM (sin θ = 0.8 NA for this dipole and pitch).
    te = np.interp(curve_na, nas, out["pol"]["TE (y-polarized)"]["contrast"])
    cos2t = 1 - 2 * (sigma_c * curve_na) ** 2
    out["pol"]["TE (y-polarized)"]["curve"] = te
    out["pol"]["TM (x-polarized)"]["curve"] = te * np.abs(cos2t)
    out["pol"]["unpolarized"]["curve"] = te * np.abs(1 + cos2t) / 2
    return out


def draw_te_tm(data, i: int, *, banner: bool = True):
    fig, (ax0, ax1) = plt.subplots(1, 2, figsize=(FIG_W, FIG_H), gridspec_kw={"width_ratios": [1.2, 1]})
    fig.suptitle("Vector imaging: TE keeps its contrast, TM collapses", fontsize=18, fontweight="bold")
    na = data["na"][i]
    colors = (BLUE, PINK, GREEN)
    for (label, p), color in zip(data["pol"].items(), colors):
        ax0.plot(p["x"][i], p["cut"][i], color=color, lw=2, label=f"{label}: C = {p['contrast'][i]:.2f}")
        ax1.plot(data["na"][: i + 1], p["contrast"][: i + 1], color=color, lw=2.5)
        ax1.plot(data["curve_na"], p["curve"], color="#ffffff", lw=1.0, ls=":", alpha=0.7)
        ax1.plot([na], [p["contrast"][i]], "o", color=color, ms=9)
    ax0.set(xlabel="x / pitch", ylabel="intensity", title="Aerial image (two periods)", ylim=(0, None))
    ax0.legend(loc="upper center", fontsize=10)
    ax0.grid(alpha=0.2)
    ax1.set(xlabel="numerical aperture (dry)", ylabel="image contrast", xlim=(0.5, 0.95), ylim=(0, 1.05),
            title="Contrast vs NA (dotted: TE × |cos 2θ| closed form)")
    ax1.grid(alpha=0.2)
    sin_t = 0.8 * na
    ax1.text(0.03, 0.08, f"NA {na:.2f}   sin θ = {sin_t:.2f}   2θ = {2 * np.degrees(np.arcsin(sin_t)):.0f}°",
             transform=ax1.transAxes, fontsize=13, fontweight="bold", color=ACCENT_COLOR)
    fig.text(0.02, 0.92, "ArF 193.368 nm · x-dipole σc 0.8 · pitch λ/(1.6 NA): the two orders cross at sin θ = 0.8 NA",
             fontsize=11, color=MUTED_COLOR)
    add_badge(fig, "vector imaging, thin mask, no film (implemented)")
    fig.tight_layout(rect=[0, 0.03, 1, 0.9])
    if banner:
        add_banner(fig)
    return fig


def render_te_tm(w: FrameWriter, n_frames: int):
    print("  Sequence 2: TE vs TM as NA grows...")
    data = te_tm_data(n_frames)
    for i in range(len(data["na"])):
        w.save(draw_te_tm(data, i))
    hold = len(data["na"]) - 1
    for _ in range(FPS):
        w.save(draw_te_tm(data, hold))
    return data


# --- Sequence 3: source landscape ---

def render_landscape(w: FrameWriter, frames_per_preset: int):
    print("  Sequence 3: source landscape...")
    presets = [p for p in huv.source_landscape() if p.average_power_w is not None]
    markers = {"demonstrated": "o", "projection": "s", "theoretical": "^"}
    for k, p in enumerate(presets):
        src = eval("huv.SourceConfig." + p.factory)
        derived = src.derived_quantities()
        for _ in range(frames_per_preset):
            fig = plt.figure(figsize=(FIG_W, FIG_H))
            gs = fig.add_gridspec(1, 2, width_ratios=[1.5, 1], wspace=0.25)
            fig.suptitle("Light-source landscape: every preset, one pipeline", fontsize=18, fontweight="bold")
            ax = fig.add_subplot(gs[0, 0])
            ax.axhspan(250, 1000, xmin=0, xmax=1, color=ACCENT_COLOR, alpha=0.08)
            ax.text(0.12, 300, "EUV HVM 250 W – 1 kW at IF", color=ACCENT_COLOR, fontsize=9)
            for j, q in enumerate(presets):
                active = j == k
                ax.plot(q.wavelength_nm, q.average_power_w, markers.get(q.maturity, "o"),
                        color=ACCENT_COLOR if active else (BLUE if j < k else "#555577"),
                        ms=14 if active else 7, zorder=3 if active else 2)
            ax.set(xscale="log", yscale="log", xlabel="wavelength (nm)", ylabel="average power (W)",
                   xlim=(0.08, 600), ylim=(1e-10, 1e5))
            ax.grid(alpha=0.2, which="major")
            for name, mk in markers.items():
                ax.plot([], [], mk, color=MUTED_COLOR, label=name)
            ax.legend(loc="lower right", fontsize=9, title="maturity", title_fontsize=9)
            ax.text(0.02, 0.97, "powers use each family's own definition (IF, laser output, 4π X-ray)",
                    transform=ax.transAxes, fontsize=9, color=MUTED_COLOR, va="top")
            info = fig.add_subplot(gs[0, 1])
            info.axis("off")
            lines = [
                (p.label, 20, ACCENT_COLOR, "bold"),
                (f"family: {p.family}", 12, TEXT_COLOR, "normal"),
                (f"λ = {p.wavelength_nm:.4g} nm   ({p.photon_energy_ev:.4g} eV)", 12, TEXT_COLOR, "normal"),
                (f"P = {p.average_power_w:.3g} W", 12, TEXT_COLOR, "normal"),
                (f"   {p.power_definition[:44]}", 10, MUTED_COLOR, "normal"),
                (f"machine: {p.maturity}", 12, TEXT_COLOR, "normal"),
                (f"model status: {_status_words(p.status)}", 12, TEXT_COLOR, "normal"),
                ("", 8, TEXT_COLOR, "normal"),
                ("derived from machine parameters:", 11, MUTED_COLOR, "normal"),
            ]
            for name, value, unit, _note in derived[1:7]:
                lines.append((f"  {name} = {value:.3g} {unit}", 10, TEXT_COLOR, "normal"))
            y = 0.95
            for text, size, color, weight in lines:
                info.text(0.0, y, text, fontsize=size, color=color, fontweight=weight, va="top",
                          transform=info.transAxes, family="monospace" if text.startswith("  ") else None)
                y -= 0.075 if size >= 12 else 0.055
            add_banner(fig)
            w.save(fig)


def _status_words(badge: str) -> str:
    words = {"✅": "implemented", "🔶": "simplified", "🧪": "theoretical", "🗺️": "planned"}
    return " / ".join(words.get(part, part) for part in badge.split("/"))


# --- Sequence 4: level-set development ---

def render_development(w: FrameWriter, n_frames: int):
    print("  Sequence 4: level-set development...")
    source = huv.SourceConfig.f2_laser(sigma=0.7)
    optics = huv.OpticsConfig(numerical_aperture=0.75)
    mask = huv.MaskConfig.line_space(150.0, 300.0)
    grid = huv.GridConfig(size=128, pixel_nm=600.0 / 128)
    t_end = 30.0
    proc = huv.simulate_volumetric(
        source, optics, mask, huv.FilmStackConfig(), huv.ResistConfig(), grid,
        dose_mj_cm2=24.0, nz=48, n_defocus_planes=4, dose_steps=5,
        peb="gaussian", peb_lateral_nm=20.0, peb_vertical_nm=10.0,
        develop="level_set", dev_time_s=t_end, surface_rate_ratio=0.3, inhibition_depth_nm=15.0,
    )
    lat = np.asarray(proc.baked.values)
    times = np.asarray(proc.level_set.arrival_times.values)
    mid = lat.shape[1] // 2
    x = np.asarray(proc.baked.x_nm)
    z = np.asarray(proc.baked.z_nm)
    plane_t = np.where(np.isfinite(times[:, mid, :]), times[:, mid, :], 1e9)
    for t in np.linspace(0.0, t_end, n_frames):
        fig, (ax0, ax1) = plt.subplots(1, 2, figsize=(FIG_W, FIG_H), gridspec_kw={"width_ratios": [1.3, 1]})
        fig.suptitle("Development: a level-set front moving through the latent image", fontsize=18,
                     fontweight="bold")
        ax0.imshow(1 - lat[:, mid, :], cmap="magma", origin="upper", aspect="auto", vmin=0, vmax=1,
                   extent=[x[0], x[-1], z[-1], z[0]])
        dissolved = np.ma.masked_where(plane_t > t, np.ones_like(plane_t))
        ax0.imshow(dissolved, cmap="Blues", alpha=0.75, origin="upper", aspect="auto", vmin=0, vmax=1.2,
                   extent=[x[0], x[-1], z[-1], z[0]])
        if 0 < t and plane_t.min() < t:
            ax0.contour(x, z, plane_t, levels=[t], colors=[BLUE], linewidths=2)
        ax0.set(xlabel="x (nm)", ylabel="depth (nm)", title="x–z slice: exposure after bake (glow), developed (blue)")
        remaining = (plane_t > t).sum(axis=0) * (z[1] - z[0])
        ax1.fill_between(x, 0, remaining, color="#1e88e5", alpha=0.7, step="mid")
        ax1.set(xlabel="x (nm)", ylabel="undissolved resist per column (nm)", xlim=(x[0], x[-1]),
                ylim=(0, (z[-1] - z[0]) * 1.2), title="Resist profile (centre row)")
        ax1.grid(alpha=0.2)
        ax1.text(0.97, 0.95, f"t = {t:4.1f} s", transform=ax1.transAxes, ha="right", va="top", fontsize=16,
                 fontweight="bold", color=ACCENT_COLOR)
        fig.text(0.02, 0.92, "F2 · 150/300 nm L/S · 150 nm resist on Si · 24 mJ/cm² · bake · surface inhibition",
                 fontsize=11, color=MUTED_COLOR)
        add_badge(fig, "exposure simplified · level set implemented/simplified")
        fig.tight_layout(rect=[0, 0.03, 1, 0.9])
        add_banner(fig)
        w.save(fig)


# --- Sequence 5: Talbot carpet scan ---

def talbot_data():
    talbot = huv.simulate_talbot(13.5, 100.0, grating="amplitude", max_order=10, carpet_nz=240,
                                 carpet_z_max_nm=2 * 1481.5)
    return {
        "carpet": np.asarray(talbot.carpet),
        "x": np.asarray(talbot.carpet_x_nm),
        "z": np.asarray(talbot.carpet_z_nm),
        "z_t": talbot.talbot_length_nm,
        "dtl": np.asarray(talbot.dtl_image)[0],
        "x_img": np.asarray(talbot.x_nm),
    }


def draw_talbot(d, k: int, *, banner: bool = True):
    carpet, x, z = d["carpet"], d["x"], d["z"]
    fig, (ax0, ax1) = plt.subplots(1, 2, figsize=(FIG_W, FIG_H), gridspec_kw={"width_ratios": [1.1, 1]})
    fig.suptitle("Talbot lithography: scanning the self-imaging carpet", fontsize=18, fontweight="bold")
    ax0.imshow(carpet, cmap=CMAP, origin="lower", aspect="auto", extent=[x[0], x[-1], z[0], z[-1]])
    ax0.axhline(z[k], color=BLUE, lw=2)
    for m, lab in ((0.5, "z_T/2"), (1.0, "z_T"), (1.5, "3z_T/2")):
        ax0.axhline(m * d["z_t"], color="#cccccc", lw=0.8, ls="--")
        ax0.text(x[-1], m * d["z_t"], f" {lab}", color="#cccccc", fontsize=9, va="center")
    ax0.set(xlabel="x (nm)", ylabel="distance behind the mask z (nm)", title="Talbot carpet, p = 100 nm, λ = 13.5 nm")
    ax1.plot(x, carpet[k], color=BLUE, lw=2, label=f"at z = {z[k]:.0f} nm")
    window = carpet[max(0, k - int(len(z) / 2) + 1): k + 1]
    ax1.plot(x, window.mean(axis=0), color=GREEN, lw=2, label="running average, up to one z_T (DTL)")
    ax1.plot(d["x_img"], d["dtl"], color=ACCENT_COLOR, lw=1.2, ls="--", label="DTL image (engine)")
    ax1.set(xlabel="x (nm)", ylabel="intensity", ylim=(0, max(2.4, carpet.max() * 1.05)),
            title="Intensity at the scanned gap")
    ax1.legend(loc="upper right", fontsize=10)
    ax1.grid(alpha=0.2)
    fig.text(0.02, 0.92, "Coherent plane wave · amplitude grating, 1:1 · scalar thin-mask orders · "
             "DTL prints at half the mask period", fontsize=11, color=MUTED_COLOR)
    add_badge(fig, "Talbot / DTL: simplified model")
    fig.tight_layout(rect=[0, 0.03, 1, 0.9])
    if banner:
        add_banner(fig)
    return fig


def render_talbot(w: FrameWriter, n_frames: int):
    print("  Sequence 5: Talbot carpet scan...")
    d = talbot_data()
    for k in np.linspace(0, len(d["z"]) - 1, n_frames).astype(int):
        w.save(draw_talbot(d, int(k)))
    return d


# --- Encoding and stills ---

def encode_video(frames_dir: Path, output_path: Path, codec_args: list[str]):
    cmd = ["ffmpeg", "-y", "-loglevel", "error", "-framerate", str(FPS), "-i", str(frames_dir / "%05d.png"),
           "-pix_fmt", "yuv420p", *codec_args, str(output_path)]
    print(f"  Encoding {output_path.name}...")
    subprocess.run(cmd, check=True)
    print(f"    -> {output_path.stat().st_size / 2**20:.1f} MiB")


def write_stills(stills_dir: Path, tf_data=None, tt_data=None, talbot=None):
    """Small tracked stills for the docs: an animated GIF and two PNGs."""
    stills_dir.mkdir(parents=True, exist_ok=True)
    print(f"  Writing stills to {stills_dir}...")
    if tf_data is None:
        tf_data = through_focus_data(48)
    with tempfile.TemporaryDirectory(prefix="huv_gif_") as tmp:
        tmp = Path(tmp)
        for i in range(0, len(tf_data["focus"]), 2):
            fig = draw_through_focus(tf_data, i, banner=False)
            fig.savefig(tmp / f"{i // 2:04d}.png", dpi=60, facecolor=fig.get_facecolor())
            plt.close(fig)
        gif = stills_dir / "demo-through-focus.gif"
        palette = tmp / "palette.png"
        subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-i", str(tmp / "%04d.png"),
                        "-vf", "scale=768:-1:flags=lanczos,palettegen=max_colors=96", str(palette)], check=True)
        subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-framerate", "12", "-i", str(tmp / "%04d.png"),
                        "-i", str(palette), "-lavfi", "scale=768:-1:flags=lanczos[x];[x][1:v]paletteuse=dither=bayer",
                        "-loop", "0", str(gif)], check=True)
    if tt_data is None:
        tt_data = te_tm_data(19)
    fig = draw_te_tm(tt_data, len(tt_data["na"]) - 1, banner=False)
    fig.savefig(stills_dir / "demo-te-tm.png", dpi=80, facecolor=fig.get_facecolor())
    plt.close(fig)
    if talbot is None:
        talbot = talbot_data()
    fig = draw_talbot(talbot, int(0.75 * (len(talbot["z"]) - 1)), banner=False)
    fig.savefig(stills_dir / "demo-talbot.png", dpi=80, facecolor=fig.get_facecolor())
    plt.close(fig)
    for f in sorted(stills_dir.iterdir()):
        print(f"    -> {f.name}: {f.stat().st_size / 2**20:.2f} MiB")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--quick", action="store_true", help="few frames per sequence (smoke test)")
    ap.add_argument("--stills-only", action="store_true", help="only write the documentation stills")
    ap.add_argument("--no-stills", action="store_true", help="skip the documentation stills")
    ap.add_argument("--out-dir", type=Path, default=Path(__file__).resolve().parent, help="video directory")
    ap.add_argument("--stills-dir", type=Path, default=STILLS_DIR, help="stills directory")
    args = ap.parse_args()
    if shutil.which("ffmpeg") is None:
        raise SystemExit("ffmpeg not found on PATH")
    setup_style()

    if args.stills_only:
        write_stills(args.stills_dir)
        return

    n = 12 if args.quick else 1
    frames_dir = Path(tempfile.mkdtemp(prefix="highuvlith_demo_"))
    print(f"Rendering frames to {frames_dir}")
    w = FrameWriter(frames_dir)
    try:
        render_title_card(w, "highuvlith", subtitle="VUV → X-ray lithography simulator",
                          footer="157 nm · 13.5 nm · 6.7 nm · X-ray", num_frames=48 // n + 1)
        render_section_title(w, "1 · Through focus", "off-axis illumination, exact defocus", 36 // n + 1)
        tf = render_through_focus(w, max(8, 144 // n))
        render_blank(w, 8 // n + 1)
        render_section_title(w, "2 · Polarization at high NA", "vector imaging, TE vs TM", 36 // n + 1)
        tt = render_te_tm(w, max(6, 96 // n))
        render_blank(w, 8 // n + 1)
        render_section_title(w, "3 · Light sources", "fourteen families, from Hg lamps to X-ray tubes", 36 // n + 1)
        render_landscape(w, max(1, 7 // n))
        render_blank(w, 8 // n + 1)
        render_section_title(w, "4 · Resist development", "volumetric exposure, bake, level set", 36 // n + 1)
        render_development(w, max(6, 120 // n))
        render_blank(w, 8 // n + 1)
        render_section_title(w, "5 · Talbot lithography", "self-imaging without a lens", 36 // n + 1)
        tb = render_talbot(w, max(6, 144 // n))
        render_blank(w, 8 // n + 1)
        render_title_card(w, "highuvlith", subtitle=REPO_URL,
                          footer="sonoransun.github.io/highuvlith", num_frames=60 // n + 1)
        print(f"\nTotal: {w.count} frames ({w.count / FPS:.1f} s at {FPS} fps)")

        args.out_dir.mkdir(parents=True, exist_ok=True)
        encode_video(frames_dir, args.out_dir / "demo_vp9.webm",
                     ["-c:v", "libvpx-vp9", "-crf", "38", "-b:v", "0", "-row-mt", "1", "-threads", "4"])
        encode_video(frames_dir, args.out_dir / "demo_mpeg4.mp4", ["-c:v", "mpeg4", "-b:v", "1800k", "-g", "48"])
        if not args.no_stills:
            write_stills(args.stills_dir, tf, tt, tb)
    finally:
        shutil.rmtree(frames_dir, ignore_errors=True)
    print("Done.")


if __name__ == "__main__":
    main()
