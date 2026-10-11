#!/usr/bin/env python3
"""The readiness ladder: one CSV, one check, one printed table (DESIGN §5.5 N4, Appendix B).

``readiness.csv`` holds one row per technology of the future section's ladder table —
columns ``technology, family, rung, qualifier, label, badge, matrix_capability, model,
page, anchor``:

* ``rung`` is one of the seven Appendix B words (``tokens.RUNGS``); ``qualifier`` carries
  the old evidence nuance in parentheses so no claim changes meaning;
* ``badge`` is the chip glyph (``✅ 🔶 🧪``, or ``∅`` = not modelled); ``;``-separated when a
  technology is graded by two capabilities (``✅;🔶``) — then ``matrix_capability`` lists the
  rows with ``|`` in the same order. The **first** glyph of every badge equals the
  headline grade of its matrix row (Appendix C); a second glyph on the *same* row (``✅;🧪``
  against one LPP row) must be one of that row's own qualifier glyphs — the part the
  technology actually uses (Gd/Tb power 🧪, the ERL preset 🧪), never an upgrade;
* ``model`` is the Markdown of the ladder table's model cell (links relative to
  ``docs/future/``), ``page``/``anchor`` the sub-page(s) the table links.

    python docs/figures/future/check_readiness.py            # validate the CSV + the pasted table
    python docs/figures/future/check_readiness.py --table    # print the collapsed ladder table (column-0 GFM, led by the stack hint)
    python docs/figures/future/check_readiness.py --table --write   # replace it in docs/future/index.md (INT / nightly)
    python docs/figures/future/check_readiness.py --json     # the rows as JSON (diagnostic)

The table lives in ``docs/future/index.md`` between ``<!-- readiness-table:start -->`` and
``<!-- readiness-table:end -->`` (inside ``<details markdown="1"><summary>The ladder as a
table</summary>``, pasted by P4). The rendering's first line is the ``<!-- huv-table: stack -->``
hint (DESIGN §4.5): ``page_kit.py`` turns the 4-column table into cards on phones, and the
comment is invisible on GitHub. With no flag the script validates the CSV (an error exits
1) and compares the pasted block with the rendering; a page without the marker block is a
printed note (tolerant, §9.0 rule 4), a block that differs is an error. Standard library +
``matrix_parse`` only, so ``make_all.py --check`` can run it as a subprocess anywhere.
"""

from __future__ import annotations

import csv
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / "reference"))

import matrix_parse  # noqa: E402
import tokens  # noqa: E402

REPO = HERE.parents[2]
CSV = HERE / "readiness.csv"
PAGE = REPO / "docs" / "future" / "index.md"
START, END = "<!-- readiness-table:start -->", "<!-- readiness-table:end -->"
#: The page-kit hint on the line before the table: 4 columns x 31 rows must stack on phones (DESIGN §4.5).
TABLE_HINT = "<!-- huv-table: stack -->"

RUNGS: tuple[str, ...] = tuple(tokens.RUNGS)
#: The six families = the six sub-pages, in nav order (rows of the matrix figure).
FAMILIES: tuple[str, ...] = (
    "More NA", "Shorter wavelength", "Brighter sources", "Resists and stochastics",
    "Alternatives to projection", "Quantum and exotic",
)
FAMILY_PAGE = {
    "More NA": "high-na-and-hyper-na.md", "Shorter wavelength": "beyond-euv.md",
    "Brighter sources": "accelerator-light-sources.md", "Resists and stochastics": "stochastic-frontier.md",
    "Alternatives to projection": "alternative-patterning.md", "Quantum and exotic": "quantum-and-exotic.md",
}
PAGE_WORD = {
    "high-na-and-hyper-na.md": "High-NA", "beyond-euv.md": "beyond EUV",
    "accelerator-light-sources.md": "accelerators", "stochastic-frontier.md": "stochastics",
    "alternative-patterning.md": "alternatives", "quantum-and-exotic.md": "quantum and exotic",
    "research-modules.md": "research modules",
}
GLYPHS = ("✅", "🔶", "🧪", "🗺", "∅")
NOT_MODELLED = "∅"
_VS = "️"
#: Rows whose badge is a qualifier grade of their matrix row (filled by :func:`validate`, printed by the CLI).
NOTES: list[str] = []


