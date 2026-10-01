//! Smith–Purcell (free-electron grating) radiation: tunable EUV / soft-X-ray
//! light from keV–100-keV electrons skimming a periodic nanostructure.
//!
//! An electron moving at velocity `beta c` past (or through) a structure of
//! period `a` radiates at every period; the contributions add in phase only
//! at the wavelengths satisfying the Smith–Purcell condition, which depends
//! on the observation angle — so a single device is tunable by angle, beam
//! energy, or period. Reaching EUV / soft-X-ray wavelengths with the
//! modest electron energies of an electron microscope needs nanometre and
//! sub-nanometre periods. Smith–Purcell emission itself has been demonstrated
//! experimentally only down to ~230 nm (Ye et al., 2019); at X-ray energies,
//! free-electron emission from the atomic planes of van-der-Waals crystals has
//! been reported at ~80 photons/s per nA of beam current (Huang et al., 2023)
//! — e.g. a = 0.335 nm, the graphite interlayer spacing, with 30 keV electrons
//! at 90° gives λ ≈ 1.02 nm (≈1.2 keV) in this model. **The EUV (13.5 nm)
//! preset is therefore purely theoretical (🧪)**, and the emitted power is
//! tiny (sub-nW even with an optimistic coupling), which is the whole story
//! for lithography: the derived quantities exist mainly to make the power gap
//! to HVM explicit and to set the estimate against the measured anchors.
//!
//! # Key equations
//!
//! - Dispersion (Smith–Purcell condition), order `m`, angle `theta` from the
//!   electron direction: `lambda = (a / m) (1/beta - cos theta)`.
//!   Tuning range: `(a/m)(1/beta - 1)` (forward) to `(a/m)(1/beta + 1)`
//!   (backward); `d-lambda / d-theta = (a/m) sin theta`.
//! - Kinematics: `gamma = 1 + T / m_e c^2`, `beta = sqrt(1 - 1/gamma^2)`.
//! - Evanescent coupling: the electron's field component at frequency
//!   `omega` decays as `exp(-omega x / (beta gamma c))`, so the radiated
//!   intensity from a grating at impact height `h` scales as
//!   `exp(-h / h_int)` with `h_int = beta gamma lambda / (4 pi)` — sub-nm for
//!   EUV with non-relativistic beams.
//! - Line width: `Delta-lambda / lambda = sqrt((1/(m N))^2 + (sin theta
//!   Delta-theta / (1/beta - cos theta))^2 + (energy-spread term)^2)`, the
//!   energy term being `(1/(beta^2 gamma^2)) ((gamma - 1)/gamma) (Delta-T/T)
//!   (1/beta) / (1/beta - cos theta)`.
//! - Power (ORDER-OF-MAGNITUDE ESTIMATE, not a grating-efficiency
//!   calculation): photons per electron
//!   `Y = alpha eps N exp(-h / h_int)`, rate `(I/e) Y`, power
//!   `(I/e) Y hc/lambda`. The scale `alpha` photons per electron per period
//!   is what an order-unity coupling gives (the same α-scale yield per
//!   period as an undulator with K ~ 1, or per transition-radiation
//!   interface); `eps ≤ 1` lumps the grating form factor, material
//!   absorption and beam overlap. `eps = 1, h = 0` is reported as the
//!   ideal-coupling value of this estimate (a heuristic scale, not a
//!   rigorous limit).
//!
//! # Model status
//!
//! 🧪 Theoretical as a lithography source (no EUV Smith–Purcell emission has
//! been measured). The dispersion relation, tuning range, interaction height
//! and line-width terms are exact kinematics (✅-grade, fixture-tested). The
//! absolute power is an explicitly labelled order-of-magnitude estimate: the
//! default coupling `eps = 1e-3` is an optimistic ASSUMPTION — it gives
//! ~4.6×10⁶ photons/s per nA, about 5×10⁴ times the measured van-der-Waals
//! X-ray anchor (~80 photons/s per nA ≈ 1.3×10⁻⁸ photons per electron), and
//! realistic EUV couplings are unknown; no grating-profile (van den Berg /
//! surface-current) efficiency is computed, beam energy loss and scattering
//! inside penetrated structures are ignored, and the angular emission pattern
//! is not modelled (the source is characterized at one observation angle).
//! The default estimate (0.67 nW at 10 nA) is ~4×10¹¹ below the 250 W HVM
//! class, and even the ideal-coupling value (eps = 1, h = 0: 0.67 µW) is
//! ~4×10⁸ below it.
//!
//! # References
//!
//! - S. J. Smith and E. M. Purcell, "Visible light from localized surface
//!   charges moving across a grating," Phys. Rev. 92, 1069 (1953).
//! - Measured anchors (hedged, author + year): Ye et al. (2019) — shortest
//!   demonstrated Smith–Purcell wavelength ~230 nm; Huang et al. (2023) —
//!   van-der-Waals-crystal free-electron X-rays, ~80 photons/s per nA.

