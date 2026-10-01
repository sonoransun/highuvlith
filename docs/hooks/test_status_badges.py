"""Tests for status_badges.py. Run: python -m pytest docs/hooks -q"""

from __future__ import annotations

from status_badges import mark_status_paragraph, on_page_content, wrap_badges


def test_status_paragraph_gets_grade_class() -> None:
    html = "<h1>T</h1>\n<p><strong>Status:</strong> 🔶 Simplified — reasons.</p>\n<p>Body ✅</p>"
    out = mark_status_paragraph(html)
    assert '<p class="huv-status huv-status--simplified"><strong>Status:</strong>' in out
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


def test_marker_followed_by_its_label_is_hidden_from_screen_readers() -> None:
    out = wrap_badges("<p>🧪 Theoretical — speculative</p>")
    assert 'title="Theoretical" aria-hidden="true">🧪</span> Theoretical' in out
    mismatched = wrap_badges("<p>🧪 Implemented</p>")
    assert 'aria-label="Theoretical"' in mismatched


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
    assert out.startswith('<p class="huv-status huv-status--implemented">')
    assert "huv-badge--implemented" in out
