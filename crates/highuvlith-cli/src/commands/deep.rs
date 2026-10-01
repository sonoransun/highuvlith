//! `highuvlith deep` — deep-layer (3D / high-aspect-ratio) process modes.
//!
//! One subcommand over the process modules that sculpt resist in depth or
//! print periodic patterns without a projection lens:
//!
//! - `liga` — LIGA deep X-ray shadow printing ([`highuvlith_core::deep_xray`]):
//!   a synchrotron bending-magnet white beam (or an absolute flux-density
//!   table) exposes hundreds of µm of PMMA; Fresnel (default) or Gaussian
//!   proximity model; reports the depth-dose top/bottom ratio, damage margin,
//!   aspect ratio, the exposure time for absolute spectra, sampling warnings,
//!   and optionally a fine 1D edge profile with the sidewall angle.
//! - `grayscale` — analytic 2.5D height mapping
//!   ([`highuvlith_core::grayscale`]): a continuous-transmittance mask carves a
//!   blazed grating, microlens array, or staircase into the resist surface.
//! - `interference` — multi-beam holographic lithography
//!   ([`highuvlith_core::interference`]): interfering plane waves record a
//!   periodic lattice; reports the fringe period and iso-surface fill fraction.
//! - `volumetric` — z-resolved Dill exposure, post-exposure bake, and 3D
//!   development ([`highuvlith_core::volumetric`]): the full aerial-imaging
//!   pipeline run through the resist depth with split-step bleaching, an
//!   optional Gaussian (anisotropic) or chemically amplified bake (`peb`),
//!   and optional fast-marching or level-set development (`develop`) with
//!   surface inhibition and developer depletion, giving per-slice CD and a
//!   sidewall angle.
//! - `talbot` — Talbot self-imaging proximity lithography
//!   ([`highuvlith_core::talbot`]): the Talbot carpet of a transmission
//!   grating, displacement-Talbot (DTL) and achromatic-Talbot (ATL) stationary
//!   images, and two-grating EUV interference lithography (`talbot_mode`).
//!
//! The TOML reuses the standard `[source]` / `[optics]` / `[mask]` / `[grid]`
//! sections (via [`SimConfig`]) plus a `[deep]` table parsed by [`DeepConfig`].
//! With `--output foo.png` each mode writes its characteristic image; otherwise
//! it writes a JSON summary.

use std::path::Path;

use anyhow::Context;
use ndarray::Array2;
use serde::{Deserialize, Serialize};

use highuvlith_core::deep_xray::{
    self, BeamFilter, BendingMagnetBeamline, DeepXrayConfig, LigaExposure, ProximityModel,
    XraySpectrum,
};
use highuvlith_core::grayscale::{self, ContrastCurve};
use highuvlith_core::interference::{self, ExposureKinetics, InterferenceSetup};
use highuvlith_core::io::image_export::{save_png, Colormap};
use highuvlith_core::materials::attenuation::Compound;
use highuvlith_core::source::{LithographySource, SourceKind};
use highuvlith_core::types::Grid2D;
use highuvlith_core::{metrics, volumetric};

use crate::config::SimConfig;
use crate::keys::{self, KeyRules, UnknownKeyHint};

/// The deep-layer process modes selectable via `[deep] mode = "…"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeepMode {
    Liga,
    Grayscale,
    Interference,
    Volumetric,
    Talbot,
}

impl DeepMode {
    /// Parse a mode tag, rejecting anything unknown with the valid list.
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        match s {
            "liga" => Ok(Self::Liga),
            "grayscale" => Ok(Self::Grayscale),
            "interference" => Ok(Self::Interference),
            "volumetric" => Ok(Self::Volumetric),
            "talbot" => Ok(Self::Talbot),
            other => anyhow::bail!(
                "unknown deep mode '{}' (expected one of: liga, grayscale, interference, \
                 volumetric, talbot)",
                other
            ),
        }
    }
}

/// The `[deep]` configuration table. Every field beyond `mode` is optional and
/// falls back to a physically reasonable default, so a minimal
/// `[deep] mode = "…"` still runs. Keys are strict: an unknown key, or a key
/// the selected mode (or sub-mode / option) does not read, is an error
/// ([`DeepConfig::key_rules`]).
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DeepConfig {
    /// Process mode: `liga`, `grayscale`, `interference`, `volumetric`, or
    /// `talbot`.
    pub mode: Option<String>,

    // --- liga ---
    /// Bending-magnet critical energy in keV. If omitted, it is derived from a
    /// `[source] type = "synchrotron" beamline = "bending_magnet"` block, or
    /// the spectrum comes from a `type = "xray_tube"` (tabulated
    /// bremsstrahlung + lines) or `type = "betatron"` (synchrotron-like) source.
    pub critical_energy_kev: Option<f64>,
    /// PMMA resist thickness in µm (LIGA is hundreds of µm).
    pub resist_thickness_um: Option<f64>,
    /// Mask-to-resist proximity gap in µm (sets the Fresnel blur).
    pub proximity_gap_um: Option<f64>,
    /// Minimum printable feature in nm for the max-aspect-ratio figure.
    pub min_feature_nm: Option<f64>,
    /// Developed-depth threshold dose in kJ/cm³ for the LIGA depth map PNG.
    pub develop_threshold_kj_cm3: Option<f64>,
    /// Lateral proximity model: `fresnel` (default, angular-spectrum
    /// propagation of the complex absorber field) or `gaussian` (legacy blur).
    pub diffraction: Option<String>,
    /// Spectral bins (log-spaced for a bending magnet; default 100).
    pub energy_bins: Option<usize>,
    /// Add the optional Grün-range photoelectron blur (crude upper bound).
    pub photoelectron_blur: Option<bool>,
    /// Upstream beam filters (`[[deep.filters]] material = "Be"`,
    /// `thickness_um = 100.0`); material is a preset or `<formula>@<density>`.
    pub filters: Option<Vec<LigaFilter>>,
    /// Mask absorber material (default `Au`) and thickness in µm (default 20).
    pub absorber: Option<String>,
    pub absorber_thickness_um: Option<f64>,
    /// Mask membrane material (default `Ti`) and thickness in µm (default 2).
    pub membrane: Option<String>,
    pub membrane_thickness_um: Option<f64>,
    /// Clearing dose at the resist bottom in kJ/cm³ (default 3).
    pub target_bottom_dose_kj_cm3: Option<f64>,
    /// Damage ceiling for the top dose in kJ/cm³ (default 20).
    pub damage_dose_kj_cm3: Option<f64>,
    /// Absolute bending-magnet exposure: source-to-mask distance in m. With
    /// `vertical_scan_mm` and a `[source]` bending magnet this gives the
    /// absolute flux (ring energy, field, `ring_current_ma`) and exposure time.
    pub source_distance_m: Option<f64>,
    /// Horizontal fan acceptance in mrad (default 5).
    pub horizontal_acceptance_mrad: Option<f64>,
    /// Height of the uniform vertical scan at the mask in mm.
    pub vertical_scan_mm: Option<f64>,
    /// Absolute spectral flux density at the mask, `[[E_keV, photons s⁻¹ mm⁻²
    /// keV⁻¹], ...]` (e.g. an X-ray tube); overrides every other spectrum.
    pub flux_density: Option<Vec<[f64; 2]>>,
    /// Refuse to run when the grid cannot resolve the Fresnel scale.
    pub strict_sampling: Option<bool>,
    /// Also compute the fine 1D straight-edge dose profile and sidewall angle.
    pub edge_profile: Option<bool>,
    /// Sampling of the edge profile in nm (default 10).
    pub edge_dx_nm: Option<f64>,
    /// Half-width of the edge-profile window in nm (default 3000).
    pub edge_half_width_nm: Option<f64>,

    // --- grayscale ---
    /// Target relief: `blazed`, `microlens`, or `staircase`.
    pub target: Option<String>,
    /// Blazed-grating period in pixels.
    pub period_px: Option<usize>,
    /// Microlens pitch in pixels.
    pub pitch_px: Option<usize>,
    /// Number of staircase steps.
    pub n_levels: Option<usize>,
    /// Peak-to-valley relief depth in nm (blazed / staircase).
    pub depth_nm: Option<f64>,
    /// Microlens sag in nm.
    pub sag_nm: Option<f64>,
    /// Resist film thickness in nm.
    pub thickness_nm: Option<f64>,
    /// Single-exposure dose in mJ/cm² used to synthesize the mask.
    pub dose_mj_cm2: Option<f64>,
    /// Contrast-curve threshold dose D_th in mJ/cm².
    pub d_th: Option<f64>,
    /// Contrast-curve clearing dose D_clear in mJ/cm².
    pub d_clear: Option<f64>,

    // --- interference ---
    /// Beam preset: `two_beam`, `three_beam_hex`, or `four_beam_umbrella`.
    pub preset: Option<String>,
    /// Air-side incidence half-angle in degrees.
    pub half_angle_deg: Option<f64>,
    /// Refractive index of the recording medium (resist).
    pub n_medium: Option<f64>,
    /// Two-photon (I²) kinetics instead of single-photon.
    pub two_photon: Option<bool>,
    /// Recording depth (z span) in nm.
    pub z_span_nm: Option<f64>,
    /// Dose scale for the Dill dose→PAC map.
    pub dose_scale: Option<f64>,
    /// Dill C coefficient for the dose→PAC map.
    pub dill_c: Option<f64>,
    /// PAC threshold for the iso-surface fill fraction (default 0.5).
    pub fill_threshold: Option<f64>,

    // --- shared (liga / interference / volumetric) ---
    /// Number of z slices through the resist.
    pub nz: Option<usize>,

    // --- volumetric ---
    /// Number of defocus planes for the volumetric aerial image.
    pub n_defocus_planes: Option<usize>,
    /// Split-step Dill dose steps (1 = static absorption).
    pub dose_steps: Option<usize>,
    /// Resist thickness in nm (default 150 = the core `FilmStack` /
    /// `ResistParams` default).
    pub resist_thickness_nm: Option<f64>,
    /// PAC threshold for the per-slice CD and depth map (default 0.5).
    pub develop_threshold: Option<f64>,

    // --- volumetric: post-exposure bake and development ---
    /// Post-exposure bake: `gaussian` (default), `none`, or `car`.
    pub peb: Option<String>,
    /// Gaussian PEB lateral diffusion length in nm (default 30 = the
    /// resist's `peb_diffusion_nm`).
    pub peb_lateral_nm: Option<f64>,
    /// Gaussian PEB vertical diffusion length in nm (default: lateral).
    pub peb_vertical_nm: Option<f64>,
    /// Depth profile of the vertical diffusivity
    /// `1 + (r − 1)·exp(−z/δ)`: surface ratio r (needs `peb_vertical_decay_nm`).
    pub peb_vertical_surface_ratio: Option<f64>,
    /// Decay length δ (nm) of the vertical-diffusivity depth profile.
    pub peb_vertical_decay_nm: Option<f64>,
    /// CAR bake time in s (default 60).
    pub car_peb_time_s: Option<f64>,
    /// CAR deprotection rate constant k_amp in 1/s (default 0.1).
    pub car_k_amp: Option<f64>,
    /// CAR acid–quencher neutralization rate constant k_q in 1/s (default 10).
    pub car_k_quench: Option<f64>,
    /// CAR base-quencher loading as a fraction of the PAG (default 0.15).
    pub car_quencher: Option<f64>,
    /// CAR acid diffusivity in nm²/s (default 2).
    pub car_acid_diffusivity_nm2_s: Option<f64>,
    /// CAR quencher diffusivity in nm²/s (default 0.5).
    pub car_quencher_diffusivity_nm2_s: Option<f64>,
    /// CAR vertical-to-lateral diffusivity ratio (default 1).
    pub car_vertical_ratio: Option<f64>,
    /// Development: `threshold` (default: PAC-threshold CDs only), `fmm`
    /// (fast marching, static rate), or `level_set` (moving boundary).
    pub develop: Option<String>,
    /// Development time in s (default 60).
    pub dev_time_s: Option<f64>,
    /// Surface inhibition: relative dissolution rate at the top surface.
    pub surface_rate_ratio: Option<f64>,
    /// Surface inhibition depth in nm.
    pub inhibition_depth_nm: Option<f64>,
    /// Lateral boundary for the bake and development: `periodic` (default)
    /// or `reflecting`.
    pub lateral_boundary: Option<String>,
    /// Developer depletion (level set only): `none`, `exponential`,
    /// `loading`, or `local_loading`.
    pub depletion: Option<String>,
    /// Developer ageing time constant in s (`exponential`).
    pub depletion_time_constant_s: Option<f64>,
    /// Developer capacity per unit area in nm of resist (`loading`,
    /// `local_loading`).
    pub loading_capacity_nm: Option<f64>,
    /// Lateral depletion length in nm (`local_loading`).
    pub loading_length_nm: Option<f64>,

    // --- talbot ---
    /// Talbot sub-mode: `carpet`, `dtl`, `atl`, or `euv_il`.
    pub talbot_mode: Option<String>,
    /// Transmission-grating period in nm (default: `[mask] pitch_nm`).
    pub grating_period_nm: Option<f64>,
    /// Grating profile: `amplitude` (binary slits) or `phase` (binary phase).
    pub grating_type: Option<String>,
    /// Open (amplitude) or phase-shifted (phase) fraction of each period.
    pub duty_cycle: Option<f64>,
    /// Phase step of a phase grating in radians (default π).
    pub phase_rad: Option<f64>,
    /// Highest retained diffraction order |n| (default 10).
    pub max_order: Option<usize>,
    /// Free-space transfer function: `exact` (angular spectrum) or `paraxial`.
    pub propagation: Option<String>,
    /// Mask-to-wafer gap in µm (dtl / atl).
    pub gap_um: Option<f64>,
    /// ATL bandwidth in nm (Gaussian FWHM or flat-top full width). Default:
    /// the `[source]` bandwidth (`bandwidth_pm` / 1000).
    pub bandwidth_nm: Option<f64>,
    /// ATL spectrum shape: `gaussian` or `flat`.
    pub spectrum_shape: Option<String>,
    /// DTL gap-scan length in Talbot lengths (default 1).
    pub scan_talbot_lengths: Option<f64>,
    /// Carpet depth in µm (default 2 Talbot lengths).
    pub carpet_z_max_um: Option<f64>,
    /// Carpet rows (propagation-distance samples; default 256).
    pub carpet_nz: Option<usize>,
    /// Two-grating EUV-IL diffraction order m (fringe period p/(2m)).
    pub il_order: Option<u32>,
    /// Resist intensity absorption coefficient in 1/nm (talbot volumes).
    pub absorption_per_nm: Option<f64>,
}

/// One `[[deep.filters]]` entry: an upstream LIGA beam filter.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LigaFilter {
    /// Preset (`Be`, `Al`, `Kapton`, `Ti`, `Si`, `diamond`, ...) or
    /// `<formula>@<density g/cm³>`.
    pub material: String,
    /// Thickness in µm.
    pub thickness_um: f64,
}

