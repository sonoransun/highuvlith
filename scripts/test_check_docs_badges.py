"""Tests for scripts/check_docs_badges.py (run: python -m pytest scripts -q).

Fixtures prove the four always-on rules and the page-form rules in both modes, the CLI
(modes, --pages, exit codes, the allowlist file), that the lint's output acceptance is the
runner's own ``find_output``, and that the committed mermaid allowlist and the documentation
tree hold what DESIGN §9.2 (G0c) requires.
"""

from __future__ import annotations

import importlib.util
import sys
from collections import Counter
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_docs_badges as cdb  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]

# --- fixture pages ----------------------------------------------------------------------------

SITE_PAGE = """---
description: A fixture page in the DESIGN §3 form.
---
# Thin film

**Status:** ✅ Implemented — exact 2×2 characteristic-matrix stack optics (🔶 oblique TM in-film intensity) · [matrix](capability-matrix.md#materials-and-film-optics)

<p class="huv-lead">What the page covers, who it is for, what it is not.</p>

<div class="huv-physics" markdown>

<figure markdown="span">

![Standing waves through 150 nm of resist on silicon.](assets/images/sim/thinfilm-light.png#gh-light-mode-only)
![Standing waves through 150 nm of resist on silicon.](assets/images/sim/thinfilm-dark.png#gh-dark-mode-only)

<figcaption><strong>In the engine.</strong> Standing waves at 157.63 nm. <em>Model: thin-film transfer matrix ✅ · volumetric exposure 🔶 (separable I·S(z)).</em></figcaption>
</figure>

| What is modeled | Grade | Dominant approximation | Details |
|---|---|---|---|
| Stack reflectance | ✅ Implemented | plane waves | [below](#usage) |
| In-film intensity | ✅ (🔶 oblique TM) | documented approximation | [below](#usage) |

**Same thing elsewhere:** Python `FilmStackConfig` · CLI `deep` · TOML `[film]` · GUI Film panel · physics → [pipeline](pipeline.md)

</div>

## How it works

The stack is solved exactly ✅ with the transfer matrix. A separate caveat sentence names one 🔶 approximation.

```math
R = |r|^2
```

## Usage

### Python

<!-- verify-example -->
```python
import highuvlith as huv
print(f"{huv.FilmStackConfig().reflectance(157.63):.4f}")
```

???+ success "Output"

    ```text
    0.2747
    ```

<details markdown="1">
<summary>Validation — 3 tests</summary>

| Claim | Test | Tolerance |
|---|---|---|
| R at normal incidence | `test_reflectance` | 1e-9 |

<!-- verify-example -->
```python
print(1 + 1)
```

???+ success "Output"

    ```text
    2
    ```

</details>

<details markdown="1">
<summary>Flowchart</summary>

```mermaid
flowchart LR
    A --> B
```

</details>

**Read next:** [Resist models](processes/resist-models.md) · [matrix rows](capability-matrix.md#materials-and-film-optics)
"""

README_PAGE = """# highuvlith

<details>
<summary>2. Immersion</summary>

<!-- verify-example -->
```python
print("Contrast: 0.454")
```

```text
Contrast: 0.454
```

</details>

```mermaid
flowchart LR
    code --> matrix
```
"""


def findings(text: str, page: str = "docs/page.md", strict: bool = False, allowlist=None):
    return cdb.check_text(text, page, allowlist, strict).findings


def rules(text: str, **kw) -> list[tuple[int, str, str]]:
    return [(f.line, f.level, f.rule) for f in findings(text, **kw)]


# --- the §3 form passes strict; the baseline forms do not -----------------------------------


def test_design_form_page_is_clean_in_strict_mode():
    assert findings(SITE_PAGE, "docs/processes/thin-film.md", strict=True) == []


def test_readme_form_is_clean_with_its_allowlist_line():
    assert findings(README_PAGE, "README.md", strict=True, allowlist={"README.md": 1}) == []


