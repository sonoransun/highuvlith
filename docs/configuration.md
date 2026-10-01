# TOML Configuration Reference

**Status:** ✅ Implemented — every table and key on this page is parsed, defaulted and validated by [`crates/highuvlith-cli/src/config.rs`](../crates/highuvlith-cli/src/config.rs) and the subcommand modules in [`crates/highuvlith-cli/src/commands/`](../crates/highuvlith-cli/src/commands); the physics behind individual keys ranges from ✅ to 🧪 (badges below, authoritative grades in the [capability matrix](./capability-matrix.md)).

This page is the reference for the TOML files read by the [CLI](./cli.md). One file describes one run. The [Python API](./python-api.md) and the [GUI](./gui.md) build the same core objects programmatically and do not read these files; where a TOML key differs from the Python keyword the difference is noted.

Every key below was checked against the code at the time of writing; when a value comes from a source or optics *preset*, the preset is named and `highuvlith sources --family <type>` prints its numbers and the TOML that selects it.

## File structure

```toml
[source]          # light source (selected by `type`)
[optics]          # projection optics (selected by `type`)
[illumination]    # optional pupil fill, overrides the source's own fill
[mask]            # pattern (selected by `pattern`)
[grid]            # simulation grid (optional)
[process]         # dose, focus, threshold, CD target (optional)
[imaging]         # engine settings (optional), incl. [imaging.vector]
[deep]            # `highuvlith deep` only (selected by `mode`)
[optimize]        # `highuvlith optimize` only (selected by `method`)
[throughput]      # `highuvlith throughput` only (optional)
```

Every table is optional at the schema level; each subcommand requires the tables it reads and says so (`this command needs an [optics] table (e.g. [optics] na = 0.75)`).