def _g(s: str) -> str:
    return s.replace(_VS, "").strip()


def load(path: Path = CSV) -> list[dict]:
    """The CSV rows with ``badges`` (list of glyphs) and ``rows`` (list of matrix row names) added."""
    with Path(path).open(newline="", encoding="utf-8") as f:
        out = []
        for i, r in enumerate(csv.DictReader(f), start=2):
            r = {k: (v or "").strip() for k, v in r.items()}
            r["line"] = i
            r["badges"] = [_g(b) for b in r["badge"].split(";") if _g(b)]
            r["rows"] = [x.strip() for x in r["matrix_capability"].split("|") if x.strip()]
            r["pages"] = [p.strip() for p in r["page"].split(";") if p.strip()]
            out.append(r)
    return out


def validate(rows: list[dict] | None = None) -> list[str]:
    """Every rule of the module docstring; returns the list of errors."""
    rows = load() if rows is None else rows
    errors: list[str] = []
    seen: set[str] = set()
    NOTES.clear()
    for r in rows:
        tag = f"readiness.csv:{r['line']} ({r['technology'][:40]!r})"
        if r["technology"] in seen:
            errors.append(f"{tag}: duplicate technology")
        seen.add(r["technology"])
        if r["rung"] not in RUNGS:
            errors.append(f"{tag}: rung {r['rung']!r} is not one of {RUNGS}")
        if r["family"] not in FAMILIES:
            errors.append(f"{tag}: family {r['family']!r} is not one of {FAMILIES}")
        if not r["label"] or len(r["label"].split()) > 6:
            errors.append(f"{tag}: label must be 1–6 words (chip text)")
        if not r["badges"]:
            errors.append(f"{tag}: empty badge")
        for b in r["badges"]:
            if b not in GLYPHS:
                errors.append(f"{tag}: badge glyph {b!r} unknown")
        if r["badges"] == [NOT_MODELLED]:
            if r["rows"]:
                errors.append(f"{tag}: '∅' with a matrix_capability")
            if r["model"].strip().lower() != "not modelled":
                errors.append(f"{tag}: '∅' rows say exactly 'not modelled'")
            continue
        if NOT_MODELLED in r["badges"]:
            errors.append(f"{tag}: '∅' cannot be combined with a grade")
        if not r["rows"]:
            errors.append(f"{tag}: graded badge without a matrix_capability")
            continue
        mrows = []
        for name in r["rows"]:
            row = matrix_parse.find(name)
            if row is None:
                errors.append(f"{tag}: matrix_capability {name!r} is not a row of capability-matrix.md")
            mrows.append(row)
        if any(m is None for m in mrows):
            continue
        if len(r["badges"]) == len(mrows):
            pairs = list(zip(r["badges"], mrows))
        elif len(mrows) == 1 and len(r["badges"]) == 2:
            pairs = [(r["badges"][0], mrows[0]), (r["badges"][1], mrows[0])]
        else:
            errors.append(f"{tag}: {len(r['badges'])} badge(s) for {len(mrows)} matrix row(s)")
            pairs = []
        for k, (b, m) in enumerate(pairs):
            head = _g(m.headline)
            quals = {_g(g): txt for g, txt in m.qualifiers}
            if b == head:
                continue
            if b in quals:
                # the grade of a *named part* of the row (its qualifier) — honest for a technology that
                # uses only that part (Gd/Tb power 🧪, the ERL preset 🧪, proximity diffraction 🔶)
                NOTES.append(f"{tag}: badge {b} is the qualifier grade of row {m.name!r} ({b} {quals[b] or 'unnamed part'}), not its headline {head}")
                continue
            errors.append(f"{tag}: badge {b} is neither the headline {head} nor a qualifier glyph of row {m.name!r}")
        # the model cell carries every badge glyph (the table is the accessible form of the chip)
        cell = _g(r["model"])
        for b in r["badges"]:
            if b not in cell:
                errors.append(f"{tag}: model cell lacks the badge glyph {b}")
        for p in r["pages"]:
            if not (REPO / "docs" / "future" / p).exists() and not (REPO / "docs" / p).exists():
                errors.append(f"{tag}: page {p!r} does not exist under docs/future/ or docs/")
    return errors


