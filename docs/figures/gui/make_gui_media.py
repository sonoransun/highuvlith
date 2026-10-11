#!/usr/bin/env python3
"""R2 · GUI media composed from the four genuine ``--screenshot`` captures (DESIGN §5.6).

The four raw captures under ``docs/assets/images/gui/`` are the one kind of file under
``docs/assets/images/`` that no script produces. This module registers them as
``AssetSpec(kind="capture", generator=None, pinned_sha256=…)`` so ``check_figures.py
--check`` verifies their hashes and the 500 KB cap instead of regenerating them, and it
composes every other GUI image **only** from those files (Pillow; hashes verified before
anything is read; no GUI code is touched, nothing is mocked):

* ``gui-layout-annotated`` — ``gui-aerial-euv.png`` with three numbered overlays
  (1 controls · 2 tab bar + view · 3 status bar) on a light / dark frame (the ``gui.md`` hero);
* ``gui-source-badge`` and ``gui-source-derived`` — 1:1 crops of ``gui-source-ssmb.png``:
  the fidelity badge of the Source tab and its derived-quantities block;
* ``gui-strip`` — the README front-ends strip: ``gui-process-window`` · ``gui-volume-talbot``
  · ``gui-aerial-euv`` re-encoded at 900 px wide, side by side.

    python docs/figures/gui/make_gui_media.py            # verify the hashes, then write every composed pair
    python docs/figures/gui/make_gui_media.py --check    # hashes + sizes only; writes nothing
"""

from __future__ import annotations

import io
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import registry  # noqa: E402
import tokens  # noqa: E402

OUT = registry.IMAGES / "gui"
GENERATOR = "docs/figures/gui/make_gui_media.py"
GUI = "highuvlith-gui"
CAPTURE_WIDTH = 1500  # every capture is 1500 × 835 (2× macOS capture downscaled, gui.md § Screenshots)

# ---------------------------------------------------------------------------
# The four captures: sha256 (pinned), the command that made them, what they show
# ---------------------------------------------------------------------------

