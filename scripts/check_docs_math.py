#!/usr/bin/env python3
r"""Check (and optionally fix) Markdown math so it renders on github.com AND on the MkDocs site.

Why: github.com runs CommonMark backslash-escape processing inside ``$...$`` and
``$$...$$`` *before* MathJax sees the TeX. Every backslash followed by ASCII punctuation
loses its backslash: ``\,`` becomes ``,``, ``\left\{`` becomes an invalid ``\left{``,
``10\%`` becomes ``10%`` (a TeX comment that swallows the rest of the formula) and
``\\`` becomes ``\``. The MkDocs site (pymdownx.arithmatex) keeps the backslashes, so these
formulas look fine locally and break only on GitHub. Fenced ```` ```math ```` blocks are
verbatim on GitHub and are rendered by arithmatex on the site (the ``math`` custom fence
in mkdocs.yml), so display math belongs in ```` ```math ```` fences.

Rules enforced:
  * every ``$$...$$`` display block is a finding; ``--fix`` rewrites it as a ```` ```math ````
    fence (indentation and ``>`` blockquote prefixes of admonitions, lists, tabs and quotes
    are kept on every line);
  * inline ``$...$`` (arithmatex "smart dollar" rules) must not contain backslash + ASCII
    punctuation. ``--fix-inline`` applies the safe rewrites ``\, \; \:`` -> space,
    ``\!`` -> nothing, ``\{ \}`` -> ``\lbrace \rbrace``, ``\|`` -> ``\Vert``; everything
    else (notably ``\%``: put the percent sign outside the math, ``$0.6$ %``) is reported
    for a hand fix;
  * ``$$...$$`` inside running text and unclosed ``$$`` are reported, never rewritten;
  * currency such as ``$150M-$380M`` (read as math by arithmatex) is reported: write ``\$``.
Fenced code blocks (``` or ~~~, any length, nested) and inline code spans are skipped.

Usage:  python scripts/check_docs_math.py [--fix] [--fix-inline] [--quiet] PATH...
Exit status: 1 if any finding remains after the requested fixes, 0 otherwise.
"""

from __future__ import annotations

import argparse
import bisect
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

PUNCT = set("!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~")
SAFE = {
    ",": " ",
    ";": " ",
    ":": " ",
    "!": "",
    "{": "\\lbrace ",
    "}": "\\rbrace ",
    "|": "\\Vert ",
}
MASK = (
    "\x02"  # arithmatex's own inline placeholder range: a math body never contains it
)

_PREFIX = re.compile(r"^((?:[ \t]*>)*[ \t]*)")
_FENCE_OPEN = re.compile(r"^(?P<fence>`{3,}(?=[^`]*$)|~{3,})")
_FENCE_CLOSE = re.compile(r"^(?P<fence>`{3,}|~{3,})[ \t]*$")
_ONE_LINE = re.compile(r"^\$\$(?P<body>.*?\S.*?)\$\$[ \t]*$")
# pymdownx.arithmatex RE_SMART_DOLLAR_INLINE (group 2 = opening $, group 3 = body)
_INLINE = re.compile(
    r"(?:(?<!\\)((?:\\{2})+)(?=\$)|(?<!\\)(\$)(?!\s)((?:\\.|[^\\$\x02\x03])+?)(?<!\s)(?:\$))"
)
_DOLLARS2 = re.compile(r"(?<!\\)\$\$")
_BACKTICKS = re.compile(r"(?<!\\)`+")
_COMMENT = re.compile(r"<!--.*?-->", re.S)


@dataclass
class Finding:
    line: int
    message: str


@dataclass
class Result:
    text: str
    findings: list[Finding] = field(default_factory=list)
    converted: int = 0
    inline_fixed: int = 0


def split_prefix(body: str) -> tuple[str, str]:
    """Container prefix (indentation and ``>`` markers) and the content after it."""
    prefix = _PREFIX.match(body).group(1)
    return prefix, body[len(prefix) :]


def escapes(tex: str) -> list[tuple[int, str]]:
    """(offset, punctuation char) of every backslash + ASCII-punctuation token in ``tex``."""
    found, i = [], 0
    while i < len(tex) - 1:
        if tex[i] == "\\":
            if tex[i + 1] in PUNCT:
                found.append((i, tex[i + 1]))
            i += 2
        else:
            i += 1
    return found


