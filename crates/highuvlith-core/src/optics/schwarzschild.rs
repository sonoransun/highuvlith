//! Schwarzschild reflective objective for EUV and soft X-ray lithography.
//!
//! A Schwarzschild objective uses two concentric spherical mirrors
//! (convex primary + concave secondary) to form an image. It provides
//! NA up to ~0.3 for EUV/soft X-ray wavelengths where refractive optics
//! are impossible.
//!
//! # Key equations
//!
//! ```text
//!   P(ρ) = R · exp[i Φ_z(ρ)]   for ε ≤ ρ ≤ 1,   0 otherwise
//!   Φ_z(ρ) = (2π/λ) z (1 − √(1 − NA²ρ²))      (paraxial option: π z NA² ρ²/λ)
//! ```
//! with `ε` the central obscuration ratio and `R` the per-mirror
//! reflectivity (two-mirror system intensity transmission `R²`).
//!
//! # Model status
//!
//! Annular scalar pupil with a constant reflectivity by default. An optional
//! [`MultilayerPupil`] replaces the constant with the angle-dependent
//! multilayer response (pupil apodization and phase) from user-supplied
//! incidence-angle maps; the multilayer's spectral response enters per
//! wavelength in `compute_multiwavelength`. No mirror figure error (no
//! Zernike terms), achromatic focus (default `chromatic_defocus = 0`).
//! Defocus phase is exact for a plane wave in vacuum; the paraxial form is
//! available via `paraxial_defocus`.

use num::Complex;
use serde::{Deserialize, Serialize};

use super::multilayer_pupil::MultilayerPupil;
use crate::types::Complex64;

/// Schwarzschild two-mirror reflective objective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchwarzschildObjective {
    /// Numerical aperture (typical: 0.08-0.33 for EUV).
    pub numerical_aperture: f64,
    /// Central obscuration ratio (fraction of pupil blocked by secondary mirror).
    /// Typically 0.2-0.4.
    pub obscuration_ratio: f64,
    /// Reduction ratio (e.g., 4.0 for 4x demagnification).
    pub reduction_ratio: f64,
    /// Per-mirror reflectivity (0-1). Squared for two-mirror system.
    pub mirror_reflectivity: f64,
    /// Flare fraction from mirror scatter.
    pub flare: f64,
    /// Use the paraxial (quadratic) defocus phase instead of the exact
    /// plane-wave form. Default `false`.
    #[serde(default)]
    pub paraxial_defocus: bool,
    /// Optional angle-dependent multilayer response of the two mirrors
    /// (apodization + phase across the pupil). `None` (default) keeps the
    /// scalar constant `mirror_reflectivity`; when set, the coating replaces
    /// it in the pupil amplitude.
    #[serde(default)]
    pub multilayer: Option<MultilayerPupil>,
}

impl SchwarzschildObjective {
    /// Create a Schwarzschild objective for EUV at 13.5nm.
    pub fn euv_standard() -> Self {
        Self {
            numerical_aperture: 0.33,
            obscuration_ratio: 0.25,
            reduction_ratio: 4.0,
            mirror_reflectivity: 0.67, // Mo/Si multilayer at 13.5nm
            flare: 0.03,
            paraxial_defocus: false,
            multilayer: None,
        }
    }

    /// Create for BEUV at 6.7nm.
    pub fn beuv() -> Self {
        Self {
            numerical_aperture: 0.25,
            obscuration_ratio: 0.3,
            reduction_ratio: 4.0,
            mirror_reflectivity: 0.50, // La/B4C multilayer at 6.7nm
            flare: 0.05,
            paraxial_defocus: false,
            multilayer: None,
        }
    }

    /// Create for soft X-ray at ~1nm.
    pub fn soft_xray(na: f64) -> crate::error::Result<Self> {
        if na.is_nan() || na <= 0.0 || na >= 1.0 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "numerical_aperture",
                value: if na.is_nan() { f64::NAN } else { na },
                reason: "must be in range (0, 1) and not NaN",
            });
        }
        Ok(Self {
            numerical_aperture: na,
            obscuration_ratio: 0.3,
            reduction_ratio: 1.0,     // typically 1:1 for X-ray microscopy
            mirror_reflectivity: 0.3, // grazing-incidence or multilayer
            flare: 0.05,
            paraxial_defocus: false,
            multilayer: None,
        })
    }

    /// Two-mirror system transmission: R².
    pub fn system_transmission(&self) -> f64 {
        self.mirror_reflectivity * self.mirror_reflectivity
    }
}

impl super::OpticalSystem for SchwarzschildObjective {
    fn pupil_function(
        &self,
        fx_norm: f64,
        fy_norm: f64,
        defocus_nm: f64,
        wavelength_nm: f64,
    ) -> Complex64 {
        let rho = (fx_norm * fx_norm + fy_norm * fy_norm).sqrt();

        // Outside aperture
        if rho > 1.0 {
            return Complex64::new(0.0, 0.0);
        }

        // Central obscuration (secondary mirror shadow)
        if rho < self.obscuration_ratio {
            return Complex64::new(0.0, 0.0);
        }

        // Transmission: two-mirror reflectivity — scalar constant, or the
        // angle-dependent multilayer response of the mirror train.
        let amplitude = match &self.multilayer {
            Some(ml) => ml.amplitude(fx_norm, fy_norm, wavelength_nm),
            None => Complex64::new(self.system_transmission().sqrt(), 0.0),
        };

        // Defocus phase of the order at sin(theta) = NA * rho
        let defocus_phase = super::defocus_phase(
            defocus_nm,
            self.numerical_aperture * rho,
            wavelength_nm,
            self.paraxial_defocus,
        );

        amplitude * Complex::from_polar(1.0, defocus_phase)
    }

