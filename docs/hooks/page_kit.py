"""MkDocs hook: the page kit — the site-only attributes the design system needs.

Pages are written in plain Markdown that reads the same on github.com; this hook adds, after
rendering (it runs after ``status_badges.py`` and after the glightbox plugin), what only the
site can use. Nothing here changes the markup the honesty hooks or the verify-example runner
match (DESIGN §4.5 "rules for the kit").

* ``<!-- huv-table: stack|key|compact|grid -->`` on the line before a table (the comment is
  invisible on GitHub) gives the ``<table>`` the class ``huv-<kind>`` and wraps it in Material's
  own scroll wrapper (``div.md-typeset__scrollwrap > div.md-typeset__table``, which Material's
  runtime adds only to ``table:not([class])``), so the overflow cue and the matrix filter work
  unchanged. ``stack``, ``key`` and ``grid`` tables get ``data-label="<th text>"`` on every
  ``td`` (the phone card layout prints it; grid tables stack on phones too); ``grid`` tables also
  get ``tr[data-huv-grade]`` + ``.huv-row--<grade>`` from the first grade glyph of the Status cell
  and, on ``capability-matrix.md`` only, the row id ``cm-<slug>`` used by the filter's ``?grade=``
  deep link.
* **Physics-Strip eyebrows** (DESIGN §3.4 / §4.5): every ``<figure>`` directly inside
  ``<div class="huv-physics">`` gets ``<span class="huv-eyebrow">Mechanism</span>`` or
  ``<span class="huv-eyebrow">In the engine</span>`` at the start of its ``<figcaption>``. The
  word comes, in order, from the caption's own lead (``<strong>Mechanism.</strong>`` /
  ``<strong>In the engine.</strong>``, which is *replaced* by the span so the word is never shown
  twice; ``<strong>In the engine: rest.</strong>`` becomes the span + ``<strong>Rest.</strong>``),
  then from the caption's trailer (``<em>Model: …</em>`` is engine output; a trailer of the
  mechanism vocabulary of DESIGN §4.6 — ``Schematic, not to scale``, ``Closed form; not the
  engine``, ``… no model badge``, ``Data chart from …``, ``… nothing is mocked``, ``Drawn from
  the matrix itself`` — is ``Mechanism``), then from position (the first of two or more figures
  is ``Mechanism``, every other figure ``In the engine``). The eyebrow is the strip's only visible
  label of what is engine output, so it must never contradict the trailer under it. Any other lead
  sentence keeps its bold and sentence case and the span is inserted before it. GitHub shows the
  Markdown as written.
* **raw image paths are resolved the way MkDocs resolves Markdown images.** MkDocs rewrites only
  the ``src`` of Markdown images; a raw ``<img>``/``<picture>``/``<source>`` (the registry's narrow
  strips and APNG/still swaps) keeps its source-file-relative path verbatim, which the browser then
  resolves against the page's *directory URL* (``history/euv/`` is one level deeper than
  ``history/euv.md``) and 404s. So every relative ``src``, ``srcset`` candidate, ``data-huv-still``
  and glightbox ``href`` is resolved against the page's source directory (then, for paths MkDocs
  already rewrote, against the page URL; then against the docs root), and if the file exists under
  ``docs/`` it is re-emitted relative to the page URL, query and ``#fragment`` kept. Absolute URLs
  and paths that resolve nowhere are left untouched (MkDocs warns about those itself).
* every ``<img>`` after the first figure image gets ``loading="lazy" decoding="async"``; every
  ``<img>`` whose ``src`` points under ``docs/assets/images/`` gets ``width``/``height`` read
  from the asset file itself (PNG IHDR, GIF header, SVG root ``width``/``height`` in CSS px —
  ``pt``/``in``/``cm``/``mm``/``pc`` converted the way the browser sizes the image, so a
  matplotlib ``width="529pt"`` chart is 705 px wide and fills the column — else the SVG
  ``viewBox``; stdlib only — there is no registry file, DESIGN §8.3 M2); an unreadable file
  gets no size. **The attributes are CSS px, not device px**: ``docs/figures/sim/style.py``
  renders every static PNG under ``assets/images/sim/`` (sheets, crops, ``-still-`` and
  ``-rest-`` frames) at ``EXPORT_SCALE`` (2x) times its designed size, and the browser sizes an
  ``<img>`` by its pixel count — a full sheet is capped by ``max-width: 100%`` anyway, but a crop
  narrower than the column (the 338 px High-NA pupil, the 389 px CLI panel) would render at
  twice its designed size — so a sim PNG's IHDR size is divided by the scale read from
  ``style.py``'s ``EXPORT_SCALE`` line (a regex over the source, no import; 2 when the file is
  unreadable, DESIGN §5.0 "PNGs at 2x"). Raster animation frames (``-anim-`` PNGs, 1x and
  800 px wide, DESIGN §5.0), GIFs, SVGs and every PNG outside ``sim/`` keep their own size.
* a glightbox ``<a>`` without ``data-gallery`` (a single-variant image: no ``#gh-*`` fragment,
  so ``auto_themed`` names no gallery) gets ``data-gallery="any"``: GLightbox pages a
  gallery-less anchor through *every* anchor on the page, hidden off-scheme variants included,
  while ``any`` holds only the scheme-neutral images, visible in both schemes (DESIGN §4.6).
* an ``<img>`` whose file name contains ``-anim-`` gets the class ``huv-anim`` and
  ``data-huv-still`` = the ``-still-`` twin **without** the ``#gh-*`` fragment (``layout.js``
  re-appends it when it swaps, DESIGN §4.12).
* the bold-led paragraphs ``**Where next:**`` / ``**Read next:**`` / ``**Part of … · see also:**``
  become ``p.huv-next``, ``**Same thing elsewhere:**`` becomes ``p.huv-rosetta-line`` and
  ``**Five limits:**`` becomes ``p.huv-strip``.
* a readiness rung word in bold inside a table cell (``<td><strong>In production</strong> …``)
  gets ``class="huv-rung"`` (an outlined slate pill — never a grade colour, Appendix B).
* links into the capability matrix get ``data-preview`` (Material's instant previews); the
  matrix page's own links are excluded.

Code, pre, script, style, svg, textarea, kbd and math elements are never rewritten: every
rule here matches tags (``<table>``, ``<img>``, ``<p>``, ``<td>``, ``<a>``), and the text of
those elements is escaped by the renderer, so it cannot contain them.
"""

