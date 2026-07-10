//! Shared accelerator- and strong-field-physics formulas for light-source
//! models.
//!
//! Pure functions used by the synchrotron, XFEL, SSMB, ICS, and HHG source
//! implementations (and retro-used by `LpaFelSource`). Everything here is
//! textbook physics with fixture tests pinning the numbers.
//!
//! # Key equations
//!
//! - Lorentz factor:            `gamma = 1 + E_kin / (m_e c^2)`, m_e c^2 = 0.51099895 MeV
//! - Undulator resonance:       `lambda_n = lambda_u (1 + K^2/2) / (2 n gamma^2)` (on-axis)
//! - Undulator strength:        `K = 0.09337 B0[T] lambda_u[mm]`
//! - Bending-magnet critical energy: `E_c[keV] = 0.665 E^2[GeV^2] B[T]`
//! - BM universal flux:         `S(y) = y Int_y^inf K_{5/3}(x) dx` (Kostroun 1980 algorithm)
//! - Ponderomotive energy:      `U_p[eV] = 9.33e-14 I[W/cm^2] lambda^2[um^2]`
//! - HHG cutoff:                `E_max = I_p + 3.17 U_p`
//! - Inverse Compton (head-on): `lambda_X = lambda_L (1 + a0^2/2 + gamma^2 theta^2) / (4 gamma^2)`
//! - SASE bandwidth:            `d-lambda/lambda ~ 2 rho` (Pierce parameter rho)

/// Electron rest energy in MeV (CODATA).
pub const ELECTRON_REST_MEV: f64 = 0.510_998_95;

/// Lorentz factor of an electron with the given kinetic energy in MeV.
pub fn gamma_from_mev(kinetic_energy_mev: f64) -> f64 {
    1.0 + kinetic_energy_mev / ELECTRON_REST_MEV
}

/// On-axis undulator resonance wavelength (nm) for harmonic `n`.
///
/// `lambda_n = lambda_u (1 + K^2/2) / (2 n gamma^2)` with the undulator
/// period given in millimetres.
pub fn undulator_resonance_nm(period_mm: f64, k: f64, gamma: f64, harmonic: usize) -> f64 {
    let period_nm = period_mm * 1e6;
    period_nm * (1.0 + k * k / 2.0) / (2.0 * harmonic as f64 * gamma * gamma)
}

/// Dimensionless undulator strength parameter `K = e B0 lambda_u / (2 pi m_e c)`
/// in engineering units: `K = 0.09337 B0[T] lambda_u[mm]`.
pub fn undulator_k_from_field(b0_tesla: f64, period_mm: f64) -> f64 {
    0.09337 * b0_tesla * period_mm
}

/// Bending-magnet critical photon energy in keV:
/// `E_c = 0.665 E^2[GeV^2] B[T]` (equivalently 3 hbar c gamma^3 / 2 rho).
/// Half the total radiated power is above E_c, half below.
pub fn critical_energy_kev(electron_energy_gev: f64, field_tesla: f64) -> f64 {
    0.665 * electron_energy_gev * electron_energy_gev * field_tesla
}

/// Universal bending-magnet flux function `S(y) = y Int_y^inf K_{5/3}(x) dx`
/// with `y = E / E_c`. Photon flux per unit bandwidth is proportional to
/// `S(y)`; S peaks near y ~ 0.29 and integrates to 8 pi / (9 sqrt(3))
/// ~ 1.612 over y (from Int_0^inf x^2 K_{5/3}(x) dx / 2).
///
/// Computed with the Kostroun exponential-sum algorithm
/// (V. O. Kostroun, Nucl. Instrum. Methods 172, 371 (1980)):
/// `Int_y^inf K_nu(t) dt = h * [e^-y / 2 + sum_r e^{-y cosh(rh)} cosh(nu r h) / cosh(rh)]`
/// which converges double-exponentially; h = 0.5 gives ~1e-10 accuracy.
pub fn bm_universal_flux(y: f64) -> f64 {
    if y <= 0.0 {
        return 0.0;
    }
    const H: f64 = 0.5;
    const NU: f64 = 5.0 / 3.0;
    let mut integral = 0.5 * (-y).exp();
    for r in 1..200 {
        let rh = r as f64 * H;
        let cosh_rh = rh.cosh();
        let term = (-y * cosh_rh).exp() * (NU * rh).cosh() / cosh_rh;
        integral += term;
        if term < 1e-14 * integral {
            break;
        }
    }
    y * H * integral
}

