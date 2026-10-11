---
description: "The simulator section: the Rust engine stage by stage, the canonical run on every front end, and the wings (sources, processes, research modules)."
---

# Simulator

<p class="huv-lead">highuvlith is a Rust lithography engine with four front ends. This section walks the engine stage by stage — source, optics, mask, Hopkins image, film, resist, metrics — and states the exactness of every stage as the capability matrix grades it. Read it in nav order to understand the physics, or jump to the canonical run below to see the same numbers come out of Python, the CLI, a TOML file and the GUI.</p>

<figure markdown="span">

<picture><source media="(max-width: 600px)" srcset="../assets/images/diagrams/pipeline-stages-narrow-light.svg"><img class="huv-no-zoom" src="../assets/images/diagrams/pipeline-stages-light.svg#gh-light-mode-only" alt="Seven stage boxes in a row, Source to Metrics, each with its grade chip, and three dashed bypass arrows for LIGA, interference and Talbot curving under the row into the film and resist stages."></picture>
<picture><source media="(max-width: 600px)" srcset="../assets/images/diagrams/pipeline-stages-narrow-dark.svg"><img class="huv-no-zoom" src="../assets/images/diagrams/pipeline-stages-dark.svg#gh-dark-mode-only" alt="Seven stage boxes in a row, Source to Metrics, each with its grade chip, and three dashed bypass arrows for LIGA, interference and Talbot curving under the row into the film and resist stages."></picture>

<figcaption><strong>Every simulation runs the same seven stages.</strong> Chips are the stage grades from the capability matrix; the bypass paths skip projection optics. <em>Chips copied from the matrix.</em></figcaption>
</figure>

