"""Figure registry and shared helpers for the simulator-driven figures.

Each figure is a :class:`FigureSpec`: ``compute(fast)`` runs the simulator once and
returns plain data; ``draw(data, theme)`` turns that data into a matplotlib figure
(or an SVG string, or a list of frames) for one theme. :mod:`make_all` runs
``compute`` once and ``draw`` twice (light and dark), writing ``<name>-light.png`` /
``<name>-dark.png`` under ``docs/assets/images/sim/`` (``site/`` for the History /
Nodes / Future pages, ``anim/`` for animations).

The canonical alt text and caption of every figure live here, next to the code that
makes it, so the page embeds can be regenerated (``make_all.py --catalog``) and
checked (``make_all.py --check``). The embed forms and the file naming are shared with
the narrative/schematic registry in ``docs/figures/registry.py`` (DESIGN §5.0):
``kind`` picks the size cap, ``fmt="apng"`` makes ``draw`` return a frame list,
``extra["crops"]`` expands into derived ``<name>-<crop>`` specs, ``extra["model_rows"]``
resolves abbreviated caption labels to matrix rows, ``extra["table"]`` declares a
marker table for ``--tables``.

Physical-correctness asserts belong in ``compute()``, never in ``draw``: ``make_all.py
--verify`` runs every ``compute(fast)`` without drawing, which is the only form CI runs.
"""

from __future__ import annotations

import os
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
DOCS = REPO / "docs"
OUT_DIR = DOCS / "assets" / "images" / "sim"
FAST_DIR = HERE / "_fast"
CACHE_DIR = HERE / "_cache"

if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))
if str(HERE.parent) not in sys.path:  # docs/figures: tokens, svgkit, registry
    sys.path.insert(0, str(HERE.parent))

import registry  # noqa: E402

#: Set by ``make_all.py --frames N``: animation specs may cap their frame lists at this (None = full).
FRAME_LIMIT: int | None = None


def native():
    """The compiled simulator module, with a helpful error if it is not built."""
    try:
        import highuvlith._native as huv  # noqa: PLC0415
    except ImportError as exc:  # pragma: no cover - environment problem
        raise SystemExit(
            "highuvlith is not importable. Build the extension first, e.g.\n"
            "  maturin develop --release -m crates/highuvlith-py/Cargo.toml\n"
            "or put a built python/highuvlith/_native*.so on PYTHONPATH=python.\n"
            f"({exc})"
        ) from exc
    return huv


@dataclass
class FigureSpec:
    """One gallery figure: how to compute it, draw it, and describe it."""

    name: str
    group: str
    compute: Callable[[bool], Any]
    draw: Callable[[Any, Any], Any]
    alt: str
    caption: str
    #: Where the figure is embedded (paths relative to docs/, or "README.md").
    pages: tuple[str, ...] = ()
    #: "" for docs/assets/images/sim/, "site" for .../sim/site/, "anim" for .../sim/anim/.
    subdir: str = ""
    #: "png" (2x, optimized), "svg" (matplotlib, or a string returned by draw), "apng" (frame list).
    fmt: str = "png"
    #: Capabilities the figure needs (see ``capabilities()``); missing -> skipped.
    requires: tuple[str, ...] = ()
    #: Set False to keep a figure out of the default run (e.g. pending a fix).
    enabled: bool = True
    disabled_reason: str = ""
    extra: dict[str, Any] = field(default_factory=dict)
    #: Registry contract (DESIGN §5.0): the size-cap kind; defaults to "figure".
    kind: str = "figure"
    #: None, "svg" (two-layer CSS animation; draw returns the animated SVG string) or "apng".
    animated: str | None = None
    #: For fmt="apng": index of the frame written as the designed still.
    still_frame: int = 0
    narrow: bool = False
    hero_on: tuple[str, ...] = ()
    platform_independent: bool = False
    root: Path = OUT_DIR
    still: bool = False
    single: bool = False
    module: str = ""

    def __post_init__(self) -> None:
        if self.fmt == "apng":
            self.animated = "apng"
            self.kind = "anim"
        elif self.fmt == "svg" and self.name.endswith("-anim"):
            self.animated = "svg"
            self.kind = "anim"
        if self.animated:
            self.still = True
        if self.kind not in registry.KINDS:
            raise ValueError(f"{self.name}: kind {self.kind!r} not in {registry.KINDS}")
        if not self.alt.strip():
            raise ValueError(f"{self.name}: alt must not be empty (length ≤ 300 is a --check rule)")

    @property
    def rel_stem(self) -> str:
        """Image path stem relative to docs/assets/images/sim/."""
        return f"{self.subdir}/{self.name}" if self.subdir else self.name

    def image_paths(self, out_dir: Path = OUT_DIR) -> list[Path]:
        """Every file the spec ships (main light/dark, stills, narrow twins, GIFs)."""
        return list(registry.all_files(self, out_dir).values())

    @property
    def is_crop(self) -> bool:
        return "crop_of" in self.extra


