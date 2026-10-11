"""The exposure-tool data set behind every History chart (no drawing, no registration).

``litho_tools.csv`` next to this module holds one row per tool generation: year, vendor,
model, wavelength, numerical aperture (NA), the vendor-stated resolution, an optics class
and the keys of the sources every value was read from. This module is imported by
``make_history_figures.py`` (the matplotlib and svgkit charts) and
``make_history_animation.py`` (the animated resolution story), so the loader, the class
encoding and the table writer exist once. It registers nothing and imports neither
numpy nor matplotlib.

Conventions
-----------
* ``k1`` is derived here as resolution × NA / wavelength. Vendors define "resolution" in
  different ways (usually the smallest dense line/space half-pitch the tool is specified
  for), so k1 is an indicator, not a measured process value.
* A blank field means no source was found; every chart skips that point.
* The marker shape of a point encodes its optics class; the colour repeats it (``CLASSES``:
  the categorical slot of ``tokens.SERIES`` and the matplotlib marker).
* Dry tools introduced from 2007 on — after immersion reached volume production (Nikon
  NSR-S609B 2005, ASML XT:1700i 2006) — are newer models of older exposure technologies and
  are drawn hollow so the leading edge stays readable.

``render_table()`` prints the GFM table (column 0, never indented) with the source list;
``make_history_figures.py --table [--write]`` is its command-line form.
"""

from __future__ import annotations

import csv
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent
DATA = HERE / "litho_tools.csv"
#: The page that carries the pasted table between ``<!-- tool-table:start -->`` / ``:end`` markers.
TABLE_PAGE = HERE.parents[1] / "history" / "tool-data.md"
TABLE_MARKER = "tool-table"

