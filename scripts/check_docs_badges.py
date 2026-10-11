#!/usr/bin/env python3
"""Lint the honesty vocabulary and the dual-surface forms of the documentation pages.

The grades ✅ Implemented · 🔶 Simplified · 🧪 Theoretical · 🗺️ Planned are copied from
``docs/capability-matrix.md`` into Status lines, tables and caption trailers; the pages are
read both on the MkDocs Material site and raw on github.com, and every ``python`` fence is
run by ``tests/docs/test_site_examples.py``. This lint keeps the forms that make all of that
true (DESIGN §3, §4.7, §4.10, §4.11, §10.1-2) without a site build or the compiled extension.

Modes (DESIGN §9.0 rule 4, the ``check_figures.py`` scheme)
    ``STRICT_DEFAULT = True`` below (flipped by the integration group): page-form rules are
    errors; ``--tolerant`` forces the tolerant mode (warnings only, exit status ignores them)
    whatever the environment says; ``--strict`` (or ``HUV_BADGELINT_STRICT=1`` in the
    environment) forces strict. ``--pages PATH ...`` restricts
    the run to files under the given paths, so a group can lint its own pages strictly while
    other groups' pages are still in the old form.

Always-on errors (every mode; all four hold on the 2026-10-03 baseline)
    tabs                 Material content-tab syntax ``=== "Title"`` anywhere (DESIGN §1.3).
    mermaid-allowlist    a ```mermaid fence outside ``<details>`` on a page that has no line in
                         ``scripts/mermaid_allowlist.txt`` (decision 6, DESIGN §4.11); in strict
                         mode a page may hold no more such fences than it has allowlist lines.
    details-form         under ``docs/`` (site pages): a ``<details`` tag without
                         ``markdown="1"``, or a ``</summary>`` that does not end its line or is
                         not followed by a blank line (DESIGN §3: a bare ``<details>`` prints
                         tables, links and bold text literally on the site; without the blank
                         line github.com prints the Markdown as text).
    unchecked-output     a *silently unchecked* verify-example output: an output-looking block
                         (``??? success "Output"``, ``???+ success "Output"`` or an ``Output…:``
                         lead-in line) within the 10 lines after a closing ```python fence that
                         is not the block the runner's ``find_output`` accepts — the same rule,
                         mirrored line for line from ``tests/docs/test_site_examples.py``: the
                         next non-blank block may be the admonition, an ``Output…:`` lead-in +
                         text fence, a bare text fence, or nothing output-like at all.

Page-form rules (warnings while tolerant, errors when strict)
    heading-glyph        a grade glyph in a heading line.
    split-grade          ``✅/🔶`` slash forms and positional forms in tables — a cell holding two
                         or more bare glyphs, or a row with bare glyphs in two or more cells
                         (Appendix C; the form is ``✅ (🔶 qualifier)``; inline code is ignored).
    status-form          a ``**Status:**`` paragraph that does not start with a glyph followed
                         by its own word, or does not end with a ``capability-matrix.md#`` link.
    status-length        a Status clause over 40 words (the glyph, its word and the link excluded).
    attr-list            ``{ .class }``, ``{ data-… }`` or ``{ width=… }`` on any page.
    only-fragment        a ``#only-light`` / ``#only-dark`` fragment (the pair is ``#gh-*``).
    figcaption-markdown  ``**``, ``__`` or ``*emphasis*`` inside ``<figcaption>`` (HTML only there).
    trailer-wording      a word directly after a glyph in an ``<em>Model: …</em>`` trailer
                         (outside a parenthesised qualifier) — the glyph ends its clause.
    glyphs-per-sentence  more than one bare grade in one prose sentence (a ``✅/🔶`` slash form
                         or ``✅ (🔶 qualifier)`` counts as one grade; tables, headings, raw HTML
                         lines, admonition titles and the Status paragraph are not prose).
    collapsed-admonition a ``???``/``???+`` line whose title is not ``success "Output"`` on a
                         site page (github.com prints it literally; use ``<details markdown="1">``).
    details-layout       the §3 layout around a ``<details>``: the tag alone on its line, the
                         ``<summary>`` on the next line, a blank line before ``</details>``; on
                         README / CONTRIBUTING the blank line after ``</summary>``.
    output-leadin        a verify-example followed by an ``Output…:`` lead-in: docs pages use the
                         ``???+``/``???`` ``success "Output"`` admonition, the README a bare
                         ```text fence with no lead-in line (DESIGN §0-15, §6.1).
    output-missing       a verify-example on a technical page (one carrying a Status line) with
                         no recorded output block (DESIGN §3.4/§3.6: every example prints).
    example-marker       a ```python fence with neither ``<!-- verify-example -->`` nor
                         ``<!-- example-skip: reason -->`` on the line above (the runner fails it).

Usage:  python scripts/check_docs_badges.py [--strict | --tolerant] [PATH ...] [--pages PATH ...]
                                            [--allowlist FILE] [--counts] [--quiet]
PATH defaults to ``docs/ README.md`` under the repository root; ``--pages`` (``nargs="+"``, so
positional PATHs go before it) restricts the run to files under its paths, and alone lints
exactly those paths (``--strict --pages docs/history`` is the per-group form). YAML front
matter, fenced code, HTML comments and inline code spans are invisible to every rule.
Exit status: 1 if any error remains (warnings count as errors in strict mode), 0 otherwise.
"""

