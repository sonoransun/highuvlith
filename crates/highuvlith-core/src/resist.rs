//! Photoresist exposure, post-exposure bake, and development.
//!
//! Turns an aerial (or standing-wave) intensity image into a developed resist
//! profile in three steps. Exposure uses the Dill model: the normalized
//! photo-active-compound concentration bleaches as `m = exp(-C · dose · I)`,
//! with bleachable (A) and non-bleachable (B) absorption setting a
//! depth-averaged Beer–Lambert coupling factor on the incident intensity.
//! Post-exposure bake diffuses the latent image as a separable Gaussian blur
//! whose standard deviation is the acid diffusion length. Development converts
//! `m` to an etch rate via the Mack model (with an `n → 1` singularity guard)
//! or a simple threshold, and [`develop`] etches vertically for the profile.
//!
//! Two post-exposure-bake (PEB) upgrades operate on z-resolved volumes
//! (`Array3` with axis order `(z, y, x)`, shared with [`crate::volumetric`]):
//! exact spectral diffusion with separate lateral and vertical diffusion
//! lengths and an optional depth-dependent vertical diffusivity
//! ([`peb_diffuse_anisotropic`]), and a chemically amplified resist (CAR)
//! acid/quencher reaction–diffusion bake ([`car_peb`]; single-slice
//! convenience [`car_peb_2d`] for the 2D latent image).
//!
//! # Key equations
//!
//! ```text
//!   Dill:  m(x,y) = exp(-C · dose · I_eff),   I_eff = coupling · I
//!   Mack:  R(m) = Rmax · (a+1)(1-m)^n / (a + (1-m)^n) + Rmin
//!          a = (n+1)/(n-1) · (1 - m_th)^n
//!
//!   Gaussian PEB, per axis (exact):   ĉ(f) ← ĉ(f) · exp(−2π² σ² f²),   σ² = 2 D t
//!   Lattice kernel (CAR diffusion):   ĉ_k  ← ĉ_k · exp(−(σ/h)² (1 − cos 2πk/N))
//!   Depth-dependent D_z(z) = D_ref·s(z):  c ← exp((σ_z²/2) · L_s) c,
//!        L_s = cell-centred finite-volume ∂z s(z) ∂z, zero flux at top/bottom
//!
//!   CAR PEB (concentrations normalized to the initial PAG concentration):
//!        ∂m/∂t = −k_amp · m · h                  (acid-catalysed deprotection)
//!        ∂h/∂t = D_h ∇²h − k_q · h · q          (acid diffusion + neutralization)
//!        ∂q/∂t = D_q ∇²q − k_q · h · q          (base-quencher diffusion)
//!        h(0) = 1 − m_exposure,  q(0) = q0,  m(0) = 1
//!   Exact local reaction over Δt (c = h − q is conserved, a = |c|,
//!   F = 1 − e^{−k_q a Δt}, φ = F / (k_q a Δt)):
//!        c ≥ 0:  h' = h / (1 + q k_q Δt φ),        q' = q e^{−k_q a Δt} / (1 + q k_q Δt φ)
//!                ∫h dt = a Δt + ln(1 + q k_q Δt φ) / k_q
//!        c < 0:  h' = h e^{−k_q a Δt} / (1 + h k_q Δt φ),  q' = q / (1 + h k_q Δt φ)
//!                ∫h dt = ln(1 + h k_q Δt φ) / k_q
//!        m' = m · exp(−k_amp ∫h dt)
//! ```
//!
//! # Model status
//!
//! The 2D path is depth-averaged. Exposure applies a single Beer–Lambert
//! coupling scalar with no z-resolved dose, and [`develop`] etches only the
//! center row vertically (`height = thickness − rate · time`), so there is no
//! lateral development front or sidewall angle. The z-resolved path is
//! [`crate::volumetric`].
//!
//! The PEB helpers are exact in the following sense. [`gaussian_diffuse_axis`]
//! multiplies the spectrum by the continuous Gaussian transfer function — the
//! exact heat-equation solution for the band-limited (trigonometric)
//! interpolant of the samples, with periodic or zero-flux (even-extension)
//! boundaries. For σ below ~1.5 samples and sharp data the band-limited kernel
//! has small negative side lobes (a unit impulse undershoots by −1.8·10⁻² at
//! σ = 0.5 sample, −1.6·10⁻⁴ at σ = 1, −2.3·10⁻⁷ at σ = 1.5, −2.6·10⁻¹¹ at
//! σ = 2); smooth, well-sampled fields are unaffected.
//! [`lattice_diffuse_axis`] instead applies the exact
//! solution of the three-point semi-discrete diffusion equation (the discrete
//! analogue of the Gaussian, `e^{−t} I_n(t)` with `t = (σ/h)²`), which is
//! strictly positivity-preserving with variance exactly σ², at the price of
//! an O(h²) dispersion error for barely resolved spatial frequencies. The
//! depth-dependent vertical diffusivity is solved exactly in time on the
//! finite-volume grid (harmonic-mean face diffusivities). Lateral (constant D)
//! and vertical operators commute, so the axis-by-axis split is exact.
//!
//! The CAR bake is the standard Mack/PROLITH-class chemically amplified
//! resist model. Approximations: concentrations are normalized to the
//! initial photo-acid-generator (PAG) concentration; deprotection is first
//! order in acid; there is no acid evaporation or trapping loss; the
//! diffusivities are constant (no deprotection- or free-volume-dependent
//! diffusivity); the time integration is Strang operator splitting —
//! lattice-exact diffusion half-steps around an exact local reaction and
//! deprotection step — with O(Δt²) splitting error (both sub-steps are
//! unconditionally stable, so the step limit is an accuracy limit, not a
//! stability limit). Default [`CarParams`] values are illustrative, not
//! fitted to a specific resist.
//!
//! # References
//!
//! - Dill et al., IEEE Trans. Electron Devices (1975) — exposure kinetics.
//! - Mack, "Development of positive photoresists" (1987) — development rate.
//! - C. A. Mack, *Fundamental Principles of Optical Lithography*, Wiley
//!   (2007) — PEB diffusion and the chemically amplified resist
//!   reaction–diffusion model.

use std::sync::Arc;

use ndarray::{Array2, Array3, Axis, Zip};
use num::Complex;
use rustfft::{Fft, FftPlanner};
use serde::{Deserialize, Serialize};

use crate::error::{LithographyError, Result};

/// Development rate model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DevelopmentModel {
    /// Simple threshold model: developed if PAC < threshold.
    Threshold { threshold: f64 },
    /// Mack development model:
    /// R(m) = Rmax * (a + 1) * (1 - m)^n / (a + (1-m)^n) + Rmin
    /// where m = normalized PAC concentration, a = (n+1)/(n-1) * (1 - mth)^n
    Mack {
        /// Maximum development rate (nm/s).
        rmax: f64,
        /// Minimum development rate (nm/s).
        rmin: f64,
        /// Threshold PAC concentration.
        mth: f64,
        /// Development selectivity.
        n: f64,
    },
}

impl DevelopmentModel {
    /// Development rate (nm/s) at normalized PAC concentration `m`.
    pub fn rate(&self, m: f64) -> f64 {
        match self {
            DevelopmentModel::Threshold { threshold } => {
                if m < *threshold {
                    1000.0 // fast development (exposed)
                } else {
                    0.01 // minimal development (unexposed)
                }
            }
            DevelopmentModel::Mack { rmax, rmin, mth, n } => {
                let m_clamped = m.clamp(0.0, 1.0);
                if (n - 1.0).abs() < 1e-12 {
                    // Fallback for n=1 singularity: linear interpolation
                    rmax * (1.0 - m_clamped) + rmin
                } else {
                    let a = (n + 1.0) / (n - 1.0) * (1.0 - mth).powf(*n);
                    let one_minus_m_n = (1.0 - m_clamped).powf(*n);
                    rmax * (a + 1.0) * one_minus_m_n / (a + one_minus_m_n) + rmin
                }
            }
        }
    }
}

impl Default for DevelopmentModel {
    fn default() -> Self {
        Self::Mack {
            rmax: 100.0,
            rmin: 0.1,
            mth: 0.5,
            n: 3.0,
        }
    }
}

/// Photoresist parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResistParams {
    /// Resist thickness in nm.
    pub thickness_nm: f64,
    /// Dill A parameter: bleachable absorption (1/um).
    /// VUV fluoropolymer resists: ~0.1-0.3 /um (much lower than DUV).
    pub dill_a: f64,
    /// Dill B parameter: non-bleachable absorption (1/um).
    /// VUV: ~0.3-0.6 /um.
    pub dill_b: f64,
    /// Dill C parameter: exposure rate constant (cm^2/mJ).
    pub dill_c: f64,
    /// Post-exposure bake (PEB) diffusion length in nm.
    pub peb_diffusion_nm: f64,
    /// Development model.
    pub development: DevelopmentModel,
}

impl ResistParams {
    /// Default VUV fluoropolymer resist parameters.
    pub fn vuv_fluoropolymer() -> Self {
        Self {
            thickness_nm: 150.0,
            dill_a: 0.2,  // Low bleachable absorption (fluoropolymer)
            dill_b: 0.45, // Moderate non-bleachable absorption
            dill_c: 0.02,
            peb_diffusion_nm: 30.0,
            development: DevelopmentModel::default(),
        }
    }

    /// Compute absorption coefficient alpha (1/nm) at a given PAC concentration m.
    /// alpha = A*m + B (in 1/um, convert to 1/nm)
    pub fn absorption(&self, m: f64) -> f64 {
        (self.dill_a * m + self.dill_b) * 1e-3 // convert from 1/um to 1/nm
    }
}

impl Default for ResistParams {
    fn default() -> Self {
        Self::vuv_fluoropolymer()
    }
}

/// Latent image: normalized PAC (photo-active compound) concentration after exposure.
/// m = 1.0 means unexposed, m = 0.0 means fully exposed.
pub struct LatentImage {
    /// PAC concentration at each (y, x) grid point (depth-averaged).
    pub pac: Array2<f64>,
}

/// 1D resist profile after development.
pub struct ResistProfile {
    /// x positions in nm.
    pub x_nm: Vec<f64>,
    /// Remaining resist height at each x position.
    pub height_nm: Vec<f64>,
    /// Original resist thickness.
    pub thickness_nm: f64,
}

/// Compute the latent image from an aerial image and exposure dose.
///
/// Uses the Dill exposure model:
///   m(x,y) = exp(-C * dose * I(x,y))
/// where I is the aerial image intensity.
pub fn expose(aerial_image: &Array2<f64>, dose_mj_cm2: f64, params: &ResistParams) -> LatentImage {
    let pac = aerial_image.mapv(|intensity| {
        // Beer-Lambert through resist (depth-averaged approximation)
        let effective_intensity = intensity * effective_coupling(params);
        // Dill first-order model
        (-params.dill_c * dose_mj_cm2 * effective_intensity).exp()
    });

    LatentImage { pac }
}

