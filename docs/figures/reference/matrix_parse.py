#!/usr/bin/env python3
"""The one parser of ``docs/capability-matrix.md`` (DESIGN §8.3, §10.1-1).

Every consumer that copies a grade reads the matrix through this module and nothing
else: ``make_legend.py`` (the legend counts), ``make_matrix_map.py`` (the tile map),
``make_pipeline_diagrams.py`` (the stage chips), ``future/check_readiness.py`` (the
readiness chips), ``check_figures.check_captions()`` and
``tests/docs/test_caption_badges.py`` (caption ``Model:`` trailers).

Grammar (parses today's matrix *and* the restyled one of DESIGN §7.9):

* A **domain table** is any GFM table whose header row, whitespace-normalised, is
  exactly ``| Capability | Status | Assumptions / limits | Code |``, under the nearest
  preceding H2 (= the domain name).
* Everything under ``## Audit log`` or ``## What changed in this revision``, inside
  ``<details>…</details>``, inside HTML comments (``<!-- huv-table: grid -->``, the
  ``huv-legend`` markers) and every table with another header (``| Badge | Name |
  Definition |``, ``| Defect | Before | Now |``, the generated legend) is ignored.
* **Capability** = ``[text](link)`` or plain text → ``name`` is the link text (backticks
  kept, whitespace normalised).
* **Status** = one glyph of ✅🔶🧪🗺️ (the U+FE0F selector optional), optionally followed
  by its word, optionally followed by ``(<qualifier>)`` whose text may hold further
  glyphs in either order (``(🔶 coherence bound; 🧪 Gd/Tb power)`` and the legacy
  ``(Ar₂ 🧪)`` both parse), and the legacy slash form ``✅/🔶`` → ``headline`` is **the
  first glyph in the cell**, ``qualifiers`` are the other glyphs with their text.
* **Assumptions** = free text, never scanned for glyphs.
* **Code** = backtick spans or ``[text](link)`` links → a list of names.

``CAPTION_ALIASES`` at the end of this file is **frozen**: it maps the ``Model:`` labels
of the 47 figure captions that existed on main when the redesign started to matrix rows,
so ``check_captions()`` reports 0 errors on that baseline. No later group adds to it —
a new caption names a row exactly or maps its label through the spec's own
``extra["model_rows"]`` (DESIGN §5.0).
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
MATRIX = REPO / "docs" / "capability-matrix.md"

GLYPHS = "✅🔶🧪🗺"
GLYPH_WORDS = {"✅": "Implemented", "🔶": "Simplified", "🧪": "Theoretical", "🗺": "Planned"}
GLYPH_SLUGS = {"✅": "implemented", "🔶": "simplified", "🧪": "theoretical", "🗺": "planned"}
HEADER = "| Capability | Status | Assumptions / limits | Code |"
IGNORED_H2 = {"audit log", "what changed in this revision"}

_CELL_SPLIT = re.compile(r"(?<!\\)\|")
_LINK = re.compile(r"\[(?P<text>[^\]]+)\]\((?P<href>[^)]+)\)")
_CODE_SPAN = re.compile(r"`([^`]+)`")
_COMMENT = re.compile(r"<!--.*?-->", re.S)
_GLYPH = re.compile("[" + GLYPHS + "]️?")
_WS = re.compile(r"\s+")


@dataclass(frozen=True)
class Row:
    domain: str
    name: str
    headline: str  # one of the four glyphs, without the U+FE0F selector
    qualifiers: tuple[tuple[str, str], ...]  # (glyph, text) for every other glyph in the Status cell
    assumptions: str
    code: tuple[str, ...]
    line: int  # 1-based line of the row in the matrix file
    status_raw: str = ""
    link: str = ""  # the Capability cell's link target, if any

    @property
    def glyphs(self) -> tuple[str, ...]:
        return (self.headline,) + tuple(g for g, _ in self.qualifiers)

    @property
    def word(self) -> str:
        return GLYPH_WORDS[self.headline]

    @property
    def slug(self) -> str:
        return GLYPH_SLUGS[self.headline]


def norm(text: str) -> str:
    """Whitespace-normalised text without the U+FE0F selector."""
    return _WS.sub(" ", text.replace("️", "")).strip()


def _clean(markdown: str) -> str:
    """Strip HTML comments and ``<details>`` blocks (both may span lines)."""
    text = _COMMENT.sub("", markdown)
    text = re.sub(r"<details\b.*?</details>", "", text, flags=re.S | re.I)
    return text


def _cells(line: str) -> list[str]:
    body = line.strip()
    if body.startswith("|"):
        body = body[1:]
    if body.endswith("|"):
        body = body[:-1]
    return [c.strip() for c in _CELL_SPLIT.split(body)]


def parse_status(cell: str) -> tuple[str, tuple[tuple[str, str], ...]]:
    """Headline glyph and the qualifier glyphs with their text, from one Status cell."""
    found = list(_GLYPH.finditer(cell))
    if not found:
        raise ValueError(f"no grade glyph in Status cell {cell!r}")
    headline = found[0].group(0).replace("️", "")
    quals: list[tuple[str, str]] = []
    rest = cell[found[0].end():]
    rest = re.sub(r"^\s*(Implemented|Simplified|Theoretical|Planned)\b", "", rest)
    # parenthetical qualifiers, split on ';'
    for group in re.findall(r"\(([^()]*)\)", rest):
        for part in group.split(";"):
            glyphs = _GLYPH.findall(part)
            text = norm(_GLYPH.sub("", part)).strip(" ,")
            for g in glyphs:
                quals.append((g.replace("️", ""), text))
    outside = re.sub(r"\([^()]*\)", "", rest)
    for g in _GLYPH.findall(outside):  # legacy slash / positional forms
        quals.append((g.replace("️", ""), ""))
    return headline, tuple(quals)


def parse(markdown: str) -> list[Row]:
    """Rows of every domain table in ``markdown`` (see the module docstring for the grammar)."""
    rows: list[Row] = []
    domain = ""
    skipping = False
    in_table = False
    want_separator = False
    # keep line numbers: strip comments/details without changing line count where possible
    cleaned = _clean(markdown)
    if cleaned.count("\n") != markdown.count("\n"):
        # multi-line blocks were removed; fall back to per-line numbers of the cleaned text
        lines = cleaned.splitlines()
    else:
        lines = cleaned.splitlines()
    for i, raw in enumerate(lines, 1):
        line = raw.rstrip()
        if line.startswith("## "):
            domain = line[3:].strip()
            skipping = norm(domain).lower() in IGNORED_H2
            in_table = False
            continue
        if line.startswith("# "):
            in_table = False
            continue
        if skipping:
            continue
        stripped = line.strip()
        if want_separator:
            want_separator = False
            if re.fullmatch(r"\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)+\|?", stripped):
                in_table = True
                continue
            in_table = False
        if not in_table:
            if stripped.startswith("|") and norm(stripped) == HEADER:
                want_separator = True
            continue
        if not stripped.startswith("|"):
            in_table = False
            continue
        cells = _cells(stripped)
        if len(cells) < 4:
            continue
        cap, status, assumptions, code = cells[0], cells[1], cells[2], cells[3]
        m = _LINK.match(cap)
        name = norm(m.group("text")) if m else norm(cap)
        link = m.group("href") if m else ""
        try:
            headline, quals = parse_status(status)
        except ValueError:
            continue  # not a capability row (no glyph) — e.g. a stray table with the same header
        names = tuple(norm(x) for x in _CODE_SPAN.findall(code)) or tuple(norm(x.group("text")) for x in _LINK.finditer(code))
        if not names and code:
            names = (norm(code),)
        rows.append(Row(domain, name, headline, quals, norm(assumptions.replace("\\|", "|")), names, i,
                        norm(status), link))
    return rows


_CACHE: dict[tuple[str, float], list[Row]] = {}


def rows(path: Path | str | None = None) -> list[Row]:
    """Rows of the matrix file (cached per path and mtime)."""
    p = Path(path) if path else MATRIX
    key = (str(p), p.stat().st_mtime)
    if key not in _CACHE:
        _CACHE.clear()
        _CACHE[key] = parse(p.read_text(encoding="utf-8"))
    return _CACHE[key]


def domains(path: Path | str | None = None) -> list[str]:
    out: list[str] = []
    for r in rows(path):
        if r.domain not in out:
            out.append(r.domain)
    return out


def by_name(path: Path | str | None = None) -> dict[str, Row]:
    return {r.name: r for r in rows(path)}


def _key(label: str) -> str:
    return norm(label).lower()


def find(label: str, path: Path | str | None = None) -> Row | None:
    """The row whose Capability text equals ``label`` (exact, then case-insensitive; whitespace normalised)."""
    want = norm(label)
    table = rows(path)
    for r in table:
        if r.name == want:
            return r
    key = want.lower()
    for r in table:
        if r.name.lower() == key:
            return r
    # tolerate missing/extra backticks around a code name
    bare = key.replace("`", "")
    for r in table:
        if r.name.lower().replace("`", "") == bare:
            return r
    return None


def counts(path: Path | str | None = None) -> dict[str, int]:
    """Headline-grade counts, slug -> number of rows."""
    out = {"implemented": 0, "simplified": 0, "theoretical": 0, "planned": 0}
    for r in rows(path):
        out[r.slug] += 1
    return out


def taxonomy(path: Path | str | None = None) -> dict[str, str]:
    """glyph -> one-line definition from the ``| Badge | Name | Definition |`` table."""
    p = Path(path) if path else MATRIX
    out: dict[str, str] = {}
    for line in p.read_text(encoding="utf-8").splitlines():
        s = line.strip()
        if not s.startswith("|"):
            continue
        cells = _cells(s)
        if len(cells) == 3 and _GLYPH.fullmatch(cells[0]):
            out[cells[0].replace("️", "")] = norm(cells[2])
    return out


# ---------------------------------------------------------------------------
# FROZEN: the Model-label aliases of the 47 captions that existed on main when the
# redesign started (DESIGN §5.0). V0 verified 0 caption errors on that baseline with
# exactly this dict and froze it: nobody adds to it. A caption written after the
# baseline names a matrix row exactly or resolves its label through the spec's own
# ``extra["model_rows"]``. Keys are whitespace-normalised, lower-cased labels as
# ``check_figures.caption_clauses()`` extracts them (legacy mode); values are exact
# Capability texts, lists of them, or — for a legacy clause that grades a part the matrix
# describes in a row's Status qualifier or Assumptions text — ``{"row": …, "glyph": …}``
# pinning the glyph the baseline caption carries.
# ---------------------------------------------------------------------------

CAPTION_ALIASES: dict[str, str | list[str] | dict] = {
    # --- imaging ---
    "scalar hopkins/socs imaging with an exactly factorized tcc": "Hopkins imaging (factorized TCC/SOCS)",
    "exact model applies the defocus phase inside the pupil for every source point (default": "Defocus / through-focus imaging",
    # the legacy KernelPhase option is graded 🔶 inside the Defocus row's Assumptions text (not a row of its own)
    "legacy kernel_phase option multiplies each mask frequency by one on-axis phase and predicts talbot-like contrast revivals that partially coherent illumination washes out":
        {"row": "Defocus / through-focus imaging", "glyph": "🔶"},
    "vector (te/tm) imaging": "Vector (polarized) high-NA imaging",
    "computed by the vector pupil model (vector_pupil_map)": "Vector (polarized) high-NA imaging",
    "computed by the vector pupil model": "Vector (polarized) high-NA imaging",
    "immersion optics": "Immersion lithography (193i)",
    "scalar and vector imaging": ["Hopkins imaging (factorized TCC/SOCS)", "Vector (polarized) high-NA imaging"],
    "per-wavelength imaging": "Broadband imaging (`compute_multiwavelength`)",
    "narrow-band approximation": "Narrow-band polychromatic (`compute_polychromatic`)",
    "hhg source": "High-harmonic generation",
    "scalar hopkins/socs imaging": "Hopkins imaging (factorized TCC/SOCS)",
    # --- masks ---
    "process window / ed analysis": "Process window / ED analysis",
    "exact thin-mask spectrum": "Exact thin-mask spectrum",
    "scalar imaging": "Hopkins imaging (factorized TCC/SOCS)",
    "cd metrics": "CD / ILS / NILS / MEEF metrics",
    "constant-threshold resist": "Process window / ED analysis",
    "kirchhoff thin-mask model": "Mask model and types",
    # --- sources ---
    "source models range from": "Laser-produced plasma (Sn 13.5; Gd 6.7; Tb 6.5 nm)",  # "✅ to 🧪" range statements: first glyph
    "presets' own status": "Laser-produced plasma (Sn 13.5; Gd 6.7; Tb 6.5 nm)",
    "dose-limited throughput": "Dose-limited wafer throughput",
    # the tube row is 🔶 with ✅ parts (lines, edges, Duane–Hunt, Be window) in its Status qualifier
    "line energies, edges, duane–hunt limit and be filtration": {"row": "Hard X-ray tube (W / Mo / Cu / Rh, 5–300 kV)", "glyph": "✅"},
    "continuum shape and absolute flux": "Hard X-ray tube (W / Mo / Cu / Rh, 5–300 kV)",
    # --- deep layers ---
    "liga depth dose with nist μ / μen and the bending-magnet spectrum per log-energy bin": "LIGA / deep X-ray depth dose and exposure time",
    "liga proximity diffraction": "LIGA proximity diffraction",
    "multilayer mirrors": "EUV/BEUV multilayer mirrors",
    "fast marching": "3D development (fast marching)",
    "level set": "3D development (level set)",
    "car peb": "Chemically amplified bake (acid/quencher reaction–diffusion)",
    "talbot / dtl": "Talbot / DTL / ATL proximity lithography",
    "multi-beam interference": "Multi-beam interference / two-photon",
    "grayscale": "Grayscale lithography",
    # --- optimization ---
    "ilt": "ILT mask optimization",
    "sraf insertion": "SRAF insertion",
    "opc (fragment model-based)": "OPC (rule, uniform model, fragment model)",
    # --- site ---
    "scalar hopkins imaging": "Hopkins imaging (factorized TCC/SOCS)",
    "illumination pupil fills": "Illumination pupil fills",
    "broadband imaging": "Broadband imaging (`compute_multiwavelength`)",
    "refractive optics": "Refractive projection optics (dry)",
    "vector imaging": "Vector (polarized) high-NA imaging",
    "multiple patterning": "Multiple patterning (LELE, SADP, SAQP)",
    "euv projection optics": "EUV scanner projection optics (NA 0.33 / High-NA 0.55)",
    "schwarzschild objective": "Schwarzschild reflective objective",
    "liga depth dose": "LIGA / deep X-ray depth dose and exposure time",
    "talbot/dtl": "Talbot / DTL / ATL proximity lithography",
    "immersion": "Immersion lithography (193i)",
    "defocus / through-focus imaging": "Defocus / through-focus imaging",
    # the Gd/Tb in-band power is the 🧪 qualifier of the LPP row (the caption grades the power, not the family)
    "gd lpp source": {"row": "Laser-produced plasma (Sn 13.5; Gd 6.7; Tb 6.5 nm)", "glyph": "🧪"},
    "ssmb": "SSMB storage ring",
    "quantum lithography": "Quantum lithography (N-photon / N00N)",
}


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    path = next((a for a in argv if not a.startswith("--")), None)
    table = rows(path)
    if "--json" in argv:
        import json  # noqa: PLC0415

        print(json.dumps([r.__dict__ for r in table], ensure_ascii=False, indent=1))
        return 0
    for r in table:
        q = " ".join(f"({g} {t})" if t else f"({g})" for g, t in r.qualifiers)
        print(f"{r.line:4d}  {r.headline} {q:24s} {r.domain:34s} {r.name}")
    c = counts(path)
    print(f"{len(table)} rows in {len(domains(path))} domains: " + ", ".join(f"{k} {v}" for k, v in c.items()))
    return 0


if __name__ == "__main__":
    sys.exit(main())
