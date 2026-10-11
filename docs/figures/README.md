# Figure sources

Every image under `docs/assets/images/` is produced by a script in this directory and registered
as a spec; none is hand-drawn or copied from elsewhere. The one exception the registry knows about
is the four raw GUI captures, registered as `kind="capture"` entries whose sha256 is pinned
(`gui/make_gui_media.py`), so `--check` verifies them instead of regenerating them.

| Directory | What it makes | Output |
|---|---|---|
| `sim/` | Figures computed by the simulator itself (`FigureSpec`s): imaging, masks, sources, deep lithography, optimization, the narrative pages' simulator figures, and the raster animations | `docs/assets/images/sim/` (`site/` narrative, `anim/` animations) |
| `diagrams/` | Scripted SVG (`AssetSpec`s, svgkit): the Spectrum Strip (D1), the pipeline stage strip, architecture layers and power planes (D2–D4), the physics / process / source schematics | `docs/assets/images/diagrams/` |
| `reference/` | The matrix parser, the legend writer (D6) and the matrix tile map (D5) | `docs/assets/images/reference/` |
| `history/`, `nodes/`, `future/` | Data charts and schematics for the narrative sections; numbers come from cited datasets | `docs/assets/images/{history,nodes,future}/` |
| `gui/`, `playground/` | Annotated real captures; the playground's static fallbacks | `docs/assets/images/{gui,playground}/` |

Shared modules at this level:

| Module | Role |
|---|---|
| `tokens.py` | The one palette / ink / typography source (brand, grades, section hues, chart chrome, categorical series, ramps, the D5 pupil colours, material fills, provenance marks, fonts, the chip geometry). `python docs/figures/tokens.py --css` prints the token block `docs/stylesheets/extra.css` carries between `/* huv-tokens:start */` and `/* huv-tokens:end */`; `docs/hooks/test_tokens.py` pins the two byte for byte. No numpy or matplotlib at import time. |
| `svgkit.py` | The SVG builder (`Svg`), the D5 pupil grammar (`pupil`, `orders`, `support_circle`, `wafer`), the chip (`chip`), the two-layer animation helper (`Anim`, `still_of`), `embed_png` and `write_pair`. Standard library only. `Svg.text(…, halo=True)` paints a surface-coloured outline behind a label (`paint-order="stroke"`) so a leader line or tick passing under it never cuts the glyphs; draw leaders first, labels last, and give labels near leaders a halo. |
| `registry.py` | `AssetSpec`, the registry, the file-naming contract, `md_embed()` (the five embed forms), `check_embeds()`, `sync_embeds()`, `catalog()`, `export_json()`. `sim/common.py`'s `FigureSpec` shares the contract. |
| `check_figures.py` | Checks both registries (`--check`), prints embeds (`--catalog`), rewrites them (`--sync-embeds --pages …`), lists the platform-independent subset (`--platform-independent [--regenerate]`), exports a diagnostic JSON (`--export-registry`). |
| `reference/matrix_parse.py` | The one parser of `docs/capability-matrix.md`; every consumer that copies a grade reads it here. |

## Renderer of record

