//! `highuvlith optimize` — mask optimization and spacer patterning.
//!
//! One subcommand over the optimization modules, selected by
//! `[optimize] method = "…"`:
//!
//! - `ilt` — pixel-based inverse lithography with the exact adjoint
//!   gradient through the engine's SOCS kernels ([`highuvlith_core::ilt`]).
//!   The target is the drawn `[mask]` (its clear area = the region that
//!   should print bright); the absorber follows `[mask] mask_type`.
//! - `opc` — fragment-based model OPC ([`highuvlith_core::opc::fragment_opc`]):
//!   edges are split into fragments whose biases are driven by the measured
//!   edge-placement error (EPE) until the printed contour matches the drawn
//!   one.
//! - `sraf` — rule-based sub-resolution assist features with a model print
//!   check at (focus, dose) corners ([`highuvlith_core::sraf`]); optionally
//!   the depth of focus of a line with and without the assists. The rule
//!   deck is the heuristic λ/NA-scaled `SrafRules::for_engine` — a starting
//!   point, not a calibrated deck.
//! - `sadp` / `saqp` — geometric self-aligned double / quadruple patterning
//!   ([`highuvlith_core::double_patterning`]): conformal spacer steps on a
//!   mandrel grating; no imaging.
//!
//! The imaging methods reuse `[source]` / `[optics]` / `[mask]` / `[grid]` /
//! `[imaging]` / `[illumination]` (via [`SimConfig`]) and print at a
//! constant threshold: `[optimize] threshold`, or the dose-to-size
//! threshold of `dose_to_size_cd_nm`, or `[process] threshold`. Every
//! method images at the source's centre wavelength.
//!
//! With `--output foo.png` the optimized mask (ILT transmission, OPC / SRAF
//! mask intensity transmittance, SADP/SAQP line pattern over one mandrel
//! period) is written as an image; otherwise a JSON summary.

use std::path::Path;

use anyhow::Context;
use ndarray::Array2;
use serde::{Deserialize, Serialize};
use serde_json::json;

use highuvlith_core::aerial::AerialImageEngine;
use highuvlith_core::double_patterning::{self, SpacerPatterningResult, SpacerTone};
use highuvlith_core::ilt::{self, ILTConfig, IltCost, IltGradient, IltInit, IltOptimizer};
use highuvlith_core::io::image_export::{save_png, Colormap};
use highuvlith_core::mask::{Mask, MaskFeature, MaskType};
use highuvlith_core::opc::{self, FragmentKind, FragmentOpcConfig, ProcessCondition};
use highuvlith_core::source::{IlluminationShape, LithographySource};
use highuvlith_core::sraf::{self, DofConfig, LineCut, PrintCheckConfig, SrafRules};
use highuvlith_core::types::{Complex64, GridConfig};

use crate::config::SimConfig;
use crate::keys::{self, KeyRules, UnknownKeyHint};

/// Every `[optimize] method` tag.
pub const OPTIMIZE_METHODS: &[&str] = &["ilt", "opc", "sraf", "sadp", "saqp"];

/// The `[optimize]` table (every key optional; which keys a method reads is
/// enforced by [`OptimizeConfig::check_keys`]). Key names follow the
/// Python keyword arguments of `highuvlith.optimize_ilt` / `fragment_opc` /
/// `insert_srafs` / `sadp` / `saqp`.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct OptimizeConfig {
    /// `"ilt"`, `"opc"`, `"sraf"`, `"sadp"` or `"saqp"` (required).
    pub method: Option<String>,
    // --- print threshold (ilt with cost = "resist", opc, sraf) ---
    /// Print threshold on the clear-field-normalized aerial image.
    pub threshold: Option<f64>,
    /// Anchor the threshold so a line printed through the cut at
    /// (`cut_x_nm`, `cut_y_nm`) is this wide in focus (dose-to-size).
    pub dose_to_size_cd_nm: Option<f64>,
    /// x of the line centre on the dose-to-size cut (nm; default 0).
    pub cut_x_nm: Option<f64>,
    /// y of the horizontal dose-to-size cut (nm; default 0).
    pub cut_y_nm: Option<f64>,
    /// `[[defocus_nm, relative_dose, weight], ...]` (default nominal only).
    pub conditions: Option<Vec<[f64; 3]>>,
    /// Iteration budget (ilt default 50, opc default 40).
    pub max_iterations: Option<usize>,
    // --- ilt ---
    /// `"image"` (default; aerial-image MSE) or `"resist"` (sigmoid resist).
    pub cost: Option<String>,
    /// Resist sigmoid steepness (default 50).
    pub steepness: Option<f64>,
    /// `"adjoint"` (default) or `"proxy"` (legacy local approximation).
    pub gradient: Option<String>,
    /// `"cg"` (default) or `"sd"`.
    pub optimizer: Option<String>,
    /// Initial line-search step, max |Δθ| (default 0.5).
    pub learning_rate: Option<f64>,
    /// Total-variation weight (default 0.01).
    pub tv_weight: Option<f64>,
    /// Binarization penalty weight (default 0).
    pub binarization_weight: Option<f64>,
    /// Iterations before the binarization penalty switches on (default 0).
    pub binarization_after: Option<usize>,
    /// Soft minimum feature (Gaussian density filter, nm; default 0 = off).
    pub min_feature_nm: Option<f64>,
    /// Mask-parametrization sigmoid steepness β (default 4).
    pub sigmoid_steepness: Option<f64>,
    /// `"target"` (default) or `"uniform"`.
    pub init: Option<String>,
    /// Uniform initial transmission (default 0.5).
    pub init_level: Option<f64>,
    /// Relative cost-decrease tolerance (default 1e-4).
    pub convergence_tol: Option<f64>,
    // --- opc ---
    pub tolerance_nm: Option<f64>,
    pub max_epe_tolerance_nm: Option<f64>,
    pub feedback_gain: Option<f64>,
    pub smoothing: Option<f64>,
    pub max_step_nm: Option<f64>,
    pub max_bias_nm: Option<f64>,
    pub max_fragment_nm: Option<f64>,
    pub corner_fragment_nm: Option<f64>,
    pub search_range_nm: Option<f64>,
    pub min_jog_nm: Option<f64>,
    pub bias_grid_nm: Option<f64>,
    pub correct_corners: Option<bool>,
    // --- sraf ---
    /// CD of the main lines the rule deck is scaled for (nm).
    pub line_cd_nm: Option<f64>,
    /// Illumination centre σ for the rule deck (default from the pupil fill).
    pub sigma_center: Option<f64>,
    /// Print-check defocus corners ±z (nm; default 150).
    pub defocus_nm: Option<f64>,
    /// Print-check relative dose excursion (default 0.08).
    pub dose_excursion: Option<f64>,
    /// Relative no-print margin (default 0.05).
    pub margin: Option<f64>,
    /// Also compare the line DOF without / with assists (default false).
    pub compare_dof: Option<bool>,
    pub cd_tolerance_pct: Option<f64>,
    pub focus_range_nm: Option<f64>,
    pub focus_steps: Option<usize>,
    pub exposure_latitude_pct: Option<f64>,
    // --- sadp / saqp ---
    pub mandrel_pitch_nm: Option<f64>,
    pub mandrel_cd_nm: Option<f64>,
    pub spacer_nm: Option<f64>,
    pub spacer1_nm: Option<f64>,
    pub spacer2_nm: Option<f64>,
    /// `"spacer_is_line"` (default) or `"spacer_is_dielectric"`.
    pub tone: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OptimizeFile {
    #[serde(default)]
    optimize: OptimizeConfig,
}

