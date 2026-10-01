//! `highuvlith sources` — every source family and preset, built from TOML.
//!
//! Each registry entry is a `[source]` TOML snippet that is built through
//! the same path as a config file (`SimConfig::from_toml_str` → `validate`
//! → `to_source`), so every row is reproducible by pasting its snippet into
//! a config. The status badge summarizes the honesty grade of the family /
//! preset (✅ implemented, 🔶 simplified, 🧪 theoretical / projection); the
//! authoritative grades live in `docs/capability-matrix.md` — keep the two
//! in step.

use highuvlith_core::source::{LithographySource, SourceKind};
use highuvlith_core::source_models::throughput::HVM_EUV_POWER_AT_IF_W;

use crate::config::{SimConfig, SOURCE_TYPES};

/// One selectable preset: its `[source]` TOML and honesty badge.
#[derive(Debug)]
pub struct Preset {
    /// `[source] type` tag.
    pub family: &'static str,
    /// Short preset name (unique within the family).
    pub name: &'static str,
    /// The `[source]` table body that selects it.
    pub toml: &'static str,
    /// Honesty badge (see the module docs).
    pub status: &'static str,
    /// One-line description.
    pub note: &'static str,
    /// The derived quantity shown in the overview table.
    pub headline: &'static str,
}

/// Every preset the `[source]` table can select.
pub const PRESETS: &[Preset] = &[
    // --- VUV / DUV / UV (type = "vuv") ---
    Preset {
        family: "vuv",
        name: "f2",
        toml: "type = \"vuv\"\npreset = \"f2\"",
        status: "✅",
        note: "F2 excimer 157.63 nm",
        headline: "photon_density_per_dose",
    },
    Preset {
        family: "vuv",
        name: "legacy",
        toml: "wavelength_nm = 157.63",
        status: "✅",
        note: "legacy explicit-wavelength source (no type tag)",
        headline: "photon_density_per_dose",
    },
    Preset {
        family: "vuv",
        name: "ar2",
        toml: "type = \"vuv\"\npreset = \"ar2\"",
        status: "🧪",
        note: "Ar2 126 nm — hypothetical lithography laser",
        headline: "photon_density_per_dose",
    },
    Preset {
        family: "vuv",
        name: "arf",
        toml: "type = \"vuv\"\npreset = \"arf\"",
        status: "✅ (🔶 params)",
        note: "ArF 193.368 nm, representative 90 W",
        headline: "e95_bandwidth",
    },
    Preset {
        family: "vuv",
        name: "krf",
        toml: "type = \"vuv\"\npreset = \"krf\"",
        status: "✅ (🔶 params)",
        note: "KrF 248.3 nm, representative 40 W",
        headline: "e95_bandwidth",
    },
    Preset {
        family: "vuv",
        name: "hg_i",
        toml: "type = \"vuv\"\npreset = \"hg_i\"",
        status: "✅ (🔶 params)",
        note: "Hg i-line 365.0 nm lamp (CW, power not modeled)",
        headline: "photon_density_per_dose",
    },
    Preset {
        family: "vuv",
        name: "hg_h",
        toml: "type = \"vuv\"\npreset = \"hg_h\"",
        status: "✅ (🔶 params)",
        note: "Hg h-line 404.7 nm lamp (CW, power not modeled)",
        headline: "photon_density_per_dose",
    },
    Preset {
        family: "vuv",
        name: "hg_g",
        toml: "type = \"vuv\"\npreset = \"hg_g\"",
        status: "✅ (🔶 params)",
        note: "Hg g-line 435.8 nm lamp (CW, power not modeled)",
        headline: "photon_density_per_dose",
    },
    // --- LPA-FEL ---
    Preset {
        family: "lpa_fel",
        name: "25nm",
        toml: "type = \"lpa_fel\"",
        status: "🔶",
        note: "25 nm design projection (BELLA-class; demonstrated: 420 nm at 1 Hz)",
        headline: "photons_per_pulse",
    },
    Preset {
        family: "lpa_fel",
        name: "25nm_machine",
        toml: "type = \"lpa_fel\"\nwavelength_nm = 25.0\nelectron_energy_mev = 500.0\n\
               period_mm = 20.0\nundulator_k = 1.6724\nnum_periods = 200\n\
               peak_current_a = 1000.0\nnorm_emittance_um = 0.5\n\
               electron_energy_spread_rel = 0.01\nbeta_m = 1.0",
        status: "🔶",
        note: "illustrative LPA undulator + beam (examples/sim_lpa_fel.toml)",
        headline: "energy_spread_over_rho",
    },
    // --- LPP ---
    Preset {
        family: "lpp",
        name: "sn_co2",
        toml: "type = \"lpp\"\nfuel = \"sn\"",
        status: "✅",
        note: "Sn 13.5 nm, CO2 drive (NXE:3400B chain, 250 W at IF)",
        headline: "drive_watts_per_if_watt",
    },
    Preset {
        family: "lpp",
        name: "sn_500w",
        toml: "type = \"lpp\"\npreset = \"nxe3800e\"",
        status: "🔶",
        note: "Sn 13.5 nm, NXE:3800E-class 500 W at IF (drive power assumed)",
        headline: "drive_watts_per_if_watt",
    },
    Preset {
        family: "lpp",
        name: "sn_1um",
        toml: "type = \"lpp\"\nfuel = \"sn\"\ndrive_laser = \"solid_state_1um\"",
        status: "🔶",
        note: "Sn 13.5 nm, 1 µm drive (reported-CE what-if)",
        headline: "drive_watts_per_if_watt",
    },
    Preset {
        family: "lpp",
        name: "sn_2um",
        toml: "type = \"lpp\"\nfuel = \"sn\"\ndrive_laser = \"thulium_2um\"",
        status: "🧪",
        note: "Sn 13.5 nm, 2 µm drive (projected CE)",
        headline: "drive_watts_per_if_watt",
    },
    Preset {
        family: "lpp",
        name: "gd",
        toml: "type = \"lpp\"\nfuel = \"gd\"",
        status: "🧪",
        note: "Gd 6.7 nm BEUV — in-band power is a projection",
        headline: "drive_watts_per_if_watt",
    },
    Preset {
        family: "lpp",
        name: "tb",
        toml: "type = \"lpp\"\nfuel = \"tb\"",
        status: "🧪",
        note: "Tb 6.5 nm BEUV — in-band power is a projection",
        headline: "drive_watts_per_if_watt",
    },
    // --- Synchrotron ---
    Preset {
        family: "synchrotron",
        name: "undulator",
        toml: "type = \"synchrotron\"",
        status: "✅ (🔶 coherence)",
        note: "538 MeV ring undulator, derived 13.5 nm (coherence 0.2 constant)",
        headline: "central_cone_flux",
    },
    Preset {
        family: "synchrotron",
        name: "compact_euv",
        toml: "type = \"synchrotron\"\nemittance_x_nm_rad = 10.0\nemittance_y_nm_rad = 0.1",
        status: "✅",
        note: "same undulator, coherence derived from the ring emittances",
        headline: "coherent_fraction",
    },
    Preset {
        family: "synchrotron",
        name: "bending_magnet",
        toml: "type = \"synchrotron\"\nbeamline = \"bending_magnet\"",
        status: "✅",
        note: "2.5 GeV / 1.5 T white beam (LIGA; flux per mrad, no total power)",
        headline: "power_per_mrad",
    },
    // --- HHG ---
    Preset {
        family: "hhg",
        name: "ne_q59",
        toml: "type = \"hhg\"",
        status: "✅/🔶",
        note: "Ne, 800 nm driver, harmonic 59 = 13.56 nm (efficiency ±1 decade)",
        headline: "driver_power_for_hvm",
    },
    Preset {
        family: "hhg",
        name: "ar_q27",
        toml: "type = \"hhg\"\ngas = \"ar\"\ndriver_intensity_w_cm2 = 2.0e14\nharmonic = 27",
        status: "✅/🔶",
        note: "Ar, 800 nm driver, harmonic 27 = 29.6 nm",
        headline: "driver_power_for_hvm",
    },
    Preset {
        family: "hhg",
        name: "ar_full_comb",
        toml: "type = \"hhg\"\ngas = \"ar\"\ndriver_intensity_w_cm2 = 2.0e14\nfull_comb = true",
        status: "✅/🔶",
        note: "Ar full harmonic comb (image it with [imaging] spectrum = \"per_wavelength\")",
        headline: "comb_line_count",
    },
    // --- XFEL ---
    Preset {
        family: "xfel",
        name: "flash",
        toml: "type = \"xfel\"\nxfel_preset = \"flash\"",
        status: "✅",
        note: "FLASH-class SASE, ρ derived from the machine",
        headline: "pierce_parameter_1d",
    },
    Preset {
        family: "xfel",
        name: "fermi",
        toml: "type = \"xfel\"\nxfel_preset = \"fermi\"",
        status: "✅",
        note: "FERMI-class seeded FEL",
        headline: "pierce_parameter_1d",
    },
    Preset {
        family: "xfel",
        name: "cw_sc",
        toml: "type = \"xfel\"\nxfel_preset = \"cw_sc\"",
        status: "🧪",
        note: "CW superconducting linac (LCLS-II-like) at 13.5 nm — projection",
        headline: "pierce_parameter_1d",
    },
    Preset {
        family: "xfel",
        name: "erl",
        toml: "type = \"xfel\"\nxfel_preset = \"erl\"",
        status: "🧪",
        note: "ERL-driven kW-class FEL (KEK design class) — projection",
        headline: "pierce_parameter_1d",
    },
    // --- ICS ---
    Preset {
        family: "ics",
        name: "stored",
        toml: "type = \"ics\"",
        status: "🧪",
        note: "13.5 nm Compton kinematics with a stored 1 nJ × 10 kHz yield",
        headline: "hvm_power_gap",
    },
    Preset {
        family: "ics",
        name: "collision",
        toml: "type = \"ics\"\nwavelength_nm = 13.5\nbunch_charge_pc = 100.0\n\
               laser_pulse_energy_mj = 10.0\nelectron_spot_um = 10.0\nlaser_spot_um = 10.0\n\
               rep_rate_hz = 1.0e8",
        status: "🧪",
        note: "yield derived from an aggressive collision design point",
        headline: "hvm_power_gap",
    },
    // --- SSMB ---
    Preset {
        family: "ssmb",
        name: "stored",
        toml: "type = \"ssmb\"",
        status: "🧪",
        note: "400 MeV ring, 1053 nm modulation, stored 1 kW projection",
        headline: "harmonic_number",
    },
    Preset {
        family: "ssmb",
        name: "radiator",
        toml: "type = \"ssmb\"\naverage_current_a = 1.0\nradiator_periods = 100\nradiator_k = 1.6",
        status: "🧪",
        note: "coherent power derived; b is the value 1 kW requires",
        headline: "bunching_factor",
    },
    // --- Entangled photons ---
    Preset {
        family: "entangled",
        name: "noon2",
        toml: "type = \"entangled\"",
        status: "🧪",
        note: "N = 2 N00N state at 157.63 nm (quantum lithography)",
        headline: "exposure_time_bound",
    },
    // --- X-ray tube ---
    Preset {
        family: "xray_tube",
        name: "w",
        toml: "type = \"xray_tube\"\nanode = \"w\"",
        status: "✅/🔶",
        note: "W anode 60 kV / 30 mA (lab LIGA)",
        headline: "mean_photon_energy_kev",
    },
    Preset {
        family: "xray_tube",
        name: "mo",
        toml: "type = \"xray_tube\"\nanode = \"mo\"",
        status: "✅/🔶",
        note: "Mo anode 50 kV",
        headline: "mean_photon_energy_kev",
    },
    Preset {
        family: "xray_tube",
        name: "cu",
        toml: "type = \"xray_tube\"\nanode = \"cu\"",
        status: "✅/🔶",
        note: "Cu anode 40 kV (Cu Kα line)",
        headline: "mean_photon_energy_kev",
    },
    Preset {
        family: "xray_tube",
        name: "rh",
        toml: "type = \"xray_tube\"\nanode = \"rh\"",
        status: "✅/🔶",
        note: "Rh anode 50 kV / 40 mA",
        headline: "mean_photon_energy_kev",
    },
    // --- DPP / LDP ---
    Preset {
        family: "dpp",
        name: "sn",
        toml: "type = \"dpp\"\nfuel = \"sn\"",
        status: "🔶",
        note: "Sn laser-assisted discharge, 13.5 nm",
        headline: "electrode_heat_load_w",
    },
    Preset {
        family: "dpp",
        name: "xe",
        toml: "type = \"dpp\"\nfuel = \"xe\"",
        status: "🔶",
        note: "Xe pinch, 13.5 nm (metrology class)",
        headline: "etendue_usable_fraction",
    },
    // --- Soft-X-ray lasers ---
    Preset {
        family: "sxrl",
        name: "ar_46nm9",
        toml: "type = \"sxrl\"\nscheme = \"ar_46nm9\"",
        status: "✅/🔶",
        note: "capillary-discharge Ne-like Ar, 46.9 nm",
        headline: "interference_min_half_pitch_nm",
    },
    Preset {
        family: "sxrl",
        name: "ag_13nm9",
        toml: "type = \"sxrl\"\nscheme = \"ag_13nm9\"",
        status: "✅/🔶",
        note: "transient Ni-like Ag, 13.9 nm",
        headline: "interference_min_half_pitch_nm",
    },
    Preset {
        family: "sxrl",
        name: "cd_13nm2",
        toml: "type = \"sxrl\"\nscheme = \"cd_13nm2\"",
        status: "✅/🔶",
        note: "transient Ni-like Cd, 13.2 nm",
        headline: "interference_min_half_pitch_nm",
    },
    Preset {
        family: "sxrl",
        name: "mo_18nm9",
        toml: "type = \"sxrl\"\nscheme = \"mo_18nm9\"",
        status: "✅/🔶",
        note: "transient Ni-like Mo, 18.9 nm",
        headline: "interference_min_half_pitch_nm",
    },
    // --- Betatron ---
    Preset {
        family: "betatron",
        name: "lwfa_100tw",
        toml: "type = \"betatron\"",
        status: "🧪",
        note: "LWFA betatron X-rays, 200 MeV / 1e19 cm^-3 (lab LIGA)",
        headline: "critical_energy_kev",
    },
    // --- Smith–Purcell ---
    Preset {
        family: "smith_purcell",
        name: "euv_13nm5",
        toml: "type = \"smith_purcell\"",
        status: "🧪",
        note: "30 keV / 10 nA beam, period derived for 13.5 nm",
        headline: "hvm_power_gap",
    },
    Preset {
        family: "smith_purcell",
        name: "xray_1nm",
        toml: "type = \"smith_purcell\"\ngrating_period_nm = 0.335",
        status: "🧪",
        note: "0.335 nm period (crystal planes) → ~1 nm",
        headline: "hvm_power_gap",
    },
];