/// Apply post-exposure bake diffusion (Gaussian blur of latent image).
pub fn peb_diffuse(latent: &mut LatentImage, diffusion_nm: f64, pixel_nm: f64) {
    if diffusion_nm <= 0.0 {
        return;
    }

    let sigma_pixels = diffusion_nm / pixel_nm;
    let kernel_radius = (3.0 * sigma_pixels).ceil() as usize;
    let kernel_size = 2 * kernel_radius + 1;

    // Build 1D Gaussian kernel
    let mut kernel = vec![0.0; kernel_size];
    let mut sum = 0.0;
    for (i, k_val) in kernel.iter_mut().enumerate().take(kernel_size) {
        let d = i as f64 - kernel_radius as f64;
        *k_val = (-d * d / (2.0 * sigma_pixels * sigma_pixels)).exp();
        sum += *k_val;
    }
    for k in &mut kernel {
        *k /= sum;
    }

    let (ny, nx) = latent.pac.dim();

    // Separable convolution: first along x, then along y
    let mut temp = Array2::zeros((ny, nx));

    // Convolve along x
    for i in 0..ny {
        for j in 0..nx {
            let mut val = 0.0;
            for (k, &k_val) in kernel.iter().enumerate().take(kernel_size) {
                let jj = j as i64 + k as i64 - kernel_radius as i64;
                let jj = jj.clamp(0, nx as i64 - 1) as usize;
                val += latent.pac[[i, jj]] * k_val;
            }
            temp[[i, j]] = val;
        }
    }

    // Convolve along y
    for i in 0..ny {
        for j in 0..nx {
            let mut val = 0.0;
            for (k, &k_val) in kernel.iter().enumerate().take(kernel_size) {
                let ii = i as i64 + k as i64 - kernel_radius as i64;
                let ii = ii.clamp(0, ny as i64 - 1) as usize;
                val += temp[[ii, j]] * k_val;
            }
            latent.pac[[i, j]] = val;
        }
    }
}

/// Compute development rate at each point from the latent image.
pub fn development_rate(latent: &LatentImage, params: &ResistParams) -> Array2<f64> {
    latent.pac.mapv(|m| params.development.rate(m))
}

/// Simulate development to extract the resist profile along the x-axis.
/// Uses a simple vertical development model (1D at each x position).
pub fn develop(
    latent: &LatentImage,
    params: &ResistParams,
    dev_time_s: f64,
    pixel_nm: f64,
) -> ResistProfile {
    let rate = development_rate(latent, params);
    let (ny, nx) = rate.dim();
    let center_row = ny / 2;

    let x_nm: Vec<f64> = (0..nx)
        .map(|j| {
            let field = nx as f64 * pixel_nm;
            -field / 2.0 + (j as f64 + 0.5) * pixel_nm
        })
        .collect();

    // Simple vertical development: if average rate * time > thickness, fully developed
    let height_nm: Vec<f64> = (0..nx)
        .map(|j| {
            let r = rate[[center_row, j]];
            let developed = r * dev_time_s;
            (params.thickness_nm - developed).max(0.0)
        })
        .collect();

    ResistProfile {
        x_nm,
        height_nm,
        thickness_nm: params.thickness_nm,
    }
}

/// Effective coupling factor accounting for depth-averaged absorption in resist.
fn effective_coupling(params: &ResistParams) -> f64 {
    let alpha = params.absorption(1.0); // initial absorption (m=1)
    let thickness = params.thickness_nm;

    if alpha * thickness < 0.01 {
        return 1.0; // optically thin
    }

    // Depth-averaged intensity: (1 - exp(-alpha*d)) / (alpha * d)
    (1.0 - (-alpha * thickness).exp()) / (alpha * thickness)
}

// ============================================================================
// Exact spectral PEB diffusion
// ============================================================================

/// Per-axis boundary condition for the spectral diffusion helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffusionBoundary {
    /// Periodic wrap-around (plain FFT) — matches the periodic field of the
    /// FFT aerial-imaging engine.
    #[default]
    Periodic,
    /// Zero-flux (reflecting) walls at both ends of the axis, via the
    /// half-sample-symmetric even extension of length `2n` (the DCT-II
    /// eigenbasis of the Neumann problem).
    Reflecting,
}

/// Which spectral transfer function a diffusion pass applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpectralKernel {
    /// Continuous Gaussian `exp(−2π²σ²f²)`.
    Continuous,
    /// Exact three-point lattice heat kernel `exp(−(σ/h)²(1 − cos 2πk/N))`.
    Lattice,
}

/// A prepared exact diffusion pass along one axis: FFT plans plus the real,
/// even transfer function (with the `1/N` inverse-FFT normalization folded in).
struct AxisDiffuser {
    axis: usize,
    n: usize,
    reflecting: bool,
    transfer: Vec<f64>,
    forward: Arc<dyn Fft<f64>>,
    inverse: Arc<dyn Fft<f64>>,
}

impl AxisDiffuser {
    /// Prepare a pass over lanes of length `n`; `None` when it is the identity
    /// (fewer than two samples, σ ≤ 0 or NaN, or a non-positive spacing).
    fn new(
        planner: &mut FftPlanner<f64>,
        n: usize,
        axis: usize,
        sigma_nm: f64,
        spacing_nm: f64,
        boundary: DiffusionBoundary,
        kernel: SpectralKernel,
    ) -> Option<Self> {
        if n < 2 || sigma_nm.is_nan() || sigma_nm <= 0.0 || spacing_nm.is_nan() || spacing_nm <= 0.0
        {
            return None;
        }
        let reflecting = boundary == DiffusionBoundary::Reflecting;
        let n_fft = if reflecting { 2 * n } else { n };
        let norm = 1.0 / n_fft as f64;
        let s = sigma_nm / spacing_nm; // σ in samples
        let transfer = (0..n_fft)
            .map(|k| {
                if k == 0 {
                    // DC passes unchanged (mass conservation); also keeps an
                    // infinite σ well defined (the line relaxes to its mean).
                    return norm;
                }
                let kk = if k <= n_fft / 2 {
                    k as f64
                } else {
                    k as f64 - n_fft as f64
                };
                let exponent = match kernel {
                    SpectralKernel::Continuous => {
                        let f = kk / n_fft as f64; // cycles per sample
                        2.0 * std::f64::consts::PI.powi(2) * s * s * f * f
                    }
                    SpectralKernel::Lattice => {
                        // 1 − cos x = 2 sin²(x/2), without cancellation.
                        let half = std::f64::consts::PI * kk / n_fft as f64;
                        2.0 * s * s * half.sin().powi(2)
                    }
                };
                (-exponent).exp() * norm
            })
            .collect();
        Some(Self {
            axis,
            n,
            reflecting,
            transfer,
            forward: planner.plan_fft_forward(n_fft),
            inverse: planner.plan_fft_inverse(n_fft),
        })
    }

    /// Apply the pass to every lane of `data` along the prepared axis. Two
    /// real lanes are packed into one complex FFT (real and imaginary parts);
    /// this is exact because the transfer function is real and even.
    fn apply(
        &self,
        data: &mut Array3<f64>,
        buf: &mut Vec<Complex<f64>>,
        scratch: &mut Vec<Complex<f64>>,
    ) {
        debug_assert_eq!(data.len_of(Axis(self.axis)), self.n);
        let n = self.n;
        let n_fft = self.transfer.len();
        let zero = Complex::new(0.0, 0.0);
        buf.clear();
        buf.resize(n_fft, zero);
        let need = self
            .forward
            .get_inplace_scratch_len()
            .max(self.inverse.get_inplace_scratch_len());
        if scratch.len() < need {
            scratch.resize(need, zero);
        }

        let mut lanes = data.lanes_mut(Axis(self.axis)).into_iter();
        while let Some(mut a) = lanes.next() {
            let mut b = lanes.next();
            match b.as_ref() {
                Some(bv) => {
                    for ((dst, &x), &y) in buf.iter_mut().zip(a.iter()).zip(bv.iter()) {
                        *dst = Complex::new(x, y);
                    }
                }
                None => {
                    for (dst, &x) in buf.iter_mut().zip(a.iter()) {
                        *dst = Complex::new(x, 0.0);
                    }
                }
            }
            if self.reflecting {
                let (lo, hi) = buf.split_at_mut(n);
                for (dst, src) in hi.iter_mut().zip(lo.iter().rev()) {
                    *dst = *src;
                }
            }
            self.forward
                .process_with_scratch(&mut buf[..], &mut scratch[..need]);
            for (v, &g) in buf.iter_mut().zip(&self.transfer) {
                *v *= g;
            }
            self.inverse
                .process_with_scratch(&mut buf[..], &mut scratch[..need]);
            for (x, v) in a.iter_mut().zip(buf.iter()) {
                *x = v.re;
            }
            if let Some(bv) = b.as_mut() {
                for (y, v) in bv.iter_mut().zip(buf.iter()) {
                    *y = v.im;
                }
            }
        }
    }
}

/// Shared driver for the two public single-axis diffusion passes.
fn diffuse_axis_with(
    data: &mut Array3<f64>,
    axis: usize,
    sigma_nm: f64,
    spacing_nm: f64,
    boundary: DiffusionBoundary,
    kernel: SpectralKernel,
) {
    assert!(axis < 3, "axis must be 0 (z), 1 (y) or 2 (x); got {axis}");
    let n = data.len_of(Axis(axis));
    let mut planner = FftPlanner::new();
    if let Some(pass) = AxisDiffuser::new(
        &mut planner,
        n,
        axis,
        sigma_nm,
        spacing_nm,
        boundary,
        kernel,
    ) {
        pass.apply(data, &mut Vec::new(), &mut Vec::new());
    }
}

/// Exact Gaussian diffusion of `data` along one axis.
///
/// Multiplies the spectrum of every lane along `axis` (0 = z, 1 = y, 2 = x
/// for the `(nz, ny, nx)` convention) by the continuous Gaussian transfer
/// function `exp(−2π²σ²f²)`, where `σ = sqrt(2·D·t)` is the diffusion length
/// in nm and `spacing_nm` the sample pitch along that axis. With
/// [`DiffusionBoundary::Periodic`] the lane is transformed as-is; with
/// [`DiffusionBoundary::Reflecting`] it is first mirrored into its
/// half-sample-symmetric extension of length `2n`, which imposes zero flux
/// at both ends. The result is the exact heat-equation solution for the
/// band-limited interpolant of the samples: the lane mean is conserved and a
/// narrow peak spreads with variance growth σ² (within 3·10⁻¹⁰ relative at
/// σ = 2 samples, roundoff-level beyond σ ≈ 2.5). For σ below ~1.5
/// samples, sharp data acquire small negative side lobes (see the module
/// docs); use [`lattice_diffuse_axis`] when strict positivity matters.
///
/// A non-positive or NaN `sigma_nm` or `spacing_nm`, or an axis of length
/// < 2, leaves `data` unchanged; an infinite σ relaxes every lane to its
/// mean.
///
/// # Panics
///
/// Panics if `axis > 2`.
pub fn gaussian_diffuse_axis(
    data: &mut Array3<f64>,
    axis: usize,
    sigma_nm: f64,
    spacing_nm: f64,
    boundary: DiffusionBoundary,
) {
    diffuse_axis_with(
        data,
        axis,
        sigma_nm,
        spacing_nm,
        boundary,
        SpectralKernel::Continuous,
    );
}

