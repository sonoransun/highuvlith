//! Shared accelerator- and strong-field-physics formulas for light-source
//! models.
//!
//! Pure functions used by every source family (synchrotron, XFEL, LPA-FEL,
//! SSMB, ICS, HHG, LPP, entangled) to derive physical quantities from
//! machine parameters. Everything here is textbook physics with fixture
//! tests pinning the numbers; the fixture values were computed
//! independently of this code (scipy/mpmath, or by hand in the test
//! comments).
//!
//! # Model status
//!
//! Implemented and fixture-tested. The FEL and inverse-Compton formulas
//! are the standard *idealized* forms (1D FEL theory, head-on Gaussian
//! Thomson luminosity, Kim's central-cone undulator flux); each function
//! documents the approximation it makes. The Ming Xie 3D gain-length fit
//! is an empirical fit (≈ 10–20 % accuracy inside its fitted range).
//!
//! # Key equations
//!
//! - Lorentz factor:            `gamma = 1 + E_kin / (m_e c^2)`, m_e c^2 = 0.51099895 MeV
//! - Undulator resonance:       `lambda_n = lambda_u (1 + K^2/2) / (2 n gamma^2)` (on-axis)
//! - Undulator strength:        `K = 0.09337 B0[T] lambda_u[mm]` (exactly `e B0 lambda_u / (2 pi m_e c)`)
//! - Bending-magnet critical energy: `E_c[keV] = 0.665 E^2[GeV^2] B[T]`
//! - BM universal flux:         `S(y) = G1(y) = y Int_y^inf K_{5/3}(x) dx` (Kostroun 1980 algorithm)
//! - BM flux per horizontal angle: `dF/dtheta = (sqrt3 / 2 pi) alpha gamma (dw/w) (I/e) G1(y)`
//!   (= `2.457e13 E[GeV] I[A] G1(y)` photons/s/mrad/0.1%BW)
//! - BM power per horizontal angle: `dP/dtheta = e gamma^4 I / (6 pi eps0 rho)` (per rad)
//! - Bessel `J_n(x)`: power series (|x| <= 12) / Miller backward recurrence (|x| > 12)
//! - Planar-undulator harmonic factor: `xi = n K^2 / (4 + 2 K^2)`,
//!   `JJ_n = J_{(n-1)/2}(xi) - J_{(n+1)/2}(xi)`,
//!   `F_n(K) = n^2 K^2 JJ_n^2 / (1 + K^2/2)^2`, `Q_n(K) = (1 + K^2/2) F_n(K) / n`
//! - Kim central-cone flux:     `F_n = pi alpha N Q_n(K) (dw/w) (I/e)` (= `1.431e14 N Q_n I[A]` ph/s/0.1%BW)
//! - On-axis angular flux density: `d2F/dOmega = alpha N^2 gamma^2 F_n(K) (dw/w) (I/e)`
//!   (= `1.744e14 N^2 E^2[GeV] I[A] F_n` ph/s/mrad^2/0.1%BW)
//! - Central-cone power in the natural line: `P_n = pi alpha Q_n(K) (I/e) E_ph,1`
//!   (for n = 1 identical to Attwood's `pi e gamma^2 I K^2 JJ^2 / (eps0 lambda_u (1 + K^2/2)^2)`)
//! - Total undulator power:     `P_T = e^3 gamma^2 B0^2 L I / (12 pi eps0 m_e^2 c^2)` (= 632.7 E^2 B0^2 L I W)
//! - Coherent fraction per plane: `zeta = eps_r / (eps + eps_r)`, `eps_r = lambda / (4 pi)`
//!   (matched-beta convolution of the electron emittance with the photon emittance)
//! - 1D Pierce parameter:       `rho = [ (1/16) (I/I_A) K^2 JJ_1^2 / (gamma^3 sigma_x^2 k_u^2) ]^(1/3)`
//! - 1D gain length / saturation: `L_g = lambda_u / (4 pi sqrt3 rho)`, `P_sat ~ rho P_beam`,
//!   `L_sat ~ lambda_u / rho`
//! - Ming Xie 3D fit:           `L_g3D = L_g (1 + Lambda(eta_d, eta_eps, eta_gamma))`,
//!   `P_sat,3D ~ 1.6 rho (L_g / L_g3D)^2 P_beam`
//! - Thomson yield (head-on, round Gaussian beams, linear regime):
//!   `N_x = sigma_T N_e N_L / (2 pi (sigma_e^2 + sigma_L^2))`
//! - Thomson collection fraction: `f(theta_c) = (3/8) [ (1 - u) + (1 - u^3)/3 ]`,
//!   `u = (cos theta_c - beta) / (1 - beta cos theta_c)`
//! - Laser critical density:    `n_c = eps0 m_e omega^2 / e^2` (= `1.115e21 / lambda^2[um]` cm^-3)
//! - Ponderomotive energy:      `U_p[eV] = 9.33e-14 I[W/cm^2] lambda^2[um^2]`
//! - HHG cutoff:                `E_max = I_p + 3.17 U_p`; Keldysh `gamma_K = sqrt(I_p / (2 U_p))`
//! - HHG critical ionization:   `eta_cr = dn / (dn + N_atm r_e lambda^2 / (2 pi))`
//! - Inverse Compton (head-on): `lambda_X = lambda_L (1 + a0^2/2 + gamma^2 theta^2) / (4 gamma^2)`
//! - SASE bandwidth:            `d-lambda/lambda ~ 2 rho` (Pierce parameter rho)
//! - Coherent (bunched-beam) undulator power: `P = 2 pi^2 alpha hbar N Q_1(K) |b|^2 (I_avg/e)(I_pk/e)`
//!   (derived in [`coherent_undulator_power_w`])
//! - Gaussian microbunch form factor: `b = exp(-(k sigma_z)^2 / 2)`

use serde::{Deserialize, Serialize};

