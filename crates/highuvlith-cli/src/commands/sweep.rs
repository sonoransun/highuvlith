//! `highuvlith sweep` — dose × focus process window (Bossung curves, ED window).

use std::path::Path;

use highuvlith_core::aerial::AerialImageEngine;
use highuvlith_core::metrics::{self, FeatureTone};
use highuvlith_core::process::{ProcessRectangle, ProcessWindow, ThresholdResist};
use highuvlith_core::source::LithographySource;
use highuvlith_core::types::Grid2D;
use indicatif::{ProgressBar, ProgressStyle};

use super::simulate::{measurement_orientation, measurement_profile, tone_label};
use crate::config::SimConfig;

/// Parse `start,stop,steps` into `steps` evenly spaced values (inclusive).
pub fn parse_range(s: &str) -> anyhow::Result<Vec<f64>> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 3 {
        anyhow::bail!("Range must be 'start,stop,steps' (e.g., '-200,200,21')");
    }
    let start: f64 = parts[0].trim().parse()?;
    let stop: f64 = parts[1].trim().parse()?;
    let steps: usize = parts[2].trim().parse()?;
    if !(start.is_finite() && stop.is_finite()) {
        anyhow::bail!("Range start and stop must be finite numbers");
    }
    if steps > 1 && start > stop {
        anyhow::bail!(
            "Invalid range: start ({}) > stop ({}) with steps = {}",
            start,
            stop,
            steps
        );
    }
    if steps < 2 {
        return Ok(vec![start]);
    }
    let step = (stop - start) / (steps - 1) as f64;
    Ok((0..steps).map(|i| start + i as f64 * step).collect())
}

/// JSON form of a process-window rectangle.
fn rectangle_json(r: &ProcessRectangle) -> serde_json::Value {
    serde_json::json!({
        "focus_min_nm": r.focus_min_nm,
        "focus_max_nm": r.focus_max_nm,
        "dose_min_mj_cm2": r.dose_min_mj_cm2,
        "dose_max_mj_cm2": r.dose_max_mj_cm2,
        "dof_nm": r.dof_nm(),
        "exposure_latitude_pct": r.exposure_latitude_pct(),
    })
}

/// A finite number, or JSON `null` (NaN CDs mean "does not print").
fn finite_or_null(v: f64) -> serde_json::Value {
    if v.is_finite() {
        serde_json::json!(v)
    } else {
        serde_json::Value::Null
    }
}

