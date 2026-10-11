"""Every ``md_embed`` form emits paths relative to the embedding page's source file (DESIGN §4.6).

MkDocs rewrites only Markdown images; a raw ``<img>``/``<source>`` path (forms 2 and 3 and
their ``srcset`` / ``data-huv-still`` values) reaches github.com and lychee verbatim and
is resolved relative to the embedding file, so the registry writes ``assets/images/…``
from a root page, ``../assets/images/…`` from a nested page or a section index and
``docs/assets/images/…`` from the README; ``docs/hooks/page_kit.py`` re-roots the raw-HTML
paths for the site's directory URLs. These tests pin that convention (verified correct on
the branch; no writer change) with the round-trip invariant: resolving an emitted path
against the page's source directory lands exactly on a file of the spec.

Stdlib only (``registry.py`` imports nothing else); the last test, over the real registries,
skips without matplotlib like the other figure tests.
"""

from __future__ import annotations

import posixpath
import re
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
for sub in ("docs/figures", "docs/figures/reference", "docs/figures/sim"):
    p = str(ROOT / sub)
    if p not in sys.path:
        sys.path.insert(0, p)

import registry  # noqa: E402

ALT = "A fixture image showing one number, 42."
CAPTION = "<strong>Fixture.</strong> One detail. <em>Schematic, not to scale.</em>"

# (page argument, expected prefix of every asset path): a root page, a nested page, a
# section index, a deeper page (none exists today; proves the rule is general) and README.
PAGES = [
    ("optics.md", ""),
    ("history/euv.md", "../"),
    ("history/index.md", "../"),
    ("deep/er/page.md", "../../"),
    ("README.md", "docs/"),
]
DOCS_PAGES = [p for p in PAGES if p[0] != "README.md"]


def page_dir(page: str) -> str:
    """Repo-relative directory of the page's source file ('' for the README)."""
    return "" if page == "README.md" else posixpath.dirname(f"docs/{page}")


def resolve(page: str, url: str) -> str:
    """The repo-relative file ``url`` points at when resolved from ``page``'s source file."""
    clean = url.split("#", 1)[0].split("?", 1)[0]
    d = page_dir(page)
    return posixpath.normpath(posixpath.join(d, clean) if d else clean)


def _spec(name: str, **kw) -> registry.AssetSpec:
    base = dict(kind="figure", alt=ALT, caption=CAPTION, pages=("optics.md",), fmt="png", subdir="sim")
    base.update(kw)
    return registry.AssetSpec(name=name, **base)


FORMS = {
    # 1. static figure (Markdown images)
    "static": _spec("fx-static"),
    # 2. strip with a narrow twin (raw <picture> + srcset)
    "strip": _spec("fx-strip", kind="strip", fmt="svg", subdir="diagrams", narrow=True),
    # 3. animated raster (APNG; reduced-motion <source>, data-huv-still, GIF for the README)
    "apng": _spec("fx-focus", kind="anim", fmt="apng", subdir="sim/anim", extra={"gif": True}),
    # 4a. animated SVG, not strip-shaped (form 1 with -anim-/-still- names)
    "anim-svg": _spec("fx-wave", kind="anim", fmt="svg", subdir="diagrams", animated="svg"),
    # 4b. animated SVG, strip-shaped (form 2) — a spec *named* …-anim
    "anim-strip": _spec("fx-hero-anim", kind="strip", fmt="svg", subdir="diagrams", animated="svg", narrow=True),
    # a capture: one file, no theme pair
    "capture": _spec("fx-shot", kind="capture", fmt="png", subdir="gui"),
    # a static figure with a cache-bust query
    "bust": _spec("fx-bust", extra={"cache_bust": 7}),
}

_ATTR = re.compile(r"""\b(src|srcset|data-huv-still)="([^"]*)\"""")
_MD_IMG = re.compile(r"!\[[^\]]*\]\(([^)\s]+)\)")


def emitted_paths(text: str) -> list[str]:
    """Every path the embed text carries: attributes (srcset split per candidate) and Markdown images."""
    out: list[str] = []
    for name, value in _ATTR.findall(text):
        if name == "srcset":
            out += [c.strip().split(" ")[0] for c in value.split(",") if c.strip()]
        else:
            out.append(value)
    out += _MD_IMG.findall(text)
    assert out, text
    return out


@pytest.mark.parametrize("form", list(FORMS))
@pytest.mark.parametrize("page,prefix", PAGES)
def test_every_path_is_relative_to_the_page_source_file(form, page, prefix):
    spec = FORMS[form]
    if page == "README.md" and form == "capture":
        pytest.skip("captures are docs-page assets")
    text = registry.md_embed(spec, page)
    files = {registry.rel_to_repo(f) for f in registry.all_files(spec).values()}
    for url in emitted_paths(text):
        assert not url.startswith("/") and "://" not in url, url
        assert url.startswith(prefix + "assets/images/" if page != "README.md" else "docs/assets/images/"), (page, url)
        assert resolve(page, url) in files, (page, url, resolve(page, url), sorted(files))