def test_baseline_style_page_fails_strict_and_passes_tolerant():
    src = (
        "# Optics ✅\n\n"
        "**Status:** ✅ Implemented — refractive optics ✅/🔶 with caveats.\n\n"
        "![alt](img-light.svg#only-light){ width=100% }\n\n"
        "Scalar ✅ and vector ✅ paths exist.\n"
    )
    tol = cdb.check_text(src, "docs/optics.md", None, strict=False)
    assert tol.errors == [] and len(tol.warnings) >= 5
    assert {f.rule for f in tol.warnings} == {
        "heading-glyph", "status-form", "split-grade", "only-fragment", "attr-list", "glyphs-per-sentence",
    }
    strict = cdb.check_text(src, "docs/optics.md", None, strict=True)
    assert strict.warnings == [] and len(strict.errors) == len(tol.warnings)


# --- always-on rule 1: tabs -------------------------------------------------------------------


def test_tabs_are_errors_in_every_mode():
    src = '## Usage\n\n=== "Python"\n\n    code\n\n===! "CLI"\n\n    more\n'
    for strict in (False, True):
        found = [(f.line, f.level) for f in findings(src, strict=strict) if f.rule == "tabs"]
        assert found == [(3, "error"), (7, "error")]


def test_tabs_inside_a_code_fence_are_not_counted():
    src = '````markdown\n=== "Python"\n````\n'
    assert rules(src) == []


# --- always-on rule 2: mermaid allowlist -----------------------------------------------------

MERMAID = "# P\n\nText.\n\n```mermaid\nflowchart LR\n    a --> b\n```\n"


def test_mermaid_outside_details_needs_an_allowlist_line():
    found = findings(MERMAID, "docs/history/origins.md")
    assert [(f.line, f.level, f.rule) for f in found] == [(5, "error", "mermaid-allowlist")]
    assert "mermaid_allowlist.txt" in found[0].message and "docs/history/origins.md: loop" in found[0].message
    assert findings(MERMAID, "docs/history/origins.md", allowlist={"docs/history/origins.md": 1}) == []


def test_mermaid_inside_details_needs_no_line():
    src = '<details markdown="1">\n<summary>Flowchart</summary>\n\n' + MERMAID.split("\n\n", 2)[2] + "\n</details>\n"
    assert findings(src, "docs/pipeline.md") == []
    # a fence inside a bare <details> on README/CONTRIBUTING is also a fallback
    src_readme = "<details>\n<summary>Flowchart</summary>\n\n```mermaid\nflowchart LR\n  a --> b\n```\n\n</details>\n"
    assert findings(src_readme, "CONTRIBUTING.md") == []


def test_mermaid_inside_an_outer_fence_is_not_a_fence():
    src = "````markdown\n```mermaid\nflowchart LR\n```\n````\n"
    assert findings(src, "docs/extending.md") == []


def test_strict_mode_counts_fences_against_allowlist_lines():
    two = MERMAID + "\n" + MERMAID.split("\n\n", 2)[2]
    assert findings(two, "docs/processes/volumetric-exposure.md", allowlist={"docs/processes/volumetric-exposure.md": 1}) == []
    strict = findings(two, "docs/processes/volumetric-exposure.md", strict=True, allowlist={"docs/processes/volumetric-exposure.md": 1})
    assert [(f.line, f.rule) for f in strict] == [(10, "mermaid-allowlist")]  # the second fence opens on line 10
    assert "permits 1" in strict[0].message
    assert findings(two, "docs/processes/volumetric-exposure.md", strict=True, allowlist={"docs/processes/volumetric-exposure.md": 2}) == []


def test_allowlist_file_parsing(tmp_path):
    f = tmp_path / "allow.txt"
    f.write_text("# comment\n\ndocs/a.md: loop\ndocs/a.md: legacy  # trailing comment\nREADME.md: decision-tree\n")
    assert cdb.load_allowlist(f) == Counter({"docs/a.md": 2, "README.md": 1})
    f.write_text("docs/a.md: flowchart\n")
    with pytest.raises(ValueError):
        cdb.load_allowlist(f)
    assert cdb.load_allowlist(tmp_path / "missing.txt") == Counter()


# --- always-on rule 3: <details markdown="1"> form under docs/ -------------------------------