from __future__ import annotations

import argparse
import os
import re
import sys
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

STRICT_DEFAULT = True  # flipped by INT (DESIGN §9.0 rule 4, §9.2 INT row); --tolerant restores the pre-INT mode
STRICT_ENV = "HUV_BADGELINT_STRICT"
ROOT = Path(__file__).resolve().parents[1]
ALLOWLIST_PATH = Path(__file__).resolve().parent / "mermaid_allowlist.txt"
CATEGORIES = ("loop", "decision-tree", "legacy")
STATUS_WORD_LIMIT = 40
OUTPUT_WINDOW = 10  # lines after a closing python fence scanned for a stray output block

GLYPHS = "✅🔶🧪🗺"
WORDS = {"✅": "Implemented", "🔶": "Simplified", "🧪": "Theoretical", "🗺": "Planned"}
_G = f"[{GLYPHS}]️?"  # one glyph, optional variation selector

# --- regexes mirrored from tests/docs/test_site_examples.py (the runner's own acceptance) ---
FENCE_RE = re.compile(r"^(?P<indent>[ \t]*)```python\s*$")
VERIFY_RE = re.compile(r"^[ \t]*<!--\s*verify-example\s*-->\s*$")
SKIP_RE = re.compile(r"^[ \t]*<!--\s*example-skip:\s*\S.*-->\s*$")
OUTPUT_RE = re.compile(r'^(?P<indent>[ \t]*)\?\?\?\+? success "Output"\s*$')
LEADIN_RE = re.compile(r"^[ \t]*Output\b[^`]*:\s*$")

# --- structure ------------------------------------------------------------------------------
_PREFIX = re.compile(r"^((?:[ \t]*>)*[ \t]*)")
_FENCE_OPEN = re.compile(r"^(?P<fence>`{3,}(?=[^`]*$)|~{3,})[ \t]*(?P<info>[^\s`{]*)")
_FENCE_CLOSE = re.compile(r"^(?P<fence>`{3,}|~{3,})[ \t]*$")
_COMMENT = re.compile(r"<!--.*?-->", re.S)
_BACKTICKS = re.compile(r"(?<!\\)`+")
_TAB = re.compile(r"^[ \t]*===[!+]? \"")
_HEADING = re.compile(r"^ {0,3}#{1,6}[ \t]")
_TABLE_ROW = re.compile(r"^[ \t]*\|")
_DETAILS_OPEN = re.compile(r"<details\b[^>]*>", re.I)
_DETAILS_CLOSE = re.compile(r"</details\s*>", re.I)
_SUMMARY_OPEN = re.compile(r"<summary\b[^>]*>", re.I)
_SUMMARY_CLOSE = re.compile(r"</summary\s*>", re.I)
_MARKDOWN_ATTR = re.compile(r"""\bmarkdown\s*=\s*(?:"1"|'1'|1)(?=[\s>/])""", re.I)
_ADMONITION = re.compile(r'^[ \t]*\?\?\?\+?[ \t]+(?P<rest>.*)$')
_OUTPUT_TITLE = re.compile(r'^success[ \t]+"Output"[ \t]*$')

