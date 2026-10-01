//! Quantum (N-photon) lithography — theoretical research module.
//!
//! Two physically different N-photon exposures are kept strictly apart:
//!
//! - **Classical N-photon absorption** of a classical aerial image,
//!   `E = I^N` ([`n_photon_absorption_image`]). A resist whose response goes
//!   as the N-th power of the intensity sharpens the image — narrower bright
//!   lines, steeper edges, higher contrast — but `I^N` contains only the
//!   spatial frequencies of `I` and their sums, so it keeps the classical
//!   fundamental period (a two-beam fringe `cos²(kx)` becomes `cos²ᴺ(kx)`).
//!   It cannot print a pitch the optics does not transmit. This is what an
//!   N-photon absorber does with classical light; for this path (and the
//!   deprecated [`compute_quantum_aerial_image`] alias) the λ/(2N) and
//!   [`QuantumLithographyParams::quantum_resolution_nm`] figures are
//!   reference numbers, not properties of the computed image.
//! - **Entangled N00N-state absorption.** N photons in the path-entangled
//!   state `(|N,0⟩ + |0,N⟩)/√2` acquire the relative phase of the two paths
//!   N times, so N-photon absorption of a two-beam interference writes
//!   fringes N times finer than the classical fringe (Boto et al. \[1\]):
//!   period `λ/(2N sinθ)` instead of `λ/(2 sinθ)` ([`TwoBeamNPhoton`],
//!   analytic). Boto et al. also argue that suitably prepared entangled
//!   states can write general patterns equivalent to classical imaging at
//!   `λ/N`; [`noon_ideal_image`] offers exactly that as the clearly labelled
//!   ideal limit — the same source, optics, mask and grid imaged at
//!   `λ_eff = λ/N` with the same NA and pupil fill.
//!
//! Decoherence is a fidelity `F`: the fraction of the N-photon exposure
//! contributed by the ideal N00N component. The rest is *classical N-photon
//! absorption* of the same light (`I^N`, or the classical N-photon fringe) —
//! not linear absorption, which an N-photon resist does not have.
//!
//! # Key equations
//!
//! ```text
//!   classical N-photon absorption   E(x) = I(x)^N                (sharpening, same period)
//!   two-beam fringe                 I(x) = 1 + cos(Kx + φ),   K = 2k sinθ = 4π sinθ / λ
//!   classical N-photon fringe       E_cl(x) = (1 + cos(Kx + φ))^N / m_N,   m_N = C(2N, N) / 2^N
//!                                           = 1 + Σ_{j=1..N} a_j cos(j(Kx + φ)),
//!                                             a_j = 2 C(2N, N−j) / C(2N, N)
//!   ideal N00N fringe               E_NOON(x) = 1 + cos(N(Kx + φ)),   period λ / (2N sinθ)
//!   fidelity mixture (two beams)    E = F E_NOON + (1 − F) E_cl              (unit mean)
//!   general masks (ideal limit)     E = F I_{λ/N}(x) + (1 − F) I_λ(x)^N     (clear field = 1)
//!   resolution (ideal limit)        0.61 λ / (N NA)   vs   classical 0.61 λ / NA
//!   flux (order of magnitude)       Φ_rel = Φ_cross / (P_HVM / E_photon)
//!   exposure-time ratio             t_N / t_cl = p_clear N P_HVM / (σ_E Φ_cross D)
//! ```
//!
//! θ is the air-side half-angle; as for classical two-beam interference
//! ([`crate::interference`]) the in-resist fringe period does not depend on
//! the resist index.
//!
//! # Model status
//!
//! 🧪 Theoretical — research projections, not engineering predictions.
//!
//! Modelled:
//!
//! - Classical N-photon absorption of any classical aerial image, exact for
//!   an ideal N-th-order absorber.
//! - Ideal two-beam N00N fringes and the classical N-photon fringe in closed
//!   form, with a fidelity mixture in which the non-N00N fraction contributes
//!   classical N-photon absorption.
//! - General masks in Boto et al.'s ideal limit: classical imaging at `λ/N`
//!   with the same NA and pupil fill, mixed with the classical N-photon image.
//! - A flux and exposure-time budget derived from the entangled-photon
//!   source's documented constants (`source_models::entangled`): the
//!   entangled-regime crossover flux (about one photon per spectral mode;
//!   it scales with the down-converted bandwidth, and 1e13 photons/s is a
//!   representative broadband value, not a universal limit), the ETPA
//!   cross-section bounds and claims, and the HVM wafer power.
//!
//! Not modelled: preparation of N00N-type states for arbitrary 2D patterns
//! (the `λ/N` equivalence presumes it); photon loss, which degrades N00N
//! states rapidly with N (represented only through F); multi-mode or
//! partially coherent entangled illumination beyond the classical engine's
//! source sampling; N-photon resist chemistry (no material has recorded an
//! entangled sub-Rayleigh pattern — the demonstrations, including the
//! two-photon experiment of D'Angelo, Chekhova and Shih \[2\], used
//! photon-coincidence detection); wavelength-dependent pupil amplitude in
//! the `λ/N` limit (coatings and materials are evaluated at `λ/N`, so the
//! limit is meaningful only for optics whose pupil does not depend on λ
//! beyond its NA); N ≥ 3 ETPA cross-sections (the N = 2 values are used as
//! optimistic stand-ins).
//!
//! # References
//!
//! 1. A. N. Boto, P. Kok, D. S. Abrams, S. L. Braunstein, C. P. Williams,
//!    J. P. Dowling, "Quantum Interferometric Optical Lithography:
//!    Exploiting Entanglement to Beat the Diffraction Limit," Phys. Rev.
//!    Lett. 85, 2733–2736 (2000), doi:10.1103/PhysRevLett.85.2733.
//! 2. M. D'Angelo, M. V. Chekhova, Y. Shih, "Two-Photon Diffraction and
//!    Quantum Lithography," Phys. Rev. Lett. 87, 013602 (2001),
//!    doi:10.1103/PhysRevLett.87.013602.
//! 3. B. Dayan, A. Pe'er, A. A. Friesem, Y. Silberberg, "Nonlinear
//!    Interactions with an Ultrahigh Flux of Broadband Entangled Photons,"
//!    Phys. Rev. Lett. 94, 043602 (2005), doi:10.1103/PhysRevLett.94.043602
//!    — Eq. (1), the entangled-regime crossover flux (one photon per
//!    spectral mode) behind the flux budget's 1e13 photons/s order.

use ndarray::Array2;
use serde::{Deserialize, Serialize};

