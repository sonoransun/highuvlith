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
//! machine parameters (electron energy, laser wavelength, a0). With the
//! optional [`IcsCollision`] (bunch charge, laser pulse energy, spot
//! sizes) the photon yield is DERIVED from the head-on Thomson luminosity
//! and the exact rest-frame dipole collection fraction, replacing the
//! stored pulse energy (the `compact_euv_13nm5` preset does this).
//! Simplified (🔶): linear Thomson regime (`a0 << 1`, no recoil), round
//! Gaussian beams, no hourglass/crossing-angle loss, and the bandwidth
//! model (quadrature of collection-angle spread, electron energy spread,
//! neglected laser bandwidth) is a documented approximation. ICS as a
//! *lithography* source is a parameterized concept — even the aggressive
//! design point sits ~6 orders of magnitude below a 250 W HVM source;
//! treat outputs as research projections (Theoretical). Anchors: operating
//! compact Compton sources deliver ~1e10 photons/s at keV–tens-of-keV
//! energies (µW class) — e.g. the Munich Compact Light Source, 1–3e10 ph/s
//! (Eggl et al., J. Synchrotron Rad. 23, 1137 (2016)); published 6.7 nm ICS
//! designs (Sakaue et al., 2011 EUVL workshop) span 1.28e-5 W (100 kHz) to
//! 1e-3 W (100 MHz) per 2 % bandwidth. No production-scale EUV-lithography
//! ICS proposal is known.
//!
//! # Key equations
//!
//! - `lambda_X = lambda_L (1 + a0^2/2 + gamma^2 theta^2) / (4 gamma^2)`
//! - `N_x = sigma_T N_e N_L / (2 pi (sigma_e^2 + sigma_L^2))` per collision
//! - collected fraction `f = (3/8)[(1 - u) + (1 - u^3)/3]`,
//!   `u = (cos theta_c - beta) / (1 - beta cos theta_c)`
//! - in-band half-angle `theta_bw = sqrt(BW (1 + a0^2/2)) / gamma`

use serde::{Deserialize, Serialize};

use super::physics;
use crate::source::{
    evaluate_illumination, evaluate_spectral_weights, sigma_from_coherence, validate_positive,
    DerivedQuantity, IlluminationShape, LithographySource, SpectralShape,
};

/// Relative bandwidth of the Mo/Si mirror band used for the in-band
/// collection figures (2 %).
const MIRROR_BAND_REL: f64 = 0.02;

/// Collision (luminosity) parameters from which the photon yield is
/// derived.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IcsCollision {
    /// Electron bunch charge in pC.
    pub bunch_charge_pc: f64,
    /// Laser pulse energy at the interaction point in mJ (for an optical
    /// enhancement cavity: the circulating pulse energy).
    pub laser_pulse_energy_mj: f64,
    /// rms electron spot size at the interaction point, µm.
    pub electron_spot_um: f64,
    /// rms laser spot size at the interaction point, µm.
    pub laser_spot_um: f64,
}

impl IcsCollision {
    /// ASSUMED aggressive high-average-power design point: 100 pC bunches
    /// and 10 mJ circulating laser pulses (a ~1 MW enhancement cavity at
    /// 100 MHz), 10 µm rms spots. Illustrative, not a demonstrated machine.
    pub fn high_average_power_design() -> Self {
        Self {
            bunch_charge_pc: 100.0,
            laser_pulse_energy_mj: 10.0,
            electron_spot_um: 10.0,
            laser_spot_um: 10.0,
        }
    }
}

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
    /// gamma^2 theta^2 and, with a collision model, the collected flux).
    pub collection_half_angle_mrad: f64,
    /// Relative rms electron energy spread (doubles into the bandwidth).
    pub electron_energy_spread_rel: f64,
    /// Stored X-ray pulse energy in nJ (per collision); superseded by the
    /// derived value when `collision` is set.
    pub pulse_energy_nj: f64,
    /// Collision repetition rate in Hz.
    pub rep_rate_hz: f64,
    /// Transverse coherence fraction (moderate, ~0.5).
    pub transverse_coherence_fraction: f64,
    /// Number of spectral samples.
    pub spectral_samples: usize,
    /// Illumination pupil shape.
    pub illumination: IlluminationShape,
    /// Optional collision parameters: when present the photon yield and
    /// pulse energy are DERIVED (Thomson luminosity x collection
    /// fraction). `None` in configs written before the field existed.
    #[serde(default)]
    pub collision: Option<IcsCollision>,
}

