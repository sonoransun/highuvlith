"""MkDocs hook: point links that leave the published docs at the repository on GitHub.

The technical pages under docs/ are written to read well on GitHub, so they link to
source files with relative paths such as ``../crates/highuvlith-core/src/aerial.rs`` or
``../../examples/sim.toml``. Those targets are not part of the built site and MkDocs
(rightly) reports them as broken under ``--strict``. Before MkDocs validates a page, this
hook rewrites each such link in the page's Markdown to an absolute repository URL:

    ../crates/highuvlith-core/src/aerial.rs#L10 -> {repo_url}/blob/{branch}/crates/highuvlith-core/src/aerial.rs#L10
    ../crates/highuvlith-core                   -> {repo_url}/tree/{branch}/crates/highuvlith-core
    ../examples/figure.png                      -> {repo_url}/raw/{branch}/examples/figure.png

Files inside docs/ that are excluded from the site (``exclude_docs``: the figure scripts
under docs/figures/, these hooks, ...) are rewritten the same way. A link is only
rewritten when its target exists in the checkout, so a mistyped source path still fails
the build instead of turning into a GitHub 404. Code spans and fenced code blocks are
left untouched. The branch is read from ``edit_uri`` (``edit/<branch>/docs/``) and
defaults to ``main``.

After rendering, links that point off-site (another host than ``site_url``) get the
class ``huv-external``; the print stylesheet prints their URL after the link text.
"""

from __future__ import annotations

import logging
import os
import posixpath
import re
from pathlib import Path
from typing import TYPE_CHECKING, Callable
from urllib.parse import quote, unquote, urlsplit

if TYPE_CHECKING:  # pragma: no cover - typing only
    from mkdocs.config.defaults import MkDocsConfig
    from mkdocs.structure.files import Files
    from mkdocs.structure.pages import Page

log = logging.getLogger("mkdocs.hooks.repo_links")

IMAGE_EXTENSIONS = frozenset(
    {".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp", ".avif", ".bmp", ".ico"}
)

# A fenced code block opener/closer: up to any indentation (fences may sit in lists or
# admonitions), then three or more backticks or tildes.
_FENCE = re.compile(r"^(?P<indent>[ \t]*)(?P<fence>`{3,}|~{3,})")
# Inline code span: a run of backticks, the shortest content, the same run again. Like
# CommonMark, a span may wrap lines but never crosses a blank line (a paragraph break).
_CODE_SPAN = re.compile(r"(?<!`)(`+)(?!`)(?:(?!\n[ \t]*\n).)+?(?<!`)\1(?!`)", re.S)
# Inline link or image target: "](" url [title] ")". The URL may be wrapped in <...>
# and may contain one level of balanced parentheses.
_INLINE_TARGET = re.compile(
    r"\]\(\s*"
    r"(?P<url><[^<>\n]*>|(?:[^\s()<>]|\([^\s()]*\))+)"
    r"(?P<title>\s+(?:\"[^\"\n]*\"|'[^'\n]*'|\([^()\n]*\)))?"
    r"\s*\)"
)
# Reference definition: [label]: url "title"
_REF_DEF = re.compile(
    r"^(?P<lead>[ ]{0,3}\[[^\]\n]+\]:[ \t]*)(?P<url><[^<>\n]*>|\S+)", re.M
)
# href="..." / src="..." inside raw HTML written in the Markdown.
_HTML_ATTR = re.compile(r"(?P<lead>\b(?:href|src)\s*=\s*)(?P<q>[\"'])(?P<url>.*?)(?P=q)")
_SCHEME = re.compile(r"^[A-Za-z][A-Za-z0-9+.-]*:")
_ANCHOR_TAG = re.compile(r"<a\s[^>]*>", re.I)
_HREF_ATTR = re.compile(r"""\bhref\s*=\s*(["'])(https?://[^"']*)\1""", re.I)
_CLASS_ATTR = re.compile(r"""\bclass\s*=\s*(["'])(.*?)\1""", re.I | re.S)
_PLACEHOLDER = "\x00huvcode{}\x00"


def branch_from_edit_uri(edit_uri: str | None, default: str = "main") -> str:
    """Return the branch named in an edit_uri such as ``edit/main/docs/``."""
    if edit_uri:
        match = re.search(r"(?:^|/)(?:edit|blob|tree|raw)/([^/]+)/", edit_uri)
        if match:
            return match.group(1)
    return default


def repo_url_for(
    url: str,
    *,
    page_src_uri: str,
    docs_dir: Path,
    repo_root: Path,
    repo_url: str,
    branch: str,
    is_published: Callable[[str], bool] = lambda _docs_path: True,
) -> str | None:
    """Return the repository URL that ``url`` should become, or None to keep it.

    ``page_src_uri`` is the page's path relative to ``docs_dir`` with forward slashes.
    ``is_published(docs_path)`` says whether a docs-relative path is part of the site.
    """
    if not url or url.startswith(("#", "/", "\\")) or _SCHEME.match(url):
        return None
    parts = urlsplit(url)
    if parts.scheme or parts.netloc or not parts.path:
        return None
    target = Path(
        os.path.normpath(docs_dir / posixpath.dirname(page_src_uri) / unquote(parts.path))
    )
    docs_rel = os.path.relpath(target, docs_dir)
    if docs_rel != ".." and not docs_rel.startswith(".." + os.sep):
        docs_path = Path(docs_rel).as_posix()
        if is_published(docs_path):
            return None  # A normal site-internal link: MkDocs resolves and validates it.
        if target.is_dir() and any(
            is_published(posixpath.normpath(f"{docs_path}/{name}"))
            for name in ("index.md", "README.md")
        ):
            return None  # A directory URL for a section index page.
    repo_rel = os.path.relpath(target, repo_root)
    if repo_rel == ".." or repo_rel.startswith(".." + os.sep):
        return None  # Outside the repository: leave it for MkDocs to report.
    if not target.exists():
        return None  # Mistyped path: keep it so the --strict build fails loudly.

    base = repo_url.rstrip("/")
    repo_path = Path(repo_rel).as_posix()
    if repo_path == ".":
        new = f"{base}/tree/{branch}"
    else:
        if target.is_dir():
            kind = "tree"
        elif target.suffix.lower() in IMAGE_EXTENSIONS:
            kind = "raw"
        else:
            kind = "blob"
        new = f"{base}/{kind}/{branch}/{quote(repo_path)}"
    if parts.query:
        new += f"?{parts.query}"
    if parts.fragment:
        new += f"#{parts.fragment}"
    return new