REGISTRY: list[FigureSpec] = []


def register(spec: FigureSpec) -> FigureSpec:
    """Register a spec and the derived crops of ``extra['crops']``; duplicate names are rejected."""
    module = spec.compute.__module__ if hasattr(spec.compute, "__module__") else ""
    spec.module = f"{module}.py" if module else ""
    registry._claim(spec.name, spec.module or "sim")
    REGISTRY.append(spec)
    for crop in registry.derive_crops(spec, lambda **f: FigureSpec(group=spec.group, compute=spec.compute,
                                                                    draw=spec.draw, root=spec.root, **f)):
        crop.module = spec.module
        registry._claim(crop.name, spec.module or "sim")
        REGISTRY.append(crop)
    return spec


def cli_binary() -> Path | None:
    """The built CLI (``$HUV_CLI`` or ``target/release/highuvlith``), or None."""
    env = os.environ.get("HUV_CLI")
    candidates = [Path(env)] if env else []
    candidates += [REPO / "target" / "release" / "highuvlith", REPO / "target" / "release" / "highuvlith.exe"]
    for c in candidates:
        if c.is_file() and os.access(c, os.X_OK):
            return c
    return None


def capabilities() -> dict[str, bool]:
    """Optional simulator API and tools the figures probe for (older/newer builds differ)."""
    huv = native()
    src = huv.SourceConfig.f2_laser(0.7)
    return {
        "illumination": hasattr(src, "with_illumination"),
        "spectrum": callable(getattr(src, "spectrum", None)),
        "pupil_fill": hasattr(src, "pupil_fill"),
        "cli": cli_binary() is not None,
        "attenuation": callable(getattr(huv, "compound_attenuation", None)),
    }


def md_embed(spec, page: str) -> str:
    """Markdown that embeds ``spec`` into ``page`` (delegates to ``registry.md_embed``)."""
    return registry.md_embed(spec, page)


# ---------------------------------------------------------------------------
# Small numeric helpers shared by the figure modules
# ---------------------------------------------------------------------------


def contrast(profile) -> float:
    """(Imax - Imin) / (Imax + Imin) of a 1D profile."""
    import numpy as np  # noqa: PLC0415

    p = np.asarray(profile, dtype=float)
    hi, lo = float(p.max()), float(p.min())
    return (hi - lo) / (hi + lo) if hi + lo > 0 else 0.0


def centre_row(image):
    """The y = 0 cross-section of an (ny, nx) image (row ny // 2)."""
    import numpy as np  # noqa: PLC0415

    img = np.asarray(image)
    return img[img.shape[0] // 2]


def fmt_si(value: float, unit: str) -> str:
    """Compact engineering notation with 3 significant digits, e.g. 1.2e-6 W -> '1.2 µW'."""
    import math  # noqa: PLC0415

    if value == 0 or not math.isfinite(value):
        return f"{value:g} {unit}"
    prefixes = [(1e3, "k"), (1.0, ""), (1e-3, "m"), (1e-6, "µ"), (1e-9, "n"), (1e-12, "p")]
    v = float(f"{value:.3g}")  # round first so 999.99 -> 1000 -> '1 k'
    for scale, p in prefixes:
        if abs(v) >= scale:
            return f"{float(f'{v / scale:.3g}'):g} {p}{unit}"
    return f"{v:.2g} {unit}"
