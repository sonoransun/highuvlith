//! EUV reflective projection optics (scanner projection box), wafer-side
//! pupil model.
//!
//! An EUV scanner images the reflective mask through a multi-mirror
//! projection box. For wafer-side imaging this module keeps what enters the
//! aerial image: a circular pupil of numerical aperture `NA`, an optional
//! **central obscuration** (a mirror blocking the pupil center — present in
//! High-NA (0.55) designs, absent in the 0.33-NA class), wavefront
//! aberrations (Fringe Zernikes), the exact defocus phase in vacuum, and a
//! uniform flare fraction.
//!
//! # Key equations
//!
//! ```text
//!   P(ρ,θ) = √T · exp[i (W_aberr(ρ,θ) + Φ_z(ρ))]   for ε ≤ ρ ≤ 1, 0 otherwise
//!   Φ_z(ρ) = (2π/λ)·z·(1 − √(1 − NA²ρ²))          (paraxial option: π z NA² ρ²/λ)
//!   f_cutoff = NA/λ;  zero order of a source point at σ is blocked for |σ| < ε
//! ```
//! `ε` = `central_obscuration` (radius, as a fraction of NA), `T` = system
//! intensity transmission.
//!
//! # Model status
//!
//! Simplified (🔶). The pupil is **isotropic on the wafer side**: the
//! anamorphic mask-side magnification of High-NA systems (4× in x, 8× in y)
//! only changes mask-side dimensions and angles and is NOT modeled — mask
//! coordinates are wafer-scale; `reduction` feeds only the vector radiometric
//! factor. Mask-3D (thick absorber, oblique-incidence shadowing) effects and
//! the non-telecentric chief ray are not modeled. The multilayer mirrors'
//! angle-dependent reflectance (pupil apodization and phase) is off by
//! default (uniform pupil); an optional [`MultilayerPupil`] adds it from
//! user-supplied incidence-angle maps. The High-NA obscuration radius of the
//! preset (0.2 NA) is an assumed representative value, not a published
//! design figure. `T` scales only absolute radiometry
//! (`clear_field_intensity`); clear-field-normalized images do not depend on
//! it.

use num::Complex;
use serde::{Deserialize, Serialize};

use super::multilayer_pupil::MultilayerPupil;
use crate::math::zernike;
use crate::types::Complex64;

fn default_one() -> f64 {
    1.0
}

/// EUV projection optics with an optional central obscuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EuvProjectionOptics {
    /// Wafer-side numerical aperture (0.33 NXE-class, 0.55 High-NA-class).
    pub numerical_aperture: f64,
    /// Central obscuration radius as a fraction of the NA (0 = unobscured).
    pub central_obscuration: f64,
    /// Nominal reduction ratio. Enters only the vector radiometric factor;
    /// anamorphic (x ≠ y) reduction is not modeled.
    pub reduction: f64,
    /// Fringe-Zernike wavefront aberrations: (fringe index, coefficient in waves).
    #[serde(default)]
    pub zernike_coefficients: Vec<(usize, f64)>,
    /// Uniform flare fraction (0 in the presets: flare is not modeled by
    /// default; EUV mirror roughness makes it non-negligible in practice —
    /// set a measured value).
    pub flare: f64,
    /// System intensity transmission (multiplies absolute radiometry only;
    /// the presets use 1: multilayer reflectance losses are not modeled).
    #[serde(default = "default_one")]
    pub transmission: f64,
    /// Use the paraxial defocus phase instead of the exact form.
    #[serde(default)]
    pub paraxial_defocus: bool,
    /// Optional angle-dependent multilayer response of the mirror train
    /// (pupil apodization + phase); multiplies `√transmission`. `None`
    /// (default) keeps a uniform pupil.
    #[serde(default)]
    pub multilayer: Option<MultilayerPupil>,
}

impl EuvProjectionOptics {
    /// Projection optics with wafer-side `na` in (0, 1) and central
    /// obscuration radius `central_obscuration` in [0, 1) (fraction of NA);
    /// 4× reduction, no flare, no aberrations, unit transmission.
    pub fn new(na: f64, central_obscuration: f64) -> crate::error::Result<Self> {
        if !(na.is_finite() && na > 0.0 && na < 1.0) {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "numerical_aperture",
                value: na,
                reason: "must be in (0, 1)",
            });
        }
        if !(central_obscuration.is_finite() && (0.0..1.0).contains(&central_obscuration)) {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "central_obscuration",
                value: central_obscuration,
                reason: "must be in [0, 1) (radius as a fraction of NA)",
            });
        }
        Ok(Self {
            numerical_aperture: na,
            central_obscuration,
            reduction: 4.0,
            zernike_coefficients: Vec::new(),
            flare: 0.0,
            transmission: 1.0,
            paraxial_defocus: false,
            multilayer: None,
        })
    }

    /// 0.33-NA EUV projection optics (NXE-class): unobscured circular pupil,
    /// 4× reduction.
    pub fn nxe_033() -> Self {
        Self::new(0.33, 0.0).expect("valid preset")
    }

    /// 0.55-NA EUV projection optics (High-NA / EXE-class) with a centrally
    /// obscured pupil. The obscuration radius 0.2·NA is an ASSUMED
    /// representative value (High-NA designs are centrally obscured; the
    /// exact figure is design-specific). Isotropic wafer-side model: the
    /// anamorphic 4×/8× mask-side magnification is not modeled (`reduction`
    /// = 4 enters only the vector radiometric factor).
    pub fn high_na_055() -> Self {
        Self::new(0.55, 0.2).expect("valid preset")
    }
}