use serde::{Deserialize, Serialize};

use super::dpp::HVM_REFERENCE_POWER_W;
use super::physics::{ELECTRON_CHARGE_C, ELECTRON_REST_MEV, FINE_STRUCTURE, HC_EV_NM};
use crate::source::{
    evaluate_illumination, evaluate_spectral_weights, DerivedQuantity, IlluminationShape,
    LithographySource, SpectralShape,
};

/// Measured van-der-Waals-crystal free-electron X-ray yield used as a
/// real-machine anchor: ~80 photons/s per nA of beam current (Huang et al.,
/// 2023; X-ray regime, reported).
pub const VDW_XRAY_ANCHOR_PHOTONS_PER_S_PER_NA: f64 = 80.0;

/// Smith–Purcell wavelength `(a/m)(1/beta - cos theta)` in nm.
pub fn smith_purcell_wavelength_nm(period_nm: f64, order: usize, beta: f64, theta_deg: f64) -> f64 {
    period_nm / order as f64 * (1.0 / beta - theta_deg.to_radians().cos())
}

/// Electron `(gamma, beta)` for a kinetic energy in keV.
pub fn electron_gamma_beta(kinetic_energy_kev: f64) -> (f64, f64) {
    let gamma = 1.0 + kinetic_energy_kev / (ELECTRON_REST_MEV * 1e3);
    (gamma, (1.0 - 1.0 / (gamma * gamma)).sqrt())
}

/// Smith–Purcell free-electron grating source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmithPurcellSource {
    /// Electron kinetic energy in keV (SEM/TEM class: 1–300 keV).
    pub electron_energy_kev: f64,
    /// Structure period along the electron path, in nm.
    pub grating_period_nm: f64,
    /// Diffraction order m (≥ 1).
    pub diffraction_order: usize,
    /// Observation angle from the electron direction, in degrees (0, 180).
    pub observation_angle_deg: f64,
    /// Number of periods interacting with the beam (sets the 1/(mN) width
    /// and the yield).
    pub num_periods: usize,
    /// Beam current in nA.
    pub beam_current_na: f64,
    /// Beam impact height above the structure in nm (0 = grazing contact /
    /// penetrating geometry).
    pub impact_height_nm: f64,
    /// Coupling efficiency eps in (0, 1] (ASSUMED; lumps form factor,
    /// absorption, beam overlap).
    pub coupling_efficiency: f64,
    /// Collection half-angle in mrad (angular contribution to the width).
    pub collection_half_angle_mrad: f64,
    /// Relative rms electron kinetic-energy spread ΔT/T.
    pub electron_energy_spread_rel: f64,
    /// Number of spectral samples.
    pub spectral_samples: usize,
    /// Illumination pupil shape (user choice; the source is not a
    /// projection-scanner source).
    pub illumination: IlluminationShape,
}

