# Contributing to highuvlith

Thanks for helping. highuvlith is a lithography simulator whose central rule is that
**honesty is the product**: every capability is graded by what the code actually computes,
and the documentation never claims more. Most of this guide is about keeping it that way.

- [Build from source](#build-from-source)
- [Tests and lints](#tests-and-lints)
- [The honesty rules](#the-honesty-rules)
- [Scientific rigor](#scientific-rigor)
- [Documentation](#documentation)
- [GitHub-safe math](#github-safe-math)
- [Pull requests](#pull-requests)
- [Releasing](#releasing)
- [GitHub Pages](#github-pages)

Orientation: [docs/architecture.md](docs/architecture.md) maps the workspace,
[docs/extending.md](docs/extending.md) has step-by-step recipes for new source families,
optical systems and process modules, [docs/capability-matrix.md](docs/capability-matrix.md)
is the status ledger, and [docs/roadmap.md](docs/roadmap.md) lists what is planned.

## Build from source

highuvlith is **not published on PyPI yet** (`pip install highuvlith` does not work);
build it from a checkout. You need a stable Rust toolchain ([rustup](https://rustup.rs))
and Python 3.10 or newer (CI tests 3.12 and 3.13).

```bash
git clone https://github.com/sonoransun/highuvlith.git
cd highuvlith
python -m venv .venv
source .venv/bin/activate          # Windows: .venv\Scripts\activate
pip install maturin numpy
maturin develop --release          # optimized build, installed into the venv
```

`maturin develop` (without `--release`) compiles faster but runs the physics much slower;
use it for quick edit-test cycles. `maturin develop` needs an active virtualenv (or conda
environment). `maturin develop --extras dev` also installs the test and lint tools.

To install the package with optional extras instead, run `pip install ".[viz]"` from the
checkout: pip builds an optimized wheel through maturin (this still needs the Rust
toolchain). The extras defined in `pyproject.toml` are `viz` (matplotlib), `interactive`
(plotly, polars), `notebook` (ipywidgets), `hdf5` (h5py), `dev` (pytest, pytest-cov,
pytest-timeout, ruff, mypy) and `all`.

The CLI and the desktop GUI are plain Cargo binaries:

```bash
cargo run --release -p highuvlith-cli -- simulate --config examples/sim.toml --output aerial.png
cargo run --release -p highuvlith-cli -- --help      # simulate, sweep, deep, materials
cargo run --release -p highuvlith-gui
```

`highuvlith-core` has two default Cargo features: `parallel` (Rayon) and `image-export`
(PNG/TIFF). Both are optional; CI also tests the crate with each of them switched off.

## Tests and lints

CI (`.github/workflows/ci.yml`) runs the following; run them before opening a pull request:

```bash
cargo fmt --check
cargo clippy -- -D warnings                          # library targets of every crate, GUI included
cargo test --workspace --exclude highuvlith-gui
cargo test -p highuvlith-core --no-default-features  # also: --features parallel / image-export
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --exclude highuvlith-gui

maturin develop && pytest tests/python/ -v --timeout=120
ruff check python/
mypy python/highuvlith/ --ignore-missing-imports
```

The example notebooks are executed headlessly in CI as well
(`pip install matplotlib jupyter nbconvert`, then for each notebook in
`examples/notebooks/`:
`MPLBACKEND=Agg jupyter nbconvert --to notebook --execute --stdout <notebook> > /dev/null`).
Every fenced `python` block in `docs/**/*.md` and `README.md` is classified by a comment
on the line above its fence (`<!-- verify-example -->` or `<!-- example-skip: reason -->`),
and CI runs the verified ones in a fresh interpreter, comparing their printed output with
the recorded one:

```bash
maturin develop --release                            # release build required, see below
pytest tests/docs/test_site_examples.py -q
python tests/docs/test_site_examples.py --update-outputs   # after an intended numeric change
```

Build the extension in **release** mode for these: each example has a 60 s limit, and at
least one (the volumetric-exposure example) takes longer than that in a debug build. To
run them against a debug build anyway, raise the limit with `HUV_EXAMPLE_TIMEOUT=300`.

Benchmarks are Criterion benches: `cargo bench -p highuvlith-core` (reports land in
`target/criterion/`).

What the test tiers pin:

| Tier | Where | What it pins |
|---|---|---|
| Unit tests | `#[cfg(test)]` in every module | Constructors reject bad input; fixture values; invariants (weights sum to 1, symmetry, bounds) |
| Analytical validation | `crates/highuvlith-core/tests/analytical_validation.rs` | Closed forms: Fresnel/Brewster/quarter-wave, the coherent three-beam image through focus |
| Imaging validation | `crates/highuvlith-core/tests/imaging_validation.rs` | SOCS against an independent Abbe summation, resolution limits, focus behaviour, determinism |
| Vector imaging | `crates/highuvlith-core/tests/vector_pupil.rs`, `vector_imaging.rs` | TE/TM two-beam closed forms, energy statements, film entrance, engine-level vector images |
| Property-based | `crates/highuvlith-core/tests/proptest_physics.rs` | Physical bounds over random inputs |
| Python | `tests/python/` | Bindings, the `api` helpers, per-family source physics, error mapping |
| Stub drift | `tests/python/test_stub_drift.py` | `python/highuvlith/_native.pyi` lists every member of the compiled module |
| Docs examples | `tests/docs/test_site_examples.py` | Every verified Python example in `docs/` and `README.md` runs and prints its recorded output (numbers within a small tolerance) |
| CLI examples | `crates/highuvlith-cli/tests/examples.rs` | Every `examples/*.toml` runs end to end through the CLI |

House rules for tests:

- Keep them fast: a Rust test should take well under ~3 s in a debug build and a Python
  test under ~5 s. Use small grids.
- A test that encodes a past bug (for example `test_default_photon_density_matches_source_trait`,
  which pins a former 1000× photon-density slip) is never weakened; if it must change,
  explain the history in a comment.
- Every new PyO3 class, function or method needs its entry in `_native.pyi`, or the stub
  drift test fails.
- `cargo clippy --all-targets` is not clean yet (a few lints in older test code); please do
  not add new ones. CI enforces clippy on library targets only.

`.pre-commit-config.yaml` wires rustfmt, clippy, `ruff` and `ruff format` into
`pre-commit` (`pip install pre-commit && pre-commit install`). CI does not enforce
`ruff format` yet.

## The honesty rules

Every capability carries one of four grades. [docs/capability-matrix.md](docs/capability-matrix.md)
is the single source of truth for them.

| Badge | Grade | Meaning |
|---|---|---|
| ✅ | Implemented | Computed by code, covered by tests, physics validated against analytical results |
| 🔶 | Simplified | Runs end-to-end with documented approximations or reduced dimensionality; the assumption is stated inline wherever the capability is described |
| 🧪 | Theoretical | Parameterized and runnable, but models speculative physics; outputs are research projections, not validated engineering |
| 🗺️ | Planned | Not in code; roadmap only; never described in prose as if it runs |

The grade shows up in four places, and they must agree:

1. the `//! # Model status` section of the module's header doc (every core module has one,
   next to `//! # Key equations`);
2. the row in [docs/capability-matrix.md](docs/capability-matrix.md);
3. the first line of the capability's documentation page:
   `**Status:** <badge> <Name> — <one-sentence justification>` (CI checks that every
   technical page has it; index pages, the capability matrix and the narrative site
   sections are exempt);
4. the "Model coverage" tables on source and process pages, where every struct field is
   tagged `live` (enters a computation), `stored (inert)` (carried and exposed, no consumer
   yet) or `planned` (not a field yet).

**Same-PR rule:** a change to what a module computes updates the module header, the
capability-matrix row and the page's Status line in the same pull request. When a roadmap
item lands, delete it from [docs/roadmap.md](docs/roadmap.md) in that pull request too.
Never describe a planned capability as if it runs, and never round a 🔶 up to ✅.

## Scientific rigor

- Use textbook or peer-reviewed physics, and write the key equations into the module header.
- Cite a reference only when you are certain of its details (authors, journal, volume,
  year). If you are not, describe the provenance generically ("standard 1D FEL theory",
  "Kramers' law"). A fabricated citation is the worst possible contribution.
- Every new formula gets a fixture test whose expected numbers were computed independently
  of the implementation (by hand, or with a separate NumPy/SciPy script you mention in the
  test), plus limit or scaling tests (for example "doubling γ quarters λ").
- Use CODATA constants; the shared ones live in
  `crates/highuvlith-core/src/source_models/physics.rs`.
- Prefer quantities derived from machine parameters over stored numbers. When a value is a
  projection or an assumption, say so in the code comment and in the docs.
- Units: lengths in nm unless the field name says otherwise (`_um`, `_mm`, `_m`), energies
  in eV or keV (the name says which), doses in mJ/cm², powers in W.

## Documentation

Technical pages in `docs/` are plain Markdown (plus mermaid diagrams) that must read well on
github.com **and** build in the MkDocs Material site. The site toolchain is pinned in
`docs/requirements-docs.txt` (MkDocs stays on 1.x: MkDocs 2 drops the plugin and theme
system the site is built on). Use a separate virtualenv:

```bash
python -m venv .venv-docs
.venv-docs/bin/pip install -r docs/requirements-docs.txt
.venv-docs/bin/mkdocs serve                   # live preview at http://127.0.0.1:8000
.venv-docs/bin/mkdocs build --strict          # what CI runs; any warning fails
```

- `HUV_DOCS_STRICT_NAV=1` turns nav entries whose page is missing into errors (CI sets
  it); without it the `docs/hooks/pending_nav.py` hook skips them with an INFO message.
  `NO_MKDOCS_2_WARNING=true` silences Material's MkDocs 2 notice.
- New pages need an entry in the `nav:` section of `mkdocs.yml`.
- Link between pages with relative links (`./pipeline.md`, `../sources/lpp.md`). Links to
  code or examples outside `docs/` (`../crates/highuvlith-core/src/aerial.rs`,
  `../../examples/sim.toml`) are rewritten to GitHub URLs on the site by
  `docs/hooks/repo_links.py`, and only when the target exists, so a typo fails the build.
- Never link `CLAUDE.md`; it is not documentation.
- The narrative sections (`docs/history/`, `docs/nodes/`, `docs/future/`) follow
  additional rules: every fact is verifiable and listed under "References", images are our
  own (mermaid, hand-written SVG, or matplotlib scripts kept under `docs/figures/`) and must
  work in light and dark mode, and simulator examples are marked with a
  `<!-- verify-example -->` comment and run before they are committed.
- Pages that describe a capability link its capability-matrix grade and never imply the
  simulator does more (there is no mask-3D/EMF model, for example).

## GitHub-safe math

GitHub applies Markdown backslash-escape processing inside `$…$` and `$$…$$` before the
math is typeset: `\,` becomes `,`, `\{` becomes `{`, `\\` becomes `\`, and `\%` becomes
`%`, which TeX reads as a comment that swallows the rest of the line. The MkDocs site is
not affected, so these breakages only show on github.com. Therefore:

- **Display math** goes in a fenced block that opens with ```` ```math ```` and closes
  with ```` ``` ````. GitHub renders it verbatim, and the site renders it exactly like
  `$$…$$`.
- **Inline math** contains no backslash followed by punctuation: drop `\,` `\;` `\!`
  (use a space or nothing), write `\lbrace` / `\rbrace` instead of `\{` / `\}` and `\Vert`
  instead of `\|`, and never write `\%` — put the percent sign outside the math
  (for example `$0.6$ %`). Letter macros such as `\lambda`, `\frac`, `\sqrt` and
  `\mathrm` are fine.
- A literal dollar sign in prose is written `\$`, so it is not read as math.

The linter enforces this (CI runs it on `docs/`, `README.md` and this file and fails on any finding):

```bash
python scripts/check_docs_math.py docs/ README.md CONTRIBUTING.md   # report; exit 1 on findings
python scripts/check_docs_math.py --fix --fix-inline docs/my-page.md
```

`--fix` converts `$$` display blocks to `math` fences; `--fix-inline` applies the safe
inline rewrites. Anything else it reports (such as `\%`) is fixed by hand.

## Pull requests

- Branch from `main` and keep pull requests focused.
- In the description, say what changed and why, and give the validation evidence: the
  tests you added, the numbers they pin, and where the independent reference values came
  from.
- Checklist: the commands under [Tests and lints](#tests-and-lints) pass; the same-PR
  honesty updates are done; `_native.pyi` covers new bindings; new TOML keys are
  documented in [docs/configuration.md](docs/configuration.md); `mkdocs build --strict`
  and the math linter pass if you touched docs.
- New source families, optical systems and process modules follow the recipes in
  [docs/extending.md](docs/extending.md) (model file, trait implementation, `SourceKind`
  wiring, Python factory and stub, CLI TOML, example config, docs page, matrix row, tests).
- Commit messages: a short imperative summary line, then a body that explains why.

By contributing you agree that your contributions are licensed under the
[MIT License](LICENSE).

## Releasing

1. Bump `version` in `pyproject.toml` and in the four `crates/*/Cargo.toml` manifests
   together.
2. Tag the release commit `vX.Y.Z` and push the tag. `.github/workflows/release.yml` then
   builds optimized wheels with `PyO3/maturin-action` (x86_64 and aarch64 Linux manylinux,
   aarch64 and x86_64 macOS, x86_64 Windows; `--find-interpreter`, no sdist) and publishes
   them to PyPI with trusted publishing (`id-token: write`, no API token).
3. Before the first release, configure a trusted publisher for the `highuvlith` project on
   PyPI (repository `sonoransun/highuvlith`, workflow `release.yml`). The package has not
   been published yet.
4. Before tagging, check that the runner labels in `release.yml` are still offered by
   GitHub — notably `macos-13`, used for the x86_64 macOS wheel; GitHub retires older
   macOS runner images periodically.

The Rust crates are not published to crates.io.

## GitHub Pages

The documentation site (history of lithography, process nodes, the future of the field,
playground, and the simulator docs) is published at
<https://sonoransun.github.io/highuvlith/> from the repository
<https://github.com/sonoransun/highuvlith>.

- `.github/workflows/pages.yml` runs on every push to `main` and on manual dispatch
  ("Run workflow"): it tests the MkDocs hooks and the playground physics, builds the site
  with `mkdocs build --strict` (missing nav pages are errors), and deploys it with
  `actions/deploy-pages`. Pull requests are checked by the `docs-site` job in
  `.github/workflows/ci.yml`.
- One-time setup: **Settings → Pages → Build and deployment → Source = GitHub Actions**.
- GitHub Pages on a **private** repository needs a paid GitHub plan (Pro, Team or
  Enterprise); on GitHub Free, Pages is available for public repositories only. While the
  repository is private, the site's "view/edit on GitHub" links and source links do not
  resolve for visitors.
- If the repository or its default branch is renamed, update `site_url`, `repo_url`,
  `repo_name` and `edit_uri` in `mkdocs.yml`; `docs/hooks/repo_links.py` derives its GitHub
  links from them.