from __future__ import annotations

import html as html_lib
import posixpath
import re
import struct
from pathlib import Path
from typing import TYPE_CHECKING
from urllib.parse import unquote

if TYPE_CHECKING:  # pragma: no cover - typing only
    from mkdocs.config.defaults import MkDocsConfig
    from mkdocs.structure.files import Files
    from mkdocs.structure.pages import Page

TABLE_KINDS = frozenset({"stack", "key", "compact", "grid"})
GRADES = {"✅": "implemented", "🔶": "simplified", "🧪": "theoretical", "🗺": "planned"}
RUNGS = ("In production", "Shipping", "Announced", "Demonstrated", "Roadmap", "Proposed", "Speculative")
MATRIX_PAGE = "capability-matrix.md"
ASSETS = "assets/images/"
SIM_ASSETS = "sim/"  # under ASSETS: what docs/figures/sim/style.py writes
STYLE_PY = Path("figures") / "sim" / "style.py"  # under docs_dir: declares EXPORT_SCALE
DEFAULT_EXPORT_SCALE = 2  # DESIGN §5.0: PNGs at 2x
EYEBROWS = ("Mechanism", "In the engine")

_TABLE_HINT = re.compile(
    r"(?:<p>\s*)?<!--\s*huv-table:\s*(?P<kind>[a-z-]+)\s*-->(?:\s*</p>)?\s*"
    r"(?P<table><table\b(?P<attrs>[^>]*)>.*?</table>)",
    re.S,
)
_THEAD = re.compile(r"<thead\b.*?</thead>", re.S)
_TH = re.compile(r"<th\b[^>]*>(?P<text>.*?)</th>", re.S)
_TBODY = re.compile(r"<tbody\b.*?</tbody>", re.S)
_TR = re.compile(r"<tr\b(?P<attrs>[^>]*)>(?P<body>.*?)</tr>", re.S)
_TD_OPEN = re.compile(r"<td\b")
_TD = re.compile(r"<td\b[^>]*>(?P<text>.*?)</td>", re.S)
_TAGS = re.compile(r"<[^>]*>")
_GLYPH = re.compile("[" + "".join(GRADES) + "]")
_CLASS_ATTR = re.compile(r"""\bclass\s*=\s*(?P<q>["'])(?P<value>.*?)(?P=q)""", re.S)