/// Planck constant x speed of light in eV·nm (CODATA).
pub const HC_EV_NM: f64 = 1_239.841_93;
/// Elementary charge in coulomb (exact, SI 2019).
pub const ELECTRON_CHARGE_C: f64 = 1.602_176_634e-19;
/// Speed of light in m/s (exact).
pub const SPEED_OF_LIGHT_M_S: f64 = 299_792_458.0;
/// Planck constant in J·s (exact).
pub const PLANCK_J_S: f64 = 6.626_070_15e-34;
/// Reduced Planck constant in J·s.
pub const HBAR_J_S: f64 = PLANCK_J_S / (2.0 * std::f64::consts::PI);
/// Vacuum permittivity in F/m (CODATA).
pub const EPSILON_0_F_M: f64 = 8.854_187_812_8e-12;
/// Fine-structure constant (CODATA).
pub const FINE_STRUCTURE: f64 = 7.297_352_569_3e-3;
/// Electron mass in kg (CODATA).
pub const ELECTRON_MASS_KG: f64 = 9.109_383_701_5e-31;
/// Classical electron radius in m (CODATA).
pub const CLASSICAL_ELECTRON_RADIUS_M: f64 = 2.817_940_326_2e-15;
/// Thomson cross-section in m^2 (CODATA).
pub const THOMSON_CROSS_SECTION_M2: f64 = 6.652_458_732_1e-29;
/// Alfvén current `4 pi eps0 m_e c^3 / e` in A.
pub const ALFVEN_CURRENT_A: f64 = 17_045.0;
/// Loschmidt constant (ideal-gas number density at 0 °C, 1 atm) in m^-3.
pub const LOSCHMIDT_M3: f64 = 2.686_780_111e25;

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
/// which converges double-exponentially in the step `h`. With h = 0.25 the
/// relative error is below 1e-14 for 0.01 <= y <= 3 and ~3e-12 at y = 10
/// (checked against 30-digit mpmath quadrature). An earlier h = 0.5 was only ~1e-8 accurate near the peak
/// and degraded to ~7e-4 at y = 10.
pub fn bm_universal_flux(y: f64) -> f64 {
    if y <= 0.0 {
        return 0.0;
    }
    const H: f64 = 0.25;
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

// ---------------------------------------------------------------------------
// Photon bookkeeping
// ---------------------------------------------------------------------------

/// Relative bandwidth of the conventional "per 0.1 % bandwidth" flux unit.
const BW_0P1: f64 = 1e-3;

/// Photon energy in joules at `wavelength_nm`: `E = hc / lambda`.
pub fn photon_energy_j(wavelength_nm: f64) -> f64 {
    HC_EV_NM / wavelength_nm * ELECTRON_CHARGE_C
}

/// Power (W) carried by a photon rate (photons/s) at `wavelength_nm`.
pub fn photon_rate_to_watts(photons_per_s: f64, wavelength_nm: f64) -> f64 {
    photons_per_s * photon_energy_j(wavelength_nm)
}

/// Photon rate (photons/s) carried by a power (W) at `wavelength_nm`.
pub fn watts_to_photon_rate(power_w: f64, wavelength_nm: f64) -> f64 {
    power_w / photon_energy_j(wavelength_nm)
}

// ---------------------------------------------------------------------------
// Bessel functions of the first kind (integer order)
// ---------------------------------------------------------------------------

/// Bessel function of the first kind `J_n(x)` for integer order `n >= 0`.
///
/// Power series `sum_m (-1)^m (x/2)^(2m+n) / (m! (m+n)!)` for `|x| <= 12`
/// (cancellation-limited absolute error ~1e-12 at the upper end) and
/// Miller's backward recurrence `J_{k-1} = (2k/x) J_k - J_{k+1}`,
/// normalized with `J_0 + 2 sum_{k>=1} J_{2k} = 1`, for `|x| > 12`.
/// Negative arguments use `J_n(-x) = (-1)^n J_n(x)`.
pub fn bessel_j(n: u32, x: f64) -> f64 {
    if x == 0.0 {
        return if n == 0 { 1.0 } else { 0.0 };
    }
    if x < 0.0 {
        let v = bessel_j(n, -x);
        return if n.is_multiple_of(2) { v } else { -v };
    }
    if x <= 12.0 {
        bessel_j_series(n, x)
    } else {
        bessel_j_miller(n, x)
    }
}

fn bessel_j_series(n: u32, x: f64) -> f64 {
    let half = 0.5 * x;
    // Leading term (x/2)^n / n!
    let mut term = 1.0;
    for k in 1..=n {
        term *= half / k as f64;
    }
    let q = half * half;
    let mut sum = term;
    let mut m = 0u32;
    while m < 400 {
        m += 1;
        term *= -q / (m as f64 * (m + n) as f64);
        sum += term;
        // Terms shrink monotonically once m (m + n) > (x/2)^2.
        let past_peak = (m as f64) * ((m + n) as f64) > q;
        if term == 0.0 || (past_peak && term.abs() <= 1e-17 * sum.abs()) {
            break;
        }
    }
    sum
}

fn bessel_j_miller(n: u32, x: f64) -> f64 {
    // Start well above max(n, x): there the minimal solution (J) dominates
    // the backward recurrence, so any tiny seed converges onto it.
    let top = n.max(x.ceil() as u32);
    let mut start = top + 30 + (40.0 * top as f64).sqrt() as u32;
    if !start.is_multiple_of(2) {
        start += 1;
    }
    let mut j_up = 0.0; // J_{k+1}
    let mut j_k = 1e-30; // J_k, arbitrary scale
    let mut ans = if start == n { j_k } else { 0.0 };
    // sum_{k >= 1} J_{2k}; `start` is even and >= 2.
    let mut even_sum = j_k;
    for k in (1..=start).rev() {
        let j_down = (2.0 * k as f64 / x) * j_k - j_up;
        j_up = j_k;
        j_k = j_down; // now J_{k-1}
        let idx = k - 1;
        if idx == n {
            ans = j_k;
        }
        if idx >= 2 && idx.is_multiple_of(2) {
            even_sum += j_k;
        }
        if j_k.abs() > 1e100 {
            j_k *= 1e-100;
            j_up *= 1e-100;
            ans *= 1e-100;
            even_sum *= 1e-100;
        }
    }
    // j_k now holds the unnormalized J_0.
    ans / (j_k + 2.0 * even_sum)
}

// ---------------------------------------------------------------------------
// Undulator radiation (planar undulator, filament beam)
// ---------------------------------------------------------------------------

/// Planar-undulator Bessel factor for odd harmonic `n`:
/// `JJ_n = J_{(n-1)/2}(xi) - J_{(n+1)/2}(xi)` with `xi = n K^2 / (4 + 2 K^2)`.
/// Returns 0 for even (or zero) harmonics, whose on-axis emission vanishes.
pub fn undulator_jj(harmonic: usize, k: f64) -> f64 {
    if harmonic == 0 || harmonic.is_multiple_of(2) {
        return 0.0;
    }
    let n = harmonic as f64;
    let xi = n * k * k / (4.0 + 2.0 * k * k);
    let h = ((harmonic - 1) / 2) as u32;
    bessel_j(h, xi) - bessel_j(h + 1, xi)
}

/// On-axis harmonic function `F_n(K) = n^2 K^2 JJ_n^2 / (1 + K^2/2)^2`
/// (K.-J. Kim's notation, X-ray Data Booklet §2.1). Zero for even `n`.
pub fn undulator_fn(harmonic: usize, k: f64) -> f64 {
    let n = harmonic as f64;
    let jj = undulator_jj(harmonic, k);
    let denom = 1.0 + k * k / 2.0;
    n * n * k * k * jj * jj / (denom * denom)
}

/// Central-cone harmonic function `Q_n(K) = (1 + K^2/2) F_n(K) / n`.
pub fn undulator_qn(harmonic: usize, k: f64) -> f64 {
    if harmonic == 0 {
        return 0.0;
    }
    (1.0 + k * k / 2.0) * undulator_fn(harmonic, k) / harmonic as f64
}

/// Kim's central-cone photon flux of undulator harmonic `n`, in
/// photons/s/0.1%BW: `F_n = pi alpha N Q_n(K) (dw/w) (I/e)`
/// (= `1.431e14 N Q_n I[A]`). Filament-beam (zero-emittance, zero energy
/// spread) value; it is the angle-integrated flux near the line peak.
pub fn undulator_central_cone_flux(
    num_periods: usize,
    k: f64,
    harmonic: usize,
    current_a: f64,
) -> f64 {
    std::f64::consts::PI
        * FINE_STRUCTURE
        * num_periods as f64
        * undulator_qn(harmonic, k)
        * BW_0P1
        * current_a
        / ELECTRON_CHARGE_C
}

/// On-axis angular photon flux density of harmonic `n`, in
/// photons/s/mrad^2/0.1%BW: `alpha N^2 gamma^2 F_n(K) (dw/w) (I/e)`
/// (= `1.744e14 N^2 E^2[GeV] I[A] F_n`). Filament beam.
pub fn undulator_on_axis_flux_density(
    num_periods: usize,
    k: f64,
    harmonic: usize,
    gamma: f64,
    current_a: f64,
) -> f64 {
    let n = num_periods as f64;
    FINE_STRUCTURE * n * n * gamma * gamma * undulator_fn(harmonic, k) * BW_0P1 * current_a
        / ELECTRON_CHARGE_C
        * 1e-6
}

/// Power (W) in the central cone within the natural line of harmonic `n`.
///
/// Kim's central-cone flux per unit relative bandwidth times the natural
/// relative line width `1/(nN)` gives `pi alpha Q_n (I/e) / n` photons/s
/// (N cancels), each carrying `n E_ph,1`:
/// `P_n = pi alpha Q_n(K) (I/e) E_ph,1`. For `n = 1` this is identical to
/// Attwood's central-cone power
/// `pi e gamma^2 I K^2 JJ^2 / (eps0 lambda_u (1 + K^2/2)^2)`.
pub fn undulator_central_cone_power_w(
    k: f64,
    harmonic: usize,
    current_a: f64,
    fundamental_wavelength_nm: f64,
) -> f64 {
    std::f64::consts::PI * FINE_STRUCTURE * undulator_qn(harmonic, k) * current_a
        / ELECTRON_CHARGE_C
        * photon_energy_j(fundamental_wavelength_nm)
}

/// Peak undulator field (T) for a given K and period:
/// `B0 = 2 pi m_e c K / (e lambda_u)` (the exact form of `K = 0.09337 B0 lambda_u[mm]`).
pub fn undulator_field_from_k(k: f64, period_mm: f64) -> f64 {
    2.0 * std::f64::consts::PI * ELECTRON_MASS_KG * SPEED_OF_LIGHT_M_S * k
        / (ELECTRON_CHARGE_C * period_mm * 1e-3)
}

/// Total power (W) radiated by a planar undulator into all harmonics and
/// angles: `P_T = e^3 gamma^2 B0^2 L I / (12 pi eps0 m_e^2 c^2)`
/// (the Larmor power of an ultrarelativistic electron with `<B^2> = B0^2/2`,
/// times `L / c` and `I / e`; = `632.7 E^2[GeV] B0^2[T] L[m] I[A]`).
pub fn undulator_total_power_w(gamma: f64, b0_t: f64, length_m: f64, current_a: f64) -> f64 {
    ELECTRON_CHARGE_C.powi(3) * gamma * gamma * b0_t * b0_t * length_m * current_a
        / (12.0
            * std::f64::consts::PI
            * EPSILON_0_F_M
            * ELECTRON_MASS_KG.powi(2)
            * SPEED_OF_LIGHT_M_S.powi(2))
}

/// Deflection parameter K that places harmonic `n` of an undulator with
/// period `period_mm` at `wavelength_nm` for a beam of Lorentz factor
/// `gamma` (the inverse resonance condition — how a gap-tunable undulator
/// is set). `None` when the wavelength is shorter than the K = 0 limit
/// `lambda_u / (2 n gamma^2)`.
pub fn undulator_k_for_wavelength(
    period_mm: f64,
    gamma: f64,
    wavelength_nm: f64,
    harmonic: usize,
) -> Option<f64> {
    let period_nm = period_mm * 1e6;
    let one_plus_k2_half = 2.0 * harmonic as f64 * gamma * gamma * wavelength_nm / period_nm;
    if one_plus_k2_half.is_nan() || one_plus_k2_half < 1.0 {
        return None;
    }
    Some((2.0 * (one_plus_k2_half - 1.0)).sqrt())
}

// ---------------------------------------------------------------------------
// Bending magnets
// ---------------------------------------------------------------------------

/// Bending radius (m) of an electron of Lorentz factor `gamma` in a field
/// `field_t`: `rho = p / (e B) = beta gamma m_e c / (e B)`.
pub fn bending_radius_m(gamma: f64, field_t: f64) -> f64 {
    let beta_gamma = (gamma * gamma - 1.0).max(0.0).sqrt();
    beta_gamma * ELECTRON_MASS_KG * SPEED_OF_LIGHT_M_S / (ELECTRON_CHARGE_C * field_t)
}

/// Bending-magnet photon flux per mrad of horizontal angle (integrated over
/// the vertical angle), in photons/s/mrad/0.1%BW:
/// `dF/dtheta = (sqrt3 / 2 pi) alpha gamma (dw/w) (I/e) G1(y)` with
/// `G1(y) = S(y)` = [`bm_universal_flux`] and `y = E / E_c`
/// (= `2.457e13 E[GeV] I[A] G1(y)`).
pub fn bm_flux_per_mrad(gamma: f64, current_a: f64, y: f64) -> f64 {
    3.0_f64.sqrt() / (2.0 * std::f64::consts::PI) * FINE_STRUCTURE * gamma * BW_0P1 * current_a
        / ELECTRON_CHARGE_C
        * bm_universal_flux(y)
        * 1e-3
}

/// Bending-magnet power (all photon energies) per mrad of horizontal angle,
/// in W/mrad: `dP/dtheta = e gamma^4 I / (6 pi eps0 rho)` per rad — the
/// energy loss per turn `U0 = e^2 gamma^4 / (3 eps0 rho)` times `I / e`,
/// spread over `2 pi`. Ultrarelativistic (`beta = 1`).
pub fn bm_power_per_mrad_w(gamma: f64, bending_radius_m: f64, current_a: f64) -> f64 {
    ELECTRON_CHARGE_C * gamma.powi(4) * current_a
        / (6.0 * std::f64::consts::PI * EPSILON_0_F_M * bending_radius_m)
        * 1e-3
}

// ---------------------------------------------------------------------------
// Transverse coherence
// ---------------------------------------------------------------------------

/// Transverse coherent fraction in one plane for an electron beam of
/// geometric emittance `emittance_m_rad` radiating at `wavelength_nm`:
/// `zeta = eps_r / (eps + eps_r)` with the diffraction-limited photon
/// emittance `eps_r = lambda / (4 pi)` (Kim's convention).
///
/// This is the smooth form of the textbook ratio `(lambda / 4 pi) / eps`:
/// it follows from convolving the electron phase space with the
/// single-electron photon phase space under optimal (matched) beta
/// functions, so it is an upper bound for real lattices. It tends to
/// `eps_r / eps` for `eps >> eps_r` and to 1 for a zero-emittance beam.
/// Conventions using `lambda / (2 pi)` give up to 2x larger fractions in the
/// incoherent limit.
pub fn coherent_fraction_per_plane(emittance_m_rad: f64, wavelength_nm: f64) -> f64 {
    let eps_r = wavelength_nm * 1e-9 / (4.0 * std::f64::consts::PI);
    if emittance_m_rad <= 0.0 {
        return 1.0;
    }
    eps_r / (emittance_m_rad + eps_r)
}

// ---------------------------------------------------------------------------
// Free-electron lasers (1D theory + Ming Xie 3D fit)
// ---------------------------------------------------------------------------

/// Planar undulator description used by the FEL / radiator estimates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UndulatorParams {
    /// Undulator period in mm.
    pub period_mm: f64,
    /// Peak (planar) deflection parameter K.
    pub k: f64,
    /// Number of periods; the magnetic length is `num_periods * period`.
    pub num_periods: usize,
}

