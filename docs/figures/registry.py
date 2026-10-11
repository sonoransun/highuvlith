"""The asset registry: every image under ``docs/assets/images/`` is a registered spec.

Two spec types share one contract (DESIGN §5.0):

* :class:`FigureSpec` (``docs/figures/sim/common.py``) — engine figures: ``compute(fast)``
  + ``draw(data, theme)``, rendered by ``make_all.py``;
* :class:`AssetSpec` (this module) — schematics, data charts, diagrams and animations
  drawn from geometry, closed forms or a CSV by a ``generator()`` callable (svgkit or
  matplotlib), plus the hash-pinned GUI ``capture`` entries that no script produces.

Both carry ``name, kind, alt, caption, pages, root, subdir, fmt, animated, still,
narrow, hero_on, requires, extra, platform_independent`` and are checked by
``check_figures.py`` with the same rules (sizes per ``kind``, embeds per page, captions,
SVG hygiene). ``pages=`` lists every page that embeds the asset (paths relative to
``docs/``, or ``README.md``); a page is the only multiplier of a figure.

Naming (``<root>/<subdir>/``):

* static: ``<name>-{light,dark}.<fmt>``;
* animated SVG (``animated="svg"``): ``<name>-anim-{light,dark}.svg`` + the designed
  still ``<name>-still-{light,dark}.svg`` (a spec may be *named* ``…-anim``: the files
  are then ``<name>-{light,dark}.svg`` and ``<name minus -anim>-still-…``);
* animated raster (``fmt="apng"``): ``<name>-anim-{light,dark}.png`` (an APNG stored as
  .png) + ``<name>-still-{light,dark}.png`` + ``<name>-anim-{light,dark}.gif`` when
  ``extra={"gif": True}`` (README only);
* narrow twins (``narrow=True``): ``<name>-narrow-{light,dark}.svg``;
* a ``capture`` (``single=True``): one file ``<name>.<fmt>`` with ``pinned_sha256``.

Crops are derived specs: a parent declares ``extra["crops"] = {"<crop>": dict(pages=…,
alt=…, caption=…, kind=…, box=…)}`` and :func:`register` expands each into a spec named
``<parent>-<crop>`` with its own pages/alt/caption/kind, inheriting generator, root,
subdir, fmt and platform_independent; the parent's renderer writes
``<parent>-<crop>-{light,dark}.<fmt>`` from the same output (``box`` = a pixel box for
PNG, a sub-``viewBox`` for SVG). A crop page is listed only on the derived spec.

:func:`md_embed` is the only writer of page embeds (the five forms of DESIGN §4.6);
``--sync-embeds`` rewrites existing blocks from it and ``--check`` compares pages with it.
"""

from __future__ import annotations

import hashlib
import json
import re
import struct
import sys
from dataclasses import dataclass, field, replace
from pathlib import Path
from typing import Any, Callable

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
DOCS = REPO / "docs"
IMAGES = DOCS / "assets" / "images"

KINDS = ("figure", "sheet", "strip", "schematic", "anim", "capture")
THEMES = ("light", "dark")

#: Size caps in KB per variant: (error above, warn at). Keyed by (kind, fmt) with the
#: special keys "narrow", "apng", "gif" and ("figure", "svg") = matplotlib SVG.
SIZE_CAPS_KB: dict[Any, tuple[int, int]] = {
    ("figure", "png"): (400, 320),
    ("sheet", "png"): (500, 400),
    ("capture", "png"): (500, 400),
    ("schematic", "svg"): (60, 48),
    ("strip", "svg"): (60, 48),
    ("figure", "svg"): (120, 96),
    ("sheet", "svg"): (120, 96),
    ("anim", "svg"): (120, 96),
    "narrow": (40, 32),
    "apng": (900, 720),
    "gif": (1024, 820),
    ("schematic", "png"): (400, 320),
    ("strip", "png"): (400, 320),
}

#: name -> file of the module that registered it (duplicate names are rejected across both registries)
NAMES: dict[str, str] = {}


