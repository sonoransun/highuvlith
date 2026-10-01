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
//! When the optional [`FelMachine`] is present (all presets), the
//! undulator K is DERIVED from the wavelength set-point (gap tuning,
//! inverse resonance) and the Pierce parameter is DERIVED from the beam
//! (1D theory) — the SASE bandwidth then uses the derived rho (live);
//! gain lengths, Ming Xie 3D degradation, saturation power/length and
//! pulse energy are reported as derived quantities. `pulse_energy_uj` is
//! stored; when a front end overrides the machine, the wavelength
//! set-point or the pulse duration of a preset without giving a pulse
//! energy, [`XfelSource::rescale_pulse_energy_from`] re-derives it at the
//! preset's `pulse_energy / saturation` ratio (exactly the saturation
//! estimate for CW-SC / ERL), so average power follows the machine. Simplified (🔶): 1D
//! theory plus an empirical 3D fit; saturation energies are flat-top
//! estimates. The CW-SC and ERL presets are design PROJECTIONS (🧪): no
//! such 13.5 nm machine exists. Measured anchors: FLASH averaged 20 mW at
//! 13.7 nm in 2007 (Ackermann et al., Nat. Photonics 1, 336) and 0.35 W at
//! 18.2 nm in 2015 (Schreiber & Faatz, High Power Laser Sci. Eng. 3, e20);
//! FERMI FEL-2 gives 10–30 µJ at 10 or 50 Hz. Design anchors for the ERL
//! preset: KEK's ERL EUV-FEL design, 800 MeV, 60 pC, 9.75 mA → 9–13.7 kW at
//! 13.5 nm (Nakamura et al., ERL2015, doi:10.18429/JACoW-ERL2015-MOPCTH010;
//! its test machine cERL has run ~1 mA and lased at 20 µm); a DESY
//! FLASH-technology design at 1.7 kW (2011); the announced xLight project
//! (120 kW FEL output, 38 kW at the scanners' input for 16 scanners).
//! The default spectrum is the smooth ensemble-average envelope —
//! honest for multi-shot exposures; `sase_spike_seed` generates a
//! single spiky realization for research use.
//! XFELs tune wavelength on request (gap-tunable undulators), so
//! `wavelength_nm` is a stored set-point here, unlike the derived
//! wavelength of `SynchrotronSource` (documented contrast).
//!
//! # Key equations
//!
//! - gap tuning: `K = sqrt(2 (2 gamma^2 lambda / lambda_u - 1))`
//! - `rho = [(1/16)(I/I_A) K^2 JJ^2 / (gamma^3 sigma_x^2 k_u^2)]^(1/3)`,
//!   `L_g = lambda_u / (4 pi sqrt3 rho)`, `P_sat ~ rho P_beam`
//! - Ming Xie: `L_g3D = L_g (1 + Lambda)`, `P_sat,3D ~ 1.6 rho (L_g/L_g3D)^2 P_beam`
//! - SASE: `d-lambda/lambda ~ 2 rho`, `t_coh = lambda^2 / (c d-lambda)`,
//!   `M = T / t_coh`, `sigma_E / E = 1 / sqrt(M)`

use rand::prelude::*;
use rand_distr::Exp1;
use serde::{Deserialize, Serialize};

use super::physics;
use crate::source::{
    evaluate_illumination, evaluate_spectral_weights, fel_estimate_quantities,
    sigma_from_coherence, DerivedQuantity, IlluminationShape, LithographySource, SpectralShape,
};

/// FEL operating mode: stochastic SASE or seeded.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XfelMode {
    /// Self-amplified spontaneous emission: bandwidth ~2 rho, Gamma
    /// pulse-energy statistics. `pierce_parameter` is the stored value,
    /// used only when no [`FelMachine`] is attached.
    Sase { pierce_parameter: f64 },
    /// Self-seeded / HGHG: externally imposed narrow bandwidth with
    /// percent-level energy stability.
    SelfSeeded { rel_bandwidth: f64 },
}

/// Linac + undulator machine parameters from which the FEL physics is
/// derived.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FelMachine {
    /// Electron kinetic energy at the undulator, MeV.
    pub electron_energy_mev: f64,
    /// Undulator period in mm (K follows from the wavelength set-point).
    pub undulator_period_mm: f64,
    /// Total magnetic length of the undulator line, m.
    pub undulator_length_m: f64,
    /// Beam parameters in the undulator.
    pub beam: physics::ElectronBeamParams,
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
    /// Optional machine parameters (electron energy, undulator period and
    /// length, beam). When present, K and the Pierce parameter are
    /// DERIVED. `None` in configs written before the field existed.
    #[serde(default)]
    pub machine: Option<FelMachine>,
}