_FIG_OR_IMG = re.compile(r"<figure\b|</figure\s*>|<img\b[^>]*>|<source\b[^>]*>|<a\b[^>]*\bglightbox\b[^>]*>", re.S)
_SCHEME = re.compile(r"^[A-Za-z][A-Za-z0-9+.-]*:")
_ATTR = re.compile(r"""\b(?P<name>[A-Za-z_:][-A-Za-z0-9_:.]*)\s*=\s*(?P<q>["'])(?P<value>.*?)(?P=q)""", re.S)
_EMOJI_CLASS = re.compile(r"\b(?:twemoji|emojione|gemoji)\b")

_P_NEXT = re.compile(r"<p>(?=<strong>(?:Where next|Read next|Part of [^<]*?see also):</strong>)")
_P_ROSETTA = re.compile(r"<p>(?=<strong>Same thing elsewhere:</strong>)")
_P_STRIP = re.compile(r"<p>(?=<strong>Five limits:</strong>)")
_CELL = re.compile(r"<td\b[^>]*>.*?</td>", re.S)
_RUNG = re.compile(r"<strong>(?P<word>" + "|".join(re.escape(r) for r in RUNGS) + r")</strong>")
_MATRIX_LINK = re.compile(r"""<a\b(?P<attrs>[^>]*\bhref\s*=\s*["'][^"']*capability-matrix[^"']*["'][^>]*)>""")

_STRIP_OPEN = re.compile(r"""<div\b[^>]*\bclass\s*=\s*(?P<q>["'])(?P<cls>[^"']*\bhuv-physics\b[^"']*)(?P=q)[^>]*>""", re.S)
_DIV_OR_FIGURE = re.compile(r"<div\b|</div\s*>|<figure\b[^>]*>|</figure\s*>", re.S)
_FIGCAPTION = re.compile(r"(?P<open><figcaption\b[^>]*>)(?P<body>.*?)</figcaption>", re.S)
_CAPTION_LEAD = re.compile(r"^\s*<strong\b[^>]*>(?P<text>.*?)</strong>", re.S)
_EYEBROW_LEAD = re.compile(r"^\s*(?P<word>" + "|".join(re.escape(w) for w in EYEBROWS) + r")\s*[.:]\s*(?P<rest>.*?)\s*$", re.S | re.I)

_SVG_TAG = re.compile(r"<svg\b[^>]*>", re.S)
_VIEWBOX = re.compile(r"""\bviewBox\s*=\s*["']\s*([-\d.eE+]+)[\s,]+([-\d.eE+]+)[\s,]+([-\d.eE+]+)[\s,]+([-\d.eE+]+)\s*["']""")
_DIM = re.compile(r"""\b(?P<name>width|height)\s*=\s*["']\s*(?P<value>\d*\.?\d+(?:[eE][-+]?\d+)?)\s*(?P<unit>[a-z%]*)\s*["']""", re.I)
# CSS absolute lengths in px (CSS Values §6.2): what the browser uses as an <img>'s intrinsic size
_UNIT_PX = {"": 1.0, "px": 1.0, "pt": 96 / 72, "pc": 16.0, "in": 96.0, "cm": 96 / 2.54, "mm": 96 / 25.4}
_EXPORT_SCALE = re.compile(r"^EXPORT_SCALE\s*=\s*(?P<value>[1-9]\d*)\s*(?:#.*)?$", re.M)
_TRAILER = re.compile(r"<em\b[^>]*>(?P<text>(?:(?!</em>).)*)</em>\s*$", re.S)  # the last <em>: figcaption > em:last-child
_ENGINE_TRAILER = re.compile(r"^\s*Model\s*:", re.I)
_MECHANISM_TRAILER = re.compile(r"not the engine|schematic|no model badge|data chart|nothing is mocked|drawn from the matrix", re.I)


