#!/usr/bin/env python3
"""The one palette / ink / typography source for every figure and for the site CSS.

Three consumers read this module and nothing else for colours:

* ``docs/figures/sim/style.py`` and every matplotlib figure script (``THEMES``,
  ``SERIES``, ``ORDINAL``, ``FIELD_ANCHORS_OKLCH``, ``DIVERGING``, ``FONT_MPL``,
  ``chip()``);
* ``docs/figures/svgkit.py`` and every scripted SVG (``theme()``, ``PUPIL``,
  ``MATERIAL``, ``GRADES``, ``FONT_SVG``, ``TYPE``, ``CHIP``);
* ``docs/stylesheets/extra.css``, whose token block between ``/* huv-tokens:start */``
  and ``/* huv-tokens:end */`` is *generated* by ``python docs/figures/tokens.py --css``
  and pinned byte-for-byte by ``docs/hooks/test_tokens.py`` (so the palette cannot fork).

The module imports nothing beyond the standard library at import time: the stdlib SVG
scripts and the CSS emitter must be able to use it without numpy or matplotlib.
``chip(ax, ...)`` imports matplotlib lazily inside the function.

Values are today's ``extra.css`` / ``style.py`` values unless marked NEW (DESIGN §4.1).
Contrast (WCAG AA 4.5:1 text, 3:1 hairlines) was computed for every token: light
section hues on ``#ffffff`` — history 5.9:1, nodes 5.5:1, future 7.1:1, simulator 5.4:1,
reference 7.6:1; dark hues on ``#201e29`` all ≥ 7:1; grade colours as chip text over
their own tint, the tint mixed into the page surface and quantised to 8 bits: light 7 %
on white — implemented 4.61:1, simplified 4.57:1, theoretical 6.30:1, planned 5.20:1
(at 12 % implemented 4.32:1 and simplified 4.25:1 failed AA; at 8 % simplified rounds
to 4.498:1); dark 18 % on ``#201e29`` — 6.33 / 6.54 / 4.89 / 4.73:1. Hence the 7 % / 18 %
tint caps. Re-run the dataviz palette validator before changing any value.
"""

from __future__ import annotations

import math
import sys

# ---------------------------------------------------------------------------
# Brand and grades
# ---------------------------------------------------------------------------

BRAND = {
    "violet900": "#26145f",
    "violet800": "#3a1f8f",
    "violet600": "#5b36c9",
    "violet300": "#b5a1ff",
    "cyan700": "#0e7490",
    "cyan300": "#22d3ee",
    "cyan200": "#5ee8ff",
}

#: Header rule and the λ colouring of every spectrum strip: (position 0..1, colour).
SPECTRUM = [(0.00, "#7c4dff"), (0.30, "#5b36c9"), (0.75, "#0e7490"), (1.00, "#22d3ee")]

#: slug -> (glyph, word, light, dark). The only colours ever used as chip fills/tints.
GRADES = {
    "implemented": ("✅", "Implemented", "#1b7f3b", "#4ade80"),
    "simplified": ("🔶", "Simplified", "#b45309", "#fbbf24"),
    "theoretical": ("🧪", "Theoretical", "#8e24aa", "#e879f9"),
    "planned": ("🗺️", "Planned", "#1565c0", "#60a5fa"),
}

#: glyph (with or without the U+FE0F selector) -> slug.
GLYPH_TO_GRADE = {g[0].replace("️", ""): slug for slug, g in GRADES.items()}
GRADE_ORDER = ("implemented", "simplified", "theoretical", "planned")

#: NEW — hairlines and eyebrows only, never text, never fills: (light, dark).
SECTION_HUES = {
    "home": ("#5b36c9", "#b5a1ff"),
    "history": ("#8a5a00", "#e3b341"),
    "nodes": ("#0f766e", "#2dd4bf"),
    "future": ("#6d28d9", "#a78bfa"),
    "simulator": ("#0e7490", "#22d3ee"),
    "reference": ("#475569", "#94a3b8"),
}

