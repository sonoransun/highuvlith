//! Entangled-photon (NOON-state) illumination source — THEORETICAL.
//!
//! N-photon path-entangled NOON states accumulate phase N times faster
//! than classical light, so an interferometric exposure writes fringes
//! at an effective wavelength lambda/N — beating the classical
//! resolution limit by N (Boto et al., Phys. Rev. Lett. 85, 2733
//! (2000)). This source model is the illumination-side bridge to the
//! existing [`crate::quantum`] research module, which applies the
//! N-photon intensity sharpening, fidelity mixing, and the eta^(N-1)
//! flux penalty.
//!
//! # Model status
//!
//! Entirely theoretical (matching `quantum.rs`): no N-photon resist
//! chemistry exists at lithographic wavelengths, SPDC pair rates are
//! ~12 orders of magnitude below dose requirements, and the pipeline
//! images this source CLASSICALLY at its physical wavelength — quantum
//! sharpening must be applied explicitly via
//! [`EntangledPhotonSource::quantum_params`] and
//! `quantum::compute_quantum_aerial_image`. The stochastic module does
//! NOT model N-photon absorption statistics.

use serde::{Deserialize, Serialize};

use crate::quantum::QuantumLithographyParams;
use crate::source::{
    evaluate_illumination, evaluate_spectral_weights, IlluminationShape, LithographySource,
    SpectralShape,
};

/// N-photon entangled (NOON-state) source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntangledPhotonSource {
    /// Physical photon wavelength in nm. (Degenerate SPDC pairs emerge
    /// at twice the pump wavelength; this field is the *photon*
    /// wavelength, not the pump.)
    pub wavelength_nm: f64,
    /// Number of entangled photons N in the NOON state (N = 2 biphoton).
    pub num_entangled_photons: usize,
    /// Entanglement fidelity in [0, 1]; degrades the quantum
    /// interference contrast via the quantum module's fidelity mixing.
    pub fidelity: f64,
    /// Entangled N-tuple generation rate in Hz (SPDC sources: kHz-MHz —
    /// documentary; the flux penalty lives in the quantum module).
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

    /// Bridge to the quantum research module: the parameter set that
    /// `quantum::compute_quantum_aerial_image` consumes, at the given
    /// imaging NA.
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
}

impl Default for EntangledPhotonSource {
    fn default() -> Self {
        Self::noon(157.63, 2, 1.0).expect("default NOON parameters are valid")
    }
}

impl LithographySource for EntangledPhotonSource {
    /// The PHYSICAL wavelength: the pipeline images classically; quantum
    /// sharpening is applied explicitly through the quantum module.
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

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
}