DETAILS_OK = '<details markdown="1">\n<summary>Validation — 2 tests</summary>\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n</details>\n'


def test_details_in_the_design_form_is_clean():
    assert findings(DETAILS_OK, "docs/pipeline.md", strict=True) == []


def test_bare_details_under_docs_is_an_error():
    src = DETAILS_OK.replace('<details markdown="1">', "<details>")
    found = findings(src, "docs/pipeline.md")
    assert [(f.line, f.level, f.rule) for f in found] == [(1, "error", "details-form")]
    assert 'markdown="1"' in found[0].message
    # an attribute-bearing tag without the markdown attribute is still bare
    assert [f.rule for f in findings(src.replace("<details>", '<details open class="x">'), "docs/pipeline.md")] == ["details-form"]


def test_summary_without_a_blank_line_after_it_is_an_error():
    src = DETAILS_OK.replace("</summary>\n\n", "</summary>\n")
    found = findings(src, "docs/pipeline.md")
    assert [(f.line, f.level, f.rule) for f in found] == [(2, "error", "details-form")]
    src2 = DETAILS_OK.replace("</summary>\n\n", "</summary> text\n\n")
    assert [(f.line, f.rule) for f in findings(src2, "docs/pipeline.md")] == [(2, "details-form")]


def test_bare_details_is_allowed_on_github_only_files():
    src = DETAILS_OK.replace('<details markdown="1">', "<details>")
    assert findings(src, "README.md") == []
    assert findings(src, "CONTRIBUTING.md") == []
    assert findings(src, "docs/figures/README.md") == []
    # the blank line after </summary> is still wanted on GitHub (a warning there)
    bad = src.replace("</summary>\n\n", "</summary>\n")
    assert [(f.level, f.rule) for f in findings(bad, "README.md")] == [("warning", "details-layout")]


def test_details_layout_warnings():
    one_line = '<details markdown="1"><summary>X</summary>\n\ntext\n\n</details>\n'
    assert [(f.level, f.rule) for f in findings(one_line, "docs/a.md")] == [("warning", "details-layout")]
    no_blank_before_close = DETAILS_OK.replace("| 1 | 2 |\n\n</details>", "| 1 | 2 |\n</details>")
    assert [(f.line, f.rule) for f in findings(no_blank_before_close, "docs/a.md")] == [(7, "details-layout")]
    assert [f.level for f in findings(one_line, "docs/a.md", strict=True)] == ["error"]


def test_details_tags_inside_code_are_ignored():
    src = "```html\n<details>\n<summary>x</summary>\n</details>\n```\n\nUse `<details>` sparingly.\n"
    assert findings(src, "docs/a.md") == []


# --- always-on rule 4: silently unchecked verify-example outputs -----------------------------

FENCE = "<!-- verify-example -->\n```python\nprint(2)\n```\n"
ADMONITION = '???+ success "Output"\n\n    ```text\n    2\n    ```\n'


def test_output_directly_after_the_fence_is_accepted():
    for page in ("docs/a.md", "README.md"):
        assert [f.rule for f in findings(FENCE + "\n" + ADMONITION, page) if f.rule == "unchecked-output"] == []
    assert findings(FENCE + "\n```text\n2\n```\n", "README.md") == []


def test_output_two_lines_below_with_a_sentence_between_is_an_error():
    src = FENCE + "\nThe output is:\n\n" + ADMONITION
    found = findings(src, "docs/a.md")
    assert [(f.line, f.level, f.rule) for f in found] == [(8, "error", "unchecked-output")]  # the admonition line
    assert "silently unchecked" in found[0].message
    # the same block in strict and tolerant mode, and on the README
    assert [f.level for f in findings(src, "docs/a.md", strict=True)] == ["error"]
    assert [f.rule for f in findings(src, "README.md")] == ["unchecked-output"]


def test_misindented_admonition_is_an_error():
    src = FENCE + "\n" + ADMONITION.replace('???+', '    ???+', 1)
    assert [(f.rule, f.level) for f in findings(src, "docs/a.md")] == [("unchecked-output", "error")]