# --- vocabulary -----------------------------------------------------------------------------
_GLYPH = re.compile(_G)
_BARE_GLYPH = re.compile(_G + r"(?!\s*(?:Implemented|Simplified|Theoretical|Planned)\b)")
_SLASH = re.compile(_G + r"\s*/\s*" + _G)
_POSITIONAL_CELL = re.compile(r"^(?:" + _G + r"[\s·,/]*){2,}$")
_GLYPH_ONLY_CELL = re.compile(r"^" + _G + r"$")
# Appendix C forms that are ONE grade for the glyphs-per-sentence count: `✅/🔶` (retired, flagged by
# split-grade) and `✅ (🔶 qualifier; 🧪 other)` (the headline glyph with its parenthesised qualifiers)
_ONE_GRADE = re.compile(_G + r"(?:\s*/\s*" + _G + r")+|" + _G + r"\s*\([^()]*?" + _G + r"[^()]*\)")
_STATUS_HEAD = re.compile(
    r"^(?P<glyph>[" + GLYPHS + r"])️?\s+(?P<word>Implemented|Simplified|Theoretical|Planned)\b"
)
_STATUS_LINK = re.compile(r"\[[^\]]*\]\([^)\s]*capability-matrix\.md#[^)\s]+\)\.?\s*$")
_ATTR_LIST = re.compile(r"\{\s*(?:\.[A-Za-z_-]|data-[A-Za-z-]|width=)")
_ONLY = re.compile(r"#only-(?:light|dark)\b")
_FIGCAPTION = re.compile(r"<figcaption\b[^>]*>(?P<body>.*?)</figcaption>", re.S | re.I)
_CAPTION_MD = re.compile(r"\*\*|__|(?<![\w*])\*(?!\s)[^*\n]+?(?<!\s)\*(?![\w*])")
_TRAILER = re.compile(r"<em>\s*Model:(?P<body>.*?)</em>", re.S | re.I)
_SENTENCE_SPLIT = re.compile(r"(?<=[.!?])\s+(?=[A-Z\"'(\[" + GLYPHS + "])")
_HTML_LINE = re.compile(r"^[ \t]*<(?!!--)[A-Za-z/]")
_LINK_TEXT = re.compile(r"!?\[([^\]]*)\]\([^)]*\)")

ALWAYS = frozenset({"tabs", "mermaid-allowlist", "details-form", "unchecked-output"})
PAGE_FORM = (
    "heading-glyph", "split-grade", "status-form", "status-length", "attr-list", "only-fragment",
    "figcaption-markdown", "trailer-wording", "glyphs-per-sentence", "collapsed-admonition",
    "details-layout", "output-leadin", "output-missing", "example-marker",
)


@dataclass
class Finding:
    line: int  # 1-based
    rule: str
    message: str
    level: str = "warning"  # "error" | "warning"

    @property
    def always(self) -> bool:
        return self.rule in ALWAYS


@dataclass
class Result:
    page: str
    strict: bool
    findings: list[Finding] = field(default_factory=list)

    @property
    def errors(self) -> list[Finding]:
        return [f for f in self.findings if f.level == "error"]

    @property
    def warnings(self) -> list[Finding]:
        return [f for f in self.findings if f.level == "warning"]


# --- page classes ---------------------------------------------------------------------------


def is_site_page(page: str) -> bool:
    """A page MkDocs renders: under docs/, outside the `exclude_docs` trees of mkdocs.yml."""
    excluded = ("docs/figures/", "docs/hooks/", "docs/overrides/", "docs/javascripts/", "docs/stylesheets/", "docs/assets/")
    return page.startswith("docs/") and not page.startswith(excluded)


def is_runner_file(page: str) -> bool:
    """A file tests/docs/test_site_examples.py scans: every docs/**/*.md and README.md."""
    return page.startswith("docs/") or page == "README.md"


# --- the runner's own output acceptance (mirrored, not imported) ---------------------------


@dataclass
class PyBlock:
    line: int  # 1-based line of the opening fence
    end: int  # 0-based index of the closing fence line
    indent: str
    kind: str  # "verify" | "skip" | "unclassified"


def extract_python(lines: list[str]) -> list[PyBlock]:
    """Python fences exactly as the runner's ``extract()`` finds them (no fence nesting)."""
    blocks, i = [], 0
    while i < len(lines):
        m = FENCE_RE.match(lines[i])
        if not m:
            i += 1
            continue
        indent = m.group("indent")
        j = i + 1
        while j < len(lines) and lines[j].rstrip() != indent + "```":
            j += 1
        prev = lines[i - 1] if i > 0 else ""
        kind = "verify" if VERIFY_RE.match(prev) else "skip" if SKIP_RE.match(prev) else "unclassified"
        blocks.append(PyBlock(i + 1, j, indent, kind))
        i = j + 1
    return blocks