# Source keys used in litho_tools.csv -> (short description, URL). The index of a key in
# this dict (1-based) is its S-number in the table and on the charts.
SOURCES: dict[str, tuple[str, str]] = {
    "KATO": ("A. Kato, 'Chronology of Lithography Milestones', v0.9, May 2007",
             "https://www.lithoguru.com/scientist/litho_history/Kato_Litho_History.pdf"),
    "MACK": ("C. A. Mack, 'Milestones in Optical Lithography Tool Suppliers', 2005",
             "https://lithoguru.com/scientist/litho_history/milestones_tools.pdf"),
    "BRUNING2007": ("J. H. Bruning, 'Optical lithography ... 40 years and holding', Proc. SPIE 6520, "
                    "652004 (2007)", "https://doi.org/10.1117/12.720631"),
    "SHMJ": ("Semiconductor History Museum of Japan, stepper exhibit",
             "https://www.shmj.or.jp/museum2010/exhibi2446.html"),
    "NIKON_HIST": ("Nikon, history of semiconductor lithography systems",
                   "https://www.nikon.com/business/semi/history/"),
    "NIKON_ARCH": ("Nikon, lithography system archive (NA, resolution per model)",
                   "https://www.nikon.com/business/semi/lineup/archives/"),
    "NIKON_LINEUP": ("Nikon, current lithography line-up",
                     "https://www.nikon.com/business/semi/lineup/"),
    "CANON_IP_HIST": ("Canon, history of industrial equipment (IP history pages)",
                      "https://global.canon/en/intellectual-property/history/industrial.html"),
    "CANON_EX6": ("Canon, FPA-3030EX6 product page",
                  "https://global.canon/en/product/indtech/semicon/fpa3030ex6.html"),
    "CANON_EX6_NEWS": ("Photonics Online, 'Canon introduces the first 0.65-NA DUV stepper'",
                       "https://www.photonicsonline.com/doc/canon-introduces-the-first-065-na-duv-stepper-0001"),
    "CANON_ES6A": ("Canon, FPA-6300ES6a product page",
                   "https://global.canon/en/product/indtech/semicon/fpa6300es6a.html"),
    "CANON_ES6A_NEWS": ("Canon news release, 5 Apr 2012",
                        "https://global.canon/en/news/2012/apr05e.html"),
    "CANON_IZ2": ("Canon, FPA-5550iZ2 product page",
                  "https://global.canon/en/product/indtech/semicon/fpa5550iz2.html"),
    "CANON_IZ2_NEWS": ("Canon news release, 11 Dec 2017",
                       "https://global.canon/en/news/2017/20171211.html"),
    "ASML_HIST": ("ASML, company history", "https://www.asml.com/en/company/about-asml/history"),
    "ASML_TWINSCAN20": ("ASML, 'TWINSCAN: 20 years of lithography innovation' (2021)",
                        "https://www.asml.com/en/company/stories/2021/twinscan-20-years-innovation"),
    "ASML_IMM_STORY": ("ASML, 'How immersion lithography saved Moore's Law' (2023)",
                       "https://www.asml.com/en/company/stories/2023/how-immersion-lithography-saved-moores-law"),
    "ASML_EUV_STORY": ("ASML, 'Making EUV: from lab to fab' (2022)",
                       "https://www.asml.com/en/company/stories/2022/making-euv-lab-to-fab"),
    "ASML_20F_2004": ("ASML Form 20-F, fiscal 2004",
                      "https://www.sec.gov/Archives/edgar/data/937966/000115697305000129/u48286e20vf.htm"),
    "ASML_20F_2006": ("ASML Form 20-F, fiscal 2006",
                      "https://www.sec.gov/Archives/edgar/data/937966/000095012307000854/u51076e20vf.htm"),
    "ASML_20F_2010": ("ASML Form 20-F, fiscal 2010",
                      "https://www.sec.gov/Archives/edgar/data/937966/000095012311013996/u09689e20vf.htm"),
    "ASML_20F_2013": ("ASML Form 20-F, fiscal 2013",
                      "https://www.sec.gov/Archives/edgar/data/937966/000119312514046822/d546896d20f.htm"),
    "ASML_20F_2015": ("ASML Form 20-F, fiscal 2015",
                      "https://www.sec.gov/Archives/edgar/data/937966/000093796616000017/a20-fasml2015.htm"),
    "ASML_20F_2017": ("ASML Form 20-F / integrated report, fiscal 2017",
                      "https://www.sec.gov/Archives/edgar/data/937966/000093796618000007/a2017integratedreportbased.htm"),
    "ASML_Q2_2021": ("ASML Q2 2021 results press release",
                     "https://www.sec.gov/Archives/edgar/data/937966/000093796621000016/pressreleasequarterlyresul.htm"),
    "ASML_Q4_2021": ("ASML Q4 2021 results press release",
                     "https://www.sec.gov/Archives/edgar/data/937966/000093796622000004/pressreleasequarterlyresul.htm"),
    "ASML_Q4_2023": ("ASML Q4 2023 results press release",
                     "https://www.sec.gov/Archives/edgar/data/937966/000093796624000003/pressreleasequarterlyresul.htm"),
    "ASML_Q1_2024": ("ASML Q1 2024 investor presentation",
                     "https://www.sec.gov/Archives/edgar/data/937966/000093796624000013/presentationinvestorrela.htm"),
    "ASML_Q2_2025": ("ASML Q2 2025 results press release",
                     "https://www.sec.gov/Archives/edgar/data/937966/000162828025034992/pressreleasequarterlyresul.htm"),
    "ASML_1980DI": ("ASML, TWINSCAN NXT:1980Di product page",
                    "https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt1980di"),
    "ASML_2100I": ("ASML, TWINSCAN NXT:2100i product page",
                   "https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt2100i"),
    "ASML_1470": ("ASML, TWINSCAN NXT:1470 product page",
                  "https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt1470"),
    "ASML_860N": ("ASML, TWINSCAN XT:860N product page",
                  "https://www.asml.com/en/products/duv-lithography-systems/twinscan-xt860n"),
    "ASML_XT260": ("ASML, TWINSCAN XT:260 product page",
                   "https://www.asml.com/en/products/duv-lithography-systems/twinscan-xt-260"),
    "ASML_3400B": ("ASML, TWINSCAN NXE:3400B product page",
                   "https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe3400b"),
    "ASML_3400C": ("ASML, TWINSCAN NXE:3400C product page",
                   "https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe3400c"),
    "ASML_3600D": ("ASML, TWINSCAN NXE:3600D product page",
                   "https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe-3600d"),
    "ASML_3800E": ("ASML, TWINSCAN NXE:3800E product page",
                   "https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe-3800e"),
    "ASML_EXE5000": ("ASML, TWINSCAN EXE:5000 product page",
                     "https://www.asml.com/en/products/euv-lithography-systems/twinscan-exe-5000"),
    "ASML_EXE5200B": ("ASML, TWINSCAN EXE:5200B product page",
                      "https://www.asml.com/en/products/euv-lithography-systems/twinscan-exe-5200b"),
    "ASML_5500_900": ("Semiconductor Online, 'PAS 5500/900 193 nm Step and Scan System' (ASML product "
                      "description: NA 0.45 to <0.6, linewidths 150-130 nm)",
                      "https://www.semiconductoronline.com/doc/pas-5500900-193-nm-step-and-scan-system-0001"),
}

#: optics class -> (tokens.SERIES slot, 1-based; matplotlib marker; legend label). The
#: marker shape is a second, colour-independent encoding of the same class.
CLASSES: dict[str, tuple[int, str, str]] = {
    "dry": (1, "o", "Dry projection (air)"),
    "immersion": (2, "s", "ArF water immersion"),
    "euv": (3, "D", "EUV (mirrors, vacuum)"),
}
LATER_DRY_FROM = 2007
LATER_DRY_LABEL = "Dry, introduced 2007 or later"


@dataclass
class Tool:
    year: int
    vendor: str
    model: str
    wavelength_nm: float | None
    na: float | None
    resolution_nm: float | None
    optics: str
    year_note: str
    sources: list[str]

    @property
    def later_dry(self) -> bool:
        return self.optics == "dry" and self.year >= LATER_DRY_FROM

    @property
    def k1(self) -> float | None:
        if None in (self.wavelength_nm, self.na, self.resolution_nm):
            return None
        return self.resolution_nm * self.na / self.wavelength_nm

    @property
    def source_numbers(self) -> list[int]:
        """The S-numbers (1-based positions in ``SOURCES``) of this row, ascending."""
        keys = list(SOURCES)
        return sorted(keys.index(k) + 1 for k in self.sources)

    @property
    def refs(self) -> str:
        return ", ".join(f"S{i}" for i in self.source_numbers)


