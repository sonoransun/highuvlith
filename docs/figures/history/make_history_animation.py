#!/usr/bin/env python3
"""The resolution story, animated (N1): ``history/resolution-story-anim`` and its still.

Run from the repository root (standard library only):

    python docs/figures/history/make_history_animation.py

Writes ``docs/assets/images/history/resolution-story-anim-{light,dark}.svg`` (a two-layer
CSS-keyframe SVG, DESIGN §4.12: the ``.anim`` layer plays once, the ``.still`` layer is the
designed resting state, and the in-file ``prefers-reduced-motion`` rule swaps them) plus
``resolution-story-still-{light,dark}.svg`` derived from it by ``svgkit.still_of``.

What it shows (DESIGN §5.5 N1): panel 3 of the static resolution-story chart on its own —
the vendor-stated resolution of every tool model in ``litho_tools.csv`` that states one,
on a log axis 8–2000 nm against the year 1973–2027. The tools appear in year order
(``animation-delay = (year − 1973) × 0.18 s``, ≈ 9.4 s), a year counter follows, a ticker
names the tool that set each new minimum R with its λ, NA and derived k₁, and six
callouts fade in at 1978, 1995, 1999, 2004, 2010 and 2023. The final frame is the
complete panel; the still is that final frame. Every number is read from the CSV; the
caption's tool count is computed, not typed.

Registered as ``resolution-story-anim`` (``kind="anim"``, the history index hero). The
README's Documentation thumbnail is the *still* (DESIGN §6.2, §5.9: nothing but the hero
moves in the README), registered beside it as ``resolution-story-still`` — a static spec
over the files this generator writes — so P9 pastes a static ``<picture>`` from
``--catalog`` and ``--sync-embeds`` never puts the animation into the README.
"""

from __future__ import annotations

import argparse
import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
FIGURES = HERE.parent
for _p in (str(FIGURES), str(HERE)):
    if _p not in sys.path:
        sys.path.insert(0, _p)

import registry  # noqa: E402
import svgkit as K  # noqa: E402
import tokens  # noqa: E402

import history_data as hd  # noqa: E402

OUT = registry.IMAGES / "history"
GENERATOR = "docs/figures/history/make_history_animation.py"
SMALL, LABEL = K.SMALL, K.LABEL

YEAR0, YEAR1 = 1973, 2027
R_LO, R_HI = 8.0, 2000.0
SECONDS_PER_YEAR = 0.18
HOLD_S = 2.5
#: (year, model, text) — the callouts; each names a CSV row, read at draw time.
CALLOUTS = [
    (1978, "DSW 4800", "wafer steppers: GCA DSW 4800, NA 0.28"),
    (1995, "NSR-S201A", "KrF step-and-scan: 250 nm, NA 0.60"),
    (1999, "NSR-S305B", "first ArF scanners: 110 nm, NA 0.68"),
    (2004, "TWINSCAN XT:1250i", "water immersion: XT:1250i, NA 0.85"),
    (2010, "TWINSCAN NXE:3100", "EUV pre-production: NXE:3100, 27 nm"),
    (2023, "TWINSCAN EXE:5000", "High-NA EUV: EXE:5000, 8 nm, NA 0.55"),
]

W, H = 960, 560
PX0, PX1, PY0, PY1 = 76, 596, 48, 468  # plot area
#: A milestone with no vendor-stated resolution (outside the plotted set) is drawn above the R axis at its
#: year, with a dashed ring, never on the log scale where a ring would read as a value the data do not hold.
NO_R_Y = PY0 - 11
NO_R_LABEL = "Milestone with no vendor-stated resolution: dashed ring above the axis at its year, not on the R scale"


def x_of(year: float) -> float:
    return PX0 + (year - YEAR0) / (YEAR1 - YEAR0) * (PX1 - PX0)


def y_of(r: float) -> float:
    return PY1 - (math.log10(r) - math.log10(R_LO)) / (math.log10(R_HI) - math.log10(R_LO)) * (PY1 - PY0)


def plotted(tools: list[hd.Tool]) -> list[hd.Tool]:
    return [t for t in tools if t.resolution_nm is not None]