const THRESHOLD_KEYS: [&str; 4] = ["threshold", "dose_to_size_cd_nm", "cut_x_nm", "cut_y_nm"];

const ILT_KEYS: &[&str] = &[
    "method",
    "cost",
    "threshold",
    "dose_to_size_cd_nm",
    "cut_x_nm",
    "cut_y_nm",
    "steepness",
    "gradient",
    "optimizer",
    "max_iterations",
    "learning_rate",
    "tv_weight",
    "binarization_weight",
    "binarization_after",
    "min_feature_nm",
    "sigmoid_steepness",
    "conditions",
    "init",
    "init_level",
    "convergence_tol",
];
const OPC_KEYS: &[&str] = &[
    "method",
    "threshold",
    "dose_to_size_cd_nm",
    "cut_x_nm",
    "cut_y_nm",
    "conditions",
    "max_iterations",
    "tolerance_nm",
    "max_epe_tolerance_nm",
    "feedback_gain",
    "smoothing",
    "max_step_nm",
    "max_bias_nm",
    "max_fragment_nm",
    "corner_fragment_nm",
    "search_range_nm",
    "min_jog_nm",
    "bias_grid_nm",
    "correct_corners",
];
const SRAF_KEYS: &[&str] = &[
    "method",
    "threshold",
    "dose_to_size_cd_nm",
    "cut_x_nm",
    "cut_y_nm",
    "line_cd_nm",
    "sigma_center",
    "defocus_nm",
    "dose_excursion",
    "margin",
    "compare_dof",
    "cd_tolerance_pct",
    "focus_range_nm",
    "focus_steps",
    "exposure_latitude_pct",
];
const DOF_KEYS: [&str; 4] = [
    "cd_tolerance_pct",
    "focus_range_nm",
    "focus_steps",
    "exposure_latitude_pct",
];
const SADP_KEYS: &[&str] = &[
    "method",
    "mandrel_pitch_nm",
    "mandrel_cd_nm",
    "spacer_nm",
    "tone",
];
const SAQP_KEYS: &[&str] = &[
    "method",
    "mandrel_pitch_nm",
    "mandrel_cd_nm",
    "spacer1_nm",
    "spacer2_nm",
    "tone",
];

impl OptimizeConfig {
    /// Keys read by a method before any conditional rule.
    pub fn method_keys(method: &str) -> Option<&'static [&'static str]> {
        Some(match method {
            "ilt" => ILT_KEYS,
            "opc" => OPC_KEYS,
            "sraf" => SRAF_KEYS,
            "sadp" => SADP_KEYS,
            "saqp" => SAQP_KEYS,
            _ => return None,
        })
    }

    /// The method tag (validated).
    pub fn method(&self) -> anyhow::Result<&str> {
        let tag = self.method.as_deref().ok_or_else(|| {
            anyhow::anyhow!(
                "[optimize] method is required (one of: {})",
                OPTIMIZE_METHODS.join(", ")
            )
        })?;
        if Self::method_keys(tag).is_none() {
            anyhow::bail!(
                "unknown [optimize] method '{tag}' (expected one of: {})",
                OPTIMIZE_METHODS.join(", ")
            );
        }
        Ok(tag)
    }

    /// Whether the method images through the engine.
    pub fn uses_imaging(&self) -> bool {
        matches!(self.method.as_deref(), Some("ilt" | "opc" | "sraf"))
    }

    /// The keys the selected method reads, after the conditional rules.
    pub fn key_rules(&self) -> anyhow::Result<KeyRules> {
        let method = self.method()?;
        let mut rules = KeyRules::new(
            "[optimize]",
            format!("method = \"{method}\""),
            Self::method_keys(method).unwrap_or_default(),
        );
        if method == "ilt" {
            if self.cost.as_deref().unwrap_or("image") == "image" {
                rules.exclude_all(
                    &THRESHOLD_KEYS,
                    "the image-fidelity cost (cost = \"image\") has no print threshold; \
                     set cost = \"resist\"",
                );
                rules.exclude("steepness", "steepness is the cost = \"resist\" sigmoid");
            }
            if self.init.as_deref().unwrap_or("target") != "uniform" {
                rules.exclude("init_level", "init_level is read with init = \"uniform\"");
            }
        }
        if matches!(method, "ilt" | "opc" | "sraf") && self.dose_to_size_cd_nm.is_none() {
            rules.exclude_all(
                &["cut_x_nm", "cut_y_nm"],
                "the cut locates the dose-to-size line (give dose_to_size_cd_nm)",
            );
        }
        if method == "sraf" && self.compare_dof != Some(true) {
            rules.exclude_all(
                &DOF_KEYS,
                "the DOF settings are read with compare_dof = true",
            );
        }
        Ok(rules)
    }

    /// Reject keys the selected method does not read.
    pub fn check_keys(&self) -> anyhow::Result<()> {
        let owners = |key: &str| -> Vec<String> {
            OPTIMIZE_METHODS
                .iter()
                .filter(|m| Self::method_keys(m).is_some_and(|k| k.contains(&key)))
                .map(|m| format!("method = \"{m}\""))
                .collect()
        };
        self.key_rules()?
            .check(&keys::present_keys(self), &owners)?;
        if self.threshold.is_some() && self.dose_to_size_cd_nm.is_some() {
            anyhow::bail!(
                "[optimize] give the print threshold once: threshold OR dose_to_size_cd_nm"
            );
        }
        Ok(())
    }

    /// Parse the `[optimize]` table of a config file (strict keys).
    pub fn from_toml_str(text: &str) -> anyhow::Result<Self> {
        Ok(keys::parse_toml::<OptimizeFile>(text, &optimize_table_hint)?.optimize)
    }

    fn conditions(&self) -> Vec<ProcessCondition> {
        match &self.conditions {
            Some(list) => list
                .iter()
                .map(|&[defocus_nm, dose, weight]| ProcessCondition {
                    defocus_nm,
                    dose,
                    weight,
                })
                .collect(),
            None => vec![ProcessCondition::nominal()],
        }
    }

    fn tone(&self) -> anyhow::Result<SpacerTone> {
        match self.tone.as_deref().unwrap_or("spacer_is_line") {
            "spacer_is_line" => Ok(SpacerTone::SpacerIsLine),
            "spacer_is_dielectric" => Ok(SpacerTone::SpacerIsDielectric),
            other => anyhow::bail!(
                "[optimize] tone must be 'spacer_is_line' or 'spacer_is_dielectric', got '{other}'"
            ),
        }
    }
}

