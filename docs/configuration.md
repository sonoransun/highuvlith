# TOML Configuration Reference

**Status:** ✅ Implemented — the loader, defaults, and validation described here are all live in [`crates/highuvlith-cli/src/config.rs`](../crates/highuvlith-cli/src/config.rs). Individual source families and optics span the full honesty spectrum; each row below links to its status in the [capability matrix](./capability-matrix.md).

This page is the complete reference for the TOML files consumed by the [CLI](./cli.md) (`highuvlith simulate` / `sweep`). One file describes one simulation: a source, an optical system, a mask, a grid, and a set of process conditions. The [Python API](./python-api.md) and [GUI](./gui.md) build the same underlying configuration objects programmatically and do not read these files.

## File structure

A config is a TOML document with up to five tables:

```toml
[source]   # illumination — required
[optics]   # projection optics — required
[mask]     # pattern geometry — required
[grid]     # simulation grid — optional
[process]  # dose / focus conditions — optional
```

- `[source]`, `[optics]`, and `[mask]` **must be present**. Every key inside them is optional and falls back to a documented default, so an empty `[optics]` table is legal, but the header itself has to appear.
- `[grid]` and `[process]` may be **omitted entirely**; the whole table defaults.

Deserialization uses [`toml`](https://docs.rs/toml) with `serde`. Unknown keys inside a table are ignored (serde default); unknown values for the `type` discriminators are rejected during validation (see [Validation](#validation)).

`SimConfig::load` reads and parses the file; `SimConfig::validate` then runs every physical and structural check before any simulation work begins. Both the `simulate` and `sweep` commands call `validate` first, so a malformed config fails fast with a descriptive error rather than producing a silently wrong image.

## `[source]`

The source table is the only one with a `type` discriminator. It selects one of **nine source families**; all other keys are family-specific and consulted only when the matching `type` is active.

```toml
[source]
type = "vuv"   # optional — defaults to "vuv" when absent
```

### Back-compatibility

Omitting `type` entirely selects `"vuv"`. This keeps every pre-multi-source config working unchanged — a `[source]` table with only `wavelength_nm`, `sigma`, and `bandwidth_pm` still deserializes and runs as an F₂/Ar₂ excimer source. See [`examples/sim.toml`](../examples/sim.toml), which omits `type`.

### Shared keys

These keys are read (where meaningful) by more than one family:

| Key | Type | Default | Units | Notes |
|-----|------|---------|-------|-------|
| `type` | string | `"vuv"` | — | Family discriminator (see below). |
| `wavelength_nm` | float | family-specific | nm | Center/selected wavelength. **Derived** (not set) for synchrotron undulator and HHG — see those families. Must be `> 0`. |
| `sigma` | float | `0.7` | — | Partial-coherence outer σ for conventional illumination. Must be in `(0, 1.0]`. |
| `bandwidth_pm` | float | family-specific | pm FWHM | Spectral bandwidth. |

### Family discriminators

| `type` | Family | Status | Wavelength source |
|--------|--------|--------|-------------------|
| `"vuv"` | VUV excimer (F₂ 157.63 nm, Ar₂ 126 nm) | ✅ | `wavelength_nm` (set) |
| `"lpa_fel"` | Laser-plasma driven FEL (20–30 nm) | 🔶 | `wavelength_nm` (set) |
| `"lpp"` | Laser-produced plasma (Sn/Gd/Tb) | ✅ | fuel preset, `wavelength_nm` override |
| `"synchrotron"` | Bending-magnet / undulator | ✅ | **derived** (undulator) / selected (BM) |
| `"hhg"` | High-harmonic generation | ✅/🔶 | **derived** from driver / harmonic |
| `"xfel"` | SASE / self-seeded FEL | ✅ | `wavelength_nm` (set) |
| `"ics"` | Inverse Compton scattering | 🧪 | `wavelength_nm` (target; e-energy derived) |
| `"ssmb"` | Steady-state microbunching | 🧪 | `wavelength_nm` (set) |
| `"entangled"` | Entangled-photon NOON source | 🧪 | `wavelength_nm` (set) |

Any other `type` value is rejected: `unknown source type '<x>' (expected one of: vuv, lpa_fel, lpp, synchrotron, hhg, xfel, ics, ssmb, entangled)`.

---

#### `type = "vuv"` — VUV excimer ✅

Excimer gas-discharge laser. `spectral_samples` (5), `pulse_energy_mj` (10), and `spectral_shape` (Lorentzian) are fixed by the builder and not configurable from TOML.

| Key | Default | Units | |
|-----|---------|-------|--|
| `wavelength_nm` | `157.63` | nm | F₂ line; use `126.0` for Ar₂. |
| `bandwidth_pm` | `1.1` | pm | |
| `sigma` | `0.7` | — | |
| `rep_rate_hz` | `4000.0` | Hz | Feeds average-power / throughput. |

```toml
[source]
# type = "vuv" implied
wavelength_nm = 157.63
sigma = 0.7
bandwidth_pm = 1.1
```

See [`examples/sim.toml`](../examples/sim.toml).

---

#### `type = "lpa_fel"` — Laser-plasma driven FEL 🔶

BELLA-class laser-plasma FEL targeting the 20–30 nm EUV band. Built from `LpaFelSource::new(wavelength, sigma)`, then the pulse/electron fields override the preset.

| Key | Default | Units | |
|-----|---------|-------|--|
| `wavelength_nm` | `25.0` | nm | |
| `sigma` | `0.7` | — | |
| `bandwidth_pm` | preset (~25) | pm | Clamped `≥ 0`. |
| `electron_energy_mev` | preset | MeV | Documentary + throughput. |
| `pulse_duration_fs` | preset | fs | |
| `rep_rate_hz` | preset | Hz | |
| `pulse_energy_uj` | preset | µJ | Feeds average power + shot-to-shot dose jitter. |

See [`examples/sim_lpa_fel.toml`](../examples/sim_lpa_fel.toml).

---

#### `type = "lpp"` — Laser-produced plasma ✅

In-band plasma source. `fuel` picks the preset (`sn_13nm5` / `gd_6nm7` / `tb_6nm5`); the power-chain keys override it.

| Key | Default | Units | |
|-----|---------|-------|--|
| `fuel` | `"sn"` | — | `"sn"` (13.5 nm), `"gd"` (6.7 nm), `"tb"` (6.5 nm). Unknown fuel is rejected. |
| `wavelength_nm` | preset | nm | Override the in-band center. |
| `bandwidth_pm` | preset | pm | |
| `drive_laser_power_w` | preset | W | Power chain: drive × CE × transport. |
| `conversion_efficiency` | preset | — | In-band CE. |
| `transport_efficiency` | preset | — | Optics/transport throughput. |
| `rep_rate_hz` | preset | Hz | |
| `sigma` | `0.7` | — | |

```toml
[source]
type = "lpp"
fuel = "sn"              # 13.5 nm Sn plasma
drive_laser_power_w = 27000.0
conversion_efficiency = 0.05
transport_efficiency = 0.2
```

See [`examples/sim_lpp_sn.toml`](../examples/sim_lpp_sn.toml) (Sn 13.5 nm) and [`examples/sim_lpp_gd.toml`](../examples/sim_lpp_gd.toml) (Gd 6.7 nm).

---

#### `type = "synchrotron"` — Bending magnet / undulator ✅

`beamline` selects the emitter. The **undulator wavelength is derived** from the resonance condition λ = λ_u(1 + K²/2)/(2nγ²) — it is not set by `wavelength_nm`.

`beamline = "undulator"` (default):

| Key | Default | Units | |
|-----|---------|-------|--|
| `electron_energy_gev` | `0.538` | GeV | |
| `period_mm` | `20.0` | mm | Undulator period λ_u. |
| `undulator_k` | `1.0` | — | Deflection parameter K. |
| `num_periods` | `100` | — | |
| `harmonic` | `1` | — | Harmonic order n. |
| `ring_current_ma` | preset | mA | Flux scaling. |
| `wavelength_nm` | *(cross-check only)* | nm | If given, must agree with the derived resonance within **5%** or the config is rejected. |

`beamline = "bending_magnet"`:

| Key | Default | Units | |
|-----|---------|-------|--|
| `electron_energy_gev` | `2.5` | GeV | |
| `field_t` | `1.5` | T | Dipole field → critical energy E_c. |
| `wavelength_nm` | `0.2` | nm | Monochromator-selected wavelength. |
| `bandwidth_pm` | `0.2` | pm | Monochromator bandwidth. |
| `ring_current_ma` | `200.0` | mA | |

```toml
[source]
type = "synchrotron"
beamline = "undulator"
electron_energy_gev = 0.538
period_mm = 20.0
undulator_k = 1.0
harmonic = 1
# wavelength_nm = 13.5   # optional; rejected if >5% off the derived resonance
```

See [`examples/sim_synchrotron.toml`](../examples/sim_synchrotron.toml) (undulator; bending-magnet variant commented inline).

---

#### `type = "hhg"` — High-harmonic generation ✅/🔶

Table-top HHG. The **wavelength is derived** as `driver_wavelength_nm / harmonic`. Even harmonics and harmonics beyond the three-step cutoff (I_p + 3.17 U_p) are rejected.

| Key | Default | Units | |
|-----|---------|-------|--|
| `gas` | `"neon"` | — | `helium`/`he`, `neon`/`ne`, `argon`/`ar`, `krypton`/`kr`, `xenon`/`xe`. Unknown gas rejected. |
| `driver_wavelength_nm` | `800.0` | nm | Ti:Sapphire driver. |
| `driver_intensity_w_cm2` | `4e14` | W/cm² | Sets the cutoff via ponderomotive energy. |
| `harmonic` | `59` | — | Must be **odd** and within the cutoff. |
| `monochromator_bandwidth_pm` | `15.0` | pm | |
| `full_comb` | `false` | — | `true` removes the monochromator → full-comb spectral bookkeeping (not honest imaging). |
| `pulse_energy_nj` | preset | nJ | |
| `rep_rate_hz` | preset | Hz | |
| `pulse_duration_fs` | preset | fs | |

```toml
[source]
type = "hhg"
gas = "ne"
driver_wavelength_nm = 800.0
driver_intensity_w_cm2 = 4.0e14
harmonic = 59              # -> 800/59 ≈ 13.56 nm
monochromator_bandwidth_pm = 15.0
```

See [`examples/sim_hhg.toml`](../examples/sim_hhg.toml) (Ne harmonic 59 → 13.56 nm, cutoff-law demo).

---

#### `type = "xfel"` — SASE / self-seeded FEL ✅

Built from the FLASH-class 13.5 nm preset; `mode` selects the statistics.

| Key | Default | Units | |
|-----|---------|-------|--|
| `mode` | `"sase"` | — | `"sase"` or `"seeded"`/`"self_seeded"`. Unknown mode rejected. |
| `pierce_parameter` | `3e-3` | — | SASE only; bandwidth ≈ 2ρ. |
| `rel_bandwidth` | `5e-5` | — | Seeded only. |
| `wavelength_nm` | preset (13.5) | nm | |
| `pulse_energy_uj` | preset | µJ | |
| `pulse_duration_fs` | preset | fs | |
| `rep_rate_hz` | preset | Hz | |

---

#### `type = "ics"` — Inverse Compton scattering 🧪

Compton back-scattering source. The **electron energy is derived** from the Compton kinematics for the target `wavelength_nm`.

| Key | Default | Units | |
|-----|---------|-------|--|
| `wavelength_nm` | `13.5` | nm | Target (scattered) wavelength. |
| `laser_wavelength_nm` | `1030.0` | nm | Scattering laser. |
| `laser_a0` | `0.1` | — | Normalized laser strength. |
| `collection_half_angle_mrad` | preset | mrad | |
| `electron_energy_spread_rel` | preset | — | |
| `pulse_energy_nj` | preset | nJ | |
| `rep_rate_hz` | preset | Hz | |

---

#### `type = "ssmb"` — Steady-state microbunching 🧪

Storage-ring SSMB design point. `SsmbSource::new` runs a harmonic-consistency check between the modulation and radiation wavelengths.

| Key | Default | Units | |
|-----|---------|-------|--|
| `ring_energy_mev` | `400.0` | MeV | |
| `modulation_wavelength_nm` | `1053.0` | nm | Modulation laser. |
| `wavelength_nm` | `13.5` | nm | Radiation wavelength. |
| `average_power_w` | `1000.0` | W | Projected kW-class power. |

---

#### `type = "entangled"` — Entangled-photon NOON source 🧪

Bridges the quantum-lithography research module; reports the physical wavelength.

| Key | Default | Units | |
|-----|---------|-------|--|
| `wavelength_nm` | `157.63` | nm | Physical (not λ/2N) wavelength. |
| `num_photons` | `2` | — | N in the NOON state. |
| `fidelity` | `1.0` | — | Entanglement fidelity in [0, 1]. |

## `[optics]`

The `type` tag selects the optical system. The wavelength used for warnings and preset selection comes from the **resolved source** (`to_source().wavelength_nm()`), so it already reflects derived undulator/HHG wavelengths.

| Key | Type | Default | Units | |
|-----|------|---------|-------|--|
| `type` | string | `"refractive"` | — | `"refractive"`, `"schwarzschild"`, or `"zone_plate"`. Unknown value rejected. |
| `na` | float | `0.75` | — | Numerical aperture; must be in `(0, 1.0]`. |
| `flare_fraction` | float | `0.02` | — | Stray-light fraction (refractive/Schwarzschild). |
| `outer_zone_width_nm` | float | `λ / (2·NA)` | nm | Zone-plate outermost zone width (`zone_plate` only). |

### `type` tags

- **`"refractive"`** (default) — CaF₂ projection lens. **Physically valid only for VUV.** Below 50 nm no transparent lens material exists, so a warning is printed to stderr and the results are not physical:
  `warning: refractive CaF2 optics selected at <λ> nm — no transparent lens material exists below ~110 nm; results are not physical. Set [optics] type = "schwarzschild" or "zone_plate".`
- **`"schwarzschild"`** — two-mirror reflective objective for EUV/BEUV/soft X-ray. The band preset is auto-chosen from the wavelength (`< 3 nm` soft-X-ray, `< 10 nm` BEUV, otherwise EUV standard), then your `na` and `flare_fraction` are applied on top.
- **`"zone_plate"`** — Fresnel diffractive optic for X-ray. `outer_zone_width_nm` defaults to the diffraction-limited `λ/(2·NA)`.

```toml
# EUV / short-wavelength source: pick a physical optic explicitly
[optics]
type = "schwarzschild"
na = 0.33
flare_fraction = 0.02
```

This is the **optics honesty guard**: it is the one place the CLI actively warns when the physics does not support the requested configuration. The [GUI](./gui.md) enforces the same rule automatically by switching to Schwarzschild below 50 nm.

## `[mask]`

Line/space pattern (the CLI always builds `Mask::line_space`).

| Key | Type | Default | Units | |
|-----|------|---------|-------|--|
| `cd_nm` | float | `65.0` | nm | Line width. Must be `> 0` and `< pitch_nm`. |
| `pitch_nm` | float | `180.0` | nm | Line + space pitch. Must be `> 0`. |

## `[grid]`

Optional; the whole table defaults.

| Key | Type | Default | Units | |
|-----|------|---------|-------|--|
| `size` | int | `256` | pixels | Grid is `size × size`. Must be a **power of two** and non-zero. |
| `pixel_nm` | float | `1.0` | nm | Pixel pitch; must be `> 0`. Field size = `size · pixel_nm`. |

## `[process]`

Optional; the whole table defaults. Values can be overridden per-run by the CLI `--focus` / `--dose` flags and the `sweep` ranges.

| Key | Type | Default | Units | |
|-----|------|---------|-------|--|
| `dose_mj_cm2` | float | `30.0` | mJ/cm² | Exposure dose (used by `sweep` as the default dose point). |
| `focus_nm` | float | `0.0` | nm | Defocus. |

## `[deep]`

Drives the [`highuvlith deep`](./cli.md#deep) subcommand's four deep-layer process modes. This table is parsed **in addition to** the standard sections above (the `deep` command reads both `SimConfig` and `[deep]` from the same file), and it is consulted only by `deep` — `simulate` and `sweep` ignore it. Every key beyond `mode` is optional and falls back to a physically reasonable default, so a minimal `[deep] mode = "…"` still runs.

| Key | Type | Default | |
|-----|------|---------|--|
| `mode` | string | *(required)* | `"liga"`, `"grayscale"`, `"interference"`, or `"volumetric"`. Missing or unknown → error. |

The remaining keys are grouped by the mode that reads them. A key given for the wrong mode is simply ignored.

### `mode = "liga"` — deep-X-ray shadow printing 🔶

Needs a **bending-magnet white beam**: either set `critical_energy_kev` here, or use `[source] type = "synchrotron"` with `beamline = "bending_magnet"` (the critical energy is then derived from `field_t` / `electron_energy_gev`). A synchrotron *undulator* source is rejected.

| Key | Default | Units | |
|-----|---------|-------|--|
| `critical_energy_kev` | *(from source)* | keV | Bending-magnet critical energy. Must be `> 0` if given. |
| `resist_thickness_um` | PMMA preset | µm | LIGA is hundreds of µm thick. |
| `proximity_gap_um` | PMMA preset | µm | Mask-to-resist gap → Fresnel blur. |
| `min_feature_nm` | `5000.0` | nm | Minimum feature for the max-aspect-ratio figure. |
| `develop_threshold_kj_cm3` | target bottom dose | kJ/cm³ | Clearing threshold for the developed-depth map. |
| `nz` | `64` | — | Depth slices. |

### `mode = "grayscale"` — 2.5D topography 🔶

Synthesizes a continuous-transmittance mask that carves a relief, then re-prints it to report the achieved height and RMS error.

| Key | Default | Units | |
|-----|---------|-------|--|
| `target` | `"blazed"` | — | `"blazed"`, `"microlens"`, or `"staircase"`. Unknown → error. |
| `period_px` | `size / 4` | pixels | Blazed grating period. |
| `pitch_px` | `size / 4` | pixels | Microlens pitch. |
| `n_levels` | `8` | — | Staircase steps. |
| `depth_nm` | `0.6·thickness` (blazed), `0.8·thickness` (staircase) | nm | Peak-to-valley relief. |
| `sag_nm` | `0.6·thickness` | nm | Microlens sag. |
| `thickness_nm` | `1000.0` | nm | Resist film thickness. |
| `dose_mj_cm2` | `100.0` | mJ/cm² | Single-exposure dose for mask synthesis. |
| `d_th` | `10.0` | mJ/cm² | Contrast-curve threshold dose. |
| `d_clear` | `100.0` | mJ/cm² | Contrast-curve clearing dose. |

### `mode = "interference"` — multi-beam holographic ✅/🔶

Interferes plane waves into a periodic lattice. The lateral extent comes from `[grid]`; the wavelength from the resolved `[source]`.

| Key | Default | Units | |
|-----|---------|-------|--|
| `preset` | `"two_beam"` | — | `"two_beam"`, `"three_beam_hex"`, or `"four_beam_umbrella"`. Unknown → error. |
| `half_angle_deg` | `30.0` | deg | Air-side incidence half-angle (sets the fringe period). |
| `n_medium` | `1.6` | — | Recording-medium index (fringe period is index-invariant). |
| `two_photon` | `false` | — | `true` uses I² (multiphoton) kinetics. |
| `z_span_nm` | field size | nm | Recording depth. |
| `dose_scale` | `1.0` | — | Dose scale for the Dill dose→PAC map. |
| `dill_c` | `0.02` | — | Dill C coefficient. |
| `fill_threshold` | `0.5` | — | PAC threshold for the iso-surface fill fraction. |
| `nz` | `64` | — | Depth slices. |

### `mode = "volumetric"` — z-resolved exposure 🔶

Runs the full aerial-imaging pipeline through the resist depth with split-step bleaching, reporting per-slice CD and a sidewall angle. Base defocus comes from `[process].focus_nm`.

| Key | Default | Units | |
|-----|---------|-------|--|
| `resist_thickness_nm` | `300.0` | nm | Overrides the default film-stack resist layer. |
| `dose_mj_cm2` | `[process].dose_mj_cm2` | mJ/cm² | Exposure dose. |
| `nz` | `64` | — | Depth slices. |
| `n_defocus_planes` | `8` | — | Defocus planes for the volumetric aerial image. |
| `dose_steps` | `1` | — | Split-step Dill dose steps (1 = static absorption). |
| `develop_threshold` | `0.5` | — | PAC threshold for per-slice CD and the depth map. |

Mode-specific values are validated when the mode runs (e.g. `critical_energy_kev > 0`, valid contrast curve, known `target`/`preset`), not during `SimConfig::validate()`.

## Validation

`SimConfig::validate()` runs these checks, in order, and returns the first failure as a descriptive error:

1. `wavelength_nm > 0` (when present).
2. `na` in `(0, 1.0]`.
3. `sigma` in `(0, 1.0]`.
4. **`to_source()`** — constructs the source, which runs every per-family check:
   - unknown `type` tag rejected;
   - unknown `fuel` (LPP), `gas` (HHG), `beamline` (synchrotron), `mode` (XFEL) rejected;
   - HHG harmonic must be odd and within the three-step cutoff;
   - synchrotron undulator: explicit `wavelength_nm` cross-checked against the derived resonance (±5%);
   - SSMB harmonic consistency between modulation and radiation wavelengths.
5. `cd_nm > 0`.
6. `pitch_nm > 0`.
7. `cd_nm < pitch_nm`.
8. `grid.size` non-zero and a power of two.
9. `pixel_nm > 0`.

Note that the optics `type` and the refractive-below-50 nm warning are evaluated when the optical system is built (`to_optics()`), during the run rather than in `validate()` — an unphysical refractive selection warns but does not abort.

## Complete examples

Every source family and deep-layer mode has a heavily-commented runnable config in `examples/`:

| Config | What it shows |
|--------|---------------|
| [`sim.toml`](../examples/sim.toml) | F₂ excimer VUV, 157.63 nm, refractive optics. Omits `type` (back-compat default). |
| [`sim_lpa_fel.toml`](../examples/sim_lpa_fel.toml) | LPA-FEL at 25 nm (BELLA target), NA 0.55. |
| [`sim_lpp_sn.toml`](../examples/sim_lpp_sn.toml) / [`sim_lpp_gd.toml`](../examples/sim_lpp_gd.toml) | LPP Sn 13.5 nm / Gd 6.7 nm, Schwarzschild optics. |
| [`sim_synchrotron.toml`](../examples/sim_synchrotron.toml) | Undulator at 13.5 nm — **derived** wavelength + cross-check. |
| [`sim_hhg.toml`](../examples/sim_hhg.toml) | HHG Ne harmonic 59 → 13.56 nm — cutoff-law demo. |
| [`sim_xfel.toml`](../examples/sim_xfel.toml) | SASE XFEL at 13.5 nm. |
| [`sim_ics.toml`](../examples/sim_ics.toml) | Inverse Compton scattering (🧪). |
| [`sim_ssmb.toml`](../examples/sim_ssmb.toml) | Steady-state microbunching (🧪). |
| [`sim_entangled.toml`](../examples/sim_entangled.toml) | Entangled-photon NOON source (🧪). |
| [`liga.toml`](../examples/liga.toml) | `deep` LIGA — 500 µm PMMA, bending-magnet beam. |
| [`grayscale.toml`](../examples/grayscale.toml) | `deep` grayscale — blazed grating relief. |
| [`interference.toml`](../examples/interference.toml) | `deep` interference — two-beam grating at 266 nm. |
| [`volumetric.toml`](../examples/volumetric.toml) | `deep` volumetric — z-resolved F₂ exposure. |

For the source physics behind each family, see the [capability matrix](./capability-matrix.md); for how these files are consumed, see the [CLI reference](./cli.md).