impl UndulatorParams {
    /// Magnetic length in m.
    pub fn length_m(&self) -> f64 {
        self.num_periods as f64 * self.period_mm * 1e-3
    }

    /// On-axis fundamental resonance wavelength (nm) for Lorentz factor `gamma`.
    pub fn resonance_nm(&self, gamma: f64) -> f64 {
        undulator_resonance_nm(self.period_mm, self.k, gamma, 1)
    }
}

/// Electron-beam parameters inside an FEL undulator.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ElectronBeamParams {
    /// Peak (slice) current in A.
    pub peak_current_a: f64,
    /// Normalized rms emittance in µm (= mm·mrad).
    pub norm_emittance_um: f64,
    /// Relative rms (slice) energy spread `sigma_gamma / gamma`.
    pub energy_spread_rel: f64,
    /// Average beta function in the undulator, in m (sets the beam size).
    pub beta_m: f64,
}

impl ElectronBeamParams {
    /// Geometric emittance `eps_n / (beta gamma)` in m·rad.
    pub fn geometric_emittance_m(&self, gamma: f64) -> f64 {
        let beta_gamma = (gamma * gamma - 1.0).max(1e-30).sqrt();
        self.norm_emittance_um * 1e-6 / beta_gamma
    }

    /// rms transverse beam size `sqrt(eps beta)` in m.
    pub fn rms_size_m(&self, gamma: f64) -> f64 {
        (self.geometric_emittance_m(gamma) * self.beta_m).sqrt()
    }
}