use crate::aerial::{AerialImageEngine, ImageNormalization, ImagingSettings};
use crate::error::{LithographyError, Result};
use crate::mask::Mask;
use crate::optics::OpticalSystem;
use crate::source::LithographySource;
use crate::source_models::entangled::{
    CLEARING_ACTIVATION_PROBABILITY, ENTANGLED_FLUX_CEILING_PHOTONS_S,
    ETPA_CROSS_SECTION_CLAIMED_MAX_CM2, ETPA_CROSS_SECTION_TIGHTEST_BOUND_CM2,
    ETPA_CROSS_SECTION_UPPER_BOUND_CM2, HVM_WAFER_POWER_W,
};
use crate::source_models::physics::photon_energy_j;
use crate::types::{Grid2D, GridConfig};

/// Largest supported number of photons per absorption event (guards the
/// closed forms against overflow; physically N00N states beyond a handful of
/// photons are far out of reach anyway).
pub const MAX_PHOTONS: usize = 256;

/// ASSUMED classical reference dose (mJ/cm²) for the exposure-time
/// comparison of [`exposure_time_ratio_for_cross_section`].
pub const REFERENCE_CLASSICAL_DOSE_MJ_CM2: f64 = 30.0;

/// Quantum lithography parameters (kept for the entangled-photon source
/// bridge and the frontends).
#[derive(Debug, Clone)]
pub struct QuantumLithographyParams {
    /// Number of entangled photons (N). N=2 is biphoton.
    pub num_entangled_photons: usize,
    /// Source wavelength (nm).
    pub wavelength_nm: f64,
    /// Numerical aperture.
    pub na: f64,
    /// Fraction of the N-photon exposure contributed by the ideal N00N
    /// component (0–1); the remainder is classical N-photon absorption.
    pub fidelity: f64,
}

impl Default for QuantumLithographyParams {
    fn default() -> Self {
        Self {
            num_entangled_photons: 2,
            wavelength_nm: 157.63,
            na: 0.75,
            fidelity: 1.0,
        }
    }
}

impl QuantumLithographyParams {
    /// Validate the parameters.
    pub fn validate(&self) -> Result<()> {
        check_photons(self.num_entangled_photons)?;
        check_fidelity(self.fidelity)?;
        if self.wavelength_nm.is_nan() || self.wavelength_nm <= 0.0 {
            return Err(LithographyError::InvalidParameter {
                name: "wavelength_nm",
                value: self.wavelength_nm,
                reason: "must be positive",
            });
        }
        if self.na.is_nan() || self.na <= 0.0 || self.na >= 1.0 {
            return Err(LithographyError::InvalidParameter {
                name: "na",
                value: self.na,
                reason: "must be in range (0, 1)",
            });
        }
        Ok(())
    }

    /// Effective wavelength of the ideal N00N limit: `λ_eff = λ / N`.
    pub fn effective_wavelength_nm(&self) -> f64 {
        self.wavelength_nm / self.num_entangled_photons as f64
    }

    /// Rayleigh resolution of the **ideal N00N limit**, `0.61 λ / (N NA)`.
    ///
    /// Reached only by the ideal-limit models ([`TwoBeamNPhoton`] at
    /// `fidelity = 1`, [`noon_ideal_image`]); classical N-photon absorption
    /// ([`n_photon_absorption_image`]) never reaches it — it keeps
    /// [`Self::classical_resolution_nm`].
    pub fn quantum_resolution_nm(&self) -> f64 {
        0.61 * self.wavelength_nm / (self.num_entangled_photons as f64 * self.na)
    }

    /// Classical Rayleigh resolution for comparison: 0.61 × λ / NA.
    pub fn classical_resolution_nm(&self) -> f64 {
        0.61 * self.wavelength_nm / self.na
    }

    /// Resolution improvement factor of the ideal N00N limit (N).
    pub fn resolution_factor(&self) -> f64 {
        self.num_entangled_photons as f64
    }

    /// Order-of-magnitude estimate of the largest entangled photon rate
    /// relative to an HVM classical exposure at the same photon energy (see
    /// [`FluxBudget`]). 1 for N = 1 (classical light).
    pub fn relative_flux(&self) -> f64 {
        self.flux_budget().relative_flux
    }

    /// Exposure-time ratio (entangled N-photon / classical) in the DEFAULT
    /// scenario — the ETPA cross-section at its loosest independent upper
    /// bound — so a **lower bound** on the true ratio (see [`FluxBudget`]).
    /// 1 for N = 1.
    pub fn exposure_time_ratio(&self) -> f64 {
        self.flux_budget().exposure_time_ratio_bound
    }

    /// Flux and exposure-time budget at this wavelength and N.
    pub fn flux_budget(&self) -> FluxBudget {
        FluxBudget::new(self.wavelength_nm, self.num_entangled_photons)
    }
}

fn check_photons(n: usize) -> Result<()> {
    if n == 0 || n > MAX_PHOTONS {
        return Err(LithographyError::InvalidParameter {
            name: "num_entangled_photons",
            value: n as f64,
            reason: "must be in 1..=256",
        });
    }
    Ok(())
}

fn check_fidelity(fidelity: f64) -> Result<()> {
    if !(0.0..=1.0).contains(&fidelity) {
        return Err(LithographyError::InvalidParameter {
            name: "fidelity",
            value: fidelity,
            reason: "must be in range [0, 1]",
        });
    }
    Ok(())
}

// ============================================================================
// Flux budget
// ============================================================================

/// Exposure-time ratio of entangled N-photon absorption to a classical
/// exposure, for an ETPA cross-section `cross_section_cm2`:
/// `t_N / t_cl = p_clear · N · P_HVM / (σ_E · Φ_cross · D)`.
///
/// The entangled exposure runs at the entangled-regime crossover flux
/// (`ENTANGLED_FLUX_CEILING_PHOTONS_S` photons/s, i.e. `Φ_cross / N`
/// N-photon events/s; above it pairs overlap and absorption turns classical) and clears when the activation probability per
/// molecule reaches `CLEARING_ACTIVATION_PROBABILITY`
/// (rate `σ_E · flux`, linear in flux as for ETPA); the classical exposure
/// delivers [`REFERENCE_CLASSICAL_DOSE_MJ_CM2`] at `HVM_WAFER_POWER_W`. The
/// exposed area cancels. Constants from `source_models::entangled`; no data
/// exist for N ≥ 3, so the N = 2 cross-section is an optimistic stand-in.
/// Infinite for a non-positive cross-section.
pub fn exposure_time_ratio_for_cross_section(num_photons: usize, cross_section_cm2: f64) -> f64 {
    if cross_section_cm2.is_nan() || cross_section_cm2 <= 0.0 {
        return f64::INFINITY;
    }
    let dose_j_cm2 = REFERENCE_CLASSICAL_DOSE_MJ_CM2 * 1e-3;
    CLEARING_ACTIVATION_PROBABILITY * num_photons as f64 * HVM_WAFER_POWER_W
        / (cross_section_cm2 * ENTANGLED_FLUX_CEILING_PHOTONS_S * dose_j_cm2)
}