#: The seven readiness rungs of DESIGN Appendix B, in order of distance from volume manufacturing.
RUNGS = ["In production", "Shipping", "Announced", "Demonstrated", "Roadmap", "Proposed", "Speculative"]

# ---------------------------------------------------------------------------
# Chart chrome (style.py's documented values, unchanged)
# ---------------------------------------------------------------------------

THEMES = {
    "light": dict(surface="#ffffff", ink="#0b0b0b", ink2="#52514e", muted="#6b6a65", grid="#e1e0d9",
                  axis="#c3c2b7", band="#f0efec"),
    "dark": dict(surface="#201e29", ink="#f4f3f8", ink2="#c3c2b7", muted="#9b99a3", grid="#32303a",
                 axis="#4d4b54", band="#2c2a35"),
}

#: Categorical slots 1–8 (identity: assign in order, never cycle). ``highuvlith.viz.style``
#: carries the same eight hexes per theme; ``tests/docs/test_viz_tokens.py`` pins that.
SERIES = {
    "light": ("#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4", "#008300", "#4a3aa7", "#e34948"),
    "dark": ("#3987e5", "#d95926", "#199e70", "#c98500", "#d55181", "#008300", "#9085e9", "#e66767"),
}

#: Names for the eight slots (the order above), used by the schematic MATERIAL table.
SERIES_NAMES = ("blue", "orange", "aqua", "yellow", "magenta", "green", "violet", "red")

#: Ordinal ramps (ordered classes such as doses): one hue, monotone lightness.
ORDINAL = {
    "light": ("#86b6ef", "#5598e7", "#2a78d6", "#1c5cab", "#104281"),
    "dark": ("#184f95", "#256abf", "#3987e5", "#6da7ec", "#9ec5f4"),
}

#: Documented blue ramp (steps 100..700), light -> dark; the field ramp's documented core.
FIELD_RAMP = (
    "#cde2fb", "#b7d3f6", "#9ec5f4", "#86b6ef", "#6da7ec", "#5598e7", "#3987e5",
    "#2a78d6", "#256abf", "#1c5cab", "#184f95", "#104281", "#0d366b",
)

#: Field-ramp anchors (OKLCH L, C, h), dark -> light, for 2-D images of intensity, dose and
#: height (mode-invariant: dark = no light, light = full).
FIELD_ANCHORS_OKLCH = (
    (0.150, 0.035, 257.5), (0.197, 0.055, 257.4), (0.244, 0.075, 257.2), (0.291, 0.090, 257.1),
    (0.338, 0.103, 256.9), (0.385, 0.118, 256.7), (0.433, 0.128, 256.5), (0.480, 0.142, 256.1),
    (0.527, 0.150, 255.9), (0.575, 0.163, 255.5), (0.622, 0.161, 255.1), (0.671, 0.136, 253.8),
    (0.717, 0.118, 253.6), (0.764, 0.097, 253.2), (0.812, 0.079, 253.4), (0.858, 0.057, 254.1),
    (0.905, 0.041, 252.8), (0.950, 0.022, 253.0), (0.985, 0.008, 253.0),
)

#: Diverging pair per theme: (blue, neutral midpoint, red).
DIVERGING = {
    "light": ("#2a78d6", "#f0efec", "#e34948"),
    "dark": ("#3987e5", "#3b3943", "#e66767"),
}

# ---------------------------------------------------------------------------
# The pupil grammar (D5) and the schematic material fills
# ---------------------------------------------------------------------------

#: D5: (brand hue, tint fraction) or a SERIES slot index (1-based) or a chrome key.
PUPIL = {
    "fill": ("violet600", 0.22),
    "coherent_core": ("cyan700", 0.35),
    "order0": 1,
    "order1": 2,
    "order2": 3,
    "outside": "muted",
    "support": "ink2",
}