/// Exact lattice diffusion of `data` along one axis: the exact-in-time
/// solution of the three-point semi-discrete diffusion equation
/// `dc_j/dt = D (c_{j+1} − 2c_j + c_{j−1}) / h²` with `σ² = 2·D·t`.
///
/// The transfer function is `exp(−(σ/h)²(1 − cos 2πk/N))`; the equivalent
/// real-space kernel is the discrete analogue of the Gaussian,
/// `e^{−t} I_n(t)` with `t = (σ/h)²` (`I_n` the modified Bessel function).
/// It is strictly positive, conserves mass, composes exactly
/// (σ₁ then σ₂ ≡ `sqrt(σ₁² + σ₂²)`), and grows the variance of a narrow peak
/// by exactly σ² for any σ — at the price of an O(h²) dispersion error for
/// barely resolved spatial frequencies relative to
/// [`gaussian_diffuse_axis`]. The reflecting boundary equals the zero-flux
/// finite-volume boundary (mirrored ghost cells). Used by [`car_peb`], whose
/// reaction step needs non-negative concentrations.
///
/// Same argument conventions, no-op cases, and panics as
/// [`gaussian_diffuse_axis`].
pub fn lattice_diffuse_axis(
    data: &mut Array3<f64>,
    axis: usize,
    sigma_nm: f64,
    spacing_nm: f64,
    boundary: DiffusionBoundary,
) {
    diffuse_axis_with(
        data,
        axis,
        sigma_nm,
        spacing_nm,
        boundary,
        SpectralKernel::Lattice,
    );
}

/// Anisotropic post-exposure-bake diffusion of a z-resolved volume.
///
/// The lateral (x, y) and vertical (z) diffusion lengths are independent,
/// `σ = sqrt(2·D·t)` along each axis. The vertical axis is always
/// zero-flux (resist/air top and resist/substrate bottom); the lateral
/// boundary is selectable. With `vertical_diffusivity_scale = Some(s)` the
/// vertical diffusivity varies with depth as `D_z(z_k) = D_ref · s_k`, where
/// `vertical_nm² = 2·D_ref·t`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PebDiffusion {
    /// Lateral (x and y) diffusion length `σ_xy = sqrt(2·D_xy·t)` in nm.
    pub lateral_nm: f64,
    /// Vertical (z) diffusion length `σ_z = sqrt(2·D_ref·t)` in nm.
    pub vertical_nm: f64,
    /// Optional per-slice diffusivity multiplier `s_k = D_z(z_k) / D_ref`
    /// (length `nz`, every entry finite and > 0). `None` = uniform.
    #[serde(default)]
    pub vertical_diffusivity_scale: Option<Vec<f64>>,
    /// Boundary for the lateral axes (z is always reflecting).
    #[serde(default)]
    pub lateral_boundary: DiffusionBoundary,
}

impl PebDiffusion {
    /// Isotropic diffusion: the same length laterally and vertically
    /// (periodic lateral boundary, uniform vertical diffusivity).
    pub fn isotropic(length_nm: f64) -> Self {
        Self::anisotropic(length_nm, length_nm)
    }

    /// Separate lateral and vertical diffusion lengths (periodic lateral
    /// boundary, uniform vertical diffusivity).
    pub fn anisotropic(lateral_nm: f64, vertical_nm: f64) -> Self {
        Self {
            lateral_nm,
            vertical_nm,
            vertical_diffusivity_scale: None,
            lateral_boundary: DiffusionBoundary::Periodic,
        }
    }

    /// Attach a parametric depth profile of the vertical diffusivity,
    /// `s(z) = 1 + (surface_ratio − 1)·exp(−z / decay_length_nm)`, sampled
    /// at the `nz` slice centres `z_k = (k + ½)·dz_nm` (z from the resist
    /// top). `surface_ratio > 1` models enhanced acid mobility near the top
    /// surface, `< 1` a denser skin; this is a convenience parametrization,
    /// not a fitted physical law.
    pub fn with_exponential_depth_profile(
        mut self,
        nz: usize,
        dz_nm: f64,
        surface_ratio: f64,
        decay_length_nm: f64,
    ) -> Self {
        let profile = (0..nz)
            .map(|k| {
                let z = (k as f64 + 0.5) * dz_nm;
                1.0 + (surface_ratio - 1.0) * (-z / decay_length_nm).exp()
            })
            .collect();
        self.vertical_diffusivity_scale = Some(profile);
        self
    }
}

/// Validate that a length is finite and non-negative.
fn check_non_negative(name: &'static str, value: f64) -> Result<()> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(LithographyError::InvalidParameter {
            name,
            value,
            reason: "must be finite and non-negative",
        })
    }
}

/// Validate a `[dz, dy, dx]` sample spacing.
fn check_spacing(spacing_nm: [f64; 3]) -> Result<()> {
    for value in spacing_nm {
        if !(value.is_finite() && value > 0.0) {
            return Err(LithographyError::InvalidParameter {
                name: "spacing_nm",
                value,
                reason: "every sample spacing must be finite and positive",
            });
        }
    }
    Ok(())
}

/// Exact-in-time propagator `exp(τ·L_s)`, `τ = σ_z²/2`, of the zero-flux,
/// cell-centred finite-volume operator `L_s = ∂z s(z) ∂z` (harmonic-mean face
/// diffusivities), returned row-major as an `nz × nz` matrix.
///
/// Computed by uniformization + scaling and squaring: with
/// `c = max_k |L_kk|`, `L + cI` is entrywise non-negative, so
/// `exp(hL) = e^{−hc} Σ_j (h(L + cI))^j / j!` sums non-negative terms only
/// (`h = τ/2^s`, `‖h(L + cI)‖∞ ≤ ½`, series to < 10⁻²⁰), and `s` squarings
/// recover `exp(τL)`. Every operation keeps the matrix entrywise
/// non-negative in floating point, so the propagator is positivity-preserving
/// and its rows sum to 1 (mass conservation) to roundoff.
fn vertical_propagator(scale: &[f64], dz_nm: f64, sigma_z_nm: f64) -> Vec<f64> {
    use nalgebra::DMatrix;
    let nz = scale.len();
    let inv_dz2 = 1.0 / (dz_nm * dz_nm);
    // Face conductances between slices k and k+1 (harmonic-mean diffusivity).
    let faces: Vec<f64> = (0..nz.saturating_sub(1))
        .map(|k| {
            let (a, b) = (scale[k], scale[k + 1]);
            2.0 * a * b / (a + b) * inv_dz2
        })
        .collect();
    let diag: Vec<f64> = (0..nz)
        .map(|k| {
            let above = if k > 0 { faces[k - 1] } else { 0.0 };
            let below = faces.get(k).copied().unwrap_or(0.0);
            -(above + below)
        })
        .collect();
    let c = diag.iter().fold(0.0_f64, |acc, d| acc.max(-d));
    let tau = 0.5 * sigma_z_nm * sigma_z_nm;
    let norm = 2.0 * c * tau; // ≥ ‖τ(L + cI)‖∞
    let squarings = if norm > 0.5 {
        (norm / 0.5).log2().ceil() as i32
    } else {
        0
    };
    let h = tau / 2.0_f64.powi(squarings);

    let mut shifted = DMatrix::<f64>::zeros(nz, nz); // h (L + cI) ≥ 0
    for (k, d) in diag.iter().enumerate() {
        shifted[(k, k)] = h * (d + c);
    }
    for (k, w) in faces.iter().enumerate() {
        shifted[(k, k + 1)] = h * w;
        shifted[(k + 1, k)] = h * w;
    }
    let mut term = DMatrix::<f64>::identity(nz, nz);
    let mut propagator = term.clone();
    for j in 1..=30 {
        term = &term * &shifted / j as f64;
        propagator += &term;
        if term.max() < 1e-20 {
            break;
        }
    }
    propagator *= (-h * c).exp();
    for _ in 0..squarings {
        propagator = &propagator * &propagator;
    }
    let mut row_major = Vec::with_capacity(nz * nz);
    for i in 0..nz {
        for k in 0..nz {
            row_major.push(propagator[(i, k)]);
        }
    }
    row_major
}

/// Anisotropic post-exposure bake of a z-resolved field, in place.
///
/// `data` has axis order `(nz, ny, nx)` and `spacing_nm = [dz, dy, dx]`.
/// The lateral axes get the exact Gaussian of [`gaussian_diffuse_axis`] with
/// `peb.lateral_nm` and `peb.lateral_boundary`; the vertical axis is always
/// zero-flux and gets either the exact Gaussian with `peb.vertical_nm`
/// (uniform diffusivity) or, with a `vertical_diffusivity_scale`, the exact
/// time propagator of the depth-dependent finite-volume diffusion operator.
/// The lateral operator (constant D) commutes with the vertical one, so the
/// axis-by-axis application is exact. Mass (the sum over the volume) is
/// conserved to roundoff.
///
/// Errors on negative or non-finite lengths, non-positive spacings, and a
/// scale vector of the wrong length or with non-positive entries.
pub fn peb_diffuse_anisotropic(
    data: &mut Array3<f64>,
    spacing_nm: [f64; 3],
    peb: &PebDiffusion,
) -> Result<()> {
    check_non_negative("lateral_nm", peb.lateral_nm)?;
    check_non_negative("vertical_nm", peb.vertical_nm)?;
    check_spacing(spacing_nm)?;
    let nz = data.len_of(Axis(0));
    if let Some(scale) = &peb.vertical_diffusivity_scale {
        if scale.len() != nz {
            return Err(LithographyError::InvalidParameter {
                name: "vertical_diffusivity_scale",
                value: scale.len() as f64,
                reason: "must have one entry per z slice (nz)",
            });
        }
        if let Some(&bad) = scale.iter().find(|s| !(s.is_finite() && **s > 0.0)) {
            return Err(LithographyError::InvalidParameter {
                name: "vertical_diffusivity_scale",
                value: bad,
                reason: "every entry must be finite and positive",
            });
        }
    }
    let [dz, dy, dx] = spacing_nm;

    gaussian_diffuse_axis(data, 2, peb.lateral_nm, dx, peb.lateral_boundary);
    gaussian_diffuse_axis(data, 1, peb.lateral_nm, dy, peb.lateral_boundary);
    match &peb.vertical_diffusivity_scale {
        None => gaussian_diffuse_axis(data, 0, peb.vertical_nm, dz, DiffusionBoundary::Reflecting),
        Some(scale) => {
            if nz > 1 && peb.vertical_nm > 0.0 {
                let propagator = vertical_propagator(scale, dz, peb.vertical_nm);
                let mut column = vec![0.0; nz];
                for mut lane in data.lanes_mut(Axis(0)) {
                    for (c, &v) in column.iter_mut().zip(lane.iter()) {
                        *c = v;
                    }
                    for (row, out) in propagator.chunks_exact(nz).zip(lane.iter_mut()) {
                        *out = row.iter().zip(&column).map(|(p, c)| p * c).sum();
                    }
                }
            }
        }
    }
    Ok(())
}

