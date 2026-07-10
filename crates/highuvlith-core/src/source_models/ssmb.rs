//! Steady-state microbunching (SSMB) storage-ring EUV source.
//!
//! SSMB uses a modulation laser to imprint and *sustain* microbunching
//! on a stored electron beam, so every turn radiates coherently at
//! harmonics of the modulation wavelength — combining storage-ring
//! average power with FEL-like coherent gain. Projections reach
//! kW-class average EUV power, which would remove the source-power
//! bottleneck of EUV lithography.
//!
//! # Model status
//!
//! Theoretical/projected: the proof-of-principle demonstrated one-turn
//! microbunching at visible wavelengths (Deng et al., Nature 590, 576
//! (2021), Tsinghua/PTB at the MLS); steady-state EUV operation at kW
//! power is a design projection, not a demonstrated machine. The
//! modulation-to-target integer-harmonic consistency check is live;
//! power numbers are projections.

use serde::{Deserialize, Serialize};

use crate::source::{
    evaluate_illumination, evaluate_spectral_weights, sigma_from_coherence, IlluminationShape,
    LithographySource, SpectralShape,
};

/// Steady-state microbunching source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsmbSource {
    /// Storage ring energy in MeV.
    pub ring_energy_mev: f64,
    /// Modulation laser wavelength in nm (e.g. 1053).
    pub modulation_wavelength_nm: f64,
    /// Radiated target wavelength in nm — must be an integer harmonic
    /// of the modulation wavelength (checked at construction).
    pub target_wavelength_nm: f64,
    /// Projected average output power in W (kW-class designs).
    pub average_power_w: f64,
    /// Relative bandwidth of the coherent harmonic (~1e-3).
    pub rel_bandwidth: f64,
    /// Transverse coherence fraction (~0.8 projected).
    pub transverse_coherence_fraction: f64,
    /// Number of spectral samples.
    pub spectral_samples: usize,
    /// Illumination pupil shape.
    pub illumination: IlluminationShape,
}

impl SsmbSource {
    /// Construct an SSMB source, enforcing that the target wavelength is
    /// an integer harmonic of the modulation laser (within 0.2%).
    pub fn new(
        ring_energy_mev: f64,
        modulation_wavelength_nm: f64,
        target_wavelength_nm: f64,
        average_power_w: f64,
    ) -> crate::error::Result<Self> {
        if target_wavelength_nm <= 0.0 || target_wavelength_nm.is_nan() {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "target_wavelength_nm",
                value: target_wavelength_nm,
                reason: "must be positive",
            });
        }
        let ratio = modulation_wavelength_nm / target_wavelength_nm;
        let harmonic = ratio.round();
        if harmonic < 1.0 || ((ratio - harmonic) / harmonic).abs() > 0.002 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "target_wavelength_nm",
                value: target_wavelength_nm,
                reason: "must be an integer harmonic of the modulation wavelength \
                         (within 0.2%) — SSMB radiates at harmonics of the \
                         modulation laser",
            });
        }
        let coherence = 0.8;
        Ok(Self {
            ring_energy_mev,
            modulation_wavelength_nm,
            target_wavelength_nm,
            average_power_w,
            rel_bandwidth: 1e-3,
            transverse_coherence_fraction: coherence,
            spectral_samples: 5,
            illumination: IlluminationShape::CoherentGaussian {
                sigma: sigma_from_coherence(coherence, 0.05),
            },
        })
    }

    /// kW-class EUV design point: 1053 nm modulation laser, harmonic 78
    /// -> 13.5 nm, 1 kW projected average power, 400 MeV-class ring.
    pub fn euv_1kw_13nm5() -> crate::error::Result<Self> {
        Self::new(400.0, 1053.0, 13.5, 1000.0)
    }

    /// The integer harmonic number modulation/target.
    pub fn harmonic(&self) -> usize {
        (self.modulation_wavelength_nm / self.target_wavelength_nm).round() as usize
    }
}

impl Default for SsmbSource {
    fn default() -> Self {
        Self::euv_1kw_13nm5().expect("EUV design preset is valid")
    }
}

impl LithographySource for SsmbSource {
    fn wavelength_nm(&self) -> f64 {
        self.target_wavelength_nm
    }

    fn bandwidth_pm(&self) -> f64 {
        self.rel_bandwidth * self.target_wavelength_nm * 1e3
    }

    fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64 {
        evaluate_illumination(&self.illumination, fx_norm, fy_norm)
    }

    fn spectral_weights(&self) -> Vec<(f64, f64)> {
        evaluate_spectral_weights(
            self.target_wavelength_nm,
            self.bandwidth_pm(),
            self.spectral_samples,
            &SpectralShape::Gaussian,
        )
    }

    /// SSMB is effectively CW: no pulse structure, power reported
    /// directly.
    fn average_power_w(&self) -> Option<f64> {
        Some(self.average_power_w)
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
    fn test_harmonic_consistency_enforced() {
        // 1053 / 13.5 = 78 exactly: accepted.
        assert!(SsmbSource::new(400.0, 1053.0, 13.5, 1000.0).is_ok());
        // 1053 / 13.9 = 75.75: not an integer harmonic -> rejected.
        assert!(SsmbSource::new(400.0, 1053.0, 13.9, 1000.0).is_err());
        // Nonsense targets rejected.
        assert!(SsmbSource::new(400.0, 1053.0, 0.0, 1000.0).is_err());
    }

    #[test]
    fn test_preset_is_78th_harmonic() {
        let src = SsmbSource::euv_1kw_13nm5().unwrap();
        assert_eq!(src.harmonic(), 78);
        assert_relative_eq!(src.wavelength_nm(), 13.5);
    }

    #[test]
    fn test_cw_power_passthrough() {
        let src = SsmbSource::euv_1kw_13nm5().unwrap();
        assert_relative_eq!(src.average_power_w().unwrap(), 1000.0);
        // CW: no pulse metadata.
        assert_eq!(src.pulse_energy_j(), None);
        assert_eq!(LithographySource::rep_rate_hz(&src), None);
    }

    #[test]
    fn test_weights_sum_to_one() {
        let src = SsmbSource::euv_1kw_13nm5().unwrap();
        let sum: f64 = src.spectral_weights().iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
    }
}