/// Wrapper to pull the `[deep]` table out of the same TOML file that carries
/// the `[source]` / `[optics]` / `[mask]` / `[grid]` sections.
#[derive(Debug, Deserialize)]
struct DeepFile {
    #[serde(default)]
    deep: DeepConfig,
}

const LIGA_KEYS: &[&str] = &[
    "mode",
    "critical_energy_kev",
    "flux_density",
    "source_distance_m",
    "horizontal_acceptance_mrad",
    "vertical_scan_mm",
    "resist_thickness_um",
    "proximity_gap_um",
    "diffraction",
    "energy_bins",
    "photoelectron_blur",
    "filters",
    "absorber",
    "absorber_thickness_um",
    "membrane",
    "membrane_thickness_um",
    "target_bottom_dose_kj_cm3",
    "damage_dose_kj_cm3",
    "develop_threshold_kj_cm3",
    "min_feature_nm",
    "strict_sampling",
    "edge_profile",
    "edge_dx_nm",
    "edge_half_width_nm",
    "nz",
];
const GRAYSCALE_KEYS: &[&str] = &[
    "mode",
    "target",
    "period_px",
    "pitch_px",
    "n_levels",
    "depth_nm",
    "sag_nm",
    "thickness_nm",
    "dose_mj_cm2",
    "d_th",
    "d_clear",
];
const INTERFERENCE_KEYS: &[&str] = &[
    "mode",
    "preset",
    "half_angle_deg",
    "n_medium",
    "two_photon",
    "z_span_nm",
    "dose_scale",
    "dill_c",
    "fill_threshold",
    "nz",
];
const GAUSSIAN_PEB_KEYS: &[&str] = &[
    "peb_lateral_nm",
    "peb_vertical_nm",
    "peb_vertical_surface_ratio",
    "peb_vertical_decay_nm",
];
const CAR_PEB_KEYS: &[&str] = &[
    "car_peb_time_s",
    "car_k_amp",
    "car_k_quench",
    "car_quencher",
    "car_acid_diffusivity_nm2_s",
    "car_quencher_diffusivity_nm2_s",
    "car_vertical_ratio",
];
const DEPLETION_KEYS: &[&str] = &[
    "depletion",
    "depletion_time_constant_s",
    "loading_capacity_nm",
    "loading_length_nm",
];
const VOLUMETRIC_KEYS: &[&str] = &[
    "mode",
    "dose_mj_cm2",
    "resist_thickness_nm",
    "nz",
    "n_defocus_planes",
    "dose_steps",
    "develop_threshold",
    "peb",
    "peb_lateral_nm",
    "peb_vertical_nm",
    "peb_vertical_surface_ratio",
    "peb_vertical_decay_nm",
    "car_peb_time_s",
    "car_k_amp",
    "car_k_quench",
    "car_quencher",
    "car_acid_diffusivity_nm2_s",
    "car_quencher_diffusivity_nm2_s",
    "car_vertical_ratio",
    "develop",
    "dev_time_s",
    "surface_rate_ratio",
    "inhibition_depth_nm",
    "lateral_boundary",
    "depletion",
    "depletion_time_constant_s",
    "loading_capacity_nm",
    "loading_length_nm",
];
/// Keys of the resist recording (Dill exposure → PAC → fill fraction).
const RECORDING_KEYS: &[&str] = &[
    "n_medium",
    "absorption_per_nm",
    "z_span_nm",
    "nz",
    "dose_scale",
    "dill_c",
    "fill_threshold",
];
const TALBOT_KEYS: &[&str] = &[
    "mode",
    "talbot_mode",
    "grating_period_nm",
    "grating_type",
    "duty_cycle",
    "phase_rad",
    "max_order",
    "propagation",
    "gap_um",
    "bandwidth_nm",
    "spectrum_shape",
    "scan_talbot_lengths",
    "carpet_z_max_um",
    "carpet_nz",
    "il_order",
    "n_medium",
    "absorption_per_nm",
    "z_span_nm",
    "nz",
    "dose_scale",
    "dill_c",
    "fill_threshold",
];

/// Every `[deep] mode` tag.
pub const DEEP_MODES: &[&str] = &["liga", "grayscale", "interference", "volumetric", "talbot"];

impl DeepConfig {
    /// Resolve and validate the process mode tag.
    pub fn resolve_mode(&self) -> anyhow::Result<DeepMode> {
        let tag = self.mode.as_deref().ok_or_else(|| {
            anyhow::anyhow!(
                "[deep] mode is required (one of: liga, grayscale, interference, volumetric, \
                 talbot)"
            )
        })?;
        DeepMode::parse(tag)
    }

    /// Keys read by a mode before any conditional rule.
    pub fn mode_keys(mode: &str) -> Option<&'static [&'static str]> {
        Some(match mode {
            "liga" => LIGA_KEYS,
            "grayscale" => GRAYSCALE_KEYS,
            "interference" => INTERFERENCE_KEYS,
            "volumetric" => VOLUMETRIC_KEYS,
            "talbot" => TALBOT_KEYS,
            _ => return None,
        })
    }

    /// The keys the selected mode reads, after the conditional rules (a
    /// sub-mode, option or source that switches a key off). `source_type`
    /// is the `[source] type` of the file, if any.
    pub fn key_rules(&self, source_type: Option<&str>) -> anyhow::Result<KeyRules> {
        let mode = self.resolve_mode()?;
        let tag = self.mode.as_deref().unwrap_or_default();
        let mut rules = KeyRules::new(
            "[deep]",
            format!("mode = \"{tag}\""),
            Self::mode_keys(tag).unwrap_or_default(),
        );
        match mode {
            DeepMode::Liga => {
                let absolute_bm = [
                    "source_distance_m",
                    "horizontal_acceptance_mrad",
                    "vertical_scan_mm",
                ];
                if self.flux_density.is_some() {
                    rules.exclude_all(
                        &[
                            "critical_energy_kev",
                            "source_distance_m",
                            "horizontal_acceptance_mrad",
                            "vertical_scan_mm",
                        ],
                        "the flux_density table is the whole (absolute) spectrum",
                    );
                } else if self.critical_energy_kev.is_some() {
                    rules.exclude_all(
                        &absolute_bm,
                        "an absolute exposure takes the spectrum from the [source] (ring / tube / \
                         betatron); critical_energy_kev is a relative bending-magnet spectrum",
                    );
                } else if matches!(source_type, Some("xray_tube" | "betatron")) {
                    rules.exclude_all(
                        &["horizontal_acceptance_mrad", "vertical_scan_mm"],
                        "a lab source is a point source: source_distance_m alone sets its flux \
                         density (inverse-square law)",
                    );
                } else if self.vertical_scan_mm.is_none() {
                    rules.exclude(
                        "horizontal_acceptance_mrad",
                        "the horizontal acceptance belongs to the absolute bending-magnet \
                         exposure (source_distance_m + vertical_scan_mm)",
                    );
                }
                if self.edge_profile != Some(true) {
                    rules.exclude_all(
                        &["edge_dx_nm", "edge_half_width_nm"],
                        "the edge-profile sampling needs edge_profile = true",
                    );
                }
                if self
                    .diffraction
                    .as_deref()
                    .map(str::to_ascii_lowercase)
                    .as_deref()
                    == Some("gaussian")
                {
                    rules.exclude(
                        "strict_sampling",
                        "the sampling check is for the Fresnel model (diffraction = \"fresnel\")",
                    );
                }
            }
            DeepMode::Grayscale => {
                // Shape keys of each target; the other targets' keys are off.
                const TARGET_KEYS: [(&str, [&str; 2]); 3] = [
                    ("blazed", ["period_px", "depth_nm"]),
                    ("microlens", ["pitch_px", "sag_nm"]),
                    ("staircase", ["n_levels", "depth_nm"]),
                ];
                let target = self.target.as_deref().unwrap_or("blazed");
                let own = TARGET_KEYS
                    .iter()
                    .find(|(t, _)| *t == target)
                    .map_or([""; 2], |(_, k)| *k);
                for key in ["period_px", "pitch_px", "n_levels", "depth_nm", "sag_nm"] {
                    if own.contains(&key) {
                        continue;
                    }
                    let readers: Vec<&str> = TARGET_KEYS
                        .iter()
                        .filter(|(_, k)| k.contains(&key))
                        .map(|(t, _)| *t)
                        .collect();
                    rules.exclude(
                        key,
                        format!(
                            "read by target = {} (this is target = \"{target}\")",
                            readers.join(" / ")
                        ),
                    );
                }
            }
            DeepMode::Interference => {}
            DeepMode::Volumetric => {
                let peb = self.peb.as_deref().unwrap_or(DEFAULT_PEB);
                if peb != "gaussian" {
                    rules.exclude_all(
                        GAUSSIAN_PEB_KEYS,
                        "the Gaussian bake keys need peb = \"gaussian\"",
                    );
                }
                if peb != "car" {
                    rules.exclude_all(
                        CAR_PEB_KEYS,
                        "the chemically amplified bake keys need peb = \"car\"",
                    );
                }
                let develop = self.develop.as_deref().unwrap_or("threshold");
                if develop == "threshold" {
                    rules.exclude_all(
                        &["dev_time_s", "surface_rate_ratio", "inhibition_depth_nm"],
                        "development keys need develop = \"fmm\" or \"level_set\" (the threshold \
                         tier only reports latent-image CDs)",
                    );
                    if peb == "none" {
                        rules.exclude(
                            "lateral_boundary",
                            "the lateral boundary applies to the bake and the development; set peb \
                             or develop",
                        );
                    }
                }
                if develop != "level_set" {
                    rules.exclude_all(
                        DEPLETION_KEYS,
                        "developer depletion needs develop = \"level_set\" (fast marching and the \
                         threshold tier assume a static rate)",
                    );
                } else {
                    let (needs, reason): (&[&str], &str) = match self.depletion.as_deref().unwrap_or("none") {
                        "exponential" => (&["depletion_time_constant_s"], "depletion = \"exponential\" reads only depletion_time_constant_s"),
                        "loading" => (&["loading_capacity_nm"], "depletion = \"loading\" reads only loading_capacity_nm"),
                        "local_loading" => (&["loading_capacity_nm", "loading_length_nm"], "depletion = \"local_loading\" reads loading_capacity_nm and loading_length_nm"),
                        _ => (&[], "the depletion parameters need depletion = \"exponential\", \"loading\" or \"local_loading\""),
                    };
                    for key in [
                        "depletion_time_constant_s",
                        "loading_capacity_nm",
                        "loading_length_nm",
                    ] {
                        if !needs.contains(&key) {
                            rules.exclude(key, reason);
                        }
                    }
                }
            }
            DeepMode::Talbot => {
                if self.grating_type.as_deref().unwrap_or("amplitude") != "phase" {
                    rules.exclude(
                        "phase_rad",
                        "phase_rad is the step of grating_type = \"phase\"",
                    );
                }
                let sub = self.talbot_mode.as_deref().unwrap_or("carpet");
                let mut off: Vec<(&'static str, &'static str)> = Vec::new();
                let carpet: &[&'static str] = &["carpet_z_max_um", "carpet_nz"];
                let atl: &[&'static str] = &["bandwidth_nm", "spectrum_shape"];
                match sub {
                    "carpet" => {
                        off.extend(RECORDING_KEYS.iter().map(|k| {
                            (
                                *k,
                                "the carpet is a free-space intensity map (no resist recording)",
                            )
                        }));
                        off.extend(
                            ["gap_um", "scan_talbot_lengths", "il_order"]
                                .iter()
                                .map(|k| (*k, "not used by talbot_mode = \"carpet\"")),
                        );
                        off.extend(atl.iter().map(|k| (*k, "the carpet is monochromatic; the band belongs to talbot_mode = \"atl\"")));
                    }
                    "dtl" => {
                        off.extend(
                            carpet
                                .iter()
                                .map(|k| (*k, "carpet keys need talbot_mode = \"carpet\"")),
                        );
                        off.extend(
                            atl.iter()
                                .map(|k| (*k, "the band belongs to talbot_mode = \"atl\"")),
                        );
                        off.push(("il_order", "il_order belongs to talbot_mode = \"euv_il\""));
                    }
                    "atl" => {
                        off.extend(
                            carpet
                                .iter()
                                .map(|k| (*k, "carpet keys need talbot_mode = \"carpet\"")),
                        );
                        off.push((
                            "scan_talbot_lengths",
                            "the gap scan belongs to talbot_mode = \"dtl\"",
                        ));
                        off.push(("il_order", "il_order belongs to talbot_mode = \"euv_il\""));
                    }
                    "euv_il" => {
                        off.extend(
                            carpet
                                .iter()
                                .map(|k| (*k, "carpet keys need talbot_mode = \"carpet\"")),
                        );
                        off.extend(
                            atl.iter()
                                .map(|k| (*k, "the band belongs to talbot_mode = \"atl\"")),
                        );
                        off.push((
                            "gap_um",
                            "two-grating interference has no mask-wafer gap scan",
                        ));
                        off.push((
                            "scan_talbot_lengths",
                            "the gap scan belongs to talbot_mode = \"dtl\"",
                        ));
                        off.push((
                            "propagation",
                            "the two-grating fringes are computed in closed form",
                        ));
                    }
                    _ => {}
                }
                for (key, reason) in off {
                    rules.exclude(key, reason);
                }
            }
        }
        Ok(rules)
    }

    /// Reject keys the selected mode does not read.
    pub fn check_keys(&self, source_type: Option<&str>) -> anyhow::Result<()> {
        let owners = |key: &str| -> Vec<String> {
            DEEP_MODES
                .iter()
                .filter(|m| Self::mode_keys(m).is_some_and(|k| k.contains(&key)))
                .map(|m| format!("mode = \"{m}\""))
                .collect()
        };
        self.key_rules(source_type)?
            .check(&keys::present_keys(self), &owners)
    }

    /// Parse the `[deep]` table of a config file (strict keys).
    pub fn from_toml_str(text: &str) -> anyhow::Result<Self> {
        Ok(keys::parse_toml::<DeepFile>(text, &deep_table_hint)?.deep)
    }
}

/// Context hint for an unknown `[deep]` key: the keys of the selected mode.
fn deep_table_hint(
    path: &[String],
    raw: &toml::Table,
    expected: &[String],
) -> Option<UnknownKeyHint> {
    if path != ["deep"] || !expected.iter().any(|e| e == "talbot_mode") {
        return None;
    }
    let mode = raw.get("deep")?.get("mode")?.as_str()?;
    Some(UnknownKeyHint {
        label: format!("[deep] mode = \"{mode}\""),
        keys: DeepConfig::mode_keys(mode)?.to_vec(),
    })
}

/// Shared tables each mode reads (the others are noted as ignored).
fn tables_read(mode: DeepMode, deep: &DeepConfig) -> &'static [&'static str] {
    match mode {
        DeepMode::Liga if deep.flux_density.is_some() || deep.critical_energy_kev.is_some() => {
            &["mask", "grid"]
        }
        DeepMode::Liga => &["source", "mask", "grid"],
        DeepMode::Grayscale => &["grid"],
        DeepMode::Interference => &["source", "grid"],
        DeepMode::Volumetric => &[
            "source",
            "optics",
            "mask",
            "grid",
            "process",
            "imaging",
            "illumination",
        ],
        DeepMode::Talbot => &["source", "mask", "grid"],
    }
}

