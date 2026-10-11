---
description: "The highuvlith command line: seven subcommands over TOML configs, what each writes and where, the recorded transcripts, and the same run in Python and the GUI."
---

# CLI

**Status:** ✅ Implemented — all seven subcommands run from [`crates/highuvlith-cli`](../crates/highuvlith-cli/src) and every bundled config is executed end to end by `crates/highuvlith-cli/tests/examples.rs`; the physics behind each command keeps its own grade · [matrix](capability-matrix.md#frontends-and-infrastructure)

<p class="huv-lead">The <code>highuvlith</code> binary is a <a href="https://docs.rs/clap">clap</a> front end over <code>highuvlith-core</code>: config-driven commands read a TOML file, the two lookup commands take flags only, and nothing in the binary computes physics of its own. It is for batch runs and scripts; interactive exploration is the GUI's job and arrays are the Python API's.</p>

**Same thing elsewhere:** Python [`simulate_line_space`](python-api.md#quick-start) · TOML [`examples/sim.toml`](../examples/sim.toml) · GUI [`--preset f2 --cd 65 --pitch 180`](gui.md) · physics [Simulation pipeline](pipeline.md)

<!-- cli-output-gallery -> cli.md -->
<figure markdown="span">

![Five PNGs the highuvlith command line wrote, pixel for pixel: the aerial image of simulate examples/sim.toml (contrast 0.4545), the LIGA developed-depth map, the volumetric x–z developed profile, the Talbot wafer-plane image (contrast 0.457) and the ILT mask of optimize examples/optimize_ilt.toml.](assets/images/sim/cli-output-gallery-light.png#gh-light-mode-only)
![Five PNGs the highuvlith command line wrote, pixel for pixel: the aerial image of simulate examples/sim.toml (contrast 0.4545), the LIGA developed-depth map, the volumetric x–z developed profile, the Talbot wafer-plane image (contrast 0.457) and the ILT mask of optimize examples/optimize_ilt.toml.](assets/images/sim/cli-output-gallery-dark.png#gh-dark-mode-only)

<figcaption><strong>What the CLI writes.</strong> Its own PNGs for five bundled configs, untouched (its own colormap and min–max normalisation); each panel names the command, its config, what the image is and the size the binary wrote. <code>simulate</code> saves the aerial image, <code>deep</code> the mode's characteristic map (LIGA developed depth, volumetric x–z profile, Talbot wafer-plane image) and <code>optimize</code> the optimized mask; the headline number under each panel is the one the command prints. <em>Grades per command as in the At-a-glance table below; the CLI's own files.</em></figcaption>
</figure>

## Find by task

| I want to … | Run | Section |
|---|---|---|
| image one condition and read contrast, CD and NILS | `highuvlith simulate --config examples/sim.toml` | [`simulate`](#simulate) |
| sweep dose × focus into a process window | `highuvlith sweep --config examples/sim_arf.toml --dose-range 20,40,5` | [`sweep`](#sweep) |
| run a deep-layer process (LIGA, grayscale, interference, volumetric, Talbot) | `highuvlith deep --config examples/liga.toml` | [`deep`](#deep) |
| optimize a mask or lay out spacers | `highuvlith optimize --config examples/optimize_ilt.toml` | [`optimize`](#optimize) |
| estimate wafers per hour for a source | `highuvlith throughput --config examples/sim_lpp_sn.toml` | [`throughput`](#throughput) |
| list every source preset with its grade | `highuvlith sources` | [`sources`](#sources) |
| look up n, k or a multilayer mirror | `highuvlith materials --name CaF2 --wavelength 157.63` | [`materials`](#materials) |
| know which file holds which result | — | [Where output goes](#where-output-goes) |
| see every TOML key | — | [TOML configuration](configuration.md) |

## At a glance

<!-- huv-table: stack -->

| Command | Input | What it computes | Physics grade | Python equivalent | GUI |
|---|---|---|---|---|---|
| [`simulate`](#simulate) | config | aerial image, contrast, printed CD and NILS at one dose/focus | ✅ Implemented imaging; source/optics grades per family | [`simulate_line_space`](python-api.md#high-level-functions), [`compute_aerial_image`](python-api.md#imaging-engine) | Aerial image tab |
| [`sweep`](#sweep) | config | dose × focus matrix, Bossung data, exposure-defocus (ED) window | 🔶 Simplified constant-threshold resist | [`process_window`](python-api.md#process-window-and-ed-analysis), `BatchSimulator` | Process window tab |
| [`deep`](#deep) | config with `[deep]` | LIGA, grayscale, interference, volumetric, Talbot | grade per mode (table below) | [`simulate_liga`](python-api-modules.md#liga-x-ray-materials-and-multilayers), [`simulate_grayscale`, `simulate_volumetric`](python-api-modules.md#volumetric-development-and-bake), [`simulate_interference`, `simulate_talbot`](python-api-modules.md#interference-talbot-and-euv-il) | Volume viewer, LIGA depth dose tabs |
| [`optimize`](#optimize) | config with `[optimize]` | ILT, fragment OPC, SRAF insertion, SADP / SAQP geometry | ILT ✅, OPC ✅, SRAF 🔶, SADP/SAQP 🔶 | [`optimize_ilt`, `fragment_opc`, `insert_srafs`, `sadp`, `saqp`](python-api.md#optimization-and-patterning) | not in the GUI |
| [`throughput`](#throughput) | config | dose-limited wafers per hour, photons per feature | 🔶 Simplified illustrative scanner presets | [`wafer_throughput`](python-api-modules.md#sources-landscape-and-throughput) | not in the GUI |
| [`sources`](#sources) | flags | every source family / preset with derived physics | grade per preset | [`source_landscape()`](python-api-modules.md#sources-landscape-and-throughput) | Source tab |
| [`materials`](#materials) | flags | n, k tables, Henke δ/β, EUV/BEUV multilayer mirrors | ✅ CXRO/NIST data, 🔶 database entries | [`refractive_index`, `xray_optical_constants`, `MultilayerMirror`](python-api-modules.md#liga-x-ray-materials-and-multilayers) | not in the GUI |

## Running

Build once, then call the binary (the same idiom as the [README](../README.md) and [Getting started](getting-started.md)):

```bash
cargo build --release -p highuvlith-cli
B=target/release/highuvlith

$B simulate --config examples/sim.toml --output aerial.png   # F2, 65/180 nm, NA 0.75
```

`cargo run -p highuvlith-cli --release -- simulate --config examples/sim.toml` does the same through cargo (arguments after `--` go to the CLI). `highuvlith --help`, `highuvlith <command> --help` and `highuvlith --version` print usage and the crate version; `--help` adds a paragraph per command (the source families, `deep` modes, `optimize` methods and `throughput` presets, kept in step with the code by a unit test), `-h` prints the one-line summaries. The examples below assume the binary is on your `PATH` and the working directory is the repository root.

## Where output goes

Every config-driven command prints a human-readable summary to **stderr** (even without `--output`, so `2> run.log` captures it); the machine-readable result goes to the `--output` file, and the two lookup commands print to **stdout**. Exit status is 0 on success and 1 on any error.

| Command | Writes | Where | Flag |
|---|---|---|---|
| `simulate`, `deep`, `optimize` | summary; an image or pretty-printed JSON | stderr; the `--output` file (`.png` → image, any other extension → JSON) | `--output` |
| `sweep` | summary; JSON | stderr; the `--output` file | `--output` |
| `throughput` | summary; JSON | stderr; the `--output` file, or stdout | `--output`, `--json` |
| `sources` | a table, or JSON | stdout | `--family`, `--json` |
| `materials` | text, or JSON | stdout | `--json` |

**Strict configs.** An unknown key, or a key the selected `type` / `mode` / `method` does not read, is an error that names the key, suggests a spelling, and lists the valid keys (details in [Configuration → Strict keys](./configuration.md#strict-keys)):

```text
$ highuvlith simulate --config bad.toml
Error: bad.toml: TOML parse error at line 5, column 1
  |
5 | conversion_efficency = 0.05
  | ^^^^^^^^^^^^^^^^^^^^
unknown key `conversion_efficency` in [source] — did you mean `conversion_efficiency`?
keys read by [source] type = "lpp": type, fuel, preset, drive_laser, wavelength_nm, ...

$ highuvlith simulate --config bad2.toml       # an X-ray-tube key in an LPP config
Error: [source] type = "lpp" does not use 1 of the given keys:
  - `anode` is only read by type = "xray_tube"
keys read by [source] type = "lpp": type, fuel, preset, drive_laser, ...
```

**Notes and warnings** (stderr, the run continues): the commensurate auto-grid (`note: pixel 2 nm -> 1.875000 nm so the 240.000 nm field holds 4 whole periods ...`; turn off with `[grid] commensurate = false`), tables the command ignores (`note: [optics] is not read by deep mode "grayscale"`; `simulate` / `sweep` likewise note a `[deep]`, `[optimize]` or `[throughput]` table), dark-field imaging, a pixel too coarse for the pupil band, and LIGA pixels coarser than the Fresnel scale.

---

## `simulate`

Aerial image, contrast, printed CD and NILS for one dose/focus condition — ✅ Implemented Hopkins imaging; the source and optics keep their own grades.

```text
highuvlith simulate --config <FILE> [--output <FILE>] [--focus <NM>] [--dose <MJ_CM2>]
```

| Flag | Short | Default | Meaning |
|---|---|---|---|
| `--config` | `-c` | required | TOML config (`[source]`, `[optics]`, `[mask]`; optional `[grid]`, `[process]`, `[imaging]`, `[illumination]`) |
| `--output` | `-o` | none | `.png` → aerial image (Inferno colormap); otherwise JSON |
| `--focus` | | `[process] focus_nm` | defocus in nm (negative values allowed) |
| `--dose` | | `[process] dose_mj_cm2` | exposure dose in mJ/cm²; moves the printing threshold to `I_th = threshold · dose_mj_cm2 / dose` (constant-threshold resist) |

The image is computed with the Hopkins/SOCS engine using the `[imaging]` settings (exact defocus by default; `spectrum = "narrow_band"` or `"per_wavelength"` for polychromatic imaging; `[imaging.vector]` for polarized imaging). Intensities are relative to the clear-field intensity of the same optics and illumination (clear mask = 1). If the pupil blocks the zero order for almost the whole source (clear field below 1 % of the incident light) the run warns `dark-field imaging` and reports absolute intensities.

**stderr** (`highuvlith simulate --config examples/sim.toml`, abridged — the canonical run; Python prints the same numbers in the [quick start](python-api.md#quick-start)):

```text
note: pixel 1 nm -> 0.703125 nm so the 180.000 nm field holds 1 whole period of the 180 nm mask cell ...
Source:  λ = 157.63 nm, σ = 0.70 (vuv)
Pupil:   Conventional { sigma: 0.7 }
Derived source physics:
  photon_energy                          7.8655e0 eV
  ...
Optics:  refractive NA = 0.750
Mask:    line/space CD = 65.0 nm, pitch = 180.0 nm (vertical) — ...
Grid:    256×256, pixel = 0.7031 nm (field 180.0 nm)
Focus:   0.0 nm
Dose:    30.00 mJ/cm² (nominal 30.00) → printing threshold I = 0.3000
Engine:  9 SOCS kernels (1.0000 of the TCC trace), 1176 source points
Spectrum: monochromatic
Results:
  Contrast:  0.4545
  I_max:     0.6631
  I_min:     0.2487
  Printed CD (dark feature at I = 0.3000): 41.20 nm
  NILS:      0.654
```

<details markdown="1">
<summary>JSON keys of `simulate` — 33 keys, with the recorded sample for `examples/sim.toml`</summary>

| Key | Meaning |
|---|---|
| `wavelength_nm`, `source_type`, `optics_type`, `multilayer_pupil`, `na`, `immersion_index` | the source and optics the run used (`multilayer_pupil` is a bool) |
| `pattern`, `mask`, `cd_nm`, `pitch_nm`, `tone` | the mask (`tone` is the feature tone the metrics use) |
| `grid_size`, `pixel_nm`, `field_nm` | the grid actually used (after the commensurate note) |
| `focus_nm`, `dose_mj_cm2`, `nominal_dose_mj_cm2`, `threshold` | the condition; `threshold` is the printing intensity after the dose scaling |
| `spectrum`, `num_kernels`, `captured_energy_fraction`, `num_source_points` | engine diagnostics |
| `clear_field_intensity`, `normalized`, `dark_field_imaging` | the normalisation the intensities use |
| `contrast`, `i_max`, `i_min`, `printed_cd_nm` (null if nothing prints), `nils`, `ils_per_nm` | the results |
| `derived_quantities` | the source's machine-derived physics, `{name, value, unit, note}` records |
| `compute_ms` | wall time of the image |

Recorded sample (`--output sim.json`, `derived_quantities` and `compute_ms` elided):

```json
{
 "captured_energy_fraction": 1.0000000000000187,
 "cd_nm": 65.0,
 "clear_field_intensity": 1.0000000000000084,
 "contrast": 0.45449527716901045,
 "dark_field_imaging": false,
 "dose_mj_cm2": 30.0,
 "field_nm": 180.0,
 "focus_nm": 0.0,
 "grid_size": 256,
 "i_max": 0.663134157247169,
 "i_min": 0.24870676469501157,
 "ils_per_nm": 0.015882725335254613,
 "immersion_index": 1.0,
 "mask": "line/space 65 nm / 180 nm (Vertical), bright field, binary",
 "multilayer_pupil": false,
 "na": 0.75,
 "nils": 0.654381310851592,
 "nominal_dose_mj_cm2": 30.0,
 "normalized": true,
 "num_kernels": 9,
 "num_source_points": 1176,
 "optics_type": "refractive",
 "pattern": "line_space",
 "pitch_nm": 180.0,
 "pixel_nm": 0.703125,
 "printed_cd_nm": 41.200820201749195,
 "source_type": "vuv",
 "spectrum": "monochromatic",
 "threshold": 0.3,
 "tone": "dark",
 "wavelength_nm": 157.63
}
```

</details>

```bash
highuvlith simulate --config examples/sim.toml --output aerial.png
highuvlith simulate --config examples/sim_lpp_sn.toml --output lpp.json
highuvlith simulate --config examples/sim.toml --dose 36 --focus 50   # summary only
```

See also: Python [`simulate_line_space`](python-api.md#high-level-functions) · TOML [`[imaging]`](configuration.md) · physics [Simulation pipeline](pipeline.md)

---

## `sweep`

Dose × focus matrix with a constant-threshold resist (🔶 Simplified): CD at every point, Bossung data, and the exposure-defocus process window.

```text
highuvlith sweep --config <FILE> [--output <FILE>] [--focus-range <START,STOP,STEPS>] [--dose-range <START,STOP,STEPS>]
```

| Flag | Short | Default | Meaning |
|---|---|---|---|
| `--config` | `-c` | required | same tables as `simulate`; `[process]` sets the CD target and tolerance |
| `--output` | `-o` | none | JSON file |
| `--focus-range` | | `-200,200,11` | focus start, stop (nm) and number of steps (negative starts allowed: `--focus-range -150,150,7`) |
| `--dose-range` | | the `[process]` dose only | dose start, stop (mJ/cm²) and steps |

One aerial image is computed per focus (through `compute_through_focus`); every dose re-thresholds it at `I_th = threshold · dose_nominal / dose`. The process window uses `[process] cd_target_nm` (default: the drawn CD) and `cd_tolerance_pct` (default 10).

**stderr** (`highuvlith sweep --config examples/sim_arf.toml --focus-range -150,150,7 --dose-range 20,40,5`):

```text
Sweep: 7 focuses × 5 doses (7 aerial images, spectrum monochromatic; constant-threshold resist, E_th = 9.000 mJ/cm² = 0.3 × 30 mJ/cm²)
Engine: 20 SOCS kernels
Process window (target CD 80.0 nm ± 10%, dark feature):
  Best focus:     -14.1 nm
  Dose-to-size:   31.38 mJ/cm²
  DOF (at size):  300.0 nm
  EL (best focus): 12.5 %
  DOF @ 5% EL:    258.3 nm (focus -128.1…130.1 nm, dose 30.59…32.16 mJ/cm²)
```

DOF values are bounded by the swept focus range (here ±150 nm).

<details markdown="1">
<summary>JSON keys of `sweep` — five top-level keys, with one recorded point</summary>

| Key | Meaning |
|---|---|
| `config` | wavelength, NA, mask, grid, spectrum |
| `focuses`, `doses` | the swept axes |
| `results` | one record per point: `focus_nm`, `dose_mj_cm2`, `intensity_threshold`, `cd_nm` (null where nothing prints), `contrast` |
| `process_window` | `best_focus_nm`, `dose_to_size_mj_cm2`, `depth_of_focus_nm`, `exposure_latitude_pct`, `iso_focal_dose_mj_cm2`, `dose_to_clear_mj_cm2`, `nominal_dose_mj_cm2`, `intensity_threshold_at_nominal`, `dose_limits`, `el_vs_dof`, `dof_at_5pct_el`, `cd_target_nm`, `cd_tolerance_pct`, `tone`, `resist_model` |

Recorded sample, the first `results` record of `highuvlith sweep --config examples/sim.toml` (the default ±200 nm focus range at the `[process]` dose; −200 nm is outside the window, so no line prints):

```json
{"cd_nm": null, "contrast": 0.17386977012651014, "dose_mj_cm2": 30.0, "focus_nm": -200.0, "intensity_threshold": 0.3}
```

</details>

```bash
highuvlith sweep --config examples/sim.toml --output pw.json                       # focus only
highuvlith sweep --config examples/sim_arf.toml --focus-range -150,150,7 --dose-range 20,40,5 --output ed.json
```

See also: Python [`process_window`](python-api.md#process-window-and-ed-analysis) · TOML [`[process]`](configuration.md) · physics [Metrics and process window](metrics-and-process-window.md)

---

## `deep`

Deep-layer / 3D process modes, selected by `[deep] mode`. Each mode reads only the tables it needs and notes the ones it ignores; keys are documented per mode in [Configuration → `[deep]`](./configuration.md#deep).

```text
highuvlith deep --config <FILE> [--output <FILE>]
```

<!-- huv-table: stack -->

| `mode` | Grade | Example | PNG (`--output x.png`) | Model page |
|---|---|---|---|---|
| `liga` | ✅ Implemented depth dose (🔶 thin-screen absorber) | `examples/liga.toml`, `sim_xray_tube.toml`, `sim_betatron.toml` | developed-depth map (Viridis) | [LIGA](./processes/liga-deep-xray.md) |
| `grayscale` | 🔶 Simplified | `examples/grayscale.toml` | achieved height map | [Grayscale](./processes/grayscale.md) |
| `interference` | ✅ Implemented (🔶 two-photon, scalar absorption) | `examples/interference.toml` | mid-depth PAC slice | [Interference](./processes/interference-volumetric.md) |
| `volumetric` | 🔶 Simplified chain | `examples/volumetric.toml` | x–z developed profile (or the mid-depth latent slice with `develop = "threshold"`) | [Volumetric](./processes/volumetric-exposure.md) |
| `talbot` | 🔶 Simplified | `examples/talbot.toml` | carpet (x–z) or printed image | [Talbot](./processes/talbot.md) |

The LIGA spectrum comes from one source, in this precedence: `[deep] flux_density` → `[deep] critical_energy_kev` → the `[source]` table (absolute, with an exposure time, when `source_distance_m` is given) — the full rule is on the [LIGA page](./processes/liga-deep-xray.md). The volumetric defaults the CLI applies when `[deep]` omits them (bake, resist thickness) are stated once, beside the Python and GUI defaults, in [Reference → Defaults by front end](reference/index.md#defaults-by-front-end); `examples/volumetric.toml` sets its own thickness and bake, so its output does not depend on them.

**stderr** (`highuvlith deep --config examples/talbot.toml`):

```text
Deep mode: Talbot
Talbot (atl):
  Wavelength:        13.506 nm
  Grating:           100.0 nm period, amplitude (duty 0.50), orders |n| ≤ 10
  Talbot length:     1480.8 nm = 2p²/λ (paraxial)
  Talbot (exact):    1474.0 nm = λ/(1 − √(1 − λ²/p²))
  Achromatic dist.:  66.67 µm = 2p²/Δλ (Δλ = 0.3000 nm)
  Gap:               150.00 µm
  Printed period:    50.00 nm (mask 100.0 nm)
  Image contrast:    0.457
  Fill fraction:     0.445  (m < 0.50, 50 nm resist)
```

<details markdown="1">
<summary>JSON keys of `deep` — per mode</summary>

| Mode | Keys (recorded from the bundled example of each mode; every mode also echoes `mode` and its own inputs) |
|---|---|
| `liga` | `top_dose_kj_cm3`, `bottom_dose_kj_cm3`, `dose_ratio`, `top_dose_rate_kj_cm3_s`, `bottom_dose_rate_kj_cm3_s`, `exposure_time_s`, `exposure_charge_ma_h`, `resist_power_density_w_mm2`, `mean_energy_top_kev`, `mean_energy_bottom_kev`, `fresnel_scale_top_nm`, `fresnel_scale_bottom_nm`, `sampling_resolved`, `proximity_model`, `edge_profile`, `developed_depth_min_um`, `developed_depth_max_um`, `max_aspect_ratio`, `min_feature_nm`, `exceeds_damage_ceiling`, `window_power_fraction`, `warnings` (+ `critical_energy_kev`, `damage_dose_kj_cm3`, `develop_threshold_kj_cm3`, `target_bottom_dose_kj_cm3`, `proximity_gap_um`, `resist_thickness_um`) |
| `grayscale` | `achieved_height_min_nm`, `achieved_height_max_nm`, `rms_error_nm` (+ `target`, `target_height_min_nm`, `target_height_max_nm`, `thickness_nm`, `d_th`, `d_clear`, `gamma`, `dose_mj_cm2`) |
| `interference` | `fringe_period_nm`, `pac_min`, `pac_max`, `fill_fraction`, `fill_threshold` (+ `preset`, `wavelength_nm`, `n_medium`, `half_angle_deg`, `dose_scale`, `dill_c`, `two_photon`) |
| `volumetric` | `cd_top_nm`, `cd_mid_nm`, `cd_bottom_nm`, `sidewall_angle_deg`, `developed`, `develop_threshold`, `car_neutralized_total` (+ `source_type`, `wavelength_nm`, `dose_mj_cm2`, `dose_steps`, `n_defocus_planes`, `nz`, `resist_thickness_nm`, `peb`, `develop`, `dev_time_s`) |
| `talbot` | `talbot_length_nm`, `talbot_length_exact_nm`, `achromatic_distance_nm`, `printed_period_nm`, `image_contrast`, `fill_fraction` (+ `grating_period_nm`, `grating_type`, `duty_cycle`, `max_order`, `propagation`, `talbot_mode`, `gap_nm`, `bandwidth_nm`, `wavelength_nm`) |

The LIGA examples use a 1.875 µm pixel and warn that it under-resolves the Fresnel scale (~0.16 µm at the resist top); the lateral dose is then the pixel-averaged shadow, and the printed edge profile (`Edge profile` lines) comes from the separate 1D Fresnel edge calculation.

</details>

```bash
highuvlith deep --config examples/liga.toml --output liga.png
highuvlith deep --config examples/sim_xray_tube.toml --output tube.json
highuvlith deep --config examples/grayscale.toml --output gray.json
highuvlith deep --config examples/interference.toml --output fringes.png
highuvlith deep --config examples/volumetric.toml --output vol.png      # ~5 s release
highuvlith deep --config examples/talbot.toml --output talbot.json
```

See also: Python [`simulate_liga`](python-api-modules.md#liga-x-ray-materials-and-multilayers), [`simulate_volumetric`](python-api-modules.md#volumetric-development-and-bake), [`simulate_talbot`](python-api-modules.md#interference-talbot-and-euv-il) · GUI [Volume viewer and LIGA depth dose](gui.md#views) · physics [Processes](processes/index.md)

---

## `optimize`

Mask optimization and spacer patterning, selected by `[optimize] method` (keys in [Configuration → `[optimize]`](./configuration.md#optimize)).

```text
highuvlith optimize --config <FILE> [--output <FILE>]
```

<!-- huv-table: stack -->

| `method` | Grade | Reads | Result |
|---|---|---|---|
| `ilt` | ✅ Implemented | imaging tables; target = the drawn `[mask]` clear area | true-adjoint (or proxy) gradient ILT: cost history, misprinted pixels drawn → optimized → binarized |
| `opc` | ✅ Implemented | imaging tables | fragment model-based OPC: edge-placement error rms/max before → after, per-fragment biases |
| `sraf` | 🔶 Simplified | imaging tables | heuristic λ/NA rule deck + model print check; optional DOF comparison (`compare_dof = true`) |
| `sadp`, `saqp` | 🔶 Simplified | `[optimize]` only | geometric conformal-spacer line pattern: line CDs, spaces, pitches, pitch walk (no imaging, etch bias or LER) |

Imaging methods use the centre wavelength (`[imaging] spectrum` must be monochromatic). `--output x.png` writes the optimized mask (for SADP/SAQP one mandrel period of the line pattern); JSON holds metrics, histories and the mask / fragments / assists / line pattern.

**stderr** of the four examples (abridged):

```text
$ highuvlith optimize --config examples/optimize_ilt.toml
  Cost:            1.0754e-1 -> 1.0555e-2 (40 iterations, max_iterations)
  Misprinted px:   drawn mask 484 -> optimized 96 (binary 184) of 4096 at I = 0.2500

$ highuvlith optimize --config examples/optimize_opc.toml
  EPE rms:   9.417 -> 0.633 nm   max |EPE|: 26.511 -> 1.731 nm
  20 fragments on 1 polygon(s); 6 iterations; converged (tolerance rms 1.051 / max 2.102 nm)

$ highuvlith optimize --config examples/optimize_sraf.toml
  Assists:   3 kept (3 placed, 0 removed, 0 shrink steps); worst no-print margin 0.283
  DOF (±10% CD, each mask at its own dose-to-size): 210.3 nm -> 275.2 nm (×1.31)

$ highuvlith optimize --config examples/optimize_sadp.toml
  2 lines per mandrel period; nominal pitch 64.00 nm
  Line CDs:  32.00 / 32.00 nm
  Spaces:    34.00 / 30.00 nm
  Pitches:   66.00 / 62.00 nm
  Pitch walk 4.000 nm, CD range 0.000 nm
```

See also: Python [Optimization and patterning](python-api.md#optimization-and-patterning) · physics [Research modules](research-modules.md)

---

## `throughput`

Dose-limited wafers per hour for the configured `[source]` — 🔶 Simplified: the scanner presets are illustrative assumptions, not vendor data.

```text
highuvlith throughput --config <FILE> [--output <FILE>] [--dose <MJ_CM2>] [--json]
```

| Flag | Short | Default | Meaning |
|---|---|---|---|
| `--config` | `-c` | required | `[source]` (required), optional `[throughput]` and `[process]` |
| `--output` | `-o` | none | JSON file |
| `--dose` | | `[throughput] dose_mj_cm2`, else `[process] dose_mj_cm2` | resist dose-to-size, mJ/cm² |
| `--json` | | off | print the JSON on stdout |

The model: power at the wafer = source power × optics transmission × mask factor; scan speed and exposure time follow from the dose over the exposed area (fields of 26 × 33 mm on a 300 mm wafer), plus per-field and per-wafer overheads; it also reports pulses per point and photons per feature (shot noise). The scanner preset (`euv_hvm`, `beuv_la_b`, `refractive`) is picked from the wavelength (≥ 100 nm, 12.4–15 nm, 6–7.5 nm) unless `[throughput] preset` is set; a wavelength outside all three bands (X-ray tubes, the betatron, the 25–47 nm lines) is an error unless the preset or the optics train is given ([details](configuration.md#throughput)); every preset number can be overridden in `[throughput]`.

```text
$ highuvlith throughput --config examples/sim_lpp_sn.toml
Throughput — dose-limited scanner model (🔶 simplified; preset scanner numbers are illustrative assumptions):
  Source:            lpp, 13.5000 nm, 250.0 W usable power into the illuminator
  Scanner preset:    euv_hvm (chosen from the wavelength)
  Optics × mask:     0.0282 × 0.6500 = 1.836 % of the source power
  Power at wafer:    4.590 W
  Dose:              30.00 mJ/cm² ([process] dose_mj_cm2)
  Fields / wafer:    84 (26 × 33 mm on a 300 mm wafer; 720.7 cm² exposed)
  Scan speed:        588.5 mm/s (dose-limited)
  Time per wafer:    4.996 s scanning, 23.40 s total (0.1 s/field, 10 s/wafer overheads)
  Wafers per hour:   153.9
  Pulses per point:  169.9
  Photons per (30 nm)²: 18349 (relative shot noise 0.738 %)
```

```bash
highuvlith throughput --config examples/sim_arf.toml --dose 30
highuvlith throughput --config examples/sim_lpp_gd.toml --json > beuv.json
```

See also: Python [`wafer_throughput`](python-api-modules.md#sources-landscape-and-throughput) · TOML [`[throughput]`](configuration.md#throughput) · physics [Sources](sources/index.md)

---

## `sources`

Every source family and preset (44 entries over 14 families) with wavelength, bandwidth, average power, power relative to a 250 W HVM source, honesty grade and one headline derived quantity.

```text
highuvlith sources [--family <TYPE>] [--json]
```

- No flags: one table row per preset.
- `--family lpp` (any `[source] type` tag): a detailed block per preset — the `[source]` TOML that selects it, pulse energy, repetition rate, coherence, shot-to-shot noise and every derived quantity with its unit and note.
- `--json`: an array of objects with `family`, `preset`, `toml`, `status`, `note`, `wavelength_nm`, `bandwidth_pm`, `average_power_w`, `hvm_power_ratio`, `photon_energy_ev`, `pulse_energy_j`, `rep_rate_hz`, `transverse_coherence`, `shot_to_shot_rms`, `headline`, `derived_quantities`.

Every `toml` snippet is a valid `[source]` table: paste it into a config.

```text
$ highuvlith sources
family         preset            λ (nm)    Δλ (pm)      P (W)    P/250 W  status             headline
...
lpp            sn_co2           13.5000      270.0      250.0      1.000  ✅                 drive_watts_per_if_watt = 86.00 -
lpp            sn_500w          13.5000      270.0      500.0      2.000  🔶                 drive_watts_per_if_watt = 86.00 -
lpp            gd               6.70000      40.00      13.57    0.05426  🧪                 drive_watts_per_if_watt = 737.1 -
```

See also: Python [`source_landscape()`](python-api-modules.md#sources-landscape-and-throughput) · GUI [Source panel](gui.md#source-panel) · physics [Sources](sources/index.md)

---

## `materials`

Optical constants and multilayer mirrors; output on stdout. The CXRO/Henke and NIST data are ✅ Implemented; the tabulated database entries are 🔶 Simplified.

```text
highuvlith materials [--wavelength <NM>] [--name <MATERIAL>] [--json]
highuvlith materials --multilayer <mo_si|la_b4c|la_b> [--wavelength <NM>] [--periods <N>] [--period-nm <NM>] [--gamma <G>] [--angle-deg <DEG>] [--json]
```

| Flag | Default | Meaning |
|---|---|---|
| `--wavelength` | 157 nm (13.5 nm with `--multilayer`) | evaluation wavelength |
| `--name` | all VUV table materials | a table material (`CaF2`, `MgF2`, `LiF`, `BaF2`, `SiO2`, `Cr`, `Si`, `AlF3`, `Na3AlF6`, `LaF3`, ...) or a Henke compound `henke:<formula>[@density_g_cm3]` |
| `--multilayer` | — | mirror preset: `mo_si`, `la_b4c` or `la_b` |
| `--periods`, `--period-nm`, `--gamma` | preset values | bilayer count, period (nm) and absorber fraction Γ |
| `--angle-deg` | 0 | incidence angle from the normal |
| `--json` | off | JSON instead of text |

```text
$ highuvlith materials --name CaF2 --wavelength 157.63
CaF2 at 157.630 nm:
  n = 1.5570
  k = 0.0000
  dn/dλ = -0.002495 /nm

$ highuvlith materials --name henke:Si3N4@3.44 --wavelength 13.5
henke:Si3N4@3.44 at 13.5000 nm:
  n = 0.973134  (δ = 2.6866e-2)
  k = 0.009318  (β = 9.3180e-3)
  attenuation length (1/e intensity) = 115.3 nm

$ highuvlith materials --multilayer mo_si
Mo/Si (Si on Mo; Γ = Mo fraction) on SiO2: 40 × 6.900 nm periods, Γ = 0.4 (ideal interfaces)
At 13.500 nm, 0° from the normal:
  R_s = 0.7293   R_p = 0.7293   R (unpolarized) = 0.7293
Peak (unpolarized, 12.82–14.18 nm): R = 0.7300 at 13.480 nm
Bandwidth (FWHM): 0.629 nm
Angular acceptance (FWHM, unpolarized, at 13.500 nm): 21.3°
Note: ideal stack: measured Mo/Si mirrors reach ~67–70 % (no interdiffusion, roughness or oxidation modeled here).
```

Multilayer reflectances are for ideal (sharp, smooth) interfaces and are upper bounds on measured mirrors; see [Materials](./materials.md).

See also: Python [`refractive_index`, `xray_optical_constants`, `MultilayerMirror`](python-api-modules.md#liga-x-ray-materials-and-multilayers) · physics [Materials](materials.md)

---

**Where next:** [GUI](gui.md) · [TOML configuration](configuration.md) · [Python API](python-api.md) · [`examples/`](../examples) — 30 runnable configs, each file's header says which command runs it · [Reference](reference/index.md)
