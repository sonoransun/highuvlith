"""Tests for repo_links.py. Run: python -m pytest docs/hooks -q"""

from __future__ import annotations

from pathlib import Path

import pytest

from repo_links import (
    branch_from_edit_uri,
    mark_external_links,
    repo_url_for,
    rewrite_markdown_links,
)

REPO_URL = "https://github.com/example/highuvlith"


@pytest.fixture()
def repo(tmp_path: Path) -> Path:
    """A miniature checkout: docs/ plus a few source files and folders outside it."""
    for rel in (
        "README.md",
        "crates/highuvlith-core/src/aerial.rs",
        "crates/highuvlith-core/src/optics/mod.rs",
        "examples/sim.toml",
        "examples/figure.png",
        "docs/index.md",
        "docs/pipeline.md",
        "docs/processes/index.md",
        "docs/processes/grayscale.md",
        "docs/figures/history/make_timeline.py",
        "docs/assets/images/history/timeline.svg",
    ):
        path = tmp_path / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("x", encoding="utf-8")
    return tmp_path


def resolver(repo: Path, page: str, published=None):
    published = published or {
        "index.md",
        "pipeline.md",
        "processes/index.md",
        "processes/grayscale.md",
        "assets/images/history/timeline.svg",
    }

    def resolve(url: str):
        return repo_url_for(
            url,
            page_src_uri=page,
            docs_dir=repo / "docs",
            repo_root=repo,
            repo_url=REPO_URL,
            branch="main",
            is_published=lambda docs_path: docs_path in published,
        )

    return resolve


def test_source_file_becomes_blob_url(repo: Path) -> None:
    resolve = resolver(repo, "pipeline.md")
    assert resolve("../crates/highuvlith-core/src/aerial.rs") == (
        f"{REPO_URL}/blob/main/crates/highuvlith-core/src/aerial.rs"
    )


def test_directory_becomes_tree_url(repo: Path) -> None:
    resolve = resolver(repo, "pipeline.md")
    assert resolve("../crates/highuvlith-core") == f"{REPO_URL}/tree/main/crates/highuvlith-core"
    assert resolve("../examples/") == f"{REPO_URL}/tree/main/examples"
    assert resolve("../") == f"{REPO_URL}/tree/main"


def test_nested_page_climbs_two_levels(repo: Path) -> None:
    resolve = resolver(repo, "processes/grayscale.md")
    assert resolve("../../crates/highuvlith-core/src/optics/mod.rs") == (
        f"{REPO_URL}/blob/main/crates/highuvlith-core/src/optics/mod.rs"
    )


def test_anchor_and_query_are_kept(repo: Path) -> None:
    resolve = resolver(repo, "index.md")
    assert resolve("../README.md#installation") == f"{REPO_URL}/blob/main/README.md#installation"
    assert resolve("../crates/highuvlith-core/src/aerial.rs?plain=1#L10-L20") == (
        f"{REPO_URL}/blob/main/crates/highuvlith-core/src/aerial.rs?plain=1#L10-L20"
    )


def test_images_use_raw_urls(repo: Path) -> None:
    resolve = resolver(repo, "index.md")
    assert resolve("../examples/figure.png") == f"{REPO_URL}/raw/main/examples/figure.png"


def test_links_inside_the_site_are_left_alone(repo: Path) -> None:
    resolve = resolver(repo, "processes/grayscale.md")
    assert resolve("../pipeline.md") is None
    assert resolve("index.md#model-status") is None
    assert resolve("../assets/images/history/timeline.svg") is None
    assert resolve("../processes/") is None  # directory URL of a section index page


def test_excluded_docs_files_go_to_github(repo: Path) -> None:
    resolve = resolver(repo, "index.md")
    assert resolve("figures/history/make_timeline.py") == (
        f"{REPO_URL}/blob/main/docs/figures/history/make_timeline.py"
    )


def test_external_absolute_and_anchor_links_are_ignored(repo: Path) -> None:
    resolve = resolver(repo, "index.md")
    for url in (
        "https://example.org/a.md",
        "mailto:someone@example.org",
        "//cdn.example.org/x.js",
        "/absolute/path.md",
        "#section",
        "",
    ):
        assert resolve(url) is None, url


def test_missing_targets_are_left_for_mkdocs_to_report(repo: Path) -> None:
    resolve = resolver(repo, "pipeline.md")
    assert resolve("../crates/highuvlith-core/src/typo.rs") is None
    assert resolve("missing-page.md") is None
    assert resolve("../../outside-the-repo.md") is None


