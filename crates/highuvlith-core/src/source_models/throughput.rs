//! Dose-limited wafer throughput for any [`LithographySource`].
//!
//! Turns a source's usable output power ([`LithographySource::average_power_w`],
//! the in-band power delivered into the illuminator — at intermediate focus
//! for plasma sources) into wafers per hour for a step-and-scan exposure
//! tool, and reports the photon statistics that set the shot-noise floor.
//!
//! # Model status
//!
//! 🔶 Simplified: a first-order *dose-limited* scanner model. The optics
//! train is a single transmission factor, every exposure field is scanned
//! at the speed that delivers the dose (optionally capped by a stage speed
//! limit), and all mechanical overheads are two lumped times. Not modeled:
//! illuminator pupil-fill losses, flare/absorber duty, stage acceleration
//! profiles, reticle exchange, metrology, and partial-field edge policies.
//! The preset scanner numbers are illustrative assumptions (stated on each
//! constructor), not vendor data.
//!
//! # Key equations
//!
//! - Power at the wafer: `P_w = P_source T_optics T_mask`
//! - Dose-limited scan speed: `v = P_w / (D W)` (`D` dose, `W` slit length
//!   across the scan); with a stage limit `v = min(v_dose, v_max)`
//! - Per-field scan time: `(H + h) / v` (`H` field length, `h` slit height)
//! - Wafer time: `N_fields ((H + h)/v + t_field) + t_wafer`;
//!   wafers/hour `= 3600 / t_wafer_total`
//! - Pulses per exposure point: `N_p = f_rep h / v` (dose averaging that
//!   suppresses per-pulse energy jitter by `1/sqrt(N_p)`)
//! - Photons per square of side `a`: `N = D a^2 / (h c / lambda)`, relative
//!   shot noise `1/sqrt(N)`

use serde::{Deserialize, Serialize};

use super::physics;
use crate::source::LithographySource;

/// In-band EUV power at intermediate focus of the NXE:3400B source (250 W;
/// ≥125 wafers/hour at 20 mJ/cm² by specification). Used as the benchmark
/// for the `hvm_power_ratio` derived quantity of every short-wavelength
/// family. The HVM requirement band has since moved up: the NXE:3800E ships
/// with a 500 W source (ASML, 2024), ~600 W sources are reported shipping,
/// and 1 kW was demonstrated in 2025 — i.e. ~250 W → ~1 kW at IF.
pub const HVM_EUV_POWER_AT_IF_W: f64 = 250.0;

/// Scanner and process assumptions for the dose-limited throughput model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThroughputParams {
    /// Transmission of illuminator + projection optics between the source
    /// hand-off point and the wafer (e.g. `R^N` for `N` mirrors).
    pub optics_transmission: f64,
    /// Mask efficiency in clear areas: multilayer reflectance (EUV) or
    /// transmittance (VUV/DUV).
    pub mask_efficiency: f64,
    /// Resist dose-to-size in mJ/cm^2.
    pub dose_mj_cm2: f64,
    /// Wafer diameter in mm.
    pub wafer_diameter_mm: f64,
    /// Exposure-field width across the scan (the slit length) in mm.
    pub field_width_mm: f64,
    /// Exposure-field length along the scan in mm.
    pub field_height_mm: f64,
    /// Illuminated slit height along the scan in mm.
    pub slit_height_mm: f64,
    /// Exposure fields per wafer; `None` counts grid fields whose centre
    /// lies on the wafer (partial edge fields included).
    pub fields_per_wafer: Option<usize>,
    /// Maximum wafer-stage scan speed in mm/s; `None` = purely dose-limited.
    pub max_scan_speed_mm_s: Option<f64>,
    /// Per-field overhead (step, settle, scan reversal) in s.
    pub field_overhead_s: f64,
    /// Per-wafer overhead (exchange, alignment, leveling) in s.
    pub wafer_overhead_s: f64,
}

impl ThroughputParams {
    /// Transmission of a train of `num_mirrors` mirrors of reflectivity `r`.
    pub fn mirror_train(num_mirrors: u32, reflectivity: f64) -> f64 {
        reflectivity.powi(num_mirrors as i32)
    }