CAPTURES: dict[str, dict] = {
    "gui-aerial-euv": dict(
        sha256="56a51f39599a5bdfa07c9c8a06041af0830412719e379483cd6132097ef3a5d8",
        command="--preset lpp-sn --optics nxe --cd 16 --pitch 32 --tab aerial",
        pages=("gui.md", "getting-started.md"),
        alt="The running desktop app on its Aerial image tab: the Source and Optics controls (Sn LPP preset, NA 0.33), "
            "the EUV 13.5 nm aerial image of 16 nm lines on a 32 nm pitch with its colour scale, the metrics rows "
            "(contrast 0.4494, printed CD 16.637 nm, NILS 1.427) and the status bar naming 21 kernels.",
        caption="<strong>Aerial image view.</strong> The Sn LPP preset (NXE:3400B class) through the EUV NXE optics, "
                "NA 0.33, 16 nm lines on a 32 nm pitch: the image, its colour scale, the metrics rows and the status bar "
                "with the kernel count and captured TCC fraction. Made with <code>" + GUI + " --preset lpp-sn --optics nxe "
                "--cd 16 --pitch 32 --tab aerial --screenshot</code>. <em>Real capture, dark theme; nothing is mocked.</em>",
    ),
    "gui-process-window": dict(
        sha256="44f2fe85b0ba7418bbdd1a8193d6716ffc9663f51029c4876aabd24fdc0c73c8",
        command="--preset arf --optics 193i --cd 60 --pitch 130 --tab process-window",
        pages=("gui.md",),
        alt="The Process window tab for ArF 193 nm in water (NA 1.35), 60 nm lines on a 130 nm pitch: the sweep "
            "controls, summary rows (DOF at dose-to-size 167.2 nm, exposure latitude 19.645 %), Bossung curves per dose, "
            "the exposure–defocus window with its max DOF × EL rectangle and the EL-versus-DOF curve.",
        caption="<strong>Process window view.</strong> ArF 193 nm with water immersion, NA 1.35, 60 nm lines on a "
                "130 nm pitch: Bossung curves per dose, the exposure–defocus window with its largest DOF × EL rectangle "
                "and the exposure-latitude curve, all from the dose-aware sweep in the panel above them. Made with "
                "<code>" + GUI + " --preset arf --optics 193i --cd 60 --pitch 130 --tab process-window --screenshot</code>. "
                "<em>Real capture, dark theme; nothing is mocked.</em>",
    ),
    "gui-volume-talbot": dict(
        sha256="97cbd256280e7e57c64af7291a41a841c0a8688611669816dd7692dc8048018a",
        command="--preset arf --tab volume --dataset talbot",
        pages=("gui.md",),
        alt="The Volume viewer tab on the Talbot carpet dataset for ArF 193 nm: dataset controls (amplitude grating, "
            "±10 orders, 2 Talbot lengths, 2 periods), the intensity profile at the z slice 1.309 nm behind the grating "
            "and the x–z section at y = 0 with its colour scale.",
        caption="<strong>Volume viewer, Talbot carpet.</strong> ArF 193 nm, a 180 nm amplitude grating with ±10 orders "
                "over two Talbot lengths: the profile at the chosen z slice and the x–z section at y = 0, the same "
                "carpet the <code>deep --mode talbot</code> command computes. Made with <code>" + GUI + " --preset arf "
                "--tab volume --dataset talbot --screenshot</code>. <em>Real capture, dark theme; nothing is mocked.</em>",
    ),
    "gui-source-ssmb": dict(
        sha256="3fee7ee3128cbf751ef41113cdeb8717fc45954805ca9d3c7e1d5ace19b0e0bf",
        command="--preset ssmb --optics nxe --tab source",
        pages=("gui.md",),
        alt="The Source tab for the SSMB storage ring 1 kW design point: the Theoretical fidelity badge with its "
            "justification, the derived-quantities table (harmonic number 78, bunching factor 0.1458, coherent power "
            "1000 W, photon rate 6.796e19 photons/s), the spectral samples and the sampled pupil fill.",
        caption="<strong>Source view.</strong> The SSMB storage ring at its 1 kW design point through the EUV NXE optics: "
                "the fidelity badge and its justification, the derived quantities with their formulas, the spectral "
                "samples and the sampled pupil fill. Made with <code>" + GUI + " --preset ssmb --optics nxe --tab source "
                "--screenshot</code>. <em>Real capture, dark theme; nothing is mocked.</em>",
    ),
}

#: Numbered overlays on ``gui-aerial-euv.png`` (x0, y0, x1, y1 in capture pixels).
LAYOUT_REGIONS: tuple[tuple[str, str, tuple[int, int, int, int]], ...] = (
    ("1", "controls", (4, 4, 336, 808)),
    ("2", "tab bar + view", (344, 4, 1496, 808)),
    ("3", "status bar", (4, 814, 1496, 832)),
)
#: 1:1 crops of ``gui-source-ssmb.png`` (x0, y0, x1, y1).
BADGE_BOX = (344, 38, 1200, 128)
DERIVED_BOX = (344, 128, 1200, 500)
STRIP_PANELS = (("gui-process-window", "Process window"), ("gui-volume-talbot", "Volume viewer"), ("gui-aerial-euv", "Aerial image"))
STRIP_WIDTH = 900
FRAME = 12


def capture_path(name: str) -> Path:
    return OUT / f"{name}.png"


def verify_captures() -> list[str]:
    """Errors for any capture whose sha256 or size differs from the pinned value."""
    errors = []
    for name, c in CAPTURES.items():
        p = capture_path(name)
        if not p.is_file():
            errors.append(f"{name}: {registry.rel_to_repo(p)} is missing")
            continue
        if registry.sha256_of(p) != c["sha256"]:
            errors.append(f"{name}: sha256 differs from the pinned value (the captures are never regenerated here)")
        if p.stat().st_size > 500 * 1024:
            errors.append(f"{name}: over the 500 KB capture cap")
    return errors


