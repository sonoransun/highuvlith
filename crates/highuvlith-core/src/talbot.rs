//! Talbot self-imaging lithography (coherent, displacement, achromatic) and
//! two-grating EUV interference lithography.
//!
//! A periodic transmission grating under coherent plane-wave illumination
//! re-images itself in free space at integer multiples of the Talbot length,
//! with a copy shifted by half a period at half the Talbot length and
//! sub-period "fractional" images in between — the Talbot carpet. Proximity
//! lithography uses this to print periodic arrays without a projection lens:
//!
//! - **Coherent Talbot printing** puts the wafer at one fixed gap; the
//!   printed image depends sensitively on the gap (its depth of field is a
//!   small fraction of the Talbot length).
//! - **Displacement Talbot lithography (DTL)** scans the gap over (an integer
//!   number of) Talbot lengths during the exposure; the time-integrated image
//!   is gap-independent and, for a 1D grating, has half the mask period.
//! - **Achromatic Talbot lithography (ATL)** uses a broadband source instead:
//!   beyond the achromatic distance the spectral average washes out the Talbot
//!   revivals and leaves the same stationary image.
//! - **Two-grating EUV interference lithography (EUV-IL)** overlaps the +m and
//!   −m diffraction orders of two transmission gratings of period `p`; the
//!   fringes have period `p/(2m)` independent of the wavelength.
//!
//! A grating is described by its Fourier orders `c_nm` (thin-mask Kirchhoff
//! transmission). Each order is a plane wave of transverse spatial frequency
//! `f = (n/p_x, m/p_y)` propagated with the exact scalar angular-spectrum
//! transfer function or its paraxial (Fresnel) approximation. The transfer
//! function depends on `|f|` only, so orders are grouped into *shells* of equal
//! `|f|`, and every incoherent average (gap scan, source spectrum) reduces to a
//! small Hermitian matrix of shell-pair weights `W_st = ⟨P_s P_t*⟩`: the image
//! is `I(r) = Σ_st W_st U_s(r) U_t(r)*`, where `U_s` is the coherent sum of the
//! shell's orders at the mask. Intensities are normalized to the incident
//! plane wave (a clear mask gives `I = 1`).
//!
//! # Key equations
//!
//! ```text
//!   grating      t(x, y) = Σ_nm c_nm · exp(2πi (n x/p_x + m y/p_y))
//!   binary 1D    line of width f·p (transmission t_l) centred at x = 0, space t_s:
//!                c_0 = t_s + (t_l − t_s)·f,   c_n = (t_l − t_s)·f·sinc(n f),
//!                sinc(u) = sin(πu)/(πu)
//!   hole array   c_nm = (π r²/A) · S_nm · 2 J₁(2π r |f|) / (2π r |f|),
//!                A = unit-cell area, S_nm = Σ_holes exp(−2πi f·r_h)
//!   propagation  U(r, z) = Σ c_nm exp(2πi f·r) · exp(i (k_z − k) z)   (e^{ikz} dropped)
//!                exact:     k_z − k = −2π|f|² / (n/λ + sqrt((n/λ)² − |f|²))
//!                           (|f| > n/λ: evanescent, amplitude e^{−2π sqrt(|f|² − (n/λ)²) z})
//!                paraxial:  k_z − k = −π λ |f|² / n
//!   Talbot       z_T = 2 p²/λ (paraxial);
//!                z_T = λ / (1 − sqrt(1 − λ²/p²)) = (p²/λ)(1 + sqrt(1 − λ²/p²))
//!                (exact rephasing of the 0 and ±1 orders)
//!   DTL          ⟨I⟩_gap∈[g, g+L]:  W_st = P_s P_t*(g + L/2) · sinc(δ_st L / 2π),
//!                δ_st = ∂(phase_s − phase_t)/∂g;  L → ∞ keeps only s = t:
//!                I_DTL = Σ_shells |U_s|²;  1D: Σ|c_n|² + 2 Re Σ_{n>0} c_n c_{−n}* e^{4πinx/p}
//!   ATL          flat-top band Δλ (paraxial): ⟨e^{−iπλzD/p²}⟩ = e^{−iπλ₀zD/p²} sinc(D Δλ z / 2p²),
//!                D = n² − n'², first zero of the 0/±1 term at z_A = 2 p²/Δλ;
//!                Gaussian band (rms σ_λ): factor exp(−(π σ_λ z D / p²)² / 2)
//!   EUV-IL       sin θ = mλ/p,  Λ = λ / (2 sin θ) = p / (2m),  V = 2 sqrt(I₁ I₂) / (I₁ + I₂)
//! ```
//!
//! Inside a resist of real index `n_r` every order continues with its own
//! `k_z` computed at `λ/n_r`, and its amplitude decays as
//! `exp(−α z / (2 cos θ))` along its slanted path (the convention of
//! [`crate::interference`]); no Fresnel transmission coefficients are applied
//! at the gap/resist interface.
//!
//! # Model status
//!
//! Simplified (documented approximations): thin-mask (Kirchhoff) grating
//! transmission — no mask-3D / rigorous EMF diffraction; scalar fields (the
//! EUV-IL conversion to [`crate::interference::InterferenceSetup`] builds TE,
//! s-polarized beams, for which scalar and vector two-beam interference
//! coincide); normally incident,
//! perfectly spatially coherent illumination (no source angular spread, which
//! blurs real DTL/ATL images); wavelength-independent Fourier coefficients
//! (the true phase step of a phase grating scales ∝ 1/λ); no Fresnel
//! coefficients or back-reflection at the resist; infinite periodic mask (no
//! finite-aperture walk-off of high orders at large gaps). Displacement and
//! paraxial achromatic averages are closed forms; exact-propagation spectral
//! averages use piecewise-linear-phase (sinc-weighted) spectral bins. The
//! intensity volumes feed [`crate::interference::expose`] and the volumetric
//! development tiers in [`crate::volumetric`].
//!
//! # References
//!
//! - H. F. Talbot (1836) and Lord Rayleigh (1881) — discovery and the paraxial
//!   self-imaging distance `2p²/λ` (classical optics provenance).
//! - J. W. Goodman, *Introduction to Fourier Optics* — angular-spectrum and
//!   Fresnel transfer functions.
//! - H. H. Solak and co-workers (Paul Scherrer Institut) — achromatic and
//!   displacement Talbot lithography and EUV interference lithography with
//!   transmission gratings (method provenance; the formulas above are derived
//!   here from the angular-spectrum model).

use std::f64::consts::PI;

use ndarray::Array2;
use serde::{Deserialize, Serialize};

use crate::error::{LithographyError, Result};
use crate::interference::{InterferenceSetup, PlaneWave};
use crate::types::{Complex64, Grid2D, Grid3D};

/// Largest supported |order index| per axis (guards runaway allocations).
const MAX_ORDER_LIMIT: usize = 512;

/// Relative tolerance for grouping orders of equal `|f|` into one shell.
const SHELL_TOLERANCE: f64 = 1e-10;

fn invalid(name: &'static str, value: f64, reason: &'static str) -> LithographyError {
    LithographyError::InvalidParameter {
        name,
        value,
        reason,
    }
}

fn require_positive(name: &'static str, value: f64) -> Result<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(invalid(name, value, "must be positive and finite"))
    }
}

fn require_non_negative(name: &'static str, value: f64) -> Result<()> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(invalid(name, value, "must be non-negative and finite"))
    }
}

fn require_max_order(max_order: usize) -> Result<()> {
    if max_order > MAX_ORDER_LIMIT {
        Err(invalid(
            "max_order",
            max_order as f64,
            "must be at most 512 orders per axis",
        ))
    } else {
        Ok(())
    }
}

// ============================================================================
// Special functions and closed-form lengths
// ============================================================================

/// Normalized sinc, `sin(πu)/(πu)` with `sinc(0) = 1`.
pub fn sinc(u: f64) -> f64 {
    let x = PI * u;
    if x.abs() < 1e-8 {
        1.0 - x * x / 6.0
    } else {
        x.sin() / x
    }
}

/// Bessel function of the first kind of order one, `J₁(x)`.
///
/// Evaluated from Bessel's integral `J₁(x) = (1/2π)∫₀^{2π} cos(τ − x sin τ) dτ`
/// with the trapezoidal rule, which converges exponentially for this periodic
/// analytic integrand once the node count exceeds `|x|` by a margin (the node
/// count grows with `|x|` accordingly).
pub fn bessel_j1(x: f64) -> f64 {
    let nodes = 2 * (x.abs().ceil() as usize) + 48;
    let sum: f64 = (0..nodes)
        .map(|j| {
            let tau = 2.0 * PI * j as f64 / nodes as f64;
            (tau - x * tau.sin()).cos()
        })
        .sum();
    sum / nodes as f64
}

/// The circular-aperture ("Airy") form factor `2 J₁(x)/x`, equal to 1 at `x = 0`.
fn airy_factor(x: f64) -> f64 {
    if x.abs() < 1e-8 {
        1.0 - x * x / 8.0
    } else {
        2.0 * bessel_j1(x) / x
    }
}

/// Paraxial Talbot (self-imaging) length `z_T = 2p²/λ` in nm.
///
/// `wavelength_nm` is the wavelength in the propagation medium (`λ_vac / n`).
pub fn talbot_length_paraxial_nm(period_nm: f64, wavelength_nm: f64) -> f64 {
    2.0 * period_nm * period_nm / wavelength_nm
}

/// Exact (non-paraxial) rephasing length of the 0 and ±1 orders,
/// `z_T = λ / (1 − sqrt(1 − λ²/p²)) = (p²/λ)(1 + sqrt(1 − λ²/p²))`, in nm.
///
/// Returns `None` when `λ ≥ p` (no propagating first order). The field is
/// exactly periodic in z with this length only when the 0 and ±1 orders are
/// the only propagating ones (`p < 2λ`, or a sinusoidal grating); with more
/// propagating orders the non-paraxial field is only quasi-periodic. Tends to
/// the paraxial `2p²/λ` as `λ/p → 0` (relative correction `−λ²/(4p²)`).
pub fn talbot_length_exact_nm(period_nm: f64, wavelength_nm: f64) -> Option<f64> {
    let e = wavelength_nm / period_nm;
    if !(e > 0.0 && e < 1.0) {
        return None;
    }
    Some(period_nm * period_nm / wavelength_nm * (1.0 + (1.0 - e * e).sqrt()))
}

/// Achromatic Talbot distance `z_A = 2p²/Δλ` in nm: the first zero of the
/// spectrally averaged 0/±1 cross term for a flat-top band of full width
/// `bandwidth_nm` (paraxial). Beyond it the ATL image is stationary.
pub fn achromatic_distance_nm(period_nm: f64, bandwidth_nm: f64) -> f64 {
    2.0 * period_nm * period_nm / bandwidth_nm
}

/// Fringe period `p/(2m)` of two-grating interference of the ±m orders of
/// gratings with period `grating_period_nm` (independent of wavelength).
pub fn two_grating_fringe_period_nm(grating_period_nm: f64, order: u32) -> f64 {
    grating_period_nm / (2.0 * order.max(1) as f64)
}

/// Dominant spatial period (nm) of a sampled 1D profile: the period of the
/// largest non-DC DFT bin of `profile − mean`. `None` for fewer than four
/// samples, a non-positive pixel, or a flat profile. Exact when the sampled
/// span is an integer number of periods.
pub fn dominant_period_nm(profile: &[f64], pixel_nm: f64) -> Option<f64> {
    let n = profile.len();
    if n < 4 || pixel_nm.is_nan() || pixel_nm <= 0.0 {
        return None;
    }
    let mean = profile.iter().sum::<f64>() / n as f64;
    let scale = profile
        .iter()
        .map(|v| (v - mean).abs())
        .fold(0.0_f64, f64::max);
    if scale <= 1e-14 * (1.0 + mean.abs()) {
        return None;
    }
    let mut best = (0usize, 0.0_f64);
    for k in 1..=n / 2 {
        let (mut re, mut im) = (0.0, 0.0);
        for (j, &v) in profile.iter().enumerate() {
            let phase = -2.0 * PI * ((k * j) % n) as f64 / n as f64;
            re += (v - mean) * phase.cos();
            im += (v - mean) * phase.sin();
        }
        let power = re * re + im * im;
        if power > best.1 {
            best = (k, power);
        }
    }
    (best.0 > 0).then(|| n as f64 * pixel_nm / best.0 as f64)
}

// ============================================================================
// Gratings
// ============================================================================

