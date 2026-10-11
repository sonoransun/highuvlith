# `scripts/cdp/` — the headless QA driver and the site QA library

Everything `scripts/qa_site.mjs` and `scripts/docs_gates.sh --qa` need to look at the built
site the way a reader does, with no npm install: Node ≥ 22 (global `fetch` + `WebSocket`) and
a Chrome binary (`$CHROME`, the usual install paths, or `google-chrome` / `chromium` on `PATH`).

| File | What it is |
|---|---|
| `driver.mjs` | A minimal Chrome DevTools Protocol session: `launch()` → `navigate`, `settle` (load + MathJax + mermaid + images), `evaluate`, `setMedia` (dark / reduced-motion / print emulation), `setViewport`, `screenshot` (full page), `close`. Console errors, uncaught exceptions and failed requests are collected in `page.log`. |
| `cdp.mjs` | The script-driven CLI from the UNDERSTAND-phase baseline, vendored: `node scripts/cdp/cdp.mjs script.json` with `goto / wait / settle / eval / shot / scroll / media / mouse / click / key` steps (the header documents the syntax). Use it for ad-hoc screenshots of your pages. |
| `qa_lib.mjs` | Pure helpers: the 13-page sample, the budgets of DESIGN §10.2, the in-page probe (`probeInPage`), the finding rules (`findingsFromProbe`, `pairFindings`), and the github.com preview (`githubPreview`, markdown-it GFM). No Chrome needed; covered by `qa_site.test.mjs`. |
| `qa_site.test.mjs` | `node --test scripts/cdp/qa_site.test.mjs` — the pure helpers on fixtures (gate 4 runs it). |
| `vendor/markdown-it.min.js`, `vendor/LICENSE` | markdown-it 14.1.0 (MIT), the renderer behind `--github-preview`. |

## `scripts/qa_site.mjs`

```bash
node scripts/qa_site.mjs --site http://127.0.0.1:8793/highuvlith --modes light,dark,print,phone --sample --out <dir>  # PR CI (~1 min)
node scripts/qa_site.mjs --site <url> --all --github-preview --out <dir>       # nightly / INT: every sitemap page, screenshots per page × mode
node scripts/qa_site.mjs --github-preview README.md CONTRIBUTING.md             # no server: markdown-it GFM render + structure checks
node scripts/qa_site.mjs --github-preview docs/future/index.md                  # any docs page (P4's details-table check)
node scripts/qa_site.mjs --self-test                                            # fixture pages prove every assert fires
```

`--site` is the URL at which `sitemap.xml` is served (the gate serves the mktemp parent dir, so
the site root is `…/highuvlith`; CI serves `site/` itself, so the root is `http://127.0.0.1:8793`).
`--pages /a/,/b/` restricts the walk; `--shots` saves screenshots even when a page passes;
`--strict` / `--tolerant` override `HUV_QA_STRICT`; `--settle <ms>` adds to the settle wait.
Exit 1 on any error, 2 on a usage or driver problem. The report is `<out>/qa-report.md` (+ `.json`):
a page × mode table, the errors, the warnings, and a per-rule count.

### Two classes of findings (DESIGN §10.4, m2)

**Structural — errors in every mode**