/// Build a SASE/seeded preset around a machine: rho, the stored Pierce
/// parameter fallback and (optionally) the pulse energy come from the
/// derived FEL estimate.
fn preset_with_machine(
    mode: XfelMode,
    machine: FelMachine,
    pulse_duration_fs: f64,
    rep_rate_hz: f64,
    coherence: f64,
    spectral_samples: usize,
    pulse_energy_uj: Option<f64>,
) -> XfelSource {
    let mut src = XfelSource {
        mode,
        wavelength_nm: 13.5,
        pulse_energy_uj: pulse_energy_uj.unwrap_or(0.0),
        pulse_duration_fs,
        rep_rate_hz,
        transverse_coherence_fraction: coherence,
        spectral_samples,
        illumination: IlluminationShape::CoherentGaussian {
            sigma: sigma_from_coherence(coherence, 0.05),
        },
        sase_spike_seed: None,
        machine: Some(machine),
    };
    if let Some(est) = src.fel_estimate() {
        if let XfelMode::Sase { pierce_parameter } = &mut src.mode {
            // Keep the stored fallback consistent with the derivation.
            *pierce_parameter = est.rho_1d;
        }
        if pulse_energy_uj.is_none() {
            src.pulse_energy_uj = est.saturation_power_3d_w * pulse_duration_fs * 1e-15 * 1e6;
        }
    }
    src
}

impl XfelSource {
    /// FLASH-class soft-X-ray SASE at 13.5 nm: 100 uJ, 30 fs, kHz-class
    /// effective rate, on ILLUSTRATIVE FLASH-like machine parameters
    /// (680 MeV, 31.4 mm variable-gap undulator, 2.5 kA, 1.5 um, beta 6 m,
    /// 7e-4 slice spread): rho is DERIVED (≈ 2.3e-3, 0.46 % bandwidth).
    pub fn flash_13nm5() -> Self {
        preset_with_machine(
            XfelMode::Sase {
                pierce_parameter: 3e-3,
            },
            FelMachine {
                electron_energy_mev: 680.0,
                undulator_period_mm: 31.4,
                undulator_length_m: 30.0,
                beam: physics::ElectronBeamParams {
                    peak_current_a: 2500.0,
                    norm_emittance_um: 1.5,
                    energy_spread_rel: 7e-4,
                    beta_m: 6.0,
                },
            },
            30.0,
            1000.0,
            0.85,
            7,
            Some(100.0),
        )
    }

    /// FERMI-class seeded (HGHG) FEL near 13.5 nm: relative bandwidth
    /// 5e-5, 20 uJ (FERMI FEL-2 delivers 10–30 uJ), 50 fs, 50 Hz → 1 mW,
    /// on ILLUSTRATIVE FERMI-like machine parameters (1.2 GeV, 35 mm
    /// radiator, 700 A, 1 um, beta 10 m).
    pub fn fermi_seeded_13nm5() -> Self {
        preset_with_machine(
            XfelMode::SelfSeeded {
                rel_bandwidth: 5e-5,
            },
            FelMachine {
                electron_energy_mev: 1200.0,
                undulator_period_mm: 35.0,
                undulator_length_m: 15.0,
                beam: physics::ElectronBeamParams {
                    peak_current_a: 700.0,
                    norm_emittance_um: 1.0,
                    energy_spread_rel: 1.25e-4,
                    beta_m: 10.0,
                },
            },
            50.0,
            50.0,
            0.95,
            5,
            Some(20.0),
        )
    }