/// One Fourier (diffraction) order of a grating.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiffractionOrder {
    /// Order index along x.
    pub n: i32,
    /// Order index along y (0 for 1D gratings).
    pub m: i32,
    /// Complex Fourier coefficient `c_nm`: the order's amplitude just behind
    /// the grating for a unit-amplitude, normally incident plane wave.
    pub amplitude: Complex64,
}

impl DiffractionOrder {
    /// Diffraction efficiency `|c_nm|²` (fraction of the incident power).
    pub fn efficiency(&self) -> f64 {
        self.amplitude.norm_sqr()
    }
}

/// A thin periodic transmission grating described by its Fourier orders.
#[derive(Debug, Clone, PartialEq)]
pub struct Grating {
    /// Period along x in nm.
    pub period_x_nm: f64,
    /// Period along y in nm; `None` for a 1D (line) grating invariant in y.
    pub period_y_nm: Option<f64>,
    /// Retained Fourier orders (higher orders are truncated).
    pub orders: Vec<DiffractionOrder>,
}

impl Grating {
    /// General binary 1D grating: a "line" of width `duty · p` centred at
    /// `x = 0` with complex amplitude transmission `t_line`, and the rest of
    /// each period with `t_space`. Orders `|n| ≤ max_order` are retained:
    /// `c_0 = t_s + (t_l − t_s)·f`, `c_n = (t_l − t_s)·f·sinc(n f)`.
    pub fn binary(
        period_nm: f64,
        duty: f64,
        t_line: Complex64,
        t_space: Complex64,
        max_order: usize,
    ) -> Result<Self> {
        require_positive("period_nm", period_nm)?;
        if !(0.0..=1.0).contains(&duty) {
            return Err(invalid("duty", duty, "must lie in [0, 1]"));
        }
        require_max_order(max_order)?;
        let dt = t_line - t_space;
        let n_max = max_order as i32;
        let orders = (-n_max..=n_max)
            .map(|n| {
                let amplitude = if n == 0 {
                    t_space + dt * duty
                } else {
                    dt * (duty * sinc(n as f64 * duty))
                };
                DiffractionOrder { n, m: 0, amplitude }
            })
            .collect();
        Ok(Self {
            period_x_nm: period_nm,
            period_y_nm: None,
            orders,
        })
    }

    /// Binary amplitude grating: an open slit of width `open_fraction · p`
    /// (amplitude 1) centred at `x = 0`, opaque elsewhere. `c_0 = f`,
    /// `c_n = f·sinc(n f)`.
    pub fn binary_amplitude(period_nm: f64, open_fraction: f64, max_order: usize) -> Result<Self> {
        Self::binary(
            period_nm,
            open_fraction,
            Complex64::new(1.0, 0.0),
            Complex64::new(0.0, 0.0),
            max_order,
        )
    }

    /// Binary phase grating: a fraction `duty` of each period carries phase
    /// `phase_rad`, the rest phase 0, both fully transmitting.
    /// `c_0 = 1 + (e^{iφ} − 1)f`, `c_n = (e^{iφ} − 1)·f·sinc(n f)`; a π step at
    /// 50 % duty cycle suppresses the zeroth order.
    pub fn binary_phase(
        period_nm: f64,
        duty: f64,
        phase_rad: f64,
        max_order: usize,
    ) -> Result<Self> {
        if !phase_rad.is_finite() {
            return Err(invalid("phase_rad", phase_rad, "must be finite"));
        }
        Self::binary(
            period_nm,
            duty,
            Complex64::from_polar(1.0, phase_rad),
            Complex64::new(1.0, 0.0),
            max_order,
        )
    }

    /// Sinusoidal amplitude grating `t(x) = mean + modulation·cos(2πx/p)`:
    /// only the orders 0 (`mean`) and ±1 (`modulation/2`).
    pub fn sinusoidal_amplitude(period_nm: f64, mean: f64, modulation: f64) -> Result<Self> {
        require_positive("period_nm", period_nm)?;
        if !mean.is_finite() || !modulation.is_finite() {
            return Err(invalid("modulation", modulation, "must be finite"));
        }
        let half = Complex64::new(0.5 * modulation, 0.0);
        let orders = vec![
            DiffractionOrder {
                n: -1,
                m: 0,
                amplitude: half,
            },
            DiffractionOrder {
                n: 0,
                m: 0,
                amplitude: Complex64::new(mean, 0.0),
            },
            DiffractionOrder {
                n: 1,
                m: 0,
                amplitude: half,
            },
        ];
        Ok(Self {
            period_x_nm: period_nm,
            period_y_nm: None,
            orders,
        })
    }

    /// 1D grating from explicit Fourier coefficients `(n, c_n)`.
    pub fn from_coefficients(period_nm: f64, coefficients: &[(i32, Complex64)]) -> Result<Self> {
        require_positive("period_nm", period_nm)?;
        if coefficients.is_empty() {
            return Err(invalid(
                "coefficients",
                0.0,
                "at least one Fourier order is required",
            ));
        }
        let orders = coefficients
            .iter()
            .map(|&(n, amplitude)| DiffractionOrder { n, m: 0, amplitude })
            .collect();
        Ok(Self {
            period_x_nm: period_nm,
            period_y_nm: None,
            orders,
        })
    }

    /// 1D grating from `samples` of the complex transmission over one period,
    /// sample `j` covering `[−p/2 + j·p/N, −p/2 + (j+1)·p/N)`.
    ///
    /// The coefficients are the exact Fourier coefficients of the
    /// piecewise-constant profile: `c_n = sinc(n/N)·(1/N)·Σ_j t_j e^{−2πi n x_j/p}`
    /// with `x_j` the sample centres.
    pub fn from_profile(samples: &[Complex64], period_nm: f64, max_order: usize) -> Result<Self> {
        let cell = Array2::from_shape_vec((1, samples.len()), samples.to_vec())
            .map_err(|e| LithographyError::NumericalError(e.to_string()))?;
        let mut grating = Self::from_unit_cell(&cell, period_nm, period_nm, max_order)?;
        grating.period_y_nm = None;
        grating.orders.retain(|o| o.m == 0);
        Ok(grating)
    }

    /// 2D separable grating `t(x, y) = t_x(x)·t_y(y)` from two 1D gratings
    /// (`gy`'s period becomes the y period): `c_nm = c_n^x · c_m^y`.
    pub fn separable(gx: &Grating, gy: &Grating) -> Result<Self> {
        if !gx.is_1d() || !gy.is_1d() {
            return Err(invalid("grating", 0.0, "separable() needs two 1D gratings"));
        }
        let mut orders = Vec::with_capacity(gx.orders.len() * gy.orders.len());
        for oy in &gy.orders {
            for ox in &gx.orders {
                orders.push(DiffractionOrder {
                    n: ox.n,
                    m: oy.n,
                    amplitude: ox.amplitude * oy.amplitude,
                });
            }
        }
        Ok(Self {
            period_x_nm: gx.period_x_nm,
            period_y_nm: Some(gy.period_x_nm),
            orders,
        })
    }

    /// 2D grating from a sampled unit cell of complex transmission.
    ///
    /// `cell[[i, j]]` is row `i` (y) and column `j` (x); the cell spans
    /// `[−p_x/2, p_x/2) × [−p_y/2, p_y/2)` with pixel centres at
    /// `−p/2 + (j + ½)·p/N`. The coefficients are the exact Fourier
    /// coefficients of the pixelated (piecewise-constant) cell — the DFT
    /// times the pixel form factor `sinc(n/N_x)·sinc(m/N_y)` — so an
    /// antialiased (area-coverage) raster of a smooth shape converges to the
    /// shape's coefficients as the raster is refined. Orders `|n| ≤ max_order`
    /// and `|m| ≤ round(max_order · p_y/p_x)` are retained.
    pub fn from_unit_cell(
        cell: &Array2<Complex64>,
        period_x_nm: f64,
        period_y_nm: f64,
        max_order: usize,
    ) -> Result<Self> {
        require_positive("period_x_nm", period_x_nm)?;
        require_positive("period_y_nm", period_y_nm)?;
        require_max_order(max_order)?;
        let (ny_c, nx_c) = cell.dim();
        if nx_c == 0 || ny_c == 0 {
            return Err(invalid("cell", 0.0, "unit cell must be non-empty"));
        }
        let n_max = max_order as i32;
        let m_max = ((max_order as f64) * period_y_nm / period_x_nm).round() as i32;
        let m_max = if ny_c == 1 { 0 } else { m_max };

        // x-transform of every row for each retained n.
        let x_centres: Vec<f64> = (0..nx_c)
            .map(|j| -0.5 * period_x_nm + (j as f64 + 0.5) * period_x_nm / nx_c as f64)
            .collect();
        let y_centres: Vec<f64> = (0..ny_c)
            .map(|i| -0.5 * period_y_nm + (i as f64 + 0.5) * period_y_nm / ny_c as f64)
            .collect();
        let n_count = (2 * n_max + 1) as usize;
        let mut row_transform = vec![Complex64::new(0.0, 0.0); ny_c * n_count];
        for (i, row) in cell.outer_iter().enumerate() {
            for (k, n) in (-n_max..=n_max).enumerate() {
                let mut acc = Complex64::new(0.0, 0.0);
                for (&t, &x) in row.iter().zip(&x_centres) {
                    acc += t * Complex64::from_polar(1.0, -2.0 * PI * n as f64 * x / period_x_nm);
                }
                row_transform[i * n_count + k] = acc;
            }
        }

        let norm = 1.0 / (nx_c * ny_c) as f64;
        let mut orders = Vec::with_capacity(n_count * (2 * m_max + 1) as usize);
        for m in -m_max..=m_max {
            let form_y = sinc(m as f64 / ny_c as f64);
            for (k, n) in (-n_max..=n_max).enumerate() {
                let mut acc = Complex64::new(0.0, 0.0);
                for (i, &y) in y_centres.iter().enumerate() {
                    acc += row_transform[i * n_count + k]
                        * Complex64::from_polar(1.0, -2.0 * PI * m as f64 * y / period_y_nm);
                }
                let amplitude = acc * (norm * form_y * sinc(n as f64 / nx_c as f64));
                orders.push(DiffractionOrder { n, m, amplitude });
            }
        }
        Ok(Self {
            period_x_nm,
            period_y_nm: Some(period_y_nm),
            orders,
        })
    }

    /// Square lattice (period `period_nm`) of circular holes of diameter
    /// `hole_diameter_nm` (amplitude 1) in an opaque screen, from the exact
    /// Airy form factor: `c_nm = (π r²/p²)·2J₁(2πr|f|)/(2πr|f|)`. Orders are
    /// truncated on a circle, `|f| ≤ max_order/p`, which keeps the retained
    /// set invariant under the lattice's rotations.
    pub fn hole_array_square(
        period_nm: f64,
        hole_diameter_nm: f64,
        max_order: usize,
    ) -> Result<Self> {
        require_positive("period_nm", period_nm)?;
        require_positive("hole_diameter_nm", hole_diameter_nm)?;
        if hole_diameter_nm > period_nm {
            return Err(invalid(
                "hole_diameter_nm",
                hole_diameter_nm,
                "holes must not overlap (diameter ≤ period)",
            ));
        }
        require_max_order(max_order)?;
        let r = 0.5 * hole_diameter_nm;
        let fill = PI * r * r / (period_nm * period_nm);
        let n_max = max_order as i32;
        let mut orders = Vec::new();
        for m in -n_max..=n_max {
            for n in -n_max..=n_max {
                if n * n + m * m > n_max * n_max {
                    continue;
                }
                let rho = ((n * n + m * m) as f64).sqrt() / period_nm;
                orders.push(DiffractionOrder {
                    n,
                    m,
                    amplitude: Complex64::new(fill * airy_factor(2.0 * PI * r * rho), 0.0),
                });
            }
        }
        Ok(Self {
            period_x_nm: period_nm,
            period_y_nm: Some(period_nm),
            orders,
        })
    }

