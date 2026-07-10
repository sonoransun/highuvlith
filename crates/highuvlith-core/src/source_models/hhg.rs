//! High-harmonic generation (HHG): table-top coherent EUV/soft-X-ray.
//!
//! An intense femtosecond driver (typically 800 nm Ti:Sapphire or
//! 1030 nm Yb) focused into a noble gas generates odd harmonics of the
//! driver frequency via the three-step model (tunnel ionization ->
//! field acceleration -> recombination). The comb extends up to the
//! cutoff `E_max = I_p + 3.17 U_p` where `U_p = 9.33e-14 I lambda^2` is
//! the ponderomotive energy. Photon flux is low (nJ-uJ per pulse) but
//! coherence is laser-like — HHG is the workhorse of table-top EUV
//! metrology and the candidate compact source for actinic mask
//! inspection.
//!
//! # Model status
//!
//! Comb spacing (2 driver photons), cutoff law, and cutoff-violation
//! rejection are implemented and textbook-exact. The plateau/rolloff
//! intensity envelope is an empirical shape. The default (and
//! lithographically honest) mode is monochromatized single-harmonic
//! imaging; full-comb mode is spectral bookkeeping only, because the
//! aerial engine's polychromatic loop assumes a narrow band (see
//! `evaluate_multiline_weights` docs).

use serde::{Deserialize, Serialize};

use super::physics;
use crate::source::{
    evaluate_illumination, evaluate_multiline_weights, evaluate_spectral_weights,
    sigma_from_coherence, IlluminationShape, LithographySource, SpectralLine, SpectralShape,
};

/// Planck constant x speed of light in eV·nm.
const HC_EV_NM: f64 = 1239.84193;

/// Noble generation gas, which fixes the ionization potential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HhgGas {
    Helium,
    Neon,
    Argon,
    Krypton,
    Xenon,
}

impl HhgGas {
    /// Ionization potential in eV (NIST atomic spectra database).
    pub fn ionization_potential_ev(&self) -> f64 {
        match self {
            HhgGas::Helium => 24.587,
            HhgGas::Neon => 21.565,
            HhgGas::Argon => 15.760,
            HhgGas::Krypton => 14.000,
            HhgGas::Xenon => 12.130,
        }
    }
}

/// Monochromator selection of a single harmonic for imaging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarmonicSelection {
    /// Odd harmonic order q; the output wavelength is lambda_driver / q.
    pub harmonic: usize,
    /// Post-monochromator bandwidth FWHM in pm.
    pub bandwidth_pm: f64,
}

/// High-harmonic generation source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HhgSource {
    /// Driver laser wavelength in nm (800 Ti:Sapphire, 1030 Yb).
    pub driver_wavelength_nm: f64,
    /// Generation gas.
    pub gas: HhgGas,
    /// Driver peak intensity in W/cm^2 (sets the cutoff via U_p).
    pub driver_intensity_w_cm2: f64,
    /// `Some` = monochromatized single-harmonic imaging mode (default,
    /// honest for imaging); `None` = full-comb bookkeeping mode.
    pub monochromator: Option<HarmonicSelection>,
    /// Pulse energy in nJ (in-band, after monochromator).
    pub pulse_energy_nj: f64,
    /// Pulse repetition rate in Hz (kHz-MHz for modern drivers).
    pub rep_rate_hz: f64,
    /// Pulse duration in fs (documentary; no time-domain physics yet).
    pub pulse_duration_fs: f64,
    /// Spectral samples per line.
    pub spectral_samples: usize,
    /// Illumination pupil (laser-like coherence: tight Gaussian).
    pub illumination: IlluminationShape,
    /// Transverse coherence fraction (laser-like, ~0.9).
    pub transverse_coherence_fraction: f64,
}