impl SmithPurcellSource {
    /// Construct from the machine parameters; the wavelength is DERIVED
    /// from the dispersion relation. Defaults: 100 periods, 10 nA, grazing
    /// contact (h = 0), eps = 1e-3 (assumed), 10 mrad collection, 1e-4
    /// energy spread.
    pub fn new(
        electron_energy_kev: f64,
        grating_period_nm: f64,
        diffraction_order: usize,
        observation_angle_deg: f64,
    ) -> crate::error::Result<Self> {
        let src = Self {
            electron_energy_kev,
            grating_period_nm,
            diffraction_order,
            observation_angle_deg,
            num_periods: 100,
            beam_current_na: 10.0,
            impact_height_nm: 0.0,
            coupling_efficiency: 1e-3,
            collection_half_angle_mrad: 10.0,
            electron_energy_spread_rel: 1e-4,
            spectral_samples: 5,
            illumination: IlluminationShape::Conventional { sigma: 0.7 },
        };
        src.validate()?;
        Ok(src)
    }

    /// Construct the structure period that puts order `m` at
    /// `target_wavelength_nm` for the given beam energy and angle:
    /// `a = m lambda / (1/beta - cos theta)` (the "derived" direction).
    pub fn for_wavelength(
        target_wavelength_nm: f64,
        electron_energy_kev: f64,
        diffraction_order: usize,
        observation_angle_deg: f64,
    ) -> crate::error::Result<Self> {
        if !(target_wavelength_nm.is_finite() && target_wavelength_nm > 0.0) {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "target_wavelength_nm",
                value: target_wavelength_nm,
                reason: "must be positive",
            });
        }
        if !(electron_energy_kev.is_finite() && electron_energy_kev > 0.0) {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "electron_energy_kev",
                value: electron_energy_kev,
                reason: "must be positive",
            });
        }
        let (_, beta) = electron_gamma_beta(electron_energy_kev);
        let factor = 1.0 / beta - observation_angle_deg.to_radians().cos();
        let period = diffraction_order as f64 * target_wavelength_nm / factor;
        Self::new(
            electron_energy_kev,
            period,
            diffraction_order,
            observation_angle_deg,
        )
    }

    /// EUV design point: 30 keV (SEM-class) electrons, first order at 90°,
    /// period derived for 13.5 nm (a = 4.43 nm), 100 periods, 10 nA.
    pub fn euv_13nm5() -> crate::error::Result<Self> {
        Self::for_wavelength(13.5, 30.0, 1, 90.0)
    }

    /// Check every field; called by the constructors and available to
    /// frontends that override fields after construction.
    pub fn validate(&self) -> crate::error::Result<()> {
        use crate::error::LithographyError::InvalidParameter;
        for (name, value) in [
            ("electron_energy_kev", self.electron_energy_kev),
            ("grating_period_nm", self.grating_period_nm),
            ("beam_current_na", self.beam_current_na),
        ] {
            if !(value.is_finite() && value > 0.0) {
                return Err(InvalidParameter {
                    name,
                    value,
                    reason: "must be positive",
                });
            }
        }
        if self.diffraction_order == 0 {
            return Err(InvalidParameter {
                name: "diffraction_order",
                value: 0.0,
                reason: "must be >= 1",
            });
        }
        if self.num_periods == 0 {
            return Err(InvalidParameter {
                name: "num_periods",
                value: 0.0,
                reason: "must be >= 1",
            });
        }
        if !(self.observation_angle_deg > 0.0 && self.observation_angle_deg < 180.0) {
            return Err(InvalidParameter {
                name: "observation_angle_deg",
                value: self.observation_angle_deg,
                reason: "must be in (0, 180) degrees from the electron direction",
            });
        }
        for (name, value) in [
            ("impact_height_nm", self.impact_height_nm),
            (
                "collection_half_angle_mrad",
                self.collection_half_angle_mrad,
            ),
            (
                "electron_energy_spread_rel",
                self.electron_energy_spread_rel,
            ),
        ] {
            if !(value.is_finite() && value >= 0.0) {
                return Err(InvalidParameter {
                    name,
                    value,
                    reason: "must be >= 0",
                });
            }
        }
        if !(self.coupling_efficiency > 0.0 && self.coupling_efficiency <= 1.0) {
            return Err(InvalidParameter {
                name: "coupling_efficiency",
                value: self.coupling_efficiency,
                reason: "must be in (0, 1]",
            });
        }
        if self.spectral_samples == 0 {
            return Err(InvalidParameter {
                name: "spectral_samples",
                value: 0.0,
                reason: "must be >= 1",
            });
        }
        Ok(())
    }

    /// Electron Lorentz factor.
    pub fn gamma(&self) -> f64 {
        electron_gamma_beta(self.electron_energy_kev).0
    }

    /// Electron velocity in units of c.
    pub fn beta(&self) -> f64 {
        electron_gamma_beta(self.electron_energy_kev).1
    }

    /// Emitted wavelength (nm) at an arbitrary observation angle.
    pub fn wavelength_at_angle_nm(&self, theta_deg: f64) -> f64 {
        smith_purcell_wavelength_nm(
            self.grating_period_nm,
            self.diffraction_order,
            self.beta(),
            theta_deg,
        )
    }

    /// Angular tuning range `((a/m)(1/beta - 1), (a/m)(1/beta + 1))` in nm.
    pub fn tuning_range_nm(&self) -> (f64, f64) {
        (
            self.wavelength_at_angle_nm(0.0),
            self.wavelength_at_angle_nm(180.0),
        )
    }

    /// Angular tuning slope `(a/m) sin theta` in nm per degree.
    pub fn tuning_slope_nm_per_deg(&self) -> f64 {
        self.grating_period_nm / self.diffraction_order as f64
            * self.observation_angle_deg.to_radians().sin()
            * std::f64::consts::PI
            / 180.0
    }

    /// Evanescent interaction height `beta gamma lambda / (4 pi)` in nm.
    pub fn interaction_height_nm(&self) -> f64 {
        self.beta() * self.gamma() * self.wavelength_nm() / (4.0 * std::f64::consts::PI)
    }

    /// Evanescent coupling factor `exp(-h / h_int)`.
    pub fn coupling_factor(&self) -> f64 {
        (-self.impact_height_nm / self.interaction_height_nm()).exp()
    }

    /// Photons per electron (order-of-magnitude estimate)
    /// `alpha eps N exp(-h/h_int)`.
    pub fn photons_per_electron(&self) -> f64 {
        FINE_STRUCTURE * self.coupling_efficiency * self.num_periods as f64 * self.coupling_factor()
    }

    /// Emitted photon rate (photons/s).
    pub fn photon_rate(&self) -> f64 {
        self.beam_current_na * 1e-9 / ELECTRON_CHARGE_C * self.photons_per_electron()
    }

    /// Emitted power estimate in W.
    pub fn emitted_power_w(&self) -> f64 {
        self.photon_rate() * self.photon_energy_ev() * ELECTRON_CHARGE_C
    }

    /// Ideal-coupling value of the power estimate (eps = 1, h = 0) in W —
    /// the heuristic scale of alpha photons per electron per period, not a
    /// rigorous limit.
    pub fn ideal_coupling_power_w(&self) -> f64 {
        self.beam_current_na * 1e-9 / ELECTRON_CHARGE_C
            * FINE_STRUCTURE
            * self.num_periods as f64
            * self.photon_energy_ev()
            * ELECTRON_CHARGE_C
    }

    /// Relative line width (quadrature of the finite-N, collection-angle
    /// and energy-spread terms).
    pub fn relative_bandwidth(&self) -> f64 {
        let gamma = self.gamma();
        let beta = self.beta();
        let theta = self.observation_angle_deg.to_radians();
        let factor = 1.0 / beta - theta.cos();
        let natural = 1.0 / (self.diffraction_order as f64 * self.num_periods as f64);
        let angular = theta.sin() * self.collection_half_angle_mrad * 1e-3 / factor;
        let energy = (1.0 / (beta * beta * gamma * gamma))
            * ((gamma - 1.0) / gamma)
            * self.electron_energy_spread_rel
            * (1.0 / beta)
            / factor;
        (natural * natural + angular * angular + energy * energy).sqrt()
    }
}