/// Ponderomotive (quiver) energy of an electron in a laser field, in eV:
/// `U_p = 9.33e-14 I[W/cm^2] lambda^2[um^2]`.
pub fn ponderomotive_ev(intensity_w_cm2: f64, driver_wavelength_um: f64) -> f64 {
    9.33e-14 * intensity_w_cm2 * driver_wavelength_um * driver_wavelength_um
}

/// HHG cutoff photon energy (eV) from the three-step model:
/// `E_max = I_p + 3.17 U_p`.
pub fn hhg_cutoff_ev(ionization_potential_ev: f64, ponderomotive_ev: f64) -> f64 {
    ionization_potential_ev + 3.17 * ponderomotive_ev
}

/// Inverse-Compton-scattered photon wavelength (nm) for a head-on
/// collision, observed at angle `theta` from the electron direction:
/// `lambda_X = lambda_L (1 + a0^2/2 + gamma^2 theta^2) / (4 gamma^2)`.
/// `a0` is the normalized laser vector potential (a0 << 1 = linear regime).
pub fn ics_wavelength_nm(laser_wavelength_nm: f64, gamma: f64, a0: f64, theta_rad: f64) -> f64 {
    laser_wavelength_nm * (1.0 + a0 * a0 / 2.0 + gamma * gamma * theta_rad * theta_rad)
        / (4.0 * gamma * gamma)
}

