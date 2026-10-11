#!/usr/bin/env python3
"""D2 · D3 · D4 — the pipeline stage strip, the architecture layers and the power planes.

Scripted SVG (svgkit, standard library only → platform independent), light/dark pairs
with narrow twins, written to ``docs/assets/images/diagrams/``:

* ``pipeline-stages`` — seven stage boxes Source · Optics · Mask · Image · Film · Resist ·
  Metrics with the grade chip of the matrix row named in ``STAGE_ROWS`` (so a grade
  change reflows the strip), the three bypass paths (LIGA · interference · Talbot) and
  the Sources / Processes wing brackets;
* ``architecture-layers`` — front ends → ``highuvlith-py`` bindings → ``highuvlith-core``
  with its five blocks; ``CORE_BLOCKS`` is checked against ``crates/highuvlith-core/src/``;
* ``power-planes`` — the same watt in five planes (plasma → 2π → collector → IF →
  illuminator/mask/projection → wafer) with the factors of ``PLANE_FACTORS``, each
  labelled typical / illustrative or calibrated, as stated on ``sources/lpp.md`` and
  ``sources/dpp.md``.

    python docs/figures/diagrams/make_pipeline_diagrams.py            # write all six pairs
    python docs/figures/diagrams/make_pipeline_diagrams.py --check    # STAGE_ROWS resolve, CORE_BLOCKS == crate dirs
"""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / "reference"))

import matrix_parse  # noqa: E402
import registry  # noqa: E402
import svgkit as K  # noqa: E402

REPO = registry.REPO
OUT = registry.IMAGES / "diagrams"
GENERATOR = "docs/figures/diagrams/make_pipeline_diagrams.py"
CORE_SRC = REPO / "crates" / "highuvlith-core" / "src"

# ---------------------------------------------------------------------------
# D2 data: stage -> (two-word subtitle, matrix row whose headline grade is the chip)
# ---------------------------------------------------------------------------

STAGE_ROWS: dict[str, tuple[str, str]] = {
    "Source": ("sampled fill", "Illumination pupil fills"),
    "Optics": ("pupil + defocus", "Refractive projection optics (dry)"),
    "Mask": ("exact spectrum", "Exact thin-mask spectrum"),
    "Image": ("Hopkins / SOCS", "Hopkins imaging (factorized TCC/SOCS)"),
    "Film": ("standing waves", "Thin-film transfer matrix"),
    "Resist": ("bake · develop", "Volumetric (z-resolved) exposure"),
    "Metrics": ("CD · NILS · window", "CD / ILS / NILS / MEEF metrics"),
}
#: Bypass paths that skip projection optics: label -> (matrix row, enters at stage)
BYPASS: dict[str, tuple[str, str]] = {
    "LIGA shadow print": ("LIGA / deep X-ray depth dose and exposure time", "Film"),
    "interference": ("Multi-beam interference / two-photon", "Resist"),
    "Talbot": ("Talbot / DTL / ATL proximity lithography", "Resist"),
}

# ---------------------------------------------------------------------------
# D3 data: core blocks -> the files/directories under crates/highuvlith-core/src/
# ---------------------------------------------------------------------------

CORE_BLOCKS: list[tuple[str, tuple[str, ...]]] = [
    ("Imaging", ("aerial.rs", "mask.rs", "metrics.rs", "math", "compute")),
    ("Optics & materials", ("optics", "materials", "thinfilm.rs")),
    ("Sources", ("source.rs", "source_models")),
    ("Process & deep layers", ("resist.rs", "volumetric.rs", "process.rs", "deep_xray.rs", "interference.rs",
                               "talbot.rs", "grayscale.rs")),
    ("Optimization & research", ("ilt.rs", "opc.rs", "sraf.rs", "double_patterning.rs", "dsa.rs", "stochastic.rs",
                                 "ptychography.rs", "quantum.rs", "mnsl.rs")),
]
#: Crate plumbing that no block claims (checked so the list stays complete).
CORE_INFRA = ("lib.rs", "error.rs", "types.rs", "io")
FRONT_ENDS = ("Python", "CLI", "GUI", "Notebooks")
TRAITS = ("LithographySource", "Optics", "Mask", "SimulationEngine")