def test_percent_encoded_paths(repo: Path) -> None:
    (repo / "examples" / "with space.toml").write_text("x", encoding="utf-8")
    resolve = resolver(repo, "index.md")
    assert resolve("../examples/with%20space.toml") == (
        f"{REPO_URL}/blob/main/examples/with%20space.toml"
    )


def test_rewrite_markdown_links_covers_link_forms(repo: Path) -> None:
    resolve = resolver(repo, "pipeline.md")
    md = (
        "See [aerial.rs](../crates/highuvlith-core/src/aerial.rs \"source\") and\n"
        "![fig](../examples/figure.png){ width=400 } plus [local](index.md).\n"
        "[![badge](../examples/figure.png)](../README.md)\n"
        "<a href=\"../examples/sim.toml\">sim</a>\n"
        "\n"
        "[ref]: ../examples/sim.toml 'Example config'\n"
        "[angle](<../README.md>)\n"
    )
    out = rewrite_markdown_links(md, resolve)
    assert f"[aerial.rs]({REPO_URL}/blob/main/crates/highuvlith-core/src/aerial.rs \"source\")" in out
    assert f"![fig]({REPO_URL}/raw/main/examples/figure.png){{ width=400 }}" in out
    assert "[local](index.md)" in out
    assert f"[![badge]({REPO_URL}/raw/main/examples/figure.png)]({REPO_URL}/blob/main/README.md)" in out
    assert f'<a href="{REPO_URL}/blob/main/examples/sim.toml">' in out
    assert f"[ref]: {REPO_URL}/blob/main/examples/sim.toml 'Example config'" in out
    assert f"[angle]({REPO_URL}/blob/main/README.md)" in out


def test_code_is_never_rewritten(repo: Path) -> None:
    resolve = resolver(repo, "pipeline.md")
    md = (
        "Inline `[x](../README.md)` stays.\n"
        "```markdown\n"
        "[x](../README.md)\n"
        "```\n"
        "    ~~~~\n"
        "    [y](../README.md)\n"
        "    ~~~~\n"
        "After the fences [z](../README.md) is rewritten.\n"
    )
    out = rewrite_markdown_links(md, resolve)
    assert out.count("](../README.md)") == 3
    assert f"[z]({REPO_URL}/blob/main/README.md)" in out


def test_stray_backtick_does_not_swallow_the_next_paragraph(repo: Path) -> None:
    resolve = resolver(repo, "pipeline.md")
    md = "A lone ` backtick here.\n\nThen [src](../README.md) and a `span`.\n"
    out = rewrite_markdown_links(md, resolve)
    assert f"[src]({REPO_URL}/blob/main/README.md)" in out
    wrapped = "Code `spanning\n[x](../README.md)` two lines stays.\n"
    assert rewrite_markdown_links(wrapped, resolve) == wrapped


def test_unclosed_fence_protects_the_rest(repo: Path) -> None:
    resolve = resolver(repo, "pipeline.md")
    md = "```\n[x](../README.md)\n"
    assert rewrite_markdown_links(md, resolve) == md


@pytest.mark.parametrize(
    ("edit_uri", "branch"),
    [
        ("edit/main/docs/", "main"),
        ("blob/develop/docs/", "develop"),
        ("https://github.com/o/r/edit/trunk/docs/", "trunk"),
        ("", "main"),
        (None, "main"),
    ],
)
def test_branch_from_edit_uri(edit_uri, branch) -> None:
    assert branch_from_edit_uri(edit_uri) == branch


def test_mark_external_links() -> None:
    html = (
        '<a href="https://github.com/o/r/blob/main/x.rs">x</a>'
        '<a href="https://site.example.io/highuvlith/pipeline/">internal absolute</a>'
        '<a href="../pipeline/">relative</a>'
        '<a class="headerlink" href="#top">¶</a>'
        '<a class="md-button" href="http://example.org" title="t">b</a>'
        "<a href='https://example.org/a'>single quotes</a>"
    )
    out = mark_external_links(html, "site.example.io")
    assert '<a class="huv-external" href="https://github.com/o/r/blob/main/x.rs">' in out
    assert '<a href="https://site.example.io/highuvlith/pipeline/">' in out
    assert '<a href="../pipeline/">' in out
    assert '<a class="headerlink" href="#top">' in out
    assert '<a class="md-button huv-external" href="http://example.org" title="t">' in out
    assert "<a class=\"huv-external\" href='https://example.org/a'>" in out
    assert mark_external_links(out, "site.example.io") == out  # idempotent
