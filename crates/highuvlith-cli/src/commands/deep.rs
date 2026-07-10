//! `highuvlith deep` — deep-layer (3D / high-aspect-ratio) process modes.
//!
//! One subcommand over the four process modules that sculpt resist in depth
//! rather than printing a 2D pattern:
//!
//! - `liga` — LIGA deep X-ray shadow printing ([`highuvlith_core::deep_xray`]):
//!   a synchrotron bending-magnet white beam exposes hundreds of µm of PMMA;
//!   reports the depth-dose top/bottom ratio, damage margin, and aspect ratio.
//! - `grayscale` — analytic 2.5D height mapping
//!   ([`highuvlith_core::grayscale`]): a continuous-transmittance mask carves a
//!   blazed grating, microlens array, or staircase into the resist surface.
//! - `interference` — multi-beam holographic lithography
//!   ([`highuvlith_core::interference`]): interfering plane waves record a
//!   periodic lattice; reports the fringe period and iso-surface fill fraction.
//! - `volumetric` — z-resolved Dill exposure + development
//!   ([`highuvlith_core::volumetric`]): the full aerial-imaging pipeline run
//!   through the resist depth with split-step bleaching, giving per-slice CD
//!   and a sidewall angle.
//!
//! The TOML reuses the standard `[source]` / `[optics]` / `[mask]` / `[grid]`
//! sections (via [`SimConfig`]) plus a `[deep]` table parsed by [`DeepConfig`].
//! With `--output foo.png` each mode writes its characteristic image; otherwise
//! it writes a JSON summary.

use std::path::Path;

use anyhow::Context;
use ndarray::Array2;
use serde::Deserialize;

use highuvlith_core::deep_xray::{self, DeepXrayConfig, LigaExposure, XraySpectrum};
use highuvlith_core::grayscale::{self, ContrastCurve};
use highuvlith_core::interference::{self, ExposureKinetics, InterferenceSetup};
use highuvlith_core::io::image_export::{save_png, Colormap};
use highuvlith_core::source::{LithographySource, SourceKind};
use highuvlith_core::types::Grid2D;
use highuvlith_core::{metrics, volumetric};

use crate::config::SimConfig;

/// The four deep-layer process modes selectable via `[deep] mode = "…"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeepMode {
    Liga,
    Grayscale,
    Interference,
    Volumetric,
}

impl DeepMode {
    /// Parse a mode tag, rejecting anything unknown with the valid list.
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        match s {
            "liga" => Ok(Self::Liga),
            "grayscale" => Ok(Self::Grayscale),
            "interference" => Ok(Self::Interference),
            "volumetric" => Ok(Self::Volumetric),
            other => anyhow::bail!(
                "unknown deep mode '{}' (expected one of: liga, grayscale, interference, volumetric)",
                other
            ),
        }
    }
}

/// The `[deep]` configuration table. Every field beyond `mode` is optional and
/// only consulted by the mode that uses it; each family falls back to a
/// physically reasonable default so a minimal `[deep] mode = "…"` still runs.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct DeepConfig {
    /// Process mode: `liga`, `grayscale`, `interference`, or `volumetric`.
    pub mode: Option<String>,

    // --- liga ---
    /// Bending-magnet critical energy in keV. If omitted, it is derived from a
    /// `[source] type = "synchrotron" beamline = "bending_magnet"` block.
    pub critical_energy_kev: Option<f64>,
    /// PMMA resist thickness in µm (LIGA is hundreds of µm).
    pub resist_thickness_um: Option<f64>,
    /// Mask-to-resist proximity gap in µm (sets the Fresnel blur).
    pub proximity_gap_um: Option<f64>,
    /// Minimum printable feature in nm for the max-aspect-ratio figure.
    pub min_feature_nm: Option<f64>,
    /// Developed-depth threshold dose in kJ/cm³ for the LIGA depth map PNG.
    pub develop_threshold_kj_cm3: Option<f64>,

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
    /// Resist thickness in nm (overrides the default film-stack layer).
    pub resist_thickness_nm: Option<f64>,
    /// PAC threshold for the per-slice CD and depth map (default 0.5).
    pub develop_threshold: Option<f64>,
}

