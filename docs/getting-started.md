---
description: "Build highuvlith from source and run the canonical 65 nm / 180 nm aerial image from Python, the command line, the desktop GUI and the notebooks, with every snippet verified in CI."
---

# Getting started

**Status:** ✅ Implemented — build-and-run steps for the Python, CLI, GUI and notebook front ends; every snippet on this page is a verified example that runs in CI against the public API · [matrix](capability-matrix.md#frontends-and-infrastructure)

<p class="huv-lead">highuvlith is one Rust physics engine with four ways in: a Python package, a command-line tool driven by TOML files, an egui desktop app and Jupyter notebooks. This page takes a developer from a clone to a first aerial image in each of them, on one canonical run whose numbers every front end reproduces. It does not explain the physics and it does not grade anything.</p>

The [simulation pipeline](pipeline.md) explains what each stage computes; keep the
[capability matrix](capability-matrix.md) open alongside for what the engine does and does not
compute.

<div class="huv-physics huv-physics--single" markdown>

<figure markdown="span">

![The canonical first run drawn: a two-period aerial image of 65 nm lines on a 180 nm pitch at 157.63 nm, NA 0.75, and its cross-section with the 0.3 threshold, a 41.2 nm printed line, contrast 0.454 and NILS 0.65.](assets/images/sim/start-first-image-light.png#gh-light-mode-only)
![The canonical first run drawn: a two-period aerial image of 65 nm lines on a 180 nm pitch at 157.63 nm, NA 0.75, and its cross-section with the 0.3 threshold, a 41.2 nm printed line, contrast 0.454 and NILS 0.65.](assets/images/sim/start-first-image-dark.png#gh-dark-mode-only)

<figcaption><strong>Your first image: the numbers printed above, drawn.</strong> 65 nm lines at 180 nm pitch, F<sub>2</sub> 157.63 nm, NA 0.75, σ 0.7 — the same run as <code>examples/sim.toml</code>; contrast, NILS and the printed CD at the 0.3 threshold are computed at figure time by the same call. <em>Model: Hopkins imaging (factorized TCC/SOCS) ✅; thin mask, 2 % uniform flare, no resist.</em></figcaption>
</figure>

| Front end | One command |
|---|---|
| Python | `huv.simulate_line_space(65.0, 180.0, na=0.75)` |
| CLI | `highuvlith simulate --config examples/sim.toml` |
| GUI | `cargo run --release -p highuvlith-gui -- --preset f2 --cd 65 --pitch 180` |
| Notebooks | `examples/notebooks/01_quickstart_aerial.ipynb` |

**Same thing elsewhere:** Python `simulate_line_space(65.0, 180.0, na=0.75)` · CLI `highuvlith simulate --config examples/sim.toml` · TOML `examples/sim.toml` · GUI `--preset f2 --cd 65 --pitch 180` · physics → [Simulation pipeline](pipeline.md)

</div>

## Install

highuvlith is not published on PyPI yet, so build it from source. You need Python 3.10
or newer and a Rust toolchain ([rustup](https://rustup.rs/)); the Python extension is
built with [maturin](https://www.maturin.rs/).

```bash
git clone https://github.com/sonoransun/highuvlith.git
cd highuvlith
python -m venv .venv && source .venv/bin/activate
pip install maturin numpy
maturin develop --release    # Rust core + Python package
```

Plotting helpers need matplotlib (`pip install matplotlib`); the optional extras declared
in [`pyproject.toml`](../pyproject.toml) (`viz`, `interactive`, `notebook`, `hdf5`,
`all`) list the packages each front end feature uses.

### Check it worked

<!-- verify-example -->
```python
import highuvlith as huv

print(huv.__version__)
```

???+ success "Output"

    ```text
    0.2.0
    ```

## First images in Python

Three examples build up from the one-liner to explicit objects; each prints a number that
CI checks against the recorded output.

### 1. Your first image

The one-liner simulates a line/space pattern with the default 157 nm F₂ source and
returns image metrics — contrast 0.454 and NILS 0.65 for 65 nm lines on a 180 nm pitch,
the figure at the top of this page:

<!-- verify-example -->
```python
import highuvlith as huv

result = huv.simulate_line_space(65.0, 180.0, na=0.75)
print(f"Contrast: {result.contrast:.3f}, NILS: {result.nils:.2f}")
```

???+ success "Output"

    ```text
    Contrast: 0.454, NILS: 0.65
    ```

This is the canonical run of the whole site: the same case as [`examples/sim.toml`](../examples/sim.toml)
on the [command line](#the-command-line), the GUI's `--preset f2 --cd 65 --pitch 180`, and the
quick start of the [Python API](python-api.md#quick-start). [`simulate_line_space`](python-api.md#high-level-functions)
returns the aerial image with its metrics, a `.status` grade and `.notes`.

### 2. Build the objects yourself

For full control, build the [`SourceConfig`](python-api.md#sourceconfig),
[`OpticsConfig`](python-api.md#opticsconfig), [`MaskConfig`](python-api.md#maskconfig) and
[`GridConfig`](python-api.md#maskconfig) yourself and hand them to a
[`SimulationEngine`](python-api.md#simulationengine). The engine precomputes the TCC/SOCS
decomposition once, so evaluating many focus or dose conditions afterwards is cheap. Two
helpers size the field: `GridConfig(size, pixel_nm)` sets the pixel directly, so you must make
the field a whole number of periods yourself (here 128 × 1.875 nm = 240 nm = two periods of
120 nm), while `mask.commensurate_grid(size, target_pixel_nm)` — used in example 3 — picks the
nearest pixel that makes the field commensurate for you.

<!-- verify-example -->
```python
import highuvlith as huv

source = huv.SourceConfig.f2_laser(sigma=0.7)
optics = huv.OpticsConfig(numerical_aperture=0.85)
mask = huv.MaskConfig.line_space(cd_nm=45.0, pitch_nm=120.0)
grid = huv.GridConfig(size=128, pixel_nm=1.875)  # 240 nm field = 2 periods (FFTs make it periodic)

engine = huv.SimulationEngine(source, optics, mask, grid=grid)
aerial = engine.compute_aerial_image(focus_nm=0.0)
print(f"Contrast: {aerial.image_contrast():.4f}")
```

???+ success "Output"

    ```text
    Contrast: 0.0896
    ```

### 3. Schwarzschild EUV

At 13.5 nm no lens material transmits, so an EUV source is imaged through a two-mirror
[Schwarzschild objective](optics.md) instead of refractive optics; the mask's
`commensurate_grid` helper keeps the field at whole periods:

<!-- verify-example -->
```python
import highuvlith as huv

source = huv.SourceConfig.lpp_sn_13nm5(sigma=0.9)
optics = huv.OpticsConfig.schwarzschild(numerical_aperture=0.33)
mask = huv.MaskConfig.line_space(cd_nm=22.0, pitch_nm=44.0)
grid = mask.commensurate_grid(size=128, target_pixel_nm=1.0)  # field = whole periods

engine = huv.SimulationEngine(source, optics, mask, grid=grid)
print(f"Contrast: {engine.compute_aerial_image(focus_nm=0.0).image_contrast():.4f}")
```

???+ success "Output"

    ```text
    Contrast: 0.7045
    ```

### 4. Through focus

Once the engine is built, `compute_through_focus` evaluates the same image at several focus
planes without redoing the decomposition. Conventional σ 0.7 illumination loses almost all of
its contrast by ±500 nm of defocus, while an x-dipole matched to the pitch (σ_c = λ/(2·pitch·NA)
≈ 0.58) keeps most of it — the numbers the animation below prints at those three planes:

<!-- verify-example -->
```python
import highuvlith as huv

optics = huv.OpticsConfig(numerical_aperture=0.75, flare_fraction=0.0)
mask = huv.MaskConfig.line_space(90.0, 180.0)
grid = huv.GridConfig(size=128, pixel_nm=360.0 / 128)  # two periods, commensurate
sigma_c = 157.63 / (2 * 180.0 * 0.75)  # λ / (2 · pitch · NA) ≈ 0.58
sources = {
    "conventional σ 0.7": huv.SourceConfig.f2_laser(0.7),
    "x-dipole": huv.SourceConfig.f2_laser().with_illumination("dipole", sigma_c, 0.1, 0.0),
}
for name, source in sources.items():
    engine = huv.SimulationEngine(source, optics, mask, grid=grid)
    planes = engine.compute_through_focus([-500.0, 0.0, 500.0])
    print(name, " ".join(f"{p.image_contrast():.2f}" for p in planes))
```

???+ success "Output"

    ```text
    conventional σ 0.7 0.01 0.59 0.01
    x-dipole 0.69 0.90 0.69
    ```

<figure markdown="span">

<picture><source media="(prefers-reduced-motion: reduce)" srcset="assets/images/sim/anim/through-focus-still-light.png"><img class="huv-no-zoom huv-anim" src="assets/images/sim/anim/through-focus-anim-light.png#gh-light-mode-only" alt="Animated focus sweep of 90 nm lines at 180 nm pitch, F2 157.63 nm, NA 0.75: the aerial image and its cut under conventional sigma 0.7 lose nearly all contrast by 500 nm of defocus while an x-dipole keeps most of it; a third panel traces contrast against focus for both with a moving marker." data-huv-still="assets/images/sim/anim/through-focus-still-light.png"></picture>
<picture><source media="(prefers-reduced-motion: reduce)" srcset="assets/images/sim/anim/through-focus-still-dark.png"><img class="huv-no-zoom huv-anim" src="assets/images/sim/anim/through-focus-anim-dark.png#gh-dark-mode-only" alt="Animated focus sweep of 90 nm lines at 180 nm pitch, F2 157.63 nm, NA 0.75: the aerial image and its cut under conventional sigma 0.7 lose nearly all contrast by 500 nm of defocus while an x-dipole keeps most of it; a third panel traces contrast against focus for both with a moving marker." data-huv-still="assets/images/sim/anim/through-focus-still-dark.png"></picture>

<figcaption><strong>Through focus, conventional versus dipole illumination.</strong> 90 nm lines on a 180 nm pitch at 157.63 nm, NA 0.75; defocus −500 to +500 nm; the marker tracks image contrast and the hollow dots are the three planes the getting-started example prints. <em>Model: defocus / through-focus imaging ✅ (exact non-paraxial phase in the pupil per source point) · scalar, thin mask, no flare, no resist.</em></figcaption>
</figure>

The [Python API reference](python-api.md) documents every class, the batch simulator and
the plotting helpers; the [simulation pipeline](pipeline.md) explains what each stage
computes.

## The command line

The `highuvlith` CLI runs the same engine from TOML configuration files. Build it once from
the repository root and run the canonical configuration:

```bash
cargo build --release -p highuvlith-cli
target/release/highuvlith simulate --config examples/sim.toml --output aerial.png
```

<figure markdown="span">

![The 256 × 256 px aerial-image PNG that highuvlith simulate --config examples/sim.toml --output writes for 65 nm lines on a 180 nm pitch, through the CLI's Inferno colormap, with its printed contrast.](assets/images/sim/cli-output-gallery-simulate-light.png#gh-light-mode-only)
![The 256 × 256 px aerial-image PNG that highuvlith simulate --config examples/sim.toml --output writes for 65 nm lines on a 180 nm pitch, through the CLI's Inferno colormap, with its printed contrast.](assets/images/sim/cli-output-gallery-simulate-dark.png#gh-dark-mode-only)

<figcaption><strong>The PNG the command line writes.</strong> <code>highuvlith simulate --config examples/sim.toml --output aerial.png</code> saves the aerial image of the canonical run (65 nm lines, 180 nm pitch, F<sub>2</sub> 157.63 nm, NA 0.75, σ 0.7) as a min–max normalised RGB PNG through its own Inferno colormap; the contrast under the panel is the one it prints. <em>The CLI's own file; the same run as the Python example above.</em></figcaption>
</figure>

The report goes to stderr; its last lines for this run are the numbers of example 1 at the
CLI's 256-pixel grid:

```text
Results:
  Contrast:  0.4545
  I_max:     0.6631
  I_min:     0.2487
  Printed CD (dark feature at I = 0.3000): 41.20 nm
  NILS:      0.654
```

Every example configuration lives in [`examples/`](../examples/); the
[configuration reference](configuration.md) lists every field and the [CLI page](cli.md)
every subcommand with its full transcript.

## The desktop GUI

```bash
cargo run --release -p highuvlith-gui -- --preset f2 --cd 65 --pitch 180
```

The app opens on the Aerial image tab with the source, optics and mask of the canonical run;
`--preset lpp-sn --optics nxe --cd 16 --pitch 32` is the EUV case captured below.

<figure markdown="span">

![The running desktop app on its Aerial image tab: the Source and Optics controls (Sn LPP preset, NA 0.33), the EUV 13.5 nm aerial image of 16 nm lines on a 32 nm pitch with its colour scale, the metrics rows (contrast 0.4494, printed CD 16.637 nm, NILS 1.427) and the status bar naming 21 kernels.](assets/images/gui/gui-aerial-euv.png)

<figcaption><strong>Aerial image view.</strong> The Sn LPP preset (NXE:3400B class) through the EUV NXE optics, NA 0.33, 16 nm lines on a 32 nm pitch: the image, its colour scale, the metrics rows and the status bar with the kernel count and captured TCC fraction. Made with <code>highuvlith-gui --preset lpp-sn --optics nxe --cd 16 --pitch 32 --tab aerial --screenshot</code>. <em>Real capture, dark theme; nothing is mocked.</em></figcaption>
</figure>

The [GUI page](gui.md) describes the sliders and presets, and what the live view does and
does not show.

## Notebooks

Six Jupyter notebooks under [`examples/notebooks/`](../examples/notebooks/) run the same
engine with plots (install matplotlib and Jupyter alongside the package); CI executes every one
of them headlessly.

| Notebook | What it does |
|---|---|
| [`01_quickstart_aerial.ipynb`](../examples/notebooks/01_quickstart_aerial.ipynb) | The one-liner, the explicit engine on a commensurate grid, through-focus imaging, a dose-aware process window and a 2-D resist profile |
| [`02_source_gallery.ipynb`](../examples/notebooks/02_source_gallery.ipynb) | Every preset of the fourteen source families: spectra, pupil fills, the same pattern through each, the per-family status |
| [`03_deep_litho.ipynb`](../examples/notebooks/03_deep_litho.ipynb) | LIGA depth dose and exposure time, the Fresnel edge, an X-ray tube as LIGA source, interference lattices, volumetric exposure and level-set development |
| [`04_imaging_physics.ipynb`](../examples/notebooks/04_imaging_physics.ipynb) | Partial coherence, off-axis illumination and depth of focus, vector TE/TM imaging, immersion, broadband HHG imaging, EUV NA 0.33 against 0.55 |
| [`05_source_physics.ipynb`](../examples/notebooks/05_source_physics.ipynb) | The power-versus-wavelength landscape, derived quantities, dose-limited throughput, parameter sensitivities, the honest gaps, photon shot noise |
| [`06_process_and_optimization.ipynb`](../examples/notebooks/06_process_and_optimization.ipynb) | Dose-aware process windows, ILT, OPC, SRAFs, level-set against fast-marching development, Talbot and EUV interference lithography |

## Where next

<div class="grid cards huv-cards" markdown>

-   **Learn**

    ---

    <span class="huv-swatch huv-swatch--history"></span>How chips have been printed, from the printing stone to High-NA EUV.

    [Start the tour →](history/index.md)

-   **Try**

    ---

    <span class="huv-swatch huv-swatch--euv"></span>Resolution, shot-noise and aerial-image calculators that run in the browser.

    [Open the playground →](playground/index.md)

-   **Run**

    ---

    <span class="huv-swatch huv-swatch--f2"></span>The engine, stage by stage: pipeline, optics, sources, processes.

    [Simulator →](simulator/index.md)

-   **Check**

    ---

    <span class="huv-swatch huv-swatch--xray"></span>Every capability graded in one table; nothing on the site claims more than it.

    [Capability matrix →](capability-matrix.md)

</div>

**Read next:** [Simulation pipeline](pipeline.md) · [Python API](python-api.md) · [Architecture](architecture.md) · [Extending highuvlith](extending.md) · [Capability matrix](capability-matrix.md#frontends-and-infrastructure)
