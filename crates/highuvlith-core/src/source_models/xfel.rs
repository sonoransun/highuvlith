//! X-ray free-electron laser (XFEL) sources: SASE and self-seeded.
//!
//! Kilometer-scale linac-driven FELs (FLASH, LCLS, European XFEL,
//! SwissFEL, FERMI) produce femtosecond pulses of extreme peak power in
//! the EUV-to-hard-X-ray range. SASE (self-amplified spontaneous
//! emission) starts from shot noise: the spectrum is a stochastic comb
//! of spikes with FWHM ~ 2 rho (Pierce parameter rho ~ 1e-3) and the
//! pulse energy fluctuates with Gamma statistics of order
//! M ~ pulse_duration / coherence_time. Seeded modes (self-seeding,
//! HGHG as at FERMI) narrow the bandwidth 10-100x and stabilize the
//! pulse energy.
//!
//! # Model status
//!
//! Implemented spectrum/statistics physics: SASE bandwidth 2*rho,
//! longitudinal mode count M, and the resulting shot-to-shot rms
//! 1/sqrt(M) feed the stochastic module's dose jitter (live).
//! The default spectrum is the smooth ensemble-average envelope —
//! honest for multi-shot exposures; `sase_spike_seed` generates a
//! single spiky realization for research use.
//! XFELs tune wavelength on request (gap-tunable undulators), so
//! `wavelength_nm` is a stored set-point here, unlike the derived
//! wavelength of `SynchrotronSource` (documented contrast).

use rand::prelude::*;
use rand_distr::Exp1;
use serde::{Deserialize, Serialize};

use super::physics;
use crate::source::{
    evaluate_illumination, evaluate_spectral_weights, sigma_from_coherence, IlluminationShape,
    LithographySource, SpectralShape,
};

/// FEL operating mode: stochastic SASE or seeded.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XfelMode {
    /// Self-amplified spontaneous emission: bandwidth ~2 rho, Gamma
    /// pulse-energy statistics.
    Sase { pierce_parameter: f64 },
    /// Self-seeded / HGHG: externally imposed narrow bandwidth with
    /// percent-level energy stability.
    SelfSeeded { rel_bandwidth: f64 },
}

/// X-ray free-electron laser source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XfelSource {
    /// Operating mode (SASE or seeded).
    pub mode: XfelMode,
    /// Wavelength set-point in nm (XFELs are gap-tunable; this is a
    /// requested value, not derived from machine parameters).
    pub wavelength_nm: f64,
    /// Pulse energy in uJ.
    pub pulse_energy_uj: f64,
    /// Pulse duration in fs.
    pub pulse_duration_fs: f64,
    /// Pulse repetition rate in Hz (effective average).
    pub rep_rate_hz: f64,
    /// Transverse coherence fraction (SASE ~0.85, seeded ~0.95).
    pub transverse_coherence_fraction: f64,
    /// Number of spectral samples for polychromatic simulation.
    pub spectral_samples: usize,
    /// Illumination pupil (near-diffraction-limited: tight Gaussian).
    pub illumination: IlluminationShape,
    /// `Some(seed)` generates one deterministic spiky SASE spectrum
    /// realization instead of the ensemble-average envelope
    /// (research/single-shot mode).
    pub sase_spike_seed: Option<u64>,
}

impl XfelSource {
    /// FLASH-class soft-X-ray SASE at 13.5 nm: rho = 3e-3
    /// (0.6% bandwidth), 100 uJ, 30 fs, kHz-class effective rate.
    pub fn flash_13nm5() -> Self {
        let coherence = 0.85;
        Self {
            mode: XfelMode::Sase {
                pierce_parameter: 3e-3,
            },
            wavelength_nm: 13.5,
            pulse_energy_uj: 100.0,
            pulse_duration_fs: 30.0,
            rep_rate_hz: 1000.0,
            transverse_coherence_fraction: coherence,
            spectral_samples: 7,
            illumination: IlluminationShape::CoherentGaussian {
                sigma: sigma_from_coherence(coherence, 0.05),
            },
            sase_spike_seed: None,
        }
    }

