//! `highuvlith simulate` — aerial image and printed CD at one dose / focus.

use std::path::Path;

use ndarray::Array2;

use highuvlith_core::aerial::{AerialImageEngine, ImageNormalization};
use highuvlith_core::mask::{LineOrientation, Mask};
use highuvlith_core::metrics::{self, FeatureTone};
use highuvlith_core::source::LithographySource;
use highuvlith_core::types::Grid2D;

use crate::config::SimConfig;

/// Cross-section through the field centre across the measured features:
/// along x for vertical lines, contacts and free-form masks, along y for
/// horizontal lines. Returns `(positions_nm, intensities)`.
pub fn measurement_profile(
    image: &Grid2D<f64>,
    orientation: LineOrientation,
) -> (Vec<f64>, Vec<f64>) {
    match orientation {
        LineOrientation::Vertical => (
            metrics::pixel_centres(image.nx(), image.x_min_nm, image.x_max_nm),
            metrics::centre_profile(&image.data),
        ),
        LineOrientation::Horizontal => {
            let transposed = image.data.t().as_standard_layout().to_owned();
            (
                metrics::pixel_centres(image.data.nrows(), image.y_min_nm, image.y_max_nm),
                metrics::centre_profile(&transposed),
            )
        }
    }
}

pub fn tone_label(tone: FeatureTone) -> &'static str {
    match tone {
        FeatureTone::Dark => "dark",
        FeatureTone::Bright => "bright",
    }
}

/// Orientation of the measured features (horizontal only for horizontal
/// line/space patterns).
pub fn measurement_orientation(config: &SimConfig) -> anyhow::Result<LineOrientation> {
    let mask = config.mask_cfg()?;
    if mask.pattern() == "line_space" {
        mask.orientation()
    } else {
        Ok(LineOrientation::Vertical)
    }
}

/// Result of one `simulate` run: the JSON summary and the aerial image.
pub struct SimulateOutput {
    pub summary: serde_json::Value,
    pub image: Array2<f64>,
}