/// Photon-flux and exposure-time budget of entangled N-photon lithography
/// relative to an HVM-class classical exposure at the same photon energy,
/// derived from the entangled-photon source's documented constants rather
/// than free parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FluxBudget {
    /// Photons per absorption event N.
    pub num_photons: usize,
    /// Photon energy at the physical wavelength (eV).
    pub photon_energy_ev: f64,
    /// Photon rate of the HVM reference wafer power `HVM_WAFER_POWER_W` at
    /// this photon energy (photons/s).
    pub hvm_photon_rate_per_s: f64,
    /// Order of the entangled-regime crossover flux (photons/s): above about
    /// one photon per spectral mode, pairs overlap and two-photon absorption
    /// turns classical. It scales with the down-converted bandwidth (Dayan
    /// et al. \[3\], Eq. (1)); 1e13 photons/s is a representative broadband
    /// value, not a universal limit.
    pub entangled_rate_ceiling_per_s: f64,
    /// Order-of-magnitude estimate of the largest usable entangled photon
    /// rate relative to the HVM rate, `min(1, crossover / HVM rate)`; 1 for
    /// N = 1.
    pub relative_flux: f64,
    /// DEFAULT exposure-time ratio: ETPA cross-section at its loosest
    /// independent upper bound (1e-23 cm²) — a LOWER bound on the ratio.
    pub exposure_time_ratio_bound: f64,
    /// Same with the tightest independent bound (1e-25 cm²).
    pub exposure_time_ratio_tightest_bound: f64,
    /// OPTIMISTIC scenario: the largest disputed early claim (1e-17 cm²).
    pub exposure_time_ratio_claimed: f64,
}

impl FluxBudget {
    /// Budget for N-photon exposure at `wavelength_nm` (N = 1 is classical:
    /// relative flux and every ratio are 1).
    pub fn new(wavelength_nm: f64, num_photons: usize) -> Self {
        let e_j = photon_energy_j(wavelength_nm);
        let hvm_rate = HVM_WAFER_POWER_W / e_j;
        let classical = num_photons <= 1;
        let ratio = |sigma: f64| {
            if classical {
                1.0
            } else {
                exposure_time_ratio_for_cross_section(num_photons, sigma)
            }
        };
        Self {
            num_photons,
            photon_energy_ev: e_j / crate::source_models::physics::ELECTRON_CHARGE_C,
            hvm_photon_rate_per_s: hvm_rate,
            entangled_rate_ceiling_per_s: ENTANGLED_FLUX_CEILING_PHOTONS_S,
            relative_flux: if classical {
                1.0
            } else {
                (ENTANGLED_FLUX_CEILING_PHOTONS_S / hvm_rate).min(1.0)
            },
            exposure_time_ratio_bound: ratio(ETPA_CROSS_SECTION_UPPER_BOUND_CM2),
            exposure_time_ratio_tightest_bound: ratio(ETPA_CROSS_SECTION_TIGHTEST_BOUND_CM2),
            exposure_time_ratio_claimed: ratio(ETPA_CROSS_SECTION_CLAIMED_MAX_CM2),
        }
    }
}

// ============================================================================
// Classical N-photon absorption
// ============================================================================

/// Classical N-photon absorption of a classical aerial image: `E = I^N`
/// (negative roundoff clamped to 0).
///
/// Sharpening without period reduction: `I^N` is brighter-line-narrower and
/// higher in contrast than `I`, but it contains no spatial period the
/// classical image lacks — the resolution stays the classical one. This is
/// what an ideal N-th-order absorber records under classical light.
pub fn n_photon_absorption_image(
    classical_aerial: &Array2<f64>,
    num_photons: usize,
) -> Result<Array2<f64>> {
    check_photons(num_photons)?;
    let n = num_photons as i32;
    Ok(classical_aerial.mapv(|i| i.max(0.0).powi(n)))
}

/// Deprecated alias kept for existing callers (the Python
/// `quantum_aerial_image` binding): classical N-photon absorption `I^N` of
/// the given classical image, after validating `params`.
///
/// `params.fidelity` has **no effect**: classical N-photon absorption
/// involves no entanglement. (The earlier `F·I^N + (1 − F)·I` mix with
/// *linear* absorption had no physical basis.) The entangled model needs
/// more than a classical image — use [`TwoBeamNPhoton`] or
/// [`noon_ideal_image`]; prefer [`n_photon_absorption_image`] for this one.
pub fn compute_quantum_aerial_image(
    classical_aerial: &Array2<f64>,
    params: &QuantumLithographyParams,
) -> Result<Array2<f64>> {
    params.validate()?;
    n_photon_absorption_image(classical_aerial, params.num_entangled_photons)
}

/// Classical image vs. its classical N-photon absorption, with the
/// resolution figures labelled honestly.
#[derive(Debug)]
pub struct QuantumComparison {
    /// Classical aerial image.
    pub classical: Array2<f64>,
    /// Classical N-photon absorption `I^N` (same period as `classical`).
    pub n_photon_absorption: Array2<f64>,
    /// Classical image contrast.
    pub classical_contrast: f64,
    /// Contrast of `I^N`.
    pub n_photon_absorption_contrast: f64,
    /// Classical Rayleigh resolution (nm) — also the resolution of `I^N`.
    pub classical_resolution_nm: f64,
    /// Rayleigh resolution of the ideal N00N limit (nm), for reference: NOT
    /// achieved by `I^N` (see [`noon_ideal_image`]).
    pub noon_limit_resolution_nm: f64,
    /// DEFAULT exposure-time ratio (a lower bound; see [`FluxBudget`]).
    pub exposure_time_ratio: f64,
}