#: Schematic fills: material -> (series name or chrome key, tint fraction into the surface).
MATERIAL = {
    "resist": ("orange", 0.30),
    "substrate": ("gray", 0.35),
    "water": ("cyan700", 0.25),
    "glass": ("blue", 0.18),
    "mirror": ("blue", 0.40),
    "absorber": ("ink", 0.85),
    "spacer": ("aqua", 0.55),
    "mandrel": ("violet", 0.40),
    "metal": ("yellow", 0.50),
}

#: One drawing vocabulary for honesty marks; every chart that uses it carries the legend.
PROVENANCE = {
    "measured": dict(fill=True),
    "calibrated": dict(fill=True, ring=True),
    "reported": dict(fill=True, alpha=0.6),
    "assumed": dict(hatch="///"),
    "announced": dict(fill=False),
    "projection": dict(fill=False, dash=(4, 3)),  # also "roadmap"
}
PROVENANCE["roadmap"] = PROVENANCE["projection"]

# ---------------------------------------------------------------------------
# Typography and the chip geometry
# ---------------------------------------------------------------------------

#: matplotlib font stack (Helvetica Neue exists on the figure machine, DESIGN §8.1).
FONT_MPL = ["Helvetica Neue", "Helvetica", "Arial", "Liberation Sans", "DejaVu Sans"]
#: Hand-built SVG ``<text>``: Roboto on the site, the system sans elsewhere.
FONT_SVG = "Roboto, system-ui, -apple-system, 'Segoe UI', 'Helvetica Neue', Arial, sans-serif"
TYPE = {"svg_base": 14, "svg_small": 12.5, "svg_label": 13, "mpl_base_pt": 9.5, "mpl_small_pt": 8.5}
#: Shared chip geometry (CSS ``.huv-chip``, ``svgkit.chip()``, ``tokens.chip()``).
CHIP = dict(pad_x=0.55, pad_y=0.08, radius=999, font_scale=0.9, weight=600, tint=0.07, tint_dark=0.18,
            border=0.35, border_dark=0.45)

#: Rhythm, type scale and shape tokens of the site CSS (DESIGN §4.1).
SPACE = {"s1": "0.4rem", "s2": "0.8rem", "s3": "1.2rem", "s4": "1.6rem", "s6": "2.4rem", "s8": "3.2rem"}
FONT_SIZES = {"h1": "2em", "h2": "1.5em", "h3": "1.2em", "h4": "1em", "lead": "0.95rem", "small": "0.72rem",
              "micro": "0.64rem"}
SHAPE = {"measure": "38em", "radius": "0.2rem", "radius-pill": "999px", "hairline": "0.05rem", "rule": "0.1rem",
         "motion": "160ms"}

# ---------------------------------------------------------------------------
# Colour helpers (pure Python)
# ---------------------------------------------------------------------------


def _hex(c: str) -> tuple[int, int, int]:
    c = c.lstrip("#")
    return int(c[0:2], 16), int(c[2:4], 16), int(c[4:6], 16)


def hex_to_rgb(c: str) -> tuple[float, float, float]:
    """``#rrggbb`` -> (r, g, b) in 0..1."""
    r, g, b = _hex(c)
    return r / 255, g / 255, b / 255


def mix(hex_color: str, hex_bg: str, frac: float) -> str:
    """Blend ``frac`` of ``hex_color`` into ``hex_bg`` (both ``#rrggbb``); moved from make_node_schematics.py."""
    a, b = _hex(hex_color), _hex(hex_bg)
    return "#%02x%02x%02x" % tuple(round(frac * x + (1 - frac) * y) for x, y in zip(a, b))


def _lin(c: float) -> float:
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def luminance(hex_color: str) -> float:
    """Relative luminance (WCAG) of a hex colour."""
    r, g, b = hex_to_rgb(hex_color)
    return 0.2126 * _lin(r) + 0.7152 * _lin(g) + 0.0722 * _lin(b)


