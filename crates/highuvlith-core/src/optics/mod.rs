//! Optical systems and their pupil functions.
//!
//! Defines the [`OpticalSystem`] contract used by the aerial-image engine: a
//! complex pupil function `P(fx, fy; defocus, λ)` whose magnitude is the pupil
//! transmission (apodization) and whose phase carries the aberrations plus the
//! defocus term, together with the numerical aperture, reduction ratio, flare
//! fraction, and chromatic-defocus queries the pipeline needs.
//!
//! [`ProjectionOptics`] is the refractive CaF2 implementation for VUV: a
//! circular pupil with Fringe-Zernike aberration phase, optional radial
//! [`Apodization`], uniform flare, and an axial-chromatic coefficient
//! (nm defocus per pm of wavelength offset) that feeds polychromatic imaging.
//! Sibling modules [`zone_plate`] and [`schwarzschild`] provide diffractive
//! and reflective alternatives through the same trait.
//!
//! # Key equations
//!
//! ```text
//!   P(ρ,θ) = A(ρ) · exp[ i ( W_aberr(ρ,θ) + Φ_z(ρ) ) ],   ρ ≤ 1
//!   Φ_z(ρ) = (2π n/λ) · z · (1 − √(1 − (NA ρ/n)²))   (exact focus phase in a medium of index n)
//!          ≈ π z NA² ρ² / (n λ)                       (paraxial option, NA ≪ n)
//!   f_cutoff = NA / λ            Rayleigh resolution = 0.61 λ / NA
//!   immersion: NA = n · sin θ_max ≤ 0.95 n;   Rayleigh DOF ≈ n λ / (2 NA²)
//! ```
//! Frequencies are normalized to `f_cutoff`; the pupil is zero for ρ > 1.
//! `Φ_z` is the phase a plane wave at `n sin θ = NA·ρ` accumulates over a
//! focus displacement `z` in the image-space medium (index `n`: 1 dry,
//! ≈ 1.437 water at 193 nm) relative to the axial wave — angular-spectrum
//! propagation, exact at any NA; the quadratic form is its small-angle limit
//! and is kept behind `paraxial_defocus`. The scalar cutoff `NA/λ` does not
//! depend on `n` (the medium changes the angles, not the transmitted mask
//! spatial frequencies).
//!
//! # Model status
//!
//! Scalar pupil model — transmission and phase only; polarization / vector
//! high-NA effects live in [`vector`] and are applied by the engine on top of
//! this scalar pupil. Aberrations are user-supplied Zernike coefficients;
//! chromatic defocus is linear in the wavelength offset. The defocus phase is
//! evaluated in the image-space medium reported by
//! [`OpticalSystem::immersion_index`] (1 unless a [`ProjectionOptics`] is
//! built for immersion); focus inside the resist (index of the resist rather
//! than the immersion fluid) is not modeled. [`euv::EuvProjectionOptics`] is
//! an isotropic wafer-side model of EUV scanner projection optics (anamorphic
//! magnification and mask-3D effects not modeled).

pub mod euv;
pub mod multilayer_pupil;
pub mod schwarzschild;
pub mod vector;
pub mod zone_plate;

use num::Complex;
use serde::{Deserialize, Serialize};

use crate::math::zernike;
use crate::types::Complex64;

/// Trait for any optical system that produces a pupil function.
/// The aerial image engine uses this interface, enabling refractive lenses,
/// zone plates, grazing-incidence mirrors, and Schwarzschild objectives.
///
/// Conventions the engine relies on:
/// - `pupil_function(fx, fy, z, λ)` is the pupil *at wavelength λ* with the
///   image plane displaced by `z` from best focus at λ; it must NOT include
///   any chromatic focus shift (the engine adds [`Self::chromatic_defocus`]
///   explicitly when it images other spectral samples).
/// - `(fx, fy)` are normalized to [`Self::cutoff_frequency`] at that λ.
pub trait OpticalSystem: Send + Sync {
    /// Evaluate pupil function at normalized frequency (fx, fy).
    fn pupil_function(
        &self,
        fx_norm: f64,
        fy_norm: f64,
        defocus_nm: f64,
        wavelength_nm: f64,
    ) -> Complex64;

