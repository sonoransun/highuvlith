//! Diamond substrate modeling for lithography.
//!
//! Diamond (C) is emerging as a next-gen substrate for quantum computing,
//! power electronics, and X-ray window applications due to its extreme
//! thermal conductivity and optical properties.
//!
//! # Model status
//!
//! Sellmeier index (transparent region only), simple thermal-dose estimate,
//! NIST X-ray window transmission. Range checks: the optical index and the
//! film-stack presets are defined only from the bandgap cutoff
//! (`hc / 5.47 eV` = 226.7 nm, [`DIAMOND_MIN_NM`]) to 2000 nm and return
//! `WavelengthOutOfRange` elsewhere - diamond absorbs strongly above its gap
//! and no absorbing-regime `n + ik` data are carried here. (Before
//! 2026-10-01 the presets substituted a fixed `n = 2.7, k = 0` below 225 nm
//! and hard-coded the 157 nm VUV resist and Si indices at every wavelength;
//! the resist and Si indices are now explicit arguments.) X-ray transmission
//! accepts 0.03 keV - 20 MeV.

use crate::error::{LithographyError, Result};
use crate::materials::dispersion::SellmeierCoefficients;
use crate::thinfilm::{FilmLayer, FilmStack};
use crate::types::Complex64;

/// Shortest wavelength (nm) at which the diamond index is evaluated: the
/// bandgap cutoff `hc / 5.47 eV`.
pub const DIAMOND_MIN_NM: f64 = 1239.84193 / 5.47;
/// Longest wavelength (nm) at which the diamond index is evaluated (the
/// database's common Sellmeier upper limit).
pub const DIAMOND_MAX_NM: f64 = 2000.0;

/// Diamond refractive index (real, `k = 0`) from [`diamond_sellmeier`] in
/// [[`DIAMOND_MIN_NM`], [`DIAMOND_MAX_NM`]]; `WavelengthOutOfRange` outside.
pub fn diamond_index(wavelength_nm: f64) -> Result<f64> {
    if !(DIAMOND_MIN_NM..=DIAMOND_MAX_NM).contains(&wavelength_nm) {
        return Err(LithographyError::WavelengthOutOfRange {
            material: "diamond".to_string(),
            wavelength_nm,
            range_nm: (DIAMOND_MIN_NM, DIAMOND_MAX_NM),
            hint: "diamond absorbs above its 5.47 eV bandgap and no n + ik data are carried \
                   for that regime; for X-rays use 'henke:C@3.515'"
                .to_string(),
        });
    }
    diamond_sellmeier().refractive_index(wavelength_nm)
}

/// Diamond Sellmeier coefficients (type IIa, UV-visible-IR).
/// Reference: Peter, 1923; valid ~225nm to far-IR.
pub fn diamond_sellmeier() -> SellmeierCoefficients {
    SellmeierCoefficients {
        b: vec![0.3306, 4.3356],
        c: vec![0.00030625, 0.011236],
    }
}

/// Diamond thermal and mechanical properties.
pub struct DiamondProperties {
    /// Thermal conductivity (W/m·K). Natural diamond: ~2200.
    pub thermal_conductivity: f64,
    /// Thermal expansion coefficient (1/K).
    pub thermal_expansion: f64,
    /// Bandgap energy (eV). Type IIa: 5.47 eV → UV cutoff at ~227nm.
    pub bandgap_ev: f64,
    /// Density (g/cm³).
    pub density: f64,
}

impl Default for DiamondProperties {
    fn default() -> Self {
        Self {
            thermal_conductivity: 2200.0,
            thermal_expansion: 1.0e-6,
            bandgap_ev: 5.47,
            density: 3.515,
        }
    }
}

impl DiamondProperties {
    /// UV cutoff wavelength (nm) from bandgap.
    pub fn uv_cutoff_nm(&self) -> f64 {
        super::energy::ev_to_nm(self.bandgap_ev)
    }

    /// Estimate maximum dose (mJ/cm²) before thermal distortion exceeds tolerance.
    /// Simple model: ΔT = dose / (ρ × c_p × thickness), distortion = α × ΔT × area^0.5
    pub fn max_dose_mj_cm2(&self, thickness_um: f64, distortion_tolerance_nm: f64) -> f64 {
        let specific_heat = 0.509; // J/(g·K) for diamond
        let rho_cgs = self.density; // g/cm³
        let thickness_cm = thickness_um * 1e-4;
        // ΔT for distortion: distortion = α × ΔT × characteristic_length
        // Simplified: max_delta_t = tolerance / (α × thickness_um * 1000)
        let max_delta_t = distortion_tolerance_nm / (self.thermal_expansion * thickness_um * 1e3);
        // dose = ρ × c_p × thickness × ΔT (in J/cm²)
        let dose_j_cm2 = rho_cgs * specific_heat * thickness_cm * max_delta_t;
        dose_j_cm2 * 1e3 // convert to mJ/cm²
    }
}

/// Resist (index `resist_n` at this wavelength) on a semi-infinite diamond
/// substrate, vacuum above. Errors outside diamond's transparent range
/// ([`diamond_index`]).
pub fn resist_on_diamond(
    resist_thickness_nm: f64,
    wavelength_nm: f64,
    resist_n: Complex64,
) -> Result<FilmStack> {
    let n_diamond = diamond_index(wavelength_nm)?;
    Ok(FilmStack::new_vuv(
        vec![FilmLayer {
            name: "resist".to_string(),
            thickness_nm: resist_thickness_nm,
            n: resist_n,
        }],
        Complex64::new(n_diamond, 0.0), // diamond substrate (transparent)
    ))
}