impl IcsSource {
    /// Construct an ICS source whose electron energy is chosen to hit
    /// `target_wavelength_nm` on axis — the honest "derived wavelength"
    /// direction: machine parameters follow from the target. The photon
    /// yield is the stored 1 nJ / 10 kHz placeholder until a collision
    /// model is attached with [`Self::with_collision`].
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
            collision: None,
        })
    }

    /// Compact EUV concept: 1030 nm Yb laser, a0 = 0.1, tuned to 13.5 nm
    /// (requires only ~1.7 MeV electrons), collecting the whole 2 %
    /// Mo/Si band (half-angle ≈ 32 mrad), with the aggressive
    /// [`IcsCollision::high_average_power_design`] at 100 MHz. The in-band
    /// power is DERIVED: ≈ 71 µW, ~3.5 million times short of 250 W.
    pub fn compact_euv_13nm5() -> crate::error::Result<Self> {
        let mut src = Self::for_wavelength(13.5, 1030.0, 0.1)?;
        src.collection_half_angle_mrad =
            physics::ics_half_angle_for_bandwidth(src.gamma(), src.laser_a0, MIRROR_BAND_REL) * 1e3;
        src.rep_rate_hz = 1.0e8;
        src.with_collision(IcsCollision::high_average_power_design())
    }

    /// Attach collision parameters (validated positive); the photon yield
    /// and pulse energy become derived.
    pub fn with_collision(mut self, collision: IcsCollision) -> crate::error::Result<Self> {
        validate_positive("bunch_charge_pc", collision.bunch_charge_pc)?;
        validate_positive("laser_pulse_energy_mj", collision.laser_pulse_energy_mj)?;
        validate_positive("electron_spot_um", collision.electron_spot_um)?;
        validate_positive("laser_spot_um", collision.laser_spot_um)?;
        self.collision = Some(collision);
        Ok(self)
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

    /// Photons scattered into all angles per collision (`None` without a
    /// collision model).
    pub fn photons_per_collision(&self) -> Option<f64> {
        let c = self.collision?;
        let n_e = c.bunch_charge_pc * 1e-12 / physics::ELECTRON_CHARGE_C;
        let n_l =
            c.laser_pulse_energy_mj * 1e-3 / physics::photon_energy_j(self.laser_wavelength_nm);
        Some(physics::thomson_photons_per_collision(
            n_e,
            n_l,
            c.electron_spot_um * 1e-6,
            c.laser_spot_um * 1e-6,
        ))
    }

    /// Fraction of the scattered photons inside the collection half-angle.
    pub fn collected_fraction(&self) -> f64 {
        physics::thomson_collection_fraction(self.gamma(), self.collection_half_angle_mrad * 1e-3)
    }

    /// Derived collected X-ray energy per collision in J (collected
    /// photons x on-axis photon energy, a <= relative-bandwidth
    /// overestimate); `None` without a collision model.
    pub fn derived_pulse_energy_j(&self) -> Option<f64> {
        let n = self.photons_per_collision()?;
        Some(n * self.collected_fraction() * physics::photon_energy_j(self.wavelength_nm()))
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

    /// Derived collected energy per collision with a collision model,
    /// else the stored value.
    fn pulse_energy_j(&self) -> Option<f64> {
        Some(
            self.derived_pulse_energy_j()
                .unwrap_or(self.pulse_energy_nj * 1e-9),
        )
    }

    fn rep_rate_hz(&self) -> Option<f64> {
        Some(self.rep_rate_hz)
    }

    fn transverse_coherence(&self) -> f64 {
        self.transverse_coherence_fraction
    }

    fn derived_quantities(&self) -> Vec<DerivedQuantity> {
        let lambda = self.wavelength_nm();
        let gamma = self.gamma();
        let power = self.average_power_w().unwrap_or(0.0);
        let theta_bw = physics::ics_half_angle_for_bandwidth(gamma, self.laser_a0, MIRROR_BAND_REL);
        let mut out = vec![
            DerivedQuantity::new(
                "photon_energy",
                physics::HC_EV_NM / lambda,
                "eV",
                "on-axis hc / lambda_X",
            ),
            DerivedQuantity::new(
                "electron_gamma",
                gamma,
                "-",
                "1 + E_kin / m_e c^2 (E_kin derived from the target wavelength)",
            ),
            DerivedQuantity::new(
                "relative_bandwidth",
                self.relative_bandwidth(),
                "-",
                "quadrature of gamma^2 theta^2 / (1 + a0^2/2) and 2 dE/E",
            ),
            DerivedQuantity::new(
                "collected_fraction",
                self.collected_fraction(),
                "-",
                "Thomson dipole pattern boosted to the lab, inside the collection half-angle",
            ),
            DerivedQuantity::new(
                "in_band_half_angle",
                theta_bw * 1e3,
                "mrad",
                "half-angle keeping the red shift within the 2 % Mo/Si band",
            ),
            DerivedQuantity::new(
                "in_band_fraction_max",
                physics::thomson_collection_fraction(gamma, theta_bw),
                "-",
                "largest collectable fraction inside the 2 % band (~1/gamma^2 cone physics)",
            ),
        ];
        if let (Some(n), Some(c)) = (self.photons_per_collision(), self.collision) {
            let q_c = c.bunch_charge_pc * 1e-12;
            out.extend([
                DerivedQuantity::new(
                    "photons_per_collision",
                    n,
                    "photons",
                    "sigma_T N_e N_L / (2 pi (sigma_e^2 + sigma_L^2)), all angles",
                ),
                DerivedQuantity::new(
                    "collected_photons_per_collision",
                    n * self.collected_fraction(),
                    "photons",
                    "photons_per_collision x collected_fraction",
                ),
                DerivedQuantity::new(
                    "electron_beam_current",
                    q_c * self.rep_rate_hz,
                    "A",
                    "bunch charge x collision rate",
                ),
                DerivedQuantity::new(
                    "electron_beam_power",
                    self.electron_energy_mev * 1e6 * q_c * self.rep_rate_hz,
                    "W",
                    "kinetic energy x average current (power the accelerator must supply)",
                ),
                DerivedQuantity::new(
                    "laser_average_power",
                    c.laser_pulse_energy_mj * 1e-3 * self.rep_rate_hz,
                    "W",
                    "laser pulse energy x rate (circulating power for a cavity)",
                ),
            ]);
        }
        out.extend([
            DerivedQuantity::new(
                "average_power",
                power,
                "W",
                if self.collision.is_some() {
                    "DERIVED collected power (Thomson luminosity x collection)"
                } else {
                    "stored pulse energy x repetition rate (no collision model)"
                },
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
        if power > 0.0 {
            out.push(DerivedQuantity::new(
                "hvm_power_gap",
                super::throughput::HVM_EUV_POWER_AT_IF_W / power,
                "-",
                "factor by which the source falls short of 250 W",
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn dq(src: &IcsSource, name: &str) -> f64 {
        src.derived_quantities()
            .into_iter()
            .find(|q| q.name == name)
            .unwrap_or_else(|| panic!("missing derived quantity {name}"))
            .value
    }

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
        // Collision parameters must be positive.
        let base = IcsSource::for_wavelength(13.5, 1030.0, 0.1).unwrap();
        let bad = IcsCollision {
            bunch_charge_pc: 0.0,
            ..IcsCollision::high_average_power_design()
        };
        assert!(base.with_collision(bad).is_err());
    }

    #[test]
    fn test_weights_sum_to_one() {
        let src = IcsSource::compact_euv_13nm5().unwrap();
        let sum: f64 = src.spectral_weights().iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_derived_yield_fixture() {
        let src = IcsSource::compact_euv_13nm5().unwrap();
        // gamma = 4.3783 (1030 nm -> 13.5 nm, a0 = 0.1)
        assert_relative_eq!(src.gamma(), 4.378_292_411_939_207, max_relative = 1e-12);
        // 100 pC x 10 mJ @ 1030 nm, 10 um spots: 1.713e6 photons/collision
        // (scipy fixture; 4.4e-8 hc-rounding tolerance).
        let n = src.photons_per_collision().unwrap();
        assert_relative_eq!(n, 1_713_256.640_174_27, max_relative = 1e-7);
        // The preset collects exactly the 2 % band: 2.825 % of the photons.
        assert_relative_eq!(
            src.collected_fraction(),
            0.028_253_814_825_674_74,
            max_relative = 1e-9
        );
        // 4.84e4 in-band photons x 91.84 eV = 0.712 pJ; x 100 MHz = 71.2 uW.
        assert_relative_eq!(
            src.pulse_energy_j().unwrap(),
            7.122_664_087_559_873e-13,
            max_relative = 1e-7
        );
        assert_relative_eq!(
            src.average_power_w().unwrap(),
            7.122_664_087_559_872e-5,
            max_relative = 1e-7
        );
        // ~3.5 million times short of 250 W.
        let gap = dq(&src, "hvm_power_gap");
        assert!((3.0e6..4.0e6).contains(&gap), "gap {gap}");
        // 10 mA of 1.726 MeV electrons: 17.3 kW of beam power per 71 uW of EUV.
        assert_relative_eq!(
            dq(&src, "electron_beam_current"),
            0.01,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            dq(&src, "electron_beam_power"),
            src.electron_energy_mev * 1e6 * 0.01,
            max_relative = 1e-12
        );
        assert!((17_000.0..17_500.0).contains(&dq(&src, "electron_beam_power")));
    }

    #[test]
    fn test_old_constructor_keeps_stored_yield() {
        let src = IcsSource::for_wavelength(13.5, 1030.0, 0.1).unwrap();
        assert!(src.collision.is_none());
        // 1 nJ x 10 kHz = 10 uW (stored placeholder)
        assert_relative_eq!(src.average_power_w().unwrap(), 1e-5, max_relative = 1e-12);
        assert_relative_eq!(src.collection_half_angle_mrad, 1.0);
    }

    #[test]
    fn test_yield_scaling() {
        let src = IcsSource::compact_euv_13nm5().unwrap();
        let c = src.collision.unwrap();
        // Doubling laser energy doubles the yield; doubling both spots quarters it.
        let hot = src
            .clone()
            .with_collision(IcsCollision {
                laser_pulse_energy_mj: 2.0 * c.laser_pulse_energy_mj,
                ..c
            })
            .unwrap();
        assert_relative_eq!(
            hot.average_power_w().unwrap() / src.average_power_w().unwrap(),
            2.0,
            max_relative = 1e-12
        );
        let wide = src
            .clone()
            .with_collision(IcsCollision {
                electron_spot_um: 2.0 * c.electron_spot_um,
                laser_spot_um: 2.0 * c.laser_spot_um,
                ..c
            })
            .unwrap();
        assert_relative_eq!(
            wide.average_power_w().unwrap() / src.average_power_w().unwrap(),
            0.25,
            max_relative = 1e-12
        );
        // A 1 mrad aperture collects ~1000x less than the full 2 % band.
        let narrow = IcsSource {
            collection_half_angle_mrad: 1.0,
            ..src.clone()
        };
        assert!(narrow.average_power_w().unwrap() < 2e-3 * src.average_power_w().unwrap());
    }
}