    /// Hexagonal lattice of circular holes with nearest-neighbour spacing
    /// `pitch_nm`, on the rectangular supercell `p_x = a`, `p_y = √3·a` holding
    /// holes at `(0, 0)` and `(a/2, √3a/2)`: structure factor
    /// `S_nm = 1 + (−1)^{n+m}` (orders with odd `n + m` vanish and are
    /// dropped). Orders are truncated on a circle, `|f| ≤ max_order/a`, so the
    /// retained set keeps the lattice's six-fold symmetry.
    pub fn hole_array_hexagonal(
        pitch_nm: f64,
        hole_diameter_nm: f64,
        max_order: usize,
    ) -> Result<Self> {
        require_positive("pitch_nm", pitch_nm)?;
        require_positive("hole_diameter_nm", hole_diameter_nm)?;
        if hole_diameter_nm > pitch_nm {
            return Err(invalid(
                "hole_diameter_nm",
                hole_diameter_nm,
                "holes must not overlap (diameter ≤ pitch)",
            ));
        }
        require_max_order(max_order)?;
        let r = 0.5 * hole_diameter_nm;
        let px = pitch_nm;
        let py = 3.0_f64.sqrt() * pitch_nm;
        let base = PI * r * r / (px * py);
        let n_max = max_order as i32;
        let m_max = (3.0_f64.sqrt() * max_order as f64).ceil() as i32;
        // |f|² ≤ (N/a)²  ⇔  3n² + m² ≤ 3N² (exact integer test).
        let limit = 3 * n_max * n_max;
        let mut orders = Vec::new();
        for m in -m_max..=m_max {
            for n in -n_max..=n_max {
                if (n + m).rem_euclid(2) != 0 || 3 * n * n + m * m > limit {
                    continue;
                }
                let fx = n as f64 / px;
                let fy = m as f64 / py;
                let rho = (fx * fx + fy * fy).sqrt();
                orders.push(DiffractionOrder {
                    n,
                    m,
                    amplitude: Complex64::new(2.0 * base * airy_factor(2.0 * PI * r * rho), 0.0),
                });
            }
        }
        Ok(Self {
            period_x_nm: px,
            period_y_nm: Some(py),
            orders,
        })
    }

    /// True for a 1D (line) grating invariant along y.
    pub fn is_1d(&self) -> bool {
        self.period_y_nm.is_none()
    }

    /// Fourier coefficient `c_nm` (zero if the order is not retained).
    pub fn coefficient(&self, n: i32, m: i32) -> Complex64 {
        self.orders
            .iter()
            .filter(|o| o.n == n && o.m == m)
            .map(|o| o.amplitude)
            .sum()
    }

    /// Diffraction efficiency `|c_nm|²` of one order.
    pub fn efficiency(&self, n: i32, m: i32) -> f64 {
        self.coefficient(n, m).norm_sqr()
    }

    /// `Σ |c_nm|²` over the retained orders (→ `⟨|t|²⟩` by Parseval as the
    /// truncation order grows).
    pub fn total_efficiency(&self) -> f64 {
        self.orders.iter().map(DiffractionOrder::efficiency).sum()
    }

    /// Transverse spatial frequency `(n/p_x, m/p_y)` of an order, in 1/nm.
    pub fn spatial_frequency(&self, order: &DiffractionOrder) -> (f64, f64) {
        let fy = match self.period_y_nm {
            Some(py) => order.m as f64 / py,
            None => 0.0,
        };
        (order.n as f64 / self.period_x_nm, fy)
    }

    /// Truncated Fourier synthesis of the transmission `t(x, y)`.
    pub fn transmission(&self, x_nm: f64, y_nm: f64) -> Complex64 {
        self.orders
            .iter()
            .map(|o| {
                let (fx, fy) = self.spatial_frequency(o);
                o.amplitude * Complex64::from_polar(1.0, 2.0 * PI * (fx * x_nm + fy * y_nm))
            })
            .sum()
    }
}

// ============================================================================
// Illumination, propagation, and exposure modes
// ============================================================================

/// Free-space transfer function used to propagate the diffraction orders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Propagation {
    /// Exact scalar angular spectrum (evanescent orders decay).
    #[default]
    AngularSpectrum,
    /// Paraxial Fresnel transfer function `exp(−iπλz|f|²/n)` for every
    /// retained order (no evanescent cutoff — valid only when the retained
    /// orders satisfy `λ|f| ≪ 1`); gives exact Talbot revivals.
    Paraxial,
}

/// Source spectrum for achromatic (ATL) averaging, centred on the setup
/// wavelength for the parametric shapes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Spectrum {
    /// Flat-top (rectangular) band of full width `bandwidth_nm`, split into
    /// `bins` equal spectral bins.
    FlatTop { bandwidth_nm: f64, bins: usize },
    /// Gaussian band of full width at half maximum `fwhm_nm`, truncated at
    /// ±4σ and split into `bins` equal spectral bins.
    Gaussian { fwhm_nm: f64, bins: usize },
    /// Discrete spectral lines `(wavelength_nm, relative weight)` — e.g. the
    /// harmonics of an HHG source. Each line is summed coherently, lines are
    /// summed incoherently.
    Lines(Vec<(f64, f64)>),
}

impl Spectrum {
    /// Nominal spectral width in nm: flat-top full width, Gaussian FWHM, or
    /// the span of the lines (0 for a single line).
    pub fn bandwidth_nm(&self) -> f64 {
        match self {
            Spectrum::FlatTop { bandwidth_nm, .. } => *bandwidth_nm,
            Spectrum::Gaussian { fwhm_nm, .. } => *fwhm_nm,
            Spectrum::Lines(lines) => {
                let lo = lines.iter().map(|l| l.0).fold(f64::INFINITY, f64::min);
                let hi = lines.iter().map(|l| l.0).fold(f64::NEG_INFINITY, f64::max);
                if lines.is_empty() {
                    0.0
                } else {
                    hi - lo
                }
            }
        }
    }

    /// Spectral bins `(λ_lo, λ_hi, weight)` with weights summing to 1. Within
    /// a bin the wavelength is treated as uniformly distributed; lines are
    /// zero-width bins.
    fn bins(&self, center_nm: f64) -> Result<Vec<(f64, f64, f64)>> {
        match self {
            Spectrum::FlatTop { bandwidth_nm, bins } => {
                require_non_negative("bandwidth_nm", *bandwidth_nm)?;
                let n = (*bins).max(1);
                let width = bandwidth_nm / n as f64;
                let lo = center_nm - 0.5 * bandwidth_nm;
                Ok((0..n)
                    .map(|j| {
                        let a = lo + j as f64 * width;
                        (a, a + width, 1.0 / n as f64)
                    })
                    .collect())
            }
            Spectrum::Gaussian { fwhm_nm, bins } => {
                require_non_negative("fwhm_nm", *fwhm_nm)?;
                let n = (*bins).max(1);
                let sigma = fwhm_nm / (2.0 * (2.0 * 2.0_f64.ln()).sqrt());
                if sigma == 0.0 {
                    return Ok(vec![(center_nm, center_nm, 1.0)]);
                }
                let span = 8.0 * sigma;
                let width = span / n as f64;
                let lo = center_nm - 0.5 * span;
                let mut out: Vec<(f64, f64, f64)> = (0..n)
                    .map(|j| {
                        let a = lo + j as f64 * width;
                        let u = (a + 0.5 * width - center_nm) / sigma;
                        (a, a + width, (-0.5 * u * u).exp())
                    })
                    .collect();
                let total: f64 = out.iter().map(|b| b.2).sum();
                for b in &mut out {
                    b.2 /= total;
                }
                Ok(out)
            }
            Spectrum::Lines(lines) => {
                if lines.is_empty() {
                    return Err(invalid("lines", 0.0, "at least one spectral line"));
                }
                let mut total = 0.0;
                for &(l, w) in lines {
                    require_positive("wavelength_nm", l)?;
                    require_non_negative("weight", w)?;
                    total += w;
                }
                if total <= 0.0 {
                    return Err(invalid("weight", total, "line weights must not all be 0"));
                }
                Ok(lines.iter().map(|&(l, w)| (l, l, w / total)).collect())
            }
        }
    }
}

/// How the exposure integrates the Talbot field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExposureMode {
    /// Single wavelength at a fixed gap.
    Coherent,
    /// Displacement Talbot lithography: the intensity averaged over gaps
    /// uniformly spanning `[gap, gap + scan_length_nm]` (closed form).
    Displacement { scan_length_nm: f64 },
    /// The infinite-scan DTL limit: only order pairs of equal `|f|` survive.
    DisplacementStationary,
    /// Achromatic Talbot lithography: incoherent average over the spectrum at
    /// a fixed gap.
    Achromatic(Spectrum),
}

/// The recording medium (resist) at the wafer.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ResistMedium {
    /// Real refractive index of the resist (may be < 1 at EUV/X-ray).
    pub index: f64,
    /// Intensity absorption coefficient in 1/nm.
    pub absorption_per_nm: f64,
}

impl Default for ResistMedium {
    fn default() -> Self {
        Self {
            index: 1.0,
            absorption_per_nm: 0.0,
        }
    }
}

/// A grating under normally incident plane-wave illumination.
#[derive(Debug, Clone, PartialEq)]
pub struct TalbotSetup {
    /// The mask grating.
    pub grating: Grating,
    /// Vacuum wavelength in nm (centre wavelength for spectral modes).
    pub wavelength_nm: f64,
    /// Refractive index of the mask–wafer gap medium (1 for vacuum).
    pub gap_index: f64,
    /// Transfer function.
    pub propagation: Propagation,
}

/// Orders of equal `|f|²` (identical transfer functions).
struct Shell {
    f2: f64,
    members: Vec<usize>,
}

/// Shell-pair weights `W_st = ⟨P_s P_t*⟩` for one image plane.
enum ShellWeights {
    /// Coherent: `W = P P†`, stored as the propagators `P_s`.
    Rank1(Vec<Complex64>),
    /// General Hermitian `K × K` matrix, row-major.
    Full(Vec<Complex64>),
}

/// The shell fields `U_s(x, y) = Σ_{a∈s} c_a e^{2πi f_a·r}` on a lateral grid.
struct ShellBasis {
    shells: usize,
    ny: usize,
    nx: usize,
    data: Vec<Complex64>,
}

impl ShellBasis {
    fn new(grating: &Grating, shells: &[Shell], xs: &[f64], ys: &[f64]) -> Self {
        let ys: &[f64] = if grating.is_1d() { &[0.0] } else { ys };
        let (nx, ny) = (xs.len(), ys.len());
        let n_max = grating.orders.iter().map(|o| o.n.abs()).max().unwrap_or(0);
        let m_max = grating.orders.iter().map(|o| o.m.abs()).max().unwrap_or(0);
        let py = grating.period_y_nm.unwrap_or(1.0);
        let ex: Vec<Vec<Complex64>> = (-n_max..=n_max)
            .map(|n| {
                xs.iter()
                    .map(|&x| {
                        Complex64::from_polar(1.0, 2.0 * PI * n as f64 * x / grating.period_x_nm)
                    })
                    .collect()
            })
            .collect();
        let ey: Vec<Vec<Complex64>> = (-m_max..=m_max)
            .map(|m| {
                ys.iter()
                    .map(|&y| Complex64::from_polar(1.0, 2.0 * PI * m as f64 * y / py))
                    .collect()
            })
            .collect();
        let plane = nx * ny;
        let mut data = vec![Complex64::new(0.0, 0.0); shells.len() * plane];
        for (s, shell) in shells.iter().enumerate() {
            let block = &mut data[s * plane..(s + 1) * plane];
            for &a in &shell.members {
                let o = &grating.orders[a];
                let exr = &ex[(o.n + n_max) as usize];
                let eyr = &ey[(o.m + m_max) as usize];
                for (row, &ey_i) in block.chunks_mut(nx).zip(eyr) {
                    let cy = o.amplitude * ey_i;
                    for (d, &e) in row.iter_mut().zip(exr) {
                        *d += cy * e;
                    }
                }
            }
        }
        Self {
            shells: shells.len(),
            ny,
            nx,
            data,
        }
    }

    /// Intensity on the basis grid, row-major `(ny, nx)`.
    fn render(&self, weights: &ShellWeights) -> Vec<f64> {
        let plane = self.nx * self.ny;
        let k = self.shells;
        match weights {
            ShellWeights::Rank1(p) => {
                let mut field = vec![Complex64::new(0.0, 0.0); plane];
                for (s, &ps) in p.iter().enumerate() {
                    if ps.norm_sqr() == 0.0 {
                        continue;
                    }
                    let block = &self.data[s * plane..(s + 1) * plane];
                    for (f, &u) in field.iter_mut().zip(block) {
                        *f += ps * u;
                    }
                }
                field.iter().map(|f| f.norm_sqr()).collect()
            }
            ShellWeights::Full(w) => {
                let mut out = vec![0.0; plane];
                for s in 0..k {
                    let us = &self.data[s * plane..(s + 1) * plane];
                    let wss = w[s * k + s].re;
                    if wss != 0.0 {
                        for (o, u) in out.iter_mut().zip(us) {
                            *o += wss * u.norm_sqr();
                        }
                    }
                    for t in (s + 1)..k {
                        let wst = w[s * k + t];
                        if wst.norm_sqr() == 0.0 {
                            continue;
                        }
                        let ut = &self.data[t * plane..(t + 1) * plane];
                        for ((o, a), b) in out.iter_mut().zip(us).zip(ut) {
                            *o += 2.0 * (wst * a * b.conj()).re;
                        }
                    }
                }
                for o in &mut out {
                    *o = o.max(0.0);
                }
                out
            }
        }
    }
}