impl HhgSource {
    /// Construct an HHG source with a selected harmonic, rejecting even
    /// harmonics and harmonics beyond the three-step-model cutoff for
    /// the given gas and intensity.
    pub fn new(
        driver_wavelength_nm: f64,
        gas: HhgGas,
        driver_intensity_w_cm2: f64,
        harmonic: usize,
        monochromator_bandwidth_pm: f64,
    ) -> crate::error::Result<Self> {
        if driver_wavelength_nm <= 0.0 || driver_wavelength_nm.is_nan() {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "driver_wavelength_nm",
                value: driver_wavelength_nm,
                reason: "must be positive",
            });
        }
        if driver_intensity_w_cm2 <= 0.0 || driver_intensity_w_cm2.is_nan() {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "driver_intensity_w_cm2",
                value: driver_intensity_w_cm2,
                reason: "must be positive",
            });
        }
        if harmonic == 0 || harmonic.is_multiple_of(2) {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "harmonic",
                value: harmonic as f64,
                reason: "HHG emits odd harmonics only (inversion symmetry of the gas)",
            });
        }

        let cutoff = Self::cutoff_energy_ev_for(gas, driver_intensity_w_cm2, driver_wavelength_nm);
        let photon_ev = harmonic as f64 * HC_EV_NM / driver_wavelength_nm;
        if photon_ev > cutoff {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "harmonic",
                value: harmonic as f64,
                reason: "harmonic energy exceeds the three-step-model cutoff \
                         I_p + 3.17 U_p for this gas and intensity",
            });
        }

        let coherence = 0.9;
        Ok(Self {
            driver_wavelength_nm,
            gas,
            driver_intensity_w_cm2,
            monochromator: Some(HarmonicSelection {
                harmonic,
                bandwidth_pm: monochromator_bandwidth_pm,
            }),
            pulse_energy_nj: 10.0,
            rep_rate_hz: 100_000.0,
            pulse_duration_fs: 20.0,
            spectral_samples: 5,
            illumination: IlluminationShape::CoherentGaussian {
                sigma: sigma_from_coherence(coherence, 0.05),
            },
            transverse_coherence_fraction: coherence,
        })
    }

    /// Argon-driven 800 nm system, harmonic 27 -> 29.6 nm.
    /// At 2e14 W/cm^2 the cutoff is 53.6 eV, comfortably above the
    /// 41.8 eV of q = 27.
    pub fn ar_800nm_30nm() -> crate::error::Result<Self> {
        let mut src = Self::new(800.0, HhgGas::Argon, 2e14, 27, 30.0)?;
        src.pulse_energy_nj = 10.0;
        src.rep_rate_hz = 100_000.0;
        Ok(src)
    }

    /// Neon-driven 800 nm system pushed to harmonic 59 -> 13.56 nm
    /// (the EUV mirror band). Requires 4e14 W/cm^2: cutoff 97.3 eV vs
    /// the 91.4 eV of q = 59.
    pub fn ne_800nm_13nm5() -> crate::error::Result<Self> {
        let mut src = Self::new(800.0, HhgGas::Neon, 4e14, 59, 15.0)?;
        src.pulse_energy_nj = 1.0;
        src.rep_rate_hz = 10_000.0;
        Ok(src)
    }

    /// Three-step-model cutoff energy in eV for a gas / intensity /
    /// driver combination.
    pub fn cutoff_energy_ev_for(
        gas: HhgGas,
        intensity_w_cm2: f64,
        driver_wavelength_nm: f64,
    ) -> f64 {
        let up = physics::ponderomotive_ev(intensity_w_cm2, driver_wavelength_nm * 1e-3);
        physics::hhg_cutoff_ev(gas.ionization_potential_ev(), up)
    }

    /// Cutoff energy in eV for this source's configuration.
    pub fn cutoff_energy_ev(&self) -> f64 {
        Self::cutoff_energy_ev_for(
            self.gas,
            self.driver_intensity_w_cm2,
            self.driver_wavelength_nm,
        )
    }

    /// The odd-harmonic comb from the plateau start (first harmonic above
    /// I_p) to the cutoff, with a flat plateau and an empirical
    /// exponential rolloff over the last ~10% below cutoff.
    pub fn comb_lines(&self) -> Vec<SpectralLine> {
        let cutoff_ev = self.cutoff_energy_ev();
        let i_p = self.gas.ionization_potential_ev();
        let driver_ev = HC_EV_NM / self.driver_wavelength_nm;

        let mut lines = Vec::new();
        let mut q = 1;
        while (q as f64) * driver_ev <= cutoff_ev {
            let e_q = q as f64 * driver_ev;
            if e_q >= i_p {
                // Empirical envelope: flat plateau, exponential rolloff
                // within 10% of cutoff.
                let rolloff_start = 0.9 * cutoff_ev;
                let intensity = if e_q <= rolloff_start {
                    1.0
                } else {
                    (-(e_q - rolloff_start) / (0.05 * cutoff_ev)).exp()
                };
                lines.push(SpectralLine {
                    center_nm: self.driver_wavelength_nm / q as f64,
                    // Individual harmonic width ~ driver bandwidth / q;
                    // use a fixed fraction of the line wavelength.
                    fwhm_pm: (self.driver_wavelength_nm / q as f64) * 1e3 * 0.01,
                    relative_intensity: intensity,
                });
            }
            q += 2;
        }
        lines
    }
}

impl Default for HhgSource {
    fn default() -> Self {
        Self::ar_800nm_30nm().expect("Ar preset is valid")
    }
}

impl LithographySource for HhgSource {
    /// Selected-harmonic wavelength in monochromatized mode; the median
    /// plateau harmonic's wavelength in full-comb mode.
    fn wavelength_nm(&self) -> f64 {
        match &self.monochromator {
            Some(sel) => self.driver_wavelength_nm / sel.harmonic as f64,
            None => {
                let lines = self.comb_lines();
                if lines.is_empty() {
                    self.driver_wavelength_nm
                } else {
                    lines[lines.len() / 2].center_nm
                }
            }
        }
    }