The figure machine is the renderer of record for every PNG, APNG, GIF and matplotlib SVG. CI never
renders a platform-dependent figure: the nightly `docs-figures.yml` runs `make_all.py --verify`
(every `compute()` and the physical asserts inside it, nothing drawn) and regenerates and diffs
only the platform-independent subset (`check_figures.py --platform-independent --regenerate`:
the svgkit SVGs, `diagrams/data/presets.json`, the marker tables through the writers' `--write`).

| | Version |
|---|---|
| Machine | macOS (Darwin 23, arm64) — the maintainer's figure machine |
| Python | `/opt/homebrew/bin/python3` 3.14.7 (the project venv runs the same interpreter) |
| numpy | 2.4.3 |
| matplotlib | 3.10.8 |
| Pillow | 12.1.1 |
| Font | Helvetica Neue (installed), then Helvetica, Arial, DejaVu Sans (`tokens.FONT_MPL`); scripted SVGs set `tokens.FONT_SVG` and are outlined nowhere |

Anyone who re-renders under other versions updates this table in the same commit. The
integrator's reproducibility check (`scripts/docs_gates.sh <worktree> --figures`, local only)
regenerates every generator with these versions and expects `git diff --exit-code docs/assets/images`
to pass; a diff that is only PNG metadata or an APNG timestamp means a writer is non-deterministic —
fix the writer, never commit a re-rendered binary to hide it. `requirements-figures.txt` pins the
three libraries for the nightly's ubuntu runner (so the figure modules import there), not as a
renderer of record.

## Simulator figures (`sim/`)

### Build the extension, then run `make_all.py`

The scripts import the compiled module `highuvlith._native`. Build it in release mode, because a debug
build is more than ten times slower:

```bash
maturin develop --release -m crates/highuvlith-py/Cargo.toml   # or: pip install . from a checkout
python docs/figures/sim/make_all.py                            # every figure -> docs/assets/images/sim/
```

If you only have the built `.so` next to `python/highuvlith/`, set `PYTHONPATH=python` instead of
installing. A full release-build run takes a few minutes.

| Option | Effect |
|---|---|
| `--only NAME …` | Figures whose name contains `NAME` (or matches a glob), e.g. `--only talbot liga` |
| `--group G` | One group: `imaging`, `masks`, `sources`, `deep`, `optim`, `site`, `home`, `anim`, `materials`, `frontends`, `deep_b`, `sources_b` |
| `--fast [--frames N]` | Coarse sampling for a smoke test (animations capped at `N` frames). Writes to `docs/figures/sim/_fast/`, never over the published images |
| `--redraw` | Reuse the cached simulation data in `_cache/` and only redraw (for style or caption work) |
| `--require CAP …` | A missing capability (`cli`, `illumination`, …) is an error, not a skip (the nightly passes `--require cli`) |
| `--verify` | Run every selected `compute()` and its physical asserts; draw nothing, write nothing — the only form CI runs |
| `--out DIR` | Another output directory |
| `--list` | Name, group, kind and the pages that embed each figure |
| `--catalog [--pages …]` | Print the Markdown embed snippet of every figure for every page (page groups paste these) |
| `--check [--strict\|--tolerant] [--pages …]` | Both registries: variants exist, sizes within the per-kind caps, SVG hygiene, every listed page embeds the asset with the registry caption, every caption `Model:` clause names a matrix row with its headline glyph, the legend and matrix-map checks, the readiness check |
| `--sync-embeds --pages <paths>` | Rewrite the existing embeds on **those pages** from the current alt and caption. Without `--pages` the command is refused; `--site-wide` is the integrator's flag, run once after every branch is merged |
| `--tables [--only NAME] [--write]` | Print the marker tables specs declare through `extra["table"]`; `--write` replaces them in place between `<!-- <marker>:start -->` / `<!-- <marker>:end -->` (integrator and nightly only) |
| `--export-registry [PATH]` | The merged registry as JSON with pixel sizes — a diagnostic; `docs/figures/registry.json` is gitignored and nothing reads it |

A figure whose API is missing from the build is skipped with a note, not failed. Each figure declares what
it needs (`requires=`), for example `SourceConfig.with_illumination` / `pupil_fill` / `spectrum` for the
illumination-shape and source-gallery figures, `cli` for the CLI output gallery. A `FIGURE_MODULES` entry
whose file is absent prints `note  <mod>.py not present (lands with <group>)` and is skipped (an error in
strict mode); so does the readiness check while `future/check_readiness.py` / `readiness.csv` are absent.

### Who runs what

* **Visual groups** edit `alt`, `caption`, `pages=` and `extra["model_rows"]` in their spec files and run
  `make_all.py --check` (tolerant: "page X does not embed Y" is the expected warning until the page lands) and
  `check_figures.py --catalog --pages …` to preview embed text. They never run `--sync-embeds` and never edit a page.
* **Page groups** paste embeds from `--catalog` and run `make_all.py --sync-embeds --pages <exactly their pages>`.
* **The integrator** runs `--sync-embeds --site-wide` once and the marker-table writers with `--write`.

### How a figure is defined

Every figure is a `FigureSpec` (`sim/common.py`) registered in one of the `fig_*.py` modules. A spec has
these parts:

- `compute(fast)` runs the simulator once and returns plain data (this is what gets cached). **Physical
  asserts live here**, never in `draw`, so `--verify` exercises them without rendering.
- `draw(data, theme)` draws one theme. `make_all` calls it twice and writes `<name>-light` and `<name>-dark`:
  a matplotlib figure becomes a 2× optimized PNG (or an SVG) with the source script in the PNG metadata; an SVG
  *string* (svgkit) is written verbatim, and for a two-layer animation the designed still is derived from it;
  a *list of figures* (`fmt="apng"`) becomes the APNG + still + optional GIF set (below).
- `alt` and `caption` are the canonical text of the embed. The caption states what was simulated, with
  which parameters, and ends in the Model trailer (below). **Edit the caption here; the page group that owns
  the page runs `--sync-embeds --pages`; don't hand-edit the copies in the pages.**
- `pages` lists where the figure is embedded. `--check` enforces it. `kind` picks the size cap (`figure` by
  default; `sheet` for contact sheets; `anim` is set automatically for animations).
- `extra` keys: `gif` (also write a README GIF), `crops` (derived `<name>-<crop>` specs, below), `model_rows`
  (caption label → matrix row), `table` (a marker table for `--tables`), `cache_bust`, `max_kb` +
  `max_kb_reason` (a documented size-cap override, printed in every `--check` report), `width` (README
  `<img width>`), `readme_fmt` (a different format for the README embed), `png_palette` (quantize a
  flat-colour PNG).

`sim/style.py` is the shared matplotlib language: palette, fonts, line weights, light/dark themes, the
OKLab field colormap, helpers for limit lines, end labels, colorbars and provenance marks — all read from
`tokens.py`. Figures use transparent backgrounds with theme-specific ink. Pages show the right
variant through the `#gh-light-mode-only` / `#gh-dark-mode-only` URL fragments: MkDocs Material and
github.com both honour them.

### Narrative, schematic and animated assets (`AssetSpec`)

Everything that is not an engine figure is an `AssetSpec` (`registry.py`) registered by the script that
writes it — `register(AssetSpec(name, kind, alt, caption, pages, generator, subdir, fmt, animated, narrow, …))`
— with the same `alt`/`caption`/`pages` contract, `kind ∈ {figure, sheet, strip, schematic, anim, capture}`
and `platform_independent=True` for svgkit output (the nightly regenerates and diffs exactly those files).
`check_figures.py` imports the generator modules by path (the list `ASSET_MODULES`; an absent one prints a
note) and checks them together with the `FigureSpec`s. The same `--catalog` / `--sync-embeds` / `--check`
commands cover both registries.

### The embed forms

`registry.md_embed(spec, page)` is the only writer of page embeds:

1. a static figure on a docs page — `<figure markdown="span">`, blank line, the two Markdown images with
   `#gh-light-mode-only` / `#gh-dark-mode-only`, blank line, the HTML `<figcaption>`;
2. a strip with a narrow twin (`narrow=True`) — one `<picture>` per variant with a `(max-width: 600px)`
   source and `class="huv-no-zoom"` on the `<img>` (kept out of the lightbox; the class is stripped on GitHub);
3. an animated raster (`fmt="apng"`) — one `<picture>` per variant with a `(prefers-reduced-motion: reduce)`
   source pointing at the still, `class="huv-no-zoom huv-anim"` and `data-huv-still`;
4. an animated SVG — form 1 (or form 2 when strip-shaped); the file name holds `-anim-`, the still `-still-`;
5. the README — `<picture>` with `prefers-color-scheme` sources; animated assets add the reduced-motion
   sources first and use the GIF; `width` is the only sizing.

### File naming and budgets

`<root>/<subdir>/<name>-{light,dark}.<fmt>`; animations `<name>-anim-{light,dark}.{svg,png}` with the
designed still `<name>-still-{light,dark}.{svg,png}` (+ `.gif` for README assets); narrow twins
`<name>-narrow-{light,dark}.svg`; crops `<parent>-<crop>-{light,dark}.<fmt>`; captures `<name>.png`. PNGs at
2×, transparent. Scripted SVGs: transparent, `font-family` = `tokens.FONT_SVG`, `<title>` + `<desc>` = alt,
viewBox 860–960 wide (strips 960 × 180–300; narrow twins 600 × 360), text ≥ 12.5 units, no `<a>`, no
`<script>`, no SMIL.

Size caps per variant (`check_figures.py --check` errors above a cap and warns at 80 %): PNG `figure` 400 KB;
`sheet` / `capture` 500 KB; scripted SVG (`schematic`, `strip`) 60 KB, narrow twins 40 KB; matplotlib SVG
120 KB; animated SVG 120 KB; APNG 900 KB; GIF 1 MB (README only, ≤ 48 frames at 1× and 800 px wide).

### Animations

* **Animated SVG** — two layers (`.anim` / `.still` groups, `svgkit.Anim`), CSS `@keyframes` only, the
  in-file `prefers-reduced-motion` rule, ≤ 3 loops then rest on the designed state (the home hero plays once),
  no `<script>`, no SMIL (`--check` refuses `<animate`, `<set`, `<animateTransform`, `<script>`). The still is
  derived by `svgkit.still_of`.
* **Raster animation** (`fmt="apng"`) — `draw()` returns a list of matplotlib figures (≤ 48, drawn at 1× and
  800 px wide); `style.write_frames` writes the APNG (`-anim-<theme>.png`, one 8-bit palette shared by every
  frame, frames flattened onto the theme surface, `loop=0`), the resting frame (`still_frame`) at 2× as
  `-still-<theme>.png` (transparent) and, with `extra={"gif": True}`, a README GIF (≤ 128 colours, no
  dithering). Pillow is imported inside the writer only, so `--check` and `--verify` never need it. A GIF over
  1 MB prints a note: reduce the frame list to 36 before touching resolution or colours.
* The page swaps animated files for their stills under reduced motion, the footer toggle and print
  (`docs/javascripts/layout.js`); the README `<picture>` lists the reduced-motion sources first.

### Crops

A parent spec declares `extra["crops"] = {"<crop>": dict(pages=…, alt=…, caption=…, kind=…, box=…)}`;
`register()` expands each into a derived spec `<parent>-<crop>` with its own pages, alt, caption and kind. The
parent's render writes `<parent>-<crop>-{light,dark}.<fmt>` from the same output (`box` = a pixel box of the
2× PNG, or a sub-viewBox for SVG). A crop page is listed only on the derived spec; `--catalog` prints the
derived embed under its own name.