@pytest.mark.parametrize("page,prefix", DOCS_PAGES)
def test_form_2_src_and_srcset_follow_the_page(page, prefix):
    text = registry.md_embed(FORMS["strip"], page)
    for theme in registry.THEMES:
        assert (
            f'<picture><source media="(max-width: 600px)" srcset="{prefix}assets/images/diagrams/fx-strip-narrow-{theme}.svg">'
            f'<img class="huv-no-zoom" src="{prefix}assets/images/diagrams/fx-strip-{theme}.svg#gh-{theme}-mode-only" alt="{ALT}"></picture>'
        ) in text


@pytest.mark.parametrize("page,prefix", DOCS_PAGES)
def test_form_3_src_srcset_and_still_follow_the_page(page, prefix):
    text = registry.md_embed(FORMS["apng"], page)
    for theme in registry.THEMES:
        still = f"{prefix}assets/images/sim/anim/fx-focus-still-{theme}.png"
        assert (
            f'<picture><source media="(prefers-reduced-motion: reduce)" srcset="{still}">'
            f'<img class="huv-no-zoom huv-anim" src="{prefix}assets/images/sim/anim/fx-focus-anim-{theme}.png#gh-{theme}-mode-only" '
            f'alt="{ALT}" data-huv-still="{still}"></picture>'
        ) in text


@pytest.mark.parametrize("page,prefix", DOCS_PAGES)
def test_form_1_and_4_markdown_images_follow_the_page(page, prefix):
    text = registry.md_embed(FORMS["static"], page)
    for theme in registry.THEMES:
        assert f"![{ALT}]({prefix}assets/images/sim/fx-static-{theme}.png#gh-{theme}-mode-only)" in text
    text = registry.md_embed(FORMS["anim-svg"], page)
    for theme in registry.THEMES:
        assert f"![{ALT}]({prefix}assets/images/diagrams/fx-wave-anim-{theme}.svg#gh-{theme}-mode-only)" in text
    text = registry.md_embed(FORMS["anim-strip"], page)
    assert f'srcset="{prefix}assets/images/diagrams/fx-hero-narrow-light.svg"' in text
    assert f'src="{prefix}assets/images/diagrams/fx-hero-anim-light.svg#gh-light-mode-only"' in text


def test_form_5_readme_paths_are_repo_relative():
    text = registry.md_embed(FORMS["apng"], "README.md")
    assert text.splitlines() == [
        "<picture>",
        '  <source media="(prefers-reduced-motion: reduce) and (prefers-color-scheme: dark)" srcset="docs/assets/images/sim/anim/fx-focus-still-dark.png">',
        '  <source media="(prefers-reduced-motion: reduce)" srcset="docs/assets/images/sim/anim/fx-focus-still-light.png">',
        '  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/images/sim/anim/fx-focus-anim-dark.gif">',
        f'  <img alt="{ALT}" width="800" src="docs/assets/images/sim/anim/fx-focus-anim-light.gif">',
        "</picture>",
    ]


def test_cache_bust_sits_between_path_and_fragment():
    text = registry.md_embed(FORMS["bust"], "history/euv.md")
    assert "](../assets/images/sim/fx-bust-light.png?v=7#gh-light-mode-only)" in text


def test_catalog_text_differs_per_page_depth():
    spec = _spec("fx-static", pages=("optics.md", "history/euv.md"))
    text = registry.catalog([spec])
    assert "<!-- fx-static -> optics.md -->" in text and "<!-- fx-static -> history/euv.md -->" in text
    assert "](assets/images/sim/fx-static-light.png#gh-light-mode-only)" in text
    assert "](../assets/images/sim/fx-static-light.png#gh-light-mode-only)" in text
    # the catalog's placeholder page for a spec with no pages yet behaves as a root page
    assert "](assets/images/sim/fx-static-light.png#gh-light-mode-only)" in registry.catalog([_spec("fx-static", pages=())])


def test_real_registries_round_trip_from_every_listed_page():
    """Every registered spec (both registries), every ``pages=`` entry, every emitted path."""
    pytest.importorskip("matplotlib")
    check_figures = pytest.importorskip("check_figures")
    specs = check_figures.load_all(verbose=False)
    assert specs
    seen = 0
    for spec in specs:
        files = {registry.rel_to_repo(f) for f in registry.all_files(spec).values()}
        for page in spec.pages:
            for url in emitted_paths(registry.md_embed(spec, page)):
                assert resolve(page, url) in files, (spec.name, page, url)
                seen += 1
    assert seen >= 94  # 47 baseline specs × 2 themes at least