    /// PROJECTION (🧪): an LCLS-II-like CW superconducting-linac
    /// architecture scaled to 13.5 nm — 1 GeV, 39 mm undulator (30 m),
    /// 1 kA / 0.5 um / 1e-4, 50 fs (50 pC) bunches at 1 MHz. The pulse
    /// energy is the DERIVED Ming-Xie saturation estimate (≈ 135 uJ,
    /// ≈ 135 W average). No such 13.5 nm machine exists.
    pub fn cw_sc_13nm5() -> Self {
        preset_with_machine(
            XfelMode::Sase {
                pierce_parameter: 0.0,
            },
            FelMachine {
                electron_energy_mev: 1000.0,
                undulator_period_mm: 39.0,
                undulator_length_m: 30.0,
                beam: physics::ElectronBeamParams {
                    peak_current_a: 1000.0,
                    norm_emittance_um: 0.5,
                    energy_spread_rel: 1e-4,
                    beta_m: 10.0,
                },
            },
            50.0,
            1.0e6,
            0.85,
            7,
            None,
        )
    }

    /// PROJECTION (🧪): an energy-recovery-linac FEL design point for EUV
    /// lithography in the class of KEK's ERL EUV-FEL design (800 MeV, 60 pC,
    /// 9.75 mA → 9–13.7 kW; Nakamura et al., ERL2015) —
    /// 800 MeV, 60 pC at 162.5 MHz (~10 mA, 7.8 MW beam power to recover),
    /// 300 A peak (200 fs), 0.6 um, 5e-4, 28 mm undulator (30 m). The pulse
    /// energy is the DERIVED Ming-Xie saturation estimate (≈ 65 uJ,
    /// ≈ 10.5 kW average). Illustrative parameters, not a specific design.
    pub fn erl_13nm5() -> Self {
        preset_with_machine(
            XfelMode::Sase {
                pierce_parameter: 0.0,
            },
            FelMachine {
                electron_energy_mev: 800.0,
                undulator_period_mm: 28.0,
                undulator_length_m: 30.0,
                beam: physics::ElectronBeamParams {
                    peak_current_a: 300.0,
                    norm_emittance_um: 0.6,
                    energy_spread_rel: 5e-4,
                    beta_m: 5.0,
                },
            },
            200.0,
            162.5e6,
            0.85,
            7,
            None,
        )
    }

    /// Electron Lorentz factor (`None` without a machine).
    pub fn gamma(&self) -> Option<f64> {
        self.machine
            .map(|m| physics::gamma_from_mev(m.electron_energy_mev))
    }

    /// Gap-tuned undulator K that puts the fundamental at the wavelength
    /// set-point; `None` without a machine or if the set-point is shorter
    /// than the K = 0 resonance.
    pub fn undulator_k(&self) -> Option<f64> {
        let m = self.machine?;
        physics::undulator_k_for_wavelength(
            m.undulator_period_mm,
            physics::gamma_from_mev(m.electron_energy_mev),
            self.wavelength_nm,
            1,
        )
    }

    /// 1D + Ming-Xie FEL estimate at the set-point (`None` without a
    /// machine or for an unreachable wavelength).
    pub fn fel_estimate(&self) -> Option<physics::FelEstimate> {
        let m = self.machine?;
        let k = self.undulator_k()?;
        let num_periods = (m.undulator_length_m / (m.undulator_period_mm * 1e-3)).round() as usize;
        let und = physics::UndulatorParams {
            period_mm: m.undulator_period_mm,
            k,
            num_periods,
        };
        Some(physics::fel_estimate(
            physics::gamma_from_mev(m.electron_energy_mev),
            &und,
            &m.beam,
        ))
    }

    /// Flat-top Ming-Xie saturation pulse energy `P_sat,3D x T` in uJ
    /// (`None` without a machine or for an unreachable wavelength).
    pub fn saturation_pulse_energy_uj(&self) -> Option<f64> {
        let est = self.fel_estimate()?;
        Some(est.saturation_power_3d_w * self.pulse_duration_fs * 1e-15 * 1e6)
    }

    /// Re-derive the stored pulse energy after the machine, wavelength
    /// set-point or pulse duration was changed relative to `reference`
    /// (normally the preset this source was built from), keeping the
    /// reference's calibration `pulse_energy / saturation estimate`:
    ///
    /// `E = E_ref x E_sat(self) / E_sat(ref)`.
    ///
    /// For the CW-SC / ERL presets (ratio 1) this is exactly the derived
    /// saturation energy of the new machine; the FLASH / FERMI presets
    /// keep their measured-class ratio (0.86 / 0.24). Returns `false` and
    /// leaves the pulse energy unchanged when either side has no
    /// saturation estimate (no machine, unreachable wavelength).
    pub fn rescale_pulse_energy_from(&mut self, reference: &XfelSource) -> bool {
        match (
            self.saturation_pulse_energy_uj(),
            reference.saturation_pulse_energy_uj(),
        ) {
            (Some(e_new), Some(e_ref)) if e_ref > 0.0 && e_new.is_finite() => {
                self.pulse_energy_uj = reference.pulse_energy_uj * e_new / e_ref;
                true
            }
            _ => false,
        }
    }

