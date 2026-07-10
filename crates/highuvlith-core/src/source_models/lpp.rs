//! Laser-produced plasma (LPP) sources: Sn at 13.5 nm, Gd/Tb at 6.7/6.5 nm.
//!
//! A high-power drive laser (CO2 for Sn, solid-state for Gd/Tb research
//! sources) vaporizes droplet targets into a dense plasma whose line
//! emission lands in the mirror band: Sn XIII-XV around 13.5 nm (the
//! 2% band Mo/Si multilayers reflect), Gd/Tb around 6.7/6.5 nm (the
//! ~0.6% band of La/B4C multilayers, "beyond-EUV").
//!
//! # Model status
//!
//! Implemented physics with real-machine parameters: the in-band spectrum
//! (Gaussian within the mirror-selected bandwidth), pupil fill, and the
//! power chain `average power = drive power x conversion efficiency x
//! transport efficiency` are live. Plasma dynamics, debris, and etendue
//! are out of scope (documented on the docs page). Plasma emission is
//! spatially incoherent: `transverse_coherence()` reports 0.
//!
//! # References
//!
//! - Sn preset: ASML NXE-class source, ~250 W in-band from ~25 kW CO2
//!   drive at ~5.5% conversion efficiency.
//! - Gd 6.7 nm: laboratory-scale demonstrations at ~0.5-0.8% CE
//!   (e.g. Otsuka et al., Appl. Phys. Lett. 97, 111503 (2010)).

use serde::{Deserialize, Serialize};

use crate::source::{
    evaluate_illumination, evaluate_spectral_weights, IlluminationShape, LithographySource,
    SpectralShape,
};

/// Plasma fuel element, which fixes the emission band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LppFuel {
    /// Tin: 13.5 nm (Mo/Si multilayer band). The production EUV fuel.
    Sn,
    /// Gadolinium: 6.7 nm (La/B4C multilayer band), "beyond-EUV".
    Gd,
    /// Terbium: 6.5 nm, alternative BEUV fuel.
    Tb,
}

/// Laser-produced plasma source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LppSource {
    /// Plasma fuel (fixes the emission band).
    pub fuel: LppFuel,
    /// In-band center wavelength in nm (13.5 for Sn, 6.7 for Gd, 6.5 for Tb).
    pub wavelength_nm: f64,
    /// Mirror-selected in-band width, FWHM in pm (the multilayer stack,
    /// not the plasma, sets the usable bandwidth).
    pub bandwidth_pm: f64,
    /// Drive laser average power in W (CO2 for Sn: tens of kW).
    pub drive_laser_power_w: f64,
    /// Conversion efficiency from drive power to in-band radiation
    /// (Sn: ~0.055 state of the art; Gd/Tb: ~0.007 demonstrated).
    pub conversion_efficiency: f64,
    /// Collector-to-wafer transport efficiency (mirror train losses).
    pub transport_efficiency: f64,
    /// Droplet / pulse repetition rate in Hz (Sn production: ~50 kHz).
    pub rep_rate_hz: f64,
    /// Number of spectral samples for polychromatic simulation.
    pub spectral_samples: usize,
    /// Spectral line shape of the in-band emission.
    pub spectral_shape: SpectralShape,
    /// Illumination pupil shape (plasma is incoherent: large sigma disk).
    pub illumination: IlluminationShape,
}

impl LppSource {
    /// Production-class Sn LPP at 13.5 nm: 25 kW CO2 drive, 5.5% CE,
    /// 2% mirror band (270 pm), 50 kHz droplets — the NXE-class
    /// parameter set (~250 W in-band before transport losses).
    pub fn sn_13nm5(sigma: f64) -> crate::error::Result<Self> {
        crate::source::validate_sigma(sigma)?;
        Ok(Self {
            fuel: LppFuel::Sn,
            wavelength_nm: 13.5,
            bandwidth_pm: 270.0, // 2% of 13.5 nm
            drive_laser_power_w: 25_000.0,
            conversion_efficiency: 0.055,
            transport_efficiency: 0.05,
            rep_rate_hz: 50_000.0,
            spectral_samples: 5,
            spectral_shape: SpectralShape::Gaussian,
            illumination: IlluminationShape::Conventional { sigma },
        })
    }

