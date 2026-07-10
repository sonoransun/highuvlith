# Examples

Runnable configs, scripts, and notebooks for highuvlith. TOML configs drive the
CLI (`highuvlith simulate` for projection sources, `highuvlith deep` for
deep-layer process modes); the `.py` scripts and notebooks use the Python API.
Each row links to the documentation page that explains the physics behind it.

## Projection source configs

Run with `highuvlith simulate --config examples/<file>` (add `--output img.png`
to render). The source family is selected by the `[source] type` tag.

| File | What it demonstrates | Run command | Docs |
|------|----------------------|-------------|------|
| `sim.toml` | F₂ excimer VUV at 157.63 nm (default `type`, refractive CaF₂) | `highuvlith simulate --config examples/sim.toml` | [vuv-excimer](../docs/sources/vuv-excimer.md) |
| `sim_lpa_fel.toml` | LPA-FEL at 25 nm (BELLA 500 MeV target) | `highuvlith simulate --config examples/sim_lpa_fel.toml` | [lpa-fel](../docs/sources/lpa-fel.md) |
| `sim_lpp_sn.toml` | Sn laser-produced plasma at 13.5 nm through a Schwarzschild objective | `highuvlith simulate --config examples/sim_lpp_sn.toml` | [lpp](../docs/sources/lpp.md) |
| `sim_lpp_gd.toml` | Gd laser-produced plasma at 6.7 nm beyond-EUV | `highuvlith simulate --config examples/sim_lpp_gd.toml` | [lpp](../docs/sources/lpp.md) |
| `sim_synchrotron.toml` | Undulator beamline with wavelength derived from machine parameters | `highuvlith simulate --config examples/sim_synchrotron.toml` | [synchrotron](../docs/sources/synchrotron.md) |
| `sim_hhg.toml` | High-harmonic-generation comb, cutoff-limited harmonic selection | `highuvlith simulate --config examples/sim_hhg.toml` | [hhg](../docs/sources/hhg.md) |
| `sim_xfel.toml` | XFEL SASE / seeded near 13.5 nm | `highuvlith simulate --config examples/sim_xfel.toml` | [xfel](../docs/sources/xfel.md) |
| `sim_ics.toml` | Inverse-Compton-scattering source (theoretical) | `highuvlith simulate --config examples/sim_ics.toml` | [inverse-compton](../docs/sources/inverse-compton.md) |
| `sim_ssmb.toml` | Steady-state-microbunching EUV design point (theoretical) | `highuvlith simulate --config examples/sim_ssmb.toml` | [ssmb](../docs/sources/ssmb.md) |
| `sim_entangled.toml` | Entangled-photon NOON source bridging quantum lithography (theoretical) | `highuvlith simulate --config examples/sim_entangled.toml` | [entangled-photon](../docs/sources/entangled-photon.md) |

## Deep-layer process configs

Run with `highuvlith deep --config examples/<file>`; the process mode is selected
by the `[deep] mode` tag.

| File | What it demonstrates | Run command | Docs |
|------|----------------------|-------------|------|
| `liga.toml` | LIGA deep-X-ray shadow print through thick PMMA, high aspect ratio | `highuvlith deep --config examples/liga.toml` | [liga-deep-xray](../docs/processes/liga-deep-xray.md) |
| `volumetric.toml` | z-resolved volumetric exposure with 3D fast-marching development | `highuvlith deep --config examples/volumetric.toml` | [volumetric-exposure](../docs/processes/volumetric-exposure.md) |
| `grayscale.toml` | Grayscale surface-relief design in a single exposure | `highuvlith deep --config examples/grayscale.toml` | [grayscale](../docs/processes/grayscale.md) |
| `interference.toml` | Multi-beam interference / two-photon holographic lithography | `highuvlith deep --config examples/interference.toml` | [interference-volumetric](../docs/processes/interference-volumetric.md) |

## Scripts

| File | What it demonstrates | Run command | Docs |
|------|----------------------|-------------|------|
| `mnsl_example.py` | Moiré nanosphere lattice sweeps and emission-pattern analysis | `python examples/mnsl_example.py` | [research-modules](../docs/research-modules.md) |
| `generate_demo.py` | Batch-renders the demo animation (`demo_vp9.webm`, `demo_mpeg4.mp4`) | `python examples/generate_demo.py` | [python-api](../docs/python-api.md) |

## Notebooks

Jupyter notebooks in [`notebooks/`](notebooks/); open with `jupyter lab examples/notebooks/<file>`.

| File | What it demonstrates | Run command | Docs |
|------|----------------------|-------------|------|
| `notebooks/01_quickstart_aerial.ipynb` | First aerial image and metrics from the Python one-liner | `jupyter lab examples/notebooks/01_quickstart_aerial.ipynb` | [python-api](../docs/python-api.md) |
| `notebooks/02_source_gallery.ipynb` | Side-by-side gallery of all nine source families | `jupyter lab examples/notebooks/02_source_gallery.ipynb` | [sources](../docs/sources/index.md) |
| `notebooks/03_deep_litho.ipynb` | Deep-layer walkthrough: LIGA, volumetric, grayscale, interference | `jupyter lab examples/notebooks/03_deep_litho.ipynb` | [processes](../docs/processes/index.md) |
