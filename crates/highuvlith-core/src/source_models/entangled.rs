//! Entangled-photon (NOON-state) illumination source — THEORETICAL.
//!
//! N-photon path-entangled NOON states accumulate phase N times faster
//! than classical light, so an interferometric exposure writes fringes
//! at an effective wavelength lambda/N — beating the classical
//! resolution limit by N (Boto et al., Phys. Rev. Lett. 85, 2733
//! (2000)). This source model is the illumination-side bridge to the
//! existing [`crate::quantum`] research module, which provides classical
//! N-photon absorption `I^N` of a classical aerial image (sharpening at the
//! classical period), the ideal N00N models (analytic two-beam fringes at
//! lambda/(2N sin(theta)); lambda/N ideal-limit images for general masks,
//! [`crate::quantum::noon_ideal_image`]) and a flux budget derived from
//! this module's constants ([`crate::quantum::FluxBudget`]).
//!
//! # Model status
//!
//! Entirely theoretical (matching `quantum.rs`): no N-photon resist
//! chemistry exists at lithographic wavelengths, SPDC pair rates are
//! many orders of magnitude below dose requirements, and the pipeline
//! images this source CLASSICALLY at its physical wavelength — quantum
//! N-photon models must be applied explicitly via
//! [`EntangledPhotonSource::quantum_params`] and the [`crate::quantum`]
//! functions. The stochastic module does
//! NOT model N-photon absorption statistics.
//!
//! No entangled (NOON) sub-Rayleigh pattern has ever been recorded in a
//! material: every entangled demonstration used coincidence detection
//! (D'Angelo et al. 2001; Kawabe et al. 2007; Rosen et al. 2012; Rozema et
//! al. 2014); the only material-recorded sub-Rayleigh fringes used classical
//! light in PMMA (Chang et al. 2006).
//!
//! The derived quantities turn `pair_rate_hz` into the exposure time needed
//! to clear a resist by entangled two-photon absorption (ETPA),
//! `t = p_clear / (sigma_E Phi)`. The ETPA cross-section is disputed: early
//! reports claimed 1e-17–1e-21 cm^2, while independent measurements
//! (Parzuchowski et al. 2021, 2025; He et al. 2024) found only upper bounds
//! of 1e-23–1e-25 cm^2, alongside several null results (Landes et al.
//! 2021/2024; Mikhaylov et al. 2022; Corona-Aquino et al. 2022; Hickam et
//! al. 2022). The DEFAULT scenario uses the bound range (so its exposure
//! times are LOWER bounds); the claims are reported as an optimistic
//! scenario. No data exist for N >= 3; the N = 2 numbers are used as
//! optimistic stand-ins. Pairs also only behave as isolated entangled pairs
//! below about one photon per spectral mode, a crossover flux of roughly
//! the down-converted bandwidth in Hz (Dayan et al., Phys. Rev. Lett. 94,
//! 043602 (2005), Eq. (1): ~1e12 pairs/s, 0.3 µW, generated below a
//! crossover of 8.2e12 photons/s, ~1.5 µW, for a 31 nm band at 1064 nm),
//! versus ≈0.67 W at the wafer of an NXE:3400C at 170 wph and 20 mJ/cm²
//! (derived) — roughly six orders of magnitude more power.
//!
//! # Key equations
//!
//! - effective wavelength `lambda / N`
//! - linear-in-flux ETPA rate per molecule `R = sigma_E Phi`
//!   (`Phi` = entangled-pair flux, pairs cm^-2 s^-1)
//! - clearing fluence `F = p_clear / sigma_E`, exposure time `t = F A / R_pairs`

use serde::{Deserialize, Serialize};

use super::physics;
use crate::quantum::QuantumLithographyParams;
use crate::source::{
    evaluate_illumination, evaluate_spectral_weights, DerivedQuantity, IlluminationShape,
    LithographySource, SpectralShape,
};