def _split_fenced(markdown: str) -> list[tuple[bool, str]]:
    """Split Markdown into (is_code, text) chunks at fenced code block boundaries."""
    chunks: list[tuple[bool, str]] = []
    buf: list[str] = []
    fence: str | None = None
    for line in markdown.splitlines(keepends=True):
        match = _FENCE.match(line)
        if fence is None:
            if match:
                if buf:
                    chunks.append((False, "".join(buf)))
                buf = [line]
                fence = match.group("fence")
            else:
                buf.append(line)
        else:
            buf.append(line)
            if (
                match
                and match.group("fence")[0] == fence[0]
                and len(match.group("fence")) >= len(fence)
                and not line[match.end() :].strip()
            ):
                chunks.append((True, "".join(buf)))
                buf = []
                fence = None
    if buf:
        chunks.append((fence is not None, "".join(buf)))
    return chunks


def rewrite_markdown_links(markdown: str, resolve: Callable[[str], str | None]) -> str:
    """Rewrite every link target in ``markdown`` for which ``resolve`` returns a URL.

    Handles inline links and images, reference definitions, and href/src attributes of
    raw HTML. Fenced code blocks and inline code spans are never modified.
    """

    def swap(url: str) -> str | None:
        bare = url[1:-1] if url.startswith("<") and url.endswith(">") else url
        return resolve(bare.strip())

    def inline(m: re.Match[str]) -> str:
        new = swap(m.group("url"))
        if new is None:
            return m.group(0)
        return f"]({new}{m.group('title') or ''})"

    def refdef(m: re.Match[str]) -> str:
        new = swap(m.group("url"))
        return m.group(0) if new is None else f"{m.group('lead')}{new}"

    def attr(m: re.Match[str]) -> str:
        new = swap(m.group("url"))
        if new is None:
            return m.group(0)
        return f"{m.group('lead')}{m.group('q')}{new}{m.group('q')}"

    out: list[str] = []
    for is_code, text in _split_fenced(markdown):
        if is_code:
            out.append(text)
            continue
        spans: list[str] = []

        def protect(m: re.Match[str]) -> str:
            spans.append(m.group(0))
            return _PLACEHOLDER.format(len(spans) - 1)

        text = _CODE_SPAN.sub(protect, text)
        text = _INLINE_TARGET.sub(inline, text)
        text = _REF_DEF.sub(refdef, text)
        text = _HTML_ATTR.sub(attr, text)
        for i, span in enumerate(spans):
            text = text.replace(_PLACEHOLDER.format(i), span, 1)
        out.append(text)
    return "".join(out)


def mark_external_links(html: str, site_host: str | None, css_class: str = "huv-external") -> str:
    """Add ``css_class`` to every ``<a href="http(s)://…">`` whose host is not ``site_host``."""

    def tag(m: re.Match[str]) -> str:
        text = m.group(0)
        href = _HREF_ATTR.search(text)
        if not href or urlsplit(href.group(2)).netloc.lower() == (site_host or "").lower():
            return text
        cls = _CLASS_ATTR.search(text)
        if cls:
            if css_class in cls.group(2).split():
                return text
            start, end = cls.span(2)
            return f"{text[:start]}{cls.group(2)} {css_class}{text[end:]}"
        return f'{text[:2]} class="{css_class}"{text[2:]}'

    return _ANCHOR_TAG.sub(tag, html)


def on_page_markdown(markdown: str, page: Page, config: MkDocsConfig, files: Files) -> str:
    repo_url = config.get("repo_url")
    if not repo_url:
        return markdown
    docs_dir = Path(config["docs_dir"]).resolve()
    repo_root = Path(config.config_file_path).resolve().parent
    branch = branch_from_edit_uri(config.get("edit_uri"))

    def is_published(docs_path: str) -> bool:
        file = files.get_file_from_path(docs_path)
        return file is not None and not file.inclusion.is_excluded()

    def resolve(url: str) -> str | None:
        new = repo_url_for(
            url,
            page_src_uri=page.file.src_uri,
            docs_dir=docs_dir,
            repo_root=repo_root,
            repo_url=repo_url,
            branch=branch,
            is_published=is_published,
        )
        if new is not None:
            log.debug("%s: %s -> %s", page.file.src_uri, url, new)
        return new

    return rewrite_markdown_links(markdown, resolve)


def on_page_content(html: str, page: Page, config: MkDocsConfig, files: Files) -> str:
    site_url = config.get("site_url") or ""
    return mark_external_links(html, urlsplit(site_url).netloc or None)