impl super::OpticalSystem for EuvProjectionOptics {
    fn pupil_function(
        &self,
        fx_norm: f64,
        fy_norm: f64,
        defocus_nm: f64,
        wavelength_nm: f64,
    ) -> Complex64 {
        let rho2 = fx_norm * fx_norm + fy_norm * fy_norm;
        if rho2 > 1.0 {
            return Complex64::new(0.0, 0.0);
        }
        let rho = rho2.sqrt();
        if rho < self.central_obscuration {
            return Complex64::new(0.0, 0.0);
        }
        let aberration = if self.zernike_coefficients.is_empty() {
            0.0
        } else {
            zernike::pupil_phase(&self.zernike_coefficients, rho, fy_norm.atan2(fx_norm))
        };
        let defocus = if defocus_nm == 0.0 {
            0.0
        } else {
            super::defocus_phase(
                defocus_nm,
                self.numerical_aperture * rho,
                wavelength_nm,
                self.paraxial_defocus,
            )
        };
        let amplitude =
            Complex::from_polar(self.transmission.max(0.0).sqrt(), aberration + defocus);
        match &self.multilayer {
            Some(ml) => amplitude * ml.amplitude(fx_norm, fy_norm, wavelength_nm),
            None => amplitude,
        }
    }

    fn na(&self) -> f64 {
        self.numerical_aperture
    }

    fn reduction(&self) -> f64 {
        self.reduction
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
    fn test_presets() {
        let nxe = EuvProjectionOptics::nxe_033();
        assert_relative_eq!(nxe.na(), 0.33);
        assert_eq!(nxe.central_obscuration, 0.0);
        let hna = EuvProjectionOptics::high_na_055();
        assert_relative_eq!(hna.na(), 0.55);
        assert_relative_eq!(hna.central_obscuration, 0.2);
        assert_eq!(hna.flare_fraction(), 0.0);
        assert_eq!(hna.reduction(), 4.0);
        assert_eq!(hna.immersion_index(), 1.0);
        // Rayleigh resolution 0.61 λ/NA at 13.5 nm.
        assert_relative_eq!(hna.rayleigh_resolution(13.5), 0.61 * 13.5 / 0.55);
    }

    #[test]
    fn test_obscured_pupil_and_defocus() {
        let o = EuvProjectionOptics::high_na_055();
        assert_eq!(o.pupil_function(0.0, 0.0, 0.0, 13.5).norm(), 0.0);
        assert_eq!(o.pupil_function(0.19, 0.0, 0.0, 13.5).norm(), 0.0);
        assert_relative_eq!(o.pupil_function(0.21, 0.0, 0.0, 13.5).norm(), 1.0);
        assert_eq!(o.pupil_function(1.01, 0.0, 0.0, 13.5).norm(), 0.0);
        let p = o.pupil_function(0.0, 0.8, 30.0, 13.5);
        let expected = crate::optics::defocus_phase(30.0, 0.55 * 0.8, 13.5, false);
        assert_relative_eq!(p.arg(), expected, epsilon = 1e-12);
        // Unobscured NXE passes the pupil centre.
        assert_relative_eq!(
            EuvProjectionOptics::nxe_033()
                .pupil_function(0.0, 0.0, 0.0, 13.5)
                .norm(),
            1.0
        );
    }

    #[test]
    fn test_transmission_and_aberrations() {
        let mut o = EuvProjectionOptics::nxe_033();
        o.transmission = 0.25;
        assert_relative_eq!(o.pupil_function(0.5, 0.0, 0.0, 13.5).norm(), 0.5);
        o.zernike_coefficients.push((9, 0.02));
        let p = o.pupil_function(0.5, 0.0, 0.0, 13.5);
        let w = crate::math::zernike::pupil_phase(&[(9, 0.02)], 0.5, 0.0);
        assert_relative_eq!(p.arg(), w, epsilon = 1e-12);
        let c = o.clone_box();
        assert_eq!(c.pupil_function(0.5, 0.0, 0.0, 13.5), p);
    }

    #[test]
    fn test_validation_and_serde_defaults() {
        assert!(EuvProjectionOptics::new(0.0, 0.0).is_err());
        assert!(EuvProjectionOptics::new(1.0, 0.0).is_err());
        assert!(EuvProjectionOptics::new(0.55, 1.0).is_err());
        assert!(EuvProjectionOptics::new(0.55, -0.1).is_err());
        assert!(EuvProjectionOptics::new(f64::NAN, 0.1).is_err());
        let minimal = r#"
            numerical_aperture = 0.55
            central_obscuration = 0.2
            reduction = 4.0
            flare = 0.0
        "#;
        let o: EuvProjectionOptics = toml::from_str(minimal).unwrap();
        assert_eq!(o.transmission, 1.0);
        assert!(o.zernike_coefficients.is_empty());
        assert!(!o.paraxial_defocus);
    }
}