def test_leadin_not_directly_after_the_fence_is_an_error():
    src = FENCE + "\nSome prose.\n\nOutput:\n\n```text\n2\n```\n"
    assert [(f.line, f.rule) for f in findings(src, "docs/a.md")] == [(8, "unchecked-output")]  # the `Output:` line


def test_fence_followed_by_prose_only_is_tolerated():
    src = FENCE + "\nThe call returns a `Result`.\n"
    assert findings(src, "docs/a.md") == []  # not a technical page: nothing at all
    tech = "**Status:** ✅ Implemented — x · [matrix](capability-matrix.md#core-imaging)\n\n" + src
    assert [(f.level, f.rule) for f in findings(tech, "docs/a.md")] == [("warning", "output-missing")]
    assert [f.level for f in findings(tech, "docs/a.md", strict=True)] == ["error"]


def test_output_leadin_is_a_warning():
    src = FENCE + "\nOutput:\n\n```text\n2\n```\n"
    assert [(f.line, f.level, f.rule) for f in findings(src, "docs/python-api.md")] == [(6, "warning", "output-leadin")]
    assert "???+" in findings(src, "docs/python-api.md")[0].message
    assert [(f.level, f.rule) for f in findings(src, "README.md")] == [("warning", "output-leadin")]
    assert [f.level for f in findings(src, "README.md", strict=True)] == ["error"]


def test_output_block_beyond_the_window_is_not_flagged():
    src = FENCE + "\n" + "prose\n" * 12 + "\n" + ADMONITION
    assert [f.rule for f in findings(src, "docs/a.md")] == []


def test_skipped_and_unclassified_fences():
    skip = "<!-- example-skip: fragment -->\n```python\nx = 1\n```\n\n" + ADMONITION
    assert findings(skip, "docs/a.md") == []
    bare = "```python\nx = 1\n```\n"
    assert [(f.line, f.level, f.rule) for f in findings(bare, "docs/a.md")] == [(1, "warning", "example-marker")]


def test_example_rules_only_apply_to_runner_scanned_files():
    src = FENCE + "\nprose\n\n" + ADMONITION
    assert findings(src, "CONTRIBUTING.md") == []
    assert [f.rule for f in findings(src, "docs/figures/README.md")] == ["unchecked-output"]


def test_output_acceptance_equals_the_runners_find_output(tmp_path):
    """The lint mirrors tests/docs/test_site_examples.py::find_output line for line."""
    runner_path = ROOT / "tests" / "docs" / "test_site_examples.py"
    if not runner_path.exists():
        pytest.skip("runner not present")
    spec = importlib.util.spec_from_file_location("huv_site_examples", runner_path)
    runner = importlib.util.module_from_spec(spec)
    sys.modules["huv_site_examples"] = runner
    spec.loader.exec_module(runner)
    cases = [
        FENCE + "\n" + ADMONITION,
        FENCE + "\n" + ADMONITION.replace("???+", "???"),
        FENCE + "\n\n\n" + ADMONITION,
        FENCE + "\nOutput:\n\n```text\n2\n```\n",
        FENCE + "\nOutput (rounded):\n```text\n2\n```\n",
        FENCE + "\n```text\n2\n```\n",
        FENCE + "\nprose\n\n" + ADMONITION,
        FENCE + "\n" + ADMONITION.replace("???+", "    ???+", 1),
        FENCE + '\n???+ success "Output"\n\n```text\n2\n```\n',
        FENCE + "\n\n",
        "    " + FENCE.replace("\n", "\n    ").rstrip(" ") + "\n    " + ADMONITION.replace("\n", "\n    ").rstrip(" "),
        "<div class=\"huv-rosetta\" markdown>\n\n**Python**\n\n" + FENCE + "\n" + ADMONITION + "\n</div>\n",
    ]
    for n, text in enumerate(cases):
        page = tmp_path / f"case{n}.md"
        page.write_text(text, encoding="utf-8")
        lines = text.splitlines()
        theirs = [runner.find_output(lines, b) for b in runner.extract(page) if b.kind == "verify"]
        ours = [cdb.find_output(lines, b) for b in cdb.extract_python(lines) if b.kind == "verify"]
        assert len(theirs) == len(ours) == 1, text
        if theirs[0] is None:
            assert ours[0] is None, text
        else:
            assert ours[0] is not None and ours[0][1] == theirs[0].stop, text
            assert ours[0][0] <= theirs[0].start