def leaders(tools: list[hd.Tool]) -> list[hd.Tool]:
    """The CSV rows that set a new minimum stated resolution, in year order."""
    out, best = [], None
    for t in plotted(tools):
        if best is None or t.resolution_nm < best:
            best = t.resolution_nm
            out.append(t)
    return out


def t_of(year: int) -> float:
    return (year - YEAR0) * SECONDS_PER_YEAR


def total_s(tools) -> float:
    return t_of(max(t.year for t in plotted(tools))) + 0.5 + HOLD_S


N_PLOTTED = len(plotted(hd.load_tools()))
ALT = (f"Vendor-stated resolution of {N_PLOTTED} tool models on a log axis from 2000 to 8 nm against their year from "
       "1973 to 2027, appearing in order of introduction with a year counter and a ticker naming each tool that set a "
       "new minimum, from 1000 nm in 1980 to 8 nm with High-NA EUV in 2023.")
CAPTION = (f"<strong>{N_PLOTTED} tool models with a stated resolution, in order of introduction.</strong> Vendor-stated "
           "resolution on a log scale; squares immersion, diamonds EUV, circles dry (open from 2007 on); the ticker "
           "names each tool that set a new minimum with its λ, NA and k₁ = R·NA/λ, an indicator, not a definition; a "
           "sample, not a catalogue (two Micrascan points are lamp-based). Data: the tool data page. "
           "<em>Data chart from litho_tools.csv; no model badge.</em>")
STILL_CAPTION = (f"<strong>{N_PLOTTED} tool models with a stated resolution, 1980–2025.</strong> Vendor-stated resolution "
                 "on a log scale, from 1000 nm to 8 nm with High-NA EUV; squares immersion, diamonds EUV, circles dry. "
                 "<em>Data chart from litho_tools.csv; no model badge.</em>")


def marker(s: K.Svg, t: hd.Tool, x: float, y: float, *, cls: str | None = None) -> None:
    slot, _m, _label = hd.CLASSES[t.optics]
    col = s.t["series"][slot - 1]
    r = 5.0
    if t.later_dry:
        s.circle(x, y, r - 0.5, fill="none", stroke=col, sw=1.6, cls=cls)
    elif t.optics == "dry":
        s.circle(x, y, r, fill=col, stroke="surface", sw=1.3, cls=cls)
    elif t.optics == "immersion":
        s.rect(x - r, y - r, 2 * r, 2 * r, fill=col, stroke="surface", sw=1.3, rx=1, cls=cls)
    else:
        s.polyline([(x, y - r - 1), (x + r + 1, y), (x, y + r + 1), (x - r - 1, y)], "surface", 1.3, fill=col, close=True,
                   cls=cls)


def ticker_lines(t: hd.Tool) -> tuple[str, str]:
    lam = "13.5" if t.wavelength_nm == 13.5 else (f"{t.wavelength_nm:.0f}" if t.wavelength_nm else "–")
    na = f"{t.na:.2f}" if t.na is not None else "–"
    k1 = f"{t.k1:.2f}" if t.k1 is not None else "–"
    return (f"{t.vendor} {t.model} ({t.year})", f"R {t.resolution_nm:.0f} nm · λ {lam} nm · NA {na} · k₁ {k1}")