/// Entry point for `highuvlith deep`.
pub fn run(config_path: &Path, output: Option<&Path>) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(config_path)
        .with_context(|| format!("reading {}", config_path.display()))?;
    let config = SimConfig::from_toml_str(&content)
        .map_err(|e| anyhow::anyhow!("{}: {e}", config_path.display()))?;
    if !config.has_table("deep") {
        anyhow::bail!(
            "{}: `highuvlith deep` needs a [deep] table with a mode (one of: {})",
            config_path.display(),
            DEEP_MODES.join(", ")
        );
    }
    let deep = DeepConfig::from_toml_str(&content)
        .map_err(|e| anyhow::anyhow!("{}: {e}", config_path.display()))?;
    validate(&config, &deep)?;
    let mode = deep.resolve_mode()?;
    eprintln!("Deep mode: {:?}", mode);
    run_mode(&config, &deep, output)
}

/// Validate the shared tables and the `[deep]` keys of a deep-mode run, and
/// note shared tables the mode does not read.
pub fn validate(config: &SimConfig, deep: &DeepConfig) -> anyhow::Result<()> {
    config.validate()?;
    let mode = deep.resolve_mode()?;
    deep.check_keys(config.source.as_ref().map(|s| s.source_type()))?;
    let read = tables_read(mode, deep);
    for table in [
        "source",
        "optics",
        "mask",
        "process",
        "imaging",
        "illumination",
    ] {
        if config.has_table(table) && !read.contains(&table) {
            eprintln!(
                "note: [{table}] is not read by deep mode \"{}\" (ignored)",
                deep.mode.as_deref().unwrap_or_default()
            );
        }
    }
    if mode == DeepMode::Volumetric && config.imaging.spectrum() != "monochromatic" {
        anyhow::bail!(
            "deep mode \"volumetric\" images at the centre wavelength only; remove [imaging] \
             spectrum = \"{}\"",
            config.imaging.spectrum()
        );
    }
    Ok(())
}

/// Run the selected mode on validated tables.
pub fn run_mode(
    config: &SimConfig,
    deep: &DeepConfig,
    output: Option<&Path>,
) -> anyhow::Result<()> {
    match deep.resolve_mode()? {
        DeepMode::Liga => run_liga(config, deep, output),
        DeepMode::Grayscale => run_grayscale(config, deep, output),
        DeepMode::Interference => run_interference(config, deep, output),
        DeepMode::Volumetric => run_volumetric(config, deep, output),
        DeepMode::Talbot => run_talbot(config, deep, output),
    }
}

/// Write `data` as a PNG if `output` ends in `.png`, otherwise write `summary`
/// as pretty JSON to `output`. A `None` output does neither.
fn write_output(
    output: Option<&Path>,
    summary: &serde_json::Value,
    image: &Array2<f64>,
    colormap: Colormap,
) -> anyhow::Result<()> {
    let Some(out_path) = output else {
        return Ok(());
    };
    if out_path.extension().is_some_and(|ext| ext == "png") {
        save_png(image, out_path, colormap)
            .map_err(|e| anyhow::anyhow!("failed to save PNG: {}", e))?;
    } else {
        std::fs::write(out_path, serde_json::to_string_pretty(summary)?)?;
    }
    eprintln!("Output written to {}", out_path.display());
    Ok(())
}

// ============================================================================
// LIGA deep X-ray
// ============================================================================

/// Energy bins for an X-ray-tube spectrum handed to the LIGA module.
const XRAY_TUBE_SPECTRUM_BINS: usize = 200;

/// Select the LIGA exposure spectrum, in order of precedence:
/// 1. `[deep] flux_density` — an absolute table (photons s⁻¹ mm⁻² keV⁻¹);
/// 2. `[deep] critical_energy_kev` — a relative bending-magnet spectrum;
/// 3. the `[source]`:
///    - synchrotron bending magnet: absolute with `source_distance_m` +
///      `vertical_scan_mm` (ring current, scan geometry), else relative;
///    - X-ray tube: absolute with `source_distance_m` (isotropic point source,
///      inverse-square law; filtered bremsstrahlung + characteristic lines),
///      else the relative tabulated spectrum;
///    - LWFA betatron: absolute with `source_distance_m` (flat-top cone of
///      half-angle K/γ, averaged over the repetition rate), else the relative
///      synchrotron-like spectrum at its derived critical energy.
fn liga_spectrum(config: &SimConfig, deep: &DeepConfig) -> anyhow::Result<XraySpectrum> {
    if let Some(table) = &deep.flux_density {
        let pairs: Vec<(f64, f64)> = table.iter().map(|p| (p[0], p[1])).collect();
        return Ok(XraySpectrum::from_flux_density(&pairs)?);
    }
    if let Some(e_c) = deep.critical_energy_kev {
        if !(e_c.is_finite() && e_c > 0.0) {
            anyhow::bail!("[deep] critical_energy_kev must be > 0, got {}", e_c);
        }
        return Ok(XraySpectrum::BendingMagnet {
            critical_energy_kev: e_c,
        });
    }
    if config.source.is_none() {
        anyhow::bail!(
            "liga mode needs a spectrum: [deep] critical_energy_kev, [deep] flux_density, or a \
             [source] (type = \"synchrotron\" beamline = \"bending_magnet\", \"xray_tube\" or \
             \"betatron\")"
        );
    }
    let distance_mm = match deep.source_distance_m {
        Some(l) if !(l.is_finite() && l > 0.0) => {
            anyhow::bail!("[deep] source_distance_m must be > 0, got {l}")
        }
        other => other.map(|l| l * 1e3),
    };
    match config.to_source()? {
        SourceKind::XrayTube(src) => Ok(match distance_mm {
            Some(d) => XraySpectrum::from_flux_density(
                &src.spectral_flux_density(d, XRAY_TUBE_SPECTRUM_BINS),
            )?,
            None => src.xray_spectrum(XRAY_TUBE_SPECTRUM_BINS),
        }),
        SourceKind::Betatron(src) => Ok(match distance_mm {
            Some(d) => XraySpectrum::from_flux_density(
                &src.spectral_flux_density(d, XRAY_TUBE_SPECTRUM_BINS),
            )?,
            None => src.xray_spectrum(),
        }),
        SourceKind::Synchrotron(src) if src.critical_energy_kev().is_some() => {
            match (deep.source_distance_m, deep.vertical_scan_mm) {
                (Some(l), Some(h)) => {
                    let b = BendingMagnetBeamline::from_synchrotron(
                        &src,
                        l,
                        deep.horizontal_acceptance_mrad.unwrap_or(5.0),
                        h,
                    )
                    .ok_or_else(|| anyhow::anyhow!("[source] is not a bending magnet"))?;
                    b.validate()?;
                    Ok(XraySpectrum::BendingMagnetBeamline(b))
                }
                (None, None) => XraySpectrum::from_synchrotron(&src)
                    .ok_or_else(|| anyhow::anyhow!("[source] has no bending-magnet spectrum")),
                _ => anyhow::bail!(
                    "absolute LIGA exposure needs both [deep] source_distance_m and \
                     vertical_scan_mm"
                ),
            }
        }
        SourceKind::Synchrotron(_) => anyhow::bail!(
            "liga mode needs a bending-magnet white beam, but [source] is a synchrotron \
             undulator (quasi-monochromatic, no white-beam spectrum); set [source] beamline = \
             \"bending_magnet\" or give [deep] critical_energy_kev"
        ),
        other => anyhow::bail!(
            "liga mode requires [deep] critical_energy_kev or flux_density, or [source] type = \
             \"synchrotron\" beamline = \"bending_magnet\", \"xray_tube\", or \"betatron\" (got \
             source type '{}')",
            other.kind_label()
        ),
    }
}

/// Build the LIGA spectrum and depth-dose config, and run the 1D depth-dose
/// exposure. Factored out so tests can assert on the exposure directly.
fn liga_setup(
    config: &SimConfig,
    deep: &DeepConfig,
) -> anyhow::Result<(DeepXrayConfig, LigaExposure)> {
    let mut dx = DeepXrayConfig::pmma_default(liga_spectrum(config, deep)?);
    if let Some(t) = deep.resist_thickness_um {
        dx.resist_thickness_um = t;
    }
    if let Some(g) = deep.proximity_gap_um {
        if g < 0.0 {
            anyhow::bail!("[deep] proximity_gap_um must be >= 0, got {}", g);
        }
        dx.proximity_gap_um = g;
    }
    if let Some(n) = deep.energy_bins {
        dx.energy_bins = n.max(1);
    }
    if let Some(b) = deep.photoelectron_blur {
        dx.photoelectron_blur = b;
    }
    if let Some(m) = &deep.diffraction {
        dx.proximity_model = match m.to_ascii_lowercase().as_str() {
            "fresnel" => ProximityModel::Fresnel,
            "gaussian" => ProximityModel::Gaussian,
            other => anyhow::bail!(
                "unknown [deep] diffraction '{}' (expected fresnel or gaussian)",
                other
            ),
        };
    }
    if deep.absorber.is_some() || deep.absorber_thickness_um.is_some() {
        dx.absorber = BeamFilter::new(
            Compound::from_spec(deep.absorber.as_deref().unwrap_or("Au"))?,
            deep.absorber_thickness_um.unwrap_or(20.0),
        );
    }
    if deep.membrane.is_some() || deep.membrane_thickness_um.is_some() {
        dx.membrane = BeamFilter::new(
            Compound::from_spec(deep.membrane.as_deref().unwrap_or("Ti"))?,
            deep.membrane_thickness_um.unwrap_or(2.0),
        );
    }
    for f in deep.filters.iter().flatten() {
        if f.thickness_um < 0.0 {
            anyhow::bail!("filter {} thickness must be >= 0", f.material);
        }
        dx.filters.push(BeamFilter::new(
            Compound::from_spec(&f.material)?,
            f.thickness_um,
        ));
    }
    if let Some(d) = deep.target_bottom_dose_kj_cm3 {
        dx.target_bottom_dose_kj_cm3 = d;
    }
    if let Some(d) = deep.damage_dose_kj_cm3 {
        dx.damage_dose_kj_cm3 = d;
    }
    let exposure = deep_xray::expose_depth(&dx);
    Ok((dx, exposure))
}

fn run_liga(config: &SimConfig, deep: &DeepConfig, output: Option<&Path>) -> anyhow::Result<()> {
    let (dx, exposure) = liga_setup(config, deep)?;

    let critical_energy_kev = match &dx.spectrum {
        XraySpectrum::BendingMagnet {
            critical_energy_kev,
        } => Some(*critical_energy_kev),
        XraySpectrum::BendingMagnetBeamline(b) => Some(b.critical_energy_kev()),
        XraySpectrum::Tabulated { .. } | XraySpectrum::FluxDensity { .. } => None,
    };
    let min_feature_nm = deep.min_feature_nm.unwrap_or(5000.0);
    let aspect = deep_xray::max_aspect_ratio(&exposure, min_feature_nm);
    let model = match dx.proximity_model {
        ProximityModel::Fresnel => "fresnel",
        ProximityModel::Gaussian => "gaussian",
    };

    eprintln!("LIGA deep X-ray (PMMA):");
    if let Some(e_c) = critical_energy_kev {
        eprintln!("  Critical energy:   {:.3} keV", e_c);
    }
    eprintln!("  Resist thickness:  {:.0} µm", dx.resist_thickness_um);
    eprintln!("  Proximity gap:     {:.0} µm", dx.proximity_gap_um);
    eprintln!("  Proximity model:   {}", model);
    eprintln!(
        "  Top dose:          {:.3} kJ/cm³",
        exposure.top_dose_kj_cm3
    );
    eprintln!(
        "  Bottom dose:       {:.3} kJ/cm³ (target {:.3})",
        exposure.bottom_dose_kj_cm3, dx.target_bottom_dose_kj_cm3
    );
    eprintln!("  Dose ratio (T/B):  {:.3}", exposure.dose_ratio);
    eprintln!(
        "  Mean photon energy (dose-weighted): top {:.2} keV, bottom {:.2} keV",
        exposure.mean_energy_top_kev, exposure.mean_energy_bottom_kev
    );
    eprintln!(
        "  Damage ceiling:    {:.1} kJ/cm³  [{}]",
        dx.damage_dose_kj_cm3,
        if exposure.exceeds_damage_ceiling {
            "EXCEEDED"
        } else {
            "ok"
        }
    );
    if let Some(t) = exposure.exposure_time_estimate {
        eprintln!(
            "  Exposure time:     {:.1} s ({:.2} h) to reach the bottom dose",
            t,
            t / 3600.0
        );
        if let Some(q) = exposure.exposure_charge_ma_h {
            eprintln!("  Exposure charge:   {:.2} mA·h", q);
        }
        if let Some(p) = exposure.resist_power_density_w_mm2 {
            eprintln!("  Power on resist:   {:.3e} W/mm² (open areas)", p);
        }
    } else {
        eprintln!(
            "  Exposure time:     n/a (relative spectrum; give absolute flux to get seconds)"
        );
    }
    eprintln!(
        "  Max aspect ratio:  {:.1}  (thickness / {:.1} µm feature)",
        aspect,
        min_feature_nm / 1e3
    );

    // Developed-depth map: rasterize the mask, expose volumetrically, and scan
    // each column for the contiguous above-threshold depth.
    let mask = config.to_mask()?;
    let grid = config.to_grid()?;
    let sampling = deep_xray::fresnel_sampling(&dx, &grid);
    eprintln!(
        "  Fresnel scale:     {:.0} nm (top) / {:.0} nm (bottom), pixel {:.0} nm [{}]",
        sampling.fresnel_scale_top_nm,
        sampling.fresnel_scale_bottom_nm,
        sampling.pixel_nm,
        if sampling.resolved {
            "resolved"
        } else {
            "under-resolved"
        }
    );
    if deep.strict_sampling.unwrap_or(false) {
        sampling
            .require_resolved()
            .map_err(|e| anyhow::anyhow!("{e}: {}", sampling.warnings.join("; ")))?;
    }
    let mut warnings: Vec<String> = sampling.warnings.clone();
    warnings.extend(exposure.warnings.iter().cloned());
    for w in &warnings {
        eprintln!("  warning: {}", w);
    }
    let nz = deep.nz.unwrap_or(64);
    let volume = deep_xray::expose_volumetric(&dx, &mask, &grid, nz)?;
    let threshold = deep
        .develop_threshold_kj_cm3
        .unwrap_or(dx.target_bottom_dose_kj_cm3);
    let depth = deep_xray::develop_depth(&volume, threshold);
    let (max_depth_um, min_depth_um) = {
        let mx = depth.iter().cloned().fold(f64::NEG_INFINITY, f64::max) / 1e3;
        let mn = depth.iter().cloned().fold(f64::INFINITY, f64::min) / 1e3;
        (mx, mn)
    };
    eprintln!(
        "  Developed depth:   {:.1}–{:.1} µm (threshold {:.2} kJ/cm³)",
        min_depth_um, max_depth_um, threshold
    );

    // Optional fine straight-edge profile for sidewall analysis.
    let edge = if deep.edge_profile.unwrap_or(false) {
        let half = deep.edge_half_width_nm.unwrap_or(3000.0);
        let dx_nm = deep.edge_dx_nm.unwrap_or(10.0);
        let depths: Vec<f64> = (0..5)
            .map(|k| k as f64 / 4.0 * dx.resist_thickness_um)
            .collect();
        let profile = deep_xray::fresnel_edge_profile_1d(&dx, -half, half, dx_nm, &depths)?;
        let edges = profile.edge_positions_nm(threshold);
        let angle = profile.sidewall_angle_deg(threshold);
        eprintln!(
            "  Edge profile ({:.0} nm sampling, threshold {:.2} kJ/cm³):",
            dx_nm, threshold
        );
        for (z, e) in depths.iter().zip(edges.iter()) {
            match e {
                Some(x) => eprintln!("    z = {:>6.1} µm: developed edge at {:+.1} nm", z, x),
                None => eprintln!("    z = {:>6.1} µm: not developed", z),
            }
        }
        if let Some(a) = angle {
            eprintln!("    sidewall angle: {:.4}° from vertical", a);
        }
        Some(serde_json::json!({
            "dx_nm": dx_nm,
            "half_width_nm": half,
            "depths_um": depths,
            "edge_positions_nm": edges,
            "sidewall_angle_deg": angle,
            "max_gradient_kj_cm3_nm": profile.max_gradient_kj_cm3_nm(),
        }))
    } else {
        None
    };

    let summary = serde_json::json!({
        "mode": "liga",
        "critical_energy_kev": critical_energy_kev,
        "proximity_model": model,
        "resist_thickness_um": dx.resist_thickness_um,
        "proximity_gap_um": dx.proximity_gap_um,
        "top_dose_kj_cm3": exposure.top_dose_kj_cm3,
        "bottom_dose_kj_cm3": exposure.bottom_dose_kj_cm3,
        "target_bottom_dose_kj_cm3": dx.target_bottom_dose_kj_cm3,
        "dose_ratio": exposure.dose_ratio,
        "mean_energy_top_kev": exposure.mean_energy_top_kev,
        "mean_energy_bottom_kev": exposure.mean_energy_bottom_kev,
        "window_power_fraction": exposure.window_power_fraction,
        "damage_dose_kj_cm3": dx.damage_dose_kj_cm3,
        "exceeds_damage_ceiling": exposure.exceeds_damage_ceiling,
        "exposure_time_s": exposure.exposure_time_estimate,
        "top_dose_rate_kj_cm3_s": exposure.top_dose_rate_kj_cm3_s,
        "bottom_dose_rate_kj_cm3_s": exposure.bottom_dose_rate_kj_cm3_s,
        "resist_power_density_w_mm2": exposure.resist_power_density_w_mm2,
        "exposure_charge_ma_h": exposure.exposure_charge_ma_h,
        "fresnel_scale_top_nm": sampling.fresnel_scale_top_nm,
        "fresnel_scale_bottom_nm": sampling.fresnel_scale_bottom_nm,
        "sampling_resolved": sampling.resolved,
        "warnings": warnings,
        "max_aspect_ratio": aspect,
        "min_feature_nm": min_feature_nm,
        "develop_threshold_kj_cm3": threshold,
        "developed_depth_min_um": min_depth_um,
        "developed_depth_max_um": max_depth_um,
        "edge_profile": edge,
    });
    write_output(output, &summary, &depth, Colormap::Viridis)
}

