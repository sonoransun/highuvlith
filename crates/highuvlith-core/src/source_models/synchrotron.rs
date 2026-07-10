//! Synchrotron radiation sources: bending magnets and undulators.
//!
//! Storage-ring synchrotron radiation spans the VUV-to-hard-X-ray range
//! with high average power and excellent stability. Bending magnets emit
//! a smooth broadband spectrum characterized entirely by the critical
//! energy `E_c = 0.665 E^2[GeV^2] B[T]`; undulators emit quasi-
//! monochromatic harmonics at `lambda_n = lambda_u (1 + K^2/2) / (2 n gamma^2)`.
//!
//! The **wavelength is DERIVED from machine parameters** here (electron
//! energy, field, undulator period/strength) — not stored as a free
//! number. This is the live-physics counterpart to what the LPA-FEL
//! model only documents.
//!
//! Bending magnets are the canonical exposure source for LIGA deep X-ray
//! lithography. The deep-X-ray module couples via
//! `deep_xray::XraySpectrum::from_synchrotron`, which reads
//! [`SynchrotronSource::critical_energy_kev`] and then samples
//! `physics::bm_universal_flux` directly;
//! [`SynchrotronSource::bm_spectral_flux`] is a convenience wrapper over
//! the same universal function for external callers.
//!
//! # Model status
//!
//! Implemented, textbook-exact and fixture-tested: undulator resonance,
//! critical energy, universal flux function S(y) (Kostroun algorithm).
//! Simplified: the undulator line is sampled with a Gaussian of the
//! natural width 1/(nN) rather than the true sinc^2; ring current scales
//! documentation-level flux only.

use serde::{Deserialize, Serialize};

use super::physics;
use crate::source::{
    evaluate_illumination, evaluate_spectral_weights, sigma_from_coherence, IlluminationShape,
    LithographySource, SpectralShape,
};

/// Which beamline of the storage ring feeds the exposure tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SynchrotronBeamline {
    /// Bending-magnet beamline: smooth broadband spectrum with critical
    /// energy E_c; a monochromator selects the working wavelength for
    /// projection imaging (LIGA uses the full white beam instead).
    BendingMagnet {
        /// Ring electron energy in GeV.
        electron_energy_gev: f64,
        /// Bending field in tesla.
        field_t: f64,
        /// Monochromator-selected wavelength in nm (for projection use).
        selected_wavelength_nm: f64,
        /// Monochromator bandwidth FWHM in pm.
        mono_bandwidth_pm: f64,
    },
    /// Undulator beamline: quasi-monochromatic odd harmonics; the
    /// wavelength is derived from the resonance condition.
    Undulator {
        /// Ring electron energy in GeV.
        electron_energy_gev: f64,
        /// Undulator period in mm.
        period_mm: f64,
        /// Dimensionless undulator strength K.
        k: f64,
        /// Number of undulator periods (sets the natural bandwidth 1/(nN)).
        num_periods: usize,
        /// Odd harmonic number (1, 3, 5, ...).
        harmonic: usize,
    },
}

/// Storage-ring synchrotron source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynchrotronSource {
    /// Beamline type and machine parameters.
    pub beamline: SynchrotronBeamline,
    /// Stored ring current in mA (scales flux; documentary for imaging).
    pub ring_current_ma: f64,
    /// Number of spectral samples for polychromatic simulation.
    pub spectral_samples: usize,
    /// Illumination pupil shape. Bending magnets are incoherent
    /// (large sigma disk); undulators are partially coherent.
    pub illumination: IlluminationShape,
    /// Transverse coherence fraction in [0, 1] (undulator ~0.2 at EUV;
    /// bending magnet ~0).
    pub transverse_coherence_fraction: f64,
}

impl SynchrotronSource {
    /// LIGA-class bending magnet: 2.5 GeV ring, 1.5 T field
    /// (E_c = 6.23 keV, lambda_c = 0.199 nm) — KIT/ANKA-class parameters.
    /// The monochromator selection is only used if this source is fed to
    /// the projection pipeline; LIGA shadow printing consumes the full
    /// white-beam spectrum via [`Self::bm_spectral_flux`].
    pub fn liga_bending_magnet() -> Self {
        Self {
            beamline: SynchrotronBeamline::BendingMagnet {
                electron_energy_gev: 2.5,
                field_t: 1.5,
                selected_wavelength_nm: 0.2,
                mono_bandwidth_pm: 0.2,
            },
            ring_current_ma: 200.0,
            spectral_samples: 5,
            illumination: IlluminationShape::Conventional { sigma: 0.8 },
            transverse_coherence_fraction: 0.0,
        }
    }