    /// EUV (13.5 nm) HVM-like scanner, ILLUSTRATIVE assumptions: 10 Mo/Si
    /// mirrors between IF and wafer (≥2 condenser/illuminator + 6
    /// projection mirrors) at R = 0.70, mask reflectance 0.65 — about 1.8 %
    /// of the IF power reaches the wafer (published estimates: roughly
    /// 1–4 %, e.g. "a dozen reflections at 70 %" ≈ 1.4 %) — 300 mm wafer,
    /// 26 x 33 mm fields, 2 mm slit, 0.1 s per-field and 10 s per-wafer
    /// overheads, no stage speed limit. Gives ~166 wafers/hour at 250 W and
    /// 20 mJ/cm² (the NXE:3400B specification is ≥125 wph, the NXE:3400C
    /// ≥170 wph) and ~154 wph at 30 mJ/cm².
    pub fn euv_hvm_like(dose_mj_cm2: f64) -> Self {
        Self {
            optics_transmission: Self::mirror_train(10, 0.70),
            mask_efficiency: 0.65,
            dose_mj_cm2,
            wafer_diameter_mm: 300.0,
            field_width_mm: 26.0,
            field_height_mm: 33.0,
            slit_height_mm: 2.0,
            fields_per_wafer: None,
            max_scan_speed_mm_s: None,
            field_overhead_s: 0.1,
            wafer_overhead_s: 10.0,
        }
    }

    /// Beyond-EUV (6.x nm) scanner with the same geometry as
    /// [`Self::euv_hvm_like`] but La/B multilayers at the record 64.1 %
    /// reflectance (6.65 nm) for all 10 mirrors and the mask: 0.641^11 ≈
    /// 0.75 % of the IF power reaches the wafer (vs ≈ 1.8 % at 13.5 nm), and
    /// the column bandwidth shrinks from ~2 % to ~0.6 %. ILLUSTRATIVE.
    pub fn beuv_la_b_like(dose_mj_cm2: f64) -> Self {
        Self {
            optics_transmission: Self::mirror_train(10, 0.641),
            mask_efficiency: 0.641,
            ..Self::euv_hvm_like(dose_mj_cm2)
        }
    }

    /// Refractive (VUV/DUV) scanner, ILLUSTRATIVE assumptions: 30 %
    /// laser-to-wafer optics transmission, 90 % mask transmission, 300 mm
    /// wafer, 26 x 33 mm fields, 8 mm slit, same overheads as
    /// [`Self::euv_hvm_like`].
    pub fn refractive_like(dose_mj_cm2: f64) -> Self {
        Self {
            optics_transmission: 0.30,
            mask_efficiency: 0.90,
            slit_height_mm: 8.0,
            ..Self::euv_hvm_like(dose_mj_cm2)
        }
    }
}

/// Result of the dose-limited throughput model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThroughputResult {
    /// Source power fed into the model (W).
    pub source_power_w: f64,
    /// Power reaching the wafer (W).
    pub power_at_wafer_w: f64,
    /// Exposure fields per wafer.
    pub fields_per_wafer: usize,
    /// Exposed area per wafer (cm^2).
    pub exposed_area_cm2: f64,
    /// Scan speed that exactly delivers the dose (mm/s).
    pub dose_limited_scan_speed_mm_s: f64,
    /// Scan speed actually used (mm/s).
    pub scan_speed_mm_s: f64,
    /// `true` if the stage speed limit, not the dose, sets the scan speed.
    pub stage_limited: bool,
    /// Time spent scanning per wafer (s).
    pub exposure_time_per_wafer_s: f64,
    /// Total time per wafer including overheads (s).
    pub total_time_per_wafer_s: f64,
    /// Wafers per hour.
    pub wafers_per_hour: f64,
    /// Source pulses delivered to each exposure point (`None` for CW
    /// sources / unknown repetition rate).
    pub pulses_per_point: Option<f64>,
}

