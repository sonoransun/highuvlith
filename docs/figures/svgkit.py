"""Scripted SVG for the documentation: one builder, one iconography, one writer.

Standard library only (``numpy`` is imported lazily inside :func:`embed_png`), so every
schematic and animation generator under ``docs/figures/`` is *platform independent*:
the nightly workflow can regenerate them and diff the output byte for byte.

What lives here (DESIGN §4.5 D2, §4.12, §5.0, §8.3):

* :class:`Svg` — the tiny builder promoted out of ``nodes/make_node_schematics.py``
  (``rect/circle/path/line/polyline/text/dim_h/dim_v/swatch_legend/render``), bound to
  ``tokens.theme(name)`` so colours are theme keys (``"ink"``, ``"blue"``,
  ``"grade:implemented"``, ``"hue:history"``, ``"brand:violet600"``) or ``#hex``;
* the **D5 pupil grammar** — :func:`pupil`, :func:`orders`, :func:`support_circle`,
  :func:`wafer`: the pupil circle of radius NA, the source fill, diffraction orders
  coloured by |m|, the dashed (1+σ)NA/λ support circle, the wafer plane;
* :func:`chip` — the grade / rung chip with the shared CHIP geometry (vector glyph
  marks, never emoji text, so the pill renders identically wherever the SVG is shown);
* :class:`Anim` — named CSS keyframes, the two-layer ``.anim`` / ``.still`` groups,
  the in-file ``prefers-reduced-motion`` rule, ≤ 3 loops (hero: once); :func:`still_of`
  derives the designed still from the animated file;
* :func:`embed_png` — a 2-D array as a ``data:image/png`` URI through the OKLab field
  ramp (for an engine carpet behind an animation);
* :func:`write_pair` — ``<name>-{light,dark}.svg`` (+ ``-still-``, ``-narrow-``, crops).

Every SVG carries ``role="img"``, ``<title>`` + ``<desc>`` (= the alt), a ``viewBox``,
``font-family`` = ``tokens.FONT_SVG``, text ≥ 12.5 units, no ``<script>``, no SMIL and
no ``<a>`` (links are inert inside ``<img>``; navigation lives in Markdown beside the
figure).
"""

from __future__ import annotations

import base64
import math
import re
import struct
import zlib
from contextlib import contextmanager
from pathlib import Path
from typing import Callable, Iterator, Sequence

try:  # imported as a sibling (docs/figures on sys.path) or as a script
    import tokens
except ImportError:  # pragma: no cover
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import tokens

FONT = tokens.FONT_SVG
MONO = "'Roboto Mono', 'SF Mono', Menlo, Consolas, 'Liberation Mono', monospace"
BASE = tokens.TYPE["svg_base"]
SMALL = tokens.TYPE["svg_small"]
LABEL = tokens.TYPE["svg_label"]
MIN_TEXT = 12.5

GLYPH_WORDS = {slug: g[1] for slug, g in tokens.GRADES.items()}


def esc(s: str) -> str:
    return str(s).replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace('"', "&quot;")


def fmt(v: float) -> str:
    """Compact coordinate text: ``12``, ``12.5``, never ``12.000``."""
    if isinstance(v, int) or float(v).is_integer():
        return str(int(v))
    return f"{v:.2f}".rstrip("0").rstrip(".")


# ---------------------------------------------------------------------------
# Text metrics (a heuristic with slack: a fallback font must never clip a label box)
# ---------------------------------------------------------------------------

_NARROW = set("iljtfrI.,:;'!|()[] ")
_WIDE = set("mwMW@")
_CAPS = set("ABCDEFGHJKLNOPQRSTUVXYZ0123456789")


def text_width(s: str, size: float = BASE, weight: int = 400) -> float:
    """Estimated advance width of ``s`` in user units for Roboto-class sans at ``size``."""
    w = 0.0
    for ch in str(s):
        if ch in _NARROW:
            w += 0.30
        elif ch in _WIDE:
            w += 0.86
        elif ch in _CAPS:
            w += 0.66
        elif ord(ch) > 0x2000:  # symbols, subscripts, arrows: assume wide
            w += 0.72
        else:
            w += 0.54
    w *= size
    if weight >= 600:
        w *= 1.06
    return w * 1.06  # slack


# ---------------------------------------------------------------------------
# The builder
# ---------------------------------------------------------------------------