impl Default for SmithPurcellSource {
    fn default() -> Self {
        Self::euv_13nm5().expect("EUV Smith-Purcell preset is valid")
    }
}

impl LithographySource for SmithPurcellSource {
    /// DERIVED from the Smith–Purcell condition at the observation angle.
    fn wavelength_nm(&self) -> f64 {
        self.wavelength_at_angle_nm(self.observation_angle_deg)
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

    /// Continuous beam: the order-of-magnitude emitted-power estimate.
    fn average_power_w(&self) -> Option<f64> {
        Some(self.emitted_power_w())
    }

    fn derived_quantities(&self) -> Vec<DerivedQuantity> {
        let power = self.emitted_power_w();
        let (lmin, lmax) = self.tuning_range_nm();
        vec![
            DerivedQuantity::new(
                "electron_beta",
                self.beta(),
                "-",
                "v/c from the beam energy",
            ),
            DerivedQuantity::new(
                "emitted_wavelength_nm",
                self.wavelength_nm(),
                "nm",
                "lambda = (a/m)(1/beta - cos theta)",
            ),
            DerivedQuantity::new(
                "photon_energy_ev",
                HC_EV_NM / self.wavelength_nm(),
                "eV",
                "hc / lambda",
            ),
            DerivedQuantity::new(
                "tuning_min_wavelength_nm",
                lmin,
                "nm",
                "forward emission limit (a/m)(1/beta - 1)",
            ),
            DerivedQuantity::new(
                "tuning_max_wavelength_nm",
                lmax,
                "nm",
                "backward emission limit (a/m)(1/beta + 1)",
            ),
            DerivedQuantity::new(
                "tuning_slope_nm_per_deg",
                self.tuning_slope_nm_per_deg(),
                "nm/deg",
                "(a/m) sin theta",
            ),
            DerivedQuantity::new(
                "interaction_height_nm",
                self.interaction_height_nm(),
                "nm",
                "evanescent 1/e height beta gamma lambda / (4 pi)",
            ),
            DerivedQuantity::new(
                "evanescent_coupling_factor",
                self.coupling_factor(),
                "-",
                "exp(-h / h_int) at the set impact height",
            ),
            DerivedQuantity::new(
                "photons_per_electron",
                self.photons_per_electron(),
                "photons",
                "ORDER-OF-MAGNITUDE alpha*eps*N*exp(-h/h_int); eps assumed",
            ),
            DerivedQuantity::new(
                "photon_rate",
                self.photon_rate(),
                "photons/s",
                "(I/e) x photons per electron",
            ),
            DerivedQuantity::new(
                "photon_rate_per_na",
                self.photon_rate() / self.beam_current_na,
                "photons/s/nA",
                "compare: measured vdW-crystal X-ray anchor ~80 photons/s/nA (Huang et al. 2023); \
                 SP demonstrated only down to ~230 nm (Ye et al. 2019)",
            ),
            DerivedQuantity::new(
                "emitted_power_w",
                power,
                "W",
                "order-of-magnitude estimate at the assumed coupling",
            ),
            DerivedQuantity::new(
                "ideal_coupling_power_w",
                self.ideal_coupling_power_w(),
                "W",
                "ideal-coupling estimate eps = 1, h = 0 (heuristic scale, not a rigorous limit)",
            ),
            DerivedQuantity::new(
                "relative_bandwidth",
                self.relative_bandwidth(),
                "-",
                "quadrature of 1/(mN), collection angle and energy spread",
            ),
            DerivedQuantity::new(
                "hvm_power_gap",
                HVM_REFERENCE_POWER_W / power,
                "x",
                "250 W HVM-class EUV power / this estimate",
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // Fixtures computed independently with NumPy.

    #[test]
    fn test_electron_kinematics_fixture() {
        let (g30, b30) = electron_gamma_beta(30.0);
        assert_relative_eq!(g30, 1.058_709, max_relative = 1e-6);
        assert_relative_eq!(b30, 0.328_376, max_relative = 1e-5);
        let (g100, b100) = electron_gamma_beta(100.0);
        assert_relative_eq!(g100, 1.195_695, max_relative = 1e-6);
        assert_relative_eq!(b100, 0.548_221, max_relative = 1e-5);
    }

    #[test]
    fn test_dispersion_relation_fixture() {
        let src = SmithPurcellSource::euv_13nm5().unwrap();
        // a = m lambda beta (theta = 90 deg) = 13.5 x 0.328376 = 4.433078 nm
        assert_relative_eq!(src.grating_period_nm, 4.433_078, max_relative = 1e-6);
        assert_relative_eq!(src.wavelength_nm(), 13.5, max_relative = 1e-12);
        // 45 deg: 10.365340 nm; second order at 90 deg: half the wavelength.
        assert_relative_eq!(
            src.wavelength_at_angle_nm(45.0),
            10.365_340,
            max_relative = 1e-6
        );
        let mut m2 = src.clone();
        m2.diffraction_order = 2;
        assert_relative_eq!(m2.wavelength_nm(), 6.75, max_relative = 1e-12);
        // vdW-lattice example: a = 0.335 nm, 30 keV, 90 deg -> 1.020171 nm.
        let vdw = SmithPurcellSource::new(30.0, 0.335, 1, 90.0).unwrap();
        assert_relative_eq!(vdw.wavelength_nm(), 1.020_171_45, max_relative = 1e-8);
    }

    #[test]
    fn test_tuning_range_and_limits() {
        let src = SmithPurcellSource::euv_13nm5().unwrap();
        let (lmin, lmax) = src.tuning_range_nm();
        let a = src.grating_period_nm;
        let b = src.beta();
        assert_relative_eq!(lmin, a * (1.0 / b - 1.0), max_relative = 1e-12);
        assert_relative_eq!(lmax, a * (1.0 / b + 1.0), max_relative = 1e-12);
        // Wavelength increases monotonically with angle.
        assert!(src.wavelength_at_angle_nm(30.0) < src.wavelength_at_angle_nm(60.0));
        // Relativistic limit: beta -> 1 at 90 deg gives lambda -> a/m.
        let fast = SmithPurcellSource::new(10_000.0, 10.0, 1, 90.0).unwrap();
        assert_relative_eq!(fast.wavelength_nm() / 10.0, 1.0, max_relative = 2e-3);
        // Faster electrons at fixed period and angle emit shorter wavelengths.
        let slow = SmithPurcellSource::new(30.0, 10.0, 1, 90.0).unwrap();
        assert!(fast.wavelength_nm() < slow.wavelength_nm());
        // Slope (a/m) sin(theta) pi/180 at 90 deg.
        assert_relative_eq!(
            src.tuning_slope_nm_per_deg(),
            a * std::f64::consts::PI / 180.0,
            max_relative = 1e-12
        );
    }

    #[test]
    fn test_interaction_height_and_evanescent_suppression() {
        let mut src = SmithPurcellSource::euv_13nm5().unwrap();
        // beta gamma lambda / (4 pi) = 0.347655 x 13.5 / 4 pi = 0.373484 nm
        assert_relative_eq!(src.interaction_height_nm(), 0.373_484, max_relative = 1e-5);
        assert_relative_eq!(src.coupling_factor(), 1.0);
        src.impact_height_nm = 5.0;
        // exp(-5 / 0.373484) = 1.534e-6: EUV SP from a beam above a grating
        // is exponentially suppressed.
        assert_relative_eq!(
            src.coupling_factor(),
            (-5.0_f64 / 0.373_483_964_815_437_7).exp(),
            max_relative = 1e-5
        );
        assert!(src.coupling_factor() < 2e-6);
    }

    #[test]
    fn test_power_estimate_fixture_and_hvm_gap() {
        let src = SmithPurcellSource::euv_13nm5().unwrap();
        // alpha x 1e-3 x 100 = 7.2974e-4 photons per electron
        assert_relative_eq!(
            src.photons_per_electron(),
            7.297_352_569_3e-4,
            max_relative = 1e-12
        );
        // 10 nA -> 4.5546e7 photons/s -> 0.67019 nW
        assert_relative_eq!(src.photon_rate(), 4.5546e7, max_relative = 1e-4);
        assert_relative_eq!(src.emitted_power_w(), 6.7019e-10, max_relative = 1e-4);
        assert_relative_eq!(src.ideal_coupling_power_w(), 6.7019e-7, max_relative = 1e-4);
        let gap = src
            .derived_quantities()
            .into_iter()
            .find(|q| q.name == "hvm_power_gap")
            .unwrap()
            .value;
        assert_relative_eq!(gap, 3.7303e11, max_relative = 1e-4);
        // Per-nA photon rate vs the measured vdW-crystal X-ray anchor
        // (~80 photons/s/nA): the assumed coupling is ~5.7e4x more optimistic.
        let per_na = src
            .derived_quantities()
            .into_iter()
            .find(|q| q.name == "photon_rate_per_na")
            .unwrap();
        assert_relative_eq!(per_na.value, 4.554_649_2e6, max_relative = 1e-7);
        assert!(per_na.note.contains("Huang"));
        assert_relative_eq!(
            per_na.value / VDW_XRAY_ANCHOR_PHOTONS_PER_S_PER_NA,
            5.693_311_5e4,
            max_relative = 1e-6
        );
        // Linear in current, periods and coupling.
        let mut hot = src.clone();
        hot.beam_current_na *= 3.0;
        hot.num_periods *= 2;
        assert_relative_eq!(
            hot.emitted_power_w() / src.emitted_power_w(),
            6.0,
            max_relative = 1e-12
        );
    }

    #[test]
    fn test_bandwidth_fixture() {
        let src = SmithPurcellSource::euv_13nm5().unwrap();
        // sqrt(0.01^2 + (0.01 beta)^2 + (4.588e-5)^2) = 0.0105255
        assert_relative_eq!(src.relative_bandwidth(), 0.010_525_5, max_relative = 1e-4);
        assert!(src.relative_bandwidth() >= 1.0 / 100.0);
        let sum: f64 = src.spectral_weights().iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_validation() {
        assert!(SmithPurcellSource::new(30.0, 4.4, 0, 90.0).is_err());
        assert!(SmithPurcellSource::new(30.0, 4.4, 1, 0.0).is_err());
        assert!(SmithPurcellSource::new(30.0, 4.4, 1, 180.0).is_err());
        assert!(SmithPurcellSource::new(30.0, -4.4, 1, 90.0).is_err());
        assert!(SmithPurcellSource::new(0.0, 4.4, 1, 90.0).is_err());
        assert!(SmithPurcellSource::for_wavelength(-13.5, 30.0, 1, 90.0).is_err());
        let mut s = SmithPurcellSource::euv_13nm5().unwrap();
        s.coupling_efficiency = 0.0;
        assert!(s.validate().is_err());
        s.coupling_efficiency = 1.5;
        assert!(s.validate().is_err());
        s.coupling_efficiency = 1e-3;
        s.impact_height_nm = -1.0;
        assert!(s.validate().is_err());
        s.impact_height_nm = 0.0;
        s.num_periods = 0;
        assert!(s.validate().is_err());
    }
}