# --- page-form rules --------------------------------------------------------------------------


def test_heading_glyphs_warn():
    src = "## Stage 2 — Optics ✅ (scalar pupil)\n\n### Vector imaging 🔶\n\n## Plain\n"
    assert rules(src) == [(1, "warning", "heading-glyph"), (3, "warning", "heading-glyph")]
    assert rules("## About `✅` markers\n") == []


def test_split_grades_warn_and_code_spans_are_ignored():
    assert rules("Graded ✅/🔶 today.\n") == [(1, "warning", "split-grade")]
    assert rules("Graded ✅ / 🔶 today.\n") == [(1, "warning", "split-grade")]
    assert rules("| a | ✅ · 🔶 |\n|---|---|\n") == [(1, "warning", "split-grade")]
    assert rules("| Capability | Status |\n|---|---|\n| X | ✅ (🔶 coherence bound) |\n") == []
    assert rules("The retired form `✅/🔶` is gone.\n") == []


def test_status_form_and_length():
    good = "**Status:** 🔶 Simplified (✅ lines and edges) — one clause · [matrix](../capability-matrix.md#light-sources)\n"
    assert rules(good) == []
    assert rules(good.replace("🔶 Simplified", "🔶 Implemented")) == [(1, "warning", "status-form")]
    assert rules(good.replace("🔶 Simplified", "✅ DUV/UV heritage sources")) == [(1, "warning", "status-form")]
    assert rules(good.replace(" · [matrix](../capability-matrix.md#light-sources)", "")) == [(1, "warning", "status-form")]
    long = "**Status:** ✅ Implemented — " + " ".join(["word"] * 41) + " · [matrix](capability-matrix.md#x)\n"
    assert rules(long) == [(1, "warning", "status-length")]
    exact = "**Status:** ✅ Implemented — " + " ".join(["word"] * 40) + " · [matrix](capability-matrix.md#x)\n"
    assert rules(exact) == []
    # a multi-line Status paragraph ending in the link is fine; only the first Status paragraph counts
    multi = "**Status:** ✅ Implemented — the clause continues\non the next line · [matrix](capability-matrix.md#x)\n\n**Status:** 🔶\n"
    assert rules(multi) == []


def test_attr_list_and_only_fragments_warn_on_site_pages_only():
    src = "![a](x-light.svg#only-light){ width=100% }\n[b](y.md){ .md-button }\n<img src='z.png#only-dark'>\n"
    found = rules(src, page="docs/a.md")
    assert found == [
        (1, "warning", "attr-list"), (1, "warning", "only-fragment"),
        (2, "warning", "attr-list"), (3, "warning", "only-fragment"),
    ]
    assert rules(src, page="README.md") == []
    assert rules("`{ .class }` in code\n", page="docs/a.md") == []


def test_figcaption_markdown_warns():
    assert rules("<figcaption>**Lead.** detail</figcaption>\n") == [(1, "warning", "figcaption-markdown")]
    assert rules("<figcaption>A *word* here</figcaption>\n") == [(1, "warning", "figcaption-markdown")]
    assert rules("<figcaption><strong>Lead.</strong> 2 * 3 = 6 <em>Model: X ✅.</em></figcaption>\n") == []


def test_trailer_wording_warns():
    assert rules("<figcaption>x <em>Model: Hopkins imaging ✅ scalar, thin mask.</em></figcaption>\n") == [(1, "warning", "trailer-wording")]
    ok = "<figcaption>x <em>Model: Hopkins imaging ✅ · EUV optics 🔶 (isotropic pupil ✅ assumed) · Talbot 🔶; thin mask — no resist.</em></figcaption>\n"
    assert rules(ok) == []