class Svg:
    """Tiny SVG builder bound to one theme (``tokens.theme(name)``)."""

    def __init__(self, width: float, height: float, theme: str | dict, title: str, desc: str, *,
                 generator: str = "", font: str = FONT):
        self.w, self.h = width, height
        self.t = tokens.theme(theme) if isinstance(theme, str) else theme
        self.theme = self.t["name"]
        self.title, self.desc = title, desc
        self.generator = generator
        self.font = font
        self.parts: list[str] = []
        self.defs: list[str] = []
        self.styles: list[str] = []
        self._markers: dict[str, str] = {}
        self._clip = 0
        self._indent = ""

    # -- colours -------------------------------------------------------
    def c(self, key: str) -> str:
        """Resolve a colour key: theme chrome, a series name, ``grade:<slug>``, ``hue:<section>``, ``brand:<name>`` or ``#hex``."""
        if key.startswith(("#", "url(")) or key in ("none", "currentColor", "transparent"):
            return key
        if key.startswith("grade:"):
            return self.t["grades"][key[6:]]
        if key.startswith("hue:"):
            return self.t["hues"][key[4:]]
        if key.startswith("brand:"):
            return tokens.BRAND[key[6:]]
        if key.startswith("pupil:"):
            return self.t["pupil"][key[6:]]
        if key.startswith("mat:"):
            return self.t["material"][key[4:]]
        if key in self.t and isinstance(self.t[key], str):
            return self.t[key]
        raise KeyError(f"unknown colour key {key!r}")

    def fill(self, mat: str) -> str:
        return self.t["material"][mat]

    def stroke(self, mat: str) -> str:
        return self.t["material_stroke"][mat]

    def tint(self, key: str, frac: float) -> str:
        """``frac`` of colour ``key`` mixed into the surface."""
        return tokens.mix(self.c(key), self.t["surface"], frac)

    # -- structure -----------------------------------------------------
    def add(self, fragment: str) -> None:
        self.parts.append(self._indent + fragment)

    @contextmanager
    def group(self, cls: str | None = None, *, transform: str | None = None, opacity: float | None = None,
              id: str | None = None, extra: str = "") -> Iterator[None]:
        attrs = ""
        if id:
            attrs += f' id="{id}"'
        if cls:
            attrs += f' class="{cls}"'
        if transform:
            attrs += f' transform="{transform}"'
        if opacity is not None:
            attrs += f' opacity="{fmt(opacity)}"'
        if extra:
            attrs += " " + extra
        self.add(f"<g{attrs}>")
        self._indent += " "
        try:
            yield
        finally:
            self._indent = self._indent[:-1]
            self.add("</g>")

    def clip_rect(self, x: float, y: float, w: float, h: float, rx: float = 0) -> str:
        """Define a (rounded) rectangular clip path and return its ``clip-path`` attribute value."""
        self._clip += 1
        cid = f"clip{self._clip}"
        r = f' rx="{fmt(rx)}"' if rx else ""
        self.defs.append(f'<clipPath id="{cid}"><rect x="{fmt(x)}" y="{fmt(y)}" width="{fmt(w)}" height="{fmt(h)}"{r}/></clipPath>')
        return f"url(#{cid})"

    def gradient_h(self, id: str, stops: Sequence[tuple[float, str]], x1: float = 0, x2: float = 1) -> str:
        """A horizontal linear gradient in ``defs``; stops are (0..1, colour)."""
        body = "".join(f'<stop offset="{fmt(p * 100)}%" stop-color="{c}"/>' for p, c in stops)
        self.defs.append(f'<linearGradient id="{id}" x1="{fmt(x1)}" y1="0" x2="{fmt(x2)}" y2="0">{body}</linearGradient>')
        return f"url(#{id})"

    def hatch(self, id: str, color: str, spacing: float = 5.0, sw: float = 1.0) -> str:
        """A diagonal hatch pattern (the ``assumed`` provenance mark); returns the fill value."""
        self.defs.append(
            f'<pattern id="{id}" width="{fmt(spacing)}" height="{fmt(spacing)}" patternUnits="userSpaceOnUse" '
            f'patternTransform="rotate(45)"><line x1="0" y1="0" x2="0" y2="{fmt(spacing)}" stroke="{color}" stroke-width="{fmt(sw)}"/></pattern>'
        )
        return f"url(#{id})"

    def style(self, css: str) -> None:
        self.styles.append(css.strip())

    # -- primitives ----------------------------------------------------
    def rect(self, x, y, w, h, mat=None, *, fill=None, stroke=None, sw=1.2, rx=0, dash=None, opacity=None, cls=None):
        if w < 0 or h < 0:  # a negative size is invalid SVG: browsers log an error and draw nothing
            raise ValueError(f"rect at ({fmt(x)}, {fmt(y)}): negative size {fmt(w)} × {fmt(h)}")
        f = self.c(fill) if fill is not None else (self.fill(mat) if mat else "none")
        s = self.c(stroke) if stroke is not None else (self.stroke(mat) if mat else "none")
        extra = f' stroke-dasharray="{dash}"' if dash else ""
        extra += f' opacity="{fmt(opacity)}"' if opacity is not None else ""
        extra += f' class="{cls}"' if cls else ""
        self.add(f'<rect x="{fmt(x)}" y="{fmt(y)}" width="{fmt(w)}" height="{fmt(h)}" rx="{fmt(rx)}" '
                 f'fill="{f}" stroke="{s}" stroke-width="{fmt(sw)}"{extra}/>')

    def circle(self, cx, cy, r, *, fill="none", stroke="ink2", sw=1.5, dash=None, opacity=None, cls=None):
        extra = f' stroke-dasharray="{dash}"' if dash else ""
        extra += f' opacity="{fmt(opacity)}"' if opacity is not None else ""
        extra += f' class="{cls}"' if cls else ""
        self.add(f'<circle cx="{fmt(cx)}" cy="{fmt(cy)}" r="{fmt(r)}" fill="{self.c(fill)}" stroke="{self.c(stroke)}" '
                 f'stroke-width="{fmt(sw)}"{extra}/>')

    def ellipse(self, cx, cy, rx, ry, *, fill="none", stroke="ink2", sw=1.5, dash=None, opacity=None, cls=None):
        extra = f' stroke-dasharray="{dash}"' if dash else ""
        extra += f' opacity="{fmt(opacity)}"' if opacity is not None else ""
        extra += f' class="{cls}"' if cls else ""
        self.add(f'<ellipse cx="{fmt(cx)}" cy="{fmt(cy)}" rx="{fmt(rx)}" ry="{fmt(ry)}" fill="{self.c(fill)}" '
                 f'stroke="{self.c(stroke)}" stroke-width="{fmt(sw)}"{extra}/>')

    def path(self, d, mat=None, *, fill=None, stroke=None, sw=1.5, dash=None, join="round", cap="round",
             opacity=None, cls=None, arrow=None):
        f = self.c(fill) if fill is not None else (self.fill(mat) if mat else "none")
        s = self.c(stroke) if stroke is not None else (self.stroke(mat) if mat else "none")
        extra = f' stroke-dasharray="{dash}"' if dash else ""
        extra += f' opacity="{fmt(opacity)}"' if opacity is not None else ""
        extra += f' class="{cls}"' if cls else ""
        extra += self._arrow_attrs(arrow, s)
        self.add(f'<path d="{d}" fill="{f}" stroke="{s}" stroke-width="{fmt(sw)}" stroke-linejoin="{join}" '
                 f'stroke-linecap="{cap}"{extra}/>')

    def line(self, x1, y1, x2, y2, color="muted", sw=1.0, dash=None, arrow=None, *, opacity=None, cls=None):
        col = self.c(color)
        extra = f' stroke-dasharray="{dash}"' if dash else ""
        extra += f' opacity="{fmt(opacity)}"' if opacity is not None else ""
        extra += f' class="{cls}"' if cls else ""
        extra += self._arrow_attrs(arrow, col)
        self.add(f'<line x1="{fmt(x1)}" y1="{fmt(y1)}" x2="{fmt(x2)}" y2="{fmt(y2)}" stroke="{col}" '
                 f'stroke-width="{fmt(sw)}" stroke-linecap="round"{extra}/>')

    def polyline(self, points: Sequence[tuple[float, float]], color="ink2", sw=1.5, *, fill="none", dash=None,
                 arrow=None, opacity=None, cls=None, close=False):
        pts = " ".join(f"{fmt(x)},{fmt(y)}" for x, y in points)
        col = self.c(color)
        extra = f' stroke-dasharray="{dash}"' if dash else ""
        extra += f' opacity="{fmt(opacity)}"' if opacity is not None else ""
        extra += f' class="{cls}"' if cls else ""
        extra += self._arrow_attrs(arrow, col)
        tag = "polygon" if close else "polyline"
        self.add(f'<{tag} points="{pts}" fill="{self.c(fill)}" stroke="{col}" stroke-width="{fmt(sw)}" '
                 f'stroke-linejoin="round" stroke-linecap="round"{extra}/>')

    def text(self, x, y, s, size=BASE, anchor="middle", color="ink", weight=400, italic=False, *, mono=False,
             opacity=None, cls=None, rotate=None, baseline=None, letter_spacing=None, halo=False):
        """One ``<text>``; ``halo=True`` (or a stroke width) paints a surface-coloured outline behind the
        glyphs (``paint-order="stroke"``) so a leader line or tick passing under the label never cuts it."""
        if size < MIN_TEXT:
            raise ValueError(f"text {s!r} at {size} units is below the {MIN_TEXT}-unit floor")
        style = ' font-style="italic"' if italic else ""
        if halo:
            style += (f' paint-order="stroke" stroke="{self.c("surface")}" stroke-width="{fmt(3 if halo is True else halo)}"'
                      ' stroke-linejoin="round"')
        style += f' font-family="{MONO}"' if mono else ""
        style += f' opacity="{fmt(opacity)}"' if opacity is not None else ""
        style += f' class="{cls}"' if cls else ""
        style += f' transform="rotate({fmt(rotate)} {fmt(x)} {fmt(y)})"' if rotate else ""
        style += f' dominant-baseline="{baseline}"' if baseline else ""
        style += f' letter-spacing="{fmt(letter_spacing)}"' if letter_spacing else ""
        self.add(f'<text x="{fmt(x)}" y="{fmt(y)}" font-size="{fmt(size)}" text-anchor="{anchor}" '
                 f'fill="{self.c(color)}" font-weight="{weight}"{style}>{esc(s)}</text>')

    def eyebrow(self, x, y, s, size=SMALL, color="ink2", anchor="start"):
        """Small-caps style label (upper-case, letter-spaced)."""
        self.text(x, y, str(s).upper(), size=size, anchor=anchor, color=color, weight=600, letter_spacing=0.6)

    def dim_h(self, x1, x2, y, label, color="ink2", size=LABEL, above=True, ticks=True):
        """Horizontal dimension: arrowed line between x1 and x2 with a label."""
        self.line(x1, y, x2, y, color, 1.1, arrow="both")
        if ticks:
            self.line(x1, y - 6, x1, y + 6, color, 1.0)
            self.line(x2, y - 6, x2, y + 6, color, 1.0)
        ty = y - 8 if above else y + size + 4
        self.text((x1 + x2) / 2, ty, label, size=size, color=color)

    def dim_v(self, x, y1, y2, label, color="ink2", size=LABEL, side="left"):
        self.line(x, y1, x, y2, color, 1.1, arrow="both")
        self.line(x - 6, y1, x + 6, y1, color, 1.0)
        self.line(x - 6, y2, x + 6, y2, color, 1.0)
        if side == "left":
            self.text(x - 10, (y1 + y2) / 2 + size / 3, label, size=size, anchor="end", color=color)
        else:
            self.text(x + 10, (y1 + y2) / 2 + size / 3, label, size=size, anchor="start", color=color)

    def swatch_legend(self, x, y, items, size=LABEL, gap=26):
        """items: list of (material_or_colour, label, kind) with kind in {'rect','outline','line','dash','dot','hollow'}."""
        cx = x
        for mat, label, kind in items:
            col = self.t["material_stroke"].get(mat) or (self.c(mat) if isinstance(mat, str) else "none")
            fillc = self.t["material"].get(mat) or self.tint(mat, 0.35)
            if kind == "rect":
                self.rect(cx, y - 11, 16, 12, fill=fillc, stroke=col, rx=2)
            elif kind == "outline":
                self.rect(cx, y - 11, 16, 12, fill="none", stroke=col, dash="3 2", rx=2)
            elif kind == "line":
                self.line(cx, y - 5, cx + 16, y - 5, col, 2.2)
            elif kind == "dash":
                self.line(cx, y - 5, cx + 16, y - 5, col, 1.6, dash="4 3")
            elif kind == "dot":
                self.circle(cx + 8, y - 5, 5, fill=col, stroke="surface", sw=1.5)
            elif kind == "hollow":
                self.circle(cx + 8, y - 5, 5, fill="none", stroke=col, sw=1.6)
            self.text(cx + 22, y, label, size=size, anchor="start", color="ink2")
            cx += 22 + text_width(label, size) + gap

    def label_box(self, x, y, s, *, size=LABEL, color="ink", fill="surface", stroke=None, pad=5, rx=3, weight=400,
                  anchor="middle", opacity=None):
        """Text on a background box sized from the heuristic metrics (with slack)."""
        w = text_width(s, size, weight) + 2 * pad
        h = size * 1.25 + 2 * pad
        bx = x - w / 2 if anchor == "middle" else (x if anchor == "start" else x - w)
        self.rect(bx, y - h / 2, w, h, fill=fill, stroke=stroke or "none", sw=1.0, rx=rx, opacity=opacity)
        tx = x if anchor == "middle" else (x + pad if anchor == "start" else x - pad)
        self.text(tx, y + size * 0.36, s, size=size, anchor=anchor, color=color, weight=weight, opacity=opacity)
        return w, h

    # -- arrows --------------------------------------------------------
    def _marker_id(self, color_hex: str) -> str:
        key = color_hex.lstrip("#")
        if key not in self._markers:
            mid = f"arr-{key}"
            self._markers[key] = mid
            self.defs.append(
                f'<marker id="{mid}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" '
                f'orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" fill="{color_hex}"/></marker>'
            )
        return self._markers[key]

    def _arrow_attrs(self, arrow, color_hex: str) -> str:
        if not arrow:
            return ""
        mid = self._marker_id(color_hex)
        out = ""
        if arrow in ("end", "both"):
            out += f' marker-end="url(#{mid})"'
        if arrow in ("start", "both"):
            out += f' marker-start="url(#{mid})"'
        return out

    # -- output --------------------------------------------------------
    def render(self) -> str:
        head = (
            f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {fmt(self.w)} {fmt(self.h)}" '
            f'width="{fmt(self.w)}" height="{fmt(self.h)}" role="img" aria-labelledby="t d" '
            f'font-family="{self.font}">\n'
        )
        if self.generator:
            head += f"<!-- generated by {self.generator}; edit the script, not this file -->\n"
        head += f'<title id="t">{esc(self.title)}</title>\n<desc id="d">{esc(self.desc)}</desc>\n'
        if self.styles:
            head += "<style>\n" + "\n".join(self.styles) + "\n</style>\n"
        if self.defs:
            head += "<defs>" + "".join(self.defs) + "</defs>\n"
        body = "\n".join(self.parts)
        text = head + body + "\n</svg>\n"
        if "<script" in text or re.search(r"<(animate|set|animateTransform|animateMotion)\b", text) or "<a " in text:
            raise ValueError("SVG must not contain <script>, SMIL or <a> elements")
        return text

    def write(self, path: Path) -> Path:
        path = Path(path)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(self.render(), encoding="utf-8")
        return path


