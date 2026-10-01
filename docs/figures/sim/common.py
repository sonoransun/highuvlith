"""Figure registry and shared helpers for the simulator-driven figures.

Each figure is a :class:`FigureSpec`: ``compute(fast)`` runs the simulator once and
returns plain data; ``draw(data, theme)`` turns that data into a matplotlib figure
for one theme. :mod:`make_all` runs ``compute`` once and ``draw`` twice (light and
dark), writing ``<name>-light.png`` / ``<name>-dark.png`` under
``docs/assets/images/sim/`` (``site/`` for the History / Nodes / Future pages).

The canonical alt text and caption of every figure live here, next to the code that
makes it, so the page embeds can be regenerated (``make_all.py --catalog``) and
checked (``make_all.py --check``).
"""

from __future__ import annotations

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
    #: "" for docs/assets/images/sim/, "site" for .../sim/site/.
    subdir: str = ""
    fmt: str = "png"
    #: Capabilities the figure needs (see ``capabilities()``); missing -> skipped.
    requires: tuple[str, ...] = ()
    #: Set False to keep a figure out of the default run (e.g. pending a fix).
    enabled: bool = True
    disabled_reason: str = ""
    extra: dict[str, Any] = field(default_factory=dict)

    @property
    def rel_stem(self) -> str:
        """Image path stem relative to docs/assets/images/sim/."""
        return f"{self.subdir}/{self.name}" if self.subdir else self.name

    def image_paths(self, out_dir: Path = OUT_DIR) -> list[Path]:
        return [out_dir / f"{self.rel_stem}-{t}.{self.fmt}" for t in ("light", "dark")]


REGISTRY: list[FigureSpec] = []


def register(spec: FigureSpec) -> FigureSpec:
    if any(s.name == spec.name for s in REGISTRY):
        raise ValueError(f"duplicate figure name {spec.name!r}")
    REGISTRY.append(spec)
    return spec


def capabilities() -> dict[str, bool]:
    """Optional simulator API the figures probe for (older/newer builds differ)."""
    huv = native()
    src = huv.SourceConfig.f2_laser(0.7)
    return {
        "illumination": hasattr(src, "with_illumination"),
        "spectrum": callable(getattr(src, "spectrum", None)),
        "pupil_fill": hasattr(src, "pupil_fill"),
    }


def md_embed(spec: FigureSpec, page: str) -> str:
    """Markdown that embeds ``spec`` into ``page`` (a path relative to docs/, or README.md).

    Docs pages get a ``<figure markdown="span">`` whose blank lines keep it valid
    on both renderers: MkDocs (md_in_html) produces ``<figure><img><img>
    <figcaption>``, and github.com (CommonMark HTML blocks end at blank lines)
    renders the two Markdown images and the caption. The
    ``#gh-light-mode-only`` / ``#gh-dark-mode-only`` fragments make MkDocs
    Material show one variant per colour scheme (github.com has honoured the
    same fragments). The README uses a ``<picture>`` element instead.
    """
    if page == "README.md":
        base = f"docs/assets/images/sim/{spec.rel_stem}"
        return (
            "<picture>\n"
            f'  <source media="(prefers-color-scheme: dark)" srcset="{base}-dark.{spec.fmt}">\n'
            f'  <img alt="{spec.alt}" src="{base}-light.{spec.fmt}">\n'
            "</picture>"
        )
    depth = len(Path(page).parent.parts)
    prefix = "../" * depth
    base = f"{prefix}assets/images/sim/{spec.rel_stem}"
    return (
        '<figure markdown="span">\n\n'
        f"![{spec.alt}]({base}-light.{spec.fmt}#gh-light-mode-only)\n"
        f"![{spec.alt}]({base}-dark.{spec.fmt}#gh-dark-mode-only)\n\n"
        f"<figcaption>{spec.caption}</figcaption>\n"
        "</figure>"
    )


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