def test_bare_glyphs_per_sentence():
    assert rules("Scalar ✅ and vector ✅ paths exist. Masks 🔶 too.\n") == [(1, "warning", "glyphs-per-sentence")]
    assert rules("Scalar ✅ paths exist. Vector ✅ paths exist. Masks 🔶 too.\n") == []
    assert rules("Both ✅ Implemented and 🔶 Simplified rows (chips) are fine.\n") == []
    assert rules("| ✅ | 🔶 | 🧪 | x |\n|---|---|---|---|\n") == [(1, "warning", "split-grade")]
    assert rules("**Status:** ✅ Implemented — ranges from ✅ to 🧪 · [matrix](capability-matrix.md#x)\n") == []
    assert rules("- bullet with ✅ and ✅ twice.\n") == [(1, "warning", "glyphs-per-sentence")]


def test_collapsed_admonitions_other_than_output_warn_on_site_pages():
    src = '??? info "Data behind the chart"\n    text\n\n???+ success "Output"\n\n    ```text\n    1\n    ```\n'
    assert rules(src, page="docs/a.md") == [(1, "warning", "collapsed-admonition")]
    assert rules(src, page="README.md") == []
    assert rules('??? success "Output"\n', page="docs/a.md") == []


def test_front_matter_and_comments_are_skipped():
    src = "---\ndescription: ✅/🔶 in front matter\n---\n<!-- ✅/🔶 in a comment\n=== \"tab\" -->\n# T\n"
    assert rules(src) == []


# --- CLI ------------------------------------------------------------------------------------


def _tree(tmp_path: Path) -> Path:
    docs = tmp_path / "docs"
    docs.mkdir()
    (docs / "good.md").write_text(SITE_PAGE, encoding="utf-8")
    (docs / "bad.md").write_text("# T ✅\n\n<details>\n<summary>x</summary>\n\ntext\n\n</details>\n", encoding="utf-8")
    (docs / "legacy.md").write_text(MERMAID, encoding="utf-8")
    (tmp_path / "README.md").write_text(README_PAGE, encoding="utf-8")
    (tmp_path / "allow.txt").write_text("docs/legacy.md: legacy\nREADME.md: legacy\n", encoding="utf-8")
    return tmp_path


def test_cli_modes_and_exit_codes(tmp_path, capsys, monkeypatch):
    _tree(tmp_path)
    monkeypatch.chdir(tmp_path)
    monkeypatch.delenv(cdb.STRICT_ENV, raising=False)
    # --tolerant: the pre-INT default (STRICT_DEFAULT is True since INT flipped it)
    assert cdb.main(["--tolerant", "--allowlist", "allow.txt", "docs", "README.md"]) == 1
    out = capsys.readouterr().out
    assert "docs/bad.md:3: error: [details-form]" in out
    assert "docs/bad.md:1: warning: [heading-glyph]" in out
    assert out.rstrip().splitlines()[-1].startswith("check_docs_badges: 1 error(s), 1 warning(s) in 1 of 4 file(s) (tolerant mode")
    # without the allowlist line, legacy.md and README.md trip the mermaid rule
    assert cdb.main(["--allowlist", "none.txt", "--quiet", "docs", "README.md"]) == 1
    out = capsys.readouterr().out
    assert "docs/legacy.md:5: error: [mermaid-allowlist]" in out and "README.md:17: error: [mermaid-allowlist]" in out
    assert "check_docs_badges:" not in out
    # --pages restricts the run (positional paths go before it: --pages is nargs="+"); good.md alone is clean even in strict mode
    assert cdb.main(["--strict", "--allowlist", "allow.txt", "docs", "README.md", "--pages", "docs/good.md"]) == 0
    assert capsys.readouterr().out.rstrip().endswith("(strict mode; always-on rules: tabs, mermaid-allowlist, details-form, unchecked-output)")
    # --pages alone lints exactly those paths; strict turns the heading warning into an error; --counts prints the per-rule table
    assert cdb.main(["--strict", "--counts", "--allowlist", "allow.txt", "--pages", "docs/bad.md"]) == 1
    out = capsys.readouterr().out
    assert "docs/bad.md:1: error: [heading-glyph]" in out and "rule counts:" in out and "heading-glyph: 1 error(s)" in out
    with pytest.raises(SystemExit) as exc:
        cdb.main(["docs/missing.md"])
    assert exc.value.code == 2