# ---------------------------------------------------------------------------
# D5: the pupil grammar
# ---------------------------------------------------------------------------


def pupil(svg: Svg, cx: float, cy: float, r: float, *, fill: str | None = "conventional", sigma: float = 0.7,
          sigma_inner: float | None = None, poles: Sequence[tuple[float, float, float]] | None = None,
          coherent: float | None = None, label: str | None = None, rim: str = "ink2", cls: str | None = None,
          obscuration: float | None = None) -> None:
    """The pupil circle of radius NA (``r`` user units ≡ 1 pupil unit) with its source fill.

    ``fill``: ``"conventional"`` (disc of radius σ), ``"annular"`` (σ_inner … σ),
    ``"dipole"`` / ``"poles"`` (``poles`` = (sx, sy, radius) in pupil units), ``None``
    (empty pupil). ``coherent`` draws the near-coherent Gaussian core (radius in pupil units)
    in the coherent-core tint. ``obscuration`` draws a central stop of that fraction.
    """
    pf = svg.t["pupil"]
    with svg.group(cls):
        # outside the pupil: nothing (the support circle is drawn separately)
        svg.circle(cx, cy, r, fill="surface", stroke="none")
        if fill == "conventional":
            svg.circle(cx, cy, sigma * r, fill=pf["fill"], stroke=svg.t["pupil_stroke"]["fill"], sw=1.0)
        elif fill == "annular":
            si = (sigma_inner if sigma_inner is not None else sigma * 0.7) * r
            so = sigma * r
            d = (f"M{fmt(cx - so)},{fmt(cy)} a{fmt(so)},{fmt(so)} 0 1,0 {fmt(2 * so)},0 a{fmt(so)},{fmt(so)} 0 1,0 {fmt(-2 * so)},0 "
                 f"M{fmt(cx - si)},{fmt(cy)} a{fmt(si)},{fmt(si)} 0 1,1 {fmt(2 * si)},0 a{fmt(si)},{fmt(si)} 0 1,1 {fmt(-2 * si)},0")
            svg.add(f'<path d="{d}" fill="{pf["fill"]}" fill-rule="evenodd" stroke="{svg.t["pupil_stroke"]["fill"]}" stroke-width="1"/>')
        elif fill in ("dipole", "poles", "quadrupole"):
            for sx, sy, pr in (poles or ((sigma, 0.0, 0.15), (-sigma, 0.0, 0.15))):
                svg.circle(cx + sx * r, cy - sy * r, pr * r, fill=pf["fill"], stroke=svg.t["pupil_stroke"]["fill"], sw=1.0)
        if coherent:
            svg.circle(cx, cy, max(coherent * r, 2.5), fill=pf["coherent_core"], stroke=svg.t["pupil_stroke"]["coherent_core"], sw=1.0)
        if obscuration:
            svg.circle(cx, cy, obscuration * r, fill=svg.tint("ink", 0.85), stroke="none")
        svg.circle(cx, cy, r, fill="none", stroke=rim, sw=1.6)
        if label:
            svg.text(cx, cy + r + LABEL + 4, label, size=LABEL, color="ink2")