// ============================================================================
// Chemically amplified resist (CAR) reaction–diffusion PEB
// ============================================================================

/// Parameters of the chemically amplified resist (CAR) post-exposure bake.
///
/// All concentrations are normalized to the initial photo-acid-generator
/// (PAG) concentration, so the rate constants are in 1/s. The [`Default`]
/// values are illustrative (a 60 s bake, acid diffusion length
/// `sqrt(2·D_h·t)` ≈ 15.5 nm, quencher loading 15 % of the PAG, fast
/// neutralization), not a fit to a specific resist.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CarParams {
    /// Bake duration in seconds.
    pub peb_time_s: f64,
    /// Catalytic deprotection rate constant `k_amp` (1/s per unit normalized
    /// acid): `∂m/∂t = −k_amp·m·h`.
    pub k_amp_per_s: f64,
    /// Acid–quencher neutralization rate constant `k_q` (1/s per unit
    /// normalized concentration).
    pub k_quench_per_s: f64,
    /// Initial, uniform base-quencher loading `q0` (fraction of the PAG).
    pub quencher_initial: f64,
    /// Lateral acid diffusivity `D_h` in nm²/s.
    pub acid_diffusivity_nm2_s: f64,
    /// Lateral quencher diffusivity `D_q` in nm²/s.
    pub quencher_diffusivity_nm2_s: f64,
    /// Vertical-to-lateral diffusivity ratio `D_z / D_xy` for both species
    /// (1 = isotropic).
    pub vertical_diffusivity_ratio: f64,
    /// Boundary for the lateral axes (z is always reflecting).
    #[serde(default)]
    pub lateral_boundary: DiffusionBoundary,
    /// Optional cap on the splitting time step in seconds; `None` uses the
    /// automatic accuracy limit (at most one sample of diffusion length per
    /// step along every axis).
    #[serde(default)]
    pub max_time_step_s: Option<f64>,
}

impl Default for CarParams {
    fn default() -> Self {
        Self {
            peb_time_s: 60.0,
            k_amp_per_s: 0.1,
            k_quench_per_s: 10.0,
            quencher_initial: 0.15,
            acid_diffusivity_nm2_s: 2.0,
            quencher_diffusivity_nm2_s: 0.5,
            vertical_diffusivity_ratio: 1.0,
            lateral_boundary: DiffusionBoundary::Periodic,
            max_time_step_s: None,
        }
    }
}

impl CarParams {
    /// Reject negative or non-finite times, rates, loadings, diffusivities,
    /// and a non-positive `max_time_step_s`.
    pub fn validate(&self) -> Result<()> {
        check_non_negative("peb_time_s", self.peb_time_s)?;
        check_non_negative("k_amp_per_s", self.k_amp_per_s)?;
        check_non_negative("k_quench_per_s", self.k_quench_per_s)?;
        check_non_negative("quencher_initial", self.quencher_initial)?;
        check_non_negative("acid_diffusivity_nm2_s", self.acid_diffusivity_nm2_s)?;
        check_non_negative(
            "quencher_diffusivity_nm2_s",
            self.quencher_diffusivity_nm2_s,
        )?;
        check_non_negative(
            "vertical_diffusivity_ratio",
            self.vertical_diffusivity_ratio,
        )?;
        if let Some(dt) = self.max_time_step_s {
            if !(dt.is_finite() && dt > 0.0) {
                return Err(LithographyError::InvalidParameter {
                    name: "max_time_step_s",
                    value: dt,
                    reason: "must be finite and positive",
                });
            }
        }
        Ok(())
    }
}

/// Output of [`car_peb`]: the three concentration fields after the bake.
#[derive(Debug, Clone)]
pub struct CarPebResult {
    /// Protected (blocked) site fraction `m` — the quantity the Mack
    /// development rate consumes (1 = fully protected, 0 = fully deprotected).
    pub protected: Array3<f64>,
    /// Remaining acid `h` (normalized to the PAG).
    pub acid: Array3<f64>,
    /// Remaining base quencher `q` (normalized to the PAG).
    pub quencher: Array3<f64>,
    /// Total neutralized acid, summed over voxels (equal to the total
    /// neutralized quencher); `Σh + neutralized_total = Σh0`.
    pub neutralized_total: f64,
    /// Number of splitting steps taken.
    pub steps: usize,
    /// Splitting time step in seconds (`peb_time_s / steps`).
    pub time_step_s: f64,
}

/// Hard cap on splitting steps, so a pathological step limit fails fast.
const MAX_CAR_STEPS: usize = 1_000_000;

/// Tolerance for slightly negative (roundoff) input acid.
const ACID_NEGATIVE_TOLERANCE: f64 = 1e-6;

/// `(1 − e^{−x}) / x`, continuous at `x = 0`.
fn one_minus_exp_over_x(x: f64) -> f64 {
    if x == 0.0 {
        1.0
    } else {
        -(-x).exp_m1() / x
    }
}

/// Exact local neutralization `dh/dt = dq/dt = −k_q·h·q` over `dt`, with the
/// exact time integral of the acid for the deprotection update. Returns
/// `(h(dt), q(dt), ∫₀^dt h dt)`. Negative inputs (roundoff) are clamped to 0.
fn car_react(h0: f64, q0: f64, k_q: f64, dt: f64) -> (f64, f64, f64) {
    let h0 = h0.max(0.0);
    let q0 = q0.max(0.0);
    if k_q == 0.0 || dt == 0.0 {
        return (h0, q0, h0 * dt);
    }
    let excess = h0 - q0; // conserved
    let a = excess.abs();
    let x = k_q * a * dt;
    let decay = (-x).exp();
    // k_q·dt·φ = F / a with F = 1 − e^{−x}; finite as a → 0.
    let kdt_phi = k_q * dt * one_minus_exp_over_x(x);
    if excess >= 0.0 {
        // Acid in excess (or balanced): quencher → 0, acid → excess.
        let denom = 1.0 + q0 * kdt_phi;
        let integral = a * dt + (q0 * kdt_phi).ln_1p() / k_q;
        (h0 / denom, q0 * decay / denom, integral)
    } else {
        // Quencher in excess: acid → 0, quencher → |excess|.
        let denom = 1.0 + h0 * kdt_phi;
        let integral = (h0 * kdt_phi).ln_1p() / k_q;
        (h0 * decay / denom, q0 / denom, integral)
    }
}

/// Prepared lattice-exact diffusion passes for one species at one time step.
struct SpeciesDiffusion {
    passes: Vec<AxisDiffuser>,
}

impl SpeciesDiffusion {
    fn new(
        planner: &mut FftPlanner<f64>,
        dims: (usize, usize, usize),
        spacing_nm: [f64; 3],
        diffusivity_nm2_s: f64,
        vertical_ratio: f64,
        time_s: f64,
        lateral: DiffusionBoundary,
    ) -> Self {
        let (nz, ny, nx) = dims;
        let [dz, dy, dx] = spacing_nm;
        let sigma_lateral = (2.0 * diffusivity_nm2_s * time_s).sqrt();
        let sigma_vertical = (2.0 * diffusivity_nm2_s * vertical_ratio * time_s).sqrt();
        let kernel = SpectralKernel::Lattice;
        let passes = [
            AxisDiffuser::new(planner, nx, 2, sigma_lateral, dx, lateral, kernel),
            AxisDiffuser::new(planner, ny, 1, sigma_lateral, dy, lateral, kernel),
            AxisDiffuser::new(
                planner,
                nz,
                0,
                sigma_vertical,
                dz,
                DiffusionBoundary::Reflecting,
                kernel,
            ),
        ]
        .into_iter()
        .flatten()
        .collect();
        Self { passes }
    }

    fn apply(
        &self,
        data: &mut Array3<f64>,
        buf: &mut Vec<Complex<f64>>,
        scratch: &mut Vec<Complex<f64>>,
    ) {
        for pass in &self.passes {
            pass.apply(data, buf, scratch);
        }
    }
}