    /// Image-side numerical aperture.
    fn na(&self) -> f64;

    /// Reduction ratio (e.g., 4.0 for 4x demagnification).
    fn reduction(&self) -> f64;

    /// Stray light / flare fraction.
    fn flare_fraction(&self) -> f64;

    /// Boxed clone, so the imaging engine can keep its own copy of the
    /// optics (it rebuilds kernels at new focus planes and wavelengths).
    fn clone_box(&self) -> Box<dyn OpticalSystem>;

    /// Chromatic defocus for a wavelength offset (pm). Default: 0 (achromatic).
    fn chromatic_defocus(&self, _delta_wavelength_pm: f64) -> f64 {
        0.0
    }

    /// Cutoff frequency in 1/nm.
    fn cutoff_frequency(&self, wavelength_nm: f64) -> f64 {
        self.na() / wavelength_nm
    }

    /// Refractive index of the image-space medium (1 = dry / vacuum). The
    /// defocus phase inside the pupil is computed in this medium, the
    /// imaging engine requires `na() < immersion_index()`, and vector
    /// imaging requires `VectorSettings::image_index` to equal it (it is
    /// inherited when left at 1). Default: 1.
    fn immersion_index(&self) -> f64 {
        1.0
    }

    /// Prepare wavelength-dependent pupil data (e.g. tabulate a multilayer
    /// coating response) before the pupil is sampled at `wavelength_nm`;
    /// errors if the optic cannot be evaluated there. The imaging engine
    /// calls it before every kernel build. Default: nothing to do.
    fn prepare(&self, _wavelength_nm: f64) -> crate::error::Result<()> {
        Ok(())
    }

    /// Rayleigh resolution limit in nm.
    fn rayleigh_resolution(&self, wavelength_nm: f64) -> f64 {
        0.61 * wavelength_nm / self.na()
    }
}

impl Clone for Box<dyn OpticalSystem> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Defocus phase (radians) of the plane-wave order at `sin θ = sin_theta`
/// for a focus displacement `defocus_nm` at `wavelength_nm`, in vacuum/air.
///
/// Exact (angular-spectrum) form `(2π/λ)·z·(1 − cos θ)`, evaluated as
/// `(2π/λ)·z·sin²θ / (1 + cos θ)` to avoid cancellation at small angles;
/// `paraxial = true` returns the small-angle limit `π·z·sin²θ/λ`. Both share
/// the sign convention (positive `z` → positive phase at the pupil edge).
/// Arguments with `sin θ ≥ 1` (evanescent) are clamped to `cos θ = 0`.
/// Identical to [`defocus_phase_in_medium`] with `medium_index = 1`.
pub fn defocus_phase(defocus_nm: f64, sin_theta: f64, wavelength_nm: f64, paraxial: bool) -> f64 {
    defocus_phase_in_medium(defocus_nm, sin_theta, 1.0, wavelength_nm, paraxial)
}

/// Defocus phase (radians) of the order with image-side numerical aperture
/// `na_rho = n·sin θ` (i.e. `NA·ρ`) for a focus displacement `defocus_nm`
/// inside a medium of refractive index `medium_index` (`n`):
///
/// ```text
///   exact:     Φ = (2π n/λ)·z·(1 − √(1 − (NAρ/n)²)) = (2π/λ)·z·(n − √(n² − NA²ρ²))
///   paraxial:  Φ = π z NA²ρ² / (n λ)
/// ```
/// `λ` is the vacuum wavelength. Continuous in `n` and equal to
/// [`defocus_phase`] at `n = 1`; orders with `NAρ ≥ n` (evanescent) are
/// clamped to `cos θ = 0`.
pub fn defocus_phase_in_medium(
    defocus_nm: f64,
    na_rho: f64,
    medium_index: f64,
    wavelength_nm: f64,
    paraxial: bool,
) -> f64 {
    let s = na_rho / medium_index;
    let s2 = s * s;
    if paraxial {
        std::f64::consts::PI * defocus_nm * medium_index * s2 / wavelength_nm
    } else {
        let cos_theta = (1.0 - s2).max(0.0).sqrt();
        2.0 * std::f64::consts::PI * medium_index * defocus_nm * s2
            / ((1.0 + cos_theta) * wavelength_nm)
    }
}

