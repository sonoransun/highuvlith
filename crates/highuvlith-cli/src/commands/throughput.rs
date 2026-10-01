//! `highuvlith throughput` — dose-limited wafers per hour for the configured
//! `[source]`.
//!
//! The scanner model is `highuvlith_core::source_models::throughput` (🔶
//! Simplified): the source's usable power (`average_power_w`, in-band at
//! intermediate focus for plasma sources) times one optics-train
//! transmission and the mask efficiency delivers the dose; every field is
//! scanned at the dose-limited speed (optionally capped by a stage limit)
//! and the mechanics are two lumped overheads. The scanner presets are
//! ILLUSTRATIVE assumptions, not vendor data. Optional `[throughput]` keys
//! override the preset (strict keys; see `docs/configuration.md`).

use std::path::Path;

use serde::{Deserialize, Serialize};

use highuvlith_core::source::{LithographySource, SourceKind};
use highuvlith_core::source_models::throughput::{
    photons_per_square, relative_shot_noise, wafer_throughput, ThroughputParams, ThroughputResult,
};

use super::sources::sig;
use crate::config::SimConfig;
use crate::keys::{self, UnknownKeyHint};

/// Scanner presets selectable with `[throughput] preset`.
pub const THROUGHPUT_PRESETS: &[&str] = &["euv_hvm", "beuv_la_b", "refractive"];

/// Wavelength band (nm, inclusive) in which each preset's optics train is a
/// real projection-lithography train: refractive CaF2/MgF2/LiF optics
/// transmit down to ~105 nm; Mo/Si multilayers reflect from the Si L-edge
/// (12.4 nm) to ~15 nm; La/B multilayers work just above the B K-edge
/// (6.6 nm). The Python `api.wafer_throughput` uses the same bands.
pub fn preset_band_nm(preset: &str) -> Option<(f64, f64)> {
    match preset {
        "refractive" => Some((100.0, f64::INFINITY)),
        "euv_hvm" => Some((12.4, 15.0)),
        "beuv_la_b" => Some((6.0, 7.5)),
        _ => None,
    }
}

/// The preset whose band contains `wavelength_nm`, if any.
pub fn preset_for_wavelength(wavelength_nm: f64) -> Option<&'static str> {
    THROUGHPUT_PRESETS
        .iter()
        .copied()
        .find(|p| preset_band_nm(p).is_some_and(|(lo, hi)| (lo..=hi).contains(&wavelength_nm)))
}

/// Every `[throughput]` key.
pub const THROUGHPUT_KEYS: &[&str] = &[
    "preset",
    "optics_transmission",
    "n_mirrors",
    "mirror_reflectivity",
    "mask_efficiency",
    "dose_mj_cm2",
    "wafer_diameter_mm",
    "field_width_mm",
    "field_height_mm",
    "slit_height_mm",
    "fields_per_wafer",
    "max_scan_speed_mm_s",
    "field_overhead_s",
    "wafer_overhead_s",
    "cd_nm",
];