def find_output(lines: list[str], block: PyBlock) -> tuple[int, int] | None:
    """``(start, stop)`` line indices of the recorded output the runner would check, or None —
    the same walk as ``tests/docs/test_site_examples.py::find_output`` (admonition, ``Output…:``
    lead-in + ```text fence, or a bare ```text fence; nothing else)."""
    k = block.end + 1
    while k < len(lines) and not lines[k].strip():
        k += 1
    if k >= len(lines):
        return None
    first = k
    m = OUTPUT_RE.match(lines[k])
    if m and m.group("indent") == block.indent:
        k += 1
        while k < len(lines) and not lines[k].strip():
            k += 1
        indent = block.indent + "    "
    else:
        indent = block.indent
        if LEADIN_RE.match(lines[k]) and lines[k].startswith(indent + "Output"):
            k += 1
            while k < len(lines) and not lines[k].strip():
                k += 1
    if k >= len(lines) or lines[k].rstrip() != indent + "```text":
        return None
    j = k + 1
    while j < len(lines) and lines[j].rstrip() != indent + "```":
        j += 1
    return first, j + 1


def output_looking(line: str) -> bool:
    return bool(OUTPUT_RE.match(line) or LEADIN_RE.match(line))


# --- structural scan ------------------------------------------------------------------------


@dataclass
class Fence:
    start: int  # 0-based line of the opening fence
    end: int  # 0-based line of the closing fence (len(lines) when unclosed)
    info: str  # first word of the info string, lower-cased
    details_depth: int  # <details> nesting depth where the fence opens


@dataclass
class Doc:
    lines: list[str]  # raw lines
    text: list[str]  # lines with HTML comments blanked (same length)
    code: set[int]  # 0-based indices of lines inside a fenced block, fence lines included
    fences: list[Fence]
    depth: list[int]  # <details> nesting depth before each line
    front_matter_end: int  # first content line index


def mask_comments(text: str) -> str:
    return _COMMENT.sub(lambda m: re.sub(r"[^\n]", " ", m.group(0)), text)


def mask_code_spans(line: str) -> str:
    """Blank inline code spans (``...``) so vocabulary rules skip them."""
    out, k = [], 0
    while True:
        m = _BACKTICKS.search(line, k)
        if not m:
            out.append(line[k:])
            break
        close = re.compile(r"(?<!`)" + m.group(0) + r"(?!`)").search(line, m.end())
        if close:
            out.append(line[k : m.start()] + " " * (close.end() - m.start()))
            k = close.end()
        else:
            out.append(line[k : m.end()])
            k = m.end()
    return "".join(out)


def scan(raw: str) -> Doc:
    lines = raw.splitlines()
    text = mask_comments(raw).splitlines()
    code: set[int] = set()
    fences: list[Fence] = []
    depth: list[int] = []
    start = 0
    if lines and lines[0].rstrip() == "---":
        for k in range(1, len(lines)):
            if lines[k].rstrip() in ("---", "..."):
                start = k + 1
                break
    for i in range(start):
        text[i] = ""  # YAML front matter is invisible to every rule
    fence: str | None = None
    open_fence: Fence | None = None
    d = 0
    for i, line in enumerate(text):
        depth.append(d)
        if i < start:
            continue
        content = _PREFIX.match(line).group(1) and line[len(_PREFIX.match(line).group(1)) :] or line
        if fence:
            code.add(i)
            m = _FENCE_CLOSE.match(content)
            if m and m.group("fence")[0] == fence[0] and len(m.group("fence")) >= len(fence):
                open_fence.end = i
                fence = None
            continue
        m = _FENCE_OPEN.match(content)
        if m:
            fence = m.group("fence")
            open_fence = Fence(i, len(lines), m.group("info").lower(), d)
            fences.append(open_fence)
            code.add(i)
            continue
        d += len(_DETAILS_OPEN.findall(line)) - len(_DETAILS_CLOSE.findall(line))
        d = max(d, 0)
    return Doc(lines, text, code, fences, depth, start)


# --- allowlist ------------------------------------------------------------------------------