def support_circle(svg: Svg, cx: float, cy: float, r: float, sigma: float, *, label: bool = True,
                   cls: str | None = None) -> None:
    """The dashed (1+σ)·NA/λ mask-support circle (the engine's rule)."""
    rr = (1 + sigma) * r
    svg.circle(cx, cy, rr, fill="none", stroke=svg.t["pupil"]["support"], sw=1.2, dash="5 4", cls=cls)
    if label:
        svg.text(cx + rr * 0.72, cy - rr * 0.72, "(1+σ)·NA/λ", size=SMALL, color="ink2", anchor="start")


def order_color(svg: Svg, m: int, inside: bool = True) -> str:
    pf = svg.t["pupil"]
    if not inside:
        return pf["outside"]
    return {0: pf["order0"], 1: pf["order1"]}.get(abs(m), pf["order2"])


def orders(svg: Svg, cx: float, cy: float, r: float, positions: Sequence[tuple[int, float, float]], *,
           stem: bool = True, dot: float = 4.0, support: float | None = None, cls: str | None = None,
           label: bool = True) -> None:
    """Diffraction orders ``(m, x, y)`` in pupil units, coloured by |m| (0 blue · ±1 orange · ±2 aqua).

    An order outside the support circle (``support`` = 1+σ) or, without one, outside the
    pupil is drawn hollow in the muted ink.
    """
    lim = support if support is not None else 1.0
    with svg.group(cls):
        for m, x, y in positions:
            inside = math.hypot(x, y) <= lim + 1e-9
            col = order_color(svg, m, inside)
            px, py = cx + x * r, cy - y * r
            if stem:
                svg.line(cx, cy, px, py, col, 1.2, opacity=0.55 if inside else 0.35)
            if inside:
                svg.circle(px, py, dot, fill=col, stroke="surface", sw=1.2)
            else:
                svg.circle(px, py, dot, fill="none", stroke=col, sw=1.4)
            if label:
                sign = "+" if m > 0 else ("−" if m < 0 else "")
                svg.text(px, py - dot - 3, f"{sign}{abs(m)}", size=SMALL, color="ink2")