def contrast_ratio(a: str, b: str) -> float:
    """WCAG contrast ratio between two hex colours."""
    la, lb = luminance(a), luminance(b)
    hi, lo = max(la, lb), min(la, lb)
    return (hi + 0.05) / (lo + 0.05)


def text_on(hex_fill: str) -> str:
    """Ink for text set inside a coloured fill: near-black or white by luminance."""
    return "#0b0b0b" if luminance(hex_fill) > 0.25 else "#ffffff"


def spectrum_color(t: float) -> str:
    """Colour of the brand gradient at position ``t`` in 0..1 (linear sRGB interpolation of the stops)."""
    t = min(1.0, max(0.0, t))
    for (p0, c0), (p1, c1) in zip(SPECTRUM, SPECTRUM[1:]):
        if t <= p1:
            f = 0.0 if p1 == p0 else (t - p0) / (p1 - p0)
            return mix(c1, c0, f)
    return SPECTRUM[-1][1]


def wavelength_color(wavelength_nm: float, lo_nm: float = 13.5, hi_nm: float = 436.0, muted: str | None = None) -> str:
    """Brand-gradient colour for a wavelength on a log axis: ``hi_nm`` -> first stop, ``lo_nm`` -> last stop.

    Below ``lo_nm`` the colour fades towards ``muted`` (the X-ray end of a strip);
    above ``hi_nm`` it stays at the first stop.
    """
    if wavelength_nm >= hi_nm:
        return SPECTRUM[0][1]
    if wavelength_nm >= lo_nm or muted is None:
        t = (math.log10(hi_nm) - math.log10(max(wavelength_nm, 1e-9))) / (math.log10(hi_nm) - math.log10(lo_nm))
        return spectrum_color(min(1.0, t))
    # one decade below lo_nm the strip is fully muted
    f = min(1.0, (math.log10(lo_nm) - math.log10(max(wavelength_nm, 1e-9))) / 1.0)
    return mix(muted, SPECTRUM[-1][1], f)


# ---------------------------------------------------------------------------
# One merged theme for the SVG / matplotlib consumers
# ---------------------------------------------------------------------------


def theme(name: str) -> dict:
    """Merged THEMES + SERIES + GRADES + SECTION_HUES + PUPIL + MATERIAL for one theme.

    Keys: ``name, surface, ink, ink2, muted, grid, axis, band, series (tuple of 8),
    blue … red (the eight named slots), grades {slug: hex}, hues {section: hex},
    brand {…}, pupil {fill, coherent_core, order0, order1, order2, outside, support},
    material {name: fill hex}, diverging (blue, mid, red), chip_tint, chip_border``.
    """
    if name not in THEMES:
        raise KeyError(f"unknown theme {name!r}; expected one of {tuple(THEMES)}")
    t = dict(THEMES[name])
    t["name"] = name
    t["series"] = SERIES[name]
    for slot, nm in zip(SERIES[name], SERIES_NAMES):
        t[nm] = slot
    t["gray"] = t["muted"]
    idx = 0 if name == "light" else 1
    t["grades"] = {slug: g[2 + idx] for slug, g in GRADES.items()}
    t["hues"] = {sec: v[idx] for sec, v in SECTION_HUES.items()}
    t["brand"] = dict(BRAND)
    t["diverging"] = DIVERGING[name]
    t["chip_tint"] = CHIP["tint"] if name == "light" else CHIP["tint_dark"]
    t["chip_border"] = CHIP["border"] if name == "light" else CHIP["border_dark"]

    def resolve(spec):
        if isinstance(spec, int):
            return SERIES[name][spec - 1]
        if isinstance(spec, tuple):
            hue, frac = spec
            base = BRAND.get(hue) or t.get(hue)
            if base is None:
                raise KeyError(f"unknown hue {hue!r}")
            return mix(base, t["surface"], frac)
        return t[spec]

    t["pupil"] = {k: resolve(v) for k, v in PUPIL.items()}
    t["pupil_stroke"] = {"fill": BRAND["violet600"], "coherent_core": BRAND["cyan700"]}
    t["material"] = {k: resolve(v) for k, v in MATERIAL.items()}
    t["material_stroke"] = {k: (BRAND.get(v[0]) or t[v[0]]) for k, v in MATERIAL.items()}
    return t


