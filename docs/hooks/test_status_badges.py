"""Tests for status_badges.py. Run: python -m pytest docs/hooks -q"""

from __future__ import annotations

from status_badges import mark_status_paragraph, on_page_content, wrap_badges


def test_status_paragraph_gets_grade_class() -> None:
    html = "<h1>T</h1>\n<p><strong>Status:</strong> 🔶 Simplified — reasons.</p>\n<p>Body ✅</p>"
    out = mark_status_paragraph(html)
    assert '<p class="huv-status huv-status--simplified" data-huv-grade="simplified"><strong>Status:</strong>' in out
    assert out.count("huv-status") == 2  # the class pair on one paragraph only


def test_only_the_first_status_paragraph_is_marked() -> None:
    html = "<p><strong>Status:</strong> ✅ a</p><p><strong>Status:</strong> 🧪 b</p>"
    out = mark_status_paragraph(html)
    assert out.count("huv-status--") == 1
    assert "huv-status--implemented" in out


def test_status_paragraph_without_marker() -> None:
    out = mark_status_paragraph("<p><strong>Status:</strong> see below</p>")
    assert 'class="huv-status huv-status--unknown"' in out


def test_page_without_status_is_unchanged() -> None:
    html = "<p>No status here.</p>"
    assert mark_status_paragraph(html) == html


def test_badges_are_wrapped_with_labels() -> None:
    out = wrap_badges("<td>✅</td><td>🗺️</td><td>✅/🔶</td>")
    assert (
        '<span class="huv-badge huv-badge--implemented" title="Implemented" '
        'role="img" aria-label="Implemented">✅</span>'
    ) in out
    assert 'huv-badge--planned" title="Planned" role="img" aria-label="Planned">🗺️</span>' in out
    assert out.count("huv-badge--") == 4


CHIP = '<span class="huv-chip huv-chip--{slug}"><span class="huv-chip__glyph" aria-hidden="true">{glyph}</span>{word}</span>'


def test_marker_followed_by_its_label_is_hidden_from_screen_readers() -> None:
    out = wrap_badges("<p>🧪 Theoretical — speculative</p>")
    assert out == "<p>" + CHIP.format(slug="theoretical", glyph="🧪", word="Theoretical") + " — speculative</p>"
    assert "huv-badge" not in out  # the chip reads the word once; no second announcement
    nbsp = wrap_badges("<p>🗺️\u00a0Planned</p>")
    assert nbsp == "<p>" + CHIP.format(slug="planned", glyph="🗺️", word="Planned") + "</p>"


def test_chip_requires_matching_label() -> None:
    mismatched = wrap_badges("<p>🧪 Implemented</p>")
    assert mismatched == (
        '<p><span class="huv-badge huv-badge--theoretical" title="Theoretical" role="img" '
        'aria-label="Theoretical">🧪</span> Implemented</p>'
    )
    bare = wrap_badges("<p>✅ — done; ✅ Implementedness; ✅\nImplemented</p>")
    assert bare.count("huv-badge--implemented") == 3 and "huv-chip" not in bare
    assert "Implementedness" in bare and "Implemented</p>" in bare


def test_chip_in_status_paragraph() -> None:
    out = on_page_content("<p><strong>Status:</strong> ✅ Implemented (🔶 coherence bound) — x · <a href=\"capability-matrix.md#light-sources\">matrix</a></p>", None, None, None)
    assert out.startswith('<p class="huv-status huv-status--implemented" data-huv-grade="implemented"><strong>Status:</strong> ')
    assert CHIP.format(slug="implemented", glyph="✅", word="Implemented") + " (" in out
    assert 'huv-badge--simplified" title="Simplified" role="img" aria-label="Simplified">🔶</span> coherence bound' in out


def test_chip_in_table_cell() -> None:
    out = wrap_badges("<td>✅ Implemented</td><td>🔶 Simplified (✅ lines and edges)</td><td>🗺️</td>")
    assert out.count("huv-chip--") == 2
    assert "<td>" + CHIP.format(slug="implemented", glyph="✅", word="Implemented") + "</td>" in out
    assert CHIP.format(slug="simplified", glyph="🔶", word="Simplified") + " (<span class=\"huv-badge huv-badge--implemented\"" in out
    assert 'aria-label="Planned">🗺️</span></td>' in out


def test_chip_not_emitted_inside_code() -> None:
    html = "<p><code>✅ Implemented</code> <kbd>🔶 Simplified</kbd></p><pre><code>🧪 Theoretical</code></pre>"
    assert wrap_badges(html) == html
    out = wrap_badges("<p>✅ Implemented <code>✅ Implemented</code></p>")
    assert out.count("huv-chip--implemented") == 1 and "<code>✅ Implemented</code>" in out


def test_split_grade_first_marker_sets_status_grade() -> None:
    out = mark_status_paragraph("<p><strong>Status:</strong> 🔶 Simplified (✅ lines and edges) — x</p>")
    assert out.startswith('<p class="huv-status huv-status--simplified" data-huv-grade="simplified">')
    out = mark_status_paragraph("<p><strong>Status:</strong> ✅ Implemented for the analytic field / 🔶 Simplified for absorption</p>")
    assert 'data-huv-grade="implemented"' in out


def test_markers_inside_glightbox_anchor_are_wrapped() -> None:
    html = (
        '<figure><a class="glightbox" href="../assets/images/sim/x-light.png" data-type="image" data-gallery="light" '
        'data-caption="Model: Hopkins imaging ✅"><img alt="a" src="../assets/images/sim/x-light.png#gh-light-mode-only"></a>'
        "<figcaption>Lead. <em>Model: Hopkins imaging ✅ · EUV projection optics 🔶 (assumed).</em></figcaption></figure>"
    )
    out = wrap_badges(html)
    assert 'data-caption="Model: Hopkins imaging ✅"' in out  # attributes are never touched
    assert '<img alt="a" src="../assets/images/sim/x-light.png#gh-light-mode-only">' in out
    assert out.count("huv-badge--implemented") == 1 and out.count("huv-badge--simplified") == 1
    assert "huv-chip" not in out  # bare glyphs in a trailer stay quiet badges


def test_hook_is_idempotent() -> None:
    once = on_page_content("<p><strong>Status:</strong> ✅ Implemented — x ✅ 🔶 Simplified</p>", None, None, None)
    assert wrap_badges(once) == once


def test_code_pre_svg_and_attributes_are_untouched() -> None:
    html = (
        '<pre class="mermaid"><code>A["✅ ok"]</code></pre>'
        "<p><code>✅</code> <kbd>🔶</kbd></p>"
        '<span class="twemoji"><svg><title>🧪</title></svg></span>'
        '<abbr title="✅ in attribute">X</abbr>'
        "<!-- 🗺️ comment -->"
    )
    assert wrap_badges(html) == html


def test_nested_elements_inside_skipped_ones() -> None:
    html = "<pre><code><span>✅</span></code></pre><p>✅</p>"
    out = wrap_badges(html)
    assert out.startswith("<pre><code><span>✅</span></code></pre>")
    assert out.count("huv-badge--implemented") == 1


def test_void_elements_do_not_confuse_the_parser() -> None:
    out = wrap_badges('<p>a<br>b<img src="x.png" alt="">✅</p>')
    assert out.count("huv-badge--implemented") == 1


def test_on_page_content_combines_both() -> None:
    html = "<p><strong>Status:</strong> ✅ Implemented — done.</p>"
    out = on_page_content(html, None, None, None)
    assert out.startswith('<p class="huv-status huv-status--implemented" data-huv-grade="implemented">')
    assert "huv-chip--implemented" in out