/// Count exposure fields of `field_width_mm` x `field_height_mm` whose
/// centres lie on a wafer of `wafer_diameter_mm`, maximized over the four
/// grid registrations (grid vertex or field centre at the wafer centre).
pub fn count_fields_per_wafer(
    wafer_diameter_mm: f64,
    field_width_mm: f64,
    field_height_mm: f64,
) -> usize {
    if wafer_diameter_mm <= 0.0 || field_width_mm <= 0.0 || field_height_mm <= 0.0 {
        return 0;
    }
    let r = 0.5 * wafer_diameter_mm;
    let nx = (r / field_width_mm).ceil() as i64 + 1;
    let ny = (r / field_height_mm).ceil() as i64 + 1;
    let mut best = 0;
    for ox in [0.0, 0.5] {
        for oy in [0.0, 0.5] {
            let mut n = 0;
            for i in -nx..=nx {
                for j in -ny..=ny {
                    let cx = (i as f64 + ox) * field_width_mm;
                    let cy = (j as f64 + oy) * field_height_mm;
                    if cx.hypot(cy) <= r {
                        n += 1;
                    }
                }
            }
            best = best.max(n);
        }
    }
    best
}

/// Dose-limited throughput for an explicit source power (W) and optional
/// repetition rate (Hz).
pub fn wafer_throughput_from_power(
    source_power_w: f64,
    rep_rate_hz: Option<f64>,
    params: &ThroughputParams,
) -> ThroughputResult {
    let power_at_wafer_w = source_power_w * params.optics_transmission * params.mask_efficiency;
    let fields = params.fields_per_wafer.unwrap_or_else(|| {
        count_fields_per_wafer(
            params.wafer_diameter_mm,
            params.field_width_mm,
            params.field_height_mm,
        )
    });
    // mJ/cm^2 -> J/mm^2
    let dose_j_mm2 = params.dose_mj_cm2 * 1e-3 / 100.0;
    let v_dose = if dose_j_mm2 > 0.0 && params.field_width_mm > 0.0 {
        power_at_wafer_w / (dose_j_mm2 * params.field_width_mm)
    } else {
        f64::INFINITY
    };
    let (v, stage_limited) = match params.max_scan_speed_mm_s {
        Some(v_max) if v_max < v_dose => (v_max, true),
        _ => (v_dose, false),
    };
    let scan_per_field = if v > 0.0 {
        (params.field_height_mm + params.slit_height_mm) / v
    } else {
        f64::INFINITY
    };
    let exposure_time = fields as f64 * scan_per_field;
    let total = exposure_time + fields as f64 * params.field_overhead_s + params.wafer_overhead_s;
    let wph = if total.is_finite() && total > 0.0 {
        3600.0 / total
    } else {
        0.0
    };
    let pulses_per_point = rep_rate_hz
        .filter(|f| *f > 0.0 && v > 0.0 && v.is_finite())
        .map(|f| f * params.slit_height_mm / v);
    ThroughputResult {
        source_power_w,
        power_at_wafer_w,
        fields_per_wafer: fields,
        exposed_area_cm2: fields as f64 * params.field_width_mm * params.field_height_mm / 100.0,
        dose_limited_scan_speed_mm_s: v_dose,
        scan_speed_mm_s: v,
        stage_limited,
        exposure_time_per_wafer_s: exposure_time,
        total_time_per_wafer_s: total,
        wafers_per_hour: wph,
        pulses_per_point,
    }
}

/// Dose-limited throughput for any source, from its
/// [`LithographySource::average_power_w`]. `None` if the source does not
/// report an average power (e.g. bending-magnet beamlines).
pub fn wafer_throughput(
    source: &(impl LithographySource + ?Sized),
    params: &ThroughputParams,
) -> Option<ThroughputResult> {
    let power = source.average_power_w()?;
    Some(wafer_throughput_from_power(
        power,
        source.rep_rate_hz(),
        params,
    ))
}

/// Mean number of photons delivered into a square of side `side_nm` by a
/// dose `dose_mj_cm2` at `wavelength_nm`: `N = D a^2 / (h c / lambda)`.
/// E.g. 30 mJ/cm^2 at 13.5 nm into a (16 nm)^2 pixel: ~5200 photons.
pub fn photons_per_square(dose_mj_cm2: f64, wavelength_nm: f64, side_nm: f64) -> f64 {
    // mJ/cm^2 -> J/nm^2: 1e-3 J / 1e14 nm^2
    let dose_j_nm2 = dose_mj_cm2 * 1e-3 * 1e-14;
    dose_j_nm2 * side_nm * side_nm / physics::photon_energy_j(wavelength_nm)
}