/// The sweep on a validated config; returns the JSON summary.
pub fn sweep(
    config: &SimConfig,
    focuses: &[f64],
    dose_range: Option<&[f64]>,
) -> anyhow::Result<serde_json::Value> {
    let source = config.to_source()?;
    let optics = config.to_optics()?;
    let mask_cfg = config.mask_cfg()?;
    let mask = config.to_mask()?;
    let grid = config.to_grid()?;
    for w in config.optics_warnings() {
        eprintln!("warning: {w}");
    }
    let nominal_dose = config.process.dose_mj_cm2;
    let doses: Vec<f64> = match dose_range {
        Some(d) => d.to_vec(),
        None => vec![nominal_dose],
    };

    // Dose enters through a constant-threshold resist anchored at the
    // nominal dose: E_th = threshold · d_nominal.
    let threshold = config.process.threshold();
    let resist = ThresholdResist::from_nominal(threshold, nominal_dose)?;
    let tone = FeatureTone::of_mask(&mask);
    let cd_target = config
        .process
        .cd_target_nm
        .or(mask_cfg.drawn_width_nm())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "sweep needs a target CD: set [process] cd_target_nm (a free-form [mask] has no \
                 drawn line or hole width)"
            )
        })?;
    let tolerance = config.process.cd_tolerance_pct();
    let spectrum = config.imaging.spectrum();

    eprintln!(
        "Sweep: {} focuses × {} doses ({} aerial images, spectrum {spectrum}; constant-threshold \
         resist, E_th = {:.3} mJ/cm² = {} × {} mJ/cm²)",
        focuses.len(),
        doses.len(),
        focuses.len(),
        resist.dose_to_clear_mj_cm2,
        threshold,
        nominal_dose
    );

    let engine = AerialImageEngine::with_settings(
        &source,
        optics.as_ref(),
        grid.clone(),
        config.to_imaging_settings()?,
    )?;
    eprintln!("Engine: {} SOCS kernels", engine.num_kernels());

    let start = std::time::Instant::now();
    // One aerial image per focus; every dose is evaluated on its centre
    // cross-section.
    let images: Vec<Grid2D<f64>> = match spectrum {
        // One mask spectrum, focus planes in parallel.
        "monochromatic" => engine.compute_through_focus(&mask, focuses),
        _ => {
            let pb = ProgressBar::new(focuses.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(
                        "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} \
                         ({eta})",
                    )
                    .unwrap()
                    .progress_chars("#>-"),
            );
            let mut out = Vec::with_capacity(focuses.len());
            for &focus in focuses {
                out.push(if spectrum == "per_wavelength" {
                    engine.compute_multiwavelength(&mask, focus)?
                } else {
                    engine.compute_polychromatic(&mask, focus, &source, optics.as_ref())
                });
                pb.inc(1);
            }
            pb.finish_with_message("Sweep complete");
            out
        }
    };
    let orientation = measurement_orientation(config)?;
    let mut contrasts = Vec::with_capacity(focuses.len());
    let mut profiles = Vec::with_capacity(focuses.len());
    let mut x_nm = Vec::new();
    for aerial in &images {
        contrasts.push(metrics::image_contrast(&aerial.data));
        let (x, profile) = measurement_profile(aerial, orientation);
        x_nm = x;
        profiles.push(profile);
    }

    let pw = ProcessWindow::from_profiles(
        x_nm, focuses, profiles, &doses, resist, tone, cd_target, tolerance,
    )?;

    let elapsed = start.elapsed();
    eprintln!(
        "Total time: {:.2?} ({:.1?} per focus)",
        elapsed,
        elapsed / focuses.len().max(1) as u32
    );

    let mut results = Vec::with_capacity(focuses.len() * doses.len());
    for (i, &dose) in doses.iter().enumerate() {
        for (j, &focus) in focuses.iter().enumerate() {
            results.push(serde_json::json!({
                "dose_mj_cm2": dose,
                "focus_nm": focus,
                "intensity_threshold": resist.intensity_threshold(dose),
                "contrast": contrasts[j],
                "cd_nm": finite_or_null(pw.cd_matrix[[i, j]]),
            }));
        }
    }

    // ED-window summary. The dose axis is continuous (every dose is
    // evaluated on the stored images), so it is meaningful even for a
    // single swept dose; the iso-focal dose needs a dose range.
    let best_focus = pw.best_focus();
    let dose_to_size = pw.dose_to_size();
    let dof = pw.depth_of_focus();
    let el = pw.exposure_latitude();
    let dof5 = pw.dof_at_el(5.0);
    let iso = if doses.len() > 1 {
        pw.iso_focal_dose()
    } else {
        None
    };
    eprintln!(
        "Process window (target CD {:.1} nm ± {}%, {} feature):",
        cd_target,
        tolerance,
        tone_label(tone)
    );
    eprintln!("  Best focus:     {:.1} nm", best_focus);
    match dose_to_size {
        Some(d) => eprintln!("  Dose-to-size:   {:.2} mJ/cm²", d),
        None => eprintln!("  Dose-to-size:   n/a (target CD not printable at best focus)"),
    }
    eprintln!("  DOF (at size):  {:.1} nm", dof);
    eprintln!("  EL (best focus): {:.1} %", el);
    match &dof5 {
        Some(r) => eprintln!(
            "  DOF @ 5% EL:    {:.1} nm (focus {:.1}…{:.1} nm, dose {:.2}…{:.2} mJ/cm²)",
            r.dof_nm(),
            r.focus_min_nm,
            r.focus_max_nm,
            r.dose_min_mj_cm2,
            r.dose_max_mj_cm2
        ),
        None => eprintln!("  DOF @ 5% EL:    n/a (no rectangle reaches 5% EL)"),
    }
    if let Some(d) = iso {
        eprintln!("  Iso-focal dose: {:.2} mJ/cm²", d);
    }

    let pitch = match mask_cfg.pattern() {
        "line_space" => Some(mask_cfg.pitch_nm.unwrap_or(180.0)),
        "contact_holes" => mask_cfg.pitch_nm,
        _ => None,
    };
    Ok(serde_json::json!({
        "config": {
            "wavelength_nm": source.wavelength_nm(),
            "source_type": source.kind_label(),
            "na": optics.na(),
            "pattern": mask_cfg.pattern(),
            "mask": mask.summary(),
            "cd_nm": mask_cfg.drawn_width_nm(),
            "pitch_nm": pitch,
            "grid_size": grid.size,
            "pixel_nm": grid.pixel_nm,
            "spectrum": spectrum,
        },
        "focuses": focuses,
        "doses": doses,
        "results": results,
        "process_window": {
            "resist_model": "constant_threshold",
            "nominal_dose_mj_cm2": nominal_dose,
            "intensity_threshold_at_nominal": threshold,
            "dose_to_clear_mj_cm2": resist.dose_to_clear_mj_cm2,
            "tone": tone_label(tone),
            "cd_target_nm": cd_target,
            "cd_tolerance_pct": tolerance,
            "best_focus_nm": best_focus,
            "dose_to_size_mj_cm2": dose_to_size,
            "depth_of_focus_nm": dof,
            "exposure_latitude_pct": el,
            "dof_at_5pct_el": dof5.as_ref().map(rectangle_json),
            "iso_focal_dose_mj_cm2": iso,
            "dose_limits": pw
                .dose_limits
                .iter()
                .map(|l| l.map(|(a, b)| [a, b]))
                .collect::<Vec<_>>(),
            "el_vs_dof": pw
                .el_vs_dof(21)
                .into_iter()
                .map(|(d, e)| [d, e])
                .collect::<Vec<_>>(),
        },
    }))
}