    fn na(&self) -> f64 {
        self.numerical_aperture
    }

    fn reduction(&self) -> f64 {
        self.reduction_ratio
    }

    fn flare_fraction(&self) -> f64 {
        self.flare
    }

    fn clone_box(&self) -> Box<dyn super::OpticalSystem> {
        Box::new(self.clone())
    }

    fn prepare(&self, wavelength_nm: f64) -> crate::error::Result<()> {
        match &self.multilayer {
            Some(ml) => ml.prepare(wavelength_nm),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optics::OpticalSystem;
    use approx::assert_relative_eq;

    #[test]
    fn test_euv_na() {
        let obj = SchwarzschildObjective::euv_standard();
        assert_relative_eq!(obj.na(), 0.33, epsilon = 1e-10);
    }

    #[test]
    fn test_annular_pupil() {
        let obj = SchwarzschildObjective::euv_standard();
        // Center is blocked
        let p_center = obj.pupil_function(0.0, 0.0, 0.0, 13.5);
        assert_relative_eq!(p_center.norm(), 0.0, epsilon = 1e-10);
        // Mid-ring passes
        let p_mid = obj.pupil_function(0.5, 0.0, 0.0, 13.5);
        assert!(p_mid.norm() > 0.0);
        // Outside blocked
        let p_out = obj.pupil_function(1.5, 0.0, 0.0, 13.5);
        assert_relative_eq!(p_out.norm(), 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_system_transmission() {
        let obj = SchwarzschildObjective::euv_standard();
        // 0.67² ≈ 0.449
        assert_relative_eq!(obj.system_transmission(), 0.67 * 0.67, epsilon = 1e-10);
    }

    #[test]
    fn test_beuv_lower_reflectivity() {
        let euv = SchwarzschildObjective::euv_standard();
        let beuv = SchwarzschildObjective::beuv();
        assert!(
            beuv.system_transmission() < euv.system_transmission(),
            "BEUV should have lower transmission than EUV"
        );
    }

    #[test]
    fn test_euv_resolution() {
        let obj = SchwarzschildObjective::euv_standard();
        let res = obj.rayleigh_resolution(13.5);
        // 0.61 × 13.5 / 0.33 ≈ 24.9nm
        assert_relative_eq!(res, 24.9, epsilon = 0.5);
    }

    #[test]
    fn test_soft_xray_invalid_na() {
        assert!(SchwarzschildObjective::soft_xray(0.0).is_err());
        assert!(SchwarzschildObjective::soft_xray(-0.1).is_err());
        assert!(SchwarzschildObjective::soft_xray(1.0).is_err());
        assert!(SchwarzschildObjective::soft_xray(f64::NAN).is_err());
        // Valid NA should succeed
        assert!(SchwarzschildObjective::soft_xray(0.15).is_ok());
    }

    #[test]
    fn test_defocus_matches_shared_formula() {
        let obj = SchwarzschildObjective::euv_standard();
        let p = obj.pupil_function(0.8, 0.0, 40.0, 13.5);
        let expected = crate::optics::defocus_phase(40.0, 0.33 * 0.8, 13.5, false);
        assert_relative_eq!(p.arg(), expected, epsilon = 1e-12);
        assert_relative_eq!(p.norm(), 0.67, epsilon = 1e-12);
        let mut par = obj.clone();
        par.paraxial_defocus = true;
        let pp = par.pupil_function(0.8, 0.0, 40.0, 13.5);
        let expected_par = std::f64::consts::PI * 40.0 * (0.33f64 * 0.8).powi(2) / 13.5;
        assert_relative_eq!(pp.arg(), expected_par, epsilon = 1e-12);
    }

    #[test]
    fn test_multilayer_pupil_replaces_constant_reflectivity() {
        use crate::materials::multilayer::MultilayerMirror;
        use crate::optics::multilayer_pupil::MirrorAngleMap;
        let coating = MultilayerMirror::mo_si(40, 6.9, 0.4).unwrap();
        let ml = MultilayerPupil::new(coating, vec![MirrorAngleMap::constant(0.0); 2]).unwrap();
        let mut obj = SchwarzschildObjective::euv_standard();
        let plain = obj.pupil_function(0.5, 0.0, 20.0, 13.5);
        obj.multilayer = Some(ml.clone());
        assert!(obj.prepare(13.5).is_ok());
        let coated = obj.pupil_function(0.5, 0.0, 20.0, 13.5);
        // Same defocus phase, amplitude from the coating instead of R.
        let m = ml.amplitude(0.5, 0.0, 13.5);
        assert!((coated - m * Complex::from_polar(1.0, plain.arg())).norm() < 1e-12);
        // Obscuration still applies.
        assert_eq!(obj.pupil_function(0.1, 0.0, 0.0, 13.5).norm(), 0.0);
        assert!(obj.prepare(3000.0).is_err());
    }
}