def _load(name: str):
    """Open one capture after verifying its hash (the only way a capture is ever read here)."""
    from PIL import Image  # noqa: PLC0415

    errs = [e for e in verify_captures() if e.startswith(name + ":")]
    if errs:
        raise RuntimeError("; ".join(errs))
    im = Image.open(capture_path(name))
    im.load()
    if im.size[0] != CAPTURE_WIDTH:
        raise RuntimeError(f"{name}: expected a {CAPTURE_WIDTH} px wide capture, got {im.size}")
    return im.convert("RGB")


def _font(size: int):
    from PIL import ImageFont  # noqa: PLC0415

    for path, index in (("/System/Library/Fonts/HelveticaNeue.ttc", 0), ("/System/Library/Fonts/Supplemental/Arial.ttf", 0),
                        ("/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf", 0)):
        try:
            return ImageFont.truetype(path, size, index=index)
        except OSError:
            continue
    return ImageFont.load_default()


def _theme(theme: str) -> dict:
    return tokens.theme(theme)


def _frame(im, theme: str, pad: int = FRAME):
    """The capture on a theme-coloured frame with a hairline border (light = white frame, dark = the dark surface)."""
    from PIL import Image, ImageDraw  # noqa: PLC0415

    t = _theme(theme)
    w, h = im.size
    out = Image.new("RGB", (w + 2 * pad, h + 2 * pad), t["surface"])
    out.paste(im, (pad, pad))
    d = ImageDraw.Draw(out)
    d.rectangle((pad - 1, pad - 1, pad + w, pad + h), outline=t["axis"], width=1)
    return out


def _dashed_rect(draw, box, color, width=3, dash=14, gap=8):
    x0, y0, x1, y1 = box
    for (ax, ay, bx, by) in ((x0, y0, x1, y0), (x1, y0, x1, y1), (x0, y1, x1, y1), (x0, y0, x0, y1)):
        length = max(abs(bx - ax), abs(by - ay))
        pos = 0
        while pos < length:
            end = min(pos + dash, length)
            if ax == bx:
                draw.line((ax, ay + pos, ax, ay + end), fill=color, width=width)
            else:
                draw.line((ax + pos, ay, ax + end, ay), fill=color, width=width)
            pos += dash + gap


def _save(im, path: Path, *, palette: int | None = None, meta: dict | None = None) -> Path:
    from PIL import PngImagePlugin  # noqa: PLC0415

    info = PngImagePlugin.PngInfo()
    for k, v in (meta or {}).items():
        info.add_text(k, v)
    if palette:
        im = im.quantize(colors=palette, method=2, dither=0)  # FASTOCTREE, no dithering of flat UI
    buf = io.BytesIO()
    im.save(buf, format="PNG", optimize=True, pnginfo=info)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(buf.getvalue())
    return path


def _meta(name: str, alt: str) -> dict:
    return {"Title": name, "Description": alt, "Source": f"{GENERATOR} (composed from the pinned captures)",
            "Software": "highuvlith figure scripts (docs/figures/gui)"}


# ---------------------------------------------------------------------------
# The composed outputs
# ---------------------------------------------------------------------------


def draw_layout(theme: str):
    from PIL import ImageDraw  # noqa: PLC0415

    t = _theme(theme)
    im = _load("gui-aerial-euv")
    d = ImageDraw.Draw(im)
    accent = t["brand"]["violet300"] if theme == "dark" else t["brand"]["violet600"]
    digit = t["brand"]["violet900"] if theme == "dark" else "#ffffff"
    font = _font(22)
    label_font = _font(18)
    for num, label, box in LAYOUT_REGIONS:
        _dashed_rect(d, box, accent)
        x0, y0, x1, y1 = box
        cx, cy = x0 + 20, y0 + 46  # under the app name / tab row, never over the "highuvlith" title
        if num == "2":
            cx, cy = x0 + 20, y0 + 44
        if num == "3":
            cx, cy = x1 - 20, y0 - 24  # the status bar is 18 px tall: the disc sits just above its right end
        d.ellipse((cx - 16, cy - 16, cx + 16, cy + 16), fill=accent)
        tw = d.textlength(num, font=font)
        d.text((cx - tw / 2, cy - 13), num, fill=digit, font=font)
        lw = d.textlength(label, font=label_font)
        lx, ly = (cx + 22, cy - 12) if num != "3" else (cx - 22 - lw, cy - 12)
        d.rounded_rectangle((lx - 6, ly - 3, lx + lw + 6, ly + 23), radius=6, fill=accent)
        d.text((lx, ly), label, fill=digit, font=label_font)
    return _frame(im, theme)