/// Context hint for an unknown `[optimize]` key: the keys of the method.
fn optimize_table_hint(
    path: &[String],
    raw: &toml::Table,
    expected: &[String],
) -> Option<UnknownKeyHint> {
    if path != ["optimize"] || !expected.iter().any(|e| e == "mandrel_pitch_nm") {
        return None;
    }
    let method = raw.get("optimize")?.get("method")?.as_str()?;
    Some(UnknownKeyHint {
        label: format!("[optimize] method = \"{method}\""),
        keys: OptimizeConfig::method_keys(method)?.to_vec(),
    })
}

/// Entry point for `highuvlith optimize`.
pub fn run(config_path: &Path, output: Option<&Path>) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(config_path)
        .with_context(|| format!("reading {}", config_path.display()))?;
    let config = SimConfig::from_toml_str(&content)
        .map_err(|e| anyhow::anyhow!("{}: {e}", config_path.display()))?;
    if !config.has_table("optimize") {
        anyhow::bail!(
            "{}: `highuvlith optimize` needs an [optimize] table with a method (one of: {})",
            config_path.display(),
            OPTIMIZE_METHODS.join(", ")
        );
    }
    let opt = OptimizeConfig::from_toml_str(&content)
        .map_err(|e| anyhow::anyhow!("{}: {e}", config_path.display()))?;
    validate(&config, &opt)?;
    let out = run_method(&config, &opt)?;
    write_output(output, &out)
}

/// Validate the shared tables and the `[optimize]` keys; note shared tables
/// the method does not read.
pub fn validate(config: &SimConfig, opt: &OptimizeConfig) -> anyhow::Result<()> {
    config.validate()?;
    opt.check_keys()?;
    let imaging = opt.uses_imaging();
    for table in [
        "source",
        "optics",
        "mask",
        "grid",
        "process",
        "imaging",
        "illumination",
    ] {
        if config.has_table(table) && !imaging {
            eprintln!(
                "note: [{table}] is not read by [optimize] method \"{}\" (ignored)",
                opt.method.as_deref().unwrap_or_default()
            );
        }
    }
    if imaging {
        config.source_cfg()?;
        config.optics_cfg()?;
        config.mask_cfg()?;
        if config.imaging.spectrum() != "monochromatic" {
            anyhow::bail!(
                "[optimize] images at the centre wavelength only; remove [imaging] spectrum = \
                 \"{}\"",
                config.imaging.spectrum()
            );
        }
    }
    Ok(())
}

/// Result of one optimize run: JSON summary and the image for `.png`.
#[derive(Debug)]
pub struct OptimizeOutput {
    pub summary: serde_json::Value,
    pub image: Array2<f64>,
}

/// Run the selected method on validated tables.
pub fn run_method(config: &SimConfig, opt: &OptimizeConfig) -> anyhow::Result<OptimizeOutput> {
    match opt.method()? {
        "ilt" => run_ilt(config, opt),
        "opc" => run_opc(config, opt),
        "sraf" => run_sraf(config, opt),
        "sadp" | "saqp" => run_spacer(opt),
        _ => unreachable!("method validated"),
    }
}

fn write_output(output: Option<&Path>, out: &OptimizeOutput) -> anyhow::Result<()> {
    let Some(path) = output else {
        return Ok(());
    };
    if path.extension().is_some_and(|ext| ext == "png") {
        save_png(&out.image, path, Colormap::Grayscale)
            .map_err(|e| anyhow::anyhow!("failed to save PNG: {}", e))?;
    } else {
        std::fs::write(path, serde_json::to_string_pretty(&out.summary)?)?;
    }
    eprintln!("Output written to {}", path.display());
    Ok(())
}

// ============================================================================
// Imaging setup shared by ilt / opc / sraf
// ============================================================================

struct Setup {
    engine: AerialImageEngine,
    mask: Mask,
    grid: GridConfig,
    sigma_center: Option<f64>,
}

