"""MkDocs hook: render the honesty badges as styled, labelled elements.

Pages carry their grade as plain Unicode markers so they read the same on GitHub:
✅ Implemented · 🔶 Simplified · 🧪 Theoretical · 🗺️ Planned (see capability-matrix.md).
After Markdown rendering this hook

* adds ``class="huv-status huv-status--<grade>"`` to the page's leading ``**Status:**``
  paragraph (the first marker in it picks the grade), so it renders as a callout, and
* turns a marker that is directly followed by its own word ("✅ Implemented") into a
  **chip** — ``<span class="huv-chip huv-chip--<grade>"><span class="huv-chip__glyph"
  aria-hidden="true">✅</span>Implemented</span>`` — so the grade reads once and the
  Markdown stays plain ``✅ Implemented`` on GitHub, and
* wraps every other marker in ``<span class="huv-badge huv-badge--<grade>" title="<Grade>"
  role="img" aria-label="<Grade>">``, giving readers a tooltip and screen readers a name. A
  mismatched label ("✅ Simplified") keeps the bare badge and the label text unchanged.

The chip lives in exactly four places by convention (DESIGN §4.7): Status lines, at-a-glance /
model tables, caption trailers and the legend; the hook does not know pages, it knows markup.

Text inside code, pre, script, style, svg, textarea and kbd elements is left untouched.
The styles live in docs/stylesheets/extra.css.
"""

from __future__ import annotations

import re
from typing import TYPE_CHECKING

if TYPE_CHECKING:  # pragma: no cover - typing only
    from mkdocs.config.defaults import MkDocsConfig
    from mkdocs.structure.files import Files
    from mkdocs.structure.pages import Page

BADGES = {
    "✅": ("implemented", "Implemented"),
    "🔶": ("simplified", "Simplified"),
    "🧪": ("theoretical", "Theoretical"),
    "🗺": ("planned", "Planned"),
}

# The marker, its optional variation selector and — consumed, not looked ahead at — an
# optional label: one space (or NBSP) and one of the four grade words (DESIGN §4.7).
_MARKER = re.compile(
    "(?P<marker>[" + "".join(BADGES) + "])(?P<vs>\ufe0f?)"
    r"(?P<label>[ \u00a0]?(?:Implemented|Simplified|Theoretical|Planned)\b)?"
)
_TOKEN = re.compile(r"<!--.*?-->|<[^>]*>", re.S)
_OPEN = re.compile(r"<([A-Za-z][A-Za-z0-9-]*)\b")
_CLOSE = re.compile(r"</([A-Za-z][A-Za-z0-9-]*)\s*>")
_SKIP = frozenset({"code", "pre", "script", "style", "svg", "textarea", "kbd", "math"})
_VOID = frozenset(
    {"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta",
     "param", "source", "track", "wbr"}
)
_STATUS_P = re.compile(r"<p>(?=\s*<strong>Status:</strong>)(?P<body>.*?)</p>", re.S)


def _badge(match: re.Match[str]) -> str:
    glyph = match.group("marker") + (match.group("vs") or "")
    slug, word = BADGES[match.group("marker")]
    label = match.group("label")
    if label is not None and label.strip() == word:
        return (
            f'<span class="huv-chip huv-chip--{slug}">'
            f'<span class="huv-chip__glyph" aria-hidden="true">{glyph}</span>{word}</span>'
        )
    badge = (
        f'<span class="huv-badge huv-badge--{slug}" title="{word}" '
        f'role="img" aria-label="{word}">{glyph}</span>'
    )
    return badge + (label or "")


def wrap_badges(html: str) -> str:
    """Wrap every status marker found in text (not in code-like elements) in a span."""
    out: list[str] = []
    stack: list[str] = []  # open elements since entering a skipped element
    pos = 0
    for token in _TOKEN.finditer(html):
        text = html[pos : token.start()]
        out.append(text if stack else _MARKER.sub(_badge, text))
        tag = token.group(0)
        out.append(tag)
        pos = token.end()
        if tag.startswith("<!--"):
            continue
        close = _CLOSE.match(tag)
        if close:
            name = close.group(1).lower()
            if name in stack:
                while stack and stack.pop() != name:
                    pass
            continue
        opened = _OPEN.match(tag)
        if not opened or tag.endswith("/>"):
            continue
        name = opened.group(1).lower()
        if name in _VOID:
            continue
        if stack or name in _SKIP or (name == "span" and ("huv-badge" in tag or "huv-chip" in tag)):
            stack.append(name)
    tail = html[pos:]
    out.append(tail if stack else _MARKER.sub(_badge, tail))
    return "".join(out)


def mark_status_paragraph(html: str) -> str:
    """Give the first ``<p><strong>Status:</strong> …</p>`` its callout classes.

    The grade is the *first* marker in the paragraph (the headline grade of a split grade
    such as ``✅ Implemented (🔶 coherence bound)``); it is also exposed as ``data-huv-grade``.
    """
    match = _STATUS_P.search(html)
    if not match:
        return html
    first = _MARKER.search(match.group("body"))
    grade = BADGES[first.group("marker")][0] if first else "unknown"
    start = match.start()
    return (
        f'{html[:start]}<p class="huv-status huv-status--{grade}" data-huv-grade="{grade}">'
        f"{html[start + 3 :]}"
    )


def on_page_content(html: str, page: Page, config: MkDocsConfig, files: Files) -> str:
    return wrap_badges(mark_status_paragraph(html))