### The caption grammar and the matrix check

Captions are HTML inside `<figcaption>` (github.com would print Markdown asterisks): a bold lead sentence,
one or two detail sentences, then the trailer in `<em>…</em>` — exactly one of `Model: <capability> <glyph>
[(<qualifier>)] · …` (grades copied from the matrix rows named) or a no-badge trailer (`Schematic, not to
scale.`, `Closed form; not the engine.`, `Data chart from <file>; no model badge.`, `Annotated real capture;
nothing is mocked.`, `Drawn from the matrix itself.`, `Chips copied from the matrix.`, …). Build provenance
lives in the registry and the PNG `Source` chunk / SVG comment, not in the caption.

`check_figures.check_captions()` — run by `make_all.py --check` in every mode and wrapped by
`tests/docs/test_caption_badges.py` — splits the trailer on ` · `, takes each clause's label (the text before
its first glyph) and resolves it in order: (1) the exact Capability text of a matrix row; (2) the spec's own
`extra["model_rows"][label]` (a row name or a list); (3) `matrix_parse.CAPTION_ALIASES`, frozen to the 47
captions that existed when the redesign started. A label that resolves nowhere, a glyph that differs from the
row's headline, or a glyph in a no-badge trailer is an error. Nobody edits another group's alias.

### Data and provenance