def wafer(svg: Svg, x: float, y: float, w: float, h: float = 10, *, label: str | None = "wafer", resist: float = 0.0,
          cls: str | None = None) -> None:
    """The wafer plane: a substrate bar (with an optional resist layer of height ``resist``)."""
    with svg.group(cls):
        if resist:
            svg.rect(x, y - resist, w, resist, "resist", sw=0.8)
        svg.rect(x, y, w, h, "substrate", sw=0.8)
        if label:
            svg.text(x + w / 2, y + h + LABEL + 2, label, size=LABEL, color="ink2")


# ---------------------------------------------------------------------------
# The chip (D2) — vector glyph marks, shared CHIP geometry
# ---------------------------------------------------------------------------


def _glyph_mark(svg: Svg, slug: str, x: float, y: float, em: float, color: str) -> None:
    """A 1 em vector mark standing in for the grade glyph: check-square, diamond, flask, map."""
    s = em * 0.9
    x0, y0 = x, y - s / 2
    if slug == "implemented":
        svg.rect(x0, y0, s, s, fill=color, stroke="none", rx=s * 0.2)
        d = f"M{fmt(x0 + s * 0.22)},{fmt(y0 + s * 0.53)} l{fmt(s * 0.2)},{fmt(s * 0.2)} l{fmt(s * 0.38)},{fmt(-s * 0.42)}"
        svg.add(f'<path d="{d}" fill="none" stroke="#ffffff" stroke-width="{fmt(s * 0.16)}" stroke-linecap="round" stroke-linejoin="round"/>')
    elif slug == "simplified":
        cx, cy = x0 + s / 2, y0 + s / 2
        svg.add(f'<path d="M{fmt(cx)},{fmt(y0)} L{fmt(x0 + s)},{fmt(cy)} L{fmt(cx)},{fmt(y0 + s)} L{fmt(x0)},{fmt(cy)} Z" fill="{color}" stroke="none"/>')
    elif slug == "theoretical":
        # a flask: neck, shoulders, round bottom
        nx0, nx1 = x0 + s * 0.38, x0 + s * 0.62
        d = (f"M{fmt(nx0)},{fmt(y0 + s * 0.05)} L{fmt(nx1)},{fmt(y0 + s * 0.05)} L{fmt(nx1)},{fmt(y0 + s * 0.38)} "
             f"L{fmt(x0 + s * 0.92)},{fmt(y0 + s * 0.82)} Q{fmt(x0 + s * 0.98)},{fmt(y0 + s * 0.98)} {fmt(x0 + s * 0.82)},{fmt(y0 + s * 0.98)} "
             f"L{fmt(x0 + s * 0.18)},{fmt(y0 + s * 0.98)} Q{fmt(x0 + s * 0.02)},{fmt(y0 + s * 0.98)} {fmt(x0 + s * 0.08)},{fmt(y0 + s * 0.82)} "
             f"L{fmt(nx0)},{fmt(y0 + s * 0.38)} Z")
        svg.add(f'<path d="{d}" fill="{color}" stroke="none"/>')
    else:  # planned: a folded map, three panels
        p = s / 3
        d = (f"M{fmt(x0)},{fmt(y0 + s * 0.2)} L{fmt(x0 + p)},{fmt(y0)} L{fmt(x0 + 2 * p)},{fmt(y0 + s * 0.2)} L{fmt(x0 + s)},{fmt(y0)} "
             f"L{fmt(x0 + s)},{fmt(y0 + s * 0.8)} L{fmt(x0 + 2 * p)},{fmt(y0 + s)} L{fmt(x0 + p)},{fmt(y0 + s * 0.8)} L{fmt(x0)},{fmt(y0 + s)} Z")
        svg.add(f'<path d="{d}" fill="{color}" stroke="none"/>')
        svg.line(x0 + p, y0, x0 + p, y0 + s * 0.8, "surface", s * 0.08)
        svg.line(x0 + 2 * p, y0 + s * 0.2, x0 + 2 * p, y0 + s, "surface", s * 0.08)