fn setup(config: &SimConfig) -> anyhow::Result<Setup> {
    let source = config.to_source()?;
    let optics = config.to_optics()?;
    let mask = config.to_mask()?;
    let grid = config.to_grid()?;
    for w in config.optics_warnings() {
        eprintln!("warning: {w}");
    }
    let engine = AerialImageEngine::with_settings(
        &source,
        optics.as_ref(),
        grid.clone(),
        config.to_imaging_settings()?,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    let sigma_center = match source.illumination() {
        IlluminationShape::Conventional { sigma }
        | IlluminationShape::CoherentGaussian { sigma } => Some(*sigma),
        IlluminationShape::Annular {
            sigma_inner,
            sigma_outer,
        } => Some(0.5 * (sigma_inner + sigma_outer)),
        IlluminationShape::Dipole { sigma_center, .. }
        | IlluminationShape::Quadrupole { sigma_center, .. } => Some(*sigma_center),
        #[allow(unreachable_patterns)]
        _ => None,
    };
    eprintln!(
        "Source:  λ = {:.3} nm ({}), pupil fill {:?}",
        source.wavelength_nm(),
        source.kind_label(),
        source.illumination()
    );
    eprintln!(
        "Optics:  {} NA = {:.3}",
        config.optics_cfg()?.optics_type(),
        optics.na()
    );
    eprintln!(
        "Mask:    {} — {}",
        config.mask_cfg()?.describe(),
        mask.summary()
    );
    eprintln!(
        "Grid:    {}×{}, pixel = {:.4} nm (field {:.1} nm); {} SOCS kernels",
        grid.size,
        grid.size,
        grid.pixel_nm,
        grid.field_size_nm(),
        engine.num_kernels()
    );
    Ok(Setup {
        engine,
        mask,
        grid,
        sigma_center,
    })
}

/// The line cut used for dose-to-size anchoring.
fn size_cut(opt: &OptimizeConfig, mask: &Mask, cd_nm: f64) -> LineCut {
    LineCut {
        x_center_nm: opt.cut_x_nm.unwrap_or(0.0),
        y_nm: opt.cut_y_nm.unwrap_or(0.0),
        half_width_nm: 1.5 * cd_nm,
        dark_line: !mask.dark_field,
    }
}

/// Print threshold: `threshold`, else dose-to-size, else `[process]`.
fn print_threshold(
    config: &SimConfig,
    opt: &OptimizeConfig,
    s: &Setup,
) -> anyhow::Result<(f64, &'static str)> {
    if let Some(t) = opt.threshold {
        if !(t.is_finite() && t > 0.0) {
            anyhow::bail!("[optimize] threshold must be finite and > 0, got {t}");
        }
        return Ok((t, "[optimize] threshold"));
    }
    if let Some(cd) = opt.dose_to_size_cd_nm {
        if !(cd.is_finite() && cd > 0.0) {
            anyhow::bail!("[optimize] dose_to_size_cd_nm must be > 0, got {cd}");
        }
        let cut = size_cut(opt, &s.mask, cd);
        let t = sraf::dose_to_size_threshold(&s.engine, &s.mask, &cut, cd)
            .map_err(|e| anyhow::anyhow!("dose-to-size for {cd} nm: {e}"))?;
        return Ok((t, "dose-to-size"));
    }
    Ok((config.process.threshold(), "[process] threshold"))
}

fn conditions_json(c: &[ProcessCondition]) -> serde_json::Value {
    c.iter()
        .map(|c| json!([c.defocus_nm, c.dose, c.weight]))
        .collect()
}

fn array_json(a: &Array2<f64>) -> serde_json::Value {
    a.rows()
        .into_iter()
        .map(|r| r.iter().copied().collect::<Vec<f64>>())
        .collect()
}

// ============================================================================
// ILT
// ============================================================================

fn run_ilt(config: &SimConfig, opt: &OptimizeConfig) -> anyhow::Result<OptimizeOutput> {
    let s = setup(config)?;
    // Target: the drawn mask's clear area (area coverage at the pixel edges).
    let target = Mask {
        mask_type: MaskType::Binary,
        ..s.mask.clone()
    }
    .rasterize_intensity(&s.grid);
    let absorber = match s.mask.mask_type {
        MaskType::Binary => Complex64::new(0.0, 0.0),
        MaskType::AttenuatedPSM {
            transmission,
            phase_deg,
        } => Complex64::from_polar(transmission.sqrt(), phase_deg.to_radians()),
        MaskType::AlternatingPSM => {
            anyhow::bail!("[optimize] method = \"ilt\" supports binary and att_psm masks")
        }
    };
    let (cost, threshold_note) = match opt.cost.as_deref().unwrap_or("image") {
        "image" => (IltCost::ImageFidelity, None),
        "resist" => {
            let (t, from) = print_threshold(config, opt, &s)?;
            let steepness = opt.steepness.unwrap_or(50.0);
            (
                IltCost::ResistContour {
                    threshold: t,
                    steepness,
                },
                Some(format!("{t:.4} ({from}), sigmoid steepness {steepness}")),
            )
        }
        other => anyhow::bail!("[optimize] cost must be 'image' or 'resist', got '{other}'"),
    };
    let gradient = match opt.gradient.as_deref().unwrap_or("adjoint") {
        "adjoint" => IltGradient::Adjoint,
        "proxy" => IltGradient::LocalProxy,
        other => anyhow::bail!("[optimize] gradient must be 'adjoint' or 'proxy', got '{other}'"),
    };
    let optimizer = match opt.optimizer.as_deref().unwrap_or("cg") {
        "cg" => IltOptimizer::ConjugateGradient,
        "sd" => IltOptimizer::SteepestDescent,
        other => anyhow::bail!("[optimize] optimizer must be 'cg' or 'sd', got '{other}'"),
    };
    let init = match opt.init.as_deref().unwrap_or("target") {
        "target" => IltInit::Target,
        "uniform" => IltInit::Uniform(opt.init_level.unwrap_or(0.5)),
        other => anyhow::bail!("[optimize] init must be 'target' or 'uniform', got '{other}'"),
    };
    let conditions = opt.conditions();
    let ilt_config = ILTConfig {
        target: target.clone(),
        learning_rate: opt.learning_rate.unwrap_or(0.5),
        regularization: opt.tv_weight.unwrap_or(0.01),
        max_iterations: opt.max_iterations.unwrap_or(50),
        convergence_tol: opt.convergence_tol.unwrap_or(1e-4),
        min_feature_nm: opt.min_feature_nm.unwrap_or(0.0),
        gradient,
        optimizer,
        cost,
        conditions: conditions.clone(),
        sigmoid_steepness: opt.sigmoid_steepness.unwrap_or(4.0),
        binarization_weight: opt.binarization_weight.unwrap_or(0.0),
        binarization_after: opt.binarization_after.unwrap_or(0),
        absorber,
        init,
        ..Default::default()
    };
    eprintln!(
        "Method:  ILT ({:?} gradient, {:?}, cost {:?}) — target = drawn [mask] clear area",
        gradient, optimizer, cost
    );
    if let Some(note) = &threshold_note {
        eprintln!("Threshold: {note}");
    }
    // The drawn mask as a reference: its printed-pixel error at the same
    // print threshold the optimizer reports.
    let problem =
        ilt::IltProblem::new(&s.engine, &ilt_config).map_err(|e| anyhow::anyhow!("{e}"))?;
    let print_t = problem.print_threshold();
    let drawn_image = problem.image(&target, 0);
    let drawn_error = ilt::pattern_error(&drawn_image, &target, print_t);
    let start = std::time::Instant::now();
    let r = ilt::optimize_ilt(&s.engine, &ilt_config).map_err(|e| anyhow::anyhow!("{e}"))?;
    let elapsed = start.elapsed();
    let initial_cost = r.cost_history.first().copied().unwrap_or(f64::NAN);
    let termination = match r.termination {
        ilt::IltTermination::Converged => "converged",
        ilt::IltTermination::MaxIterations => "max_iterations",
        ilt::IltTermination::LineSearchFailed => "line_search_failed",
    };
    eprintln!("Compute: {elapsed:.2?}");
    eprintln!("Results:");
    eprintln!(
        "  Cost:            {initial_cost:.4e} -> {:.4e} ({} iterations, {termination})",
        r.final_cost, r.iterations
    );
    eprintln!(
        "  Misprinted px:   drawn mask {drawn_error} -> optimized {} (binary {}) of {} at I = {print_t:.4}",
        r.pattern_error,
        r.binary_pattern_error,
        target.len()
    );
    let summary = json!({
        "method": "ilt",
        "gradient": format!("{gradient:?}"),
        "optimizer": format!("{optimizer:?}"),
        "cost": opt.cost.as_deref().unwrap_or("image"),
        "print_threshold": print_t,
        "conditions": conditions_json(&conditions),
        "iterations": r.iterations,
        "termination": termination,
        "converged": r.converged,
        "initial_cost": initial_cost,
        "final_cost": r.final_cost,
        "cost_history": r.cost_history,
        "stage_starts": r.stage_starts,
        "drawn_pattern_error": drawn_error,
        "pattern_error": r.pattern_error,
        "binary_pattern_error": r.binary_pattern_error,
        "grid": { "size": s.grid.size, "pixel_nm": s.grid.pixel_nm },
        "mask_transmittance": array_json(&r.mask_transmittance),
        "binary_mask": array_json(&r.binary_mask),
    });
    Ok(OptimizeOutput {
        summary,
        image: r.mask_transmittance,
    })
}

// ============================================================================
// Fragment OPC
// ============================================================================

fn run_opc(config: &SimConfig, opt: &OptimizeConfig) -> anyhow::Result<OptimizeOutput> {
    let s = setup(config)?;
    let (threshold, from) = print_threshold(config, opt, &s)?;
    let mut c = FragmentOpcConfig::for_engine(&s.engine, threshold);
    macro_rules! set {
        ($($field:ident),*) => { $( if let Some(v) = opt.$field { c.$field = v; } )* };
    }
    set!(
        max_iterations,
        tolerance_nm,
        max_epe_tolerance_nm,
        feedback_gain,
        smoothing,
        max_step_nm,
        max_bias_nm,
        max_fragment_nm,
        corner_fragment_nm,
        search_range_nm,
        min_jog_nm,
        bias_grid_nm,
        correct_corners
    );
    c.conditions = opt.conditions();
    eprintln!("Method:  fragment model OPC (constant-threshold resist)");
    eprintln!("Threshold: {threshold:.4} ({from})");
    let start = std::time::Instant::now();
    let r = opc::fragment_opc(&s.mask, &s.engine, &c).map_err(|e| anyhow::anyhow!("{e}"))?;
    let elapsed = start.elapsed();
    let first = r
        .history
        .first()
        .cloned()
        .unwrap_or_else(|| r.final_stats.clone());
    eprintln!("Compute: {elapsed:.2?}");
    eprintln!("Results:");
    eprintln!(
        "  EPE rms:   {:.3} -> {:.3} nm   max |EPE|: {:.3} -> {:.3} nm",
        first.epe_rms_nm, r.final_stats.epe_rms_nm, first.epe_max_nm, r.final_stats.epe_max_nm
    );
    eprintln!(
        "  {} fragments on {} polygon(s); {} iterations; {} (tolerance rms {:.3} / max {:.3} nm)",
        r.fragments.len(),
        r.polygons.len(),
        r.history.len(),
        if r.converged {
            "converged"
        } else {
            "NOT converged"
        },
        c.tolerance_nm,
        c.max_epe_tolerance_nm
    );
    if r.final_stats.not_found > 0 {
        eprintln!(
            "  warning: {} controlled fragment(s) found no printed edge within ±{:.1} nm",
            r.final_stats.not_found, c.search_range_nm
        );
    }
    let fragments: Vec<serde_json::Value> = r
        .fragments
        .iter()
        .map(|f| {
            let (x, y) = f.control_point();
            json!({
                "polygon": f.polygon,
                "kind": match f.kind {
                    FragmentKind::Edge => "edge",
                    FragmentKind::Corner => "corner",
                    FragmentKind::LineEnd => "line_end",
                },
                "x_nm": x,
                "y_nm": y,
                "length_nm": f.length(),
                "bias_nm": f.bias_nm,
                "epe_nm": f.epe_nm,
                "found": f.found,
            })
        })
        .collect();
    let history: Vec<serde_json::Value> = r
        .history
        .iter()
        .map(|h| {
            json!({"iteration": h.iteration, "epe_rms_nm": h.epe_rms_nm,
                        "epe_max_nm": h.epe_max_nm, "not_found": h.not_found})
        })
        .collect();
    let summary = json!({
        "method": "opc",
        "threshold": threshold,
        "threshold_source": from,
        "conditions": conditions_json(&c.conditions),
        "converged": r.converged,
        "iterations": r.history.len(),
        "initial_epe_rms_nm": first.epe_rms_nm,
        "initial_epe_max_nm": first.epe_max_nm,
        "final_epe_rms_nm": r.final_stats.epe_rms_nm,
        "final_epe_max_nm": r.final_stats.epe_max_nm,
        "history": history,
        "polygons": r.polygons,
        "fragments": fragments,
    });
    Ok(OptimizeOutput {
        summary,
        image: r.mask.rasterize_intensity(&s.grid),
    })
}

// ============================================================================
// SRAF
// ============================================================================

fn run_sraf(config: &SimConfig, opt: &OptimizeConfig) -> anyhow::Result<OptimizeOutput> {
    let s = setup(config)?;
    let line_cd = opt
        .line_cd_nm
        .or(opt.dose_to_size_cd_nm)
        .or(config.mask_cfg()?.drawn_width_nm())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "[optimize] method = \"sraf\" needs line_cd_nm (the main-line CD the rule deck is \
                 scaled for) for a free-form [mask]"
            )
        })?;
    let sigma_center = opt.sigma_center.or(s.sigma_center).ok_or_else(|| {
        anyhow::anyhow!("[optimize] give sigma_center (no centre σ for this pupil fill)")
    })?;
    let (threshold, from) = print_threshold(config, opt, &s)?;
    let rules = SrafRules::for_engine(&s.engine, line_cd, sigma_center);
    let defocus = opt.defocus_nm.unwrap_or(150.0);
    let excursion = opt.dose_excursion.unwrap_or(0.08);
    let check = PrintCheckConfig {
        margin: opt.margin.unwrap_or(0.05),
        ..PrintCheckConfig::corners(threshold, defocus, excursion)
    };
    eprintln!(
        "Method:  SRAF insertion — heuristic λ/NA rule deck (line CD {line_cd} nm, σ_c \
         {sigma_center:.3}) + model print check"
    );
    eprintln!(
        "Threshold: {threshold:.4} ({from}); print check at focus ±{defocus} nm × dose 1±{excursion}, margin {}",
        check.margin
    );
    let start = std::time::Instant::now();
    let r = sraf::insert_srafs(&s.mask, &s.engine, &rules, &check)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let assists: Vec<serde_json::Value> = r
        .assists
        .iter()
        .filter_map(|a| match a {
            MaskFeature::Rect { x, y, w, h } => Some(json!({"x": x, "y": y, "w": w, "h": h})),
            _ => None,
        })
        .collect();
    let worst_margin = r
        .reports
        .iter()
        .map(|rep| rep.worst_margin)
        .fold(f64::INFINITY, f64::min);
    eprintln!("Results:");
    eprintln!(
        "  Assists:   {} kept ({} placed, {} removed, {} shrink steps); worst no-print margin {}",
        r.assists.len(),
        r.placed,
        r.removed,
        r.shrink_steps,
        if worst_margin.is_finite() {
            format!("{worst_margin:.3}")
        } else {
            "n/a".to_string()
        }
    );
    let mut summary = json!({
        "method": "sraf",
        "rule_deck": "heuristic SrafRules::for_engine (not calibrated)",
        "line_cd_nm": line_cd,
        "sigma_center": sigma_center,
        "threshold": threshold,
        "threshold_source": from,
        "print_check": {"defocus_nm": defocus, "dose_excursion": excursion, "margin": check.margin},
        "placed": r.placed,
        "removed": r.removed,
        "shrink_steps": r.shrink_steps,
        "worst_margin": if worst_margin.is_finite() { json!(worst_margin) } else { json!(null) },
        "assists": assists,
    });
    if opt.compare_dof == Some(true) {
        let dof = DofConfig {
            target_cd_nm: line_cd,
            cd_tolerance_pct: opt.cd_tolerance_pct.unwrap_or(10.0),
            focus_range_nm: opt.focus_range_nm.unwrap_or(300.0),
            focus_steps: opt.focus_steps.unwrap_or(13),
            exposure_latitude_pct: opt.exposure_latitude_pct.unwrap_or(0.0),
        };
        let cut = size_cut(opt, &s.mask, line_cd);
        let cmp = sraf::compare_sraf_dof(&s.engine, &s.mask, &r.mask, &cut, &dof)
            .map_err(|e| anyhow::anyhow!("DOF comparison: {e}"))?;
        eprintln!(
            "  DOF (±{}% CD, each mask at its own dose-to-size): {:.1} nm -> {:.1} nm (×{:.2})",
            dof.cd_tolerance_pct,
            cmp.without.dof_nm,
            cmp.with.dof_nm,
            cmp.gain()
        );
        summary["dof"] = json!({
            "config": dof,
            "without_nm": cmp.without.dof_nm,
            "with_nm": cmp.with.dof_nm,
            "gain": cmp.gain(),
            "focus_nm": cmp.without.focus_nm,
            "cd_without_nm": cmp.without.cd_nm,
            "cd_with_nm": cmp.with.cd_nm,
        });
    }
    eprintln!("Compute: {:.2?}", start.elapsed());
    Ok(OptimizeOutput {
        summary,
        image: r.mask.rasterize_intensity(&s.grid),
    })
}