/// 1D FEL (Pierce) parameter for a planar undulator at the fundamental:
/// `rho = [ (1/16) (I/I_A) K^2 JJ_1^2 / (gamma^3 sigma_x^2 k_u^2) ]^(1/3)`
/// with `k_u = 2 pi / lambda_u` and `I_A` the Alfvén current (standard 1D
/// FEL theory, e.g. the Huang–Kim review form). Round beam of rms size
/// `rms_beam_size_m`.
pub fn pierce_parameter_1d(
    peak_current_a: f64,
    rms_beam_size_m: f64,
    gamma: f64,
    k: f64,
    period_mm: f64,
) -> f64 {
    let ku = 2.0 * std::f64::consts::PI / (period_mm * 1e-3);
    let jj = undulator_jj(1, k);
    ((1.0 / 16.0) * (peak_current_a / ALFVEN_CURRENT_A) * k * k * jj * jj
        / (gamma.powi(3) * rms_beam_size_m.powi(2) * ku * ku))
        .cbrt()
}

/// 1D power gain length `L_g = lambda_u / (4 pi sqrt3 rho)` in m.
pub fn fel_gain_length_1d_m(period_mm: f64, rho: f64) -> f64 {
    period_mm * 1e-3 / (4.0 * std::f64::consts::PI * 3.0_f64.sqrt() * rho)
}

/// Electron-beam power `gamma m_e c^2 I / e` in W (peak power for a peak
/// current, average power for an average current).
pub fn electron_beam_power_w(gamma: f64, current_a: f64) -> f64 {
    gamma * ELECTRON_REST_MEV * 1e6 * current_a
}

/// Scaled parameters and gain-length degradation of the Ming Xie fit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MingXieFit {
    /// Diffraction parameter `L_g / (2 k sigma_x^2)`.
    pub eta_d: f64,
    /// Emittance parameter `(L_g / beta) (4 pi eps / lambda)`.
    pub eta_eps: f64,
    /// Energy-spread parameter `4 pi (L_g / lambda_u) sigma_gamma/gamma`.
    pub eta_gamma: f64,
    /// Degradation `Lambda`: `L_g3D = L_g1D (1 + Lambda)`.
    pub lambda: f64,
}

/// Ming Xie's 19 fit coefficients `a1..a19` (M. Xie, Proc. PAC 1995).
const XIE_A: [f64; 19] = [
    0.45, 0.57, 0.55, 1.6, 3.0, 2.0, 0.35, 2.9, 2.4, 51.0, 0.95, 3.0, 5.4, 0.7, 1.9, 1140.0, 2.2,
    2.9, 3.2,
];

/// Ming Xie's empirical fit for the 3D gain-length degradation:
///
/// ```text
/// Lambda = a1 eta_d^a2 + a3 eta_eps^a4 + a5 eta_g^a6 + a7 eta_eps^a8 eta_g^a9
///        + a10 eta_d^a11 eta_g^a12 + a13 eta_d^a14 eta_eps^a15
///        + a16 eta_d^a17 eta_eps^a18 eta_g^a19
/// ```
///
/// An empirical fit (≈ 10–20 % accuracy for scaled parameters up to order
/// unity); values far outside that range only indicate "no useful gain".
pub fn ming_xie(
    gain_length_1d_m: f64,
    wavelength_nm: f64,
    rms_beam_size_m: f64,
    beta_m: f64,
    geometric_emittance_m: f64,
    period_mm: f64,
    energy_spread_rel: f64,
) -> MingXieFit {
    let lam = wavelength_nm * 1e-9;
    let k = 2.0 * std::f64::consts::PI / lam;
    let l = gain_length_1d_m;
    let eta_d = l / (2.0 * k * rms_beam_size_m * rms_beam_size_m);
    let eta_eps = (l / beta_m) * (4.0 * std::f64::consts::PI * geometric_emittance_m / lam);
    let eta_gamma = 4.0 * std::f64::consts::PI * (l / (period_mm * 1e-3)) * energy_spread_rel;
    let a = &XIE_A;
    let lambda = a[0] * eta_d.powf(a[1])
        + a[2] * eta_eps.powf(a[3])
        + a[4] * eta_gamma.powf(a[5])
        + a[6] * eta_eps.powf(a[7]) * eta_gamma.powf(a[8])
        + a[9] * eta_d.powf(a[10]) * eta_gamma.powf(a[11])
        + a[12] * eta_d.powf(a[13]) * eta_eps.powf(a[14])
        + a[15] * eta_d.powf(a[16]) * eta_eps.powf(a[17]) * eta_gamma.powf(a[18]);
    MingXieFit {
        eta_d,
        eta_eps,
        eta_gamma,
        lambda,
    }
}

/// FEL performance estimate from machine parameters (1D theory plus the
/// Ming Xie 3D correction). Every field is a derived quantity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FelEstimate {
    /// Electron Lorentz factor.
    pub gamma: f64,
    /// On-axis fundamental resonance wavelength of the undulator, nm.
    pub resonant_wavelength_nm: f64,
    /// rms beam size in the undulator, m.
    pub rms_beam_size_m: f64,
    /// 1D Pierce parameter.
    pub rho_1d: f64,
    /// 1D power gain length, m.
    pub gain_length_1d_m: f64,
    /// Ming Xie scaled parameters and degradation.
    pub xie: MingXieFit,
    /// 3D gain length `L_g1D (1 + Lambda)`, m.
    pub gain_length_3d_m: f64,
    /// Peak electron-beam power, W.
    pub beam_power_w: f64,
    /// 1D saturation power `rho P_beam`, W.
    pub saturation_power_1d_w: f64,
    /// Ming Xie saturation power `1.6 rho (L_g1D / L_g3D)^2 P_beam`, W.
    pub saturation_power_3d_w: f64,
    /// 1D saturation length `lambda_u / rho`, m.
    pub saturation_length_1d_m: f64,
    /// 3D saturation length estimate `(lambda_u / rho) (1 + Lambda)`, m.
    pub saturation_length_3d_m: f64,
    /// Undulator magnetic length, m.
    pub undulator_length_m: f64,
    /// Energy-spread criterion `sigma_gamma / (gamma rho)` (must be < 1).
    pub energy_spread_over_rho: f64,
    /// Emittance criterion `eps / (lambda / 4 pi)` (≲ 1 for full transverse coherence).
    pub emittance_over_photon_emittance: f64,
}

/// Evaluate the FEL estimate for a beam of Lorentz factor `gamma` in the
/// given undulator. The radiation wavelength is the undulator's on-axis
/// fundamental resonance.
pub fn fel_estimate(
    gamma: f64,
    undulator: &UndulatorParams,
    beam: &ElectronBeamParams,
) -> FelEstimate {
    let wavelength_nm = undulator.resonance_nm(gamma);
    let sigma = beam.rms_size_m(gamma);
    let eps = beam.geometric_emittance_m(gamma);
    let rho = pierce_parameter_1d(
        beam.peak_current_a,
        sigma,
        gamma,
        undulator.k,
        undulator.period_mm,
    );
    let lg = fel_gain_length_1d_m(undulator.period_mm, rho);
    let xie = ming_xie(
        lg,
        wavelength_nm,
        sigma,
        beam.beta_m,
        eps,
        undulator.period_mm,
        beam.energy_spread_rel,
    );
    let one_plus = 1.0 + xie.lambda;
    let p_beam = electron_beam_power_w(gamma, beam.peak_current_a);
    let l_sat = undulator.period_mm * 1e-3 / rho;
    FelEstimate {
        gamma,
        resonant_wavelength_nm: wavelength_nm,
        rms_beam_size_m: sigma,
        rho_1d: rho,
        gain_length_1d_m: lg,
        xie,
        gain_length_3d_m: lg * one_plus,
        beam_power_w: p_beam,
        saturation_power_1d_w: rho * p_beam,
        saturation_power_3d_w: 1.6 * rho * p_beam / (one_plus * one_plus),
        saturation_length_1d_m: l_sat,
        saturation_length_3d_m: l_sat * one_plus,
        undulator_length_m: undulator.length_m(),
        energy_spread_over_rho: beam.energy_spread_rel / rho,
        emittance_over_photon_emittance: eps
            / (wavelength_nm * 1e-9 / (4.0 * std::f64::consts::PI)),
    }
}

// ---------------------------------------------------------------------------
// Inverse Compton / Thomson scattering
// ---------------------------------------------------------------------------

