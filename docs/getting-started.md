# Getting started

**Status:** ✅ Implemented — build-and-run steps for the Python, CLI and GUI frontends; every snippet uses the public API documented in the [Python API reference](python-api.md).

highuvlith is one Rust physics engine with four ways in: a Python package, a command-line
tool driven by TOML files, an egui desktop app, and Jupyter notebooks. This page gets
each of them running and points to where to go next. For what the engine does and does
not compute, keep the [capability matrix](capability-matrix.md) open alongside.

## Install

highuvlith is not published on PyPI yet, so build it from source. You need Python 3.10
or newer and a Rust toolchain ([rustup](https://rustup.rs/)); the Python extension is
built with [maturin](https://www.maturin.rs/).

```bash
git clone https://github.com/sonoransun/highuvlith.git
cd highuvlith
python -m venv .venv && source .venv/bin/activate
pip install maturin numpy
maturin develop --release    # builds the Rust core and installs the Python package
```

Plotting helpers need matplotlib (`pip install matplotlib`); the optional extras declared
in [`pyproject.toml`](../pyproject.toml) (`viz`, `interactive`, `notebook`, `hdf5`,
`all`) list the packages each frontend feature uses.

## A first aerial image in Python

The one-liner simulates a line/space pattern with the default 157 nm F₂ source and returns
image metrics:

<!-- verify-example -->
```python
import highuvlith as huv

result = huv.simulate_line_space(65.0, 180.0, na=0.75)
print(f"Contrast: {result.contrast:.3f}, NILS: {result.nils:.2f}")
```

??? success "Output"

    ```text
    Contrast: 0.454, NILS: 0.65
    ```

For full control, build the source, optics, mask and grid yourself. The engine
precomputes the TCC/SOCS decomposition once, so evaluating many focus or dose conditions
afterwards is cheap:

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

??? success "Output"

    ```text
    Contrast: 0.0896
    ```

At 13.5 nm no lens material transmits, so an EUV source is imaged through a two-mirror
Schwarzschild objective instead of refractive optics:

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

??? success "Output"

    ```text
    Contrast: 0.7045
    ```

The [Python API reference](python-api.md) documents every class, the batch simulator and
the plotting helpers; the [simulation pipeline](pipeline.md) explains what each stage
computes.

<figure markdown="span">

![Animation of a focus sweep for 90 nm lines at 180 nm pitch with the F2 laser at NA 0.75: the aerial image and its cross-section under conventional illumination σ 0.7 lose almost all contrast by ±500 nm defocus, while a dipole matched to the pitch keeps most of it; a third panel traces contrast against defocus for both.](assets/images/demo/demo-through-focus.gif)

<figcaption>Through focus, conventional vs dipole (a still sequence from the demo video, <code>examples/generate_demo.py</code>). 90 nm lines / 180 nm pitch, F2 157.63 nm, NA 0.75 (k1 ≈ 0.43), engine <code>compute_through_focus</code>. Contrast at best focus is 0.59 with conventional σ 0.7 and 0.89 with a dipole at σ_c = λ/(2·pitch·NA) ≈ 0.58; at ±500 nm the conventional image is nearly flat (0.01) while the dipole keeps 0.67. Model: exact non-paraxial defocus and dipole illumination ✅ (scalar imaging, thin mask).</figcaption>
</figure>

## The command line

The `highuvlith` CLI runs the same engine from TOML configuration files. Build it from the
repository root and run one of the bundled examples:

```bash
cargo run --release -p highuvlith-cli -- simulate --config examples/sim.toml --output aerial.png
```

Every example configuration lives in [`examples/`](../examples/); the
[configuration reference](configuration.md) lists every field and the [CLI page](cli.md)
every subcommand.

## The desktop GUI

```bash
cargo run --release -p highuvlith-gui
```

The [GUI page](gui.md) describes the sliders and presets, and what the live view does and
does not show.

## Where next

- **See the physics:** [pipeline](pipeline.md) → [optics](optics.md) →
  [sources](sources/index.md) → [processes](processes/index.md).
- **Check what is real:** the [capability matrix](capability-matrix.md) grades every
  feature ✅ 🔶 🧪 🗺️.
- **Build intuition first:** the [playground](playground/index.md) has resolution,
  shot-noise and aerial-image calculators that run in the browser.
- **Change the code:** [architecture](architecture.md) and [extending](extending.md).