/// Compare a classical aerial image with its classical N-photon absorption.
pub fn compare_classical_quantum(
    classical_aerial: &Array2<f64>,
    params: &QuantumLithographyParams,
) -> Result<QuantumComparison> {
    params.validate()?;
    let n_photon = n_photon_absorption_image(classical_aerial, params.num_entangled_photons)?;
    Ok(QuantumComparison {
        classical_contrast: image_contrast(classical_aerial),
        n_photon_absorption_contrast: image_contrast(&n_photon),
        classical: classical_aerial.clone(),
        n_photon_absorption: n_photon,
        classical_resolution_nm: params.classical_resolution_nm(),
        noon_limit_resolution_nm: params.quantum_resolution_nm(),
        exposure_time_ratio: params.exposure_time_ratio(),
    })
}

fn image_contrast(img: &Array2<f64>) -> f64 {
    let max = img.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min = img.iter().cloned().fold(f64::INFINITY, f64::min);
    if max + min > 0.0 {
        (max - min) / (max + min)
    } else {
        0.0
    }
}

// ============================================================================
// Two-beam fringes: ideal N00N vs. classical N-photon absorption
// ============================================================================

/// N-photon exposure of a two-beam interference pattern (closed form):
/// the ideal N00N fringe `1 + cos(N(Kx + φ))`, the classical N-photon
/// fringe `(1 + cos(Kx + φ))^N / m_N`, and their fidelity mixture, all with
/// unit mean.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TwoBeamNPhoton {
    /// Physical (vacuum) photon wavelength λ in nm.
    pub wavelength_nm: f64,
    /// Air-side half-angle θ of each beam from the normal, in degrees,
    /// in the open interval (0, 90).
    pub half_angle_deg: f64,
    /// Photons per absorption event N (1..=256).
    pub num_photons: usize,
    /// Fraction F of the exposure from the ideal N00N component; the rest
    /// is classical N-photon absorption of the classical fringe.
    pub fidelity: f64,
    /// Relative single-photon phase φ between the two beams (rad); the N00N
    /// state picks up `Nφ`, so both patterns shift by the same `−φ/K`.
    pub phase_rad: f64,
}

impl TwoBeamNPhoton {
    /// Validated constructor with zero relative phase.
    pub fn new(
        wavelength_nm: f64,
        half_angle_deg: f64,
        num_photons: usize,
        fidelity: f64,
    ) -> Result<Self> {
        let s = Self {
            wavelength_nm,
            half_angle_deg,
            num_photons,
            fidelity,
            phase_rad: 0.0,
        };
        s.validate()?;
        Ok(s)
    }

    /// Validate the parameters.
    pub fn validate(&self) -> Result<()> {
        if !(self.wavelength_nm.is_finite() && self.wavelength_nm > 0.0) {
            return Err(LithographyError::InvalidParameter {
                name: "wavelength_nm",
                value: self.wavelength_nm,
                reason: "must be positive and finite",
            });
        }
        if !(self.half_angle_deg > 0.0 && self.half_angle_deg < 90.0) {
            return Err(LithographyError::InvalidParameter {
                name: "half_angle_deg",
                value: self.half_angle_deg,
                reason: "must be in the open interval (0, 90) degrees",
            });
        }
        check_photons(self.num_photons)?;
        check_fidelity(self.fidelity)?;
        if !self.phase_rad.is_finite() {
            return Err(LithographyError::InvalidParameter {
                name: "phase_rad",
                value: self.phase_rad,
                reason: "must be finite",
            });
        }
        Ok(())
    }

    /// Classical fringe wavenumber `K = 2k sinθ = 4π sinθ / λ` (rad/nm).
    pub fn fringe_wavenumber_per_nm(&self) -> f64 {
        4.0 * std::f64::consts::PI * self.half_angle_deg.to_radians().sin() / self.wavelength_nm
    }

    /// Classical fringe period `λ / (2 sinθ)` (nm) — also the fundamental
    /// period of classical N-photon absorption.
    pub fn classical_period_nm(&self) -> f64 {
        self.wavelength_nm / (2.0 * self.half_angle_deg.to_radians().sin())
    }

    /// Ideal N00N fringe period `λ / (2N sinθ)` (nm).
    pub fn noon_period_nm(&self) -> f64 {
        self.classical_period_nm() / self.num_photons as f64
    }

    /// Phase argument `Kx + φ` at position `x_nm`.
    fn argument(&self, x_nm: f64) -> f64 {
        self.fringe_wavenumber_per_nm() * x_nm + self.phase_rad
    }

    /// Ideal N00N exposure `1 + cos(N(Kx + φ))` (unit mean).
    pub fn noon_exposure(&self, x_nm: f64) -> f64 {
        1.0 + (self.num_photons as f64 * self.argument(x_nm)).cos()
    }

    /// Classical N-photon exposure `(1 + cos(Kx + φ))^N / m_N` (unit mean,
    /// `m_N = C(2N, N)/2^N`).
    pub fn classical_exposure(&self, x_nm: f64) -> f64 {
        classical_fringe(
            self.argument(x_nm),
            self.num_photons,
            classical_fringe_mean(self.num_photons),
        )
    }

    /// Mixed exposure `F·E_NOON + (1 − F)·E_cl` (unit mean).
    pub fn exposure(&self, x_nm: f64) -> f64 {
        self.fidelity * self.noon_exposure(x_nm)
            + (1.0 - self.fidelity) * self.classical_exposure(x_nm)
    }

    /// Mixed exposure at every position in `x_nm`.
    pub fn profile(&self, x_nm: &[f64]) -> Vec<f64> {
        let mean = classical_fringe_mean(self.num_photons);
        let n = self.num_photons as f64;
        x_nm.iter()
            .map(|&x| {
                let u = self.argument(x);
                self.fidelity * (1.0 + (n * u).cos())
                    + (1.0 - self.fidelity) * classical_fringe(u, self.num_photons, mean)
            })
            .collect()
    }

    /// Exact cosine amplitude of the `j`-th harmonic `cos(j(Kx + φ))` in
    /// [`Self::exposure`]: 1 for `j = 0` (the mean),
    /// `F·[j = N] + (1 − F)·2C(2N, N−j)/C(2N, N)` for `1 ≤ j ≤ N`, and 0 above.
    pub fn harmonic_amplitude(&self, j: usize) -> f64 {
        let n = self.num_photons;
        if j == 0 {
            return 1.0;
        }
        if j > n {
            return 0.0;
        }
        // C(2N, N−j)/C(2N, N) = Π_{i=1..j} (N − i + 1)/(N + i).
        let ratio: f64 = (1..=j)
            .map(|i| (n - i + 1) as f64 / (n + i) as f64)
            .product();
        let noon = if j == n { 1.0 } else { 0.0 };
        self.fidelity * noon + (1.0 - self.fidelity) * 2.0 * ratio
    }
}

