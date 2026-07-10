# Python API Reference

**Status:** ✅ Implemented — the bindings in [`crates/highuvlith-py`](../crates/highuvlith-py/src) cross the boundary zero-copy (numpy) and release the GIL for batch and deep-layer compute. The type stub [`_native.pyi`](../python/highuvlith/_native.pyi) is drift-guarded against the bindings by [`tests/python/test_stub_drift.py`](../tests/python/test_stub_drift.py). Individual capabilities span the honesty spectrum; see the [capability matrix](./capability-matrix.md).

The Python package `highuvlith` layers a small pure-Python convenience API ([`python/highuvlith/`](../python/highuvlith)) over the native extension `highuvlith._native` (the compiled Rust). Import the top-level package for everything except visualization:

```python
import highuvlith as huv
```

You must build the extension before importing (`maturin develop`, or `maturin develop --release` for optimized compute). The module installs as `highuvlith._native`; the config/result classes are re-exported at the `highuvlith` top level.

## Package layout

| Import path | Contents |
|-------------|----------|
| `highuvlith` | Config, engine, and result classes; convenience functions; MNSL API. |
| `highuvlith._native` | Compiled Rust classes/functions (imported by the above; rarely used directly). |
| `highuvlith.viz` | Matplotlib plots. Requires `matplotlib`. **Imported separately** (`from highuvlith import viz`). |
| `highuvlith.viz.plotly_viz` | Interactive Plotly plots. Requires `plotly`. |
| `highuvlith.interactive` | Jupyter ipywidgets explorers. Requires `ipywidgets`. |

`viz` and `interactive` are **not** pulled into the top-level namespace — import them explicitly so the heavy plotting dependencies stay optional.

---

## Configuration classes

### `SourceConfig`

Wraps any of the nine source families behind one Python class. Use the default constructor for a custom VUV source, or a static factory for a preset. All factories validate their inputs and raise `ValueError` on bad parameters.

**Constructor** — custom VUV excimer:

```python
huv.SourceConfig(wavelength_nm=157.63, sigma_outer=0.7, bandwidth_pm=1.1, spectral_samples=5)
```

**Factories by family:**

| Family | Factory | Signature (defaults shown) |
|--------|---------|----------------------------|
| VUV | `SourceConfig.f2_laser` | `(sigma=0.7)` — F₂ 157.63 nm |
| VUV | `SourceConfig.ar2_laser` | `(sigma=0.7)` — Ar₂ 126 nm |
| LPA-FEL | `SourceConfig.lpa_fel_bella_25nm` | `(sigma=0.7)` — BELLA 500 MeV target, ~25 nm |
| LPA-FEL | `SourceConfig.lpa_fel` | `(wavelength_nm, sigma=0.7, electron_energy_mev=500.0, bandwidth_pm=25.0, pulse_duration_fs=10.0, rep_rate_hz=1000.0)` |
| LPP | `SourceConfig.lpp_sn_13nm5` | `(sigma=0.9)` — Sn 13.5 nm |
| LPP | `SourceConfig.lpp_gd_6nm7` | `(sigma=0.9)` — Gd 6.7 nm (BEUV) |
| Synchrotron | `SourceConfig.synchrotron_undulator` | `(electron_energy_gev=0.538, period_mm=20.0, k=1.0, num_periods=100, harmonic=1)` — λ derived from resonance |
| Synchrotron | `SourceConfig.synchrotron_liga_bending_magnet` | `()` — 2.5 GeV, 1.5 T, E_c = 6.23 keV |
| HHG | `SourceConfig.hhg` | `(driver_wavelength_nm=800.0, gas="neon", driver_intensity_w_cm2=4e14, harmonic=59, monochromator_bandwidth_pm=15.0)` — odd harmonics, cutoff-checked |
| HHG | `SourceConfig.hhg_ne_13nm5` | `()` — Ne, harmonic 59 → 13.56 nm |
| XFEL | `SourceConfig.xfel_flash_13nm5` | `()` — FLASH-class SASE |
| XFEL | `SourceConfig.xfel_fermi_seeded` | `()` — FERMI-class seeded |
| ICS 🧪 | `SourceConfig.ics` | `(target_wavelength_nm=13.5, laser_wavelength_nm=1030.0, laser_a0=0.1)` — e-energy derived |
| SSMB 🧪 | `SourceConfig.ssmb_euv_13nm5` | `()` — projected kW EUV design point |
| Entangled 🧪 | `SourceConfig.entangled_noon` | `(wavelength_nm=157.63, n=2, fidelity=1.0)` |