def test_environment_forces_strict_and_tolerant_overrides_it(monkeypatch):
    monkeypatch.delenv(cdb.STRICT_ENV, raising=False)
    assert cdb.resolve_mode(False, False) is cdb.STRICT_DEFAULT
    assert cdb.resolve_mode(True, False) is True
    assert cdb.resolve_mode(False, True) is False
    monkeypatch.setenv(cdb.STRICT_ENV, "1")
    assert cdb.resolve_mode(False, False) is True
    assert cdb.resolve_mode(False, True) is False
    monkeypatch.setenv(cdb.STRICT_ENV, "0")
    assert cdb.resolve_mode(False, False) is cdb.STRICT_DEFAULT


def test_page_key_is_repository_relative(tmp_path, monkeypatch):
    assert cdb.page_key(ROOT / "docs" / "index.md") == "docs/index.md"
    assert cdb.page_key(ROOT / "README.md") == "README.md"
    monkeypatch.chdir(tmp_path)
    (tmp_path / "docs").mkdir()
    (tmp_path / "docs" / "x.md").write_text("")
    assert cdb.page_key(Path("docs/x.md")) == "docs/x.md"


# --- the committed allowlist and the documentation tree ------------------------------------

DESIGN_4_11_TABLE = {  # rows (a) and (b) of DESIGN §4.11: path -> allowed categories
    "docs/history/origins.md": {"loop"},
    "docs/processes/resist-models.md": {"loop"},
    "docs/processes/volumetric-exposure.md": {"loop", "decision-tree"},
    "docs/processes/liga-deep-xray.md": {"loop"},
    "docs/processes/grayscale.md": {"loop"},
    "docs/processes/interference-volumetric.md": {"loop"},
    "docs/reference/index.md": {"loop"},
    "docs/extending.md": {"loop"},
    "CONTRIBUTING.md": {"loop"},
    "docs/nodes/index.md": {"decision-tree"},
    "docs/future/alternative-patterning.md": {"decision-tree"},
    "docs/masks.md": {"decision-tree"},
    "docs/processes/index.md": {"decision-tree"},
}


def test_committed_allowlist_holds_the_design_table_and_valid_categories():
    path = cdb.ALLOWLIST_PATH
    assert path.exists(), "scripts/mermaid_allowlist.txt is committed"
    entries = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.split("#", 1)[0].strip()
        if line:
            page, _, category = line.rpartition(":")
            entries.append((page.strip(), category.strip()))
    assert all(c in cdb.CATEGORIES for _, c in entries)
    kept = {(p, c) for p, c in entries if c != "legacy"}
    expected = {(p, c) for p, cats in DESIGN_4_11_TABLE.items() for c in cats}
    assert kept == expected, "non-legacy lines are exactly DESIGN §4.11 rows (a) and (b)"
    assert ("docs/roadmap.md", "loop") not in entries and not any(p == "docs/roadmap.md" for p, _ in entries)
    counts = cdb.load_allowlist(path)
    assert counts["docs/processes/volumetric-exposure.md"] == 2 and counts["docs/architecture.md"] in (0, 3)


def test_documentation_tree_has_no_always_on_errors():
    """The four always-on rules hold on the tree (0 errors in tolerant mode, DESIGN §9.2 G0c)."""
    docs, readme = ROOT / "docs", ROOT / "README.md"
    if not docs.is_dir() or not readme.exists():
        pytest.skip("documentation tree not present")
    allowlist = cdb.load_allowlist()
    errors = []
    for path in cdb.collect([docs, readme, ROOT / "CONTRIBUTING.md"]):
        page = cdb.page_key(path)
        res = cdb.check_text(path.read_text(encoding="utf-8"), page, allowlist, strict=False)
        errors += [f"{page}:{f.line}: [{f.rule}] {f.message}" for f in res.errors]
    assert errors == []