    /// Compact EUV undulator: 538 MeV ring, 20 mm period, K = 1,
    /// 100 periods, first harmonic -> 13.5 nm with ~1% natural bandwidth.
    pub fn compact_euv_undulator() -> crate::error::Result<Self> {
        Self::undulator(0.538, 20.0, 1.0, 100, 1)
    }

    /// Construct an undulator beamline; the emitted wavelength is derived
    /// from the resonance condition. Rejects even harmonics (on-axis
    /// undulator emission contains odd harmonics only).
    pub fn undulator(
        electron_energy_gev: f64,
        period_mm: f64,
        k: f64,
        num_periods: usize,
        harmonic: usize,
    ) -> crate::error::Result<Self> {
        if harmonic == 0 || harmonic.is_multiple_of(2) {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "harmonic",
                value: harmonic as f64,
                reason: "on-axis undulator harmonics must be odd (1, 3, 5, ...)",
            });
        }
        for (name, value) in [
            ("electron_energy_gev", electron_energy_gev),
            ("period_mm", period_mm),
            ("k", k),
        ] {
            if value <= 0.0 || value.is_nan() {
                return Err(crate::error::LithographyError::InvalidParameter {
                    name,
                    value,
                    reason: "must be positive",
                });
            }
        }
        if num_periods == 0 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "num_periods",
                value: 0.0,
                reason: "must be positive",
            });
        }
        let coherence = 0.2;
        Ok(Self {
            beamline: SynchrotronBeamline::Undulator {
                electron_energy_gev,
                period_mm,
                k,
                num_periods,
                harmonic,
            },
            ring_current_ma: 200.0,
            spectral_samples: 5,
            illumination: IlluminationShape::CoherentGaussian {
                sigma: sigma_from_coherence(coherence, 0.05),
            },
            transverse_coherence_fraction: coherence,
        })
    }

    /// Bending-magnet critical energy in keV, if this is a bending-magnet
    /// beamline (`None` for undulators). Feeds the LIGA depth-dose module.
    pub fn critical_energy_kev(&self) -> Option<f64> {
        match &self.beamline {
            SynchrotronBeamline::BendingMagnet {
                electron_energy_gev,
                field_t,
                ..
            } => Some(physics::critical_energy_kev(*electron_energy_gev, *field_t)),
            SynchrotronBeamline::Undulator { .. } => None,
        }
    }

    /// Universal bending-magnet flux function S(y) at y = E / E_c
    /// (photon flux per unit relative bandwidth, arbitrary units).
    /// Returns 0 for undulator beamlines. Convenience wrapper over
    /// `physics::bm_universal_flux`; the LIGA depth-dose module couples
    /// through `deep_xray::XraySpectrum::from_synchrotron` (which reads
    /// `critical_energy_kev()`) rather than calling this method.
    pub fn bm_spectral_flux(&self, photon_energy_kev: f64) -> f64 {
        match self.critical_energy_kev() {
            Some(e_c) if e_c > 0.0 => physics::bm_universal_flux(photon_energy_kev / e_c),
            _ => 0.0,
        }
    }
}

impl Default for SynchrotronSource {
    fn default() -> Self {
        Self::compact_euv_undulator().expect("compact undulator preset is valid")
    }
}

impl LithographySource for SynchrotronSource {
    /// DERIVED from machine parameters for undulators (resonance
    /// condition); monochromator-selected for bending magnets.
    fn wavelength_nm(&self) -> f64 {
        match &self.beamline {
            SynchrotronBeamline::BendingMagnet {
                selected_wavelength_nm,
                ..
            } => *selected_wavelength_nm,
            SynchrotronBeamline::Undulator {
                electron_energy_gev,
                period_mm,
                k,
                harmonic,
                ..
            } => {
                let gamma = physics::gamma_from_mev(electron_energy_gev * 1000.0);
                physics::undulator_resonance_nm(*period_mm, *k, gamma, *harmonic)
            }
        }
    }