@dataclass
class AssetSpec:
    """One registered asset that a script under ``docs/figures/`` writes (or a pinned capture)."""

    name: str
    kind: str
    alt: str
    caption: str
    pages: tuple[str, ...] = ()
    generator: Callable[..., Any] | None = None
    group: str = ""
    root: Path = IMAGES
    subdir: str = ""
    fmt: str = "svg"
    #: None, "svg" (two-layer CSS animation) or "apng" (frame list).
    animated: str | None = None
    #: Ships ``-still-`` variants (set automatically for animated specs).
    still: bool = False
    #: Ships ``-narrow-`` twins (strips wider than 3:1).
    narrow: bool = False
    hero_on: tuple[str, ...] = ()
    requires: tuple[str, ...] = ()
    platform_independent: bool = False
    #: ``capture``: one file, no light/dark pair.
    single: bool = False
    #: sha256 of the file(s) for captures: a hex string (single) or {filename: hex}.
    pinned_sha256: str | dict[str, str] | None = None
    enabled: bool = True
    disabled_reason: str = ""
    extra: dict[str, Any] = field(default_factory=dict)
    module: str = ""

    def __post_init__(self) -> None:
        if self.kind not in KINDS:
            raise ValueError(f"{self.name}: kind {self.kind!r} not in {KINDS}")
        if self.animated:
            self.still = True
        if self.kind == "capture":
            self.single = True
        if self.fmt == "apng":
            self.animated = "apng"
            self.still = True
        if not self.alt.strip():
            raise ValueError(f"{self.name}: alt must not be empty (length ≤ 300 is a --check rule)")

    @property
    def rel_stem(self) -> str:
        return f"{self.subdir}/{self.name}" if self.subdir else self.name


# ---------------------------------------------------------------------------
# File naming shared by both spec types (duck-typed: FigureSpec has the same fields)
# ---------------------------------------------------------------------------


def _ext(spec) -> str:
    return "png" if spec.fmt == "apng" else spec.fmt


def is_animated(spec) -> bool:
    return bool(getattr(spec, "animated", None)) or spec.fmt == "apng"


def anim_stem(spec) -> str:
    """Stem (relative to root) of the animated file, without the theme suffix."""
    base = spec.rel_stem
    return base if base.endswith("-anim") else base + "-anim"


def still_stem(spec) -> str:
    return anim_stem(spec)[: -len("-anim")] + "-still"


def root_of(spec) -> Path:
    return Path(getattr(spec, "root", IMAGES))


def main_file(spec, theme: str = "light", out_dir: Path | None = None) -> Path:
    """The file a page embeds for ``theme`` (the animated file for animations)."""
    root = out_dir or root_of(spec)
    if getattr(spec, "single", False):
        return root / f"{spec.rel_stem}.{_ext(spec)}"
    stem = anim_stem(spec) if is_animated(spec) else spec.rel_stem
    return root / f"{stem}-{theme}.{_ext(spec)}"


def still_file(spec, theme: str = "light", out_dir: Path | None = None) -> Path | None:
    if not is_animated(spec):
        return None
    root = out_dir or root_of(spec)
    return root / f"{still_stem(spec)}-{theme}.{_ext(spec)}"


def narrow_file(spec, theme: str = "light", out_dir: Path | None = None) -> Path | None:
    if not getattr(spec, "narrow", False):
        return None
    root = out_dir or root_of(spec)
    stem = spec.rel_stem[: -len("-anim")] if spec.rel_stem.endswith("-anim") else spec.rel_stem
    return root / f"{stem}-narrow-{theme}.svg"


def gif_file(spec, theme: str = "light", out_dir: Path | None = None) -> Path | None:
    if spec.fmt != "apng" or not spec.extra.get("gif"):
        return None
    root = out_dir or root_of(spec)
    return root / f"{anim_stem(spec)}-{theme}.gif"


def readme_file(spec, theme: str = "light", out_dir: Path | None = None) -> Path:
    """The file the README embeds: the GIF for raster animations, ``extra['readme_fmt']`` when set."""
    g = gif_file(spec, theme, out_dir)
    if g is not None:
        return g
    rf = spec.extra.get("readme_fmt")
    if rf and rf != spec.fmt:
        root = out_dir or root_of(spec)
        return root / f"{spec.rel_stem}-{theme}.{rf}"
    return main_file(spec, theme, out_dir)


