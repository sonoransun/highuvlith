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
//! Theoretical/projected: the proof-of-principle experiments demonstrated
//! the mechanism for one turn at 1064 nm (Deng et al., Nature 590, 576
//! (2021), Tsinghua/PTB at the MLS; Kruschinski et al., Commun. Phys. 7, 160
//! (2024)); steady-state EUV operation at kW power is a design projection,
//! not a demonstrated machine. The preset matches the Tsinghua EUV design
//! class (≥400 MeV, ≥1 A, ≥1 kW in a 2 % band; Acta Phys. Sin. 71, 152901);
//! a SLAC design quoted 1.12 kW per tool (IPAC2016, TUXB01). The
//! modulation-to-target integer-harmonic consistency check is live. With
//! the optional [`SsmbRadiator`] the average power is DERIVED from first
//! principles — coherent emission of a bunched beam,
//! `P = 2 pi^2 alpha hbar N Q_1(K) |b|^2 (I_avg/e)(I_pk/e)` (see
//! `physics::coherent_undulator_power_w`) — so the quadratic sensitivity
//! to the bunching factor `b` is explicit. The preset's `b` is the value
//! the 1 kW projection *requires* with its radiator (≈ 0.146 at harmonic
//! 78, i.e. ≈ 4 nm rms microbunches sustained turn after turn); it is an
//! assumption, not a demonstrated value. Microbunching dynamics (laser
//! power, lattice, quantum excitation, intra-beam scattering) are not
//! modeled.
//!
//! # Key equations
//!
//! - `lambda_r = lambda_mod / n` (integer harmonic `n`)
//! - radiator resonance: `lambda_u = 2 gamma^2 lambda_r / (1 + K^2/2)`
//! - coherent power `P = 2 pi^2 alpha hbar N Q_1(K) |b|^2 (I_avg/e)(I_pk/e)`
//! - enhancement over spontaneous emission `P_coh / P_inc = N |b|^2 (I_pk/e) lambda / c`
//! - Gaussian microbunches: `b = exp(-(k sigma_z)^2 / 2)`

use serde::{Deserialize, Serialize};

use super::physics;
use crate::source::{
    evaluate_illumination, evaluate_spectral_weights, sigma_from_coherence, validate_positive,
    DerivedQuantity, IlluminationShape, LithographySource, SpectralShape,
};

/// Beam + radiator parameters from which the coherent power is derived.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SsmbRadiator {
    /// Average stored current in A.
    pub average_current_a: f64,
    /// Peak current in A (equal to the average for a uniformly filled,
    /// DC-like ring; larger for RF-bunched fills).
    pub peak_current_a: f64,
    /// Bunching factor `|b_n|` at the target harmonic.
    pub bunching_factor: f64,
    /// Radiator undulator periods.
    pub num_periods: usize,
    /// Radiator deflection parameter K (the period follows from resonance
    /// with the ring energy and target wavelength).
    pub k: f64,
}