/// Chemically amplified resist (CAR) post-exposure bake on a z-resolved
/// volume: acid-catalysed deprotection with acid/base-quencher
/// reaction–diffusion (the standard Mack/PROLITH-class model; see the module
/// docs for the equations and approximations).
///
/// `acid0` is the photogenerated acid `h0 = 1 − m_exposure` with axis order
/// `(nz, ny, nx)` — for a Dill latent image, the decomposed PAG fraction.
/// `spacing_nm = [dz, dy, dx]`. The quencher starts uniform at
/// `params.quencher_initial` and the protected fraction at 1. Lateral axes
/// use `params.lateral_boundary`; z is zero-flux. Axes of length 1 are not
/// diffused (a `(1, ny, nx)` array is a 2D bake).
///
/// Time integration is Strang splitting: lattice-exact diffusion
/// ([`lattice_diffuse_axis`]) of acid and quencher over half steps around an
/// exact local neutralization + deprotection step (consecutive half steps are
/// merged into one full step, which is exact because the diffusion
/// propagators compose). The step is the largest one keeping the diffusion
/// length per step ≤ one sample along every diffused axis (capped by
/// `params.max_time_step_s`), with an integer number of steps ending exactly
/// at `peb_time_s`; without diffusion a single exact step is taken. Both
/// sub-steps are unconditionally stable; the limit controls the O(Δt²)
/// splitting error.
///
/// Errors on invalid parameters or spacings, non-finite acid, acid below
/// −1e-6 (tiny negative roundoff is clamped to 0), or a step limit that
/// would need more than 10⁶ steps.
pub fn car_peb(
    acid0: &Array3<f64>,
    spacing_nm: [f64; 3],
    params: &CarParams,
) -> Result<CarPebResult> {
    params.validate()?;
    check_spacing(spacing_nm)?;
    let mut min_acid = f64::INFINITY;
    for &v in acid0.iter() {
        if !v.is_finite() {
            return Err(LithographyError::InvalidParameter {
                name: "acid0",
                value: v,
                reason: "acid concentration must be finite",
            });
        }
        min_acid = min_acid.min(v);
    }
    if min_acid < -ACID_NEGATIVE_TOLERANCE {
        return Err(LithographyError::InvalidParameter {
            name: "acid0",
            value: min_acid,
            reason: "acid concentration must be non-negative (h0 = 1 - m_exposure)",
        });
    }

    let dims = acid0.dim();
    let mut acid = acid0.mapv(|v| v.max(0.0));
    let mut quencher = Array3::from_elem(dims, params.quencher_initial);
    let mut protected = Array3::from_elem(dims, 1.0);
    if params.peb_time_s == 0.0 || acid0.is_empty() {
        return Ok(CarPebResult {
            protected,
            acid,
            quencher,
            neutralized_total: 0.0,
            steps: 0,
            time_step_s: 0.0,
        });
    }

    // Accuracy-limited step: diffusion length per step ≤ 1 sample per axis.
    let (nz, ny, nx) = dims;
    let [dz, dy, dx] = spacing_nm;
    let d_max = params
        .acid_diffusivity_nm2_s
        .max(params.quencher_diffusivity_nm2_s);
    let mut dt_limit = f64::INFINITY;
    for (n, h, d) in [
        (nx, dx, d_max),
        (ny, dy, d_max),
        (nz, dz, d_max * params.vertical_diffusivity_ratio),
    ] {
        if n > 1 && d > 0.0 {
            dt_limit = dt_limit.min(h * h / (2.0 * d));
        }
    }
    if let Some(cap) = params.max_time_step_s {
        dt_limit = dt_limit.min(cap);
    }
    let steps_f = if dt_limit.is_finite() {
        // The 1e-9 guard stops roundoff (e.g. 6/(1/3) = 6.000…01) from
        // adding a spurious extra step.
        (params.peb_time_s / dt_limit - 1e-9).ceil().max(1.0)
    } else {
        1.0
    };
    if steps_f > MAX_CAR_STEPS as f64 {
        return Err(LithographyError::InvalidParameter {
            name: "max_time_step_s",
            value: dt_limit,
            reason: "the bake would need more than 10^6 splitting steps; raise \
                     max_time_step_s or coarsen the grid",
        });
    }
    let steps = steps_f as usize;
    let dt = params.peb_time_s / steps as f64;

    let mut planner = FftPlanner::new();
    let ratio = params.vertical_diffusivity_ratio;
    let lateral = params.lateral_boundary;
    let make = |planner: &mut FftPlanner<f64>, d: f64, t: f64| {
        SpeciesDiffusion::new(planner, dims, spacing_nm, d, ratio, t, lateral)
    };
    let acid_half = make(&mut planner, params.acid_diffusivity_nm2_s, 0.5 * dt);
    let acid_full = make(&mut planner, params.acid_diffusivity_nm2_s, dt);
    let quencher_half = make(&mut planner, params.quencher_diffusivity_nm2_s, 0.5 * dt);
    let quencher_full = make(&mut planner, params.quencher_diffusivity_nm2_s, dt);
    let mut buf = Vec::new();
    let mut scratch = Vec::new();

    let k_q = params.k_quench_per_s;
    let k_amp = params.k_amp_per_s;
    let mut neutralized_total = 0.0;
    for step in 0..steps {
        // Leading half step, or the merged trailing + leading half steps.
        let (acid_pass, quencher_pass) = if step == 0 {
            (&acid_half, &quencher_half)
        } else {
            (&acid_full, &quencher_full)
        };
        acid_pass.apply(&mut acid, &mut buf, &mut scratch);
        quencher_pass.apply(&mut quencher, &mut buf, &mut scratch);

        Zip::from(&mut acid)
            .and(&mut quencher)
            .and(&mut protected)
            .for_each(|h, q, m| {
                let h_before = h.max(0.0);
                let (h_after, q_after, acid_integral) = car_react(*h, *q, k_q, dt);
                neutralized_total += h_before - h_after;
                *h = h_after;
                *q = q_after;
                *m *= (-k_amp * acid_integral).exp();
            });
    }
    acid_half.apply(&mut acid, &mut buf, &mut scratch);
    quencher_half.apply(&mut quencher, &mut buf, &mut scratch);
    // The lattice kernel is positive; clamp FFT roundoff (~1e-17) so the
    // returned concentrations are non-negative.
    acid.mapv_inplace(|v| v.max(0.0));
    quencher.mapv_inplace(|v| v.max(0.0));

    Ok(CarPebResult {
        protected,
        acid,
        quencher,
        neutralized_total,
        steps,
        time_step_s: dt,
    })
}