def all_files(spec, out_dir: Path | None = None) -> dict[str, Path]:
    """Every file the spec must ship: {"main-light": …, "still-dark": …, "narrow-light": …, "gif-light": …}."""
    out: dict[str, Path] = {}
    if getattr(spec, "single", False):
        out["main"] = main_file(spec, "", out_dir)
        return out
    for t in THEMES:
        out[f"main-{t}"] = main_file(spec, t, out_dir)
        s = still_file(spec, t, out_dir)
        if s is not None:
            out[f"still-{t}"] = s
        n = narrow_file(spec, t, out_dir)
        if n is not None:
            out[f"narrow-{t}"] = n
        g = gif_file(spec, t, out_dir)
        if g is not None:
            out[f"gif-{t}"] = g
        rf = spec.extra.get("readme_fmt")
        if rf and rf != spec.fmt and "README.md" in spec.pages:
            out[f"readme-{t}"] = readme_file(spec, t, out_dir)
    return out


def cap_for(spec, variant: str) -> tuple[int, int]:
    """(error KB, warn KB) for one variant key of :func:`all_files`."""
    if variant.startswith("narrow"):
        return SIZE_CAPS_KB["narrow"]
    if variant.startswith("gif"):
        return SIZE_CAPS_KB["gif"]
    ext = _ext(spec)
    if variant.startswith("readme"):
        ext = spec.extra.get("readme_fmt", ext)
        return SIZE_CAPS_KB.get((spec.kind if spec.kind != "anim" else "figure", ext), SIZE_CAPS_KB[("figure", "png")])
    if spec.fmt == "apng":
        if variant.startswith("still"):
            return SIZE_CAPS_KB[("figure", "png")]
        return SIZE_CAPS_KB["apng"]
    kind = spec.kind
    if variant.startswith("still") and kind == "anim":
        kind = "schematic" if getattr(spec, "platform_independent", False) else "figure"
    cap = SIZE_CAPS_KB.get((kind, ext))
    if cap is None:
        cap = SIZE_CAPS_KB[("figure", ext)] if ("figure", ext) in SIZE_CAPS_KB else SIZE_CAPS_KB[("figure", "png")]
    mk = spec.extra.get("max_kb")
    if mk:
        return (int(mk), int(mk * 0.8))
    return cap


def rel_to_repo(path: Path) -> str:
    try:
        return path.resolve().relative_to(REPO.resolve()).as_posix()
    except ValueError:
        return path.as_posix()


# ---------------------------------------------------------------------------
# The registry
# ---------------------------------------------------------------------------

REGISTRY: list[AssetSpec] = []


def _claim(name: str, module: str) -> None:
    if name in NAMES:
        raise ValueError(f"duplicate asset name {name!r} (already registered by {NAMES[name]}; now {module})")
    NAMES[name] = module


def derive_crops(spec, make: Callable[..., Any]) -> list:
    """The derived ``<parent>-<crop>`` specs declared in ``spec.extra['crops']``.

    ``make(**fields)`` builds a spec of the parent's class. Pages listed on the parent
    may not appear on a crop (a crop page is listed only on the derived spec).
    """
    crops = spec.extra.get("crops") or {}
    out = []
    for crop, cfg in crops.items():
        pages = tuple(cfg.get("pages", ()))
        clash = set(pages) & set(spec.pages)
        if clash:
            raise ValueError(f"{spec.name}: crop {crop!r} lists page(s) {sorted(clash)} that the parent also lists")
        fields = dict(
            name=f"{spec.name}-{crop}",
            kind=cfg.get("kind", spec.kind),
            alt=cfg.get("alt", spec.alt),
            caption=cfg.get("caption", spec.caption),
            pages=pages,
            fmt=spec.fmt,
            subdir=spec.subdir,
            requires=tuple(getattr(spec, "requires", ())),
            enabled=getattr(spec, "enabled", True),
            extra={"crop_of": spec.name, "box": cfg.get("box"), "model_rows": cfg.get("model_rows", {})},
        )
        out.append(make(**fields))
    return out