/// Refractive index of ultrapure water at the ArF line (193.4 nm), near room
/// temperature: published measurements used for ArF immersion lithography
/// give n ≈ 1.436–1.437 (1.44 is the common rounded figure). Used by
/// [`ProjectionOptics::immersion_193i`].
pub const WATER_INDEX_193NM: f64 = 1.437;

/// Largest usable immersion NA as a fraction of the medium index: the
/// marginal ray must stay clear of grazing incidence in the fluid.
pub const IMMERSION_NA_MARGIN: f64 = 0.95;

fn default_immersion_index() -> f64 {
    1.0
}

/// Pupil apodization model (transmission variation across the pupil).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum Apodization {
    /// Uniform transmission.
    #[default]
    Uniform,
    /// Radial transmission profile: T(rho) = 1 - alpha * rho^2.
    Quadratic { alpha: f64 },
    /// Gaussian apodization: T(rho) = exp(-alpha * rho^2).
    Gaussian { alpha: f64 },
}

/// Projection optics specification for VUV lithography.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionOptics {
    /// Numerical aperture (image side).
    pub na: f64,
    /// Reduction ratio (e.g., 4.0 for 4x reduction).
    pub reduction: f64,
    /// Zernike aberration coefficients: (fringe_index, coefficient_in_waves).
    pub zernike_coefficients: Vec<(usize, f64)>,
    /// Flare fraction (stray light added as uniform background).
    pub flare_fraction: f64,
    /// Axial chromatic aberration coefficient (nm defocus per pm bandwidth).
    /// For CaF2-only lens at 157nm, typical value ~10-30 nm/pm.
    pub axial_chromatic_nm_per_pm: f64,
    /// Pupil apodization from coating effects.
    pub apodization: Apodization,
    /// Use the paraxial (quadratic) defocus phase `π z NA² ρ²/(n λ)` instead
    /// of the exact `(2π n/λ) z (1 − √(1 − (NAρ/n)²))`. Default `false`.
    #[serde(default)]
    pub paraxial_defocus: bool,
    /// Refractive index of the immersion medium between the last lens
    /// element and the wafer (1.0 = dry). Build immersion optics with
    /// [`ProjectionOptics::immersion`]; `NA` may then exceed 1 (up to
    /// `IMMERSION_NA_MARGIN · n`).
    #[serde(default = "default_immersion_index")]
    pub immersion_index: f64,
}