def load_tools() -> list[Tool]:
    """Every row of ``litho_tools.csv`` sorted by (year, model); unknown sources or classes raise."""

    def num(s: str) -> float | None:
        return float(s) if s.strip() else None

    tools = []
    with DATA.open(newline="", encoding="utf-8") as fh:
        for r in csv.DictReader(fh):
            keys = [k.strip() for k in r["sources"].split(";") if k.strip()]
            unknown = [k for k in keys if k not in SOURCES]
            if unknown or not keys:
                raise ValueError(f"{r['model']}: unknown or missing sources {unknown}")
            if r["optics"] not in CLASSES:
                raise ValueError(f"{r['model']}: unknown optics class {r['optics']}")
            tools.append(Tool(int(r["year"]), r["vendor"], r["model"], num(r["wavelength_nm"]),
                              num(r["na"]), num(r["resolution_nm"]), r["optics"],
                              r["year_note"], keys))
    return sorted(tools, key=lambda t: (t.year, t.model))


def find_tool(tools: list[Tool], model: str) -> Tool:
    for t in tools:
        if t.model == model:
            return t
    raise KeyError(f"no tool model {model!r} in litho_tools.csv")


#: Exposure-technology lanes: (label, short label, test). The era ribbon and the tool-lanes
#: chart span each lane from the first to the last tool year in the data set.
LANES: list[tuple[str, str, object]] = [
    ("g-line  436 nm", "g-line", lambda t: t.wavelength_nm == 436),
    ("i-line  365 nm", "i-line", lambda t: t.wavelength_nm == 365),
    ("KrF  248 nm", "KrF", lambda t: t.wavelength_nm == 248),
    ("ArF dry  193 nm", "ArF dry", lambda t: t.wavelength_nm == 193 and t.optics == "dry"),
    ("ArF immersion  193 nm", "193i", lambda t: t.optics == "immersion"),
    ("EUV  13.5 nm, NA 0.25–0.33", "EUV", lambda t: t.optics == "euv" and t.na is not None and t.na < 0.5),
    ("High-NA EUV  13.5 nm, NA 0.55", "High-NA", lambda t: t.optics == "euv" and t.na is not None and t.na >= 0.5),
]


def lane_years(tools: list[Tool], test) -> list[int]:
    """Sorted distinct years of the tools a lane test selects."""
    return sorted({t.year for t in tools if test(t)})


# ---------------------------------------------------------------------------------------
# The data table (column-0 GFM) and its in-place writer
# ---------------------------------------------------------------------------------------


def render_table(tools: list[Tool]) -> str:
    """The tool table and its source list as column-0 GFM text (what the tool-data page pastes)."""
    lines = ["| Year | Vendor | Model | λ (nm) | NA | R (nm) | k₁ | Sources |",
             "|---:|---|---|---:|---:|---:|---:|---|"]
    for t in tools:
        k1 = f"{t.k1:.2f}" if t.k1 is not None else "–"
        lam = ("13.5" if t.wavelength_nm == 13.5 else f"{t.wavelength_nm:.0f}") \
            if t.wavelength_nm else "–"
        na = f"{t.na:.3g}" if t.na is not None else "–"
        res = f"{t.resolution_nm:.0f}" if t.resolution_nm is not None else "–"
        lines.append(f"| {t.year} | {t.vendor} | {t.model} | {lam} | {na} | {res} | {k1} | {t.refs} |")
    lines.append("")
    for i, (desc, url) in enumerate(SOURCES.values(), 1):
        lines.append(f"- **S{i}** [{desc}]({url})")
    return "\n".join(lines) + "\n"


def write_table(text: str, page: Path = TABLE_PAGE, marker: str = TABLE_MARKER) -> bool:
    """Replace the block between ``<!-- marker:start -->`` / ``<!-- marker:end -->`` on ``page``.

    Returns True when the file was written; warns and writes nothing when the page or the
    marker block is absent (INT and the nightly workflow are the only callers, DESIGN §9.1).
    """
    start, end = f"<!-- {marker}:start -->", f"<!-- {marker}:end -->"
    if not page.exists():
        print(f"warning: {page} does not exist; nothing written")
        return False
    src = page.read_text(encoding="utf-8")
    a, b = src.find(start), src.find(end)
    if a < 0 or b < 0 or b < a:
        print(f"warning: {page.name} has no {start} / {end} block; nothing written")
        return False
    new = src[: a + len(start)] + "\n\n" + text.rstrip("\n") + "\n\n" + src[b:]
    if new != src:
        page.write_text(new, encoding="utf-8")
        print(f"updated the {marker} block in {page.name}")
        return True
    print(f"{page.name}: {marker} block already up to date")
    return False