def register(spec: AssetSpec, *, module: str | None = None) -> AssetSpec:
    """Register an :class:`AssetSpec` (and its crops); duplicate names are rejected across both registries."""
    import inspect  # noqa: PLC0415

    if module is None:
        frame = inspect.stack()[1]
        module = Path(frame.filename).name
    spec.module = module
    _claim(spec.name, module)
    REGISTRY.append(spec)
    for crop in derive_crops(spec, lambda **f: AssetSpec(generator=spec.generator, root=spec.root, group=spec.group,
                                                           platform_independent=spec.platform_independent, **f)):
        crop.module = module
        _claim(crop.name, module)
        REGISTRY.append(crop)
    return spec


def all_specs() -> list:
    """Every registered spec: the sim FigureSpecs (when ``common`` is imported) followed by the AssetSpecs."""
    specs: list = []
    sim = sys.modules.get("common")
    if sim is not None and hasattr(sim, "REGISTRY"):
        specs.extend(sim.REGISTRY)
    specs.extend(REGISTRY)
    return specs


# ---------------------------------------------------------------------------
# Embeds (the only writer; DESIGN §4.6 forms 1–5)
# ---------------------------------------------------------------------------


def _url(spec, page: str, path: Path) -> str:
    """URL of ``path`` as seen from ``page`` (``docs/<page>`` or README.md), plus any cache-bust."""
    rel = rel_to_repo(path)  # docs/assets/images/...
    if page == "README.md":
        url = rel
    else:
        depth = len(Path(page).parent.parts)
        url = "../" * depth + rel[len("docs/"):]
    v = spec.extra.get("cache_bust")
    return f"{url}?v={v}" if v else url


def _width(spec, page: str) -> str:
    w = spec.extra.get("width")
    if isinstance(w, dict):
        w = w.get(page)
    if w is None and page == "README.md" and is_animated(spec):
        w = 800
    return f' width="{w}"' if w else ""


def md_embed(spec, page: str) -> str:
    """Markdown/HTML that embeds ``spec`` into ``page`` (a path relative to docs/, or README.md).

    Docs pages get a ``<figure markdown="span">`` whose blank lines keep it valid on
    both renderers (MkDocs md_in_html and github.com); the README gets a ``<picture>``
    with ``prefers-color-scheme`` sources.
    """
    alt = spec.alt.replace('"', "&quot;") if page == "README.md" else spec.alt
    if page == "README.md":
        img = _url(spec, page, readme_file(spec, "light"))
        dark = _url(spec, page, readme_file(spec, "dark"))
        lines = ["<picture>"]
        if is_animated(spec):
            lines.append(f'  <source media="(prefers-reduced-motion: reduce) and (prefers-color-scheme: dark)" srcset="{_url(spec, page, still_file(spec, "dark"))}">')
            lines.append(f'  <source media="(prefers-reduced-motion: reduce)" srcset="{_url(spec, page, still_file(spec, "light"))}">')
        lines.append(f'  <source media="(prefers-color-scheme: dark)" srcset="{dark}">')
        lines.append(f'  <img alt="{alt}"{_width(spec, page)} src="{img}">')
        lines.append("</picture>")
        return "\n".join(lines)

    if getattr(spec, "single", False):
        body = f"![{spec.alt}]({_url(spec, page, main_file(spec, ''))})"
    elif spec.fmt == "apng":
        parts = []
        for t in THEMES:
            still = _url(spec, page, still_file(spec, t))
            anim = _url(spec, page, main_file(spec, t))
            parts.append(
                f'<picture><source media="(prefers-reduced-motion: reduce)" srcset="{still}">'
                f'<img class="huv-no-zoom huv-anim" src="{anim}#gh-{t}-mode-only" alt="{alt}" data-huv-still="{still}"></picture>'
            )
        body = "\n".join(parts)
    elif getattr(spec, "narrow", False):
        parts = []
        for t in THEMES:
            parts.append(
                f'<picture><source media="(max-width: 600px)" srcset="{_url(spec, page, narrow_file(spec, t))}">'
                f'<img class="huv-no-zoom" src="{_url(spec, page, main_file(spec, t))}#gh-{t}-mode-only" alt="{alt}"></picture>'
            )
        body = "\n".join(parts)
    else:
        body = "\n".join(f"![{spec.alt}]({_url(spec, page, main_file(spec, t))}#gh-{t}-mode-only)" for t in THEMES)
    return (
        '<figure markdown="span">\n\n'
        f"{body}\n\n"
        f"<figcaption>{spec.caption}</figcaption>\n"
        "</figure>"
    )