# --- image sizes (stdlib only) -----------------------------------------------------------------


def _png_size(head: bytes) -> tuple[int, int] | None:
    if len(head) >= 24 and head[:8] == b"\x89PNG\r\n\x1a\n" and head[12:16] == b"IHDR":
        width, height = struct.unpack(">II", head[16:24])
        return (width, height) if width and height else None
    return None


def _gif_size(head: bytes) -> tuple[int, int] | None:
    if len(head) >= 10 and head[:4] == b"GIF8":
        width, height = struct.unpack("<HH", head[6:10])
        return (width, height) if width and height else None
    return None


def _svg_size(text: str) -> tuple[int, int] | None:
    """The intrinsic CSS-px size of an SVG shown as an ``<img>``: its root ``width``/``height``
    when both are absolute lengths (unitless = px; ``pt`` etc. converted), else its ``viewBox``."""
    tag = _SVG_TAG.search(text)
    if not tag:
        return None
    attrs = tag.group(0)
    dims: dict[str, float] = {}
    for m in _DIM.finditer(attrs):
        factor = _UNIT_PX.get(m.group("unit").lower())
        if factor is not None:  # em, rem, % and friends have no size without a container
            dims[m.group("name").lower()] = float(m.group("value")) * factor
    if dims.get("width", 0) > 0 and dims.get("height", 0) > 0:
        return (round(dims["width"]), round(dims["height"]))
    box = _VIEWBOX.search(attrs)
    if box:
        try:
            width, height = float(box.group(3)), float(box.group(4))
        except ValueError:
            width = height = 0.0
        if width > 0 and height > 0:
            return (round(width), round(height))
    return None


def image_size(path: Path) -> tuple[int, int] | None:
    """Pixel size of a PNG/APNG, GIF or SVG file (SVG: CSS px, see :func:`_svg_size`); ``None`` when unreadable."""
    try:
        with open(path, "rb") as fh:
            head = fh.read(4096)
    except OSError:
        return None
    size = _png_size(head) or _gif_size(head)
    if size:
        return size
    if b"<svg" in head[:4096]:
        return _svg_size(head.decode("utf-8", "replace"))
    return None


_scale_cache: dict[tuple[Path, int], int] = {}


def export_scale(docs_dir: Path | None) -> int:
    """``EXPORT_SCALE`` of ``docs/figures/sim/style.py`` (the device px per CSS px of every static sim
    PNG), read from the source text, not imported (the docs venv has no matplotlib); the file is
    cached by mtime. :data:`DEFAULT_EXPORT_SCALE` when it is absent, unreadable or silent."""
    if docs_dir is None:
        return DEFAULT_EXPORT_SCALE
    path = docs_dir / STYLE_PY
    try:
        key = (path, path.stat().st_mtime_ns)
    except OSError:
        return DEFAULT_EXPORT_SCALE
    cached = _scale_cache.get(key)
    if cached is not None:
        return cached
    try:
        m = _EXPORT_SCALE.search(path.read_text(encoding="utf-8", errors="replace"))
    except OSError:
        return DEFAULT_EXPORT_SCALE
    scale = int(m.group("value")) if m else DEFAULT_EXPORT_SCALE
    _scale_cache[key] = scale
    return scale


def render_scale(path: Path, docs_dir: Path | None) -> int:
    """Device px per CSS px of one asset file: :func:`export_scale` for a static PNG under
    ``assets/images/sim/`` (sheets, crops and the ``-still-`` / ``-rest-`` frames ``style.py`` renders at
    ``EXPORT_SCALE``), 1 for everything else (``-anim-`` frames render at 1x, DESIGN §5.0; GIFs, SVGs
    and PNGs of other generators are what they are)."""
    if docs_dir is None or path.suffix.lower() != ".png" or "-anim-" in path.name:
        return 1
    try:
        rel = path.relative_to(docs_dir / "assets" / "images").as_posix()
    except ValueError:
        return 1
    return export_scale(docs_dir) if rel.startswith(SIM_ASSETS) else 1