impl ProjectionOptics {
    /// Create optics with default VUV parameters.
    pub fn new(na: f64) -> crate::error::Result<Self> {
        if na.is_nan() || na <= 0.0 || na >= 1.0 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "numerical_aperture",
                value: if na.is_nan() { f64::NAN } else { na },
                reason: "must be in range (0, 1) and not NaN",
            });
        }
        Ok(Self {
            na,
            reduction: 4.0,
            zernike_coefficients: Vec::new(),
            flare_fraction: 0.02, // 2% flare typical for VUV
            axial_chromatic_nm_per_pm: 15.0,
            apodization: Apodization::default(),
            paraxial_defocus: false,
            immersion_index: 1.0,
        })
    }

    /// Immersion projection optics: numerical aperture `na = n·sin θ_max`
    /// with an immersion medium of index `immersion_index` (`n ≥ 1`), valid
    /// for `0 < na ≤ IMMERSION_NA_MARGIN · n`. Defaults as [`Self::new`]
    /// except `axial_chromatic_nm_per_pm = 0` (the 157 nm CaF2 value of
    /// `new` does not apply; set the coefficient of your lens).
    pub fn immersion(na: f64, immersion_index: f64) -> crate::error::Result<Self> {
        if !(immersion_index.is_finite() && immersion_index >= 1.0) {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "immersion_index",
                value: immersion_index,
                reason: "must be finite and >= 1",
            });
        }
        if !(na.is_finite() && na > 0.0 && na <= IMMERSION_NA_MARGIN * immersion_index) {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "numerical_aperture",
                value: na,
                reason: "must satisfy 0 < NA <= 0.95 * immersion_index",
            });
        }
        Ok(Self {
            na,
            reduction: 4.0,
            zernike_coefficients: Vec::new(),
            flare_fraction: 0.02,
            axial_chromatic_nm_per_pm: 0.0,
            apodization: Apodization::default(),
            paraxial_defocus: false,
            immersion_index,
        })
    }

    /// ArF water-immersion preset ("193i"): NA 1.35 in water
    /// ([`WATER_INDEX_193NM`] = 1.437), 4× reduction, 2 % flare, no chromatic
    /// aberration. Scalar pupil — polarization effects at this NA need
    /// `ImagingModel::Vector`.
    pub fn immersion_193i() -> Self {
        Self::immersion(1.35, WATER_INDEX_193NM).expect("NA 1.35 in water is valid")
    }

    /// Maximum spatial frequency that passes through the pupil, in 1/nm.
    pub fn cutoff_frequency(&self, wavelength_nm: f64) -> f64 {
        self.na / wavelength_nm
    }

    /// Rayleigh resolution limit: 0.61 * lambda / NA.
    pub fn rayleigh_resolution(&self, wavelength_nm: f64) -> f64 {
        0.61 * wavelength_nm / self.na
    }

    /// Depth of focus (paraxial Rayleigh criterion in the image medium):
    /// ± n·λ / (2·NA²) — λ/(2 NA²) for dry optics.
    pub fn rayleigh_dof(&self, wavelength_nm: f64) -> f64 {
        self.immersion_index * wavelength_nm / (2.0 * self.na * self.na)
    }

    /// Evaluate the pupil function at normalized frequency (fx, fy),
    /// where frequencies are in units of NA/lambda.
    ///
    /// Returns Complex64: magnitude is transmission, phase includes
    /// aberrations and defocus.
    pub fn pupil_function(
        &self,
        fx_norm: f64,
        fy_norm: f64,
        defocus_nm: f64,
        wavelength_nm: f64,
    ) -> Complex64 {
        let rho2 = fx_norm * fx_norm + fy_norm * fy_norm;

        // Outside pupil
        if rho2 > 1.0 {
            return Complex64::new(0.0, 0.0);
        }
        let rho = rho2.sqrt();

        // Aberration phase from Zernike coefficients
        let aberration_phase = if self.zernike_coefficients.is_empty() {
            0.0
        } else {
            let theta = fy_norm.atan2(fx_norm);
            zernike::pupil_phase(&self.zernike_coefficients, rho, theta)
        };

        // Defocus phase of the order at n·sin(theta) = NA·rho in the
        // image-space medium.
        let defocus_phase = if defocus_nm == 0.0 {
            0.0
        } else {
            defocus_phase_in_medium(
                defocus_nm,
                self.na * rho,
                self.immersion_index,
                wavelength_nm,
                self.paraxial_defocus,
            )
        };

        let total_phase = aberration_phase + defocus_phase;

        // Pupil transmission (apodization)
        let transmission = match &self.apodization {
            Apodization::Uniform => 1.0,
            Apodization::Quadratic { alpha } => (1.0 - alpha * rho2).max(0.0),
            Apodization::Gaussian { alpha } => (-alpha * rho2).exp(),
        };

        Complex::from_polar(transmission, total_phase)
    }

    /// Chromatic defocus for a wavelength offset from center (in pm).
    pub fn chromatic_defocus(&self, delta_wavelength_pm: f64) -> f64 {
        self.axial_chromatic_nm_per_pm * delta_wavelength_pm
    }
}

impl OpticalSystem for ProjectionOptics {
    fn pupil_function(
        &self,
        fx_norm: f64,
        fy_norm: f64,
        defocus_nm: f64,
        wavelength_nm: f64,
    ) -> Complex64 {
        // Delegate to inherent method via UFCS
        ProjectionOptics::pupil_function(self, fx_norm, fy_norm, defocus_nm, wavelength_nm)
    }