def page_path(page: str) -> Path:
    return REPO / page if page == "README.md" else DOCS / page


def embed_marker(spec, page: str) -> str:
    """The substring that identifies an embed of ``spec`` on ``page``: ``/<stem>-light.<ext>``."""
    f = readme_file(spec, "light") if page == "README.md" else main_file(spec, "light" if not getattr(spec, "single", False) else "")
    return "/" + f.name


def find_embeds(text: str, spec, page: str) -> list[tuple[int, int]]:
    """(start, end) spans of every existing embed block of ``spec`` on ``page``."""
    open_tag, close_tag = ("<picture>", "</picture>") if page == "README.md" else ('<figure markdown="span">', "</figure>")
    marker = embed_marker(spec, page)
    spans: list[tuple[int, int]] = []
    pos = 0
    while True:
        pos = text.find(marker, pos)
        if pos < 0:
            break
        start = text.rfind(open_tag, 0, pos)
        end = text.find(close_tag, pos)
        if start < 0 or end < 0:
            pos += len(marker)
            continue
        end += len(close_tag)
        if spans and start <= spans[-1][1]:
            pos = end
            continue
        spans.append((start, end))
        pos = end
    return spans


def _page_selected(page: str, pages: list[str] | None) -> bool:
    """``pages`` entries are repo-relative paths or prefixes (``docs/history``, ``docs/pipeline.md``, ``README.md``)."""
    if not pages:
        return True
    full = "README.md" if page == "README.md" else f"docs/{page}"
    for p in pages:
        p = p.rstrip("/")
        if p.startswith("./"):
            p = p[2:]
        if full == p or full.startswith(p + "/"):
            return True
    return False


_FIGCAP = re.compile(r"<figcaption>(.*?)</figcaption>", re.S)


def check_embeds(specs, pages: list[str] | None = None, strict: bool = False) -> tuple[list[str], list[str]]:
    """Page-side checks: every listed page exists, embeds the asset and carries its caption.

    Returns ``(errors, warnings)``. Page-side findings are warnings unless ``strict``
    (DESIGN §9.0 rule 4).
    """
    errors: list[str] = []
    warnings: list[str] = []
    out = errors if strict else warnings
    for spec in specs:
        for page in spec.pages:
            if not _page_selected(page, pages):
                continue
            path = page_path(page)
            if not path.exists():
                out.append(f"{spec.name}: page {page} does not exist (yet)")
                continue
            text = path.read_text(encoding="utf-8")
            marker = embed_marker(spec, page)
            if marker not in text:
                out.append(f"{spec.name}: {page} does not embed {marker.lstrip('/')}")
                continue
            if page != "README.md" and not getattr(spec, "single", False):
                dark = "/" + main_file(spec, "dark").name
                if dark not in text:
                    out.append(f"{spec.name}: {page} embeds the light variant but not {dark.lstrip('/')}")
            for start, end in find_embeds(text, spec, page):
                block = text[start:end]
                if page == "README.md":
                    want = md_embed(spec, page)
                    if block != want:
                        out.append(f"{spec.name}: README.md embed differs from the registry (run --sync-embeds --pages README.md)")
                else:
                    m = _FIGCAP.search(block)
                    if not m or m.group(1).strip() != spec.caption.strip():
                        out.append(f"{spec.name}: caption on {page} differs from the registry (run --sync-embeds --pages {page})")
                    elif block != md_embed(spec, page):
                        out.append(f"{spec.name}: embed block on {page} differs from the registry form (run --sync-embeds --pages {page})")
    return errors, warnings


