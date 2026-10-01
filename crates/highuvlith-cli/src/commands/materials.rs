//! `highuvlith materials` — optical constants and multilayer mirrors.
//!
//! - no options: `n`, `k` of a material set at `--wavelength` (default
//!   157 nm): the VUV tables above 41.3 nm, the Henke-computed X-ray / EUV /
//!   BEUV entries at 0.0413–41.3 nm;
//! - `--name M`: one material (a table name or `henke:<formula>[@density]`),
//!   with dn/dλ for Sellmeier materials and δ, β and the intensity
//!   attenuation length for Henke materials;
//! - `--multilayer mo_si|la_b4c|la_b`: an EUV/BEUV Bragg mirror (Parratt
//!   recursion, `materials::multilayer`): R_s, R_p, R at (λ, θ), the peak,
//!   the FWHM bandwidth and the angular acceptance of the ideal stack.
//!
//! `--json` prints the same numbers as JSON on stdout.

use highuvlith_core::error::LithographyError;
use highuvlith_core::materials::database::MaterialsDatabase;
use highuvlith_core::materials::multilayer::MultilayerMirror;
use highuvlith_core::types::Polarization;

use super::sources::sig;

/// Command-line arguments of `highuvlith materials`.
pub struct MaterialsArgs {
    pub wavelength: Option<f64>,
    pub name: Option<String>,
    pub multilayer: Option<String>,
    pub periods: Option<usize>,
    pub period_nm: Option<f64>,
    pub gamma: Option<f64>,
    pub angle_deg: Option<f64>,
    pub json: bool,
}

/// VUV materials listed by the table above 41.3 nm.
const VUV_TABLE: &[&str] = &[
    "CaF2",
    "MgF2",
    "LiF",
    "BaF2",
    "SiO2",
    "Cr",
    "Si",
    "AlF3",
    "Na3AlF6",
    "LaF3",
    "GdF3",
    "VUV_resist",
    "VUV_BARC",
];

/// Upper end of the Henke (CXRO) tables, nm.
const HENKE_MAX_NM: f64 = 41.3;

/// Multilayer presets: `(name, periods, period_nm, gamma, default λ, label,
/// measured-vs-ideal note)` — the defaults of the Python `MultilayerMirror`
/// factories (WP-E1).
const MULTILAYERS: &[(&str, usize, f64, f64, f64, &str, &str)] = &[
    (
        "mo_si",
        40,
        6.9,
        0.4,
        13.5,
        "Mo/Si (Si on Mo; Γ = Mo fraction) on SiO2",
        "ideal stack: measured Mo/Si mirrors reach ~67–70 % (no interdiffusion, roughness or \
         oxidation modeled here)",
    ),
    (
        "la_b4c",
        200,
        3.37,
        0.4,
        6.7,
        "La/B4C (B4C on La; Γ = La fraction) on Si",
        "ideal stack: interface roughness (σ ≈ 0.3 nm) costs several % at 6.x nm",
    ),
    (
        "la_b",
        200,
        3.33,
        0.4,
        6.65,
        "La/B (B on La; Γ = La fraction) on Si",
        "ideal stack: the measured La/B record is 64.1 % at 6.65 nm",
    ),
];

/// A report: human-readable text and the same numbers as JSON.
pub struct MaterialsReport {
    pub text: String,
    pub json: serde_json::Value,
}

pub fn run(args: &MaterialsArgs) -> anyhow::Result<()> {
    let report = report(args)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report.json)?);
    } else {
        print!("{}", report.text);
    }
    Ok(())
}

/// Dispatch on the options (see the module docs).
pub fn report(args: &MaterialsArgs) -> anyhow::Result<MaterialsReport> {
    if args.multilayer.is_some() {
        if args.name.is_some() {
            anyhow::bail!("give either --name (a material) or --multilayer (a mirror), not both");
        }
        return multilayer_report(args);
    }
    if args.periods.is_some()
        || args.period_nm.is_some()
        || args.gamma.is_some()
        || args.angle_deg.is_some()
    {
        anyhow::bail!(
            "--periods / --period-nm / --gamma / --angle-deg describe a multilayer mirror: add \
             --multilayer mo_si | la_b4c | la_b"
        );
    }
    let db = MaterialsDatabase::new();
    let wl = args.wavelength.unwrap_or(157.0);
    if !(wl.is_finite() && wl > 0.0) {
        anyhow::bail!("--wavelength must be finite and > 0, got {wl}");
    }
    match args.name.as_deref() {
        Some(name) => material_report(&db, name, wl),
        None => Ok(table_report(&db, wl)),
    }
}