/// Transfer of one plane-wave order of spatial frequency² `f2` through a
/// homogeneous slab of thickness `d` (index `index`, intensity absorption
/// `alpha`) at vacuum wavelength `lambda`: `(amplitude, unwrapped phase)`
/// relative to the normally incident carrier.
fn slab_transfer(
    f2: f64,
    lambda: f64,
    index: f64,
    d: f64,
    alpha: f64,
    propagation: Propagation,
) -> (f64, f64) {
    let nu = index / lambda;
    match propagation {
        Propagation::Paraxial => ((-0.5 * alpha * d).exp(), -PI * f2 * d / nu),
        Propagation::AngularSpectrum => {
            let s2 = nu * nu - f2;
            if s2 > 0.0 {
                let root = s2.sqrt();
                let cos_theta = root / nu;
                (
                    (-0.5 * alpha * d / cos_theta).exp(),
                    -2.0 * PI * f2 / (nu + root) * d,
                )
            } else {
                let kappa = 2.0 * PI * (-s2).sqrt();
                ((-(kappa + 0.5 * alpha) * d).exp(), -2.0 * PI * nu * d)
            }
        }
    }
}

/// Phase slope `∂phase/∂d` of the slab transfer (per nm of path), used for
/// the closed-form gap-scan average.
fn slab_phase_slope(f2: f64, lambda: f64, index: f64, propagation: Propagation) -> f64 {
    let nu = index / lambda;
    match propagation {
        Propagation::Paraxial => -PI * f2 / nu,
        Propagation::AngularSpectrum => {
            let s2 = nu * nu - f2;
            if s2 > 0.0 {
                -2.0 * PI * f2 / (nu + s2.sqrt())
            } else {
                -2.0 * PI * nu
            }
        }
    }
}

impl TalbotSetup {
    /// A grating in vacuum under a unit plane wave of `wavelength_nm`, with
    /// exact angular-spectrum propagation.
    pub fn new(grating: Grating, wavelength_nm: f64) -> Result<Self> {
        require_positive("wavelength_nm", wavelength_nm)?;
        require_positive("period_x_nm", grating.period_x_nm)?;
        if let Some(py) = grating.period_y_nm {
            require_positive("period_y_nm", py)?;
        }
        if grating.orders.is_empty() {
            return Err(invalid("orders", 0.0, "grating has no Fourier orders"));
        }
        Ok(Self {
            grating,
            wavelength_nm,
            gap_index: 1.0,
            propagation: Propagation::AngularSpectrum,
        })
    }

    fn validate(&self) -> Result<()> {
        require_positive("wavelength_nm", self.wavelength_nm)?;
        require_positive("gap_index", self.gap_index)?;
        Ok(())
    }

    /// Paraxial Talbot length `2p_x²/(λ/n_gap)` of the x period, in nm.
    pub fn talbot_length_nm(&self) -> f64 {
        talbot_length_paraxial_nm(
            self.grating.period_x_nm,
            self.wavelength_nm / self.gap_index,
        )
    }

    /// Exact first-order-pair Talbot length of the x period (see
    /// [`talbot_length_exact_nm`]).
    pub fn talbot_length_exact_nm(&self) -> Option<f64> {
        talbot_length_exact_nm(
            self.grating.period_x_nm,
            self.wavelength_nm / self.gap_index,
        )
    }

    /// Whether an order of spatial frequency² `f2` propagates in the gap.
    fn propagates(&self, f2: f64) -> bool {
        match self.propagation {
            Propagation::Paraxial => true,
            Propagation::AngularSpectrum => {
                let nu = self.gap_index / self.wavelength_nm;
                f2 < nu * nu
            }
        }
    }

    /// Far-field mean intensity `Σ|c|²` over the orders that propagate in the
    /// gap (all retained orders in paraxial mode) — the period-averaged
    /// intensity at any gap ≫ λ (energy conservation).
    pub fn propagating_efficiency(&self) -> f64 {
        self.grating
            .orders
            .iter()
            .filter(|o| {
                let (fx, fy) = self.grating.spatial_frequency(o);
                self.propagates(fx * fx + fy * fy)
            })
            .map(DiffractionOrder::efficiency)
            .sum()
    }

    fn shells(&self) -> Vec<Shell> {
        let mut keyed: Vec<(f64, usize)> = self
            .grating
            .orders
            .iter()
            .enumerate()
            .map(|(i, o)| {
                let (fx, fy) = self.grating.spatial_frequency(o);
                (fx * fx + fy * fy, i)
            })
            .collect();
        keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut shells: Vec<Shell> = Vec::new();
        for (f2, i) in keyed {
            match shells.last_mut() {
                Some(last) if (f2 - last.f2).abs() <= SHELL_TOLERANCE * f2.max(last.f2) => {
                    last.members.push(i);
                }
                _ => shells.push(Shell {
                    f2,
                    members: vec![i],
                }),
            }
        }
        shells
    }

    /// `(amplitude, phase)` of a shell after the gap and an optional depth
    /// inside the resist, at vacuum wavelength `lambda`.
    fn shell_transfer(
        &self,
        f2: f64,
        lambda: f64,
        gap_nm: f64,
        resist: Option<(&ResistMedium, f64)>,
    ) -> (f64, f64) {
        let (mut amp, mut phase) =
            slab_transfer(f2, lambda, self.gap_index, gap_nm, 0.0, self.propagation);
        if let Some((medium, depth)) = resist {
            let (a2, p2) = slab_transfer(
                f2,
                lambda,
                medium.index,
                depth,
                medium.absorption_per_nm,
                self.propagation,
            );
            amp *= a2;
            phase += p2;
        }
        (amp, phase)
    }

    fn weights(
        &self,
        shells: &[Shell],
        gap_nm: f64,
        resist: Option<(&ResistMedium, f64)>,
        mode: &ExposureMode,
    ) -> Result<ShellWeights> {
        let lam = self.wavelength_nm;
        let k = shells.len();
        let polar = |(a, p): (f64, f64)| Complex64::from_polar(a, p);
        match mode {
            ExposureMode::Coherent => Ok(ShellWeights::Rank1(
                shells
                    .iter()
                    .map(|s| polar(self.shell_transfer(s.f2, lam, gap_nm, resist)))
                    .collect(),
            )),
            ExposureMode::Displacement { scan_length_nm } => {
                let l = *scan_length_nm;
                require_non_negative("scan_length_nm", l)?;
                let mid = gap_nm + 0.5 * l;
                let p: Vec<Complex64> = shells
                    .iter()
                    .map(|s| polar(self.shell_transfer(s.f2, lam, mid, resist)))
                    .collect();
                let slope: Vec<f64> = shells
                    .iter()
                    .map(|s| slab_phase_slope(s.f2, lam, self.gap_index, self.propagation))
                    .collect();
                let mut w = vec![Complex64::new(0.0, 0.0); k * k];
                for s in 0..k {
                    for t in 0..k {
                        let damp = sinc((slope[s] - slope[t]) * l / (2.0 * PI));
                        w[s * k + t] = p[s] * p[t].conj() * damp;
                    }
                }
                Ok(ShellWeights::Full(w))
            }
            ExposureMode::DisplacementStationary => {
                let mut w = vec![Complex64::new(0.0, 0.0); k * k];
                for (s, shell) in shells.iter().enumerate() {
                    if self.propagates(shell.f2) {
                        let (a, _) = self.shell_transfer(shell.f2, lam, 0.0, resist);
                        w[s * k + s] = Complex64::new(a * a, 0.0);
                    }
                }
                Ok(ShellWeights::Full(w))
            }
            ExposureMode::Achromatic(spectrum) => {
                let mut w = vec![Complex64::new(0.0, 0.0); k * k];
                let closed_form = self.propagation == Propagation::Paraxial
                    && matches!(
                        spectrum,
                        Spectrum::FlatTop { .. } | Spectrum::Gaussian { .. }
                    );
                if closed_form {
                    // Validate the spectrum parameters (the bins themselves are unused).
                    spectrum.bins(lam)?;
                    // Paraxial phases are linear in λ: phase_s = −π λ f_s² D_eff.
                    let d_eff =
                        gap_nm / self.gap_index + resist.map_or(0.0, |(m, depth)| depth / m.index);
                    let amp =
                        resist.map_or(1.0, |(m, depth)| (-0.5 * m.absorption_per_nm * depth).exp());
                    for s in 0..k {
                        for t in 0..k {
                            let a = -PI * (shells[s].f2 - shells[t].f2) * d_eff;
                            let envelope = match spectrum {
                                Spectrum::FlatTop { bandwidth_nm, .. } => {
                                    sinc(a * bandwidth_nm / (2.0 * PI))
                                }
                                Spectrum::Gaussian { fwhm_nm, .. } => {
                                    let sigma = fwhm_nm / (2.0 * (2.0 * 2.0_f64.ln()).sqrt());
                                    (-0.5 * (a * sigma).powi(2)).exp()
                                }
                                Spectrum::Lines(_) => unreachable!("lines use the bin path"),
                            };
                            w[s * k + t] = Complex64::from_polar(amp * amp * envelope, a * lam);
                        }
                    }
                    return Ok(ShellWeights::Full(w));
                }
                for (lo, hi, weight) in spectrum.bins(lam)? {
                    let mid = 0.5 * (lo + hi);
                    let edges: Vec<(f64, f64, f64)> = shells
                        .iter()
                        .map(|s| {
                            let (a, _) = self.shell_transfer(s.f2, mid, gap_nm, resist);
                            let (_, p_lo) = self.shell_transfer(s.f2, lo, gap_nm, resist);
                            let (_, p_hi) = self.shell_transfer(s.f2, hi, gap_nm, resist);
                            (a, 0.5 * (p_lo + p_hi), p_hi - p_lo)
                        })
                        .collect();
                    for s in 0..k {
                        let (a_s, phi_s, dphi_s) = edges[s];
                        if a_s == 0.0 {
                            continue;
                        }
                        for t in 0..k {
                            let (a_t, phi_t, dphi_t) = edges[t];
                            let envelope = sinc((dphi_s - dphi_t) / (2.0 * PI));
                            w[s * k + t] +=
                                Complex64::from_polar(weight * a_s * a_t * envelope, phi_s - phi_t);
                        }
                    }
                }
                Ok(ShellWeights::Full(w))
            }
        }
    }

    /// Complex field `U(x, y, z)` at distance `z_nm` behind the grating
    /// (coherent, centre wavelength; the common carrier `e^{ikz}` dropped).
    pub fn field(&self, x_nm: f64, y_nm: f64, z_nm: f64) -> Complex64 {
        self.grating
            .orders
            .iter()
            .map(|o| {
                let (fx, fy) = self.grating.spatial_frequency(o);
                let (a, p) = slab_transfer(
                    fx * fx + fy * fy,
                    self.wavelength_nm,
                    self.gap_index,
                    z_nm,
                    0.0,
                    self.propagation,
                );
                o.amplitude
                    * Complex64::from_polar(a, p)
                    * Complex64::from_polar(1.0, 2.0 * PI * (fx * x_nm + fy * y_nm))
            })
            .sum()
    }

    fn render_plane(
        &self,
        xs: &[f64],
        ys: &[f64],
        gap_nm: f64,
        mode: &ExposureMode,
    ) -> Result<Array2<f64>> {
        self.validate()?;
        require_non_negative("gap_nm", gap_nm)?;
        let shells = self.shells();
        let basis = ShellBasis::new(&self.grating, &shells, xs, ys);
        let weights = self.weights(&shells, gap_nm, None, mode)?;
        Ok(broadcast(&basis, basis.render(&weights), ys.len()))
    }

    /// Coherent free-space intensity at distance `z_nm` behind the grating on
    /// the lateral grid `grid` (overwrites `grid.data`).
    pub fn intensity_plane(&self, grid: &mut Grid2D<f64>, z_nm: f64) -> Result<()> {
        self.exposure_image(grid, z_nm, &ExposureMode::Coherent)
    }

