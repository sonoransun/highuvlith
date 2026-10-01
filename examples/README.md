# Examples

Runnable configs, scripts and notebooks for highuvlith. TOML configs drive the
CLI (`highuvlith simulate` for projection imaging, `highuvlith deep` for the
deep-layer process modes selected by `[deep] mode`); the `.py` scripts and the
notebooks use the Python API. Each row links to the documentation page that
explains the physics, and the badge in brackets in a file's header comment says
how mature the model or the machine is (see the
[capability matrix](../docs/capability-matrix.md)).

## Setup

Run everything from the repository root.

```bash
# CLI: install the binary (or replace `highuvlith` below by
# `cargo run --release -p highuvlith-cli --`)
cargo install --path crates/highuvlith-cli

# Python: highuvlith is not on PyPI; build the extension into a virtualenv
python -m venv .venv && source .venv/bin/activate
pip install maturin numpy matplotlib jupyter
maturin develop --release
```

## Projection imaging configs (`highuvlith simulate`)

The source family is selected by the `[source] type` tag (`vuv` when absent). Add
`--output img.png` to render the aerial image. EUV-range examples use mirror
optics (`type = "euv_projection"` or `"schwarzschild"`); the CLI warns when
refractive optics are selected below 50 nm.

| File | What it demonstrates | Run command | Docs |
|------|----------------------|-------------|------|
| `sim.toml` | F₂ excimer at 157.63 nm, refractive NA 0.75, 65 nm lines / 180 nm pitch | `highuvlith simulate --config examples/sim.toml` | [vuv-excimer](../docs/sources/vuv-excimer.md) |
| `sim_hg_gline.toml` | Mercury g-line lamp (435.8 nm), the first wafer-stepper wavelength | `highuvlith simulate --config examples/sim_hg_gline.toml` | [duv-heritage](../docs/sources/duv-heritage.md) |
| `sim_hg_iline.toml` | Mercury i-line lamp (365.0 nm) | `highuvlith simulate --config examples/sim_hg_iline.toml` | [duv-heritage](../docs/sources/duv-heritage.md) |
| `sim_krf.toml` | KrF excimer laser (248.3 nm) | `highuvlith simulate --config examples/sim_krf.toml` | [duv-heritage](../docs/sources/duv-heritage.md) |
| `sim_arf.toml` | ArF excimer laser (193.368 nm) | `highuvlith simulate --config examples/sim_arf.toml` | [duv-heritage](../docs/sources/duv-heritage.md) |
| `sim_lpa_fel.toml` | Laser-plasma-accelerator FEL at 25 nm, derived 2ρλ bandwidth (design projection) | `highuvlith simulate --config examples/sim_lpa_fel.toml` | [lpa-fel](../docs/sources/lpa-fel.md) |
| `sim_lpp_sn.toml` | Tin laser-produced plasma at 13.5 nm, 250 W at IF (production EUV) | `highuvlith simulate --config examples/sim_lpp_sn.toml` | [lpp](../docs/sources/lpp.md) |
| `sim_lpp_gd.toml` | Gadolinium LPP at 6.7 nm (beyond-EUV; in-band power is a projection) | `highuvlith simulate --config examples/sim_lpp_gd.toml` | [lpp](../docs/sources/lpp.md) |
| `sim_dpp.toml` | Discharge-produced plasma, Sn at 13.5 nm (2π-at-source and IF powers) | `highuvlith simulate --config examples/sim_dpp.toml` | [dpp](../docs/sources/dpp.md) |
| `sim_synchrotron.toml` | Undulator beamline, wavelength derived from machine parameters | `highuvlith simulate --config examples/sim_synchrotron.toml` | [synchrotron](../docs/sources/synchrotron.md) |
| `sim_hhg.toml` | High-harmonic generation in neon, q = 59 (13.56 nm), cutoff law | `highuvlith simulate --config examples/sim_hhg.toml` | [hhg](../docs/sources/hhg.md) |
| `sim_xfel.toml` | SASE XFEL at 13.5 nm (FLASH-class), Pierce parameter derived | `highuvlith simulate --config examples/sim_xfel.toml` | [xfel](../docs/sources/xfel.md) |
| `sim_xfel_cw_sc.toml` | CW superconducting-linac FEL at 13.5 nm (projection) | `highuvlith simulate --config examples/sim_xfel_cw_sc.toml` | [xfel](../docs/sources/xfel.md) |
| `sim_xfel_erl.toml` | Energy-recovery-linac FEL, kW-class design point (projection) | `highuvlith simulate --config examples/sim_xfel_erl.toml` | [xfel](../docs/sources/xfel.md) |
| `sim_sxrl.toml` | Plasma soft-X-ray laser, Ni-like Ag at 13.9 nm | `highuvlith simulate --config examples/sim_sxrl.toml` | [soft-xray-laser](../docs/sources/soft-xray-laser.md) |
| `sim_ics.toml` | Inverse Compton scattering at 13.5 nm (theoretical) | `highuvlith simulate --config examples/sim_ics.toml` | [inverse-compton](../docs/sources/inverse-compton.md) |
| `sim_ssmb.toml` | Steady-state microbunching, kW-class at 13.5 nm (projection) | `highuvlith simulate --config examples/sim_ssmb.toml` | [ssmb](../docs/sources/ssmb.md) |
| `sim_smith_purcell.toml` | Smith–Purcell free-electron grating at 13.5 nm (theoretical) | `highuvlith simulate --config examples/sim_smith_purcell.toml` | [smith-purcell](../docs/sources/smith-purcell.md) |
| `sim_entangled.toml` | Entangled-photon (N00N) source: derived flux and exposure-time bounds; the CLI image is classical | `highuvlith simulate --config examples/sim_entangled.toml` | [entangled-photon](../docs/sources/entangled-photon.md) |