    /// FERMI-class seeded (HGHG) FEL near 13.5 nm: relative bandwidth
    /// 5e-5, 50 uJ, 50 fs, 50 Hz.
    pub fn fermi_seeded_13nm5() -> Self {
        let coherence = 0.95;
        Self {
            mode: XfelMode::SelfSeeded {
                rel_bandwidth: 5e-5,
            },
            wavelength_nm: 13.5,
            pulse_energy_uj: 50.0,
            pulse_duration_fs: 50.0,
            rep_rate_hz: 50.0,
            transverse_coherence_fraction: coherence,
            spectral_samples: 5,
            illumination: IlluminationShape::CoherentGaussian {
                sigma: sigma_from_coherence(coherence, 0.05),
            },
            sase_spike_seed: None,
        }
    }

    /// Longitudinal mode count `M ~ max(1, T_pulse / t_coh)` with
    /// coherence time `t_coh = lambda^2 / (c * d-lambda)`. Governs the
    /// SASE pulse-energy statistics (Gamma of order M).
    pub fn longitudinal_modes(&self) -> f64 {
        let bw_nm = self.bandwidth_pm() * 1e-3;
        if bw_nm <= 0.0 {
            return 1.0;
        }
        let lambda_m = self.wavelength_nm * 1e-9;
        let d_lambda_m = bw_nm * 1e-9;
        let c = 2.99792458e8;
        let t_coh_s = lambda_m * lambda_m / (c * d_lambda_m);
        let t_pulse_s = self.pulse_duration_fs * 1e-15;
        (t_pulse_s / t_coh_s).max(1.0)
    }
}

impl Default for XfelSource {
    fn default() -> Self {
        Self::flash_13nm5()
    }
}

impl LithographySource for XfelSource {
    fn wavelength_nm(&self) -> f64 {
        self.wavelength_nm
    }

    /// SASE: d-lambda/lambda ~ 2 rho. Seeded: the imposed bandwidth.
    fn bandwidth_pm(&self) -> f64 {
        match &self.mode {
            XfelMode::Sase { pierce_parameter } => {
                physics::sase_bandwidth_pm(self.wavelength_nm, *pierce_parameter)
            }
            XfelMode::SelfSeeded { rel_bandwidth } => rel_bandwidth * self.wavelength_nm * 1e3,
        }
    }

    fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64 {
        evaluate_illumination(&self.illumination, fx_norm, fy_norm)
    }

    fn spectral_weights(&self) -> Vec<(f64, f64)> {
        match (&self.mode, self.sase_spike_seed) {
            (XfelMode::Sase { .. }, Some(seed)) => {
                // One deterministic spiky realization: M spikes at random
                // positions within the envelope, exponentially distributed
                // amplitudes (Gamma with M=1 per spike), weighted by the
                // Gaussian ensemble envelope.
                let m = self.longitudinal_modes().round().max(1.0) as usize;
                let bw_nm = self.bandwidth_pm() * 1e-3;
                let sigma = bw_nm / (2.0 * (2.0_f64.ln()).sqrt());
                let half_range = 2.5 * bw_nm;
                let mut rng = StdRng::seed_from_u64(seed);

                let mut weights: Vec<(f64, f64)> = (0..m)
                    .map(|_| {
                        let dw = rng.random_range(-half_range..half_range);
                        let envelope = (-dw * dw / (2.0 * sigma * sigma)).exp();
                        let amplitude: f64 = rng.sample(Exp1);
                        (self.wavelength_nm + dw, envelope * amplitude)
                    })
                    .collect();
                weights.sort_by(|a, b| a.0.total_cmp(&b.0));
                let total: f64 = weights.iter().map(|(_, w)| w).sum();
                if total > 0.0 {
                    for (_, w) in &mut weights {
                        *w /= total;
                    }
                }
                weights
            }
            _ => evaluate_spectral_weights(
                self.wavelength_nm,
                self.bandwidth_pm(),
                self.spectral_samples,
                &SpectralShape::Gaussian,
            ),
        }
    }