/// Wrapper to pull the `[deep]` table out of the same TOML file that carries
/// the `[source]` / `[optics]` / `[mask]` / `[grid]` sections.
#[derive(Debug, Deserialize)]
struct DeepFile {
    #[serde(default)]
    deep: DeepConfig,
}

impl DeepConfig {
    /// Resolve and validate the process mode tag.
    pub fn resolve_mode(&self) -> anyhow::Result<DeepMode> {
        let tag = self.mode.as_deref().ok_or_else(|| {
            anyhow::anyhow!(
                "[deep] mode is required (one of: liga, grayscale, interference, volumetric)"
            )
        })?;
        DeepMode::parse(tag)
    }
}

/// Entry point for `highuvlith deep`.
pub fn run(config_path: &Path, output: Option<&Path>) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(config_path)
        .with_context(|| format!("reading {}", config_path.display()))?;
    let config: SimConfig = toml::from_str(&content)
        .with_context(|| format!("parsing {} as a simulation config", config_path.display()))?;
    let deep: DeepConfig = toml::from_str::<DeepFile>(&content)
        .with_context(|| format!("parsing [deep] table in {}", config_path.display()))?
        .deep;
    config.validate()?;
    let mode = deep.resolve_mode()?;

    eprintln!("Deep mode: {:?}", mode);
    match mode {
        DeepMode::Liga => run_liga(&config, &deep, output),
        DeepMode::Grayscale => run_grayscale(&config, &deep, output),
        DeepMode::Interference => run_interference(&config, &deep, output),
        DeepMode::Volumetric => run_volumetric(&config, &deep, output),
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

/// Build the LIGA spectrum and depth-dose config, and run the 1D depth-dose
/// exposure. Factored out so tests can assert on the exposure directly.
fn liga_setup(
    config: &SimConfig,
    deep: &DeepConfig,
) -> anyhow::Result<(DeepXrayConfig, LigaExposure)> {
    // Spectrum: an explicit critical energy wins; otherwise derive it from a
    // bending-magnet synchrotron source. Undulators have no white-beam
    // spectrum, so they are rejected with a pointer to the fix.
    let spectrum = if let Some(e_c) = deep.critical_energy_kev {
        if e_c <= 0.0 {
            anyhow::bail!("[deep] critical_energy_kev must be > 0, got {}", e_c);
        }
        XraySpectrum::BendingMagnet {
            critical_energy_kev: e_c,
        }
    } else {
        match config.to_source()? {
            SourceKind::Synchrotron(src) => {
                XraySpectrum::from_synchrotron(&src).ok_or_else(|| {
                    anyhow::anyhow!(
                        "liga mode needs a bending-magnet white beam, but [source] is a \
                         synchrotron undulator (quasi-monochromatic, no white-beam spectrum); \
                         set [source] beamline = \"bending_magnet\" or give [deep] critical_energy_kev"
                    )
                })?
            }
            other => anyhow::bail!(
                "liga mode requires [deep] critical_energy_kev, or [source] type = \
                 \"synchrotron\" beamline = \"bending_magnet\" (got source type '{}')",
                other.kind_label()
            ),
        }
    };

    let mut dx = DeepXrayConfig::pmma_default(spectrum);
    if let Some(t) = deep.resist_thickness_um {
        dx.resist_thickness_um = t;
    }
    if let Some(g) = deep.proximity_gap_um {
        dx.proximity_gap_um = g;
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
        XraySpectrum::Tabulated { .. } => None,
    };
    let min_feature_nm = deep.min_feature_nm.unwrap_or(5000.0);
    let aspect = deep_xray::max_aspect_ratio(&exposure, min_feature_nm);

    eprintln!("LIGA deep X-ray (PMMA):");
    if let Some(e_c) = critical_energy_kev {
        eprintln!("  Critical energy:   {:.3} keV", e_c);
    }
    eprintln!("  Resist thickness:  {:.0} µm", dx.resist_thickness_um);
    eprintln!("  Proximity gap:     {:.0} µm", dx.proximity_gap_um);
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
        "  Damage ceiling:    {:.1} kJ/cm³  [{}]",
        dx.damage_dose_kj_cm3,
        if exposure.exceeds_damage_ceiling {
            "EXCEEDED"
        } else {
            "ok"
        }
    );
    eprintln!(
        "  Max aspect ratio:  {:.1}  (thickness / {:.1} µm feature)",
        aspect,
        min_feature_nm / 1e3
    );

    // Developed-depth map: rasterize the mask, expose volumetrically, and scan
    // each column for the contiguous above-threshold depth.
    let mask = config.to_mask()?;
    let grid = config.to_grid()?;
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

    let summary = serde_json::json!({
        "mode": "liga",
        "critical_energy_kev": critical_energy_kev,
        "resist_thickness_um": dx.resist_thickness_um,
        "proximity_gap_um": dx.proximity_gap_um,
        "top_dose_kj_cm3": exposure.top_dose_kj_cm3,
        "bottom_dose_kj_cm3": exposure.bottom_dose_kj_cm3,
        "target_bottom_dose_kj_cm3": dx.target_bottom_dose_kj_cm3,
        "dose_ratio": exposure.dose_ratio,
        "damage_dose_kj_cm3": dx.damage_dose_kj_cm3,
        "exceeds_damage_ceiling": exposure.exceeds_damage_ceiling,
        "max_aspect_ratio": aspect,
        "min_feature_nm": min_feature_nm,
        "develop_threshold_kj_cm3": threshold,
        "developed_depth_min_um": min_depth_um,
        "developed_depth_max_um": max_depth_um,
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
    use highuvlith_core::aerial::AerialImageEngine;
    use highuvlith_core::resist::ResistParams;
    use highuvlith_core::thinfilm::FilmStack;
    use highuvlith_core::volumetric::VolumetricExposureConfig;

    let source = config.to_source()?;
    let optics = config.to_optics()?;
    let mask = config.to_mask()?;
    let grid = config.to_grid()?;
    let wavelength_nm = source.wavelength_nm();

    let engine = AerialImageEngine::new(&source, optics.as_ref(), grid.clone(), 20)?;

    // Default film stack / resist, with the resist layer resized to the
    // requested thickness.
    let resist_thickness_nm = deep.resist_thickness_nm.unwrap_or(300.0);
    let mut stack = FilmStack::default();
    stack.layers[0].thickness_nm = resist_thickness_nm;
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

    let latent =
        volumetric::expose_volumetric(&engine, &mask, &stack, &resist, wavelength_nm, &vcfg)?;

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
    eprintln!(
        "  CD (top/mid/bot):  {} / {} / {}",
        fmt_cd(top),
        fmt_cd(mid),
        fmt_cd(bottom)
    );
    let sidewall = match (top, bottom) {
        (Some(cd_top), Some(cd_bottom)) => {
            let angle = metrics::sidewall_angle_deg(cd_top, cd_bottom, resist_thickness_nm);
            eprintln!("  Sidewall angle:    {:.1}°", angle);
            Some(angle)
        }
        _ => {
            eprintln!("  Sidewall angle:    n/a (CD not measurable at top and bottom)");
            None
        }
    };

    // Middle-z PAC slice for the PNG.
    let mid_k = latent.pac.data.dim().0 / 2;
    let mid_slice = latent
        .pac
        .data
        .index_axis(ndarray::Axis(0), mid_k)
        .to_owned();

    let summary = serde_json::json!({
        "mode": "volumetric",
        "source_type": source.kind_label(),
        "wavelength_nm": wavelength_nm,
        "resist_thickness_nm": resist_thickness_nm,
        "dose_mj_cm2": vcfg.dose_mj_cm2,
        "nz": vcfg.nz,
        "n_defocus_planes": vcfg.n_defocus_planes,
        "dose_steps": vcfg.dose_steps,
        "develop_threshold": threshold,
        "cd_top_nm": top,
        "cd_mid_nm": mid,
        "cd_bottom_nm": bottom,
        "sidewall_angle_deg": sidewall,
    });
    write_output(output, &summary, &mid_slice, Colormap::Inferno)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(toml_str: &str) -> (SimConfig, DeepConfig) {
        let config: SimConfig = toml::from_str(toml_str).unwrap();
        let deep = toml::from_str::<DeepFile>(toml_str).unwrap().deep;
        (config, deep)
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
}