| rule | assert |
|---|---|
| `overflow-x` | `html.scrollWidth <= html.clientWidth` (no horizontal page scroll), in every mode incl. 390 px |
| `mathjax-error` | `mjx-merror` count = 0 |
| `broken-image` | no `<img>` that is complete with `naturalWidth` 0 (and whose URL does not answer 200 to a HEAD) |
| `pair-visible` | exactly one visible `<img>` per `#gh-light-mode-only`/`#gh-dark-mode-only` (or `#only-light`/`#only-dark`) pair — in normal mode, under reduced-motion emulation, with the footer motion toggle pressed, and in print emulation; a pair inside a closed `<details>` or inside `figure[data-huv-fallback]` (a calculator's fallback figure, hidden on mount by `playground.js`) is hidden by design and skipped |
| `pair-gallery` | the hidden variant's `a.glightbox[data-gallery]` differs from the visible one's (when the lightbox wraps them) |
| `lightbox-gallery` | no visible `a.glightbox` lacks a `data-gallery` (GLightbox opens a gallery-less anchor on the page-wide list) or shares its gallery with a hidden anchor — in the same four passes as `pair-visible`; `page_kit.py` gives single-variant images `data-gallery="any"` (DESIGN §4.6: the gallery a reader opens holds only what they can see) |
| `focus-zero` | no rendered focusable element inside `<article>` (`a[href]`, `button`, form controls, `[tabindex]`) has a 0x0 box — an invisible keyboard tab stop (the off-scheme variant's lightbox anchor, before `extra.css` hid the anchor with its image); pymdownx's empty per-line `a#__codelineno-*` anchors are upstream's and skipped |
| `overflow-clipped` | no `div.arithmatex`, `.huv-diagram`, `.md-typeset__scrollwrap` or `pre > code` whose content is wider than its box while the box cannot scroll (content cut off) |
| `overflow-cue` | a scrollable watched block lacks `data-huv-overflow="true"` — an **error once `layout.js` is loaded** (the cue machinery exists), a page-form warning before that (the baseline has 7 + 3 scrolling equations and no `layout.js`) |
| `fence-marker` | every python code block has `<!-- verify-example -->` or `<!-- example-skip: … -->` directly before it (also inside admonitions, where Python-Markdown wraps the comment in a `<p>`) |
| `calc-mounted` | `[data-huv-mounted]` count = `[data-huv-calc]` count |
| `details-leak` | no `details` element whose text (outside `pre`/`code`) contains `|---`, `](` or `![` (a collapsed block whose Markdown leaked as text, DESIGN B1) |

**Page-form — warnings unless `HUV_QA_STRICT=1` or `--strict`**

| rule | assert |
|---|---|
| `status-chip` | every `p.huv-status` holds a `.huv-chip` |
| `section-marker` | `.huv-section[data-huv-section]` exists and equals the section expected from the path (`history`, `nodes`, `future`, `reference` for the Reference/Develop pages, `home`, else `simulator` — the Playground resolves to `simulator` by design, §4.2) |
| `hero-fold` | on a page that declares a hero (`.huv-lead + figure`, `.huv-lead + p + figure`, `.huv-lead + .huv-actions + figure`, or a `.huv-physics figure`) the figure starts within 900 px at 1280 px / two screens at 390 px |
| `lazy-loading` | every `<img>` after the first figure image has `loading="lazy"` |
| `image-bytes` | visible image bytes ≤ 2.5 MB (`python-api`, `sources/index`, `gui` ≤ 4 MB) |
| `anim-autoplay`, `anim-count`, `anim-weight` | ≤ 1 autoplaying raster per page, ≤ 3 animated assets, APNG/GIF ≤ 1 MB, animated SVG ≤ 120 KB |
| `anim-still`, `anim-class`, `anim-still-swap` | every `-anim-` image carries `.huv-anim` and a resolvable `data-huv-still`, and shows its still under reduced motion and with the toggle on |
| `lightbox` | every figure image (a `#gh-*` variant inside `<figure>`) is wrapped by the lightbox (`a.glightbox`); `.huv-no-zoom` opts an image out (strips, raw `<picture>` embeds) |
| `effect-hidden` | no reveal effect leaves a visible figure at opacity < 0.05: on the first screen in normal mode, anywhere on the page under reduced-motion emulation, anywhere in print emulation (an effect may never hide content; the CSS-only scroll-reveal must be off under `prefers-reduced-motion` and in print) |
| `mermaid-pending`, `mathjax-pending` | a diagram / equation did not render within the settle window (offline?) |
| `console` | an uncaught exception or a failed request (the api.github.com release lookup is ignored) |

### `--github-preview`

Renders Markdown through markdown-it with GFM options (raw HTML passes through, as on github.com)
and checks, on the source and on the rendered HTML:

* **errors**: a *rule* paragraph (one containing "must", "never", "not modelled", "not the engine") that leaks `!!!`, `???`, `===`, `{ .` or `:material-` (§10.1-8); any `???` line other than `success "Output"` on a docs page, and any `!!!`/`???` line in README/CONTRIBUTING; `=== "` tabs; a `details` whose text holds `|---`, `](` or `![`; `<details>` without `markdown="1"` (docs pages) or whose `</summary>` is not followed by a blank line; a `<picture>` without `<source>` / `<img alt>`; a python fence without its marker; an output-looking block within 10 lines of a fence that is not the next non-blank block (silently unchecked); a `???` output in the README.
* **warnings**: attr_list / icon shortcodes on a docs page (errors in the README); an `Output:` lead-in; a verify-example with no output block; `<img>` inside `<picture>` without `width`; Markdown emphasis inside `<figcaption>`.

The rendered HTML of every file is archived under `<out>/github-preview/` for a human look.

## `scripts/docs_gates.sh`

The gate runner of DESIGN §9.0 rule 5 (`scripts/docs_gates.sh <worktree> [--examples] [--qa] [--figures] [--strict]`);
its header lists the gates, the flags and the environment overrides. The orchestrator's
`tools/docs_gates.sh` delegates to it with the identical invocation.