- Everything except the data charts comes from the simulator at the parameters given in the caption.
- `sim/data/source_power_vs_wavelength.csv` lists the published source powers on the landscape figure, one
  citation and URL per row (see `sim/data/README.md`); the Future section's landscape charts read the same file.
  It is the only copy of this dataset. `history/litho_tools.csv` is the only tool dataset;
  `future/readiness.csv` the only readiness dataset.
- `diagrams/data/presets.json` holds every `SourceConfig` preset's wavelength, how it is set and the family's
  matrix row; `make_spectrum_strip.py --refresh` rebuilds it from the engine and `--check` compares.
- Provenance marks follow `tokens.PROVENANCE` (measured filled · calibrated ringed · reported 60 % · assumed
  hatched · announced hollow · projection dashed) with a legend on every chart that uses them.
- The `masks-ls-geometry-fix` figure re-draws the v1 rasterization from the v1 source code. Its caption
  says that it is re-drawn, not simulated.

### Where the picture is seen

| Surface | Light / dark | Animation | Lightbox |
|---|---|---|---|
| The site (Material) | the `#gh-*` fragment or the `<picture>` source; the reader's scheme toggle | CSS keyframes play; `layout.js` swaps to the still under reduced motion, the footer toggle and print | glightbox on Markdown-image figures; `huv-no-zoom` strips and animations excluded |
| github.com | `#gh-*` fragments honoured; README `<picture>` follows the OS scheme | SVG keyframes play; GIFs loop; the in-file reduced-motion rule applies | none |
| Apps / previews | whichever variant the renderer picks | usually the still | none |