def draw_crop(theme: str, box: tuple[int, int, int, int]):
    im = _load("gui-source-ssmb").crop(box)
    return _frame(im, theme, pad=8)


def draw_strip(theme: str):
    from PIL import Image, ImageDraw  # noqa: PLC0415

    t = _theme(theme)
    gap, label_h = 24, 40
    panels = []
    for name, label in STRIP_PANELS:
        im = _load(name)
        w, h = im.size
        nh = round(h * STRIP_WIDTH / w)
        panels.append((im.resize((STRIP_WIDTH, nh), Image.LANCZOS), label))
    ph = panels[0][0].size[1]
    W = 2 * FRAME + len(panels) * STRIP_WIDTH + (len(panels) - 1) * gap
    H = 2 * FRAME + ph + label_h
    out = Image.new("RGB", (W, H), t["surface"])
    d = ImageDraw.Draw(out)
    font = _font(24)
    x = FRAME
    for im, label in panels:
        out.paste(im, (x, FRAME))
        d.rectangle((x - 1, FRAME - 1, x + STRIP_WIDTH, FRAME + ph), outline=t["axis"], width=1)
        d.text((x + 2, FRAME + ph + 10), label, fill=t["ink2"], font=font)
        x += STRIP_WIDTH + gap
    return out


def _write_pair(name: str, draw, alt: str, *, palette: int | None = None) -> list[Path]:
    paths = []
    for theme in ("light", "dark"):
        p = OUT / f"{name}-{theme}.png"
        paths.append(_save(draw(theme), p, palette=palette, meta=_meta(name, alt)))
    return paths


LAYOUT_ALT = ("The running desktop app, dark theme, with three numbered overlays on the Aerial image view: 1 the Source, "
              "Optics and Imaging controls, 2 the tab bar and the EUV aerial image with its metrics, 3 the status bar "
              "reporting 21 kernels, the captured TCC fraction and the contrast 0.4494.")
LAYOUT_CAPTION = ("<strong>The running app.</strong> Real <code>--screenshot</code> capture, dark theme; nothing is mocked: "
                  "1 controls, 2 views, 3 status bar showing kernels, captured TCC fraction and contrast. The app is "
                  "dark-themed; the light variant is the same capture on a light frame. "
                  "<em>Annotated real capture; nothing is mocked.</em>")
BADGE_ALT = ("The Source tab header for the SSMB storage ring: the TOML type tag, the Theoretical badge and its one-line "
             "justification (the 1 kW point needs a bunching factor 0.146 never demonstrated), and the set-point line "
             "13.500 nm, 1000.0 W average power.")
BADGE_CAPTION = ("<strong>The fidelity badge.</strong> Every source family carries its matrix grade and the one-sentence "
                 "reason in the app itself — here SSMB's 🧪 Theoretical badge, 1:1 from the Source tab. "
                 "<em>Real capture, 1:1 crop; nothing is mocked.</em>")
DERIVED_ALT = ("The derived-quantities table of the Source tab for SSMB: photon energy 91.840 eV, harmonic number 78, "
               "electron gamma 783.8, bunching factor 0.1458, coherent power 1000.0 W, incoherent power 1.674 W, "
               "projected power 1000.0 W and photon rate 6.796e19 photons/s, each with its formula.")
DERIVED_CAPTION = ("<strong>Derived quantities, with their formulas.</strong> The informational block under every source: "
                   "each number names the formula it comes from and the imaging pipeline never reads it — 1:1 from the "
                   "SSMB Source tab. <em>Real capture, 1:1 crop; nothing is mocked.</em>")