/// CAR post-exposure bake of the depth-averaged 2D latent image.
///
/// Treats `latent.pac` (the Dill PAG fraction remaining after exposure) as a
/// single slice: the acid is `h0 = 1 − m` (entries with `m > 1` count as no
/// acid), the bake runs with lateral spacing `pixel_nm` and no vertical
/// diffusion, and the protected-site fraction is written back into
/// `latent.pac`. After this call `pac` therefore holds the protected
/// fraction that [`development_rate`] / [`develop`] consume, not the PAG.
pub fn car_peb_2d(
    latent: &mut LatentImage,
    params: &CarParams,
    pixel_nm: f64,
) -> Result<CarPebResult> {
    let (ny, nx) = latent.pac.dim();
    let acid0 = Array3::from_shape_fn((1, ny, nx), |(_, i, j)| (1.0 - latent.pac[[i, j]]).max(0.0));
    let result = car_peb(&acid0, [pixel_nm, pixel_nm, pixel_nm], params)?;
    latent.pac.assign(&result.protected.index_axis(Axis(0), 0));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use ndarray::Array2;

    #[test]
    fn test_unexposed_pac_is_one() {
        let aerial = Array2::zeros((64, 64)); // zero intensity = no exposure
        let params = ResistParams::vuv_fluoropolymer();
        let latent = expose(&aerial, 30.0, &params);
        for &m in latent.pac.iter() {
            assert_relative_eq!(m, 1.0, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_high_dose_pac_near_zero() {
        let aerial = Array2::ones((64, 64)); // uniform max intensity
        let params = ResistParams::vuv_fluoropolymer();
        let latent = expose(&aerial, 1000.0, &params); // very high dose
        for &m in latent.pac.iter() {
            assert!(m < 0.01, "High dose should drive PAC near zero");
        }
    }

    #[test]
    fn test_higher_dose_lower_pac() {
        let aerial = Array2::from_elem((64, 64), 0.5);
        let params = ResistParams::vuv_fluoropolymer();
        let latent_low = expose(&aerial, 10.0, &params);
        let latent_high = expose(&aerial, 50.0, &params);
        assert!(
            latent_high.pac[[32, 32]] < latent_low.pac[[32, 32]],
            "Higher dose should give lower PAC"
        );
    }

    #[test]
    fn test_mack_development_rate() {
        let params = ResistParams::vuv_fluoropolymer();

        // Fully exposed (m=0): rate should be near rmax
        let exposed = Array2::from_elem((1, 1), 0.0);
        let latent_exposed = LatentImage { pac: exposed };
        let rate = development_rate(&latent_exposed, &params);
        assert!(rate[[0, 0]] > 50.0);

        // Unexposed (m=1): rate should be near rmin
        let unexposed = Array2::from_elem((1, 1), 1.0);
        let latent_unexposed = LatentImage { pac: unexposed };
        let rate = development_rate(&latent_unexposed, &params);
        assert!(rate[[0, 0]] < 1.0);
    }

    #[test]
    fn test_beer_lambert_coupling() {
        let params = ResistParams {
            thickness_nm: 150.0,
            dill_a: 0.0,
            dill_b: 0.0, // zero absorption
            dill_c: 0.02,
            ..ResistParams::vuv_fluoropolymer()
        };
        let coupling = effective_coupling(&params);
        assert_relative_eq!(coupling, 1.0, epsilon = 0.01);
    }

    #[test]
    fn test_peb_diffusion_broadens() {
        let mut pac = Array2::zeros((64, 64));
        pac[[32, 32]] = 0.5; // delta function
        let mut latent = LatentImage { pac };

        let before_max = latent.pac[[32, 32]];
        peb_diffuse(&mut latent, 10.0, 1.0);
        let after_max = latent.pac[[32, 32]];

        assert!(
            after_max < before_max,
            "PEB diffusion should reduce peak concentration"
        );
    }

    #[test]
    fn test_development_profile() {
        let mut pac = Array2::from_elem((64, 64), 0.95); // almost unexposed
                                                         // Create exposed region in center
        for j in 28..36 {
            for i in 0..64 {
                pac[[i, j]] = 0.1;
            }
        }

        let params = ResistParams::vuv_fluoropolymer();
        let latent = LatentImage { pac };
        // Short development time so unexposed region survives
        let profile = develop(&latent, &params, 1.0, 2.0);

        assert_eq!(profile.x_nm.len(), 64);
        // Exposed region should have lower remaining height
        let center_height = profile.height_nm[32];
        let edge_height = profile.height_nm[10];
        assert!(
            center_height < edge_height,
            "Exposed region height ({}) should be less than unexposed ({})",
            center_height,
            edge_height
        );
    }

    #[test]
    fn test_mack_n_near_one_no_panic() {
        let params = ResistParams {
            development: DevelopmentModel::Mack {
                rmax: 100.0,
                rmin: 0.1,
                mth: 0.5,
                n: 1.001, // near-singular n~1
            },
            ..ResistParams::vuv_fluoropolymer()
        };
        let pac = Array2::from_elem((1, 1), 0.5);
        let latent = LatentImage { pac };
        let rate = development_rate(&latent, &params);
        assert!(
            rate[[0, 0]].is_finite(),
            "Rate should be finite for n near 1"
        );
    }

    #[test]
    fn test_expose_zero_dose_pac_one() {
        let aerial = Array2::from_elem((64, 64), 0.8);
        let params = ResistParams::vuv_fluoropolymer();
        let latent = expose(&aerial, 0.0, &params);
        for &m in latent.pac.iter() {
            assert_relative_eq!(m, 1.0, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_develop_zero_time_full_thickness() {
        let pac = Array2::from_elem((64, 64), 0.1); // fully exposed
        let params = ResistParams::vuv_fluoropolymer();
        let latent = LatentImage { pac };
        let profile = develop(&latent, &params, 0.0, 2.0);
        for &h in &profile.height_nm {
            assert_relative_eq!(h, params.thickness_nm, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_threshold_model_exposed() {
        let params = ResistParams {
            development: DevelopmentModel::Threshold { threshold: 0.5 },
            ..ResistParams::vuv_fluoropolymer()
        };
        // m = 0.3 is below threshold 0.5 => exposed => fast development rate
        let pac = Array2::from_elem((1, 1), 0.3);
        let latent = LatentImage { pac };
        let rate = development_rate(&latent, &params);
        assert!(
            rate[[0, 0]] > 100.0,
            "Exposed region should have high development rate"
        );

        // m = 0.8 is above threshold => unexposed => slow rate
        let pac2 = Array2::from_elem((1, 1), 0.8);
        let latent2 = LatentImage { pac: pac2 };
        let rate2 = development_rate(&latent2, &params);
        assert!(
            rate2[[0, 0]] < 1.0,
            "Unexposed region should have low development rate"
        );
    }

    // ------------------------------------------------------------------
    // Exact spectral PEB diffusion
    // ------------------------------------------------------------------

    /// Second moment of `data` about index `center` along `axis`, in nm²,
    /// normalized by the total mass.
    fn variance_along(data: &Array3<f64>, axis: usize, center: usize, spacing: f64) -> f64 {
        let mut num = 0.0;
        let mut den = 0.0;
        for ((k, i, j), &v) in data.indexed_iter() {
            let idx = [k, i, j][axis] as f64;
            let d = (idx - center as f64) * spacing;
            num += v * d * d;
            den += v;
        }
        num / den
    }

    /// A unit impulse at `idx` in a zero volume of shape `dims`.
    fn impulse(dims: (usize, usize, usize), idx: (usize, usize, usize)) -> Array3<f64> {
        let mut data = Array3::zeros(dims);
        data[idx] = 1.0;
        data
    }

    #[test]
    fn test_gaussian_diffuse_axis_fixture_periodic_and_reflecting() {
        // numpy: real(ifft(fft(delta_8) * exp(-2 pi^2 sigma^2 f^2))), sigma = h = 1.
        // The -5.2e-4 entry is the band-limited kernel's (documented) side lobe.
        let periodic = [
            0.3979274504228891,
            0.24294886396924764,
            0.05309575211597309,
            0.00525316519179579,
            -0.00052301297692203,
            0.00525316519179579,
            0.05309575211597309,
            0.24294886396924764,
        ];
        // numpy: even extension of delta_6 (impulse at index 0), sigma = 1.5, h = 1.
        let reflecting = [
            0.47892685095606924,
            0.3223054039772888,
            0.14533400857697118,
            0.04359150537532278,
            0.00863063490410241,
            0.0012115962102456,
        ];
        for axis in 0..3 {
            // Periodic lane of length 8 along `axis`; two lanes so the
            // real/imaginary packing path is exercised as well.
            let mut dims = [2, 2, 2];
            dims[axis] = 8;
            let mut data = Array3::<f64>::zeros((dims[0], dims[1], dims[2]));
            data[[0, 0, 0]] = 1.0;
            let mut other = [1, 1, 1];
            other[axis] = 0;
            data[[other[0], other[1], other[2]]] = 2.0;
            gaussian_diffuse_axis(&mut data, axis, 1.0, 1.0, DiffusionBoundary::Periodic);
            for (t, &expected) in periodic.iter().enumerate() {
                let mut idx = [0, 0, 0];
                idx[axis] = t;
                assert_relative_eq!(data[idx], expected, epsilon = 1e-14);
                let mut idx2 = other;
                idx2[axis] = t;
                assert_relative_eq!(data[idx2], 2.0 * expected, epsilon = 1e-14);
            }

            let mut dims = [1, 1, 1];
            dims[axis] = 6;
            let mut data = Array3::<f64>::zeros((dims[0], dims[1], dims[2]));
            data[[0, 0, 0]] = 1.0;
            gaussian_diffuse_axis(&mut data, axis, 1.5, 1.0, DiffusionBoundary::Reflecting);
            for (t, &expected) in reflecting.iter().enumerate() {
                let mut idx = [0, 0, 0];
                idx[axis] = t;
                assert_relative_eq!(data[idx], expected, epsilon = 1e-14);
            }
        }
    }

    #[test]
    fn test_lattice_kernel_matches_modified_bessel() {
        // e^{-t} I_n(t), t = (sigma/h)^2, from the power series of I_n
        // (independent of the FFT implementation).
        let cases: [(f64, [f64; 4]); 2] = [
            (
                1.0,
                [
                    0.4657596075936405,
                    0.20791041534970847,
                    0.04993877689422354,
                    0.008155307772814294,
                ],
            ),
            (
                1.5,
                [
                    0.2874319388973915,
                    0.21121661600759037,
                    0.09968383577953338,
                    0.03400090795508657,
                ],
            ),
        ];
        for (sigma, expected) in cases {
            let n = 64;
            let c = n / 2;
            let mut data = impulse((1, 1, n), (0, 0, c));
            lattice_diffuse_axis(&mut data, 2, sigma * 2.0, 2.0, DiffusionBoundary::Periodic);
            for (d, &e) in expected.iter().enumerate() {
                assert_relative_eq!(data[[0, 0, c + d]], e, epsilon = 1e-14);
                assert_relative_eq!(data[[0, 0, c - d]], e, epsilon = 1e-14);
            }
            // Strictly non-negative, unlike the band-limited Gaussian.
            assert!(data.iter().all(|&v| v > -1e-16));
        }
    }

    #[test]
    fn test_gaussian_variance_exact_per_axis() {
        // Anisotropic spacing and lengths; impulse far from every boundary.
        let dims = (40, 36, 48);
        let center = (20, 18, 24);
        let spacing = [2.5, 2.0, 1.5]; // [dz, dy, dx]
        let peb = PebDiffusion {
            lateral_nm: 4.0,  // 2 px in y, 2.67 px in x
            vertical_nm: 6.0, // 2.4 px in z
            vertical_diffusivity_scale: None,
            lateral_boundary: DiffusionBoundary::Periodic,
        };
        let mut data = impulse(dims, center);
        peb_diffuse_anisotropic(&mut data, spacing, &peb).unwrap();

        assert_relative_eq!(data.sum(), 1.0, epsilon = 1e-12);
        assert_relative_eq!(
            variance_along(&data, 2, center.2, 1.5),
            16.0,
            max_relative = 1e-9
        );
        assert_relative_eq!(
            variance_along(&data, 1, center.1, 2.0),
            16.0,
            max_relative = 1e-9
        );
        assert_relative_eq!(
            variance_along(&data, 0, center.0, 2.5),
            36.0,
            max_relative = 1e-9
        );
        // sigma >= 2 px: side lobes are below 1e-10 of the impulse.
        assert!(data.iter().all(|&v| v > -1e-10));

        // Reflecting lateral boundary gives the same result far from the walls.
        let mut refl = impulse(dims, center);
        let peb_refl = PebDiffusion {
            lateral_boundary: DiffusionBoundary::Reflecting,
            ..peb
        };
        peb_diffuse_anisotropic(&mut refl, spacing, &peb_refl).unwrap();
        assert_relative_eq!(
            variance_along(&refl, 2, center.2, 1.5),
            16.0,
            max_relative = 1e-9
        );
        assert_relative_eq!(
            variance_along(&refl, 1, center.1, 2.0),
            16.0,
            max_relative = 1e-9
        );
    }

    #[test]
    fn test_lattice_variance_exact_even_for_small_sigma() {
        // sigma = 0.3 sample: the lattice kernel still adds exactly sigma^2.
        for boundary in [DiffusionBoundary::Periodic, DiffusionBoundary::Reflecting] {
            let mut data = impulse((1, 1, 33), (0, 0, 16));
            lattice_diffuse_axis(&mut data, 2, 0.6, 2.0, boundary);
            assert_relative_eq!(
                variance_along(&data, 2, 16, 2.0),
                0.36,
                max_relative = 1e-12
            );
            assert_relative_eq!(data.sum(), 1.0, epsilon = 1e-14);
        }
    }

    #[test]
    fn test_peb_axes_independent_and_commuting() {
        let dims = (12, 16, 20);
        let spacing = [3.0, 2.0, 2.0];
        let mut base = Array3::<f64>::zeros(dims);
        for ((k, i, j), v) in base.indexed_iter_mut() {
            *v = 0.5
                + 0.3 * (0.4 * j as f64).sin() * (0.3 * i as f64).cos()
                + 0.1 * (0.5 * k as f64).cos();
        }

        // Lateral-only diffusion leaves every column's z-profile moments
        // untouched on average: the (y, x)-summed z profile is invariant.
        let z_profile = |d: &Array3<f64>| -> Vec<f64> {
            (0..dims.0)
                .map(|k| d.index_axis(Axis(0), k).sum())
                .collect()
        };
        let mut lateral = base.clone();
        peb_diffuse_anisotropic(&mut lateral, spacing, &PebDiffusion::anisotropic(5.0, 0.0))
            .unwrap();
        for (a, b) in z_profile(&lateral).iter().zip(z_profile(&base).iter()) {
            assert_relative_eq!(*a, *b, epsilon = 1e-11);
        }
        // Vertical-only diffusion leaves the z-summed lateral map untouched.
        let xy_map = |d: &Array3<f64>| d.sum_axis(Axis(0));
        let mut vertical = base.clone();
        peb_diffuse_anisotropic(&mut vertical, spacing, &PebDiffusion::anisotropic(0.0, 7.0))
            .unwrap();
        for (a, b) in xy_map(&vertical).iter().zip(xy_map(&base).iter()) {
            assert_relative_eq!(*a, *b, epsilon = 1e-11);
        }

        // Operators commute: lateral-then-vertical == vertical-then-lateral
        // == the combined call.
        let mut lv = lateral.clone();
        peb_diffuse_anisotropic(&mut lv, spacing, &PebDiffusion::anisotropic(0.0, 7.0)).unwrap();
        let mut vl = vertical.clone();
        peb_diffuse_anisotropic(&mut vl, spacing, &PebDiffusion::anisotropic(5.0, 0.0)).unwrap();
        let mut both = base.clone();
        peb_diffuse_anisotropic(&mut both, spacing, &PebDiffusion::anisotropic(5.0, 7.0)).unwrap();
        for ((a, b), c) in lv.iter().zip(vl.iter()).zip(both.iter()) {
            assert_relative_eq!(*a, *b, epsilon = 1e-12);
            assert_relative_eq!(*a, *c, epsilon = 1e-12);
        }
    }

    /// Max |FV − continuous| for a Gaussian bump (σ_bump = 12 nm at z = 40)
    /// under vertical diffusion σ = 9 nm, sampled at `dz`; also checks that
    /// the uniform-scale FV propagator equals the reflecting lattice kernel.
    fn fv_vs_gaussian_max_diff(dz: f64) -> f64 {
        let nz = (96.0 / dz) as usize;
        let sigma = 9.0;
        // Smooth z profiles (Gaussian bump; linear slope) in two columns.
        let mut base = Array3::<f64>::zeros((nz, 1, 2));
        for k in 0..nz {
            let z = (k as f64 + 0.5) * dz;
            base[[k, 0, 0]] = (-(z - 40.0).powi(2) / (2.0 * 12.0 * 12.0)).exp();
            base[[k, 0, 1]] = 0.2 + 0.01 * z;
        }
        let uniform = PebDiffusion::anisotropic(0.0, sigma);
        let scaled = PebDiffusion {
            vertical_diffusivity_scale: Some(vec![1.0; nz]),
            ..uniform.clone()
        };
        let mut fv = base.clone();
        peb_diffuse_anisotropic(&mut fv, [dz, 1.0, 1.0], &scaled).unwrap();
        // The finite-volume propagator with uniform D is exactly the
        // reflecting lattice kernel.
        let mut lattice = base.clone();
        lattice_diffuse_axis(&mut lattice, 0, sigma, dz, DiffusionBoundary::Reflecting);
        for (a, b) in fv.iter().zip(lattice.iter()) {
            assert_relative_eq!(*a, *b, epsilon = 1e-12);
        }
        let mut gauss = base.clone();
        peb_diffuse_anisotropic(&mut gauss, [dz, 1.0, 1.0], &uniform).unwrap();
        fv.iter()
            .zip(gauss.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max)
    }

    #[test]
    fn test_depth_profile_uniform_scale_matches_lattice_and_gaussian() {
        // FV (lattice) vs continuous Gaussian differ only by the O(dz²)
        // spatial discretization: numpy gives 2.46e-3 at dz = 4 nm (3
        // samples per bump sigma) and 6.3e-4 at dz = 2 nm.
        let coarse = fv_vs_gaussian_max_diff(4.0);
        let fine = fv_vs_gaussian_max_diff(2.0);
        assert!(coarse < 3e-3, "dz = 4: FV vs Gaussian differ by {coarse}");
        assert!(fine < 8e-4, "dz = 2: FV vs Gaussian differ by {fine}");
        let order = (coarse / fine).log2();
        assert!((1.7..2.3).contains(&order), "convergence order {order}");
    }

    #[test]
    fn test_depth_profile_conserves_mass_and_slows_low_d_region() {
        // Top half D = D_ref, bottom half D = 0.1 D_ref; sigma_ref = 6 nm.
        let nz = 60;
        let dz = 2.0;
        let scale: Vec<f64> = (0..nz)
            .map(|k| if k < nz / 2 { 1.0 } else { 0.1 })
            .collect();
        let peb = PebDiffusion {
            vertical_diffusivity_scale: Some(scale),
            ..PebDiffusion::anisotropic(0.0, 6.0)
        };
        let mut data = Array3::<f64>::zeros((nz, 1, 2));
        data[[15, 0, 0]] = 1.0; // impulse in the fast top region (5 sigma from walls)
        data[[45, 0, 1]] = 1.0; // impulse in the slow bottom region
        peb_diffuse_anisotropic(&mut data, [dz, 1.0, 1.0], &peb).unwrap();

        let column = |j: usize| -> Array3<f64> {
            let mut c = Array3::<f64>::zeros((nz, 1, 1));
            for k in 0..nz {
                c[[k, 0, 0]] = data[[k, 0, j]];
            }
            c
        };
        let (top, bottom) = (column(0), column(1));
        assert_relative_eq!(top.sum(), 1.0, epsilon = 1e-12);
        assert_relative_eq!(bottom.sum(), 1.0, epsilon = 1e-12);
        // Variance growth is 2 D t: sigma_ref^2 = 36 nm^2 in the fast region
        // and exactly 10x less (3.6 nm^2) where D is 10x lower (numpy:
        // 35.99944 and 3.600000).
        assert_relative_eq!(variance_along(&top, 0, 15, dz), 36.0, max_relative = 1e-4);
        assert_relative_eq!(variance_along(&bottom, 0, 45, dz), 3.6, max_relative = 1e-4);
        // The uniformized propagator is entrywise non-negative.
        assert!(data.iter().all(|&v| v >= 0.0));
    }

    #[test]
    fn test_peb_zero_sigma_identity_and_invalid_params() {
        let mut data = Array3::<f64>::from_shape_fn((4, 5, 6), |(k, i, j)| {
            ((k * 31 + i * 7 + j * 3) % 11) as f64 / 10.0
        });
        let original = data.clone();
        peb_diffuse_anisotropic(&mut data, [1.0, 1.0, 1.0], &PebDiffusion::isotropic(0.0)).unwrap();
        assert_eq!(data, original);
        gaussian_diffuse_axis(&mut data, 1, 0.0, 1.0, DiffusionBoundary::Periodic);
        lattice_diffuse_axis(&mut data, 0, -3.0, 1.0, DiffusionBoundary::Reflecting);
        assert_eq!(data, original);

        let bad = [
            PebDiffusion::anisotropic(-1.0, 1.0),
            PebDiffusion::anisotropic(1.0, f64::NAN),
            PebDiffusion {
                vertical_diffusivity_scale: Some(vec![1.0; 3]), // nz is 4
                ..PebDiffusion::isotropic(1.0)
            },
            PebDiffusion {
                vertical_diffusivity_scale: Some(vec![1.0, 0.0, 1.0, 1.0]),
                ..PebDiffusion::isotropic(1.0)
            },
        ];
        for peb in &bad {
            assert!(peb_diffuse_anisotropic(&mut data, [1.0, 1.0, 1.0], peb).is_err());
        }
        assert!(
            peb_diffuse_anisotropic(&mut data, [1.0, 0.0, 1.0], &PebDiffusion::isotropic(1.0))
                .is_err()
        );
        // A valid exponential depth profile is accepted and well formed.
        let profiled =
            PebDiffusion::isotropic(2.0).with_exponential_depth_profile(4, 1.0, 3.0, 2.0);
        let scale = profiled.vertical_diffusivity_scale.clone().unwrap();
        assert_relative_eq!(scale[0], 1.0 + 2.0 * (-0.25_f64).exp(), epsilon = 1e-15);
        assert!(peb_diffuse_anisotropic(&mut data, [1.0, 1.0, 1.0], &profiled).is_ok());
    }

    // ------------------------------------------------------------------
    // Chemically amplified resist PEB
    // ------------------------------------------------------------------

    fn no_diffusion_params() -> CarParams {
        CarParams {
            acid_diffusivity_nm2_s: 0.0,
            quencher_diffusivity_nm2_s: 0.0,
            ..CarParams::default()
        }
    }

    #[test]
    fn test_car_no_quencher_no_diffusion_exact_exponential() {
        // m = exp(-k_amp h0 t): exp(-1.2), exp(-3), exp(-5.4).
        let expected = [
            0.301194211912202,
            0.049787068367863944,
            0.004516580942612666,
        ];
        let acid0 = Array3::from_shape_vec((1, 1, 3), vec![0.2, 0.5, 0.9]).unwrap();
        let params = CarParams {
            peb_time_s: 60.0,
            k_amp_per_s: 0.1,
            quencher_initial: 0.0,
            ..no_diffusion_params()
        };
        let result = car_peb(&acid0, [1.0, 1.0, 1.0], &params).unwrap();
        assert_eq!(result.steps, 1, "no diffusion => one exact step");
        for (m, e) in result.protected.iter().zip(expected) {
            assert_relative_eq!(*m, e, max_relative = 1e-12);
        }
        // Acid is catalytic: unchanged without quencher.
        for (h, h0) in result.acid.iter().zip(acid0.iter()) {
            assert_relative_eq!(*h, *h0, epsilon = 1e-15);
        }
        // Many small steps give the same exact answer.
        let stepped = car_peb(
            &acid0,
            [1.0, 1.0, 1.0],
            &CarParams {
                max_time_step_s: Some(0.7),
                ..params
            },
        )
        .unwrap();
        assert!(stepped.steps > 80);
        for (m, e) in stepped.protected.iter().zip(expected) {
            assert_relative_eq!(*m, e, max_relative = 1e-12);
        }
    }

    #[test]
    fn test_car_reaction_only_matches_rk4_fixtures() {
        // (h0, q0, k_q, k_amp, t) -> (h, q, m) from an independent RK4
        // integration (200k steps) of dh/dt = dq/dt = -k_q h q,
        // dm/dt = -k_amp m h.
        let cases = [
            (
                0.6,
                0.2,
                0.5,
                0.1,
                3.0,
                0.4895584597537028,
                0.08955845975369713,
                0.8515601387159313,
            ),
            (0.3, 0.3, 2.0, 0.2, 1.0, 0.1875, 0.1875, 0.9540870513287041),
            (
                0.2,
                0.5,
                1.5,
                0.3,
                2.0,
                0.05826365252118443,
                0.35826365252118714,
                0.935505939100767,
            ),
            (
                0.8,
                0.1,
                50.0,
                0.1,
                2.0,
                0.7,
                3.47851851890052e-32,
                0.8691260931667487,
            ),
        ];
        for (h0, q0, k_q, k_amp, t, h_e, q_e, m_e) in cases {
            let acid0 = Array3::from_elem((1, 1, 1), h0);
            for max_step in [None, Some(t / 7.0)] {
                let params = CarParams {
                    peb_time_s: t,
                    k_amp_per_s: k_amp,
                    k_quench_per_s: k_q,
                    quencher_initial: q0,
                    max_time_step_s: max_step,
                    ..no_diffusion_params()
                };
                let r = car_peb(&acid0, [1.0, 1.0, 1.0], &params).unwrap();
                assert_relative_eq!(r.acid[[0, 0, 0]], h_e, max_relative = 1e-10);
                assert_relative_eq!(r.protected[[0, 0, 0]], m_e, max_relative = 1e-10);
                if q_e > 1e-20 {
                    assert_relative_eq!(r.quencher[[0, 0, 0]], q_e, max_relative = 1e-10);
                } else {
                    assert!(r.quencher[[0, 0, 0]] < 1e-30);
                }
                assert_relative_eq!(r.neutralized_total, h0 - h_e, epsilon = 1e-12);
            }
        }
    }

    #[test]
    fn test_car_pure_diffusion_variance_is_2dt() {
        // No reactions: the acid impulse spreads with variance 2 D t per
        // axis, D_z = ratio * D. D = 1.5 nm^2/s, t = 2 s, ratio = 0.5. The
        // grid keeps the (Poisson-like) lattice-kernel tails < 1e-14 at the
        // boundaries.
        let dims = (24, 36, 48);
        let center = (12, 18, 24);
        let spacing = [2.0, 1.5, 1.0];
        for lateral in [DiffusionBoundary::Periodic, DiffusionBoundary::Reflecting] {
            let params = CarParams {
                peb_time_s: 2.0,
                k_amp_per_s: 0.0,
                k_quench_per_s: 0.0,
                quencher_initial: 0.0,
                acid_diffusivity_nm2_s: 1.5,
                quencher_diffusivity_nm2_s: 0.0,
                vertical_diffusivity_ratio: 0.5,
                lateral_boundary: lateral,
                max_time_step_s: None,
            };
            let r = car_peb(&impulse(dims, center), spacing, &params).unwrap();
            assert!(r.steps >= 6);
            assert_relative_eq!(r.acid.sum(), 1.0, epsilon = 1e-12);
            assert_relative_eq!(
                variance_along(&r.acid, 2, center.2, 1.0),
                6.0,
                max_relative = 1e-8
            );
            assert_relative_eq!(
                variance_along(&r.acid, 1, center.1, 1.5),
                6.0,
                max_relative = 1e-8
            );
            assert_relative_eq!(
                variance_along(&r.acid, 0, center.0, 2.0),
                3.0,
                max_relative = 1e-8
            );
            assert!(r.acid.iter().all(|&v| v >= 0.0));
            // Nothing reacts without quencher or k_amp.
            assert!(r.protected.iter().all(|&m| m == 1.0));
        }
    }

    /// Smooth periodic line/space acid profile along x (exposed space in the
    /// middle of the field), `n` samples.
    fn line_acid(n: usize, peak: f64) -> Array3<f64> {
        Array3::from_shape_fn((1, 1, n), |(_, _, j)| {
            let x = (j as f64 + 0.5) / n as f64;
            peak * 0.5 * (1.0 + (3.0 * (2.0 * std::f64::consts::PI * (x - 0.5)).cos()).tanh())
        })
    }

    #[test]
    fn test_car_mass_balance() {
        let mut acid0 = Array3::<f64>::zeros((6, 8, 16));
        for ((k, i, j), v) in acid0.indexed_iter_mut() {
            let lateral =
                0.5 * (1.0 + (3.0 * (2.0 * std::f64::consts::PI * j as f64 / 16.0).cos()).tanh());
            *v = 0.7 * lateral * (-0.05 * k as f64).exp() * (1.0 + 0.1 * (i as f64).sin());
        }
        let params = CarParams {
            peb_time_s: 20.0,
            k_amp_per_s: 0.2,
            k_quench_per_s: 5.0,
            quencher_initial: 0.2,
            acid_diffusivity_nm2_s: 2.0,
            quencher_diffusivity_nm2_s: 0.7,
            vertical_diffusivity_ratio: 0.6,
            lateral_boundary: DiffusionBoundary::Periodic,
            max_time_step_s: None,
        };
        let r = car_peb(&acid0, [3.0, 2.0, 2.0], &params).unwrap();
        let h0_sum = acid0.sum();
        let q0_sum = 0.2 * acid0.len() as f64;
        // h - q is conserved by both diffusion and neutralization.
        let before = h0_sum - q0_sum;
        let after = r.acid.sum() - r.quencher.sum();
        assert_relative_eq!(after, before, max_relative = 1e-12);
        // Neutralization removes equal amounts of acid and quencher.
        assert_relative_eq!(
            r.acid.sum() + r.neutralized_total,
            h0_sum,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            r.quencher.sum() + r.neutralized_total,
            q0_sum,
            max_relative = 1e-12
        );
        assert!(
            r.neutralized_total > 0.1 * q0_sum,
            "quencher must actually react"
        );
        // Physical bounds.
        assert!(r.acid.iter().all(|&v| v >= 0.0));
        assert!(r.quencher.iter().all(|&v| v >= 0.0));
        assert!(r.protected.iter().all(|&m| m > 0.0 && m <= 1.0));
    }

    #[test]
    fn test_car_quencher_reduces_deprotection_and_sharpens_edge() {
        let n = 64;
        let dx = 2.0;
        let acid0 = line_acid(n, 0.6);
        let run = |q0: f64| {
            let params = CarParams {
                peb_time_s: 60.0,
                k_amp_per_s: 0.1,
                k_quench_per_s: 10.0,
                quencher_initial: q0,
                acid_diffusivity_nm2_s: 2.0,
                quencher_diffusivity_nm2_s: 0.5,
                ..CarParams::default()
            };
            car_peb(&acid0, [1.0, 1.0, dx], &params).unwrap().protected
        };
        let mut last_deprotection = f64::INFINITY;
        let mut last_slope = 0.0;
        let mut last_dark_m = 0.0;
        for q0 in [0.0, 0.1, 0.2, 0.3] {
            let m = run(q0);
            let deprotection = 1.0 - m.mean().unwrap();
            let (m_min, m_max) = m
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
                    (lo.min(v), hi.max(v))
                });
            // Normalized edge slope of the deprotection profile.
            let slope = (1..n)
                .map(|j| (m[[0, 0, j]] - m[[0, 0, j - 1]]).abs() / dx)
                .fold(0.0, f64::max)
                / (m_max - m_min);
            assert!(
                deprotection < last_deprotection,
                "q0 = {q0}: deprotection must drop"
            );
            assert!(
                slope > last_slope,
                "q0 = {q0}: normalized edge slope must rise"
            );
            assert!(
                m_max >= last_dark_m,
                "q0 = {q0}: dark region must stay better protected"
            );
            last_deprotection = deprotection;
            last_slope = slope;
            last_dark_m = m_max;
        }
        // With quencher the unexposed line is essentially fully protected.
        assert!(last_dark_m > 0.999);
    }

    #[test]
    fn test_car_strang_splitting_is_second_order() {
        let acid0 = line_acid(32, 0.7);
        let spacing = [1.0, 1.0, 2.0];
        let params = |dt: f64| CarParams {
            peb_time_s: 6.0,
            k_amp_per_s: 0.3,
            k_quench_per_s: 2.0,
            quencher_initial: 0.2,
            acid_diffusivity_nm2_s: 3.0,
            quencher_diffusivity_nm2_s: 1.0,
            vertical_diffusivity_ratio: 1.0,
            lateral_boundary: DiffusionBoundary::Periodic,
            max_time_step_s: Some(dt),
        };
        let reference = car_peb(&acid0, spacing, &params(6.0 / 960.0)).unwrap();
        let error = |dt: f64| {
            let r = car_peb(&acid0, spacing, &params(dt)).unwrap();
            let max_abs = |a: &Array3<f64>, b: &Array3<f64>| {
                a.iter()
                    .zip(b.iter())
                    .map(|(x, y)| (x - y).abs())
                    .fold(0.0, f64::max)
            };
            (
                max_abs(&r.protected, &reference.protected),
                max_abs(&r.acid, &reference.acid),
            )
        };
        let (em1, eh1) = error(0.5);
        let (em2, eh2) = error(0.25);
        let (rm, rh) = (em1 / em2, eh1 / eh2);
        assert!(
            (3.5..4.5).contains(&rm),
            "m error ratio {rm} ({em1:e} -> {em2:e})"
        );
        assert!(
            (3.5..4.5).contains(&rh),
            "h error ratio {rh} ({eh1:e} -> {eh2:e})"
        );
        assert!(em1 < 5e-4, "splitting error {em1:e} unexpectedly large");
    }

    #[test]
    fn test_car_default_step_is_accuracy_limited() {
        // Auto step keeps sqrt(2 D dt) <= 1 sample: dx = 2 nm, D = 3 -> dt <= 2/3 s.
        let acid0 = line_acid(32, 0.7);
        let params = CarParams {
            peb_time_s: 6.0,
            acid_diffusivity_nm2_s: 3.0,
            quencher_diffusivity_nm2_s: 1.0,
            ..CarParams::default()
        };
        let r = car_peb(&acid0, [1.0, 1.0, 2.0], &params).unwrap();
        assert_eq!(r.steps, 9);
        assert_relative_eq!(r.time_step_s, 6.0 / 9.0, epsilon = 1e-15);
        assert!((2.0 * 3.0 * r.time_step_s).sqrt() <= 2.0 + 1e-12);
    }

    #[test]
    fn test_car_2d_writes_protected_fraction() {
        let (ny, nx) = (6, 32);
        let pac = Array2::from_shape_fn((ny, nx), |(_, j)| {
            let x = (j as f64 + 0.5) / nx as f64;
            // Dill PAG remaining: exposed space (low m) in the middle.
            1.0 - 0.6 * (-((x - 0.5) / 0.15).powi(2)).exp()
        });
        let params = CarParams::default();
        let mut latent = LatentImage { pac: pac.clone() };
        let result = car_peb_2d(&mut latent, &params, 2.0).unwrap();
        assert_eq!(result.protected.dim(), (1, ny, nx));
        for ((i, j), &m) in latent.pac.indexed_iter() {
            assert_eq!(m, result.protected[[0, i, j]]);
        }
        // Same as the explicit single-slice 3D call.
        let acid0 = Array3::from_shape_fn((1, ny, nx), |(_, i, j)| 1.0 - pac[[i, j]]);
        let direct = car_peb(&acid0, [2.0, 2.0, 2.0], &params).unwrap();
        for (a, b) in result.protected.iter().zip(direct.protected.iter()) {
            assert_relative_eq!(*a, *b, epsilon = 1e-15);
        }
        // The exposed centre deprotects, the edges stay protected, and the
        // Mack rate therefore develops the centre fastest.
        assert!(latent.pac[[0, nx / 2]] < 0.5);
        assert!(latent.pac[[0, 0]] > 0.99);
        let rate = development_rate(&latent, &ResistParams::default());
        assert!(rate[[0, nx / 2]] > 100.0 * rate[[0, 0]]);

        // Unexposed resist (m = 1 everywhere): no acid, nothing deprotects.
        let mut dark = LatentImage {
            pac: Array2::from_elem((ny, nx), 1.0),
        };
        car_peb_2d(&mut dark, &params, 2.0).unwrap();
        assert!(dark.pac.iter().all(|&m| m == 1.0));
    }

    #[test]
    fn test_car_zero_time_identity_and_invalid_params() {
        let acid0 = line_acid(16, 0.5);
        let r = car_peb(
            &acid0,
            [1.0, 1.0, 1.0],
            &CarParams {
                peb_time_s: 0.0,
                ..CarParams::default()
            },
        )
        .unwrap();
        assert_eq!(r.steps, 0);
        assert_eq!(r.acid, acid0);
        assert!(r.protected.iter().all(|&m| m == 1.0));
        assert!(r.quencher.iter().all(|&q| q == 0.15));

        let ok = CarParams::default();
        let bad = [
            CarParams {
                peb_time_s: -1.0,
                ..ok.clone()
            },
            CarParams {
                k_amp_per_s: -0.1,
                ..ok.clone()
            },
            CarParams {
                k_quench_per_s: f64::NAN,
                ..ok.clone()
            },
            CarParams {
                quencher_initial: -0.1,
                ..ok.clone()
            },
            CarParams {
                acid_diffusivity_nm2_s: -1.0,
                ..ok.clone()
            },
            CarParams {
                quencher_diffusivity_nm2_s: f64::INFINITY,
                ..ok.clone()
            },
            CarParams {
                vertical_diffusivity_ratio: -1.0,
                ..ok.clone()
            },
            CarParams {
                max_time_step_s: Some(0.0),
                ..ok.clone()
            },
            CarParams {
                max_time_step_s: Some(1e-9),
                ..ok.clone()
            }, // > 10^6 steps
        ];
        for params in &bad {
            assert!(
                car_peb(&acid0, [1.0, 1.0, 1.0], params).is_err(),
                "{params:?}"
            );
        }
        assert!(car_peb(&acid0, [1.0, -1.0, 1.0], &ok).is_err());
        let negative = Array3::from_elem((1, 1, 4), -0.01);
        assert!(car_peb(&negative, [1.0, 1.0, 1.0], &ok).is_err());
        let nan = Array3::from_elem((1, 1, 4), f64::NAN);
        assert!(car_peb(&nan, [1.0, 1.0, 1.0], &ok).is_err());
        // Roundoff-level negatives are clamped, not rejected.
        let tiny = Array3::from_elem((1, 1, 4), -1e-12);
        assert!(car_peb(&tiny, [1.0, 1.0, 1.0], &ok).is_ok());
    }
}