# ---------------------------------------------------------------------------
# D4 data: planes and the factors between them, as stated on the source pages
# ---------------------------------------------------------------------------

PLANES = [
    ("Plasma / undulator", "drive power in"),
    ("Into 2π sr", "in-band emission"),
    ("Collector", "collected"),
    ("Intermediate focus", "at IF — the scanner hand-off"),
    ("Illuminator · mask · projection", "10 mirrors × mask"),
    ("Wafer", "at the wafer"),
]
#: (label, provenance word, source) between consecutive planes
PLANE_FACTORS = [
    ("× CE (6 % Sn)", "typical / illustrative", "sources/lpp.md § The light"),
    ("× Ω_c · η_c", "assumed (DPP)", "sources/dpp.md § The power chain"),
    ("× η = 250/1290 · f_étendue", "calibrated", "sources/lpp.md § The power chain"),
    ("× 0.70¹⁰ · 0.65", "illustrative", "sources/lpp.md § From source power to wafers per hour"),
    ("= dose × area / time", "throughput model 🔶", "sources/index.md § Throughput"),
]


# ---------------------------------------------------------------------------
# D2 — pipeline-stages
# ---------------------------------------------------------------------------


def _stage_chip_slug(stage: str) -> str:
    row = matrix_parse.find(STAGE_ROWS[stage][1])
    if row is None:
        raise SystemExit(f"STAGE_ROWS: no matrix row named {STAGE_ROWS[stage][1]!r}")
    return row.slug