/// Most optimistic early-claim ETPA cross-section (cm^2). Early claims
/// spanned 1e-17–1e-21 cm^2 and are DISPUTED.
pub const ETPA_CROSS_SECTION_CLAIMED_MAX_CM2: f64 = 1e-17;
/// Least optimistic early-claim ETPA cross-section (cm^2).
pub const ETPA_CROSS_SECTION_CLAIMED_MIN_CM2: f64 = 1e-21;
/// Loosest independent upper bound on the ETPA cross-section (cm^2) — the
/// DEFAULT scenario; the true value may be far smaller.
pub const ETPA_CROSS_SECTION_UPPER_BOUND_CM2: f64 = 1e-23;
/// Tightest independent upper bound on the ETPA cross-section (cm^2).
pub const ETPA_CROSS_SECTION_TIGHTEST_BOUND_CM2: f64 = 1e-25;
/// Order of magnitude of the entangled-regime crossover flux (photons/s):
/// above about one photon per spectral mode, `Phi_max ~ Δν` of the
/// down-converted light, pairs overlap and two-photon processes turn
/// classical (quadratic). Dayan et al., Phys. Rev. Lett. 94, 043602 (2005),
/// Eq. (1), report a crossover of 8.2e12 photons/s (~1.5 µW) for a 31 nm
/// band at 1064 nm and ran ~1e12 pairs/s (0.3 µW). The crossover scales
/// with the bandwidth; 1e13 is a representative broadband value, not a
/// universal limit.
pub const ENTANGLED_FLUX_CEILING_PHOTONS_S: f64 = 1e13;
/// EUV power at the wafer of an NXE:3400C (≥170 wph at 20 mJ/cm²), in W —
/// a DERIVED reference figure for the HVM photon-rate comparison.
pub const HVM_WAFER_POWER_W: f64 = 0.67;
/// ASSUMED fraction of resist molecules that must react to clear
/// (chemically-amplified-like, optimistic).
pub const CLEARING_ACTIVATION_PROBABILITY: f64 = 0.1;
/// Reference exposure area for the exposure-time figures (cm^2).
pub const REFERENCE_EXPOSURE_AREA_CM2: f64 = 1.0;

/// N-photon entangled (NOON-state) source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntangledPhotonSource {
    /// Physical photon wavelength in nm. (Degenerate SPDC pairs emerge
    /// at twice the pump wavelength; this field is the *photon*
    /// wavelength, not the pump.)
    pub wavelength_nm: f64,
    /// Number of entangled photons N in the NOON state (N = 2 biphoton).
    pub num_entangled_photons: usize,
    /// Entanglement fidelity in [0, 1]: the share of the N-photon exposure
    /// from the ideal N00N component in the quantum module (the rest is
    /// classical N-photon absorption).
    pub fidelity: f64,
    /// Entangled N-tuple generation rate in Hz (SPDC sources: kHz-MHz).
    /// Enters the derived exposure-time estimates only; the quantum
    /// module's flux budget is set by the entangled-regime flux ceiling,
    /// not by this rate.
    pub pair_rate_hz: f64,
    /// Spectral bandwidth FWHM in pm.
    pub bandwidth_pm: f64,
    /// Number of spectral samples.
    pub spectral_samples: usize,
    /// Illumination pupil (interferometric setups are fully coherent).
    pub illumination: IlluminationShape,
}