/// Whether `name` is computed from the Henke tables at `wl` (Henke entries,
/// and the VUV names "Si", "Cr", "SiO2" that fall back to Henke there).
fn is_henke(db: &MaterialsDatabase, name: &str, wl: f64) -> bool {
    name.starts_with("henke:") || db.xray_material(name).is_some() || wl <= HENKE_MAX_NM
}

fn material_report(db: &MaterialsDatabase, name: &str, wl: f64) -> anyhow::Result<MaterialsReport> {
    // Out-of-range wavelengths are an error (exit 1), never a clamped or
    // extrapolated value; the message names the data range and alternative.
    let n = db
        .refractive_index(name, wl)
        .map_err(|e| anyhow::anyhow!("{name} at {wl} nm: {e}"))?;
    let mut text = format!("{name} at {} nm:\n", sig(wl, 6));
    let mut json = serde_json::json!({
        "material": name,
        "wavelength_nm": wl,
        "n": n.re,
        "k": n.im,
    });
    if is_henke(db, name, wl) {
        // Henke: n = 1 − δ + iβ; intensity attenuation length λ / (4π β).
        let (delta, beta) = (1.0 - n.re, n.im);
        let attenuation = (beta > 0.0).then(|| wl / (4.0 * std::f64::consts::PI * beta));
        text.push_str(&format!("  n = {:.6}  (δ = {:.4e})\n", n.re, delta));
        text.push_str(&format!("  k = {:.6}  (β = {:.4e})\n", n.im, beta));
        if let Some(l) = attenuation {
            text.push_str(&format!(
                "  attenuation length (1/e intensity) = {} nm\n",
                sig(l, 4)
            ));
        }
        json["delta"] = delta.into();
        json["beta"] = beta.into();
        json["attenuation_length_nm"] = attenuation.into();
        if let Some(m) = db.xray_material(name) {
            text.push_str(&format!(
                "  formula {} at {} g/cm³ (Henke / CXRO optical constants)\n",
                m.formula, m.density_g_cm3
            ));
            json["formula"] = m.formula.clone().into();
            json["density_g_cm3"] = m.density_g_cm3.into();
        }
    } else {
        text.push_str(&format!("  n = {:.4}\n  k = {:.4}\n", n.re, n.im));
        if let Ok(disp) = db.dispersion(name, wl) {
            text.push_str(&format!("  dn/dλ = {:.6} /nm\n", disp));
            json["dn_dlambda_per_nm"] = disp.into();
        }
    }
    Ok(MaterialsReport { text, json })
}