**Getters** (read-only properties):

| Property | Type | |
|----------|------|--|
| `wavelength_nm` | float | Resolved wavelength (derived value for synchrotron/HHG). |
| `bandwidth_pm` | float | Spectral FWHM. |
| `sigma_outer` | float | Partial-coherence σ (0 for sources without a pupil σ). |
| `spectral_samples` | int | Polychromatic sample count. |
| `kind` | str | Family label (`"vuv"`, `"lpa_fel"`, `"lpp"`, …). |
| `electron_energy_mev` | float \| None | LPA-FEL only; `None` for families without an e-beam stage. |
| `pulse_duration_fs` | float \| None | `None` when the model does not track pulse duration. |
| `rep_rate_hz` | float | 0.0 for CW/untracked sources. |
| **`average_power_w`** | float \| None | **Live** — pulse energy × rep rate (or the model's own value). `None` when untracked. |
| **`shot_to_shot_rms`** | float | **Live** — relative rms pulse-energy jitter (feeds LER dose jitter). 0 = stable. |
| **`transverse_coherence_fraction`** | float \| None | **Live** — coherence in [0, 1]; `None` for sources that don't model it. |

The three bolded getters expose derived physics: `average_power_w` and `shot_to_shot_rms` drive throughput and stochastic dose-jitter analysis; `transverse_coherence_fraction` sets SOCS source-sampling density.

```python
src = huv.SourceConfig.lpa_fel_bella_25nm(sigma=0.7)
print(src.kind, src.wavelength_nm, src.average_power_w, src.transverse_coherence_fraction)
```

### `OpticsConfig`

```python
huv.OpticsConfig(numerical_aperture=0.75, reduction=4.0, flare_fraction=0.02)  # refractive CaF2 (default)
huv.OpticsConfig.schwarzschild(numerical_aperture=0.33, obscuration_ratio=0.25,
                               reduction=4.0, mirror_reflectivity=0.67, flare=0.03)
huv.OpticsConfig.zone_plate(outer_zone_width_nm, design_wavelength_nm)
```

| Member | | |
|--------|--|--|
| `schwarzschild(...)` | static factory | Two-mirror reflective objective for EUV/BEUV/soft X-ray (defaults: 13.5 nm Mo/Si preset; use `mirror_reflectivity=0.50`, `obscuration_ratio=0.3` for the 6.7 nm La/B₄C band). |
| `zone_plate(outer_zone_width_nm, design_wavelength_nm)` | static factory | Fresnel zone plate; NA = λ/(2·Δr_N), strongly chromatic. |
| `add_aberration(fringe_index, coefficient_waves)` | method | Append a Zernike aberration term (refractive only; raises `ValueError` otherwise). |
| `rayleigh_resolution(wavelength_nm)` | method | Rayleigh resolution (nm) at a wavelength. |
| `kind` | getter | `"refractive"`, `"schwarzschild"`, or `"zone_plate"`. |
| `numerical_aperture`, `reduction`, `flare_fraction` | getters | |

> **Choose optics honestly.** Refractive CaF₂ lenses are physically impossible below ~110 nm — use `schwarzschild()` for EUV/BEUV and `zone_plate()` for soft X-ray. Unlike the [CLI](./configuration.md#optics) (which warns) and the [GUI](./gui.md) (which auto-selects), the Python constructor default stays refractive without a warning, so the choice is yours.

### `MaskConfig`

Static factories only:

```python
huv.MaskConfig.line_space(cd_nm, pitch_nm)
huv.MaskConfig.contact_hole(diameter_nm, pitch_x_nm, pitch_y_nm)
```

### `ResistConfig`

```python
huv.ResistConfig(thickness_nm=150.0, dill_a=0.2, dill_b=0.45, dill_c=0.02,
                 peb_diffusion_nm=30.0, model="mack")
huv.ResistConfig.vuv_fluoropolymer()   # preset
```

`model` is `"mack"` or `"threshold"`. Getters: `thickness_nm`, `dill_a`, `dill_b`, `dill_c`, `peb_diffusion_nm`.

### `FilmStackConfig`

```python
stack = huv.FilmStackConfig()
stack.add_layer(name, thickness_nm, n_real, n_imag)   # top to bottom
stack.set_substrate(n_real, n_imag)
```

Used by `expose_volumetric` to place the resist within a multilayer stack.

### `ProcessConfig`

```python
huv.ProcessConfig(dose_mj_cm2=30.0, focus_nm=0.0, development_time_s=60.0)
```

Read/write attributes: `dose_mj_cm2`, `focus_nm`, `development_time_s`. **Standalone value holder** — `SimulationEngine` does not take a `ProcessConfig`; dose/focus/dev-time are passed as method arguments (`compute_resist_profile(dose_mj_cm2=…, focus_nm=…, dev_time_s=…)`). Use it to carry process conditions in your own code.

### `GridConfig`

```python
huv.GridConfig(size=512, pixel_nm=1.0)
```

`size` must be a power of two. Getters: `size`, `pixel_nm`; method `field_size_nm()`. Note the class default is `size=512`, but when a `SimulationEngine`/`BatchSimulator` is created with `grid=None` it falls back to the core default (256).

---

## Simulation

### `SimulationEngine`

Precomputes the TCC/SOCS decomposition once, then evaluates many conditions cheaply.

```python
engine = huv.SimulationEngine(source, optics, mask, resist=None, grid=None, max_kernels=30)
```

`resist` defaults to `ResistConfig.vuv_fluoropolymer()`; `grid` defaults to the core grid.

| Method | Returns | |
|--------|---------|--|
| `compute_aerial_image(focus_nm=0.0)` | `AerialImageResult` | Monochromatic aerial image. |
| `compute_polychromatic(focus_nm=0.0)` | `AerialImageResult` | Spectral-integrated image (VUV chromatic aberration). |
| `measure_cd(dose_mj_cm2=30.0, focus_nm=0.0, threshold=0.3)` | float | CD via aerial-image thresholding. Raises if no crossing. |
| `compute_resist_profile(dose_mj_cm2=30.0, focus_nm=0.0, dev_time_s=60.0)` | `ResistProfileResult` | Expose → PEB diffuse → develop. |
| `image_contrast(focus_nm=0.0)` | float | (I_max − I_min)/(I_max + I_min). |
| `num_kernels()` | int | SOCS kernel count. |

Getters `source`, `optics`, `mask`, `resist`, `grid` return the stored configs.

### `BatchSimulator`

Parameter sweeps with the **GIL released** during compute, so batches parallelize.

```python
batch = huv.BatchSimulator(source, optics, mask, grid=None, max_kernels=30)
```

| Method | Returns | |
|--------|---------|--|
| `process_window(doses, focuses, cd_threshold=0.3, cd_target_nm=65.0, cd_tolerance_pct=10.0)` | `ProcessWindowResult` | CD(dose, focus) matrix + DOF/EL. GIL released. |
| `batch_defocus(focuses)` | `list[(focus_nm, ndarray)]` | Aerial image per focus. GIL released. |

---

## Result classes

### `AerialImageResult`

| Member | | |
|--------|--|--|
| `intensity` | property → ndarray (ny, nx) | Zero-copy intensity grid. |
| `x_nm`, `y_nm` | property → ndarray | Cell-center coordinates. |
| `cross_section(y_nm=0.0)` | method → (x, values) | 1D cut along x at nearest row. |
| `image_contrast()` | method → float | |
| `nils(threshold=0.3)` | method → float \| None | Normalized image log-slope; `None` if no crossing. |

### `ResistProfileResult`

`x_nm`, `height_nm` (properties → ndarray), `thickness_nm` (property → float).

### `ProcessWindowResult`

`cd_matrix` (doses × focuses ndarray), `doses`, `focuses` (properties); `depth_of_focus()` → nm, `exposure_latitude()` → percent (methods).

### Deep-layer results

| Class | Members |
|-------|---------|
| `VolumetricResult` | `values` (nz, ny, nx), `x_nm`/`y_nm`/`z_nm`, `shape`; `cd_at_z(threshold)` → list, `depth_map(threshold)` → `HeightMapResult`. A z-resolved PAC / dose / arrival-time field. |
| `HeightMapResult` | `values` (ny, nx), `x_nm`/`y_nm`, `shape`. A 2D height/depth map (nm). |
| `LigaResult` | `depth_dose` → (z_um, dose_kj_cm3), `dose_ratio`, `top_dose_kj_cm3`, `bottom_dose_kj_cm3`, `exceeds_damage_ceiling`, `volume` → `VolumetricResult`, `developed_depth` → `HeightMapResult`. |

### Python dataclasses (from `highuvlith.api`)

| Class | Fields |
|-------|--------|
| `FullResult` | `aerial`, `contrast`, `config`, `resist_profile`, `cd_nm`, `nils`. |
| `GrayscaleResult` | `target_height_nm`, `transmittance`, `height_map`. |
| `QuantumResult` | `classical`, `quantum`, `classical_contrast`, `quantum_contrast`. |

---

## Convenience functions

High-level one-liners in `highuvlith.api`, re-exported at the top level.

### Imaging

```python
huv.simulate_line_space(cd_nm, pitch_nm, *, wavelength_nm=157.63, na=0.75, sigma=0.7,
                        focus_nm=0.0, dose_mj_cm2=30.0, grid_size=256, pixel_nm=1.0,
                        with_resist=False, max_kernels=20) -> FullResult

huv.simulate_contact_hole(diameter_nm, pitch_x_nm, pitch_y_nm=None, *, wavelength_nm=157.63,
                          na=0.75, sigma=0.7, focus_nm=0.0, grid_size=256, pixel_nm=1.0,
                          max_kernels=20) -> FullResult

huv.sweep_focus(cd_nm=65.0, pitch_nm=180.0, *, wavelength_nm=157.63, na=0.75, sigma=0.7,
                focus_min=-300.0, focus_max=300.0, focus_steps=21, grid_size=128,
                pixel_nm=2.0) -> dict
```

`sweep_focus` returns `{"focuses", "contrasts", "best_focus_nm", "best_contrast"}`. All three validate inputs (positive, in-range, power-of-two grid) and raise `ValueError` on violation.

```python
result = huv.simulate_line_space(65.0, 180.0, na=0.75, with_resist=True)
print(f"contrast={result.contrast:.3f}  NILS={result.nils:.2f}")
```

### Deep-layer

```python
huv.simulate_liga(critical_energy_kev=6.23, resist_thickness_um=500.0, *, cd_nm=5000.0,
                  pitch_nm=10000.0, grid_size=128, pixel_nm=200.0, nz=64) -> LigaResult

huv.simulate_interference(preset="two_beam", *, wavelength_nm=200.0, n_medium=1.6,
                          half_angle_deg=30.0, nx=128, ny=128, nz=32, x_span_nm=1000.0,
                          y_span_nm=1000.0, z_span_nm=500.0, dose_scale=1.0, dill_c=0.02,
                          two_photon=False) -> VolumetricResult

huv.simulate_grayscale(target_height_nm, thickness_nm, *, d_th, d_clear,
                       exposure_dose_mj_cm2, wavelength_nm=157.63, na=0.75, sigma=0.7,
                       pixel_nm=2.0, max_kernels=20) -> GrayscaleResult

huv.simulate_quantum_line_space(cd_nm, pitch_nm, *, n=2, fidelity=1.0, wavelength_nm=157.63,
                                na=0.75, sigma=0.7, focus_nm=0.0, grid_size=256, pixel_nm=1.0,
                                max_kernels=20) -> QuantumResult
```

- `simulate_interference` `preset` is `"two_beam"` (1D grating), `"three_beam_hex"` (2D hex), or `"four_beam_umbrella"` (FCC-like 3D).
- `simulate_grayscale` `target_height_nm` is a square power-of-two array; generate one with `blazed_grating` or `microlens_array` (below).
- `simulate_quantum_line_space` is theoretical (🧪) — NOON-state N-photon sharpening on top of the classical image.

### Low-level deep-layer functions

Imported at the top level and re-exported from `highuvlith.api` where wrapped:

| Function | Returns | |
|----------|---------|--|
| `expose_volumetric(source, optics, mask, film_stack, resist, grid, dose_mj_cm2=30.0, nz=64, n_defocus_planes=8, dose_steps=1, base_defocus_nm=0.0, resist_layer=0, max_kernels=20)` | `VolumetricResult` | z-resolved PAC latent image through the film stack. GIL released. |
| `develop_fast_marching(volume, resist, pixel_xy_nm, pixel_z_nm)` | `VolumetricResult` | Fast-marching arrival times (s); undercut/sidewalls. GIL released. |
| `height_map_from_times(times, dev_time_s)` | `HeightMapResult` | Remaining height (nm) after a develop time. |
| `grayscale_height_map(source, optics, grid, transmittance, dose_mj_cm2, d_th, d_clear, thickness_nm, max_kernels=20)` | `HeightMapResult` | Image a transmittance mask → height map. GIL released. |
| `grayscale_transmittance_for_target(target_height, thickness_nm, d_th, d_clear, exposure_dose)` | ndarray | Synthesize the mask that prints a target relief. |
| `blazed_grating(n, period_px, depth_nm, thickness_nm)` | ndarray | Blazed-grating target height map. |
| `microlens_array(n, pitch_px, sag_nm, thickness_nm)` | ndarray | Microlens-array target height map. |
| `quantum_aerial_image(classical, n=2, wavelength_nm=157.63, na=0.75, fidelity=1.0)` | ndarray | Apply N-photon sharpening to a classical image. |

---

## MNSL (Moiré Nanosphere Lithography)

Classes (re-exported at top level): `MnslConfig`, `MnslEngine`, `MnslResult`, `NanosphereArrayConfig`, `SpherePacking`, `SubstrateCoupling`, and the low-level `py_simulate_moire_emission`.

High-level API from `highuvlith.mnsl` (also top-level):

| Function | |
|----------|--|
| `simulate_moire_emission(sphere_diameter_nm, array_pitch_nm, rotation_angle_deg, *, separation_nm=100.0, wavelength_nm=157.0, grid_size=256, pixel_nm=2.0, sphere_material="silica", coupling_strength=0.5, enable_nearfield=True)` | One-call emission → `MnslSimResult`. |
| `create_nanosphere_array(diameter_nm, pitch_nm, *, orientation_deg=0.0, material="silica", packing="hcp")` | Build a `NanosphereArrayConfig`. |
| `sweep_rotation_angle(...)` | Enhancement vs rotation angle. |
| `sweep_separation(...)` | Enhancement vs layer separation. |
| `optimize_moire_parameters(...)` | Grid-search angle × separation for peak enhancement. |

`MnslSimResult` exposes `emission_pattern`, `moire_pattern`, `enhancement_factors`, `coordinates`, `cross_section_x/y`, `moire_period_nm`, `peak_enhancement`, `total_emission_power`, `peak_positions`, `num_peaks`.

```python
r = huv.simulate_moire_emission(200.0, 300.0, 5.0)
print(f"period={r.moire_period_nm:.1f} nm  peak={r.peak_enhancement:.2f}×  peaks={r.num_peaks}")
```

---

## Visualization

Import `viz` explicitly (`from highuvlith import viz`). Matplotlib functions return the `Axes`; Plotly functions return the `Figure`.

### Matplotlib (`highuvlith.viz`, requires `matplotlib`)

| Function | |
|----------|--|
| `plot_aerial(result, *, title=None, cmap="inferno", show_colorbar=True, ax=None)` | Aerial heatmap. |
| `plot_cross_section(result, ...)` | 1D intensity cut. |
| `plot_bossung(pw, *, title=None, ax=None)` | CD-vs-focus Bossung curves from a `ProcessWindowResult`. |
| `plot_ed_window(pw, *, cd_target_nm=65.0, cd_tolerance_pct=10.0, title=None, ax=None)` | Exposure-defocus window. |
| `plot_resist_profile(result, *, title=None, ax=None)` | Developed resist topography. |
| `plot_emission_pattern`, `plot_moire_analysis`, `plot_rotation_sweep`, `plot_optimization_heatmap` | MNSL plots. |

### Plotly (`highuvlith.viz.plotly_viz`, requires `plotly`)

| Function | |
|----------|--|
| `plot_aerial_plotly(result, *, title=None, colorscale="Inferno")` | Interactive heatmap with hover. |
| `plot_cross_section_plotly(result, *, y_nm=0.0, threshold=None, title=None)` | Interactive cross-section. |
| `plot_bossung_plotly(pw, *, title=None)` | Interactive Bossung curves. |

### Jupyter (`highuvlith.interactive`, requires `ipywidgets`)

| Function | |
|----------|--|
| `interactive_aerial(...)` | Six-slider live aerial-image + cross-section explorer. |
| `interactive_focus_sweep(cd_nm=65.0, pitch_nm=180.0, grid_size=128)` | Live contrast-vs-focus curve. |

```python
from highuvlith import viz
result = huv.simulate_line_space(65.0, 180.0, with_resist=True)
viz.plot_aerial(result.aerial)
viz.plot_resist_profile(result.resist_profile)
```

Example notebooks ship in [`examples/notebooks/`](../examples/notebooks) — `01_quickstart_aerial`, `02_source_gallery`, and `03_deep_litho` (executed in CI). The ipywidgets helpers above are used within them.

---

## The `_native.pyi` stub and drift guard

Every public class, function, and member exposed by `highuvlith._native` has a matching entry in [`python/highuvlith/_native.pyi`](../python/highuvlith/_native.pyi), which is the source of type information for editors and type checkers (the package ships `py.typed`).

[`tests/python/test_stub_drift.py`](../tests/python/test_stub_drift.py) enforces this: it introspects the loaded native module and asserts that every non-underscore name — classes, module-level functions, and each class's public members — appears in the stub. **Convention:** any PR that changes the PyO3 bindings must update `_native.pyi` in the same PR, or the drift test fails. When adding a factory, getter, or function to the Rust bindings, add its stub entry before merging.

---

## End-to-end example

```python
import highuvlith as huv
from highuvlith import viz

# 25 nm LPA-FEL through a Schwarzschild reflective objective (Mo/Si preset)
source = huv.SourceConfig.lpa_fel_bella_25nm(sigma=0.7)
optics = huv.OpticsConfig.schwarzschild(numerical_aperture=0.55)
mask   = huv.MaskConfig.line_space(cd_nm=30.0, pitch_nm=100.0)
grid   = huv.GridConfig(size=256, pixel_nm=1.0)

engine = huv.SimulationEngine(source, optics, mask, grid=grid, max_kernels=20)
aerial = engine.compute_aerial_image(focus_nm=0.0)
print(source.kind, source.wavelength_nm, source.average_power_w)
print("contrast", aerial.image_contrast())

# Process window with the GIL released
batch = huv.BatchSimulator(source, optics, mask, grid=grid)
pw = batch.process_window(doses=[20, 30, 40], focuses=[-100, 0, 100])
print("DOF", pw.depth_of_focus(), "EL", pw.exposure_latitude())
viz.plot_bossung(pw)
```

See the [CLI reference](./cli.md) for the file-driven equivalent and the [configuration reference](./configuration.md) for the TOML schema.