    /// Pierce parameter used for the SASE bandwidth: DERIVED from the
    /// machine when present, else the stored mode value (`None` for
    /// seeded mode without a machine).
    pub fn pierce_parameter(&self) -> Option<f64> {
        if let Some(est) = self.fel_estimate() {
            return Some(est.rho_1d);
        }
        match &self.mode {
            XfelMode::Sase { pierce_parameter } => Some(*pierce_parameter),
            XfelMode::SelfSeeded { .. } => None,
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

    /// SASE: d-lambda/lambda ~ 2 rho (rho derived from the machine when
    /// present). Seeded: the imposed bandwidth.
    fn bandwidth_pm(&self) -> f64 {
        match &self.mode {
            XfelMode::Sase { .. } => physics::sase_bandwidth_pm(
                self.wavelength_nm,
                self.pierce_parameter().unwrap_or(0.0),
            ),
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

    fn derived_quantities(&self) -> Vec<DerivedQuantity> {
        let lambda = self.wavelength_nm;
        let tau = self.pulse_duration_fs * 1e-15;
        let power = self.pulse_energy_uj * 1e-6 * self.rep_rate_hz;
        let bw_nm = self.bandwidth_pm() * 1e-3;
        let mut out = vec![
            DerivedQuantity::new(
                "photon_energy",
                physics::HC_EV_NM / lambda,
                "eV",
                "hc / lambda",
            ),
            DerivedQuantity::new(
                "average_power",
                power,
                "W",
                "pulse energy x repetition rate",
            ),
            DerivedQuantity::new(
                "photon_rate",
                physics::watts_to_photon_rate(power, lambda),
                "photons/s",
                "average power / photon energy",
            ),
            DerivedQuantity::new(
                "hvm_power_ratio",
                power / super::throughput::HVM_EUV_POWER_AT_IF_W,
                "-",
                "average power / 250 W (production 13.5 nm HVM at IF)",
            ),
            DerivedQuantity::new(
                "coherence_time",
                if bw_nm > 0.0 {
                    (lambda * 1e-9).powi(2) / (physics::SPEED_OF_LIGHT_M_S * bw_nm * 1e-9) * 1e15
                } else {
                    f64::INFINITY
                },
                "fs",
                "lambda^2 / (c d-lambda)",
            ),
            DerivedQuantity::new(
                "longitudinal_modes",
                self.longitudinal_modes(),
                "-",
                "M = T / t_coh; SASE pulse-energy rms = 1/sqrt(M)",
            ),
        ];
        if let Some(m) = self.machine {
            let gamma = physics::gamma_from_mev(m.electron_energy_mev);
            out.push(DerivedQuantity::new(
                "electron_gamma",
                gamma,
                "-",
                "1 + E_kin / m_e c^2",
            ));
            match self.undulator_k() {
                Some(k) => out.push(DerivedQuantity::new(
                    "undulator_k",
                    k,
                    "-",
                    "gap-tuned K placing the fundamental at the set-point (inverse resonance)",
                )),
                None => out.push(DerivedQuantity::new(
                    "wavelength_reachable",
                    0.0,
                    "bool",
                    "set-point shorter than the K = 0 resonance lambda_u / (2 gamma^2)",
                )),
            }
            if let Some(est) = self.fel_estimate() {
                out.extend(fel_estimate_quantities(&est, tau, lambda));
                if let XfelMode::SelfSeeded { .. } = self.mode {
                    out.push(DerivedQuantity::new(
                        "seeded_note",
                        1.0,
                        "bool",
                        "seeded start-up: the SASE saturation length / bandwidth above are \
                         upper bounds, the imposed seed bandwidth is used for imaging",
                    ));
                }
                let e_sat = est.saturation_power_3d_w * tau;
                if e_sat > 0.0 {
                    out.push(DerivedQuantity::new(
                        "pulse_energy_over_saturation",
                        self.pulse_energy_uj * 1e-6 / e_sat,
                        "-",
                        "stored pulse energy / Ming-Xie saturation estimate",
                    ));
                }
                // Average electron-beam power: charge = I_peak x T (flat top),
                // kinetic energy (what the linac supplies / must recover).
                let beam_avg =
                    m.electron_energy_mev * 1e6 * m.beam.peak_current_a * tau * self.rep_rate_hz;
                out.push(DerivedQuantity::new(
                    "beam_power_average",
                    beam_avg,
                    "W",
                    "E_kin x (I_peak T) x f_rep: beam power to dump or energy-recover",
                ));
                if beam_avg > 0.0 {
                    out.push(DerivedQuantity::new(
                        "fel_efficiency",
                        power / beam_avg,
                        "-",
                        "average FEL power / average beam power (~rho at saturation)",
                    ));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn dq(src: &XfelSource, name: &str) -> f64 {
        src.derived_quantities()
            .into_iter()
            .find(|q| q.name == name)
            .unwrap_or_else(|| panic!("missing derived quantity {name}"))
            .value
    }

    #[test]
    fn test_sase_bandwidth_is_two_rho() {
        let src = XfelSource::flash_13nm5();
        let rho = src.pierce_parameter().unwrap();
        // rho is derived from the FLASH-like beam (scipy fixture 2.3137e-3).
        assert_relative_eq!(rho, 0.002_313_671_640_192_722_4, max_relative = 1e-6);
        assert_relative_eq!(
            src.bandwidth_pm(),
            2.0 * rho * 13.5 * 1e3,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            src.bandwidth_pm(),
            62.469_134_285_203_5,
            max_relative = 1e-6
        );
        // Without a machine the stored Pierce parameter is used.
        let stored = XfelSource {
            machine: None,
            mode: XfelMode::Sase {
                pierce_parameter: 3e-3,
            },
            ..src
        };
        assert_relative_eq!(stored.bandwidth_pm(), 81.0, epsilon = 1e-9);
    }

    #[test]
    fn test_gap_tuning_rederives_rho() {
        let src = XfelSource::flash_13nm5();
        // K = sqrt(2 (2 gamma^2 lambda / lambda_u - 1)) = 1.0247 at 13.5 nm
        assert_relative_eq!(
            src.undulator_k().unwrap(),
            1.024_676_417_225_126,
            max_relative = 1e-9
        );
        // Tuning to a longer wavelength opens K and changes rho (live).
        let longer = XfelSource {
            wavelength_nm: 20.0,
            ..src.clone()
        };
        assert!(longer.undulator_k().unwrap() > src.undulator_k().unwrap());
        assert!(longer.pierce_parameter().unwrap() != src.pierce_parameter().unwrap());
        // Below the K = 0 limit the set-point is unreachable.
        let too_short = XfelSource {
            wavelength_nm: 1.0,
            ..src
        };
        assert!(too_short.undulator_k().is_none());
        assert_eq!(dq(&too_short, "wavelength_reachable"), 0.0);
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
        // M = 30 fs / t_coh = 3.083 with the derived bandwidth.
        assert_relative_eq!(m, 3.082_761_368_970_08, max_relative = 1e-6);
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
        for src in [
            XfelSource::flash_13nm5(),
            XfelSource::fermi_seeded_13nm5(),
            XfelSource::cw_sc_13nm5(),
            XfelSource::erl_13nm5(),
        ] {
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
        // The stored 100 uJ is consistent with the derived saturation
        // estimate (116 uJ, scipy fixture).
        assert_relative_eq!(
            dq(&src, "saturation_pulse_energy_3d"),
            116.397_060_886_825_09e-6,
            max_relative = 1e-5
        );
    }

    #[test]
    fn test_machine_class_presets() {
        // CW-SC: derived 135 uJ at 1 MHz -> 135 W.
        let cw = XfelSource::cw_sc_13nm5();
        assert_relative_eq!(
            cw.pierce_parameter().unwrap(),
            0.002_544_370_503_129_166_6,
            max_relative = 1e-6
        );
        assert_relative_eq!(
            cw.pulse_energy_uj,
            134.954_780_105_047_8,
            max_relative = 1e-5
        );
        assert_relative_eq!(
            cw.average_power_w().unwrap(),
            134.954_780_105_047_8,
            max_relative = 1e-5
        );
        // ERL: derived 64.8 uJ at 162.5 MHz -> 10.5 kW (kW-class projection).
        let erl = XfelSource::erl_13nm5();
        assert_relative_eq!(
            erl.pulse_energy_uj,
            64.840_383_870_085_15,
            max_relative = 1e-5
        );
        assert_relative_eq!(
            erl.average_power_w().unwrap(),
            10_536.562_378_888_835,
            max_relative = 1e-5
        );
        assert!(dq(&erl, "hvm_power_ratio") > 40.0);
        // 7.8 MW of beam power must be energy-recovered:
        // 800 MeV x 60 pC x 162.5 MHz. Efficiency ~ 0.14 %.
        assert_relative_eq!(dq(&erl, "beam_power_average"), 7.8e6, max_relative = 1e-9);
        assert!(dq(&erl, "fel_efficiency") < 2.0 * erl.pierce_parameter().unwrap());
        // Both saturate within their 30 m undulators.
        assert!(dq(&cw, "undulator_over_saturation_length") > 1.0);
        assert!(dq(&erl, "undulator_over_saturation_length") > 1.0);
        // Many modes per pulse at 200 fs -> small single-pulse jitter.
        assert!(erl.shot_to_shot_rms() < 0.3);
    }

    #[test]
    fn test_machine_override_rescales_pulse_energy() {
        // Reference numbers from an independent pure-Python evaluation of
        // the 1D Pierce + Ming Xie formulas (series Bessel functions).
        let erl = XfelSource::erl_13nm5();
        assert_relative_eq!(erl.pulse_energy_uj, 64.840384, max_relative = 1e-6);
        let mut hot = erl.clone();
        hot.machine.as_mut().unwrap().beam.peak_current_a = 600.0;
        assert!(hot.rescale_pulse_energy_from(&erl));
        // ERL at 600 A: E_sat = 195.04 uJ -> 31.69 kW at 162.5 MHz (was
        // left at the 300 A preset's 10.54 kW before the rescale).
        assert_relative_eq!(hot.pulse_energy_uj, 195.039454, max_relative = 1e-6);
        assert_relative_eq!(
            hot.average_power_w().unwrap(),
            31_693.9113,
            max_relative = 1e-6
        );
        assert_relative_eq!(
            dq(&hot, "pulse_energy_over_saturation"),
            1.0,
            epsilon = 1e-12
        );

        // FLASH keeps its measured-class calibration (100 uJ / 116.397 uJ).
        let flash = XfelSource::flash_13nm5();
        let ratio = dq(&flash, "pulse_energy_over_saturation");
        assert_relative_eq!(ratio, 0.859128, max_relative = 1e-5);
        let mut low = flash.clone();
        low.machine.as_mut().unwrap().beam.peak_current_a = 1250.0;
        assert!(low.rescale_pulse_energy_from(&flash));
        assert_relative_eq!(low.pulse_energy_uj, 32.387739, max_relative = 1e-6);
        assert_relative_eq!(
            dq(&low, "pulse_energy_over_saturation"),
            ratio,
            max_relative = 1e-12
        );

        // Flat-top scaling: doubling T at fixed peak current doubles E.
        let mut long = erl.clone();
        long.pulse_duration_fs *= 2.0;
        assert!(long.rescale_pulse_energy_from(&erl));
        assert_relative_eq!(
            long.pulse_energy_uj,
            2.0 * erl.pulse_energy_uj,
            max_relative = 1e-12
        );

        // Unchanged machine -> unchanged pulse energy (bit-identical).
        let mut same = flash.clone();
        assert!(same.rescale_pulse_energy_from(&flash));
        assert_eq!(same.pulse_energy_uj, flash.pulse_energy_uj);

        // No machine -> nothing to derive; the stored value stays.
        let mut bare = flash.clone();
        bare.machine = None;
        bare.pulse_energy_uj = 7.0;
        assert!(!bare.rescale_pulse_energy_from(&flash));
        assert_eq!(bare.pulse_energy_uj, 7.0);
    }

    #[test]
    fn test_legacy_toml_without_machine_parses() {
        let src = XfelSource {
            machine: None,
            ..XfelSource::flash_13nm5()
        };
        let toml_str = toml::to_string(&src).unwrap();
        assert!(!toml_str.contains("machine"));
        let parsed: XfelSource = toml::from_str(&toml_str).unwrap();
        assert!(parsed.machine.is_none());
        assert!(parsed.fel_estimate().is_none());
    }
}