    /// Wafer-plane image at gap `gap_nm` for the given exposure mode on the
    /// lateral grid `grid` (overwrites `grid.data`).
    pub fn exposure_image(
        &self,
        grid: &mut Grid2D<f64>,
        gap_nm: f64,
        mode: &ExposureMode,
    ) -> Result<()> {
        let xs: Vec<f64> = (0..grid.nx()).map(|j| grid.x_at(j)).collect();
        let ys: Vec<f64> = (0..grid.ny()).map(|i| grid.y_at(i)).collect();
        grid.data = self.render_plane(&xs, &ys, gap_nm, mode)?;
        Ok(())
    }

    /// Talbot carpet `I(x, z)` along `y = 0`: a [`Grid2D`] whose columns are x
    /// (cell centres over `x_range`) and whose rows are the propagation
    /// distance z (cell centres over `z_range`; the grid's `y` extent is the
    /// z range). Coherent at the centre wavelength, or spectrally averaged
    /// when `spectrum` is given (the ATL carpet, whose revivals wash out
    /// beyond the achromatic distance).
    pub fn carpet(
        &self,
        x_range: (f64, f64),
        nx: usize,
        z_range: (f64, f64),
        nz: usize,
        spectrum: Option<&Spectrum>,
    ) -> Result<Grid2D<f64>> {
        self.validate()?;
        let mut grid = Grid2D::<f64>::new(nx, nz, x_range, z_range)?;
        // Rows sit at cell centres; the first one must not lie before the mask
        // (a range starting half a row below 0 puts row 0 exactly at z = 0).
        if grid.y_at(0) < -1e-9 * grid.pixel_size_y() {
            return Err(invalid(
                "z_range",
                grid.y_at(0),
                "carpet rows must lie at z ≥ 0 behind the grating",
            ));
        }
        let xs: Vec<f64> = (0..nx).map(|j| grid.x_at(j)).collect();
        let shells = self.shells();
        let basis = ShellBasis::new(&self.grating, &shells, &xs, &[0.0]);
        let mode = match spectrum {
            Some(s) => ExposureMode::Achromatic(s.clone()),
            None => ExposureMode::Coherent,
        };
        for i in 0..nz {
            let z = grid.y_at(i).max(0.0);
            let row = basis.render(&self.weights(&shells, z, None, &mode)?);
            for (d, v) in grid.data.row_mut(i).iter_mut().zip(row) {
                *d = v;
            }
        }
        Ok(grid)
    }

    /// Intensity volume inside a resist whose top surface sits at gap
    /// `gap_nm` from the grating, for the given exposure mode.
    ///
    /// `grid.z_at(k)` is the depth below the resist top (must be ≥ 0; the
    /// usual `z_range = (0, thickness)`). Orders enter the resist with unit
    /// transmission (no Fresnel coefficients) and continue with their own
    /// in-resist `k_z`; amplitudes decay as `exp(−α z/(2 cos θ))`. The result
    /// (normalized to the incident intensity) feeds
    /// [`crate::interference::expose`] and the volumetric development tiers.
    pub fn resist_volume(
        &self,
        grid: &mut Grid3D<f64>,
        gap_nm: f64,
        medium: &ResistMedium,
        mode: &ExposureMode,
    ) -> Result<()> {
        self.validate()?;
        require_non_negative("gap_nm", gap_nm)?;
        require_positive("resist index", medium.index)?;
        require_non_negative("absorption_per_nm", medium.absorption_per_nm)?;
        if grid.z_min_nm < 0.0 {
            return Err(invalid(
                "z_min_nm",
                grid.z_min_nm,
                "depths below the resist top must be non-negative",
            ));
        }
        let xs: Vec<f64> = (0..grid.nx()).map(|j| grid.x_at(j)).collect();
        let ys: Vec<f64> = (0..grid.ny()).map(|i| grid.y_at(i)).collect();
        let shells = self.shells();
        let basis = ShellBasis::new(&self.grating, &shells, &xs, &ys);
        for k in 0..grid.nz() {
            let depth = grid.z_at(k);
            let weights = self.weights(&shells, gap_nm, Some((medium, depth)), mode)?;
            let plane = broadcast(&basis, basis.render(&weights), ys.len());
            grid.data.index_axis_mut(ndarray::Axis(0), k).assign(&plane);
        }
        Ok(())
    }
}

/// Reshape a rendered basis plane to `(ny, nx)`, replicating the single row
/// of a y-invariant (1D grating) basis.
fn broadcast(basis: &ShellBasis, values: Vec<f64>, ny: usize) -> Array2<f64> {
    let nx = basis.nx;
    if basis.ny == ny {
        Array2::from_shape_vec((ny, nx), values).expect("render returns ny·nx samples")
    } else {
        Array2::from_shape_fn((ny, nx), |(_, j)| values[j])
    }
}

// ============================================================================
// Two-grating EUV interference lithography
// ============================================================================

/// Two-grating (±m order) interference lithography, as used for EUV-IL with
/// transmission gratings: the +m order of one grating and the −m order of a
/// second, identical-period grating overlap at the wafer.
///
/// With `sin θ = mλ/p` the fringe period is `λ/(2 sin θ) = p/(2m)` — set by the
/// grating period alone, independent of wavelength (and of the resist index).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TwoGratingInterference {
    /// Period of the two transmission gratings in nm.
    pub grating_period_nm: f64,
    /// Vacuum wavelength in nm.
    pub wavelength_nm: f64,
    /// Diffraction order `m ≥ 1` used by both gratings.
    pub order: u32,
    /// Intensity of beam 1 (grating 1's +m order) relative to the incident beam.
    pub beam_intensity_1: f64,
    /// Intensity of beam 2 (grating 2's −m order) relative to the incident beam.
    pub beam_intensity_2: f64,
    /// Phase of beam 2 relative to beam 1 in radians (shifts the fringes).
    pub relative_phase_rad: f64,
}

impl TwoGratingInterference {
    /// Beams from the `+m` and `−m` orders of `grating` (a 1D grating design
    /// used for both gratings): `I₁ = |c_{+m}|²`, `I₂ = |c_{−m}|²`.
    pub fn new(grating: &Grating, wavelength_nm: f64, order: u32) -> Result<Self> {
        if !grating.is_1d() {
            return Err(invalid(
                "grating",
                0.0,
                "two-grating interference needs a 1D grating",
            ));
        }
        let m = order as i32;
        let retained = |k: i32| grating.orders.iter().any(|o| o.n == k && o.m == 0);
        if !retained(m) || !retained(-m) {
            return Err(invalid(
                "order",
                order as f64,
                "the ±m orders are not retained in the grating (raise max_order)",
            ));
        }
        Self::from_efficiencies(
            grating.period_x_nm,
            wavelength_nm,
            order,
            grating.efficiency(m, 0),
            grating.efficiency(-m, 0),
        )
    }

    /// Beams with explicit diffraction efficiencies (relative intensities).
    pub fn from_efficiencies(
        grating_period_nm: f64,
        wavelength_nm: f64,
        order: u32,
        efficiency_1: f64,
        efficiency_2: f64,
    ) -> Result<Self> {
        require_positive("grating_period_nm", grating_period_nm)?;
        require_positive("wavelength_nm", wavelength_nm)?;
        if order == 0 {
            return Err(invalid("order", 0.0, "diffraction order must be ≥ 1"));
        }
        require_non_negative("efficiency_1", efficiency_1)?;
        require_non_negative("efficiency_2", efficiency_2)?;
        let s = order as f64 * wavelength_nm / grating_period_nm;
        if s >= 1.0 {
            return Err(invalid(
                "order",
                order as f64,
                "order is evanescent: m·λ/p must be < 1",
            ));
        }
        Ok(Self {
            grating_period_nm,
            wavelength_nm,
            order,
            beam_intensity_1: efficiency_1,
            beam_intensity_2: efficiency_2,
            relative_phase_rad: 0.0,
        })
    }

    /// `sin θ = mλ/p` of each beam in vacuum.
    pub fn sin_theta(&self) -> f64 {
        self.order as f64 * self.wavelength_nm / self.grating_period_nm
    }

    /// Diffraction (half-)angle θ in degrees.
    pub fn diffraction_angle_deg(&self) -> f64 {
        self.sin_theta().asin().to_degrees()
    }

    /// Fringe period `λ/(2 sin θ) = p/(2m)` in nm.
    pub fn fringe_period_nm(&self) -> f64 {
        self.wavelength_nm / (2.0 * self.sin_theta())
    }

    /// Fringe visibility `2 sqrt(I₁I₂)/(I₁ + I₂)` (TE beams).
    pub fn visibility(&self) -> f64 {
        let total = self.beam_intensity_1 + self.beam_intensity_2;
        if total <= 0.0 {
            return 0.0;
        }
        2.0 * (self.beam_intensity_1 * self.beam_intensity_2).sqrt() / total
    }

    /// Analytic TE two-beam intensity
    /// `I₁ + I₂ + 2 sqrt(I₁I₂)·cos(4πm x/p − φ)` at lateral position `x_nm`.
    pub fn intensity_at(&self, x_nm: f64) -> f64 {
        let k = 4.0 * PI * self.order as f64 / self.grating_period_nm;
        self.beam_intensity_1
            + self.beam_intensity_2
            + 2.0
                * (self.beam_intensity_1 * self.beam_intensity_2).sqrt()
                * (k * x_nm - self.relative_phase_rad).cos()
    }