def draw(theme: str, *, animate: bool = True) -> str:
    tools = hd.load_tools()
    pts = plotted(tools)
    lead = leaders(tools)
    years = sorted({t.year for t in pts})
    T = total_s(tools)
    s = K.Svg(W, H, theme, "The resolution story, animated", ALT, generator=GENERATOR)
    hist = "hue:history"
    anim = K.Anim(s, loops=1) if animate else None
    # --- axes and grid
    s.eyebrow(PX0, 24, "Vendor-stated resolution R, by year of introduction", size=SMALL)
    for r, lab in ((8, "8"), (13, "13"), (38, "38"), (100, "100"), (250, "250"), (500, "500"), (1000, "1000"), (2000, "2000")):
        y = y_of(r)
        s.line(PX0, y, PX1, y, "grid", 0.8)
        s.text(PX0 - 8, y + 4.5, lab, size=SMALL, anchor="end", color="ink2")
    for yr in range(1975, 2026, 5):
        x = x_of(yr)
        s.line(x, PY1, x, PY1 + 5, "axis", 1)
        s.text(x, PY1 + 20, str(yr), size=SMALL, color="ink2")
    s.line(PX0, PY1, PX1, PY1, "axis", 1.2)
    s.line(PX0, PY0, PX0, PY1, "axis", 1.2)
    s.text(PX0 - 8, PY0 - 10, "R (nm), log", size=SMALL, anchor="end", color="ink2")
    s.text((PX0 + PX1) / 2, PY1 + 40, "year of introduction or first shipment", size=SMALL, color="ink2")
    # k1 = 0.25 is a floor on k1, not on R; the chart marks the EUV restart instead with a label
    # --- legend (static), under the axis label
    lx, ly = PX0 + 4, PY1 + 62
    for cls, (slot, _m, label) in hd.CLASSES.items():
        tt = hd.Tool(1990, "", "", None, None, 1.0, cls, "", [])
        marker(s, tt, lx + 6, ly - 4)
        s.text(lx + 16, ly, label, size=SMALL, anchor="start", color="ink2")
        lx += 24 + K.text_width(label, SMALL)
    tt = hd.Tool(2010, "", "", None, None, 1.0, "dry", "", [])
    marker(s, tt, lx + 6, ly - 4)
    s.text(lx + 16, ly, hd.LATER_DRY_LABEL, size=SMALL, anchor="start", color="ink2")
    lx, ly2 = PX0 + 4, ly + 18
    s.circle(lx + 6, ly2 - 4, 5, fill="none", stroke=hist, sw=1.4, dash="2 2")
    s.text(lx + 16, ly2, NO_R_LABEL, size=SMALL, anchor="start", color="ink2")
    # --- the side panel: counter, ticker, callout list
    SX = PX1 + 24
    s.eyebrow(SX, 24, "Year", size=SMALL)
    s.eyebrow(SX, 118, "New minimum R (vendor-stated)", size=SMALL)
    s.eyebrow(SX, 214, "Milestones", size=SMALL)
    callout_rows = []
    for i, (yr, model, text) in enumerate(CALLOUTS):
        t = hd.find_tool(tools, model)
        assert t.year == yr, (model, t.year, yr)
        callout_rows.append((yr, t, text, 236 + i * 36))

    def draw_points(dynamic: bool) -> None:
        by_year: dict[int, list[hd.Tool]] = {}
        for t in pts:
            by_year.setdefault(t.year, []).append(t)
        for yr, group in sorted(by_year.items()):
            with s.group(cls=f"y{yr}" if dynamic else None):
                for t in group:
                    marker(s, t, x_of(t.year), y_of(t.resolution_nm))

    def draw_counter(yr: int, cls: str | None) -> None:
        s.text(SX, 72, str(yr), size=44, anchor="start", color=hist, weight=700, cls=cls)

    def draw_ticker(t: hd.Tool, cls: str | None) -> None:
        a, b = ticker_lines(t)
        with s.group(cls=cls):
            s.text(SX, 142, a, size=SMALL, anchor="start", color="ink", weight=600)
            s.text(SX, 160, b, size=SMALL, anchor="start", color="ink2")
            s.text(SX, 184, f"{t.resolution_nm:.0f} nm", size=22, anchor="start", color="ink", weight=700)

    def draw_callouts(cls_prefix: str | None) -> None:
        for i, (yr, t, text, y) in enumerate(callout_rows):
            cls = f"{cls_prefix}{i}" if cls_prefix else None
            with s.group(cls=cls):
                px = x_of(t.year)
                if t.resolution_nm is not None:
                    py = y_of(t.resolution_nm)
                    s.circle(px, py, 9, fill="none", stroke=hist, sw=1.6)
                else:  # no stated resolution: above the axis with the legend's dashed ring, not at a fake R
                    py = NO_R_Y
                    s.circle(px, py, 7, fill="none", stroke=hist, sw=1.4, dash="2 2")
                s.line(px + 9, py, PX1 + 10, py, hist, 0.9, dash="2 3", opacity=0.9)
                s.line(PX1 + 10, py, SX - 6, y - 4, hist, 0.9, dash="2 3", opacity=0.9)
                s.text(SX, y, str(yr), size=LABEL, anchor="start", color=hist, weight=700)
                s.text(SX + 42, y, text, size=SMALL, anchor="start", color="ink2")

    # --- the still layer (the final frame) and the animated layer
    if anim is None:
        draw_points(False)
        draw_counter(years[-1], None)
        draw_ticker(lead[-1], None)
        draw_callouts(None)
        return s.render()
    with anim.layer("still"):
        draw_points(False)
        draw_counter(years[-1], None)
        draw_ticker(lead[-1], None)
        draw_callouts(None)
    with anim.layer("anim"):
        draw_points(True)
        for j, yr in enumerate(years):
            draw_counter(yr, f"c{j}")
        for j, t in enumerate(lead):
            draw_ticker(t, f"k{j}")
        draw_callouts("m")
    # keyframes: one "appear" ramp shared by the per-year groups
    anim.keyframes("appear", 1.0).at(0, "opacity:0").at(1.0, "opacity:1")
    s.style(" ".join(f".y{yr}{{opacity:0}}" for yr in years))
    for yr in years:
        anim.animate(f".y{yr}", "appear", 0.35, delay_s=t_of(yr), iterations=1)

    def staged(prefix: str, times: list[float]) -> None:
        """Show item j from times[j] until times[j+1] (the last one stays)."""
        for j, t0 in enumerate(times):
            last = j + 1 == len(times)
            t1 = times[j + 1] if not last else T
            kf = anim.keyframes(f"{prefix}{j}", T)
            kf.at(0, "opacity:0").at(max(t0 - 0.02, 0), "opacity:0").at(t0, "opacity:1")
            if not last:
                kf.at(max(t1 - 0.02, t0), "opacity:1").at(t1, "opacity:0").at(T, "opacity:0")
            else:
                kf.at(T, "opacity:1")
            anim.animate(f".{prefix}{j}", f"{prefix}{j}", T, iterations=1)

    staged("c", [t_of(yr) for yr in years])
    staged("k", [t_of(t.year) for t in lead])
    s.style(" ".join(f".m{i}{{opacity:0}}" for i in range(len(callout_rows))))
    for i, (yr, _t, _text, _y) in enumerate(callout_rows):
        anim.animate(f".m{i}", "appear", 0.6, delay_s=t_of(yr), iterations=1)
    anim.install()
    return s.render()