    fn na(&self) -> f64 {
        self.na
    }

    fn reduction(&self) -> f64 {
        self.reduction
    }

    fn flare_fraction(&self) -> f64 {
        self.flare_fraction
    }

    fn clone_box(&self) -> Box<dyn OpticalSystem> {
        Box::new(self.clone())
    }

    fn chromatic_defocus(&self, delta_wavelength_pm: f64) -> f64 {
        ProjectionOptics::chromatic_defocus(self, delta_wavelength_pm)
    }

    fn immersion_index(&self) -> f64 {
        self.immersion_index
    }
}

impl Default for ProjectionOptics {
    fn default() -> Self {
        Self::new(0.75).expect("default NA 0.75 is valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_cutoff_frequency() {
        let optics = ProjectionOptics::new(0.75).unwrap();
        let fc = optics.cutoff_frequency(157.0);
        assert_relative_eq!(fc, 0.75 / 157.0, epsilon = 1e-10);
    }

    #[test]
    fn test_rayleigh_resolution() {
        let optics = ProjectionOptics::new(0.75).unwrap();
        let res = optics.rayleigh_resolution(157.0);
        // 0.61 * 157 / 0.75 = 127.7 nm
        assert_relative_eq!(res, 0.61 * 157.0 / 0.75, epsilon = 1e-10);
    }

    #[test]
    fn test_pupil_inside() {
        let optics = ProjectionOptics::new(0.75).unwrap();
        let p = optics.pupil_function(0.0, 0.0, 0.0, 157.0);
        assert_relative_eq!(p.norm(), 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_pupil_outside() {
        let optics = ProjectionOptics::new(0.75).unwrap();
        let p = optics.pupil_function(1.5, 0.0, 0.0, 157.0);
        assert_relative_eq!(p.norm(), 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_pupil_at_edge() {
        let optics = ProjectionOptics::new(0.75).unwrap();
        let p = optics.pupil_function(1.0, 0.0, 0.0, 157.0);
        assert_relative_eq!(p.norm(), 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_defocus_adds_phase() {
        let optics = ProjectionOptics::new(0.75).unwrap();
        let p0 = optics.pupil_function(0.5, 0.0, 0.0, 157.0);
        let p1 = optics.pupil_function(0.5, 0.0, 100.0, 157.0);
        // Same magnitude, different phase
        assert_relative_eq!(p0.norm(), p1.norm(), epsilon = 1e-10);
        assert!((p0.arg() - p1.arg()).abs() > 1e-6);
    }

    #[test]
    fn test_aberrated_pupil() {
        let mut optics = ProjectionOptics::new(0.75).unwrap();
        // Add spherical aberration (Z9)
        optics.zernike_coefficients.push((9, 0.05));
        let p = optics.pupil_function(0.5, 0.0, 0.0, 157.0);
        // Should still have unit magnitude (no apodization)
        assert_relative_eq!(p.norm(), 1.0, epsilon = 1e-10);
        // But non-zero phase
        assert!(p.arg().abs() > 1e-6);
    }

    #[test]
    fn test_invalid_na_zero() {
        assert!(ProjectionOptics::new(0.0).is_err());
    }

    #[test]
    fn test_invalid_na_one() {
        assert!(ProjectionOptics::new(1.0).is_err());
        assert!(ProjectionOptics::new(-0.5).is_err());
        assert!(ProjectionOptics::new(f64::NAN).is_err());
    }

    #[test]
    fn test_defocus_phase_fixtures() {
        // Independent values (Python):
        //   exact    = 2π/λ · z · (1 − √(1 − s²))
        //   paraxial = π z s² / λ
        assert_relative_eq!(
            defocus_phase(1000.0, 0.1, 100.0, false),
            0.31494861522999035,
            epsilon = 1e-14
        );
        assert_relative_eq!(
            defocus_phase(1000.0, 0.1, 100.0, true),
            0.31415926535897937,
            epsilon = 1e-14
        );
        assert_relative_eq!(
            defocus_phase(100.0, 0.9, 13.5, false),
            26.254876499452216,
            epsilon = 1e-12
        );
        assert_relative_eq!(
            defocus_phase(100.0, 0.9, 13.5, true),
            18.84955592153876,
            epsilon = 1e-12
        );
        assert_relative_eq!(
            defocus_phase(-250.0, 0.45, 157.63, false),
            -1.0659795274875334,
            epsilon = 1e-13
        );
    }

    #[test]
    fn test_defocus_phase_paraxial_limit_and_scaling() {
        // Small angle: exact → paraxial with relative error ≈ s²/4.
        for &s in &[1e-3, 1e-2, 0.05] {
            let ex = defocus_phase(500.0, s, 193.0, false);
            let px = defocus_phase(500.0, s, 193.0, true);
            assert!(((ex / px) - 1.0 - s * s / 4.0).abs() < s.powi(4));
        }
        // Linear in z, inverse in λ.
        let base = defocus_phase(100.0, 0.6, 50.0, false);
        assert_relative_eq!(
            defocus_phase(200.0, 0.6, 50.0, false),
            2.0 * base,
            epsilon = 1e-12
        );
        assert_relative_eq!(
            defocus_phase(100.0, 0.6, 100.0, false),
            0.5 * base,
            epsilon = 1e-12
        );
        // Evanescent arguments are clamped (no NaN).
        assert!(defocus_phase(100.0, 1.2, 50.0, false).is_finite());
    }

    #[test]
    fn test_pupil_uses_exact_defocus_by_default() {
        let optics = ProjectionOptics::new(0.9).unwrap();
        let p = optics.pupil_function(1.0, 0.0, 100.0, 157.0);
        let expected = defocus_phase(100.0, 0.9, 157.0, false);
        assert_relative_eq!(p.arg(), expected, epsilon = 1e-12);
        let mut par = optics.clone();
        par.paraxial_defocus = true;
        let pp = par.pupil_function(1.0, 0.0, 100.0, 157.0);
        assert_relative_eq!(
            pp.arg(),
            std::f64::consts::PI * 100.0 * 0.81 / 157.0,
            epsilon = 1e-12
        );
    }

    #[test]
    fn test_paraxial_flag_defaults_false_in_serde() {
        // Old TOML/JSON without the field still parses (serde default).
        let old = r#"
            na = 0.75
            reduction = 4.0
            zernike_coefficients = []
            flare_fraction = 0.02
            axial_chromatic_nm_per_pm = 15.0
            apodization = "Uniform"
        "#;
        let o: ProjectionOptics = toml::from_str(old).unwrap();
        assert!(!o.paraxial_defocus);
    }

    #[test]
    fn test_clone_box_preserves_pupil() {
        let mut optics = ProjectionOptics::new(0.6).unwrap();
        optics.zernike_coefficients.push((9, 0.03));
        let boxed: Box<dyn OpticalSystem> = optics.clone_box();
        let cloned = boxed.clone();
        for &(fx, fy, z) in &[(0.3, 0.1, 0.0), (-0.5, 0.4, 80.0)] {
            let a = optics.pupil_function(fx, fy, z, 157.0);
            let b = cloned.pupil_function(fx, fy, z, 157.0);
            assert_eq!(a, b);
        }
        assert_eq!(cloned.na(), 0.6);
    }

    #[test]
    fn test_defocus_phase_in_medium_fixtures() {
        // Independent values (Python), Φ = 2π n/λ · z · (1 − √(1 − (NAρ/n)²)),
        // paraxial π z (NAρ)²/(n λ):
        assert_relative_eq!(
            defocus_phase_in_medium(100.0, 1.35, 1.437, 193.368, false),
            3.0692902284513828,
            epsilon = 1e-12
        );
        assert_relative_eq!(
            defocus_phase_in_medium(100.0, 1.35, 1.437, 193.368, true),
            2.0605162135233135,
            epsilon = 1e-12
        );
        assert_relative_eq!(
            defocus_phase_in_medium(-250.0, 0.8, 1.437, 193.0, false),
            -1.9800119323377656,
            epsilon = 1e-12
        );
        assert_relative_eq!(
            defocus_phase_in_medium(-250.0, 0.8, 1.437, 193.0, true),
            -1.812407197545141,
            epsilon = 1e-12
        );
    }

    #[test]
    fn test_defocus_phase_in_medium_is_continuous_at_n_1() {
        for &s in &[0.0, 0.1, 0.5, 0.9, 0.99] {
            for &paraxial in &[false, true] {
                let dry = defocus_phase(250.0, s, 193.0, paraxial);
                assert_eq!(defocus_phase_in_medium(250.0, s, 1.0, 193.0, paraxial), dry);
                // A tiny index step changes the phase by O(Δn), not O(1).
                let wet = defocus_phase_in_medium(250.0, s, 1.0 + 1e-9, 193.0, paraxial);
                assert!(
                    (wet - dry).abs() <= 1e-7 * dry.abs().max(1.0),
                    "{s}: {dry} vs {wet}"
                );
            }
        }
        // The medium lowers the phase at fixed NAρ (longer DOF): 1.84 → 1.03 rad
        // at NAρ = 0.9, z = 100 nm, λ = 193 nm (water vs air).
        let dry = defocus_phase_in_medium(100.0, 0.9, 1.0, 193.0, false);
        let wet = defocus_phase_in_medium(100.0, 0.9, 1.437, 193.0, false);
        assert_relative_eq!(dry, 1.8364809986663466, epsilon = 1e-12);
        assert_relative_eq!(wet, 1.0311781772329183, epsilon = 1e-12);
    }

    #[test]
    fn test_immersion_constructors_and_validation() {
        // Dry constructor still rejects NA >= 1 (back-compat).
        assert!(ProjectionOptics::new(1.2).is_err());
        let o = ProjectionOptics::immersion_193i();
        assert_relative_eq!(o.na, 1.35);
        assert_relative_eq!(o.immersion_index, WATER_INDEX_193NM);
        assert_relative_eq!(OpticalSystem::immersion_index(&o), 1.437);
        assert_eq!(o.axial_chromatic_nm_per_pm, 0.0);
        // Cutoff stays NA/λ; DOF grows with n.
        assert_relative_eq!(OpticalSystem::cutoff_frequency(&o, 193.368), 1.35 / 193.368);
        assert_relative_eq!(o.rayleigh_dof(193.368), 76.23314567901234, epsilon = 1e-9);
        // NA above the 0.95·n margin, n < 1, non-finite inputs are rejected.
        assert!(ProjectionOptics::immersion(1.40, 1.437).is_err());
        assert!(ProjectionOptics::immersion(0.9, 0.99).is_err());
        assert!(ProjectionOptics::immersion(f64::NAN, 1.437).is_err());
        assert!(ProjectionOptics::immersion(1.2, f64::INFINITY).is_err());
        assert!(ProjectionOptics::immersion(0.0, 1.437).is_err());
        // Dry optics report n = 1 and serde defaults old configs to dry.
        assert_eq!(
            OpticalSystem::immersion_index(&ProjectionOptics::default()),
            1.0
        );
        let old = r#"
            na = 0.75
            reduction = 4.0
            zernike_coefficients = []
            flare_fraction = 0.02
            axial_chromatic_nm_per_pm = 15.0
            apodization = "Uniform"
        "#;
        let parsed: ProjectionOptics = toml::from_str(old).unwrap();
        assert_eq!(parsed.immersion_index, 1.0);
    }

    #[test]
    fn test_immersion_pupil_uses_medium_defocus() {
        let o = ProjectionOptics::immersion_193i();
        let p = o.pupil_function(0.6, 0.0, 40.0, 193.368);
        let expected = defocus_phase_in_medium(40.0, 1.35 * 0.6, 1.437, 193.368, false);
        assert_relative_eq!(p.arg(), expected, epsilon = 1e-12);
        let boxed = o.clone_box();
        assert_eq!(boxed.immersion_index(), 1.437);
        assert_eq!(boxed.pupil_function(0.6, 0.0, 40.0, 193.368), p);
    }
}
