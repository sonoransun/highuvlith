//! TOML configuration of the `highuvlith` CLI.
//!
//! One file describes one run. Shared tables: `[source]`, `[optics]`,
//! `[mask]`, `[grid]`, `[process]`, `[imaging]`, `[illumination]`; tables
//! owned by one subcommand: `[deep]` (`highuvlith deep`), `[throughput]`
//! (`highuvlith throughput`), `[optimize]` (`highuvlith optimize`). Every
//! table is optional at the schema level; each subcommand requires the tables
//! it reads and says so. The reference is `docs/configuration.md`.
//!
//! **Strict keys** (see [`crate::keys`]): an unknown key is an error with a
//! did-you-mean hint, and so is a key that exists but is not read by the
//! selected `[source] type`, `[optics] type`, `[mask] pattern` or
//! `[illumination] shape` — silently ignored keys were a reported trap.
//! Back-compat: a `[source]` without `type` is the legacy VUV source.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use highuvlith_core::source::{IlluminationShape, SourceKind};

use crate::keys::{self, KeyRules, UnknownKeyHint};

// ============================================================================
// Top level
// ============================================================================

/// Top-level tables a config may contain.
#[cfg(test)]
pub const TABLES: &[&str] = &[
    "source",
    "optics",
    "mask",
    "grid",
    "process",
    "imaging",
    "illumination",
    "deep",
    "throughput",
    "optimize",
];

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimConfig {
    #[serde(default)]
    pub source: Option<SourceConfig>,
    #[serde(default)]
    pub optics: Option<OpticsConfig>,
    #[serde(default)]
    pub mask: Option<MaskConfig>,
    #[serde(default)]
    pub grid: GridConfig,
    #[serde(default)]
    pub process: ProcessConfig,
    #[serde(default)]
    pub imaging: ImagingConfig,
    #[serde(default)]
    pub illumination: Option<IlluminationConfig>,
    /// `[deep]` — accepted here; parsed (strict keys) and validated by
    /// `highuvlith deep`.
    #[serde(default)]
    #[allow(dead_code)]
    pub deep: Option<toml::Value>,
    /// `[throughput]` — accepted here; parsed and validated by
    /// `highuvlith throughput`.
    #[serde(default)]
    #[allow(dead_code)]
    pub throughput: Option<toml::Value>,
    /// `[optimize]` — accepted here; parsed and validated by
    /// `highuvlith optimize`.
    #[serde(default)]
    #[allow(dead_code)]
    pub optimize: Option<toml::Value>,
    /// Top-level tables present in the file (filled by [`SimConfig::from_toml_str`]).
    #[serde(skip)]
    pub tables: BTreeSet<String>,
}

// =====================================================================
/// `[source]`: one flat table for every source family, selected by `type`.
///
/// Back-compatible: without `type` the legacy VUV source is built
/// (`wavelength_nm`, `sigma`, `bandwidth_pm`, `rep_rate_hz`). Family-specific
/// keys are optional and default to each family's reference preset, but a
/// key the selected family does not read is rejected by
/// [`SimConfig::validate`] (see [`SourceConfig::family_keys`]).
///
/// For sources whose wavelength is DERIVED from machine parameters
/// (synchrotron undulator, HHG, X-ray tube, betatron, Smith–Purcell with a
/// grating period, soft-X-ray-laser lines) an explicit `wavelength_nm` is a
/// cross-check: it is rejected if it disagrees with the derived value.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SourceConfig {
    #[serde(rename = "type")]
    pub source_type: Option<String>,
    pub wavelength_nm: Option<f64>,
    pub sigma: Option<f64>,
    pub bandwidth_pm: Option<f64>,
    /// Spectral samples for polychromatic imaging (every family).
    pub spectral_samples: Option<usize>,
    // --- LPA-FEL / XFEL shared pulse fields ---
    pub electron_energy_mev: Option<f64>,
    pub pulse_duration_fs: Option<f64>,
    pub rep_rate_hz: Option<f64>,
    pub pulse_energy_uj: Option<f64>,
    // --- LPP (type = "lpp") ---
    /// Plasma fuel. `type = "lpp"`: "sn" (13.5 nm), "gd" (6.7 nm), or "tb"
    /// (6.5 nm); `type = "dpp"`: "sn" (default) or "xe".
    pub fuel: Option<String>,
    pub drive_laser_power_w: Option<f64>,
    pub conversion_efficiency: Option<f64>,
    pub transport_efficiency: Option<f64>,
    // --- Synchrotron (type = "synchrotron") ---
    /// Beamline: "undulator" (default) or "bending_magnet".
    pub beamline: Option<String>,
    pub electron_energy_gev: Option<f64>,
    pub field_t: Option<f64>,
    pub period_mm: Option<f64>,
    pub undulator_k: Option<f64>,
    pub num_periods: Option<usize>,
    /// Odd harmonic (synchrotron undulator and HHG).
    pub harmonic: Option<usize>,
    pub ring_current_ma: Option<f64>,
    // --- HHG (type = "hhg") ---
    /// Generation gas: "helium"/"he", "neon"/"ne", "argon"/"ar",
    /// "krypton"/"kr", or "xenon"/"xe".
    pub gas: Option<String>,
    pub driver_wavelength_nm: Option<f64>,
    pub driver_intensity_w_cm2: Option<f64>,
    pub monochromator_bandwidth_pm: Option<f64>,
    /// true = no monochromator: every harmonic of the comb is imaged.
    pub full_comb: Option<bool>,
    pub pulse_energy_nj: Option<f64>,
    // --- XFEL (type = "xfel") ---
    /// Mode: "sase" or "seeded" (default: the preset's).
    pub mode: Option<String>,
    pub pierce_parameter: Option<f64>,
    pub rel_bandwidth: Option<f64>,
    // --- ICS (type = "ics") ---
    pub laser_wavelength_nm: Option<f64>,
    pub laser_a0: Option<f64>,
    pub collection_half_angle_mrad: Option<f64>,
    pub electron_energy_spread_rel: Option<f64>,
    // --- SSMB (type = "ssmb") ---
    pub modulation_wavelength_nm: Option<f64>,
    pub average_power_w: Option<f64>,
    pub ring_energy_mev: Option<f64>,
    // --- Entangled (type = "entangled") ---
    pub num_photons: Option<usize>,
    pub fidelity: Option<f64>,
    // --- Machine-parameter physics of the existing families ---
    /// LPP drive laser: "co2", "solid_state_1um" (or "1um"), "thulium_2um" (or "2um").
    pub drive_laser: Option<String>,
    /// Emitting-plasma diameter in µm (LPP and DPP; converted for DPP).
    pub source_diameter_um: Option<f64>,
    /// Emitting-plasma diameter in mm (DPP and LPP; converted for LPP).
    pub source_diameter_mm: Option<f64>,
    /// Collector solid angle (sr) of LPP and DPP. `collector_solid_angle_sr`
    /// (the DPP spelling) is an alias.
    #[serde(alias = "collector_solid_angle_sr")]
    pub collection_solid_angle_sr: Option<f64>,
    /// Illuminator étendue budget (mm^2 sr) of the LPP and DPP étendue checks.
    pub illuminator_etendue_mm2_sr: Option<f64>,
    /// Synchrotron ring horizontal emittance (nm rad); derives the coherent fraction.
    pub emittance_x_nm_rad: Option<f64>,
    /// Synchrotron ring vertical emittance (nm rad).
    pub emittance_y_nm_rad: Option<f64>,
    /// FEL peak current (A); SSMB peak current.
    pub peak_current_a: Option<f64>,
    /// FEL normalized emittance (µm).
    pub norm_emittance_um: Option<f64>,
    /// FEL average beta function in the undulator (m).
    pub beta_m: Option<f64>,
    /// XFEL base preset: "flash" (default), "fermi", "cw_sc", or "erl".
    pub xfel_preset: Option<String>,
    /// XFEL undulator line length (m).
    pub undulator_length_m: Option<f64>,
    /// HHG driver average power (W); derives the harmonic power.
    pub driver_average_power_w: Option<f64>,
    /// HHG full-comb passband `[min_nm, max_nm]` (filter / multilayer band).
    pub comb_passband_nm: Option<[f64; 2]>,
    /// Electron bunch charge (pC): ICS collision and LWFA betatron.
    pub bunch_charge_pc: Option<f64>,
    /// ICS laser pulse energy at the interaction point (mJ).
    pub laser_pulse_energy_mj: Option<f64>,
    /// ICS rms electron spot size (µm).
    pub electron_spot_um: Option<f64>,
    /// ICS rms laser spot size (µm).
    pub laser_spot_um: Option<f64>,
    /// SSMB average stored current (A).
    pub average_current_a: Option<f64>,
    /// SSMB bunching factor |b_n| at the target harmonic.
    pub bunching_factor: Option<f64>,
    /// SSMB radiator periods.
    pub radiator_periods: Option<usize>,
    /// SSMB radiator K.
    pub radiator_k: Option<f64>,
    /// Entangled N-tuple rate (Hz).
    pub pair_rate_hz: Option<f64>,
    /// Named preset. `type = "vuv"`: "f2", "ar2", "arf", "krf", "hg_i",
    /// "hg_h", "hg_g" (explicit `wavelength_nm` / `bandwidth_pm` /
    /// `rep_rate_hz` then override the preset; without a preset the legacy
    /// explicit-wavelength VUV source is built). `type = "xfel"`: alias of
    /// `xfel_preset`. `type = "lpp"` (Sn only): "nxe3400b" (250 W at IF,
    /// the default) or "nxe3800e" (500 W at IF).
    pub preset: Option<String>,
    // --- X-ray tube (type = "xray_tube") ---
    /// Anode: "w", "mo", "cu", or "rh" (each has its own preset kVp/mA).
    pub anode: Option<String>,
    /// Tube voltage in kV (= maximum photon energy in keV).
    pub kvp: Option<f64>,
    /// Tube current in mA (same name as the Python kwarg and the serialized
    /// source; `tube_current_ma` accepted as an alias).
    #[serde(alias = "tube_current_ma")]
    pub current_ma: Option<f64>,
    /// Be exit-window thickness in µm.
    pub be_window_um: Option<f64>,
    // --- DPP / LDP (type = "dpp"; `fuel` = "sn" | "xe") ---
    pub electrical_power_w: Option<f64>,
    pub source_length_mm: Option<f64>,
    pub collector_efficiency: Option<f64>,
    // --- Soft-X-ray laser (type = "sxrl") ---
    /// Scheme: "ar_46nm9", "ag_13nm9", "cd_13nm2", or "mo_18nm9".
    pub scheme: Option<String>,
    pub pulse_duration_ps: Option<f64>,
    /// Relative linewidth Δλ/λ (same name as the Python kwarg and the
    /// serialized source; the shared `rel_bandwidth` key is accepted as a
    /// fallback).
    pub rel_linewidth: Option<f64>,
    // --- Betatron (type = "betatron"; `electron_energy_mev`) ---
    pub plasma_density_cm3: Option<f64>,
    pub betatron_amplitude_um: Option<f64>,
    pub interaction_length_mm: Option<f64>,
    // --- Smith-Purcell (type = "smith_purcell"; `num_periods`) ---
    pub electron_energy_kev: Option<f64>,
    pub grating_period_nm: Option<f64>,
    pub diffraction_order: Option<usize>,
    pub observation_angle_deg: Option<f64>,
    pub beam_current_na: Option<f64>,
    pub impact_height_nm: Option<f64>,
    pub coupling_efficiency: Option<f64>,
}

/// Every `[source] type` tag.
pub const SOURCE_TYPES: &[&str] = &[
    "vuv",
    "lpa_fel",
    "lpp",
    "synchrotron",
    "hhg",
    "xfel",
    "ics",
    "ssmb",
    "entangled",
    "xray_tube",
    "dpp",
    "sxrl",
    "betatron",
    "smith_purcell",
];

/// `(type, beamline)` contexts with their own key sets.
const SOURCE_CONTEXTS: &[(&str, Option<&str>)] = &[
    ("vuv", None),
    ("lpa_fel", None),
    ("lpp", None),
    ("synchrotron", Some("undulator")),
    ("synchrotron", Some("bending_magnet")),
    ("hhg", None),
    ("xfel", None),
    ("ics", None),
    ("ssmb", None),
    ("entangled", None),
    ("xray_tube", None),
    ("dpp", None),
    ("sxrl", None),
    ("betatron", None),
    ("smith_purcell", None),
];

const VUV_KEYS: &[&str] = &[
    "type",
    "preset",
    "wavelength_nm",
    "bandwidth_pm",
    "sigma",
    "rep_rate_hz",
    "spectral_samples",
];
const LPA_FEL_KEYS: &[&str] = &[
    "type",
    "wavelength_nm",
    "sigma",
    "bandwidth_pm",
    "spectral_samples",
    "electron_energy_mev",
    "pulse_duration_fs",
    "rep_rate_hz",
    "pulse_energy_uj",
    "period_mm",
    "undulator_k",
    "num_periods",
    "peak_current_a",
    "norm_emittance_um",
    "electron_energy_spread_rel",
    "beta_m",
];
const LPP_KEYS: &[&str] = &[
    "type",
    "fuel",
    "preset",
    "drive_laser",
    "wavelength_nm",
    "bandwidth_pm",
    "sigma",
    "spectral_samples",
    "drive_laser_power_w",
    "conversion_efficiency",
    "transport_efficiency",
    "rep_rate_hz",
    "source_diameter_um",
    "source_diameter_mm",
    "collection_solid_angle_sr",
    "illuminator_etendue_mm2_sr",
];
const SYNCHROTRON_UNDULATOR_KEYS: &[&str] = &[
    "type",
    "beamline",
    "electron_energy_gev",
    "period_mm",
    "undulator_k",
    "num_periods",
    "harmonic",
    "ring_current_ma",
    "emittance_x_nm_rad",
    "emittance_y_nm_rad",
    "electron_energy_spread_rel",
    "wavelength_nm",
    "spectral_samples",
];
const SYNCHROTRON_BM_KEYS: &[&str] = &[
    "type",
    "beamline",
    "electron_energy_gev",
    "field_t",
    "ring_current_ma",
    "wavelength_nm",
    "bandwidth_pm",
    "spectral_samples",
];
const HHG_KEYS: &[&str] = &[
    "type",
    "gas",
    "driver_wavelength_nm",
    "driver_intensity_w_cm2",
    "harmonic",
    "monochromator_bandwidth_pm",
    "full_comb",
    "comb_passband_nm",
    "pulse_energy_nj",
    "rep_rate_hz",
    "pulse_duration_fs",
    "driver_average_power_w",
    "conversion_efficiency",
    "wavelength_nm",
    "spectral_samples",
];
/// XFEL machine keys (they re-derive ρ, the gain length and the saturation power).
const XFEL_MACHINE_KEYS: &[&str] = &[
    "electron_energy_mev",
    "period_mm",
    "undulator_length_m",
    "peak_current_a",
    "norm_emittance_um",
    "electron_energy_spread_rel",
    "beta_m",
];
const XFEL_KEYS: &[&str] = &[
    "type",
    "xfel_preset",
    "preset",
    "mode",
    "pierce_parameter",
    "rel_bandwidth",
    "wavelength_nm",
    "pulse_energy_uj",
    "pulse_duration_fs",
    "rep_rate_hz",
    "electron_energy_mev",
    "period_mm",
    "undulator_length_m",
    "peak_current_a",
    "norm_emittance_um",
    "electron_energy_spread_rel",
    "beta_m",
    "spectral_samples",
];
/// ICS collision keys (they derive the photon yield).
const ICS_COLLISION_KEYS: &[&str] = &[
    "bunch_charge_pc",
    "laser_pulse_energy_mj",
    "electron_spot_um",
    "laser_spot_um",
];
const ICS_KEYS: &[&str] = &[
    "type",
    "wavelength_nm",
    "laser_wavelength_nm",
    "laser_a0",
    "electron_energy_mev",
    "collection_half_angle_mrad",
    "electron_energy_spread_rel",
    "pulse_energy_nj",
    "rep_rate_hz",
    "bunch_charge_pc",
    "laser_pulse_energy_mj",
    "electron_spot_um",
    "laser_spot_um",
    "spectral_samples",
];
const SSMB_KEYS: &[&str] = &[
    "type",
    "ring_energy_mev",
    "modulation_wavelength_nm",
    "wavelength_nm",
    "average_power_w",
    "average_current_a",
    "peak_current_a",
    "bunching_factor",
    "radiator_periods",
    "radiator_k",
    "spectral_samples",
];
const ENTANGLED_KEYS: &[&str] = &[
    "type",
    "wavelength_nm",
    "num_photons",
    "fidelity",
    "pair_rate_hz",
    "spectral_samples",
];
const XRAY_TUBE_KEYS: &[&str] = &[
    "type",
    "anode",
    "kvp",
    "current_ma",
    "be_window_um",
    "wavelength_nm",
    "spectral_samples",
];
const DPP_KEYS: &[&str] = &[
    "type",
    "fuel",
    "sigma",
    "wavelength_nm",
    "bandwidth_pm",
    "spectral_samples",
    "electrical_power_w",
    "conversion_efficiency",
    "source_diameter_mm",
    "source_diameter_um",
    "source_length_mm",
    "collection_solid_angle_sr",
    "collector_efficiency",
    "illuminator_etendue_mm2_sr",
    "rep_rate_hz",
];
const SXRL_KEYS: &[&str] = &[
    "type",
    "scheme",
    "rel_linewidth",
    "rel_bandwidth",
    "pulse_energy_uj",
    "rep_rate_hz",
    "pulse_duration_ps",
    "wavelength_nm",
    "spectral_samples",
];
const BETATRON_KEYS: &[&str] = &[
    "type",
    "electron_energy_mev",
    "plasma_density_cm3",
    "betatron_amplitude_um",
    "interaction_length_mm",
    "bunch_charge_pc",
    "rep_rate_hz",
    "pulse_duration_fs",
    "wavelength_nm",
    "spectral_samples",
];
const SMITH_PURCELL_KEYS: &[&str] = &[
    "type",
    "wavelength_nm",
    "grating_period_nm",
    "electron_energy_kev",
    "diffraction_order",
    "observation_angle_deg",
    "num_periods",
    "beam_current_na",
    "impact_height_nm",
    "coupling_efficiency",
    "collection_half_angle_mrad",
    "electron_energy_spread_rel",
    "spectral_samples",
];

/// Relative tolerance of the derived-wavelength cross-checks.
const DERIVED_WAVELENGTH_RTOL: f64 = 0.05;
/// HHG harmonics are only 2/q apart (3.4 % at q = 59), so its cross-check is tighter.
const HHG_WAVELENGTH_RTOL: f64 = 0.01;

impl SourceConfig {
    pub fn source_type(&self) -> &str {
        self.source_type.as_deref().unwrap_or("vuv")
    }

    pub fn sigma(&self) -> f64 {
        self.sigma.unwrap_or(0.7)
    }