def load_allowlist(path: Path | None = None) -> Counter:
    """``path: category`` lines → the number of mermaid fences each page may hold outside
    ``<details>`` (one line per fence; a path may repeat)."""
    path = path or ALLOWLIST_PATH
    counts: Counter = Counter()
    if not path.exists():
        return counts
    for n, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        page, _, category = line.rpartition(":")
        page, category = page.strip(), category.strip()
        if not page or category not in CATEGORIES:
            raise ValueError(
                f"{path}:{n}: expected '<path>: loop|decision-tree|legacy', got {raw.strip()!r}"
            )
        counts[page] += 1
    return counts


# --- the checker ----------------------------------------------------------------------------


class _Checker:
    def __init__(self, raw: str, page: str, allowlist: Counter, strict: bool):
        self.doc = scan(raw)
        self.page = page
        self.site = is_site_page(page)
        self.allowlist = allowlist
        self.res = Result(page, strict)
        self.strict = strict
        self.status_lines: set[int] = set()
        self.technical = False

    # reporting ---------------------------------------------------------------------------
    def error(self, i: int, rule: str, message: str) -> None:
        self.res.findings.append(Finding(i + 1, rule, message, "error"))

    def warn(self, i: int, rule: str, message: str) -> None:
        self.res.findings.append(Finding(i + 1, rule, message, "error" if self.strict else "warning"))

    def run(self) -> Result:
        self.check_tabs()
        self.check_mermaid()
        self.check_details()
        self.check_admonitions()
        self.check_status()  # sets self.technical and self.status_lines first
        self.check_headings()
        self.check_vocabulary()
        self.check_captions()
        self.check_examples()
        self.res.findings.sort(key=lambda f: (f.line, f.rule))
        return self.res

    # always-on ---------------------------------------------------------------------------
    def check_tabs(self) -> None:
        for i, line in enumerate(self.doc.text):
            if i in self.doc.code:
                continue
            if _TAB.match(line):
                self.error(i, "tabs", 'Material tab syntax `=== "…"` is never used (DESIGN §1.3): write sequential sections or a <div class="huv-rosetta" markdown> block')

    def check_mermaid(self) -> None:
        outside = [f for f in self.doc.fences if f.info == "mermaid" and f.details_depth == 0]
        allowed = self.allowlist.get(self.page, 0)
        if not outside:
            return
        if allowed == 0:
            for f in outside:
                self.error(
                    f.start, "mermaid-allowlist",
                    f"mermaid fence outside <details> on a page with no line in {ALLOWLIST_PATH.name} "
                    "(DESIGN §4.11 keeps process loops and decision trees only): move it into "
                    '<details markdown="1"><summary>…</summary> as the GitHub fallback, replace it with a '
                    f"scripted SVG or table, or add '{self.page}: loop|decision-tree' to the allowlist",
                )
        elif self.strict and len(outside) > allowed:
            self.error(
                outside[allowed].start, "mermaid-allowlist",
                f"page holds {len(outside)} mermaid fences outside <details> but {ALLOWLIST_PATH.name} "
                f"permits {allowed} (one line per fence, DESIGN §4.11)",
            )

    def check_details(self) -> None:
        lines, text, code = self.doc.lines, self.doc.text, self.doc.code
        for i, raw in enumerate(text):
            if i in code:
                continue
            line = mask_code_spans(raw)
            for m in _DETAILS_OPEN.finditer(line):
                tag = m.group(0)
                if self.site:
                    if not _MARKDOWN_ATTR.search(tag):
                        self.error(
                            i, "details-form",
                            '<details> under docs/ is written <details markdown="1"> (DESIGN §3): without the '
                            "attribute a table, a link, bold text or a list inside it prints literally on the site",
                        )
                    if line.strip() != tag:
                        self.warn(i, "details-layout", '<details markdown="1"> stands alone on its line, the <summary> on the next line (DESIGN §3)')
                    else:
                        nxt = text[i + 1] if i + 1 < len(text) else ""
                        if not (_SUMMARY_OPEN.match(nxt.strip()) and _SUMMARY_CLOSE.search(nxt)):
                            self.warn(i, "details-layout", "the line after <details markdown=\"1\"> is <summary>…</summary> (DESIGN §3)")
            for m in _SUMMARY_CLOSE.finditer(line):
                after = line[m.end():].strip()
                blank_next = i + 1 < len(text) and not text[i + 1].strip()
                if after or not blank_next:
                    msg = (
                        "a blank line follows </summary> (DESIGN §3): github.com renders the Markdown after a "
                        "<details> block only when the blank line closes the HTML block"
                        + ("; nothing else goes on the </summary> line" if after else "")
                    )
                    if self.site:
                        self.error(i, "details-form", msg)
                    else:
                        self.warn(i, "details-layout", msg)
            if self.site and _DETAILS_CLOSE.search(line):
                prev = text[i - 1] if i > 0 else ""
                if line.strip().startswith("</details") and prev.strip():
                    self.warn(i, "details-layout", "a blank line precedes </details> (DESIGN §3: content at column 0, a blank line, </details>)")

    # page-form ---------------------------------------------------------------------------
    def check_admonitions(self) -> None:
        if not self.site:
            return
        for i, line in enumerate(self.doc.text):
            if i in self.doc.code:
                continue
            m = _ADMONITION.match(line)
            if m and not _OUTPUT_TITLE.match(m.group("rest").strip()):
                self.warn(
                    i, "collapsed-admonition",
                    '???/???+ is reserved for success "Output" (DESIGN §3, §4.10): github.com prints this line '
                    'literally and its body as code; use <details markdown="1"><summary>…</summary>',
                )

    def check_status(self) -> None:
        text, code = self.doc.text, self.doc.code
        for i, line in enumerate(text):
            if i in code or not line.startswith("**Status:**"):
                continue
            j = i
            while j + 1 < len(text) and text[j + 1].strip() and (j + 1) not in code:
                j += 1
            self.status_lines = set(range(i, j + 1))
            self.technical = self.site
            para = " ".join(text[k].strip() for k in range(i, j + 1))
            body = para[len("**Status:**"):].strip()
            head = _STATUS_HEAD.match(body)
            if not head or WORDS[head.group("glyph")] != head.group("word"):
                self.warn(
                    i, "status-form",
                    "the Status line starts with the headline glyph followed by its own word "
                    "(`**Status:** ✅ Implemented — …`, Appendix C split form), so the hook emits the chip",
                )
            if not _STATUS_LINK.search(body):
                self.warn(i, "status-form", "the Status line ends with its matrix link `· [matrix](capability-matrix.md#<domain>)` (DESIGN §4.7)")
            clause = _STATUS_LINK.sub("", body)
            if head:
                clause = clause[head.end():]
            clause = _GLYPH.sub(" ", _LINK_TEXT.sub(r"\1", mask_code_spans(clause)))
            words = [w for w in clause.split() if re.search(r"\w", w)]
            if len(words) > STATUS_WORD_LIMIT:
                self.warn(i, "status-length", f"the Status clause has {len(words)} words; the limit is {STATUS_WORD_LIMIT} (DESIGN §4.7: long caveats move to the glance table and the lead)")
            break

    def check_headings(self) -> None:
        for i, line in enumerate(self.doc.text):
            if i in self.doc.code or not _HEADING.match(line):
                continue
            if _GLYPH.search(mask_code_spans(line)):
                self.warn(i, "heading-glyph", "no grade glyph in a heading (DESIGN §3): the grade goes into the first body line or the Status line")

    def check_vocabulary(self) -> None:
        text, code = self.doc.text, self.doc.code
        para: list[tuple[int, str]] = []

        def flush() -> None:
            if not para:
                return
            first = para[0][0]
            prose = " ".join(t for _, t in para)
            prose = _FIGCAPTION.sub(" ", prose)
            prose = _TRAILER.sub(" ", prose)
            for sentence in _SENTENCE_SPLIT.split(prose):
                n = len(_BARE_GLYPH.findall(_ONE_GRADE.sub("✅", sentence)))
                if n > 1:
                    self.warn(first, "glyphs-per-sentence", f"{n} bare grade glyphs in one sentence; at most one per prose sentence (DESIGN §3): name each capability and put chips in tables")
                    break
            para.clear()

        for i, raw in enumerate(text):
            if i in code:
                flush()
                continue
            line = mask_code_spans(raw)
            if self.site:
                if _ATTR_LIST.search(line):
                    self.warn(i, "attr-list", "attr_list `{ … }` prints literally on github.com (DESIGN §0-8): use the hook-styled forms (page_kit.py adds data-preview, widths come from the file)")
                if _ONLY.search(line):
                    self.warn(i, "only-fragment", "`#only-light`/`#only-dark` is retired: the light/dark pair uses `#gh-light-mode-only` / `#gh-dark-mode-only` (DESIGN §0-5, §4.6)")
            if _SLASH.search(line):
                self.warn(i, "split-grade", "slash-joined grades are retired (Appendix C): write the headline grade first with the qualifier in parentheses, e.g. `✅ (🔶 coherence bound)`")
            if _TABLE_ROW.match(line):
                cells = [c.strip() for c in line.strip().strip("|").split("|")]
                if any(_POSITIONAL_CELL.match(c) for c in cells) or sum(bool(_GLYPH_ONLY_CELL.match(c)) for c in cells) >= 2:
                    self.warn(i, "split-grade", "positional multi-glyph cell is retired (Appendix C): each named capability carries its own badge, `✅ (🔶 qualifier)` for a split grade")
            # prose sentences: skip headings, tables, admonition titles, raw HTML lines and the Status paragraph
            if (not line.strip() or _HEADING.match(line) or _TABLE_ROW.match(line) or _ADMONITION.match(line)
                    or line.lstrip().startswith("!!!") or _HTML_LINE.match(line) or i in self.status_lines):
                flush()
                continue
            para.append((i, line.strip()))
        flush()

    def check_captions(self) -> None:
        joined = "\n".join(self.doc.text)
        offsets = [0]
        for line in self.doc.text:
            offsets.append(offsets[-1] + len(line) + 1)

        def line_of(pos: int) -> int:
            lo, hi = 0, len(offsets) - 1
            while lo < hi:
                mid = (lo + hi + 1) // 2
                if offsets[mid] <= pos:
                    lo = mid
                else:
                    hi = mid - 1
            return lo

        for m in _FIGCAPTION.finditer(joined):
            i = line_of(m.start())
            if i in self.doc.code:
                continue
            if _CAPTION_MD.search(m.group("body")):
                self.warn(i, "figcaption-markdown", "Markdown emphasis inside <figcaption> prints as asterisks on github.com: use <strong>/<em> (DESIGN §4.6 caption grammar)")
        for m in _TRAILER.finditer(joined):
            i = line_of(m.start())
            if i in self.doc.code:
                continue
            body = m.group("body")
            depth, k = 0, 0
            while k < len(body):
                ch = body[k]
                if ch == "(":
                    depth += 1
                elif ch == ")":
                    depth = max(depth - 1, 0)
                elif ch in GLYPHS and depth == 0:
                    rest = body[k + 1 :].lstrip("️ \t\n")
                    if rest and (rest[0].isalnum()):
                        self.warn(i, "trailer-wording", f"`{ch}` is followed by `{rest.split()[0]}` in the Model trailer: a glyph ends its clause (` · `, `;`, `.`) and qualifiers go in parentheses after it (DESIGN §4.6, §5.0)")
                        break
                k += 1

    def check_examples(self) -> None:
        if not is_runner_file(self.page):
            return
        lines = self.doc.lines
        readme = self.page == "README.md"
        for b in extract_python(lines):
            if b.kind == "unclassified":
                self.warn(b.line - 1, "example-marker", "a ```python fence needs `<!-- verify-example -->` or `<!-- example-skip: reason -->` on the line directly above (tests/docs/test_site_examples.py fails without it)")
                continue
            if b.kind != "verify":
                continue
            out = find_output(lines, b)
            window = range(b.end + 1, min(b.end + 1 + OUTPUT_WINDOW, len(lines)))
            accepted = range(out[0], out[1]) if out else range(0)
            for k in window:
                if k not in accepted and output_looking(lines[k]):
                    self.error(
                        k, "unchecked-output",
                        f"output block {k - b.end} line(s) after the python fence at line {b.line} is not the fence's next "
                        "non-blank block, so tests/docs/test_site_examples.py never checks it (silently unchecked): put the "
                        '???+/??? success "Output" admonition (README: the bare ```text fence) directly after the fence, nothing between',
                    )
                    break
            if out is None:
                if self.technical:
                    self.warn(b.line - 1, "output-missing", 'this verify-example records no output; technical pages print something and keep it in a ???+/??? success "Output" admonition (DESIGN §3.4, §3.6; run --update-outputs)')
                continue
            first = lines[out[0]]
            if LEADIN_RE.match(first):
                if readme:
                    self.warn(out[0], "output-leadin", "README outputs are a bare ```text fence directly after the python fence, with no `Output:` line (DESIGN §6.1, Appendix D)")
                else:
                    self.warn(out[0], "output-leadin", 'an `Output:` lead-in on a docs page: use `???+ success "Output"` (≤ 6 lines) / `??? success "Output"` holding the 4-space-indented ```text fence (DESIGN §0-15; --update-outputs writes it)')