/// The optional `[throughput]` table (every key overrides the preset).
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ThroughputConfig {
    /// `"euv_hvm"`, `"beuv_la_b"` or `"refractive"`; default chosen from
    /// the source wavelength (≥ 100 nm refractive, 12.4–15 nm euv_hvm,
    /// 6–7.5 nm beuv_la_b; any other wavelength is an error unless the
    /// preset or the optics transmission is given).
    pub preset: Option<String>,
    /// Illuminator + projection transmission, source hand-off → wafer.
    pub optics_transmission: Option<f64>,
    /// Mirror count of a mirror train (with `mirror_reflectivity`;
    /// transmission = R^n). Alternative to `optics_transmission`.
    pub n_mirrors: Option<u32>,
    /// Per-mirror reflectivity of the mirror train.
    pub mirror_reflectivity: Option<f64>,
    /// Mask reflectance (EUV) / transmittance (VUV/DUV) in clear areas.
    pub mask_efficiency: Option<f64>,
    /// Resist dose-to-size (mJ/cm²).
    pub dose_mj_cm2: Option<f64>,
    /// Wafer diameter (mm).
    pub wafer_diameter_mm: Option<f64>,
    /// Exposure-field width across the scan (mm).
    pub field_width_mm: Option<f64>,
    /// Exposure-field length along the scan (mm).
    pub field_height_mm: Option<f64>,
    /// Illuminated slit height along the scan (mm).
    pub slit_height_mm: Option<f64>,
    /// Fields per wafer (default: counted on the wafer).
    pub fields_per_wafer: Option<usize>,
    /// Wafer-stage scan-speed limit (mm/s; default: none).
    pub max_scan_speed_mm_s: Option<f64>,
    /// Per-field overhead (s).
    pub field_overhead_s: Option<f64>,
    /// Per-wafer overhead (s).
    pub wafer_overhead_s: Option<f64>,
    /// Side of the photon-statistics square (nm); default: the drawn
    /// `[mask]` line / hole width.
    pub cd_nm: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
struct ThroughputFile {
    #[serde(default)]
    throughput: Option<ThroughputConfig>,
}

fn throughput_hint(
    path: &[String],
    _raw: &toml::Table,
    expected: &[String],
) -> Option<UnknownKeyHint> {
    (path == ["throughput"] && expected.iter().any(|e| e == "n_mirrors")).then(|| UnknownKeyHint {
        label: "[throughput]".to_string(),
        keys: THROUGHPUT_KEYS.to_vec(),
    })
}

impl ThroughputConfig {
    /// Parse the optional `[throughput]` table of a config file (strict keys).
    pub fn from_toml_str(text: &str) -> anyhow::Result<Self> {
        Ok(keys::parse_toml::<ThroughputFile>(text, &throughput_hint)?
            .throughput
            .unwrap_or_default())
    }
}

/// A fraction in (0, 1].
fn fraction(key: &str, v: f64) -> anyhow::Result<f64> {
    if !(v.is_finite() && v > 0.0 && v <= 1.0) {
        anyhow::bail!("[throughput] {key} must be in (0, 1], got {v}");
    }
    Ok(v)
}

/// A finite value > 0.
fn positive(key: &str, v: f64) -> anyhow::Result<f64> {
    if !(v.is_finite() && v > 0.0) {
        anyhow::bail!("[throughput] {key} must be finite and > 0, got {v}");
    }
    Ok(v)
}

/// A finite value >= 0.
fn non_negative(key: &str, v: f64) -> anyhow::Result<f64> {
    if !(v.is_finite() && v >= 0.0) {
        anyhow::bail!("[throughput] {key} must be finite and >= 0, got {v}");
    }
    Ok(v)
}

/// Resolved inputs of one throughput evaluation.
#[derive(Debug, Clone)]
pub struct ThroughputSetup {
    /// Scanner preset the parameters start from.
    pub preset: String,
    /// `true` when the preset was chosen from the wavelength.
    pub preset_from_wavelength: bool,
    /// Parameters after the `[throughput]` overrides.
    pub params: ThroughputParams,
    /// Where the dose came from (`--dose`, `[throughput]`, `[process]`, default).
    pub dose_from: &'static str,
    /// Side of the photon-statistics square (nm), if known.
    pub cd_nm: Option<f64>,
    /// Caveats about the preset at this wavelength.
    pub notes: Vec<String>,
}

/// Resolve the preset, the overrides and the dose for a source wavelength.
pub fn setup(
    config: &SimConfig,
    table: &ThroughputConfig,
    dose_override: Option<f64>,
    wavelength_nm: f64,
) -> anyhow::Result<ThroughputSetup> {
    let (dose, dose_from) = match (dose_override, table.dose_mj_cm2) {
        (Some(d), _) => (d, "--dose"),
        (None, Some(d)) => (d, "[throughput] dose_mj_cm2"),
        (None, None) if config.has_table("process") => {
            (config.process.dose_mj_cm2, "[process] dose_mj_cm2")
        }
        (None, None) => (30.0, "default 30 mJ/cm²"),
    };
    if !(dose.is_finite() && dose > 0.0) {
        anyhow::bail!("the throughput dose ({dose_from}) must be finite and > 0, got {dose}");
    }
    let preset_from_wavelength = table.preset.is_none();
    let optics_given = table.optics_transmission.is_some()
        || table.n_mirrors.is_some()
        || table.mirror_reflectivity.is_some();
    let preset = match (
        table.preset.as_deref(),
        preset_for_wavelength(wavelength_nm),
    ) {
        (Some(p), _) => p.to_string(),
        (None, Some(p)) => p.to_string(),
        // No projection optics preset at this wavelength: with an explicit
        // optics train the nearest preset only supplies mask / field values.
        (None, None) if optics_given => if wavelength_nm >= 100.0 {
            "refractive"
        } else if wavelength_nm < 10.0 {
            "beuv_la_b"
        } else {
            "euv_hvm"
        }
        .to_string(),
        (None, None) => anyhow::bail!(
            "no projection-lithography optics preset covers {wavelength_nm:.4} nm (refractive \
             >= 100 nm, euv_hvm 12.4–15 nm, beuv_la_b 6–7.5 nm){}; set [throughput] preset \
             and/or optics_transmission for a hypothetical optics train",
            if wavelength_nm < 3.0 {
                " — X-ray sources print by proximity / LIGA, not projection"
            } else {
                ""
            }
        ),
    };
    let mut params = match preset.as_str() {
        "euv_hvm" => ThroughputParams::euv_hvm_like(dose),
        "beuv_la_b" => ThroughputParams::beuv_la_b_like(dose),
        "refractive" => ThroughputParams::refractive_like(dose),
        other => anyhow::bail!(
            "unknown [throughput] preset '{}' (expected one of: {})",
            other,
            THROUGHPUT_PRESETS.join(", ")
        ),
    };
    let explicit_optics = match (
        table.optics_transmission,
        table.n_mirrors,
        table.mirror_reflectivity,
    ) {
        (Some(_), Some(_), _) | (Some(_), _, Some(_)) => anyhow::bail!(
            "[throughput] give the optics transmission once: optics_transmission OR n_mirrors + \
             mirror_reflectivity (transmission = R^n)"
        ),
        (Some(t), None, None) => {
            params.optics_transmission = fraction("optics_transmission", t)?;
            true
        }
        (None, Some(n), Some(r)) => {
            if n == 0 {
                anyhow::bail!(
                    "[throughput] n_mirrors must be >= 1 (use optics_transmission = 1.0 for no \
                     optics)"
                );
            }
            params.optics_transmission =
                ThroughputParams::mirror_train(n, fraction("mirror_reflectivity", r)?);
            true
        }
        (None, Some(_), None) | (None, None, Some(_)) => anyhow::bail!(
            "[throughput] n_mirrors and mirror_reflectivity go together (transmission = R^n); \
             or give optics_transmission"
        ),
        (None, None, None) => false,
    };
    if let Some(v) = table.mask_efficiency {
        params.mask_efficiency = fraction("mask_efficiency", v)?;
    }
    if let Some(v) = table.wafer_diameter_mm {
        params.wafer_diameter_mm = positive("wafer_diameter_mm", v)?;
    }
    if let Some(v) = table.field_width_mm {
        params.field_width_mm = positive("field_width_mm", v)?;
    }
    if let Some(v) = table.field_height_mm {
        params.field_height_mm = positive("field_height_mm", v)?;
    }
    if let Some(v) = table.slit_height_mm {
        params.slit_height_mm = positive("slit_height_mm", v)?;
    }
    if let Some(n) = table.fields_per_wafer {
        if n == 0 {
            anyhow::bail!("[throughput] fields_per_wafer must be >= 1");
        }
        params.fields_per_wafer = Some(n);
    }
    if let Some(v) = table.max_scan_speed_mm_s {
        params.max_scan_speed_mm_s = Some(positive("max_scan_speed_mm_s", v)?);
    }
    if let Some(v) = table.field_overhead_s {
        params.field_overhead_s = non_negative("field_overhead_s", v)?;
    }
    if let Some(v) = table.wafer_overhead_s {
        params.wafer_overhead_s = non_negative("wafer_overhead_s", v)?;
    }
    let cd_nm = match table.cd_nm {
        Some(cd) => Some(positive("cd_nm", cd)?),
        None => config.mask.as_ref().and_then(|m| m.drawn_width_nm()),
    };

    let mut notes = Vec::new();
    if !explicit_optics {
        if let Some((lo, hi)) = preset_band_nm(&preset) {
            if !(lo..=hi).contains(&wavelength_nm) {
                notes.push(format!(
                    "hypothetical: {wavelength_nm:.4} nm is outside the {preset} optics band \
                     ({lo}–{hi} nm), so its optics train is a placeholder — set [throughput] \
                     optics_transmission (or n_mirrors + mirror_reflectivity) for optics that \
                     work there"
                ));
            }
        }
    } else if preset_from_wavelength && preset_for_wavelength(wavelength_nm).is_none() {
        notes.push(format!(
            "no optics preset covers {wavelength_nm:.4} nm: the given optics transmission is \
             used with the {preset} preset's mask and field parameters"
        ));
    }
    Ok(ThroughputSetup {
        preset,
        preset_from_wavelength,
        params,
        dose_from,
        cd_nm,
        notes,
    })
}

/// One evaluated throughput run.
pub struct ThroughputReport {
    /// `[source] type` of the evaluated source.
    pub source_type: &'static str,
    /// Source wavelength (nm).
    pub wavelength_nm: f64,
    /// Resolved preset, parameters and dose.
    pub setup: ThroughputSetup,
    /// Model output.
    pub result: ThroughputResult,
    /// `(cd_nm, photons per cd², relative shot noise)` when a square is known.
    pub photon_stats: Option<(f64, f64, f64)>,
    /// `(per-pulse rms, pulses per point, per-exposure rms)` when known.
    pub jitter: Option<(f64, f64, f64)>,
}

/// Evaluate the throughput of the config's source.
pub fn evaluate(
    config: &SimConfig,
    table: &ThroughputConfig,
    dose_override: Option<f64>,
) -> anyhow::Result<ThroughputReport> {
    let source: SourceKind = config.to_source()?;
    let wavelength = source.wavelength_nm();
    let setup = setup(config, table, dose_override, wavelength)?;
    let params = &setup.params;
    let result = wafer_throughput(&source, params).ok_or_else(|| {
        anyhow::anyhow!(
            "the [source] (type = \"{}\") reports no average power — e.g. a CW mercury lamp \
             (power not modeled) or a bending-magnet fan (flux per mrad only); the throughput \
             model needs a source power",
            source.kind_label()
        )
    })?;
    let photon_stats = setup.cd_nm.map(|cd| {
        (
            cd,
            photons_per_square(params.dose_mj_cm2, wavelength, cd),
            relative_shot_noise(params.dose_mj_cm2, wavelength, cd),
        )
    });
    let rms = source.shot_to_shot_rms();
    let jitter = match result.pulses_per_point {
        Some(n) if n > 0.0 && rms > 0.0 => Some((rms, n, rms / n.sqrt())),
        _ => None,
    };
    Ok(ThroughputReport {
        source_type: source.kind_label(),
        wavelength_nm: wavelength,
        setup,
        result,
        photon_stats,
        jitter,
    })
}

impl ThroughputReport {
    /// Fraction of the source power that reaches the wafer.
    fn transmission(&self) -> f64 {
        self.setup.params.optics_transmission * self.setup.params.mask_efficiency
    }

    /// Human-readable summary (printed on stderr).
    pub fn summary(&self) -> String {
        let (params, result, setup) = (&self.setup.params, &self.result, &self.setup);
        let mut s = String::new();
        s.push_str(
            "Throughput — dose-limited scanner model (🔶 simplified; preset scanner numbers are \
             illustrative assumptions):\n",
        );
        s.push_str(&format!(
            "  Source:            {}, {} nm, {} W usable power into the illuminator\n",
            self.source_type,
            sig(self.wavelength_nm, 6),
            sig(result.source_power_w, 4)
        ));
        s.push_str(&format!(
            "  Scanner preset:    {}{}\n",
            setup.preset,
            if setup.preset_from_wavelength {
                " (chosen from the wavelength)"
            } else {
                ""
            }
        ));
        s.push_str(&format!(
            "  Optics × mask:     {:.4} × {:.4} = {:.3} % of the source power\n",
            params.optics_transmission,
            params.mask_efficiency,
            100.0 * self.transmission()
        ));
        s.push_str(&format!(
            "  Power at wafer:    {} W\n",
            sig(result.power_at_wafer_w, 4)
        ));
        s.push_str(&format!(
            "  Dose:              {:.2} mJ/cm² ({})\n",
            params.dose_mj_cm2, setup.dose_from
        ));
        s.push_str(&format!(
            "  Fields / wafer:    {} ({} × {} mm on a {} mm wafer; {:.1} cm² exposed)\n",
            result.fields_per_wafer,
            params.field_width_mm,
            params.field_height_mm,
            params.wafer_diameter_mm,
            result.exposed_area_cm2
        ));
        s.push_str(&format!(
            "  Scan speed:        {} mm/s{}\n",
            sig(result.scan_speed_mm_s, 4),
            if result.stage_limited {
                format!(
                    " (stage-limited; the dose allows {} mm/s)",
                    sig(result.dose_limited_scan_speed_mm_s, 4)
                )
            } else {
                " (dose-limited)".to_string()
            }
        ));
        s.push_str(&format!(
            "  Time per wafer:    {} s scanning, {} s total ({} s/field, {} s/wafer \
             overheads)\n",
            sig(result.exposure_time_per_wafer_s, 4),
            sig(result.total_time_per_wafer_s, 4),
            params.field_overhead_s,
            params.wafer_overhead_s
        ));
        s.push_str(&format!(
            "  Wafers per hour:   {}\n",
            sig(result.wafers_per_hour, 4)
        ));
        match result.pulses_per_point {
            Some(n) => s.push_str(&format!("  Pulses per point:  {}\n", sig(n, 4))),
            None => s.push_str("  Pulses per point:  n/a (CW source or unknown repetition rate)\n"),
        }
        if let Some((rms, n, per_exposure)) = self.jitter {
            s.push_str(&format!(
                "  Dose jitter:       {:.3} % rms per exposure point ({:.2} % per pulse / √{:.1})\n",
                100.0 * per_exposure,
                100.0 * rms,
                n
            ));
        }
        if let Some((cd, n, noise)) = self.photon_stats {
            s.push_str(&format!(
                "  Photons per ({cd} nm)²: {} (relative shot noise {:.3} %)\n",
                sig(n, 4),
                100.0 * noise
            ));
        }
        for note in &setup.notes {
            s.push_str(&format!("  note: {note}\n"));
        }
        s
    }

    /// The report as JSON (stdout with `--json`, `--output` file).
    pub fn to_json(&self) -> serde_json::Value {
        let setup = &self.setup;
        serde_json::json!({
            "model": "dose-limited scanner model (simplified; illustrative scanner presets)",
            "source_type": self.source_type,
            "wavelength_nm": self.wavelength_nm,
            "preset": setup.preset,
            "preset_from_wavelength": setup.preset_from_wavelength,
            "dose_mj_cm2": setup.params.dose_mj_cm2,
            "dose_from": setup.dose_from,
            "optics_and_mask_transmission": self.transmission(),
            "params": &setup.params,
            "result": &self.result,
            "photon_statistics": self.photon_stats.map(|(cd, n, noise)| serde_json::json!({
                "cd_nm": cd,
                "photons_per_square": n,
                "relative_shot_noise": noise,
            })),
            "dose_jitter": self.jitter.map(|(rms, n, per)| serde_json::json!({
                "shot_to_shot_rms": rms,
                "pulses_per_point": n,
                "per_exposure_rms": per,
            })),
            "notes": setup.notes,
        })
    }
}

pub fn run(
    config_path: &Path,
    output: Option<&Path>,
    dose: Option<f64>,
    json: bool,
) -> anyhow::Result<()> {
    let text = std::fs::read_to_string(config_path)
        .map_err(|e| anyhow::anyhow!("reading {}: {e}", config_path.display()))?;
    let with_path = |e: anyhow::Error| anyhow::anyhow!("{}: {e}", config_path.display());
    let config = SimConfig::from_toml_str(&text).map_err(with_path)?;
    config.validate()?;
    let table = ThroughputConfig::from_toml_str(&text).map_err(with_path)?;
    let report = evaluate(&config, &table, dose)?;
    eprint!("{}", report.summary());
    if json {
        println!("{}", serde_json::to_string_pretty(&report.to_json())?);
    }
    if let Some(out_path) = output {
        std::fs::write(out_path, serde_json::to_string_pretty(&report.to_json())?)?;
        eprintln!("Output written to {}", out_path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(text: &str, dose: Option<f64>) -> anyhow::Result<ThroughputReport> {
        let config = SimConfig::from_toml_str(text)?;
        config.validate()?;
        let table = ThroughputConfig::from_toml_str(text)?;
        evaluate(&config, &table, dose)
    }

    /// Independent evaluation of the documented model equations.
    fn expected_wph(p: &ThroughputParams, power_w: f64, fields: usize) -> f64 {
        let p_wafer = power_w * p.optics_transmission * p.mask_efficiency;
        let dose_j_mm2 = p.dose_mj_cm2 * 1e-3 / 100.0;
        let v = p_wafer / (dose_j_mm2 * p.field_width_mm);
        let v = p.max_scan_speed_mm_s.map_or(v, |m| v.min(m));
        let t = fields as f64 * ((p.field_height_mm + p.slit_height_mm) / v + p.field_overhead_s)
            + p.wafer_overhead_s;
        3600.0 / t
    }

    /// The WP-C anchors: Sn LPP (250 W at IF) on the EUV HVM-like column.
    #[test]
    fn test_lpp_sn_hvm_anchor() {
        let r = eval("[source]\ntype = \"lpp\"\n", None).unwrap();
        assert_eq!(r.setup.preset, "euv_hvm");
        assert!(r.setup.preset_from_wavelength);
        assert_eq!(r.result.fields_per_wafer, 84);
        assert!((r.result.power_at_wafer_w - 4.5902).abs() < 1e-3);
        assert!(
            (r.result.wafers_per_hour - 153.87).abs() < 0.05,
            "{}",
            r.result.wafers_per_hour
        );
        assert!((r.result.pulses_per_point.unwrap() - 169.9).abs() < 0.2);
        assert!(r.setup.notes.is_empty());
        // --dose 20 mJ/cm²: ≈ 166 wafers/hour (NXE:3400B spec ≥ 125).
        let r20 = eval("[source]\ntype = \"lpp\"\n", Some(20.0)).unwrap();
        assert!((r20.result.wafers_per_hour - 165.7).abs() < 0.1);
        assert_eq!(r20.setup.dose_from, "--dose");
        assert!(
            (r20.result.wafers_per_hour
                - expected_wph(&r20.setup.params, 250.0, r20.result.fields_per_wafer))
            .abs()
                < 1e-9
        );
    }

    /// ArF (90 W) on the refractive preset (chosen from 193 nm): ≈ 185 WPH,
    /// ≈ 15 pulses per exposure point (WP-C notes).
    #[test]
    fn test_arf_refractive_anchor() {
        let r = eval("[source]\ntype = \"vuv\"\npreset = \"arf\"\n", None).unwrap();
        assert_eq!(r.setup.preset, "refractive");
        assert!(
            (r.result.wafers_per_hour - 184.6).abs() < 0.1,
            "{}",
            r.result.wafers_per_hour
        );
        assert!((r.result.pulses_per_point.unwrap() - 15.4).abs() < 0.1);
    }

    #[test]
    fn test_explicit_keys_override_the_preset() {
        let text = "[source]\ntype = \"lpp\"\n[throughput]\nn_mirrors = 12\n\
                    mirror_reflectivity = 0.7\nmask_efficiency = 0.6\nfields_per_wafer = 100\n\
                    wafer_overhead_s = 0.0\nfield_overhead_s = 0.05\nslit_height_mm = 1.0\n\
                    max_scan_speed_mm_s = 400.0\ndose_mj_cm2 = 40.0\n";
        let r = eval(text, None).unwrap();
        let p = &r.setup.params;
        assert!((p.optics_transmission - 0.7f64.powi(12)).abs() < 1e-15);
        assert_eq!(p.mask_efficiency, 0.6);
        assert_eq!(r.result.fields_per_wafer, 100);
        assert_eq!(r.setup.dose_from, "[throughput] dose_mj_cm2");
        assert!(
            (r.result.wafers_per_hour - expected_wph(p, 250.0, 100)).abs() < 1e-9,
            "{}",
            r.result.wafers_per_hour
        );
        // optics_transmission form.
        let r = eval(
            "[source]\ntype = \"lpp\"\n[throughput]\noptics_transmission = 0.02\n",
            None,
        )
        .unwrap();
        assert_eq!(r.setup.params.optics_transmission, 0.02);
        // Named preset wins over the wavelength choice.
        let r = eval(
            "[source]\ntype = \"lpp\"\n[throughput]\npreset = \"beuv_la_b\"\n",
            None,
        )
        .unwrap();
        assert!((r.setup.params.optics_transmission - 0.641f64.powi(10)).abs() < 1e-15);
        assert!(!r.setup.preset_from_wavelength);
    }

    #[test]
    fn test_dose_precedence() {
        let process = "[source]\ntype = \"lpp\"\n[process]\ndose_mj_cm2 = 20.0\n";
        let r = eval(process, None).unwrap();
        assert_eq!(r.setup.params.dose_mj_cm2, 20.0);
        assert_eq!(r.setup.dose_from, "[process] dose_mj_cm2");
        let r = eval(
            &format!("{process}[throughput]\ndose_mj_cm2 = 25.0\n"),
            None,
        )
        .unwrap();
        assert_eq!(r.setup.params.dose_mj_cm2, 25.0);
        let r = eval(
            &format!("{process}[throughput]\ndose_mj_cm2 = 25.0\n"),
            Some(35.0),
        )
        .unwrap();
        assert_eq!(r.setup.params.dose_mj_cm2, 35.0);
        let r = eval("[source]\ntype = \"lpp\"\n", None).unwrap();
        assert_eq!(r.setup.params.dose_mj_cm2, 30.0);
        assert!(eval("[source]\ntype = \"lpp\"\n", Some(-1.0)).is_err());
    }

    /// Photons per (16 nm)² at 30 mJ/cm² and 13.5 nm ≈ 5220 (1.4 % shot
    /// noise), with the square taken from the [mask] line width; the
    /// per-exposure dose jitter is the per-pulse rms over √N_pulses.
    #[test]
    fn test_photon_statistics_and_dose_jitter() {
        let r = eval(
            "[source]\ntype = \"lpp\"\n[mask]\ncd_nm = 16.0\npitch_nm = 32.0\n",
            None,
        )
        .unwrap();
        let stats = &r.to_json()["photon_statistics"];
        assert_eq!(stats["cd_nm"], 16.0);
        let n = stats["photons_per_square"].as_f64().unwrap();
        assert!((n - 5219.6).abs() < 0.5, "{n}");
        let noise = stats["relative_shot_noise"].as_f64().unwrap();
        assert!((noise - 1.0 / n.sqrt()).abs() < 1e-15);
        // DPP: 5 % shot-to-shot jitter, averaged over the pulses per point.
        let r = eval(
            "[source]\ntype = \"dpp\"\n[throughput]\ncd_nm = 20.0\n",
            None,
        )
        .unwrap();
        let j = &r.to_json()["dose_jitter"];
        let (rms, pulses, per) = (
            j["shot_to_shot_rms"].as_f64().unwrap(),
            j["pulses_per_point"].as_f64().unwrap(),
            j["per_exposure_rms"].as_f64().unwrap(),
        );
        assert_eq!(rms, 0.05);
        assert!((per - rms / pulses.sqrt()).abs() < 1e-15);
        // No mask and no cd_nm: no photon statistics.
        let r = eval("[source]\ntype = \"lpp\"\n", None).unwrap();
        assert!(r.to_json()["photon_statistics"].is_null());
    }

    #[test]
    fn test_errors_and_notes() {
        let err = |text: &str| eval(text, None).err().expect("should fail").to_string();
        let both = err(
            "[source]\ntype = \"lpp\"\n[throughput]\noptics_transmission = 0.02\nn_mirrors = 10\n\
             mirror_reflectivity = 0.7\n",
        );
        assert!(both.contains("give the optics transmission once"), "{both}");
        let half = err("[source]\ntype = \"lpp\"\n[throughput]\nn_mirrors = 10\n");
        assert!(half.contains("go together"), "{half}");
        let unknown = err("[source]\ntype = \"lpp\"\n[throughput]\ndose = 30.0\n");
        assert!(
            unknown.contains("unknown key `dose` in [throughput]"),
            "{unknown}"
        );
        assert!(unknown.contains("did you mean `dose_mj_cm2`?"), "{unknown}");
        assert!(unknown.contains("keys read by [throughput]"), "{unknown}");
        let lamp = err("[source]\ntype = \"vuv\"\npreset = \"hg_i\"\n");
        assert!(lamp.contains("reports no average power"), "{lamp}");
        for bad in [
            "optics_transmission = 1.5",
            "mask_efficiency = 0.0",
            "preset = \"duv\"",
            "fields_per_wafer = 0",
            "wafer_diameter_mm = 0.0",
            "field_overhead_s = -1.0",
        ] {
            assert!(
                eval(
                    &format!("[source]\ntype = \"lpp\"\n[throughput]\n{bad}\n"),
                    None
                )
                .is_err(),
                "should reject {bad}"
            );
        }
        // No preset covers a 25 nm LPA-FEL or a hard-X-ray tube: the
        // wavelength-chosen preset is refused (as api.wafer_throughput does).
        let fel = err("[source]\ntype = \"lpa_fel\"\n");
        assert!(
            fel.contains("no projection-lithography optics preset"),
            "{fel}"
        );
        let tube = err("[source]\ntype = \"xray_tube\"\n");
        assert!(tube.contains("proximity / LIGA"), "{tube}");
        // An explicit preset out of its band runs, labelled hypothetical.
        let r = eval(
            "[source]\ntype = \"lpa_fel\"\n[throughput]\npreset = \"euv_hvm\"\n",
            None,
        )
        .unwrap();
        assert!(
            r.setup.notes[0].starts_with("hypothetical"),
            "{:?}",
            r.setup.notes
        );
        // An explicit transmission runs, and says which preset fills the rest.
        let r = eval(
            "[source]\ntype = \"xray_tube\"\n[throughput]\noptics_transmission = 1.0\n",
            None,
        )
        .unwrap();
        assert_eq!(r.setup.preset, "beuv_la_b");
        assert!(
            r.setup.notes[0].contains("mask and field parameters"),
            "{:?}",
            r.setup.notes
        );
        // In-band sources need no note: Sn LPP (13.5), Ag SXRL (13.9), Gd LPP (6.7).
        for src in [
            "type = \"lpp\"",
            "type = \"sxrl\"\nscheme = \"ag_13nm9\"",
            "type = \"lpp\"\nfuel = \"gd\"",
        ] {
            let r = eval(&format!("[source]\n{src}\n"), None).unwrap();
            assert!(r.setup.notes.is_empty(), "{src}: {:?}", r.setup.notes);
        }
    }
}