// ============================================================================
// SADP / SAQP
// ============================================================================

fn required(name: &str, v: Option<f64>) -> anyhow::Result<f64> {
    v.ok_or_else(|| anyhow::anyhow!("[optimize] {name} is required"))
}

fn run_spacer(opt: &OptimizeConfig) -> anyhow::Result<OptimizeOutput> {
    let method = opt.method()?;
    let p = required("mandrel_pitch_nm", opt.mandrel_pitch_nm)?;
    let w = required("mandrel_cd_nm", opt.mandrel_cd_nm)?;
    let tone = opt.tone()?;
    let r: SpacerPatterningResult = if method == "sadp" {
        let sp = required("spacer_nm", opt.spacer_nm)?;
        double_patterning::sadp(p, w, sp, tone)
    } else {
        let s1 = required("spacer1_nm", opt.spacer1_nm)?;
        let s2 = required("spacer2_nm", opt.spacer2_nm)?;
        double_patterning::saqp(p, w, s1, s2, tone)
    }
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    eprintln!(
        "Method:  {} (geometric, conformal spacers; no imaging, deposition/etch bias or LER)",
        method.to_uppercase()
    );
    eprintln!("Mandrel: pitch {p} nm, CD {w} nm, tone {tone:?}");
    let fmt = |v: &[f64]| {
        v.iter()
            .map(|x| format!("{x:.2}"))
            .collect::<Vec<_>>()
            .join(" / ")
    };
    eprintln!("Results:");
    eprintln!(
        "  {} lines per mandrel period; nominal pitch {:.2} nm",
        r.pattern.len(),
        r.nominal_pitch_nm
    );
    eprintln!("  Line CDs:  {} nm", fmt(&r.line_cds_nm));
    eprintln!("  Spaces:    {} nm", fmt(&r.spaces_nm));
    eprintln!("  Pitches:   {} nm", fmt(&r.pitches_nm));
    eprintln!(
        "  Pitch walk {:.3} nm, CD range {:.3} nm",
        r.pitch_walk_nm, r.cd_range_nm
    );
    let summary = json!({
        "method": method,
        "mandrel_pitch_nm": p,
        "mandrel_cd_nm": w,
        "result": r,
    });
    Ok(OptimizeOutput {
        image: line_pattern_image(&r),
        summary,
    })
}

