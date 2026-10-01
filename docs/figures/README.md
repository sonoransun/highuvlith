# Figure sources

Every image under `docs/assets/images/` is produced by a script in this directory.
None is hand-drawn or copied from elsewhere.

| Directory | What it makes | Output |
|---|---|---|
| `sim/` | Figures computed by the simulator itself: imaging, masks, sources, deep lithography, optimization, and the simulator figures on the History / Nodes / Future pages | `docs/assets/images/sim/` (`site/` for the narrative pages) |
| `history/`, `nodes/`, `future/` | Data charts and schematics for the narrative site sections. The numbers come from sourced datasets that are cited in each script | `docs/assets/images/{history,nodes,future}/` |

## Simulator figures (`sim/`)

### Build the extension, then run `make_all.py`

The scripts import the compiled module `highuvlith._native`. Build it in release mode, because a debug
build is more than ten times slower:

```bash
maturin develop --release -m crates/highuvlith-py/Cargo.toml   # or: pip install . from a checkout
python docs/figures/sim/make_all.py                            # every figure -> docs/assets/images/sim/
```

If you only have the built `.so` next to `python/highuvlith/`, set `PYTHONPATH=python` instead of
installing. A full release-build run takes under a minute on a laptop (well inside the 10-minute budget).

| Option | Effect |
|---|---|
| `--only NAME …` | Figures whose name contains `NAME` (or matches a glob), e.g. `--only talbot liga` |
| `--group G` | One group: `imaging`, `masks`, `sources`, `deep`, `optim`, `site` |
| `--fast` | Coarse sampling for a smoke test. Writes to `docs/figures/sim/_fast/`, never over the published images |
| `--redraw` | Reuse the cached simulation data in `_cache/` and only redraw (for style or caption work) |
| `--out DIR` | Another output directory |
| `--list` | Name, group and the pages that embed each figure |
| `--catalog` | Print the Markdown embed snippet of every figure for every page |
| `--check` | Verify that both variants exist and that every listed page embeds them |
| `--sync-embeds` | Rewrite the existing embeds in the pages from the current alt text and caption |

A figure whose API is missing from the build is skipped with a note, not failed. Each figure declares what
it needs (`requires=`), for example `SourceConfig.with_illumination` / `pupil_fill` / `spectrum` for the
illumination-shape and source-gallery figures.

### How a figure is defined

Every figure is a `FigureSpec` (`sim/common.py`) registered in one of the `fig_*.py` modules. A spec has
these parts:

- `compute(fast)` runs the simulator once and returns plain data (this is what gets cached).
- `draw(data, theme)` draws one theme. `make_all` calls it twice and writes `<name>-light.png` and
  `<name>-dark.png` at 2× resolution, optimized, with the source script recorded in the PNG metadata.
- `alt` and `caption` are the canonical text of the embed. The caption states what was simulated, with
  which parameters, and the capability-matrix badge of each model involved. **Edit the caption here and
  run `--sync-embeds`; don't hand-edit the copies in the pages.**
- `pages` lists where the figure is embedded. `--check` enforces it.

`sim/style.py` is the one shared visual language: palette, fonts, line weights, light/dark themes, the
OKLab field colormap, and helpers for limit lines, end labels and colorbars. The palette is the same one
`highuvlith.viz` uses. Figures use transparent backgrounds with theme-specific ink. Pages show the right
variant through the `#gh-light-mode-only` / `#gh-dark-mode-only` URL fragments: MkDocs Material and
github.com both honour them.

### Data and provenance

- Everything except the source landscape comes from the simulator at the parameters given in the caption.
- `sim/data/source_power_vs_wavelength.csv` lists the published source powers on the landscape figure, one
  citation and URL per row (see `sim/data/README.md`). The simulator's own preset powers are plotted next to
  them and are labelled as such.
- The `masks-ls-geometry-fix` figure re-draws the v1 rasterization from the v1 source code. Its caption
  says that it is re-drawn, not simulated.

### Adding a figure

1. Write `compute` and `draw` in the matching `fig_*.py`. Use `style` helpers only, with no ad-hoc colours.
2. `register(FigureSpec(...))` with alt text, a caption that carries the model badges, and `pages`.
3. Run `make_all.py --only <name>`, look at both PNGs, then insert `make_all.py --catalog --only <name>`
   into the page as a small hunk after a heading.
4. Run `make_all.py --check` and `mkdocs build --strict`.
