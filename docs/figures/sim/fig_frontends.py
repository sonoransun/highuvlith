"""Front-end figures: R1, the CLI output gallery (DESIGN §5.6).

``cli-output-gallery`` is a 1 × 5 contact sheet of the PNGs the *built binary itself*
writes for five bundled configs (``simulate examples/sim.toml``, ``deep examples/liga.toml``,
``deep examples/volumetric.toml``, ``deep examples/talbot.toml``, ``optimize
examples/optimize_ilt.toml``). ``compute()`` runs the binary into a temporary directory,
checks that every PNG exists and is a non-blank RGB image, and keeps the pixels; ``draw()``
shows them with ``interpolation="nearest"`` and never touches the CLI's own colormap or
min–max normalisation — light and dark differ only in the frame and the labels.

The spec carries ``requires=("cli",)``: without ``target/release/highuvlith`` (or
``$HUV_CLI``) ``make_all.py`` skips it, and ``make_all.py --require cli`` turns the skip
into an error (the nightly builds the CLI and runs ``--verify --require cli``). The
``simulate`` panel is also written as the derived crop ``cli-output-gallery-simulate``
(``extra["crops"]``) for ``getting-started.md``.
"""

from __future__ import annotations

import re
import subprocess
import tempfile
from pathlib import Path

import numpy as np
from matplotlib import pyplot as plt

import style as S
from common import REPO, FigureSpec, cli_binary, register

# ---------------------------------------------------------------------------
# What the CLI writes (crates/highuvlith-cli/src/commands/{simulate,deep,optimize}.rs,
# highuvlith-core/src/io/image_export.rs: save_png = RGB, min–max normalised, one colormap)
# ---------------------------------------------------------------------------

#: (key, subcommand, config, what the PNG shows, the CLI's colormap, regex for one headline number on stdout)
RUNS: tuple[tuple[str, str, str, str, str, str], ...] = (
    ("simulate", "simulate", "examples/sim.toml", "aerial image", "Inferno", r"Contrast[^\d]*([\d.]+)"),
    ("liga", "deep", "examples/liga.toml", "developed-depth map", "Viridis", r"sidewall angle:\s*([\d.]+°)"),
    ("volumetric", "deep", "examples/volumetric.toml", "x–z developed profile", "Inferno", r"Developed sidewall:\s*([\d.]+°)"),
    ("talbot", "deep", "examples/talbot.toml", "wafer-plane image", "Inferno", r"Image contrast:\s*([\d.]+)"),
    ("optimize", "optimize", "examples/optimize_ilt.toml", "optimized mask (ILT)", "Grayscale", r"optimized (\d+) \(binary"),
)
HEADLINE_WORD = {"simulate": "contrast", "liga": "sidewall", "volumetric": "sidewall", "talbot": "contrast",
                 "optimize": "misprinted px"}

#: Pixel box (x, y, w, h) of the ``simulate`` panel in the 2× PNG, measured in ``draw()``; the render refuses
#: to write when the layout has moved more than CROP_TOL px from it (update the constant and re-render).
SIMULATE_CROP_BOX = (4, 52, 389, 563)
CROP_TOL = 6


def _read_png(path: Path) -> np.ndarray:
    """The PNG as an (h, w, 3) uint8 array, exactly the bytes the CLI wrote (no resampling)."""
    import matplotlib.image as mimg  # noqa: PLC0415

    arr = mimg.imread(str(path))
    if arr.dtype != np.uint8:
        arr = np.clip(np.rint(arr * 255), 0, 255).astype(np.uint8)
    if arr.ndim == 2:
        arr = np.stack([arr] * 3, axis=-1)
    return np.ascontiguousarray(arr[..., :3])


def cli_compute(fast: bool):
    """Run the five commands into a temporary directory and keep what they wrote (asserts inside)."""
    binary = cli_binary()
    if binary is None:
        raise RuntimeError("the CLI is not built: cargo build --release -p highuvlith-cli (or set HUV_CLI)")
    panels = []
    with tempfile.TemporaryDirectory(prefix="huv-cli-") as tmp:
        for key, sub, cfg, what, cmap, rx in RUNS:
            out = Path(tmp) / f"{key}.png"
            proc = subprocess.run([str(binary), sub, "--config", str(REPO / cfg), "--output", str(out)],
                                  cwd=str(REPO), capture_output=True, text=True, timeout=600)
            if proc.returncode != 0:
                raise RuntimeError(f"highuvlith {sub} --config {cfg} failed ({proc.returncode}): {proc.stderr.strip()[-400:]}")
            assert out.is_file(), f"{sub} {cfg}: the CLI did not write {out.name}"
            rgb = _read_png(out)
            assert rgb.ndim == 3 and rgb.shape[2] == 3 and min(rgb.shape[:2]) >= 16, f"{key}: unexpected PNG shape {rgb.shape}"
            assert int(rgb.max()) > int(rgb.min()), f"{key}: the PNG is blank (min == max)"
            report = proc.stdout + "\n" + proc.stderr  # the CLI prints its report on stderr
            m = re.search(rx, report)
            panels.append({"key": key, "command": f"highuvlith {sub}", "config": cfg, "what": what, "cmap": cmap,
                           "rgb": rgb, "headline": m.group(1) if m else None, "report_tail": report.strip()[-200:]})
    sim = next(p for p in panels if p["key"] == "simulate")
    assert sim["rgb"].shape[0] == sim["rgb"].shape[1], "simulate: examples/sim.toml renders a square grid"
    assert all(p["headline"] for p in panels), "every command prints its headline number"
    return {"panels": panels, "binary": str(binary.relative_to(REPO)) if binary.is_relative_to(REPO) else binary.name}