def render_table(rows: list[dict] | None = None) -> str:
    """The collapsed ladder table as column-0 GFM (DESIGN round-4 ruling 13c), led by
    :data:`TABLE_HINT` so the site stacks it on phones (the hint is part of the marker block)."""
    rows = load() if rows is None else rows
    order = {r: i for i, r in enumerate(RUNGS)}
    forder = {f: i for i, f in enumerate(FAMILIES)}
    rows = sorted(rows, key=lambda r: (order[r["rung"]], forder[r["family"]]))
    lines = [TABLE_HINT, "| Technology | Readiness (Sep 2026) | highuvlith model | Page |", "|---|---|---|---|"]
    for r in rows:
        rung = f"**{r['rung']}**" + (f" ({r['qualifier']})" if r["qualifier"] else "")
        pages = []
        for p in r["pages"]:
            word = PAGE_WORD.get(p, p.replace(".md", ""))
            href = p if p.endswith(".md") and (REPO / "docs" / "future" / p).exists() else f"../{p}"
            if r["anchor"] and href == p:
                href = f"{p}#{r['anchor']}"
            pages.append(f"[{word}]({href})")
        lines.append(f"| {r['technology']} | {rung} | {r['model']} | {', '.join(pages)} |")
    return "\n".join(lines) + "\n"


def _block_re() -> re.Pattern:
    return re.compile(re.escape(START) + r"\n(.*?)\n" + re.escape(END), re.S)


def check_page(rows: list[dict] | None = None, strict: bool = False) -> tuple[list[str], list[str]]:
    """Compare the pasted table on ``docs/future/index.md`` with the rendering."""
    errors: list[str] = []
    warnings: list[str] = []
    if not PAGE.exists():
        warnings.append(f"{PAGE.name} not found")
        return errors, warnings
    text = PAGE.read_text(encoding="utf-8")
    m = _block_re().search(text)
    if not m:
        note = "docs/future/index.md has no <!-- readiness-table:start/end --> block yet (P4 pastes `check_readiness.py --table`)"
        (errors if strict else warnings).append(note)
        return errors, warnings
    want = render_table(rows).strip()
    got = m.group(1).strip()
    if got != want:
        errors.append("the readiness table on docs/future/index.md differs from readiness.csv (run check_readiness.py --table --write)")
    return errors, warnings


def write_table(rows: list[dict] | None = None) -> bool:
    """Replace the marker block in place; returns False (and warns) when the block is absent."""
    if not PAGE.exists():
        print(f"warning: {PAGE} not found; nothing written")
        return False
    text = PAGE.read_text(encoding="utf-8")
    m = _block_re().search(text)
    if not m:
        print("warning: docs/future/index.md has no readiness-table marker block; nothing written")
        return False
    new = text[: m.start(1)] + "\n" + render_table(rows).strip() + "\n" + text[m.end(1):]
    if new != text:
        PAGE.write_text(new, encoding="utf-8")
        print("wrote the readiness table into docs/future/index.md")
    else:
        print("docs/future/index.md readiness table already current")
    return True


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    rows = load()
    errors = validate(rows)
    if "--json" in argv:
        print(json.dumps([{k: v for k, v in r.items() if k != "line"} for r in rows], ensure_ascii=False, indent=1))
        return 0
    if "--table" in argv:
        if errors:
            for e in errors:
                print("error:", e)
            return 1
        if "--write" in argv:
            write_table(rows)
            return 0
        sys.stdout.write(render_table(rows))
        return 0
    strict = "--strict" in argv
    pe, pw = check_page(rows, strict=strict)
    for w in NOTES + pw:
        print("note:", w)
    for e in errors + pe:
        print("error:", e)
    print(f"readiness check: {len(rows)} rows, {len(errors) + len(pe)} error(s)")
    return 1 if (errors or pe) else 0


if __name__ == "__main__":
    sys.exit(main())