    /// Keys read by a `(type, beamline)` context before any conditional
    /// rule; `None` for an unknown type or beamline.
    pub fn family_keys(tag: &str, beamline: Option<&str>) -> Option<&'static [&'static str]> {
        Some(match tag {
            "vuv" => VUV_KEYS,
            "lpa_fel" => LPA_FEL_KEYS,
            "lpp" => LPP_KEYS,
            "synchrotron" => match beamline.unwrap_or("undulator") {
                "undulator" => SYNCHROTRON_UNDULATOR_KEYS,
                "bending_magnet" => SYNCHROTRON_BM_KEYS,
                _ => return None,
            },
            "hhg" => HHG_KEYS,
            "xfel" => XFEL_KEYS,
            "ics" => ICS_KEYS,
            "ssmb" => SSMB_KEYS,
            "entangled" => ENTANGLED_KEYS,
            "xray_tube" => XRAY_TUBE_KEYS,
            "dpp" => DPP_KEYS,
            "sxrl" => SXRL_KEYS,
            "betatron" => BETATRON_KEYS,
            "smith_purcell" => SMITH_PURCELL_KEYS,
            _ => return None,
        })
    }

    fn context_label(tag: &str, beamline: Option<&str>) -> String {
        match (tag, beamline) {
            ("synchrotron", b) => format!(
                "type = \"synchrotron\", beamline = \"{}\"",
                b.unwrap_or("undulator")
            ),
            _ => format!("type = \"{tag}\""),
        }
    }

    /// The contexts that read `key` (for "it is only read by ..." hints).
    pub fn key_owners(key: &str) -> Vec<String> {
        SOURCE_CONTEXTS
            .iter()
            .filter(|(t, b)| Self::family_keys(t, *b).is_some_and(|keys| keys.contains(&key)))
            .map(|(t, b)| Self::context_label(t, *b))
            .collect()
    }

    fn unknown_type_error(tag: &str) -> anyhow::Error {
        anyhow::anyhow!(
            "unknown source type '{}' (expected one of: {})",
            tag,
            SOURCE_TYPES.join(", ")
        )
    }

    /// The effective XFEL preset tag (`xfel_preset`, or its alias `preset`).
    fn xfel_preset_tag(&self) -> anyhow::Result<&str> {
        match (self.xfel_preset.as_deref(), self.preset.as_deref()) {
            (Some(a), Some(b)) if a != b => anyhow::bail!(
                "[source] xfel_preset = \"{a}\" and preset = \"{b}\" disagree (preset is an alias \
                 of xfel_preset for type = \"xfel\"; give one)"
            ),
            (Some(a), _) => Ok(a),
            (None, Some(b)) => Ok(b),
            (None, None) => Ok("flash"),
        }
    }

    /// The keys this source reads, after the conditional rules (a key that
    /// only takes effect together with another one, or that another key
    /// replaces). `pupil_fill_from_illumination`: an `[illumination]` table
    /// sets the pupil fill, so `sigma` is not read.
    pub fn key_rules(&self, pupil_fill_from_illumination: bool) -> anyhow::Result<KeyRules> {
        let tag = self.source_type();
        let beamline = self.beamline.as_deref();
        let keys = Self::family_keys(tag, beamline).ok_or_else(|| {
            if tag == "synchrotron" {
                anyhow::anyhow!(
                    "unknown synchrotron beamline '{}' (expected 'undulator' or 'bending_magnet')",
                    beamline.unwrap_or_default()
                )
            } else {
                Self::unknown_type_error(tag)
            }
        })?;
        let mut rules = KeyRules::new("[source]", Self::context_label(tag, beamline), keys);
        if pupil_fill_from_illumination {
            rules.exclude(
                "sigma",
                "the pupil fill comes from the [illumination] table (remove one of them)",
            );
        }
        match tag {
            "lpa_fel" => {
                if self.period_mm.is_none() && self.undulator_k.is_none() {
                    rules.exclude_all(
                        &[
                            "num_periods",
                            "peak_current_a",
                            "norm_emittance_um",
                            "electron_energy_spread_rel",
                            "beta_m",
                        ],
                        "the undulator and beam keys need an undulator (give period_mm and \
                         undulator_k)",
                    );
                }
            }
            "synchrotron" if beamline.unwrap_or("undulator") == "undulator" => {
                if self.emittance_x_nm_rad.is_none() && self.emittance_y_nm_rad.is_none() {
                    rules.exclude(
                        "electron_energy_spread_rel",
                        "the ring energy spread is read together with emittance_x_nm_rad / \
                         emittance_y_nm_rad (the ring beam)",
                    );
                }
            }
            "hhg" => {
                if self.full_comb == Some(true) {
                    let reason = "full_comb = true removes the monochromator and images every \
                                  harmonic of the comb";
                    rules.exclude_all(&["monochromator_bandwidth_pm", "harmonic"], reason);
                    rules.exclude(
                        "wavelength_nm",
                        "the full comb has no single wavelength (it is the comb's median line)",
                    );
                } else {
                    rules.exclude(
                        "comb_passband_nm",
                        "the passband filters the comb only with full_comb = true",
                    );
                }
                if self.driver_average_power_w.is_some() {
                    rules.exclude(
                        "pulse_energy_nj",
                        "the pulse energy is derived from driver_average_power_w (remove one)",
                    );
                }
            }
            "xfel" => {
                let preset = self.xfel_preset_tag()?;
                let seeded = match self.mode.as_deref() {
                    Some(m) => matches!(m, "seeded" | "self_seeded"),
                    None => matches!(preset, "fermi" | "fermi_seeded"),
                };
                if seeded {
                    rules.exclude(
                        "pierce_parameter",
                        "the Pierce parameter sets the SASE bandwidth (mode = \"sase\"); seeded \
                         mode uses rel_bandwidth",
                    );
                } else {
                    rules.exclude(
                        "rel_bandwidth",
                        "rel_bandwidth is the seeded-mode bandwidth; SASE derives it from ρ",
                    );
                }
                if self.pierce_parameter.is_some() {
                    rules.exclude_all(
                        XFEL_MACHINE_KEYS,
                        "an explicit pierce_parameter disables the machine derivation that these \
                         keys feed",
                    );
                }
            }
            "ics" if ICS_COLLISION_KEYS.iter().any(|k| self.is_set(k)) => {
                rules.exclude(
                    "pulse_energy_nj",
                    "the pulse energy is derived from the collision keys (bunch_charge_pc, \
                     laser_pulse_energy_mj, electron_spot_um, laser_spot_um)",
                );
            }
            "sxrl" if self.rel_linewidth.is_some() => {
                rules.exclude(
                    "rel_bandwidth",
                    "rel_bandwidth is the fallback spelling of rel_linewidth (give one)",
                );
            }
            // bunching_factor is itself a radiator key: with it the power is derived.
            "ssmb" if self.bunching_factor.is_some() => {
                rules.exclude(
                    "average_power_w",
                    "with the radiator keys and an explicit bunching_factor the power is \
                     derived (P ∝ b²); average_power_w only sets the b the projection needs \
                     when bunching_factor is omitted",
                );
            }
            _ => {}
        }
        Ok(rules)
    }

    /// Whether the key is set (by its canonical name).
    fn is_set(&self, key: &str) -> bool {
        keys::present_keys(self).iter().any(|k| k == key)
    }

    /// Reject keys this source does not read.
    pub fn check_keys(&self, pupil_fill_from_illumination: bool) -> anyhow::Result<()> {
        self.key_rules(pupil_fill_from_illumination)?
            .check(&keys::present_keys(self), &Self::key_owners)
    }

    /// FEL electron-beam parameters, if any beam field is given; missing
    /// fields take the supplied family defaults
    /// `(peak_current_a, norm_emittance_um, energy_spread_rel, beta_m)`.
    fn fel_beam(
        &self,
        defaults: (f64, f64, f64, f64),
    ) -> Option<highuvlith_core::source_models::physics::ElectronBeamParams> {
        let any = self.peak_current_a.is_some()
            || self.norm_emittance_um.is_some()
            || self.electron_energy_spread_rel.is_some()
            || self.beta_m.is_some();
        any.then(
            || highuvlith_core::source_models::physics::ElectronBeamParams {
                peak_current_a: self.peak_current_a.unwrap_or(defaults.0),
                norm_emittance_um: self.norm_emittance_um.unwrap_or(defaults.1),
                energy_spread_rel: self.electron_energy_spread_rel.unwrap_or(defaults.2),
                beta_m: self.beta_m.unwrap_or(defaults.3),
            },
        )
    }

    /// Plasma diameter in µm from either spelling (`source_diameter_um` or
    /// `source_diameter_mm`); giving both is an error.
    fn plasma_diameter_um(&self) -> anyhow::Result<Option<f64>> {
        match (self.source_diameter_um, self.source_diameter_mm) {
            (Some(_), Some(_)) => anyhow::bail!(
                "[source] give the plasma diameter once: source_diameter_um OR source_diameter_mm \
                 (1 mm = 1000 µm)"
            ),
            (Some(um), None) => Ok(Some(um)),
            (None, Some(mm)) => Ok(Some(mm * 1e3)),
            (None, None) => Ok(None),
        }
    }

    /// Check value ranges that do not need the source to be built.
    pub fn validate_values(&self) -> anyhow::Result<()> {
        if let Some(w) = self.wavelength_nm {
            if !(w.is_finite() && w > 0.0) {
                anyhow::bail!("wavelength_nm must be finite and > 0, got {}", w);
            }
        }
        if let Some(sigma) = self.sigma {
            if !(sigma > 0.0 && sigma <= 1.0) {
                anyhow::bail!("sigma must be in (0, 1.0], got {}", sigma);
            }
        }
        if self.spectral_samples == Some(0) {
            anyhow::bail!("spectral_samples must be >= 1");
        }
        self.plasma_diameter_um()?;
        Ok(())
    }
}

/// Set the pupil fill and the spectral sample count of any source family.
fn set_fill_and_samples(
    src: &mut SourceKind,
    fill: Option<IlluminationShape>,
    samples: Option<usize>,
) {
    macro_rules! apply {
        ($s:expr) => {{
            if let Some(f) = fill {
                $s.illumination = f;
            }
            if let Some(n) = samples {
                $s.spectral_samples = n;
            }
        }};
    }
    match src {
        SourceKind::Vuv(s) => apply!(s),
        SourceKind::LpaFel(s) => apply!(s),
        SourceKind::Lpp(s) => apply!(s),
        SourceKind::Synchrotron(s) => apply!(s),
        SourceKind::Hhg(s) => apply!(s),
        SourceKind::Xfel(s) => apply!(s),
        SourceKind::Ics(s) => apply!(s),
        SourceKind::Ssmb(s) => apply!(s),
        SourceKind::Entangled(s) => apply!(s),
        SourceKind::XrayTube(s) => apply!(s),
        SourceKind::Dpp(s) => apply!(s),
        SourceKind::Sxrl(s) => apply!(s),
        SourceKind::Betatron(s) => apply!(s),
        SourceKind::SmithPurcell(s) => apply!(s),
    }
}

// ============================================================================
// [illumination]
// ============================================================================

/// `[illumination]`: the illuminator's pupil fill. Overrides the source
/// family's own fill (e.g. the near-coherent Gaussian of an FEL, or the
/// `[source] sigma` disk). The source's power / étendue bookkeeping is not
/// changed by it.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct IlluminationConfig {
    /// `conventional`, `annular`, `dipole`, `quadrupole`, or `coherent_gaussian`.
    pub shape: Option<String>,
    /// Disk radius (`conventional`) or Gaussian width (`coherent_gaussian`).
    pub sigma: Option<f64>,
    /// `annular` inner radius.
    pub sigma_inner: Option<f64>,
    /// `annular` outer radius.
    pub sigma_outer: Option<f64>,
    /// `dipole` / `quadrupole` pole-centre radius.
    pub sigma_center: Option<f64>,
    /// `dipole` / `quadrupole` pole radius.
    pub sigma_radius: Option<f64>,
    /// `dipole` pole axis in degrees (0 = poles on the x axis).
    pub orientation_deg: Option<f64>,
    /// `quadrupole` angular width of each pole sector in degrees.
    pub opening_angle_deg: Option<f64>,
}

pub const ILLUMINATION_SHAPES: &[&str] = &[
    "conventional",
    "annular",
    "dipole",
    "quadrupole",
    "coherent_gaussian",
];

impl IlluminationConfig {
    fn shape_keys(shape: &str) -> Option<&'static [&'static str]> {
        Some(match shape {
            "conventional" | "coherent_gaussian" => &["shape", "sigma"],
            "annular" => &["shape", "sigma_inner", "sigma_outer"],
            "dipole" => &["shape", "sigma_center", "sigma_radius", "orientation_deg"],
            "quadrupole" => &["shape", "sigma_center", "sigma_radius", "opening_angle_deg"],
            _ => return None,
        })
    }

    fn shape(&self) -> anyhow::Result<&str> {
        let shape = self.shape.as_deref().ok_or_else(|| {
            anyhow::anyhow!(
                "[illumination] needs a shape (one of: {})",
                ILLUMINATION_SHAPES.join(", ")
            )
        })?;
        if Self::shape_keys(shape).is_none() {
            anyhow::bail!(
                "unknown [illumination] shape '{}' (expected one of: {})",
                shape,
                ILLUMINATION_SHAPES.join(", ")
            );
        }
        Ok(shape)
    }

    pub fn check_keys(&self) -> anyhow::Result<()> {
        let shape = self.shape()?;
        let rules = KeyRules::new(
            "[illumination]",
            format!("shape = \"{shape}\""),
            Self::shape_keys(shape).unwrap_or_default(),
        );
        let owners = |key: &str| -> Vec<String> {
            ILLUMINATION_SHAPES
                .iter()
                .filter(|s| Self::shape_keys(s).is_some_and(|k| k.contains(&key)))
                .map(|s| format!("shape = \"{s}\""))
                .collect()
        };
        rules.check(&keys::present_keys(self), &owners)
    }

    /// The pupil fill, with its parameters checked.
    pub fn to_shape(&self) -> anyhow::Result<IlluminationShape> {
        let shape = self.shape()?;
        let need = |v: Option<f64>, key: &str| {
            v.ok_or_else(|| anyhow::anyhow!("[illumination] shape = \"{shape}\" needs {key}"))
        };
        let unit = |v: f64, key: &str, allow_zero: bool| -> anyhow::Result<f64> {
            let ok = v.is_finite() && v <= 1.0 && (v > 0.0 || (allow_zero && v == 0.0));
            if !ok {
                anyhow::bail!(
                    "[illumination] {key} must be in {}0, 1], got {v}",
                    if allow_zero { "[" } else { "(" }
                );
            }
            Ok(v)
        };
        Ok(match shape {
            "conventional" => IlluminationShape::Conventional {
                sigma: unit(need(self.sigma, "sigma")?, "sigma", false)?,
            },
            "coherent_gaussian" => IlluminationShape::CoherentGaussian {
                sigma: unit(need(self.sigma, "sigma")?, "sigma", false)?,
            },
            "annular" => {
                let inner = unit(need(self.sigma_inner, "sigma_inner")?, "sigma_inner", true)?;
                let outer = unit(need(self.sigma_outer, "sigma_outer")?, "sigma_outer", false)?;
                if inner >= outer {
                    anyhow::bail!(
                        "[illumination] sigma_inner ({inner}) must be < sigma_outer ({outer})"
                    );
                }
                IlluminationShape::Annular {
                    sigma_inner: inner,
                    sigma_outer: outer,
                }
            }
            "dipole" | "quadrupole" => {
                let center = unit(
                    need(self.sigma_center, "sigma_center")?,
                    "sigma_center",
                    false,
                )?;
                let radius = unit(
                    need(self.sigma_radius, "sigma_radius")?,
                    "sigma_radius",
                    false,
                )?;
                if center + radius > 1.0 + 1e-12 || radius > center {
                    anyhow::bail!(
                        "[illumination] poles must lie inside the pupil and off axis: need \
                         sigma_radius <= sigma_center and sigma_center + sigma_radius <= 1 (got \
                         {center} + {radius})"
                    );
                }
                if shape == "dipole" {
                    let orientation = self.orientation_deg.unwrap_or(0.0);
                    if !orientation.is_finite() {
                        anyhow::bail!("[illumination] orientation_deg must be finite");
                    }
                    IlluminationShape::Dipole {
                        sigma_center: center,
                        sigma_radius: radius,
                        orientation_deg: orientation,
                    }
                } else {
                    let opening = self.opening_angle_deg.unwrap_or(90.0);
                    if !(opening > 0.0 && opening <= 90.0) {
                        anyhow::bail!(
                            "[illumination] opening_angle_deg must be in (0, 90], got {opening}"
                        );
                    }
                    IlluminationShape::Quadrupole {
                        sigma_center: center,
                        sigma_radius: radius,
                        opening_angle_deg: opening,
                    }
                }
            }
            _ => unreachable!("shape() validated the tag"),
        })
    }
}

// ============================================================================
// [optics]
// ============================================================================

/// `[optics]`: the projection optics, selected by `type`:
/// `"refractive"` (default; CaF2/fused-silica lens — VUV/DUV only, optional
/// immersion), `"schwarzschild"` (two-mirror reflective objective),
/// `"euv_projection"` (EUV scanner projection box, NA 0.33 / High-NA 0.55
/// presets) or `"zone_plate"` (Fresnel diffractive — X-ray).
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct OpticsConfig {
    #[serde(rename = "type")]
    pub optics_type: Option<String>,
    /// Named starting point (explicit keys override it): refractive
    /// `"immersion_193i"`; schwarzschild `"euv"`, `"beuv"`, `"soft_xray"`
    /// (default: chosen from the wavelength); euv_projection `"nxe"`,
    /// `"high_na"`.
    pub preset: Option<String>,
    /// Numerical aperture (image side). Default 0.75 (refractive), the
    /// preset's value otherwise.
    pub na: Option<f64>,
    /// Uniform flare (stray-light) fraction.
    pub flare_fraction: Option<f64>,
    /// Refractive only: immersion-medium index (e.g. 1.437 for water at
    /// 193 nm). NA may then exceed 1 (up to 0.95 × index) and the defocus
    /// phase is evaluated in the medium.
    pub immersion_index: Option<f64>,
    /// Central obscuration radius as a fraction of NA (schwarzschild,
    /// euv_projection).
    pub central_obscuration: Option<f64>,
    /// Schwarzschild per-mirror reflectivity (absolute radiometry only).
    pub mirror_reflectivity: Option<f64>,
    /// EUV projection system transmission (absolute radiometry only).
    pub transmission: Option<f64>,
    /// Fringe-Zernike wavefront aberrations `[[index, coefficient_waves], ...]`
    /// (refractive, euv_projection).
    pub zernike: Option<Vec<(usize, f64)>>,
    /// Refractive axial chromatic aberration (nm defocus per pm), used by
    /// polychromatic imaging.
    pub axial_chromatic_nm_per_pm: Option<f64>,
    /// Paraxial instead of exact defocus phase.
    pub paraxial_defocus: Option<bool>,
    /// Zone-plate outermost zone width (nm); default λ/(2 NA).
    pub outer_zone_width_nm: Option<f64>,
    /// Opt-in angle-dependent multilayer pupil (schwarzschild,
    /// euv_projection): `[optics.multilayer_pupil]`.
    pub multilayer_pupil: Option<MultilayerPupilConfig>,
}

/// `[optics.multilayer_pupil]`: the angle-dependent multilayer amplitude and
/// phase across a reflective pupil (🔶; mirrors Python
/// `OpticsConfig.with_multilayer_pupil`). The incidence-angle maps are user
/// assumptions — real maps come from ray-tracing the design.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MultilayerPupilConfig {
    /// One `[center_deg, tilt_deg, azimuth_deg, radial_deg]` incidence-angle
    /// map per mirror: θ(p) = |θc + t·(p·u_a) + g·|p|²| (p normalized to NA).
    pub mirrors: Vec<[f64; 4]>,
    /// `"mo_si"` (default), `"la_b4c"` or `"la_b"`.
    #[serde(default)]
    pub coating: Option<String>,
    /// Bilayer count (default 40).
    #[serde(default)]
    pub periods: Option<usize>,
    /// Bilayer period in nm (default 6.9).
    #[serde(default)]
    pub period_nm: Option<f64>,
    /// Heavy-layer (absorber) fraction Γ (default 0.4).
    #[serde(default)]
    pub gamma: Option<f64>,
}

impl MultilayerPupilConfig {
    /// Build the core pupil (validates the coating and every angle map).
    pub fn to_pupil(
        &self,
    ) -> anyhow::Result<highuvlith_core::optics::multilayer_pupil::MultilayerPupil> {
        use highuvlith_core::materials::multilayer::MultilayerMirror;
        use highuvlith_core::optics::multilayer_pupil::{MirrorAngleMap, MultilayerPupil};
        let core = |e: highuvlith_core::error::LithographyError| {
            anyhow::anyhow!("[optics.multilayer_pupil] {e}")
        };
        let (periods, period_nm, gamma) = (
            self.periods.unwrap_or(40),
            self.period_nm.unwrap_or(6.9),
            self.gamma.unwrap_or(0.4),
        );
        let stack = match self.coating.as_deref().unwrap_or("mo_si") {
            "mo_si" => MultilayerMirror::mo_si(periods, period_nm, gamma),
            "la_b4c" => MultilayerMirror::la_b4c(periods, period_nm, gamma),
            "la_b" => MultilayerMirror::la_b(periods, period_nm, gamma),
            other => anyhow::bail!(
                "[optics.multilayer_pupil] coating must be \"mo_si\", \"la_b4c\" or \"la_b\", \
                 got \"{other}\""
            ),
        }
        .map_err(core)?;
        let maps = self
            .mirrors
            .iter()
            .map(
                |&[center_deg, tilt_deg, azimuth_deg, radial_deg]| MirrorAngleMap {
                    center_deg,
                    tilt_deg,
                    azimuth_deg,
                    radial_deg,
                },
            )
            .collect();
        MultilayerPupil::new(stack, maps).map_err(core)
    }
}

pub const OPTICS_TYPES: &[&str] = &[
    "refractive",
    "schwarzschild",
    "euv_projection",
    "zone_plate",
];

impl OpticsConfig {
    pub fn optics_type(&self) -> &str {
        self.optics_type.as_deref().unwrap_or("refractive")
    }