/// The `simulate` pipeline on a validated config.
pub fn simulate(
    config: &SimConfig,
    focus_override: Option<f64>,
    dose_override: Option<f64>,
) -> anyhow::Result<SimulateOutput> {
    let source = config.to_source()?;
    let optics = config.to_optics()?;
    let mask_cfg = config.mask_cfg()?;
    let mask: Mask = config.to_mask()?;
    let grid = config.to_grid()?;
    for w in config.optics_warnings() {
        eprintln!("warning: {w}");
    }

    let focus = focus_override.unwrap_or(config.process.focus_nm);
    let nominal_dose = config.process.dose_mj_cm2;
    let dose = dose_override.unwrap_or(nominal_dose);
    if !(dose.is_finite() && dose > 0.0) {
        anyhow::bail!("--dose must be finite and > 0, got {dose}");
    }
    // Constant-threshold resist: the contour that prints at dose d is
    // I = threshold · d_nominal / d (clear-field units).
    let threshold = config.process.threshold() * nominal_dose / dose;
    let tone = FeatureTone::of_mask(&mask);

    eprintln!(
        "Source:  λ = {:.2} nm, σ = {:.2} ({})",
        source.wavelength_nm(),
        source.sigma_outer().unwrap_or(0.0),
        source.kind_label(),
    );
    eprintln!("Pupil:   {:?}", source.illumination());
    // Physics derived from the source's machine parameters (informational;
    // the imaging pipeline never reads these).
    let derived = source.derived_quantities();
    if !derived.is_empty() {
        eprintln!("Derived source physics:");
        for q in &derived {
            eprintln!("  {:<34} {:>12.4e} {}", q.name, q.value, q.unit);
        }
    }
    let optics_cfg = config.optics_cfg()?;
    eprintln!(
        "Optics:  {} NA = {:.3}{}{}",
        optics_cfg.optics_type(),
        optics.na(),
        if optics.immersion_index() > 1.0 {
            format!(" (immersion n = {})", optics.immersion_index())
        } else {
            String::new()
        },
        match &optics_cfg.multilayer_pupil {
            Some(ml) => format!(
                ", {} × {} multilayer pupil (🔶 user angle maps)",
                ml.mirrors.len(),
                ml.coating.as_deref().unwrap_or("mo_si")
            ),
            None => String::new(),
        }
    );
    eprintln!("Mask:    {} — {}", mask_cfg.describe(), mask.summary());
    eprintln!(
        "Grid:    {}×{}, pixel = {:.4} nm (field {:.1} nm)",
        grid.size,
        grid.size,
        grid.pixel_nm,
        grid.field_size_nm()
    );
    eprintln!("Focus:   {:.1} nm", focus);
    eprintln!(
        "Dose:    {:.2} mJ/cm² (nominal {:.2}) → printing threshold I = {:.4}",
        dose, nominal_dose, threshold
    );

    let engine = AerialImageEngine::with_settings(
        &source,
        optics.as_ref(),
        grid.clone(),
        config.to_imaging_settings()?,
    )?;
    eprintln!(
        "Engine:  {} SOCS kernels ({:.4} of the TCC trace), {} source points",
        engine.num_kernels(),
        engine.captured_energy_fraction(),
        engine.source_points().len()
    );
    let diagnostics = engine.kernel_diagnostics(focus, source.wavelength_nm())?;
    let dark_field = matches!(
        engine.settings().normalization,
        ImageNormalization::ClearField
    ) && !diagnostics.normalized;
    if dark_field {
        eprintln!(
            "warning: dark-field imaging — the pupil (e.g. a central obscuration) blocks the \
             zero order for almost the whole source (clear field {:.2e}); intensities are \
             absolute, not relative to the clear field",
            diagnostics.clear_field_intensity
        );
    }
    if diagnostics.support_exceeds_nyquist {
        eprintln!(
            "warning: pixel_nm = {} is too coarse — the band (1+σ)·NA/λ that reaches the \
             pupil exceeds the grid Nyquist frequency, so part of the image spectrum is \
             lost; use pixel_nm ≤ λ/(2·NA·(1+σ))",
            grid.pixel_nm
        );
    }
    let spectrum = config.imaging.spectrum();
    eprintln!("Spectrum: {spectrum}");
    if let highuvlith_core::aerial::ImagingModel::Vector(v) = &engine.settings().imaging_model {
        eprintln!(
            "Imaging: vector ({:?}, image index {}, obliquity {})",
            v.polarization, v.image_index, v.obliquity
        );
    }

    let start = std::time::Instant::now();
    let aerial = match spectrum {
        "narrow_band" => engine.compute_polychromatic(&mask, focus, &source, optics.as_ref()),
        "per_wavelength" => engine.compute_multiwavelength(&mask, focus)?,
        _ => engine.compute(&mask, focus),
    };
    let elapsed = start.elapsed();

    let contrast = metrics::image_contrast(&aerial.data);
    let max_i = aerial
        .data
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    let min_i = aerial.data.iter().cloned().fold(f64::INFINITY, f64::min);

    let (x_nm, profile) = measurement_profile(&aerial, measurement_orientation(config)?);
    let printed_cd = metrics::measure_cd_periodic(&profile, &x_nm, threshold, tone);
    let nils = metrics::nils_periodic(&profile, &x_nm, threshold, tone, None);
    let ils = metrics::image_log_slope(&profile, &x_nm, threshold, tone);

    eprintln!("Compute: {:.2?}", elapsed);
    eprintln!("Results:");
    eprintln!("  Contrast:  {:.4}", contrast);
    eprintln!("  I_max:     {:.4}", max_i);
    eprintln!("  I_min:     {:.4}", min_i);
    match printed_cd {
        Some(cd) => eprintln!(
            "  Printed CD ({} feature at I = {:.4}): {:.2} nm",
            tone_label(tone),
            threshold,
            cd
        ),
        None => eprintln!(
            "  Printed CD: n/a — no {} feature at I = {:.4} (nothing prints, or it merges)",
            tone_label(tone),
            threshold
        ),
    }
    if let Some(n) = nils {
        eprintln!("  NILS:      {:.3}", n);
    }

    let pitch = match mask_cfg.pattern() {
        "line_space" => Some(mask_cfg.pitch_nm.unwrap_or(180.0)),
        "contact_holes" => mask_cfg.pitch_nm,
        _ => None,
    };
    let summary = serde_json::json!({
        "wavelength_nm": source.wavelength_nm(),
        "source_type": source.kind_label(),
        "optics_type": optics_cfg.optics_type(),
        "multilayer_pupil": optics_cfg.multilayer_pupil.is_some(),
        "na": optics.na(),
        "immersion_index": optics.immersion_index(),
        "pattern": mask_cfg.pattern(),
        "mask": mask.summary(),
        "cd_nm": mask_cfg.drawn_width_nm(),
        "pitch_nm": pitch,
        "focus_nm": focus,
        "dose_mj_cm2": dose,
        "nominal_dose_mj_cm2": nominal_dose,
        "threshold": threshold,
        "tone": tone_label(tone),
        "spectrum": spectrum,
        "captured_energy_fraction": engine.captured_energy_fraction(),
        "clear_field_intensity": engine.clear_field_intensity(focus),
        "normalized": diagnostics.normalized,
        "dark_field_imaging": dark_field,
        "num_source_points": engine.source_points().len(),
        "grid_size": grid.size,
        "pixel_nm": grid.pixel_nm,
        "field_nm": grid.field_size_nm(),
        "contrast": contrast,
        "i_max": max_i,
        "i_min": min_i,
        "printed_cd_nm": printed_cd,
        "nils": nils,
        "ils_per_nm": ils,
        "compute_ms": elapsed.as_secs_f64() * 1000.0,
        "num_kernels": engine.num_kernels(),
        "derived_quantities": derived,
    });
    Ok(SimulateOutput {
        summary,
        image: aerial.data,
    })
}

pub fn run(
    config_path: &Path,
    output: Option<&Path>,
    focus_override: Option<f64>,
    dose_override: Option<f64>,
) -> anyhow::Result<()> {
    let config = SimConfig::load(config_path)?;
    config.validate()?;
    for w in config.unread_table_notes("simulate") {
        eprintln!("note: {w}");
    }
    let out = simulate(&config, focus_override, dose_override)?;
    if let Some(out_path) = output {
        if out_path.extension().is_some_and(|ext| ext == "png") {
            use highuvlith_core::io::image_export::{save_png, Colormap};
            save_png(&out.image, out_path, Colormap::Inferno)
                .map_err(|e| anyhow::anyhow!("Failed to save PNG: {}", e))?;
        } else {
            std::fs::write(out_path, serde_json::to_string_pretty(&out.summary)?)?;
        }
        eprintln!("Output written to {}", out_path.display());
    }
    Ok(())
}