    fn bandwidth_pm(&self) -> f64 {
        match &self.monochromator {
            Some(sel) => sel.bandwidth_pm,
            None => {
                // Full comb: report the span from longest to shortest line.
                let lines = self.comb_lines();
                match (lines.first(), lines.last()) {
                    (Some(a), Some(b)) => (a.center_nm - b.center_nm).abs() * 1e3,
                    _ => 0.0,
                }
            }
        }
    }

    fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64 {
        evaluate_illumination(&self.illumination, fx_norm, fy_norm)
    }

    fn spectral_weights(&self) -> Vec<(f64, f64)> {
        match &self.monochromator {
            Some(sel) => evaluate_spectral_weights(
                self.driver_wavelength_nm / sel.harmonic as f64,
                sel.bandwidth_pm,
                self.spectral_samples,
                &SpectralShape::Gaussian,
            ),
            None => evaluate_multiline_weights(
                &self.comb_lines(),
                self.spectral_samples,
                &SpectralShape::Gaussian,
            ),
        }
    }

    fn pulse_energy_j(&self) -> Option<f64> {
        Some(self.pulse_energy_nj * 1e-9)
    }

    fn rep_rate_hz(&self) -> Option<f64> {
        Some(self.rep_rate_hz)
    }

    fn pulse_duration_s(&self) -> Option<f64> {
        Some(self.pulse_duration_fs * 1e-15)
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
    fn test_comb_spacing_is_two_driver_photons() {
        let src = HhgSource {
            monochromator: None,
            ..HhgSource::ar_800nm_30nm().unwrap()
        };
        let lines = src.comb_lines();
        assert!(
            lines.len() >= 3,
            "Ar at 2e14 should have several plateau harmonics"
        );
        let driver_ev = HC_EV_NM / 800.0;
        for pair in lines.windows(2) {
            let e0 = HC_EV_NM / pair[0].center_nm;
            let e1 = HC_EV_NM / pair[1].center_nm;
            assert_relative_eq!(e1 - e0, 2.0 * driver_ev, epsilon = 1e-9);
        }
    }

    #[test]
    fn test_cutoff_rejection() {
        // Ar at 2e14 W/cm^2: cutoff 53.6 eV; q = 59 (91.4 eV) must be rejected.
        assert!(HhgSource::new(800.0, HhgGas::Argon, 2e14, 59, 15.0).is_err());
        // Ne at 4e14 (cutoff 97.3 eV) accepts q = 59.
        assert!(HhgSource::new(800.0, HhgGas::Neon, 4e14, 59, 15.0).is_ok());
    }

    #[test]
    fn test_even_harmonic_rejected() {
        assert!(HhgSource::new(800.0, HhgGas::Argon, 2e14, 26, 15.0).is_err());
        assert!(HhgSource::new(800.0, HhgGas::Argon, 2e14, 0, 15.0).is_err());
    }

    #[test]
    fn test_preset_wavelengths() {
        let ar = HhgSource::ar_800nm_30nm().unwrap();
        assert_relative_eq!(ar.wavelength_nm(), 800.0 / 27.0, epsilon = 1e-9);
        assert!((29.0..30.0).contains(&ar.wavelength_nm()));

        let ne = HhgSource::ne_800nm_13nm5().unwrap();
        assert_relative_eq!(ne.wavelength_nm(), 800.0 / 59.0, epsilon = 1e-9);
        assert!((13.4..13.7).contains(&ne.wavelength_nm()));
    }

    #[test]
    fn test_weights_sum_to_one_in_both_modes() {
        let mono = HhgSource::ar_800nm_30nm().unwrap();
        let mono_weights = mono.spectral_weights();
        let sum: f64 = mono_weights.iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-12);

        let comb = HhgSource {
            monochromator: None,
            ..mono
        };
        let comb_weights = comb.spectral_weights();
        let sum: f64 = comb_weights.iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
        // Comb mode spans many lines.
        assert!(comb_weights.len() > mono_weights.len());
    }

    #[test]
    fn test_plateau_starts_above_ionization_potential() {
        let src = HhgSource {
            monochromator: None,
            ..HhgSource::ar_800nm_30nm().unwrap()
        };
        let i_p = src.gas.ionization_potential_ev();
        for line in src.comb_lines() {
            assert!(HC_EV_NM / line.center_nm >= i_p);
        }
    }

    #[test]
    fn test_pulse_metadata_live() {
        let src = HhgSource::ar_800nm_30nm().unwrap();
        assert_relative_eq!(src.pulse_energy_j().unwrap(), 10.0e-9, epsilon = 1e-18);
        // 10 nJ x 100 kHz = 1 mW average
        assert_relative_eq!(src.average_power_w().unwrap(), 1.0e-3, epsilon = 1e-12);
        assert_relative_eq!(src.transverse_coherence(), 0.9);
    }
}