    fn type_keys(tag: &str) -> Option<&'static [&'static str]> {
        Some(match tag {
            "refractive" => &[
                "type",
                "preset",
                "na",
                "flare_fraction",
                "immersion_index",
                "zernike",
                "axial_chromatic_nm_per_pm",
                "paraxial_defocus",
            ],
            "schwarzschild" => &[
                "type",
                "preset",
                "na",
                "flare_fraction",
                "central_obscuration",
                "mirror_reflectivity",
                "paraxial_defocus",
                "multilayer_pupil",
            ],
            "euv_projection" => &[
                "type",
                "preset",
                "na",
                "flare_fraction",
                "central_obscuration",
                "zernike",
                "transmission",
                "paraxial_defocus",
                "multilayer_pupil",
            ],
            "zone_plate" => &["type", "na", "outer_zone_width_nm", "paraxial_defocus"],
            _ => return None,
        })
    }

    pub fn key_rules(&self) -> anyhow::Result<KeyRules> {
        let tag = self.optics_type();
        let keys = Self::type_keys(tag).ok_or_else(|| {
            anyhow::anyhow!(
                "unknown optics type '{}' (expected one of: {})",
                tag,
                OPTICS_TYPES.join(", ")
            )
        })?;
        let mut rules = KeyRules::new("[optics]", format!("type = \"{tag}\""), keys);
        if tag == "zone_plate" && self.outer_zone_width_nm.is_some() {
            rules.exclude(
                "na",
                "a zone plate's NA is λ/(2·outer_zone_width_nm); give na OR outer_zone_width_nm",
            );
        }
        Ok(rules)
    }

    pub fn check_keys(&self) -> anyhow::Result<()> {
        let owners = |key: &str| -> Vec<String> {
            OPTICS_TYPES
                .iter()
                .filter(|t| Self::type_keys(t).is_some_and(|k| k.contains(&key)))
                .map(|t| format!("type = \"{t}\""))
                .collect()
        };
        self.key_rules()?.check(&keys::present_keys(self), &owners)
    }

    /// Value checks that do not need the wavelength.
    pub fn validate_values(&self) -> anyhow::Result<()> {
        let fraction = |v: Option<f64>, key: &str, max_inclusive: bool| -> anyhow::Result<()> {
            if let Some(x) = v {
                let ok = x.is_finite() && x >= 0.0 && (x < 1.0 || (max_inclusive && x == 1.0));
                if !ok {
                    anyhow::bail!(
                        "[optics] {key} must be in [0, 1{}, got {x}",
                        if max_inclusive { "]" } else { ")" }
                    );
                }
            }
            Ok(())
        };
        fraction(self.flare_fraction, "flare_fraction", false)?;
        fraction(self.central_obscuration, "central_obscuration", false)?;
        fraction(self.mirror_reflectivity, "mirror_reflectivity", true)?;
        fraction(self.transmission, "transmission", true)?;
        if let Some(n) = self.immersion_index {
            if !(n.is_finite() && n >= 1.0) {
                anyhow::bail!("[optics] immersion_index must be >= 1, got {n}");
            }
        }
        if let Some(na) = self.na {
            let max = self.immersion_index.map(|n| 0.95 * n);
            let ok = na.is_finite() && na > 0.0 && max.map_or(na < 1.0, |m| na <= m);
            if !ok {
                match max {
                    Some(m) => anyhow::bail!(
                        "[optics] immersion NA must be in (0, 0.95 × immersion_index = {m:.4}], got \
                         {na}"
                    ),
                    None => anyhow::bail!(
                        "[optics] NA must be in (0, 1) for dry optics, got {na} (NA > 1 needs \
                         immersion: type = \"refractive\" with immersion_index)"
                    ),
                }
            }
        }
        for &(index, coefficient) in self.zernike.iter().flatten() {
            if !(1..=37).contains(&index) {
                anyhow::bail!(
                    "[optics] zernike index {index} is not a standard Fringe Zernike term (1–37)"
                );
            }
            if !coefficient.is_finite() {
                anyhow::bail!("[optics] zernike coefficient for index {index} must be finite");
            }
        }
        if let Some(w) = self.outer_zone_width_nm {
            if !(w.is_finite() && w > 0.0) {
                anyhow::bail!("[optics] outer_zone_width_nm must be > 0, got {w}");
            }
        }
        if let Some(c) = self.axial_chromatic_nm_per_pm {
            if !c.is_finite() {
                anyhow::bail!("[optics] axial_chromatic_nm_per_pm must be finite");
            }
        }
        Ok(())
    }
}

// ============================================================================
// [mask]
// ============================================================================

/// `[mask]`: the pattern (`pattern = "line_space"` by default,
/// `"contact_holes"` or `"features"`), its tone and the mask technology.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MaskConfig {
    /// `"line_space"` (default), `"contact_holes"`, or `"features"`.
    pub pattern: Option<String>,
    /// line_space: opaque line width (nm); default 65.
    pub cd_nm: Option<f64>,
    /// line_space pitch / contact_holes x pitch (nm); line_space default 180.
    pub pitch_nm: Option<f64>,
    /// line_space: `"vertical"` (lines along y; default) or `"horizontal"`.
    pub orientation: Option<String>,
    /// line_space: position of one line centre along the periodic axis (nm).
    pub offset_nm: Option<f64>,
    /// contact_holes: side of the square hole (nm).
    pub diameter_nm: Option<f64>,
    /// contact_holes: y pitch (nm); default `pitch_nm`.
    pub pitch_y_nm: Option<f64>,
    /// features: `[[mask.features]]` entries (painted in order).
    pub features: Option<Vec<FeatureConfig>>,
    /// Tone override: `true` = absorber background with clear features.
    /// Default: bright field for line_space / features, dark field for
    /// contact_holes.
    pub dark_field: Option<bool>,
    /// `"binary"` (default) or `"att_psm"` (attenuated phase shift).
    pub mask_type: Option<String>,
    /// att_psm absorber intensity transmission (default 0.06).
    pub transmission: Option<f64>,
    /// att_psm absorber phase in degrees (default 180).
    pub phase_deg: Option<f64>,
}

/// One `[[mask.features]]` entry (wafer-scale nm; the same names as the
/// Python `MaskConfig.from_features` dicts).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeatureConfig {
    /// Rectangle centred at (x, y), `w × h`.
    Rect { x: f64, y: f64, w: f64, h: f64 },
    /// Polygon `[[x, y], ...]` (even-odd fill).
    Polygon { vertices: Vec<[f64; 2]> },
    /// Rectangle with its own intensity transmittance in [0, 1].
    GrayRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        transmittance: f64,
    },
    /// Infinite grating of `cd`-wide lines every `pitch`.
    LineSpace {
        cd: f64,
        pitch: f64,
        #[serde(default)]
        orientation: Option<String>,
        #[serde(default)]
        offset: f64,
    },
    /// Infinite lattice of `w × h` rectangles.
    RectArray {
        w: f64,
        h: f64,
        pitch_x: f64,
        pitch_y: f64,
        #[serde(default)]
        offset_x: f64,
        #[serde(default)]
        offset_y: f64,
    },
}

pub const MASK_PATTERNS: &[&str] = &["line_space", "contact_holes", "features"];

fn parse_orientation(tag: Option<&str>) -> anyhow::Result<highuvlith_core::mask::LineOrientation> {
    use highuvlith_core::mask::LineOrientation;
    match tag.unwrap_or("vertical") {
        "vertical" => Ok(LineOrientation::Vertical),
        "horizontal" => Ok(LineOrientation::Horizontal),
        other => anyhow::bail!(
            "unknown orientation '{}' (expected 'vertical' or 'horizontal')",
            other
        ),
    }
}

impl FeatureConfig {
    fn to_feature(&self) -> anyhow::Result<highuvlith_core::mask::MaskFeature> {
        use highuvlith_core::mask::MaskFeature;
        Ok(match self {
            FeatureConfig::Rect { x, y, w, h } => MaskFeature::Rect {
                x: *x,
                y: *y,
                w: *w,
                h: *h,
            },
            FeatureConfig::Polygon { vertices } => MaskFeature::Polygon {
                vertices: vertices.iter().map(|v| (v[0], v[1])).collect(),
            },
            FeatureConfig::GrayRect {
                x,
                y,
                w,
                h,
                transmittance,
            } => MaskFeature::GrayRect {
                x: *x,
                y: *y,
                w: *w,
                h: *h,
                transmittance: *transmittance,
            },
            FeatureConfig::LineSpace {
                cd,
                pitch,
                orientation,
                offset,
            } => MaskFeature::LineSpace {
                cd: *cd,
                pitch: *pitch,
                orientation: parse_orientation(orientation.as_deref())?,
                offset: *offset,
            },
            FeatureConfig::RectArray {
                w,
                h,
                pitch_x,
                pitch_y,
                offset_x,
                offset_y,
            } => MaskFeature::RectArray {
                w: *w,
                h: *h,
                pitch_x: *pitch_x,
                pitch_y: *pitch_y,
                offset_x: *offset_x,
                offset_y: *offset_y,
            },
        })
    }
}

impl MaskConfig {
    /// A line/space mask (the legacy `[mask] cd_nm / pitch_nm` form).
    #[cfg(test)]
    pub fn line_space(cd_nm: f64, pitch_nm: f64) -> Self {
        Self {
            cd_nm: Some(cd_nm),
            pitch_nm: Some(pitch_nm),
            ..Self::default()
        }
    }

    pub fn pattern(&self) -> &str {
        self.pattern.as_deref().unwrap_or("line_space")
    }

    fn pattern_keys(pattern: &str) -> Option<&'static [&'static str]> {
        Some(match pattern {
            "line_space" => &[
                "pattern",
                "cd_nm",
                "pitch_nm",
                "orientation",
                "offset_nm",
                "dark_field",
                "mask_type",
                "transmission",
                "phase_deg",
            ],
            "contact_holes" => &[
                "pattern",
                "diameter_nm",
                "pitch_nm",
                "pitch_y_nm",
                "dark_field",
                "mask_type",
                "transmission",
                "phase_deg",
            ],
            "features" => &[
                "pattern",
                "features",
                "dark_field",
                "mask_type",
                "transmission",
                "phase_deg",
            ],
            _ => return None,
        })
    }

    pub fn key_rules(&self) -> anyhow::Result<KeyRules> {
        let pattern = self.pattern();
        let keys = Self::pattern_keys(pattern).ok_or_else(|| {
            anyhow::anyhow!(
                "unknown [mask] pattern '{}' (expected one of: {})",
                pattern,
                MASK_PATTERNS.join(", ")
            )
        })?;
        let mut rules = KeyRules::new("[mask]", format!("pattern = \"{pattern}\""), keys);
        if self.mask_type.as_deref().unwrap_or("binary") != "att_psm" {
            rules.exclude_all(
                &["transmission", "phase_deg"],
                "transmission and phase_deg describe the absorber of mask_type = \"att_psm\"",
            );
        }
        Ok(rules)
    }

    pub fn check_keys(&self) -> anyhow::Result<()> {
        let owners = |key: &str| -> Vec<String> {
            MASK_PATTERNS
                .iter()
                .filter(|p| Self::pattern_keys(p).is_some_and(|k| k.contains(&key)))
                .map(|p| format!("pattern = \"{p}\""))
                .collect()
        };
        self.key_rules()?.check(&keys::present_keys(self), &owners)
    }

    /// Line orientation of a line_space pattern (vertical otherwise).
    pub fn orientation(&self) -> anyhow::Result<highuvlith_core::mask::LineOrientation> {
        parse_orientation(self.orientation.as_deref())
    }

    /// Drawn width of the measured feature: the line (line_space) or the
    /// hole (contact_holes); `None` for free-form features.
    pub fn drawn_width_nm(&self) -> Option<f64> {
        match self.pattern() {
            "line_space" => Some(self.cd_nm.unwrap_or(65.0)),
            "contact_holes" => self.diameter_nm,
            _ => None,
        }
    }

    /// Short human-readable description for run summaries.
    pub fn describe(&self) -> String {
        match self.pattern() {
            "line_space" => format!(
                "line/space CD = {:.1} nm, pitch = {:.1} nm ({})",
                self.cd_nm.unwrap_or(65.0),
                self.pitch_nm.unwrap_or(180.0),
                self.orientation.as_deref().unwrap_or("vertical")
            ),
            "contact_holes" => format!(
                "contact holes {:.1} nm on {:.1} × {:.1} nm",
                self.diameter_nm.unwrap_or(f64::NAN),
                self.pitch_nm.unwrap_or(f64::NAN),
                self.pitch_y_nm.or(self.pitch_nm).unwrap_or(f64::NAN)
            ),
            _ => format!(
                "{} free-form feature(s)",
                self.features.as_ref().map_or(0, Vec::len)
            ),
        }
    }

    /// Build the mask (validated).
    pub fn to_mask(&self) -> anyhow::Result<highuvlith_core::mask::Mask> {
        use highuvlith_core::mask::{Mask, MaskType};
        let core = |e: highuvlith_core::error::LithographyError| anyhow::anyhow!("[mask] {e}");
        let mut mask = match self.pattern() {
            "line_space" => Mask::line_space_with(
                self.cd_nm.unwrap_or(65.0),
                self.pitch_nm.unwrap_or(180.0),
                self.orientation()?,
                self.offset_nm.unwrap_or(0.0),
            )
            .map_err(core)?,
            "contact_holes" => {
                let d = self.diameter_nm.ok_or_else(|| {
                    anyhow::anyhow!("[mask] pattern = \"contact_holes\" needs diameter_nm")
                })?;
                let px = self.pitch_nm.ok_or_else(|| {
                    anyhow::anyhow!("[mask] pattern = \"contact_holes\" needs pitch_nm")
                })?;
                Mask::contact_hole(d, px, self.pitch_y_nm.unwrap_or(px)).map_err(core)?
            }
            "features" => {
                let features = self.features.as_deref().unwrap_or_default();
                if features.is_empty() {
                    anyhow::bail!(
                        "[mask] pattern = \"features\" needs at least one [[mask.features]] entry"
                    );
                }
                Mask {
                    mask_type: MaskType::Binary,
                    features: features
                        .iter()
                        .map(FeatureConfig::to_feature)
                        .collect::<anyhow::Result<_>>()?,
                    dark_field: false,
                }
            }
            other => anyhow::bail!(
                "unknown [mask] pattern '{}' (expected one of: {})",
                other,
                MASK_PATTERNS.join(", ")
            ),
        };
        if let Some(dark) = self.dark_field {
            mask.dark_field = dark;
        }
        mask.mask_type = match self.mask_type.as_deref().unwrap_or("binary") {
            "binary" => MaskType::Binary,
            "att_psm" => MaskType::AttenuatedPSM {
                transmission: self.transmission.unwrap_or(0.06),
                phase_deg: self.phase_deg.unwrap_or(180.0),
            },
            other => anyhow::bail!(
                "unknown [mask] mask_type '{}' (expected 'binary' or 'att_psm')",
                other
            ),
        };
        mask.validate().map_err(core)?;
        Ok(mask)
    }
}

// ============================================================================
// [grid], [process], [imaging]
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridConfig {
    #[serde(default = "default_grid_size")]
    pub size: usize,
    #[serde(default = "default_pixel")]
    pub pixel_nm: f64,
    /// Adjust `pixel_nm` (never coarser, unless one pitch does not fit) so
    /// the field holds a whole number of mask pitches. The FFT makes the
    /// field periodic, so an incommensurate field images a truncated
    /// grating. Default true.
    #[serde(default = "default_commensurate")]
    pub commensurate: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessConfig {
    /// Exposure dose (mJ/cm²); the NOMINAL dose of the constant-threshold
    /// resist (default 30).
    #[serde(default = "default_dose")]
    pub dose_mj_cm2: f64,
    /// Focus / defocus (nm).
    #[serde(default)]
    pub focus_nm: f64,
    /// Intensity threshold (clear-field units) that prints at the nominal
    /// dose; at dose d the printed contour is `threshold · dose_mj_cm2 / d`
    /// (default 0.3).
    #[serde(default)]
    pub threshold: Option<f64>,
    /// Target CD for the process window (default: the drawn line / hole width).
    #[serde(default)]
    pub cd_target_nm: Option<f64>,
    /// Process-window CD tolerance, ± % of the target (default 10).
    #[serde(default)]
    pub cd_tolerance_pct: Option<f64>,
}

impl ProcessConfig {
    pub fn threshold(&self) -> f64 {
        self.threshold.unwrap_or(0.3)
    }

    pub fn cd_tolerance_pct(&self) -> f64 {
        self.cd_tolerance_pct.unwrap_or(10.0)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if !(self.dose_mj_cm2.is_finite() && self.dose_mj_cm2 > 0.0) {
            anyhow::bail!(
                "[process] dose_mj_cm2 must be finite and > 0, got {}",
                self.dose_mj_cm2
            );
        }
        if !self.focus_nm.is_finite() {
            anyhow::bail!("[process] focus_nm must be finite");
        }
        let t = self.threshold();
        if !(t.is_finite() && t > 0.0) {
            anyhow::bail!("[process] threshold must be finite and > 0, got {t}");
        }
        if let Some(cd) = self.cd_target_nm {
            if !(cd.is_finite() && cd > 0.0) {
                anyhow::bail!("[process] cd_target_nm must be > 0, got {cd}");
            }
        }
        let tol = self.cd_tolerance_pct();
        if !(tol.is_finite() && tol > 0.0 && tol < 100.0) {
            anyhow::bail!("[process] cd_tolerance_pct must be in (0, 100), got {tol}");
        }
        Ok(())
    }
}

/// Imaging-engine settings (`[imaging]` table; every key optional).
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ImagingConfig {
    /// `"exact"` (default; defocus inside the pupil at every source point)
    /// or `"kernel_phase"` (legacy fast approximation, valid only for
    /// on-axis, near-coherent illumination).
    pub defocus_model: Option<String>,
    /// Maximum SOCS kernels per kernel set (default 20).
    pub max_kernels: Option<usize>,
    /// Stop once the kernels capture this fraction of the TCC trace
    /// (default 1.0 = no energy cut).
    pub kernel_energy_fraction: Option<f64>,
    /// Source points across the source's bounding box (default: adaptive;
    /// 1 = a single coherent point).
    pub source_points_per_axis: Option<usize>,
    /// Spectrum handling in `simulate` / `sweep`: `"monochromatic"`
    /// (default, center wavelength), `"narrow_band"` (focus shift per
    /// spectral sample with center-wavelength kernels) or `"per_wavelength"`
    /// (exact: kernels rebuilt at every spectral sample).
    pub spectrum: Option<String>,
    /// `"clear_field"` (default: relative intensity, clear mask = 1) or
    /// `"absolute"` (relative to the incident illumination).
    pub normalization: Option<String>,
    /// Vector (polarized) imaging: an `[imaging.vector]` table with the
    /// `VectorSettings` keys (`polarization = { type = "te" }`, `obliquity`,
    /// `film = { n = 1.7, k = 0.03 }`, ...; all optional — an empty table is
    /// unpolarized vector imaging). Absent = scalar imaging. The engine takes
    /// `reduction` from the optics and requires `image_index` to equal the
    /// optics' immersion index (left at 1.0 it is inherited).
    pub vector: Option<highuvlith_core::optics::vector::VectorSettings>,
}

impl ImagingConfig {
    /// Spectrum mode, defaulting to `"monochromatic"`.
    pub fn spectrum(&self) -> &str {
        self.spectrum.as_deref().unwrap_or("monochromatic")
    }
}

fn default_commensurate() -> bool {
    true
}
fn default_grid_size() -> usize {
    256
}
fn default_pixel() -> f64 {
    1.0
}
fn default_dose() -> f64 {
    30.0
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            size: default_grid_size(),
            pixel_nm: default_pixel(),
            commensurate: default_commensurate(),
        }
    }
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            dose_mj_cm2: default_dose(),
            focus_nm: 0.0,
            threshold: None,
            cd_target_nm: None,
            cd_tolerance_pct: None,
        }
    }
}

// ============================================================================
// Loading, validation and builders
// ============================================================================

/// Context hints for unknown-key errors in the shared tables.
fn shared_table_hint(
    path: &[String],
    raw: &toml::Table,
    expected: &[String],
) -> Option<UnknownKeyHint> {
    let table = raw.get(path.first()?.as_str())?.as_table()?;
    let tag = |key: &str| table.get(key).and_then(|v| v.as_str());
    let has = |marker: &str| expected.iter().any(|e| e == marker);
    match (path.len(), path[0].as_str()) {
        (1, "source") if has("anode") => {
            let t = tag("type").unwrap_or("vuv");
            let b = tag("beamline");
            Some(UnknownKeyHint {
                label: format!("[source] {}", SourceConfig::context_label(t, b)),
                keys: SourceConfig::family_keys(t, b)?.to_vec(),
            })
        }
        (1, "optics") if has("outer_zone_width_nm") => {
            let t = tag("type").unwrap_or("refractive");
            Some(UnknownKeyHint {
                label: format!("[optics] type = \"{t}\""),
                keys: OpticsConfig::type_keys(t)?.to_vec(),
            })
        }
        (1, "mask") if has("diameter_nm") => {
            let p = tag("pattern").unwrap_or("line_space");
            Some(UnknownKeyHint {
                label: format!("[mask] pattern = \"{p}\""),
                keys: MaskConfig::pattern_keys(p)?.to_vec(),
            })
        }
        (1, "illumination") if has("opening_angle_deg") => {
            let s = tag("shape")?;
            Some(UnknownKeyHint {
                label: format!("[illumination] shape = \"{s}\""),
                keys: IlluminationConfig::shape_keys(s)?.to_vec(),
            })
        }
        _ => None,
    }
}

