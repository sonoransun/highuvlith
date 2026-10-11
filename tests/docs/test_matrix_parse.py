"""The matrix grammar of ``docs/figures/reference/matrix_parse.py`` (DESIGN §8.3, M4).

(a) today's matrix parses to 11 domains, the row count the legend prints and the
documented split forms; (b) a fixture in the restyled form of DESIGN §7.9 — linked
Capability and Code cells, Appendix C split forms, ``<!-- huv-table: grid -->`` lines,
the generated legend block, the changelog inside ``<details>`` under ``## Audit log`` —
parses to identical ``(domain, name, headline)`` triples.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
for sub in ("docs/figures", "docs/figures/reference"):
    p = str(ROOT / sub)
    if p not in sys.path:
        sys.path.insert(0, p)

import matrix_parse  # noqa: E402

MATRIX = ROOT / "docs" / "capability-matrix.md"


def test_today_matrix_parses():
    rows = matrix_parse.rows(MATRIX)
    assert len(matrix_parse.domains(MATRIX)) == 11
    counts = matrix_parse.counts(MATRIX)
    assert sum(counts.values()) == len(rows) >= 70
    assert all(r.headline in "✅🔶🧪🗺" for r in rows)
    assert len({r.name for r in rows}) == len(rows), "duplicate capability names"


def test_documented_split_forms():
    tube = matrix_parse.find("Hard X-ray tube (W / Mo / Cu / Rh, 5–300 kV)", MATRIX)
    assert tube is not None and tube.headline == "🔶" and "✅" in tube.glyphs
    vuv = matrix_parse.find("VUV excimer (F₂ 157.63 nm; Ar₂ 126 nm)", MATRIX)
    assert vuv is not None and vuv.headline == "✅" and "🧪" in vuv.glyphs
    lpp = matrix_parse.find("Laser-produced plasma (Sn 13.5; Gd 6.7; Tb 6.5 nm)", MATRIX)
    assert lpp is not None and lpp.headline == "✅" and set(lpp.glyphs) == {"✅", "🔶", "🧪"}
    assert matrix_parse.find("hopkins imaging (factorized tcc/socs)", MATRIX) is not None  # case-insensitive


def test_status_cell_grammar():
    assert matrix_parse.parse_status("✅") == ("✅", ())
    assert matrix_parse.parse_status("✅ Implemented") == ("✅", ())
    assert matrix_parse.parse_status("✅/🔶")[0] == "✅"
    assert matrix_parse.parse_status("🔶/✅")[0] == "🔶"
    h, q = matrix_parse.parse_status("✅ (🔶 coherence bound; 🧪 Gd/Tb power)")
    assert h == "✅" and q == (("🔶", "coherence bound"), ("🧪", "Gd/Tb power"))
    h, q = matrix_parse.parse_status("✅ (Ar₂ 🧪)")
    assert h == "✅" and q == (("🧪", "Ar₂"),)
    h, q = matrix_parse.parse_status("🔶 (✅ lines, edges, Duane–Hunt, Be window)")
    assert h == "🔶" and q[0][0] == "✅"


def _restyle(text: str) -> str:
    """Today's matrix rewritten in the P8a form (same rows, same grades)."""
    out = []
    in_table = False
    changelog: list[str] = []
    collecting = False
    for line in text.splitlines():
        if line.startswith("## What changed in this revision"):
            collecting = True
            continue
        if collecting:
            if line.startswith("## "):
                collecting = False
            else:
                changelog.append(line)
                continue
        if line.strip() == "## Status taxonomy":
            out.append(line)
            continue
        if line.startswith("| Capability | Status |"):
            out.append("<!-- huv-table: grid -->")
            in_table = True
            out.append(line)
            continue
        if in_table and line.startswith("|") and not line.startswith("|--"):
            cells = line.strip().strip("|").split(" | ")
            if len(cells) >= 4:
                name = cells[0].strip()
                slug = re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-")
                if not name.startswith("["):  # idempotent once P8a has linked the cell
                    cells[0] = f"[{name}](pipeline.md#{slug})"
                status = cells[1].strip()
                m = re.fullmatch(r"([✅🔶🧪🗺]️?)/([✅🔶🧪🗺]️?)", status)
                if m:
                    status = f"{m.group(1)} ({m.group(2)} named parts)"
                status = re.sub(r"\((Ar₂) (🧪)\)", r"(\2 \1)", status)
                cells[1] = status
                codes = re.findall(r"`([^`]+)`", cells[3])
                if codes and "](" not in cells[3]:  # idempotent once P8a has linked the cell
                    cells[3] = ", ".join(f"[{c}](../crates/highuvlith-core/src/{c})" for c in codes)
                line = "| " + " | ".join(cells) + " |"
            out.append(line)
            continue
        if in_table and not line.startswith("|"):
            in_table = False
        if line.startswith("## Core imaging"):
            out.append("<!-- huv-legend:start -->")
            out.append("| ✅ Implemented · 0 | 🔶 Simplified · 0 | 🧪 Theoretical · 0 | 🗺️ Planned · 0 |")
            out.append("|---|---|---|---|")
            out.append("| a | b | c | d |")
            out.append("<!-- huv-legend:end -->")
            out.append("")
        out.append(line)
    out.append("")
    out.append("## Audit log")
    out.append("")
    out.append('<details markdown="1">')
    out.append("<summary>What changed in the 2026-09-30 revision</summary>")
    out.append("")
    out.extend(changelog)
    out.append("")
    out.append("</details>")
    return "\n".join(out) + "\n"


def test_restyled_matrix_parses_identically(tmp_path):
    text = MATRIX.read_text(encoding="utf-8")
    fixture = tmp_path / "capability-matrix.md"
    fixture.write_text(_restyle(text), encoding="utf-8")
    today = [(r.domain, r.name, r.headline) for r in matrix_parse.rows(MATRIX)]
    restyled = [(r.domain, r.name, r.headline) for r in matrix_parse.rows(fixture)]
    assert restyled == today
    assert matrix_parse.counts(fixture) == matrix_parse.counts(MATRIX)
    assert all(r.link for r in matrix_parse.rows(fixture))


def test_details_and_comments_are_ignored(tmp_path):
    fixture = tmp_path / "m.md"
    fixture.write_text(
        "# M\n\n## Alpha\n\n| Capability | Status | Assumptions / limits | Code |\n|---|---|---|---|\n| A | ✅ | x | `a.rs` |\n\n"
        "<details markdown=\"1\">\n<summary>old</summary>\n\n| Capability | Status | Assumptions / limits | Code |\n|---|---|---|---|\n| Hidden | 🗺️ | y | `h.rs` |\n\n</details>\n\n"
        "<!-- | Capability | Status | Assumptions / limits | Code |\n|---|---|---|---|\n| Commented | 🧪 | z | `c.rs` | -->\n\n"
        "## Audit log\n\n| Capability | Status | Assumptions / limits | Code |\n|---|---|---|---|\n| Logged | 🔶 | w | `l.rs` |\n",
        encoding="utf-8")
    rows = matrix_parse.rows(fixture)
    assert [(r.domain, r.name, r.headline) for r in rows] == [("Alpha", "A", "✅")]