def _measure_crop(fig, artists, pad_in: float = 0.08) -> tuple[int, int, int, int]:
    """Pixel box of ``artists`` (an axes with its title, plus its footer texts) in the saved 2× PNG."""
    fig.canvas.draw()
    rend = fig.canvas.get_renderer()
    tb = fig.get_tightbbox(rend)  # inches
    boxes = [a.get_tightbbox(rend) for a in artists]
    x0 = min(b.x0 for b in boxes) / fig.dpi
    x1 = max(b.x1 for b in boxes) / fig.dpi
    y0 = min(b.y0 for b in boxes) / fig.dpi
    y1 = max(b.y1 for b in boxes) / fig.dpi
    scale = S.LOGICAL_DPI * S.EXPORT_SCALE
    margin = 0.06  # inches of air around the panel
    left = max(0.0, (x0 - tb.x0 + pad_in - margin)) * scale
    top = max(0.0, (tb.y1 - y1 + pad_in - margin)) * scale
    w = (x1 - x0 + 2 * margin) * scale
    h = (y1 - y0 + 2 * margin) * scale
    return tuple(int(round(v)) for v in (left, top, w, h))


def cli_draw(d, th: S.Theme):
    panels = d["panels"]
    n = len(panels)
    fig = plt.figure(figsize=(S.WIDE_WIDTH + 0.8, 3.3))
    gs = fig.add_gridspec(1, n, wspace=0.14, left=0.012, right=0.988, top=0.80, bottom=0.21)
    axes = []
    for k, p in enumerate(panels):
        ax = fig.add_subplot(gs[0, k])
        ax.imshow(p["rgb"], interpolation="nearest", aspect="equal")
        ax.set_anchor("N")  # the image hugs its title; a 4:1 strip leaves air below, never above
        ax.set_xticks([])
        ax.set_yticks([])
        for sp in ax.spines.values():
            sp.set_edgecolor(th.axis)
            sp.set_linewidth(S.HAIR_PT)
        ax.set_facecolor("none")
        ax.set_title(f"{p['command']}\n{p['config']}", fontsize=S.SMALL_PT + 0.5, color=th.ink, family="monospace",
                     linespacing=1.35, pad=5)
        h, w = p["rgb"].shape[:2]
        cell = gs[0, k].get_position(fig)
        cx = (cell.x0 + cell.x1) / 2
        line1 = f"{p['what']} · {w} × {h} px"
        line2 = p["cmap"] + (f" · {HEADLINE_WORD[p['key']]} {p['headline']}" if p["headline"] else "")
        t1 = fig.text(cx, cell.y0 - 0.035, line1, ha="center", va="top", fontsize=S.SMALL_PT - 0.5, color=th.ink2)
        t2 = fig.text(cx, cell.y0 - 0.095, line2, ha="center", va="top", fontsize=S.SMALL_PT - 0.5, color=th.muted)
        axes.append([ax, t1, t2])
    fig.text(0.012, 0.975, "What the CLI writes with --output <file>.png — its own files, shown pixel for pixel",
             ha="left", va="top", fontsize=S.SMALL_PT + 0.5, color=th.ink2)
    box = _measure_crop(fig, axes[0])  # the simulate panel with its title and footers
    if any(abs(a - b) > CROP_TOL for a, b in zip(box, SIMULATE_CROP_BOX)):
        raise AssertionError(f"cli-output-gallery: the simulate panel sits at {box} in the 2x PNG; SIMULATE_CROP_BOX is "
                             f"{SIMULATE_CROP_BOX} — update the constant (and re-render) so the crop stays on the panel")
    return fig


register(FigureSpec(
    name="cli-output-gallery",
    group="frontends",
    compute=cli_compute,
    draw=cli_draw,
    kind="sheet",
    requires=("cli",),
    alt="Five PNGs the highuvlith command line wrote, pixel for pixel: the aerial image of simulate examples/sim.toml "
        "(contrast 0.4545), the LIGA developed-depth map, the volumetric x–z developed profile, the Talbot wafer-plane "
        "image (contrast 0.457) and the ILT mask of optimize examples/optimize_ilt.toml.",
    caption="<strong>What the CLI writes.</strong> Its own PNGs for five bundled configs, untouched (its own colormap and "
            "min–max normalisation); each panel names the command, its config, what the image is and the size the binary "
            "wrote. <code>simulate</code> saves the aerial image, <code>deep</code> the mode's characteristic map "
            "(LIGA developed depth, volumetric x–z profile, Talbot wafer-plane image) and <code>optimize</code> the "
            "optimized mask; the headline number under each panel is the one the command prints. "
            "<em>Grades per command as in the At-a-glance table below; the CLI's own files.</em>",
    pages=("cli.md",),
    hero_on=("cli.md",),
    extra={
        "crops": {
            "simulate": dict(
                pages=("getting-started.md",),
                kind="figure",
                box=SIMULATE_CROP_BOX,
                alt="The 256 × 256 px aerial-image PNG that highuvlith simulate --config examples/sim.toml --output writes "
                    "for 65 nm lines on a 180 nm pitch, through the CLI's Inferno colormap, with its printed contrast.",
                caption="<strong>The PNG the command line writes.</strong> <code>highuvlith simulate --config "
                        "examples/sim.toml --output aerial.png</code> saves the aerial image of the canonical run "
                        "(65 nm lines, 180 nm pitch, F<sub>2</sub> 157.63 nm, NA 0.75, σ 0.7) as a min–max normalised "
                        "RGB PNG through its own Inferno colormap; the contrast under the panel is the one it prints. "
                        "<em>The CLI's own file; the same run as the Python example above.</em>",
            ),
        },
    },
))
