---
description: "The Python API by physics area: sources and throughput, vector pupil helpers, LIGA and X-ray materials, the volumetric resist chain, interference and Talbot, optimization companions, stochastics, the quantum and MNSL research modules and the viz helpers — with verified examples for most areas."
---

# Python API modules

**Status:** ✅ Implemented — the same PyO3 bindings as the [Python API](python-api.md) page, grouped by physics area; entry points in a verify-example are run in CI, the rest are documented but not run here; each keeps the grade of its matrix row · [matrix](capability-matrix.md#frontends-and-infrastructure)

<p class="huv-lead">The second half of the Python reference: the entry points per physics area, from the source landscape to the research modules. The core classes, the canonical run, the engine and the process window are on the Python API page; this page is for the call you reach for once the image exists.</p>

**Same thing elsewhere:** CLI [`deep`](cli.md#deep), [`throughput`](cli.md#throughput), [`sources`](cli.md#sources), [`materials`](cli.md#materials) · TOML [`[deep]`](configuration.md#deep) · GUI [Volume viewer, LIGA, Source tabs](gui.md#views) · physics [Sources](sources/index.md), [Processes](processes/index.md)

## Find by task

<!-- huv-table: stack -->

| I want to … | Use | Section |
|---|---|---|
| list every source preset or estimate wafers per hour | `source_landscape`, `wafer_throughput` | [Sources: landscape and throughput](#sources-landscape-and-throughput) |
| inspect the vector pupil or a two-beam TE/TM contrast | `VectorSettings`, `pupil_polarization_map`, `vector_two_beam_contrast` | [Vector imaging](#vector-imaging) |
| expose thick PMMA with X-rays, look up n, k or a multilayer | `simulate_liga`, `refractive_index`, `MultilayerMirror` | [LIGA, X-ray materials and multilayers](#liga-x-ray-materials-and-multilayers) |
| expose, bake and develop a resist volume | `simulate_volumetric`, `peb_gaussian`, `develop_level_set` | [Volumetric, development and bake](#volumetric-development-and-bake) |
| print with interference, Talbot or EUV-IL gratings | `simulate_interference`, `simulate_talbot`, `simulate_euv_il` | [Interference, Talbot and EUV-IL](#interference-talbot-and-euv-il) |
| estimate shot noise and line-edge roughness | `compute_ler_lwr`, `photons_per_square` | [Stochastics](#stochastics) |
| try the N-photon / N00N or moiré research models | `TwoBeamNPhoton`, `simulate_quantum_line_space`, `simulate_moire_emission` | [Quantum](#quantum-n-photon-research-module) · [MNSL](#mnsl) |
| plot a result in both themes | `highuvlith.viz` | [Visualization](#visualization) |
| optimize a mask or lay out spacers | `optimize_ilt`, `fragment_opc`, `sadp` | [Python API → Optimization and patterning](python-api.md#optimization-and-patterning) |

## Sources: landscape and throughput

`source_landscape()` returns a list of `SourcePreset` records, one per preset: `label`, `factory` (e.g. `"lpp_sn_13nm5()"`), `family`; `wavelength_nm`, `bandwidth_pm`, `photon_energy_ev`, `average_power_w` (W or None), `power_definition`; `maturity` (`"demonstrated"`, `"projection"` or `"theoretical"`, describing the machine) and `status` (the model grade); `broadband`, `note`, and `derived` (`{name: (value, unit)}`). Numbers come live from the factories; `maturity` and `status` are curated labels taken from the capability matrix. Powers follow each family's own definition, so they are **not** all measured at the same plane. `viz.plot_source_landscape()` plots this list.

`wafer_throughput(source, *, dose_mj_cm2=30, preset="auto", **overrides)` (🔶 Simplified) wraps `SourceConfig.wafer_throughput`. With `preset="auto"` the optics-train assumptions follow the wavelength: `"refractive"` above 100 nm (optics 0.30, mask 0.90, 8 mm slit); `"euv"` from 12.4 to 15 nm (10 Mo/Si mirrors at R 0.70, mask 0.65); `"beuv"` from 6 to 7.5 nm (La/B mirrors). Any other wavelength has no projection-lithography optics preset, so `"auto"` raises `ValueError` rather than quote an EUV-optics figure, unless you give the optics train (`optics_transmission`, or `n_mirrors` + `mirror_reflectivity`); the nearest preset then supplies only the mask and field parameters, and a note says so. This covers the X-ray tubes and the betatron (0.15–0.5 nm LIGA/proximity sources) and the 25–47 nm LPA-FEL, Ar HHG and Ar soft-X-ray-laser lines. An explicit `preset=` is honoured at any wavelength as a hypothetical train, and outside the preset's band a `"hypothetical: …"` entry is added to `notes`. It returns the native dict (`wafers_per_hour`, `power_at_wafer_w`, `pulses_per_point`, `scan_speed_mm_s`, `stage_limited`, …) plus `status` and `notes`, or `None` when the source models no power (Hg lamps, the entangled-photon source). The CLI `throughput` subcommand applies the same bands; the native `SourceConfig.wafer_throughput` has no wavelength check and applies whatever `preset` it is given (default `"euv"`).

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

???+ success "Output"

    ```text
    lpp 13.5 250.0 W at IF
      photon_energy = 91.84 eV
      in_band_emission_2pi = 1290 W
      drive_laser_wavelength = 10.6 um
    153.9 euv 🔶
    30 Hg g-line demonstrated ✅/🔶
    ```

The same list, drawn beside 51 published values:

<!-- sources-landscape -> python-api.md -->
<figure markdown="span">

![Average source power at 13.5 nm per family, where only tin LPP reaches the 250 W to 1 kW band and accelerator sources are hollow projections, then on log-log axes against wavelength from X-ray tubes to excimer lasers with 51 published values beside the simulator's presets.](assets/images/sim/sources-landscape-light.png#gh-light-mode-only)
![Average source power at 13.5 nm per family, where only tin LPP reaches the 250 W to 1 kW band and accelerator sources are hollow projections, then on log-log axes against wavelength from X-ray tubes to excimer lasers with 51 published values beside the simulator's presets.](assets/images/sim/sources-landscape-dark.png#gh-dark-mode-only)

<figcaption><strong>The source landscape: 51 published values beside every preset.</strong> Blue: measured or shipping = filled, projection or design = open, discharge plasmas into 2π at the source = squares, ranges as bars — each cited in <code>docs/figures/sim/data/source_power_vs_wavelength.csv</code>; orange diamonds: what the presets report as <code>average_power_w</code> (open = projection or what-if; X-ray tubes: 4π emission with their spectral span). In (b) published points sit 4 % left and presets 4 % right of their wavelength so both stay visible. Powers are defined at different planes (IF, laser output, source), so compare orders of magnitude, not percent. <em>Data chart from source_power_vs_wavelength.csv and the presets; grades per family on each family page; no model badge.</em></figcaption>
</figure>

CLI: [`sources`](cli.md#sources), [`throughput`](cli.md#throughput) · GUI: [Source panel](gui.md#source-panel) · physics: [Sources](sources/index.md).

## Vector imaging

Vector imaging is ✅ Implemented; see [Vector imaging](./vector-imaging.md).

```text
VectorSettings(polarization="unpolarized", angle_deg=0.0, image_index=1.0,
               obliquity=True, reduction=4.0, film_n=None, film_k=0.0)
```

`polarization` is one of `"unpolarized"`, `"x"`, `"y"`, `"te"` (azimuthal), `"tm"` (radial) or `"linear"` (at `angle_deg`). Pass the settings to `SimulationEngine(…, vector=…)`; the engine replaces `reduction` with the optics' value, and `image_index` = 1 inherits the immersion index. Methods: `validate(na)` and `columns_per_source_point()`. Pupil-physics helpers:

<!-- huv-table: stack -->

| Function | Returns |
|---|---|
| `pupil_polarization_map(na, polarization="x", *, n=65, …)` | `PupilPolarizationMap` (`px`, `py`, `field` complex `(n_states, 3, n, n)`, `settings`, `na`) |
| `vector_two_beam_contrast(na, polarization="x", *, p0=1.0, sx=0.5, sy=0, …)` | float, the fringe contrast of two orders at pupil `(±p0, 0)` |
| `vector_pupil_field(settings, na, px, py, sx=0, sy=0)` / `vector_pupil_map(settings, na, n=65, sx=0, sy=0)` | complex ndarray |
| `radiometric_factor(pupil_na, image_index=1, reduction=4)` | (cos θ_obj / cos θ_img)^½ |
| `fresnel_coefficients(n1, n2_real, n2_imag, sin_theta1)` | dict of r_s, r_p, t_s, t_p, R/T |

`pupil_polarization_map` takes the same keyword set as `VectorSettings` plus `sx`, `sy` (the source point); `vector_two_beam_contrast` takes the `VectorSettings` keywords too. CLI · TOML: `[imaging.vector]` in [configuration](configuration.md) · GUI: Imaging settings ([gui.md](gui.md#imaging-settings)).

## LIGA, X-ray materials and multilayers

See [LIGA / deep X-ray](./processes/liga-deep-xray.md) and [Materials](./materials.md).

<!-- huv-table: stack -->

| Function / class | Grade | Notes |
|---|---|---|
| `simulate_liga(critical_energy_kev=None, resist_thickness_um=500, **kw)` → `LigaResult` | ✅ Implemented depth dose and exposure time (🔶 thin-screen absorber laterally) | Absolute exposure needs either a bending-magnet `source` with `source_distance_m` and `vertical_scan_mm`, or an absolute `flux_density` table. `warn=True` turns sampling and beamline warnings into `UserWarning`s. |
| `LigaResult` | | `depth_dose` → `(z_um, dose_kj_cm3)`, `dose_ratio`, `top_dose_kj_cm3`, `bottom_dose_kj_cm3`, `exceeds_damage_ceiling`, `exposure_time_s`, `top_dose_rate_kj_cm3_s`, `bottom_dose_rate_kj_cm3_s`, `resist_power_density_w_mm2`, `exposure_charge_ma_h`, `mean_energy_top_kev`, `mean_energy_bottom_kev`, `window_power_fraction`, `diffraction`, `spectrum`, `fresnel_scale_nm`, `sampling_resolved`, `warnings`, `volume`, `developed_depth`, `status`, `notes` |
| `liga_edge_profile(…)` → `LigaEdgeProfile` | ✅ Implemented (🔶 thin-screen absorber) | Analytic Fresnel knife edge: `x_nm`, `z_um`, `dose_kj_cm3`, `edge_positions_nm(threshold)`, `sidewall_angle_deg(threshold)`, `max_gradient_kj_cm3_nm()` |
| `MultilayerMirror.mo_si(periods=40, period_nm=6.9, gamma=0.4)` / `.la_b4c(…)` / `.la_b(…)` | ✅ Implemented | Parratt recursion with Névot–Croce roughness. `with_capping`, `with_roughness`, `reflectance`, `reflectance_curve`, `reflectance_vs_angle`, `amplitude`, `peak(λmin, λmax)` → `(λ, R)`, `bandwidth_fwhm_nm`, `angular_acceptance_fwhm_deg`, `pupil_response` |
| `tune_multilayer_period(family, target_nm, periods, gamma=0.4, angle_deg=0, polarization="te")` | ✅ Implemented | Period (nm) that puts the peak at `target_nm`. |
| `xray_optical_constants(formula, wavelengths_nm, density_g_cm3=None)` → `(delta, beta)` | ✅ Implemented | Henke/CXRO, n = 1 − δ + iβ. |
| `refractive_index(material, wavelength_nm)` → complex, `material_names()` | 🔶 Simplified | Database entries. Every entry has a data range and lookups never extrapolate: the VUV entries (e.g. `"Si"`, `"CaF2"`) cover the VUV only; `"Si"`, `"Cr"`, `"SiO2"` fall back to their Henke values at 0.0413–41.3 nm, everything else out of range raises `ValueError` naming the range and the alternative (`_euv`/`_beuv` entries or `"henke:<formula>[@density]"`). See [materials](materials.md#range-policy-no-silent-extrapolation). |

```text
liga_edge_profile(critical_energy_kev=None, resist_thickness_um=500, **kw)
```

<details markdown="1">
<summary>Full signature of `simulate_liga` — the 30 keywords by group</summary>

```text
simulate_liga(critical_energy_kev=None, resist_thickness_um=500, *,
    # geometry
    cd_nm=5000, pitch_nm=10000, grid_size=128, pixel_nm=200, nz=64, proximity_gap_um=100,
    # spectrum (one of: critical energy, a bending-magnet source, a flux-density table, a spectrum table)
    source=None, source_distance_m=None, horizontal_acceptance_mrad=5, vertical_scan_mm=None,
    flux_density=None, spectrum_table=None, energy_bins=100,
    # exposure and dose targets
    target_bottom_dose_kj_cm3=3, damage_dose_kj_cm3=20, diffraction="fresnel", photoelectron_blur=False,
    # mask stack
    filters=None, absorber="Au", absorber_thickness_um=20, membrane="Ti", membrane_thickness_um=2,
    # output and checks
    strict_sampling=False, warn=True) -> LigaResult
liga_edge_profile(critical_energy_kev=None, resist_thickness_um=500, *, proximity_gap_um=100,
    x_min_nm=-3000, x_max_nm=3000, dx_nm=10, depths_um=None, …) -> LigaEdgeProfile
```

`filters` is a list of `(material, thickness_um)` pairs.

</details>

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

???+ success "Output"

    ```text
    top/bottom 21.41, exposure 631 s, ✅/🔶
    100 0.1675
    Mo/Si peak 0.730 at 13.48 nm
    Si at 13.5 nm: n = 0.9990 + 0.00183i
    ```

The example uses 20 energy bins and a 16-pixel grid so that it runs fast. With the default 100 bins, the [LIGA page](./processes/liga-deep-xray.md) documents a top/bottom ratio of 20.8 and an exposure of 605 s for this geometry. The same call at full resolution gives:

<!-- liga-depth-dose -> python-api.md -->
<figure markdown="span">

![Log-scale depth-dose curves through 500 micrometres of PMMA for no filter and Be, Kapton and Al filters; unfiltered the top dose is about 62 kJ/cm3, far above the 20 kJ/cm3 damage ceiling, while the aluminium filter keeps it near 9 kJ/cm3 at the cost of a longer exposure.](assets/images/sim/liga-depth-dose-light.png#gh-light-mode-only)
![Log-scale depth-dose curves through 500 micrometres of PMMA for no filter and Be, Kapton and Al filters; unfiltered the top dose is about 62 kJ/cm3, far above the 20 kJ/cm3 damage ceiling, while the aluminium filter keeps it near 9 kJ/cm3 at the cost of a longer exposure.](assets/images/sim/liga-depth-dose-dark.png#gh-dark-mode-only)

<figcaption><strong>A harder beam flattens the depth dose.</strong> 500 µm PMMA exposed through 20 µm Au on 2 µm Ti, every curve scaled so the bottom receives 3 kJ/cm³: a filtered beam keeps the top below the 20 kJ/cm³ damage ceiling at the price of exposure time, which is absolute (bending-magnet flux at 200 mA, 15 m, 50 mm scan). <em>Model: LIGA / deep X-ray depth dose and exposure time ✅ (NIST μ and μ<sub>en</sub>, bending-magnet photons per ln E; scan-averaged collimated beam, no beamline mirrors).</em></figcaption>
</figure>

CLI: [`deep` mode `liga`](cli.md#deep), [`materials`](cli.md#materials) · TOML: `[deep]` in [configuration](configuration.md#deep) · GUI: LIGA depth dose tab ([gui.md](gui.md#views)).

## Volumetric, development and bake

See [Volumetric exposure](./processes/volumetric-exposure.md) and [Resist models](./processes/resist-models.md).

<!-- huv-table: stack -->

| Function | Grade | Returns / notes |
|---|---|---|
| `expose_volumetric(source, optics, mask, film_stack, resist, grid, **kw)` | 🔶 Simplified | `VolumetricResult`, the PAC latent image (nz, ny, nx). |
| `simulate_volumetric(source, optics, mask, film_stack, resist, grid, **kw)` | 🔶 Simplified chain | `VolumetricProcessResult` with `latent`, `baked`, `arrival_times`, `height_map`, `developed_cd_nm`, `dev_time_s`, `car`, `level_set`, and `notes` (over- or under-development, the same checks as the CLI; the default 30 mJ/cm² over-develops the default stack, see [Configuration → choosing the dose](configuration.md#mode-volumetric)). The defaults it applies are stated beside the CLI's and GUI's in [Reference → Defaults by front end](reference/index.md#defaults-by-front-end). |
| `develop_fast_marching(volume, resist, pixel_xy_nm, pixel_z_nm, **kw)` | ✅ Implemented | Arrival times (s). |
| `develop_level_set(volume, resist, dev_time_s, **kw)` | ✅ Implemented (🔶 first-order front) | `LevelSetResult`: `arrival_times`, `phi`, `steps`, `reinitializations`, `dissolved_thickness_nm`, `final_rate_factor`, `height_map()`, `developed(time_s=None)`. |
| `development_rate(volume, resist, surface_rate_ratio=1, inhibition_depth_nm=0)` | | Rate volume (nm/s). |
| `peb_gaussian(volume, lateral_nm, **kw)` | ✅ Implemented | Exact anisotropic Gaussian bake. |
| `peb_car(volume, **kw)` | 🔶 Simplified | `CarPebResult`: `protected`, `acid`, `quencher`, `neutralized_total`, `steps`, `time_step_s`. |
| `height_map_from_times(times, dev_time_s)` | | `HeightMapResult` (nm). |
| `simulate_grayscale(target_height_nm, thickness_nm, **kw)` → `GrayscaleResult`; `grayscale_height_map`, `grayscale_transmittance_for_target`, `blazed_grating`, `microlens_array` | 🔶 Simplified | Grayscale 2.5D lithography ([page](./processes/grayscale.md)). |

<details markdown="1">
<summary>Full signatures of the volumetric chain</summary>

```text
expose_volumetric(source, optics, mask, film_stack, resist, grid, dose_mj_cm2=30, nz=64, n_defocus_planes=8,
                  dose_steps=1, base_defocus_nm=0, resist_layer=0, max_kernels=20)
simulate_volumetric(source, optics, mask, film_stack, resist, grid, *, dose_mj_cm2=30, nz=32,
                  n_defocus_planes=4, dose_steps=1, base_defocus_nm=0, max_kernels=20,
                  peb="gaussian" | "none" | "car", peb_lateral_nm=None, peb_vertical_nm=None, car=None,
                  develop="fmm" | "level_set", dev_time_s=60, surface_rate_ratio=1.0, inhibition_depth_nm=0.0,
                  lateral="periodic", depletion="none", depletion_time_constant_s=None,
                  loading_capacity_nm=None, loading_length_nm=None)
develop_fast_marching(volume, resist, pixel_xy_nm, pixel_z_nm, surface_rate_ratio=1, inhibition_depth_nm=0,
                  lateral="reflecting")
develop_level_set(volume, resist, dev_time_s, *, surface_rate_ratio=1, inhibition_depth_nm=0, lateral="periodic",
                  depletion="none", …)
peb_gaussian(volume, lateral_nm, *, vertical_nm=None, vertical_scale=None, vertical_surface_ratio=None,
                  vertical_decay_nm=None, lateral="periodic")
peb_car(volume, *, peb_time_s=60, k_amp=0.1, k_quench=10, quencher=0.15, acid_diffusivity_nm2_s=2,
                  quencher_diffusivity_nm2_s=0.5, vertical_ratio=1, lateral="periodic", max_time_step_s=None)
simulate_grayscale(target_height_nm, thickness_nm, *, d_th, d_clear, exposure_dose_mj_cm2, …)
```

</details>

`VolumetricResult` provides `values` (a read-only (nz, ny, nx) view), `x_nm`, `y_nm`, `z_nm`, `shape` and `extent_nm`, `cd_at_z(threshold)` and `depth_map(threshold)`, and `VolumetricResult.from_array(values, x_range_nm, y_range_nm, z_range_nm)`, which wraps your own array. `HeightMapResult` provides `values`, a read-only (ny, nx) view in nm, plus its coordinates.

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

???+ success "Output"

    ```text
    (8, 32, 32) ✅/🔶 (32, 32)
    ```

The two development solvers at full resolution, on the same exposure map:

<!-- volumetric-levelset-vs-fmm -> python-api.md -->
<figure markdown="span">

![Two cross-sections of a developing resist trench over an exposure map: the level-set and fast-marching development fronts after 3 seconds coincide; with surface inhibition both show a narrowed neck a few nanometres below the top surface (T-top).](assets/images/sim/volumetric-levelset-vs-fmm-light.png#gh-light-mode-only)
![Two cross-sections of a developing resist trench over an exposure map: the level-set and fast-marching development fronts after 3 seconds coincide; with surface inhibition both show a narrowed neck a few nanometres below the top surface (T-top).](assets/images/sim/volumetric-levelset-vs-fmm-dark.png#gh-dark-mode-only)

<figcaption><strong>Two solvers, one front.</strong> 3-D development of an exposed trench (PAC m = 1 − 0.9·exp(−x²/2·20²), uniform in depth; default Mack resist): the moving-boundary level set (thin line) and the static-rate fast-marching arrival time (thick line) agree to within a grid cell after 3 s, and surface inhibition (top-surface rate × 0.1, 10 nm decay) necks the opening just below the surface — the T-top. <em>Model: 3D development (fast marching) ✅ · 3D development (level set) ✅ (🔶 first order; phenomenological inhibition constants).</em></figcaption>
</figure>

CLI: [`deep` modes `volumetric`, `grayscale`](cli.md#deep) · TOML: [`[deep]`](configuration.md#deep) · GUI: Volume viewer ([gui.md](gui.md#views)).

## Interference, Talbot and EUV-IL

<!-- huv-table: stack -->

| Function | Grade | Notes |
|---|---|---|
| `simulate_interference(preset="two_beam", **kw)` | ✅ Implemented (🔶 two-photon, scalar absorption) | Multi-beam lattice exposed into a PAC volume ([page](./processes/interference-volumetric.md)). |
| `simulate_talbot(wavelength_nm=13.5, period_nm=100, **kw)` → `TalbotResult` | 🔶 Simplified | `carpet` (nz, nx), `coherent_image`, `stationary_image`, `dtl_image`, `atl_image`, `talbot_length_nm`, `talbot_length_exact_nm`, `achromatic_distance_nm`, `efficiencies`, … ([page](./processes/talbot.md)). |
| `simulate_euv_il(grating_period_nm=100, wavelength_nm=13.5, **kw)` → `EuvIlResult` | 🔶 Simplified | Fringe period p/(2m): `fringe_period_nm`, `diffraction_angle_deg`, `visibility`, `beam_intensities`, `intensity`, `pac`. |

```text
simulate_interference(preset="two_beam", *, wavelength_nm=200, n_medium=1.6, half_angle_deg=30, nx=128, ny=128,
                      nz=32, x_span_nm=1000, y_span_nm=1000, z_span_nm=500, dose_scale=1, dill_c=0.02,
                      two_photon=False)
simulate_talbot(wavelength_nm=13.5, period_nm=100, *, grating="amplitude", duty_cycle=0.5, …, exposure="dtl", …)
simulate_euv_il(grating_period_nm=100, wavelength_nm=13.5, *, order=1, …)
```

<!-- verify-example -->
```python
import highuvlith as huv

talbot = huv.simulate_talbot(wavelength_nm=13.5, period_nm=100.0, nx=64, carpet_nz=32)
print(f"z_T = {talbot.talbot_length_nm:.1f} nm, {talbot.status}")

il = huv.simulate_euv_il(grating_period_nm=100.0, nx=32, ny=2, nz=4)
print(f"fringe period {il.fringe_period_nm:.1f} nm, visibility {il.visibility:.3f}")
```

???+ success "Output"

    ```text
    z_T = 1481.5 nm, 🔶
    fringe period 50.0 nm, visibility 1.000
    ```

The same call at full resolution gives:

<!-- talbot-carpet -> python-api.md -->
<figure markdown="span">

![A Talbot carpet: the intensity behind a 100 nm grating, 25 percent open, lit at 13.5 nm repeats itself at the Talbot length and shows shifted and frequency-doubled images in between; beside it, the displacement Talbot image averaged over the gap has half the grating period.](assets/images/sim/talbot-carpet-light.png#gh-light-mode-only)
![A Talbot carpet: the intensity behind a 100 nm grating, 25 percent open, lit at 13.5 nm repeats itself at the Talbot length and shows shifted and frequency-doubled images in between; beside it, the displacement Talbot image averaged over the gap has half the grating period.](assets/images/sim/talbot-carpet-dark.png#gh-dark-mode-only)

<figcaption><strong>Talbot self-imaging at 13.5 nm.</strong> (a) Intensity behind a 100 nm-period binary amplitude grating, 25 % open (±10 orders, exact angular-spectrum propagation): the grating re-images at the Talbot length (2p²/λ ≈ 1481 nm paraxially, 1475 nm exactly for the ±1 orders) with the half-shifted and doubled images in between; a 50 % open grating is uniform at z<sub>T</sub>/4 and has no doubled image. (b) Averaging over a scanned gap (displacement Talbot lithography) gives a stationary image with half the period. <em>Model: Talbot / DTL / ATL 🔶 (scalar thin mask, coherent normal illumination, wavelength-independent grating coefficients).</em></figcaption>
</figure>

CLI: [`deep` modes `interference`, `talbot`](cli.md#deep) · GUI: Volume viewer, Talbot dataset ([gui.md](gui.md#views)) · physics: [Interference](processes/interference-volumetric.md), [Talbot](processes/talbot.md).

## Stochastics

<!-- huv-table: stack -->

| Function | Grade | Notes |
|---|---|---|
| `compute_ler_lwr(aerial, dose_mj_cm2, threshold=0.3, **kw)` → `LerResult` | 🔶 Simplified | Poisson photon noise on one image row plus Gamma dose jitter. Fields: `ler_3sigma_nm`, `lwr_3sigma_nm`, `cd_mean_nm`, `cd_sigma_nm`, edges, `measured_realizations`. Keywords: `source`, `photon_density_per_mj_cm2`, `dose_jitter_rms`, `pulses_per_exposure`, `absorbed_fraction`, `num_realizations=100`. |
| `photons_per_square(dose_mj_cm2, wavelength_nm, side_nm)`, `relative_shot_noise(…)` | ✅ Implemented | D·a² / (hc/λ) and 1/√N. |
| `resist_absorbed_fraction(thickness_nm, absorption_per_um)` | ✅ Implemented | 1 − exp(−α d). |

<!-- verify-example -->
```python
import highuvlith as huv

aerial = huv.simulate_line_space(100.0, 300.0, grid_size=128, pixel_nm=2.0).aerial
ler = huv.compute_ler_lwr(aerial, 30.0, source=huv.SourceConfig.f2_laser(), num_realizations=20)
print(ler.measured_realizations, ler.status)
print(round(huv.photons_per_square(30.0, 13.5, 16.0)), round(huv.relative_shot_noise(30.0, 13.5, 16.0), 4))
```

???+ success "Output"

    ```text
    20 🔶
    5219 0.0138
    ```

Physics: [The stochastic frontier](future/stochastic-frontier.md), [Research modules](research-modules.md).

## Quantum (N-photon) research module

Everything here is 🧪 Theoretical. No N-photon resist has recorded an entangled sub-Rayleigh pattern. The module keeps two physically different models apart:

- **Classical N-photon absorption** $E = I^N$ (`n_photon_absorption_image(classical, num_photons=2)`). It sharpens lines and raises contrast but **keeps the classical period**, so it gives no resolution gain. The old `quantum_aerial_image(classical, n=2, …)` is a deprecated alias whose `fidelity` argument has no effect.
- **Ideal entangled N00N models** (Boto et al., PRL 85, 2733, 2000), in two forms: `TwoBeamNPhoton(wavelength_nm, half_angle_deg, num_photons=2, fidelity=1.0, phase_rad=0.0)` gives two-beam fringes in closed form, at period $\lambda/(2N \sin\theta)$ instead of $\lambda/(2 \sin\theta)$ (methods `classical_period_nm()`, `noon_period_nm()`, `fringe_wavenumber_per_nm()`, `exposure(x)`, `noon_exposure(x)`, `classical_exposure(x)`, `profile(x_array)`, `harmonic_amplitude(j)`, `flux_budget()`); `SimulationEngine.compute_noon_ideal_image(num_photons, fidelity, focus_nm, mask)` → `NoonImageResult` handles general masks by imaging the same system at $\lambda/N$ (`classical`, `n_photon_absorption`, `noon_limit` and `exposure`, each an `AerialImageResult`, plus `effective_wavelength_nm`, `classical_resolution_nm`, `noon_limit_resolution_nm` and `flux`; it needs clear-field normalization and a pixel no larger than $\lambda/(4 N \mathrm{NA})$).

The fidelity mixture for general masks is

```math
E = F \, I_{\lambda/N} + (1 - F) \, I_{\lambda}^{N}
```

where the non-N00N fraction is classical N-photon absorption, not linear absorption. `quantum_flux_budget(wavelength_nm=157.63, num_photons=2)` returns a dict derived from the entangled-source constants: `photon_energy_ev`, `hvm_photon_rate_per_s`, `entangled_rate_ceiling_per_s`, `relative_flux`, and the exposure-time ratios `exposure_time_ratio_bound` (a **lower** bound), `exposure_time_ratio_tightest_bound` and `exposure_time_ratio_claimed`. `simulate_quantum_line_space(cd_nm, pitch_nm, *, n=2, fidelity=1.0, model="n_photon_absorption" | "noon", …)` → `QuantumResult` returns `classical`, `quantum`, the two contrasts, `model`, the resolution figures, `exposure_time_ratio_bound`, `status` and `notes`.

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

???+ success "Output"

    ```text
    157.63000000000002 78.81500000000001
    4.47e+10
    classical 0.000, N00N limit 0.731, 🧪
    ```

A 100 nm pitch lies below the classical cutoff $\lambda/(\mathrm{NA}(1+\sigma)) \approx 140$ nm, so the classical image (and $I^2$) has zero contrast. The ideal $\lambda/2$ limit resolves it, at an exposure time about $4.5 \times 10^{10}$ times the classical one (a lower bound: the loosest ETPA cross-section bound). Physics: [Quantum and exotic lithography](future/quantum-and-exotic.md), [Entangled photons](sources/entangled-photon.md).

## MNSL

MNSL is 🧪 Theoretical: the lattice geometry and the moiré period are exact, but the emission map is a heuristic. The native classes are exported as `MnslConfig`, `MnslEngine`, `MnslResult`, `NanosphereArrayConfig`, `SpherePacking`, `SubstrateCoupling` and `py_simulate_moire_emission`. The Python helpers are `simulate_moire_emission(sphere_diameter_nm, array_pitch_nm, rotation_angle_deg, *, separation_nm=100, wavelength_nm=157, grid_size=256, pixel_nm=2, …)`, which returns an `MnslSimResult` (the native result as `.result`, its arrays as NumPy, `coordinates`, `cross_section_x/y`, `num_peaks`, and the native `status` (🧪) and `notes` forwarded), plus `create_nanosphere_array`, `sweep_rotation_angle`, `sweep_separation` and `optimize_moire_parameters`. `MnslResult` fields: `emission_pattern`, `moire_pattern`, `enhancement_factors`, `moire_period_nm`, `peak_enhancement`, `peak_positions`, `status`, `notes`. Physics: [Research modules](research-modules.md).

## Visualization

`highuvlith.viz` needs matplotlib and is imported separately (`from highuvlith import viz`). Every helper takes `theme="light"|"dark"` (default: the current theme) and `ax=` (or `fig=` for multi-panel figures), returns the Axes or Figure without showing it, and uses one validated palette (`viz.use_theme(...)`, `viz.THEMES`, `categorical`, `ordinal_colors`, `sequential_cmap`, `diverging_cmap`). `viz.export_light_dark(plot_fn, stem, *args, formats=("png",), dpi=200, **kwargs)` renders a helper in both themes for documentation pages (`<stem>-light.<ext>`, `<stem>-dark.<ext>`).

<!-- huv-table: stack -->

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

???+ success "Output"

    ```text
    ['aerial-light.svg', 'aerial-dark.svg']
    log log
    ```

**Where next:** [Python API](python-api.md) — the core classes and the canonical run · [CLI](cli.md) · [GUI](gui.md) · [Research modules](research-modules.md) · [Reference](reference/index.md)
