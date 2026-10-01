"""Tests for scripts/check_docs_math.py (run: python -m pytest scripts -q)."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_docs_math as cdm  # noqa: E402


def fixed(text: str, **kw) -> str:
    return cdm.check_text(text, fix=True, **kw).text


def lines_and_messages(text: str, **kw) -> list[tuple[int, str]]:
    return [(f.line, f.message) for f in cdm.check_text(text, **kw).findings]


# --- display math -> ```math fences -------------------------------------------------------


def test_display_block_on_its_own_lines():
    src = "Intro.\n\n$$\nx = \\frac{a}{b}\\,c\n$$\n\nAfter.\n"
    res = cdm.check_text(src, fix=True)
    assert res.text == "Intro.\n\n```math\nx = \\frac{a}{b}\\,c\n```\n\nAfter.\n"
    assert res.converted == 1 and res.findings == []


def test_one_line_display_block():
    assert fixed("$$ E = mc^2 $$\n") == "```math\nE = mc^2\n```\n"
    assert fixed("$$\\gamma = \\frac{1}{2}$$") == "```math\n\\gamma = \\frac{1}{2}\n```"


def test_opener_and_closer_with_content():
    src = "$$\\frac{d F}{d\\theta} = a\n= 2.457\\times10^{13}\\ \\text{0.1\\%BW}$$\n"
    assert (
        fixed(src)
        == "```math\n\\frac{d F}{d\\theta} = a\n= 2.457\\times10^{13}\\ \\text{0.1\\%BW}\n```\n"
    )


def test_admonition_indentation_is_kept():
    src = "!!! note\n    Text.\n\n    $$\n    x^2\n    $$\n\n    More.\n"
    assert (
        fixed(src)
        == "!!! note\n    Text.\n\n    ```math\n    x^2\n    ```\n\n    More.\n"
    )


def test_list_item_lazy_line_is_reindented():
    src = "1. Item\n\n    $$\n    a +\nb\n    $$\n"
    assert fixed(src) == "1. Item\n\n    ```math\n    a +\n    b\n    ```\n"


def test_blockquote_prefixes():
    assert fixed("> $$\n> x\n> $$\n") == "> ```math\n> x\n> ```\n"
    assert fixed("> > $$ y $$\n") == "> > ```math\n> > y\n> > ```\n"
    assert fixed("> $$\nlazy\n> $$\n") == "> ```math\n> lazy\n> ```\n"


def test_block_glued_to_text_gets_blank_lines():
    src = "Some text:\n$$\nx\n$$\nmore text\n"
    assert fixed(src) == "Some text:\n\n```math\nx\n```\n\nmore text\n"
    ((line, msg),) = lines_and_messages(src)
    assert line == 2 and "does not render" in msg
    assert fixed("> t\n> $$ x $$\n") == "> t\n>\n> ```math\n> x\n> ```\n"


def test_code_fences_and_spans_untouched():
    src = (
        "```bash\necho $$ $HOME\n$$\nx \\, y\n$$\n```\n\n"
        "~~~\n$$\n$a\\,b$\n~~~\n\n"
        "````markdown\n```\n$$\nx\n$$\n```\n$c\\;d$\n````\n\n"
        "Use `$$x$$` and ``$a\\,b$`` literally.\n"
    )
    res = cdm.check_text(src, fix=True, fix_inline=True)
    assert res.text == src and res.findings == []


def test_existing_math_fence_untouched():
    src = "```math\nx = \\left\\{ a \\right\\} \\, 10\\%\n```\n"
    res = cdm.check_text(src, fix=True, fix_inline=True)
    assert res.text == src and res.findings == []


def test_display_inside_running_text_is_reported_not_rewritten():
    src = "The value $$x^2$$ is inline.\n"
    res = cdm.check_text(src, fix=True)
    assert res.text == src
    assert [f.line for f in res.findings] == [1]
    assert "running text" in res.findings[0].message
    src2 = "$$a$$ and more\n"
    assert fixed(src2) == src2


def test_unclosed_display_is_reported():
    src = "Text.\n\n$$\nx = 1\n\nmore\n"
    res = cdm.check_text(src, fix=True)
    assert res.text == src
    assert [(f.line, "unclosed" in f.message) for f in res.findings] == [(3, True)]


def test_report_mode_flags_every_display_block():
    src = "$$\nx\n$$\n\n$$ \\{y\\} $$\n"
    res = cdm.check_text(src)
    assert res.text == src
    assert [f.line for f in res.findings] == [1, 5]
    assert all("```math" in f.message for f in res.findings)
    assert "\\{" in res.findings[1].message


def test_fix_is_idempotent():
    src = (
        "# T\n\n$$\na\\,b\n$$\n\n!!! note\n    $$ x $$\n\nText $a\\,b$ and $\\{c\\}$.\n"
        "> $$\n> y\n> $$\n"
    )
    once = fixed(src, fix_inline=True)
    assert fixed(once, fix_inline=True) == once
    assert cdm.check_text(once).findings == []


# --- inline math ----------------------------------------------------------------------------


@pytest.mark.parametrize("esc", [",", ";", "!", "{", "}", "|", "%"])
def test_required_inline_escapes_are_reported(esc):
    src = f"line one\n\nsecond paragraph with $a\\{esc}b$ inside\n"
    ((line, msg),) = lines_and_messages(src)
    assert line == 3 and f"\\{esc}" in msg


def test_multiline_inline_span_reports_the_escape_line():
    src = "text $a +\nb\\,c$ end\n"
    assert [ln for ln, _ in lines_and_messages(src)] == [2]


def test_fix_inline_rewrites_safe_escapes():
    src = "Here $a\\,b\\;c\\!d \\{x\\} \\|v\\|$ and $\\left\\{ y \\right.$ end.\n"
    res = cdm.check_text(src, fix_inline=True)
    assert (
        res.text
        == "Here $a b cd \\lbrace x\\rbrace \\Vert v\\Vert$ and $\\left\\lbrace y \\right.$ end.\n"
    )
    assert res.inline_fixed == 2 and res.findings == []


def test_fix_inline_keeps_math_delimiters_valid():
    assert cdm.check_text("x $\\,y\\,$ z\n", fix_inline=True).text == "x $y$ z\n"


def test_percent_is_left_for_a_hand_fix():
    src = "about $0.6\\%$ of it\n"
    res = cdm.check_text(src, fix_inline=True)
    assert res.text == src
    (f,) = res.findings
    assert "\\%" in f.message and "outside the math" in f.message
    res = cdm.check_text("$a\\,b\\%$\n", fix_inline=True)
    assert res.text == "$a b\\%$\n" and len(res.findings) == 1


def test_other_punctuation_escapes_are_reported():
    for esc in ("\\\\", "\\#", "\\&", "\\_", "\\:"):
        ((_, msg),) = lines_and_messages(f"x $a{esc}b$\n")
        assert esc in msg


def test_currency_read_as_math_is_reported():
    ((line, msg),) = lines_and_messages("It cost $150M–$380M in total.\n")
    assert line == 1 and "currency" in msg and "\\$" in msg


def test_escaped_dollar_is_not_a_delimiter():
    assert lines_and_messages("Prices \\$5 and \\$6 today; $x$ is math.\n") == []
    assert lines_and_messages("A \\$ sign then $a\\,b$\n") == [
        (1, lines_and_messages("$a\\,b$\n")[0][1])
    ]


def test_smart_dollar_rules():
    # a space after the opening $ or before the closing $ means "not math"
    assert lines_and_messages("from $ 5 \\, to 6 $ ok\n") == []


def test_html_comments_are_skipped():
    assert lines_and_messages("<!-- $a\\,b$ -->\n") == []


# --- CLI ------------------------------------------------------------------------------------


def test_cli_exit_codes_and_output(tmp_path, capsys):
    good = tmp_path / "good.md"
    good.write_text("```math\nx\n```\n\n$y$\n")
    bad = tmp_path / "sub" / "bad.md"
    bad.parent.mkdir()
    bad.write_text("$$\nx\\,y\n$$\n\nand $a\\,b$\n")
    assert cdm.main([str(tmp_path)]) == 1
    out = capsys.readouterr().out
    assert f"{bad}:1: display math" in out and f"{bad}:5: inline math" in out
    assert "2 finding(s) in 1 of 2 file(s)" in out
    assert cdm.main(["--fix", "--fix-inline", str(tmp_path)]) == 0
    assert (
        "rewrote 1 file(s): 1 display block(s), 1 inline span(s)"
        in capsys.readouterr().out
    )
    assert bad.read_text() == "```math\nx\\,y\n```\n\nand $a b$\n"
    assert cdm.main(["--quiet", str(good)]) == 0
    assert capsys.readouterr().out == ""
    with pytest.raises(SystemExit) as exc:
        cdm.main([str(tmp_path / "missing.md")])
    assert exc.value.code == 2


def test_cli_keeps_crlf_line_endings(tmp_path):
    f = tmp_path / "crlf.md"
    f.write_bytes(b"$$\r\nx\r\n$$\r\n")
    assert cdm.main(["--fix", "--quiet", str(f)]) == 0
    assert f.read_bytes() == b"```math\r\nx\r\n```\r\n"