def grade_of(text: str) -> str | None:
    """Slug of the first grade glyph in ``text`` (None when there is none)."""
    for ch in text:
        slug = GLYPH_TO_GRADE.get(ch)
        if slug:
            return slug
    return None


def chip(ax, grade: str, xy, theme_: dict | str, *, size_pt: float | None = None, word: bool = True,
         ha: str = "left", va: str = "center", zorder: float = 5, outlined: bool = False, label: str | None = None):
    """Draw one grade (or rung) chip on a matplotlib axes with the shared CHIP geometry.

    ``grade`` is a slug (``implemented`` …), a glyph, or — with ``outlined=True`` — a
    readiness rung word (outlined slate pill, never a grade colour). ``xy`` is in data
    coordinates; the chip is laid out in points so it survives any axis scale. Returns
    the text artist. matplotlib is imported here, not at module level.
    """
    import matplotlib.patches as mpatches  # noqa: PLC0415
    from matplotlib import transforms  # noqa: PLC0415

    th = theme(theme_) if isinstance(theme_, str) else theme_
    size_pt = size_pt or TYPE["mpl_small_pt"] * CHIP["font_scale"]
    if outlined:
        colour = th["ink2"]
        text = label or grade
        face = "none"
        edge = th["axis"]
    else:
        slug = grade if grade in GRADES else grade_of(grade) or "implemented"
        glyph, wordtxt, _l, _d = GRADES[slug]
        colour = th["grades"][slug]
        text = label or (f"{_mark(slug)} {wordtxt}" if word else _mark(slug))
        face = mix(colour, th["surface"], th["chip_tint"])
        edge = mix(colour, th["surface"], th["chip_border"])
    # bbox pad / rounding are fractions of the font size: 0.35 em padding, fully rounded ends
    bbox = dict(boxstyle="round,pad=0.35,rounding_size=0.85", facecolor=face, edgecolor=edge, linewidth=0.6)
    t = ax.text(xy[0], xy[1], text, fontsize=size_pt, fontweight=CHIP["weight"], color=colour, ha=ha, va=va,
                zorder=zorder, bbox=bbox, transform=ax.transData,
                clip_on=False)
    return t


def _mark(slug: str) -> str:
    """Plain-text stand-in for the grade glyph in matplotlib (emoji fonts are not on the stack)."""
    return {"implemented": "✓", "simplified": "◆", "theoretical": "⚗", "planned": "▭"}[slug]


# ---------------------------------------------------------------------------
# The CSS token block
# ---------------------------------------------------------------------------