/// Scattered photons per head-on collision of round Gaussian electron and
/// laser beams in the linear Thomson regime:
/// `N_x = sigma_T N_e N_L / (2 pi (sigma_e^2 + sigma_L^2))`.
/// Assumes `a0 << 1`, recoil negligible, and laser Rayleigh range and
/// electron beta* much longer than the pulse/bunch lengths (no hourglass
/// loss).
pub fn thomson_photons_per_collision(
    n_electrons: f64,
    n_laser_photons: f64,
    sigma_e_m: f64,
    sigma_l_m: f64,
) -> f64 {
    THOMSON_CROSS_SECTION_M2 * n_electrons * n_laser_photons
        / (2.0 * std::f64::consts::PI * (sigma_e_m * sigma_e_m + sigma_l_m * sigma_l_m))
}

/// Fraction of Thomson-backscattered photons inside a lab-frame cone of
/// half-angle `half_angle_rad` around the electron direction.
///
/// In the electron rest frame the azimuth-averaged dipole pattern is
/// `(3 / 16 pi)(1 + cos^2 theta')` (for linear or unpolarized light); the
/// cone maps to `cos theta'_c = (cos theta_c - beta) / (1 - beta cos theta_c)`
/// (aberration, exact), giving `f = (3/8) [(1 - u) + (1 - u^3)/3]` with
/// `u = cos theta'_c`. For `gamma >> 1`, half the photons fall inside
/// `theta = 1/gamma`.
pub fn thomson_collection_fraction(gamma: f64, half_angle_rad: f64) -> f64 {
    if half_angle_rad <= 0.0 {
        return 0.0;
    }
    if half_angle_rad >= std::f64::consts::PI {
        return 1.0;
    }
    let beta = (1.0 - 1.0 / (gamma * gamma)).max(0.0).sqrt();
    // Cancellation-free 1 - u = (1 + beta)(1 - cos theta) / (1 - beta cos theta).
    let one_minus_cos = 2.0 * (0.5 * half_angle_rad).sin().powi(2);
    let one_minus_beta = 1.0 / (gamma * gamma * (1.0 + beta));
    let w = (1.0 + beta) * one_minus_cos / (one_minus_beta + beta * one_minus_cos);
    let u = 1.0 - w;
    (3.0 / 8.0) * w * (1.0 + (1.0 + u + u * u) / 3.0)
}

/// Lab half-angle (rad) inside which the Compton-scattered wavelength stays
/// within a relative red-shift `rel_bandwidth` of the on-axis line:
/// `gamma^2 theta^2 / (1 + a0^2/2) <= rel_bandwidth`.
pub fn ics_half_angle_for_bandwidth(gamma: f64, a0: f64, rel_bandwidth: f64) -> f64 {
    (rel_bandwidth * (1.0 + a0 * a0 / 2.0)).sqrt() / gamma
}

// ---------------------------------------------------------------------------
// Laser plasmas and strong-field physics
// ---------------------------------------------------------------------------

/// Plasma critical density (cm^-3) for a laser of wavelength
/// `laser_wavelength_um`: `n_c = eps0 m_e omega^2 / e^2`
/// (= `1.115e21 / lambda^2[um]`).
pub fn plasma_critical_density_cm3(laser_wavelength_um: f64) -> f64 {
    let omega = 2.0 * std::f64::consts::PI * SPEED_OF_LIGHT_M_S / (laser_wavelength_um * 1e-6);
    EPSILON_0_F_M * ELECTRON_MASS_KG * omega * omega / (ELECTRON_CHARGE_C * ELECTRON_CHARGE_C)
        * 1e-6
}

/// Keldysh parameter `gamma_K = sqrt(I_p / (2 U_p))` (< 1: tunnelling
/// regime, where the three-step HHG picture applies).
pub fn keldysh_parameter(ionization_potential_ev: f64, ponderomotive_ev: f64) -> f64 {
    (ionization_potential_ev / (2.0 * ponderomotive_ev)).sqrt()
}

/// Critical ionization fraction for plane-wave HHG phase matching: the
/// free-electron dispersion `-eta N_atm r_e lambda^2 / (2 pi)` cancels the
/// neutral-gas dispersion `(1 - eta) dn` at
/// `eta_cr = dn / (dn + N_atm r_e lambda^2 / (2 pi))`, where `dn` is the
/// gas refractivity `n - 1` at the driver wavelength (at 0 °C, 1 atm; the
/// harmonic's own refractivity is neglected). Above `eta_cr` the harmonics
/// cannot be phase-matched regardless of pressure.
pub fn hhg_critical_ionization(refractivity_stp: f64, driver_wavelength_nm: f64) -> f64 {
    let lam = driver_wavelength_nm * 1e-9;
    let plasma =
        LOSCHMIDT_M3 * CLASSICAL_ELECTRON_RADIUS_M * lam * lam / (2.0 * std::f64::consts::PI);
    refractivity_stp / (refractivity_stp + plasma)
}

// ---------------------------------------------------------------------------
// Coherent emission from a microbunched beam (SSMB, CHG)
// ---------------------------------------------------------------------------

/// Average coherent power (W) radiated by a density-modulated electron beam
/// in a planar radiator undulator tuned to the bunching wavelength:
/// `P = 2 pi^2 alpha hbar N Q_1(K) |b|^2 (I_avg / e)(I_pk / e)`
/// (≈ `591.8 W × N Q_1 |b|^2 I_avg[A] I_pk[A]`).
///
/// Derivation: `N_e` electrons radiate `|E_1(w)|^2 |sum_j exp(i w t_j)|^2
/// = |E_1|^2 (N_e + N_e (N_e - 1) |b(w)|^2)`. For a quasi-CW beam of
/// duration `T` modulated at `w0` with bunching `b`,
/// `Int |b(w)|^2 dw = 2 pi |b|^2 / T`, so the coherent term radiates
/// `P = 2 pi (I/e)^2 |b|^2 dW_1/dw`; Kim's central-cone single-electron
/// spectral density `dW_1/dw = pi alpha N Q_1 hbar` gives the formula, and
/// a beam with time structure replaces `I^2` by `<I^2> = I_avg I_pk`.
/// Assumptions: transversely coherent emission (beam size well below the
/// radiation mode size), bunching constant along the radiator, radiator
/// detuned for maximum angle-integrated emission (exactly on the on-axis
/// resonance the angle-integrated density — hence `P` — is half this).
pub fn coherent_undulator_power_w(
    num_periods: usize,
    k: f64,
    bunching_factor: f64,
    average_current_a: f64,
    peak_current_a: f64,
) -> f64 {
    let pi = std::f64::consts::PI;
    2.0 * pi
        * pi
        * FINE_STRUCTURE
        * HBAR_J_S
        * num_periods as f64
        * undulator_qn(1, k)
        * bunching_factor
        * bunching_factor
        * (average_current_a / ELECTRON_CHARGE_C)
        * (peak_current_a / ELECTRON_CHARGE_C)
}

/// Bunching factor of Gaussian microbunches of rms length `sigma_z_nm` at
/// `wavelength_nm`: `b = exp(-(k sigma_z)^2 / 2)`, `k = 2 pi / lambda`.
pub fn gaussian_microbunch_bunching(sigma_z_nm: f64, wavelength_nm: f64) -> f64 {
    let ks = 2.0 * std::f64::consts::PI * sigma_z_nm / wavelength_nm;
    (-0.5 * ks * ks).exp()
}

