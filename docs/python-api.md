# Python API Reference

**Status:** ✅ Implemented — PyO3 bindings that release the GIL in every compute path and hand the large result arrays to NumPy as zero-copy read-only views; each capability keeps its own badge from the [capability matrix](./capability-matrix.md).

The Python package `highuvlith` is a thin layer over the compiled Rust engine `highuvlith._native` ([`crates/highuvlith-py`](../crates/highuvlith-py/src)). It has three parts:

- the native configuration, engine and result classes;
- the convenience functions in [`highuvlith.api`](../python/highuvlith/api.py) (also re-exported at top level);
- the plotting package [`highuvlith.viz`](../python/highuvlith/viz).

Everything except `viz` and `interactive` is importable as `import highuvlith as huv`.

Units throughout: lengths in nm unless the name says otherwise (`_um`, `_mm`, `_m`), doses in mJ/cm², power in W, photon energies in eV (or keV where named `_kev`), angles in degrees (`_deg`) or radians (`_rad`). Pupil-fill σ is in units of NA/λ.

## Contents

- [Installation](#installation)
- [Quick start](#quick-start)
- [Runtime behaviour: GIL, arrays, honesty fields](#runtime-behaviour-gil-arrays-honesty-fields)
- [Configuration classes](#configuration-classes)
- [Imaging engine](#imaging-engine)
- [Process window and ED analysis](#process-window-and-ed-analysis)
- [High-level functions](#high-level-functions)
- [Sources: landscape and throughput](#sources-landscape-and-throughput)
- [Vector imaging](#vector-imaging)
- [LIGA, X-ray materials and multilayers](#liga-x-ray-materials-and-multilayers)
- [Volumetric, development and bake](#volumetric-development-and-bake)
- [Interference, Talbot and EUV-IL](#interference-talbot-and-euv-il)
- [Optimization and patterning](#optimization-and-patterning)
- [Stochastics](#stochastics)
- [Quantum (N-photon) research module](#quantum-n-photon-research-module)
- [MNSL](#mnsl)
- [Visualization](#visualization)
- [Type stub and drift guard](#type-stub-and-drift-guard)

---

## Installation

highuvlith is **not published on PyPI**, so build it from source. You need a Rust toolchain and Python ≥ 3.10. The extension is built with [maturin](https://www.maturin.rs/); `pyproject.toml` points it at `crates/highuvlith-py` and installs the module as `highuvlith._native`.

```bash
git clone https://github.com/sonoransun/highuvlith.git
cd highuvlith
python -m venv .venv && source .venv/bin/activate
pip install maturin numpy
maturin develop --release          # optimized build, installed into the venv
```

`maturin develop` without `--release` builds a debug extension: it compiles faster and runs much slower. To install with optional extras from the checkout, use `pip install ".[viz]"`. The extras defined in `pyproject.toml` are:

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

<!-- verify-example -->
```python
import highuvlith as huv

r = huv.simulate_line_space(100.0, 300.0, grid_size=128, pixel_nm=2.0)
print(f"contrast {r.contrast:.3f}, CD {r.cd_nm:.1f} nm, NILS {r.nils:.2f}")
print(r.config["pixel_nm"], r.config["periods_in_field"], r.status)
```

Output:

```text
contrast 0.849, CD 98.2 nm, NILS 2.67
2.34375 1 ✅
```

This is a 100 nm opaque line at 300 nm pitch, imaged with a 157.63 nm source (σ 0.7) at NA 0.75. The requested 2 nm pixel became 2.34375 nm so that the 128-pixel field holds exactly one pitch (see [commensurate grids](#maskconfig)).

The same simulation with the explicit objects:

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

Output:

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

| What | How it crosses into Python |
|---|---|
| `AerialImageResult.intensity`, `VolumetricResult.values`, `HeightMapResult.values`, the `TalbotResult` images and carpet, the `LeleResult` images, `DsaResult.pattern` | **Zero-copy, read-only** NumPy view of the Rust buffer (the result classes are frozen, so the buffer never changes). The view keeps its owner alive. Call `.copy()` if you need a writable array. |
| Arrays returned by a function call (`n_photon_absorption_image`, `blazed_grating`, `pupil_map`, `rasterize`, …) | Moved into NumPy with no copy (`into_pyarray`); you own them. |
| Small accessors (coordinate axes `x_nm`/`y_nm`/`z_nm`, histories, depth-dose curves, lists of tuples) | Fresh copies. |

**Honesty fields.** Results of simplified or theoretical models carry `.status` and `.notes`:

- `.status` is the capability badge, `"✅"`, `"🔶"`, `"🧪"` or split such as `"✅/🔶"`;
- `.notes` is a list of the stated approximations.

Classes that have them: `ResistProfileResult`, `ProcessWindowResult`, `LigaResult`, `LigaEdgeProfile`, `LevelSetResult`, `CarPebResult`, `TalbotResult`, `EuvIlResult`, `IltResult`, `OpcResult`, `SrafResult`, `DofComparison`, `SpacerPatterningResult`, `LeleResult`, `DsaResult`, `LerResult`, `MnslResult`, `TwoBeamNPhoton`, `NoonImageResult`, plus the `api` dataclasses `FullResult`, `QuantumResult` and `IlluminationComparison`.

**Validation.** Bad parameters raise `ValueError` before any compute starts. Inputs are checked three times: in the Python wrappers, in the PyO3 shims and in the Rust core.

**Commensurate grids.** If a periodic mask's field does not hold a whole number of periods, constructing an engine emits a `UserWarning`. The FFT's periodic boundary would otherwise insert a defect at the field edge. The `api` functions always choose a commensurate grid.

---

## Configuration classes

### `SourceConfig`

One class wraps all 14 source families (the core `SourceKind` variants; DUV/UV heritage presets are VUV-family `"vuv"` sources, listed separately below). The default constructor `SourceConfig(wavelength_nm=157.63, sigma_outer=0.7, bandwidth_pm=1.1, spectral_samples=5)` builds a generic excimer-like source. The static factories build presets; each family keeps its badge from the [capability matrix](./capability-matrix.md) and has its own page under [docs/sources](./sources/index.md).

| Family (badge) | Factories (defaults) |
|---|---|
| VUV excimer ✅ (Ar₂ 🧪) | `f2_laser(sigma=0.7)` 157.63 nm; `ar2_laser(sigma=0.7)` 126 nm (hypothetical) |
| DUV/UV heritage ✅ (🔶 parameters) | `arf_laser()` 193.368 nm; `krf_laser()` 248.3 nm; `hg_i_line()` / `hg_h_line()` / `hg_g_line()` (CW lamps, `sigma=0.7`) |
| LPA-FEL 🔶 | `lpa_fel_bella_25nm(sigma=0.7)` (design projection); `lpa_fel(wavelength_nm, sigma=0.7, electron_energy_mev=500, bandwidth_pm=None, pulse_duration_fs=10, rep_rate_hz=1000, pulse_energy_uj=None, undulator_period_mm=None, undulator_k=None, num_periods=200, peak_current_a=None, norm_emittance_um=None, energy_spread_rel=None, beta_m=None)` |
| Laser-produced plasma ✅ (🔶 CE/étendue; Gd/Tb power 🧪) | `lpp_sn_13nm5(sigma=0.9, drive_laser="co2")` (250 W at IF; `"solid_state_1um"`, `"thulium_2um"`); `lpp_sn_13nm5_500w()`; `lpp_gd_6nm7()`; `lpp_tb_6nm5()` |
| Discharge plasma 🔶 | `dpp_sn_13nm5(sigma=0.9)`; `dpp_xe_13nm5(sigma=0.9)` |
| Synchrotron ✅ (🔶 coherence) | `synchrotron_undulator(electron_energy_gev=0.538, period_mm=20, k=1, num_periods=100, harmonic=1, ring_current_ma=200, emittance_x_nm_rad=None, emittance_y_nm_rad=None, energy_spread_rel=5e-4)`; `synchrotron_compact_euv()`; `synchrotron_liga_bending_magnet()` (2.5 GeV, 1.5 T) |
| HHG ✅/🔶 | `hhg(driver_wavelength_nm=800, gas="neon", driver_intensity_w_cm2=4e14, harmonic=59, monochromator_bandwidth_pm=15, full_comb=False, driver_average_power_w=None, conversion_efficiency=None, comb_passband_nm=None)`; `hhg_ne_13nm5()`; `hhg_ar_30nm()` |
| XFEL ✅ (🧪 CW-SC/ERL) | `xfel_flash_13nm5()`; `xfel_fermi_seeded()`; `xfel_cw_sc_13nm5()`; `xfel_erl_13nm5()` |
| Soft-X-ray laser ✅/🔶 | `sxrl_ar_46nm9()`; `sxrl_ag_13nm9()`; `sxrl(scheme="ag_13nm9", pulse_energy_uj=None, rep_rate_hz=None, pulse_duration_ps=None, rel_linewidth=None)` (`ar_46nm9`, `ag_13nm9`, `cd_13nm2`, `mo_18nm9`) |
| X-ray tube 🔶/✅ | `xray_tube(anode="W", kvp=None, current_ma=None, be_window_um=250)` (`W`, `Mo`, `Cu`, `Rh`; for LIGA/proximity, not projection imaging) |
| Inverse Compton 🧪 | `ics(target_wavelength_nm=13.5, laser_wavelength_nm=1030, laser_a0=0.1, …)`; `ics_compact_euv_13nm5()` |
| SSMB 🧪 | `ssmb(ring_energy_mev=400, modulation_wavelength_nm=1053, target_wavelength_nm=13.5, average_power_w=1000, …)`; `ssmb_euv_13nm5()` |
| Betatron 🧪 | `betatron(electron_energy_mev=200, plasma_density_cm3=1e19, betatron_amplitude_um=1, interaction_length_mm=3, bunch_charge_pc=50, rep_rate_hz=10)` |
| Smith–Purcell 🧪 | `smith_purcell(target_wavelength_nm=13.5, electron_energy_kev=30, diffraction_order=1, observation_angle_deg=90, grating_period_nm=None, num_periods=100, beam_current_na=10, coupling_efficiency=1e-3, impact_height_nm=0)` |
| Entangled photons 🧪 | `entangled_noon(wavelength_nm=157.63, n=2, fidelity=1.0, pair_rate_hz=1e6)` |

Properties and methods:

| Member | Returns | Meaning |
|---|---|---|
| `wavelength_nm`, `bandwidth_pm` | float | Centre wavelength (nm), FWHM bandwidth (pm). For synchrotron/HHG/ICS these are derived; for tubes/betatron `wavelength_nm` is hc/⟨E⟩. |
| `sigma_outer`, `spectral_samples`, `kind` | float, int, str | Pupil σ, polychromatic sample count, family tag (`"vuv"`, `"lpp"`, `"xray_tube"`, …). |
| `photon_energy_ev`, `photon_density_per_mj_cm2` | float | Photon energy (eV); incident photons per nm² at 1 mJ/cm². |
| `average_power_w`, `pulse_energy_j`, `rep_rate_hz`, `pulse_duration_fs`, `electron_energy_mev` | float or None | Each family's own definition: plasma sources in band at IF, lasers at the output, tubes as filtered 4π X-ray power. `None` means not modeled. |
| `shot_to_shot_rms`, `transverse_coherence_fraction` | float / float or None | Relative pulse-energy jitter (drives stochastic dose jitter); coherent fraction. |
| `derived_quantities()` | list of `(name, value, unit, note)` | Physics derived from machine parameters (gain lengths, flux, coherent fraction, HVM gaps, …). |
| `derived_quantity(name)` | float or None | One of the above by name. |
| `wafer_throughput(dose_mj_cm2=30, …, preset="euv")` | dict or None | Dose-limited wafers/hour (🔶); see [throughput](#sources-landscape-and-throughput). |
| `illumination` | tuple | Pupil fill: `("conventional", σ)`, `("annular", σ_in, σ_out)`, `("dipole", σ_c, σ_r, orientation_deg)`, `("quadrupole", σ_c, σ_r, opening_angle_deg)`, `("coherent_gaussian", σ)`. |
| `with_illumination(shape, *params)` | SourceConfig | Copy with the pupil fill replaced; spectrum and power are unchanged. |
| `pupil_fill(n=101, extent=1.0)` | ndarray (n, n) | Relative source intensity over σ ∈ [−extent, extent]², indexed `[iy, ix]`. |
| `spectrum()` | list of `(wavelength_nm, weight)` | The spectral samples the engine uses. |
| `xray_spectrum(n_bins=200)` | list of `(E_keV, fraction)` or None | Relative spectrum (X-ray tube, betatron). |
| `spectral_flux_density(distance_mm, n_bins=200)` | list of `(E_keV, photons s⁻¹ mm⁻² keV⁻¹)` or None | Absolute flux density at a distance (tube, betatron); feed it to `simulate_liga(flux_density=…)`. |

### `OpticsConfig`

| Constructor / preset | Badge | Notes |
|---|---|---|
| `OpticsConfig(numerical_aperture=0.75, reduction=4.0, flare_fraction=0.02)` | ✅ | Dry refractive projection optics, NA < 1. |
| `OpticsConfig.immersion(numerical_aperture=1.35, immersion_index=1.437, reduction=4.0, flare_fraction=0.02)` / `immersion_193i()` | ✅ | NA up to 0.95·n; defocus is applied in the medium. |
| `OpticsConfig.euv_projection(numerical_aperture=0.33, central_obscuration=0.0, reduction=4.0, flare=0.0)` / `euv_nxe()` / `euv_high_na()` | 🔶 | Isotropic wafer-side pupil. High-NA uses NA 0.55 with an **assumed** 0.2·NA obscuration; no anamorphic magnification and no mask 3D. |
| `OpticsConfig.schwarzschild(numerical_aperture=0.33, obscuration_ratio=0.25, reduction=4.0, mirror_reflectivity=0.67, flare=0.03)` | ✅ | Annular reflective objective. |
| `OpticsConfig.zone_plate(outer_zone_width_nm, design_wavelength_nm)` | ✅ | First-order Fresnel zone plate. |
| `.with_multilayer_pupil(mirrors, coating="mo_si", periods=40, period_nm=6.9, gamma=0.4)` | 🔶 | Angle-dependent multilayer amplitude/phase across the pupil from user-supplied incidence-angle maps, `mirrors = [(center_deg, tilt_deg, azimuth_deg, radial_deg), …]`. |

Other members:

- `add_aberration(fringe_index, coefficient_waves)` adds a Fringe Zernike term.
- `with_paraxial_defocus(enabled=True)` switches to the legacy paraxial defocus phase.
- `pupil_map(wavelength_nm, n=65, focus_nm=0.0)` returns the complex pupil, an ndarray of shape (n, n).
- `rayleigh_resolution(wavelength_nm)` returns nm.
- Properties: `numerical_aperture`, `reduction`, `flare_fraction`, `immersion_index`, `central_obscuration`, `paraxial_defocus`, `has_multilayer_pupil`, `kind`.

### `MaskConfig`

Thin (Kirchhoff) mask model, 🔶: binary and attenuated PSM are modeled, and there is no mask 3D. Spectra are exact closed-form Fourier series wherever possible (✅). See [Masks and metrics](./masks-and-metrics.md).

| Member | Meaning |
|---|---|
| `MaskConfig.line_space(cd_nm, pitch_nm, orientation="vertical", offset_nm=0.0)` | Bright field, opaque lines of width `cd_nm`, one centred at x = 0, periodic over the field. |
| `MaskConfig.contact_hole(diameter_nm, pitch_x_nm, pitch_y_nm=None)` | Dark field, clear square holes, periodic. |
| `MaskConfig.from_features(features, dark_field=False, mask_type="binary", transmission=0.06, phase_deg=180.0)` | **Canonical** custom mask. Features are painted in order, later ones on top. Each item is either a feature dict (`rect`, `gray_rect`, `polygon`, `line_space`, `rect_array`) or a shorthand: an `(x, y, w, h)` rectangle (centre and size, nm) or a vertex list (polygon). |
| `mask_from_features(rects=None, polygons=None, …)` | **Deprecated** alias that emits a `DeprecationWarning`; use `MaskConfig.from_features`. |
| `features()` | The feature dicts (the inverse of `from_features`). |
| `periodicity()` → `(px, py or None)` or None; `check_commensurate(grid)`; `commensurate_grid(size=256, target_pixel_nm=1.0)` | Periodicity of the mask and grids that respect it. |
| `rasterize(grid)` (complex128), `rasterize_intensity(grid)` (float64), `spectrum(grid)` (`numpy.fft.fft2` layout and scale), `spectrum_method(grid)` (`"analytic"`, `"mixed"` or `"raster"`), `validate()` | Sampling, spectrum and validation. |

### `GridConfig`

`GridConfig(size=512, pixel_nm=1.0)` is a square field of `size` pixels. Other members:

- `GridConfig.commensurate(pitch_x_nm, pitch_y_nm=None, size=256, target_pixel_nm=1.0)` returns a grid whose field is a whole multiple of the pitch(es). The pixel is the largest one not coarser than the target, unless one period does not fit in `size` pixels.
- `field_size_nm()`, `periods_in_field(pitch_nm)` and `is_commensurate_with(pitch_nm)` inspect the field.

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

Output:

```text
analytic None
1.40625 180.0 True
```

### Resist, film stack, process

- **`ResistConfig(thickness_nm=150, dill_a=0.2, dill_b=0.45, dill_c=0.02, peb_diffusion_nm=30, model="mack")`** and `ResistConfig.vuv_fluoropolymer()` configure the resist. The 2D resist path is 🔶: depth-averaged Dill, with development along the centre row only. Dill A and B are in 1/µm, C in cm²/mJ.
- **`FilmStackConfig(layers=None, substrate_n=None, substrate_k=None, superstrate_n=1.0)`** builds the thin-film transfer matrix (✅). With no arguments it is 150 nm of resist on Si. Layers are `(name, thickness_nm, n, k)`. Methods: `add_layer`, `set_substrate`, `set_superstrate`, `reflectance(λ, angle_deg=0, polarization="unpolarized")`, `transmittance(…)`, `amplitude_coefficients(λ, angle_deg=0, polarization="te")` → `(r, t)`, and `intensity_profile(λ, z_nm, …)` → `|E(z)|²`.
- **`ProcessConfig(dose_mj_cm2=30, focus_nm=0, development_time_s=60)`** holds the process parameters.

---

## Imaging engine

### `SimulationEngine`

```text
SimulationEngine(source, optics, mask, resist=None, grid=None, max_kernels=30,
                 defocus_model="exact", kernel_energy_fraction=1.0,
                 source_points_per_axis=None, vector=None,
                 normalization="clear_field", kernel_cache_capacity=32)
```

The engine builds the factorized Hopkins TCC and its SOCS kernels once, then evaluates many masks and focus planes. Kernel sets are cached per (focus, λ). Scalar Hopkins imaging and exact non-paraxial defocus are ✅. Keyword arguments:

| Keyword | Meaning |
|---|---|
| `max_kernels` | Upper bound on the number of SOCS kernels. |
| `kernel_energy_fraction` | Stop once this fraction of the TCC trace is captured (≤ 1). |
| `source_points_per_axis` | `None` = adaptive source sampling (≥ 24 cells across the fill), or a fixed count. |
| `defocus_model` | `"exact"` (defocus inside the pupil at every source point, the default) or `"kernel_phase"` (legacy fast approximation). |
| `vector` | `None` for scalar imaging, or a [`VectorSettings`](#vector-imaging) for polarized imaging (✅). |
| `normalization` | `"clear_field"` (default: divide by the exact clear-field intensity TCC(0,0) of the same illumination, optics and focus) or `"absolute"`. |
| `kernel_cache_capacity` | Number of cached kernel sets. |

| Method | Returns | Notes |
|---|---|---|
| `compute_aerial_image(focus_nm=0.0, mask=None)` | `AerialImageResult` | Monochromatic image. `mask` images another mask with the same kernels. |
| `compute_through_focus(focus_nm: list, mask=None)` | list of `AerialImageResult` | One mask spectrum, planes computed in parallel. |
| `compute_multiwavelength(focus_nm=0.0, mask=None)` | `AerialImageResult` | ✅ exact: the TCC is rebuilt at every spectral sample. |
| `compute_polychromatic(focus_nm=0.0, mask=None)` | `AerialImageResult` | 🔶 narrow band: centre-λ kernels with a chromatic focus shift; valid for Δλ/λ ≪ 1. |
| `compute_from_transmittance(transmittance, focus_nm=0.0)` | `AerialImageResult` | Complex `(n, n)` array on the engine grid, e.g. an ILT mask. |
| `compute_noon_ideal_image(num_photons=2, fidelity=1.0, focus_nm=0.0, mask=None)` | `NoonImageResult` | 🧪 Ideal N00N limit; see [quantum](#quantum-n-photon-research-module). |
| `clear_field_intensity(focus_nm=0.0)` | float | Absolute TCC(0,0); multiply a normalized image by it to recover absolute intensity. |
| `measure_cd(dose_mj_cm2=30, focus_nm=0, threshold=0.3, dose_to_clear_mj_cm2=None, mask=None)` | float (nm) | With `dose_to_clear_mj_cm2`, the threshold is E_th / dose. |
| `image_contrast(focus_nm=0, mask=None)` | float | (I_max − I_min)/(I_max + I_min). |
| `compute_resist_profile(dose_mj_cm2=30, focus_nm=0, dev_time_s=60)` | `ResistProfileResult` | 🔶 2D resist. |
| `kernel_eigenvalues(focus_nm=0)`, `kernels(focus_nm=0)`, `num_kernels()`, `captured_energy_fraction()`, `imaging_diagnostics(focus_nm=0)` | arrays / dict | Kernel diagnostics: `num_kernels`, `captured_energy_fraction`, `num_frequencies`, `num_columns`, `support_exceeds_nyquist`, `clear_field_intensity`, `normalized`, …. |
| `source_points()`, `spectral_samples()` | arrays / list | `(sx, sy, weight)` source sampling; `(λ, weight)` samples. |
| `wavelength_nm`, `numerical_aperture`, `source`, `optics`, `mask`, `resist`, `grid` | properties | |

### `AerialImageResult`

| Member | Meaning |
|---|---|
| `intensity` | (ny, nx) ndarray, read-only zero-copy view, clear-field normalized by default. |
| `x_nm`, `y_nm`, `pixel_nm`, `extent_nm`, `shape` | Pixel-centre coordinates (nm); `extent_nm` = `(x_min, x_max, y_min, y_max)` for `imshow(origin="lower")`. |
| `cross_section(y_nm=0.0)` | `(x_nm, intensity)` along x. |
| `image_contrast()` | (I_max − I_min)/(I_max + I_min). |
| `cd(threshold=0.3, tone="dark")` | Printed width (nm) of the feature nearest the centre. `"dark"` = below threshold (opaque line), `"bright"` = above (space or hole). Sub-pixel, periodic-aware (✅). |
| `nils_periodic(threshold=0.3, tone="dark", width_nm=None)`, `image_log_slope(…)`, `features(…)` | NILS = w·\|dI/dx\|/I_th; ILS (1/nm); every feature as `(left, right, width, centre)` in nm. `nils(threshold)` is the legacy variant. |

### `BatchSimulator`

`BatchSimulator(source, optics, mask, grid=None, max_kernels=30, <same imaging keywords as SimulationEngine>)` runs sweeps over one engine:

- `batch_defocus(focuses)` → `[(focus_nm, intensity ndarray), …]`;
- `process_window(doses, focuses, cd_threshold=0.3, cd_target_nm=65, cd_tolerance_pct=10)`;
- `process_window_threshold(doses, focuses, dose_to_clear_mj_cm2, cd_target_nm=65, cd_tolerance_pct=10)`.

## Process window and ED analysis

`ProcessWindowResult` is 🔶. It uses a constant-threshold resist (a point clears where dose · I ≥ E_th): no blur, diffusion or development, rectangular windows only, and the CD taken on the y = 0 cut. Build it with `BatchSimulator.process_window_threshold`, with `api.process_window` (below), or from data:

- `ProcessWindowResult.from_cd_matrix(doses, focuses, cd_matrix, …)` takes a tabulated focus-exposure matrix;
- `ProcessWindowResult.from_profiles(x_nm, focuses, profiles, doses, dose_to_clear_mj_cm2, tone="dark", …)` takes intensity cross-sections.

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

Output:

```text
E_th 9.24 mJ/cm2, dose-to-size 30.00 mJ/cm2
DOF at 5 % EL: 451 nm around 1 nm
🔶 13
```

---

## High-level functions

These are defined in `highuvlith.api` and re-exported at top level. The line/space helpers choose commensurate grids and record the grid actually used in `config` (`pixel_nm`, `pixel_nm_requested`, `field_nm`, `periods_in_field`).

| Function | Returns | Notes |
|---|---|---|
| `simulate_line_space(cd_nm, pitch_nm, *, wavelength_nm=157.63, na=0.75, sigma=0.7, focus_nm=0, dose_mj_cm2=30, grid_size=256, pixel_nm=1.0, with_resist=False, max_kernels=20, immersion=False, imaging="scalar", polarization="unpolarized", polarization_angle_deg=0, illumination=None, source=None)` | `FullResult` | `immersion`: `True` = water (n = 1.437) or an index value. `imaging="vector"` with `polarization` ∈ `unpolarized`, `x`, `y`, `te`, `tm`, `linear`. `illumination` is a `with_illumination` spec. `source` overrides `wavelength_nm`/`sigma`. `cd_nm`/`nils` are measured at a 0.3 intensity threshold. |
| `simulate_contact_hole(diameter_nm, pitch_x_nm, pitch_y_nm=None, *, …)` | `FullResult` | `cd_nm` is the hole width along x. |
| `sweep_focus(cd_nm=65, pitch_nm=180, *, …, focus_min=-300, focus_max=300, focus_steps=21)` | dict | `focuses`, `contrasts`, `best_focus_nm`, `best_contrast` + grid info. |
| `compare_illumination(cd_nm, pitch_nm, illuminations=None, *, source=None, wavelength_nm=157.63, na=0.75, immersion=False, imaging="scalar", polarization="unpolarized", focus_nm=0, focuses_nm=None, grid_size=128, pixel_nm=2.0, max_kernels=20)` | list of `IlluminationComparison` | `name`, `illumination`, `contrast`, `cd_nm`, `nils`, `focuses_nm`, `contrast_through_focus`, `status`. The default set is conventional, annular, x-dipole and x/y quadrupole. |
| `process_window(cd_nm, pitch_nm, *, source=None, wavelength_nm=157.63, na=0.75, sigma=0.7, illumination=None, immersion=False, dose_to_clear_mj_cm2=None, doses_mj_cm2=None, focuses_nm=None, cd_tolerance_pct=10, grid_size=128, pixel_nm=2.0, max_kernels=20)` | `ProcessWindowResult` (🔶) | Without E_th, it is set so the line prints at `cd_nm` in focus at 30 mJ/cm². |
| `source_landscape()`, `wafer_throughput(source, *, dose_mj_cm2=30, preset="auto", **overrides)` | see [below](#sources-landscape-and-throughput) | |
| `simulate_quantum_line_space(cd_nm, pitch_nm, *, n=2, fidelity=1.0, model="n_photon_absorption", …)` | `QuantumResult` (🧪) | See [quantum](#quantum-n-photon-research-module). |

`FullResult` fields: `aerial`, `contrast`, `config`, `resist_profile`, `cd_nm`, `nils`, `status`, `notes`. Its `status` is ✅ for imaging and `"✅/🔶"` when `with_resist=True`.

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

Output:

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

Output:

```text
conventional σ0.7        contrast 0.580  at ±200 nm 0.222
annular 0.5–0.8          contrast 0.639  at ±200 nm 0.469
dipole-x 0.7/0.2         contrast 0.887  at ±200 nm 0.570
quadrupole x/y 0.7/0.2   contrast 0.518  at ±200 nm 0.332
```

## Sources: landscape and throughput

`source_landscape()` returns a list of `SourcePreset` records, one per preset. Fields:

- `label`, `factory` (e.g. `"lpp_sn_13nm5()"`), `family`;
- `wavelength_nm`, `bandwidth_pm`, `photon_energy_ev`, `average_power_w` (W or None), `power_definition`;
- `maturity` (`"demonstrated"`, `"projection"` or `"theoretical"`, describing the machine) and `status` (the model badge);
- `broadband`, `note`, and `derived` (`{name: (value, unit)}`).

Numbers come live from the factories. `maturity` and `status` are curated labels taken from the capability matrix. Powers follow each family's own definition, so they are **not** all measured at the same plane. `viz.plot_source_landscape()` plots this list.

`wafer_throughput(source, *, dose_mj_cm2=30, preset="auto", **overrides)` (🔶) wraps `SourceConfig.wafer_throughput`. With `preset="auto"` the optics-train assumptions follow the wavelength:

- `"refractive"` above 100 nm: optics 0.30, mask 0.90, 8 mm slit;
- `"euv"` from 12.4 to 15 nm: 10 Mo/Si mirrors at R 0.70, mask 0.65;
- `"beuv"` from 6 to 7.5 nm: La/B mirrors.

Any other wavelength has no projection-lithography optics preset, so `"auto"` raises `ValueError` rather than quote an EUV-optics figure, unless you give the optics train (`optics_transmission`, or `n_mirrors` + `mirror_reflectivity`); the nearest preset then supplies only the mask and field parameters, and a note says so. This covers the X-ray tubes and the betatron (0.15–0.5 nm LIGA/proximity sources) and the 25–47 nm LPA-FEL, Ar HHG and Ar soft-X-ray-laser lines. An explicit `preset=` is honoured at any wavelength as a hypothetical train, and outside the preset's band a `"hypothetical: …"` entry is added to `notes`.

It returns the native dict (`wafers_per_hour`, `power_at_wafer_w`, `pulses_per_point`, `scan_speed_mm_s`, `stage_limited`, …) plus `status` and `notes`, or `None` when the source models no power (Hg lamps, the entangled-photon source). The CLI `throughput` subcommand applies the same bands. The native `SourceConfig.wafer_throughput` has no wavelength check; it applies whatever `preset` it is given (default `"euv"`).

<!-- verify-example -->
```python
import highuvlith as huv

lpp = huv.SourceConfig.lpp_sn_13nm5()
print(lpp.kind, lpp.wavelength_nm, round(lpp.average_power_w, 1), "W at IF")
for name, value, unit, note in lpp.derived_quantities()[:3]:
    print(f"  {name} = {value:.4g} {unit}")
wph = huv.wafer_throughput(lpp, dose_mj_cm2=30.0)
print(round(wph["wafers_per_hour"], 1), wph["preset"], wph["status"])

rows = huv.source_landscape()
print(len(rows), rows[0].label, rows[0].maturity, rows[0].status)
```

Output:

```text
lpp 13.5 250.0 W at IF
  photon_energy = 91.84 eV
  in_band_emission_2pi = 1290 W
  drive_laser_wavelength = 10.6 um
153.9 euv 🔶
30 Hg g-line demonstrated ✅/🔶
```

## Vector imaging

Vector imaging is ✅; see [Vector imaging](./vector-imaging.md).

**`VectorSettings`**

```text
VectorSettings(polarization="unpolarized", angle_deg=0.0, image_index=1.0,
               obliquity=True, reduction=4.0, film_n=None, film_k=0.0)
```

- `polarization` is one of `"unpolarized"`, `"x"`, `"y"`, `"te"` (azimuthal), `"tm"` (radial) or `"linear"` (at `angle_deg`).
- Pass the settings to `SimulationEngine(…, vector=…)`. The engine replaces `reduction` with the optics' value, and `image_index` = 1 inherits the immersion index.
- Methods: `validate(na)` and `columns_per_source_point()`.

Pupil-physics helpers:

| Function | Returns |
|---|---|
| `pupil_polarization_map(na, polarization="x", *, n=65, sx=0, sy=0, angle_deg=0, image_index=1, obliquity=True, reduction=4, film_n=None, film_k=0)` | `PupilPolarizationMap` (`px`, `py`, `field` complex `(n_states, 3, n, n)`, `settings`, `na`) |
| `vector_two_beam_contrast(na, polarization="x", *, p0=1.0, sx=0.5, sy=0, …)` | float, the fringe contrast of two orders at pupil `(±p0, 0)` |
| `vector_pupil_field(settings, na, px, py, sx=0, sy=0)` / `vector_pupil_map(settings, na, n=65, sx=0, sy=0)` | complex ndarray |
| `radiometric_factor(pupil_na, image_index=1, reduction=4)` | (cos θ_obj / cos θ_img)^½ |
| `fresnel_coefficients(n1, n2_real, n2_imag, sin_theta1)` | dict of r_s, r_p, t_s, t_p, R/T |

## LIGA, X-ray materials and multilayers

See [LIGA / deep X-ray](./processes/liga-deep-xray.md) and [Materials](./materials.md).

| Function / class | Badge | Notes |
|---|---|---|
| `simulate_liga(critical_energy_kev=None, resist_thickness_um=500, *, cd_nm=5000, pitch_nm=10000, grid_size=128, pixel_nm=200, nz=64, diffraction="fresnel", proximity_gap_um=100, energy_bins=100, photoelectron_blur=False, filters=None, absorber="Au", absorber_thickness_um=20, membrane="Ti", membrane_thickness_um=2, target_bottom_dose_kj_cm3=3, damage_dose_kj_cm3=20, source=None, source_distance_m=None, horizontal_acceptance_mrad=5, vertical_scan_mm=None, flux_density=None, spectrum_table=None, strict_sampling=False, warn=True)` → `LigaResult` | ✅ depth dose and exposure time; ✅/🔶 lateral | Absolute exposure needs either a bending-magnet `source` with `source_distance_m` and `vertical_scan_mm`, or an absolute `flux_density` table. `warn=True` turns sampling and beamline warnings into `UserWarning`s. |
| `LigaResult` | | `depth_dose` → `(z_um, dose_kj_cm3)`, `dose_ratio`, `top_dose_kj_cm3`, `bottom_dose_kj_cm3`, `exceeds_damage_ceiling`, `exposure_time_s`, `top_dose_rate_kj_cm3_s`, `bottom_dose_rate_kj_cm3_s`, `resist_power_density_w_mm2`, `exposure_charge_ma_h`, `mean_energy_top_kev`, `mean_energy_bottom_kev`, `window_power_fraction`, `diffraction`, `spectrum`, `fresnel_scale_nm`, `sampling_resolved`, `warnings`, `volume`, `developed_depth`, `status`, `notes` |
| `liga_edge_profile(critical_energy_kev=None, resist_thickness_um=500, *, proximity_gap_um=100, x_min_nm=-3000, x_max_nm=3000, dx_nm=10, depths_um=None, …)` → `LigaEdgeProfile` | ✅/🔶 | Analytic Fresnel knife edge: `x_nm`, `z_um`, `dose_kj_cm3`, `edge_positions_nm(threshold)`, `sidewall_angle_deg(threshold)`, `max_gradient_kj_cm3_nm()` |
| `MultilayerMirror.mo_si(periods=40, period_nm=6.9, gamma=0.4)` / `.la_b4c(…)` / `.la_b(…)` | ✅ | Parratt recursion with Névot–Croce roughness. `with_capping`, `with_roughness`, `reflectance`, `reflectance_curve`, `reflectance_vs_angle`, `amplitude`, `peak(λmin, λmax)` → `(λ, R)`, `bandwidth_fwhm_nm`, `angular_acceptance_fwhm_deg`, `pupil_response` |
| `tune_multilayer_period(family, target_nm, periods, gamma=0.4, angle_deg=0, polarization="te")` | ✅ | Period (nm) that puts the peak at `target_nm`. |
| `xray_optical_constants(formula, wavelengths_nm, density_g_cm3=None)` → `(delta, beta)` | ✅ | Henke/CXRO, n = 1 − δ + iβ. |
| `refractive_index(material, wavelength_nm)` → complex, `material_names()` | 🔶 | Database entries. Every entry has a data range and lookups never extrapolate: the VUV entries (e.g. `"Si"`, `"CaF2"`) cover the VUV only; `"Si"`, `"Cr"`, `"SiO2"` fall back to their Henke values at 0.0413–41.3 nm, everything else out of range raises `ValueError` naming the range and the alternative (`_euv`/`_beuv` entries or `"henke:<formula>[@density]"`). See [materials](materials.md#range-policy-no-silent-extrapolation). |

<!-- verify-example -->
```python
import highuvlith as huv

liga = huv.simulate_liga(
    source=huv.SourceConfig.synchrotron_liga_bending_magnet(), source_distance_m=15.0, vertical_scan_mm=50.0,
    grid_size=16, nz=8, energy_bins=20, diffraction="gaussian", warn=False,
)
print(f"top/bottom {liga.dose_ratio:.2f}, exposure {liga.exposure_time_s:.0f} s, {liga.status}")

tube = huv.SourceConfig.xray_tube(anode="Cu")
flux = tube.spectral_flux_density(distance_mm=100.0, n_bins=100)
print(len(flux), round(tube.wavelength_nm, 4))

mirror = huv.MultilayerMirror.mo_si()
lam, r = mirror.peak(13.0, 14.0)
print(f"Mo/Si peak {r:.3f} at {lam:.2f} nm")
n = huv.refractive_index("henke:Si", 13.5)   # Henke/CXRO at 13.5 nm
print(f"Si at 13.5 nm: n = {n.real:.4f} + {n.imag:.5f}i")
```

Output:

```text
top/bottom 21.41, exposure 631 s, ✅/🔶
100 0.1675
Mo/Si peak 0.730 at 13.48 nm
Si at 13.5 nm: n = 0.9990 + 0.00183i
```

The example uses 20 energy bins and a 16-pixel grid so that it runs fast. With the default 100 bins, the [LIGA page](./processes/liga-deep-xray.md) documents a top/bottom ratio of 20.8 and an exposure of 605 s for this geometry.

## Volumetric, development and bake

See [Volumetric exposure](./processes/volumetric-exposure.md) and [Resist models](./processes/resist-models.md).

| Function | Badge | Returns / notes |
|---|---|---|
| `expose_volumetric(source, optics, mask, film_stack, resist, grid, dose_mj_cm2=30, nz=64, n_defocus_planes=8, dose_steps=1, base_defocus_nm=0, resist_layer=0, max_kernels=20)` | 🔶 | `VolumetricResult`, the PAC latent image (nz, ny, nx). |
| `simulate_volumetric(source, optics, mask, film_stack, resist, grid, *, …, peb="gaussian"\|"none"\|"car", car=None, develop="fmm"\|"level_set", dev_time_s=60, …)` | 🔶 (chain) | `VolumetricProcessResult` with `latent`, `baked`, `arrival_times`, `height_map`, `developed_cd_nm`, `dev_time_s`, `car`, `level_set`, and `notes` (over- or under-development, the same checks as the CLI; the default 30 mJ/cm² over-develops the default stack, see [Configuration → choosing the dose](configuration.md#mode-volumetric)). |
| `develop_fast_marching(volume, resist, pixel_xy_nm, pixel_z_nm, surface_rate_ratio=1, inhibition_depth_nm=0, lateral="reflecting")` | ✅ | Arrival times (s). |
| `develop_level_set(volume, resist, dev_time_s, *, surface_rate_ratio=1, inhibition_depth_nm=0, lateral="periodic", depletion="none", …)` | ✅/🔶 | `LevelSetResult`: `arrival_times`, `phi`, `steps`, `reinitializations`, `dissolved_thickness_nm`, `final_rate_factor`, `height_map()`, `developed(time_s=None)`. |
| `development_rate(volume, resist, surface_rate_ratio=1, inhibition_depth_nm=0)` | | Rate volume (nm/s). |
| `peb_gaussian(volume, lateral_nm, *, vertical_nm=None, vertical_scale=None, vertical_surface_ratio=None, vertical_decay_nm=None, lateral="periodic")` | ✅ | Exact anisotropic Gaussian bake. |
| `peb_car(volume, *, peb_time_s=60, k_amp=0.1, k_quench=10, quencher=0.15, acid_diffusivity_nm2_s=2, quencher_diffusivity_nm2_s=0.5, vertical_ratio=1, lateral="periodic", max_time_step_s=None)` | 🔶 | `CarPebResult`: `protected`, `acid`, `quencher`, `neutralized_total`, `steps`, `time_step_s`. |
| `height_map_from_times(times, dev_time_s)` | | `HeightMapResult` (nm). |
| `grayscale_height_map(…)`, `grayscale_transmittance_for_target(…)`, `blazed_grating(…)`, `microlens_array(…)`, `simulate_grayscale(target_height_nm, thickness_nm, *, d_th, d_clear, exposure_dose_mj_cm2, …)` → `GrayscaleResult` | 🔶 | Grayscale 2.5D lithography ([page](./processes/grayscale.md)). |

`VolumetricResult` provides:

- `values`, a read-only (nz, ny, nx) view;
- `x_nm`, `y_nm`, `z_nm`, `shape` and `extent_nm`;
- `cd_at_z(threshold)` and `depth_map(threshold)`;
- `VolumetricResult.from_array(values, x_range_nm, y_range_nm, z_range_nm)`, which wraps your own array.

`HeightMapResult` provides `values`, a read-only (ny, nx) view in nm, plus its coordinates.

<!-- verify-example -->
```python
import highuvlith as huv

source = huv.SourceConfig.f2_laser(sigma=0.7)
optics = huv.OpticsConfig(numerical_aperture=0.75)
mask = huv.MaskConfig.line_space(cd_nm=100.0, pitch_nm=300.0)
grid = mask.commensurate_grid(size=32, target_pixel_nm=10.0)
res = huv.simulate_volumetric(
    source, optics, mask, huv.FilmStackConfig(), huv.ResistConfig.vuv_fluoropolymer(), grid,
    nz=8, n_defocus_planes=2, develop="level_set", dev_time_s=30.0,
)
print(res.latent.shape, res.level_set.status, res.height_map.shape)
```

Output:

```text
(8, 32, 32) ✅/🔶 (32, 32)
```

## Interference, Talbot and EUV-IL

| Function | Badge | Notes |
|---|---|---|
| `simulate_interference(preset="two_beam", *, wavelength_nm=200, n_medium=1.6, half_angle_deg=30, nx=128, ny=128, nz=32, x_span_nm=1000, y_span_nm=1000, z_span_nm=500, dose_scale=1, dill_c=0.02, two_photon=False)` | ✅/🔶 | Multi-beam lattice exposed into a PAC volume ([page](./processes/interference-volumetric.md)). |
| `simulate_talbot(wavelength_nm=13.5, period_nm=100, *, grating="amplitude", duty_cycle=0.5, …, exposure="dtl", …)` → `TalbotResult` | 🔶 | `carpet` (nz, nx), `coherent_image`, `stationary_image`, `dtl_image`, `atl_image`, `talbot_length_nm`, `achromatic_distance_nm`, `efficiencies`, … ([page](./processes/talbot.md)). |
| `simulate_euv_il(grating_period_nm=100, wavelength_nm=13.5, *, order=1, …)` → `EuvIlResult` | 🔶 | Fringe period p/(2m): `fringe_period_nm`, `diffraction_angle_deg`, `visibility`, `beam_intensities`, `intensity`, `pac`. |

<!-- verify-example -->
```python
import highuvlith as huv

talbot = huv.simulate_talbot(wavelength_nm=13.5, period_nm=100.0, nx=64, carpet_nz=32)
print(f"z_T = {talbot.talbot_length_nm:.1f} nm, {talbot.status}")

il = huv.simulate_euv_il(grating_period_nm=100.0, nx=32, ny=2, nz=4)
print(f"fringe period {il.fringe_period_nm:.1f} nm, visibility {il.visibility:.3f}")
```

Output:

```text
z_T = 1481.5 nm, 🔶
fringe period 50.0 nm, visibility 1.000
```

## Optimization and patterning

See [Research modules](./research-modules.md).

| Function | Badge | Returns |
|---|---|---|
| `optimize_ilt(source, optics, grid, target, **kwargs)` (`cost`, `gradient="adjoint"\|"proxy"`, `optimizer`, `max_iterations`, `tv_weight`, `binarization_weight`, `conditions`, `illumination`, …); `contact_target(size_nm, pitch_nm, count, grid, *, shape="round")`, `ilt_contact_target(…)` | ✅ (adjoint), 🔶 (proxy) | `IltResult`: `mask`, `aerial_image`, `binary_mask`, `cost_history`, `termination`, `pattern_error`, `snapshots`, … |
| `fragment_opc(source, optics, mask, grid, threshold, **kwargs)` | ✅ | `OpcResult`: `mask`, `polygons`, `final_epe_rms_nm`, `final_epe_max_nm`, `epe_rms_history`, `fragment_*`, … |
| `insert_srafs(source, optics, mask, grid, *, line_cd_nm, sigma_center, threshold, **kwargs)`; `compare_sraf_dof(…)`; `isolated_line_sraf_study(cd_nm=80, *, …)` → `SrafStudy` (`threshold`, `sraf`, `dof`, `status`, `notes`); `dose_to_size_threshold(…)` | 🔶 | `SrafResult` (`assists`, `placed`, `removed`, `worst_margin`); `DofComparison` (`dof_without_nm`, `dof_with_nm`, `gain`, `focus_nm`, `cd_without_nm`, `cd_with_nm`). |
| `simulate_lele(source, optics, grid, cd_nm, pitch_nm, overlay_x_nm=0, overlay_y_nm=0, dose1_mj_cm2=30, dose2_mj_cm2=30, …)` | 🔶 | `LeleResult`: `aerial1`, `aerial2`, `aerial2_overlay`, `printed_pattern(threshold_mj_cm2)`, `lines(threshold_mj_cm2)`, … |
| `sadp(mandrel_pitch_nm, mandrel_cd_nm, spacer_nm, *, tone="spacer_is_line")`, `saqp(…)` | 🔶 (purely geometric) | `SpacerPatterningResult`: `lines`, `pitches_nm`, `nominal_pitch_nm`, `pitch_walk_nm`, `cd_range_nm`. |
| `simulate_dsa(template, pixel_nm, l0_nm=28, chi_n=20, volume_fraction=0.5, morphology="lamellar", …)`, `dsa_confined_lamellae(trench_nm, l0_nm)` | 🔶 (analytic, not SCFT) | `DsaResult`: `pattern`, `assembled_cd_nm`, `heuristic_defect_index` (uncalibrated), `commensurability()`. |

<!-- verify-example -->
```python
import highuvlith as huv

walk = huv.sadp(mandrel_pitch_nm=80.0, mandrel_cd_nm=20.0, spacer_nm=20.0)
print(walk.nominal_pitch_nm, walk.pitch_walk_nm, walk.status)
```

Output (pitch walk 2W + 2t − P = 0 for this choice):

```text
40.0 0.0 🔶
```

## Stochastics

| Function | Badge | Notes |
|---|---|---|
| `compute_ler_lwr(aerial, dose_mj_cm2, threshold=0.3, source=None, photon_density_per_mj_cm2=None, dose_jitter_rms=None, pulses_per_exposure=None, absorbed_fraction=None, num_realizations=100)` → `LerResult` | 🔶 | Poisson photon noise on one image row plus Gamma dose jitter. Fields: `ler_3sigma_nm`, `lwr_3sigma_nm`, `cd_mean_nm`, `cd_sigma_nm`, edges, `measured_realizations`. |
| `photons_per_square(dose_mj_cm2, wavelength_nm, side_nm)`, `relative_shot_noise(…)` | ✅ | D·a² / (hc/λ) and 1/√N. |
| `resist_absorbed_fraction(thickness_nm, absorption_per_um)` | ✅ | 1 − exp(−α d). |

<!-- verify-example -->
```python
import highuvlith as huv

aerial = huv.simulate_line_space(100.0, 300.0, grid_size=128, pixel_nm=2.0).aerial
ler = huv.compute_ler_lwr(aerial, 30.0, source=huv.SourceConfig.f2_laser(), num_realizations=20)
print(ler.measured_realizations, ler.status)
print(round(huv.photons_per_square(30.0, 13.5, 16.0)), round(huv.relative_shot_noise(30.0, 13.5, 16.0), 4))
```

Output:

```text
20 🔶
5219 0.0138
```

## Quantum (N-photon) research module

Everything here is 🧪 Theoretical. No N-photon resist has recorded an entangled sub-Rayleigh pattern. The module keeps two physically different models apart:

- **Classical N-photon absorption** $E = I^N$ (`n_photon_absorption_image(classical, num_photons=2)`). It sharpens lines and raises contrast but **keeps the classical period**, so it gives no resolution gain. The old `quantum_aerial_image(classical, n=2, …)` is a deprecated alias whose `fidelity` argument has no effect.
- **Ideal entangled N00N models** (Boto et al., PRL 85, 2733, 2000), which come in two forms:
  - `TwoBeamNPhoton(wavelength_nm, half_angle_deg, num_photons=2, fidelity=1.0, phase_rad=0.0)` gives two-beam fringes in closed form, at period $\lambda/(2N \sin\theta)$ instead of $\lambda/(2 \sin\theta)$. Methods: `classical_period_nm()`, `noon_period_nm()`, `fringe_wavenumber_per_nm()`, `exposure(x)`, `noon_exposure(x)`, `classical_exposure(x)`, `profile(x_array)`, `harmonic_amplitude(j)`, `flux_budget()`.
  - `SimulationEngine.compute_noon_ideal_image(num_photons, fidelity, focus_nm, mask)` → `NoonImageResult` handles general masks. It images the same system at $\lambda/N$. The result holds `classical`, `n_photon_absorption`, `noon_limit` and `exposure` (each an `AerialImageResult`), plus `effective_wavelength_nm`, `classical_resolution_nm`, `noon_limit_resolution_nm` and `flux`. It needs clear-field normalization and a pixel no larger than $\lambda/(4 N \mathrm{NA})$.

The fidelity mixture for general masks is:

```math
E = F \, I_{\lambda/N} + (1 - F) \, I_{\lambda}^{N}
```

The non-N00N fraction is classical N-photon absorption, not linear absorption.

- `quantum_flux_budget(wavelength_nm=157.63, num_photons=2)` returns a dict derived from the entangled-source constants: `photon_energy_ev`, `hvm_photon_rate_per_s`, `entangled_rate_ceiling_per_s`, `relative_flux`, and the exposure-time ratios `exposure_time_ratio_bound` (a **lower** bound), `exposure_time_ratio_tightest_bound` and `exposure_time_ratio_claimed`.
- `simulate_quantum_line_space(cd_nm, pitch_nm, *, n=2, fidelity=1.0, model="n_photon_absorption"|"noon", …)` → `QuantumResult` returns `classical`, `quantum`, the two contrasts, `model`, the resolution figures, `exposure_time_ratio_bound`, `status` and `notes`.

<!-- verify-example -->
```python
import highuvlith as huv

tb = huv.TwoBeamNPhoton(157.63, 30.0, num_photons=2, fidelity=1.0)
print(tb.classical_period_nm(), tb.noon_period_nm())

budget = huv.quantum_flux_budget(157.63, 2)
print(f"{budget['exposure_time_ratio_bound']:.3g}")

q = huv.simulate_quantum_line_space(50.0, 100.0, model="noon", sigma=0.5, grid_size=64, pixel_nm=2.0)
print(f"classical {q.classical_contrast:.3f}, N00N limit {q.quantum_contrast:.3f}, {q.status}")
```

Output:

```text
157.63000000000002 78.81500000000001
4.47e+10
classical 0.000, N00N limit 0.731, 🧪
```

A 100 nm pitch lies below the classical cutoff $\lambda/(\mathrm{NA}(1+\sigma)) \approx 140$ nm, so the classical image (and $I^2$) has zero contrast. The ideal $\lambda/2$ limit resolves it, at an exposure time about $4.5 \times 10^{10}$ times the classical one (a lower bound: the loosest ETPA cross-section bound).

## MNSL

MNSL is 🧪: the lattice geometry and the moiré period are exact, but the emission map is a heuristic. The native classes are exported as `MnslConfig`, `MnslEngine`, `MnslResult`, `NanosphereArrayConfig`, `SpherePacking`, `SubstrateCoupling` and `py_simulate_moire_emission`. The Python helpers are:

- `simulate_moire_emission(sphere_diameter_nm, array_pitch_nm, rotation_angle_deg, *, separation_nm=100, wavelength_nm=157, grid_size=256, pixel_nm=2, …)`, which returns an `MnslSimResult` (the native result as `.result`, its arrays as NumPy, `coordinates`, `cross_section_x/y`, `num_peaks`, and the native `status` (🧪) and `notes` forwarded);
- `create_nanosphere_array`, `sweep_rotation_angle`, `sweep_separation` and `optimize_moire_parameters`.

`MnslResult` fields: `emission_pattern`, `moire_pattern`, `enhancement_factors`, `moire_period_nm`, `peak_enhancement`, `peak_positions`, `status`, `notes`.

## Visualization

`highuvlith.viz` needs matplotlib and is imported separately (`from highuvlith import viz`). Every helper:

- takes `theme="light"|"dark"` (default: the current theme) and `ax=` (or `fig=` for multi-panel figures);
- returns the Axes or Figure without showing it;
- uses one validated palette, with `viz.use_theme(...)`, `viz.THEMES`, `categorical`, `ordinal_colors`, `sequential_cmap` and `diverging_cmap`.

`viz.export_light_dark(plot_fn, stem, *args, formats=("png",), dpi=200, **kwargs)` renders a helper in both themes for documentation pages (`<stem>-light.<ext>`, `<stem>-dark.<ext>`).

| Area | Helpers |
|---|---|
| Imaging | `plot_aerial`, `plot_cross_section`, `plot_polarization_contrast` (TE/TM two-beam contrast vs NA) |
| Sources | `plot_source` (spectrum + pupil fill), `plot_source_landscape` (log λ vs log power, EUV HVM band) |
| Process window / resist | `plot_bossung`, `plot_ed_window`, `plot_el_vs_dof`, `plot_resist_profile` |
| Deep-layer | `plot_xz_slice`, `plot_depth_dose`, `plot_height_map`, `plot_development_front` (level set vs FMM), `plot_isosurface` (plotly), `plot_liga_edge_profile`, `plot_talbot_carpet` |
| Optimization | `plot_ilt_evolution`, `plot_sraf_dof` |
| MNSL | `plot_emission_pattern`, `plot_moire_analysis`, `plot_rotation_sweep`, `plot_optimization_heatmap` |

`highuvlith.viz.plotly_viz` (plotly) adds `plot_aerial_plotly`, `plot_cross_section_plotly` and `plot_bossung_plotly`. `highuvlith.interactive` (ipywidgets) adds `interactive_aerial(...)` and `interactive_focus_sweep(...)`: slider explorers for a line/space aerial image and its focus sweep, for use in your own notebooks. The notebooks in [`examples/notebooks/`](../examples/notebooks) do not use them (they run headlessly in CI); `tests/python/test_interactive.py` runs both helpers against stand-in widgets.

<!-- verify-example -->
```python
import tempfile
import highuvlith as huv
from highuvlith import viz

r = huv.simulate_line_space(100.0, 300.0, grid_size=64, pixel_nm=4.0)
with tempfile.TemporaryDirectory() as out:
    paths = viz.export_light_dark(viz.plot_aerial, f"{out}/aerial", r.aerial, formats=("svg",))
    print([p.name for p in paths])
ax = viz.plot_source_landscape(theme="dark")
print(ax.get_xscale(), ax.get_yscale())
```

Output:

```text
['aerial-light.svg', 'aerial-dark.svg']
log log
```

## Type stub and drift guard

Every public name in `highuvlith._native` has an entry in [`_native.pyi`](../python/highuvlith/_native.pyi), and the package ships `py.typed`. [`tests/python/test_stub_drift.py`](../tests/python/test_stub_drift.py) loads the native module and fails if the stub misses a class, function or member. It also fails if the stub has the wrong member kind (property, staticmethod or method) or different parameter names, order or defaults. Any change to the PyO3 bindings must update the stub in the same change.

See the [CLI reference](./cli.md) for the file-driven equivalent and the [configuration reference](./configuration.md) for the TOML schema.