def gen() -> list[Path]:
    return K.write_pair("resolution-story-anim", lambda t: draw(t, animate=True), OUT)


_MODULE = "make_history_animation.py"
registry.register(registry.AssetSpec(
    name="resolution-story-anim", kind="anim", group="history", subdir="history", fmt="svg", animated="svg",
    alt=ALT, caption=CAPTION, pages=("history/index.md",), hero_on=("history/index.md",), generator=gen,
    platform_independent=True, extra={"loops": 1},
), module=_MODULE)
# The README thumbnail: the still as a static spec over the files gen() writes (see the module docstring).
registry.register(registry.AssetSpec(
    name="resolution-story-still", kind="figure", group="history", subdir="history", fmt="svg",
    alt=ALT, caption=STILL_CAPTION, pages=("README.md",), generator=None, platform_independent=True,
    extra={"still_of": "resolution-story-anim", "width": {"README.md": 300}},
), module=_MODULE)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--leaders", action="store_true", help="print the ticker rows (the tools that set a new minimum R)")
    args = ap.parse_args(argv)
    if args.leaders:
        for t in leaders(hd.load_tools()):
            print(" · ".join(ticker_lines(t)))
        return 0
    for p in gen():
        print(f"wrote {registry.rel_to_repo(p)} ({K.size_kb(p):.0f} KB)")
    print(f"{N_PLOTTED} tools plotted; plays once over {total_s(hd.load_tools()):.1f} s")
    return 0


if __name__ == "__main__":
    sys.exit(main())