/// Build a preset's source through the config path.
pub fn build(preset: &Preset) -> anyhow::Result<SourceKind> {
    let config = SimConfig::from_toml_str(&format!("[source]\n{}\n", preset.toml))?;
    config.validate()?;
    config.to_source()
}

/// The presets of `family` (all when `None`), each with its built source.
pub fn entries(family: Option<&str>) -> anyhow::Result<Vec<(&'static Preset, SourceKind)>> {
    if let Some(f) = family {
        if !SOURCE_TYPES.contains(&f) {
            anyhow::bail!(
                "unknown source family '{}' (expected one of: {})",
                f,
                SOURCE_TYPES.join(", ")
            );
        }
    }
    PRESETS
        .iter()
        .filter(|p| family.is_none_or(|f| p.family == f))
        .map(|p| {
            build(p)
                .map(|src| (p, src))
                .map_err(|e| anyhow::anyhow!("preset {}/{}: {e}", p.family, p.name))
        })
        .collect()
}

/// `P / 250 W` (the NXE:3400B source at intermediate focus).
fn hvm_ratio(src: &SourceKind) -> Option<f64> {
    src.average_power_w().map(|p| p / HVM_EUV_POWER_AT_IF_W)
}

/// `x` with `digits` significant digits: fixed notation for magnitudes in
/// [1e-3, 1e6), scientific otherwise.
pub(crate) fn sig(x: f64, digits: usize) -> String {
    let a = x.abs();
    if x == 0.0 {
        "0".to_string()
    } else if !a.is_finite() || !(1e-3..1e6).contains(&a) {
        format!("{:.*e}", digits.saturating_sub(1), x)
    } else {
        let decimals = (digits as i32 - 1 - a.log10().floor() as i32).max(0) as usize;
        format!("{x:.decimals$}")
    }
}

