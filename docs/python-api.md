---
description: "The highuvlith Python package: the canonical run, the configuration, engine and result classes, the high-level functions with verified examples, and the runtime contract (GIL, zero-copy arrays, honesty fields)."
---

# Python API

**Status:** ✅ Implemented — PyO3 bindings that release the GIL in every compute path and hand the large result arrays to NumPy as zero-copy read-only views; each capability keeps its own grade from the matrix · [matrix](capability-matrix.md#frontends-and-infrastructure)

<p class="huv-lead">The package <code>highuvlith</code> is a thin layer over the compiled Rust engine <code>highuvlith._native</code>: native configuration, engine and result classes, the convenience functions of <code>highuvlith.api</code> (re-exported at top level) and the plotting package <code>highuvlith.viz</code>. It is for scripts, notebooks and arrays; it adds no physics of its own, so every result carries the grade of the core model behind it.</p>

**Same thing elsewhere:** CLI [`simulate`](cli.md#simulate) · TOML [`examples/sim.toml`](../examples/sim.toml) · GUI [`--preset f2 --cd 65 --pitch 180`](gui.md#running) · physics [Simulation pipeline](pipeline.md)

<!-- start-first-image -> python-api.md -->
<figure markdown="span">

![The canonical first run drawn: a two-period aerial image of 65 nm lines on a 180 nm pitch at 157.63 nm, NA 0.75, and its cross-section with the 0.3 threshold, a 41.2 nm printed line, contrast 0.454 and NILS 0.65.](assets/images/sim/start-first-image-light.png#gh-light-mode-only)
![The canonical first run drawn: a two-period aerial image of 65 nm lines on a 180 nm pitch at 157.63 nm, NA 0.75, and its cross-section with the 0.3 threshold, a 41.2 nm printed line, contrast 0.454 and NILS 0.65.](assets/images/sim/start-first-image-dark.png#gh-dark-mode-only)

<figcaption><strong>Your first image: the numbers printed above, drawn.</strong> 65 nm lines at 180 nm pitch, F<sub>2</sub> 157.63 nm, NA 0.75, σ 0.7 — the same run as <code>examples/sim.toml</code>; contrast, NILS and the printed CD at the 0.3 threshold are computed at figure time by the same call. <em>Model: Hopkins imaging (factorized TCC/SOCS) ✅; thin mask, 2 % uniform flare, no resist.</em></figcaption>
</figure>

Units throughout: lengths in nm unless the name says otherwise (`_um`, `_mm`, `_m`), doses in mJ/cm², power in W, photon energies in eV (or keV where named `_kev`), angles in degrees (`_deg`) or radians (`_rad`). Pupil-fill σ is in units of NA/λ. Everything except `viz` and `interactive` is importable as `import highuvlith as huv`.

## Find by task

<!-- huv-table: stack -->

| I want to … | Use | Section |
|---|---|---|
| image a line/space or contact pattern in one call | `simulate_line_space`, `simulate_contact_hole` | [Quick start](#quick-start), [High-level functions](#high-level-functions) |
| build source, optics, mask and grid myself and reuse the engine | `SourceConfig`, `OpticsConfig`, `MaskConfig`, `GridConfig`, `SimulationEngine` | [Configuration classes](#configuration-classes), [Imaging engine](#imaging-engine) |
| sweep dose and focus into a process window | `process_window`, `BatchSimulator` | [Process window and ED analysis](#process-window-and-ed-analysis) |
| compare illumination shapes or polarizations | `compare_illumination`, `imaging="vector"` | [High-level functions](#high-level-functions), [Vector imaging](python-api-modules.md#vector-imaging) |
| list every source preset or estimate wafers per hour | `source_landscape`, `wafer_throughput` | [Sources: landscape and throughput](python-api-modules.md#sources-landscape-and-throughput) |
| expose thick PMMA with X-rays, look up n, k or a multilayer | `simulate_liga`, `refractive_index`, `MultilayerMirror` | [LIGA, X-ray materials and multilayers](python-api-modules.md#liga-x-ray-materials-and-multilayers) |
| expose, bake and develop a resist volume | `simulate_volumetric`, `peb_gaussian`, `develop_level_set` | [Volumetric, development and bake](python-api-modules.md#volumetric-development-and-bake) |
| print with interference, Talbot or EUV-IL gratings | `simulate_interference`, `simulate_talbot`, `simulate_euv_il` | [Interference, Talbot and EUV-IL](python-api-modules.md#interference-talbot-and-euv-il) |
| optimize a mask, insert assists, lay out spacers | `optimize_ilt`, `fragment_opc`, `insert_srafs`, `sadp` | [Optimization and patterning](#optimization-and-patterning) |
| estimate shot noise and line-edge roughness | `compute_ler_lwr`, `photons_per_square` | [Stochastics](python-api-modules.md#stochastics) |
| know what crosses the GIL and which arrays are views | — | [Runtime behaviour](#runtime-behaviour-gil-arrays-honesty-fields) |
| plot a result | `highuvlith.viz` | [Visualization](python-api-modules.md#visualization) |

## At a glance

<!-- huv-table: stack -->

| Name | Purpose | Grade | Section |
|---|---|---|---|
| `SourceConfig` | one class for all 14 source families and their presets | ✅ Implemented bindings; grade per family | [Configuration classes](#sourceconfig) |
| `OpticsConfig` | refractive, immersion, EUV projection, Schwarzschild, zone plate | grade per optic | [Configuration classes](#opticsconfig) |
| `MaskConfig`, `GridConfig` | thin-mask patterns with exact spectra; commensurate grids | 🔶 Simplified mask model; ✅ Implemented spectra and grids | [Configuration classes](#maskconfig) |
| `SimulationEngine`, `AerialImageResult` | the Hopkins/SOCS engine and its image | ✅ Implemented | [Imaging engine](#imaging-engine) |
| `BatchSimulator`, `ProcessWindowResult` | sweeps and the exposure-defocus window | 🔶 Simplified constant-threshold resist | [Process window and ED analysis](#process-window-and-ed-analysis) |
| `highuvlith.api` | one-call wrappers that choose commensurate grids | the wrapped model's grade | [High-level functions](#high-level-functions) |
| `simulate_liga`, `MultilayerMirror`, `refractive_index` | deep X-ray exposure; CXRO/NIST constants; materials | ✅ Implemented depth dose; 🔶 Simplified database | [LIGA, X-ray materials and multilayers](python-api-modules.md#liga-x-ray-materials-and-multilayers) |
| `simulate_volumetric`, `peb_*`, `develop_*` | the 3-D resist chain | 🔶 Simplified chain; ✅ Implemented bake and solvers | [Volumetric, development and bake](python-api-modules.md#volumetric-development-and-bake) |
| `optimize_ilt`, `fragment_opc`, `insert_srafs`, `sadp`, `simulate_dsa` | computational lithography and patterning | ✅ Implemented ILT/OPC; 🔶 Simplified SRAF, spacers, DSA | [Optimization and patterning](#optimization-and-patterning) |
| `compute_ler_lwr`, `TwoBeamNPhoton`, `MnslEngine` | stochastics and the research modules | 🔶 Simplified LER; 🧪 Theoretical quantum, MNSL | [Stochastics](python-api-modules.md#stochastics) · [Quantum](python-api-modules.md#quantum-n-photon-research-module) · [MNSL](python-api-modules.md#mnsl) |
| `highuvlith.viz` | matplotlib and plotly helpers with one validated palette | — | [Visualization](python-api-modules.md#visualization) |

## Installation

highuvlith is **not published on PyPI**, so build it from source. You need a Rust toolchain and Python ≥ 3.10. The extension is built with [maturin](https://www.maturin.rs/); `pyproject.toml` points it at `crates/highuvlith-py` and installs the module as `highuvlith._native`.

```bash
git clone https://github.com/sonoransun/highuvlith.git
cd highuvlith
python -m venv .venv && source .venv/bin/activate
pip install maturin numpy
maturin develop --release          # optimized build, into the venv
```

`maturin develop` without `--release` builds a debug extension: it compiles faster and runs much slower. To install with optional extras from the checkout, use `pip install ".[viz]"`. The extras defined in `pyproject.toml`:

<!-- huv-table: stack -->

| Extra | Pulls in | Needed for |
|---|---|---|
| `viz` | matplotlib ≥ 3.7 | `highuvlith.viz` |
| `interactive` | plotly ≥ 5, polars ≥ 1 | `highuvlith.viz.plotly_viz`, `plot_isosurface`, MNSL plotly backend |
| `notebook` | ipywidgets ≥ 8 | `highuvlith.interactive` |
| `hdf5` | h5py ≥ 3.9 | HDF5 export in `highuvlith.io` |
| `dev` | pytest, ruff, mypy | the test suite and linters |
| `all` | viz + interactive + notebook + hdf5 | everything |

The package version is `0.2.0` (`highuvlith.__version__`).

## Quick start

### Your first image

The canonical run — 65 nm lines at 180 nm pitch with the default F₂ 157.63 nm source (σ 0.7) at NA 0.75 — is one call; it is the figure above, `examples/sim.toml` on the [CLI](cli.md#simulate) and example 1 of [Getting started](getting-started.md):

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

### Same run, four front ends

The same numbers come out of every front end; the Python block below is executed in CI and the CLI transcript it matches is recorded on the [CLI page](cli.md#simulate) (contrast 0.4545, printed CD 41.20 nm, NILS 0.654). The requested 1 nm pixel became 0.703125 nm so that the 256-pixel field holds exactly one pitch ([commensurate grids](#maskconfig)).

<div class="huv-rosetta" markdown>

**Python**

<!-- verify-example -->
```python
import highuvlith as huv

r = huv.simulate_line_space(65.0, 180.0, na=0.75)
print(f"contrast {r.contrast:.4f}, CD {r.cd_nm:.2f} nm, NILS {r.nils:.3f}, {r.status}")
print(r.config["pixel_nm"], r.config["periods_in_field"])
```

???+ success "Output"

    ```text
    contrast 0.4545, CD 41.20 nm, NILS 0.654, ✅
    0.703125 1
    ```

**CLI**

```bash
cargo build --release -p highuvlith-cli
target/release/highuvlith simulate --config examples/sim.toml --output aerial.png
```

**TOML**

```toml
# examples/sim.toml (abridged)
[source]
type = "vuv"
preset = "f2"
sigma = 0.7

[optics]
na = 0.75

[mask]
cd_nm = 65.0
pitch_nm = 180.0
```

**GUI**

```bash
cargo run --release -p highuvlith-gui -- --preset f2 --cd 65 --pitch 180
```

</div>

### Build the objects yourself

For full control, build the source, optics, mask and grid explicitly; the engine precomputes the TCC/SOCS decomposition once, so evaluating many focus planes or masks afterwards is cheap. This is a 100 nm line at 300 nm pitch; `commensurate_grid` turns the requested 2 nm pixel into 2.34375 nm so the 128-pixel field holds exactly one pitch, whereas a bare `GridConfig(size, pixel_nm)` is used as given (and warns if the field is not commensurate):

<!-- verify-example -->
```python
import numpy as np
import highuvlith as huv

source = huv.SourceConfig.f2_laser(sigma=0.7)            # 157.63 nm
optics = huv.OpticsConfig(numerical_aperture=0.75)
mask = huv.MaskConfig.line_space(cd_nm=100.0, pitch_nm=300.0)
grid = mask.commensurate_grid(size=128, target_pixel_nm=2.0)
engine = huv.SimulationEngine(source, optics, mask, grid=grid, max_kernels=20)

img = engine.compute_aerial_image(focus_nm=0.0)
print(f"contrast {img.image_contrast():.4f}, CD {img.cd(threshold=0.3, tone='dark'):.2f} nm")
print("read-only view:", not img.intensity.flags.writeable, img.intensity.shape)

planes = engine.compute_through_focus([0.0, 100.0, 200.0])
print("contrast vs focus:", [round(p.image_contrast(), 4) for p in planes])
broad = engine.compute_multiwavelength(focus_nm=0.0)
print(f"multiwavelength contrast {broad.image_contrast():.4f}")
d = engine.imaging_diagnostics()
print(d["num_kernels"], round(d["captured_energy_fraction"], 4), round(engine.clear_field_intensity(), 4))
```

???+ success "Output"

    ```text
    contrast 0.8488, CD 98.20 nm
    read-only view: True (128, 128)
    contrast vs focus: [0.8488, 0.7355, 0.4678]
    multiwavelength contrast 0.8466
    20 0.9983 1.0
    ```

## Runtime behaviour: GIL, arrays, honesty fields

**GIL.** Every compute call runs inside `py.allow_threads`, so other Python threads keep running and thread pools run in parallel. That covers engine construction, all `SimulationEngine` image methods, `BatchSimulator` sweeps and process windows, volumetric/LIGA/Talbot/interference, level set and PEB, ILT/OPC/SRAF/LELE/DSA, stochastic LER, multilayer scans and N00N images. Inside, the Rust core parallelizes with Rayon, and the images are bit-reproducible whatever the thread count.

**Arrays.**

<!-- huv-table: stack -->

| What | How it crosses into Python |
|---|---|
| `AerialImageResult.intensity`, `VolumetricResult.values`, `HeightMapResult.values`, the `TalbotResult` images and carpet, the `LeleResult` images, `DsaResult.pattern` | **Zero-copy, read-only** NumPy view of the Rust buffer (the result classes are frozen, so the buffer never changes). The view keeps its owner alive. Call `.copy()` if you need a writable array. |
| Arrays returned by a function call (`n_photon_absorption_image`, `blazed_grating`, `pupil_map`, `rasterize`, …) | Moved into NumPy with no copy (`into_pyarray`); you own them. |
| Small accessors (coordinate axes `x_nm`/`y_nm`/`z_nm`, histories, depth-dose curves, lists of tuples) | Fresh copies. |

**Honesty fields.** Results of simplified or theoretical models carry `.status` and `.notes`: `.status` is the capability badge, `"✅"`, `"🔶"`, `"🧪"` or a split such as `"✅/🔶"` (the native string keeps the slash form); `.notes` is a list of the stated approximations. Classes that have them: `ResistProfileResult`, `ProcessWindowResult`, `LigaResult`, `LigaEdgeProfile`, `LevelSetResult`, `CarPebResult`, `TalbotResult`, `EuvIlResult`, `IltResult`, `OpcResult`, `SrafResult`, `DofComparison`, `SpacerPatterningResult`, `LeleResult`, `DsaResult`, `LerResult`, `MnslResult`, `TwoBeamNPhoton`, `NoonImageResult`, plus the `api` dataclasses `FullResult`, `QuantumResult` and `IlluminationComparison`.

**Validation.** Bad parameters raise `ValueError` before any compute starts. Inputs are checked three times: in the Python wrappers, in the PyO3 shims and in the Rust core.

**Commensurate grids.** If a periodic mask's field does not hold a whole number of periods, constructing an engine emits a `UserWarning`. The FFT's periodic boundary would otherwise insert a defect at the field edge. The `api` functions always choose a commensurate grid.

## Configuration classes

### `SourceConfig`

One class wraps all 14 source families (the core `SourceKind` variants; DUV/UV heritage presets are VUV-family `"vuv"` sources, listed separately below). The default constructor `SourceConfig(wavelength_nm=157.63, sigma_outer=0.7, bandwidth_pm=1.1, spectral_samples=5)` builds a generic excimer-like source. The static factories build presets; each family keeps its grade from the [capability matrix](./capability-matrix.md#light-sources) and has its own page under [Sources](./sources/index.md). CLI: `highuvlith sources` lists the same presets with their `[source]` TOML.

<!-- huv-table: stack -->

| Family | Grade | Factories |
|---|---|---|
| [VUV excimer](sources/vuv-excimer.md) | ✅ Implemented (🧪 Ar₂ preset) | `f2_laser(sigma=0.7)` 157.63 nm; `ar2_laser(sigma=0.7)` 126 nm (hypothetical) |
| [DUV/UV heritage](sources/duv-heritage.md) | ✅ Implemented (🔶 parameters) | `arf_laser()` 193.368 nm; `krf_laser()` 248.3 nm; `hg_i_line()` / `hg_h_line()` / `hg_g_line()` (CW lamps, `sigma=0.7`) |
| [LPA-FEL](sources/lpa-fel.md) | 🔶 Simplified | `lpa_fel_bella_25nm(sigma=0.7)` (design projection); `lpa_fel(…)` |
| [Laser-produced plasma](sources/lpp.md) | ✅ Implemented (🔶 CE defaults, étendue; 🧪 Gd/Tb power) | `lpp_sn_13nm5(sigma=0.9, drive_laser="co2")` (250 W at IF; `"solid_state_1um"`, `"thulium_2um"`); `lpp_sn_13nm5_500w()`; `lpp_gd_6nm7()`; `lpp_tb_6nm5()` |
| [Discharge plasma](sources/dpp.md) | 🔶 Simplified | `dpp_sn_13nm5(sigma=0.9)`; `dpp_xe_13nm5(sigma=0.9)` |
| [Synchrotron](sources/synchrotron.md) | ✅ Implemented (🔶 coherence) | `synchrotron_undulator(…)`; `synchrotron_compact_euv()`; `synchrotron_liga_bending_magnet()` (2.5 GeV, 1.5 T) |
| [HHG](sources/hhg.md) | ✅ Implemented (🔶 harmonic intensities) | `hhg(…)`; `hhg_ne_13nm5()`; `hhg_ar_30nm()` |
| [XFEL](sources/xfel.md) | ✅ Implemented (🧪 CW-SC and ERL presets) | `xfel_flash_13nm5()`; `xfel_fermi_seeded()`; `xfel_cw_sc_13nm5()`; `xfel_erl_13nm5()` |
| [Soft-X-ray laser](sources/soft-xray-laser.md) | ✅ Implemented (🔶 assumed splits, no gain) | `sxrl_ar_46nm9()`; `sxrl_ag_13nm9()`; `sxrl(scheme="ag_13nm9", …)` (`ar_46nm9`, `ag_13nm9`, `cd_13nm2`, `mo_18nm9`) |
| [X-ray tube](sources/xray-tube.md) | 🔶 Simplified (✅ lines, edges, Duane–Hunt, Be window) | `xray_tube(anode="W", …)` (`W`, `Mo`, `Cu`, `Rh`; for LIGA/proximity, not projection imaging) |
| [Inverse Compton](sources/inverse-compton.md) | 🧪 Theoretical | `ics(…)`; `ics_compact_euv_13nm5()` |
| [SSMB](sources/ssmb.md) | 🧪 Theoretical | `ssmb(…)`; `ssmb_euv_13nm5()` |
| [Betatron](sources/betatron.md) | 🧪 Theoretical | `betatron(…)` |
| [Smith–Purcell](sources/smith-purcell.md) | 🧪 Theoretical | `smith_purcell(…)` |
| [Entangled photons](sources/entangled-photon.md) | 🧪 Theoretical | `entangled_noon(wavelength_nm=157.63, n=2, fidelity=1.0, pair_rate_hz=1e6)` |

<details markdown="1">
<summary>Full signatures of the parameterised factories</summary>

```text
SourceConfig.lpa_fel(wavelength_nm, sigma=0.7, electron_energy_mev=500, bandwidth_pm=None,
                     pulse_duration_fs=10, rep_rate_hz=1000, pulse_energy_uj=None,
                     undulator_period_mm=None, undulator_k=None, num_periods=200,
                     peak_current_a=None, norm_emittance_um=None, energy_spread_rel=None, beta_m=None)
SourceConfig.synchrotron_undulator(electron_energy_gev=0.538, period_mm=20, k=1, num_periods=100,
                     harmonic=1, ring_current_ma=200, emittance_x_nm_rad=None,
                     emittance_y_nm_rad=None, energy_spread_rel=5e-4)
SourceConfig.hhg(driver_wavelength_nm=800, gas="neon", driver_intensity_w_cm2=4e14, harmonic=59,
                     monochromator_bandwidth_pm=15, full_comb=False, driver_average_power_w=None,
                     conversion_efficiency=None, comb_passband_nm=None)
SourceConfig.sxrl(scheme="ag_13nm9", pulse_energy_uj=None, rep_rate_hz=None,
                     pulse_duration_ps=None, rel_linewidth=None)
SourceConfig.xray_tube(anode="W", kvp=None, current_ma=None, be_window_um=250)
SourceConfig.ics(target_wavelength_nm=13.5, laser_wavelength_nm=1030, laser_a0=0.1, …)
SourceConfig.ssmb(ring_energy_mev=400, modulation_wavelength_nm=1053, target_wavelength_nm=13.5,
                     average_power_w=1000, …)
SourceConfig.betatron(electron_energy_mev=200, plasma_density_cm3=1e19, betatron_amplitude_um=1,
                     interaction_length_mm=3, bunch_charge_pc=50, rep_rate_hz=10)
SourceConfig.smith_purcell(target_wavelength_nm=13.5, electron_energy_kev=30, diffraction_order=1,
                     observation_angle_deg=90, grating_period_nm=None, num_periods=100,
                     beam_current_na=10, coupling_efficiency=1e-3, impact_height_nm=0)
```

</details>

Properties and methods:

<!-- huv-table: stack -->

| Member | Returns | Meaning |
|---|---|---|
| `wavelength_nm`, `bandwidth_pm` | float | Centre wavelength (nm), FWHM bandwidth (pm). For synchrotron/HHG/ICS these are derived; for tubes/betatron `wavelength_nm` is hc/⟨E⟩. |
| `sigma_outer`, `spectral_samples`, `kind` | float, int, str | Pupil σ, polychromatic sample count, family tag (`"vuv"`, `"lpp"`, `"xray_tube"`, …). |
| `photon_energy_ev`, `photon_density_per_mj_cm2` | float | Photon energy (eV); incident photons per nm² at 1 mJ/cm². |
| `average_power_w`, `pulse_energy_j`, `rep_rate_hz`, `pulse_duration_fs`, `electron_energy_mev` | float or None | Each family's own definition: plasma sources in band at IF, lasers at the output, tubes as filtered 4π X-ray power. `None` means not modeled. |
| `shot_to_shot_rms`, `transverse_coherence_fraction` | float / float or None | Relative pulse-energy jitter (drives stochastic dose jitter); coherent fraction. |
| `derived_quantities()` | list of `(name, value, unit, note)` | Physics derived from machine parameters (gain lengths, flux, coherent fraction, HVM gaps, …). |
| `derived_quantity(name)` | float or None | One of the above by name. |
| `wafer_throughput(dose_mj_cm2=30, …)` | dict or None | Dose-limited wafers/hour (🔶); see [throughput](python-api-modules.md#sources-landscape-and-throughput). |
| `illumination` | tuple | Pupil fill: `("conventional", σ)`, `("annular", σ_in, σ_out)`, `("dipole", σ_c, σ_r, orientation_deg)`, `("quadrupole", σ_c, σ_r, opening_angle_deg)`, `("coherent_gaussian", σ)`. |
| `with_illumination(shape, *params)` | SourceConfig | Copy with the pupil fill replaced; spectrum and power are unchanged. |
| `pupil_fill(n=101, extent=1.0)` | ndarray (n, n) | Relative source intensity over σ ∈ [−extent, extent]², indexed `[iy, ix]`. |
| `spectrum()` | list of `(wavelength_nm, weight)` | The spectral samples the engine uses. |
| `xray_spectrum(n_bins=200)` | list of `(E_keV, fraction)` or None | Relative spectrum (X-ray tube, betatron). |
| `spectral_flux_density(distance_mm, n_bins=200)` | list of `(E_keV, photons s⁻¹ mm⁻² keV⁻¹)` or None | Absolute flux density at a distance (tube, betatron); feed it to `simulate_liga(flux_density=…)`. |

### `OpticsConfig`

<!-- huv-table: stack -->

| Constructor / preset | Grade | Notes |
|---|---|---|
| `OpticsConfig(…)` | ✅ Implemented | Dry refractive projection optics, NA < 1. |
| `OpticsConfig.immersion(…)` / `immersion_193i()` | ✅ Implemented | NA up to 0.95·n; defocus is applied in the medium. |
| `OpticsConfig.euv_projection(…)` / `euv_nxe()` / `euv_high_na()` | 🔶 Simplified | Isotropic wafer-side pupil. High-NA uses NA 0.55 with an **assumed** 0.2·NA obscuration; no anamorphic magnification and no mask 3D. |
| `OpticsConfig.schwarzschild(…)` | ✅ Implemented | Annular reflective objective. |
| `OpticsConfig.zone_plate(…)` | ✅ Implemented | First-order Fresnel zone plate. |
| `.with_multilayer_pupil(…)` | 🔶 Simplified | Angle-dependent multilayer amplitude/phase across the pupil from user-supplied incidence-angle maps. |

```text
OpticsConfig(numerical_aperture=0.75, reduction=4.0, flare_fraction=0.02)
OpticsConfig.immersion(numerical_aperture=1.35, immersion_index=1.437, reduction=4.0, flare_fraction=0.02)
OpticsConfig.euv_projection(numerical_aperture=0.33, central_obscuration=0.0, reduction=4.0, flare=0.0)
OpticsConfig.schwarzschild(numerical_aperture=0.33, obscuration_ratio=0.25, reduction=4.0, mirror_reflectivity=0.67, flare=0.03)
OpticsConfig.zone_plate(outer_zone_width_nm, design_wavelength_nm)
OpticsConfig.with_multilayer_pupil(mirrors, coating="mo_si", periods=40, period_nm=6.9, gamma=0.4)
    # mirrors = [(center_deg, tilt_deg, azimuth_deg, radial_deg), …]
```

Other members: `add_aberration(fringe_index, coefficient_waves)` adds a Fringe Zernike term; `with_paraxial_defocus(enabled=True)` switches to the legacy paraxial defocus phase; `pupil_map(wavelength_nm, n=65, focus_nm=0.0)` returns the complex pupil, an ndarray of shape (n, n); `rayleigh_resolution(wavelength_nm)` returns nm; properties `numerical_aperture`, `reduction`, `flare_fraction`, `immersion_index`, `central_obscuration`, `paraxial_defocus`, `has_multilayer_pupil`, `kind`. CLI · TOML: `[optics]` in [configuration](configuration.md) · physics: [Optical systems](optics.md).

### `MaskConfig`

Thin (Kirchhoff) mask model, 🔶 Simplified: binary and attenuated PSM are modeled, and there is no mask 3D. Spectra are exact closed-form Fourier series wherever possible (✅ Implemented). See [Masks](./masks.md).

<!-- huv-table: stack -->

| Member | Meaning |
|---|---|
| `MaskConfig.line_space(…)` | Bright field, opaque lines of width `cd_nm`, one centred at x = 0, periodic over the field. |
| `MaskConfig.contact_hole(…)` | Dark field, clear square holes, periodic. |
| `MaskConfig.from_features(…)` | **Canonical** custom mask. Features are painted in order, later ones on top. Each item is either a feature dict (`rect`, `gray_rect`, `polygon`, `line_space`, `rect_array`) or a shorthand: an `(x, y, w, h)` rectangle (centre and size, nm) or a vertex list (polygon). |
| `mask_from_features(…)` | **Deprecated** alias that emits a `DeprecationWarning`; use `MaskConfig.from_features`. |
| `features()` | The feature dicts (the inverse of `from_features`). |
| `periodicity()`, `check_commensurate(grid)`, `commensurate_grid(size=256, target_pixel_nm=1.0)` | Periodicity of the mask (`(px, py or None)` or None) and grids that respect it. |
| `rasterize(grid)`, `rasterize_intensity(grid)`, `spectrum(grid)`, `spectrum_method(grid)`, `validate()` | Sampling (complex128 / float64), the spectrum (`numpy.fft.fft2` layout and scale), its method (`"analytic"`, `"mixed"` or `"raster"`), validation. |

```text
MaskConfig.line_space(cd_nm, pitch_nm, orientation="vertical", offset_nm=0.0)
MaskConfig.contact_hole(diameter_nm, pitch_x_nm, pitch_y_nm=None)
MaskConfig.from_features(features, dark_field=False, mask_type="binary", transmission=0.06, phase_deg=180.0)
mask_from_features(rects=None, polygons=None, …)
```

**`GridConfig(size=512, pixel_nm=1.0)`** is a square field of `size` pixels; `GridConfig.commensurate(pitch_x_nm, pitch_y_nm=None, size=256, target_pixel_nm=1.0)` returns a grid whose field is a whole multiple of the pitch(es) — the pixel is the largest one not coarser than the target, unless one period does not fit in `size` pixels; `field_size_nm()`, `periods_in_field(pitch_nm)` and `is_commensurate_with(pitch_nm)` inspect the field.

<!-- verify-example -->
```python
import highuvlith as huv

# (x, y, w, h) rectangle shorthands (nm, centre and size) painted in order.
mask = huv.MaskConfig.from_features([(0.0, 0.0, 80.0, 400.0), (200.0, 0.0, 40.0, 400.0)])
grid = huv.GridConfig(size=64, pixel_nm=8.0)
print(mask.spectrum_method(grid), mask.periodicity())
pitch_grid = huv.GridConfig.commensurate(180.0, None, 128, 2.0)
print(pitch_grid.pixel_nm, pitch_grid.field_size_nm(), pitch_grid.is_commensurate_with(180.0))
```

???+ success "Output"

    ```text
    analytic None
    1.40625 180.0 True
    ```

The same primitives, rasterized and transformed at full resolution:

<!-- masks-primitives-spectra -> python-api.md -->
<figure markdown="span">

![Top: rasterized transmittance of a 65/180 nm grating, a 90 nm contact array, a serif line, and the same line as an attenuated PSM (amplitude 0.24, phase 180 degrees). Bottom: the grating's spectrum with exactly zero off-harmonic orders, and the serif pattern's 2-D spectrum inside the pupil circles.](assets/images/sim/masks-primitives-spectra-light.png#gh-light-mode-only)
![Top: rasterized transmittance of a 65/180 nm grating, a 90 nm contact array, a serif line, and the same line as an attenuated PSM (amplitude 0.24, phase 180 degrees). Bottom: the grating's spectrum with exactly zero off-harmonic orders, and the serif pattern's 2-D spectrum inside the pupil circles.](assets/images/sim/masks-primitives-spectra-dark.png#gh-dark-mode-only)

<figcaption><strong>Mask primitives and their exact spectra.</strong> Rasterized transmittance from <code>MaskConfig.rasterize</code> for the canonical 65/180 nm grating on a two-period commensurate field, a 90/180 nm contact-hole array, a line with two overlapping hammerhead serifs (features painted in order) and the same serif line as an attenuated PSM (|t| = √0.06, phase 180°); each panel names the <code>spectrum_method</code> the engine uses. Below, |S| from <code>MaskConfig.spectrum</code>: the grating's off-harmonic orders are exactly zero and its first orders sit outside NA/λ but inside the (1+σ)NA/λ support of F<sub>2</sub> 157.63 nm, NA 0.75, σ 0.7, which is why the canonical run images them only through off-axis source points. <em>Model: exact thin-mask spectra ✅ · antialiased rasterization ✅ · mask model 🔶 (Kirchhoff thin mask; alternating-PSM phase regions not modelled).</em></figcaption>
</figure>

### Resist, film stack, process

- **`ResistConfig(thickness_nm=150, dill_a=0.2, dill_b=0.45, dill_c=0.02, peb_diffusion_nm=30, model="mack")`** and `ResistConfig.vuv_fluoropolymer()` configure the resist. The 2D resist path is 🔶 Simplified: depth-averaged Dill, with development along the centre row only. Dill A and B are in 1/µm, C in cm²/mJ.
- **`FilmStackConfig(layers=None, substrate_n=None, substrate_k=None, superstrate_n=1.0)`** builds the thin-film transfer matrix (✅ Implemented). With no arguments it is 150 nm of resist on Si. Layers are `(name, thickness_nm, n, k)`. Methods: `add_layer`, `set_substrate`, `set_superstrate`, `reflectance(λ, angle_deg=0, polarization="unpolarized")`, `transmittance(…)`, `amplitude_coefficients(λ, angle_deg=0, polarization="te")` → `(r, t)`, and `intensity_profile(λ, z_nm, …)` → `|E(z)|²`.
- **`ProcessConfig(dose_mj_cm2=30, focus_nm=0, development_time_s=60)`** holds the process parameters.

CLI · TOML: `[process]` and the `[deep]` resist keys in [configuration](configuration.md) · physics: [Resist models](processes/resist-models.md), [Thin film](processes/thin-film.md).

## Imaging engine

### `SimulationEngine`

```text
SimulationEngine(source, optics, mask, resist=None, grid=None, max_kernels=30,
                 defocus_model="exact", kernel_energy_fraction=1.0,
                 source_points_per_axis=None, vector=None,
                 normalization="clear_field", kernel_cache_capacity=32)
```

The engine builds the factorized Hopkins TCC and its SOCS kernels once, then evaluates many masks and focus planes. Kernel sets are cached per (focus, λ). Scalar Hopkins imaging and exact non-paraxial defocus are ✅ Implemented. Keyword arguments:

<!-- huv-table: stack -->

| Keyword | Meaning |
|---|---|
| `max_kernels` | Upper bound on the number of SOCS kernels. |
| `kernel_energy_fraction` | Stop once this fraction of the TCC trace is captured (≤ 1). |
| `source_points_per_axis` | `None` = adaptive source sampling (≥ 24 cells across the fill), or a fixed count. |
| `defocus_model` | `"exact"` (defocus inside the pupil at every source point, the default) or `"kernel_phase"` (legacy fast approximation). |
| `vector` | `None` for scalar imaging, or a [`VectorSettings`](python-api-modules.md#vector-imaging) for polarized imaging (✅ Implemented). |
| `normalization` | `"clear_field"` (default: divide by the exact clear-field intensity TCC(0,0) of the same illumination, optics and focus) or `"absolute"`. |
| `kernel_cache_capacity` | Number of cached kernel sets. |

<!-- huv-table: stack -->

| Method | Returns | Notes |
|---|---|---|
| `compute_aerial_image(focus_nm=0.0, mask=None)` | `AerialImageResult` | Monochromatic image. `mask` images another mask with the same kernels. |
| `compute_through_focus(focus_nm: list, mask=None)` | list of `AerialImageResult` | One mask spectrum, planes computed in parallel. |
| `compute_multiwavelength(focus_nm=0.0, mask=None)` | `AerialImageResult` | ✅ Implemented exact: the TCC is rebuilt at every spectral sample. |
| `compute_polychromatic(focus_nm=0.0, mask=None)` | `AerialImageResult` | 🔶 Simplified narrow band: centre-λ kernels with a chromatic focus shift; valid for Δλ/λ ≪ 1. |
| `compute_from_transmittance(…)` | `AerialImageResult` | Complex `(n, n)` array on the engine grid, e.g. an ILT mask. |
| `compute_noon_ideal_image(…)` | `NoonImageResult` | 🧪 Theoretical ideal N00N limit; see [quantum](python-api-modules.md#quantum-n-photon-research-module). |
| `clear_field_intensity(focus_nm=0.0)` | float | Absolute TCC(0,0); multiply a normalized image by it to recover absolute intensity. |
| `measure_cd(…)` | float (nm) | With `dose_to_clear_mj_cm2`, the threshold is E_th / dose. |
| `image_contrast(focus_nm=0, mask=None)` | float | (I_max − I_min)/(I_max + I_min). |
| `compute_resist_profile(…)` | `ResistProfileResult` | 🔶 Simplified 2D resist. |
| `kernel_eigenvalues(focus_nm=0)`, `kernels(focus_nm=0)`, `num_kernels()`, `captured_energy_fraction()`, `imaging_diagnostics(focus_nm=0)` | arrays / dict | Kernel diagnostics: `num_kernels`, `captured_energy_fraction`, `num_frequencies`, `num_columns`, `support_exceeds_nyquist`, `clear_field_intensity`, `normalized`, …. |
| `source_points()`, `spectral_samples()` | arrays / list | `(sx, sy, weight)` source sampling; `(λ, weight)` samples. |
| `wavelength_nm`, `numerical_aperture`, `source`, `optics`, `mask`, `resist`, `grid` | properties | |

```text
compute_from_transmittance(transmittance, focus_nm=0.0)
compute_noon_ideal_image(num_photons=2, fidelity=1.0, focus_nm=0.0, mask=None)
measure_cd(dose_mj_cm2=30, focus_nm=0, threshold=0.3, dose_to_clear_mj_cm2=None, mask=None)
compute_resist_profile(dose_mj_cm2=30, focus_nm=0, dev_time_s=60)
```

### `AerialImageResult`

<!-- huv-table: stack -->

| Member | Meaning |
|---|---|
| `intensity` | (ny, nx) ndarray, read-only zero-copy view, clear-field normalized by default. |
| `x_nm`, `y_nm`, `pixel_nm`, `extent_nm`, `shape` | Pixel-centre coordinates (nm); `extent_nm` = `(x_min, x_max, y_min, y_max)` for `imshow(origin="lower")`. |
| `cross_section(y_nm=0.0)` | `(x_nm, intensity)` along x. |
| `image_contrast()` | (I_max − I_min)/(I_max + I_min). |
| `cd(threshold=0.3, tone="dark")` | Printed width (nm) of the feature nearest the centre. `"dark"` = below threshold (opaque line), `"bright"` = above (space or hole). Sub-pixel, periodic-aware (✅ Implemented). |
| `nils_periodic(…)`, `image_log_slope(…)`, `features(…)` | NILS = w·\|dI/dx\|/I_th; ILS (1/nm); every feature as `(left, right, width, centre)` in nm. `nils(threshold)` is the legacy variant. |

```text
nils_periodic(threshold=0.3, tone="dark", width_nm=None)
```

### `BatchSimulator`

`BatchSimulator(source, optics, mask, grid=None, max_kernels=30, <same imaging keywords as SimulationEngine>)` runs sweeps over one engine: `batch_defocus(focuses)` → `[(focus_nm, intensity ndarray), …]`; `process_window(doses, focuses, cd_threshold=0.3, cd_target_nm=65, cd_tolerance_pct=10)`; `process_window_threshold(doses, focuses, dose_to_clear_mj_cm2, cd_target_nm=65, cd_tolerance_pct=10)`.

CLI: [`simulate`](cli.md#simulate) · TOML: `[imaging]`, `[grid]` in [configuration](configuration.md) · physics: [Simulation pipeline](pipeline.md).

## Process window and ED analysis

`ProcessWindowResult` is 🔶 Simplified. It uses a constant-threshold resist (a point clears where dose · I ≥ E_th): no blur, diffusion or development, rectangular windows only, and the CD taken on the y = 0 cut. Build it with `BatchSimulator.process_window_threshold`, with `api.process_window` (below), or from data: `ProcessWindowResult.from_cd_matrix(doses, focuses, cd_matrix, …)` takes a tabulated focus-exposure matrix; `ProcessWindowResult.from_profiles(x_nm, focuses, profiles, doses, dose_to_clear_mj_cm2, tone="dark", …)` takes intensity cross-sections.

<!-- huv-table: stack -->

| Member | Units / meaning |
|---|---|
| `cd_matrix`, `doses`, `focuses`, `dose_limits`, `tone`, `dose_to_clear_mj_cm2`, `cd_target_nm`, `cd_tolerance_pct` | CD (nm) per (dose, focus), NaN where nothing prints; per-focus in-spec `(dose_min, dose_max)`. |
| `depth_of_focus()`, `exposure_latitude()`, `best_focus()`, `nominal_dose()` | nm, %, nm, mJ/cm². |
| `dose_to_size()`, `dose_to_size_at(focus_nm)`, `dose_limits_at(focus_nm)`, `iso_focal_dose()` | mJ/cm² (or None). |
| `cd_at(dose_mj_cm2, focus_nm)`, `spec_limits_nm()`, `bossung_curves()` | Interpolated CD; spec band; `[(dose, [(focus, cd), …]), …]`. |
| `el_vs_dof(n_points=51)`, `dof_at_el(el_pct=5)`, `el_at_dof(dof_nm)`, `max_area_rectangle()` | ED-window analysis; rectangles are dicts with `focus_min_nm`, `focus_max_nm`, `dose_min_mj_cm2`, `dose_max_mj_cm2`, `dof_nm`, `exposure_latitude_pct`, `focus_centre_nm`, `dose_centre_mj_cm2`. |
| `summary()`, `status`, `notes` | |

<!-- verify-example -->
```python
import numpy as np
import highuvlith as huv

pw = huv.process_window(
    90.0, 180.0, grid_size=64, pixel_nm=3.0,
    doses_mj_cm2=list(np.linspace(24.0, 36.0, 13)),
    focuses_nm=list(np.linspace(-400.0, 400.0, 17)),
)
print(f"E_th {pw.dose_to_clear_mj_cm2:.2f} mJ/cm2, dose-to-size {pw.dose_to_size():.2f} mJ/cm2")
w = pw.dof_at_el(5.0)
print(f"DOF at 5 % EL: {w['dof_nm']:.0f} nm around {w['focus_centre_nm']:.0f} nm")
print(pw.status, len(pw.bossung_curves()))
```

???+ success "Output"

    ```text
    E_th 9.24 mJ/cm2, dose-to-size 30.00 mJ/cm2
    DOF at 5 % EL: 451 nm around 1 nm
    🔶 13
    ```

The same call at full resolution, for the canonical 65/180 nm lines, gives:

<!-- masks-process-window -> python-api.md -->
<figure markdown="span">

![Three panels for 65 nm lines on a 180 nm pitch: Bossung curves of printed CD versus focus for five doses; the focus-dose window where the CD stays within ten percent, with the 5 percent exposure-latitude and maximum-area rectangles; and exposure latitude versus depth of focus.](assets/images/sim/masks-process-window-light.png#gh-light-mode-only)
![Three panels for 65 nm lines on a 180 nm pitch: Bossung curves of printed CD versus focus for five doses; the focus-dose window where the CD stays within ten percent, with the 5 percent exposure-latitude and maximum-area rectangles; and exposure latitude versus depth of focus.](assets/images/sim/masks-process-window-dark.png#gh-dark-mode-only)

<figcaption><strong>The dose-aware process window of 65 nm lines on a 180 nm pitch.</strong> F<sub>2</sub> 157.63 nm, NA 0.75, σ 0.7, 2 % flare, from <code>BatchSimulator.process_window</code>: printed CD per (dose, focus) with a constant-threshold resist, Bossung curves around the dose-to-size, the continuous dose limits, the largest-DOF window at 5 % exposure latitude, the max-area rectangle and the EL–DOF curve. <em>Model: process window / ED analysis 🔶 (constant threshold; no resist blur, diffusion or development; CD on the y = 0 cut).</em></figcaption>
</figure>

CLI: [`sweep`](cli.md#sweep) · GUI: Process window tab ([gui.md](gui.md#views)) · physics: [Metrics and process window](metrics-and-process-window.md).

## High-level functions

Defined in `highuvlith.api` and re-exported at top level. The line/space helpers choose commensurate grids and record the grid actually used in `config` (`pixel_nm`, `pixel_nm_requested`, `field_nm`, `periods_in_field`).

<!-- huv-table: stack -->

| Function | Purpose | Grade | Returns |
|---|---|---|---|
| `simulate_line_space(cd_nm, pitch_nm, **kw)` | one line/space image with its metrics; `cd_nm`/`nils` at a 0.3 threshold | ✅ Implemented (🔶 with `with_resist=True`) | `FullResult` |
| `simulate_contact_hole(diameter_nm, pitch_x_nm, pitch_y_nm=None, **kw)` | the same for a contact array; `cd_nm` is the hole width along x | ✅ Implemented | `FullResult` |
| `sweep_focus(cd_nm=65, pitch_nm=180, **kw)` | contrast through focus | ✅ Implemented | dict: `focuses`, `contrasts`, `best_focus_nm`, `best_contrast` + grid info |
| `compare_illumination(cd_nm, pitch_nm, illuminations=None, **kw)` | conventional, annular, x-dipole and x/y quadrupole side by side | ✅ Implemented | list of `IlluminationComparison` (`name`, `illumination`, `contrast`, `cd_nm`, `nils`, `focuses_nm`, `contrast_through_focus`, `status`) |
| `process_window(cd_nm, pitch_nm, **kw)` | the dose × focus window; without E_th it is set so the line prints at `cd_nm` in focus at 30 mJ/cm² | 🔶 Simplified | `ProcessWindowResult` |
| `source_landscape()`, `wafer_throughput(source, **kw)` | every preset; dose-limited wafers per hour | see [below](python-api-modules.md#sources-landscape-and-throughput) | |
| `simulate_quantum_line_space(cd_nm, pitch_nm, **kw)` | the N-photon / N00N research model | 🧪 Theoretical | `QuantumResult` ([quantum](python-api-modules.md#quantum-n-photon-research-module)) |

<details markdown="1">
<summary>Full signatures of the imaging helpers</summary>

```text
simulate_line_space(cd_nm, pitch_nm, *, wavelength_nm=157.63, na=0.75, sigma=0.7, focus_nm=0,
                    dose_mj_cm2=30, grid_size=256, pixel_nm=1.0, with_resist=False, max_kernels=20,
                    immersion=False, imaging="scalar", polarization="unpolarized",
                    polarization_angle_deg=0, illumination=None, source=None)
    # immersion: True = water (n = 1.437) or an index value; imaging="vector" with
    # polarization in unpolarized, x, y, te, tm, linear; illumination is a with_illumination
    # spec; source overrides wavelength_nm / sigma.
simulate_contact_hole(diameter_nm, pitch_x_nm, pitch_y_nm=None, *, <the same keywords>)
sweep_focus(cd_nm=65, pitch_nm=180, *, <the same keywords>, focus_min=-300, focus_max=300, focus_steps=21)
compare_illumination(cd_nm, pitch_nm, illuminations=None, *, source=None, wavelength_nm=157.63, na=0.75,
                    immersion=False, imaging="scalar", polarization="unpolarized", focus_nm=0,
                    focuses_nm=None, grid_size=128, pixel_nm=2.0, max_kernels=20)
process_window(cd_nm, pitch_nm, *, source=None, wavelength_nm=157.63, na=0.75, sigma=0.7,
                    illumination=None, immersion=False, dose_to_clear_mj_cm2=None, doses_mj_cm2=None,
                    focuses_nm=None, cd_tolerance_pct=10, grid_size=128, pixel_nm=2.0, max_kernels=20)
```

</details>

`FullResult` fields: `aerial`, `contrast`, `config`, `resist_profile`, `cd_nm`, `nils`, `status`, `notes`. Its `status` is `"✅"` for imaging and `"✅/🔶"` when `with_resist=True`.

Water immersion with vector imaging: a 45 nm line at 90 nm pitch through an x-dipole. The two interfering orders meet in water at sin θ = λ/(2pn) = 0.748, so TM fringes keep only about |cos 2θ| = 0.12 of the TE modulation:

<!-- verify-example -->
```python
import highuvlith as huv

common = dict(
    wavelength_nm=193.368, na=1.35, immersion=True,        # water, n = 1.437
    illumination=("dipole", 0.8, 0.1, 0.0), grid_size=64, pixel_nm=1.5,
)
for pol in ("te", "tm"):
    r = huv.simulate_line_space(45.0, 90.0, imaging="vector", polarization=pol, **common)
    print(pol, round(r.contrast, 3))
print("scalar", round(huv.simulate_line_space(45.0, 90.0, **common).contrast, 3))
```

???+ success "Output"

    ```text
    te 0.877
    tm 0.106
    scalar 0.887
    ```

Illumination comparison at k₁ = 90 · 0.75 / 157.63 ≈ 0.43 (contrast in focus and at 200 nm defocus):

<!-- verify-example -->
```python
import highuvlith as huv

for row in huv.compare_illumination(90.0, 180.0, grid_size=64, pixel_nm=3.0):
    print(f"{row.name:24s} contrast {row.contrast:.3f}  "
          f"at ±200 nm {row.contrast_through_focus[0]:.3f}")
```

???+ success "Output"

    ```text
    conventional σ0.7        contrast 0.580  at ±200 nm 0.222
    annular 0.5–0.8          contrast 0.639  at ±200 nm 0.469
    dipole-x 0.7/0.2         contrast 0.887  at ±200 nm 0.570
    quadrupole x/y 0.7/0.2   contrast 0.518  at ±200 nm 0.332
    ```

CLI: [`simulate`](cli.md#simulate) · TOML: `[illumination]` in [configuration](configuration.md) · physics: [Simulation pipeline](pipeline.md), [Vector imaging](vector-imaging.md).

## Optimization and patterning

See [Research modules](./research-modules.md).

<!-- huv-table: stack -->

| Function | Grade | Returns |
|---|---|---|
| `optimize_ilt(source, optics, grid, target, **kw)`; `contact_target(size_nm, pitch_nm, count, grid, *, shape="round")`, `ilt_contact_target(…)` | ✅ Implemented adjoint (🔶 proxy gradient) | `IltResult`: `mask`, `aerial_image`, `binary_mask`, `cost_history`, `termination`, `pattern_error`, `snapshots`, … |
| `fragment_opc(source, optics, mask, grid, threshold, **kw)` | ✅ Implemented | `OpcResult`: `mask`, `polygons`, `final_epe_rms_nm`, `final_epe_max_nm`, `epe_rms_history`, `fragment_*`, … |
| `insert_srafs(source, optics, mask, grid, *, line_cd_nm, sigma_center, threshold, **kw)`; `compare_sraf_dof(…)`; `isolated_line_sraf_study(cd_nm=80, **kw)` → `SrafStudy`; `dose_to_size_threshold(…)` | 🔶 Simplified | `SrafResult` (`assists`, `placed`, `removed`, `worst_margin`); `DofComparison` (`dof_without_nm`, `dof_with_nm`, `gain`, `focus_nm`, `cd_without_nm`, `cd_with_nm`); `SrafStudy` (`threshold`, `sraf`, `dof`, `status`, `notes`) |
| `simulate_lele(source, optics, grid, cd_nm, pitch_nm, **kw)` | 🔶 Simplified | `LeleResult`: `aerial1`, `aerial2`, `aerial2_overlay`, `printed_pattern(threshold_mj_cm2)`, `lines(threshold_mj_cm2)`, … |
| `sadp(mandrel_pitch_nm, mandrel_cd_nm, spacer_nm, *, tone="spacer_is_line")`, `saqp(…)` | 🔶 Simplified, purely geometric | `SpacerPatterningResult`: `lines`, `pitches_nm`, `nominal_pitch_nm`, `pitch_walk_nm`, `cd_range_nm`. |
| `simulate_dsa(template, pixel_nm, l0_nm=28, chi_n=20, **kw)`, `dsa_confined_lamellae(trench_nm, l0_nm)` | 🔶 Simplified, analytic (not SCFT) | `DsaResult`: `pattern`, `assembled_cd_nm`, `heuristic_defect_index` (uncalibrated), `commensurability()`. |

`optimize_ilt` keywords: `cost`, `gradient="adjoint"` or `"proxy"`, `optimizer`, `max_iterations`, `tv_weight`, `binarization_weight`, `conditions`, `illumination`, …; `simulate_lele` keywords: `overlay_x_nm=0`, `overlay_y_nm=0`, `dose1_mj_cm2=30`, `dose2_mj_cm2=30`, …; `simulate_dsa` keywords: `volume_fraction=0.5`, `morphology="lamellar"`, ….

<!-- verify-example -->
```python
import highuvlith as huv

walk = huv.sadp(mandrel_pitch_nm=80.0, mandrel_cd_nm=20.0, spacer_nm=20.0)
print(walk.nominal_pitch_nm, walk.pitch_walk_nm, walk.status)
```

???+ success "Output"

    ```text
    40.0 0.0 🔶
    ```

The pitch walk is 2W + 2t − P = 0 for this choice. The inverse-lithography solver at full resolution:

<!-- optim-ilt-contacts -> python-api.md -->
<figure markdown="span">

![Inverse lithography for a two-by-two contact array: the target, the continuous mask after 0, 10, 20 and 35 iterations growing serifs and assist-like rings, the printed image of the final binary mask with its contour on the target, and cost histories: the adjoint keeps descending, the proxy stalls.](assets/images/sim/optim-ilt-contacts-light.png#gh-light-mode-only)
![Inverse lithography for a two-by-two contact array: the target, the continuous mask after 0, 10, 20 and 35 iterations growing serifs and assist-like rings, the printed image of the final binary mask with its contour on the target, and cost histories: the adjoint keeps descending, the proxy stalls.](assets/images/sim/optim-ilt-contacts-dark.png#gh-dark-mode-only)

<figcaption><strong>Inverse lithography of a contact array.</strong> 100 nm contacts on a 200 nm pitch (F<sub>2</sub> 157.63 nm, NA 0.75, σ 0.6, 8 kernels, 64 × 8 nm grid): sigmoid-resist cost with the exact adjoint gradient through the SOCS kernels, conjugate gradients, binarization penalty switched on after iteration 20 (converged at 35). Bottom right: on 90 nm contacts the legacy local proxy stops after a few iterations while the adjoint drives the cost down ~18×. <em>Model: ILT mask optimization ✅ (constant-threshold sigmoid resist; pixel mask, no MRC or polygon extraction).</em></figcaption>
</figure>

CLI: [`optimize`](cli.md#optimize) · TOML: [`[optimize]`](configuration.md#optimize) · physics: [Research modules](research-modules.md).

## Physics modules

The per-area entry points — sources and throughput, the vector-imaging pupil helpers, LIGA and X-ray materials, the volumetric resist chain, interference / Talbot / EUV-IL, stochastics, the quantum and MNSL research modules and `highuvlith.viz` — are on [Python API modules](python-api-modules.md), each with a verified example and the figure the call produces.

## Type stub and drift guard

Every public name in `highuvlith._native` has an entry in [`_native.pyi`](../python/highuvlith/_native.pyi), and the package ships `py.typed`. [`tests/python/test_stub_drift.py`](../tests/python/test_stub_drift.py) loads the native module and fails if the stub misses a class, function or member. It also fails if the stub has the wrong member kind (property, staticmethod or method) or different parameter names, order or defaults. Any change to the PyO3 bindings must update the stub in the same change.

**Where next:** [CLI](cli.md) — the file-driven equivalent · [TOML configuration](configuration.md) — the schema · [GUI](gui.md) · [Extending highuvlith](extending.md) · [Reference](reference/index.md)
