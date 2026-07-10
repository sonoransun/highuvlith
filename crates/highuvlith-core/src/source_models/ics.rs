//! Inverse Compton scattering (ICS) sources.
//!
//! A laser pulse collides head-on with a relativistic electron bunch;
//! the backscattered photons are upshifted by the double-Doppler factor
//! ~4 gamma^2. Because only MeV-class electron energies are needed to
//! reach EUV/soft-X-ray from an optical laser, ICS promises compact
//! (room-sized) short-wavelength sources.
//!
//! # Model status
//!
//! The Compton kinematics are exact and the wavelength is DERIVED from
//! machine parameters (electron energy, laser wavelength, a0). The
//! bandwidth model (quadrature of collection-angle spread, electron
//! energy spread, laser bandwidth) is a documented approximation. ICS
//! as a *lithography* source is a parameterized concept — flux at
//! wafer-relevant dose rates has not been demonstrated; treat outputs
//! as research projections (Theoretical).

use serde::{Deserialize, Serialize};

use super::physics;
use crate::source::{
    evaluate_illumination, evaluate_spectral_weights, sigma_from_coherence, IlluminationShape,
    LithographySource, SpectralShape,
};

/// Inverse Compton scattering source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcsSource {
    /// Electron kinetic energy in MeV (MeV-class for EUV output).
    pub electron_energy_mev: f64,
    /// Scattering laser wavelength in nm (e.g. 1030 Yb).
    pub laser_wavelength_nm: f64,
    /// Normalized laser vector potential a0 (a0 << 1 = linear Compton;
    /// larger a0 redshifts the line via the 1 + a0^2/2 term).
    pub laser_a0: f64,
    /// Collection half-angle in mrad (sets the angular bandwidth term
    /// gamma^2 theta^2).
    pub collection_half_angle_mrad: f64,
    /// Relative rms electron energy spread (doubles into the bandwidth).
    pub electron_energy_spread_rel: f64,
    /// Pulse energy in nJ (X-ray, per collision).
    pub pulse_energy_nj: f64,
    /// Collision repetition rate in Hz.
    pub rep_rate_hz: f64,
    /// Transverse coherence fraction (moderate, ~0.5).
    pub transverse_coherence_fraction: f64,
    /// Number of spectral samples.
    pub spectral_samples: usize,
    /// Illumination pupil shape.
    pub illumination: IlluminationShape,
}

impl IcsSource {
    /// Construct an ICS source whose electron energy is chosen to hit
    /// `target_wavelength_nm` on axis — the honest "derived wavelength"
    /// direction: machine parameters follow from the target.
    pub fn for_wavelength(
        target_wavelength_nm: f64,
        laser_wavelength_nm: f64,
        laser_a0: f64,
    ) -> crate::error::Result<Self> {
        if target_wavelength_nm <= 0.0 || target_wavelength_nm.is_nan() {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "target_wavelength_nm",
                value: target_wavelength_nm,
                reason: "must be positive",
            });
        }
        if laser_wavelength_nm <= target_wavelength_nm {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "laser_wavelength_nm",
                value: laser_wavelength_nm,
                reason: "must exceed the target wavelength (Compton upshift)",
            });
        }
        // lambda_X = lambda_L (1 + a0^2/2) / (4 gamma^2)
        // => gamma = sqrt(lambda_L (1 + a0^2/2) / (4 lambda_X))
        let gamma = (laser_wavelength_nm * (1.0 + laser_a0 * laser_a0 / 2.0)
            / (4.0 * target_wavelength_nm))
            .sqrt();
        let electron_energy_mev = (gamma - 1.0) * physics::ELECTRON_REST_MEV;

        let coherence = 0.5;
        Ok(Self {
            electron_energy_mev,
            laser_wavelength_nm,
            laser_a0,
            collection_half_angle_mrad: 1.0,
            electron_energy_spread_rel: 0.005,
            pulse_energy_nj: 1.0,
            rep_rate_hz: 10_000.0,
            transverse_coherence_fraction: coherence,
            spectral_samples: 5,
            illumination: IlluminationShape::CoherentGaussian {
                sigma: sigma_from_coherence(coherence, 0.05),
            },
        })
    }

    /// Compact EUV concept: 1030 nm Yb laser, a0 = 0.1, tuned to 13.5 nm
    /// (requires only ~1.7 MeV electrons).
    pub fn compact_euv_13nm5() -> crate::error::Result<Self> {
        Self::for_wavelength(13.5, 1030.0, 0.1)
    }

    /// Electron Lorentz factor.
    pub fn gamma(&self) -> f64 {
        physics::gamma_from_mev(self.electron_energy_mev)
    }

    /// Relative bandwidth from the quadrature of the collection-angle
    /// spread (gamma^2 theta^2), electron energy spread (2 dE/E), and
    /// (neglected) laser bandwidth. Documented approximation.
    pub fn relative_bandwidth(&self) -> f64 {
        let theta = self.collection_half_angle_mrad * 1e-3;
        let angular = self.gamma() * self.gamma() * theta * theta
            / (1.0 + self.laser_a0 * self.laser_a0 / 2.0);
        let energy = 2.0 * self.electron_energy_spread_rel;
        (angular * angular + energy * energy).sqrt()
    }
}