/// Four significant digits (table and derived quantities).
fn num(x: f64) -> String {
    sig(x, 4)
}

fn num_or_dash(x: Option<f64>) -> String {
    x.map_or_else(|| "—".to_string(), num)
}

/// Terminal width of a string: the badge emoji take two columns.
fn display_width(s: &str) -> usize {
    s.chars()
        .map(|c| {
            if matches!(c, '✅' | '🔶' | '🧪') {
                2
            } else {
                1
            }
        })
        .sum()
}

fn pad(s: &str, width: usize) -> String {
    let w = display_width(s);
    format!("{s}{}", " ".repeat(width.saturating_sub(w)))
}

/// The overview table (one row per preset).
pub fn render_table(rows: &[(&'static Preset, SourceKind)]) -> String {
    let status_w = rows
        .iter()
        .map(|(p, _)| display_width(p.status))
        .max()
        .unwrap_or(6)
        .max(6);
    let mut out = String::new();
    out.push_str(&format!(
        "{:<14} {:<13} {:>10} {:>10} {:>10} {:>10}  {}  {}\n",
        "family",
        "preset",
        "λ (nm)",
        "Δλ (pm)",
        "P (W)",
        "P/250 W",
        pad("status", status_w),
        "headline"
    ));
    out.push_str(&format!("{}\n", "-".repeat(118)));
    for (p, src) in rows {
        let headline = src
            .derived_quantities()
            .into_iter()
            .find(|q| q.name == p.headline)
            .map_or_else(String::new, |q| {
                format!("{} = {} {}", q.name, num(q.value), q.unit)
            });
        out.push_str(&format!(
            "{:<14} {:<13} {:>10} {:>10} {:>10} {:>10}  {}  {}\n",
            p.family,
            p.name,
            sig(src.wavelength_nm(), 6),
            num(src.bandwidth_pm()),
            num_or_dash(src.average_power_w()),
            num_or_dash(hvm_ratio(src)),
            pad(p.status, status_w),
            headline
        ));
    }
    out.push_str(
        "\nP = average_power_w(): usable output into the illuminator (plasma sources: in-band \
         power at intermediate focus; — = not modeled, e.g. CW lamps, bending-magnet fan). \
         P/250 W: ratio to the NXE:3400B source.\nBadges: ✅ implemented, 🔶 simplified, \
         🧪 theoretical / projection — authoritative grades in docs/capability-matrix.md.\n\
         Details and the TOML of each preset: highuvlith sources --family <type>\n",
    );
    out
}

/// Detailed blocks (TOML snippet, beam numbers, every derived quantity).
pub fn render_details(rows: &[(&'static Preset, SourceKind)]) -> String {
    let mut out = String::new();
    for (p, src) in rows {
        out.push_str(&format!(
            "{} / {}  {} — {}\n",
            p.family, p.name, p.status, p.note
        ));
        out.push_str("  [source]\n");
        for line in p.toml.lines() {
            out.push_str(&format!("  {line}\n"));
        }
        out.push_str(&format!(
            "  wavelength      {} nm (photon {} eV)\n",
            sig(src.wavelength_nm(), 6),
            num(src.photon_energy_ev())
        ));
        out.push_str(&format!(
            "  bandwidth       {} pm\n",
            num(src.bandwidth_pm())
        ));
        out.push_str(&format!(
            "  pulse energy    {} J\n",
            num_or_dash(src.pulse_energy_j())
        ));
        out.push_str(&format!(
            "  rep rate        {} Hz\n",
            num_or_dash(LithographySource::rep_rate_hz(src))
        ));
        out.push_str(&format!(
            "  average power   {} W (P/250 W = {})\n",
            num_or_dash(src.average_power_w()),
            num_or_dash(hvm_ratio(src))
        ));
        out.push_str(&format!(
            "  coherence       {}  (transverse coherent fraction; 0 = unknown/incoherent)\n",
            num(src.transverse_coherence())
        ));
        out.push_str(&format!(
            "  shot-to-shot    {} rms\n",
            num(src.shot_to_shot_rms())
        ));
        let derived = src.derived_quantities();
        if !derived.is_empty() {
            out.push_str("  derived:\n");
            for q in derived {
                out.push_str(&format!(
                    "    {:<36} {:>12} {:<28} {}\n",
                    q.name,
                    num(q.value),
                    q.unit,
                    q.note
                ));
            }
        }
        out.push('\n');
    }
    out
}

/// JSON array of the presets (every number the table and details show).
pub fn to_json(rows: &[(&'static Preset, SourceKind)]) -> serde_json::Value {
    let finite = |x: Option<f64>| x.filter(|v| v.is_finite());
    serde_json::Value::Array(
        rows.iter()
            .map(|(p, src)| {
                serde_json::json!({
                    "family": p.family,
                    "preset": p.name,
                    "toml": format!("[source]\n{}\n", p.toml),
                    "status": p.status,
                    "note": p.note,
                    "wavelength_nm": src.wavelength_nm(),
                    "photon_energy_ev": src.photon_energy_ev(),
                    "bandwidth_pm": src.bandwidth_pm(),
                    "average_power_w": finite(src.average_power_w()),
                    "pulse_energy_j": finite(src.pulse_energy_j()),
                    "rep_rate_hz": finite(LithographySource::rep_rate_hz(src)),
                    "transverse_coherence": src.transverse_coherence(),
                    "shot_to_shot_rms": src.shot_to_shot_rms(),
                    "hvm_power_ratio": finite(hvm_ratio(src)),
                    "headline": p.headline,
                    "derived_quantities": src.derived_quantities(),
                })
            })
            .collect(),
    )
}

pub fn run(family: Option<&str>, json: bool) -> anyhow::Result<()> {
    let rows = entries(family)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&to_json(&rows))?);
    } else if family.is_some() {
        print!("{}", render_details(&rows));
    } else {
        print!("{}", render_table(&rows));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(family: &str, name: &str) -> SourceKind {
        let p = PRESETS
            .iter()
            .find(|p| p.family == family && p.name == name)
            .unwrap_or_else(|| panic!("no preset {family}/{name}"));
        build(p).unwrap()
    }

    /// Every preset builds through the TOML path, every family is covered,
    /// names are unique, and the headline quantity exists.
    #[test]
    fn test_every_preset_builds_from_its_toml() {
        let rows = entries(None).unwrap();
        assert_eq!(rows.len(), PRESETS.len());
        for (p, src) in &rows {
            let lambda = src.wavelength_nm();
            assert!(
                lambda.is_finite() && lambda > 0.0,
                "{}/{}",
                p.family,
                p.name
            );
            assert_eq!(src.kind_label(), p.family, "{}/{}", p.family, p.name);
            assert!(
                src.derived_quantities()
                    .iter()
                    .any(|q| q.name == p.headline),
                "{}/{}: no derived quantity '{}'",
                p.family,
                p.name,
                p.headline
            );
        }
        for family in SOURCE_TYPES {
            assert!(
                PRESETS.iter().any(|p| p.family == *family),
                "family {family} has no preset"
            );
        }
        let mut names: Vec<(&str, &str)> = PRESETS.iter().map(|p| (p.family, p.name)).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), PRESETS.len(), "duplicate preset names");
    }

    /// Spot checks against the WP-C / WP-D notes.
    #[test]
    fn test_preset_numbers_match_the_source_models() {
        let power = |f: &str, n: &str| find(f, n).average_power_w().unwrap();
        assert!((power("lpp", "sn_co2") - 250.0).abs() < 1e-9);
        assert!((power("vuv", "arf") - 90.0).abs() < 1e-9);
        assert!((power("xfel", "cw_sc") - 134.95).abs() < 0.1);
        // DPP Sn: 18 kW x 2 % CE = 360 W into 2pi; x (1.5 sr / 2pi) x 0.4 = 34.3775 W at IF.
        assert!((power("dpp", "sn") - 34.3775).abs() < 1e-3);
        assert!((power("lpp", "sn_500w") - 500.0).abs() < 1e-9);
        assert!((power("ssmb", "stored") - 1000.0).abs() < 1e-9);
        assert!((power("ics", "collision") - 7.1227e-5).abs() < 1e-8);
        assert!((find("vuv", "arf").wavelength_nm() - 193.368).abs() < 1e-12);
        assert!((find("smith_purcell", "xray_1nm").wavelength_nm() - 1.020).abs() < 2e-3);
        // CW lamps and the bending-magnet fan report no average power.
        assert!(find("vuv", "hg_i").average_power_w().is_none());
        assert!(find("synchrotron", "bending_magnet")
            .average_power_w()
            .is_none());
    }

    #[test]
    fn test_family_filter_json_and_rendering() {
        let lpp = entries(Some("lpp")).unwrap();
        assert_eq!(lpp.len(), 6);
        let json = to_json(&lpp);
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 6);
        for key in [
            "family",
            "preset",
            "toml",
            "status",
            "wavelength_nm",
            "bandwidth_pm",
            "average_power_w",
            "pulse_energy_j",
            "rep_rate_hz",
            "transverse_coherence",
            "hvm_power_ratio",
            "derived_quantities",
        ] {
            assert!(arr[0].get(key).is_some(), "missing JSON key {key}");
        }
        assert_eq!(arr[0]["average_power_w"].as_f64().unwrap().round(), 250.0);
        assert!(arr[0]["derived_quantities"].as_array().unwrap().len() > 5);
        // The JSON toml snippet round-trips through the config parser.
        let toml = arr[4]["toml"].as_str().unwrap();
        let cfg = SimConfig::from_toml_str(toml).unwrap();
        cfg.validate().unwrap();
        assert!((cfg.to_source().unwrap().wavelength_nm() - 6.7).abs() < 1e-9);
        // Unknown family: error naming the valid ones.
        let err = entries(Some("laser")).unwrap_err().to_string();
        assert!(
            err.contains("smith_purcell") && err.contains("xray_tube"),
            "{err}"
        );
        // Renderers.
        let table = render_table(&entries(None).unwrap());
        assert!(table.contains("sn_co2") && table.contains("lwfa_100tw"));
        assert!(table.contains("drive_watts_per_if_watt = 86.00"), "{table}");
        assert!(
            table.contains("193.368") && table.contains("365.015"),
            "{table}"
        );
        let details = render_details(&entries(Some("hhg")).unwrap());
        assert!(details.contains("full_comb = true"));
        assert!(details.contains("comb_line_count"));
    }
}