impl SimConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Self::from_toml_str(&text).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))
    }

    /// Parse a config (strict keys) without validating values.
    pub fn from_toml_str(text: &str) -> anyhow::Result<Self> {
        let mut config: Self = keys::parse_toml(text, &shared_table_hint)?;
        config.tables = toml::from_str::<toml::Table>(text)
            .map(|t| t.keys().cloned().collect())
            .unwrap_or_default();
        Ok(config)
    }

    /// `[source]`, or an error naming the command that needs it.
    pub fn source_cfg(&self) -> anyhow::Result<&SourceConfig> {
        self.source
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("this command needs a [source] table"))
    }

    /// `[optics]`, or an error naming the command that needs it.
    pub fn optics_cfg(&self) -> anyhow::Result<&OpticsConfig> {
        self.optics.as_ref().ok_or_else(|| {
            anyhow::anyhow!("this command needs an [optics] table (e.g. [optics] na = 0.75)")
        })
    }

    /// `[mask]`, or an error naming the command that needs it.
    pub fn mask_cfg(&self) -> anyhow::Result<&MaskConfig> {
        self.mask.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "this command needs a [mask] table (e.g. [mask] cd_nm = 65.0, pitch_nm = 180.0)"
            )
        })
    }

    /// Whether a top-level table was written in the file.
    pub fn has_table(&self, name: &str) -> bool {
        self.tables.contains(name)
    }

    /// One note per subcommand table (`[deep]`, `[optimize]`,
    /// `[throughput]`) present in the file but not read by `subcommand`.
    /// They are warnings, not errors: one config may serve several
    /// subcommands, and each table is validated by the subcommand that
    /// reads it.
    pub fn unread_table_notes(&self, subcommand: &str) -> Vec<String> {
        ["deep", "optimize", "throughput"]
            .into_iter()
            .filter(|t| *t != subcommand && self.has_table(t))
            .map(|t| {
                format!(
                    "[{t}] is not read by {subcommand} (ignored and not validated; the {t} \
                     subcommand reads it)"
                )
            })
            .collect()
    }

    /// Validate every table that is present: strict keys for the selected
    /// type / pattern / shape, value ranges, and every per-family check of
    /// the source builders.
    pub fn validate(&self) -> anyhow::Result<()> {
        if let Some(src) = &self.source {
            src.check_keys(self.illumination.is_some())?;
            src.validate_values()?;
        }
        if let Some(ill) = &self.illumination {
            ill.check_keys()?;
            ill.to_shape()?;
        }
        if let Some(optics) = &self.optics {
            optics.check_keys()?;
            optics.validate_values()?;
        }
        if let Some(mask) = &self.mask {
            mask.check_keys()?;
            mask.to_mask()?;
        }
        // Building the source runs every per-family check (known type
        // tag, odd harmonics, HHG cutoff, SSMB harmonic consistency,
        // derived-wavelength cross-checks, ...).
        if self.source.is_some() {
            self.to_source()?;
            if self.optics.is_some() {
                self.to_optics()?;
            }
        }
        if self.grid.size == 0 || !self.grid.size.is_power_of_two() {
            anyhow::bail!("grid size must be a power of 2, got {}", self.grid.size);
        }
        if !(self.grid.pixel_nm.is_finite() && self.grid.pixel_nm > 0.0) {
            anyhow::bail!("pixel_nm must be > 0, got {}", self.grid.pixel_nm);
        }
        self.process.validate()?;
        self.to_imaging_settings()?;
        match self.imaging.spectrum() {
            "monochromatic" | "narrow_band" | "per_wavelength" => {}
            other => anyhow::bail!(
                "[imaging] spectrum must be 'monochromatic', 'narrow_band' or \
                 'per_wavelength', got '{other}'"
            ),
        }
        Ok(())
    }

    /// Engine settings from the `[imaging]` table (defaults: exact defocus,
    /// 20 kernels, no energy cut, adaptive source sampling).
    pub fn to_imaging_settings(&self) -> anyhow::Result<highuvlith_core::aerial::ImagingSettings> {
        use highuvlith_core::aerial::{DefocusModel, ImagingSettings};
        let im = &self.imaging;
        let defocus_model = match im.defocus_model.as_deref().unwrap_or("exact") {
            "exact" => DefocusModel::Exact,
            "kernel_phase" => DefocusModel::KernelPhase,
            other => anyhow::bail!(
                "[imaging] defocus_model must be 'exact' or 'kernel_phase', got '{other}'"
            ),
        };
        let max_kernels = im.max_kernels.unwrap_or(20);
        if max_kernels == 0 {
            anyhow::bail!("[imaging] max_kernels must be >= 1");
        }
        let normalization = match im.normalization.as_deref().unwrap_or("clear_field") {
            "clear_field" => highuvlith_core::aerial::ImageNormalization::ClearField,
            "absolute" => highuvlith_core::aerial::ImageNormalization::Absolute,
            other => anyhow::bail!(
                "[imaging] normalization must be 'clear_field' or 'absolute', got '{other}'"
            ),
        };
        let kernel_energy_fraction = im.kernel_energy_fraction.unwrap_or(1.0);
        if !(kernel_energy_fraction > 0.0 && kernel_energy_fraction <= 1.0) {
            anyhow::bail!(
                "[imaging] kernel_energy_fraction must be in (0, 1], got {kernel_energy_fraction}"
            );
        }
        if im.source_points_per_axis == Some(0) {
            anyhow::bail!("[imaging] source_points_per_axis must be >= 1");
        }
        Ok(ImagingSettings {
            max_kernels,
            kernel_energy_fraction,
            source_points_per_axis: im.source_points_per_axis,
            defocus_model,
            normalization,
            imaging_model: match &im.vector {
                Some(v) => highuvlith_core::aerial::ImagingModel::Vector(v.clone()),
                None => highuvlith_core::aerial::ImagingModel::Scalar,
            },
            ..ImagingSettings::default()
        })
    }

    /// Build the source (every per-family check runs here), with the
    /// `[illumination]` pupil fill and `spectral_samples` applied.
    pub fn to_source(&self) -> anyhow::Result<SourceKind> {
        let mut src = self.build_family_source()?;
        let fill = self
            .illumination
            .as_ref()
            .map(IlluminationConfig::to_shape)
            .transpose()?;
        set_fill_and_samples(&mut src, fill, self.source_cfg()?.spectral_samples);
        Ok(src)
    }

    fn build_family_source(&self) -> anyhow::Result<SourceKind> {
        use highuvlith_core::source::{
            BetatronSource, DppSource, SmithPurcellSource, SxrlScheme, SxrlSource, XrayAnode,
            XrayTubeSource,
        };
        use highuvlith_core::source::{
            EntangledPhotonSource, HhgGas, HhgSource, IcsSource, LithographySource, LpaFelSource,
            LppSource, SpectralShape, SsmbSource, SynchrotronBeamline, SynchrotronSource,
            VuvSource, XfelMode, XfelSource,
        };
        use highuvlith_core::source_models::{
            ics::IcsCollision,
            lpp::{LppDriveLaser, LppGeometry},
            physics,
            ssmb::SsmbRadiator,
            synchrotron::StorageRingBeam,
        };
        let s = self.source_cfg()?;
        let sigma = s.sigma();
        let err = |e: highuvlith_core::error::LithographyError| anyhow::anyhow!("{}", e);
        let positive = |name: &str, v: Option<f64>| -> anyhow::Result<()> {
            match v {
                Some(x) if !(x.is_finite() && x > 0.0) => {
                    anyhow::bail!("{name} must be finite and > 0, got {x}")
                }
                _ => Ok(()),
            }
        };
        // Cross-check of an explicit wavelength against a derived one.
        let cross_check = |derived: f64, rtol: f64, what: &str| -> anyhow::Result<()> {
            if let Some(w) = s.wavelength_nm {
                if ((w - derived) / derived).abs() > rtol {
                    anyhow::bail!(
                        "wavelength_nm = {w} nm disagrees with the {what} {derived:.4} nm by more \
                         than {:.0}%; omit wavelength_nm (the machine sets the wavelength) or fix \
                         the machine parameters",
                        rtol * 100.0
                    );
                }
            }
            Ok(())
        };

        match s.source_type() {
            "vuv" => {
                let Some(preset) = s.preset.as_deref() else {
                    // Legacy explicit-wavelength source (back-compat).
                    return Ok(SourceKind::Vuv(VuvSource {
                        wavelength_nm: s.wavelength_nm.unwrap_or(157.63),
                        bandwidth_pm: s.bandwidth_pm.unwrap_or(1.1),
                        spectral_samples: 5,
                        spectral_shape: SpectralShape::Lorentzian,
                        pulse_energy_mj: 10.0,
                        rep_rate_hz: s.rep_rate_hz.unwrap_or(4000.0),
                        illumination: IlluminationShape::Conventional { sigma },
                    }));
                };
                let mut src = match preset {
                    "f2" => VuvSource::f2_laser(sigma),
                    "ar2" => VuvSource::ar2_laser(sigma),
                    "arf" => VuvSource::arf_laser(sigma),
                    "krf" => VuvSource::krf_laser(sigma),
                    "hg_i" | "i_line" => VuvSource::hg_i_line(sigma),
                    "hg_h" | "h_line" => VuvSource::hg_h_line(sigma),
                    "hg_g" | "g_line" => VuvSource::hg_g_line(sigma),
                    other => anyhow::bail!(
                        "unknown vuv preset '{}' (expected f2, ar2, arf, krf, hg_i, hg_h, hg_g)",
                        other
                    ),
                }
                .map_err(err)?;
                if let Some(w) = s.wavelength_nm {
                    src.wavelength_nm = w;
                }
                if let Some(bw) = s.bandwidth_pm {
                    src.bandwidth_pm = bw.max(0.0);
                }
                if let Some(rr) = s.rep_rate_hz {
                    src.rep_rate_hz = rr.max(0.0);
                }
                Ok(SourceKind::Vuv(src))
            }
            "lpa_fel" => {
                let mut fel =
                    LpaFelSource::new(s.wavelength_nm.unwrap_or(25.0), sigma).map_err(err)?;
                // An explicit bandwidth overrides the SASE 2 rho lambda that
                // the undulator + beam would otherwise derive.
                if let Some(bw) = s.bandwidth_pm {
                    fel.bandwidth_pm = bw.max(0.0);
                    fel.bandwidth_override_pm = Some(bw.max(0.0));
                }
                if let Some(e) = s.electron_energy_mev {
                    fel.electron_energy_mev = e;
                }
                if let Some(tau) = s.pulse_duration_fs {
                    fel.pulse_duration_fs = tau;
                }
                if let Some(rr) = s.rep_rate_hz {
                    fel.rep_rate_hz = rr;
                }
                if let Some(pe) = s.pulse_energy_uj {
                    fel.pulse_energy_uj = pe;
                }
                // Optional undulator (+ beam): the resonance wavelength is
                // derived and must agree with wavelength_nm within 5%.
                match (s.period_mm, s.undulator_k) {
                    (Some(period_mm), Some(k)) => {
                        let und = physics::UndulatorParams {
                            period_mm,
                            k,
                            num_periods: s.num_periods.unwrap_or(200),
                        };
                        let derived = und.resonance_nm(fel.gamma());
                        let beam = s.fel_beam((1000.0, 0.5, 0.01, 1.0));
                        fel = fel.with_machine(und, beam).map_err(|e| {
                            anyhow::anyhow!(
                                "{e}; the undulator resonance for these machine parameters \
                                 is {derived:.3} nm"
                            )
                        })?;
                    }
                    (None, None) => {}
                    _ => anyhow::bail!("an LPA-FEL undulator needs both period_mm and undulator_k"),
                }
                Ok(SourceKind::LpaFel(fel))
            }
            "lpp" => {
                let driver = match s.drive_laser.as_deref() {
                    None => None,
                    Some("co2") => Some(LppDriveLaser::Co2),
                    Some("solid_state_1um" | "1um" | "nd_yag") => {
                        Some(LppDriveLaser::SolidState1um)
                    }
                    Some("thulium_2um" | "2um" | "thulium") => Some(LppDriveLaser::Thulium2um),
                    Some(other) => anyhow::bail!(
                        "unknown lpp drive_laser '{}' (expected 'co2', 'solid_state_1um', or \
                         'thulium_2um')",
                        other
                    ),
                };
                let sn_500w = match s.preset.as_deref() {
                    None | Some("nxe3400b") => false,
                    Some("nxe3800e") => true,
                    Some(other) => anyhow::bail!(
                        "unknown lpp preset '{}' (expected 'nxe3400b' or 'nxe3800e')",
                        other
                    ),
                };
                if sn_500w && s.fuel.as_deref().is_some_and(|f| f != "sn") {
                    anyhow::bail!("lpp preset 'nxe3800e' is a Sn source (fuel = \"sn\")");
                }
                let mut lpp = match s.fuel.as_deref().unwrap_or("sn") {
                    // Sn: the drive laser also sets the default CE.
                    "sn" => {
                        let mut sn = match driver {
                            Some(d) => LppSource::sn_with_drive_laser(sigma, d).map_err(err)?,
                            None => LppSource::sn_13nm5(sigma).map_err(err)?,
                        };
                        if sn_500w {
                            let e = LppSource::sn_13nm5_500w(sigma).map_err(err)?;
                            sn.drive_laser_power_w = e.drive_laser_power_w;
                        }
                        sn
                    }
                    "gd" => LppSource::gd_6nm7(sigma).map_err(err)?,
                    "tb" => LppSource::tb_6nm5(sigma).map_err(err)?,
                    other => anyhow::bail!(
                        "unknown lpp fuel '{}' (expected 'sn', 'gd', or 'tb')",
                        other
                    ),
                };
                if driver.is_some() {
                    lpp.drive_laser = driver;
                }
                let diameter_um = s.plasma_diameter_um()?;
                if diameter_um.is_some()
                    || s.collection_solid_angle_sr.is_some()
                    || s.illuminator_etendue_mm2_sr.is_some()
                {
                    positive("source_diameter", diameter_um)?;
                    positive("collection_solid_angle_sr", s.collection_solid_angle_sr)?;
                    positive("illuminator_etendue_mm2_sr", s.illuminator_etendue_mm2_sr)?;
                    let mut g = lpp.geometry.unwrap_or_else(LppGeometry::nxe_like);
                    if let Some(d) = diameter_um {
                        g.source_diameter_um = d;
                    }
                    if let Some(o) = s.collection_solid_angle_sr {
                        g.collection_solid_angle_sr = o;
                    }
                    if let Some(e) = s.illuminator_etendue_mm2_sr {
                        g.illuminator_etendue_mm2_sr = e;
                    }
                    lpp.geometry = Some(g);
                }
                if let Some(w) = s.wavelength_nm {
                    lpp.wavelength_nm = w;
                }
                if let Some(bw) = s.bandwidth_pm {
                    lpp.bandwidth_pm = bw;
                }
                if let Some(p) = s.drive_laser_power_w {
                    lpp.drive_laser_power_w = p;
                }
                if let Some(ce) = s.conversion_efficiency {
                    lpp.conversion_efficiency = ce;
                }
                if let Some(te) = s.transport_efficiency {
                    lpp.transport_efficiency = te;
                }
                if let Some(rr) = s.rep_rate_hz {
                    lpp.rep_rate_hz = rr;
                }
                Ok(SourceKind::Lpp(lpp))
            }
            "synchrotron" => match s.beamline.as_deref().unwrap_or("undulator") {
                "undulator" => {
                    let mut src = SynchrotronSource::undulator(
                        s.electron_energy_gev.unwrap_or(0.538),
                        s.period_mm.unwrap_or(20.0),
                        s.undulator_k.unwrap_or(1.0),
                        s.num_periods.unwrap_or(100),
                        s.harmonic.unwrap_or(1),
                    )
                    .map_err(err)?;
                    if let Some(ma) = s.ring_current_ma {
                        src.ring_current_ma = ma;
                    }
                    // Ring emittances DERIVE the coherent fraction (and pupil).
                    if s.emittance_x_nm_rad.is_some() || s.emittance_y_nm_rad.is_some() {
                        let beam = StorageRingBeam {
                            emittance_x_nm_rad: s.emittance_x_nm_rad.unwrap_or(10.0),
                            emittance_y_nm_rad: s.emittance_y_nm_rad.unwrap_or(0.1),
                            energy_spread_rel: s.electron_energy_spread_rel.unwrap_or(5e-4),
                        };
                        if beam.emittance_x_nm_rad < 0.0
                            || beam.emittance_y_nm_rad < 0.0
                            || beam.energy_spread_rel < 0.0
                        {
                            anyhow::bail!("ring emittances and energy spread must be >= 0");
                        }
                        src = src.with_ring_beam(beam);
                    }
                    // Derived-wavelength cross-check: the resonance
                    // condition, not the config, sets the wavelength.
                    cross_check(
                        src.wavelength_nm(),
                        DERIVED_WAVELENGTH_RTOL,
                        "undulator resonance (electron_energy_gev / period_mm / undulator_k / \
                         harmonic)",
                    )?;
                    Ok(SourceKind::Synchrotron(src))
                }
                "bending_magnet" => {
                    let src = SynchrotronSource {
                        beamline: SynchrotronBeamline::BendingMagnet {
                            electron_energy_gev: s.electron_energy_gev.unwrap_or(2.5),
                            field_t: s.field_t.unwrap_or(1.5),
                            selected_wavelength_nm: s.wavelength_nm.unwrap_or(0.2),
                            mono_bandwidth_pm: s.bandwidth_pm.unwrap_or(0.2),
                        },
                        ring_current_ma: s.ring_current_ma.unwrap_or(200.0),
                        spectral_samples: 5,
                        illumination: IlluminationShape::Conventional { sigma: 0.8 },
                        transverse_coherence_fraction: 0.0,
                        ring_beam: None,
                    };
                    Ok(SourceKind::Synchrotron(src))
                }
                other => anyhow::bail!(
                    "unknown synchrotron beamline '{}' (expected 'undulator' or 'bending_magnet')",
                    other
                ),
            },
            "hhg" => {
                let gas = match s.gas.as_deref().unwrap_or("neon") {
                    "helium" | "he" => HhgGas::Helium,
                    "neon" | "ne" => HhgGas::Neon,
                    "argon" | "ar" => HhgGas::Argon,
                    "krypton" | "kr" => HhgGas::Krypton,
                    "xenon" | "xe" => HhgGas::Xenon,
                    other => anyhow::bail!(
                        "unknown hhg gas '{}' (expected helium/neon/argon/krypton/xenon)",
                        other
                    ),
                };
                let full_comb = s.full_comb == Some(true);
                // The full comb needs no selected harmonic: build with the
                // fundamental (always inside the cutoff), then drop the
                // monochromator.
                let harmonic = if full_comb {
                    s.harmonic.unwrap_or(1)
                } else {
                    s.harmonic.unwrap_or(59)
                };
                let mut src = HhgSource::new(
                    s.driver_wavelength_nm.unwrap_or(800.0),
                    gas,
                    s.driver_intensity_w_cm2.unwrap_or(4e14),
                    harmonic,
                    s.monochromator_bandwidth_pm.unwrap_or(15.0),
                )
                .map_err(err)?;
                if full_comb {
                    src.monochromator = None;
                } else {
                    cross_check(
                        src.wavelength_nm(),
                        HHG_WAVELENGTH_RTOL,
                        "selected harmonic driver_wavelength_nm / harmonic",
                    )?;
                }
                if let Some(pe) = s.pulse_energy_nj {
                    src.pulse_energy_nj = pe;
                }
                if let Some(rr) = s.rep_rate_hz {
                    src.rep_rate_hz = rr;
                }
                if let Some(tau) = s.pulse_duration_fs {
                    src.pulse_duration_fs = tau;
                }
                positive("driver_average_power_w", s.driver_average_power_w)?;
                // Power: a driver power derives it (P_driver x eta_q); an
                // explicit pulse energy alone is used as stored; with neither
                // the constructor's default driver power applies.
                match (s.driver_average_power_w, s.pulse_energy_nj) {
                    (Some(p), _) => src.driver_average_power_w = Some(p),
                    (None, Some(_)) => src.driver_average_power_w = None,
                    (None, None) => {}
                }
                if let Some(ce) = s.conversion_efficiency {
                    if !(ce > 0.0 && ce <= 1.0) {
                        anyhow::bail!("hhg conversion_efficiency must be in (0, 1], got {ce}");
                    }
                    src.conversion_efficiency = Some(ce);
                }
                src.comb_passband_nm = s.comb_passband_nm;
                Ok(SourceKind::Hhg(src))
            }
            "xfel" => {
                let preset = match s.xfel_preset_tag()? {
                    "flash" => XfelSource::flash_13nm5(),
                    "fermi" | "fermi_seeded" => XfelSource::fermi_seeded_13nm5(),
                    "cw_sc" => XfelSource::cw_sc_13nm5(),
                    "erl" => XfelSource::erl_13nm5(),
                    other => anyhow::bail!(
                        "unknown xfel_preset '{}' (expected 'flash', 'fermi', 'cw_sc', or 'erl')",
                        other
                    ),
                };
                let mut src = preset.clone();
                if let Some(mode) = s.mode.as_deref() {
                    src.mode = match mode {
                        "sase" => XfelMode::Sase {
                            pierce_parameter: s.pierce_parameter.unwrap_or(3e-3),
                        },
                        "seeded" | "self_seeded" => XfelMode::SelfSeeded {
                            rel_bandwidth: s.rel_bandwidth.unwrap_or(5e-5),
                        },
                        other => anyhow::bail!(
                            "unknown xfel mode '{}' (expected 'sase' or 'seeded')",
                            other
                        ),
                    };
                } else if let (Some(rb), XfelMode::SelfSeeded { rel_bandwidth }) =
                    (s.rel_bandwidth, &mut src.mode)
                {
                    *rel_bandwidth = rb;
                }
                if let Some(rho) = s.pierce_parameter {
                    // An explicit Pierce parameter wins over the machine
                    // derivation (legacy configs keep their bandwidth).
                    if let XfelMode::Sase { pierce_parameter } = &mut src.mode {
                        *pierce_parameter = rho;
                    }
                    src.machine = None;
                }
                if let Some(m) = src.machine.as_mut() {
                    if let Some(e) = s.electron_energy_mev {
                        m.electron_energy_mev = e;
                    }
                    if let Some(p) = s.period_mm {
                        m.undulator_period_mm = p;
                    }
                    if let Some(l) = s.undulator_length_m {
                        m.undulator_length_m = l;
                    }
                    if let Some(beam) = s.fel_beam((
                        m.beam.peak_current_a,
                        m.beam.norm_emittance_um,
                        m.beam.energy_spread_rel,
                        m.beam.beta_m,
                    )) {
                        m.beam = beam;
                    }
                }
                if let Some(w) = s.wavelength_nm {
                    src.wavelength_nm = w;
                }
                if src.machine.is_some() && src.undulator_k().is_none() {
                    anyhow::bail!(
                        "xfel wavelength_nm = {} nm is shorter than the K = 0 resonance of the \
                         undulator at this electron energy; raise electron_energy_mev or \
                         shorten period_mm",
                        src.wavelength_nm
                    );
                }
                if let Some(tau) = s.pulse_duration_fs {
                    src.pulse_duration_fs = tau;
                }
                match s.pulse_energy_uj {
                    Some(pe) => src.pulse_energy_uj = pe,
                    // Machine / set-point / duration overrides re-derive the
                    // pulse energy at the preset's pulse-energy-to-saturation
                    // ratio (a no-op when nothing changed).
                    None => {
                        src.rescale_pulse_energy_from(&preset);
                    }
                }
                if let Some(rr) = s.rep_rate_hz {
                    src.rep_rate_hz = rr;
                }
                Ok(SourceKind::Xfel(src))
            }
            "ics" => {
                let mut src = IcsSource::for_wavelength(
                    s.wavelength_nm.unwrap_or(13.5),
                    s.laser_wavelength_nm.unwrap_or(1030.0),
                    s.laser_a0.unwrap_or(0.1),
                )
                .map_err(err)?;
                // The wavelength is the input here and the electron energy
                // is DERIVED from the Compton condition. If the config
                // also pins electron_energy_mev, it must satisfy
                // lambda_X = lambda_L (1 + a0^2/2) / (4 gamma^2) within 5%.
                if let Some(e_mev) = s.electron_energy_mev {
                    let derived = src.electron_energy_mev;
                    if ((e_mev - derived) / derived).abs() > 0.05 {
                        anyhow::bail!(
                            "electron_energy_mev = {e_mev} MeV is inconsistent with the \
                             Compton condition for the requested wavelength (needs \
                             ~{derived:.3} MeV); omit electron_energy_mev or fix \
                             wavelength_nm / laser_wavelength_nm / laser_a0"
                        );
                    }
                    src.electron_energy_mev = e_mev;
                }
                if let Some(a) = s.collection_half_angle_mrad {
                    src.collection_half_angle_mrad = a;
                }
                if let Some(es) = s.electron_energy_spread_rel {
                    src.electron_energy_spread_rel = es;
                }
                if let Some(pe) = s.pulse_energy_nj {
                    src.pulse_energy_nj = pe;
                }
                if let Some(rr) = s.rep_rate_hz {
                    src.rep_rate_hz = rr;
                }
                // Collision parameters DERIVE the photon yield (Thomson
                // luminosity x collection fraction).
                if s.bunch_charge_pc.is_some()
                    || s.laser_pulse_energy_mj.is_some()
                    || s.electron_spot_um.is_some()
                    || s.laser_spot_um.is_some()
                {
                    let d = IcsCollision::high_average_power_design();
                    let collision = IcsCollision {
                        bunch_charge_pc: s.bunch_charge_pc.unwrap_or(d.bunch_charge_pc),
                        laser_pulse_energy_mj: s
                            .laser_pulse_energy_mj
                            .unwrap_or(d.laser_pulse_energy_mj),
                        electron_spot_um: s.electron_spot_um.unwrap_or(d.electron_spot_um),
                        laser_spot_um: s.laser_spot_um.unwrap_or(d.laser_spot_um),
                    };
                    if s.collection_half_angle_mrad.is_none() {
                        // Collect the whole 2 % Mo/Si band by default.
                        src.collection_half_angle_mrad =
                            physics::ics_half_angle_for_bandwidth(src.gamma(), src.laser_a0, 0.02)
                                * 1e3;
                    }
                    src = src.with_collision(collision).map_err(err)?;
                }
                Ok(SourceKind::Ics(src))
            }
            "ssmb" => {
                let mut src = SsmbSource::new(
                    s.ring_energy_mev.unwrap_or(400.0),
                    s.modulation_wavelength_nm.unwrap_or(1053.0),
                    s.wavelength_nm.unwrap_or(13.5),
                    s.average_power_w.unwrap_or(1000.0),
                )
                .map_err(err)?;
                // Beam/radiator parameters DERIVE the coherent power.
                if s.average_current_a.is_some()
                    || s.peak_current_a.is_some()
                    || s.bunching_factor.is_some()
                    || s.radiator_periods.is_some()
                    || s.radiator_k.is_some()
                {
                    let avg = s.average_current_a.unwrap_or(1.0);
                    let mut radiator = SsmbRadiator {
                        average_current_a: avg,
                        peak_current_a: s.peak_current_a.unwrap_or(avg),
                        bunching_factor: 1.0,
                        num_periods: s.radiator_periods.unwrap_or(100),
                        k: s.radiator_k.unwrap_or(1.6),
                    };
                    // Without an explicit b, use the b the projection requires.
                    radiator.bunching_factor = s.bunching_factor.unwrap_or_else(|| {
                        SsmbSource::bunching_required(&radiator, src.average_power_w).min(1.0)
                    });
                    src = src.with_radiator(radiator).map_err(err)?;
                }
                Ok(SourceKind::Ssmb(src))
            }
            "entangled" => {
                let mut src = EntangledPhotonSource::noon(
                    s.wavelength_nm.unwrap_or(157.63),
                    s.num_photons.unwrap_or(2),
                    s.fidelity.unwrap_or(1.0),
                )
                .map_err(err)?;
                if let Some(rate) = s.pair_rate_hz {
                    if !(rate.is_finite() && rate >= 0.0) {
                        anyhow::bail!("pair_rate_hz must be finite and >= 0, got {rate}");
                    }
                    src.pair_rate_hz = rate;
                }
                Ok(SourceKind::Entangled(src))
            }
            "xray_tube" => {
                let name = s.anode.as_deref().unwrap_or("w");
                let anode = XrayAnode::from_name(name).ok_or_else(|| {
                    anyhow::anyhow!(
                        "unknown xray_tube anode '{}' (expected 'w', 'mo', 'cu', or 'rh')",
                        name
                    )
                })?;
                let mut src = XrayTubeSource::preset(anode).map_err(err)?;
                if let Some(v) = s.kvp {
                    src.kvp = v;
                }
                if let Some(ma) = s.current_ma {
                    src.current_ma = ma;
                }
                if let Some(t) = s.be_window_um {
                    src.be_window_um = t;
                }
                src.validate().map_err(err)?;
                // The trait wavelength is DERIVED (photon-weighted mean of
                // the filtered spectrum); a given wavelength_nm must agree.
                cross_check(
                    src.wavelength_nm(),
                    DERIVED_WAVELENGTH_RTOL,
                    "X-ray tube's mean photon wavelength (set by anode / kvp / be_window_um)",
                )?;
                Ok(SourceKind::XrayTube(src))
            }
            "dpp" => {
                let mut src = match s.fuel.as_deref().unwrap_or("sn") {
                    "sn" => DppSource::sn_13nm5(sigma).map_err(err)?,
                    "xe" => DppSource::xe_13nm5(sigma).map_err(err)?,
                    other => {
                        anyhow::bail!("unknown dpp fuel '{}' (expected 'sn' or 'xe')", other)
                    }
                };
                if let Some(w) = s.wavelength_nm {
                    src.wavelength_nm = w;
                }
                if let Some(bw) = s.bandwidth_pm {
                    src.bandwidth_pm = bw;
                }
                if let Some(p) = s.electrical_power_w {
                    src.electrical_power_w = p;
                }
                if let Some(ce) = s.conversion_efficiency {
                    src.conversion_efficiency = ce;
                }
                if let Some(d_um) = s.plasma_diameter_um()? {
                    src.source_diameter_mm = d_um * 1e-3;
                }
                if let Some(l) = s.source_length_mm {
                    src.source_length_mm = l;
                }
                if let Some(o) = s.collection_solid_angle_sr {
                    src.collector_solid_angle_sr = o;
                }
                if let Some(e) = s.collector_efficiency {
                    src.collector_efficiency = e;
                }
                if let Some(g) = s.illuminator_etendue_mm2_sr {
                    src.illuminator_etendue_mm2_sr = g;
                }
                if let Some(rr) = s.rep_rate_hz {
                    src.rep_rate_hz = rr;
                }
                src.validate().map_err(err)?;
                Ok(SourceKind::Dpp(src))
            }
            "sxrl" => {
                let name = s.scheme.as_deref().unwrap_or("ar_46nm9");
                let scheme = SxrlScheme::from_name(name).ok_or_else(|| {
                    anyhow::anyhow!(
                        "unknown sxrl scheme '{}' (expected 'ar_46nm9', 'ag_13nm9', \
                         'cd_13nm2', or 'mo_18nm9')",
                        name
                    )
                })?;
                let mut src = SxrlSource::preset(scheme).map_err(err)?;
                if let Some(rel) = s.rel_linewidth.or(s.rel_bandwidth) {
                    src.rel_linewidth = rel;
                }
                if let Some(pe) = s.pulse_energy_uj {
                    src.pulse_energy_uj = pe;
                }
                if let Some(rr) = s.rep_rate_hz {
                    src.rep_rate_hz = rr;
                }
                if let Some(tau) = s.pulse_duration_ps {
                    src.pulse_duration_ps = tau;
                }
                src.validate().map_err(err)?;
                // The lasing line is a fixed atomic transition.
                if let Some(w) = s.wavelength_nm {
                    let line = scheme.wavelength_nm();
                    if ((w - line) / line).abs() > 0.01 {
                        anyhow::bail!(
                            "wavelength_nm = {w} nm is not the {line} nm lasing line of scheme \
                             '{name}' (a fixed atomic transition); pick a different scheme or \
                             omit wavelength_nm"
                        );
                    }
                }
                Ok(SourceKind::Sxrl(src))
            }
            "betatron" => {
                let mut src = BetatronSource::new(
                    s.electron_energy_mev.unwrap_or(200.0),
                    s.plasma_density_cm3.unwrap_or(1e19),
                    s.betatron_amplitude_um.unwrap_or(1.0),
                    s.interaction_length_mm.unwrap_or(3.0),
                    s.bunch_charge_pc.unwrap_or(50.0),
                    s.rep_rate_hz.unwrap_or(10.0),
                )
                .map_err(err)?;
                if let Some(tau) = s.pulse_duration_fs {
                    src.pulse_duration_fs = tau;
                }
                src.validate().map_err(err)?;
                cross_check(
                    src.wavelength_nm(),
                    DERIVED_WAVELENGTH_RTOL,
                    "betatron wavelength (hc / mean photon energy, derived from \
                     electron_energy_mev / plasma_density_cm3 / betatron_amplitude_um)",
                )?;
                Ok(SourceKind::Betatron(src))
            }
            "smith_purcell" => {
                let e_kev = s.electron_energy_kev.unwrap_or(30.0);
                let order = s.diffraction_order.unwrap_or(1);
                let angle = s.observation_angle_deg.unwrap_or(90.0);
                let mut src = match s.grating_period_nm {
                    // Period given: the wavelength is DERIVED from the
                    // Smith-Purcell condition and cross-checked.
                    Some(period) => {
                        let src =
                            SmithPurcellSource::new(e_kev, period, order, angle).map_err(err)?;
                        cross_check(
                            src.wavelength_nm(),
                            DERIVED_WAVELENGTH_RTOL,
                            "Smith-Purcell wavelength (a/m)(1/beta - cos theta)",
                        )?;
                        src
                    }
                    // No period: derive the period for the target wavelength.
                    None => SmithPurcellSource::for_wavelength(
                        s.wavelength_nm.unwrap_or(13.5),
                        e_kev,
                        order,
                        angle,
                    )
                    .map_err(err)?,
                };
                if let Some(n) = s.num_periods {
                    src.num_periods = n;
                }
                if let Some(i) = s.beam_current_na {
                    src.beam_current_na = i;
                }
                if let Some(h) = s.impact_height_nm {
                    src.impact_height_nm = h;
                }
                if let Some(c) = s.coupling_efficiency {
                    src.coupling_efficiency = c;
                }
                if let Some(a) = s.collection_half_angle_mrad {
                    src.collection_half_angle_mrad = a;
                }
                if let Some(es) = s.electron_energy_spread_rel {
                    src.electron_energy_spread_rel = es;
                }
                src.validate().map_err(err)?;
                Ok(SourceKind::SmithPurcell(src))
            }
            other => Err(SourceConfig::unknown_type_error(other)),
        }
    }

    /// Warnings about physically questionable optics (printed by the run
    /// commands, not by `validate`).
    pub fn optics_warnings(&self) -> Vec<String> {
        use highuvlith_core::source::LithographySource;
        let (Some(optics), Ok(src)) = (self.optics.as_ref(), self.to_source()) else {
            return Vec::new();
        };
        let wavelength = src.wavelength_nm();
        let mut out = Vec::new();
        if optics.optics_type() == "refractive" && wavelength < 50.0 {
            out.push(format!(
                "refractive CaF2 optics selected at {wavelength:.2} nm — no transparent lens \
                 material exists below ~110 nm; results are not physical. Set [optics] type = \
                 \"euv_projection\", \"schwarzschild\" or \"zone_plate\"."
            ));
        }
        out
    }

    /// Build the optical system (needs `[source]` for the wavelength and
    /// `[optics]`).
    pub fn to_optics(&self) -> anyhow::Result<Box<dyn highuvlith_core::optics::OpticalSystem>> {
        use highuvlith_core::optics::euv::EuvProjectionOptics;
        use highuvlith_core::optics::schwarzschild::SchwarzschildObjective;
        use highuvlith_core::optics::zone_plate::FresnelZonePlate;
        use highuvlith_core::optics::ProjectionOptics;
        use highuvlith_core::source::LithographySource;

        let o = self.optics_cfg()?;
        o.validate_values()?;
        let core = |e: highuvlith_core::error::LithographyError| anyhow::anyhow!("[optics] {e}");
        let wavelength = self.to_source()?.wavelength_nm();
        let paraxial = o.paraxial_defocus.unwrap_or(false);
        let unknown_preset = |p: &str, valid: &str| {
            anyhow::anyhow!(
                "unknown [optics] preset '{p}' for type = \"{}\" (expected {valid})",
                o.optics_type()
            )
        };
        match o.optics_type() {
            "refractive" => {
                let mut optics = match o.preset.as_deref() {
                    Some("immersion_193i") => {
                        let mut p = ProjectionOptics::immersion_193i();
                        if o.na.is_some() || o.immersion_index.is_some() {
                            p = ProjectionOptics::immersion(
                                o.na.unwrap_or(p.na),
                                o.immersion_index.unwrap_or(p.immersion_index),
                            )
                            .map_err(core)?;
                        }
                        p
                    }
                    Some(other) => return Err(unknown_preset(other, "'immersion_193i'")),
                    None => match o.immersion_index {
                        Some(n) => ProjectionOptics::immersion(o.na.unwrap_or(0.75), n),
                        None => ProjectionOptics::new(o.na.unwrap_or(0.75)),
                    }
                    .map_err(core)?,
                };
                if let Some(f) = o.flare_fraction {
                    optics.flare_fraction = f;
                }
                if let Some(z) = &o.zernike {
                    optics.zernike_coefficients = z.clone();
                }
                if let Some(c) = o.axial_chromatic_nm_per_pm {
                    optics.axial_chromatic_nm_per_pm = c;
                }
                optics.paraxial_defocus = paraxial;
                Ok(Box::new(optics))
            }
            "schwarzschild" => {
                // Start from the preset matching the wavelength band (or the
                // named one), then apply the configured values.
                let preset = o.preset.as_deref().unwrap_or(if wavelength < 3.0 {
                    "soft_xray"
                } else if wavelength < 10.0 {
                    "beuv"
                } else {
                    "euv"
                });
                let mut objective = match preset {
                    "soft_xray" => {
                        SchwarzschildObjective::soft_xray(o.na.unwrap_or(0.1)).map_err(core)?
                    }
                    "beuv" => SchwarzschildObjective::beuv(),
                    "euv" => SchwarzschildObjective::euv_standard(),
                    other => return Err(unknown_preset(other, "'euv', 'beuv' or 'soft_xray'")),
                };
                if let Some(na) = o.na {
                    objective.numerical_aperture = na;
                }
                if let Some(f) = o.flare_fraction {
                    objective.flare = f;
                }
                if let Some(c) = o.central_obscuration {
                    objective.obscuration_ratio = c;
                }
                if let Some(r) = o.mirror_reflectivity {
                    objective.mirror_reflectivity = r;
                }
                objective.paraxial_defocus = paraxial;
                if let Some(ml) = &o.multilayer_pupil {
                    objective.multilayer = Some(ml.to_pupil()?);
                }
                Ok(Box::new(objective))
            }
            "zone_plate" => {
                let zone_width = o
                    .outer_zone_width_nm
                    .unwrap_or(wavelength / (2.0 * o.na.unwrap_or(0.1)));
                let mut zp = FresnelZonePlate::new(zone_width, wavelength).map_err(core)?;
                zp.paraxial_defocus = paraxial;
                Ok(Box::new(zp))
            }
            "euv_projection" => {
                // EUV scanner projection optics: isotropic wafer-side pupil
                // with optional central obscuration (anamorphic
                // magnification and mask-3D effects not modeled).
                let base = match o.preset.as_deref() {
                    None | Some("nxe") => EuvProjectionOptics::nxe_033(),
                    Some("high_na") => EuvProjectionOptics::high_na_055(),
                    Some(other) => return Err(unknown_preset(other, "'nxe' or 'high_na'")),
                };
                let mut euv = EuvProjectionOptics::new(
                    o.na.unwrap_or(base.numerical_aperture),
                    o.central_obscuration.unwrap_or(base.central_obscuration),
                )
                .map_err(core)?;
                if let Some(f) = o.flare_fraction {
                    euv.flare = f;
                }
                if let Some(z) = &o.zernike {
                    euv.zernike_coefficients = z.clone();
                }
                if let Some(t) = o.transmission {
                    euv.transmission = t;
                }
                euv.paraxial_defocus = paraxial;
                if let Some(ml) = &o.multilayer_pupil {
                    euv.multilayer = Some(ml.to_pupil()?);
                }
                Ok(Box::new(euv))
            }
            other => anyhow::bail!(
                "unknown optics type '{}' (expected one of: {})",
                other,
                OPTICS_TYPES.join(", ")
            ),
        }
    }

    pub fn to_mask(&self) -> anyhow::Result<highuvlith_core::mask::Mask> {
        self.mask_cfg()?.to_mask()
    }

    /// The simulation grid. With `[grid] commensurate = true` (the default)
    /// the pixel is adjusted so the field holds a whole number of mask
    /// pitches, and a note is printed to stderr when it changes.
    pub fn to_grid(&self) -> anyhow::Result<highuvlith_core::types::GridConfig> {
        let requested = highuvlith_core::types::GridConfig::new(self.grid.size, self.grid.pixel_nm)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        if !self.grid.commensurate || self.mask.is_none() {
            return Ok(requested);
        }
        let mask = self.to_mask()?;
        let grid = mask
            .commensurate_grid(self.grid.size, self.grid.pixel_nm)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        if (grid.pixel_nm - requested.pixel_nm).abs() > 1e-12 * requested.pixel_nm {
            let cell = mask
                .periodicity()
                .map(|(p1, p2)| match p2 {
                    Some(p2) => highuvlith_core::types::common_period_nm(p1, p2).unwrap_or(p1),
                    None => p1,
                })
                .unwrap_or(grid.field_size_nm());
            let periods = (grid.field_size_nm() / cell).round();
            eprintln!(
                "note: pixel {} nm -> {:.6} nm so the {:.3} nm field holds {} whole period{} of \
                 the {} nm mask cell (the FFT makes the field periodic); set [grid] commensurate \
                 = false to keep the configured pixel",
                requested.pixel_nm,
                grid.pixel_nm,
                grid.field_size_nm(),
                periods,
                if periods == 1.0 { "" } else { "s" },
                cell
            );
        }
        Ok(grid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use highuvlith_core::source::LithographySource;

    fn valid_config() -> SimConfig {
        SimConfig {
            source: Some(SourceConfig {
                source_type: Some("vuv".to_string()),
                wavelength_nm: Some(157.63),
                sigma: Some(0.7),
                bandwidth_pm: Some(1.1),
                ..SourceConfig::default()
            }),
            optics: Some(OpticsConfig {
                na: Some(0.75),
                flare_fraction: Some(0.02),
                ..OpticsConfig::default()
            }),
            mask: Some(MaskConfig::line_space(65.0, 180.0)),
            ..SimConfig::default()
        }
    }

    /// A bare source of `tag` (family defaults) in an otherwise valid config.
    fn family_config(tag: &str) -> SimConfig {
        let mut cfg = valid_config();
        cfg.source = Some(SourceConfig {
            source_type: Some(tag.to_string()),
            ..SourceConfig::default()
        });
        cfg
    }

    fn src(cfg: &mut SimConfig) -> &mut SourceConfig {
        cfg.source.as_mut().unwrap()
    }

    fn parse(text: &str) -> anyhow::Result<SimConfig> {
        let cfg = SimConfig::from_toml_str(text)?;
        cfg.validate()?;
        Ok(cfg)
    }

    fn parse_err(text: &str) -> String {
        match parse(text) {
            Ok(_) => panic!("expected an error for:\n{text}"),
            Err(e) => e.to_string(),
        }
    }

    fn source_from_toml(source_toml: &str) -> anyhow::Result<SourceKind> {
        let toml_str = format!(
            "[source]\n{source_toml}\n[optics]\nna = 0.33\n[mask]\ncd_nm = 30.0\npitch_nm = 60.0\n"
        );
        parse(&toml_str)?.to_source()
    }

    fn dq(src: &SourceKind, name: &str) -> f64 {
        src.derived_quantities()
            .into_iter()
            .find(|q| q.name == name)
            .unwrap_or_else(|| panic!("missing derived quantity {name}"))
            .value
    }

    // ---------------------------------------------------------------------
    // [imaging], [optics] basics (carried over)
    // ---------------------------------------------------------------------

    #[test]
    fn test_imaging_table_defaults_and_parsing() {
        let settings = valid_config().to_imaging_settings().unwrap();
        assert_eq!(settings.max_kernels, 20);
        assert_eq!(
            settings.defocus_model,
            highuvlith_core::aerial::DefocusModel::Exact
        );
        let cfg = parse(
            r#"
            [source]
            wavelength_nm = 157.63
            [optics]
            na = 0.75
            [mask]
            cd_nm = 65.0
            pitch_nm = 180.0
            [imaging]
            defocus_model = "kernel_phase"
            max_kernels = 12
            kernel_energy_fraction = 0.99
            source_points_per_axis = 21
            spectrum = "per_wavelength"
        "#,
        )
        .unwrap();
        let s = cfg.to_imaging_settings().unwrap();
        assert_eq!(s.max_kernels, 12);
        assert_eq!(s.source_points_per_axis, Some(21));
        assert_eq!(cfg.imaging.spectrum(), "per_wavelength");
    }

    #[test]
    fn test_immersion_and_euv_optics_tables() {
        let wet = r#"
            [source]
            wavelength_nm = 193.368
            [optics]
            na = 1.35
            immersion_index = 1.437
            [mask]
            cd_nm = 45.0
            pitch_nm = 90.0
            [imaging]
            normalization = "absolute"
        "#;
        let cfg = parse(wet).unwrap();
        let optics = cfg.to_optics().unwrap();
        assert_eq!(optics.na(), 1.35);
        assert_eq!(optics.immersion_index(), 1.437);
        assert_eq!(
            cfg.to_imaging_settings().unwrap().normalization,
            highuvlith_core::aerial::ImageNormalization::Absolute
        );
        // NA > 1 without an immersion medium, or above 0.95·n, is rejected.
        assert!(parse(&wet.replace("immersion_index = 1.437", "")).is_err());
        assert!(parse(&wet.replace("na = 1.35", "na = 1.40")).is_err());
        let euv = r#"
            [source]
            wavelength_nm = 13.5
            [optics]
            type = "euv_projection"
            na = 0.55
            central_obscuration = 0.2
            flare_fraction = 0.0
            [mask]
            cd_nm = 8.0
            pitch_nm = 16.0
        "#;
        let cfg = parse(euv).unwrap();
        let optics = cfg.to_optics().unwrap();
        assert_eq!(optics.na(), 0.55);
        assert_eq!(optics.pupil_function(0.1, 0.0, 0.0, 13.5).norm(), 0.0);
        assert_eq!(optics.pupil_function(0.5, 0.0, 0.0, 13.5).norm(), 1.0);
    }

    #[test]
    fn test_optics_presets_and_type_specific_keys() {
        let base = "[source]\nwavelength_nm = 13.5\n[mask]\ncd_nm = 8.0\npitch_nm = 16.0\n";
        // High-NA preset: NA 0.55, obscuration 0.2 (assumed); explicit keys override.
        let hi = parse(&format!(
            "{base}[optics]\ntype = \"euv_projection\"\npreset = \"high_na\"\n"
        ))
        .unwrap();
        let o = hi.to_optics().unwrap();
        assert_eq!(o.na(), 0.55);
        assert_eq!(o.pupil_function(0.15, 0.0, 0.0, 13.5).norm(), 0.0);
        let nxe = parse(&format!("{base}[optics]\ntype = \"euv_projection\"\n")).unwrap();
        assert_eq!(nxe.to_optics().unwrap().na(), 0.33);
        // 193i preset: NA 1.35 in water.
        let arf = parse(
            "[source]\ntype = \"vuv\"\npreset = \"arf\"\n[optics]\npreset = \"immersion_193i\"\n\
             [mask]\ncd_nm = 45.0\npitch_nm = 90.0\n",
        )
        .unwrap();
        let o = arf.to_optics().unwrap();
        assert_eq!(o.na(), 1.35);
        assert_eq!(o.immersion_index(), 1.437);
        // Keys of another optics type are rejected with the owner named.
        let err = parse_err(&format!(
            "{base}[optics]\ntype = \"schwarzschild\"\nna = 0.3\nimmersion_index = 1.44\n"
        ));
        assert!(
            err.contains("`immersion_index` is only read by type = \"refractive\""),
            "{err}"
        );
        let err = parse_err(
            "[source]\nwavelength_nm = 157.63\n[optics]\nna = 0.75\ncentral_obscuration = 0.2\n\
             [mask]\ncd_nm = 65.0\npitch_nm = 180.0\n",
        );
        assert!(
            err.contains("`central_obscuration` is only read by"),
            "{err}"
        );
        // Zone plate: na and outer_zone_width_nm are alternatives.
        let err = parse_err(&format!(
            "{base}[optics]\ntype = \"zone_plate\"\nna = 0.1\nouter_zone_width_nm = 50.0\n"
        ));
        assert!(err.contains("give na OR outer_zone_width_nm"), "{err}");
        // flare_fraction is not a zone-plate key (the zone plate has no flare model).
        let err = parse_err(&format!(
            "{base}[optics]\ntype = \"zone_plate\"\nflare_fraction = 0.1\n"
        ));
        assert!(err.contains("`flare_fraction`"), "{err}");
        // Schwarzschild obscuration is settable (a smaller one avoids dark field).
        let sw = parse(&format!(
            "{base}[optics]\ntype = \"schwarzschild\"\nna = 0.3\ncentral_obscuration = 0.0\n"
        ))
        .unwrap();
        assert!(
            sw.to_optics()
                .unwrap()
                .pupil_function(0.0, 0.0, 0.0, 13.5)
                .norm()
                > 0.0
        );
        // Unknown preset / Zernike index out of range.
        assert!(parse(&format!(
            "{base}[optics]\ntype = \"euv_projection\"\npreset = \"exe\"\n"
        ))
        .is_err());
        assert!(parse(&format!(
            "{base}[optics]\ntype = \"euv_projection\"\nzernike = [[40, 0.01]]\n"
        ))
        .is_err());
    }

    #[test]
    fn test_multilayer_pupil_table() {
        let base = "[source]\ntype = \"lpp\"\n[mask]\ncd_nm = 30.0\npitch_nm = 60.0\n\
                    [optics]\ntype = \"euv_projection\"\n";
        let pupil_amp = |extra: &str, p: f64| {
            parse(&format!("{base}{extra}"))
                .unwrap()
                .to_optics()
                .unwrap()
                .pupil_function(p, 0.0, 0.0, 13.5)
                .norm()
        };
        assert!((pupil_amp("", 0.0) - 1.0).abs() < 1e-12);
        // Two Mo/Si mirrors at normal incidence everywhere: amplitude = R of
        // one mirror (|r|² = 0.7293, `highuvlith materials --multilayer mo_si`).
        let uniform = "[optics.multilayer_pupil]\nmirrors = [[0, 0, 0, 0], [0, 0, 0, 0]]\n";
        assert!((pupil_amp(uniform, 0.0) - 0.7293).abs() < 5e-4);
        assert!((pupil_amp(uniform, 0.0) - pupil_amp(uniform, 0.9)).abs() < 1e-9);
        // A radial angle map apodizes the pupil edge.
        let graded = "[optics.multilayer_pupil]\nmirrors = [[0, 0, 0, 15], [0, 0, 0, 15]]\n\
                      coating = \"mo_si\"\nperiods = 40\nperiod_nm = 6.9\ngamma = 0.4\n";
        assert!(pupil_amp(graded, 0.9) < 0.95 * pupil_amp(graded, 0.0));
        // Schwarzschild takes it too.
        let sw = format!(
            "[source]\ntype = \"lpp\"\n[mask]\ncd_nm = 30.0\npitch_nm = 60.0\n[optics]\n\
             type = \"schwarzschild\"\n{uniform}"
        );
        assert!(parse(&sw).is_ok());
        // Refractive optics do not; bad coating / unknown key / empty mirrors.
        let refr = parse_err(&format!(
            "[source]\nwavelength_nm = 157.63\n[mask]\ncd_nm = 65.0\npitch_nm = 180.0\n\
             [optics]\nna = 0.75\n{uniform}"
        ));
        assert!(refr.contains("multilayer_pupil"), "{refr}");
        assert!(refr.contains("euv_projection"), "{refr}");
        let coat = parse_err(&format!(
            "{base}[optics.multilayer_pupil]\nmirrors = [[0, 0, 0, 0]]\ncoating = \"gold\"\n"
        ));
        assert!(coat.contains("coating"), "{coat}");
        let typo = parse_err(&format!(
            "{base}[optics.multilayer_pupil]\nmirrors = [[0, 0, 0, 0]]\nperiod = 7.0\n"
        ));
        assert!(typo.contains("period"), "{typo}");
        parse_err(&format!("{base}[optics.multilayer_pupil]\nmirrors = []\n"));
    }

    #[test]
    fn test_zernike_aberrations_enter_the_pupil() {
        let cfg = parse(
            "[source]\nwavelength_nm = 157.63\n[optics]\nna = 0.75\nzernike = [[9, 0.05], [7, 0.02]]\n\
             paraxial_defocus = true\n[mask]\ncd_nm = 65.0\npitch_nm = 180.0\n",
        )
        .unwrap();
        let optics = cfg.to_optics().unwrap();
        // Spherical + coma: the pupil edge carries a phase.
        let p = optics.pupil_function(0.7, 0.0, 0.0, 157.63);
        assert!((p.norm() - 1.0).abs() < 1e-12);
        assert!(p.arg().abs() > 1e-3);
    }

    #[test]
    fn test_imaging_vector_table() {
        use highuvlith_core::aerial::ImagingModel;
        use highuvlith_core::optics::vector::IlluminationPolarization;
        let base = "[source]\nwavelength_nm = 157.63\n[optics]\nna = 0.9\n[mask]\n\
                    cd_nm = 65.0\npitch_nm = 180.0\n";
        // Absent: scalar.
        let cfg: SimConfig = toml::from_str(base).unwrap();
        assert!(matches!(
            cfg.to_imaging_settings().unwrap().imaging_model,
            ImagingModel::Scalar
        ));
        // Empty table: unpolarized vector imaging with defaults.
        let cfg: SimConfig = toml::from_str(&format!("{base}[imaging.vector]\n")).unwrap();
        match cfg.to_imaging_settings().unwrap().imaging_model {
            ImagingModel::Vector(v) => {
                assert_eq!(v.polarization, IlluminationPolarization::Unpolarized)
            }
            ImagingModel::Scalar => panic!("expected vector imaging"),
        }
        // TE polarization with a resist film entrance.
        let text = format!(
            "{base}[imaging.vector]\npolarization = {{ type = \"te\" }}\nobliquity = false\n\
             film = {{ n = 1.7, k = 0.03 }}\n"
        );
        let cfg: SimConfig = toml::from_str(&text).unwrap();
        cfg.validate().unwrap();
        match cfg.to_imaging_settings().unwrap().imaging_model {
            ImagingModel::Vector(v) => {
                assert_eq!(v.polarization, IlluminationPolarization::Te);
                assert!(!v.obliquity);
                assert_eq!(v.film.map(|f| f.n), Some(1.7));
            }
            ImagingModel::Scalar => panic!("expected vector imaging"),
        }
    }

    #[test]
    fn test_imaging_table_rejects_bad_values() {
        for (key, value) in [
            ("defocus_model", "\"paraxial\""),
            ("max_kernels", "0"),
            ("kernel_energy_fraction", "1.5"),
            ("source_points_per_axis", "0"),
            ("spectrum", "\"broadband\""),
            ("normalization", "\"peak\""),
        ] {
            let text = format!(
                "[source]\nwavelength_nm = 157.63\n[optics]\nna = 0.75\n[mask]\n\
                 cd_nm = 65.0\npitch_nm = 180.0\n[imaging]\n{key} = {value}\n"
            );
            assert!(parse(&text).is_err(), "{key} = {value} should be rejected");
        }
    }

    #[test]
    fn test_valid_config_passes() {
        valid_config().validate().unwrap();
    }

    #[test]
    fn test_zero_wavelength() {
        let mut cfg = valid_config();
        src(&mut cfg).wavelength_nm = Some(0.0);
        assert!(cfg.validate().is_err());
        src(&mut cfg).wavelength_nm = Some(f64::NAN);
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_na_out_of_range() {
        let mut cfg = valid_config();
        cfg.optics.as_mut().unwrap().na = Some(1.5);
        assert!(cfg.validate().is_err());
        cfg.optics.as_mut().unwrap().na = Some(0.0);
        assert!(cfg.validate().is_err());
        // NA = 1 is not a dry lens either.
        cfg.optics.as_mut().unwrap().na = Some(1.0);
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_sigma_out_of_range() {
        let mut cfg = valid_config();
        src(&mut cfg).sigma = Some(0.0);
        assert!(cfg.validate().is_err());
        src(&mut cfg).sigma = Some(1.1);
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_cd_gte_pitch() {
        let mut cfg = valid_config();
        cfg.mask = Some(MaskConfig::line_space(200.0, 180.0));
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_grid_not_power_of_two() {
        let mut cfg = valid_config();
        cfg.grid.size = 100;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_zero_pixel() {
        let mut cfg = valid_config();
        cfg.grid.pixel_nm = 0.0;
        assert!(cfg.validate().is_err());
    }

    // ---------------------------------------------------------------------
    // [source] families (carried over, now with clean per-family keys)
    // ---------------------------------------------------------------------

    #[test]
    fn test_unknown_source_type_rejected() {
        let cfg = family_config("tachyon");
        let err = cfg.validate().unwrap_err().to_string();
        assert!(err.contains("unknown source type 'tachyon'"), "{err}");
        assert!(err.contains("smith_purcell"), "{err}");
    }

    #[test]
    fn test_synchrotron_source_type_accepted() {
        let cfg = family_config("synchrotron");
        cfg.validate().unwrap();
        assert!(matches!(
            cfg.to_source().unwrap(),
            SourceKind::Synchrotron(_)
        ));
    }

    #[test]
    fn test_undulator_derived_wavelength_cross_check() {
        let mut cfg = family_config("synchrotron");
        // Machine params give ~13.5 nm; asking for 20 nm must fail.
        src(&mut cfg).wavelength_nm = Some(20.0);
        assert!(cfg.validate().is_err());
        // Consistent request passes.
        src(&mut cfg).wavelength_nm = Some(13.5);
        cfg.validate().unwrap();
    }

    #[test]
    fn test_lpa_fel_source_type_accepted() {
        let mut cfg = valid_config();
        let s = src(&mut cfg);
        s.source_type = Some("lpa_fel".to_string());
        s.wavelength_nm = Some(25.0);
        s.bandwidth_pm = Some(25.0);
        s.electron_energy_mev = Some(500.0);
        cfg.validate().unwrap();
        assert!(matches!(cfg.to_source().unwrap(), SourceKind::LpaFel(_)));
    }

    #[test]
    fn test_all_source_types_build_with_family_defaults() {
        for tag in SOURCE_TYPES {
            let cfg = family_config(tag);
            cfg.validate()
                .unwrap_or_else(|e| panic!("{tag} should validate: {e}"));
            assert_eq!(cfg.to_source().unwrap().kind_label(), *tag);
        }
    }

    #[test]
    fn test_ics_electron_energy_cross_check() {
        let mut cfg = family_config("ics");
        src(&mut cfg).wavelength_nm = Some(13.5);
        // 13.5 nm from a 1030 nm laser needs ~1.73 MeV; 10 MeV is
        // inconsistent with the Compton condition.
        src(&mut cfg).electron_energy_mev = Some(10.0);
        assert!(cfg.validate().is_err());
        // A consistent value passes.
        src(&mut cfg).electron_energy_mev = Some(1.73);
        cfg.validate().unwrap();
        // Omitting it derives the energy silently.
        src(&mut cfg).electron_energy_mev = None;
        cfg.validate().unwrap();
    }

    #[test]
    fn test_hhg_cutoff_violation_rejected_from_config() {
        let mut cfg = family_config("hhg");
        src(&mut cfg).gas = Some("argon".to_string());
        src(&mut cfg).driver_intensity_w_cm2 = Some(2e14);
        src(&mut cfg).harmonic = Some(59); // beyond the Ar cutoff at 2e14
        assert!(cfg.validate().is_err());
        src(&mut cfg).harmonic = Some(27); // within cutoff
        cfg.validate().unwrap();
    }

    #[test]
    fn test_hhg_wavelength_cross_check_is_one_percent() {
        // q = 59 at 800 nm is 13.559 nm; 13.5 nm is within 1 %.
        source_from_toml("type = \"hhg\"\nharmonic = 59\nwavelength_nm = 13.5").unwrap();
        // 14.0 nm is the q = 57 line (3.3 % away) — a different harmonic.
        let err = source_from_toml("type = \"hhg\"\nharmonic = 59\nwavelength_nm = 14.0")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("disagrees with the selected harmonic"),
            "{err}"
        );
    }

    #[test]
    fn test_legacy_family_tomls_unchanged_by_new_fields() {
        // No machine fields: stored / legacy behaviour.
        let ics = source_from_toml("type = \"ics\"\nwavelength_nm = 13.5").unwrap();
        assert!((ics.average_power_w().unwrap() - 1e-5).abs() < 1e-15); // 1 nJ x 10 kHz
        let ssmb = source_from_toml("type = \"ssmb\"").unwrap();
        assert_eq!(ssmb.average_power_w(), Some(1000.0));
        let syn = source_from_toml("type = \"synchrotron\"").unwrap();
        assert!((syn.transverse_coherence() - 0.2).abs() < 1e-12);
        // An explicit Pierce parameter still sets the SASE bandwidth (2 rho lambda).
        let xfel = source_from_toml(
            "type = \"xfel\"\nmode = \"sase\"\nwavelength_nm = 13.5\npierce_parameter = 3.0e-3",
        )
        .unwrap();
        assert!((xfel.bandwidth_pm() - 81.0).abs() < 1e-9);
        // LPP with an explicit legacy transport factor on the (now
        // NXE:3400B: 21.5 kW x 6 %) preset: 1290 W x 0.05 = 64.5 W.
        let lpp = source_from_toml("type = \"lpp\"\ntransport_efficiency = 0.05").unwrap();
        assert!((lpp.average_power_w().unwrap() - 64.5).abs() < 1e-9);
        assert!(matches!(lpp, SourceKind::Lpp(_)));
    }

    #[test]
    fn test_lpa_fel_undulator_resonance_rule() {
        use highuvlith_core::source::LithographySource;
        // 500 MeV, 20 mm period: K = 1.6724 resonates at 25 nm.
        let ok = source_from_toml(
            "type = \"lpa_fel\"\nwavelength_nm = 25.0\nelectron_energy_mev = 500.0\n\
             period_mm = 20.0\nundulator_k = 1.6724\npeak_current_a = 1000.0",
        )
        .unwrap();
        assert!(dq(&ok, "pierce_parameter_1d") > 1e-3);
        assert!(dq(&ok, "energy_spread_over_rho") > 1.0); // LPA default 1 % spread
                                                          // Bandwidth: the derived 2 rho lambda unless given explicitly.
        let sase = 2.0 * dq(&ok, "pierce_parameter_1d") * 25.0 * 1e3;
        assert!((ok.bandwidth_pm() - sase).abs() < 1e-9 * sase);
        let pinned = source_from_toml(
            "type = \"lpa_fel\"\nwavelength_nm = 25.0\nelectron_energy_mev = 500.0\n\
             period_mm = 20.0\nundulator_k = 1.6724\npeak_current_a = 1000.0\n\
             bandwidth_pm = 2.5",
        )
        .unwrap();
        assert_eq!(pinned.bandwidth_pm(), 2.5);
        // K = 1.2 resonates near 18 nm: > 5 % off the 25 nm set-point.
        let bad = source_from_toml(
            "type = \"lpa_fel\"\nwavelength_nm = 25.0\nelectron_energy_mev = 500.0\n\
             period_mm = 20.0\nundulator_k = 1.2",
        );
        let msg = bad.unwrap_err().to_string();
        assert!(msg.contains("resonance"), "{msg}");
        // Half an undulator spec is rejected.
        assert!(source_from_toml("type = \"lpa_fel\"\nperiod_mm = 20.0").is_err());
    }

    #[test]
    fn test_lpp_nxe3800e_preset() {
        use highuvlith_core::source::LithographySource;
        let e = source_from_toml("type = \"lpp\"\npreset = \"nxe3800e\"").unwrap();
        assert!((e.average_power_w().unwrap() - 500.0).abs() < 1e-9);
        let b = source_from_toml("type = \"lpp\"\npreset = \"nxe3400b\"").unwrap();
        assert!((b.average_power_w().unwrap() - 250.0).abs() < 1e-9);
        assert!(source_from_toml("type = \"lpp\"\npreset = \"nxe9999\"").is_err());
        assert!(source_from_toml("type = \"lpp\"\nfuel = \"gd\"\npreset = \"nxe3800e\"").is_err());
    }

    #[test]
    fn test_lpp_drive_laser_and_etendue_fields() {
        let tm = source_from_toml("type = \"lpp\"\nfuel = \"sn\"\ndrive_laser = \"thulium_2um\"")
            .unwrap();
        let co2 = source_from_toml("type = \"lpp\"\nfuel = \"sn\"").unwrap();
        assert!(tm.average_power_w().unwrap() < co2.average_power_w().unwrap());
        assert!((dq(&tm, "drive_laser_wavelength") - 2.0).abs() < 1e-12);
        // A 1 mm source overfills the default 3.3 mm^2 sr budget.
        let big = source_from_toml("type = \"lpp\"\nsource_diameter_um = 1000.0").unwrap();
        assert!(dq(&big, "etendue_limited_fraction") < 0.9);
        assert!(source_from_toml("type = \"lpp\"\ndrive_laser = \"plasma_gun\"").is_err());
        assert!(source_from_toml("type = \"lpp\"\nsource_diameter_um = -1.0").is_err());
    }

    #[test]
    fn test_synchrotron_emittance_fields() {
        let src = source_from_toml(
            "type = \"synchrotron\"\nemittance_x_nm_rad = 10.0\nemittance_y_nm_rad = 0.1",
        )
        .unwrap();
        // zeta_x zeta_y at 13.5065 nm = 0.0888 (replaces the 0.2 constant).
        assert!((src.transverse_coherence() - 0.088_789_176_375_747_46).abs() < 1e-9);
        assert!(dq(&src, "central_cone_power") > 0.2);
    }

    #[test]
    fn test_xfel_presets_and_machine_fields() {
        let erl = source_from_toml("type = \"xfel\"\nxfel_preset = \"erl\"").unwrap();
        assert!(erl.average_power_w().unwrap() > 1e4); // kW-class projection
        let cw = source_from_toml("type = \"xfel\"\nxfel_preset = \"cw_sc\"").unwrap();
        assert!((cw.average_power_w().unwrap() - 134.95).abs() < 0.1);
        // Beam overrides re-derive rho: more current -> larger rho.
        let hot =
            source_from_toml("type = \"xfel\"\nxfel_preset = \"cw_sc\"\npeak_current_a = 2000.0")
                .unwrap();
        assert!(dq(&hot, "pierce_parameter_1d") > dq(&cw, "pierce_parameter_1d"));
        // ... and re-derive the pulse energy (CW-SC/ERL: the saturation
        // estimate itself), unless pulse_energy_uj is given explicitly.
        assert!((dq(&hot, "pulse_energy_over_saturation") - 1.0).abs() < 1e-12);
        assert!(hot.average_power_w().unwrap() > cw.average_power_w().unwrap());
        let erl600 =
            source_from_toml("type = \"xfel\"\nxfel_preset = \"erl\"\npeak_current_a = 600.0")
                .unwrap();
        // Independent 1D + Ming Xie evaluation: 195.039 uJ x 162.5 MHz.
        assert!((erl600.average_power_w().unwrap() - 31_693.91).abs() < 0.1);
        let pinned = source_from_toml(
            "type = \"xfel\"\nxfel_preset = \"erl\"\npeak_current_a = 600.0\npulse_energy_uj = 65.0",
        )
        .unwrap();
        assert!((pinned.pulse_energy_j().unwrap() - 65e-6).abs() < 1e-15);
        // Explicit rho drops the machine: the stored pulse energy stays.
        let legacy =
            source_from_toml("type = \"xfel\"\nxfel_preset = \"flash\"\npierce_parameter = 2e-3")
                .unwrap();
        assert!((legacy.pulse_energy_j().unwrap() - 100e-6).abs() < 1e-15);
        // Unreachable set-point for the undulator is a config error.
        assert!(source_from_toml("type = \"xfel\"\nwavelength_nm = 0.5").is_err());
        assert!(source_from_toml("type = \"xfel\"\nxfel_preset = \"lcls\"").is_err());
        // preset is an alias of xfel_preset; contradicting values are an error.
        let err = source_from_toml("type = \"xfel\"\nxfel_preset = \"erl\"\npreset = \"flash\"")
            .unwrap_err()
            .to_string();
        assert!(err.contains("disagree"), "{err}");
    }

    #[test]
    fn test_hhg_driver_power_and_passband() {
        let src = source_from_toml(
            "type = \"hhg\"\ngas = \"ar\"\ndriver_intensity_w_cm2 = 2.0e14\nharmonic = 27\n\
             driver_average_power_w = 50.0",
        )
        .unwrap();
        // 50 W x 1e-5 (Ar plateau estimate) = 0.5 mW
        assert!((src.average_power_w().unwrap() - 5e-4).abs() < 1e-12);
        assert!(dq(&src, "power_vs_measured_record") > 1.0); // projection
                                                             // Default: the assumed 5 W driver -> 50 uW; an explicit pulse energy
                                                             // alone is used as stored (0.2 nJ x 10 kHz = 2 uW).
        let default = source_from_toml(
            "type = \"hhg\"\ngas = \"ar\"\ndriver_intensity_w_cm2 = 2.0e14\nharmonic = 27",
        )
        .unwrap();
        assert!((default.average_power_w().unwrap() - 5e-5).abs() < 1e-12);
        let stored = source_from_toml(
            "type = \"hhg\"\ngas = \"ar\"\ndriver_intensity_w_cm2 = 2.0e14\nharmonic = 27\n\
             pulse_energy_nj = 0.2",
        )
        .unwrap();
        assert!((stored.average_power_w().unwrap() - 2e-6).abs() < 1e-15);
        // Full comb: no harmonic needed (the comb is imaged), passband applies.
        let comb = source_from_toml(
            "type = \"hhg\"\ngas = \"ar\"\ndriver_intensity_w_cm2 = 2.0e14\n\
             full_comb = true\ncomb_passband_nm = [25.0, 35.0]",
        )
        .unwrap();
        assert!((dq(&comb, "comb_line_count") - 5.0).abs() < 1e-12);
        assert!(source_from_toml("type = \"hhg\"\nconversion_efficiency = 2.0").is_err());
    }

    #[test]
    fn test_ics_ssmb_entangled_machine_fields() {
        let ics = source_from_toml(
            "type = \"ics\"\nwavelength_nm = 13.5\nbunch_charge_pc = 100.0\n\
             laser_pulse_energy_mj = 10.0\nrep_rate_hz = 1.0e8",
        )
        .unwrap();
        // Derived: 71 uW in the 2 % band (collection defaults to the band).
        assert!((ics.average_power_w().unwrap() - 7.1227e-5).abs() < 1e-8);
        let ssmb = source_from_toml("type = \"ssmb\"\nbunching_factor = 0.1").unwrap();
        assert!((ssmb.average_power_w().unwrap() - 470.421).abs() < 1e-2);
        let ent = source_from_toml("type = \"entangled\"\npair_rate_hz = 1.0e9").unwrap();
        // Default (bound) scenario: 0.1 / 1e-23 cm^2 / 1e9 pairs/s = 1e13 s.
        assert!((dq(&ent, "exposure_time_bound") / 1e13 - 1.0).abs() < 1e-12);
        assert!(source_from_toml("type = \"entangled\"\npair_rate_hz = -1.0").is_err());
    }

    #[test]
    fn test_vuv_heritage_presets() {
        let arf = source_from_toml("type = \"vuv\"\npreset = \"arf\"").unwrap();
        assert!((arf.wavelength_nm() - 193.368).abs() < 1e-12);
        assert!((arf.average_power_w().unwrap() - 90.0).abs() < 1e-9);
        let krf = source_from_toml("preset = \"krf\"").unwrap(); // type defaults to vuv
        assert!((krf.wavelength_nm() - 248.3).abs() < 1e-12);
        let lamp = source_from_toml("type = \"vuv\"\npreset = \"hg_i\"").unwrap();
        assert!((lamp.wavelength_nm() - 365.0153).abs() < 1e-12);
        assert_eq!(lamp.average_power_w(), None); // CW lamp
        assert_eq!(lamp.spectral_weights().len(), 1);
        let g = source_from_toml("type = \"vuv\"\npreset = \"g_line\"").unwrap();
        assert!((g.wavelength_nm() - 435.8328).abs() < 1e-12);
        // Explicit fields override the preset.
        let tuned =
            source_from_toml("type = \"vuv\"\npreset = \"arf\"\nbandwidth_pm = 0.3").unwrap();
        assert!((tuned.bandwidth_pm() - 0.3).abs() < 1e-12);
        // No preset: the legacy explicit-wavelength source.
        let legacy = source_from_toml("wavelength_nm = 193.368").unwrap();
        match legacy {
            SourceKind::Vuv(v) => {
                assert!((v.bandwidth_pm - 1.1).abs() < 1e-12);
                assert!((v.rep_rate_hz - 4000.0).abs() < 1e-12);
            }
            _ => panic!("expected VUV"),
        }
        assert!(source_from_toml("type = \"vuv\"\npreset = \"xecl\"").is_err());
        // `preset` is also accepted for the XFEL family.
        let erl = source_from_toml("type = \"xfel\"\npreset = \"erl\"").unwrap();
        assert!(erl.average_power_w().unwrap() > 1e4);
    }

    #[test]
    fn test_coherent_euv_examples_are_bright_field() {
        // Regression: these laser-like (sigma ~0.05) sources were paired with a
        // centrally obscured Schwarzschild, whose obscuration blocks the zero
        // order: the examples imaged in dark field.
        use highuvlith_core::aerial::AerialImageEngine;
        use highuvlith_core::source::LithographySource;
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        for name in [
            "sim_hhg.toml",
            "sim_ssmb.toml",
            "sim_xfel.toml",
            "sim_xfel_erl.toml",
            "sim_xfel_cw_sc.toml",
            "sim_ics.toml",
            // P2: same defect at sigma ~0.11-0.17 (clear field 4 % / 16 %, I_max 5.1 / 1.7).
            "sim_sxrl.toml",
            "sim_synchrotron.toml",
        ] {
            let cfg = SimConfig::load(&dir.join(name)).unwrap();
            let source = cfg.to_source().unwrap();
            let optics = cfg.to_optics().unwrap();
            let engine = AerialImageEngine::with_settings(
                &source,
                optics.as_ref(),
                cfg.to_grid().unwrap(),
                cfg.to_imaging_settings().unwrap(),
            )
            .unwrap();
            let d = engine
                .kernel_diagnostics(0.0, source.wavelength_nm())
                .unwrap();
            assert!(
                d.normalized,
                "{name}: dark field (clear field {:.2e})",
                d.clear_field_intensity
            );
        }
    }

    #[test]
    fn test_toml_hhg_parsing() {
        let cfg = parse(
            r#"
            [source]
            type = "hhg"
            gas = "ne"
            driver_wavelength_nm = 800.0
            driver_intensity_w_cm2 = 4.0e14
            harmonic = 59
            monochromator_bandwidth_pm = 15.0

            [optics]
            type = "schwarzschild"
            na = 0.3

            [mask]
            cd_nm = 30.0
            pitch_nm = 100.0
        "#,
        )
        .unwrap();
        match cfg.to_source().unwrap() {
            SourceKind::Hhg(s) => assert!((s.wavelength_nm() - 800.0 / 59.0).abs() < 1e-9),
            _ => panic!("expected Hhg variant"),
        }
    }

    /// Back-compat: a pre-v2 config (no `type` tag, every table present)
    /// still parses, validates and builds the legacy F2 source.
    #[test]
    fn test_toml_back_compat_without_type_tag() {
        let cfg = parse(
            r#"
            # Example VUV lithography simulation configuration
            [source]
            wavelength_nm = 157.63  # F2 excimer laser
            sigma = 0.7
            bandwidth_pm = 1.1

            [optics]
            na = 0.75
            flare_fraction = 0.02

            [mask]
            cd_nm = 65.0
            pitch_nm = 180.0

            [grid]
            size = 256
            pixel_nm = 1.0

            [process]
            dose_mj_cm2 = 30.0
            focus_nm = 0.0
        "#,
        )
        .unwrap();
        assert_eq!(cfg.source.as_ref().unwrap().source_type(), "vuv");
        match cfg.to_source().unwrap() {
            SourceKind::Vuv(v) => {
                assert_eq!(v.wavelength_nm, 157.63);
                assert!(matches!(
                    v.illumination,
                    IlluminationShape::Conventional { sigma } if sigma == 0.7
                ));
            }
            _ => panic!("expected the legacy VUV source"),
        }
        // Minimal legacy form: empty [optics], no [grid]/[process].
        let minimal = parse("[source]\n[optics]\n[mask]\n").unwrap();
        assert_eq!(minimal.to_optics().unwrap().na(), 0.75);
        assert_eq!(minimal.to_source().unwrap().wavelength_nm(), 157.63);
    }

    #[test]
    fn test_toml_lpa_fel_parsing() {
        let cfg = parse(
            r#"
            [source]
            type = "lpa_fel"
            wavelength_nm = 25.0
            sigma = 0.7
            bandwidth_pm = 25.0
            electron_energy_mev = 500.0
            pulse_duration_fs = 10.0
            rep_rate_hz = 1000.0

            [optics]
            type = "schwarzschild"
            na = 0.55

            [mask]
            cd_nm = 30.0
            pitch_nm = 100.0
        "#,
        )
        .unwrap();
        match cfg.to_source().unwrap() {
            SourceKind::LpaFel(s) => {
                assert!((s.wavelength_nm - 25.0).abs() < 1e-9);
                assert!((s.electron_energy_mev - 500.0).abs() < 1e-9);
                assert!((s.pulse_duration_fs - 10.0).abs() < 1e-9);
            }
            _ => panic!("expected LpaFel variant"),
        }
    }

    #[test]
    fn test_to_grid_is_commensurate_with_the_pitch() {
        // 65/180 on 256 px at 1 nm: one 180 nm period, pixel 180/256.
        let cfg = valid_config();
        let grid = cfg.to_grid().unwrap();
        assert_eq!(grid.size, 256);
        assert!((grid.pixel_nm - 0.703125).abs() < 1e-15);
        assert!(cfg.to_mask().unwrap().check_commensurate(&grid).is_ok());
        // Opting out keeps the configured pixel.
        let mut raw = valid_config();
        raw.grid.commensurate = false;
        assert_eq!(raw.to_grid().unwrap().pixel_nm, 1.0);
    }

    #[test]
    fn test_grid_commensurate_defaults_to_true_in_toml() {
        let cfg = parse(
            "[source]\nwavelength_nm = 157.63\n[optics]\nna = 0.75\n[mask]\ncd_nm = 65.0\n\
             pitch_nm = 180.0\n[grid]\nsize = 128\npixel_nm = 2.0\n",
        )
        .unwrap();
        assert!(cfg.grid.commensurate);
        assert!((cfg.to_grid().unwrap().pixel_nm - 180.0 / 128.0).abs() < 1e-12);
    }

    #[test]
    fn test_lab_compact_key_names_aliases_and_fallbacks() {
        use highuvlith_core::source::SourceKind;
        let parse = |body: &str| -> SimConfig {
            toml::from_str(&format!(
                "[source]\n{body}\n\n[optics]\nna = 0.3\n\n[mask]\ncd_nm = 30.0\npitch_nm = 60.0\n"
            ))
            .unwrap()
        };
        // X-ray tube current: `current_ma` (Python / serialized name) and the
        // older `tube_current_ma` alias.
        for key in ["current_ma", "tube_current_ma"] {
            let cfg = parse(&format!("type = \"xray_tube\"\nanode = \"w\"\n{key} = 5.0"));
            match cfg.to_source().unwrap() {
                SourceKind::XrayTube(t) => assert_eq!(t.current_ma, 5.0, "{key}"),
                _ => panic!("expected XrayTube"),
            }
        }
        // SXRL linewidth: `rel_linewidth`, falling back to `rel_bandwidth`.
        for (key, value) in [("rel_linewidth", 2e-4), ("rel_bandwidth", 3e-4)] {
            let cfg = parse(&format!(
                "type = \"sxrl\"\nscheme = \"ag_13nm9\"\n{key} = {value}"
            ));
            match cfg.to_source().unwrap() {
                SourceKind::Sxrl(x) => assert_eq!(x.rel_linewidth, value, "{key}"),
                _ => panic!("expected Sxrl"),
            }
        }
        // Both linewidth spellings at once is ambiguous.
        assert!(parse_err(
            "[source]\ntype = \"sxrl\"\nscheme = \"ag_13nm9\"\nrel_linewidth = 2e-4\n\
             rel_bandwidth = 3e-4\n[optics]\nna = 0.3\n[mask]\ncd_nm = 30.0\npitch_nm = 60.0\n"
        )
        .contains("fallback spelling of rel_linewidth"));
        // DPP collector solid angle: own key, or the LPP `collection_solid_angle_sr`.
        for key in ["collector_solid_angle_sr", "collection_solid_angle_sr"] {
            let cfg = parse(&format!("type = \"dpp\"\nfuel = \"xe\"\n{key} = 1.0"));
            match cfg.to_source().unwrap() {
                SourceKind::Dpp(d) => assert_eq!(d.collector_solid_angle_sr, 1.0, "{key}"),
                _ => panic!("expected Dpp"),
            }
        }
    }

    #[test]
    fn test_toml_xray_tube_anode_presets_and_overrides() {
        let mut cfg = parse(
            "[source]\ntype = \"xray_tube\"\nanode = \"cu\"\n[optics]\nna = 0.2\n[mask]\n\
             cd_nm = 20000.0\npitch_nm = 40000.0\n",
        )
        .unwrap();
        match cfg.to_source().unwrap() {
            SourceKind::XrayTube(s) => {
                // Cu preset: 40 kV / 40 mA; Cu Ka dominates -> ~0.1675 nm mean.
                assert_eq!(s.anode, highuvlith_core::source::XrayAnode::Cu);
                assert!((s.kvp - 40.0).abs() < 1e-12 && (s.current_ma - 40.0).abs() < 1e-12);
                assert!((s.wavelength_nm() - 0.1675).abs() < 1e-3);
            }
            _ => panic!("expected XrayTube"),
        }
        src(&mut cfg).kvp = Some(30.0);
        src(&mut cfg).current_ma = Some(5.0);
        src(&mut cfg).be_window_um = Some(100.0);
        match cfg.to_source().unwrap() {
            SourceKind::XrayTube(s) => {
                assert!((s.kvp - 30.0).abs() < 1e-12);
                assert!((s.current_ma - 5.0).abs() < 1e-12);
                assert!((s.be_window_um - 100.0).abs() < 1e-12);
            }
            _ => panic!("expected XrayTube"),
        }
        // Out-of-range voltage and unknown anodes are rejected.
        src(&mut cfg).kvp = Some(2.0);
        assert!(cfg.validate().is_err());
        src(&mut cfg).kvp = None;
        src(&mut cfg).anode = Some("unobtainium".to_string());
        assert!(cfg.validate().is_err());
        // An explicit wavelength must match the derived mean wavelength.
        src(&mut cfg).anode = Some("cu".to_string());
        src(&mut cfg).current_ma = None;
        src(&mut cfg).be_window_um = None;
        src(&mut cfg).wavelength_nm = Some(13.5);
        assert!(cfg.validate().is_err());
        src(&mut cfg).wavelength_nm = Some(0.1675);
        cfg.validate().unwrap();
    }

    #[test]
    fn test_dpp_fuel_and_etendue_overrides() {
        let mut cfg = family_config("dpp");
        src(&mut cfg).fuel = Some("xe".to_string());
        let limited = cfg.to_source().unwrap();
        // A generous illuminator removes the etendue cut: more power at IF.
        src(&mut cfg).illuminator_etendue_mm2_sr = Some(100.0);
        let open = cfg.to_source().unwrap();
        assert!(open.average_power_w().unwrap() > limited.average_power_w().unwrap());
        match open {
            SourceKind::Dpp(s) => {
                assert!((s.power_at_if_w() - s.collected_power_w()).abs() < 1e-12);
            }
            _ => panic!("expected Dpp"),
        }
        src(&mut cfg).collector_efficiency = Some(1.5);
        assert!(cfg.validate().is_err());
        src(&mut cfg).collector_efficiency = None;
        src(&mut cfg).fuel = Some("gd".to_string());
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_sxrl_fixed_line_cross_check() {
        let mut cfg = family_config("sxrl");
        src(&mut cfg).scheme = Some("ag_13nm9".to_string());
        // 13.5 nm is not the 13.9 nm Ni-like Ag line.
        src(&mut cfg).wavelength_nm = Some(13.5);
        assert!(cfg.validate().is_err());
        src(&mut cfg).wavelength_nm = Some(13.9);
        cfg.validate().unwrap();
        assert!((cfg.to_source().unwrap().wavelength_nm() - 13.9).abs() < 1e-12);
        src(&mut cfg).scheme = Some("krypton_laser".to_string());
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_betatron_undulator_regime_rejected() {
        let mut cfg = family_config("betatron");
        cfg.validate().unwrap();
        // r_beta = 0.1 um gives K < 1 (undulator regime): not modelled.
        src(&mut cfg).betatron_amplitude_um = Some(0.1);
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_smith_purcell_period_or_target() {
        let mut cfg = family_config("smith_purcell");
        // Target wavelength -> derived period (4.433 nm at 30 keV, 90 deg).
        src(&mut cfg).wavelength_nm = Some(13.5);
        match cfg.to_source().unwrap() {
            SourceKind::SmithPurcell(s) => {
                assert!((s.grating_period_nm - 4.433_078).abs() < 1e-5);
                assert!((s.wavelength_nm() - 13.5).abs() < 1e-9);
            }
            _ => panic!("expected SmithPurcell"),
        }
        // Period given: the wavelength is derived and cross-checked.
        src(&mut cfg).grating_period_nm = Some(4.433_078);
        cfg.validate().unwrap();
        src(&mut cfg).wavelength_nm = Some(20.0);
        assert!(cfg.validate().is_err());
        src(&mut cfg).wavelength_nm = None;
        src(&mut cfg).observation_angle_deg = Some(180.0);
        assert!(cfg.validate().is_err());
    }

    // ---------------------------------------------------------------------
    // Strict keys
    // ---------------------------------------------------------------------

    /// serde's own field list of a struct (via the unknown-field error).
    fn serde_fields<T: serde::de::DeserializeOwned>() -> Vec<String> {
        let err = match toml::from_str::<T>("__no_such_key__ = 1") {
            Ok(_) => panic!("deny_unknown_fields is not set"),
            Err(e) => e.message().to_string(),
        };
        let pos = err
            .find("expected")
            .expect("serde lists the expected fields");
        err[pos..]
            .split('`')
            .skip(1)
            .step_by(2)
            .map(str::to_string)
            .collect()
    }

    /// Every [source] field is read by at least one family, and every
    /// family key is a real field — catches a field added without
    /// registering it (it would be accepted by serde but rejected for every
    /// type) and a typo in the key lists.
    #[test]
    fn test_source_key_lists_match_the_struct() {
        let fields = serde_fields::<SourceConfig>();
        for field in &fields {
            if field == "collector_solid_angle_sr" || field == "tube_current_ma" {
                continue; // aliases of collection_solid_angle_sr / current_ma
            }
            assert!(
                !SourceConfig::key_owners(field).is_empty(),
                "[source] field `{field}` is read by no family"
            );
        }
        for (tag, beamline) in SOURCE_CONTEXTS {
            for key in SourceConfig::family_keys(tag, *beamline).unwrap() {
                assert!(
                    fields.iter().any(|f| f == key),
                    "{tag}: `{key}` is not a field"
                );
            }
        }
        for tag in OPTICS_TYPES {
            let fields = serde_fields::<OpticsConfig>();
            for key in OpticsConfig::type_keys(tag).unwrap() {
                assert!(fields.iter().any(|f| f == key), "optics {tag}: `{key}`");
            }
        }
        for pattern in MASK_PATTERNS {
            let fields = serde_fields::<MaskConfig>();
            for key in MaskConfig::pattern_keys(pattern).unwrap() {
                assert!(fields.iter().any(|f| f == key), "mask {pattern}: `{key}`");
            }
        }
        for shape in ILLUMINATION_SHAPES {
            let fields = serde_fields::<IlluminationConfig>();
            for key in IlluminationConfig::shape_keys(shape).unwrap() {
                assert!(
                    fields.iter().any(|f| f == key),
                    "illumination {shape}: `{key}`"
                );
            }
        }
        assert_eq!(
            serde_fields::<SimConfig>(),
            TABLES.iter().map(|s| s.to_string()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_unknown_keys_are_errors_with_suggestions() {
        // Typo in [source] of an LPP config: did-you-mean + the LPP keys + line.
        let err = parse_err(
            "[source]\ntype = \"lpp\"\nconversion_efficency = 0.05\n[optics]\nna = 0.33\n\
             [mask]\ncd_nm = 30.0\npitch_nm = 60.0\n",
        );
        assert!(err.contains("line 3"), "{err}");
        assert!(
            err.contains("unknown key `conversion_efficency` in [source]"),
            "{err}"
        );
        assert!(
            err.contains("did you mean `conversion_efficiency`?"),
            "{err}"
        );
        assert!(
            err.contains("keys read by [source] type = \"lpp\": type, fuel"),
            "{err}"
        );
        // Python keyword names are not TOML keys (numerical_aperture vs na).
        let err = parse_err(
            "[source]\nwavelength_nm = 157.63\n[optics]\nnumerical_aperture = 0.75\n\
             [mask]\ncd_nm = 65.0\npitch_nm = 180.0\n",
        );
        assert!(
            err.contains("unknown key `numerical_aperture` in [optics]"),
            "{err}"
        );
        assert!(
            err.contains("keys read by [optics] type = \"refractive\""),
            "{err}"
        );
        // Every shared table is strict.
        for (table, key) in [
            ("grid", "pixel"),
            ("process", "dose"),
            ("imaging", "max_kernel"),
            ("mask", "cd"),
        ] {
            let err = parse_err(&format!(
                "[source]\nwavelength_nm = 157.63\n[optics]\nna = 0.75\n[{table}]\n{key} = 1\n"
            ));
            assert!(
                err.contains(&format!("unknown key `{key}` in [{table}]")),
                "{err}"
            );
            assert!(err.contains("did you mean"), "{err}");
        }
        // Unknown top-level tables too.
        let err = parse_err("[source]\n[optic]\nna = 0.75\n");
        assert!(
            err.contains("unknown key `optic` in the top level"),
            "{err}"
        );
        assert!(err.contains("did you mean `optics`?"), "{err}");
    }

    #[test]
    fn test_family_keys_not_read_by_the_selected_type_are_errors() {
        // A key of another family: named, with its owner and the valid keys.
        let err = source_from_toml("type = \"lpp\"\nanode = \"w\"")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("[source] type = \"lpp\" does not use 1 of the given keys"),
            "{err}"
        );
        assert!(
            err.contains("`anode` is only read by type = \"xray_tube\""),
            "{err}"
        );
        assert!(
            err.contains("keys read by [source] type = \"lpp\": type, fuel"),
            "{err}"
        );
        // `sigma` is not read by families whose pupil fill is derived.
        for tag in ["xfel", "hhg", "synchrotron", "ics", "ssmb", "sxrl"] {
            let err = source_from_toml(&format!("type = \"{tag}\"\nsigma = 0.5"))
                .unwrap_err()
                .to_string();
            assert!(err.contains("`sigma` is only read by"), "{tag}: {err}");
        }
        // bandwidth_pm is not an undulator key (the line width is derived).
        let err = source_from_toml("type = \"synchrotron\"\nbandwidth_pm = 100.0")
            .unwrap_err()
            .to_string();
        assert!(err.contains("`bandwidth_pm`"), "{err}");
        // ... but the bending-magnet monochromator reads it.
        source_from_toml(
            "type = \"synchrotron\"\nbeamline = \"bending_magnet\"\nbandwidth_pm = 0.2",
        )
        .unwrap();
        // Several offenders are reported together.
        let err = source_from_toml("type = \"dpp\"\nkvp = 30.0\nharmonic = 3")
            .unwrap_err()
            .to_string();
        assert!(err.contains("does not use 2 of the given keys"), "{err}");
    }

    /// LPP and DPP read the same quantities: the solid-angle spellings are
    /// aliases for both families, the µm / mm diameters are both accepted
    /// (converted), and giving both diameters is an error.
    #[test]
    fn test_lpp_and_dpp_etendue_keys_are_harmonized() {
        let lpp = |extra: &str| source_from_toml(&format!("type = \"lpp\"\n{extra}"));
        let geometry = |src: SourceKind| match src {
            SourceKind::Lpp(s) => s.geometry.expect("geometry set"),
            _ => panic!("expected LPP"),
        };
        // DPP spelling in an LPP config is now read (it was silently ignored).
        let a = geometry(lpp("collector_solid_angle_sr = 3.0").unwrap());
        let b = geometry(lpp("collection_solid_angle_sr = 3.0").unwrap());
        assert_eq!(a.collection_solid_angle_sr, 3.0);
        assert_eq!(b.collection_solid_angle_sr, 3.0);
        // mm diameter in an LPP config: converted to µm.
        let g = geometry(lpp("source_diameter_mm = 0.25").unwrap());
        assert!((g.source_diameter_um - 250.0).abs() < 1e-9);
        let g = geometry(lpp("source_diameter_um = 250.0").unwrap());
        assert!((g.source_diameter_um - 250.0).abs() < 1e-9);
        // Both spellings of one quantity: rejected.
        let err = lpp("source_diameter_um = 200.0\nsource_diameter_mm = 0.2")
            .unwrap_err()
            .to_string();
        assert!(err.contains("give the plasma diameter once"), "{err}");
        let err = lpp("collection_solid_angle_sr = 3.0\ncollector_solid_angle_sr = 3.0")
            .unwrap_err()
            .to_string();
        assert!(err.contains("duplicate"), "{err}");
        // DPP: the LPP spellings are read as well.
        let dpp = |extra: &str| match source_from_toml(&format!("type = \"dpp\"\n{extra}")) {
            Ok(SourceKind::Dpp(s)) => s,
            other => panic!("expected DPP, got {other:?}"),
        };
        assert_eq!(
            dpp("collection_solid_angle_sr = 1.2").collector_solid_angle_sr,
            1.2
        );
        assert_eq!(
            dpp("collector_solid_angle_sr = 1.2").collector_solid_angle_sr,
            1.2
        );
        assert!((dpp("source_diameter_um = 400.0").source_diameter_mm - 0.4).abs() < 1e-12);
        assert!((dpp("source_diameter_mm = 0.5").source_diameter_mm - 0.5).abs() < 1e-12);
        // The two spellings give identical sources.
        let um = dpp("source_diameter_um = 300.0");
        let mm = dpp("source_diameter_mm = 0.3");
        assert!((um.power_at_if_w() - mm.power_at_if_w()).abs() < 1e-12);
    }

    #[test]
    fn test_unread_subcommand_tables_are_noted() {
        let text = "[source]\ntype = \"vuv\"\n[optics]\n[mask]\n[deep]\nmode = \"liga\"\n\
                    [throughput]\npreset = \"refractive\"\n";
        let cfg = SimConfig::from_toml_str(text).unwrap();
        let w = cfg.unread_table_notes("simulate");
        assert_eq!(w.len(), 2, "{w:?}");
        assert!(
            w[0].starts_with("[deep] is not read by simulate (ignored"),
            "{w:?}"
        );
        assert!(
            w[1].ends_with("the throughput subcommand reads it)"),
            "{w:?}"
        );
        assert_eq!(cfg.unread_table_notes("deep").len(), 1);
        let plain = SimConfig::from_toml_str("[source]\ntype = \"vuv\"\n").unwrap();
        assert!(plain.unread_table_notes("sweep").is_empty());
    }

    #[test]
    fn test_conditional_keys_are_errors_with_reasons() {
        let reject = |toml: &str, needle: &str| {
            let err = source_from_toml(toml).unwrap_err().to_string();
            assert!(err.contains(needle), "{toml}\n→ {err}");
        };
        // HHG: the passband needs the full comb; the monochromator does not.
        reject(
            "type = \"hhg\"\ncomb_passband_nm = [12.0, 20.0]",
            "only with full_comb = true",
        );
        reject(
            "type = \"hhg\"\nfull_comb = true\nmonochromator_bandwidth_pm = 15.0",
            "removes the monochromator",
        );
        reject(
            "type = \"hhg\"\ndriver_average_power_w = 10.0\npulse_energy_nj = 1.0",
            "derived from driver_average_power_w",
        );
        // XFEL: SASE vs seeded bandwidth keys; ρ given disables the machine.
        reject(
            "type = \"xfel\"\nrel_bandwidth = 1e-4",
            "seeded-mode bandwidth",
        );
        reject(
            "type = \"xfel\"\nmode = \"seeded\"\npierce_parameter = 2e-3",
            "seeded mode uses rel_bandwidth",
        );
        reject(
            "type = \"xfel\"\npierce_parameter = 2e-3\npeak_current_a = 3000.0",
            "disables the machine derivation",
        );
        source_from_toml("type = \"xfel\"\nxfel_preset = \"fermi\"\nrel_bandwidth = 1e-4").unwrap();
        // ICS: the collision derives the pulse energy.
        reject(
            "type = \"ics\"\nbunch_charge_pc = 100.0\npulse_energy_nj = 1.0",
            "derived from the collision keys",
        );
        // SSMB: radiator + explicit b derive the power.
        reject(
            "type = \"ssmb\"\nbunching_factor = 0.1\naverage_power_w = 500.0",
            "the power is derived",
        );
        source_from_toml("type = \"ssmb\"\naverage_current_a = 1.0\naverage_power_w = 500.0")
            .unwrap();
        // LPA-FEL: beam keys need an undulator.
        reject(
            "type = \"lpa_fel\"\npeak_current_a = 1000.0",
            "need an undulator",
        );
        // Synchrotron: the energy spread belongs to the ring beam.
        reject(
            "type = \"synchrotron\"\nelectron_energy_spread_rel = 1e-3",
            "the ring beam",
        );
    }

    #[test]
    fn test_spectral_samples_and_illumination_override() {
        let base = "[optics]\ntype = \"euv_projection\"\n[mask]\ncd_nm = 30.0\npitch_nm = 60.0\n";
        // An annular illuminator fill replaces the XFEL's near-coherent Gaussian.
        let cfg = parse(&format!(
            "[source]\ntype = \"xfel\"\nspectral_samples = 3\n{base}[illumination]\n\
             shape = \"annular\"\nsigma_inner = 0.5\nsigma_outer = 0.8\n"
        ))
        .unwrap();
        let s = cfg.to_source().unwrap();
        assert!(matches!(
            s.illumination(),
            IlluminationShape::Annular { sigma_inner, sigma_outer }
                if *sigma_inner == 0.5 && *sigma_outer == 0.8
        ));
        assert_eq!(s.spectral_samples(), 3);
        assert_eq!(s.intensity_at(0.0, 0.0), 0.0);
        assert_eq!(s.intensity_at(0.6, 0.0), 1.0);
        // Dipole / quadrupole / coherent Gaussian.
        for (shape, keys) in [
            (
                "dipole",
                "sigma_center = 0.7\nsigma_radius = 0.15\norientation_deg = 90.0",
            ),
            (
                "quadrupole",
                "sigma_center = 0.7\nsigma_radius = 0.2\nopening_angle_deg = 30.0",
            ),
            ("coherent_gaussian", "sigma = 0.2"),
            ("conventional", "sigma = 0.5"),
        ] {
            parse(&format!(
                "[source]\ntype = \"xfel\"\n{base}[illumination]\nshape = \"{shape}\"\n{keys}\n"
            ))
            .unwrap_or_else(|e| panic!("{shape}: {e}"));
        }
        // [source] sigma and [illumination] both set the fill: rejected.
        let err = parse_err(&format!(
            "[source]\ntype = \"lpp\"\nsigma = 0.5\n{base}[illumination]\nshape = \"conventional\"\n\
             sigma = 0.9\n"
        ));
        assert!(err.contains("comes from the [illumination] table"), "{err}");
        // Keys of another shape, missing keys, out-of-range values.
        let err = parse_err(&format!(
            "[source]\ntype = \"xfel\"\n{base}[illumination]\nshape = \"annular\"\n\
             sigma_inner = 0.5\nsigma_outer = 0.8\nsigma_radius = 0.1\n"
        ));
        assert!(
            err.contains("`sigma_radius` is only read by shape = \"dipole\""),
            "{err}"
        );
        for bad in [
            "shape = \"annular\"\nsigma_outer = 0.8",
            "shape = \"annular\"\nsigma_inner = 0.8\nsigma_outer = 0.5",
            "shape = \"dipole\"\nsigma_center = 0.9\nsigma_radius = 0.2",
            "shape = \"conventional\"\nsigma = 1.2",
            "shape = \"donut\"",
            "sigma = 0.5",
        ] {
            assert!(
                parse(&format!(
                    "[source]\ntype = \"xfel\"\n{base}[illumination]\n{bad}\n"
                ))
                .is_err(),
                "should reject [illumination] {bad}"
            );
        }
    }

    #[test]
    fn test_mask_patterns_orientation_and_mask_type() {
        use highuvlith_core::mask::{LineOrientation, MaskFeature, MaskType};
        let base = "[source]\nwavelength_nm = 193.368\n[optics]\nna = 0.93\n";
        // Horizontal lines with an offset.
        let cfg = parse(&format!(
            "{base}[mask]\ncd_nm = 80.0\npitch_nm = 160.0\norientation = \"horizontal\"\n\
             offset_nm = 10.0\n"
        ))
        .unwrap();
        match &cfg.to_mask().unwrap().features[0] {
            MaskFeature::LineSpace {
                orientation,
                offset,
                ..
            } => {
                assert_eq!(*orientation, LineOrientation::Horizontal);
                assert_eq!(*offset, 10.0);
            }
            other => panic!("expected LineSpace, got {other:?}"),
        }
        // Contact holes: dark field, rectangular lattice, att-PSM.
        let cfg = parse(&format!(
            "{base}[mask]\npattern = \"contact_holes\"\ndiameter_nm = 90.0\npitch_nm = 200.0\n\
             pitch_y_nm = 250.0\nmask_type = \"att_psm\"\ntransmission = 0.06\n"
        ))
        .unwrap();
        let mask = cfg.to_mask().unwrap();
        assert!(mask.dark_field);
        assert!(
            matches!(mask.mask_type, MaskType::AttenuatedPSM { transmission, phase_deg }
            if transmission == 0.06 && phase_deg == 180.0)
        );
        assert_eq!(cfg.mask.as_ref().unwrap().drawn_width_nm(), Some(90.0));
        // The commensurate grid holds whole periods of both pitches (1000 nm cell).
        let grid = cfg.to_grid().unwrap();
        assert!(mask.check_commensurate(&grid).is_ok());
        // Free-form features.
        let cfg = parse(&format!(
            "{base}[mask]\npattern = \"features\"\ndark_field = true\n\
             [[mask.features]]\ntype = \"rect\"\nx = 0.0\ny = 0.0\nw = 90.0\nh = 500.0\n\
             [[mask.features]]\ntype = \"polygon\"\nvertices = [[100, 0], [150, 0], [150, 60]]\n"
        ))
        .unwrap();
        let mask = cfg.to_mask().unwrap();
        assert_eq!(mask.features.len(), 2);
        assert!(mask.dark_field);
        assert_eq!(cfg.mask.as_ref().unwrap().drawn_width_nm(), None);
        // Keys of another pattern / of att_psm on a binary mask: rejected.
        let err = parse_err(&format!(
            "{base}[mask]\npattern = \"contact_holes\"\ndiameter_nm = 90.0\npitch_nm = 200.0\n\
             cd_nm = 90.0\n"
        ));
        assert!(
            err.contains("`cd_nm` is only read by pattern = \"line_space\""),
            "{err}"
        );
        let err = parse_err(&format!(
            "{base}[mask]\ncd_nm = 80.0\npitch_nm = 160.0\nphase_deg = 90.0\n"
        ));
        assert!(err.contains("mask_type = \"att_psm\""), "{err}");
        // Missing / bad values.
        for bad in [
            "pattern = \"contact_holes\"\npitch_nm = 200.0",
            "pattern = \"contact_holes\"\ndiameter_nm = 250.0\npitch_nm = 200.0",
            "pattern = \"features\"",
            "pattern = \"hexagons\"",
            "orientation = \"diagonal\"",
            "mask_type = \"alt_psm\"",
        ] {
            assert!(
                parse(&format!("{base}[mask]\n{bad}\n")).is_err(),
                "should reject {bad}"
            );
        }
        // Unknown keys inside a feature are caught, with the feature's keys.
        let err = parse_err(&format!(
            "{base}[mask]\npattern = \"features\"\n[[mask.features]]\ntype = \"rect\"\nx = 0.0\n\
             y = 0.0\nwidth = 90.0\nh = 500.0\n"
        ));
        assert!(
            err.contains("unknown key `width` in [mask.features]"),
            "{err}"
        );
    }

    #[test]
    fn test_process_table_values() {
        let base = "[source]\nwavelength_nm = 157.63\n[optics]\nna = 0.75\n[mask]\ncd_nm = 65.0\n\
                    pitch_nm = 180.0\n[process]\n";
        let cfg = parse(&format!(
            "{base}threshold = 0.25\ncd_target_nm = 60.0\ncd_tolerance_pct = 5.0\n"
        ))
        .unwrap();
        assert_eq!(cfg.process.threshold(), 0.25);
        assert_eq!(cfg.process.cd_tolerance_pct(), 5.0);
        for bad in [
            "threshold = -0.1",
            "cd_tolerance_pct = 120.0",
            "dose_mj_cm2 = 0.0",
            "cd_target_nm = -5.0",
        ] {
            assert!(
                parse(&format!("{base}{bad}\n")).is_err(),
                "should reject {bad}"
            );
        }
    }

    /// Tables a command does not need may be omitted (e.g. a throughput
    /// study only needs [source]); commands that need them say so.
    #[test]
    fn test_optional_tables_and_requirements() {
        let cfg = parse("[source]\ntype = \"lpp\"\n").unwrap();
        cfg.to_source().unwrap();
        let err = cfg.to_optics().err().unwrap().to_string();
        assert!(err.contains("needs an [optics] table"), "{err}");
        let err = cfg.to_mask().unwrap_err().to_string();
        assert!(err.contains("needs a [mask] table"), "{err}");
        // Grid without a mask: the configured pixel.
        assert_eq!(cfg.to_grid().unwrap().pixel_nm, 1.0);
        assert!(cfg.tables.contains("source") && !cfg.tables.contains("optics"));
        let empty = parse("").unwrap();
        assert!(empty.to_source().is_err());
    }
}