/// At most 4 decimals, trailing zeros dropped (`165`, `0.0413`).
fn trim_nm(x: f64) -> String {
    let s = format!("{x:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn table_report(db: &MaterialsDatabase, wl: f64) -> MaterialsReport {
    // At X-ray/EUV/BEUV wavelengths (Henke tables: 0.0413-41.3 nm) list the
    // Henke-computed entries instead; any formula works via --name henke:<f>.
    let xray = db.xray_material_names();
    let (materials, decimals): (Vec<&str>, usize) = if wl <= HENKE_MAX_NM {
        (xray.iter().map(String::as_str).collect(), 6)
    } else {
        (VUV_TABLE.to_vec(), 4)
    };
    let width = decimals + 4;
    let mut text = format!(
        "{:<14} {:>width$} {:>width$}  (at {} nm)\n",
        "Material",
        "n",
        "k",
        sig(wl, 6)
    );
    text.push_str(&format!("{}\n", "-".repeat(16 + 2 * width)));
    let mut rows = Vec::new();
    for mat in &materials {
        match db.refractive_index(mat, wl) {
            Ok(n) => {
                text.push_str(&format!(
                    "{:<14} {:>width$.decimals$} {:>width$.decimals$}\n",
                    mat, n.re, n.im
                ));
                rows.push(serde_json::json!({ "material": mat, "n": n.re, "k": n.im }));
            }
            Err(e) => {
                let range = match e {
                    LithographyError::WavelengthOutOfRange { range_nm, .. } => Some(range_nm),
                    _ => None,
                };
                let note = range
                    .map(|(a, b)| format!("  (data range {}-{} nm)", trim_nm(a), trim_nm(b)))
                    .unwrap_or_default();
                text.push_str(&format!(
                    "{:<14} {:>width$} {:>width$}{note}\n",
                    mat, "N/A", "N/A"
                ));
                rows.push(serde_json::json!({
                    "material": mat, "n": null, "k": null, "data_range_nm": range,
                }));
            }
        }
    }
    if wl <= HENKE_MAX_NM {
        text.push_str(
            "Henke/CXRO optical constants; any compound via --name henke:<formula>[@density].\n",
        );
    }
    MaterialsReport {
        text,
        json: serde_json::json!({ "wavelength_nm": wl, "materials": rows }),
    }
}

fn multilayer_report(args: &MaterialsArgs) -> anyhow::Result<MaterialsReport> {
    let tag = args.multilayer.as_deref().unwrap_or_default();
    let &(_, periods0, period0, gamma0, wl0, label, note) =
        MULTILAYERS.iter().find(|m| m.0 == tag).ok_or_else(|| {
            anyhow::anyhow!(
                "unknown --multilayer '{}' (expected one of: {})",
                tag,
                MULTILAYERS
                    .iter()
                    .map(|m| m.0)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
    let periods = args.periods.unwrap_or(periods0);
    if periods == 0 {
        anyhow::bail!("--periods must be >= 1");
    }
    let period_nm = args.period_nm.unwrap_or(period0);
    let gamma = args.gamma.unwrap_or(gamma0);
    let wl = args.wavelength.unwrap_or(wl0);
    let angle_deg = args.angle_deg.unwrap_or(0.0);
    if !(angle_deg.is_finite() && (0.0..90.0).contains(&angle_deg)) {
        anyhow::bail!("--angle-deg must be in [0, 90) (from the surface normal), got {angle_deg}");
    }
    if !(wl.is_finite() && wl > 0.0 && wl <= HENKE_MAX_NM) {
        anyhow::bail!(
            "--wavelength must be in (0, {HENKE_MAX_NM}] nm for a multilayer (Henke tables), got \
             {wl}"
        );
    }
    let core = |e: highuvlith_core::error::LithographyError| anyhow::anyhow!("multilayer: {e}");
    let mirror = match tag {
        "mo_si" => MultilayerMirror::mo_si(periods, period_nm, gamma),
        "la_b4c" => MultilayerMirror::la_b4c(periods, period_nm, gamma),
        _ => MultilayerMirror::la_b(periods, period_nm, gamma),
    }
    .map_err(core)?;
    let theta = angle_deg.to_radians();
    let r_s = mirror
        .reflectance(wl, theta, Polarization::TE)
        .map_err(core)?;
    let r_p = mirror
        .reflectance(wl, theta, Polarization::TM)
        .map_err(core)?;
    let r = 0.5 * (r_s + r_p);
    let (lo, hi) = (0.95 * wl, (1.05 * wl).min(HENKE_MAX_NM));
    let (peak_wl, peak_r) = mirror
        .peak(theta, Polarization::Unpolarized, lo, hi)
        .map_err(core)?;
    let fwhm = mirror
        .bandwidth_fwhm_nm(theta, Polarization::Unpolarized, lo, hi)
        .ok();
    let acceptance = mirror
        .angular_acceptance_fwhm_deg(wl, Polarization::Unpolarized)
        .ok();

    let mut text = format!(
        "{label}: {periods} × {} nm periods, Γ = {gamma} (ideal interfaces)\n",
        sig(period_nm, 4)
    );
    text.push_str(&format!(
        "At {} nm, {}° from the normal:\n  R_s = {:.4}   R_p = {:.4}   R (unpolarized) = {:.4}\n",
        sig(wl, 5),
        angle_deg,
        r_s,
        r_p,
        r
    ));
    text.push_str(&format!(
        "Peak (unpolarized, {}–{} nm): R = {:.4} at {} nm\n",
        sig(lo, 4),
        sig(hi, 4),
        peak_r,
        sig(peak_wl, 5)
    ));
    match fwhm {
        Some(w) => text.push_str(&format!("Bandwidth (FWHM): {} nm\n", sig(w, 3))),
        None => text.push_str("Bandwidth (FWHM): not reached inside the ±5 % window\n"),
    }
    match acceptance {
        Some(a) => text.push_str(&format!(
            "Angular acceptance (FWHM, unpolarized, at {} nm): {}°\n",
            sig(wl, 5),
            sig(a, 3)
        )),
        None => text.push_str("Angular acceptance: half maximum not reached within 0–60°\n"),
    }
    text.push_str(&format!("Note: {note}.\n"));
    let json = serde_json::json!({
        "multilayer": tag,
        "description": label,
        "periods": periods,
        "period_nm": period_nm,
        "gamma": gamma,
        "wavelength_nm": wl,
        "angle_deg": angle_deg,
        "r_s": r_s,
        "r_p": r_p,
        "r_unpolarized": r,
        "peak_wavelength_nm": peak_wl,
        "peak_reflectance": peak_r,
        "fwhm_bandwidth_nm": fwhm,
        "angular_acceptance_fwhm_deg": acceptance,
        "note": note,
    });
    Ok(MaterialsReport { text, json })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args() -> MaterialsArgs {
        MaterialsArgs {
            wavelength: None,
            name: None,
            multilayer: None,
            periods: None,
            period_nm: None,
            gamma: None,
            angle_deg: None,
            json: false,
        }
    }

    fn ml(tag: &str) -> MaterialsArgs {
        MaterialsArgs {
            multilayer: Some(tag.to_string()),
            ..args()
        }
    }

    /// WP-E1 fixtures (CXRO-validated model): Mo/Si 40 × 6.9 nm, Γ = 0.4 on
    /// SiO2 — R(13.5 nm, 0°) = 0.729, peak 73.0 % at 13.480 nm, FWHM 0.629 nm;
    /// at 10° R_s = 0.532, R_p = 0.441.
    #[test]
    fn test_mo_si_multilayer_matches_e1_fixtures() {
        let r = report(&ml("mo_si")).unwrap().json;
        assert!(
            (r["r_unpolarized"].as_f64().unwrap() - 0.729).abs() < 2e-3,
            "{r}"
        );
        assert_eq!(r["r_s"], r["r_p"]); // normal incidence
        assert!(
            (r["peak_wavelength_nm"].as_f64().unwrap() - 13.48).abs() < 0.01,
            "{r}"
        );
        assert!(
            (r["peak_reflectance"].as_f64().unwrap() - 0.730).abs() < 2e-3,
            "{r}"
        );
        assert!(
            (r["fwhm_bandwidth_nm"].as_f64().unwrap() - 0.629).abs() < 0.01,
            "{r}"
        );
        assert!(r["angular_acceptance_fwhm_deg"].as_f64().unwrap() > 10.0);
        let tilted = report(&MaterialsArgs {
            angle_deg: Some(10.0),
            ..ml("mo_si")
        })
        .unwrap()
        .json;
        assert!(
            (tilted["r_s"].as_f64().unwrap() - 0.532).abs() < 3e-3,
            "{tilted}"
        );
        assert!(
            (tilted["r_p"].as_f64().unwrap() - 0.441).abs() < 3e-3,
            "{tilted}"
        );
    }

    /// La/B 200 × 3.33 nm, Γ = 0.4: ideal peak ≈ 80 % near 6.65 nm (WP-E1:
    /// 80.3 % at 6.65 nm with the tuned 3.329 nm period).
    #[test]
    fn test_la_b_multilayer_peak() {
        let r = report(&ml("la_b")).unwrap().json;
        let (wl, peak) = (
            r["peak_wavelength_nm"].as_f64().unwrap(),
            r["peak_reflectance"].as_f64().unwrap(),
        );
        assert!((wl - 6.65).abs() < 0.02, "{r}");
        assert!((peak - 0.80).abs() < 0.01, "{r}");
        assert_eq!(r["wavelength_nm"], 6.65);
    }

    #[test]
    fn test_multilayer_flag_errors() {
        let err = |a: MaterialsArgs| report(&a).err().expect("should fail").to_string();
        assert!(err(MaterialsArgs {
            periods: Some(40),
            ..args()
        })
        .contains("add --multilayer"));
        assert!(err(ml("w_si")).contains("expected one of: mo_si, la_b4c, la_b"));
        assert!(err(MaterialsArgs {
            angle_deg: Some(90.0),
            ..ml("mo_si")
        })
        .contains("--angle-deg"));
        assert!(err(MaterialsArgs {
            wavelength: Some(157.0),
            ..ml("mo_si")
        })
        .contains("--wavelength"));
        assert!(err(MaterialsArgs {
            periods: Some(0),
            ..ml("mo_si")
        })
        .contains("--periods"));
        assert!(err(MaterialsArgs {
            name: Some("Si".to_string()),
            ..ml("mo_si")
        })
        .contains("not both"));
    }

    #[test]
    fn test_material_lookup_and_table() {
        // Henke compound: n = 1 − δ + iβ, attenuation length λ/(4πβ).
        let r = report(&MaterialsArgs {
            name: Some("henke:Si3N4@3.44".to_string()),
            wavelength: Some(13.5),
            ..args()
        })
        .unwrap()
        .json;
        let (n, k) = (r["n"].as_f64().unwrap(), r["k"].as_f64().unwrap());
        assert!(n < 1.0 && k > 0.0);
        assert!((r["delta"].as_f64().unwrap() - (1.0 - n)).abs() < 1e-15);
        let l = r["attenuation_length_nm"].as_f64().unwrap();
        assert!((l - 13.5 / (4.0 * std::f64::consts::PI * k)).abs() < 1e-9);
        // Sellmeier material: dispersion reported.
        let r = report(&MaterialsArgs {
            name: Some("CaF2".to_string()),
            ..args()
        })
        .unwrap()
        .json;
        assert!(r["dn_dlambda_per_nm"].as_f64().unwrap() < 0.0);
        // Unknown material: an error (non-zero exit), not a printed message.
        assert!(report(&MaterialsArgs {
            name: Some("unobtainium".to_string()),
            ..args()
        })
        .is_err());
        // Tables: VUV above 41.3 nm, Henke entries below.
        let vuv = report(&args()).unwrap();
        assert!(vuv.text.contains("CaF2"));
        assert_eq!(
            vuv.json["materials"].as_array().unwrap().len(),
            VUV_TABLE.len()
        );
        let euv = report(&MaterialsArgs {
            wavelength: Some(13.5),
            ..args()
        })
        .unwrap();
        assert!(euv.text.contains("Henke/CXRO"));
        assert!(!euv.json["materials"].as_array().unwrap().is_empty());
    }

    /// Range safety (WP-M): "Si" at 13.5 nm is the Henke value (CXRO
    /// delta 9.987e-4), out-of-range lookups are errors naming the range,
    /// and the table shows the data range in place of a value.
    #[test]
    fn test_out_of_range_lookups() {
        let r = report(&MaterialsArgs {
            name: Some("Si".to_string()),
            wavelength: Some(13.5),
            ..args()
        })
        .unwrap()
        .json;
        assert!(
            (r["delta"].as_f64().unwrap() - 0.000998703763).abs() < 2e-6,
            "{r}"
        );
        let err = report(&MaterialsArgs {
            name: Some("Si".to_string()),
            wavelength: Some(80.0),
            ..args()
        })
        .err()
        .unwrap()
        .to_string();
        assert!(err.contains("henke:Si") && err.contains("125-161"), "{err}");
        let t = report(&MaterialsArgs {
            wavelength: Some(157.63),
            ..args()
        })
        .unwrap();
        assert!(t.text.contains("(data range 165-2000 nm)"), "{}", t.text);
    }
}