/// SASE FEL bandwidth (FWHM, pm) from the Pierce parameter:
/// `d-lambda/lambda ~ 2 rho`, so `d-lambda[pm] = 2 rho lambda[nm] * 1e3`.
pub fn sase_bandwidth_pm(wavelength_nm: f64, pierce_parameter: f64) -> f64 {
    2.0 * pierce_parameter * wavelength_nm * 1e3
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_gamma_from_mev() {
        // 500 MeV: gamma = 1 + 500/0.511 ~ 979.5
        let gamma = gamma_from_mev(500.0);
        assert_relative_eq!(gamma, 1.0 + 500.0 / 0.510_998_95, epsilon = 1e-12);
        assert!((979.0..981.0).contains(&gamma));
    }

    #[test]
    fn test_undulator_resonance_fixture() {
        // lambda_u = 20 mm, K = 1, E = 538 MeV, n = 1:
        // gamma = 1053.8, lambda = 2e7 * 1.5 / (2 * gamma^2) ~ 13.5 nm
        let gamma = gamma_from_mev(538.0);
        let lambda = undulator_resonance_nm(20.0, 1.0, gamma, 1);
        assert!(
            (13.3..13.7).contains(&lambda),
            "expected ~13.5 nm, got {lambda}"
        );

        // Third harmonic is exactly a third of the fundamental.
        let lambda3 = undulator_resonance_nm(20.0, 1.0, gamma, 3);
        assert_relative_eq!(lambda3, lambda / 3.0, epsilon = 1e-12);
    }

    #[test]
    fn test_undulator_resonance_scales_inverse_gamma_squared() {
        // Doubling electron energy (gamma) quarters the wavelength —
        // the reason the BELLA 100 -> 500 MeV upgrade moves 420 -> ~25 nm.
        let l1 = undulator_resonance_nm(20.0, 1.0, 1000.0, 1);
        let l2 = undulator_resonance_nm(20.0, 1.0, 2000.0, 1);
        assert_relative_eq!(l1 / l2, 4.0, epsilon = 1e-12);
    }

    #[test]
    fn test_undulator_k_fixture() {
        // 1 T, 20 mm period: K = 0.09337 * 1 * 20 = 1.867
        assert_relative_eq!(undulator_k_from_field(1.0, 20.0), 1.8674, epsilon = 1e-3);
    }

    #[test]
    fn test_critical_energy_fixture() {
        // LIGA-class bending magnet: 2.5 GeV, 1.5 T
        // E_c = 0.665 * 6.25 * 1.5 = 6.234 keV
        assert_relative_eq!(critical_energy_kev(2.5, 1.5), 6.234, epsilon = 1e-3);
    }

    #[test]
    fn test_bm_universal_flux_peak() {
        // S(y) peaks near y ~ 0.29 with a maximum of ~0.92.
        let mut best_y = 0.0;
        let mut best_s = 0.0;
        let mut y = 0.05;
        while y < 1.0 {
            let s = bm_universal_flux(y);
            if s > best_s {
                best_s = s;
                best_y = y;
            }
            y += 0.005;
        }
        assert!(
            (0.25..0.33).contains(&best_y),
            "peak should be near y~0.29, got {best_y}"
        );
        assert!(
            (0.88..0.95).contains(&best_s),
            "peak value should be ~0.92, got {best_s}"
        );
    }

    #[test]
    fn test_bm_universal_flux_asymptotes() {
        // y -> 0: S(y) -> 2.1495 y^(1/3)
        let y: f64 = 1e-4;
        let expected = 2.1495 * y.powf(1.0 / 3.0);
        let actual = bm_universal_flux(y);
        assert!(
            ((actual - expected) / expected).abs() < 0.02,
            "small-y asymptote: expected {expected}, got {actual}"
        );

        // Large y: exponential decay dominates; S(10) << S(1)
        assert!(bm_universal_flux(10.0) < 1e-3);
        assert!(bm_universal_flux(10.0) > 0.0);

        // Non-positive y is zero flux.
        assert_eq!(bm_universal_flux(0.0), 0.0);
        assert_eq!(bm_universal_flux(-1.0), 0.0);
    }

    #[test]
    fn test_ponderomotive_and_cutoff_fixture() {
        // Ar driver case: 800 nm, 2e14 W/cm^2
        // U_p = 9.33e-14 * 2e14 * 0.64 = 11.94 eV
        let up = ponderomotive_ev(2e14, 0.8);
        assert_relative_eq!(up, 11.94, epsilon = 0.02);

        // Cutoff for Ar (I_p = 15.76 eV): 15.76 + 3.17 * 11.94 = 53.6 eV
        let cutoff = hhg_cutoff_ev(15.760, up);
        assert!((53.0..54.5).contains(&cutoff), "got {cutoff}");
    }

    #[test]
    fn test_ics_wavelength_fixture() {
        // 1030 nm laser, on-axis, small a0: lambda_X = lambda_L / (4 gamma^2).
        // For 13.5 nm output: gamma = sqrt(1030 / (4 * 13.5)) = 4.367
        let gamma = (1030.0_f64 / (4.0 * 13.5)).sqrt();
        let lambda = ics_wavelength_nm(1030.0, gamma, 0.0, 0.0);
        assert_relative_eq!(lambda, 13.5, epsilon = 1e-9);

        // Nonlinear redshift (a0) and off-axis redshift (theta) both
        // lengthen the wavelength.
        assert!(ics_wavelength_nm(1030.0, gamma, 0.5, 0.0) > lambda);
        assert!(ics_wavelength_nm(1030.0, gamma, 0.0, 1e-3) > lambda);
    }

    #[test]
    fn test_sase_bandwidth_fixture() {
        // rho = 1e-3 at 13.5 nm: d-lambda = 2e-3 * 13.5 nm = 27 pm
        assert_relative_eq!(sase_bandwidth_pm(13.5, 1e-3), 27.0, epsilon = 1e-9);
    }
}