impl Default for IcsSource {
    fn default() -> Self {
        Self::compact_euv_13nm5().expect("compact EUV preset is valid")
    }
}

impl LithographySource for IcsSource {
    /// DERIVED: on-axis Compton kinematics from machine parameters.
    fn wavelength_nm(&self) -> f64 {
        physics::ics_wavelength_nm(self.laser_wavelength_nm, self.gamma(), self.laser_a0, 0.0)
    }

    fn bandwidth_pm(&self) -> f64 {
        self.relative_bandwidth() * self.wavelength_nm() * 1e3
    }

    fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64 {
        evaluate_illumination(&self.illumination, fx_norm, fy_norm)
    }

    fn spectral_weights(&self) -> Vec<(f64, f64)> {
        evaluate_spectral_weights(
            self.wavelength_nm(),
            self.bandwidth_pm(),
            self.spectral_samples,
            &SpectralShape::Gaussian,
        )
    }

    fn pulse_energy_j(&self) -> Option<f64> {
        Some(self.pulse_energy_nj * 1e-9)
    }

    fn rep_rate_hz(&self) -> Option<f64> {
        Some(self.rep_rate_hz)
    }

    fn transverse_coherence(&self) -> f64 {
        self.transverse_coherence_fraction
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_wavelength_derived_from_kinematics() {
        let src = IcsSource::compact_euv_13nm5().unwrap();
        assert_relative_eq!(src.wavelength_nm(), 13.5, epsilon = 1e-9);
        // MeV-class electrons suffice — the compact-source selling point.
        assert!(
            (1.5..2.0).contains(&src.electron_energy_mev),
            "expected ~1.7 MeV, got {}",
            src.electron_energy_mev
        );
    }

    #[test]
    fn test_bandwidth_monotone_in_collection_angle() {
        let base = IcsSource::compact_euv_13nm5().unwrap();
        let mut prev = 0.0;
        for angle in [0.5, 1.0, 2.0, 4.0] {
            let src = IcsSource {
                collection_half_angle_mrad: angle,
                ..base.clone()
            };
            let bw = src.bandwidth_pm();
            assert!(bw > prev, "bandwidth must grow with collection angle");
            prev = bw;
        }
    }

    #[test]
    fn test_nonlinear_redshift() {
        // Larger a0 at fixed electron energy redshifts the line.
        let linear = IcsSource {
            laser_a0: 0.0,
            ..IcsSource::compact_euv_13nm5().unwrap()
        };
        let nonlinear = IcsSource {
            laser_a0: 0.5,
            ..linear.clone()
        };
        assert!(nonlinear.wavelength_nm() > linear.wavelength_nm());
    }

    #[test]
    fn test_invalid_targets_rejected() {
        assert!(IcsSource::for_wavelength(0.0, 1030.0, 0.1).is_err());
        assert!(IcsSource::for_wavelength(-1.0, 1030.0, 0.1).is_err());
        // Laser must be longer than target (upshift only).
        assert!(IcsSource::for_wavelength(13.5, 10.0, 0.1).is_err());
    }

    #[test]
    fn test_weights_sum_to_one() {
        let src = IcsSource::compact_euv_13nm5().unwrap();
        let sum: f64 = src.spectral_weights().iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
    }
}