impl EntangledPhotonSource {
    /// Create an N-photon NOON source at the given physical wavelength.
    pub fn noon(
        wavelength_nm: f64,
        num_entangled_photons: usize,
        fidelity: f64,
    ) -> crate::error::Result<Self> {
        if wavelength_nm <= 0.0 || wavelength_nm.is_nan() {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "wavelength_nm",
                value: wavelength_nm,
                reason: "must be positive",
            });
        }
        if num_entangled_photons < 2 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "num_entangled_photons",
                value: num_entangled_photons as f64,
                reason: "NOON states need N >= 2 (N = 1 is classical light)",
            });
        }
        if !(0.0..=1.0).contains(&fidelity) || fidelity.is_nan() {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "fidelity",
                value: fidelity,
                reason: "must be in [0, 1]",
            });
        }
        Ok(Self {
            wavelength_nm,
            num_entangled_photons,
            fidelity,
            pair_rate_hz: 1e6,
            bandwidth_pm: wavelength_nm * 1e-3,
            spectral_samples: 3,
            illumination: IlluminationShape::CoherentGaussian { sigma: 0.05 },
        })
    }

    /// Bridge to the quantum research module: the parameter set behind
    /// its N-photon image models and flux budget
    /// ([`QuantumLithographyParams::flux_budget`]), at the given imaging NA.
    pub fn quantum_params(&self, na: f64) -> QuantumLithographyParams {
        QuantumLithographyParams {
            num_entangled_photons: self.num_entangled_photons,
            wavelength_nm: self.wavelength_nm,
            na,
            fidelity: self.fidelity,
        }
    }

    /// Effective interferometric wavelength lambda / N.
    pub fn effective_wavelength_nm(&self) -> f64 {
        self.wavelength_nm / self.num_entangled_photons as f64
    }

    /// Exposure time (s) to clear `area_cm2` of resist by entangled
    /// N-photon absorption with cross-section `cross_section_cm2`:
    /// `t = (p_clear / sigma_E) A / R_pairs`. Infinite for a zero pair rate.
    pub fn clearing_time_s(&self, cross_section_cm2: f64, area_cm2: f64) -> f64 {
        if self.pair_rate_hz <= 0.0 || cross_section_cm2 <= 0.0 {
            return f64::INFINITY;
        }
        CLEARING_ACTIVATION_PROBABILITY / cross_section_cm2 * area_cm2 / self.pair_rate_hz
    }
}

impl Default for EntangledPhotonSource {
    fn default() -> Self {
        Self::noon(157.63, 2, 1.0).expect("default NOON parameters are valid")
    }
}

impl LithographySource for EntangledPhotonSource {
    /// The PHYSICAL wavelength: the pipeline images classically; N-photon
    /// models are applied explicitly through the quantum module.
    fn wavelength_nm(&self) -> f64 {
        self.wavelength_nm
    }

    fn bandwidth_pm(&self) -> f64 {
        self.bandwidth_pm
    }

    fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64 {
        evaluate_illumination(&self.illumination, fx_norm, fy_norm)
    }

    fn spectral_weights(&self) -> Vec<(f64, f64)> {
        evaluate_spectral_weights(
            self.wavelength_nm,
            self.bandwidth_pm,
            self.spectral_samples,
            &SpectralShape::Gaussian,
        )
    }

    fn transverse_coherence(&self) -> f64 {
        1.0
    }