def fix_inline_body(tex: str) -> tuple[str, bool]:
    """Apply the safe rewrites to an inline math body; return (new body, applied?)."""
    out, i = [], 0
    while i < len(tex):
        if tex[i] == "\\" and i + 1 < len(tex):
            out.append(SAFE.get(tex[i + 1], tex[i : i + 2]))
            i += 2
        else:
            out.append(tex[i])
            i += 1
    new = re.sub(r" {2,}", " ", "".join(out)).strip(" ")
    trailing = len(new) - len(new.rstrip("\\"))
    # smart-dollar math may not start/end with whitespace, nor end in a lone backslash
    if not new or new[0].isspace() or new[-1].isspace() or trailing % 2:
        return tex, False
    return new, True


def _names(chars) -> str:
    return " ".join(f"`\\{c}`" for c in sorted(chars))


def _hint(chars: set[str]) -> str:
    if not chars - set(SAFE):
        return "run --fix-inline"
    hints = (
        ["run --fix-inline for " + _names(chars & set(SAFE))]
        if chars & set(SAFE)
        else []
    )
    if "%" in chars:
        hints.append(
            "fix by hand: put the percent sign outside the math, e.g. `$0.6$ %`"
        )
    if "\\" in chars:
        hints.append("fix by hand: GitHub turns `\\\\` into `\\` (use a ```math fence)")
    if not chars - set(SAFE) - {"%", "\\"}:
        return "; ".join(hints)
    return "; ".join(
        hints + ["rewrite the rest by hand or move the formula into a ```math fence"]
    )


def _snippet(tex: str) -> str:
    tex = " ".join(tex.split())
    return tex if len(tex) <= 40 else tex[:37] + "..."