def check_text(text: str, page: str = "docs/page.md", allowlist: Counter | dict | None = None, strict: bool = False) -> Result:
    """Lint one Markdown document. ``page`` is its repository-relative posix path (it decides
    whether the file is a site page and is the key into the mermaid allowlist)."""
    return _Checker(text, page, Counter(allowlist or {}), strict).run()


# --- CLI ------------------------------------------------------------------------------------


def page_key(path: Path) -> str:
    """The repository-relative posix path used in messages and as the allowlist key."""
    resolved = path.resolve()
    for base in (ROOT, Path.cwd().resolve()):
        try:
            return resolved.relative_to(base).as_posix()
        except ValueError:
            continue
    return path.as_posix()


def collect(paths: list[Path]) -> list[Path]:
    files: set[Path] = set()
    for p in paths:
        if p.is_dir():
            files.update(f for f in p.rglob("*.md") if f.is_file())
        elif p.is_file():
            files.add(p)
        else:
            raise FileNotFoundError(p)
    return sorted(files, key=lambda f: page_key(f))


def restrict(files: list[Path], pages: list[Path]) -> list[Path]:
    roots = [p.resolve() for p in pages]
    return [f for f in files if any(f.resolve() == r or r in f.resolve().parents for r in roots)]


def resolve_mode(strict_flag: bool, tolerant_flag: bool, env: dict | None = None) -> bool:
    env = os.environ if env is None else env
    if strict_flag:
        return True
    if tolerant_flag:
        return False
    if env.get(STRICT_ENV, "").strip().lower() in ("1", "true", "yes", "on"):
        return True
    return STRICT_DEFAULT


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(
        description="Lint the honesty vocabulary and dual-surface forms of the docs pages (DESIGN §10.1-2).",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="Always-on errors: tabs, mermaid-allowlist, details-form, unchecked-output. "
        "Page-form rules warn in tolerant mode and error in strict mode.",
    )
    ap.add_argument("paths", nargs="*", type=Path, help="Markdown files or directories (default: docs/ README.md)")
    mode = ap.add_mutually_exclusive_group()
    mode.add_argument("--strict", action="store_true", help="page-form findings are errors (also: HUV_BADGELINT_STRICT=1)")
    mode.add_argument("--tolerant", action="store_true", help="page-form findings are warnings even when HUV_BADGELINT_STRICT is set")
    ap.add_argument("--pages", nargs="+", type=Path, metavar="PATH", help="restrict the run to files under these paths (alone: lint exactly these paths); positional PATHs go before it")
    ap.add_argument("--allowlist", type=Path, default=ALLOWLIST_PATH, help="mermaid allowlist file (default: scripts/mermaid_allowlist.txt)")
    ap.add_argument("--counts", action="store_true", help="print the number of findings per rule")
    ap.add_argument("--quiet", "-q", action="store_true", help="print findings only (no summary)")
    args = ap.parse_args(argv)
    strict = resolve_mode(args.strict, args.tolerant)
    try:
        if args.paths:
            files = collect(args.paths)
            if args.pages:
                files = restrict(files, args.pages)
        elif args.pages:
            files = collect(args.pages)  # `--pages` alone lints exactly those files / directories
        else:
            files = collect([ROOT / "docs", ROOT / "README.md"])
        allowlist = load_allowlist(args.allowlist)
    except FileNotFoundError as exc:
        ap.error(f"no such file or directory: {exc}")
    except ValueError as exc:
        ap.error(str(exc))
    errors = warnings = bad_files = 0
    per_rule: Counter = Counter()
    for path in files:
        page = page_key(path)
        res = check_text(path.read_text(encoding="utf-8"), page, allowlist, strict)
        for f in res.findings:
            print(f"{page}:{f.line}: {f.level}: [{f.rule}] {f.message}")
            per_rule[(f.rule, f.level)] += 1
        errors += len(res.errors)
        warnings += len(res.warnings)
        bad_files += bool(res.findings)
    if args.counts and per_rule:
        print("rule counts:")
        for (rule, level), n in sorted(per_rule.items()):
            print(f"  {rule}: {n} {level}(s)")
    if not args.quiet:
        print(
            f"check_docs_badges: {errors} error(s), {warnings} warning(s) in {bad_files} of {len(files)} file(s) "
            f"({'strict' if strict else 'tolerant'} mode; always-on rules: tabs, mermaid-allowlist, details-form, unchecked-output)"
        )
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