**Stage by stage:** [Source](../sources/index.md) · [Optics](../optics.md) · [Mask](../masks.md) · [Image](../pipeline.md#stage-4--aerial-image) · [Film](../processes/thin-film.md) · [Resist](../processes/resist-models.md) · [Metrics](../metrics-and-process-window.md) · bypasses: [LIGA](../processes/liga-deep-xray.md) · [Interference](../processes/interference-volumetric.md) · [Talbot](../processes/talbot.md)

## Start here

<!-- huv-table: stack -->

| I want to … | Go to | Grade of what you will use |
|---|---|---|
| Print my first aerial image in five lines of Python | [Getting started](../getting-started.md) | ✅ Implemented |
| Understand what the engine computes at each stage | [Simulation pipeline](../pipeline.md) | ✅ Implemented |
| Pick an optic — dry, immersion, EUV, Schwarzschild, zone plate | [Optical systems](../optics.md) | ✅ Implemented (🔶 EUV projection) |
| Image at NA ≳ 0.8 or inside a resist | [Vector imaging](../vector-imaging.md) | ✅ Implemented |
| Draw a mask and get its exact spectrum | [Masks](../masks.md) | ✅ Implemented (🔶 mask model) |
| Measure CD, NILS and a process window | [Metrics & process window](../metrics-and-process-window.md) | 🔶 Simplified (✅ CD / ILS / NILS / MEEF metrics) |
| Look up n, k, δ, β or a Mo/Si mirror | [Materials](../materials.md) | 🔶 Simplified (✅ CXRO/Henke constants, NIST attenuation, multilayer mirrors) |
| Expose a resist in depth and develop it | [Processes](../processes/index.md) | 🔶 Simplified |
| Choose a light source and see what is live in its model | [Sources](../sources/index.md) | ✅ Implemented (per family) |
| Try OPC, ILT, SRAF, multiple patterning, stochastics | [Research modules](../research-modules.md) | 🔶 Simplified |
| See where the physics lives in the code | [Architecture](../architecture.md) | ✅ Implemented |

Every grade in this table is copied from the [capability matrix](../capability-matrix.md); a page's own **Status** line is the authority for that page.

### Reading order

<div class="grid cards huv-cards" markdown>

-   **[Getting started](../getting-started.md)** · ✅ Implemented

    ---

    Install, run the canonical 65 nm / 180 nm case, and see the same image from Python, the CLI and the GUI.

-   **[Simulation pipeline](../pipeline.md)** · ✅ Implemented

    ---

    The seven stages in order: source sampling, pupil, exact mask spectrum, Hopkins/SOCS imaging, film, resist, metrics.

-   **[Optical systems](../optics.md)** · ✅ Implemented

    ---

    One pupil contract, four optics: refractive dry and immersion, EUV projection 🔶, Schwarzschild, Fresnel zone plate.

-   **[Vector imaging](../vector-imaging.md)** · ✅ Implemented

    ---

    TE/TM per diffraction order, the radiometric factor, the immersion index and the film-entrance Fresnel transmission.

-   **[Masks](../masks.md)** · ✅ Implemented

    ---

    Thin-mask primitives with closed-form spectra, commensurate grids, antialiased rasterization; mask model 🔶.

-   **[Metrics & process window](../metrics-and-process-window.md)** · 🔶 Simplified

    ---

    Sub-pixel CD, ILS, NILS and MEEF are ✅ Implemented; the dose-aware process window and ED analysis are 🔶 Simplified.

-   **[Materials](../materials.md)** · 🔶 Simplified

    ---

    CXRO/Henke constants, NIST attenuation, Sellmeier and VUV tables with a stated range policy, Mo/Si and La/B mirrors.

-   **[Architecture](../architecture.md)** · ✅ Implemented

    ---

    Under the hood: the crate layout, the factorized TCC data flow and the validation layering — read it last.

</div>

## The canonical run

One case threads this whole section: **65 nm lines on a 180 nm pitch, F₂ 157.63 nm, NA 0.75, σ 0.7** — the contents of `examples/sim.toml`. Python, the CLI, the TOML file and the GUI run it below; `tests/docs/test_rosetta.py` asserts that the Python numbers equal the CLI transcript on the [CLI page](../cli.md).

<div class="huv-physics huv-physics--single" markdown>

<figure markdown="span">

![The canonical first run drawn: a two-period aerial image of 65 nm lines on a 180 nm pitch at 157.63 nm, NA 0.75, and its cross-section with the 0.3 threshold, a 41.2 nm printed line, contrast 0.454 and NILS 0.65.](../assets/images/sim/start-first-image-light.png#gh-light-mode-only)
![The canonical first run drawn: a two-period aerial image of 65 nm lines on a 180 nm pitch at 157.63 nm, NA 0.75, and its cross-section with the 0.3 threshold, a 41.2 nm printed line, contrast 0.454 and NILS 0.65.](../assets/images/sim/start-first-image-dark.png#gh-dark-mode-only)

<figcaption><strong>Your first image: the numbers printed above, drawn.</strong> 65 nm lines at 180 nm pitch, F<sub>2</sub> 157.63 nm, NA 0.75, σ 0.7 — the same run as <code>examples/sim.toml</code>; contrast, NILS and the printed CD at the 0.3 threshold are computed at figure time by the same call. <em>Model: Hopkins imaging (factorized TCC/SOCS) ✅; thin mask, 2 % uniform flare, no resist.</em></figcaption>
</figure>

| Front end | One command | Where it is documented |
|---|---|---|
| Python | `huv.simulate_line_space(65.0, 180.0, na=0.75)` | [Python API](../python-api.md) |
| CLI | `highuvlith simulate --config examples/sim.toml` | [CLI](../cli.md) · [TOML configuration](../configuration.md) |
| GUI | `cargo run --release -p highuvlith-gui -- --preset f2 --cd 65 --pitch 180` | [GUI](../gui.md) |
| Notebooks | `examples/notebooks/01_quickstart_aerial.ipynb` | six notebooks, executed headlessly in CI |

**Same thing elsewhere:** Python `simulate_line_space(65.0, 180.0, na=0.75)` · CLI `highuvlith simulate --config examples/sim.toml` · TOML `examples/sim.toml` · GUI `--preset f2 --cd 65 --pitch 180` · physics → [Simulation pipeline](../pipeline.md)

</div>

<div class="huv-rosetta" markdown>

**Python**

<!-- verify-example -->
```python
import highuvlith as huv

result = huv.simulate_line_space(65.0, 180.0, na=0.75)
print(f"Contrast: {result.contrast:.3f}, NILS: {result.nils:.2f}, CD: {result.cd_nm:.1f} nm")
```

???+ success "Output"

    ```text
    Contrast: 0.454, NILS: 0.65, CD: 41.2 nm
    ```

**CLI**

```bash
highuvlith simulate --config examples/sim.toml --output aerial.png
# Results:
#   Contrast:  0.4545
#   Printed CD (dark feature at I = 0.3000): 41.20 nm
#   NILS:      0.654
```

**TOML**

```toml
# examples/sim.toml — no `type` in [source]: the legacy F2 excimer source
[source]
wavelength_nm = 157.63
sigma = 0.7
bandwidth_pm = 1.1

[optics]
na = 0.75
flare_fraction = 0.02

[mask]
cd_nm = 65.0
pitch_nm = 180.0

[grid]
size     = 256    # the CLI snaps the pixel to 0.703 nm so the field holds one pitch
pixel_nm = 1.0

[process]
dose_mj_cm2 = 30.0
focus_nm = 0.0
```

**GUI**

```text
cargo run --release -p highuvlith-gui -- --preset f2 --cd 65 --pitch 180
```

</div>

The Python wrapper and the CLI both snap the 256-pixel grid to a commensurate 0.703 nm pixel (one pitch per field), keep 9 SOCS kernels capturing the whole TCC trace from 1 176 source points, and apply the preset's 2 % flare — which is why the four front ends agree to the printed digits. The [pipeline page](../pipeline.md) shows how this one image is built from source points, mask orders and kernels.

## The wings

The imaging chain above is the trunk. Two wings hang off it, and a third holds the research modules.

<div class="grid cards huv-cards" markdown>

-   **[Sources](../sources/index.md)** · 15 families

    ---

    Excimer and lamp lines, plasma EUV, accelerator concepts, table-top coherent EUV, lab X-ray and quantum sources — each page says which quantities are live, derived or inert, and carries its own grade copied from the matrix.

-   **[Processes](../processes/index.md)** · 7 pages

    ---

    Thin-film standing waves, 3-D development, LIGA and interference are ✅ Implemented (with 🔶 parts: the level-set front, LIGA's thin-screen absorber, the interference model's scalar absorption and two-photon kinetics); the 2-D and volumetric resist models, grayscale and Talbot are 🔶 Simplified — the stages after the aerial image, and the paths that bypass the optics.

-   **[Research modules](../research-modules.md)** · ILT · OPC · SRAF · patterning · stochastics

    ---

    ILT, OPC and photon shot noise are ✅ Implemented; SRAF, multiple patterning and LER/LWR are 🔶 Simplified; the quantum and MNSL models are 🧪 Theoretical — every one graded in the matrix.

</div>

## Check what is real

Nothing in this section claims more than the [capability matrix](../capability-matrix.md), the single source of truth for the four grades — ✅ Implemented, 🔶 Simplified, 🧪 Theoretical, 🗺️ Planned (defined in its [status taxonomy](../capability-matrix.md#status-taxonomy)). Every technical page opens with a **Status** line copied from its matrix rows, every figure caption ends with the model grades it rests on, and every Python example on these pages is run in CI with its printed output checked. When a page and the matrix disagree, the matrix wins and the page is the bug.

**Where next:** [Getting started](../getting-started.md) · [Simulation pipeline](../pipeline.md) · [Capability matrix](../capability-matrix.md)