/// rms microbunch length (nm) that yields bunching `b` at `wavelength_nm`
/// (inverse of [`gaussian_microbunch_bunching`]): `sigma_z = sqrt(2 ln(1/b)) / k`.
pub fn microbunch_length_for_bunching_nm(bunching_factor: f64, wavelength_nm: f64) -> f64 {
    if bunching_factor >= 1.0 {
        return 0.0;
    }
    if bunching_factor <= 0.0 {
        return f64::INFINITY;
    }
    (2.0 * (1.0 / bunching_factor).ln()).sqrt() * wavelength_nm / (2.0 * std::f64::consts::PI)
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
    fn test_bm_universal_flux_mpmath_fixtures() {
        // y Int_y^inf K_{5/3}: 30-digit mpmath quadrature values.
        let cases = [
            (0.1, 0.818_185_534_872_853_3),
            (0.3, 0.917_705_470_842_465_7),
            (1.0, 0.651_422_815_355_364),
            (3.0, 0.128_565_710_009_063_81),
            (10.0, 1.922_382_643_008_689_7e-4),
        ];
        for (y, expected) in cases {
            assert_relative_eq!(bm_universal_flux(y), expected, max_relative = 1e-11);
        }
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

    // Fixture values below were computed independently with
    // scipy.special / mpmath (see the WP-C notes), not with this code.

    #[test]
    fn test_photon_conversions() {
        // 13.5 nm: 91.84 eV = 1.4714e-17 J
        assert_relative_eq!(
            photon_energy_j(13.5),
            1239.84193 / 13.5 * 1.602176634e-19,
            max_relative = 1e-12
        );
        // 1 W at 13.5 nm = 6.796e16 photons/s; round trip is exact.
        let rate = watts_to_photon_rate(1.0, 13.5);
        assert_relative_eq!(rate, 6.7962e16, max_relative = 1e-4);
        assert_relative_eq!(photon_rate_to_watts(rate, 13.5), 1.0, max_relative = 1e-12);
        // Shorter wavelength -> fewer photons per joule.
        assert!(watts_to_photon_rate(1.0, 6.7) < rate);
    }

    #[test]
    fn test_bessel_series_branch_fixtures() {
        let cases = [
            (0, 1.0, 0.765_197_686_557_966_6),
            (1, 1.0, 0.440_050_585_744_933_55),
            (2, 3.0, 0.486_091_260_585_891_2),
            (5, 10.0, -0.234_061_528_186_793_6),
            (1, 0.25, 0.124_025_977_322_726_97),
            (3, 0.5, 0.002_563_729_994_587_244),
            (20, 5.0, 2.770_330_052_128_941_7e-11),
            (2, 12.0, -0.084_930_494_878_604_8),
        ];
        for (n, x, expected) in cases {
            let got = bessel_j(n, x);
            assert!(
                (got - expected).abs() <= 1e-12 + 1e-10 * expected.abs(),
                "J_{n}({x}) = {got}, expected {expected}"
            );
        }
        // First zero of J_0.
        assert!(bessel_j(0, 2.404_825_557_695_773).abs() < 1e-14);
        // Special values and parity J_n(-x) = (-1)^n J_n(x).
        assert_eq!(bessel_j(0, 0.0), 1.0);
        assert_eq!(bessel_j(4, 0.0), 0.0);
        assert_relative_eq!(
            bessel_j(1, -2.0),
            -0.576_724_807_756_873_4,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            bessel_j(2, -2.0),
            0.352_834_028_615_637_7,
            max_relative = 1e-12
        );
    }

    #[test]
    fn test_bessel_miller_branch_fixtures() {
        let cases = [
            (0, 15.0, -0.014_224_472_826_780_773),
            (3, 15.0, -0.194_018_257_820_122_63),
            (7, 20.0, -0.184_221_397_720_594_43),
            (0, 30.0, -0.086_367_983_581_040_21),
            (12, 30.0, 0.148_253_351_099_660_1),
            (1, 12.5, -0.165_483_804_614_759_72),
        ];
        for (n, x, expected) in cases {
            let got = bessel_j(n, x);
            assert!(
                (got - expected).abs() <= 1e-12,
                "J_{n}({x}) = {got}, expected {expected}"
            );
        }
        // The two branches agree across the switch point.
        for n in [0, 1, 4, 9] {
            let s = bessel_j_series(n, 12.0);
            let m = bessel_j_miller(n, 12.0);
            assert!((s - m).abs() < 1e-11, "n={n}: series {s} vs Miller {m}");
        }
    }

    #[test]
    fn test_undulator_harmonic_functions_fixture() {
        // (harmonic, K, JJ_n, F_n, Q_n) from scipy.special.jv
        let cases = [
            (
                1,
                1.0,
                0.910_023_286_398_312_3,
                0.368_063_280_794_304_34,
                0.552_094_921_191_456_5,
            ),
            (
                3,
                1.0,
                0.211_664_434_216_191_27,
                0.179_207_330_848_241_45,
                0.089_603_665_424_120_72,
            ),
            (
                5,
                1.0,
                0.070_347_802_814_130_1,
                0.054_986_815_119_730_33,
                0.016_496_044_535_919_1,
            ),
            (
                1,
                2.0,
                0.808_051_985_300_826_8,
                0.290_199_115_977_158_9,
                0.870_597_347_931_476_8,
            ),
            (
                3,
                2.0,
                0.325_147_100_813_033_05,
                0.422_882_548_668_482_74,
                0.422_882_548_668_482_8,
            ),
            (
                5,
                2.0,
                0.192_685_111_852_378_1,
                0.412_528_359_217_371_8,
                0.247_517_015_530_423_07,
            ),
            (
                1,
                1.6,
                0.841_425_359_248_282_7,
                0.348_659_469_466_839_8,
                0.794_943_590_384_394_8,
            ),
        ];
        for (n, k, jj, f, q) in cases {
            assert_relative_eq!(undulator_jj(n, k), jj, max_relative = 1e-12);
            assert_relative_eq!(undulator_fn(n, k), f, max_relative = 1e-11);
            assert_relative_eq!(undulator_qn(n, k), q, max_relative = 1e-11);
        }
        // Even harmonics vanish on axis.
        assert_eq!(undulator_fn(2, 1.0), 0.0);
        assert_eq!(undulator_qn(0, 1.0), 0.0);
    }

    #[test]
    fn test_undulator_weak_field_limits() {
        // K -> 0: JJ_1 -> 1 so F_1 -> K^2 and Q_1 -> K^2.
        let k: f64 = 1e-3;
        assert_relative_eq!(undulator_fn(1, k) / (k * k), 1.0, epsilon = 1e-5);
        assert_relative_eq!(undulator_qn(1, k) / (k * k), 1.0, epsilon = 1e-5);
        // Harmonic n scales as K^(2n) at small K: F_3(2K)/F_3(K) -> 2^6.
        let ratio = undulator_fn(3, 0.02) / undulator_fn(3, 0.01);
        assert_relative_eq!(ratio, 64.0, max_relative = 1e-3);
        // Stronger K pushes relative power into higher harmonics.
        let r1 = undulator_qn(3, 1.0) / undulator_qn(1, 1.0);
        let r2 = undulator_qn(3, 2.0) / undulator_qn(1, 2.0);
        assert_relative_eq!(r1, 0.162_297_572_364_459, max_relative = 1e-10);
        assert!(r2 > r1);
    }

    #[test]
    fn test_kim_flux_constants() {
        // pi alpha 1e-3 / e = 1.431e14 (X-ray Data Booklet central-cone constant)
        let q = undulator_qn(1, 1.0);
        assert_relative_eq!(
            undulator_central_cone_flux(1, 1.0, 1, 1.0) / q,
            1.430_885_255_462_256_6e14,
            max_relative = 1e-9
        );
        // alpha gamma^2 1e-3 / e per mrad^2 at 1 GeV = 1.744e14
        let gamma_1gev = 1e3 / ELECTRON_REST_MEV;
        let f = undulator_fn(1, 1.0);
        assert_relative_eq!(
            undulator_on_axis_flux_density(1, 1.0, 1, gamma_1gev, 1.0) / f,
            1.744_274_855_671_048_4e14,
            max_relative = 1e-9
        );
        // Bending magnet (sqrt3/2pi) alpha gamma 1e-3/e per mrad at 1 GeV = 2.457e13
        let y = 0.3;
        assert_relative_eq!(
            bm_flux_per_mrad(gamma_1gev, 1.0, y) / bm_universal_flux(y),
            2.457_059_577_081_784e13,
            max_relative = 1e-9
        );
        // Flux is linear in current and N; brightness quadratic in N.
        assert_relative_eq!(
            undulator_central_cone_flux(200, 1.0, 1, 0.5)
                / undulator_central_cone_flux(100, 1.0, 1, 0.5),
            2.0,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            undulator_on_axis_flux_density(200, 1.0, 1, 1000.0, 0.5)
                / undulator_on_axis_flux_density(100, 1.0, 1, 1000.0, 0.5),
            4.0,
            max_relative = 1e-12
        );
    }

    #[test]
    fn test_compact_undulator_power_fixture() {
        // 538 MeV, lambda_u = 20 mm, K = 1, N = 100, I = 200 mA.
        let gamma = gamma_from_mev(538.0);
        let lambda1 = undulator_resonance_nm(20.0, 1.0, gamma, 1);
        assert_relative_eq!(lambda1, 13.506_478_089_298_37, max_relative = 1e-12);
        // Attwood central-cone power: 0.2324 W
        let p_cen = undulator_central_cone_power_w(1.0, 1, 0.2, lambda1);
        assert_relative_eq!(p_cen, 0.232_371_665_180_677_5, max_relative = 1e-7);
        // Kim flux: 1.58e15 photons/s/0.1%BW
        assert_relative_eq!(
            undulator_central_cone_flux(100, 1.0, 1, 0.2),
            1.579_968_964_696_903_5e15,
            max_relative = 1e-9
        );
        // Field from K and total (all-harmonic, all-angle) power: 21.04 W
        let b0 = undulator_field_from_k(1.0, 20.0);
        assert_relative_eq!(b0, 0.535_487_302_785_998_7, max_relative = 1e-9);
        assert_relative_eq!(
            undulator_total_power_w(gamma, b0, 2.0, 0.2),
            21.044_539_286_759_285,
            max_relative = 1e-8
        );
        // Engineering constant: P_T = 632.7 W per (GeV^2 T^2 m A).
        assert_relative_eq!(
            undulator_total_power_w(1e3 / ELECTRON_REST_MEV, 1.0, 1.0, 1.0),
            632.691_383_583_122_8,
            max_relative = 1e-8
        );
        // Only ~1% of the undulator's total power lands in the usable line.
        assert!(p_cen / undulator_total_power_w(gamma, b0, 2.0, 0.2) < 0.02);
    }

    #[test]
    fn test_undulator_k_inverse_resonance() {
        let gamma = gamma_from_mev(680.0);
        let k = undulator_k_for_wavelength(31.4, gamma, 13.5, 1).unwrap();
        assert_relative_eq!(k, 1.024_676_417_225_126, max_relative = 1e-9);
        assert_relative_eq!(
            undulator_resonance_nm(31.4, k, gamma, 1),
            13.5,
            max_relative = 1e-12
        );
        // Shorter than the K = 0 limit is unreachable.
        let lambda_min = undulator_resonance_nm(31.4, 0.0, gamma, 1);
        assert!(undulator_k_for_wavelength(31.4, gamma, 0.9 * lambda_min, 1).is_none());
    }

    #[test]
    fn test_bending_magnet_power_fixture() {
        // Energy loss per turn at 1 GeV (gamma = 1956.95), rho = 1 m is
        // 88.46 keV, so the ring-integrated power at 1 A is 88.46 kW.
        let gamma_1gev = 1e3 / ELECTRON_REST_MEV;
        let total = bm_power_per_mrad_w(gamma_1gev, 1.0, 1.0) * 2000.0 * std::f64::consts::PI;
        assert_relative_eq!(total, 88_461.6, max_relative = 2e-5);
        // Bending radius: rho[m] = 3.3356 p[GeV/c] / B[T].
        assert_relative_eq!(
            bending_radius_m(gamma_1gev, 1.0),
            3.335_64,
            max_relative = 1e-5
        );
        // LIGA preset: 2.5 GeV, 1.5 T, 200 mA -> 19.80 W/mrad.
        let gamma = gamma_from_mev(2500.0);
        let rho = bending_radius_m(gamma, 1.5);
        assert_relative_eq!(rho, 5.560_537_925_985_226, max_relative = 1e-6);
        assert_relative_eq!(
            bm_power_per_mrad_w(gamma, rho, 0.2),
            19.797_428_131_162_015,
            max_relative = 1e-6
        );
        // Photon flux per mrad at y = 0.3: 1.128e13 photons/s/mrad/0.1%BW.
        assert_relative_eq!(
            bm_flux_per_mrad(gamma, 0.2, 0.3),
            1.127_658_953_950_396_3e13,
            max_relative = 1e-8
        );
    }

    #[test]
    fn test_coherent_fraction_fixture_and_limits() {
        assert_relative_eq!(
            coherent_fraction_per_plane(10e-9, 13.5),
            0.097_008_051_697_548_56,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            coherent_fraction_per_plane(1e-9, 13.5),
            0.517_908_695_450_039_4,
            max_relative = 1e-12
        );
        // Zero emittance -> fully coherent; eps >> lambda/4pi -> ratio.
        assert_eq!(coherent_fraction_per_plane(0.0, 13.5), 1.0);
        let eps = 1e-6;
        let eps_r = 13.5e-9 / (4.0 * std::f64::consts::PI);
        assert_relative_eq!(
            coherent_fraction_per_plane(eps, 13.5),
            eps_r / eps,
            max_relative = 2e-3
        );
        // Halving the wavelength at fixed emittance lowers coherence.
        assert!(coherent_fraction_per_plane(10e-9, 6.7) < coherent_fraction_per_plane(10e-9, 13.5));
    }

    #[test]
    fn test_pierce_parameter_fixture_and_scaling() {
        // gamma = 2000, I = 1 kA, K = 1.5, sigma = 50 um, lambda_u = 30 mm
        let rho = pierce_parameter_1d(1000.0, 50e-6, 2000.0, 1.5, 30.0);
        assert_relative_eq!(rho, 0.001_896_046_584_629_284_8, max_relative = 1e-9);
        let lg = fel_gain_length_1d_m(30.0, rho);
        assert_relative_eq!(lg, 0.726_945_344_976_304_9, max_relative = 1e-9);
        let p_beam = electron_beam_power_w(2000.0, 1000.0);
        assert_relative_eq!(p_beam, 1.021_997_9e12, max_relative = 1e-12);
        // rho ~ I^(1/3) and sigma^(-2/3)
        let rho_2i = pierce_parameter_1d(2000.0, 50e-6, 2000.0, 1.5, 30.0);
        assert_relative_eq!(rho_2i / rho, 2.0_f64.cbrt(), max_relative = 1e-12);
        let rho_2s = pierce_parameter_1d(1000.0, 100e-6, 2000.0, 1.5, 30.0);
        assert_relative_eq!(rho_2s / rho, 2.0_f64.powf(-2.0 / 3.0), max_relative = 1e-12);
    }

    #[test]
    fn test_ming_xie_fixture_and_limits() {
        // Same case, beta = 5 m, eps = 1 um / 2000, sigma_gamma/gamma = 1e-4.
        let rho = pierce_parameter_1d(1000.0, 50e-6, 2000.0, 1.5, 30.0);
        let lg = fel_gain_length_1d_m(30.0, rho);
        let lambda_nm = undulator_resonance_nm(30.0, 1.5, 2000.0, 1);
        assert_relative_eq!(lambda_nm, 7.968_75, max_relative = 1e-12);
        let fit = ming_xie(lg, lambda_nm, 50e-6, 5.0, 1e-6 / 2000.0, 30.0, 1e-4);
        assert_relative_eq!(fit.eta_d, 0.184_392_006_110_678_85, max_relative = 1e-9);
        assert_relative_eq!(fit.eta_eps, 0.114_636_105_052_303_4, max_relative = 1e-9);
        assert_relative_eq!(fit.eta_gamma, 0.030_450_215_404_518_1, max_relative = 1e-9);
        assert_relative_eq!(fit.lambda, 0.218_913_073_050_972_16, max_relative = 1e-9);

        // Ideal beam (huge size -> no diffraction, zero emittance/spread): Lambda -> 0.
        let ideal = ming_xie(lg, lambda_nm, 1.0, 5.0, 0.0, 30.0, 0.0);
        assert!(ideal.lambda < 1e-4, "ideal Lambda = {}", ideal.lambda);
        // Energy spread equal to rho gives eta_gamma = 1/sqrt3, Lambda ~ 3 eta^2 = 1.
        let spread = ming_xie(lg, lambda_nm, 1.0, 5.0, 0.0, 30.0, rho);
        assert_relative_eq!(spread.eta_gamma, 1.0 / 3.0_f64.sqrt(), max_relative = 1e-12);
        assert_relative_eq!(spread.lambda, 1.0, epsilon = 1e-4);
    }

    #[test]
    fn test_fel_estimate_saturation_consistency() {
        let und = UndulatorParams {
            period_mm: 30.0,
            k: 1.5,
            num_periods: 1000,
        };
        let gamma = 2000.0;
        // beta chosen so that sigma = sqrt(eps_n beta / (beta gamma)) ~ 50 um.
        let beam = ElectronBeamParams {
            peak_current_a: 1000.0,
            norm_emittance_um: 1.0,
            energy_spread_rel: 1e-4,
            beta_m: 5.0,
        };
        let est = fel_estimate(gamma, &und, &beam);
        assert_relative_eq!(est.rms_beam_size_m, 50e-6, max_relative = 1e-6);
        assert_relative_eq!(est.rho_1d, 0.001_896_046_584_629_284_8, max_relative = 1e-6);
        assert_relative_eq!(
            est.saturation_power_1d_w,
            1.937_755_627_793_301e9,
            max_relative = 1e-6
        );
        assert_relative_eq!(
            est.gain_length_3d_m / est.gain_length_1d_m,
            1.0 + est.xie.lambda,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            est.saturation_length_1d_m,
            0.03 / est.rho_1d,
            max_relative = 1e-12
        );
        assert_relative_eq!(est.undulator_length_m, 30.0, max_relative = 1e-12);
        assert!(est.energy_spread_over_rho < 1.0);
    }

    #[test]
    fn test_thomson_yield_fixture() {
        // 100 pC, 10 mJ at 1030 nm, 10 um rms spots: 1.713e6 photons/collision.
        let n_e = 100e-12 / ELECTRON_CHARGE_C;
        let n_l = 10e-3 / photon_energy_j(1030.0);
        let n_x = thomson_photons_per_collision(n_e, n_l, 10e-6, 10e-6);
        // The fixture used h c from the exact SI h and c; HC_EV_NM is the
        // rounded 1239.84193 eV·nm (4.4e-8 relative), hence the tolerance.
        assert_relative_eq!(n_x, 1_713_256.640_174_27, max_relative = 1e-7);
        // Doubling both spot sizes quarters the yield; yield is bilinear.
        assert_relative_eq!(
            thomson_photons_per_collision(n_e, n_l, 20e-6, 20e-6),
            n_x / 4.0,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            thomson_photons_per_collision(2.0 * n_e, n_l, 10e-6, 10e-6),
            2.0 * n_x,
            max_relative = 1e-12
        );
    }

    #[test]
    fn test_thomson_collection_fraction_fixture() {
        // gamma for 13.5 nm from 1030 nm at a0 = 0.1; 2% in-band cone.
        let gamma = 4.378_292_411_939_207;
        let theta_bw = ics_half_angle_for_bandwidth(gamma, 0.1, 0.02);
        assert_relative_eq!(theta_bw, 0.032_381_224_333_251_954, max_relative = 1e-12);
        // Fixture confirmed by direct lab-frame quadrature of the boosted
        // dipole pattern: 0.02825
        assert_relative_eq!(
            thomson_collection_fraction(gamma, theta_bw),
            0.028_253_814_825_674_74,
            max_relative = 1e-9
        );
        assert_relative_eq!(
            thomson_collection_fraction(gamma, 1e-3),
            2.799_810_362_402_732_6e-5,
            max_relative = 1e-7
        );
        assert_eq!(thomson_collection_fraction(gamma, 0.0), 0.0);
        assert_eq!(
            thomson_collection_fraction(gamma, std::f64::consts::PI),
            1.0
        );
        // Ultrarelativistic textbook limit: half the photons inside 1/gamma.
        assert_relative_eq!(
            thomson_collection_fraction(1e4, 1e-4),
            0.5,
            max_relative = 1e-6
        );
    }

    #[test]
    fn test_plasma_critical_density_fixture() {
        assert_relative_eq!(
            plasma_critical_density_cm3(1.0),
            1.114_854_216_171_690_5e21,
            max_relative = 1e-9
        );
        assert_relative_eq!(
            plasma_critical_density_cm3(10.6),
            9.922_162_835_276_704e18,
            max_relative = 1e-9
        );
        // n_c ~ 1/lambda^2
        assert_relative_eq!(
            plasma_critical_density_cm3(1.0) / plasma_critical_density_cm3(2.0),
            4.0,
            max_relative = 1e-12
        );
    }

    #[test]
    fn test_hhg_critical_ionization_and_keldysh() {
        // Ar (n - 1 = 2.81e-4) at 800 nm: 3.5 %; Ne (6.7e-5): 0.86 %
        assert_relative_eq!(
            hhg_critical_ionization(2.81e-4, 800.0),
            0.035_155_982_993_349_26,
            max_relative = 1e-6
        );
        assert_relative_eq!(
            hhg_critical_ionization(6.7e-5, 800.0),
            0.008_612_988_063_330_766,
            max_relative = 1e-6
        );
        // Longer drivers tolerate less ionization.
        assert!(hhg_critical_ionization(2.81e-4, 1030.0) < hhg_critical_ionization(2.81e-4, 800.0));
        // Keldysh for Ar at 2e14 W/cm^2, 800 nm: U_p = 11.94 eV -> 0.812
        let up = ponderomotive_ev(2e14, 0.8);
        assert_relative_eq!(
            keldysh_parameter(15.760, up),
            0.812_301_587_613_254_5,
            max_relative = 1e-9
        );
    }

    #[test]
    fn test_coherent_undulator_power_fixture() {
        // Constant: 2 pi^2 alpha hbar / e^2 = 591.77 W/A^2 per (N Q_1 |b|^2).
        let q = undulator_qn(1, 1.6);
        assert_relative_eq!(
            coherent_undulator_power_w(1, 1.6, 1.0, 1.0, 1.0) / q,
            591.766_592_901_99,
            max_relative = 1e-9
        );
        // N = 100, K = 1.6, b = 0.1, 1 A DC beam: 470.4 W
        assert_relative_eq!(
            coherent_undulator_power_w(100, 1.6, 0.1, 1.0, 1.0),
            470.421_060_031_048_46,
            max_relative = 1e-9
        );
        // Coherent: quadratic in bunching and in current (DC beam).
        assert_relative_eq!(
            coherent_undulator_power_w(100, 1.6, 0.2, 1.0, 1.0)
                / coherent_undulator_power_w(100, 1.6, 0.1, 1.0, 1.0),
            4.0,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            coherent_undulator_power_w(100, 1.6, 0.1, 2.0, 2.0)
                / coherent_undulator_power_w(100, 1.6, 0.1, 1.0, 1.0),
            4.0,
            max_relative = 1e-12
        );
    }

    #[test]
    fn test_microbunch_form_factor_round_trip() {
        // b = 0.1 at 13.5 nm needs 4.61 nm rms microbunches.
        let sigma = microbunch_length_for_bunching_nm(0.1, 13.5);
        assert_relative_eq!(sigma, 4.610_804_860_681_496, max_relative = 1e-9);
        assert_relative_eq!(
            gaussian_microbunch_bunching(sigma, 13.5),
            0.1,
            max_relative = 1e-12
        );
        assert_eq!(microbunch_length_for_bunching_nm(1.0, 13.5), 0.0);
        assert!(microbunch_length_for_bunching_nm(0.0, 13.5).is_infinite());
    }
}