def css() -> str:
    """The ``:root`` / ``[data-md-color-scheme="slate"]`` block that ``extra.css`` carries verbatim."""
    g = {slug: v[2] for slug, v in GRADES.items()}
    gd = {slug: v[3] for slug, v in GRADES.items()}
    h = {k: v[0] for k, v in SECTION_HUES.items()}
    hd = {k: v[1] for k, v in SECTION_HUES.items()}
    stops = ",".join(f"{c} {int(round(p * 100))}%" for p, c in SPECTRUM)
    b = BRAND
    s, f, sh = SPACE, FONT_SIZES, SHAPE
    return (
        "/* huv-tokens:start — generated by `python docs/figures/tokens.py --css`; do not edit by hand */\n"
        ":root {\n"
        f"  --huv-violet-900:{b['violet900']}; --huv-violet-800:{b['violet800']}; --huv-violet-600:{b['violet600']}; --huv-violet-300:{b['violet300']};\n"
        f"  --huv-cyan-700:{b['cyan700']};  --huv-cyan-300:{b['cyan300']};  --huv-cyan-200:{b['cyan200']};\n"
        f"  --huv-spectrum: linear-gradient(90deg,{stops});\n"
        f"  --huv-implemented:{g['implemented']}; --huv-simplified:{g['simplified']}; --huv-theoretical:{g['theoretical']}; --huv-planned:{g['planned']};\n"
        f"  --huv-hue-home:{h['home']}; --huv-hue-history:{h['history']}; --huv-hue-nodes:{h['nodes']};\n"
        f"  --huv-hue-future:{h['future']}; --huv-hue-simulator:{h['simulator']}; --huv-hue-reference:{h['reference']};\n"
        "  --huv-section: var(--huv-hue-simulator);          /* overridden per page, §4.2 */\n"
        f"  --huv-s1:{s['s1']}; --huv-s2:{s['s2']}; --huv-s3:{s['s3']}; --huv-s4:{s['s4']}; --huv-s6:{s['s6']}; --huv-s8:{s['s8']};   /* 0.4 rem rhythm */\n"
        f"  --huv-fs-h1:{f['h1']}; --huv-fs-h2:{f['h2']}; --huv-fs-h3:{f['h3']}; --huv-fs-h4:{f['h4']};\n"
        f"  --huv-fs-lead:{f['lead']}; --huv-fs-small:{f['small']}; --huv-fs-micro:{f['micro']};\n"
        f"  --huv-measure:{sh['measure']}; --huv-radius:{sh['radius']}; --huv-radius-pill:{sh['radius-pill']};\n"
        f"  --huv-hairline:{sh['hairline']}; --huv-rule:{sh['rule']}; --huv-motion:{sh['motion']};\n"
        f"  --huv-chip-tint:{int(CHIP['tint'] * 100)}%; --huv-chip-border:{int(CHIP['border'] * 100)}%;\n"
        "}\n"
        '[data-md-color-scheme="slate"] {\n'
        "  --md-hue:250;\n"
        f"  --huv-implemented:{gd['implemented']}; --huv-simplified:{gd['simplified']}; --huv-theoretical:{gd['theoretical']}; --huv-planned:{gd['planned']};\n"
        f"  --huv-hue-home:{hd['home']}; --huv-hue-history:{hd['history']}; --huv-hue-nodes:{hd['nodes']};\n"
        f"  --huv-hue-future:{hd['future']}; --huv-hue-simulator:{hd['simulator']}; --huv-hue-reference:{hd['reference']};\n"
        f"  --huv-chip-tint:{int(CHIP['tint_dark'] * 100)}%; --huv-chip-border:{int(CHIP['border_dark'] * 100)}%;\n"
        "}\n"
        "/* huv-tokens:end */\n"
    )


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if "--css" in argv:
        sys.stdout.write(css())
        return 0
    if "--contrast" in argv:
        for sec, (l, d) in SECTION_HUES.items():
            print(f"{sec:10s} light {contrast_ratio(l, THEMES['light']['surface']):4.1f}:1   dark {contrast_ratio(d, THEMES['dark']['surface']):4.1f}:1")
        for slug, (_g, _w, l, d) in GRADES.items():
            tl = mix(l, THEMES["light"]["surface"], CHIP["tint"])
            td = mix(d, THEMES["dark"]["surface"], CHIP["tint_dark"])
            print(f"{slug:12s} chip text on tint light {contrast_ratio(l, tl):4.1f}:1   dark {contrast_ratio(d, td):4.1f}:1")
        return 0
    print(__doc__.split("\n\n")[0])
    print("usage: tokens.py --css | --contrast")
    return 0


if __name__ == "__main__":
    sys.exit(main())