class _Checker:
    def __init__(self, text: str, fix: bool, fix_inline: bool):
        self.lines = text.splitlines(keepends=True)
        self.fix, self.fix_inline = fix, fix_inline
        self.out: list[str] = []
        self.res = Result(text="")
        self.para: list[tuple[int, str]] = []

    def run(self) -> Result:
        lines, i, fence = self.lines, 0, None
        if (
            lines and lines[0].rstrip("\r\n") == "---"
        ):  # YAML front matter: pass through
            end = next(
                (
                    k
                    for k in range(1, len(lines))
                    if lines[k].rstrip("\r\n") in ("---", "...")
                ),
                None,
            )
            if end is not None:
                self.out.extend(lines[: end + 1])
                i = end + 1
        while i < len(lines):
            body = lines[i].rstrip("\r\n")
            content = split_prefix(body)[1]
            if fence:
                self.out.append(lines[i])
                m = _FENCE_CLOSE.match(content)
                if (
                    m
                    and m.group("fence")[0] == fence[0]
                    and len(m.group("fence")) >= len(fence)
                ):
                    fence = None
                i += 1
                continue
            m = _FENCE_OPEN.match(content)
            if m:
                self.flush()
                fence = m.group("fence")
                self.out.append(lines[i])
                i += 1
                continue
            if not content.strip():
                self.flush()
                self.out.append(lines[i])
                i += 1
                continue
            if content.startswith("$$"):
                end = self.display(i)
                if end is not None:
                    i = end + 1
                    continue
            self.para.append((i + 1, lines[i]))
            i += 1
        self.flush()
        self.res.text = "".join(self.out)
        return self.res

    def display(self, i: int) -> int | None:
        """Handle a standalone $$ block starting at line i; return its last line or None."""
        lines = self.lines
        body0 = lines[i].rstrip("\r\n")
        prefix, content = split_prefix(body0)
        eol = lines[i][len(body0) :] or "\n"
        one = _ONE_LINE.match(content)
        if one and "$$" not in one.group("body"):
            tex_lines, end = [prefix + one.group("body").strip() + eol], i
        else:
            rest = content[2:]
            if "$$" in rest:
                return None  # e.g. "$$a$$ text": running text, reported by flush()
            tex_lines = [prefix + rest.strip() + eol] if rest.strip() else []
            end = None
            for j in range(i + 1, len(lines)):
                bj = lines[j].rstrip("\r\n")
                cj = split_prefix(bj)[1]
                if not cj.strip():
                    return (
                        None  # blank line before the closer: not one block (unclosed)
                    )
                if cj.rstrip().endswith("$$"):
                    if "$$" in cj.rstrip()[:-2]:
                        return None
                    before = bj.rstrip()[:-2].rstrip()
                    if split_prefix(before)[1]:
                        tex_lines.append(
                            self.reprefix(prefix, before) + lines[j][len(bj) :]
                        )
                    end = j
                    break
                if "$$" in cj:
                    return None
                tex_lines.append(self.reprefix(prefix, bj) + lines[j][len(bj) :])
            if end is None:
                return None
        glued_before = bool(
            self.para
        )  # text right above (no blank line): separate it on --fix
        # ... and if that text is in the same container, arithmatex does not render the block at all
        same_para = (
            glued_before and split_prefix(self.para[-1][1].rstrip("\r\n"))[0] == prefix
        )
        self.flush()
        tex = "".join(split_prefix(t.rstrip("\r\n"))[1] + "\n" for t in tex_lines)
        nxt = lines[end + 1].rstrip("\r\n") if end + 1 < len(lines) else ""
        glued_after = bool(split_prefix(nxt)[1].strip())
        if self.fix:
            closer_eol = lines[end][len(lines[end].rstrip("\r\n")) :]
            tex_lines = [t if t.endswith(("\n", "\r")) else t + "\n" for t in tex_lines]
            blank = (
                prefix.rstrip() + eol
            )  # keeps a blockquote open, plain newline otherwise
            self.out.extend([blank] if glued_before else [])
            self.out.append(f"{prefix}```math{eol}")
            self.out.extend(tex_lines)
            self.out.append(f"{prefix}```{closer_eol or (eol if glued_after else '')}")
            self.out.extend([blank] if glued_after else [])
            self.res.converted += 1
        else:
            self.out.extend(lines[i : end + 1])
            bad = {c for _, c in escapes(tex)}
            extra = f"; it uses {_names(bad)}, which GitHub strips" if bad else ""
            if same_para:
                extra += "; no blank line before it, so the site does not render it"
            self.res.findings.append(
                Finding(
                    i + 1,
                    f"display math $$...$$: use a ```math fence (run --fix){extra}",
                )
            )
        return end

    @staticmethod
    def reprefix(prefix: str, body: str) -> str:
        """Keep a body line inside the container of the fence (indent / blockquote prefix)."""
        if not prefix.strip():
            lead = len(body) - len(body.lstrip(" \t"))
            return body if lead >= len(prefix) else prefix + body.lstrip(" \t")
        return body if body.startswith(prefix) else prefix + split_prefix(body)[1]

    def flush(self) -> None:
        """Lint (and fix) inline math in the paragraph collected so far."""
        if not self.para:
            return
        first = self.para[0][0]
        text = "".join(line for _, line in self.para)
        self.para = []
        starts, pos = [], 0
        for line in text.splitlines(keepends=True):
            starts.append(pos)
            pos += len(line)

        def line_of(offset: int) -> int:
            return first + bisect.bisect_right(starts, offset) - 1

        masked = list(text)

        def mask(a: int, b: int) -> None:
            for k in range(a, b):
                if masked[k] != "\n":
                    masked[k] = MASK

        for start in starts:  # code spans (line-local, as in Python-Markdown)
            end = text.find("\n", start)
            end = len(text) if end < 0 else end
            k = start
            while True:
                m = _BACKTICKS.search(text, k, end)
                if not m:
                    break
                close = re.compile(r"(?<!`)" + m.group(0) + r"(?!`)").search(
                    text, m.end(), end
                )
                if close:
                    mask(m.start(), close.end())
                    k = close.end()
                else:
                    k = m.end()
        for m in _COMMENT.finditer(text):
            mask(m.start(), m.end())
        dollars = [m.start() for m in _DOLLARS2.finditer("".join(masked))]
        for a, b in zip(dollars[::2], dollars[1::2]):
            self.res.findings.append(
                Finding(
                    line_of(a),
                    "display math $$...$$ inside running text is not a "
                    "display block: put it on its own lines as a ```math fence",
                )
            )
            mask(a, b + 2)
        if len(dollars) % 2:
            self.res.findings.append(
                Finding(
                    line_of(dollars[-1]),
                    "unclosed $$ display math (no closing $$ in " "this paragraph)",
                )
            )
            mask(dollars[-1], dollars[-1] + 2)
        view = "".join(masked)
        edits: list[tuple[int, int, str]] = []
        for m in _INLINE.finditer(view):
            if m.group(2) is None:
                continue
            a, b = m.span(3)
            tex = text[a:b]
            if tex[0].isdigit() and view[m.end() : m.end() + 1].isdigit():
                self.res.findings.append(
                    Finding(
                        line_of(m.start()),
                        f"'${_snippet(tex)}$' looks like currency "
                        "read as math: write \\$ for a literal dollar sign",
                    )
                )
                continue
            found = escapes(tex)
            if not found:
                continue
            if self.fix_inline:
                new, applied = fix_inline_body(tex)
                if applied and new != tex:
                    edits.append((a, b, new))
                    self.res.inline_fixed += 1
                    found = [(o, c) for o, c in found if c not in SAFE]
            by_line: dict[int, set[str]] = {}
            for off, c in found:
                by_line.setdefault(line_of(a + off), set()).add(c)
            for ln, chars in sorted(by_line.items()):
                self.res.findings.append(
                    Finding(
                        ln,
                        f"inline math ${_snippet(tex)}$ uses {_names(chars)}, which GitHub "
                        f"strips: {_hint(chars)}",
                    )
                )
        for a, b, new in sorted(edits, reverse=True):
            text = text[:a] + new + text[b:]
        self.out.append(text)