def chip(svg: Svg, x: float, y: float, grade: str, *, size: float = SMALL, anchor: str = "start", word: bool = True,
         outlined: bool = False, label: str | None = None, cls: str | None = None, opacity: float | None = None) -> float:
    """Draw one chip with its centre line at ``y``; returns the chip width.

    ``grade`` is a slug (``implemented`` …) or a glyph; with ``outlined=True`` it is a
    readiness rung word drawn as the outlined slate pill (never a grade colour). ``size``
    is the surrounding font size: the chip text is 0.9 × it (``CHIP['font_scale']``).
    """
    g = tokens.CHIP
    f = size * g["font_scale"]
    h = f * 1.25 + 2 * g["pad_y"] * f
    if outlined:
        text = label or grade
        tw = text_width(text, f, 600)
        w = g["pad_x"] * f + tw + g["pad_x"] * f
        bx = x if anchor == "start" else (x - w / 2 if anchor == "middle" else x - w)
        with svg.group(cls, opacity=opacity):
            svg.rect(bx, y - h / 2, w, h, fill="none", stroke="axis", sw=0.9, rx=h / 2)
            svg.text(bx + g["pad_x"] * f, y + f * 0.36, text, size=max(f, MIN_TEXT), anchor="start", color="ink2", weight=600)
        return w
    slug = grade if grade in tokens.GRADES else (tokens.grade_of(grade) or "implemented")
    colour = svg.t["grades"][slug]
    text = label if label is not None else (GLYPH_WORDS[slug] if word else "")
    em = f
    tw = text_width(text, f, 600) if text else 0.0
    w = 0.4 * f + em * 0.9 + (0.3 * f + tw if text else 0.0) + g["pad_x"] * f
    bx = x if anchor == "start" else (x - w / 2 if anchor == "middle" else x - w)
    with svg.group(cls, opacity=opacity):
        svg.rect(bx, y - h / 2, w, h, fill=tokens.mix(colour, svg.t["surface"], svg.t["chip_tint"]),
                 stroke=tokens.mix(colour, svg.t["surface"], svg.t["chip_border"]), sw=0.9, rx=h / 2)
        _glyph_mark(svg, slug, bx + 0.4 * f, y, em, colour)
        if text:
            svg.text(bx + 0.4 * f + em * 0.9 + 0.3 * f, y + f * 0.36, text, size=max(f, MIN_TEXT), anchor="start",
                     color=colour, weight=600)
    return w