    fn pulse_energy_j(&self) -> Option<f64> {
        Some(self.pulse_energy_uj * 1e-6)
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

    /// SASE: 1/sqrt(M) from Gamma statistics of M longitudinal modes
    /// (Saldin/Schmueser). Seeded: 2% residual jitter.
    fn shot_to_shot_rms(&self) -> f64 {
        match &self.mode {
            XfelMode::Sase { .. } => 1.0 / self.longitudinal_modes().sqrt(),
            XfelMode::SelfSeeded { .. } => 0.02,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_sase_bandwidth_is_two_rho() {
        let src = XfelSource::flash_13nm5();
        // 2 * 3e-3 * 13.5 nm = 81 pm
        assert_relative_eq!(src.bandwidth_pm(), 81.0, epsilon = 1e-9);
    }

    #[test]
    fn test_seeded_much_narrower_than_sase() {
        let sase = XfelSource::flash_13nm5();
        let seeded = XfelSource::fermi_seeded_13nm5();
        assert!(seeded.bandwidth_pm() < sase.bandwidth_pm() / 50.0);
    }

    #[test]
    fn test_sase_jitter_from_mode_count() {
        let src = XfelSource::flash_13nm5();
        let m = src.longitudinal_modes();
        assert!(m >= 1.0);
        assert_relative_eq!(src.shot_to_shot_rms(), 1.0 / m.sqrt(), epsilon = 1e-12);

        // Longer pulses average over more modes -> lower jitter.
        let long_pulse = XfelSource {
            pulse_duration_fs: 300.0,
            ..XfelSource::flash_13nm5()
        };
        assert!(long_pulse.shot_to_shot_rms() < src.shot_to_shot_rms());

        // Seeded mode is stable.
        assert_relative_eq!(XfelSource::fermi_seeded_13nm5().shot_to_shot_rms(), 0.02);
    }

    #[test]
    fn test_spike_realization_deterministic_and_normalized() {
        let src = XfelSource {
            sase_spike_seed: Some(42),
            ..XfelSource::flash_13nm5()
        };
        let w1 = src.spectral_weights();
        let w2 = src.spectral_weights();
        assert_eq!(w1.len(), w2.len());
        for (a, b) in w1.iter().zip(w2.iter()) {
            assert_relative_eq!(a.0, b.0, epsilon = 1e-15);
            assert_relative_eq!(a.1, b.1, epsilon = 1e-15);
        }
        let sum: f64 = w1.iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-12);

        // Different seed -> different realization.
        let other = XfelSource {
            sase_spike_seed: Some(43),
            ..XfelSource::flash_13nm5()
        };
        let w3 = other.spectral_weights();
        assert!(w1
            .iter()
            .zip(w3.iter())
            .any(|(a, b)| (a.0 - b.0).abs() > 1e-12));
    }

    #[test]
    fn test_ensemble_weights_sum_to_one() {
        for src in [XfelSource::flash_13nm5(), XfelSource::fermi_seeded_13nm5()] {
            let sum: f64 = src.spectral_weights().iter().map(|(_, w)| w).sum();
            assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
        }
    }

    #[test]
    fn test_pulse_metadata_live() {
        let src = XfelSource::flash_13nm5();
        assert_relative_eq!(src.pulse_energy_j().unwrap(), 1.0e-4, epsilon = 1e-12);
        // 100 uJ x 1 kHz = 0.1 W
        assert_relative_eq!(src.average_power_w().unwrap(), 0.1, epsilon = 1e-12);
        assert_relative_eq!(src.photon_energy_ev(), 91.84, epsilon = 0.01);
    }
}