def css_size(path: Path, docs_dir: Path | None) -> tuple[int, int] | None:
    """The ``width``/``height`` an ``<img>`` of ``path`` declares: :func:`image_size` divided by
    :func:`render_scale` (rounded half up), so a 2x crop is laid out at its designed 1x size."""
    size = image_size(path)
    if not size:
        return None
    scale = render_scale(path, docs_dir)
    if scale <= 1:
        return size
    width, height = ((v + scale // 2) // scale for v in size)
    return (max(width, 1), max(height, 1))


def asset_path(src: str, docs_dir: Path | None) -> Path | None:
    """The file under ``docs/assets/images/`` an ``<img src>`` points at, or ``None``."""
    if docs_dir is None:
        return None
    clean = src.split("#", 1)[0].split("?", 1)[0]
    pos = clean.find(ASSETS)
    if pos < 0:
        return None
    rest = unquote(clean[pos + len(ASSETS) :])
    if not rest or rest.startswith("/") or ".." in rest.split("/"):
        return None
    return docs_dir / "assets" / "images" / rest


def still_url(src: str) -> str | None:
    """``…/x-anim-light.svg#gh-light-mode-only`` -> ``…/x-still-light.svg`` (no fragment)."""
    clean = src.split("#", 1)[0].split("?", 1)[0]
    head, _, name = clean.rpartition("/")
    if "-anim-" not in name:
        return None
    name = name[::-1].replace("-mina-", "-llits-", 1)[::-1]  # the last "-anim-" only
    return f"{head}/{name}" if head else name


# --- paths: what MkDocs does for Markdown images, applied to raw HTML --------------------------


def relative_url(target: str, page_url: str) -> str:
    """``target`` (docs-root relative) as a URL relative to ``page_url``'s directory, like
    ``mkdocs.utils.get_relative_url`` (a last component with a dot is a file and is dropped)."""
    head, _, tail = (page_url or "").rpartition("/")
    base = head if "." in tail else (page_url or "")
    base_parts = [p for p in posixpath.normpath("/" + base)[1:].split("/") if p]
    dest_parts = [p for p in posixpath.normpath("/" + target)[1:].split("/") if p]
    common = 0
    for a, b in zip(base_parts, dest_parts):
        if a != b:
            break
        common += 1
    rel = [".."] * (len(base_parts) - common) + dest_parts[common:]
    return "/".join(rel) or "."


def resolve_path(path: str, src_uri: str, page_url: str, docs_dir: Path | None) -> str | None:
    """The page-URL-relative form of a relative image ``path``, or ``None`` to leave it alone.

    Candidates, in order: relative to the page's source directory (what MkDocs does for a
    Markdown image), relative to the page URL (a path MkDocs already rewrote — makes the pass
    idempotent), relative to the docs root. The first that names an existing file wins.
    """
    if docs_dir is None or not path:
        return None
    if path.startswith(("/", "#", "data:")) or _SCHEME.match(path):
        return None
    base, hash_sep, fragment = path.partition("#")
    base, q_sep, query = base.partition("?")
    if not base:
        return None
    src_dir = posixpath.dirname(src_uri or "")
    for cand in (
        posixpath.normpath(posixpath.join(src_dir, base)),
        posixpath.normpath(posixpath.join(page_url or "", base)),
        posixpath.normpath(base),
    ):
        if cand.startswith("../") or cand in ("..", ".", ""):
            continue
        if (docs_dir / cand).is_file():
            return relative_url(cand, page_url or "") + q_sep + query + hash_sep + fragment
    return None


def resolve_srcset(srcset: str, src_uri: str, page_url: str, docs_dir: Path | None) -> str | None:
    """Every URL of a ``srcset`` resolved through :func:`resolve_path`; ``None`` when unchanged."""
    changed = False
    out = []
    for cand in srcset.split(","):
        cand = cand.strip()
        if not cand:
            continue
        url, _, descriptor = cand.partition(" ")
        new = resolve_path(url, src_uri, page_url, docs_dir)
        if new is not None:
            changed = True
            url = new
        out.append(url + (" " + descriptor.strip() if descriptor.strip() else ""))
    return ", ".join(out) if changed else None


def _set_attr(tag: str, name: str, value: str) -> str:
    pattern = re.compile(r"""(\b%s\s*=\s*)(?P<q>["'])(?:.*?)(?P=q)""" % re.escape(name), re.S | re.I)
    quoted = html_lib.escape(value, quote=True)
    return pattern.sub(lambda m: f"{m.group(1)}{m.group('q')}{quoted}{m.group('q')}", tag, count=1)


def kit_paths(tag: str, attrs: dict[str, str], src_uri: str | None, page_url: str | None, docs_dir: Path | None) -> str:
    """Rewrite the path-carrying attributes of one ``<img>``/``<source>``/glightbox ``<a>`` tag."""
    if src_uri is None or page_url is None:
        return tag
    for name in ("src", "href", "data-huv-still"):
        if name in attrs:
            new = resolve_path(attrs[name], src_uri, page_url, docs_dir)
            if new is not None and new != attrs[name]:
                tag = _set_attr(tag, name, new)
                attrs[name] = new
    if "srcset" in attrs:
        new = resolve_srcset(attrs["srcset"], src_uri, page_url, docs_dir)
        if new is not None and new != attrs["srcset"]:
            tag = _set_attr(tag, "srcset", new)
            attrs["srcset"] = new
    return tag


# --- tables ------------------------------------------------------------------------------------


def _text(fragment: str) -> str:
    return " ".join(html_lib.unescape(_TAGS.sub("", fragment)).split())


def _slug(text: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")


def _with_class(attrs: str, cls: str) -> str:
    m = _CLASS_ATTR.search(attrs)
    if m:
        if cls in m.group("value").split():
            return attrs
        return attrs[: m.start("value")] + m.group("value") + " " + cls + attrs[m.end("value") :]
    return f' class="{cls}"' + attrs


def _label_cells(table: str) -> str:
    head = _THEAD.search(table)
    # a chip or badge span abuts its word ("<span …>✅</span>Implemented"): keep the glyph and the word apart
    labels = [_text(m.group("text").replace("</span>", "</span> ")) for m in _TH.finditer(head.group(0))] if head else []
    if not labels:
        return table

    def row(m: re.Match[str]) -> str:
        n = 0

        def cell(c: re.Match[str]) -> str:
            nonlocal n
            label = labels[n] if n < len(labels) else ""
            n += 1
            if not label or "data-label=" in m.group("body")[c.start() : c.start() + 200]:
                return c.group(0)
            return f'<td data-label="{html_lib.escape(label, quote=True)}"'

        return f"<tr{m.group('attrs')}>{_TD_OPEN.sub(cell, m.group('body'))}</tr>"

    def body(b: re.Match[str]) -> str:
        return _TR.sub(row, b.group(0))

    return _TBODY.sub(body, table)


def _grade_rows(table: str, ids: bool) -> str:
    seen: dict[str, int] = {}

    def row(m: re.Match[str]) -> str:
        cells = _TD.findall(m.group("body"))
        if len(cells) < 2 or "data-huv-grade" in m.group("attrs"):
            return m.group(0)
        glyph = _GLYPH.search(_TAGS.sub("", cells[1]))
        if not glyph:
            return m.group(0)
        grade = GRADES[glyph.group(0)]
        attrs = _with_class(m.group("attrs"), f"huv-row--{grade}") + f' data-huv-grade="{grade}"'
        if ids:
            slug = _slug(_text(cells[0])) or "row"
            seen[slug] = seen.get(slug, 0) + 1
            if seen[slug] > 1:
                slug = f"{slug}-{seen[slug]}"
            attrs += f' id="cm-{slug}"'
        return f"<tr{attrs}>{m.group('body')}</tr>"

    def body(b: re.Match[str]) -> str:
        return _TR.sub(row, b.group(0))

    return _TBODY.sub(body, table)


def kit_tables(html: str, *, matrix: bool = False) -> str:
    """Apply every ``<!-- huv-table: … -->`` hint to the table that follows it."""

    def hint(m: re.Match[str]) -> str:
        kind = m.group("kind")
        if kind not in TABLE_KINDS:
            return m.group(0)
        table = m.group("table")
        attrs = m.group("attrs")
        table = f"<table{_with_class(attrs, f'huv-{kind}')}>" + table[len(attrs) + 7 :]
        if kind in ("stack", "key", "grid"):
            table = _label_cells(table)
        if kind == "grid":
            table = _grade_rows(table, ids=matrix)
        return f'<div class="md-typeset__scrollwrap"><div class="md-typeset__table">{table}</div></div>'

    return _TABLE_HINT.sub(hint, html)


# --- images ------------------------------------------------------------------------------------


def _attrs(tag: str) -> dict[str, str]:
    return {m.group("name").lower(): html_lib.unescape(m.group("value")) for m in _ATTR.finditer(tag)}


def _add(tag: str, extra: str) -> str:
    end = -2 if tag.endswith("/>") else -1
    return tag[:end].rstrip() + " " + extra + (" />" if end == -2 else ">")


def kit_images(html: str, docs_dir: Path | None = None, src_uri: str | None = None, page_url: str | None = None) -> str:
    """Paths resolved like Markdown images, lazy loading after the first figure image, sizes from
    the files (in CSS px: a 2x sim PNG at half its pixels, :func:`css_size`), ``-anim-`` stills,
    ``data-gallery="any"`` on gallery-less lightbox anchors.
    ``src_uri``/``page_url`` are the page's source path and URL; when either is ``None`` no path is
    rewritten."""
    depth = 0
    first_seen = False

    def visit(m: re.Match[str]) -> str:
        nonlocal depth, first_seen
        tag = m.group(0)
        if tag.startswith("<figure"):
            depth += 1
            return tag
        if tag.startswith("</figure"):
            depth = max(depth - 1, 0)
            return tag
        attrs = _attrs(tag)
        if _EMOJI_CLASS.search(attrs.get("class", "")):
            return tag
        tag = kit_paths(tag, attrs, src_uri, page_url, docs_dir)
        if tag.startswith("<a") and "glightbox" in attrs.get("class", "").split() and "data-gallery" not in attrs:
            return _add(tag, 'data-gallery="any"')
        if not tag.startswith("<img"):
            return tag
        in_figure = depth > 0
        extra: list[str] = []
        if first_seen and "loading" not in attrs:
            extra.append('loading="lazy" decoding="async"')
        if in_figure and not first_seen:
            first_seen = True
        src = attrs.get("src", "")
        if src and "width" not in attrs and "height" not in attrs:
            path = asset_path(src, docs_dir)
            size = css_size(path, docs_dir) if path else None
            if size:
                extra.append(f'width="{size[0]}" height="{size[1]}"')
        still = still_url(src) if src else None
        if still:
            if "huv-anim" not in attrs.get("class", "").split():
                tag = f"<img{_with_class(tag[4:], 'huv-anim')}"
            if "data-huv-still" not in attrs:
                extra.append(f'data-huv-still="{html_lib.escape(still, quote=True)}"')
        return _add(tag, " ".join(extra)) if extra else tag

    return _FIG_OR_IMG.sub(visit, html)


# --- Physics-Strip eyebrows --------------------------------------------------------------------


def _eyebrow_caption(m: re.Match[str], role: str) -> str:
    body = m.group("body")
    if 'class="huv-eyebrow"' in body:
        return m.group(0)
    lead = _CAPTION_LEAD.match(body)
    if lead:
        own = _EYEBROW_LEAD.match(lead.group("text"))
        if own:
            word = next(w for w in EYEBROWS if w.lower() == own.group("word").lower())
            rest = own.group("rest")
            new = f'<span class="huv-eyebrow">{word}</span>'
            tail = body[lead.end() :]
            if rest:
                new += f"<strong>{rest[0].upper() + rest[1:]}</strong>"
            else:
                tail = tail.lstrip()
            return f"{m.group('open')}{new}{tail}</figcaption>"
    return f"{m.group('open')}<span class=\"huv-eyebrow\">{role}</span>{body.lstrip()}</figcaption>"


def trailer_role(caption_body: str) -> str | None:
    """The eyebrow a caption's trailer (its trailing ``<em>``) dictates: ``In the engine`` for a
    ``Model:`` trailer, ``Mechanism`` for the mechanism vocabulary of DESIGN §4.6, else ``None``."""
    trailer = _TRAILER.search(caption_body.strip())
    if not trailer:
        return None
    text = _text(trailer.group("text"))
    if _ENGINE_TRAILER.match(text):
        return EYEBROWS[1]
    if _MECHANISM_TRAILER.search(text):
        return EYEBROWS[0]
    return None


def kit_strips(html: str) -> str:
    """``.huv-eyebrow`` spans on the captions of the figures directly inside ``.huv-physics``: the
    word the caption's own lead names, else the one its trailer dictates, else by position."""
    out: list[str] = []
    pos = 0
    for strip in _STRIP_OPEN.finditer(html):
        if strip.start() < pos:
            continue
        depth, fig_start, fig_depth = 1, None, 0
        figures: list[tuple[int, int]] = []
        end = len(html)
        for tok in _DIV_OR_FIGURE.finditer(html, strip.end()):
            text = tok.group(0)
            if text.startswith("<div"):
                depth += 1
            elif text.startswith("</div"):
                depth -= 1
                if depth == 0:
                    end = tok.end()
                    break
            elif text.startswith("<figure"):
                if fig_start is None:
                    if depth == 1:
                        fig_start, fig_depth = tok.start(), 0
                else:
                    fig_depth += 1
            elif fig_start is not None:
                if fig_depth == 0:
                    figures.append((fig_start, tok.end()))
                    fig_start = None
                else:
                    fig_depth -= 1
        out.append(html[pos : strip.start()])
        last = strip.start()
        for i, (fs, fe) in enumerate(figures):
            positional = EYEBROWS[0] if (i == 0 and len(figures) > 1) else EYEBROWS[1]
            caption = _FIGCAPTION.search(html, fs, fe)
            role = (trailer_role(caption.group("body")) if caption else None) or positional
            out.append(html[last:fs])
            out.append(_FIGCAPTION.sub(lambda c, r=role: _eyebrow_caption(c, r), html[fs:fe], count=1))
            last = fe
        out.append(html[last:end])
        pos = end
    out.append(html[pos:])
    return "".join(out)


# --- paragraphs, rungs, previews ----------------------------------------------------------------


def kit_paragraphs(html: str) -> str:
    html = _P_NEXT.sub('<p class="huv-next">', html)
    html = _P_ROSETTA.sub('<p class="huv-rosetta-line">', html)
    return _P_STRIP.sub('<p class="huv-strip">', html)


def kit_rungs(html: str) -> str:
    return _CELL.sub(lambda m: _RUNG.sub(r'<strong class="huv-rung">\g<word></strong>', m.group(0)), html)


def kit_previews(html: str, *, matrix: bool = False) -> str:
    if matrix:
        return html

    def link(m: re.Match[str]) -> str:
        attrs = m.group("attrs")
        if "data-preview" in attrs or "://" in attrs:
            return m.group(0)
        return f'<a{attrs} data-preview="true">'

    return _MATRIX_LINK.sub(link, html)


def apply(html: str, *, docs_dir: Path | None = None, matrix: bool = False, src_uri: str | None = None, page_url: str | None = None) -> str:
    """Every kit rule in order (strips before tables, tables before images so the wrapper exists)."""
    html = kit_strips(html)
    html = kit_tables(html, matrix=matrix)
    html = kit_images(html, docs_dir, src_uri, page_url)
    html = kit_paragraphs(html)
    html = kit_rungs(html)
    return kit_previews(html, matrix=matrix)


def on_page_content(html: str, page: Page, config: MkDocsConfig, files: Files) -> str:
    docs_dir = Path(config["docs_dir"]) if config and config.get("docs_dir") else None
    src_uri = getattr(getattr(page, "file", None), "src_uri", None)
    page_url = getattr(page, "url", None)
    return apply(html, docs_dir=docs_dir, matrix=(src_uri == MATRIX_PAGE), src_uri=src_uri, page_url=page_url)