/// Mean of `(1 + cos u)^N` over a period: `C(2N, N)/2^N = Π (N + i)/(2i)`.
fn classical_fringe_mean(n: usize) -> f64 {
    (1..=n).map(|i| (n + i) as f64 / (2 * i) as f64).product()
}

/// `(1 + cos u)^N / mean`.
fn classical_fringe(u: f64, n: usize, mean: f64) -> f64 {
    (1.0 + u.cos()).max(0.0).powi(n as i32) / mean
}

// ============================================================================
// General masks: the ideal N00N limit (classical imaging at λ/N)
// ============================================================================

/// N-photon exposure settings for [`noon_ideal_image`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NoonExposure {
    /// Photons per absorption event N (1..=256).
    pub num_photons: usize,
    /// Fraction F of the exposure from the ideal N00N limit; the rest is
    /// classical N-photon absorption `I^N`.
    pub fidelity: f64,
}

impl NoonExposure {
    /// Validate the parameters.
    pub fn validate(&self) -> Result<()> {
        check_photons(self.num_photons)?;
        check_fidelity(self.fidelity)
    }
}

/// Result of [`noon_ideal_image`]. All images are clear-field normalized
/// (a clear mask images to 1 before flare).
#[derive(Debug, Clone)]
pub struct NoonImage {
    /// Classical aerial image `I_λ`.
    pub classical: Grid2D<f64>,
    /// Classical N-photon absorption `I_λ^N` (sharpening, classical period).
    pub n_photon_absorption: Grid2D<f64>,
    /// Ideal N00N limit: the same system imaged at `λ/N` (`I_{λ/N}`).
    pub noon_limit: Grid2D<f64>,
    /// Exposure `F·I_{λ/N} + (1 − F)·I_λ^N`.
    pub exposure: Grid2D<f64>,
    /// Photons per absorption event N.
    pub num_photons: usize,
    /// Fidelity F used for the mixture.
    pub fidelity: f64,
    /// Physical wavelength λ (nm).
    pub wavelength_nm: f64,
    /// Effective wavelength `λ/N` of the ideal limit (nm).
    pub effective_wavelength_nm: f64,
    /// Classical Rayleigh resolution `0.61 λ/NA` (nm).
    pub classical_resolution_nm: f64,
    /// Ideal-limit Rayleigh resolution `0.61 λ/(N NA)` (nm).
    pub noon_limit_resolution_nm: f64,
    /// Flux and exposure-time budget.
    pub flux: FluxBudget,
}

/// A source seen at `λ/factor`: same normalized pupil fill, same relative
/// spectrum.
struct WavelengthScaledSource<'a, S: ?Sized> {
    inner: &'a S,
    factor: f64,
}

impl<S: LithographySource + ?Sized> LithographySource for WavelengthScaledSource<'_, S> {
    fn wavelength_nm(&self) -> f64 {
        self.inner.wavelength_nm() / self.factor
    }

    fn bandwidth_pm(&self) -> f64 {
        self.inner.bandwidth_pm() / self.factor
    }

    fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64 {
        self.inner.intensity_at(fx_norm, fy_norm)
    }

    fn spectral_weights(&self) -> Vec<(f64, f64)> {
        self.inner
            .spectral_weights()
            .into_iter()
            .map(|(lambda, w)| (lambda / self.factor, w))
            .collect()
    }

    fn transverse_coherence(&self) -> f64 {
        self.inner.transverse_coherence()
    }
}