/// Steady-state microbunching source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsmbSource {
    /// Storage ring energy in MeV (sets the radiator period with
    /// `radiator`).
    pub ring_energy_mev: f64,
    /// Modulation laser wavelength in nm (e.g. 1053).
    pub modulation_wavelength_nm: f64,
    /// Radiated target wavelength in nm — must be an integer harmonic
    /// of the modulation wavelength (checked at construction).
    pub target_wavelength_nm: f64,
    /// Projected average output power in W (kW-class designs). Stored
    /// projection; superseded by the derived coherent power when
    /// `radiator` is set.
    pub average_power_w: f64,
    /// Relative bandwidth of the coherent harmonic (~1e-3).
    pub rel_bandwidth: f64,
    /// Transverse coherence fraction (~0.8 projected).
    pub transverse_coherence_fraction: f64,
    /// Number of spectral samples.
    pub spectral_samples: usize,
    /// Illumination pupil shape.
    pub illumination: IlluminationShape,
    /// Optional beam/radiator parameters for the derived coherent power.
    /// `None` in configs written before the field existed.
    #[serde(default)]
    pub radiator: Option<SsmbRadiator>,
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
            radiator: None,
        })
    }

    /// kW-class EUV design point: 1053 nm modulation laser, harmonic 78
    /// -> 13.5 nm, 400 MeV-class ring, 1 A DC-like stored beam, 100-period
    /// K = 1.6 radiator (7.3 mm period). The bunching factor is set to the
    /// value the 1 kW projection requires (≈ 0.146), so the derived power
    /// reproduces the projection; see the module docs for what that
    /// implies.
    pub fn euv_1kw_13nm5() -> crate::error::Result<Self> {
        let src = Self::new(400.0, 1053.0, 13.5, 1000.0)?;
        let mut radiator = SsmbRadiator {
            average_current_a: 1.0,
            peak_current_a: 1.0,
            bunching_factor: 1.0,
            num_periods: 100,
            k: 1.6,
        };
        radiator.bunching_factor = Self::bunching_required(&radiator, src.average_power_w);
        src.with_radiator(radiator)
    }

    /// Attach beam/radiator parameters (validated); the average power
    /// becomes the derived coherent power.
    pub fn with_radiator(mut self, radiator: SsmbRadiator) -> crate::error::Result<Self> {
        validate_positive("average_current_a", radiator.average_current_a)?;
        validate_positive("peak_current_a", radiator.peak_current_a)?;
        validate_positive("radiator_periods", radiator.num_periods as f64)?;
        validate_positive("radiator_k", radiator.k)?;
        if !(0.0..=1.0).contains(&radiator.bunching_factor) {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "bunching_factor",
                value: radiator.bunching_factor,
                reason: "must be in [0, 1]",
            });
        }
        if radiator.peak_current_a < radiator.average_current_a {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "peak_current_a",
                value: radiator.peak_current_a,
                reason: "must be >= average_current_a",
            });
        }
        self.radiator = Some(radiator);
        Ok(self)
    }

    /// The integer harmonic number modulation/target.
    pub fn harmonic(&self) -> usize {
        (self.modulation_wavelength_nm / self.target_wavelength_nm).round() as usize
    }

    /// Ring Lorentz factor.
    pub fn gamma(&self) -> f64 {
        physics::gamma_from_mev(self.ring_energy_mev)
    }

    /// Radiator period (mm) resonant at the target wavelength for the
    /// ring energy and radiator K: `2 gamma^2 lambda_r / (1 + K^2/2)`.
    pub fn radiator_period_mm(&self) -> Option<f64> {
        let r = self.radiator?;
        let g = self.gamma();
        Some(2.0 * g * g * self.target_wavelength_nm * 1e-6 / (1.0 + r.k * r.k / 2.0))
    }

    /// Coherent power (W) the radiator would emit at full bunching
    /// (`b = 1`); the derived power is this times `b^2`.
    fn power_at_full_bunching(radiator: &SsmbRadiator) -> f64 {
        physics::coherent_undulator_power_w(
            radiator.num_periods,
            radiator.k,
            1.0,
            radiator.average_current_a,
            radiator.peak_current_a,
        )
    }

    /// Bunching factor needed for `target_power_w` with this radiator.
    pub fn bunching_required(radiator: &SsmbRadiator, target_power_w: f64) -> f64 {
        let p1 = Self::power_at_full_bunching(radiator);
        if p1 > 0.0 && target_power_w > 0.0 {
            (target_power_w / p1).sqrt()
        } else {
            0.0
        }
    }

    /// Derived coherent average power in W (`None` without a radiator).
    pub fn coherent_power_w(&self) -> Option<f64> {
        let r = self.radiator?;
        Some(physics::coherent_undulator_power_w(
            r.num_periods,
            r.k,
            r.bunching_factor,
            r.average_current_a,
            r.peak_current_a,
        ))
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

    /// SSMB is effectively CW: no pulse structure. Derived coherent power
    /// with a radiator, else the stored projection.
    fn average_power_w(&self) -> Option<f64> {
        Some(self.coherent_power_w().unwrap_or(self.average_power_w))
    }

    fn transverse_coherence(&self) -> f64 {
        self.transverse_coherence_fraction
    }

    fn derived_quantities(&self) -> Vec<DerivedQuantity> {
        let lambda = self.target_wavelength_nm;
        let power = LithographySource::average_power_w(self).unwrap_or(0.0);
        let mut out = vec![
            DerivedQuantity::new(
                "photon_energy",
                physics::HC_EV_NM / lambda,
                "eV",
                "hc / lambda",
            ),
            DerivedQuantity::new(
                "harmonic_number",
                self.harmonic() as f64,
                "-",
                "modulation wavelength / target wavelength",
            ),
            DerivedQuantity::new("electron_gamma", self.gamma(), "-", "1 + E_kin / m_e c^2"),
        ];
        if let (Some(r), Some(period)) = (self.radiator, self.radiator_period_mm()) {
            let p_full = Self::power_at_full_bunching(&r);
            let p_inc =
                physics::undulator_central_cone_power_w(r.k, 1, r.average_current_a, lambda);
            out.extend([
                DerivedQuantity::new(
                    "radiator_period",
                    period,
                    "mm",
                    "2 gamma^2 lambda / (1 + K^2/2): resonant radiator period",
                ),
                DerivedQuantity::new(
                    "radiator_length",
                    r.num_periods as f64 * period * 1e-3,
                    "m",
                    "num_periods x period",
                ),
                DerivedQuantity::new(
                    "bunching_factor",
                    r.bunching_factor,
                    "-",
                    "|b_n| at the target harmonic (ASSUMED; must be sustained every turn)",
                ),
                DerivedQuantity::new(
                    "microbunch_rms_length",
                    physics::microbunch_length_for_bunching_nm(r.bunching_factor, lambda),
                    "nm",
                    "Gaussian microbunch length giving this b: sqrt(2 ln(1/b)) / k",
                ),
                DerivedQuantity::new(
                    "coherent_power",
                    power,
                    "W",
                    "2 pi^2 alpha hbar N Q_1 |b|^2 (I_avg/e)(I_pk/e) (= average_power_w)",
                ),
                DerivedQuantity::new(
                    "power_at_full_bunching",
                    p_full,
                    "W",
                    "coherent power at b = 1; P = this x b^2 (quadratic sensitivity)",
                ),
                DerivedQuantity::new(
                    "incoherent_power",
                    p_inc,
                    "W",
                    "spontaneous central-cone power of the same radiator and current",
                ),
                DerivedQuantity::new(
                    "coherent_enhancement",
                    if p_inc > 0.0 { power / p_inc } else { 0.0 },
                    "-",
                    "P_coh / P_inc = N |b|^2 (I_pk/e) lambda / c",
                ),
                DerivedQuantity::new(
                    "bunching_for_projection",
                    Self::bunching_required(&r, self.average_power_w),
                    "-",
                    "b needed to reach the stored projected power with this radiator",
                ),
                DerivedQuantity::new(
                    "bunching_for_hvm",
                    Self::bunching_required(&r, super::throughput::HVM_EUV_POWER_AT_IF_W),
                    "-",
                    "b needed for 250 W with this radiator",
                ),
            ]);
        }
        out.extend([
            DerivedQuantity::new(
                "projected_power",
                self.average_power_w,
                "W",
                "stored design projection",
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
        ]);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn dq(src: &SsmbSource, name: &str) -> f64 {
        src.derived_quantities()
            .into_iter()
            .find(|q| q.name == name)
            .unwrap_or_else(|| panic!("missing derived quantity {name}"))
            .value
    }

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
        // The preset's b is chosen so the derived power equals the 1 kW projection.
        assert_relative_eq!(src.average_power_w().unwrap(), 1000.0, max_relative = 1e-12);
        // CW: no pulse metadata.
        assert_eq!(src.pulse_energy_j(), None);
        assert_eq!(LithographySource::rep_rate_hz(&src), None);
        // Without a radiator the stored projection is reported.
        let stored = SsmbSource::new(400.0, 1053.0, 13.5, 750.0).unwrap();
        assert_relative_eq!(stored.average_power_w().unwrap(), 750.0);
    }

    #[test]
    fn test_weights_sum_to_one() {
        let src = SsmbSource::euv_1kw_13nm5().unwrap();
        let sum: f64 = src.spectral_weights().iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_derived_coherent_power_fixture() {
        let src = SsmbSource::euv_1kw_13nm5().unwrap();
        // b required for 1 kW with 1 A, N = 100, K = 1.6: 0.1458 (scipy fixture)
        assert_relative_eq!(
            src.radiator.unwrap().bunching_factor,
            0.145_799_697_155_692_71,
            max_relative = 1e-9
        );
        // ... i.e. 4.22 nm rms microbunches at 13.5 nm.
        assert_relative_eq!(
            dq(&src, "microbunch_rms_length"),
            4.216_412_461_474_229,
            max_relative = 1e-8
        );
        // Resonant radiator period at 400 MeV, K = 1.6: 7.27 mm.
        assert_relative_eq!(
            dq(&src, "radiator_period"),
            7.274_745_361_423_866,
            max_relative = 1e-9
        );
        // b = 0.1 would give 470.4 W (quadratic sensitivity).
        let weaker = src
            .clone()
            .with_radiator(SsmbRadiator {
                bunching_factor: 0.1,
                ..src.radiator.unwrap()
            })
            .unwrap();
        assert_relative_eq!(
            weaker.average_power_w().unwrap(),
            470.421_060_031_048_46,
            max_relative = 1e-9
        );
        // Coherent enhancement = N |b|^2 (I/e) lambda / c
        let r = src.radiator.unwrap();
        let expected =
            100.0 * r.bunching_factor.powi(2) * (1.0 / 1.602_176_634e-19) * 13.5e-9 / 299_792_458.0;
        assert_relative_eq!(
            dq(&src, "coherent_enhancement"),
            expected,
            max_relative = 1e-6
        );
    }

    #[test]
    fn test_radiator_validation() {
        let src = SsmbSource::new(400.0, 1053.0, 13.5, 1000.0).unwrap();
        let good = SsmbRadiator {
            average_current_a: 1.0,
            peak_current_a: 1.0,
            bunching_factor: 0.1,
            num_periods: 100,
            k: 1.6,
        };
        assert!(src.clone().with_radiator(good).is_ok());
        assert!(src
            .clone()
            .with_radiator(SsmbRadiator {
                bunching_factor: 1.5,
                ..good
            })
            .is_err());
        assert!(src
            .with_radiator(SsmbRadiator {
                peak_current_a: 0.5,
                ..good
            })
            .is_err());
    }
}