    /// Monochromator bandwidth (bending magnet) or the natural undulator
    /// line width d-lambda/lambda = 1/(nN).
    fn bandwidth_pm(&self) -> f64 {
        match &self.beamline {
            SynchrotronBeamline::BendingMagnet {
                mono_bandwidth_pm, ..
            } => *mono_bandwidth_pm,
            SynchrotronBeamline::Undulator {
                num_periods,
                harmonic,
                ..
            } => {
                let rel = 1.0 / (*harmonic as f64 * *num_periods as f64);
                rel * self.wavelength_nm() * 1e3
            }
        }
    }

    fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64 {
        evaluate_illumination(&self.illumination, fx_norm, fy_norm)
    }

    fn spectral_weights(&self) -> Vec<(f64, f64)> {
        // Gaussian line sampling; the true undulator line is sinc^2
        // (documented simplification).
        evaluate_spectral_weights(
            self.wavelength_nm(),
            self.bandwidth_pm(),
            self.spectral_samples,
            &SpectralShape::Gaussian,
        )
    }

    fn transverse_coherence(&self) -> f64 {
        self.transverse_coherence_fraction
    }

    // Storage rings are quasi-CW at MHz bunch rates with sub-percent
    // stability: pulse metadata stays None and shot_to_shot_rms 0.
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_undulator_wavelength_is_derived() {
        let src = SynchrotronSource::compact_euv_undulator().unwrap();
        let lambda = src.wavelength_nm();
        assert!(
            (13.3..13.7).contains(&lambda),
            "538 MeV / 20 mm / K=1 should give ~13.5 nm, got {lambda}"
        );

        // Doubling ring energy quarters the wavelength.
        let hot = SynchrotronSource::undulator(1.076, 20.0, 1.0, 100, 1).unwrap();
        assert_relative_eq!(lambda / hot.wavelength_nm(), 4.0, epsilon = 0.02);
    }

    #[test]
    fn test_undulator_natural_bandwidth() {
        let src = SynchrotronSource::compact_euv_undulator().unwrap();
        // 1/(1*100) = 1% of 13.5 nm = 135 pm (within resonance rounding)
        let rel = src.bandwidth_pm() / (src.wavelength_nm() * 1e3);
        assert_relative_eq!(rel, 0.01, epsilon = 1e-9);
    }

    #[test]
    fn test_even_harmonic_rejected() {
        assert!(SynchrotronSource::undulator(0.5, 20.0, 1.0, 100, 2).is_err());
        assert!(SynchrotronSource::undulator(0.5, 20.0, 1.0, 100, 0).is_err());
        assert!(SynchrotronSource::undulator(0.5, 20.0, 1.0, 100, 3).is_ok());
    }

    #[test]
    fn test_liga_bending_magnet_critical_energy() {
        let src = SynchrotronSource::liga_bending_magnet();
        // 0.665 * 2.5^2 * 1.5 = 6.234 keV
        assert_relative_eq!(src.critical_energy_kev().unwrap(), 6.234, epsilon = 1e-3);

        // Flux function peaks below E_c and vanishes far above it.
        let e_c = src.critical_energy_kev().unwrap();
        assert!(src.bm_spectral_flux(0.3 * e_c) > src.bm_spectral_flux(5.0 * e_c));
        assert!(src.bm_spectral_flux(50.0 * e_c) < 1e-6);
    }

    #[test]
    fn test_undulator_has_no_bm_spectrum() {
        let src = SynchrotronSource::compact_euv_undulator().unwrap();
        assert_eq!(src.critical_energy_kev(), None);
        assert_eq!(src.bm_spectral_flux(1.0), 0.0);
    }

    #[test]
    fn test_spectral_weights_sum_to_one() {
        for src in [
            SynchrotronSource::liga_bending_magnet(),
            SynchrotronSource::compact_euv_undulator().unwrap(),
        ] {
            let sum: f64 = src.spectral_weights().iter().map(|(_, w)| w).sum();
            assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
        }
    }

    #[test]
    fn test_undulator_coherent_gaussian_pupil() {
        let src = SynchrotronSource::compact_euv_undulator().unwrap();
        assert_relative_eq!(src.transverse_coherence(), 0.2);
        // Graded pupil: center brighter than mid-radius.
        assert!(src.intensity_at(0.0, 0.0) > src.intensity_at(0.1, 0.0));
    }
}