`highuvlith sweep --config <file>` runs a focus/dose sweep over any of these
configs ([CLI reference](../docs/cli.md)).

## Deep-layer and X-ray configs (`highuvlith deep`)

| File | `[deep] mode` | What it demonstrates | Run command | Docs |
|------|---------------|----------------------|-------------|------|
| `liga.toml` | `liga` | Bending-magnet LIGA through 500 µm PMMA: depth dose, absolute exposure time, Fresnel edge | `highuvlith deep --config examples/liga.toml` | [liga-deep-xray](../docs/processes/liga-deep-xray.md) |
| `sim_xray_tube.toml` | `liga` | Laboratory X-ray tube (W, 60 kV) as a LIGA source | `highuvlith deep --config examples/sim_xray_tube.toml` | [xray-tube](../docs/sources/xray-tube.md) |
| `sim_betatron.toml` | `liga` | Laser-wakefield betatron X-rays as a LIGA-style source (theoretical) | `highuvlith deep --config examples/sim_betatron.toml` | [betatron](../docs/sources/betatron.md) |
| `volumetric.toml` | `volumetric` | z-resolved exposure into 300 nm resist, anisotropic bake, level-set development with surface inhibition | `highuvlith deep --config examples/volumetric.toml --output profile.png` | [volumetric-exposure](../docs/processes/volumetric-exposure.md) |
| `grayscale.toml` | `grayscale` | Blazed grating in photoresist (analytic 2.5D grayscale) | `highuvlith deep --config examples/grayscale.toml` | [grayscale](../docs/processes/grayscale.md) |
| `interference.toml` | `interference` | Two-beam holographic grating at 266 nm | `highuvlith deep --config examples/interference.toml` | [interference-volumetric](../docs/processes/interference-volumetric.md) |
| `talbot.toml` | `talbot` | Talbot / achromatic Talbot lithography, 100 nm grating at 13.5 nm | `highuvlith deep --config examples/talbot.toml` | [talbot](../docs/processes/talbot.md) |

## Notebooks

Jupyter notebooks in [`notebooks/`](notebooks/). They are stored without outputs;
open them with `jupyter lab examples/notebooks/<file>`, or run one headlessly
with the command in the table (`jupyter execute` does not write outputs back into
the file). Each runs in seconds with a release build; CI executes all six.

| File | What it demonstrates | Run command | Docs |
|------|----------------------|-------------|------|
| `notebooks/01_quickstart_aerial.ipynb` | One-line simulation, the engine and commensurate grids, exact through-focus imaging, a dose-aware ED process window, a 2D resist profile | `jupyter execute examples/notebooks/01_quickstart_aerial.ipynb` | [python-api](../docs/python-api.md) |
| `notebooks/02_source_gallery.ipynb` | All 30 presets of the fourteen source families: table, spectra and pupil fills, equal-k₁ imaging, why coherent sources need off-axis light, status per family | `jupyter execute examples/notebooks/02_source_gallery.ipynb` | [sources](../docs/sources/index.md) |
| `notebooks/03_deep_litho.ipynb` | LIGA depth dose, filters and absolute exposure time; Fresnel vs Gaussian edge; X-ray tube LIGA; interference; volumetric exposure, bake and level-set development | `jupyter execute examples/notebooks/03_deep_litho.ipynb` | [processes](../docs/processes/index.md) |
| `notebooks/04_imaging_physics.ipynb` | Resolution vs partial coherence, off-axis DOF with exact vs legacy defocus, TE/TM vector imaging, 193 nm immersion, per-wavelength HHG comb imaging, EUV NA 0.33 vs 0.55 | `jupyter execute examples/notebooks/04_imaging_physics.ipynb` | [pipeline](../docs/pipeline.md), [vector-imaging](../docs/vector-imaging.md) |
| `notebooks/05_source_physics.ipynb` | Power-wavelength landscape, derived quantities, dose-limited throughput, FEL/SSMB/HHG sensitivities, the gaps of the theoretical sources, photon shot noise | `jupyter execute examples/notebooks/05_source_physics.ipynb` | [sources](../docs/sources/index.md) |
| `notebooks/06_process_and_optimization.ipynb` | ED windows by illumination, adjoint vs proxy ILT, fragment OPC, SRAF DOF, level set vs fast marching, CAR quencher, Talbot carpet / DTL / ATL, EUV-IL | `jupyter execute examples/notebooks/06_process_and_optimization.ipynb` | [research-modules](../docs/research-modules.md), [talbot](../docs/processes/talbot.md) |

## Scripts and demo video

| File | What it demonstrates | Run command | Docs |
|------|----------------------|-------------|------|
| `mnsl_example.py` | Moiré nanosphere (MNSL) emission maps and parameter sweeps (🧪 heuristic emission model); writes four PNGs to the current directory | `python examples/mnsl_example.py` | [research-modules](../docs/research-modules.md) |
| `generate_demo.py` | Renders the demo video (through focus, TE vs TM, source landscape, level-set development, Talbot carpet) and the stills in `docs/assets/images/demo/`; needs ffmpeg with libvpx-vp9 | `python examples/generate_demo.py` (`--quick` for a smoke test, `--stills-only` for the stills) | [python-api](../docs/python-api.md) |
| `demo_vp9.webm`, `demo_mpeg4.mp4` | The rendered video (about 45 s, 1920 × 1080). Not tracked in git: generate it with the command above | `python examples/generate_demo.py` | — |