### Adding a figure

1. Write `compute` and `draw` in the matching `fig_*.py` (or a `generator()` in the matching schematic
   script). Use `style` / `svgkit` helpers only, with colours from `tokens`, no ad-hoc values.
2. `register(FigureSpec(...))` / `register(AssetSpec(...))` with alt text, a caption that ends in the Model
   trailer (resolving through `extra["model_rows"]` where a label abbreviates a row), `kind` and `pages`.
3. Run `make_all.py --only <name>` (or the script), look at both variants, then hand the `--catalog` snippet to
   the page group that owns the page (or paste it yourself when you own the page) and run
   `make_all.py --sync-embeds --pages <that page>`.
4. Run `make_all.py --check`, `pytest tests/docs/test_caption_badges.py` and `mkdocs build --strict`.

## The legend, the tile map and the marker tables

* `reference/make_legend.py` prints the four-cell grade legend, led by `<!-- huv-table: stack -->` so it stacks on phones (`--check` compares the three copies in
  `docs/index.md`, `docs/capability-matrix.md` and `README.md` between `<!-- huv-legend:start -->` /
  `<!-- huv-legend:end -->`; `--write` replaces them — integrator and nightly only).
* `reference/make_matrix_map.py` draws one tile per matrix row (`--svg-only` is what the nightly regenerates;
  the README PNG pair is platform dependent); `--check` asserts tile count = rows and counts = legend.
* `history/make_history_figures.py --table`, `future/check_readiness.py --table` and `make_all.py --tables`
  print the other marker tables as column-0 GFM; page groups paste the printed text, `--write` is the
  integrator's and the nightly's.