/// One mandrel period of the final lines as a 64 × 512 image (1 = line).
fn line_pattern_image(r: &SpacerPatterningResult) -> Array2<f64> {
    let n = 512;
    let period = r.pattern.period_nm;
    let row: Vec<f64> = (0..n)
        .map(|j| {
            let x = (j as f64 + 0.5) / n as f64 * period;
            let inside = r.pattern.lines.iter().any(|&(a, b)| {
                let (a, b) = (a.rem_euclid(period), b.rem_euclid(period));
                if a <= b {
                    x >= a && x < b
                } else {
                    x >= a || x < b
                }
            });
            f64::from(u8::from(inside))
        })
        .collect();
    Array2::from_shape_fn((64, n), |(_, j)| row[j])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<(SimConfig, OptimizeConfig), String> {
        let config = SimConfig::from_toml_str(text).map_err(|e| e.to_string())?;
        let opt = OptimizeConfig::from_toml_str(text).map_err(|e| e.to_string())?;
        validate(&config, &opt).map_err(|e| e.to_string())?;
        Ok((config, opt))
    }

    /// F2 157.63 nm, NA 0.75, conventional σ 0.6 — the core ILT/OPC test
    /// system; 64 × 12 nm field.
    const VUV: &str = r#"
        [source]
        wavelength_nm = 157.63
        sigma = 0.6

        [optics]
        na = 0.75

        [grid]
        size = 64
        pixel_nm = 12.0
        commensurate = false

        [imaging]
        max_kernels = 12
    "#;

    #[test]
    fn test_method_required_and_unknown_rejected() {
        let err = parse("[optimize]\n").unwrap_err();
        assert!(err.contains("method is required"), "{err}");
        let err = parse("[optimize]\nmethod = \"genetic\"\n").unwrap_err();
        assert!(err.contains("unknown [optimize] method 'genetic'"), "{err}");
    }

    #[test]
    fn test_unknown_and_foreign_keys_are_errors() {
        // Typo: did-you-mean and the method's keys.
        let err = parse(
            "[optimize]\nmethod = \"sadp\"\nmandrel_pitch_nm = 128.0\nmandrel_cd_nm = 32.0\n\
             spacer_thickness_nm = 32.0\n",
        )
        .unwrap_err();
        assert!(
            err.contains("unknown key `spacer_thickness_nm` in [optimize]"),
            "{err}"
        );
        assert!(
            err.contains("keys read by [optimize] method = \"sadp\""),
            "{err}"
        );
        // A key of another method: owner named.
        let err = parse(
            "[optimize]\nmethod = \"sadp\"\nmandrel_pitch_nm = 128.0\nmandrel_cd_nm = 32.0\n\
             spacer_nm = 32.0\nspacer2_nm = 10.0\n",
        )
        .unwrap_err();
        assert!(
            err.contains("`spacer2_nm` is only read by method = \"saqp\""),
            "{err}"
        );
        // Conditional rules: steepness / threshold need cost = "resist".
        let err = parse(&format!(
            "{VUV}[mask]\ncd_nm = 90.0\npitch_nm = 384.0\n[optimize]\nmethod = \"ilt\"\n\
             threshold = 0.3\n"
        ))
        .unwrap_err();
        assert!(err.contains("`threshold` is not used here"), "{err}");
        // DOF keys need compare_dof = true.
        let err = parse(&format!(
            "{VUV}[mask]\ncd_nm = 90.0\npitch_nm = 384.0\n[optimize]\nmethod = \"sraf\"\n\
             focus_steps = 9\n"
        ))
        .unwrap_err();
        assert!(err.contains("compare_dof = true"), "{err}");
        // Both threshold spellings at once.
        let err = parse(&format!(
            "{VUV}[mask]\ncd_nm = 90.0\npitch_nm = 384.0\n[optimize]\nmethod = \"opc\"\n\
             threshold = 0.3\ndose_to_size_cd_nm = 90.0\n"
        ))
        .unwrap_err();
        assert!(err.contains("threshold OR dose_to_size_cd_nm"), "{err}");
        // Imaging methods need the imaging tables.
        let err = parse("[optimize]\nmethod = \"opc\"\n").unwrap_err();
        assert!(err.contains("[source]"), "{err}");
    }

    #[test]
    fn test_sadp_halves_the_pitch_and_walk_fixture() {
        // P = 128, W = 34, s = 32: spaces W = 34 and P − W − 2s = 30,
        // spacer-centre pitches W + s = 66 and P − (W + s) = 62 → walk 4 nm
        // = |2(W + s) − P| (hand calculation); nominal pitch P/2 = 64.
        let (_, opt) = parse(
            "[optimize]\nmethod = \"sadp\"\nmandrel_pitch_nm = 128.0\nmandrel_cd_nm = 34.0\n\
             spacer_nm = 32.0\n",
        )
        .unwrap();
        let out = run_spacer(&opt).unwrap();
        let r = &out.summary["result"];
        assert_eq!(r["nominal_pitch_nm"].as_f64().unwrap(), 64.0);
        assert!((r["pitch_walk_nm"].as_f64().unwrap() - 4.0).abs() < 1e-9);
        assert!((double_patterning::sadp_pitch_walk_nm(128.0, 34.0, 32.0) - 4.0).abs() < 1e-9);
        let mut spaces: Vec<f64> = r["spaces_nm"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        spaces.sort_by(f64::total_cmp);
        assert!((spaces[0] - 30.0).abs() < 1e-9 && (spaces[1] - 34.0).abs() < 1e-9);
        // The image: 2 lines of 32 nm in a 128 nm period → 50 % line fraction.
        let frac = out.image.row(0).sum() / out.image.ncols() as f64;
        assert!((frac - 0.5).abs() < 1e-2, "{frac}");
        // SAQP: 48/16/16 → pitch 32 (P/4), no walk.
        let (_, opt) = parse(
            "[optimize]\nmethod = \"saqp\"\nmandrel_pitch_nm = 128.0\nmandrel_cd_nm = 48.0\n\
             spacer1_nm = 16.0\nspacer2_nm = 16.0\n",
        )
        .unwrap();
        let out = run_spacer(&opt).unwrap();
        assert_eq!(
            out.summary["result"]["nominal_pitch_nm"].as_f64().unwrap(),
            32.0
        );
        assert!(
            out.summary["result"]["pitch_walk_nm"]
                .as_f64()
                .unwrap()
                .abs()
                < 1e-9
        );
        // Missing spacer: error naming it.
        let (_, opt) = parse(
            "[optimize]\nmethod = \"sadp\"\nmandrel_pitch_nm = 128.0\nmandrel_cd_nm = 34.0\n",
        )
        .unwrap();
        assert!(run_spacer(&opt)
            .unwrap_err()
            .to_string()
            .contains("spacer_nm is required"));
    }

    #[test]
    fn test_ilt_lowers_the_cost_and_misprints() {
        // 2 × 2 array of 90 nm square contacts at 180 nm pitch on a dark
        // field (k₁ ≈ 0.43), sigmoid-resist cost at t = 0.25.
        let text = format!(
            "{VUV}[mask]\npattern = \"features\"\ndark_field = true\n\
             features = [\n\
               {{ type = \"rect\", x = -90.0, y = -90.0, w = 90.0, h = 90.0 }},\n\
               {{ type = \"rect\", x = 90.0, y = -90.0, w = 90.0, h = 90.0 }},\n\
               {{ type = \"rect\", x = -90.0, y = 90.0, w = 90.0, h = 90.0 }},\n\
               {{ type = \"rect\", x = 90.0, y = 90.0, w = 90.0, h = 90.0 }},\n\
             ]\n\
             [optimize]\nmethod = \"ilt\"\ncost = \"resist\"\nthreshold = 0.25\n\
             max_iterations = 15\n"
        );
        let (config, opt) = parse(&text).unwrap();
        let out = run_method(&config, &opt).unwrap();
        let s = &out.summary;
        let c0 = s["initial_cost"].as_f64().unwrap();
        let c1 = s["final_cost"].as_f64().unwrap();
        assert!(c1 < 0.5 * c0, "cost {c0} -> {c1}");
        assert!(
            s["pattern_error"].as_u64().unwrap() < s["drawn_pattern_error"].as_u64().unwrap(),
            "{s}"
        );
        assert_eq!(out.image.dim(), (64, 64));
    }

    #[test]
    fn test_opc_reduces_line_end_epe() {
        // 90 nm × 500 nm line at its dose-to-size threshold: the uncorrected
        // line ends pull back; OPC cuts the rms EPE > 5×.
        let text = format!(
            "{VUV}[mask]\npattern = \"features\"\n\
             features = [{{ type = \"rect\", x = 0.0, y = 0.0, w = 90.0, h = 500.0 }}]\n\
             [optimize]\nmethod = \"opc\"\ndose_to_size_cd_nm = 90.0\n"
        );
        let (config, opt) = parse(&text).unwrap();
        let out = run_method(&config, &opt).unwrap();
        let s = &out.summary;
        let before = s["initial_epe_rms_nm"].as_f64().unwrap();
        let after = s["final_epe_rms_nm"].as_f64().unwrap();
        assert!(before > 5.0 * after, "EPE rms {before} -> {after}");
        assert!(s["converged"].as_bool().unwrap());
        assert_eq!(s["threshold_source"], "dose-to-size");
    }

    #[test]
    fn test_sraf_places_non_printing_assists() {
        // Isolated 80 nm line, annular 0.5–0.8 (σ_c 0.65 from the fill):
        // assists survive the print check.
        let text = r#"
            [source]
            wavelength_nm = 157.63
            [illumination]
            shape = "annular"
            sigma_inner = 0.5
            sigma_outer = 0.8
            [optics]
            na = 0.75
            [grid]
            size = 64
            pixel_nm = 12.0
            commensurate = false
            [imaging]
            max_kernels = 12
            [mask]
            pattern = "features"
            features = [{ type = "rect", x = 0.0, y = 0.0, w = 80.0, h = 768.0 }]
            [optimize]
            method = "sraf"
            dose_to_size_cd_nm = 80.0
        "#;
        let (config, opt) = parse(text).unwrap();
        let out = run_method(&config, &opt).unwrap();
        let s = &out.summary;
        assert!((s["sigma_center"].as_f64().unwrap() - 0.65).abs() < 1e-12);
        assert_eq!(s["line_cd_nm"].as_f64().unwrap(), 80.0);
        assert!(s["assists"].as_array().unwrap().len() >= 2, "{s}");
        assert!(s["worst_margin"].as_f64().unwrap() >= 0.05 - 1e-9);
    }
}