// ============================================================================
// Grayscale 2.5D
// ============================================================================

/// Descending staircase target height: `n_levels` equal-width bands along x,
/// stepping from the full `thickness` down to `thickness - depth`.
fn staircase_height(n: usize, n_levels: usize, depth_nm: f64, thickness_nm: f64) -> Array2<f64> {
    let levels = n_levels.max(1);
    let denom = (levels - 1).max(1) as f64;
    Array2::from_shape_fn((n, n), |(_i, j)| {
        let level = (j * levels / n).min(levels - 1);
        thickness_nm - depth_nm * level as f64 / denom
    })
}

fn run_grayscale(
    config: &SimConfig,
    deep: &DeepConfig,
    output: Option<&Path>,
) -> anyhow::Result<()> {
    let target = deep.target.as_deref().unwrap_or("blazed");
    let n = config.grid.size;
    let thickness_nm = deep.thickness_nm.unwrap_or(1000.0);
    let dose = deep.dose_mj_cm2.unwrap_or(100.0);
    let d_th = deep.d_th.unwrap_or(10.0);
    let d_clear = deep.d_clear.unwrap_or(100.0);
    let curve = ContrastCurve::new(d_th, d_clear)?;

    let target_height = match target {
        "blazed" => {
            let period_px = deep.period_px.unwrap_or(n / 4).max(1);
            let depth_nm = deep.depth_nm.unwrap_or(0.6 * thickness_nm);
            grayscale::blazed_grating(n, period_px, depth_nm, thickness_nm)
        }
        "microlens" => {
            let pitch_px = deep.pitch_px.unwrap_or(n / 4).max(1);
            let sag_nm = deep.sag_nm.unwrap_or(0.6 * thickness_nm);
            grayscale::microlens_array(n, pitch_px, sag_nm, thickness_nm)
        }
        "staircase" => {
            let n_levels = deep.n_levels.unwrap_or(8);
            let depth_nm = deep.depth_nm.unwrap_or(0.8 * thickness_nm);
            staircase_height(n, n_levels, depth_nm, thickness_nm)
        }
        other => anyhow::bail!(
            "unknown grayscale target '{}' (expected one of: blazed, microlens, staircase)",
            other
        ),
    };

    // Synthesize the transmittance that prints this relief, then re-print it in
    // the ideal 2.5D model (local dose = exposure_dose × transmittance) so the
    // achieved height and its RMS error vs the target are self-consistent.
    let map =
        grayscale::GrayscaleMap::from_target_height(&target_height, thickness_nm, &curve, dose)
            .context("synthesizing grayscale transmittance")?;
    let field = config.grid.pixel_nm * n as f64;
    let half = field / 2.0;
    let aerial = Grid2D {
        data: map.transmittance.clone(),
        x_min_nm: -half,
        x_max_nm: half,
        y_min_nm: -half,
        y_max_nm: half,
    };
    let achieved = grayscale::height_map(&aerial, dose, &curve, thickness_nm);

    let target_min = target_height.iter().cloned().fold(f64::INFINITY, f64::min);
    let target_max = target_height
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    let achieved_min = achieved.data.iter().cloned().fold(f64::INFINITY, f64::min);
    let achieved_max = achieved
        .data
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    let rms_error = {
        let mse: f64 = target_height
            .iter()
            .zip(achieved.data.iter())
            .map(|(t, a)| (t - a) * (t - a))
            .sum::<f64>()
            / target_height.len() as f64;
        mse.sqrt()
    };

    eprintln!("Grayscale 2.5D ({target}):");
    eprintln!(
        "  Contrast slope γ:  {:.3}  (D_th {:.1}, D_clear {:.1} mJ/cm²)",
        curve.gamma, d_th, d_clear
    );
    eprintln!("  Film thickness:    {:.0} nm", thickness_nm);
    eprintln!("  Exposure dose:     {:.1} mJ/cm²", dose);
    eprintln!(
        "  Target height:     {:.1}–{:.1} nm",
        target_min, target_max
    );
    eprintln!(
        "  Achieved height:   {:.1}–{:.1} nm",
        achieved_min, achieved_max
    );
    eprintln!("  RMS error:         {:.3} nm", rms_error);

    let summary = serde_json::json!({
        "mode": "grayscale",
        "target": target,
        "gamma": curve.gamma,
        "d_th": d_th,
        "d_clear": d_clear,
        "thickness_nm": thickness_nm,
        "dose_mj_cm2": dose,
        "target_height_min_nm": target_min,
        "target_height_max_nm": target_max,
        "achieved_height_min_nm": achieved_min,
        "achieved_height_max_nm": achieved_max,
        "rms_error_nm": rms_error,
    });
    write_output(output, &summary, &achieved.data, Colormap::Viridis)
}

// ============================================================================
// Multi-beam interference
// ============================================================================

fn run_interference(
    config: &SimConfig,
    deep: &DeepConfig,
    output: Option<&Path>,
) -> anyhow::Result<()> {
    let wavelength_nm = config.to_source()?.wavelength_nm();
    let preset = deep.preset.as_deref().unwrap_or("two_beam");
    let n_medium = deep.n_medium.unwrap_or(1.6);
    let half_angle_deg = deep.half_angle_deg.unwrap_or(30.0);

    let setup = match preset {
        "two_beam" => InterferenceSetup::two_beam(wavelength_nm, n_medium, half_angle_deg)?,
        "three_beam_hex" => {
            InterferenceSetup::three_beam_hex(wavelength_nm, n_medium, half_angle_deg)?
        }
        "four_beam_umbrella" => {
            InterferenceSetup::four_beam_umbrella(wavelength_nm, n_medium, half_angle_deg)?
        }
        other => anyhow::bail!(
            "unknown interference preset '{}' (expected one of: two_beam, three_beam_hex, \
             four_beam_umbrella)",
            other
        ),
    };

    // Lateral extent from the shared grid; recording depth from [deep].
    let n = config.grid.size;
    let field = config.grid.pixel_nm * n as f64;
    let half = field / 2.0;
    let nz = deep.nz.unwrap_or(64);
    let z_span_nm = deep.z_span_nm.unwrap_or(field);
    let mut volume = highuvlith_core::types::Grid3D::<f64>::new(
        n,
        n,
        nz,
        (-half, half),
        (-half, half),
        (0.0, z_span_nm),
    )?;
    setup.intensity(&mut volume);

    let two_photon = deep.two_photon.unwrap_or(false);
    let kinetics = if two_photon {
        ExposureKinetics::TwoPhoton
    } else {
        ExposureKinetics::OnePhoton
    };
    let dose_scale = deep.dose_scale.unwrap_or(1.0);
    let dill_c = deep.dill_c.unwrap_or(0.02);
    let pac = interference::expose(&volume, dose_scale, dill_c, kinetics);

    let fill_threshold = deep.fill_threshold.unwrap_or(0.5);
    let fill = interference::iso_surface_fill_fraction(&pac, fill_threshold);

    // Fringe period Λ = λ / (2 sin θ_air), index-invariant. This is the exact
    // grating period for `two_beam`; for the hex/umbrella presets it is the
    // pairwise-beam reference scale of the resulting lattice.
    let sin_theta = half_angle_deg.to_radians().sin();
    let fringe_period_nm = wavelength_nm / (2.0 * sin_theta);
    let period_label = if preset == "two_beam" {
        "Fringe period:    "
    } else {
        "Fringe period(2b):"
    };

    let pac_min = pac.data.iter().cloned().fold(f64::INFINITY, f64::min);
    let pac_max = pac.data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    eprintln!("Interference ({preset}):");
    eprintln!("  Wavelength:        {:.2} nm", wavelength_nm);
    eprintln!(
        "  Air half-angle:    {:.1}° (n_medium {:.2})",
        half_angle_deg, n_medium
    );
    eprintln!(
        "  {}  {:.1} nm  = λ / (2 sin θ)",
        period_label, fringe_period_nm
    );
    eprintln!(
        "  Kinetics:          {}",
        if two_photon {
            "two-photon (I²)"
        } else {
            "single-photon (I)"
        }
    );
    eprintln!("  PAC range:         {:.3}–{:.3}", pac_min, pac_max);
    eprintln!(
        "  Fill fraction:     {:.3}  (m < {:.2})",
        fill, fill_threshold
    );

    // Middle-z PAC slice for the PNG.
    let mid = nz / 2;
    let mid_slice = pac.data.index_axis(ndarray::Axis(0), mid).to_owned();

    let summary = serde_json::json!({
        "mode": "interference",
        "preset": preset,
        "wavelength_nm": wavelength_nm,
        "half_angle_deg": half_angle_deg,
        "n_medium": n_medium,
        "fringe_period_nm": fringe_period_nm,
        "two_photon": two_photon,
        "dose_scale": dose_scale,
        "dill_c": dill_c,
        "fill_threshold": fill_threshold,
        "fill_fraction": fill,
        "pac_min": pac_min,
        "pac_max": pac_max,
    });
    write_output(output, &summary, &mid_slice, Colormap::Inferno)
}

// ============================================================================
// Volumetric exposure + development
// ============================================================================

fn run_volumetric(
    config: &SimConfig,
    deep: &DeepConfig,
    output: Option<&Path>,
) -> anyhow::Result<()> {
    let (summary, image) = volumetric_run(config, deep)?;
    write_output(output, &summary, &image, Colormap::Inferno)
}

/// Parse the lateral boundary tag shared by the bake and development.
fn parse_lateral_boundary(
    tag: Option<&str>,
) -> anyhow::Result<highuvlith_core::resist::DiffusionBoundary> {
    use highuvlith_core::resist::DiffusionBoundary;
    match tag.unwrap_or("periodic") {
        "periodic" => Ok(DiffusionBoundary::Periodic),
        "reflecting" => Ok(DiffusionBoundary::Reflecting),
        other => anyhow::bail!(
            "unknown [deep] lateral_boundary '{}' (expected periodic or reflecting)",
            other
        ),
    }
}

/// Build the post-exposure-bake model from `[deep]`.
/// Default volumetric bake. Gaussian, like Python's `simulate_volumetric`
/// (σ = the resist's `peb_diffusion_nm` = 30 nm on both axes): without a bake
/// the standing-wave nodes of the default stack stall a development front
/// about 23 nm below the top.
const DEFAULT_PEB: &str = "gaussian";
/// Default volumetric resist thickness (nm): the core film-stack default.
const DEFAULT_RESIST_THICKNESS_NM: f64 = 150.0;