    fn derived_quantities(&self) -> Vec<DerivedQuantity> {
        let lambda = self.wavelength_nm;
        let area = REFERENCE_EXPOSURE_AREA_CM2;
        let n_note = if self.num_entangled_photons == 2 {
            ""
        } else {
            " (no N >= 3 data: N = 2 cross-section used as an optimistic stand-in)"
        };
        let fluence = |sigma: f64| CLEARING_ACTIVATION_PROBABILITY / sigma;
        let e_ph = physics::photon_energy_j(lambda);
        // A classical 30 mJ/cm^2 exposure at the physical wavelength.
        let classical_photons = 30e-3 / e_ph;
        let power = self.pair_rate_hz * self.num_entangled_photons as f64 * e_ph;
        let hvm_rate = HVM_WAFER_POWER_W / e_ph;
        vec![
            DerivedQuantity::new(
                "effective_wavelength",
                self.effective_wavelength_nm(),
                "nm",
                "lambda / N (interferometric fringe period scale)",
            ),
            DerivedQuantity::new(
                "photon_power",
                power,
                "W",
                "pair rate x N x photon energy (optical power in the entangled beam)",
            ),
            DerivedQuantity::new(
                "clearing_fluence_bound",
                fluence(ETPA_CROSS_SECTION_UPPER_BOUND_CM2),
                "pairs/cm^2",
                &format!(
                    "p_clear / sigma_E at the 1e-23 cm^2 upper bound (DEFAULT scenario): a \
                     LOWER bound on the fluence{n_note}"
                ),
            ),
            DerivedQuantity::new(
                "exposure_time_bound",
                self.clearing_time_s(ETPA_CROSS_SECTION_UPPER_BOUND_CM2, area),
                "s",
                "DEFAULT: time to clear 1 cm^2 at the stored pair rate with sigma_E at \
                 its loosest independent bound (1e-23 cm^2) - a LOWER bound on the time",
            ),
            DerivedQuantity::new(
                "exposure_time_bound_tight",
                self.clearing_time_s(ETPA_CROSS_SECTION_TIGHTEST_BOUND_CM2, area),
                "s",
                "same with the tightest independent bound (1e-25 cm^2)",
            ),
            DerivedQuantity::new(
                "clearing_fluence_claims_optimistic",
                fluence(ETPA_CROSS_SECTION_CLAIMED_MAX_CM2),
                "pairs/cm^2",
                &format!("OPTIMISTIC scenario: disputed early claim sigma_E = 1e-17 cm^2{n_note}"),
            ),
            DerivedQuantity::new(
                "exposure_time_claims_optimistic",
                self.clearing_time_s(ETPA_CROSS_SECTION_CLAIMED_MAX_CM2, area),
                "s",
                "OPTIMISTIC scenario: time to clear 1 cm^2 with the largest disputed claim \
                 (1e-17 cm^2)",
            ),
            DerivedQuantity::new(
                "exposure_time_claims_pessimistic",
                self.clearing_time_s(ETPA_CROSS_SECTION_CLAIMED_MIN_CM2, area),
                "s",
                "optimistic-scenario range, smallest early claim (1e-21 cm^2)",
            ),
            DerivedQuantity::new(
                "pair_rate_for_1s_bound",
                fluence(ETPA_CROSS_SECTION_UPPER_BOUND_CM2) * area,
                "pairs/s",
                "pair rate needed to clear 1 cm^2 in 1 s at the sigma_E upper bound",
            ),
            DerivedQuantity::new(
                "entangled_flux_ceiling",
                ENTANGLED_FLUX_CEILING_PHOTONS_S,
                "photons/s",
                "order of the entangled-regime crossover (one photon per spectral mode, \
                 Phi_max ~ bandwidth in Hz): 8.2e12 photons/s for a 31 nm band at 1064 nm, \
                 Dayan et al., PRL 94, 043602 (2005), Eq. (1); scales with bandwidth",
            ),
            DerivedQuantity::new(
                "hvm_wafer_photon_rate",
                hvm_rate,
                "photons/s",
                "0.67 W (NXE:3400C at the wafer, derived) at this photon energy",
            ),
            DerivedQuantity::new(
                "hvm_rate_over_flux_ceiling",
                hvm_rate / ENTANGLED_FLUX_CEILING_PHOTONS_S,
                "-",
                "HVM wafer photon rate / entangled-regime ceiling",
            ),
            DerivedQuantity::new(
                "classical_photons_30mj",
                classical_photons,
                "photons/cm^2",
                "photons in a classical 30 mJ/cm^2 dose at the physical wavelength",
            ),
            DerivedQuantity::new(
                "classical_rate_gap",
                if self.pair_rate_hz > 0.0 {
                    classical_photons / self.pair_rate_hz
                } else {
                    f64::INFINITY
                },
                "-",
                "classical photons per cm^2 per second of a 30 mJ/cm^2/s exposure / pair rate",
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn dq(src: &EntangledPhotonSource, name: &str) -> f64 {
        src.derived_quantities()
            .into_iter()
            .find(|q| q.name == name)
            .unwrap_or_else(|| panic!("missing derived quantity {name}"))
            .value
    }

    #[test]
    fn test_quantum_params_round_trip() {
        let src = EntangledPhotonSource::noon(157.63, 4, 0.9).unwrap();
        let params = src.quantum_params(0.75);
        assert_eq!(params.num_entangled_photons, 4);
        assert_relative_eq!(params.wavelength_nm, 157.63);
        assert_relative_eq!(params.na, 0.75);
        assert_relative_eq!(params.fidelity, 0.9);
    }

    #[test]
    fn test_n2_halves_effective_wavelength() {
        let src = EntangledPhotonSource::noon(157.63, 2, 1.0).unwrap();
        assert_relative_eq!(src.effective_wavelength_nm(), 157.63 / 2.0, epsilon = 1e-12);
        // The trait reports the PHYSICAL wavelength.
        assert_relative_eq!(src.wavelength_nm(), 157.63);
    }

    #[test]
    fn test_invalid_parameters_rejected() {
        assert!(EntangledPhotonSource::noon(0.0, 2, 1.0).is_err());
        assert!(EntangledPhotonSource::noon(157.63, 1, 1.0).is_err());
        assert!(EntangledPhotonSource::noon(157.63, 2, 1.5).is_err());
        assert!(EntangledPhotonSource::noon(157.63, 2, -0.1).is_err());
    }

    #[test]
    fn test_weights_sum_to_one() {
        let src = EntangledPhotonSource::noon(157.63, 2, 1.0).unwrap();
        let sum: f64 = src.spectral_weights().iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_fully_coherent() {
        let src = EntangledPhotonSource::default();
        assert_relative_eq!(src.transverse_coherence(), 1.0);
    }

    #[test]
    fn test_exposure_time_range() {
        let src = EntangledPhotonSource::default();
        // DEFAULT (bounds): p = 0.1 / 1e-23 = 1e22 pairs/cm^2 -> >= 1e16 s at
        // 1e6 pairs/s (~3e8 years); tightest bound 1e-25 -> 1e18 s.
        assert_relative_eq!(
            dq(&src, "clearing_fluence_bound"),
            1e22,
            max_relative = 1e-12
        );
        assert_relative_eq!(dq(&src, "exposure_time_bound"), 1e16, max_relative = 1e-12);
        assert_relative_eq!(
            dq(&src, "exposure_time_bound_tight"),
            1e18,
            max_relative = 1e-12
        );
        // OPTIMISTIC (disputed claims 1e-17..1e-21): 1e10 s .. 1e14 s.
        assert_relative_eq!(
            dq(&src, "exposure_time_claims_optimistic"),
            1e10,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            dq(&src, "exposure_time_claims_pessimistic"),
            1e14,
            max_relative = 1e-12
        );
        assert!(dq(&src, "exposure_time_bound") > dq(&src, "exposure_time_claims_pessimistic"));
        // Exposure time scales inversely with pair rate.
        let bright = EntangledPhotonSource {
            pair_rate_hz: 1e9,
            ..src.clone()
        };
        assert_relative_eq!(
            bright.clearing_time_s(ETPA_CROSS_SECTION_UPPER_BOUND_CM2, 1.0),
            1e13,
            max_relative = 1e-12
        );
        // Classical 30 mJ/cm^2 at 157.63 nm: 2.38e16 photons/cm^2 (scipy fixture).
        assert_relative_eq!(
            dq(&src, "classical_photons_30mj"),
            2.380_583_487_946_524e16,
            max_relative = 1e-7
        );
        // HVM wafer photon rate at 157.63 nm vs the 1e13 photons/s ceiling.
        let rate = 0.67 / (1239.84193 / 157.63 * 1.602176634e-19);
        assert_relative_eq!(
            dq(&src, "hvm_wafer_photon_rate"),
            rate,
            max_relative = 1e-12
        );
        assert!(dq(&src, "hvm_rate_over_flux_ceiling") > 1e4);
        // Zero pair rate -> infinite time, no NaN.
        let dark = EntangledPhotonSource {
            pair_rate_hz: 0.0,
            ..src
        };
        assert!(dark
            .clearing_time_s(ETPA_CROSS_SECTION_UPPER_BOUND_CM2, 1.0)
            .is_infinite());
    }
}