    /// The equivalent [`InterferenceSetup`]: two TE (`ŷ`-polarized) plane
    /// waves at `±θ_med` in the xz-plane, `sin θ_med = sin θ / n_medium`
    /// (Snell; `n_medium` may be below 1 at EUV), amplitudes `sqrt(I₁)`,
    /// `sqrt(I₂)`, beam 2 carrying `relative_phase_rad`. Use
    /// [`InterferenceSetup::intensity`] + [`crate::interference::expose`] for
    /// PAC volumes.
    pub fn to_interference_setup(
        &self,
        medium_index: f64,
        absorption_per_nm: f64,
    ) -> Result<InterferenceSetup> {
        require_positive("medium_index", medium_index)?;
        require_non_negative("absorption_per_nm", absorption_per_nm)?;
        let s = self.sin_theta() / medium_index;
        if s >= 1.0 {
            return Err(invalid(
                "medium_index",
                medium_index,
                "beams are totally internally reflected: sin θ / n must be < 1",
            ));
        }
        let c = (1.0 - s * s).sqrt();
        let pol = [0.0, 1.0, 0.0];
        let beams = vec![
            PlaneWave::new([s, 0.0, c], self.beam_intensity_1.sqrt(), 0.0, pol)?,
            PlaneWave::new(
                [-s, 0.0, c],
                self.beam_intensity_2.sqrt(),
                self.relative_phase_rad,
                pol,
            )?,
        ];
        Ok(InterferenceSetup {
            beams,
            wavelength_vacuum_nm: self.wavelength_nm,
            medium_index,
            absorption_per_nm,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn c(re: f64, im: f64) -> Complex64 {
        Complex64::new(re, im)
    }

    fn line_grid(period_nm: f64, periods: usize, nx: usize) -> Grid2D<f64> {
        Grid2D::<f64>::new(nx, 1, (0.0, period_nm * periods as f64), (-0.5, 0.5)).unwrap()
    }

    fn row(grid: &Grid2D<f64>) -> Vec<f64> {
        grid.data.row(0).to_vec()
    }

    fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
        a.iter()
            .zip(b)
            .map(|(x, y)| (x - y).abs())
            .fold(0.0, f64::max)
    }

    // ---------------------------------------------------------------- fixtures

    #[test]
    fn test_sinc_and_bessel_fixtures() {
        assert_relative_eq!(sinc(0.0), 1.0, epsilon = 1e-15);
        assert_relative_eq!(sinc(0.5), 2.0 / PI, epsilon = 1e-15);
        assert!(sinc(2.0).abs() < 1e-15);
        // Reference values from the power series evaluated with 60-digit
        // decimal arithmetic (independent of the trapezoid rule used here).
        assert_relative_eq!(bessel_j1(1.0), 0.440_050_585_744_933_5, epsilon = 1e-14);
        assert_relative_eq!(bessel_j1(5.0), -0.327_579_137_591_465_2, epsilon = 1e-14);
        assert_relative_eq!(bessel_j1(10.0), 0.043_472_746_168_861_44, epsilon = 1e-14);
        assert_relative_eq!(bessel_j1(30.0), -0.118_751_062_616_622_9, epsilon = 1e-13);
        assert!(bessel_j1(3.831_705_970_207_512).abs() < 1e-14);
        assert_relative_eq!(bessel_j1(-1.0), -bessel_j1(1.0), epsilon = 1e-15);
    }

    #[test]
    fn test_binary_amplitude_coefficients_fixture() {
        let g = Grating::binary_amplitude(100.0, 0.5, 5).unwrap();
        assert_eq!(g.orders.len(), 11);
        assert_relative_eq!(g.coefficient(0, 0).re, 0.5, epsilon = 1e-15);
        assert_relative_eq!(g.coefficient(1, 0).re, 1.0 / PI, epsilon = 1e-15);
        assert_relative_eq!(g.coefficient(-1, 0).re, 1.0 / PI, epsilon = 1e-15);
        assert!(g.coefficient(2, 0).norm() < 1e-16);
        assert_relative_eq!(
            g.coefficient(3, 0).re,
            -0.106_103_295_394_596_9,
            epsilon = 1e-15
        );

        let g = Grating::binary_amplitude(100.0, 0.3, 3).unwrap();
        assert_relative_eq!(
            g.coefficient(1, 0).re,
            0.257_518_107_400_241_95,
            epsilon = 1e-15
        );
        assert_relative_eq!(
            g.coefficient(2, 0).re,
            0.151_365_345_728_131_4,
            epsilon = 1e-15
        );
        assert_relative_eq!(
            g.efficiency(1, 0),
            0.257_518_107_400_241_95_f64.powi(2),
            epsilon = 1e-15
        );
    }

    #[test]
    fn test_binary_phase_coefficients_fixture() {
        // π step, 50 % duty: zeroth order suppressed, |c±1|² = 4/π².
        let g = Grating::binary_phase(100.0, 0.5, PI, 5).unwrap();
        assert!(g.coefficient(0, 0).norm() < 1e-15);
        assert_relative_eq!(
            g.efficiency(1, 0),
            0.405_284_734_569_351_16,
            epsilon = 1e-15
        );
        assert_relative_eq!(
            g.efficiency(-1, 0),
            0.405_284_734_569_351_16,
            epsilon = 1e-15
        );

        let g = Grating::binary_phase(100.0, 0.5, PI / 2.0, 3).unwrap();
        let c0 = g.coefficient(0, 0);
        assert_relative_eq!(c0.re, 0.5, epsilon = 1e-15);
        assert_relative_eq!(c0.im, 0.5, epsilon = 1e-15);
        let c1 = g.coefficient(1, 0);
        assert_relative_eq!(c1.re, -0.318_309_886_183_790_64, epsilon = 1e-15);
        assert_relative_eq!(c1.im, 1.0 / PI, epsilon = 1e-15);

        let g = Grating::binary_phase(100.0, 0.25, PI / 3.0, 3).unwrap();
        let (c0, c1, c2) = (
            g.coefficient(0, 0),
            g.coefficient(1, 0),
            g.coefficient(2, 0),
        );
        assert_relative_eq!(c0.re, 0.875, epsilon = 1e-15);
        assert_relative_eq!(c0.im, 0.216_506_350_946_109_65, epsilon = 1e-15);
        assert_relative_eq!(c1.re, -0.112_539_539_519_638_23, epsilon = 1e-15);
        assert_relative_eq!(c1.im, 0.194_924_200_308_419, epsilon = 1e-15);
        assert_relative_eq!(c2.re, -0.079_577_471_545_947_66, epsilon = 1e-15);
        assert_relative_eq!(c2.im, 0.137_832_223_855_448_02, epsilon = 1e-15);
    }

    #[test]
    fn test_parseval_total_efficiency() {
        // Σ|c_n|² → ⟨|t|²⟩: open fraction for an amplitude grating, 1 for a
        // phase grating. The tail beyond N is ≈ 2f²·Σ_{n>N} sinc² ≲ 1/(π² N).
        let amp = Grating::binary_amplitude(100.0, 0.3, 400).unwrap();
        assert!((amp.total_efficiency() - 0.3).abs() < 1e-3);
        let phase = Grating::binary_phase(100.0, 0.4, 1.0, 400).unwrap();
        assert!((phase.total_efficiency() - 1.0).abs() < 1e-3);
        let sine = Grating::sinusoidal_amplitude(100.0, 0.5, 0.4).unwrap();
        // ⟨(0.5 + 0.4 cos)²⟩ = 0.25 + 0.08.
        assert_relative_eq!(sine.total_efficiency(), 0.33, epsilon = 1e-15);
    }

    #[test]
    fn test_from_profile_is_exact_for_pixelated_profile() {
        // First five of ten samples open: a 50 % slit centred at x = −p/4, so
        // c_n = 0.5·sinc(n/2)·e^{iπn/2} exactly.
        let mut samples = vec![c(0.0, 0.0); 10];
        for s in samples.iter_mut().take(5) {
            *s = c(1.0, 0.0);
        }
        let g = Grating::from_profile(&samples, 80.0, 6).unwrap();
        assert!(g.is_1d());
        for n in -6..=6 {
            let expected = Complex64::from_polar(0.5 * sinc(n as f64 / 2.0), PI * n as f64 / 2.0);
            let got = g.coefficient(n, 0);
            assert!(
                (got - expected).norm() < 1e-14,
                "n = {n}: {got} vs {expected}"
            );
        }
    }

    #[test]
    fn test_hole_array_coefficients_fixture() {
        // Square lattice p = 100 nm, d = 50 nm (values from numpy with the
        // J1 power series).
        let sq = Grating::hole_array_square(100.0, 50.0, 3).unwrap();
        assert_relative_eq!(
            sq.coefficient(0, 0).re,
            0.196_349_540_849_362_07,
            epsilon = 1e-14
        );
        assert_relative_eq!(
            sq.coefficient(1, 0).re,
            0.141_706_022_226_468_47,
            epsilon = 1e-13
        );
        assert_relative_eq!(
            sq.coefficient(0, -1).re,
            0.141_706_022_226_468_47,
            epsilon = 1e-13
        );
        assert_relative_eq!(
            sq.coefficient(1, 1).re,
            0.097_726_510_124_880_43,
            epsilon = 1e-13
        );

        // Hexagonal lattice a = 100 nm, d = 50 nm: the first reciprocal shell
        // (±1, ±1), (0, ±2) is six-fold degenerate; odd n + m vanish.
        let hex = Grating::hole_array_hexagonal(100.0, 50.0, 3).unwrap();
        assert_relative_eq!(
            hex.coefficient(0, 0).re,
            0.226_724_920_529_277_23,
            epsilon = 1e-14
        );
        for (n, m) in [(1, 1), (-1, 1), (1, -1), (-1, -1), (0, 2), (0, -2)] {
            assert_relative_eq!(
                hex.coefficient(n, m).re,
                0.145_427_823_452_099_87,
                epsilon = 1e-13
            );
        }
        assert!(hex.coefficient(1, 0).norm() < 1e-15);
        assert!(hex.coefficient(0, 1).norm() < 1e-15);
        assert_relative_eq!(
            hex.coefficient(2, 0).re,
            0.041_080_686_250_082,
            epsilon = 1e-13
        );
    }

    #[test]
    fn test_unit_cell_sampler_converges_to_analytic_holes() {
        // Antialiased (8×8 supersampled) raster of one hole per cell.
        let (p, d, n) = (100.0, 50.0, 128usize);
        let r = 0.5 * d;
        let sub = 8usize;
        let cell = Array2::from_shape_fn((n, n), |(i, j)| {
            let mut inside = 0usize;
            for a in 0..sub {
                for b in 0..sub {
                    let x = -0.5 * p + (j as f64 + (b as f64 + 0.5) / sub as f64) * p / n as f64;
                    let y = -0.5 * p + (i as f64 + (a as f64 + 0.5) / sub as f64) * p / n as f64;
                    if x * x + y * y <= r * r {
                        inside += 1;
                    }
                }
            }
            c(inside as f64 / (sub * sub) as f64, 0.0)
        });
        let sampled = Grating::from_unit_cell(&cell, p, p, 2).unwrap();
        let exact = Grating::hole_array_square(p, d, 3).unwrap();
        for (nn, mm) in [(0, 0), (1, 0), (1, 1), (2, 1)] {
            let diff = (sampled.coefficient(nn, mm) - exact.coefficient(nn, mm)).norm();
            assert!(
                diff < 2e-4,
                "({nn},{mm}): sampled vs exact differ by {diff}"
            );
        }
    }

    // ------------------------------------------------------------ Talbot lengths

    #[test]
    fn test_talbot_length_fixtures_and_limits() {
        assert_relative_eq!(
            talbot_length_paraxial_nm(100.0, 13.5),
            1_481.481_481_481_481_5,
            epsilon = 1e-9
        );
        assert_relative_eq!(
            talbot_length_exact_nm(100.0, 13.5).unwrap(),
            1_474.700_443_308_358_7,
            epsilon = 1e-9
        );
        // λ/p = 1/2: exact 2 + √3 vs paraxial 4 (p = 1).
        assert_relative_eq!(
            talbot_length_exact_nm(1.0, 0.5).unwrap(),
            2.0 + 3.0_f64.sqrt(),
            epsilon = 1e-14
        );
        assert_relative_eq!(talbot_length_paraxial_nm(1.0, 0.5), 4.0, epsilon = 1e-15);
        // λ/p → 0: exact/paraxial = 1 − ε²/4 + O(ε⁴).
        let eps = 0.01;
        let ratio = talbot_length_exact_nm(1.0, eps).unwrap() / talbot_length_paraxial_nm(1.0, eps);
        assert!(
            (ratio - (1.0 - eps * eps / 4.0)).abs() < 1e-8,
            "ratio {ratio}"
        );
        // No propagating first order for λ ≥ p.
        assert!(talbot_length_exact_nm(10.0, 10.0).is_none());
        assert!(talbot_length_exact_nm(10.0, 12.0).is_none());
        // Scaling: doubling the period quadruples z_T; z_A = 2p²/Δλ.
        assert_relative_eq!(
            talbot_length_paraxial_nm(200.0, 13.5) / talbot_length_paraxial_nm(100.0, 13.5),
            4.0,
            epsilon = 1e-14
        );
        assert_relative_eq!(
            achromatic_distance_nm(100.0, 0.3),
            66_666.666_666_666_67,
            epsilon = 1e-8
        );
        assert_relative_eq!(
            two_grating_fringe_period_nm(100.0, 1),
            50.0,
            epsilon = 1e-15
        );
        assert_relative_eq!(
            two_grating_fringe_period_nm(100.0, 2),
            25.0,
            epsilon = 1e-15
        );
    }

    #[test]
    fn test_dominant_period_helper() {
        let prof: Vec<f64> = (0..64)
            .map(|j| (2.0 * PI * j as f64 / 16.0).cos())
            .collect();
        assert_relative_eq!(
            dominant_period_nm(&prof, 2.0).unwrap(),
            32.0,
            epsilon = 1e-12
        );
        assert!(dominant_period_nm(&[1.0; 16], 1.0).is_none());
    }

    // ------------------------------------------------------------ self-imaging

    #[test]
    fn test_paraxial_self_image_and_half_talbot_shift() {
        let p = 100.0;
        let mut setup =
            TalbotSetup::new(Grating::binary_amplitude(p, 0.5, 15).unwrap(), 13.5).unwrap();
        setup.propagation = Propagation::Paraxial;
        let zt = setup.talbot_length_nm();
        let nx = 64;
        let mut g0 = line_grid(p, 1, nx);
        let mut gt = line_grid(p, 1, nx);
        let mut gh = line_grid(p, 1, nx);
        setup.intensity_plane(&mut g0, 0.0).unwrap();
        setup.intensity_plane(&mut gt, zt).unwrap();
        setup.intensity_plane(&mut gh, 0.5 * zt).unwrap();
        let (i0, it, ih) = (row(&g0), row(&gt), row(&gh));
        // Revival at z_T.
        assert!(
            max_abs_diff(&i0, &it) < 1e-10,
            "self-image error {}",
            max_abs_diff(&i0, &it)
        );
        // z = 0 reproduces the truncated |t|².
        for (j, &v) in i0.iter().enumerate() {
            let x = g0.x_at(j);
            assert!((v - setup.grating.transmission(x, 0.0).norm_sqr()).abs() < 1e-12);
        }
        // Half Talbot: shifted by p/2 = nx/2 samples.
        let shifted: Vec<f64> = (0..nx).map(|j| i0[(j + nx / 2) % nx]).collect();
        assert!(max_abs_diff(&ih, &shifted) < 1e-10);
        // And the image at z_T/4 is genuinely different (not trivially constant).
        let mut gq = line_grid(p, 1, nx);
        setup.intensity_plane(&mut gq, 0.25 * zt).unwrap();
        assert!(max_abs_diff(&row(&gq), &i0) > 0.1);
    }

    #[test]
    fn test_exact_first_order_pair_rephases_at_exact_length() {
        // Sinusoidal grating (orders 0, ±1 only) at λ/p = 0.6: strongly
        // non-paraxial. The exact field revives at the exact z_T and not at
        // the paraxial one.
        let (p, lambda) = (100.0, 60.0);
        let setup =
            TalbotSetup::new(Grating::sinusoidal_amplitude(p, 0.5, 0.5).unwrap(), lambda).unwrap();
        let zt_exact = setup.talbot_length_exact_nm().unwrap();
        let zt_par = setup.talbot_length_nm();
        assert!((zt_exact / zt_par - 0.9).abs() < 1e-12); // (1 + 0.8)/2
        let mut g0 = line_grid(p, 1, 50);
        let mut ge = line_grid(p, 1, 50);
        let mut gp = line_grid(p, 1, 50);
        setup.intensity_plane(&mut g0, 0.0).unwrap();
        setup.intensity_plane(&mut ge, 3.0 * zt_exact).unwrap();
        setup.intensity_plane(&mut gp, 3.0 * zt_par).unwrap();
        assert!(max_abs_diff(&row(&g0), &row(&ge)) < 1e-10);
        assert!(max_abs_diff(&row(&g0), &row(&gp)) > 0.1);
    }

    #[test]
    fn test_energy_conservation_with_evanescent_orders() {
        // p = 100 nm at 13.5 nm: orders |n| ≤ 7 propagate, |n| = 8..15 are
        // evanescent. Far behind the mask the period-averaged intensity is
        // Σ_{|n|≤7} |c_n|².
        let setup =
            TalbotSetup::new(Grating::binary_amplitude(100.0, 0.4, 15).unwrap(), 13.5).unwrap();
        let expected: f64 = (-7..=7).map(|n| setup.grating.efficiency(n, 0)).sum();
        assert_relative_eq!(setup.propagating_efficiency(), expected, epsilon = 1e-14);
        let mut g = line_grid(100.0, 1, 128);
        setup.intensity_plane(&mut g, 5_000.0).unwrap();
        let mean = g.data.mean().unwrap();
        assert!((mean - expected).abs() < 1e-12, "mean {mean} vs {expected}");
        // Right behind the mask the evanescent orders still contribute.
        setup.intensity_plane(&mut g, 0.0).unwrap();
        assert!((g.data.mean().unwrap() - setup.grating.total_efficiency()).abs() < 1e-12);
    }

    #[test]
    fn test_clear_mask_is_unity_everywhere() {
        let clear = Grating::from_coefficients(100.0, &[(0, c(1.0, 0.0))]).unwrap();
        let setup = TalbotSetup::new(clear, 13.5).unwrap();
        let mut g = line_grid(100.0, 1, 16);
        setup.intensity_plane(&mut g, 12_345.0).unwrap();
        for &v in g.data.iter() {
            assert_relative_eq!(v, 1.0, epsilon = 1e-15);
        }
    }

    #[test]
    fn test_carpet_rows_and_revival() {
        let p = 100.0;
        let mut setup =
            TalbotSetup::new(Grating::binary_amplitude(p, 0.5, 9).unwrap(), 13.5).unwrap();
        setup.propagation = Propagation::Paraxial;
        let zt = setup.talbot_length_nm();
        let nz = 33; // rows at z = i·z_T/32
        let dz = zt / 32.0;
        let carpet = setup
            .carpet((0.0, 2.0 * p), 64, (-0.5 * dz, zt + 0.5 * dz), nz, None)
            .unwrap();
        assert_eq!(carpet.data.dim(), (nz, 64));
        let first = carpet.data.row(0).to_vec();
        let last = carpet.data.row(nz - 1).to_vec();
        // Row 0 sits at z = 0 exactly (cell centre of the first z bin).
        assert!(carpet.y_at(0).abs() < 1e-9);
        assert!(max_abs_diff(&first, &last) < 1e-9);
    }

    // ------------------------------------------------------------------- DTL

    #[test]
    fn test_dtl_stationary_equals_brute_force_gap_average() {
        // Paraxial: averaging the coherent image over one Talbot length kills
        // every cross term with n² ≠ n'² (integer cycles). Brute force: 128
        // midpoint gap samples (> max |n² − n'²| = 36).
        let p = 100.0;
        let mut setup =
            TalbotSetup::new(Grating::binary_amplitude(p, 0.3, 6).unwrap(), 13.5).unwrap();
        setup.propagation = Propagation::Paraxial;
        let zt = setup.talbot_length_nm();
        let nx = 40;
        let samples = 128;
        let mut brute = vec![0.0; nx];
        let mut g = line_grid(p, 1, nx);
        for j in 0..samples {
            let z = 7_000.0 + (j as f64 + 0.5) * zt / samples as f64;
            setup.intensity_plane(&mut g, z).unwrap();
            for (b, v) in brute.iter_mut().zip(g.data.row(0)) {
                *b += v / samples as f64;
            }
        }
        let mut stat = line_grid(p, 1, nx);
        setup
            .exposure_image(&mut stat, 7_000.0, &ExposureMode::DisplacementStationary)
            .unwrap();
        let mut scan = line_grid(p, 1, nx);
        setup
            .exposure_image(
                &mut scan,
                7_000.0,
                &ExposureMode::Displacement { scan_length_nm: zt },
            )
            .unwrap();
        assert!(max_abs_diff(&brute, &row(&stat)) < 1e-12);
        assert!(max_abs_diff(&row(&scan), &row(&stat)) < 1e-12);
    }

    #[test]
    fn test_dtl_image_has_half_period_and_closed_form() {
        // I_DTL = Σ|c_n|² + 2 Σ_{n>0} c_n² cos(4πnx/p) for real symmetric c_n.
        let p = 100.0;
        let setup = TalbotSetup::new(Grating::binary_amplitude(p, 0.3, 7).unwrap(), 13.5).unwrap();
        let nx = 64;
        let mut g = line_grid(p, 2, nx);
        setup
            .exposure_image(&mut g, 1e5, &ExposureMode::DisplacementStationary)
            .unwrap();
        let img = row(&g);
        let px = g.pixel_size_x();
        assert_relative_eq!(
            dominant_period_nm(&img, px).unwrap(),
            p / 2.0,
            epsilon = 1e-9
        );
        for (j, &v) in img.iter().enumerate() {
            let x = g.x_at(j);
            let mut expected = setup.grating.total_efficiency();
            for n in 1..=7 {
                let cn = setup.grating.coefficient(n, 0).re;
                expected += 2.0 * cn * cn * (4.0 * PI * n as f64 * x / p).cos();
            }
            assert!((v - expected).abs() < 1e-12);
        }
        // The DTL image is gap independent.
        let mut g2 = line_grid(p, 2, nx);
        setup
            .exposure_image(&mut g2, 3.3e4, &ExposureMode::DisplacementStationary)
            .unwrap();
        assert!(max_abs_diff(&img, &row(&g2)) < 1e-14);
    }

    #[test]
    fn test_pi_phase_grating_dtl_and_zero_order() {
        let p = 200.0;
        let setup = TalbotSetup::new(Grating::binary_phase(p, 0.5, PI, 9).unwrap(), 13.5).unwrap();
        assert!(setup.grating.coefficient(0, 0).norm() < 1e-15);
        let mut g = line_grid(p, 2, 64);
        setup
            .exposure_image(&mut g, 5e4, &ExposureMode::DisplacementStationary)
            .unwrap();
        assert_relative_eq!(
            dominant_period_nm(&row(&g), g.pixel_size_x()).unwrap(),
            p / 2.0,
            epsilon = 1e-9
        );
    }

    #[test]
    fn test_exact_finite_scan_converges_to_stationary() {
        // Exact propagation: a finite scan leaves sinc(δL/2π) residuals that
        // shrink as the scan grows.
        let p = 100.0;
        let setup = TalbotSetup::new(Grating::binary_amplitude(p, 0.5, 7).unwrap(), 13.5).unwrap();
        let zt = setup.talbot_length_exact_nm().unwrap();
        let mut stat = line_grid(p, 1, 32);
        setup
            .exposure_image(&mut stat, 2e4, &ExposureMode::DisplacementStationary)
            .unwrap();
        let err = |l: f64| {
            let mut g = line_grid(p, 1, 32);
            setup
                .exposure_image(
                    &mut g,
                    2e4,
                    &ExposureMode::Displacement { scan_length_nm: l },
                )
                .unwrap();
            max_abs_diff(&row(&g), &row(&stat))
        };
        let (e1, e20) = (err(zt), err(20.0 * zt));
        assert!(e20 < e1, "longer scans must converge: {e1} → {e20}");
        assert!(e20 < 0.02, "20-z_T scan residual {e20}");
    }

    // ------------------------------------------------------------------- ATL

    #[test]
    fn test_atl_flat_top_is_stationary_exactly_at_z_a() {
        // Three-order (sinusoidal) grating, paraxial, flat-top band: the only
        // cross term carries sinc(Δλ z / 2p²), which vanishes at z = z_A.
        let (p, lambda, band) = (100.0, 13.5, 0.3);
        let mut setup =
            TalbotSetup::new(Grating::sinusoidal_amplitude(p, 0.5, 0.4).unwrap(), lambda).unwrap();
        setup.propagation = Propagation::Paraxial;
        let z_a = achromatic_distance_nm(p, band);
        let spectrum = Spectrum::FlatTop {
            bandwidth_nm: band,
            bins: 16,
        };
        let mut stat = line_grid(p, 1, 32);
        setup
            .exposure_image(&mut stat, z_a, &ExposureMode::DisplacementStationary)
            .unwrap();
        let mut atl = line_grid(p, 1, 32);
        setup
            .exposure_image(&mut atl, z_a, &ExposureMode::Achromatic(spectrum.clone()))
            .unwrap();
        assert!(max_abs_diff(&row(&atl), &row(&stat)) < 1e-12);
        // Closed form at z_A/2: I = c0² + 4c1²cos²(kx) + 4c0c1 cos(πλz/p²)·sinc(½)·cos(kx).
        let z = 0.5 * z_a;
        setup
            .exposure_image(&mut atl, z, &ExposureMode::Achromatic(spectrum.clone()))
            .unwrap();
        let (c0, c1) = (0.5, 0.2);
        for (j, &v) in row(&atl).iter().enumerate() {
            let kx = 2.0 * PI * atl.x_at(j) / p;
            let expected = c0 * c0
                + 4.0 * c1 * c1 * kx.cos().powi(2)
                + 4.0 * c0 * c1 * (PI * lambda * z / (p * p)).cos() * sinc(0.5) * kx.cos();
            assert!((v - expected).abs() < 1e-12, "{v} vs {expected}");
        }
        // The bin path (exact propagation) agrees closely with the closed form
        // for this nearly paraxial case.
        let exact = TalbotSetup {
            propagation: Propagation::AngularSpectrum,
            ..setup.clone()
        };
        let mut atl_exact = line_grid(p, 1, 32);
        exact
            .exposure_image(&mut atl_exact, z_a, &ExposureMode::Achromatic(spectrum))
            .unwrap();
        assert!(max_abs_diff(&row(&atl_exact), &row(&stat)) < 5e-3);
    }

    #[test]
    fn test_atl_gaussian_converges_beyond_z_a_only() {
        // Binary grating, exact propagation, Gaussian band FWHM 0.5 nm:
        // z_A = 40 µm. Beyond ~2 z_A the ATL image is the stationary image and
        // no longer depends on the gap; well inside z_A it is not.
        let (p, lambda, fwhm) = (100.0, 13.5, 0.5);
        let setup =
            TalbotSetup::new(Grating::binary_amplitude(p, 0.5, 7).unwrap(), lambda).unwrap();
        let z_a = achromatic_distance_nm(p, fwhm);
        let mode = ExposureMode::Achromatic(Spectrum::Gaussian {
            fwhm_nm: fwhm,
            bins: 256,
        });
        let image = |z: f64, m: &ExposureMode| {
            let mut g = line_grid(p, 1, 32);
            setup.exposure_image(&mut g, z, m).unwrap();
            row(&g)
        };
        let stat = image(z_a, &ExposureMode::DisplacementStationary);
        let far1 = image(2.0 * z_a, &mode);
        let far2 = image(2.5 * z_a, &mode);
        assert!(
            max_abs_diff(&far1, &stat) < 2e-3,
            "{}",
            max_abs_diff(&far1, &stat)
        );
        assert!(max_abs_diff(&far1, &far2) < 2e-3);
        let near = image(0.05 * z_a, &mode);
        assert!(
            max_abs_diff(&near, &stat) > 0.05,
            "{}",
            max_abs_diff(&near, &stat)
        );
    }

    #[test]
    fn test_atl_carpet_washes_out() {
        // Spectrally averaged carpet: the lateral modulation of the
        // non-stationary part decays with z; far rows equal each other.
        let p = 100.0;
        let setup = TalbotSetup::new(Grating::binary_amplitude(p, 0.5, 5).unwrap(), 13.5).unwrap();
        let spectrum = Spectrum::Gaussian {
            fwhm_nm: 1.0,
            bins: 128,
        };
        let z_a = achromatic_distance_nm(p, 1.0);
        let carpet = setup
            .carpet((0.0, p), 32, (3.0 * z_a, 3.2 * z_a), 3, Some(&spectrum))
            .unwrap();
        let r0 = carpet.data.row(0).to_vec();
        let r2 = carpet.data.row(2).to_vec();
        assert!(max_abs_diff(&r0, &r2) < 1e-3);
    }

    // ---------------------------------------------------------------- volumes

    #[test]
    fn test_index_matched_volume_equals_free_space_propagation() {
        let p = 100.0;
        let setup = TalbotSetup::new(Grating::binary_amplitude(p, 0.4, 7).unwrap(), 13.5).unwrap();
        let medium = ResistMedium::default(); // n = 1, α = 0
        let gap = 3_000.0;
        let mut vol = Grid3D::<f64>::new(32, 1, 4, (0.0, p), (-0.5, 0.5), (0.0, 80.0)).unwrap();
        setup
            .resist_volume(&mut vol, gap, &medium, &ExposureMode::Coherent)
            .unwrap();
        for k in 0..4 {
            let mut g = line_grid(p, 1, 32);
            setup.intensity_plane(&mut g, gap + vol.z_at(k)).unwrap();
            let slice: Vec<f64> = (0..32).map(|j| vol.data[[k, 0, j]]).collect();
            assert!(max_abs_diff(&slice, &row(&g)) < 1e-10);
        }
    }

    #[test]
    fn test_dtl_volume_is_depth_invariant_up_to_absorption() {
        // Stationary DTL keeps only equal-|f| pairs, which share the in-resist
        // k_z: with α = 0 every depth slice is identical; paraxial absorption
        // scales it by e^{−αz}.
        let p = 100.0;
        let mut setup =
            TalbotSetup::new(Grating::binary_amplitude(p, 0.5, 5).unwrap(), 13.5).unwrap();
        let mut vol = Grid3D::<f64>::new(16, 1, 5, (0.0, p), (-0.5, 0.5), (0.0, 100.0)).unwrap();
        let clear = ResistMedium {
            index: 0.97,
            absorption_per_nm: 0.0,
        };
        setup
            .resist_volume(&mut vol, 1e4, &clear, &ExposureMode::DisplacementStationary)
            .unwrap();
        for k in 1..5 {
            for j in 0..16 {
                assert!((vol.data[[k, 0, j]] - vol.data[[0, 0, j]]).abs() < 1e-13);
            }
        }
        setup.propagation = Propagation::Paraxial;
        let alpha = 0.004;
        let absorbing = ResistMedium {
            index: 0.97,
            absorption_per_nm: alpha,
        };
        setup
            .resist_volume(
                &mut vol,
                1e4,
                &absorbing,
                &ExposureMode::DisplacementStationary,
            )
            .unwrap();
        for k in 1..5 {
            let ratio = vol.data[[k, 0, 3]] / vol.data[[0, 0, 3]];
            let expected = (-alpha * (vol.z_at(k) - vol.z_at(0))).exp();
            assert_relative_eq!(ratio, expected, epsilon = 1e-12);
        }
    }

    #[test]
    fn test_volume_feeds_pac_and_development() {
        use crate::interference::{expose, ExposureKinetics};
        use crate::volumetric::{develop_depth_map, VolumetricLatentImage};
        let p = 100.0;
        let setup = TalbotSetup::new(Grating::binary_amplitude(p, 0.5, 7).unwrap(), 13.5).unwrap();
        let mut vol =
            Grid3D::<f64>::new(32, 2, 8, (0.0, 2.0 * p), (0.0, 10.0), (0.0, 60.0)).unwrap();
        let medium = ResistMedium {
            index: 0.97,
            absorption_per_nm: 0.004,
        };
        setup
            .resist_volume(
                &mut vol,
                5e4,
                &medium,
                &ExposureMode::DisplacementStationary,
            )
            .unwrap();
        let pac = expose(&vol, 20.0, 0.1, ExposureKinetics::OnePhoton);
        assert!(pac.data.iter().all(|m| (0.0..=1.0).contains(m)));
        let depth = develop_depth_map(&VolumetricLatentImage { pac }, 0.5);
        let max = depth.data.iter().cloned().fold(0.0, f64::max);
        let min = depth.data.iter().cloned().fold(f64::INFINITY, f64::min);
        assert!(
            max > min,
            "the DTL grating must print a modulated depth map"
        );
    }

    // ----------------------------------------------------------------- EUV-IL

    #[test]
    fn test_euv_il_fringe_period_is_wavelength_independent() {
        for (lambda, order, expected) in [(13.5, 1u32, 50.0), (6.7, 1, 50.0), (13.5, 2, 25.0)] {
            let il =
                TwoGratingInterference::from_efficiencies(100.0, lambda, order, 0.1, 0.1).unwrap();
            assert_relative_eq!(il.fringe_period_nm(), expected, epsilon = 1e-12);
            let setup = il.to_interference_setup(0.97, 0.0).unwrap();
            // 8 fringes across the grid so the DFT bin is exact.
            let span = 8.0 * expected;
            let mut grid =
                Grid3D::<f64>::new(128, 1, 1, (0.0, span), (0.0, 1.0), (0.0, 1.0)).unwrap();
            setup.intensity(&mut grid);
            let prof: Vec<f64> = (0..128).map(|j| grid.data[[0, 0, j]]).collect();
            assert_relative_eq!(
                dominant_period_nm(&prof, span / 128.0).unwrap(),
                expected,
                epsilon = 1e-9
            );
        }
        assert_relative_eq!(
            TwoGratingInterference::from_efficiencies(100.0, 13.5, 1, 1.0, 1.0)
                .unwrap()
                .diffraction_angle_deg(),
            7.758_619_888_586_816,
            epsilon = 1e-12
        );
        // Evanescent order rejected.
        assert!(TwoGratingInterference::from_efficiencies(100.0, 60.0, 2, 1.0, 1.0).is_err());
    }

    #[test]
    fn test_euv_il_visibility_formula() {
        let il = TwoGratingInterference::from_efficiencies(100.0, 13.5, 1, 1.0, 0.25).unwrap();
        assert_relative_eq!(il.visibility(), 0.8, epsilon = 1e-15);
        let xs: Vec<f64> = (0..200).map(|j| j as f64 * 0.25).collect();
        let vals: Vec<f64> = xs.iter().map(|&x| il.intensity_at(x)).collect();
        let (mx, mn) = (
            vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            vals.iter().cloned().fold(f64::INFINITY, f64::min),
        );
        assert_relative_eq!((mx - mn) / (mx + mn), 0.8, epsilon = 1e-12);
        // The InterferenceSetup reproduces the analytic fringes (n = 1, α = 0;
        // the common longitudinal phase cancels).
        let setup = il.to_interference_setup(1.0, 0.0).unwrap();
        let mut grid = Grid3D::<f64>::new(64, 1, 1, (0.0, 100.0), (0.0, 1.0), (0.0, 1e-3)).unwrap();
        setup.intensity(&mut grid);
        for j in 0..64 {
            assert!((grid.data[[0, 0, j]] - il.intensity_at(grid.x_at(j))).abs() < 1e-9);
        }
    }

    #[test]
    fn test_euv_il_beam_intensities_from_grating() {
        let amp = Grating::binary_amplitude(100.0, 0.5, 5).unwrap();
        let il = TwoGratingInterference::new(&amp, 13.5, 1).unwrap();
        assert_relative_eq!(
            il.beam_intensity_1,
            0.101_321_183_642_337_78,
            epsilon = 1e-15
        );
        assert_relative_eq!(il.visibility(), 1.0, epsilon = 1e-15);
        let phase = Grating::binary_phase(100.0, 0.5, PI, 5).unwrap();
        let il = TwoGratingInterference::new(&phase, 13.5, 1).unwrap();
        assert_relative_eq!(
            il.beam_intensity_2,
            0.405_284_734_569_351_16,
            epsilon = 1e-15
        );
    }

    #[test]
    fn test_invalid_inputs_rejected() {
        assert!(Grating::binary_amplitude(0.0, 0.5, 3).is_err());
        assert!(Grating::binary_amplitude(100.0, 1.5, 3).is_err());
        assert!(Grating::binary_amplitude(100.0, 0.5, 10_000).is_err());
        assert!(Grating::hole_array_square(100.0, 120.0, 3).is_err());
        assert!(Grating::hole_array_hexagonal(100.0, 0.0, 3).is_err());
        assert!(Grating::from_coefficients(100.0, &[]).is_err());
        let g = Grating::binary_amplitude(100.0, 0.5, 3).unwrap();
        assert!(TalbotSetup::new(g.clone(), -1.0).is_err());
        let setup = TalbotSetup::new(g.clone(), 13.5).unwrap();
        let mut grid = line_grid(100.0, 1, 8);
        assert!(setup.intensity_plane(&mut grid, -5.0).is_err());
        assert!(setup
            .exposure_image(
                &mut grid,
                10.0,
                &ExposureMode::Displacement {
                    scan_length_nm: -1.0
                }
            )
            .is_err());
        assert!(setup
            .exposure_image(
                &mut grid,
                10.0,
                &ExposureMode::Achromatic(Spectrum::Lines(vec![]))
            )
            .is_err());
        let mut vol = Grid3D::<f64>::new(8, 1, 2, (0.0, 100.0), (0.0, 1.0), (-5.0, 5.0)).unwrap();
        assert!(setup
            .resist_volume(
                &mut vol,
                10.0,
                &ResistMedium::default(),
                &ExposureMode::Coherent
            )
            .is_err());
        assert!(TwoGratingInterference::new(
            &Grating::hole_array_square(100.0, 50.0, 2).unwrap(),
            13.5,
            1
        )
        .is_err());
        // Order not retained by the truncation (max_order = 3).
        assert!(TwoGratingInterference::new(&g, 13.5, 4).is_err());
        let il = TwoGratingInterference::from_efficiencies(100.0, 13.5, 1, 1.0, 1.0).unwrap();
        assert!(il.to_interference_setup(0.1, 0.0).is_err()); // sin θ / n > 1
    }

    #[test]
    fn test_hex_lattice_images_are_six_fold_symmetric() {
        // Hexagonal hole array centred on a hole at the origin: the Fourier
        // shells are six-fold symmetric and the propagator depends on |f|
        // only, so coherent and DTL images are invariant under a 60° rotation
        // about the hole (and under lattice translations).
        let a = 100.0;
        let setup =
            TalbotSetup::new(Grating::hole_array_hexagonal(a, 50.0, 3).unwrap(), 13.5).unwrap();
        let at = |x: f64, y: f64, mode: &ExposureMode| {
            let mut g =
                Grid2D::<f64>::new(1, 1, (x - 1e-3, x + 1e-3), (y - 1e-3, y + 1e-3)).unwrap();
            setup.exposure_image(&mut g, 4e4, mode).unwrap();
            g.data[[0, 0]]
        };
        let (s60, c60) = (PI / 3.0).sin_cos();
        for mode in [ExposureMode::Coherent, ExposureMode::DisplacementStationary] {
            for &(x, y) in &[(13.0, 7.0), (31.0, -22.0), (-40.0, 5.5)] {
                let v = at(x, y, &mode);
                let rotated = at(c60 * x - s60 * y, s60 * x + c60 * y, &mode);
                assert!((v - rotated).abs() < 1e-12, "{mode:?}: {v} vs {rotated}");
                let translated = at(x + 0.5 * a, y + 0.5 * 3.0_f64.sqrt() * a, &mode);
                assert!((v - translated).abs() < 1e-12);
            }
        }
    }
}