fn volumetric_peb(
    deep: &DeepConfig,
    resist: &highuvlith_core::resist::ResistParams,
    nz: usize,
    dz_nm: f64,
    lateral: highuvlith_core::resist::DiffusionBoundary,
) -> anyhow::Result<highuvlith_core::volumetric::PebModel> {
    use highuvlith_core::resist::{CarParams, PebDiffusion};
    use highuvlith_core::volumetric::PebModel;
    Ok(match deep.peb.as_deref().unwrap_or(DEFAULT_PEB) {
        "none" => PebModel::None,
        "gaussian" => {
            let lateral_nm = deep.peb_lateral_nm.unwrap_or(resist.peb_diffusion_nm);
            let mut peb =
                PebDiffusion::anisotropic(lateral_nm, deep.peb_vertical_nm.unwrap_or(lateral_nm));
            peb.lateral_boundary = lateral;
            match (deep.peb_vertical_surface_ratio, deep.peb_vertical_decay_nm) {
                (None, None) => {}
                (Some(ratio), Some(decay)) if decay > 0.0 => {
                    peb = peb.with_exponential_depth_profile(nz, dz_nm, ratio, decay);
                }
                _ => anyhow::bail!(
                    "[deep] a depth-dependent PEB needs both peb_vertical_surface_ratio and a \
                     positive peb_vertical_decay_nm"
                ),
            }
            PebModel::Gaussian(peb)
        }
        "car" => {
            let d = CarParams::default();
            PebModel::ChemicallyAmplified(CarParams {
                peb_time_s: deep.car_peb_time_s.unwrap_or(d.peb_time_s),
                k_amp_per_s: deep.car_k_amp.unwrap_or(d.k_amp_per_s),
                k_quench_per_s: deep.car_k_quench.unwrap_or(d.k_quench_per_s),
                quencher_initial: deep.car_quencher.unwrap_or(d.quencher_initial),
                acid_diffusivity_nm2_s: deep
                    .car_acid_diffusivity_nm2_s
                    .unwrap_or(d.acid_diffusivity_nm2_s),
                quencher_diffusivity_nm2_s: deep
                    .car_quencher_diffusivity_nm2_s
                    .unwrap_or(d.quencher_diffusivity_nm2_s),
                vertical_diffusivity_ratio: deep
                    .car_vertical_ratio
                    .unwrap_or(d.vertical_diffusivity_ratio),
                lateral_boundary: lateral,
                max_time_step_s: None,
            })
        }
        other => anyhow::bail!(
            "unknown [deep] peb '{}' (expected one of: none, gaussian, car)",
            other
        ),
    })
}

/// Build the developer depletion model from `[deep]`.
fn volumetric_depletion(
    deep: &DeepConfig,
) -> anyhow::Result<highuvlith_core::volumetric::DeveloperDepletion> {
    use highuvlith_core::volumetric::DeveloperDepletion;
    let need = |value: Option<f64>, key: &str| {
        value.ok_or_else(|| anyhow::anyhow!("[deep] depletion needs {}", key))
    };
    Ok(match deep.depletion.as_deref().unwrap_or("none") {
        "none" => DeveloperDepletion::None,
        "exponential" => DeveloperDepletion::Exponential {
            time_constant_s: need(deep.depletion_time_constant_s, "depletion_time_constant_s")?,
        },
        "loading" => DeveloperDepletion::Loading {
            capacity_nm: need(deep.loading_capacity_nm, "loading_capacity_nm")?,
        },
        "local_loading" => DeveloperDepletion::LocalLoading {
            capacity_nm: need(deep.loading_capacity_nm, "loading_capacity_nm")?,
            length_nm: need(deep.loading_length_nm, "loading_length_nm")?,
        },
        other => anyhow::bail!(
            "unknown [deep] depletion '{}' (expected one of: none, exponential, loading, \
             local_loading)",
            other
        ),
    })
}

/// Over- / under-development diagnostics for a developed height map.
///
/// The right volumetric dose depends on the pattern, optics, stack and
/// development time, so no fixed default develops every config; instead the
/// run says when the result looks degenerate. Over: less than a quarter of
/// the resist volume remains (a legitimate result only for mostly clear
/// patterns). Under: nothing clears to the substrate (the thinnest point is
/// thicker than one depth slice).
fn development_notes(
    height: &ndarray::Array2<f64>,
    thickness_nm: f64,
    dz_nm: f64,
    dose: (f64, &str),
    dev_time_s: f64,
) -> Vec<String> {
    let (dose_mj_cm2, dose_from) = dose;
    let n = height.len().max(1) as f64;
    let mean_h = height.sum() / n;
    let min_h = height.iter().copied().fold(f64::INFINITY, f64::min);
    let mut notes = Vec::new();
    if mean_h < 0.25 * thickness_nm {
        notes.push(format!(
            "over-developed? only {:.0} % of the resist volume remains (mean {mean_h:.1} nm of \
             {thickness_nm:.0} nm) at {dose_mj_cm2:.1} mJ/cm² ({dose_from}) after {dev_time_s:.0} \
             s; unless the pattern is mostly clear, lower [deep] dose_mj_cm2 or dev_time_s",
            100.0 * mean_h / thickness_nm
        ));
    } else if min_h > dz_nm {
        notes.push(format!(
            "under-developed: nothing clears to the substrate (thinnest remaining {min_h:.1} nm) \
             at {dose_mj_cm2:.1} mJ/cm² ({dose_from}) after {dev_time_s:.0} s; raise [deep] \
             dose_mj_cm2 or dev_time_s"
        ));
    }
    notes
}

/// The volumetric pipeline: exposure → post-exposure bake → development.
/// Returns the JSON summary and the image to write (the x–z developed
/// profile through the centre row when development ran, otherwise the
/// mid-depth latent slice).
fn volumetric_run(
    config: &SimConfig,
    deep: &DeepConfig,
) -> anyhow::Result<(serde_json::Value, Array2<f64>)> {
    use highuvlith_core::aerial::AerialImageEngine;
    use highuvlith_core::materials::database::MaterialsDatabase;
    use highuvlith_core::resist::ResistParams;
    use highuvlith_core::volumetric::{
        DeveloperDepletion, DevelopmentOptions, LevelSetConfig, PebModel, SurfaceInhibition,
        VolumetricExposureConfig,
    };

    let source = config.to_source()?;
    let optics = config.to_optics()?;
    let mask = config.to_mask()?;
    let grid = config.to_grid()?;
    let wavelength_nm = source.wavelength_nm();

    let engine = AerialImageEngine::with_settings(
        &source,
        optics.as_ref(),
        grid.clone(),
        config.to_imaging_settings()?,
    )?;

    // Default resist-on-Si film stack evaluated at the source wavelength from
    // the materials database (VUV tables or Henke EUV entries); other
    // wavelengths have no default stack and are rejected rather than reusing
    // the 157 nm constants.
    let resist_thickness_nm = deep
        .resist_thickness_nm
        .unwrap_or(DEFAULT_RESIST_THICKNESS_NM);
    let stack = MaterialsDatabase::new()
        .resist_on_silicon_stack(resist_thickness_nm, wavelength_nm)
        .context("volumetric mode: building the default resist-on-Si film stack")?;
    let resist = ResistParams {
        thickness_nm: resist_thickness_nm,
        ..ResistParams::default()
    };

    let vcfg = VolumetricExposureConfig {
        dose_mj_cm2: deep.dose_mj_cm2.unwrap_or(config.process.dose_mj_cm2),
        nz: deep.nz.unwrap_or(64),
        n_defocus_planes: deep.n_defocus_planes.unwrap_or(8),
        dose_steps: deep.dose_steps.unwrap_or(1),
        base_defocus_nm: config.process.focus_nm,
        resist_layer: 0,
    };

    let mut latent =
        volumetric::expose_volumetric(&engine, &mask, &stack, &resist, wavelength_nm, &vcfg)?;

    // ---- Post-exposure bake ----
    let lateral = parse_lateral_boundary(deep.lateral_boundary.as_deref())?;
    let dz_nm = latent.pac.pixel_size_z();
    let dx_nm = latent.pac.pixel_size_x();
    let peb = volumetric_peb(deep, &resist, vcfg.nz, dz_nm, lateral)?;
    let car_state = volumetric::apply_peb(&mut latent, &peb)?;
    let peb_label = match &peb {
        PebModel::None => "none",
        PebModel::Gaussian(_) => "gaussian",
        PebModel::ChemicallyAmplified(_) => "car",
    };

    let threshold = deep.develop_threshold.unwrap_or(0.5);
    let cds = metrics::cd_at_z(&latent.pac, threshold);
    let nz = cds.len();
    let fmt_cd = |cd: Option<f64>| match cd {
        Some(v) => format!("{v:.1} nm"),
        None => "n/a".to_string(),
    };
    let top = cds.first().copied().flatten();
    let mid = cds.get(nz / 2).copied().flatten();
    let bottom = cds.last().copied().flatten();

    eprintln!("Volumetric exposure ({}):", source.kind_label());
    eprintln!("  Wavelength:        {:.2} nm", wavelength_nm);
    eprintln!("  Resist thickness:  {:.0} nm", resist_thickness_nm);
    eprintln!("  Dose:              {:.1} mJ/cm²", vcfg.dose_mj_cm2);
    eprintln!(
        "  Split-step:        {} dose step(s), {} defocus plane(s)",
        vcfg.dose_steps, vcfg.n_defocus_planes
    );
    eprintln!("  Post-exposure bake: {peb_label}");
    eprintln!(
        "  Latent CD (top/mid/bot): {} / {} / {}  (centre feature at the PAC m = {:.2} \
         contour; not a developed CD)",
        fmt_cd(top),
        fmt_cd(mid),
        fmt_cd(bottom),
        threshold
    );
    let sidewall = match (top, bottom) {
        (Some(cd_top), Some(cd_bottom)) => {
            let angle = metrics::sidewall_angle_deg(cd_top, cd_bottom, resist_thickness_nm);
            eprintln!("  Latent sidewall:   {:.1}°", angle);
            Some(angle)
        }
        _ => {
            eprintln!("  Latent sidewall:   n/a (latent CD not measurable at top and bottom)");
            None
        }
    };

    // ---- Development ----
    let develop = deep.develop.as_deref().unwrap_or("threshold");
    let dev_time_s = deep.dev_time_s.unwrap_or(60.0);
    let depletion = volumetric_depletion(deep)?;
    let surface_inhibition = match (deep.surface_rate_ratio, deep.inhibition_depth_nm) {
        (None, None) => None,
        (ratio, depth) => Some(SurfaceInhibition::new(
            ratio.unwrap_or(1.0),
            depth.unwrap_or(0.0),
        )?),
    };
    let options = DevelopmentOptions {
        surface_inhibition,
        lateral_boundary: lateral,
    };
    if develop != "level_set" && depletion != DeveloperDepletion::None {
        anyhow::bail!(
            "[deep] depletion needs develop = \"level_set\" (fast marching and the threshold \
             tier assume a static rate)"
        );
    }
    let (times, level_set) = match develop {
        "threshold" => (None, None),
        "fmm" => (
            Some(volumetric::develop_fast_marching_with(
                &latent, &resist, dx_nm, dz_nm, &options,
            )?),
            None,
        ),
        "level_set" => {
            let ls_config = LevelSetConfig {
                depletion,
                ..LevelSetConfig::new(dev_time_s)
            };
            let result = volumetric::develop_level_set(&latent, &resist, &options, &ls_config)?;
            (Some(result.arrival_times.clone()), Some(result))
        }
        other => anyhow::bail!(
            "unknown [deep] develop '{}' (expected one of: threshold, fmm, level_set)",
            other
        ),
    };

    let mut developed_json = serde_json::Value::Null;
    let image = if let Some(times) = &times {
        let developed = volumetric::developed_indicator(times, dev_time_s);
        let dev_cds = metrics::cd_at_z(&developed, 0.5);
        let d_top = dev_cds.first().copied().flatten();
        let d_mid = dev_cds.get(nz / 2).copied().flatten();
        let d_bottom = dev_cds.last().copied().flatten();
        let d_sidewall = match (d_top, d_bottom) {
            (Some(a), Some(b)) => Some(metrics::sidewall_angle_deg(a, b, resist_thickness_nm)),
            _ => None,
        };
        let height = volumetric::height_map_from_times(times, dev_time_s);
        let mean_height = height.data.mean().unwrap_or(resist_thickness_nm);
        eprintln!("  Development:       {develop}, {dev_time_s:.1} s");
        eprintln!(
            "  Developed CD (top/mid/bot): {} / {} / {}",
            fmt_cd(d_top),
            fmt_cd(d_mid),
            fmt_cd(d_bottom)
        );
        match d_sidewall {
            Some(angle) => eprintln!("  Developed sidewall: {:.1}°", angle),
            None => eprintln!("  Developed sidewall: n/a"),
        }
        eprintln!("  Mean remaining height: {:.1} nm", mean_height);
        let dose_from = if deep.dose_mj_cm2.is_some() {
            "[deep] dose_mj_cm2"
        } else {
            "[process] dose_mj_cm2 fallback"
        };
        let dev_notes = development_notes(
            &height.data,
            resist_thickness_nm,
            dz_nm,
            (vcfg.dose_mj_cm2, dose_from),
            dev_time_s,
        );
        for n in &dev_notes {
            eprintln!("note: {n}");
        }
        developed_json = serde_json::json!({
            "cd_top_nm": d_top,
            "cd_mid_nm": d_mid,
            "cd_bottom_nm": d_bottom,
            "sidewall_angle_deg": d_sidewall,
            "mean_remaining_height_nm": mean_height,
            "max_remaining_height_nm": height.data.iter().copied().fold(0.0, f64::max),
            "min_remaining_height_nm": height.data.iter().copied().fold(f64::INFINITY, f64::min),
            "level_set_steps": level_set.as_ref().map(|r| r.steps),
            "dissolved_thickness_nm": level_set.as_ref().map(|r| r.dissolved_thickness_nm),
            "final_rate_factor": level_set.as_ref().map(|r| r.final_rate_factor),
            "notes": dev_notes,
        });
        // x–z developed profile through the centre row.
        let row = developed.data.dim().1 / 2;
        developed.data.index_axis(ndarray::Axis(1), row).to_owned()
    } else {
        // Middle-z latent slice.
        let mid_k = latent.pac.data.dim().0 / 2;
        latent
            .pac
            .data
            .index_axis(ndarray::Axis(0), mid_k)
            .to_owned()
    };

    let summary = serde_json::json!({
        "mode": "volumetric",
        "source_type": source.kind_label(),
        "wavelength_nm": wavelength_nm,
        "resist_thickness_nm": resist_thickness_nm,
        "dose_mj_cm2": vcfg.dose_mj_cm2,
        "nz": vcfg.nz,
        "n_defocus_planes": vcfg.n_defocus_planes,
        "dose_steps": vcfg.dose_steps,
        "peb": peb_label,
        "car_neutralized_total": car_state.as_ref().map(|c| c.neutralized_total),
        "develop_threshold": threshold,
        "cd_top_nm": top,
        "cd_mid_nm": mid,
        "cd_bottom_nm": bottom,
        "sidewall_angle_deg": sidewall,
        "develop": develop,
        "dev_time_s": dev_time_s,
        "developed": developed_json,
    });
    Ok((summary, image))
}