pub fn run(
    config_path: &Path,
    output: Option<&Path>,
    focus_range: &str,
    dose_range: Option<&str>,
) -> anyhow::Result<()> {
    let config = SimConfig::load(config_path)?;
    config.validate()?;
    for w in config.unread_table_notes("sweep") {
        eprintln!("note: {w}");
    }
    let focuses = parse_range(focus_range)?;
    let doses = dose_range.map(parse_range).transpose()?;
    let summary = sweep(&config, &focuses, doses.as_deref())?;
    if let Some(out_path) = output {
        std::fs::write(out_path, serde_json::to_string_pretty(&summary)?)?;
        eprintln!("Output written to {}", out_path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_range_valid() {
        let vals = parse_range("-200,200,5").unwrap();
        assert_eq!(vals.len(), 5);
        assert!((vals[0] - (-200.0)).abs() < 1e-10);
        assert!((vals[4] - 200.0).abs() < 1e-10);
        // Whitespace around the numbers is accepted.
        assert_eq!(parse_range(" -1, 1 , 3").unwrap(), vec![-1.0, 0.0, 1.0]);
    }

    #[test]
    fn test_parse_range_reversed_error() {
        let result = parse_range("200,-200,5");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("start"), "Error should mention start: {}", msg);
    }

    #[test]
    fn test_parse_range_single_step() {
        let vals = parse_range("100,200,1").unwrap();
        assert_eq!(vals.len(), 1);
        assert!((vals[0] - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_range_bad_format() {
        assert!(parse_range("1,2").is_err());
        assert!(parse_range("1,2,3,4").is_err());
        assert!(parse_range("nan,2,3").is_err());
    }

    const SWEEP_TOML: &str = "[source]\nwavelength_nm = 157.63\nsigma = 0.5\nbandwidth_pm = 0.01\n\
         [optics]\nna = 0.75\n\
         [mask]\ncd_nm = 150.0\npitch_nm = 300.0\n\
         [grid]\nsize = 64\npixel_nm = 4.6875\n\
         [process]\ndose_mj_cm2 = 30.0\n";

    #[test]
    fn test_sweep_writes_dose_aware_json() {
        // 1:1 lines on a 300 nm pitch in a one-period field (first order
        // inside NA/λ), three doses around the 30 mJ/cm² nominal.
        let dir = std::env::temp_dir().join(format!("huv_sweep_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = dir.join("sweep.toml");
        std::fs::write(&cfg, SWEEP_TOML).unwrap();
        let out = dir.join("sweep.json");
        run(&cfg, Some(&out), "-100,100,3", Some("25,35,3")).unwrap();
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        let results = json["results"].as_array().unwrap();
        assert_eq!(results.len(), 9);
        let pw = &json["process_window"];
        assert_eq!(pw["resist_model"], "constant_threshold");
        assert!((pw["dose_to_clear_mj_cm2"].as_f64().unwrap() - 9.0).abs() < 1e-12);
        assert_eq!(pw["tone"], "dark");
        assert!(pw["best_focus_nm"].as_f64().unwrap().is_finite());
        // Dose-aware: at focus 0 the dark line narrows with dose.
        let cd_at = |dose: f64| {
            results
                .iter()
                .find(|r| {
                    r["focus_nm"].as_f64() == Some(0.0) && r["dose_mj_cm2"].as_f64() == Some(dose)
                })
                .and_then(|r| r["cd_nm"].as_f64())
                .unwrap()
        };
        assert!(cd_at(25.0) > cd_at(30.0) && cd_at(30.0) > cd_at(35.0));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// [process] threshold / cd_target_nm / cd_tolerance_pct drive the
    /// process window, and horizontal lines are measured along y: rotating
    /// the pattern leaves every CD unchanged.
    #[test]
    fn test_sweep_process_keys_and_horizontal_lines() {
        let focuses = [-60.0, 0.0, 60.0];
        let doses = [27.0, 30.0, 33.0];
        let run_with = |extra_mask: &str, extra_process: &str| {
            let text = SWEEP_TOML
                .replace(
                    "pitch_nm = 300.0\n",
                    &format!("pitch_nm = 300.0\n{extra_mask}"),
                )
                .replace(
                    "dose_mj_cm2 = 30.0\n",
                    &format!("dose_mj_cm2 = 30.0\n{extra_process}"),
                );
            let cfg = SimConfig::from_toml_str(&text).unwrap();
            cfg.validate().unwrap();
            sweep(&cfg, &focuses, Some(&doses)).unwrap()
        };
        let base = run_with(
            "",
            "threshold = 0.35\ncd_target_nm = 140.0\ncd_tolerance_pct = 5.0\n",
        );
        let pw = &base["process_window"];
        assert_eq!(pw["cd_target_nm"], 140.0);
        assert_eq!(pw["cd_tolerance_pct"], 5.0);
        assert!((pw["dose_to_clear_mj_cm2"].as_f64().unwrap() - 10.5).abs() < 1e-12);
        let rotated = run_with(
            "orientation = \"horizontal\"\n",
            "threshold = 0.35\ncd_target_nm = 140.0\ncd_tolerance_pct = 5.0\n",
        );
        for (a, b) in base["results"]
            .as_array()
            .unwrap()
            .iter()
            .zip(rotated["results"].as_array().unwrap())
        {
            let (ca, cb) = (a["cd_nm"].as_f64().unwrap(), b["cd_nm"].as_f64().unwrap());
            assert!((ca - cb).abs() < 1e-6, "vertical {ca} vs horizontal {cb}");
        }
    }
}