/// Relative Poisson shot noise `1/sqrt(N)` of the photon count in a square
/// of side `side_nm` (see [`photons_per_square`]).
pub fn relative_shot_noise(dose_mj_cm2: f64, wavelength_nm: f64, side_nm: f64) -> f64 {
    let n = photons_per_square(dose_mj_cm2, wavelength_nm, side_nm);
    if n > 0.0 {
        1.0 / n.sqrt()
    } else {
        f64::INFINITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_field_count_300mm() {
        // Independently counted (Python grid scan): 84 fields of 26 x 33 mm
        // with centres on a 300 mm wafer.
        assert_eq!(count_fields_per_wafer(300.0, 26.0, 33.0), 84);
        assert_eq!(count_fields_per_wafer(0.0, 26.0, 33.0), 0);
        // Smaller fields -> more of them.
        assert!(count_fields_per_wafer(300.0, 13.0, 16.5) > 4 * 80);
    }

    #[test]
    fn test_euv_hvm_like_fixture() {
        // 250 W at IF, 0.7^10 x 0.65 -> 4.590 W at the wafer.
        let p = ThroughputParams::euv_hvm_like(30.0);
        let r = wafer_throughput_from_power(250.0, Some(50_000.0), &p);
        assert_relative_eq!(
            r.power_at_wafer_w,
            4.590_222_796_249_997,
            max_relative = 1e-9
        );
        // v = P_w / (D W) = 4.5902 / (3e-4 J/mm^2 * 26 mm) = 588.5 mm/s
        assert_relative_eq!(
            r.scan_speed_mm_s,
            4.590_222_796_249_997 / (3e-4 * 26.0),
            max_relative = 1e-12
        );
        assert!(!r.stage_limited);
        // 84 fields x (35 mm / v) + 84 x 0.1 s + 10 s
        let t = 84.0 * 35.0 / r.scan_speed_mm_s + 8.4 + 10.0;
        assert_relative_eq!(r.total_time_per_wafer_s, t, max_relative = 1e-12);
        assert_relative_eq!(r.wafers_per_hour, 3600.0 / t, max_relative = 1e-12);
        assert!(
            (140.0..170.0).contains(&r.wafers_per_hour),
            "{}",
            r.wafers_per_hour
        );
        // 50 kHz x 2 mm / 588.5 mm/s = 170 pulses per point.
        assert_relative_eq!(
            r.pulses_per_point.unwrap(),
            50_000.0 * 2.0 / r.scan_speed_mm_s,
            max_relative = 1e-12
        );
        assert_relative_eq!(r.exposed_area_cm2, 84.0 * 8.58, max_relative = 1e-12);
    }

    #[test]
    fn test_beuv_column_transmission() {
        // 0.641^11 = 0.75 % (record La/B mirrors) vs 0.70^10 x 0.65 = 1.84 %.
        let beuv = ThroughputParams::beuv_la_b_like(30.0);
        let euv = ThroughputParams::euv_hvm_like(30.0);
        let t_beuv = beuv.optics_transmission * beuv.mask_efficiency;
        assert_relative_eq!(t_beuv, 0.641_f64.powi(11), max_relative = 1e-12);
        assert!((0.0074..0.0076).contains(&t_beuv));
        let t_euv = euv.optics_transmission * euv.mask_efficiency;
        assert!((0.01..0.04).contains(&t_euv), "EUV column {t_euv}");
        // At 250 W and 20 mJ/cm^2 the EUV-like model gives ~166 wph (NXE:3400B
        // spec >= 125, NXE:3400C >= 170).
        let r = wafer_throughput_from_power(250.0, None, &ThroughputParams::euv_hvm_like(20.0));
        assert!(
            (150.0..180.0).contains(&r.wafers_per_hour),
            "{}",
            r.wafers_per_hour
        );
    }

    #[test]
    fn test_throughput_scaling_limits() {
        let p = ThroughputParams::euv_hvm_like(30.0);
        // Low power: exposure dominates, doubling power nearly doubles WPH.
        let lo = wafer_throughput_from_power(1.0, None, &p);
        let lo2 = wafer_throughput_from_power(2.0, None, &p);
        assert!(lo2.wafers_per_hour / lo.wafers_per_hour > 1.9);
        // High power: overhead-limited, WPH saturates at 3600 / overheads.
        let hi = wafer_throughput_from_power(1e6, None, &p);
        let ceiling = 3600.0 / (84.0 * 0.1 + 10.0);
        assert!(hi.wafers_per_hour < ceiling && hi.wafers_per_hour > 0.99 * ceiling);
        // Doubling the dose halves the dose-limited scan speed.
        let d2 = wafer_throughput_from_power(250.0, None, &ThroughputParams::euv_hvm_like(60.0));
        let d1 = wafer_throughput_from_power(250.0, None, &p);
        assert_relative_eq!(
            d2.dose_limited_scan_speed_mm_s,
            d1.dose_limited_scan_speed_mm_s / 2.0,
            max_relative = 1e-12
        );
        // A stage limit caps the speed.
        let capped = wafer_throughput_from_power(
            250.0,
            None,
            &ThroughputParams {
                max_scan_speed_mm_s: Some(300.0),
                ..p.clone()
            },
        );
        assert!(capped.stage_limited);
        assert_relative_eq!(capped.scan_speed_mm_s, 300.0);
        // Zero power -> zero throughput, no NaN.
        let zero = wafer_throughput_from_power(0.0, Some(1e3), &p);
        assert_eq!(zero.wafers_per_hour, 0.0);
        assert!(zero.pulses_per_point.is_none());
    }

    #[test]
    fn test_photons_per_square_fixture() {
        // 30 mJ/cm^2 = 3e-16 J/nm^2; (16 nm)^2 at 13.5 nm (1.4714e-17 J):
        // 3e-16 * 256 / 1.4714e-17 = 5219.6 photons, 1.38 % shot noise.
        let n = photons_per_square(30.0, 13.5, 16.0);
        assert_relative_eq!(
            n,
            3e-16 * 256.0 / (1239.84193 / 13.5 * 1.602176634e-19),
            max_relative = 1e-12
        );
        assert!((5200.0..5240.0).contains(&n));
        assert_relative_eq!(relative_shot_noise(30.0, 13.5, 16.0), 1.0 / n.sqrt());
        // Same dose at 157.63 nm carries 11.7x more photons.
        assert_relative_eq!(
            photons_per_square(30.0, 157.63, 16.0) / n,
            157.63 / 13.5,
            max_relative = 1e-12
        );
    }

    #[test]
    fn test_throughput_through_trait() {
        struct Cw(f64);
        impl LithographySource for Cw {
            fn wavelength_nm(&self) -> f64 {
                13.5
            }
            fn bandwidth_pm(&self) -> f64 {
                0.0
            }
            fn intensity_at(&self, _: f64, _: f64) -> f64 {
                1.0
            }
            fn spectral_weights(&self) -> Vec<(f64, f64)> {
                vec![(13.5, 1.0)]
            }
            fn average_power_w(&self) -> Option<f64> {
                Some(self.0)
            }
        }
        struct NoPower;
        impl LithographySource for NoPower {
            fn wavelength_nm(&self) -> f64 {
                13.5
            }
            fn bandwidth_pm(&self) -> f64 {
                0.0
            }
            fn intensity_at(&self, _: f64, _: f64) -> f64 {
                1.0
            }
            fn spectral_weights(&self) -> Vec<(f64, f64)> {
                vec![(13.5, 1.0)]
            }
        }
        let p = ThroughputParams::euv_hvm_like(20.0);
        let r = wafer_throughput(&Cw(250.0), &p).unwrap();
        assert_eq!(r.pulses_per_point, None); // CW
        assert_relative_eq!(r.source_power_w, 250.0);
        assert!(wafer_throughput(&NoPower, &p).is_none());
    }
}