STRIP_ALT = ("Three captures of the desktop app side by side: the Process window tab with Bossung curves and the "
             "exposure–defocus window, the Volume viewer on a Talbot carpet, and the Aerial image tab for EUV 16 nm "
             "lines on a 32 nm pitch.")
STRIP_CAPTION = ("<strong>Three of the GUI's views.</strong> Process window (ArF 193i, 60 nm on 130 nm), Volume viewer "
                 "(Talbot carpet, ArF 193 nm) and Aerial image (EUV 13.5 nm, 16 nm on 32 nm), real <code>--screenshot</code> "
                 "captures re-encoded at 900 px wide. <em>Real captures, dark theme; nothing is mocked.</em>")


def gen_layout() -> list[Path]:
    return _write_pair("gui-layout-annotated", draw_layout, LAYOUT_ALT)


def gen_badge() -> list[Path]:
    return _write_pair("gui-source-badge", lambda t: draw_crop(t, BADGE_BOX), BADGE_ALT)


def gen_derived() -> list[Path]:
    return _write_pair("gui-source-derived", lambda t: draw_crop(t, DERIVED_BOX), DERIVED_ALT)


def gen_strip() -> list[Path]:
    return _write_pair("gui-strip", draw_strip, STRIP_ALT, palette=255)


# ---------------------------------------------------------------------------
# Registration (at import time; drawing only under __main__ / through the generators)
# ---------------------------------------------------------------------------

for _name, _c in CAPTURES.items():
    registry.register(registry.AssetSpec(
        name=_name, kind="capture", group="gui", subdir="gui", fmt="png", generator=None,
        pinned_sha256=_c["sha256"], alt=_c["alt"], caption=_c["caption"], pages=_c["pages"],
        extra={"command": f"{GUI} {_c['command']} --screenshot {_name}.png"},
    ), module="make_gui_media.py")

registry.register(registry.AssetSpec(
    name="gui-layout-annotated", kind="sheet", group="gui", subdir="gui", fmt="png",
    alt=LAYOUT_ALT, caption=LAYOUT_CAPTION, pages=("gui.md",), hero_on=("gui.md",), generator=gen_layout,
), module="make_gui_media.py")
registry.register(registry.AssetSpec(
    name="gui-source-badge", kind="sheet", group="gui", subdir="gui", fmt="png",
    alt=BADGE_ALT, caption=BADGE_CAPTION, pages=("gui.md",), generator=gen_badge,
), module="make_gui_media.py")
registry.register(registry.AssetSpec(
    name="gui-source-derived", kind="sheet", group="gui", subdir="gui", fmt="png",
    alt=DERIVED_ALT, caption=DERIVED_CAPTION, pages=("gui.md",), generator=gen_derived,
), module="make_gui_media.py")
registry.register(registry.AssetSpec(
    name="gui-strip", kind="sheet", group="gui", subdir="gui", fmt="png",
    alt=STRIP_ALT, caption=STRIP_CAPTION, pages=("README.md",), generator=gen_strip,
    extra={"width": {"README.md": 840}},
), module="make_gui_media.py")

GENERATORS = (("gui-layout-annotated", gen_layout), ("gui-source-badge", gen_badge),
              ("gui-source-derived", gen_derived), ("gui-strip", gen_strip))


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    errors = verify_captures()
    for e in errors:
        print("error:", e)
    if "--check" in argv:
        for name, _fn in GENERATORS:
            for theme in ("light", "dark"):
                p = OUT / f"{name}-{theme}.png"
                if not p.exists():
                    errors.append(f"{name}: {p.name} missing")
                    print("error:", errors[-1])
        print(f"gui media check: {len(errors)} error(s)")
        return 1 if errors else 0
    if errors:
        return 1
    only = [a for a in argv if not a.startswith("--")]
    for name, fn in GENERATORS:
        if only and not any(o in name for o in only):
            continue
        for p in fn():
            print(f"wrote {registry.rel_to_repo(p)} ({p.stat().st_size / 1024:.1f} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