def draw_pipeline(theme: str, narrow: bool = False) -> str:
    stages = list(STAGE_ROWS)
    if not narrow:
        W, H = 960, 200
        s = K.Svg(W, H, theme, "The seven simulation stages", PIPELINE_ALT, generator=GENERATOR)
        bw, gap, m = 120, 12, 24
        y0, bh = 34, 76
        pos = {st: (m + i * (bw + gap), y0) for i, st in enumerate(stages)}
        rows_boxes = [stages]
    else:
        W, H = 600, 340
        s = K.Svg(W, H, theme, "The seven simulation stages", PIPELINE_ALT, generator=GENERATOR)
        bw, gap, m = 132, 12, 12
        bh = 76
        pos = {}
        for i, st in enumerate(stages[:4]):
            pos[st] = (m + i * (bw + gap), 40)
        for i, st in enumerate(stages[4:]):
            pos[st] = (m + (bw + gap) / 2 + i * (bw + gap), 170)
        rows_boxes = [stages[:4], stages[4:]]
    # wing brackets
    def bracket(x1, x2, y, label):
        s.line(x1, y, x2, y, "hue:simulator", 1.6)
        s.line(x1, y, x1, y + 6, "hue:simulator", 1.6)
        s.line(x2, y, x2, y + 6, "hue:simulator", 1.6)
        s.eyebrow((x1 + x2) / 2, y - 5, label, size=12.5, color="hue:simulator", anchor="middle")

    sx, sy = pos["Source"]
    bracket(sx, sx + bw, sy - 12, "Sources wing")
    fx, fy = pos["Film"]
    mx, my = pos["Metrics"]
    if narrow:
        bracket(fx, mx + bw, fy - 12, "Processes wing")
    else:
        bracket(fx, mx + bw, fy - 12, "Processes wing")
    # boxes
    for st in stages:
        x, y = pos[st]
        sub, _row = STAGE_ROWS[st]
        s.rect(x, y, bw, bh, fill=s.tint("hue:simulator", 0.06), stroke="axis", sw=1.0, rx=6)
        s.text(x + bw / 2, y + 22, st, size=14, color="ink", weight=600)
        s.text(x + bw / 2, y + 39, sub, size=12.5, color="ink2")
        K.chip(s, x + bw / 2, y + 60, _stage_chip_slug(st), size=13.5, anchor="middle")
    # arrows between consecutive boxes
    for a, b in zip(stages, stages[1:]):
        (xa, ya), (xb, yb) = pos[a], pos[b]
        if abs(ya - yb) < 1:
            s.line(xa + bw + 1, ya + bh / 2, xb - 2, yb + bh / 2, "ink2", 1.4, arrow="end")
        else:  # narrow: row wrap
            s.path(f"M{K.fmt(xa + bw / 2)},{K.fmt(ya + bh)} C{K.fmt(xa + bw / 2)},{K.fmt(ya + bh + 40)} "
                   f"{K.fmt(xb + bw / 2)},{K.fmt(yb - 40)} {K.fmt(xb + bw / 2)},{K.fmt(yb - 2)}", stroke="ink2", sw=1.4, arrow="end")
    # bypass paths: from under Source, curving below the row into Film / Resist
    yb = (pos["Source"][1] + bh) + (18 if not narrow else 14)
    labels_y = H - 10
    for i, (label, (row_name, enters)) in enumerate(BYPASS.items()):
        row = matrix_parse.find(row_name)
        if row is None:
            raise SystemExit(f"BYPASS: no matrix row named {row_name!r}")
        x1 = pos["Source"][0] + bw * (0.25 + 0.25 * i)
        y1 = pos["Source"][1] + bh
        ex, ey = pos[enters]
        x2 = ex + bw * (0.3 + 0.2 * i)
        y2 = ey + bh if not narrow else ey
        depth = yb + 12 * i + 8
        if not narrow:
            d = (f"M{K.fmt(x1)},{K.fmt(y1)} C{K.fmt(x1)},{K.fmt(depth + 22)} {K.fmt(x2)},{K.fmt(depth + 22)} "
                 f"{K.fmt(x2)},{K.fmt(y2 + 2)}")
            s.path(d, stroke="hue:simulator", sw=1.3, dash="5 4", arrow="end", opacity=0.9)
            tx = 60 + i * 320
            cw = K.chip(s, tx, labels_y - 5, row.slug, size=12.5, word=False)
            s.text(tx + cw + 6, labels_y, f"{label} — bypasses projection optics", size=12.5, anchor="start", color="ink2")
        else:
            d = (f"M{K.fmt(x1)},{K.fmt(y1)} C{K.fmt(x1)},{K.fmt(y1 + 40)} {K.fmt(x2 - 60)},{K.fmt(y2 - 30)} "
                 f"{K.fmt(x2)},{K.fmt(y2 - 2)}")
            s.path(d, stroke="hue:simulator", sw=1.3, dash="5 4", arrow="end", opacity=0.9)
            ly = H - 58 + 18 * i
            cw = K.chip(s, 14, ly - 5, row.slug, size=12.5, word=False)
            s.text(14 + cw + 6, ly, f"{label} — bypasses projection optics", size=12.5, anchor="start", color="ink2")
    return s.render()


PIPELINE_ALT = ("Seven stage boxes in a row, Source to Metrics, each with its grade chip, and three dashed bypass "
                "arrows for LIGA, interference and Talbot curving under the row into the film and resist stages.")
PIPELINE_CAPTION = ("<strong>Every simulation runs the same seven stages.</strong> Chips are the stage grades from the "
                    "capability matrix; the bypass paths skip projection optics. <em>Chips copied from the matrix.</em>")


# ---------------------------------------------------------------------------
# D3 — architecture-layers
# ---------------------------------------------------------------------------


