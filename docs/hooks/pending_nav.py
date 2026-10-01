"""MkDocs hook: let a partial checkout build while pages are still landing.

The site is assembled from several branches written in parallel. mkdocs.yml lists every
page the integrated site will have, but a single branch contains only some of them. By
default this hook drops nav entries whose page is not in the checkout (and any section
left empty), and logs what it dropped at INFO level, so a partial tree still passes
``mkdocs build --strict``.

Set ``HUV_DOCS_STRICT_NAV=1`` - or build in CI, where ``CI=true`` is set - to turn the
pruning off. A missing page then produces MkDocs' own "not found" warning and fails a
--strict build, which is what the Pages workflow relies on. ``HUV_DOCS_STRICT_NAV=0``
forces pruning even in CI.
"""

from __future__ import annotations

import logging
import os
import re
from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Callable

if TYPE_CHECKING:  # pragma: no cover - typing only
    from mkdocs.config.defaults import MkDocsConfig
    from mkdocs.structure.files import Files

log = logging.getLogger("mkdocs.hooks.pending_nav")

_TRUE = {"1", "true", "yes", "on"}
_FALSE = {"0", "false", "no", "off"}
_EXTERNAL = re.compile(r"^(?:[A-Za-z][A-Za-z0-9+.-]*:|//|/)")


def strict_mode(environ: Mapping[str, str] = os.environ) -> bool:
    """True when missing nav pages must fail the build."""
    explicit = environ.get("HUV_DOCS_STRICT_NAV")
    if explicit is not None and explicit.strip().lower() in _TRUE | _FALSE:
        return explicit.strip().lower() in _TRUE
    return environ.get("CI", "").strip().lower() in _TRUE


def prune_nav(nav: Any, exists: Callable[[str], bool]) -> tuple[Any, list[str]]:
    """Return ``nav`` without entries whose page is missing, plus the dropped paths.

    ``nav`` has the shape of the ``nav`` key in mkdocs.yml: a list whose items are page
    paths, ``{title: path}`` mappings or ``{title: [children]}`` sections. External links
    and absolute paths are always kept; sections that end up empty are removed.
    """
    dropped: list[str] = []

    def keep(path: str) -> bool:
        if _EXTERNAL.match(path) or exists(path):
            return True
        dropped.append(path)
        return False

    def walk(items: list[Any]) -> list[Any]:
        result: list[Any] = []
        for item in items:
            if isinstance(item, str):
                if keep(item):
                    result.append(item)
            elif isinstance(item, dict):
                entry: dict[str, Any] = {}
                for title, value in item.items():
                    if isinstance(value, str):
                        if keep(value):
                            entry[title] = value
                    elif isinstance(value, list):
                        children = walk(value)
                        if children:
                            entry[title] = children
                    else:
                        entry[title] = value
                if entry:
                    result.append(entry)
            else:
                result.append(item)
        return result

    if not isinstance(nav, list):
        return nav, dropped
    return walk(nav), dropped


def on_files(files: Files, config: MkDocsConfig) -> Files:
    nav = config["nav"]
    if not nav or strict_mode():
        return files
    pruned, dropped = prune_nav(nav, lambda path: files.get_file_from_path(path) is not None)
    if dropped:
        config["nav"] = pruned
        log.info(
            "pending_nav: skipped %d nav entr%s for pages not in this checkout: %s "
            "(set HUV_DOCS_STRICT_NAV=1 to make missing pages an error, as CI does)",
            len(dropped),
            "y" if len(dropped) == 1 else "ies",
            ", ".join(dropped),
        )
    return files