// ============================================================================
// Talbot (coherent / DTL / ATL) and two-grating EUV interference lithography
// ============================================================================

/// Build the Talbot setup from `[deep]`: grating period (default: the `[mask]`
/// pitch), profile, truncation order, and transfer function; the wavelength
/// comes from `[source]`.
fn talbot_setup(
    config: &SimConfig,
    deep: &DeepConfig,
) -> anyhow::Result<highuvlith_core::talbot::TalbotSetup> {
    use highuvlith_core::talbot::{Grating, Propagation, TalbotSetup};

    let wavelength_nm = config.to_source()?.wavelength_nm();
    let mask_pitch = config
        .mask
        .as_ref()
        .filter(|m| m.pattern() == "line_space")
        .map(|m| m.pitch_nm.unwrap_or(180.0));
    let period = deep.grating_period_nm.or(mask_pitch).ok_or_else(|| {
        anyhow::anyhow!(
            "talbot needs a grating period: set [deep] grating_period_nm (or a line/space [mask] \
             pitch_nm)"
        )
    })?;
    let duty = deep.duty_cycle.unwrap_or(0.5);
    let max_order = deep.max_order.unwrap_or(10);
    let grating = match deep.grating_type.as_deref().unwrap_or("amplitude") {
        "amplitude" => Grating::binary_amplitude(period, duty, max_order)?,
        "phase" => Grating::binary_phase(
            period,
            duty,
            deep.phase_rad.unwrap_or(std::f64::consts::PI),
            max_order,
        )?,
        other => anyhow::bail!(
            "unknown grating_type '{}' (expected one of: amplitude, phase)",
            other
        ),
    };
    let mut setup = TalbotSetup::new(grating, wavelength_nm)?;
    setup.propagation = match deep.propagation.as_deref().unwrap_or("exact") {
        "exact" => Propagation::AngularSpectrum,
        "paraxial" => Propagation::Paraxial,
        other => anyhow::bail!(
            "unknown propagation '{}' (expected one of: exact, paraxial)",
            other
        ),
    };
    Ok(setup)
}

/// Michelson contrast `(max − min)/(max + min)` of a profile.
fn michelson_contrast(profile: &[f64]) -> f64 {
    let max = profile.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min = profile.iter().cloned().fold(f64::INFINITY, f64::min);
    if max + min > 0.0 {
        (max - min) / (max + min)
    } else {
        0.0
    }
}