/// A diamond membrane on silicon (index `silicon_n` at this wavelength),
/// vacuum above. Errors outside diamond's transparent range
/// ([`diamond_index`]).
pub fn diamond_on_silicon(
    diamond_thickness_nm: f64,
    wavelength_nm: f64,
    silicon_n: Complex64,
) -> Result<FilmStack> {
    let n_diamond = diamond_index(wavelength_nm)?;
    Ok(FilmStack::new_vuv(
        vec![FilmLayer {
            name: "diamond".to_string(),
            thickness_nm: diamond_thickness_nm,
            n: Complex64::new(n_diamond, 0.0),
        }],
        silicon_n,
    ))
}

/// X-ray transmission `exp(-mu t)` through a diamond window (3.515 g/cm^3)
/// of `thickness_um` at `energy_kev`, with the NIST carbon attenuation
/// coefficient of [`crate::materials::attenuation`] (photoabsorption +
/// scattering, 30 eV - 20 MeV; other energies are an error). At hard X-ray
/// energies (>5 keV) diamond is nearly transparent.
pub fn xray_transmission(thickness_um: f64, energy_kev: f64) -> Result<f64> {
    crate::materials::attenuation::check_energy_kev(energy_kev)?;
    let mu_per_um = crate::materials::attenuation::Compound::diamond().mu_per_um(energy_kev);
    Ok((-mu_per_um * thickness_um).exp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_diamond_refractive_index() {
        let s = diamond_sellmeier();
        let n = s.refractive_index(589.0).unwrap(); // sodium D line
        assert_relative_eq!(n, 2.417, epsilon = 0.01);
    }

    #[test]
    fn test_diamond_uv_cutoff() {
        let props = DiamondProperties::default();
        let cutoff = props.uv_cutoff_nm();
        assert!(
            cutoff > 225.0 && cutoff < 230.0,
            "Diamond UV cutoff should be ~227nm, got {}",
            cutoff
        );
    }

    #[test]
    fn test_diamond_thermal_capacity() {
        let props = DiamondProperties::default();
        // Diamond should tolerate very high doses due to thermal conductivity
        let max_dose = props.max_dose_mj_cm2(500.0, 1.0);
        assert!(
            max_dose > 100.0,
            "Diamond should tolerate high dose, got {} mJ/cm²",
            max_dose
        );
    }

    #[test]
    fn test_xray_transmission_high_energy() {
        // At 10 keV, 100μm diamond should be highly transparent
        let t = xray_transmission(100.0, 10.0).unwrap();
        assert!(
            t > 0.9,
            "Diamond should be >90% transparent at 10 keV, got {:.1}%",
            t * 100.0
        );
    }

    #[test]
    fn test_xray_transmission_nist_fixture() {
        // NIST carbon mu/rho = 2.373 cm^2/g at 10 keV: 100 um of diamond
        // transmits exp(-2.373 * 3.515 * 0.01) = 0.9200.
        assert_relative_eq!(
            xray_transmission(100.0, 10.0).unwrap(),
            0.91998,
            epsilon = 1e-4
        );
    }

    #[test]
    fn test_xray_transmission_low_energy() {
        // At 1 keV, diamond absorbs more
        let t_low = xray_transmission(100.0, 1.0).unwrap();
        let t_high = xray_transmission(100.0, 10.0).unwrap();
        assert!(t_low < t_high, "Lower energy should have more absorption");
    }

    #[test]
    fn test_resist_on_diamond_stack() {
        let resist = Complex64::new(1.5, 0.001);
        let stack = resist_on_diamond(150.0, 589.0, resist).unwrap();
        assert_eq!(stack.layers.len(), 1);
        assert_eq!(stack.layers[0].n, resist);
        // Diamond substrate: the Sellmeier index (independent evaluation of
        // the Peter coefficients at 589 nm: 2.41073), k = 0.
        assert!((stack.substrate.re - 2.41073).abs() < 1e-5);
        assert_eq!(stack.substrate.im, 0.0);
        let si = Complex64::new(3.88, 0.02);
        let m = diamond_on_silicon(500.0, 633.0, si).unwrap();
        assert_eq!(m.substrate, si);
    }

    #[test]
    fn test_diamond_index_range() {
        // hc / 5.47 eV = 226.66 nm.
        assert!((DIAMOND_MIN_NM - 226.6621).abs() < 1e-3);
        assert!(diamond_index(248.0).is_ok());
        // VUV (absorbing) and EUV: typed errors, no stand-in value.
        for wl in [157.63, 13.5, 2500.0] {
            assert!(matches!(
                diamond_index(wl),
                Err(LithographyError::WavelengthOutOfRange { .. })
            ));
            assert!(resist_on_diamond(100.0, wl, Complex64::new(1.65, 0.015)).is_err());
            assert!(diamond_on_silicon(100.0, wl, Complex64::new(0.88, 2.1)).is_err());
        }
        assert!(xray_transmission(10.0, 0.001).is_err());
        assert!(xray_transmission(10.0, 5.0e4).is_err());
    }
}