    /// Gd LPP at 6.7 nm (beyond-EUV): La/B4C mirror band ~0.6% (40 pm),
    /// demonstrated conversion efficiency ~0.7%.
    pub fn gd_6nm7(sigma: f64) -> crate::error::Result<Self> {
        crate::source::validate_sigma(sigma)?;
        Ok(Self {
            fuel: LppFuel::Gd,
            wavelength_nm: 6.7,
            bandwidth_pm: 40.0,
            drive_laser_power_w: 10_000.0,
            conversion_efficiency: 0.007,
            transport_efficiency: 0.05,
            rep_rate_hz: 10_000.0,
            spectral_samples: 5,
            spectral_shape: SpectralShape::Gaussian,
            illumination: IlluminationShape::Conventional { sigma },
        })
    }

    /// Tb LPP at 6.5 nm, alternative BEUV fuel with similar parameters
    /// to Gd.
    pub fn tb_6nm5(sigma: f64) -> crate::error::Result<Self> {
        crate::source::validate_sigma(sigma)?;
        Ok(Self {
            fuel: LppFuel::Tb,
            wavelength_nm: 6.5,
            bandwidth_pm: 39.0,
            drive_laser_power_w: 10_000.0,
            conversion_efficiency: 0.006,
            transport_efficiency: 0.05,
            rep_rate_hz: 10_000.0,
            spectral_samples: 5,
            spectral_shape: SpectralShape::Gaussian,
            illumination: IlluminationShape::Conventional { sigma },
        })
    }

    /// In-band power delivered to the wafer plane, in W:
    /// drive power x conversion efficiency x transport efficiency.
    pub fn in_band_power_w(&self) -> f64 {
        self.drive_laser_power_w * self.conversion_efficiency * self.transport_efficiency
    }
}

impl Default for LppSource {
    fn default() -> Self {
        Self::sn_13nm5(0.9).expect("default sigma 0.9 is valid")
    }
}

impl LithographySource for LppSource {
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
            &self.spectral_shape,
        )
    }

    fn pulse_energy_j(&self) -> Option<f64> {
        if self.rep_rate_hz > 0.0 {
            Some(self.in_band_power_w() / self.rep_rate_hz)
        } else {
            None
        }
    }

    fn rep_rate_hz(&self) -> Option<f64> {
        Some(self.rep_rate_hz)
    }

    fn average_power_w(&self) -> Option<f64> {
        Some(self.in_band_power_w())
    }

    // Plasma emission is spatially incoherent: keep the trait default
    // transverse_coherence() = 0.
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_sn_preset_band_and_energy() {
        let src = LppSource::sn_13nm5(0.9).unwrap();
        assert_relative_eq!(src.wavelength_nm(), 13.5);
        // 13.5 nm photon: hc/lambda = 91.84 eV
        assert_relative_eq!(src.photon_energy_ev(), 91.84, epsilon = 0.01);
        // 2% band
        assert_relative_eq!(
            src.bandwidth_pm / (src.wavelength_nm * 1e3),
            0.02,
            epsilon = 1e-9
        );
    }

    #[test]
    fn test_power_chain_is_live() {
        let src = LppSource::sn_13nm5(0.9).unwrap();
        // 25 kW x 0.055 x 0.05 = 68.75 W at wafer
        assert_relative_eq!(src.in_band_power_w(), 68.75, epsilon = 1e-9);
        assert_relative_eq!(src.average_power_w().unwrap(), 68.75, epsilon = 1e-9);
        // Per-droplet in-band energy: 68.75 / 50 kHz = 1.375 mJ
        assert_relative_eq!(src.pulse_energy_j().unwrap(), 1.375e-3, epsilon = 1e-12);
    }

    #[test]
    fn test_beuv_presets_in_band() {
        let gd = LppSource::gd_6nm7(0.9).unwrap();
        assert_relative_eq!(gd.wavelength_nm(), 6.7);
        assert!((184.0..186.0).contains(&gd.photon_energy_ev()));

        let tb = LppSource::tb_6nm5(0.9).unwrap();
        assert_relative_eq!(tb.wavelength_nm(), 6.5);
    }

    #[test]
    fn test_spectral_weights_sum_to_one() {
        for src in [
            LppSource::sn_13nm5(0.9).unwrap(),
            LppSource::gd_6nm7(0.9).unwrap(),
        ] {
            let sum: f64 = src.spectral_weights().iter().map(|(_, w)| w).sum();
            assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
        }
    }

    #[test]
    fn test_incoherent_plasma() {
        let src = LppSource::sn_13nm5(0.9).unwrap();
        assert_eq!(src.transverse_coherence(), 0.0);
    }

    #[test]
    fn test_invalid_sigma_rejected() {
        assert!(LppSource::sn_13nm5(0.0).is_err());
        assert!(LppSource::gd_6nm7(1.5).is_err());
    }
}