fn run_talbot(config: &SimConfig, deep: &DeepConfig, output: Option<&Path>) -> anyhow::Result<()> {
    use highuvlith_core::talbot::{
        achromatic_distance_nm, dominant_period_nm, ExposureMode, ResistMedium, Spectrum,
        TwoGratingInterference,
    };
    use highuvlith_core::types::Grid3D;

    let source = config.to_source()?;
    let setup = talbot_setup(config, deep)?;
    let wavelength_nm = setup.wavelength_nm;
    let period = setup.grating.period_x_nm;
    let z_t = setup.talbot_length_nm();
    let z_t_exact = setup.talbot_length_exact_nm();
    // Bandwidth: explicit, else derived from the source model (pm → nm).
    let bandwidth_nm = deep
        .bandwidth_nm
        .unwrap_or(source.bandwidth_pm() * 1e-3)
        .max(0.0);
    let spectrum = match deep.spectrum_shape.as_deref().unwrap_or("gaussian") {
        "gaussian" => Spectrum::Gaussian {
            fwhm_nm: bandwidth_nm,
            bins: 256,
        },
        "flat" => Spectrum::FlatTop {
            bandwidth_nm,
            bins: 256,
        },
        other => anyhow::bail!(
            "unknown spectrum_shape '{}' (expected one of: gaussian, flat)",
            other
        ),
    };
    let z_a = (bandwidth_nm > 0.0).then(|| achromatic_distance_nm(period, bandwidth_nm));
    let talbot_mode = deep.talbot_mode.as_deref().unwrap_or("carpet");

    let n = config.grid.size;
    let field = config.grid.pixel_nm * n as f64;
    let half = field / 2.0;
    let nz = deep.nz.unwrap_or(16);
    let z_span_nm = deep.z_span_nm.unwrap_or(50.0);
    let medium = ResistMedium {
        index: deep.n_medium.unwrap_or(1.0),
        absorption_per_nm: deep.absorption_per_nm.unwrap_or(0.0),
    };
    let dose_scale = deep.dose_scale.unwrap_or(1.0);
    let dill_c = deep.dill_c.unwrap_or(0.02);
    let fill_threshold = deep.fill_threshold.unwrap_or(0.5);

    eprintln!("Talbot ({talbot_mode}):");
    eprintln!("  Wavelength:        {:.3} nm", wavelength_nm);
    eprintln!(
        "  Grating:           {:.1} nm period, {} (duty {:.2}), orders |n| ≤ {}",
        period,
        deep.grating_type.as_deref().unwrap_or("amplitude"),
        deep.duty_cycle.unwrap_or(0.5),
        deep.max_order.unwrap_or(10)
    );
    eprintln!("  Talbot length:     {:.1} nm = 2p²/λ (paraxial)", z_t);
    if let Some(z) = z_t_exact {
        eprintln!("  Talbot (exact):    {:.1} nm = λ/(1 − √(1 − λ²/p²))", z);
    }
    if let Some(z) = z_a {
        eprintln!(
            "  Achromatic dist.:  {:.2} µm = 2p²/Δλ (Δλ = {:.4} nm)",
            z / 1e3,
            bandwidth_nm
        );
    }

    let mut summary = serde_json::json!({
        "mode": "talbot",
        "talbot_mode": talbot_mode,
        "wavelength_nm": wavelength_nm,
        "grating_period_nm": period,
        "grating_type": deep.grating_type.as_deref().unwrap_or("amplitude"),
        "duty_cycle": deep.duty_cycle.unwrap_or(0.5),
        "max_order": deep.max_order.unwrap_or(10),
        "propagation": deep.propagation.as_deref().unwrap_or("exact"),
        "talbot_length_nm": z_t,
        "talbot_length_exact_nm": z_t_exact,
        "bandwidth_nm": bandwidth_nm,
        "achromatic_distance_nm": z_a,
    });

    let image = match talbot_mode {
        "carpet" => {
            let z_max = deep.carpet_z_max_um.map_or(2.0 * z_t, |u| u * 1e3);
            let rows = deep.carpet_nz.unwrap_or(256);
            let carpet = setup.carpet((-half, half), n, (0.0, z_max), rows, None)?;
            eprintln!(
                "  Carpet:            {} × {} (x × z), z ∈ [0, {:.1}] nm",
                n, rows, z_max
            );
            summary["carpet_z_max_nm"] = z_max.into();
            summary["carpet_rows"] = rows.into();
            carpet.data
        }
        "dtl" | "atl" => {
            let (mode, default_gap) = if talbot_mode == "dtl" {
                let scan = deep.scan_talbot_lengths.unwrap_or(1.0) * z_t;
                summary["scan_length_nm"] = scan.into();
                (
                    ExposureMode::Displacement {
                        scan_length_nm: scan,
                    },
                    10.0 * z_t,
                )
            } else {
                let z = z_a.ok_or_else(|| {
                    anyhow::anyhow!(
                        "talbot_mode = \"atl\" needs a bandwidth: set [deep] bandwidth_nm or a \
                         [source] with bandwidth_pm > 0"
                    )
                })?;
                (ExposureMode::Achromatic(spectrum), 2.0 * z)
            };
            let gap = deep.gap_um.map_or(default_gap, |u| u * 1e3);
            let mut img =
                highuvlith_core::types::Grid2D::<f64>::new(n, n, (-half, half), (-half, half))?;
            setup.exposure_image(&mut img, gap, &mode)?;
            let centre = img.data.row(n / 2).to_vec();
            let printed = dominant_period_nm(&centre, config.grid.pixel_nm);
            let contrast = michelson_contrast(&centre);
            // Record in a resist layer at the gap → PAC → fill fraction.
            let mut vol =
                Grid3D::<f64>::new(n, 1, nz, (-half, half), (-0.5, 0.5), (0.0, z_span_nm))?;
            setup.resist_volume(&mut vol, gap, &medium, &mode)?;
            let pac = interference::expose(&vol, dose_scale, dill_c, ExposureKinetics::OnePhoton);
            let fill = interference::iso_surface_fill_fraction(&pac, fill_threshold);
            eprintln!("  Gap:               {:.2} µm", gap / 1e3);
            match printed {
                Some(p) => eprintln!("  Printed period:    {:.2} nm (mask {:.1} nm)", p, period),
                None => eprintln!("  Printed period:    n/a (flat image)"),
            }
            eprintln!("  Image contrast:    {:.3}", contrast);
            eprintln!(
                "  Fill fraction:     {:.3}  (m < {:.2}, {:.0} nm resist)",
                fill, fill_threshold, z_span_nm
            );
            summary["gap_nm"] = gap.into();
            summary["printed_period_nm"] = printed.into();
            summary["image_contrast"] = contrast.into();
            summary["fill_fraction"] = fill.into();
            img.data
        }
        "euv_il" => {
            let order = deep.il_order.unwrap_or(1);
            if order as usize > deep.max_order.unwrap_or(10) {
                anyhow::bail!(
                    "[deep] il_order = {} exceeds max_order = {}; raise max_order",
                    order,
                    deep.max_order.unwrap_or(10)
                );
            }
            let il = TwoGratingInterference::new(&setup.grating, wavelength_nm, order)?;
            let beams = il.to_interference_setup(medium.index, medium.absorption_per_nm)?;
            let mut vol =
                Grid3D::<f64>::new(n, n, nz, (-half, half), (-half, half), (0.0, z_span_nm))?;
            beams.intensity(&mut vol);
            let pac = interference::expose(&vol, dose_scale, dill_c, ExposureKinetics::OnePhoton);
            let fill = interference::iso_surface_fill_fraction(&pac, fill_threshold);
            let mid = pac.data.index_axis(ndarray::Axis(0), nz / 2).to_owned();
            let top: Vec<f64> = (0..n).map(|j| vol.data[[0, n / 2, j]]).collect();
            let printed = dominant_period_nm(&top, config.grid.pixel_nm);
            eprintln!(
                "  Diffraction:       order ±{}, θ = {:.3}° (sin θ = mλ/p)",
                order,
                il.diffraction_angle_deg()
            );
            eprintln!(
                "  Fringe period:     {:.2} nm = p/(2m) (λ-independent)",
                il.fringe_period_nm()
            );
            eprintln!(
                "  Beam intensities:  {:.4} / {:.4}  → visibility {:.3}",
                il.beam_intensity_1,
                il.beam_intensity_2,
                il.visibility()
            );
            if let Some(p) = printed {
                eprintln!("  Printed period:    {:.2} nm (FFT of the top slice)", p);
            }
            eprintln!(
                "  Fill fraction:     {:.3}  (m < {:.2})",
                fill, fill_threshold
            );
            summary["il_order"] = order.into();
            summary["diffraction_angle_deg"] = il.diffraction_angle_deg().into();
            summary["fringe_period_nm"] = il.fringe_period_nm().into();
            summary["beam_intensity_1"] = il.beam_intensity_1.into();
            summary["beam_intensity_2"] = il.beam_intensity_2.into();
            summary["visibility"] = il.visibility().into();
            summary["printed_period_nm"] = printed.into();
            summary["fill_fraction"] = fill.into();
            mid
        }
        other => anyhow::bail!(
            "unknown talbot_mode '{}' (expected one of: carpet, dtl, atl, euv_il)",
            other
        ),
    };
    write_output(output, &summary, &image, Colormap::Inferno)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(toml_str: &str) -> (SimConfig, DeepConfig) {
        let config = SimConfig::from_toml_str(toml_str).unwrap();
        let deep = DeepConfig::from_toml_str(toml_str).unwrap();
        (config, deep)
    }

    /// Parse and validate (strict keys); the error text on failure.
    fn validated(toml_str: &str) -> Result<(SimConfig, DeepConfig), String> {
        let config = SimConfig::from_toml_str(toml_str).map_err(|e| e.to_string())?;
        let deep = DeepConfig::from_toml_str(toml_str).map_err(|e| e.to_string())?;
        validate(&config, &deep).map_err(|e| e.to_string())?;
        Ok((config, deep))
    }

    const BASE_SECTIONS: &str = r#"
        [source]
        wavelength_nm = 157.63

        [optics]
        na = 0.75

        [mask]
        cd_nm = 400.0
        pitch_nm = 800.0

        [grid]
        size = 64
        pixel_nm = 20.0
    "#;

    #[test]
    fn test_mode_parse_valid() {
        assert_eq!(DeepMode::parse("liga").unwrap(), DeepMode::Liga);
        assert_eq!(DeepMode::parse("grayscale").unwrap(), DeepMode::Grayscale);
        assert_eq!(
            DeepMode::parse("interference").unwrap(),
            DeepMode::Interference
        );
        assert_eq!(DeepMode::parse("volumetric").unwrap(), DeepMode::Volumetric);
    }

    #[test]
    fn test_mode_parse_unknown_rejected() {
        let err = DeepMode::parse("holography").unwrap_err().to_string();
        assert!(err.contains("unknown deep mode"), "{err}");
        assert!(err.contains("liga"), "{err}");
    }

    #[test]
    fn test_missing_mode_rejected() {
        let (_c, deep) = parse(BASE_SECTIONS);
        assert!(deep.resolve_mode().is_err());
    }

    #[test]
    fn test_parse_liga_config() {
        let toml_str = format!(
            "{BASE_SECTIONS}\n[deep]\nmode = \"liga\"\ncritical_energy_kev = 6.23\n\
             resist_thickness_um = 500.0\nproximity_gap_um = 100.0\nnz = 64\n"
        );
        let (_c, deep) = parse(&toml_str);
        assert_eq!(deep.resolve_mode().unwrap(), DeepMode::Liga);
        assert_eq!(deep.critical_energy_kev, Some(6.23));
        assert_eq!(deep.resist_thickness_um, Some(500.0));
        assert_eq!(deep.nz, Some(64));
    }

    #[test]
    fn test_parse_grayscale_config() {
        let toml_str = format!(
            "{BASE_SECTIONS}\n[deep]\nmode = \"grayscale\"\ntarget = \"blazed\"\n\
             period_px = 16\ndepth_nm = 600.0\nthickness_nm = 1000.0\ndose_mj_cm2 = 120.0\n\
             d_th = 10.0\nd_clear = 100.0\n"
        );
        let (_c, deep) = parse(&toml_str);
        assert_eq!(deep.resolve_mode().unwrap(), DeepMode::Grayscale);
        assert_eq!(deep.target.as_deref(), Some("blazed"));
        assert_eq!(deep.period_px, Some(16));
    }

    #[test]
    fn test_parse_interference_config() {
        let toml_str = format!(
            "{BASE_SECTIONS}\n[deep]\nmode = \"interference\"\npreset = \"two_beam\"\n\
             half_angle_deg = 30.0\nn_medium = 1.6\ntwo_photon = false\nnz = 16\n\
             z_span_nm = 500.0\n"
        );
        let (_c, deep) = parse(&toml_str);
        assert_eq!(deep.resolve_mode().unwrap(), DeepMode::Interference);
        assert_eq!(deep.preset.as_deref(), Some("two_beam"));
        assert_eq!(deep.half_angle_deg, Some(30.0));
    }

    #[test]
    fn test_parse_volumetric_config() {
        let toml_str = format!(
            "{BASE_SECTIONS}\n[deep]\nmode = \"volumetric\"\ndose_mj_cm2 = 30.0\nnz = 16\n\
             n_defocus_planes = 4\ndose_steps = 5\nresist_thickness_nm = 300.0\n"
        );
        let (_c, deep) = parse(&toml_str);
        assert_eq!(deep.resolve_mode().unwrap(), DeepMode::Volumetric);
        assert_eq!(deep.dose_steps, Some(5));
        assert_eq!(deep.resist_thickness_nm, Some(300.0));
    }

    /// Volumetric defaults match Python's `simulate_volumetric` and the core:
    /// Gaussian bake at the resist's 30 nm diffusion length, 150 nm resist.
    #[test]
    fn test_volumetric_defaults_bake_and_core_thickness() {
        let text = "[source]\nwavelength_nm = 157.63\n[optics]\nna = 0.75\n[mask]\n\
                    cd_nm = 150.0\npitch_nm = 300.0\n[grid]\nsize = 32\npixel_nm = 18.75\n\
                    [deep]\nmode = \"volumetric\"\nnz = 8\n";
        let (config, deep) = validated(text).unwrap();
        let (summary, _) = volumetric_run(&config, &deep).unwrap();
        assert_eq!(summary["peb"], "gaussian");
        assert_eq!(summary["resist_thickness_nm"], 150.0);
        // Gaussian bake keys need no explicit peb; peb = "none" rejects them.
        assert!(validated(&format!("{text}peb_lateral_nm = 10.0\n")).is_ok());
        let err = validated(&format!("{text}peb = \"none\"\npeb_lateral_nm = 10.0\n"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("peb_lateral_nm"), "{err}");
    }

    /// The dose falls back to [process] (30 mJ/cm², as Python's default);
    /// the run notes over- and under-development instead of guessing a
    /// pattern-independent volumetric dose.
    #[test]
    fn test_volumetric_development_notes() {
        let base = "[source]\nwavelength_nm = 157.63\n[optics]\nna = 0.75\n[mask]\n\
                    cd_nm = 150.0\npitch_nm = 300.0\n[grid]\nsize = 32\npixel_nm = 18.75\n\
                    [deep]\nmode = \"volumetric\"\nnz = 8\ndevelop = \"fmm\"\n";
        let notes = |extra: &str| {
            let (config, deep) = validated(&format!("{base}{extra}")).unwrap();
            let (summary, _) = volumetric_run(&config, &deep).unwrap();
            assert_eq!(summary["peb"], "gaussian");
            summary["developed"]["notes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_str().unwrap().to_string())
                .collect::<Vec<_>>()
        };
        let fallback = notes("");
        assert_eq!(fallback.len(), 1, "{fallback:?}");
        assert!(fallback[0].starts_with("over-developed?"), "{fallback:?}");
        assert!(fallback[0].contains("30.0 mJ/cm² ([process] dose_mj_cm2 fallback)"));
        // Near dose-to-size for this 150/300 L/S (bottom CD 150 nm): no note.
        assert!(notes("dose_mj_cm2 = 12.0\n").is_empty());
        let under = notes("dose_mj_cm2 = 4.0\n");
        assert!(under[0].starts_with("under-developed"), "{under:?}");
    }

    #[test]
    fn test_parse_liga_extended_options() {
        let toml_str = format!(
            "{BASE_SECTIONS}\n[deep]\nmode = \"liga\"\n\
             diffraction = \"gaussian\"\nenergy_bins = 32\nabsorber = \"Au@17.5\"\n\
             absorber_thickness_um = 25.0\nflux_density = [[7.999, 2.5e14], [8.001, 2.5e14]]\n\
             edge_profile = true\nedge_dx_nm = 20.0\n\
             [[deep.filters]]\nmaterial = \"Be\"\nthickness_um = 100.0\n\
             [[deep.filters]]\nmaterial = \"Kapton\"\nthickness_um = 50.0\n"
        );
        let (config, deep) = validated(&toml_str).unwrap();
        assert_eq!(deep.diffraction.as_deref(), Some("gaussian"));
        let filters = deep.filters.as_ref().unwrap();
        assert_eq!(filters.len(), 2);
        assert_eq!(filters[1].material, "Kapton");
        let (dx, exposure) = liga_setup(&config, &deep).unwrap();
        assert_eq!(dx.filters.len(), 2);
        assert_eq!(dx.proximity_model, ProximityModel::Gaussian);
        assert_eq!(dx.absorber.compound.density_g_cm3, 17.5);
        // The flux-density table is an absolute spectrum.
        assert!(exposure.exposure_time_estimate.is_some());
        // Giving another spectrum as well is an error (it used to be
        // silently overridden by the table).
        let both = toml_str.replace(
            "mode = \"liga\"\n",
            "mode = \"liga\"\ncritical_energy_kev = 6.23\n",
        );
        let err = validated(&both).unwrap_err();
        assert!(
            err.contains("`critical_energy_kev` is not used here"),
            "{err}"
        );
    }

    #[test]
    fn test_liga_absolute_bending_magnet_exposure_time() {
        // [source] bending magnet + geometry: 2.5 GeV / 1.5 T / 200 mA,
        // 15 m, 50 mm scan, defaults (2 um Ti, 500 um PMMA, 3 kJ/cm^3):
        // t = 605.27 s (independent numpy calculation, see deep_xray tests).
        let toml_str = r#"
            [source]
            type = "synchrotron"
            beamline = "bending_magnet"
            electron_energy_gev = 2.5
            field_t = 1.5
            ring_current_ma = 200.0

            [optics]
            na = 0.2

            [mask]
            cd_nm = 20000.0
            pitch_nm = 40000.0

            [grid]
            size = 16
            pixel_nm = 5000.0

            [deep]
            mode = "liga"
            source_distance_m = 15.0
            vertical_scan_mm = 50.0
        "#;
        let (config, deep) = parse(toml_str);
        let (_dx, exposure) = liga_setup(&config, &deep).unwrap();
        let t = exposure.exposure_time_estimate.unwrap();
        assert!((t / 605.265 - 1.0).abs() < 1e-4, "exposure time {t}");
        // Geometry without the ring parameters is rejected.
        let bad = toml_str.replace("vertical_scan_mm = 50.0", "");
        let (config, deep) = parse(&bad);
        assert!(liga_setup(&config, &deep).is_err());
    }

    const TINY_VOLUMETRIC: &str = r#"
        [source]
        wavelength_nm = 157.63

        [optics]
        na = 0.75

        [mask]
        cd_nm = 90.0
        pitch_nm = 256.0

        [grid]
        size = 32
        pixel_nm = 8.0

        [deep]
        mode = "volumetric"
        dose_mj_cm2 = 60.0
        nz = 8
        n_defocus_planes = 2
        resist_thickness_nm = 120.0
    "#;

    #[test]
    fn test_parse_volumetric_bake_and_development_keys() {
        let toml_str = format!(
            "{TINY_VOLUMETRIC}peb = \"car\"\ncar_quencher = 0.2\ncar_k_amp = 0.3\n\
             develop = \"level_set\"\ndev_time_s = 20.0\nsurface_rate_ratio = 0.2\n\
             inhibition_depth_nm = 10.0\nlateral_boundary = \"reflecting\"\n\
             depletion = \"exponential\"\ndepletion_time_constant_s = 30.0\n"
        );
        let (_c, deep) = parse(&toml_str);
        assert_eq!(deep.peb.as_deref(), Some("car"));
        assert_eq!(deep.car_quencher, Some(0.2));
        assert_eq!(deep.develop.as_deref(), Some("level_set"));
        assert_eq!(deep.depletion_time_constant_s, Some(30.0));
        assert!(matches!(
            volumetric_depletion(&deep).unwrap(),
            highuvlith_core::volumetric::DeveloperDepletion::Exponential { time_constant_s }
                if time_constant_s == 30.0
        ));
    }

    /// Tiny end-to-end volumetric runs: CAR bake + level set with surface
    /// inhibition, and Gaussian bake + fast marching, both develop the
    /// exposed spaces; bad combinations are rejected.
    #[test]
    fn test_volumetric_bake_and_development_end_to_end() {
        let run = |extra: &str| {
            let toml_str = format!("{TINY_VOLUMETRIC}{extra}");
            let (config, deep) = parse(&toml_str);
            volumetric_run(&config, &deep)
        };
        let (summary, image) = run(
            "peb = \"car\"\ndevelop = \"level_set\"\ndev_time_s = 30.0\n\
             surface_rate_ratio = 0.3\ninhibition_depth_nm = 10.0\n",
        )
        .unwrap();
        assert_eq!(summary["peb"], "car");
        assert!(summary["car_neutralized_total"].as_f64().unwrap() >= 0.0);
        assert!(summary["developed"]["level_set_steps"].as_u64().unwrap() > 0);
        let height = summary["developed"]["mean_remaining_height_nm"]
            .as_f64()
            .unwrap();
        assert!((0.0..=120.0).contains(&height));
        assert_eq!(image.dim(), (8, 32)); // x–z developed profile

        let (summary, _) =
            run("peb = \"gaussian\"\npeb_vertical_nm = 5.0\ndevelop = \"fmm\"\n").unwrap();
        assert_eq!(summary["develop"], "fmm");
        assert!(summary["developed"]["mean_remaining_height_nm"].is_number());

        // Depletion requires the level set; unknown tags are rejected.
        assert!(
            run("develop = \"fmm\"\ndepletion = \"loading\"\nloading_capacity_nm = 50.0\n")
                .is_err()
        );
        assert!(run("peb = \"bake\"\n").is_err());
        assert!(run("develop = \"etch\"\n").is_err());
        assert!(run("lateral_boundary = \"sideways\"\n").is_err());
    }

    /// Tiny end-to-end LIGA run: the top-of-resist dose must exceed the bottom
    /// (top/bottom ratio > 1) for a hard bending-magnet beam into thick PMMA.
    #[test]
    fn test_liga_end_to_end_dose_ratio() {
        let toml_str = format!(
            "{BASE_SECTIONS}\n[deep]\nmode = \"liga\"\ncritical_energy_kev = 6.23\n\
             resist_thickness_um = 500.0\nproximity_gap_um = 100.0\nnz = 16\n"
        );
        let (config, deep) = parse(&toml_str);
        let (dx, exposure) = liga_setup(&config, &deep).unwrap();
        assert!(
            exposure.dose_ratio > 1.0,
            "expected top/bottom dose ratio > 1, got {}",
            exposure.dose_ratio
        );
        assert_eq!(dx.resist_thickness_um, 500.0);

        // The volumetric develop path also runs on a 64-grid without error.
        let grid = config.to_grid().unwrap();
        let mask = config.to_mask().unwrap();
        let volume = deep_xray::expose_volumetric(&dx, &mask, &grid, 16).unwrap();
        let depth = deep_xray::develop_depth(&volume, dx.target_bottom_dose_kj_cm3);
        assert_eq!(depth.dim(), (config.grid.size, config.grid.size));
    }

    #[test]
    fn test_parse_talbot_config() {
        let toml_str = format!(
            "{BASE_SECTIONS}\n[deep]\nmode = \"talbot\"\ntalbot_mode = \"atl\"\n\
             grating_period_nm = 100.0\nduty_cycle = 0.5\ngrating_type = \"amplitude\"\n\
             max_order = 7\ngap_um = 100.0\nbandwidth_nm = 0.3\nspectrum_shape = \"gaussian\"\n\
             il_order = 1\nabsorption_per_nm = 0.004\n"
        );
        let (_c, deep) = parse(&toml_str);
        assert_eq!(deep.resolve_mode().unwrap(), DeepMode::Talbot);
        assert_eq!(deep.talbot_mode.as_deref(), Some("atl"));
        assert_eq!(deep.grating_period_nm, Some(100.0));
        assert_eq!(deep.bandwidth_nm, Some(0.3));
        assert_eq!(deep.il_order, Some(1));
        assert_eq!(DeepMode::parse("talbot").unwrap(), DeepMode::Talbot);
    }

    /// Tiny end-to-end Talbot runs: the grating period defaults to the [mask]
    /// pitch, the Talbot length is 2p²/λ, and every sub-mode runs.
    #[test]
    fn test_talbot_end_to_end_modes() {
        for mode in ["carpet", "dtl", "atl", "euv_il"] {
            let toml_str = format!(
                "{BASE_SECTIONS}\n[deep]\nmode = \"talbot\"\ntalbot_mode = \"{mode}\"\n\
                 max_order = 3\nnz = 4\nbandwidth_nm = 5.0\n"
            );
            let (config, deep) = parse(&toml_str);
            let setup = talbot_setup(&config, &deep).unwrap();
            assert_eq!(setup.grating.period_x_nm, 800.0);
            let expected = 2.0 * 800.0 * 800.0 / 157.63;
            assert!((setup.talbot_length_nm() - expected).abs() < 1e-9 * expected);
            let out = std::env::temp_dir().join(format!("highuvlith_talbot_{mode}.json"));
            run_talbot(&config, &deep, Some(&out)).unwrap();
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
            assert_eq!(json["talbot_mode"], mode);
            std::fs::remove_file(&out).ok();
        }
        // An unknown sub-mode is rejected with the valid list.
        let toml_str =
            format!("{BASE_SECTIONS}\n[deep]\nmode = \"talbot\"\ntalbot_mode = \"moire\"\n");
        let (config, deep) = parse(&toml_str);
        let err = run_talbot(&config, &deep, None).unwrap_err().to_string();
        assert!(err.contains("euv_il"), "{err}");
    }

    /// Lab-scale LIGA: an X-ray tube (tabulated spectrum) and a betatron
    /// (synchrotron-like critical energy) both drive the depth-dose path.
    #[test]
    fn test_liga_from_xray_tube_and_betatron_sources() {
        let sections = |source: &str| {
            format!(
                "[source]\n{source}\n\n[optics]\nna = 0.2\n\n[mask]\ncd_nm = 20000.0\n\
                 pitch_nm = 40000.0\n\n[grid]\nsize = 64\npixel_nm = 2000.0\n\n[deep]\n\
                 mode = \"liga\"\nresist_thickness_um = 200.0\nnz = 16\n"
            )
        };
        let tube = sections("type = \"xray_tube\"\nanode = \"w\"\nkvp = 60.0");
        let (config, deep) = parse(&tube);
        config.validate().unwrap();
        let (dx, exposure) = liga_setup(&config, &deep).unwrap();
        assert!(matches!(dx.spectrum, XraySpectrum::Tabulated { .. }));
        assert!(
            exposure.dose_ratio > 1.0,
            "tube ratio {}",
            exposure.dose_ratio
        );

        let beta = sections("type = \"betatron\"\nelectron_energy_mev = 200.0");
        let (config, deep) = parse(&beta);
        let (dx, exposure) = liga_setup(&config, &deep).unwrap();
        match dx.spectrum {
            XraySpectrum::BendingMagnet {
                critical_energy_kev,
            } => assert!((critical_energy_kev - 8.069).abs() < 1e-2),
            _ => panic!("expected BendingMagnet"),
        }
        assert!(exposure.dose_ratio > 1.0);

        // The new example configs drive the same path.
        for content in [
            include_str!("../../../../examples/sim_xray_tube.toml"),
            include_str!("../../../../examples/sim_betatron.toml"),
        ] {
            let (config, deep) = parse(content);
            config.validate().unwrap();
            assert_eq!(deep.resolve_mode().unwrap(), DeepMode::Liga);
            liga_setup(&config, &deep).unwrap();
        }
    }

    /// The volumetric default stack comes from the materials database at the
    /// source wavelength; where the database has no resist entry (e.g.
    /// 193 nm) the run fails instead of reusing the 157 nm constants.
    #[test]
    fn test_volumetric_default_stack_is_wavelength_checked() {
        let toml_str = TINY_VOLUMETRIC.replace("wavelength_nm = 157.63", "wavelength_nm = 193.0");
        assert_ne!(toml_str, TINY_VOLUMETRIC);
        let (config, deep) = parse(&toml_str);
        let err = format!("{:#}", volumetric_run(&config, &deep).unwrap_err());
        assert!(
            err.contains("VUV_resist") && err.contains("build the film stack explicitly"),
            "{err}"
        );
    }
}

#[cfg(test)]
mod strict_tests {
    use super::*;

    const GRID: &str = "[grid]\nsize = 32\npixel_nm = 20.0\n";

    fn check(toml_str: &str) -> Result<(), String> {
        let config = SimConfig::from_toml_str(toml_str).map_err(|e| e.to_string())?;
        let deep = DeepConfig::from_toml_str(toml_str).map_err(|e| e.to_string())?;
        validate(&config, &deep).map_err(|e| e.to_string())
    }

    #[test]
    fn test_unknown_deep_key_suggests_the_mode_keys() {
        let err = check(&format!(
            "{GRID}[deep]\nmode = \"grayscale\"\nthickness = 1000.0\n"
        ))
        .unwrap_err();
        assert!(err.contains("unknown key `thickness` in [deep]"), "{err}");
        assert!(err.contains("did you mean `thickness_nm`?"), "{err}");
        assert!(
            err.contains("keys read by [deep] mode = \"grayscale\""),
            "{err}"
        );
        assert!(err.contains("line 6"), "{err}");
        // [[deep.filters]] entries are strict too.
        let err = check(&format!(
            "{GRID}[mask]\ncd_nm = 400.0\npitch_nm = 800.0\n[deep]\nmode = \"liga\"\n\
             critical_energy_kev = 6.2\n[[deep.filters]]\nmaterial = \"Be\"\nthickness = 5.0\n"
        ))
        .unwrap_err();
        assert!(
            err.contains("unknown key `thickness` in [deep.filters]"),
            "{err}"
        );
    }

    #[test]
    fn test_keys_of_other_modes_and_options_are_errors() {
        let reject = |body: &str, needle: &str| {
            let text = format!(
                "[source]\nwavelength_nm = 157.63\n[optics]\nna = 0.75\n[mask]\ncd_nm = 400.0\n\
                 pitch_nm = 800.0\n{GRID}[deep]\n{body}\n"
            );
            let err = check(&text).unwrap_err();
            assert!(err.contains(needle), "{body}\n→ {err}");
        };
        // A key of another mode: named with the mode that reads it.
        reject(
            "mode = \"interference\"\ntarget = \"blazed\"",
            "`target` is only read by mode = \"grayscale\"",
        );
        reject(
            "mode = \"grayscale\"\nnz = 16",
            "`nz` is only read by mode = ",
        );
        // Sub-mode / option rules.
        reject(
            "mode = \"grayscale\"\ntarget = \"microlens\"\nperiod_px = 8",
            "read by target = blazed",
        );
        reject(
            "mode = \"volumetric\"\npeb = \"car\"\npeb_lateral_nm = 20.0",
            "need peb = \"gaussian\"",
        );
        reject(
            "mode = \"volumetric\"\npeb = \"gaussian\"\ncar_quencher = 0.2",
            "need peb = \"car\"",
        );
        reject(
            "mode = \"volumetric\"\ndev_time_s = 30.0",
            "development keys need develop",
        );
        reject(
            "mode = \"volumetric\"\ndevelop = \"fmm\"\ndepletion = \"loading\"\nloading_capacity_nm = 50.0",
            "developer depletion needs develop = \"level_set\"",
        );
        reject(
            "mode = \"volumetric\"\ndevelop = \"level_set\"\ndepletion = \"exponential\"\n\
             loading_length_nm = 30.0",
            "reads only depletion_time_constant_s",
        );
        reject(
            "mode = \"talbot\"\ntalbot_mode = \"carpet\"\ngap_um = 10.0",
            "not used by talbot_mode = \"carpet\"",
        );
        reject(
            "mode = \"talbot\"\ntalbot_mode = \"dtl\"\nbandwidth_nm = 0.3",
            "the band belongs to talbot_mode = \"atl\"",
        );
        reject(
            "mode = \"talbot\"\ntalbot_mode = \"atl\"\nphase_rad = 1.0",
            "grating_type = \"phase\"",
        );
        reject(
            "mode = \"liga\"\ncritical_energy_kev = 6.2\nedge_dx_nm = 5.0",
            "needs edge_profile = true",
        );
        reject(
            "mode = \"liga\"\ncritical_energy_kev = 6.2\nsource_distance_m = 15.0",
            "absolute exposure takes the spectrum from the [source]",
        );
        reject(
            "mode = \"liga\"\ncritical_energy_kev = 6.2\ndiffraction = \"gaussian\"\n\
             strict_sampling = true",
            "sampling check is for the Fresnel model",
        );
        // Volumetric images monochromatically.
        let text = format!(
            "[source]\nwavelength_nm = 157.63\n[optics]\nna = 0.75\n[mask]\ncd_nm = 400.0\n\
             pitch_nm = 800.0\n{GRID}[imaging]\nspectrum = \"per_wavelength\"\n[deep]\n\
             mode = \"volumetric\"\n"
        );
        assert!(check(&text).unwrap_err().contains("centre wavelength only"));
        // A missing [deep] table names the modes.
        let mut cfg = std::env::temp_dir();
        cfg.push(format!("huv_deep_missing_{}.toml", std::process::id()));
        std::fs::write(&cfg, "[source]\nwavelength_nm = 157.63\n").unwrap();
        let err = run(&cfg, None).unwrap_err().to_string();
        assert!(err.contains("needs a [deep] table"), "{err}");
        std::fs::remove_file(&cfg).ok();
    }

    /// Modes that need no projection optics run without [optics] (and
    /// grayscale without [source] / [mask]); unread tables are only noted.
    #[test]
    fn test_modes_run_with_only_the_tables_they_read() {
        let (config, deep) = (
            SimConfig::from_toml_str(&format!(
                "{GRID}[deep]\nmode = \"grayscale\"\ntarget = \"staircase\"\nn_levels = 4\n"
            ))
            .unwrap(),
            DeepConfig::from_toml_str(&format!(
                "{GRID}[deep]\nmode = \"grayscale\"\ntarget = \"staircase\"\nn_levels = 4\n"
            ))
            .unwrap(),
        );
        validate(&config, &deep).unwrap();
        run_mode(&config, &deep, None).unwrap();
        // Interference needs only a wavelength.
        let text = format!(
            "[source]\nwavelength_nm = 266.0\n{GRID}[deep]\nmode = \"interference\"\nnz = 4\n"
        );
        let (config, deep) = (
            SimConfig::from_toml_str(&text).unwrap(),
            DeepConfig::from_toml_str(&text).unwrap(),
        );
        validate(&config, &deep).unwrap();
        run_mode(&config, &deep, None).unwrap();
        // LIGA without a mask: the depth map needs one.
        let text = format!("{GRID}[deep]\nmode = \"liga\"\ncritical_energy_kev = 6.2\nnz = 4\n");
        let (config, deep) = (
            SimConfig::from_toml_str(&text).unwrap(),
            DeepConfig::from_toml_str(&text).unwrap(),
        );
        validate(&config, &deep).unwrap();
        let err = run_mode(&config, &deep, None).unwrap_err().to_string();
        assert!(err.contains("needs a [mask] table"), "{err}");
    }

    /// X-ray tube at `source_distance_m`: an absolute spectrum (isotropic
    /// point source), so the exposure time scales with the distance squared
    /// and inversely with the tube current — exact identities of the
    /// inverse-square law and of the dose being linear in the photon flux.
    #[test]
    fn test_liga_xray_tube_absolute_exposure_time_scaling() {
        let liga = |extra_source: &str, distance_m: f64| {
            let text = format!(
                "[source]\ntype = \"xray_tube\"\nanode = \"w\"\nkvp = 60.0\n{extra_source}\n\
                 [mask]\ncd_nm = 20000.0\npitch_nm = 40000.0\n[grid]\nsize = 16\npixel_nm = 5000.0\n\
                 [deep]\nmode = \"liga\"\nresist_thickness_um = 200.0\nenergy_bins = 24\n\
                 source_distance_m = {distance_m}\n"
            );
            let (config, deep) = (
                SimConfig::from_toml_str(&text).unwrap(),
                DeepConfig::from_toml_str(&text).unwrap(),
            );
            validate(&config, &deep).unwrap();
            let (dx, exposure) = liga_setup(&config, &deep).unwrap();
            assert!(matches!(dx.spectrum, XraySpectrum::FluxDensity { .. }));
            exposure.exposure_time_estimate.expect("absolute spectrum")
        };
        let t1 = liga("tube_current_ma = 30.0", 0.1);
        let t2 = liga("tube_current_ma = 30.0", 0.2);
        let t3 = liga("tube_current_ma = 60.0", 0.1);
        assert!(t1.is_finite() && t1 > 0.0);
        assert!((t2 / t1 - 4.0).abs() < 1e-9, "inverse square: {t1} -> {t2}");
        assert!((t3 / t1 - 0.5).abs() < 1e-9, "current: {t1} -> {t3}");
        // A bending-magnet scan height is not a lab-source key.
        let text = "[source]\ntype = \"xray_tube\"\n[mask]\ncd_nm = 20000.0\npitch_nm = 40000.0\n\
                    [deep]\nmode = \"liga\"\nsource_distance_m = 0.1\nvertical_scan_mm = 10.0\n";
        assert!(check(text)
            .unwrap_err()
            .contains("lab source is a point source"));
        // The betatron's absolute spectrum works the same way.
        let text = "[source]\ntype = \"betatron\"\n[mask]\ncd_nm = 20000.0\npitch_nm = 40000.0\n\
                    [grid]\nsize = 16\npixel_nm = 5000.0\n[deep]\nmode = \"liga\"\n\
                    resist_thickness_um = 100.0\nenergy_bins = 24\nsource_distance_m = 1.0\n";
        let (config, deep) = (
            SimConfig::from_toml_str(text).unwrap(),
            DeepConfig::from_toml_str(text).unwrap(),
        );
        validate(&config, &deep).unwrap();
        let (_, exposure) = liga_setup(&config, &deep).unwrap();
        assert!(exposure.exposure_time_estimate.unwrap() > 0.0);
    }

    /// The betatron's relative (`BendingMagnet`, log bins in E) and absolute
    /// (flux-density table, 200 equal bins in E) spectra describe the same
    /// shape, so the top/bottom dose ratio may differ only by quadrature
    /// error (the attenuation tables carry absorption edges, which the
    /// midpoint rule integrates to first order in the bin width).
    #[test]
    fn test_betatron_relative_and_absolute_dose_ratio_agree() {
        let ratio = |extra: &str| {
            let text = format!(
                "[source]\ntype = \"betatron\"\n[mask]\ncd_nm = 20000.0\npitch_nm = 40000.0\n\
                 [grid]\nsize = 16\npixel_nm = 5000.0\n[deep]\nmode = \"liga\"\n\
                 resist_thickness_um = 100.0\n{extra}"
            );
            let (config, deep) = (
                SimConfig::from_toml_str(&text).unwrap(),
                DeepConfig::from_toml_str(&text).unwrap(),
            );
            liga_setup(&config, &deep).unwrap().1.dose_ratio
        };
        let relative_default = ratio("");
        let relative_fine = ratio("energy_bins = 1000\n");
        let absolute = ratio("source_distance_m = 0.5\n");
        // Converged value ~4.560 (5000-20000 log bins); the default 100
        // bins sit ~0.6 % high, the 200-bin absolute table ~0.15 % high.
        assert!(
            (relative_fine / absolute - 1.0).abs() < 3e-3,
            "{relative_fine} vs {absolute}"
        );
        assert!(
            (relative_default / absolute - 1.0).abs() < 1e-2,
            "{relative_default} vs {absolute}"
        );
    }
}

#[cfg(test)]
mod example_tests {
    use super::*;

    /// Every `examples/*.toml` parses with strict keys and validates for the
    /// subcommand it is written for (`[deep]` → `deep`, `[optimize]` →
    /// `optimize`, otherwise `simulate`). The integration test `tests/examples.rs` runs them.
    #[test]
    fn test_every_example_config_validates() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let mut count = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "toml") {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let text = std::fs::read_to_string(&path).unwrap();
            let config = SimConfig::from_toml_str(&text)
                .unwrap_or_else(|e| panic!("{name} does not parse: {e}"));
            if config.has_table("deep") {
                let deep = DeepConfig::from_toml_str(&text)
                    .unwrap_or_else(|e| panic!("{name} [deep] does not parse: {e}"));
                validate(&config, &deep).unwrap_or_else(|e| panic!("{name}: {e}"));
            } else if config.has_table("optimize") {
                let opt = crate::commands::optimize::OptimizeConfig::from_toml_str(&text)
                    .unwrap_or_else(|e| panic!("{name} [optimize] does not parse: {e}"));
                crate::commands::optimize::validate(&config, &opt)
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
            } else {
                config
                    .validate()
                    .unwrap_or_else(|e| panic!("{name} does not validate: {e}"));
                config.to_optics().unwrap();
                config.to_mask().unwrap();
            }
            count += 1;
        }
        assert!(count >= 26, "found only {count} example configs");
    }
}