def sync_embeds(specs, pages: list[str] | None, *, site_wide: bool = False) -> int:
    """Rewrite every existing embed of ``specs`` on the selected pages from :func:`md_embed`.

    Refused without ``pages`` unless ``site_wide`` (INT's flag, DESIGN §9.1). Only
    blocks that already exist are rewritten; new embeds are pasted from ``--catalog``.
    """
    if not pages and not site_wide:
        raise SystemExit("site-wide sync is INT's step (DESIGN §9.1); pass --pages <your pages> or --site-wide")
    changed = 0
    for spec in specs:
        for page in spec.pages:
            if not _page_selected(page, pages):
                continue
            path = page_path(page)
            if not path.exists():
                continue
            text = path.read_text(encoding="utf-8")
            new = md_embed(spec, page)
            spans = find_embeds(text, spec, page)
            if not spans:
                continue
            edited = text
            for start, end in reversed(spans):
                if edited[start:end] != new:
                    edited = edited[:start] + new + edited[end:]
            if edited != text:
                path.write_text(edited, encoding="utf-8")
                print(f"updated {spec.name} in {page}")
                changed += 1
    print(f"{changed} embed(s) updated")
    return changed


def catalog(specs, pages: list[str] | None = None) -> str:
    """The embed text of every spec for every selected page (what page groups paste)."""
    out: list[str] = []
    for spec in specs:
        for page in spec.pages or ("docs/<page>.md",):
            if page != "docs/<page>.md" and not _page_selected(page, pages):
                continue
            out.append(f"<!-- {spec.name} -> {page} -->")
            out.append(md_embed(spec, page if page != "docs/<page>.md" else "page.md"))
            out.append("")
    return "\n".join(out)


# ---------------------------------------------------------------------------
# Image metadata (stdlib only) and the diagnostic export
# ---------------------------------------------------------------------------

_VIEWBOX = re.compile(r'viewBox="\s*[-\d.]+\s+[-\d.]+\s+([\d.]+)\s+([\d.]+)\s*"')
_WH = re.compile(r'<svg[^>]*\swidth="([\d.]+)(?:px)?"[^>]*\sheight="([\d.]+)(?:px)?"', re.S)


def image_size(path: Path) -> tuple[int, int] | None:
    """(width, height) in pixels from a PNG IHDR or an SVG viewBox / width+height; None if unreadable."""
    try:
        data = path.read_bytes()
    except OSError:
        return None
    if data[:8] == b"\x89PNG\r\n\x1a\n" and len(data) >= 24:
        w, h = struct.unpack(">II", data[16:24])
        return int(w), int(h)
    if data[:6] == b"GIF89a" or data[:6] == b"GIF87a":
        w, h = struct.unpack("<HH", data[6:10])
        return int(w), int(h)
    head = data[:4096].decode("utf-8", "replace")
    m = _VIEWBOX.search(head) or _WH.search(head)
    if m:
        return int(float(m.group(1))), int(float(m.group(2)))
    return None


def sha256_of(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def export_json(specs, path: Path | None = None) -> str:
    """The merged registry as JSON (a diagnostic: nothing reads it, nothing commits it)."""
    items = []
    for spec in specs:
        files = {}
        for key, p in all_files(spec).items():
            size = image_size(p) if p.exists() else None
            files[key] = {"path": rel_to_repo(p), "bytes": p.stat().st_size if p.exists() else None,
                          "px": list(size) if size else None}
        items.append({
            "name": spec.name, "kind": spec.kind, "group": getattr(spec, "group", ""),
            "fmt": spec.fmt, "animated": getattr(spec, "animated", None), "narrow": getattr(spec, "narrow", False),
            "platform_independent": getattr(spec, "platform_independent", False),
            "pages": list(spec.pages), "alt": spec.alt, "caption": spec.caption,
            "module": getattr(spec, "module", "") or getattr(getattr(spec, "compute", None), "__module__", ""),
            "files": files,
        })
    text = json.dumps(items, ensure_ascii=False, indent=1)
    if path is not None:
        Path(path).write_text(text + "\n", encoding="utf-8")
    return text