| Subcommand | Reads | Notes |
|---|---|---|
| `simulate`, `sweep` | `[source]` `[optics]` `[mask]` (required); `[illumination]` `[grid]` `[process]` `[imaging]` | `[deep]`, `[optimize]`, `[throughput]` are not read; each one present prints `note: [deep] is not read by simulate (ignored and not validated; the deep subcommand reads it)` on stderr and the run continues (one file may serve several subcommands; each table is validated by the subcommand that reads it). |
| `deep` | `[deep]` (required) + the shared tables of the mode — see [`[deep]`](#deep) | A shared table the mode does not read prints `note: [optics] is not read by deep mode "liga" (ignored)`. |
| `optimize` | `[optimize]` (required); `ilt` / `opc` / `sraf` also `[source]` `[optics]` `[mask]` `[grid]` `[process]` `[imaging]` `[illumination]` | `sadp` / `saqp` read only `[optimize]` and note any other table as ignored. |
| `throughput` | `[source]` (required), optional `[throughput]`, `[process] dose_mj_cm2`, `[mask]` (drawn CD for photon statistics) | |
| `sources`, `materials` | no config file | |

## Strict keys

Silently ignored keys were a reported trap (for example a Python keyword such as `numerical_aperture` written into TOML, or the DPP spelling of a key in an LPP config). Every table is therefore strict:

1. **Unknown keys are errors** with the line, a did-you-mean suggestion and the keys the selected context reads:

    ```text
    Error: typo.toml: TOML parse error at line 3, column 1
      |
    3 | conversion_efficency = 0.05
      | ^^^^^^^^^^^^^^^^^^^^
    unknown key `conversion_efficency` in [source] — did you mean `conversion_efficiency`?
    keys read by [source] type = "lpp": type, fuel, preset, drive_laser, wavelength_nm, bandwidth_pm, sigma, spectral_samples, drive_laser_power_w, conversion_efficiency, transport_efficiency, rep_rate_hz, source_diameter_um, source_diameter_mm, collection_solid_angle_sr, illuminator_etendue_mm2_sr
    ```

2. **Keys that exist but are not read by the selected `type` / `pattern` / `shape` / `mode` / `method` are errors** naming the context that does read them:

    ```text
    Error: [source] type = "lpp" does not use 1 of the given keys:
      - `collector_efficiency` is only read by type = "dpp" — did you mean `conversion_efficiency`?
    keys read by [source] type = "lpp": type, fuel, preset, drive_laser, ...
    ```

3. **Conditional rules**: a key that only takes effect together with another one (or that another key replaces) is rejected with the reason, e.g.

    ```text
    Error: [deep] mode = "volumetric" does not use 1 of the given keys:
      - `car_k_amp` is not used here: the chemically amplified bake keys need peb = "car"
    ```

Nested tables are strict too (`[imaging.vector]`, `[[mask.features]]`, `[[deep.filters]]`).

## `[source]`

One flat table for every source family, selected by `type`. Family keys are optional and default to the family's reference preset.

**Back-compatibility.** Without `type` the legacy VUV source is built: `wavelength_nm` (default 157.63), `sigma` (0.7), `bandwidth_pm` (1.1), `rep_rate_hz` (4000) — every pre-multi-source config (e.g. [`examples/sim.toml`](../examples/sim.toml)) runs unchanged.

**Derived wavelengths.** For families whose wavelength is set by the machine (synchrotron undulator, HHG, X-ray tube, betatron, Smith–Purcell with a grating period, soft-X-ray-laser lines) an explicit `wavelength_nm` is a *cross-check*, rejected when it disagrees with the derived value by more than 5 % (HHG: 1 %, because adjacent harmonics are only $2/q$ apart; SXRL: 1 % of the fixed lasing line).

### Shared keys

| Key | Type | Default | Units | Read by | Validation |
|---|---|---|---|---|---|
| `type` | string | `"vuv"` | — | all | one of `vuv`, `lpa_fel`, `lpp`, `synchrotron`, `hhg`, `xfel`, `ics`, `ssmb`, `entangled`, `xray_tube`, `dpp`, `sxrl`, `betatron`, `smith_purcell` |
| `wavelength_nm` | float | family preset | nm | all except the families noted | finite, > 0; cross-checked where derived |
| `sigma` | float | 0.7 | — | `vuv`, `lpa_fel`, `lpp`, `dpp` only (others derive their fill) | (0, 1]; not allowed together with `[illumination]` |
| `bandwidth_pm` | float | preset | pm | `vuv`, `lpa_fel`, `lpp`, `dpp`, synchrotron `bending_magnet` | — |
| `spectral_samples` | integer | preset | — | all | ≥ 1 (used by polychromatic imaging) |

### Per-family keys

The lists are the exact key sets in `config.rs` (`*_KEYS`); a key outside the row of the selected family is rejected.

| `type` (status) | Keys read (besides `type`) |
|---|---|
| `vuv` ✅ (Ar₂ 🧪; DUV heritage ✅ with 🔶 parameters) | `preset`, `wavelength_nm`, `bandwidth_pm`, `sigma`, `rep_rate_hz`, `spectral_samples` |
| `lpa_fel` 🔶 | `wavelength_nm`, `sigma`, `bandwidth_pm`, `spectral_samples`, `electron_energy_mev`, `pulse_duration_fs`, `rep_rate_hz`, `pulse_energy_uj`, `period_mm`, `undulator_k`, `num_periods`, `peak_current_a`, `norm_emittance_um`, `electron_energy_spread_rel`, `beta_m` |
| `lpp` ✅ (🔶 CE / étendue; Gd/Tb power 🧪) | `fuel`, `preset`, `drive_laser`, `wavelength_nm`, `bandwidth_pm`, `sigma`, `spectral_samples`, `drive_laser_power_w`, `conversion_efficiency`, `transport_efficiency`, `rep_rate_hz`, `source_diameter_um`, `source_diameter_mm`, `collection_solid_angle_sr`, `illuminator_etendue_mm2_sr` |
| `synchrotron`, `beamline = "undulator"` ✅ (🔶 coherence) | `beamline`, `electron_energy_gev`, `period_mm`, `undulator_k`, `num_periods`, `harmonic`, `ring_current_ma`, `emittance_x_nm_rad`, `emittance_y_nm_rad`, `electron_energy_spread_rel`, `wavelength_nm`, `spectral_samples` |
| `synchrotron`, `beamline = "bending_magnet"` ✅ | `beamline`, `electron_energy_gev`, `field_t`, `ring_current_ma`, `wavelength_nm`, `bandwidth_pm`, `spectral_samples` |
| `hhg` ✅/🔶 | `gas`, `driver_wavelength_nm`, `driver_intensity_w_cm2`, `harmonic`, `monochromator_bandwidth_pm`, `full_comb`, `comb_passband_nm`, `pulse_energy_nj`, `rep_rate_hz`, `pulse_duration_fs`, `driver_average_power_w`, `conversion_efficiency`, `wavelength_nm`, `spectral_samples` |
| `xfel` ✅ (CW-SC / ERL 🧪) | `xfel_preset`, `preset`, `mode`, `pierce_parameter`, `rel_bandwidth`, `wavelength_nm`, `pulse_energy_uj`, `pulse_duration_fs`, `rep_rate_hz`, `electron_energy_mev`, `period_mm`, `undulator_length_m`, `peak_current_a`, `norm_emittance_um`, `electron_energy_spread_rel`, `beta_m`, `spectral_samples` |
| `ics` 🧪 | `wavelength_nm`, `laser_wavelength_nm`, `laser_a0`, `electron_energy_mev`, `collection_half_angle_mrad`, `electron_energy_spread_rel`, `pulse_energy_nj`, `rep_rate_hz`, `bunch_charge_pc`, `laser_pulse_energy_mj`, `electron_spot_um`, `laser_spot_um`, `spectral_samples` |
| `ssmb` 🧪 | `ring_energy_mev`, `modulation_wavelength_nm`, `wavelength_nm`, `average_power_w`, `average_current_a`, `peak_current_a`, `bunching_factor`, `radiator_periods`, `radiator_k`, `spectral_samples` |
| `entangled` 🧪 | `wavelength_nm`, `num_photons`, `fidelity`, `pair_rate_hz`, `spectral_samples` |
| `xray_tube` 🔶/✅ | `anode`, `kvp`, `current_ma`, `be_window_um`, `wavelength_nm`, `spectral_samples` |
| `dpp` 🔶 | `fuel`, `sigma`, `wavelength_nm`, `bandwidth_pm`, `spectral_samples`, `electrical_power_w`, `conversion_efficiency`, `source_diameter_mm`, `source_diameter_um`, `source_length_mm`, `collection_solid_angle_sr`, `collector_efficiency`, `illuminator_etendue_mm2_sr`, `rep_rate_hz` |
| `sxrl` ✅/🔶 | `scheme`, `rel_linewidth`, `rel_bandwidth`, `pulse_energy_uj`, `rep_rate_hz`, `pulse_duration_ps`, `wavelength_nm`, `spectral_samples` |
| `betatron` 🧪 | `electron_energy_mev`, `plasma_density_cm3`, `betatron_amplitude_um`, `interaction_length_mm`, `bunch_charge_pc`, `rep_rate_hz`, `pulse_duration_fs`, `wavelength_nm`, `spectral_samples` |
| `smith_purcell` 🧪 | `wavelength_nm`, `grating_period_nm`, `electron_energy_kev`, `diffraction_order`, `observation_angle_deg`, `num_periods`, `beam_current_na`, `impact_height_nm`, `coupling_efficiency`, `collection_half_angle_mrad`, `electron_energy_spread_rel`, `spectral_samples` |

### Aliases, spellings and conditional rules

| Case | Behaviour |
|---|---|
| `collection_solid_angle_sr` / `collector_solid_angle_sr` | Same key (sr) for **both** LPP and DPP; the DPP spelling is a serde alias. Giving both spellings is a duplicate-key error. |
| `source_diameter_um` / `source_diameter_mm` | Both accepted by LPP and DPP, converted (1 mm = 1000 µm). Giving both: `[source] give the plasma diameter once: source_diameter_um OR source_diameter_mm (1 mm = 1000 µm)`. |
| `current_ma` / `tube_current_ma` | X-ray tube current (mA); `current_ma` is the Python / serialized name, `tube_current_ma` an alias. |
| `rel_linewidth` / `rel_bandwidth` (SXRL) | Δλ/λ; `rel_bandwidth` is the fallback spelling. Both at once: rejected (`rel_bandwidth is the fallback spelling of rel_linewidth (give one)`). |
| `preset` / `xfel_preset` (XFEL) | `preset` is an alias; if both are given they must agree. |
| `sigma` with `[illumination]` | Rejected: the pupil fill comes from `[illumination]`. |
| `lpa_fel` beam keys | `num_periods`, `peak_current_a`, `norm_emittance_um`, `electron_energy_spread_rel`, `beta_m` need an undulator (`period_mm` **and** `undulator_k`). |
| synchrotron undulator `electron_energy_spread_rel` | Read only together with `emittance_x_nm_rad` / `emittance_y_nm_rad` (the ring beam). |
| HHG `full_comb = true` | `harmonic`, `monochromator_bandwidth_pm` and `wavelength_nm` rejected; without it `comb_passband_nm` is rejected. |
| HHG `driver_average_power_w` | Derives the harmonic power (driver × efficiency); `pulse_energy_nj` is then rejected. |
| XFEL seeded vs SASE | Seeded mode (`mode = "seeded"`, or the `fermi` preset) rejects `pierce_parameter`; SASE rejects `rel_bandwidth`. An explicit `pierce_parameter` disables the machine derivation, so the machine keys are then rejected. |
| ICS collision keys | Any of `bunch_charge_pc`, `laser_pulse_energy_mj`, `electron_spot_um`, `laser_spot_um` derives the yield; `pulse_energy_nj` is then rejected. |
| SSMB `bunching_factor` | With it the power is derived (∝ b²), so `average_power_w` is rejected. |

### Family details

Defaults below are the values the CLI builder uses when a key is absent; "preset" means the family constructor's reference value (print it with `highuvlith sources --family <type>`).

#### `vuv` — excimer lasers and Hg lamps

| Key | Default | Notes |
|---|---|---|
| `preset` | none (legacy explicit source) | `f2`, `ar2` (🧪), `arf`, `krf`, `hg_i` / `i_line`, `hg_h` / `h_line`, `hg_g` / `g_line`. Explicit `wavelength_nm` / `bandwidth_pm` / `rep_rate_hz` override the preset. Lamps are CW (no average power). |
| `wavelength_nm`, `bandwidth_pm`, `rep_rate_hz` | 157.63 nm, 1.1 pm, 4000 Hz (no preset) | — |

Page: [VUV excimer](./sources/vuv-excimer.md), [DUV heritage](./sources/duv-heritage.md).

#### `lpa_fel` — laser-plasma-accelerator FEL

| Key | Default | Units | Notes |
|---|---|---|---|
| `wavelength_nm` | 25.0 | nm | With an undulator: must match the derived resonance within 5 %. |
| `period_mm`, `undulator_k` | — | mm, — | Give both or neither. |
| `num_periods` | 200 | — | Needs the undulator. |
| `peak_current_a`, `norm_emittance_um`, `electron_energy_spread_rel`, `beta_m` | 1000, 0.5, 0.01, 1.0 | A, µm, —, m | Beam; derive ρ, gain length and the bandwidth 2ρλ. |
| `bandwidth_pm` | derived 2ρλ (with undulator) / preset | pm | An explicit value overrides the derived SASE bandwidth. |
| `electron_energy_mev`, `pulse_duration_fs`, `rep_rate_hz`, `pulse_energy_uj` | preset | MeV, fs, Hz, µJ | The 25 nm / 1 kHz preset is a design projection. |

Page: [LPA-FEL](./sources/lpa-fel.md).

#### `lpp` — laser-produced plasma

| Key | Default | Units | Notes |
|---|---|---|---|
| `fuel` | `"sn"` | — | `sn` (13.5 nm), `gd` (6.7 nm), `tb` (6.5 nm). |
| `preset` | `"nxe3400b"` | — | Sn only: `nxe3400b` (250 W at IF) or `nxe3800e` (500 W at IF; the 43 kW drive power behind it is an assumed scaling). `nxe3800e` with a non-Sn fuel is rejected. |
| `drive_laser` | CO₂ (Sn) | — | `co2`, `solid_state_1um` (`1um`, `nd_yag`), `thulium_2um` (`2um`, `thulium`); for Sn it also sets the default CE. |
| `drive_laser_power_w`, `conversion_efficiency`, `transport_efficiency`, `rep_rate_hz` | preset | W, —, —, Hz | Power at IF = drive × CE × transport (2π → IF) × étendue fraction. |
| `source_diameter_um` / `_mm`, `collection_solid_angle_sr`, `illuminator_etendue_mm2_sr` | NXE-like geometry when any is given | µm / mm, sr, mm² sr | Turn on the étendue check; each must be > 0. |

Page: [LPP](./sources/lpp.md).

#### `synchrotron`

| Key | Default (undulator / bending magnet) | Units | Notes |
|---|---|---|---|
| `beamline` | `"undulator"` | — | or `"bending_magnet"`. |
| `electron_energy_gev` | 0.538 / 2.5 | GeV | |
| `period_mm`, `undulator_k`, `num_periods`, `harmonic` | 20, 1.0, 100, 1 | mm, —, —, — | Undulator; wavelength derived from the resonance condition (odd harmonics). |
| `ring_current_ma` | preset / 200 | mA | Absolute flux and power. |
| `emittance_x_nm_rad`, `emittance_y_nm_rad`, `electron_energy_spread_rel` | 10, 0.1, 5e-4 (when the beam is given) | nm rad, nm rad, — | Derive the transverse coherent fraction and the pupil fill; must be ≥ 0. |
| `field_t` | 1.5 | T | Bending magnet. |
| `wavelength_nm`, `bandwidth_pm` | 0.2 nm, 0.2 pm | | Bending magnet: the monochromator selection. |

Page: [Synchrotron](./sources/synchrotron.md).

#### `hhg` — high-harmonic generation

| Key | Default | Units | Notes |
|---|---|---|---|
| `gas` | `"neon"` | — | `helium`/`he`, `neon`/`ne`, `argon`/`ar`, `krypton`/`kr`, `xenon`/`xe`. |
| `driver_wavelength_nm`, `driver_intensity_w_cm2` | 800, 4e14 | nm, W/cm² | |
| `harmonic` | 59 (1 with `full_comb`) | — | Odd, inside the cutoff. |
| `monochromator_bandwidth_pm` | 15 | pm | |
| `full_comb`, `comb_passband_nm` | false, none | —, `[min_nm, max_nm]` | Image the whole comb, optionally filtered. |
| `driver_average_power_w` | constructor default | W | Derives the harmonic power (driver × efficiency); > 0. |
| `conversion_efficiency` | gas/λ estimate | — | (0, 1]. |
| `pulse_energy_nj`, `rep_rate_hz`, `pulse_duration_fs` | preset | nJ, Hz, fs | A pulse energy alone (no driver power) is used as stored. |

Page: [HHG](./sources/hhg.md).

#### `xfel`

| Key | Default | Units | Notes |
|---|---|---|---|
| `xfel_preset` (`preset`) | `"flash"` | — | `flash`, `fermi` (`fermi_seeded`), `cw_sc` (🧪), `erl` (🧪). |
| `mode` | preset's | — | `sase` or `seeded` (`self_seeded`). |
| `pierce_parameter` | derived from the machine (3e-3 if `mode = "sase"` is forced without a machine) | — | Explicit value disables the machine derivation. |
| `rel_bandwidth` | 5e-5 | — | Seeded mode only. |
| `electron_energy_mev`, `period_mm`, `undulator_length_m`, `peak_current_a`, `norm_emittance_um`, `electron_energy_spread_rel`, `beta_m` | preset machine | MeV, mm, m, A, µm, —, m | Re-derive ρ, gain length, saturation power — and, unless `pulse_energy_uj` is given, the pulse energy (preset's pulse-energy-to-saturation ratio kept). A `wavelength_nm` below the K = 0 resonance is rejected. |
| `pulse_energy_uj`, `pulse_duration_fs`, `rep_rate_hz` | preset (pulse energy rescaled by machine / `wavelength_nm` / `pulse_duration_fs` overrides) | µJ, fs, Hz | An explicit `pulse_energy_uj` is kept as given. |

Page: [XFEL](./sources/xfel.md).

#### `ics` — inverse Compton scattering

| Key | Default | Units | Notes |
|---|---|---|---|
| `wavelength_nm` | 13.5 | nm | Target; the electron energy is derived from the Compton condition. |
| `laser_wavelength_nm`, `laser_a0` | 1030, 0.1 | nm, — | |
| `electron_energy_mev` | derived | MeV | If given, must satisfy the Compton condition within 5 %. |
| `bunch_charge_pc`, `laser_pulse_energy_mj`, `electron_spot_um`, `laser_spot_um` | high-average-power design point | pC, mJ, µm, µm | Any of them derives the yield (Thomson luminosity × collection). |
| `collection_half_angle_mrad` | the 2 % Mo/Si band (collision mode) | mrad | |
| `electron_energy_spread_rel`, `pulse_energy_nj`, `rep_rate_hz` | preset | —, nJ, Hz | |

Page: [Inverse Compton](./sources/inverse-compton.md).

#### `ssmb` — steady-state microbunching

| Key | Default | Units | Notes |
|---|---|---|---|
| `ring_energy_mev`, `modulation_wavelength_nm`, `wavelength_nm`, `average_power_w` | 400, 1053, 13.5, 1000 | MeV, nm, nm, W | The radiated wavelength must be an integer harmonic of the modulation. |
| `average_current_a`, `peak_current_a`, `bunching_factor`, `radiator_periods`, `radiator_k` | 1.0, = average, (required b), 100, 1.6 | A, A, —, —, — | Any of them derives the coherent power; without `bunching_factor` the b that `average_power_w` needs is reported. |

Page: [SSMB](./sources/ssmb.md).

#### `entangled` — N00N source

| Key | Default | Units | Notes |
|---|---|---|---|
| `wavelength_nm`, `num_photons`, `fidelity` | 157.63, 2, 1.0 | nm, —, — | |
| `pair_rate_hz` | preset | Hz | ≥ 0. |

Page: [Entangled photon](./sources/entangled-photon.md).

#### `xray_tube`

| Key | Default | Units | Notes |
|---|---|---|---|
| `anode` | `"w"` | — | `w` (60 kV / 30 mA), `mo` (50 / 40), `cu` (40 / 40), `rh` (50 / 40). |
| `kvp` | anode preset | kV | = maximum photon energy (keV). |
| `current_ma` (`tube_current_ma`) | anode preset | mA | |
| `be_window_um` | preset | µm | |
| `wavelength_nm` | derived (photon-weighted mean) | nm | Cross-check, 5 %. |

Absolute flux at the mask for LIGA: `[deep] source_distance_m` (see [`[deep]` LIGA](#mode-liga)). Page: [X-ray tube](./sources/xray-tube.md).

#### `dpp` — discharge-produced plasma

| Key | Default | Units | Notes |
|---|---|---|---|
| `fuel` | `"sn"` | — | `sn` (laser-assisted) or `xe`. |
| `electrical_power_w`, `conversion_efficiency`, `collector_efficiency`, `rep_rate_hz` | preset | W, —, —, Hz | Power at IF = P_elec × CE (2π) × collector × étendue cut. |
| `source_diameter_mm` (`_um`), `source_length_mm`, `collection_solid_angle_sr`, `illuminator_etendue_mm2_sr` | preset | mm, mm, sr, mm² sr | |

Page: [DPP](./sources/dpp.md).

#### `sxrl` — plasma soft-X-ray laser

| Key | Default | Units | Notes |
|---|---|---|---|
| `scheme` | `"ar_46nm9"` | — | `ar_46nm9`, `ag_13nm9`, `cd_13nm2`, `mo_18nm9` (fixed lasing lines). |
| `rel_linewidth` (`rel_bandwidth`) | preset | — | Δλ/λ. |
| `pulse_energy_uj`, `rep_rate_hz`, `pulse_duration_ps` | preset | µJ, Hz, ps | |
| `wavelength_nm` | the line | nm | Must be within 1 % of the scheme's line. |

Page: [Soft-X-ray laser](./sources/soft-xray-laser.md).

#### `betatron` — laser-wakefield betatron X-rays

| Key | Default | Units | Notes |
|---|---|---|---|
| `electron_energy_mev`, `plasma_density_cm3`, `betatron_amplitude_um`, `interaction_length_mm`, `bunch_charge_pc`, `rep_rate_hz` | 200, 1e19, 1.0, 3.0, 50, 10 | MeV, cm⁻³, µm, mm, pC, Hz | Derive the critical energy, photon number and wavelength. |
| `pulse_duration_fs` | preset | fs | |
| `wavelength_nm` | derived (hc / mean photon energy) | nm | Cross-check, 5 %. |

Page: [Betatron](./sources/betatron.md).

#### `smith_purcell`

| Key | Default | Units | Notes |
|---|---|---|---|
| `electron_energy_kev`, `diffraction_order`, `observation_angle_deg` | 30, 1, 90 | keV, —, deg | |
| `grating_period_nm` | derived for `wavelength_nm` | nm | Given: the wavelength $\lambda = (a/m)(1/\beta - \cos\theta)$ is derived and cross-checked (5 %). |
| `wavelength_nm` | 13.5 (without a period) | nm | |
| `num_periods`, `beam_current_na`, `impact_height_nm`, `coupling_efficiency`, `collection_half_angle_mrad`, `electron_energy_spread_rel` | preset | —, nA, nm, —, mrad, — | Power is a theoretical projection (EUV emission undemonstrated). |

Page: [Smith–Purcell](./sources/smith-purcell.md).

### Presets at a glance

`highuvlith sources` lists every preset with λ, bandwidth, usable power and a headline derived quantity; `--family <type>` prints the `[source]` TOML of each preset and all derived quantities. The CLI preset names map to TOML as follows (non-obvious cases):

| `sources` preset | `[source]` TOML |
|---|---|
| `vuv / legacy` | `wavelength_nm = 157.63` (no `type`) |
| `lpp / sn_co2`, `sn_500w` | `type = "lpp"` + `fuel = "sn"` / `preset = "nxe3800e"` |
| `lpp / sn_1um`, `sn_2um` | `drive_laser = "solid_state_1um"` / `"thulium_2um"` |
| `synchrotron / compact_euv` | undulator + `emittance_x_nm_rad = 10.0`, `emittance_y_nm_rad = 0.1` |
| `hhg / ar_q27`, `ar_full_comb` | `gas = "ar"`, `driver_intensity_w_cm2 = 2.0e14`, `harmonic = 27` / `full_comb = true` |
| `ics / collision` | the four collision keys + `rep_rate_hz = 1.0e8` |
| `ssmb / radiator` | `average_current_a = 1.0`, `radiator_periods = 100`, `radiator_k = 1.6` |
| `smith_purcell / xray_1nm` | `grating_period_nm = 0.335` |

## `[illumination]`

Optional pupil fill that **overrides** the source family's own fill (e.g. the near-coherent Gaussian of an FEL). Source power and étendue bookkeeping are unchanged. With `[illumination]` present, `[source] sigma` is rejected.

| `shape` | Keys | Defaults / validation |
|---|---|---|
| `conventional` | `sigma` | required, (0, 1] |
| `coherent_gaussian` | `sigma` (Gaussian width) | required, (0, 1] |
| `annular` | `sigma_inner`, `sigma_outer` | required; inner in [0, 1], outer in (0, 1], inner < outer |
| `dipole` | `sigma_center`, `sigma_radius`, `orientation_deg` | poles at `orientation_deg` and `orientation_deg` + 180° (default 0 = poles on x); `sigma_radius` ≤ `sigma_center`, `sigma_center + sigma_radius` ≤ 1 |
| `quadrupole` | `sigma_center`, `sigma_radius`, `opening_angle_deg` | same pole checks; opening angle in (0, 90], default 90 |

## `[optics]`

Selected by `type` (default `"refractive"`). Keys of another type are rejected naming the owner.

| Key | Type | Units | `refractive` | `schwarzschild` | `euv_projection` | `zone_plate` |
|---|---|---|---|---|---|---|
| `preset` | string | — | `immersion_193i` | `euv`, `beuv`, `soft_xray` (default from λ: < 3 nm soft_xray, < 10 nm beuv, else euv) | `nxe` (default), `high_na` | — |
| `na` | float | — | default 0.75 | preset (0.33 / 0.25 / 0.1) | preset (0.33 / 0.55) | default 0.1 (or from `outer_zone_width_nm`) |
| `flare_fraction` | float | — | default 0.02 | preset (0.03 / 0.05 / 0.05) | default 0 | — |
| `immersion_index` | float | — | default 1.0 (preset 1.437) | — | — | — |
| `central_obscuration` | float | fraction of NA | — | preset (0.25 / 0.3 / 0.3) | preset (0 / 0.2 assumed) | — |
| `mirror_reflectivity` | float | — | — | preset (0.67 / 0.50 / 0.3) | — | — |
| `transmission` | float | — | — | — | default 1.0 | — |
| `zernike` | `[[index, waves], ...]` | waves | ✓ | — | ✓ | — |
| `axial_chromatic_nm_per_pm` | float | nm defocus / pm | default 15 (a CaF₂ / 157 nm figure) | — | — | — |
| `paraxial_defocus` | bool | — | false | false | false | false |
| `outer_zone_width_nm` | float | nm | — | — | — | default λ/(2·NA) |
| `multilayer_pupil` | table | — | — | opt-in | opt-in | — |

Validation: `flare_fraction` and `central_obscuration` in [0, 1); `mirror_reflectivity`, `transmission` in [0, 1]; dry NA in (0, 1); with `immersion_index` (≥ 1) NA ≤ 0.95 × index; Zernike indices 1–37 (Fringe); zone plate: `na` **or** `outer_zone_width_nm`, not both. `mirror_reflectivity` and `transmission` matter only for `[imaging] normalization = "absolute"`.

Status: refractive ✅, immersion ✅, Schwarzschild ✅, zone plate ✅, EUV projection 🔶 (isotropic wafer-side pupil; the High-NA 0.2·NA obscuration is an assumption; anamorphic magnification and mask 3D not modeled). See [Optics](./optics.md).

The opt-in angle-dependent **multilayer pupil** (🔶) is the `[optics.multilayer_pupil]` sub-table (Schwarzschild and EUV projection only; same parameters as Python `OpticsConfig.with_multilayer_pupil`):

```toml
[optics]
type = "euv_projection"

[optics.multilayer_pupil]
mirrors   = [[0.0, 0.0, 0.0, 15.0], [0.0, 0.0, 0.0, 15.0]]  # per mirror: [center_deg, tilt_deg, azimuth_deg, radial_deg]
coating   = "mo_si"   # mo_si (default) | la_b4c | la_b
periods   = 40        # bilayers (default 40)
period_nm = 6.9       # default 6.9
gamma     = 0.4       # heavy-layer fraction Γ (default 0.4)
```

Each mirror's incidence angle across the pupil is θ(p) = |center + tilt·(p·u_azimuth) + radial·|p|²| (p normalized to NA); `mirrors` must list at least one mirror. The maps are **user assumptions**: real maps come from ray-tracing a design, which the repository does not contain. All mirrors share one coating, and the amplitude is polarization-averaged even in vector mode. With the default clear-field normalization the angle dependence shows up as apodization and phase, not as a dose change. `mirror_reflectivity` (Schwarzschild) is ignored while a multilayer is attached, and `transmission` (EUV projection) multiplies it. Unknown keys in the sub-table are errors. `simulate` prints the mirror count and coating on its `Optics:` line and sets `multilayer_pupil = true` in its JSON. See [Optics](./optics.md).

A refractive lens selected below 50 nm prints a warning (no transparent lens material exists there). A centrally obscured Schwarzschild paired with a nearly coherent fill (σ ≲ obscuration) blocks the zero order: the CLI warns about dark-field imaging when the clear-field intensity falls below 1 % — use `euv_projection` or a larger fill.

## `[mask]`

Selected by `pattern` (default `"line_space"`).

| Key | Type | Units | `line_space` | `contact_holes` | `features` |
|---|---|---|---|---|---|
| `cd_nm` | float | nm | opaque line width, default 65 | — | — |
| `pitch_nm` | float | nm | default 180 | x pitch, required | — |
| `orientation` | string | — | `vertical` (lines along y, default) / `horizontal` | — | — |
| `offset_nm` | float | nm | centre of one line along the periodic axis, default 0 | — | — |
| `diameter_nm` | float | nm | — | side of the square hole, required | — |
| `pitch_y_nm` | float | nm | — | default `pitch_nm` | — |
| `features` | array of tables | nm | — | — | `[[mask.features]]`, painted in order |
| `dark_field` | bool | — | default false | default true | default false |
| `mask_type` | string | — | `binary` (default) or `att_psm` | same | same |
| `transmission`, `phase_deg` | float | —, deg | att_psm only: 0.06, 180 | same | same |

`[[mask.features]]` entries are tagged by `type` (same names as the Python `MaskConfig.from_features` dicts): `rect` (`x`, `y`, `w`, `h`), `polygon` (`vertices = [[x, y], ...]`, even-odd fill), `gray_rect` (`x`, `y`, `w`, `h`, `transmittance` in [0, 1]), `line_space` (`cd`, `pitch`, optional `orientation`, `offset`), `rect_array` (`w`, `h`, `pitch_x`, `pitch_y`, optional `offset_x`, `offset_y`).

Status: thin Kirchhoff mask 🔶 (binary + attenuated PSM), exact analytic spectra ✅ — see [Masks and metrics](./masks-and-metrics.md).

## `[grid]`

| Key | Type | Default | Units | Validation |
|---|---|---|---|---|
| `size` | integer | 256 | pixels | power of two |
| `pixel_nm` | float | 1.0 | nm | > 0 |
| `commensurate` | bool | true | — | see below |

The FFT makes the field periodic, so an incommensurate field images a truncated grating. With `commensurate = true` (default) the pixel is adjusted so the field holds a whole number of mask periods, never coarser unless one period does not fit, and a note is printed:

```text
note: pixel 2 nm -> 1.875000 nm so the 240.000 nm field holds 4 whole periods of the 60 nm mask cell (the FFT makes the field periodic); set [grid] commensurate = false to keep the configured pixel
```

## `[process]`

| Key | Type | Default | Units | Validation | Read by |
|---|---|---|---|---|---|
| `dose_mj_cm2` | float | 30 | mJ/cm² | finite, > 0 | nominal dose of the threshold resist (`simulate`, `sweep`, `optimize`); `throughput` dose fallback; volumetric dose fallback |
| `focus_nm` | float | 0 | nm | finite | `simulate` (overridden by `--focus`), volumetric |
| `threshold` | float | 0.3 | clear-field intensity | finite, > 0 | printing threshold at the nominal dose |
| `cd_target_nm` | float | drawn line / hole width | nm | > 0 | `sweep` (required for a free-form mask) |
| `cd_tolerance_pct` | float | 10 | % of target | (0, 100) | `sweep` process window |

Constant-threshold resist (🔶): at dose $d$ the printed contour is the intensity

```math
I_{\mathrm{print}}(d) = \mathrm{threshold} \cdot \frac{d_{\mathrm{nominal}}}{d}
```

which is how `simulate --dose` and the `sweep` dose axis move the edge.

## `[imaging]`

| Key | Type | Default | Validation / meaning |
|---|---|---|---|
| `defocus_model` | string | `"exact"` | `exact` (defocus inside the pupil per source point) or `kernel_phase` (legacy approximation, valid only for on-axis near-coherent fills) |
| `max_kernels` | integer | 20 | ≥ 1 SOCS kernels per kernel set |
| `kernel_energy_fraction` | float | 1.0 | (0, 1]; stop once the kernels capture this fraction of the TCC trace |
| `source_points_per_axis` | integer | adaptive | ≥ 1 (1 = single coherent point) |
| `spectrum` | string | `"monochromatic"` | `monochromatic`, `narrow_band` (focus shift per spectral sample, center-λ kernels, 🔶), `per_wavelength` (TCC rebuilt per sample, ✅). Read by `simulate` and `sweep`; `deep` volumetric and `optimize` require `monochromatic`. |
| `normalization` | string | `"clear_field"` | `clear_field` (clear mask = 1) or `absolute` (relative to the incident illumination; includes mirror reflectivity / transmission) |
| `vector` | table | absent (scalar) | `[imaging.vector]`, below |

### `[imaging.vector]`

Present (even empty) = vector (polarized) imaging ✅; absent = scalar. Keys (strict):

| Key | Default | Meaning |
|---|---|---|
| `polarization` | `{ type = "unpolarized" }` | `unpolarized`, `x`, `y`, `te` (alias `azimuthal`), `tm` (alias `radial`), `{ type = "linear", angle_deg = 45.0 }` |
| `image_index` | 1.0 | image-space index; left at 1.0 it inherits the optics' `immersion_index`, any other value must equal it |
| `obliquity` | true | radiometric obliquity amplitude factor |
| `reduction` | 4.0 | replaced by the optics' reduction in the engine |
| `film` | none | `{ n = 1.7, k = 0.03 }` film-entrance Fresnel transmission |

```toml
[imaging.vector]
polarization = { type = "te" }
film = { n = 1.7, k = 0.03 }
```

See [Vector imaging](./vector-imaging.md).

## `[deep]`

Selected by `mode` (required): `liga`, `grayscale`, `interference`, `volumetric`, `talbot`. Shared tables read per mode:

| Mode | Shared tables read |
|---|---|
| `liga` | `[source]` `[mask]` `[grid]` (only `[mask]` `[grid]` with `flux_density` or `critical_energy_kev`) |
| `grayscale` | `[grid]` |
| `interference` | `[source]` `[grid]` |
| `volumetric` | `[source]` `[optics]` `[mask]` `[grid]` `[process]` `[imaging]` `[illumination]` |
| `talbot` | `[source]` `[mask]` `[grid]` |

### mode: liga

Deep-X-ray shadow printing (✅ depth dose and exposure time; ✅/🔶 Fresnel proximity). See [LIGA](./processes/liga-deep-xray.md).

| Key | Default | Units | Notes |
|---|---|---|---|
| `flux_density` | — | `[[E_keV, photons s⁻¹ mm⁻² keV⁻¹], ...]` | Absolute spectrum at the mask; overrides every other spectrum (then `critical_energy_kev` and the geometry keys are rejected). |
| `critical_energy_kev` | from `[source]` | keV | Relative bending-magnet spectrum (then the absolute-exposure keys are rejected). |
| `source_distance_m` | — | m | Absolute exposure from the `[source]`: bending magnet (with `vertical_scan_mm`), X-ray tube or betatron (point source, inverse square; then `horizontal_acceptance_mrad` / `vertical_scan_mm` are rejected). |
| `horizontal_acceptance_mrad` | 5 | mrad | Bending magnet; needs `vertical_scan_mm`. |
| `vertical_scan_mm` | — | mm | Bending-magnet scan height. |
| `resist_thickness_um`, `proximity_gap_um` | 500, 100 | µm | gap ≥ 0 |
| `diffraction` | `"fresnel"` | — | or `gaussian` (legacy blur; then `strict_sampling` is rejected) |
| `energy_bins` | 100 | — | |
| `photoelectron_blur` | false | — | Grün-range upper bound |
| `filters` | none | `[[deep.filters]] material = "Be"`, `thickness_um = 100.0` | material preset or `<formula>@<density>` |
| `absorber`, `absorber_thickness_um` | `Au`, 20 | —, µm | |
| `membrane`, `membrane_thickness_um` | `Ti`, 2 | —, µm | |
| `target_bottom_dose_kj_cm3`, `damage_dose_kj_cm3` | 3, 20 | kJ/cm³ | |
| `develop_threshold_kj_cm3` | `target_bottom_dose_kj_cm3` | kJ/cm³ | depth-map PNG |
| `min_feature_nm` | 5000 | nm | aspect-ratio figure |
| `nz` | 64 | — | depth slices |
| `strict_sampling` | false | — | refuse to run when the pixel under-resolves the Fresnel scale (otherwise a warning) |
| `edge_profile`, `edge_dx_nm`, `edge_half_width_nm` | false, 10, 3000 | —, nm, nm | fine 1D edge profile and sidewall angle; the sampling keys need `edge_profile = true` |

Spectrum precedence: `flux_density` → `critical_energy_kev` → `[source]` (bending magnet: absolute with `source_distance_m` + `vertical_scan_mm`, else relative; X-ray tube: absolute with `source_distance_m`, else its relative tabulated spectrum; betatron: absolute with `source_distance_m`, else relative at its derived critical energy).

### mode: grayscale

2.5D height map (🔶). See [Grayscale](./processes/grayscale.md).

| Key | Default | Units | Notes |
|---|---|---|---|
| `target` | `"blazed"` | — | `blazed`, `microlens`, `staircase` |
| `period_px`, `depth_nm` | grid/4, 0.6 × thickness | px, nm | `blazed` |
| `pitch_px`, `sag_nm` | grid/4, 0.6 × thickness | px, nm | `microlens` |
| `n_levels`, `depth_nm` | 8, 0.8 × thickness | —, nm | `staircase` |
| `thickness_nm` | 1000 | nm | |
| `dose_mj_cm2`, `d_th`, `d_clear` | 100, 10, 100 | mJ/cm² | contrast curve |

Keys of another `target` are rejected.

### mode: interference

Multi-beam holographic recording (✅/🔶). See [Interference / volumetric](./processes/interference-volumetric.md).

| Key | Default | Units |
|---|---|---|
| `preset` | `"two_beam"` (`three_beam_hex`, `four_beam_umbrella`) | — |
| `half_angle_deg` | 30 | deg |
| `n_medium` | 1.6 | — |
| `two_photon` | false | — |
| `z_span_nm`, `nz` | field size, 64 | nm, — |
| `dose_scale`, `dill_c`, `fill_threshold` | 1.0, 0.02, 0.5 | — |

### mode: volumetric

z-resolved Dill exposure, bake and development (🔶 exposure; ✅ FMM; ✅/🔶 level set). See [Volumetric exposure](./processes/volumetric-exposure.md) and [Resist models](./processes/resist-models.md).

| Key | Default | Units | Notes |
|---|---|---|---|
| `dose_mj_cm2` | `[process] dose_mj_cm2` (30) | mJ/cm² | same default as Python `simulate_volumetric`; usually too high for development, see the note below the table |
| `resist_thickness_nm` | 150 | nm | resist layer of the default resist-on-Si stack, built from the materials database at the source wavelength (`VUV_resist` on Si in the VUV tables' range, `EUV_resist` on Henke Si at 0.0413–41.3 nm, an error elsewhere); the `ResistParams` default (was 300 before 2026-10-01) |
| `nz`, `n_defocus_planes`, `dose_steps` | 64, 8, 1 | — | `dose_steps` > 1 = split-step bleaching |
| `develop_threshold` | 0.5 | PAC | per-slice CD |
| `peb` | `"gaussian"` | — | `gaussian`, `none`, `car`. The default bake matches Python `simulate_volumetric` (σ = the resist `peb_diffusion_nm` = 30 nm on both axes; the GUI uses σxy 10 / σz 20 nm). Without a bake the standing-wave nodes of the default resist-on-Si stack stall a development front about 23 nm below the top. Default was `none` before 2026-10-01. |
| `peb_lateral_nm`, `peb_vertical_nm`, `peb_vertical_surface_ratio`, `peb_vertical_decay_nm` | resist diffusion length, = lateral, —, — | nm | `peb = "gaussian"` only |
| `car_peb_time_s`, `car_k_amp`, `car_k_quench`, `car_quencher`, `car_acid_diffusivity_nm2_s`, `car_quencher_diffusivity_nm2_s`, `car_vertical_ratio` | 60, 0.1, 10, 0.15, 2, 0.5, 1 | s, 1/s, 1/s, —, nm²/s, nm²/s, — | `peb = "car"` only |
| `develop` | `"threshold"` | — | `threshold`, `fmm`, `level_set` |
| `dev_time_s`, `surface_rate_ratio`, `inhibition_depth_nm` | 60, —, — | s, —, nm | `fmm` / `level_set` only |
| `lateral_boundary` | `"periodic"` | — | or `reflecting`; needs a bake or development |
| `depletion` | `"none"` | — | `exponential` (`depletion_time_constant_s`), `loading` (`loading_capacity_nm`), `local_loading` (`loading_capacity_nm`, `loading_length_nm`); `level_set` only |

**Choosing the dose.** No fixed volumetric dose suits every config: the window depends on the pattern, the optics, the stack and the development time. The 30 mJ/cm² fallback is the nominal dose of the threshold resist and Python's default; since the exposing intensity in the resist was corrected for its index (×1.65 for the default stack, 2026-10-01) it over-develops the default stack. Measured with the defaults (F2 157.63 nm, NA 0.75, 150/300 nm L/S, 150 nm resist on Si, Gaussian bake 30 nm, `develop = "fmm"` or `"level_set"` for 60 s, 64 × 9.375 nm grid, `nz` = 16; `fmm` and `level_set` give the same CDs at these doses):

| Dose (mJ/cm²) | Developed CD mid / bottom (nm) | Mean remaining height (nm) |
|---|---|---|
| 6 | n/a / n/a (spaces do not clear) | 130–131 |
| 10 | 169 / 188 | 78 |
| 12 | 131 / 131 | 61 |
| 15 | 113 / 94 | 44 |
| 30 (fallback) | 56 / 19 | 8.8 |

The open-frame dose-to-clear of this stack is 6.7 mJ/cm² at 60 s (9.0 at 30 s). After `fmm` or `level_set` development the run prints `note: under-developed: …` when nothing clears to the substrate, and `note: over-developed? only N % of the resist volume remains …` when less than a quarter of the resist remains (legitimate only for mostly clear patterns). Both notes name the dose and where it came from; the JSON carries them in `developed.notes`, with `max_remaining_height_nm` and `min_remaining_height_nm`, and Python `simulate_volumetric` returns the same checks in `.notes`. The GUI Volume tab starts from its own tuned set (15 mJ/cm², σxy 10 / σz 20 nm bake, 30 s), see [GUI](./gui.md).

### mode: talbot

Talbot / DTL / ATL / two-grating EUV-IL (🔶). See [Talbot](./processes/talbot.md).

| Key | Default | Units | Sub-modes |
|---|---|---|---|
| `talbot_mode` | `"carpet"` | — | `carpet`, `dtl`, `atl`, `euv_il` |
| `grating_period_nm` | `[mask] pitch_nm` | nm | all |
| `grating_type`, `duty_cycle`, `phase_rad` | `amplitude`, 0.5, π | —, —, rad | `phase_rad` only with `grating_type = "phase"` |
| `max_order` | 10 | — | all |
| `propagation` | `"exact"` | — | `exact` / `paraxial`; not `euv_il` |
| `carpet_z_max_um`, `carpet_nz` | 2 Talbot lengths, 256 | µm, — | `carpet` |
| `gap_um` | 10 Talbot lengths (dtl) / 2 achromatic distances (atl) | µm | `dtl`, `atl` |
| `scan_talbot_lengths` | 1 | — | `dtl` |
| `bandwidth_nm`, `spectrum_shape` | `[source] bandwidth_pm` / 1000, `gaussian` (or `flat`) | nm | `atl` |
| `il_order` | 1 | — | `euv_il` (≤ `max_order`) |
| `n_medium`, `absorption_per_nm`, `z_span_nm`, `nz`, `dose_scale`, `dill_c`, `fill_threshold` | 1.0, 0, 50, 16, 1.0, 0.02, 0.5 | —, 1/nm, nm, —, —, —, — | resist recording; not `carpet` |

## `[optimize]`

Selected by `method` (required): `ilt` ✅, `opc` ✅, `sraf` 🔶, `sadp` / `saqp` 🔶 (geometric). Key names follow the Python keyword arguments. See [Research modules](./research-modules.md).

**Print threshold** (`ilt` with `cost = "resist"`, `opc`, `sraf`): `threshold`, or `dose_to_size_cd_nm` (anchors the threshold so a line on the horizontal cut at (`cut_x_nm`, `cut_y_nm`), default (0, 0), prints this wide in focus), else `[process] threshold`. `threshold` and `dose_to_size_cd_nm` together are an error; the cut keys need `dose_to_size_cd_nm`. Imaging methods use the centre wavelength only (`[imaging] spectrum` must be `monochromatic`).

| Method | Key | Default | Units / notes |
|---|---|---|---|
| `ilt` | `cost` | `"image"` | or `resist` (sigmoid resist; enables the threshold keys and `steepness`, default 50) |
| | `gradient`, `optimizer` | `adjoint`, `cg` | `proxy` (legacy), `sd` |
| | `max_iterations`, `learning_rate`, `convergence_tol` | 50, 0.5, 1e-4 | |
| | `tv_weight`, `binarization_weight`, `binarization_after` | 0.01, 0, 0 | |
| | `min_feature_nm`, `sigmoid_steepness` | 0 (off), 4 | nm, — |
| | `init`, `init_level` | `target`, 0.5 | `init_level` only with `init = "uniform"` |
| | `conditions` | nominal | `[[defocus_nm, relative_dose, weight], ...]` |
| `opc` | `max_iterations` | 40 | |
| | `tolerance_nm`, `max_epe_tolerance_nm`, `feedback_gain`, `smoothing`, `max_step_nm`, `max_bias_nm`, `max_fragment_nm`, `corner_fragment_nm`, `search_range_nm`, `min_jog_nm`, `bias_grid_nm`, `correct_corners` | λ/NA-scaled defaults | override the engine-derived fragment-OPC settings |
| | `conditions` | nominal | as for `ilt` |
| `sraf` | `line_cd_nm` | `dose_to_size_cd_nm`, else the drawn width | nm |
| | `sigma_center` | from the pupil fill (annular: mean radius) | — |
| | `defocus_nm`, `dose_excursion`, `margin` | 150, 0.08, 0.05 | print-check corners |
| | `compare_dof` | false | with true: `cd_tolerance_pct` (10), `focus_range_nm` (300), `focus_steps` (13), `exposure_latitude_pct` (0) |
| `sadp` | `mandrel_pitch_nm`, `mandrel_cd_nm`, `spacer_nm` | required | nm |
| `saqp` | `mandrel_pitch_nm`, `mandrel_cd_nm`, `spacer1_nm`, `spacer2_nm` | required | nm |
| `sadp`, `saqp` | `tone` | `"spacer_is_line"` | or `spacer_is_dielectric` |

## `[throughput]`

Optional overrides of the dose-limited scanner model (🔶; preset scanner numbers are illustrative assumptions, not vendor data).

| Key | Default | Units | Notes |
|---|---|---|---|
| `preset` | from λ: ≥ 100 nm `refractive`, 12.4–15 nm `euv_hvm`, 6–7.5 nm `beuv_la_b`; any other λ is an error unless `preset` or the optics train is set | — | `euv_hvm` (10 mirrors × 0.70, mask 0.65), `beuv_la_b` (10 × 0.641, mask 0.641), `refractive` (optics 0.30, mask 0.90, slit 8 mm) |
| `optics_transmission` | preset | — | (0, 1]; **or** `n_mirrors` + `mirror_reflectivity` (transmission R^n), not both |
| `n_mirrors`, `mirror_reflectivity` | — | —, — | go together; n ≥ 1, R in (0, 1] |
| `mask_efficiency` | preset | — | (0, 1] |
| `dose_mj_cm2` | `[process] dose_mj_cm2`, else 30 | mJ/cm² | `--dose` overrides |
| `wafer_diameter_mm`, `field_width_mm`, `field_height_mm`, `slit_height_mm` | 300, 26, 33, 2 | mm | > 0 |
| `fields_per_wafer` | counted on the wafer | — | ≥ 1 |
| `max_scan_speed_mm_s` | none | mm/s | > 0 |
| `field_overhead_s`, `wafer_overhead_s` | 0.1, 10 | s | ≥ 0 |
| `cd_nm` | drawn `[mask]` width | nm | photon-statistics square |

The bands are where each preset's optics are a real projection train (refractive CaF₂/MgF₂/LiF optics transmit down to ~105 nm; Mo/Si mirrors from the Si L-edge at 12.4 nm to ~15 nm; La/B mirrors just above the B K-edge at 6.6 nm); Python's `api.wafer_throughput` uses the same bands. Outside every band (X-ray tubes and the betatron, which print by proximity / LIGA; the 25–47 nm LPA-FEL, Ar HHG and Ar or Mo soft-X-ray-laser lines) the run stops with an error rather than quote a mirror-train figure. Setting `preset` explicitly runs anyway, with a `hypothetical: …` note; setting only the optics train (`optics_transmission`, or `n_mirrors` + `mirror_reflectivity`) runs with the nearest preset's mask and field values and a note saying so.

## Validation order

`SimConfig::validate` (called by every run command before any work): strict keys and value ranges of `[source]`, `[illumination]`, `[optics]`, `[mask]` → build the source (every per-family check: known tags, odd harmonics, HHG cutoff, SSMB harmonic consistency, derived-wavelength cross-checks) → build the optics → grid (`size` a power of two, `pixel_nm` > 0) → `[process]` ranges → `[imaging]` settings. Subcommand tables (`[deep]`, `[optimize]`, `[throughput]`) are then validated by their subcommand. The first failure aborts with its message.

## Examples

Every file in [`examples/`](https://github.com/sonoransun/highuvlith/tree/main/examples) runs through the CLI:

| Config | Subcommand | What it shows |
|---|---|---|
| [`sim.toml`](../examples/sim.toml) | `simulate` | F₂ 157.63 nm, legacy `[source]` without `type` |
| [`sim_arf.toml`](../examples/sim_arf.toml), [`sim_krf.toml`](../examples/sim_krf.toml), [`sim_hg_iline.toml`](../examples/sim_hg_iline.toml), [`sim_hg_gline.toml`](../examples/sim_hg_gline.toml) | `simulate` | DUV / UV heritage presets |
| [`sim_lpa_fel.toml`](../examples/sim_lpa_fel.toml) | `simulate` | LPA-FEL with undulator + beam |
| [`sim_lpp_sn.toml`](../examples/sim_lpp_sn.toml), [`sim_lpp_gd.toml`](../examples/sim_lpp_gd.toml), [`sim_dpp.toml`](../examples/sim_dpp.toml) | `simulate` | plasma sources |
| [`sim_synchrotron.toml`](../examples/sim_synchrotron.toml), [`sim_hhg.toml`](../examples/sim_hhg.toml), [`sim_xfel.toml`](../examples/sim_xfel.toml), [`sim_xfel_cw_sc.toml`](../examples/sim_xfel_cw_sc.toml), [`sim_xfel_erl.toml`](../examples/sim_xfel_erl.toml), [`sim_sxrl.toml`](../examples/sim_sxrl.toml) | `simulate` | coherent / derived-wavelength sources (`euv_projection` optics) |
| [`sim_ics.toml`](../examples/sim_ics.toml), [`sim_ssmb.toml`](../examples/sim_ssmb.toml), [`sim_smith_purcell.toml`](../examples/sim_smith_purcell.toml), [`sim_entangled.toml`](../examples/sim_entangled.toml) | `simulate` | 🧪 sources |
| [`liga.toml`](../examples/liga.toml), [`sim_xray_tube.toml`](../examples/sim_xray_tube.toml), [`sim_betatron.toml`](../examples/sim_betatron.toml) | `deep` | LIGA with bending magnet / tube / betatron spectra |
| [`grayscale.toml`](../examples/grayscale.toml), [`interference.toml`](../examples/interference.toml), [`volumetric.toml`](../examples/volumetric.toml), [`talbot.toml`](../examples/talbot.toml) | `deep` | the other deep modes |
| [`optimize_ilt.toml`](../examples/optimize_ilt.toml), [`optimize_opc.toml`](../examples/optimize_opc.toml), [`optimize_sraf.toml`](../examples/optimize_sraf.toml), [`optimize_sadp.toml`](../examples/optimize_sadp.toml) | `optimize` | ILT, fragment OPC, SRAF, SADP |

Any `simulate` config also works with `sweep` and `throughput`. For the commands and output formats see the [CLI reference](./cli.md).