def draw_architecture(theme: str, narrow: bool = False) -> str:
    longest = max(len(files) for _t, files in CORE_BLOCKS)
    W, H = (960, 196 + 70 + longest * 15) if not narrow else (600, 360)
    s = K.Svg(W, H, theme, "One physics crate, thin adapters", ARCH_ALT, generator=GENERATOR)
    m = 24 if not narrow else 12
    inner = W - 2 * m
    # tier 1: front ends
    y1, h1 = 22, 50
    n = len(FRONT_ENDS)
    gap = 12
    fw = (inner - (n - 1) * gap) / n
    s.eyebrow(m, y1 - 6, "Front ends", size=12.5)
    for i, fe in enumerate(FRONT_ENDS):
        x = m + i * (fw + gap)
        s.rect(x, y1, fw, h1, fill=s.tint("hue:reference", 0.07), stroke="axis", sw=1.0, rx=6)
        s.text(x + fw / 2, y1 + h1 / 2 + 5, fe, size=14, color="ink", weight=600)
        s.line(x + fw / 2, y1 + h1 + 1, x + fw / 2, y1 + h1 + 26, "ink2", 1.3, arrow="end")
    # tier 2: bindings
    y2, h2 = y1 + h1 + 28, 40
    s.rect(m, y2, inner, h2, fill=s.tint("brand:violet600", 0.10), stroke="brand:violet600", sw=1.0, rx=6)
    s.text(m + 12, y2 + h2 / 2 + 5, "highuvlith-py — PyO3 bindings (highuvlith._native) · the CLI and GUI link the core directly" if not narrow
           else "highuvlith-py — PyO3 bindings · CLI and GUI link the core", size=13, anchor="start", color="ink", weight=500)
    s.line(W / 2, y2 + h2 + 1, W / 2, y2 + h2 + 30, "ink2", 1.3, arrow="end")
    if not narrow:
        s.text(W / 2 + 8, y2 + h2 + 19, " · ".join(TRAITS), size=12.5, anchor="start", color="ink2", italic=True)
    else:
        s.text(W / 2 + 8, y2 + h2 + 19, " · ".join(TRAITS[:2]) + " · …", size=12.5, anchor="start", color="ink2", italic=True)
    # tier 3: core
    y3 = y2 + h2 + 34
    h3 = H - y3 - 16
    s.rect(m, y3, inner, h3, fill=s.tint("hue:simulator", 0.05), stroke="hue:simulator", sw=1.2, rx=8)
    s.eyebrow(m + 12, y3 + 18, "highuvlith-core — every physics computation", size=12.5, color="hue:simulator")
    nb = len(CORE_BLOCKS)
    if not narrow:
        bw = (inner - 24 - (nb - 1) * gap) / nb
        for i, (title, files) in enumerate(CORE_BLOCKS):
            x = m + 12 + i * (bw + gap)
            y = y3 + 30
            bh = h3 - 42
            s.rect(x, y, bw, bh, fill="surface", stroke="axis", sw=1.0, rx=5)
            s.text(x + 8, y + 20, title, size=13.5, anchor="start", color="ink", weight=600)
            for j, f in enumerate(files):
                s.text(x + 8, y + 40 + j * 15, f, size=12.5, anchor="start", color="ink2", mono=True)
    else:
        cols = 2
        bw = (inner - 24 - gap) / cols
        bh = (h3 - 42 - gap * 2) / 3
        for i, (title, files) in enumerate(CORE_BLOCKS):
            x = m + 12 + (i % cols) * (bw + gap)
            y = y3 + 30 + (i // cols) * (bh + gap)
            s.rect(x, y, bw, bh, fill="surface", stroke="axis", sw=1.0, rx=5)
            s.text(x + 8, y + 18, title, size=13, anchor="start", color="ink", weight=600)
            s.text(x + 8, y + 34, ", ".join(f.replace(".rs", "") for f in files[:3]) + (" …" if len(files) > 3 else ""),
                   size=12.5, anchor="start", color="ink2", mono=True)
    return s.render()


ARCH_ALT = ("Three tiers: four front ends (Python, CLI, GUI, Notebooks) above the PyO3 bindings crate, above the "
            "highuvlith-core crate with five labelled blocks: imaging, optics and materials, sources, process and deep "
            "layers, optimization and research.")
ARCH_CAPTION = ("<strong>One physics crate, thin adapters.</strong> Front ends call the bindings; the bindings call the "
                "core; nothing in a front end computes physics. <em>Schematic of the crate layout; no model badge.</em>")


# ---------------------------------------------------------------------------
# D4 — power-planes
# ---------------------------------------------------------------------------


def draw_power_planes(theme: str, narrow: bool = False) -> str:
    W, H = (960, 240) if not narrow else (600, 384)
    s = K.Svg(W, H, theme, "The same watt, five planes", PLANES_ALT, generator=GENERATOR)
    m = 12
    n = len(PLANES)
    names = [("Plasma or", "undulator"), ("Into 2π sr",), ("Collector",), ("Intermediate", "focus"),
             ("Illuminator ·", "mask · projection"), ("Wafer",)]
    if not narrow:
        gap = 46
        bw = (W - 2 * m - (n - 1) * gap) / n
        yb, bh = 92, 60
        pos = [(m + i * (bw + gap), yb) for i in range(n)]
    else:
        gap = 60
        bw = (W - 2 * m - 2 * gap) / 3
        bh = 60
        pos = [(m + (i % 3) * (bw + gap), 100 + (i // 3) * 170) for i in range(n)]
    s.eyebrow(m, 20, "Where the watts are counted — the convention name under each plane", size=12.5)
    for i, ((name, note), lines, (x, y)) in enumerate(zip(PLANES, names, pos)):
        hot = i == 3  # intermediate focus: the plane every scanner number is quoted at
        s.rect(x, y, bw, bh, fill=s.tint("hue:simulator", 0.12 if hot else 0.05), stroke="hue:simulator" if hot else "axis",
               sw=1.4 if hot else 1.0, rx=6)
        for li, line in enumerate(lines):
            s.text(x + bw / 2, y + 18 + li * 15, line, size=13, color="ink", weight=600)
        s.text(x + bw / 2, y + bh - 8, note if len(note) < 22 else note.split(" — ")[0], size=12.5, color="ink2", italic=True)
    for i, (label, prov, _src) in enumerate(PLANE_FACTORS):
        (xa, ya), (xb, yb2) = pos[i], pos[i + 1]
        txt = f"{label} — {prov}"
        if abs(ya - yb2) < 1:
            s.line(xa + bw + 1, ya + bh / 2, xb - 2, yb2 + bh / 2, "ink2", 1.3, arrow="end")
            cx = (xa + bw + xb) / 2
            ly = ya - 16 - (i % 2) * 16  # alternate heights so neighbouring labels never collide
            s.line(cx, ly + 4, cx, ya + bh / 2 - 6, "axis", 0.8, opacity=0.8)
            s.text(cx, ly, txt, size=12.5, color="hue:simulator" if prov.startswith("calibrated") else "ink2")
        else:
            s.path(f"M{K.fmt(xa + bw / 2)},{K.fmt(ya + bh)} C{K.fmt(xa + bw / 2)},{K.fmt(ya + bh + 60)} "
                   f"{K.fmt(xb + bw / 2)},{K.fmt(yb2 - 60)} {K.fmt(xb + bw / 2)},{K.fmt(yb2 - 2)}", stroke="ink2", sw=1.3, arrow="end")
            s.text(W / 2, ya + bh + 56, txt, size=12.5, color="hue:simulator" if prov.startswith("calibrated") else "ink2")
    foot = ("Factors are illustrative orders of magnitude; η = 250/1290 is calibrated to a reported pair "
            "(NXE:3400B: 21.5 kW × 6 % → 250 W at IF). Wafer-side losses belong to the throughput model.")
    cut = foot.index("(NXE")
    s.text(m, H - 24, foot[:cut].strip(), size=12.5, anchor="start", color="ink2")
    s.text(m, H - 8, foot[cut:].strip(), size=12.5, anchor="start", color="ink2")
    return s.render()


PLANES_ALT = ("Six boxes in a row from plasma or undulator through into-2π, collector, intermediate focus and the "
              "illuminator-mask-projection train to the wafer, with the multiplicative factor between each pair labelled "
              "typical, assumed, calibrated or illustrative.")
PLANES_CAPTION = ("<strong>The same watt, five planes.</strong> Numbers on this site always say which plane they are in; "
                  "factors are illustrative orders of magnitude, η is calibrated to a reported pair. "
                  "<em>Schematic; no model badge.</em>")


# ---------------------------------------------------------------------------
# Registration, generation, check
# ---------------------------------------------------------------------------


def gen_pipeline() -> list[Path]:
    return K.write_pair("pipeline-stages", lambda t: draw_pipeline(t), OUT, narrow=lambda t: draw_pipeline(t, narrow=True))


def gen_architecture() -> list[Path]:
    return K.write_pair("architecture-layers", lambda t: draw_architecture(t), OUT, narrow=lambda t: draw_architecture(t, narrow=True))


def gen_power_planes() -> list[Path]:
    return K.write_pair("power-planes", lambda t: draw_power_planes(t), OUT, narrow=lambda t: draw_power_planes(t, narrow=True))


registry.register(registry.AssetSpec(
    name="pipeline-stages", kind="strip", group="diagrams", subdir="diagrams", fmt="svg", narrow=True,
    alt=PIPELINE_ALT, caption=PIPELINE_CAPTION,
    pages=("pipeline.md", "simulator/index.md", "processes/index.md", "README.md"),
    hero_on=("simulator/index.md",), generator=gen_pipeline, platform_independent=True,
    extra={"width": {"README.md": 820}},  # README <picture> width (DESIGN §6.2; INT late-binding edit)
), module="make_pipeline_diagrams.py")

registry.register(registry.AssetSpec(
    name="architecture-layers", kind="schematic", group="diagrams", subdir="diagrams", fmt="svg", narrow=True,
    alt=ARCH_ALT, caption=ARCH_CAPTION,
    pages=("architecture.md", "extending.md", "README.md"), hero_on=("architecture.md", "extending.md"),
    generator=gen_architecture, platform_independent=True,
    extra={"width": {"README.md": 820}},  # README <picture> width (DESIGN §6.2; INT late-binding edit)
), module="make_pipeline_diagrams.py")

registry.register(registry.AssetSpec(
    name="power-planes", kind="schematic", group="diagrams", subdir="diagrams", fmt="svg", narrow=True,
    alt=PLANES_ALT, caption=PLANES_CAPTION,
    pages=("sources/index.md",), generator=gen_power_planes, platform_independent=True,
), module="make_pipeline_diagrams.py")


def check() -> tuple[list[str], list[str]]:
    errors: list[str] = []
    warnings: list[str] = []
    for stage, (_sub, row) in STAGE_ROWS.items():
        if matrix_parse.find(row) is None:
            errors.append(f"pipeline-stages: STAGE_ROWS[{stage!r}] names no matrix row: {row!r}")
    for label, (row, enters) in BYPASS.items():
        if matrix_parse.find(row) is None:
            errors.append(f"pipeline-stages: BYPASS[{label!r}] names no matrix row: {row!r}")
        if enters not in STAGE_ROWS:
            errors.append(f"pipeline-stages: BYPASS[{label!r}] enters unknown stage {enters!r}")
    if CORE_SRC.exists():
        listed = {f for _t, files in CORE_BLOCKS for f in files}
        present = {p.name for p in CORE_SRC.iterdir() if not p.name.startswith(".")}
        for f in sorted(listed - present):
            errors.append(f"architecture-layers: CORE_BLOCKS lists {f!r}, absent from crates/highuvlith-core/src/")
        for f in sorted(present - listed - set(CORE_INFRA)):
            errors.append(f"architecture-layers: crates/highuvlith-core/src/{f} is in no CORE_BLOCKS entry")
    else:
        warnings.append("architecture-layers: crates/highuvlith-core/src/ not found; CORE_BLOCKS not checked")
    return errors, warnings


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if "--check" in argv:
        e, w = check()
        for x in w:
            print("warning:", x)
        for x in e:
            print("error:", x)
        print(f"pipeline diagrams check: {len(e)} error(s), {len(w)} warning(s)")
        return 1 if e else 0
    only = [a for a in argv if not a.startswith("--")]
    for name, fn in (("pipeline-stages", gen_pipeline), ("architecture-layers", gen_architecture), ("power-planes", gen_power_planes)):
        if only and not any(o in name for o in only):
            continue
        for p in fn():
            print(f"wrote {registry.rel_to_repo(p)} ({p.stat().st_size / 1024:.1f} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
