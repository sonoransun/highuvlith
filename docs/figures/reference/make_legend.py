#!/usr/bin/env python3
"""D6 — the generated grade legend (DESIGN §4.7, §5.1).

Prints the four-cell GFM table that ``docs/index.md``, ``docs/capability-matrix.md``
and ``README.md`` carry between ``<!-- huv-legend:start -->`` / ``<!-- huv-legend:end -->``:
the header row is ``<glyph> <Word> · <row count>`` per grade (the hook renders the glyph +
word as a chip), the body row is the taxonomy's one-line definition, copied verbatim
from the matrix. Counts are headline grades of the parsed matrix rows. The table is led by
``<!-- huv-table: stack -->`` (DESIGN §4.5): on phones the four grades stack as four labelled
cards instead of a 500 px row that hides the fourth grade behind a sideways scroll; on GitHub
the comment is invisible.

    python docs/figures/reference/make_legend.py            # print hint + table (page groups paste it)
    python docs/figures/reference/make_legend.py --check    # the three copies equal the generated text
    python docs/figures/reference/make_legend.py --write    # replace in place (INT and the nightly only)

A file whose markers are absent is a warning (tolerant) / error (strict) for ``--check``
and is skipped with a warning by ``--write``; a block that is present but differs is an
error in every mode (DESIGN §9.0 rule 4).
"""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))

import matrix_parse  # noqa: E402
import tokens  # noqa: E402

REPO = matrix_parse.REPO
MARK_START = "<!-- huv-legend:start -->"
MARK_END = "<!-- huv-legend:end -->"
TABLE_HINT = "<!-- huv-table: stack -->"  # page_kit.py stacks the four cells on phones
LEGEND_FILES = ("docs/index.md", "docs/capability-matrix.md", "README.md")


def table(path: Path | str | None = None) -> str:
    """The legend as column-0 GFM (header = chip word · count, body = the taxonomy definition)."""
    counts = matrix_parse.counts(path)
    defs = matrix_parse.taxonomy(path)
    head, body = [], []
    for slug in tokens.GRADE_ORDER:
        glyph, word, _l, _d = tokens.GRADES[slug]
        head.append(f"{glyph} {word} · {counts[slug]}")
        body.append(defs.get(glyph.replace("️", ""), ""))
    return "| " + " | ".join(head) + " |\n|---|---|---|---|\n| " + " | ".join(body) + " |"


def body(path: Path | str | None = None) -> str:
    """What goes between the markers: the stack hint on its own line, then the table."""
    return f"{TABLE_HINT}\n{table(path)}"


def block(path: Path | str | None = None) -> str:
    return f"{MARK_START}\n{body(path)}\n{MARK_END}"


def _find_block(text: str) -> tuple[int, int] | None:
    a = text.find(MARK_START)
    b = text.find(MARK_END, a + 1) if a >= 0 else -1
    if a < 0 or b < 0:
        return None
    return a, b + len(MARK_END)


def check(strict: bool = False, path: Path | str | None = None) -> tuple[list[str], list[str]]:
    """``(errors, warnings)`` over the three legend files."""
    errors: list[str] = []
    warnings: list[str] = []
    want = block(path)
    for rel in LEGEND_FILES:
        f = REPO / rel
        if not f.exists():
            (errors if strict else warnings).append(f"legend: {rel} does not exist")
            continue
        text = f.read_text(encoding="utf-8")
        span = _find_block(text)
        if span is None:
            (errors if strict else warnings).append(f"legend: {rel} has no {MARK_START} … {MARK_END} block yet")
            continue
        have = text[span[0]:span[1]]
        if have.strip() != want.strip():
            errors.append(f"legend: the block in {rel} differs from make_legend.py output (paste it again, or INT runs --write)")
    return errors, warnings


def write(path: Path | str | None = None) -> list[str]:
    """Replace the block in place in every legend file that has markers; returns the files changed."""
    changed: list[str] = []
    want = block(path)
    for rel in LEGEND_FILES:
        f = REPO / rel
        if not f.exists():
            print(f"warning: {rel} does not exist; nothing written")
            continue
        text = f.read_text(encoding="utf-8")
        span = _find_block(text)
        if span is None:
            print(f"warning: {rel} has no legend marker block; nothing written")
            continue
        new = text[:span[0]] + want + text[span[1]:]
        if new != text:
            f.write_text(new, encoding="utf-8")
            changed.append(rel)
            print(f"updated legend in {rel}")
    return changed


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if "--check" in argv:
        errors, warnings = check(strict="--strict" in argv)
        for w in warnings:
            print(f"warning: {w}")
        for e in errors:
            print(f"error: {e}")
        print(f"legend check: {len(errors)} error(s), {len(warnings)} warning(s)")
        return 1 if errors else 0
    if "--write" in argv:
        write()
        return 0
    if "--counts" in argv:
        print(matrix_parse.counts())
        return 0
    print(body())
    return 0


if __name__ == "__main__":
    sys.exit(main())