/// N-photon exposure of a general mask in the **ideal N00N limit**
/// (Boto et al. \[1\]): suitably prepared entangled states write the pattern
/// of classical imaging at `λ_eff = λ/N`. The same source (normalized pupil
/// fill and relative spectrum), optics (NA, pupil, aberrations as optical
/// path), mask and grid are imaged at `λ/N`; the decoherent fraction
/// contributes classical N-photon absorption of the classical image:
/// `E = F·I_{λ/N} + (1 − F)·I_λ^N`.
///
/// This is an idealization, not a simulation of state preparation: it
/// assumes the required entangled states exist for the pattern, and the
/// optic is evaluated at `λ/N` (wavelength-dependent coatings or materials
/// are therefore evaluated at the wrong wavelength; an optic that cannot be
/// evaluated at `λ/N` returns its error). Requires clear-field
/// normalization (the default) so the two terms share a scale, and a grid
/// fine enough for the `λ/N` image (`pixel ≤ λ/(4 N NA)`), else an error.
pub fn noon_ideal_image(
    source: &(impl LithographySource + ?Sized),
    optics: &(impl OpticalSystem + ?Sized),
    grid: GridConfig,
    settings: ImagingSettings,
    mask: &Mask,
    defocus_nm: f64,
    exposure: &NoonExposure,
) -> Result<NoonImage> {
    exposure.validate()?;
    if settings.normalization != ImageNormalization::ClearField {
        return Err(LithographyError::InvalidParameter {
            name: "normalization",
            value: 0.0,
            reason: "the N00N mixture needs clear-field normalized images",
        });
    }
    let n = exposure.num_photons;
    let wavelength_nm = source.wavelength_nm();
    let na = optics.na();
    let max_pixel = wavelength_nm / (4.0 * n as f64 * na);
    if grid.pixel_nm > max_pixel {
        return Err(LithographyError::InvalidParameter {
            name: "pixel_nm",
            value: grid.pixel_nm,
            reason: "grid too coarse for the λ/N image: pixel must be at most λ/(4·N·NA)",
        });
    }

    let classical_engine =
        AerialImageEngine::with_settings(source, optics, grid.clone(), settings.clone())?;
    let classical = classical_engine.compute(mask, defocus_nm);
    let scaled = WavelengthScaledSource {
        inner: source,
        factor: n as f64,
    };
    let noon_engine = AerialImageEngine::with_settings(&scaled, optics, grid, settings)?;
    let noon_limit = noon_engine.compute(mask, defocus_nm);

    let n_photon_data = n_photon_absorption_image(&classical.data, n)?;
    let f = exposure.fidelity;
    let exposure_data = &noon_limit.data * f + &n_photon_data * (1.0 - f);
    let like = |data: Array2<f64>| Grid2D {
        data,
        x_min_nm: classical.x_min_nm,
        x_max_nm: classical.x_max_nm,
        y_min_nm: classical.y_min_nm,
        y_max_nm: classical.y_max_nm,
    };

    Ok(NoonImage {
        n_photon_absorption: like(n_photon_data),
        exposure: like(exposure_data),
        classical,
        noon_limit,
        num_photons: n,
        fidelity: f,
        wavelength_nm,
        effective_wavelength_nm: wavelength_nm / n as f64,
        classical_resolution_nm: 0.61 * wavelength_nm / na,
        noon_limit_resolution_nm: 0.61 * wavelength_nm / (n as f64 * na),
        flux: FluxBudget::new(wavelength_nm, n),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_effective_wavelength() {
        let params = QuantumLithographyParams::default();
        assert_relative_eq!(params.effective_wavelength_nm(), 157.63 / 2.0);
    }

    #[test]
    fn test_quantum_resolution_better() {
        let params = QuantumLithographyParams::default();
        assert!(params.quantum_resolution_nm() < params.classical_resolution_nm());
        assert_relative_eq!(
            params.quantum_resolution_nm(),
            params.classical_resolution_nm() / 2.0,
            epsilon = 0.01
        );
    }

    /// Classical N-photon absorption sharpens (higher contrast) — the
    /// deprecated alias computes exactly that.
    #[test]
    fn test_quantum_sharpening() {
        let n = 64;
        let aerial = Array2::from_shape_fn((n, n), |(_, j)| {
            let x = (j as f64 - 32.0) / 10.0;
            0.5 + 0.3 * (-x * x / 2.0).exp()
        });
        let params = QuantumLithographyParams {
            num_entangled_photons: 2,
            fidelity: 1.0,
            ..Default::default()
        };
        let quantum = compute_quantum_aerial_image(&aerial, &params).unwrap();
        assert!(image_contrast(&quantum) > image_contrast(&aerial));
    }

    /// The alias is classical N-photon absorption I^N for every fidelity
    /// (no linear-absorption mix), equal to `n_photon_absorption_image`.
    #[test]
    fn test_alias_is_classical_n_photon_absorption_for_any_fidelity() {
        let aerial = Array2::from_shape_fn((8, 8), |(i, j)| 0.1 * (i + j) as f64 / 14.0 + 0.4);
        let reference = n_photon_absorption_image(&aerial, 3).unwrap();
        for fidelity in [0.0, 0.5, 1.0] {
            let params = QuantumLithographyParams {
                num_entangled_photons: 3,
                fidelity,
                ..Default::default()
            };
            assert_eq!(
                compute_quantum_aerial_image(&aerial, &params).unwrap(),
                reference
            );
        }
        // 0.5^2 = 0.25 at N = 2; negative roundoff is clamped to 0.
        let flat = Array2::from_elem((2, 2), 0.5);
        assert_relative_eq!(n_photon_absorption_image(&flat, 2).unwrap()[[0, 0]], 0.25);
        let neg = Array2::from_elem((1, 1), -1e-17);
        assert_eq!(n_photon_absorption_image(&neg, 3).unwrap()[[0, 0]], 0.0);
        // N = 1 is the identity.
        assert_eq!(n_photon_absorption_image(&aerial, 1).unwrap(), aerial);
    }

    /// Discrete Fourier cosine amplitudes 2·Re(c_j) of one row sampled over
    /// an integer number of periods.
    fn cosine_amplitudes(samples: &[f64], max_bin: usize) -> Vec<f64> {
        let n = samples.len() as f64;
        (0..=max_bin)
            .map(|b| {
                let mut re = 0.0;
                let mut im = 0.0;
                for (t, &v) in samples.iter().enumerate() {
                    let a = 2.0 * std::f64::consts::PI * b as f64 * t as f64 / n;
                    re += v * a.cos();
                    im -= v * a.sin();
                }
                let scale = if b == 0 { 1.0 } else { 2.0 };
                scale * (re * re + im * im).sqrt() / n
            })
            .collect()
    }

    /// Four classical periods sampled with 256 points.
    fn two_beam_samples(setup: &TwoBeamNPhoton) -> Vec<f64> {
        let span = 4.0 * setup.classical_period_nm();
        let x: Vec<f64> = (0..256).map(|t| span * t as f64 / 256.0).collect();
        setup.profile(&x)
    }

    /// Ideal N00N fringe: the spectrum peaks at N·K (bin 4N for 4 classical
    /// periods), with period λ/(2N sinθ); classical N-photon absorption keeps
    /// its fundamental at K (bin 4).
    #[test]
    fn test_two_beam_noon_peaks_at_n_k_classical_keeps_k() {
        for n in [2usize, 3, 4] {
            let noon = TwoBeamNPhoton::new(193.0, 30.0, n, 1.0).unwrap();
            assert_relative_eq!(noon.classical_period_nm(), 193.0, max_relative = 1e-12);
            assert_relative_eq!(
                noon.noon_period_nm(),
                193.0 / n as f64,
                max_relative = 1e-12
            );
            // Independent fixture: K = 4π sin30°/193 = 0.03255536428590459 rad/nm.
            assert_relative_eq!(
                noon.fringe_wavenumber_per_nm(),
                0.032_555_364_285_904_59,
                max_relative = 1e-12
            );

            let spectrum = cosine_amplitudes(&two_beam_samples(&noon), 4 * n + 4);
            let peak = (1..spectrum.len())
                .max_by(|&a, &b| spectrum[a].total_cmp(&spectrum[b]))
                .unwrap();
            assert_eq!(peak, 4 * n, "N00N fringe must peak at N·K");
            assert_relative_eq!(spectrum[4 * n], 1.0, epsilon = 1e-12);
            assert!(
                spectrum[4] < 1e-12,
                "no classical fundamental in the ideal N00N fringe"
            );

            let classical = TwoBeamNPhoton {
                fidelity: 0.0,
                ..noon
            };
            let spectrum = cosine_amplitudes(&two_beam_samples(&classical), 4 * n + 4);
            let peak = (1..spectrum.len())
                .max_by(|&a, &b| spectrum[a].total_cmp(&spectrum[b]))
                .unwrap();
            assert_eq!(
                peak, 4,
                "classical N-photon absorption keeps the fundamental at K"
            );
            // Fixture: a_1 = 2N/(N+1) (4/3, 1.5, 1.6 for N = 2, 3, 4).
            assert_relative_eq!(
                spectrum[4],
                2.0 * n as f64 / (n as f64 + 1.0),
                epsilon = 1e-12
            );
        }
    }

    /// Harmonic amplitudes of the mixture match the closed form and an
    /// independent numpy fixture (N = 3, F = 0.7: 0.45, 0.18, 0.73).
    #[test]
    fn test_two_beam_mixture_harmonics_fixture() {
        let setup = TwoBeamNPhoton::new(193.0, 30.0, 3, 0.7).unwrap();
        let spectrum = cosine_amplitudes(&two_beam_samples(&setup), 16);
        assert_relative_eq!(spectrum[0], 1.0, epsilon = 1e-12); // unit mean
        for (j, expected) in [(1usize, 0.45), (2, 0.18), (3, 0.73)] {
            assert_relative_eq!(setup.harmonic_amplitude(j), expected, epsilon = 1e-12);
            assert_relative_eq!(spectrum[4 * j], expected, epsilon = 1e-12);
        }
        assert_eq!(setup.harmonic_amplitude(4), 0.0);
        assert!(spectrum[16] < 1e-12);
        // Classical N-photon fringe mean m_N = C(2N, N)/2^N: 1.5, 2.5, 4.375.
        for (n, m) in [(2usize, 1.5), (3, 2.5), (4, 4.375)] {
            assert_relative_eq!(classical_fringe_mean(n), m, epsilon = 1e-12);
        }
    }

    /// F = 0 is exactly classical N-photon absorption of the classical
    /// fringe; N = 1 is the classical fringe for any F.
    #[test]
    fn test_two_beam_limits() {
        let classical = TwoBeamNPhoton {
            phase_rad: 0.4,
            ..TwoBeamNPhoton::new(248.0, 20.0, 3, 0.0).unwrap()
        };
        let k = classical.fringe_wavenumber_per_nm();
        for t in 0..20 {
            let x = 7.3 * t as f64;
            let expected = (1.0 + (k * x + 0.4).cos()).powi(3) / 2.5;
            assert_relative_eq!(classical.exposure(x), expected, epsilon = 1e-12);
            assert_eq!(classical.exposure(x), classical.classical_exposure(x));
        }
        let single = TwoBeamNPhoton::new(248.0, 20.0, 1, 0.37).unwrap();
        for t in 0..20 {
            let x = 5.1 * t as f64;
            let expected = 1.0 + (single.fringe_wavenumber_per_nm() * x).cos();
            assert_relative_eq!(single.exposure(x), expected, epsilon = 1e-12);
        }
    }

    #[test]
    fn test_two_beam_rejects_invalid() {
        assert!(TwoBeamNPhoton::new(0.0, 30.0, 2, 1.0).is_err());
        assert!(TwoBeamNPhoton::new(193.0, 0.0, 2, 1.0).is_err());
        assert!(TwoBeamNPhoton::new(193.0, 90.0, 2, 1.0).is_err());
        assert!(TwoBeamNPhoton::new(193.0, 30.0, 0, 1.0).is_err());
        assert!(TwoBeamNPhoton::new(193.0, 30.0, 257, 1.0).is_err());
        assert!(TwoBeamNPhoton::new(193.0, 30.0, 2, 1.5).is_err());
        assert!(TwoBeamNPhoton::new(193.0, 30.0, 2, f64::NAN).is_err());
        let bad_phase = TwoBeamNPhoton {
            phase_rad: f64::INFINITY,
            ..TwoBeamNPhoton::new(193.0, 30.0, 2, 1.0).unwrap()
        };
        assert!(bad_phase.validate().is_err());
    }

    /// Flux budget from the entangled-source constants, against independent
    /// fixtures: at 157.63 nm the HVM rate is 5.3166e17 photons/s, so the
    /// relative flux is ≤ 1.8809e-5; the DEFAULT exposure-time ratio is
    /// 0.1·N·0.67 / (1e-23·1e13·0.03) = 4.467e10 for N = 2.
    #[test]
    fn test_flux_budget_tied_to_entangled_source_constants() {
        let params = QuantumLithographyParams::default();
        let budget = params.flux_budget();
        assert_relative_eq!(
            budget.photon_energy_ev,
            7.865_520_078_665_229,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            budget.hvm_photon_rate_per_s,
            5.316_636_456_413_904e17,
            max_relative = 1e-9
        );
        assert_relative_eq!(
            budget.relative_flux,
            1.880_888_430_491_831_4e-5,
            max_relative = 1e-9
        );
        assert_relative_eq!(params.relative_flux(), budget.relative_flux);
        assert_relative_eq!(
            budget.exposure_time_ratio_bound,
            4.466_666_666_666_667e10,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            budget.exposure_time_ratio_tightest_bound,
            4.466_666_666_666_667e12,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            budget.exposure_time_ratio_claimed,
            4.466_666_666_666_667e4,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            params.exposure_time_ratio(),
            budget.exposure_time_ratio_bound
        );
        // Linear in N (the crossover flux caps photons, so N-photon events are
        // N times rarer): N = 3 -> 6.7e10.
        assert_relative_eq!(
            exposure_time_ratio_for_cross_section(3, ETPA_CROSS_SECTION_UPPER_BOUND_CM2),
            6.7e10,
            max_relative = 1e-12
        );
        // EUV photons are 11.7x more energetic -> fewer HVM photons/s.
        assert_relative_eq!(
            FluxBudget::new(13.5, 2).relative_flux,
            2.196_181_061_469_832_5e-4,
            max_relative = 1e-9
        );
        // N = 1 is classical light.
        let classical = FluxBudget::new(157.63, 1);
        assert_eq!(classical.relative_flux, 1.0);
        assert_eq!(classical.exposure_time_ratio_bound, 1.0);
        assert!(exposure_time_ratio_for_cross_section(2, 0.0).is_infinite());
    }

    #[test]
    fn test_exposure_time_very_long() {
        let params = QuantumLithographyParams {
            num_entangled_photons: 2,
            ..Default::default()
        };
        assert!(params.exposure_time_ratio() > 1e10);
    }

    #[test]
    fn test_compare_classical_quantum_labels_resolution_honestly() {
        let aerial = Array2::from_shape_fn((4, 16), |(_, j)| 0.5 + 0.4 * (j as f64 * 0.7).cos());
        let params = QuantumLithographyParams::default();
        let cmp = compare_classical_quantum(&aerial, &params).unwrap();
        assert_eq!(
            cmp.n_photon_absorption,
            n_photon_absorption_image(&aerial, 2).unwrap()
        );
        assert!(cmp.n_photon_absorption_contrast > cmp.classical_contrast);
        assert_relative_eq!(
            cmp.noon_limit_resolution_nm,
            cmp.classical_resolution_nm / 2.0
        );
        assert!(compare_classical_quantum(
            &aerial,
            &QuantumLithographyParams {
                fidelity: 2.0,
                ..params
            }
        )
        .is_err());
    }

    fn small_system() -> (
        crate::source::VuvSource,
        crate::optics::ProjectionOptics,
        Mask,
        GridConfig,
    ) {
        let source = crate::source::VuvSource::f2_laser(0.5).unwrap();
        let optics = crate::optics::ProjectionOptics::new(0.75).unwrap();
        // 100 nm pitch: below the classical cutoff λ/(NA(1+σ)) = 140 nm at
        // 157.63 nm, above the λ/2 cutoff of 70 nm. 64 × 3.125 nm = 2 pitches.
        let mask = Mask::line_space(50.0, 100.0).unwrap();
        let grid = GridConfig {
            size: 64,
            pixel_nm: 3.125,
        };
        (source, optics, mask, grid)
    }

    fn settings() -> ImagingSettings {
        ImagingSettings {
            max_kernels: 12,
            ..ImagingSettings::default()
        }
    }

    /// At N = 1 the λ/N engine is the classical engine: identical images.
    #[test]
    fn test_noon_ideal_image_n1_reproduces_classical_exactly() {
        let (source, optics, _, grid) = small_system();
        let mask = Mask::line_space(100.0, 200.0).unwrap();
        let result = noon_ideal_image(
            &source,
            &optics,
            grid,
            settings(),
            &mask,
            0.0,
            &NoonExposure {
                num_photons: 1,
                fidelity: 0.6,
            },
        )
        .unwrap();
        assert_eq!(result.noon_limit.data, result.classical.data);
        assert_eq!(result.n_photon_absorption.data, result.classical.data);
        for (e, c) in result
            .exposure
            .data
            .iter()
            .zip(result.classical.data.iter())
        {
            assert_relative_eq!(*e, *c, epsilon = 1e-12);
        }
        assert!(image_contrast(&result.classical.data) > 0.1);
    }

    /// The ideal N00N limit resolves a pitch the classical system cannot;
    /// classical N-photon absorption cannot (sharpening adds no resolution);
    /// F = 0 is exactly classical N-photon absorption.
    #[test]
    fn test_noon_limit_resolves_what_n_photon_absorption_cannot() {
        let (source, optics, mask, grid) = small_system();
        let ideal = noon_ideal_image(
            &source,
            &optics,
            grid.clone(),
            settings(),
            &mask,
            0.0,
            &NoonExposure {
                num_photons: 2,
                fidelity: 1.0,
            },
        )
        .unwrap();
        let classical_contrast = image_contrast(&ideal.classical.data);
        let n_photon_contrast = image_contrast(&ideal.n_photon_absorption.data);
        let noon_contrast = image_contrast(&ideal.noon_limit.data);
        assert!(
            classical_contrast < 1e-6,
            "pitch unresolved classically: {classical_contrast}"
        );
        assert!(
            n_photon_contrast < 1e-6,
            "I^N cannot resolve it: {n_photon_contrast}"
        );
        assert!(
            noon_contrast > 0.2,
            "λ/2 limit resolves it: {noon_contrast}"
        );
        assert_eq!(ideal.exposure.data, ideal.noon_limit.data);
        assert_relative_eq!(
            ideal.effective_wavelength_nm,
            157.63 / 2.0,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            ideal.noon_limit_resolution_nm,
            ideal.classical_resolution_nm / 2.0
        );
        assert!(ideal.flux.exposure_time_ratio_bound > 1e10);

        let decohered = noon_ideal_image(
            &source,
            &optics,
            grid,
            settings(),
            &mask,
            0.0,
            &NoonExposure {
                num_photons: 2,
                fidelity: 0.0,
            },
        )
        .unwrap();
        assert_eq!(decohered.exposure.data, decohered.n_photon_absorption.data);
    }

    /// Scale invariance of diffraction: imaging at λ/N with every length
    /// scaled by 1/N is the classical image. The N = 2 ideal limit of a
    /// 100 nm pitch on a 200 nm field equals the classical image of a 200 nm
    /// pitch on a 400 nm field, pixel for pixel.
    #[test]
    fn test_noon_limit_is_classical_imaging_at_lambda_over_n() {
        let (source, optics, mask, grid) = small_system();
        let ideal = noon_ideal_image(
            &source,
            &optics,
            grid,
            settings(),
            &mask,
            0.0,
            &NoonExposure {
                num_photons: 2,
                fidelity: 1.0,
            },
        )
        .unwrap();
        let scaled_grid = GridConfig {
            size: 64,
            pixel_nm: 6.25,
        };
        let engine =
            AerialImageEngine::with_settings(&source, &optics, scaled_grid, settings()).unwrap();
        let scaled = engine.compute(&Mask::line_space(100.0, 200.0).unwrap(), 0.0);
        for (a, b) in ideal.noon_limit.data.iter().zip(scaled.data.iter()) {
            assert_relative_eq!(*a, *b, epsilon = 1e-9);
        }
        assert!(image_contrast(&scaled.data) > 0.2);
    }

    #[test]
    fn test_noon_ideal_image_rejects_invalid() {
        let (source, optics, mask, grid) = small_system();
        let ok = NoonExposure {
            num_photons: 2,
            fidelity: 1.0,
        };
        let run = |grid: GridConfig, settings: ImagingSettings, exposure: NoonExposure| {
            noon_ideal_image(&source, &optics, grid, settings, &mask, 0.0, &exposure)
        };
        assert!(run(
            grid.clone(),
            settings(),
            NoonExposure {
                num_photons: 0,
                ..ok
            }
        )
        .is_err());
        assert!(run(
            grid.clone(),
            settings(),
            NoonExposure {
                fidelity: -0.1,
                ..ok
            }
        )
        .is_err());
        // λ/(4·N·NA) = 157.63/6 = 26.3 nm: a 40 nm pixel cannot hold the λ/2 image.
        let coarse = GridConfig {
            size: 64,
            pixel_nm: 40.0,
        };
        assert!(run(coarse, settings(), ok).is_err());
        let absolute = ImagingSettings {
            normalization: ImageNormalization::Absolute,
            ..settings()
        };
        assert!(run(grid, absolute, ok).is_err());
    }

    #[test]
    fn test_validate_rejects_zero_photons() {
        let params = QuantumLithographyParams {
            num_entangled_photons: 0,
            ..Default::default()
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_bad_fidelity() {
        for fidelity in [1.5, -0.1, f64::NAN] {
            let params = QuantumLithographyParams {
                fidelity,
                ..Default::default()
            };
            assert!(params.validate().is_err());
        }
    }
}