def check_text(text: str, fix: bool = False, fix_inline: bool = False) -> Result:
    """Check one Markdown document; with fix/fix_inline, Result.text is the rewritten text."""
    res = _Checker(text, fix, fix_inline).run()
    res.findings.sort(key=lambda f: f.line)
    return res


def collect(paths: list[Path]) -> list[Path]:
    files: set[Path] = set()
    for p in paths:
        if p.is_dir():
            files.update(f for f in p.rglob("*.md") if f.is_file())
        elif p.is_file():
            files.add(p)
        else:
            raise FileNotFoundError(p)
    return sorted(files, key=str)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(
        description="Check Markdown math for GitHub + MkDocs rendering."
    )
    ap.add_argument(
        "paths",
        nargs="+",
        type=Path,
        help="Markdown files or directories (*.md, recursive)",
    )
    ap.add_argument(
        "--fix", action="store_true", help="convert $$ display blocks to ```math fences"
    )
    ap.add_argument(
        "--fix-inline",
        action="store_true",
        help="apply the safe inline rewrites (\\, \\; \\: \\! \\{ \\} \\|)",
    )
    ap.add_argument(
        "--quiet",
        "-q",
        action="store_true",
        help="print findings only (no fix notices, no summary)",
    )
    args = ap.parse_args(argv)
    try:
        files = collect(args.paths)
    except FileNotFoundError as exc:
        ap.error(f"no such file or directory: {exc}")
    total = bad_files = converted = fixed = changed = 0
    for path in files:
        text = path.read_bytes().decode("utf-8")  # bytes: keep the file's line endings
        res = check_text(text, args.fix, args.fix_inline)
        if res.text != text:
            path.write_bytes(res.text.encode("utf-8"))
            changed += 1
            if not args.quiet:
                print(
                    f"{path}: converted {res.converted} display block(s), fixed {res.inline_fixed} inline span(s)"
                )
        for f in res.findings:
            print(f"{path}:{f.line}: {f.message}")
        total += len(res.findings)
        bad_files += bool(res.findings)
        converted += res.converted
        fixed += res.inline_fixed
    if not args.quiet:
        summary = f"check_docs_math: {total} finding(s) in {bad_files} of {len(files)} file(s)"
        if args.fix or args.fix_inline:
            summary += f"; rewrote {changed} file(s): {converted} display block(s), {fixed} inline span(s)"
        print(summary)
    return 1 if total else 0


if __name__ == "__main__":
    sys.exit(main())