# ---------------------------------------------------------------------------
# Animation: named keyframes, two layers, the reduced-motion rule
# ---------------------------------------------------------------------------

ANIM_START = "<!--huv-anim:start-->"
ANIM_END = "<!--huv-anim:end-->"
STILL_START = "<!--huv-still:start-->"
STILL_END = "<!--huv-still:end-->"


class Keyframes:
    """One ``@keyframes`` block authored in seconds against a total duration."""

    def __init__(self, name: str, total_s: float):
        self.name, self.total = name, total_s
        self.steps: list[tuple[float, str]] = []

    def at(self, t_s: float, css: str) -> "Keyframes":
        self.steps.append((max(0.0, min(1.0, t_s / self.total)), css))
        return self

    def pct(self, t_s: float) -> float:
        return 100.0 * max(0.0, min(1.0, t_s / self.total))

    def render(self) -> str:
        body = "".join(f"{fmt(round(p * 100, 3))}%{{{css}}}" for p, css in sorted(self.steps, key=lambda s: s[0]))
        return f"@keyframes {self.name}{{{body}}}"


class Anim:
    """Collects keyframes and animation rules for one two-layer SVG.

    ``loops`` ≤ 3 (``1`` for the hero); every rule gets ``animation-fill-mode: forwards``
    so the drawing rests on its designed state; the reduced-motion rule hides ``.anim``
    and shows ``.still`` inside the file itself.
    """

    def __init__(self, svg: Svg, *, loops: int = 3):
        if not 1 <= loops <= 3:
            raise ValueError("animations loop at most 3 times (the hero once)")
        self.svg, self.loops = svg, loops
        self.frames: list[Keyframes] = []
        self.rules: list[str] = []
        self._names: set[str] = set()

    def keyframes(self, name: str, total_s: float) -> Keyframes:
        if name in self._names:
            raise ValueError(f"duplicate keyframes {name!r}")
        self._names.add(name)
        kf = Keyframes(name, total_s)
        self.frames.append(kf)
        return kf

    def animate(self, selector: str, name: str, duration_s: float, *, delay_s: float = 0.0, timing: str = "linear",
                iterations: int | None = None, fill: str = "forwards") -> None:
        n = iterations if iterations is not None else self.loops
        self.rules.append(f"{selector}{{animation:{name} {fmt(duration_s)}s {timing} {fmt(delay_s)}s {n} {fill}}}")

    def hold(self, seconds: float) -> float:
        """Convenience for authors building timelines: returns ``seconds`` (documents a pause)."""
        return seconds

    @contextmanager
    def layer(self, which: str) -> Iterator[None]:
        """``with anim.layer("anim"):`` / ``("still")`` — the two groups ``still_of`` tells apart."""
        if which not in ("anim", "still"):
            raise ValueError("layer is 'anim' or 'still'")
        self.svg.add(ANIM_START if which == "anim" else STILL_START)
        with self.svg.group(which):
            yield
        self.svg.add(ANIM_END if which == "anim" else STILL_END)

    def install(self) -> None:
        """Emit the ``<style>`` block (call once, before ``render``)."""
        css = [".still{display:none}"]
        css += [kf.render() for kf in self.frames]
        css += self.rules
        css.append("@media (prefers-reduced-motion: reduce){.anim{display:none}.still{display:inline}}")
        self.svg.style("\n".join(css))


def still_of(svg_text: str) -> str:
    """The designed still of a two-layer animated SVG: the ``.anim`` layer removed, ``.still`` shown, no keyframes."""
    out = re.sub(re.escape(ANIM_START) + r".*?" + re.escape(ANIM_END), "", svg_text, flags=re.S)
    out = out.replace(STILL_START, "").replace(STILL_END, "")
    out = re.sub(r"<style>.*?</style>\n?", "<style>\n.still{display:inline}\n</style>\n", out, count=1, flags=re.S)
    return out


# ---------------------------------------------------------------------------
# A 2-D array as a PNG data URI (for an engine carpet behind an animation)
# ---------------------------------------------------------------------------


def _oklab_to_rgb(L: float, a: float, b: float) -> tuple[float, float, float]:
    l_ = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3
    m_ = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3
    s_ = (L - 0.0894841775 * a - 1.2914855480 * b) ** 3
    r = 4.0767416621 * l_ - 3.3077115913 * m_ + 0.2309699292 * s_
    g = -1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_
    bb = -0.0041960863 * l_ - 0.7034186147 * m_ + 1.7076147010 * s_

    def enc(c: float) -> float:
        c = min(1.0, max(0.0, c))
        return 12.92 * c if c <= 0.0031308 else 1.055 * c ** (1 / 2.4) - 0.055

    return enc(r), enc(g), enc(bb)


def field_ramp(n: int = 256) -> list[tuple[int, int, int]]:
    """The OKLab field ramp (dark = 0, light = max) as ``n`` 8-bit RGB triples — pure Python."""
    anchors = [(L, C * math.cos(math.radians(h)), C * math.sin(math.radians(h))) for L, C, h in tokens.FIELD_ANCHORS_OKLCH]
    k = len(anchors) - 1
    out = []
    for i in range(n):
        t = i / (n - 1) * k
        j = min(int(t), k - 1)
        f = t - j
        lab = tuple(anchors[j][c] * (1 - f) + anchors[j + 1][c] * f for c in range(3))
        r, g, b = _oklab_to_rgb(*lab)
        out.append((round(r * 255), round(g * 255), round(b * 255)))
    return out


def png_bytes(rows: Sequence[bytes], width: int, height: int) -> bytes:
    """Encode 8-bit RGB scanlines (``width*3`` bytes each) as a PNG (stdlib zlib)."""

    def chunk(tag: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    raw = b"".join(b"\x00" + r for r in rows)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def embed_png(array, cmap: Sequence[tuple[int, int, int]] | None = None, size: tuple[int, int] | None = None,
              vmin: float | None = None, vmax: float | None = None) -> str:
    """A 2-D array (rows = y, first row = top) as ``data:image/png;base64,…`` through ``cmap`` (default: the field ramp).

    ``size`` = (width, height) resamples by nearest neighbour; numpy is imported here.
    """
    import numpy as np  # noqa: PLC0415

    a = np.asarray(array, dtype=float)
    if a.ndim != 2:
        raise ValueError("embed_png expects a 2-D array")
    lo = float(a.min()) if vmin is None else vmin
    hi = float(a.max()) if vmax is None else vmax
    norm = np.clip((a - lo) / (hi - lo if hi > lo else 1.0), 0.0, 1.0)
    if size:
        w, h = size
        ys = (np.arange(h) * a.shape[0] / h).astype(int)
        xs = (np.arange(w) * a.shape[1] / w).astype(int)
        norm = norm[ys][:, xs]
    ramp = list(cmap) if cmap is not None else field_ramp()
    idx = (norm * (len(ramp) - 1)).round().astype(int)
    lut = np.asarray(ramp, dtype=np.uint8)
    rgb = lut[idx]
    rows = [rgb[i].tobytes() for i in range(rgb.shape[0])]
    data = png_bytes(rows, rgb.shape[1], rgb.shape[0])
    return "data:image/png;base64," + base64.b64encode(data).decode("ascii")


# ---------------------------------------------------------------------------
# The writer
# ---------------------------------------------------------------------------


def _crop_svg(svg_text: str, box: tuple[float, float, float, float]) -> str:
    """The same SVG with its viewBox (and width/height) set to ``box`` = (x, y, w, h)."""
    x, y, w, h = box
    out = re.sub(r'viewBox="[^"]*"', f'viewBox="{fmt(x)} {fmt(y)} {fmt(w)} {fmt(h)}"', svg_text, count=1)
    out = re.sub(r'(<svg[^>]*?)\swidth="[^"]*"', rf'\1 width="{fmt(w)}"', out, count=1)
    out = re.sub(r'(<svg[^>]*?)\sheight="[^"]*"', rf'\1 height="{fmt(h)}"', out, count=1)
    return out


def write_pair(name: str, draw: Callable[[str], str], out_dir: Path, *, narrow: Callable[[str], str] | None = None,
               crops: dict[str, tuple[float, float, float, float]] | None = None, still: bool = True) -> list[Path]:
    """Write ``<name>-light.svg`` and ``<name>-dark.svg`` (plus stills, narrow twins and crops).

    ``draw(theme) -> str`` returns the SVG text for one theme. A ``name`` ending in
    ``-anim`` whose SVG holds the two layers also gets ``<name minus -anim>-still-<theme>.svg``
    from :func:`still_of`. ``narrow(theme) -> str`` writes ``<base>-narrow-<theme>.svg``.
    ``crops`` = {crop: (x, y, w, h)} writes ``<name>-<crop>-<theme>.svg`` as sub-viewBoxes.
    Output is deterministic (no timestamps), so the nightly can diff it.
    """
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    base = name[: -len("-anim")] if name.endswith("-anim") else name
    paths: list[Path] = []
    for theme in ("light", "dark"):
        text = draw(theme)
        p = out_dir / f"{name}-{theme}.svg"
        p.write_text(text, encoding="utf-8")
        paths.append(p)
        if still and name.endswith("-anim") and ANIM_START in text:
            sp = out_dir / f"{base}-still-{theme}.svg"
            sp.write_text(still_of(text), encoding="utf-8")
            paths.append(sp)
        if narrow is not None:
            npth = out_dir / f"{base}-narrow-{theme}.svg"
            npth.write_text(narrow(theme), encoding="utf-8")
            paths.append(npth)
        for crop, box in (crops or {}).items():
            cp = out_dir / f"{name}-{crop}-{theme}.svg"
            cp.write_text(_crop_svg(text, box), encoding="utf-8")
            paths.append(cp)
    return paths


def size_kb(path: Path) -> float:
    return Path(path).stat().st_size / 1024
