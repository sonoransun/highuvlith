//! Inverse Lithography Technology (ILT): pixel-based mask optimization with
//! the exact adjoint gradient through the SOCS imaging kernels.
//!
//! The mask is a continuous transmission map `m ∈ (0, 1)` on the simulation
//! grid, parametrized by an unconstrained field `θ` through a sigmoid (and an
//! optional Gaussian density filter that sets a soft minimum feature size).
//! The cost combines an image-fidelity term — mean-squared aerial-image error
//! or a sigmoid-resist contour error — summed over one or more (focus, dose)
//! process conditions, with total-variation and binarization regularizers.
//! Its gradient is computed **exactly** by back-propagating through the same
//! SOCS kernels the engine images with ([`AerialImageEngine::kernels`]),
//! including the engine's uniform flare, and the optimizer is nonlinear
//! conjugate gradient (Polak–Ribière+) or steepest descent with an Armijo
//! backtracking line search, so the cost decreases monotonically.
//!
//! The legacy **local proxy** gradient (`∂C/∂I` used in place of `∂C/∂m`,
//! i.e. `∂I(x)/∂m(x′) ≈ δ(x − x′)`) is kept as [`IltGradient::LocalProxy`]
//! for comparison: it is an approximation that ignores the optical
//! convolution and stalls once the mask saturates at the target shape.
//!
//! # Key equations
//!
//! FFT conventions are those of `Fft2D`: `F` (forward) is the unnormalized
//! DFT `Σ_x e^{−2πi k·x/n}`, `F⁻¹` (inverse) carries `1/N`, `N = n²`. Hence
//! `Fᴴ = N·F⁻¹`, and the adjoint of `L_k t = F⁻¹(K_k ⊙ F t)` under
//! `⟨u, v⟩ = Σ u* v` is `L_kᴴ v = F⁻¹(K_k* ⊙ F v)`.
//!
//! ```text
//!   parametrization   θ̃ = G_s ⊛ θ  (optional),   m = 1 / (1 + e^{−β θ̃})
//!   transmittance     t = a + (1 − a)·m            (a = absorber amplitude; 0 for binary)
//!   forward, focus z  A_k = F⁻¹(K_k(z) ⊙ F t),  I₀ = Σ_k λ_k |A_k|²,  I = (1 − φ) I₀ + φ ⟨I₀⟩
//!   fidelity term     Φ = d·I                     (ImageFidelity)
//!                     Φ = 1 / (1 + e^{−α (d·I − t_r)})   (ResistContour)
//!   cost              C = Σ_c w_c (1/N) Σ_x (Φ_c − Z)²
//!                       + μ_TV (1/N) Σ_x (√(|∇⁺m|² + ε²) − ε) + μ_B (1/N) Σ_x 4 m (1 − m)
//!   image adjoint     G = ∂C/∂I,   G₀ = (1 − φ) G + (φ/N) Σ_x G       (flare)
//!   mask gradient     ∂C/∂m = 2 Re[ (1 − a)* Σ_k λ_k F⁻¹( K_k* ⊙ F(G₀ ⊙ A_k) ) ]  (+ regularizers)
//!   chain rule        ∂C/∂θ = G_s ⊛ ( ∂C/∂m · β m (1 − m) )          (G_s is self-adjoint)
//! ```
//!
//! Derivation of the mask gradient: `δI₀ = Σ_k λ_k 2 Re[A_k* δA_k]` and
//! `δA_k = L_k δt`, so `δC = Σ_x G₀ δI₀ = 2 Re Σ_k λ_k ⟨L_kᴴ(G₀ A_k), δt⟩`;
//! with `δt = (1 − a) δm` for real `δm` this gives the expression above. It
//! is verified against central finite differences in the tests (relative
//! error ≲ 1e-6).
//!
//! The Gaussian density filter has standard deviation `s = min_feature_nm /
//! 1.349`: a θ-box narrower than `min_feature_nm` on an equal-and-opposite
//! background is erased by filter + projection (`erf(w / (2√2 s)) < ½`). It
//! is a soft length-scale control borrowed from density-filtered topology
//! optimization, not a mask-rule check.
//!
//! # Model status
//!
//! - The gradient is exact for the forward model used (SOCS kernels as
//!   provided by the engine, uniform flare, the chosen fidelity term); the
//!   forward model itself inherits the engine's approximations (e.g. kernel
//!   truncation) and the thin-mask transmittance model.
//! - The resist is a constant-threshold sigmoid on the aerial image — no
//!   acid diffusion, development, or etch.
//! - The optimized mask is a continuous pixel map; [`ILTResult::binary_mask`]
//!   thresholds it at `m = 0.5`. No mask-rule check or polygon extraction.
//! - Process-window-aware ILT sums the cost over (focus, dose) conditions;
//!   each focus uses the engine's kernel set at that focus.
//! - With the resist-contour cost, a start whose whole image lies far below
//!   the threshold (`α·(t − I_max) ≳ 10`) saturates the sigmoid: gradients
//!   vanish and the optimizer stops almost at once. Lower the steepness,
//!   start from a printing mask, or warm up with the image-fidelity cost.
//!
//! # References
//!
//! - A. Poonawala and P. Milanfar, "Mask design for optical
//!   microlithography — an inverse imaging problem", IEEE Transactions on
//!   Image Processing 16(3) (2007) — sigmoid mask/resist formulation.

use std::sync::Arc;

use ndarray::{Array2, Zip};
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::aerial::{AerialImageEngine, KernelSet};
use crate::error::{LithographyError, Result};
use crate::math::fft2d::Fft2D;
pub use crate::opc::ProcessCondition;
use crate::opc::{
    apply_flare, fft_freq, intensity_from_fields, kernel_fields, validate_conditions,
};
use crate::types::{Complex64, GridConfig};

/// Gradient used to drive the ILT update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum IltGradient {
    /// Exact adjoint through the SOCS kernels (default).
    #[default]
    Adjoint,
    /// Legacy local proxy: `∂C/∂I` used in place of `∂C/∂m`
    /// (`∂I(x)/∂m(x′) ≈ δ(x − x′)`). An approximation, kept for comparison.
    LocalProxy,
}

/// Image-fidelity term of the ILT cost.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum IltCost {
    /// Mean-squared aerial-image error `(1/N) Σ (d·I − Z)²` (default; the
    /// target is an intensity pattern).
    #[default]
    ImageFidelity,
    /// Sigmoid-resist contour error `(1/N) Σ (R − Z)²`,
    /// `R = 1/(1 + exp(−steepness·(d·I − threshold)))`; the target is the
    /// desired printed (bright) region.
    ResistContour {
        /// Print threshold on the dose-scaled aerial image.
        threshold: f64,
        /// Sigmoid steepness α (1/intensity units), e.g. 25–100.
        steepness: f64,
    },
}

/// Descent method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum IltOptimizer {
    /// Steepest descent with Armijo backtracking.
    SteepestDescent,
    /// Polak–Ribière+ nonlinear conjugate gradient with Armijo backtracking
    /// and automatic restarts (default).
    #[default]
    ConjugateGradient,
}

/// Initial mask.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum IltInit {
    /// Start from the target pattern: `θ = Z − ½` (default).
    #[default]
    Target,
    /// Uniform transmission `m₀ ∈ (0, 1)`.
    Uniform(f64),
    /// A given transmission map (clamped to `[0.01, 0.99]`).
    Mask(Array2<f64>),
}

/// Why the optimizer stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IltTermination {
    /// Relative cost decrease stayed below `convergence_tol` for three
    /// consecutive iterations, or the gradient vanished.
    Converged,
    /// The iteration budget was spent.
    MaxIterations,
    /// No step along the search direction decreased the cost (the typical
    /// stall of the local proxy).
    LineSearchFailed,
}

/// ILT optimization configuration.
#[derive(Debug, Clone)]
pub struct ILTConfig {
    /// Target pattern on the engine grid (1 = bright / printed, 0 = dark).
    pub target: Array2<f64>,
    /// Initial trial step of the line search, as the maximum |Δθ| of one
    /// update (adapted every iteration).
    pub learning_rate: f64,
    /// Total-variation regularization weight μ_TV.
    pub regularization: f64,
    /// Maximum iterations.
    pub max_iterations: usize,
    /// Convergence tolerance on the relative cost decrease per iteration.
    pub convergence_tol: f64,
    /// Soft minimum feature size (nm) of the Gaussian density filter
    /// (0 = no filter). See the module docs for the exact definition.
    pub min_feature_nm: f64,
    /// Gradient (exact adjoint by default).
    pub gradient: IltGradient,
    /// Descent method.
    pub optimizer: IltOptimizer,
    /// Image-fidelity term.
    pub cost: IltCost,
    /// Process conditions (focus, dose, weight). Default: nominal only.
    pub conditions: Vec<ProcessCondition>,
    /// Sigmoid steepness β of the mask parametrization `m = σ(β θ̃)`.
    pub sigmoid_steepness: f64,
    /// Binarization (manufacturability) penalty weight μ_B on `4 m (1 − m)`.
    pub binarization_weight: f64,
    /// Continuation: run this many iterations with μ_B = 0 before switching
    /// the binarization penalty on (0 = on from the start). A penalty that is
    /// active from the first iteration tends to freeze the mask at the
    /// target shape before the fidelity term has grown it.
    pub binarization_after: usize,
    /// Smoothing ε of the total variation.
    pub tv_epsilon: f64,
    /// Complex amplitude of the `m = 0` (absorber) state: 0 for a binary
    /// mask, `√T·e^{iφ}` for an attenuated PSM.
    pub absorber: Complex64,
    /// Initial mask.
    pub init: IltInit,
    /// Record the mask every this many iterations (0 = no snapshots).
    pub snapshot_every: usize,
}

impl Default for ILTConfig {
    fn default() -> Self {
        Self {
            target: Array2::zeros((1, 1)),
            learning_rate: 0.5,
            regularization: 0.01,
            max_iterations: 100,
            convergence_tol: 1e-4,
            min_feature_nm: 0.0,
            gradient: IltGradient::Adjoint,
            optimizer: IltOptimizer::ConjugateGradient,
            cost: IltCost::ImageFidelity,
            conditions: vec![ProcessCondition::nominal()],
            sigmoid_steepness: 4.0,
            binarization_weight: 0.0,
            binarization_after: 0,
            tv_epsilon: 0.01,
            absorber: Complex64::new(0.0, 0.0),
            init: IltInit::Target,
            snapshot_every: 0,
        }
    }
}

/// ILT optimization result.
#[derive(Debug, Clone)]
pub struct ILTResult {
    /// Optimized continuous mask transmission `m ∈ (0, 1)`.
    pub mask_transmittance: Array2<f64>,
    /// Aerial image of the optimized mask at the first condition's focus
    /// (not dose-scaled).
    pub aerial_image: Array2<f64>,
    /// Cost of the initial mask followed by the cost after each iteration.
    /// With a binarization warm-up the objective changes at each entry of
    /// `stage_starts` (the cost is monotone within a stage).
    pub cost_history: Vec<f64>,
    /// Indices into `cost_history` where each optimization stage starts
    /// (`[0]`, or `[0, k]` with a binarization warm-up).
    pub stage_starts: Vec<usize>,
    /// Final cost value.
    pub final_cost: f64,
    /// Number of optimizer iterations (`cost_history.len() − stage_starts.len()`).
    pub iterations: usize,
    /// Whether the optimizer reached [`IltTermination::Converged`].
    pub converged: bool,
    /// Why the optimizer stopped.
    pub termination: IltTermination,
    /// Final unconstrained parameters θ.
    pub theta: Array2<f64>,
    /// `m > 0.5` thresholded to 0/1.
    pub binary_mask: Array2<f64>,
    /// Aerial image of the binary mask at the first condition's focus.
    pub binary_aerial_image: Array2<f64>,
    /// Pixels whose printed state (`dose·I > print threshold`) differs from
    /// the target (`Z > 0.5`), continuous mask, first condition.
    pub pattern_error: usize,
    /// The same count for the binary mask.
    pub binary_pattern_error: usize,
    /// L2 norm of ∂C/∂θ at the initial mask and after each iteration.
    pub gradient_norm_history: Vec<f64>,
    /// `(iteration, m)` snapshots (iteration 0, every `snapshot_every`, and
    /// the final mask) when `snapshot_every > 0`.
    pub snapshots: Vec<(usize, Array2<f64>)>,
}

/// Count pixels whose printed state (`image > threshold`) differs from the
/// target (`target > 0.5`).
pub fn pattern_error(image: &Array2<f64>, target: &Array2<f64>, threshold: f64) -> usize {
    image
        .iter()
        .zip(target.iter())
        .filter(|(&i, &z)| (i > threshold) != (z > 0.5))
        .count()
}

/// Create a target aerial image for a line/space pattern (bright spaces of
/// width `pitch − cd`, dark lines of width `cd`).
pub fn create_target_line_space(cd_nm: f64, pitch_nm: f64, grid: &GridConfig) -> Array2<f64> {
    let n = grid.size;
    let field = grid.field_size_nm();
    let half = field / 2.0;
    let pixel = grid.pixel_nm;

    let mut target = Array2::zeros((n, n));
    for i in 0..n {
        for j in 0..n {
            let x = -half + (j as f64 + 0.5) * pixel;
            // Periodic pattern: space (bright) / line (dark)
            let x_mod = ((x / pitch_nm) + 0.5).rem_euclid(1.0);
            let space_frac = (pitch_nm - cd_nm) / pitch_nm;
            target[[i, j]] = if x_mod < space_frac { 1.0 } else { 0.0 };
        }
    }
    target
}

/// Build a target from an indicator function of physical position (nm),
/// area-averaged over `supersample²` sub-pixel samples per pixel.
pub fn create_target_from_fn(
    grid: &GridConfig,
    supersample: usize,
    inside: impl Fn(f64, f64) -> bool,
) -> Array2<f64> {
    let n = grid.size;
    let p = grid.pixel_nm;
    let half = grid.field_size_nm() / 2.0;
    let s = supersample.max(1);
    let inv = 1.0 / (s * s) as f64;
    Array2::from_shape_fn((n, n), |(i, j)| {
        let mut hits = 0usize;
        for a in 0..s {
            let y = -half + (i as f64 + (a as f64 + 0.5) / s as f64) * p;
            for b in 0..s {
                let x = -half + (j as f64 + (b as f64 + 0.5) / s as f64) * p;
                if inside(x, y) {
                    hits += 1;
                }
            }
        }
        hits as f64 * inv
    })
}

/// Shape of the holes in [`create_target_contact_array`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContactShape {
    /// Square holes of side `size`.
    Square,
    /// Round holes of diameter `size` (a curvilinear target).
    Round,
}

/// Target for a centred `count_x × count_y` array of holes (value 1 inside)
/// on a dark background, area-averaged at the pixel edges.
pub fn create_target_contact_array(
    size_nm: f64,
    pitch_x_nm: f64,
    pitch_y_nm: f64,
    count_x: usize,
    count_y: usize,
    shape: ContactShape,
    grid: &GridConfig,
) -> Array2<f64> {
    let centres: Vec<(f64, f64)> = (0..count_y)
        .flat_map(|iy| {
            (0..count_x).map(move |ix| {
                (
                    (ix as f64 - (count_x as f64 - 1.0) / 2.0) * pitch_x_nm,
                    (iy as f64 - (count_y as f64 - 1.0) / 2.0) * pitch_y_nm,
                )
            })
        })
        .collect();
    let r = size_nm / 2.0;
    create_target_from_fn(grid, 4, |x, y| {
        centres.iter().any(|&(cx, cy)| match shape {
            ContactShape::Square => (x - cx).abs() <= r && (y - cy).abs() <= r,
            ContactShape::Round => (x - cx).powi(2) + (y - cy).powi(2) <= r * r,
        })
    })
}

/// The ILT objective bound to an engine: evaluates the mask
/// parametrization, the cost, and its exact (or proxy) gradient.
///
/// [`optimize_ilt`] builds one internally; it is public so the gradient can
/// be checked or used by other optimizers.
pub struct IltProblem {
    grid: GridConfig,
    target: Array2<f64>,
    cost: IltCost,
    conditions: Vec<ProcessCondition>,
    kernel_sets: Vec<Arc<KernelSet>>,
    flare: f64,
    beta: f64,
    tv_weight: f64,
    tv_eps: f64,
    bin_weight: f64,
    absorber: Complex64,
    /// Gaussian filter transfer function (FFT layout), if enabled.
    filter: Option<Array2<f64>>,
    gradient: IltGradient,
    fft: Fft2D,
}

impl IltProblem {
    /// Bind `config` to `engine` (fetches one kernel set per condition).
    pub fn new(engine: &AerialImageEngine, config: &ILTConfig) -> Result<Self> {
        let grid = engine.grid().clone();
        let n = grid.size;
        if config.target.dim() != (n, n) {
            return Err(LithographyError::DimensionMismatch {
                expected: format!("{n}x{n} target (engine grid)"),
                got: format!("{}x{}", config.target.nrows(), config.target.ncols()),
            });
        }
        if config.target.iter().any(|v| !v.is_finite()) {
            return Err(LithographyError::InvalidParameter {
                name: "target",
                value: f64::NAN,
                reason: "must be finite",
            });
        }
        validate_conditions(&config.conditions)?;
        let positive = |name: &'static str, v: f64| {
            if v.is_finite() && v > 0.0 {
                Ok(())
            } else {
                Err(LithographyError::InvalidParameter {
                    name,
                    value: v,
                    reason: "must be positive and finite",
                })
            }
        };
        let non_negative = |name: &'static str, v: f64| {
            if v.is_finite() && v >= 0.0 {
                Ok(())
            } else {
                Err(LithographyError::InvalidParameter {
                    name,
                    value: v,
                    reason: "must be non-negative and finite",
                })
            }
        };
        positive("sigmoid_steepness", config.sigmoid_steepness)?;
        positive("learning_rate", config.learning_rate)?;
        positive("tv_epsilon", config.tv_epsilon)?;
        non_negative("regularization", config.regularization)?;
        non_negative("binarization_weight", config.binarization_weight)?;
        non_negative("min_feature_nm", config.min_feature_nm)?;
        if let IltCost::ResistContour {
            threshold,
            steepness,
        } = config.cost
        {
            positive("threshold", threshold)?;
            positive("steepness", steepness)?;
        }

        let filter = if config.min_feature_nm > 0.0 {
            let s = config.min_feature_nm / 1.349;
            let df = grid.freq_step();
            let pi2 = std::f64::consts::PI * std::f64::consts::PI;
            Some(Array2::from_shape_fn((n, n), |(i, j)| {
                let fx = fft_freq(j, n, df);
                let fy = fft_freq(i, n, df);
                (-2.0 * pi2 * s * s * (fx * fx + fy * fy)).exp()
            }))
        } else {
            None
        };

        let kernel_sets = config
            .conditions
            .iter()
            .map(|c| engine.kernels(c.defocus_nm))
            .collect();

        Ok(Self {
            grid,
            target: config.target.clone(),
            cost: config.cost,
            conditions: config.conditions.clone(),
            kernel_sets,
            flare: engine.flare_fraction(),
            beta: config.sigmoid_steepness,
            tv_weight: config.regularization,
            tv_eps: config.tv_epsilon,
            bin_weight: config.binarization_weight,
            absorber: config.absorber,
            filter,
            gradient: config.gradient,
            fft: Fft2D::new(),
        })
    }

    /// Grid the problem lives on.
    pub fn grid(&self) -> &GridConfig {
        &self.grid
    }

    /// Print threshold used for pattern-error counts: the resist threshold,
    /// or 0.5 for the image-fidelity cost.
    pub fn print_threshold(&self) -> f64 {
        match self.cost {
            IltCost::ResistContour { threshold, .. } => threshold,
            IltCost::ImageFidelity => 0.5,
        }
    }

    /// Initial θ for an initialization choice.
    pub fn initial_theta(&self, init: &IltInit) -> Result<Array2<f64>> {
        let logit = |m: f64| {
            let m = m.clamp(0.01, 0.99);
            (m / (1.0 - m)).ln() / self.beta
        };
        match init {
            IltInit::Target => Ok(self.target.mapv(|z| z - 0.5)),
            IltInit::Uniform(m0) => {
                if !(m0.is_finite() && *m0 > 0.0 && *m0 < 1.0) {
                    return Err(LithographyError::InvalidParameter {
                        name: "init",
                        value: *m0,
                        reason: "uniform initial transmission must be in (0, 1)",
                    });
                }
                Ok(Array2::from_elem(self.target.dim(), logit(*m0)))
            }
            IltInit::Mask(m) => {
                if m.dim() != self.target.dim() {
                    return Err(LithographyError::DimensionMismatch {
                        expected: format!("{:?}", self.target.dim()),
                        got: format!("{:?}", m.dim()),
                    });
                }
                Ok(m.mapv(logit))
            }
        }
    }

    fn filtered(&self, x: &Array2<f64>) -> Array2<f64> {
        match &self.filter {
            None => x.clone(),
            Some(h) => {
                let mut spec = x.mapv(|v| Complex64::new(v, 0.0));
                self.fft.forward(&mut spec);
                spec.zip_mut_with(h, |s, g| *s *= g);
                self.fft.inverse(&mut spec);
                spec.mapv(|c| c.re)
            }
        }
    }

    /// Mask transmission `m = σ(β · G_s ⊛ θ)`.
    pub fn mask(&self, theta: &Array2<f64>) -> Array2<f64> {
        let beta = self.beta;
        self.filtered(theta)
            .mapv(|v| 1.0 / (1.0 + (-beta * v).exp()))
    }

    fn spectrum(&self, m: &Array2<f64>) -> Array2<Complex64> {
        let a = self.absorber;
        let one_minus_a = Complex64::new(1.0, 0.0) - a;
        let mut t = m.mapv(|v| a + one_minus_a * v);
        self.fft.forward(&mut t);
        t
    }

    /// Aerial image (flare included, not dose-scaled) of mask `m` at the
    /// focus of condition `condition`.
    pub fn image(&self, m: &Array2<f64>, condition: usize) -> Array2<f64> {
        let kset = &self.kernel_sets[condition];
        let spec = self.spectrum(m);
        let fields = kernel_fields(kset, &spec, &self.fft);
        let mut image = intensity_from_fields(kset, &fields);
        apply_flare(&mut image, self.flare);
        image
    }

    /// Fidelity cost of one condition and, optionally, `∂C_c/∂I`.
    fn data_term(
        &self,
        image: &Array2<f64>,
        cond: &ProcessCondition,
        want_gradient: bool,
    ) -> (f64, Option<Array2<f64>>) {
        let inv_n = 1.0 / image.len() as f64;
        let d = cond.dose;
        let mut cost = 0.0;
        let mut grad = if want_gradient {
            Some(Array2::zeros(image.dim()))
        } else {
            None
        };
        match self.cost {
            IltCost::ImageFidelity => {
                for ((idx, &i), &z) in image.indexed_iter().zip(self.target.iter()) {
                    let r = d * i - z;
                    cost += r * r;
                    if let Some(g) = grad.as_mut() {
                        g[idx] = 2.0 * r * d * inv_n;
                    }
                }
            }
            IltCost::ResistContour {
                threshold,
                steepness,
            } => {
                for ((idx, &i), &z) in image.indexed_iter().zip(self.target.iter()) {
                    let rr = 1.0 / (1.0 + (-steepness * (d * i - threshold)).exp());
                    let r = rr - z;
                    cost += r * r;
                    if let Some(g) = grad.as_mut() {
                        g[idx] = 2.0 * r * steepness * rr * (1.0 - rr) * d * inv_n;
                    }
                }
            }
        }
        (cost * inv_n, grad)
    }

    /// Regularization cost and its gradient with respect to `m`.
    fn regularization(&self, m: &Array2<f64>) -> (f64, Array2<f64>) {
        let (ny, nx) = m.dim();
        let inv_n = 1.0 / (nx * ny) as f64;
        let mut grad = Array2::zeros((ny, nx));
        let mut cost = 0.0;
        if self.tv_weight > 0.0 {
            let eps = self.tv_eps;
            let mut dx = Array2::zeros((ny, nx));
            let mut dy = Array2::zeros((ny, nx));
            let mut s = Array2::zeros((ny, nx));
            for i in 0..ny {
                for j in 0..nx {
                    let gx = m[[i, (j + 1) % nx]] - m[[i, j]];
                    let gy = m[[(i + 1) % ny, j]] - m[[i, j]];
                    let si = (gx * gx + gy * gy + eps * eps).sqrt();
                    dx[[i, j]] = gx;
                    dy[[i, j]] = gy;
                    s[[i, j]] = si;
                    cost += self.tv_weight * (si - eps) * inv_n;
                }
            }
            for i in 0..ny {
                for j in 0..nx {
                    let jl = (j + nx - 1) % nx;
                    let iu = (i + ny - 1) % ny;
                    let g = -(dx[[i, j]] + dy[[i, j]]) / s[[i, j]]
                        + dx[[i, jl]] / s[[i, jl]]
                        + dy[[iu, j]] / s[[iu, j]];
                    grad[[i, j]] += self.tv_weight * g * inv_n;
                }
            }
        }
        if self.bin_weight > 0.0 {
            Zip::from(&mut grad).and(m).for_each(|g, &v| {
                cost += self.bin_weight * 4.0 * v * (1.0 - v) * inv_n;
                *g += self.bin_weight * 4.0 * (1.0 - 2.0 * v) * inv_n;
            });
        }
        (cost, grad)
    }

    /// Total cost at θ.
    pub fn cost(&self, theta: &Array2<f64>) -> f64 {
        let m = self.mask(theta);
        let spec = self.spectrum(&m);
        let mut total = 0.0;
        for (kset, cond) in self.kernel_sets.iter().zip(&self.conditions) {
            let fields = kernel_fields(kset, &spec, &self.fft);
            let mut image = intensity_from_fields(kset, &fields);
            apply_flare(&mut image, self.flare);
            total += cond.weight * self.data_term(&image, cond, false).0;
        }
        total + self.regularization(&m).0
    }

    /// Cost and gradient `∂C/∂θ` at θ, using the configured gradient kind.
    pub fn cost_and_gradient(&self, theta: &Array2<f64>) -> (f64, Array2<f64>) {
        self.cost_and_gradient_with(theta, self.gradient)
    }

    /// Cost and gradient `∂C/∂θ` at θ with an explicit gradient kind.
    pub fn cost_and_gradient_with(
        &self,
        theta: &Array2<f64>,
        kind: IltGradient,
    ) -> (f64, Array2<f64>) {
        let m = self.mask(theta);
        let spec = self.spectrum(&m);
        let n_pix = m.len() as f64;
        let conj_c = (Complex64::new(1.0, 0.0) - self.absorber).conj();
        let mut total = 0.0;
        let mut dcdm: Array2<f64> = Array2::zeros(m.dim());

        for (kset, cond) in self.kernel_sets.iter().zip(&self.conditions) {
            let fields = kernel_fields(kset, &spec, &self.fft);
            let mut image = intensity_from_fields(kset, &fields);
            apply_flare(&mut image, self.flare);
            let (c, g) = self.data_term(&image, cond, true);
            total += cond.weight * c;
            let g = g.expect("gradient requested");
            // Flare adjoint: G0 = (1 − φ) G + (φ/N) Σ G.
            let g_sum = g.sum();
            let g0 = g.mapv(|v| (1.0 - self.flare) * v + self.flare * g_sum / n_pix);

            match kind {
                IltGradient::Adjoint => {
                    let back = |(kernel, (lambda, a)): (
                        &Array2<Complex64>,
                        (&f64, &Array2<Complex64>),
                    )| {
                        let mut v = Array2::from_shape_fn(a.dim(), |idx| a[idx] * g0[idx]);
                        self.fft.forward(&mut v);
                        v.zip_mut_with(kernel, |x, k| *x *= k.conj());
                        self.fft.inverse(&mut v);
                        v.mapv_inplace(|x| x * *lambda);
                        v
                    };
                    #[cfg(feature = "parallel")]
                    let parts: Vec<Array2<Complex64>> = kset
                        .kernels
                        .par_iter()
                        .zip(kset.eigenvalues.par_iter().zip(fields.par_iter()))
                        .map(back)
                        .collect();
                    #[cfg(not(feature = "parallel"))]
                    let parts: Vec<Array2<Complex64>> = kset
                        .kernels
                        .iter()
                        .zip(kset.eigenvalues.iter().zip(fields.iter()))
                        .map(back)
                        .collect();
                    let w2 = 2.0 * cond.weight;
                    for part in &parts {
                        dcdm.zip_mut_with(part, |d, q| *d += w2 * (conj_c * q).re);
                    }
                }
                IltGradient::LocalProxy => {
                    dcdm.zip_mut_with(&g0, |d, g| *d += cond.weight * g);
                }
            }
        }

        let (reg_cost, reg_grad) = self.regularization(&m);
        total += reg_cost;
        dcdm += &reg_grad;

        // Chain through the sigmoid and the (self-adjoint) density filter.
        let beta = self.beta;
        Zip::from(&mut dcdm)
            .and(&m)
            .for_each(|d, &v| *d *= beta * v * (1.0 - v));
        (total, self.filtered(&dcdm))
    }
}

fn dot(a: &Array2<f64>, b: &Array2<f64>) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Mutable optimizer state shared across stages.
struct IltTrace {
    cost_history: Vec<f64>,
    gradient_norm_history: Vec<f64>,
    snapshots: Vec<(usize, Array2<f64>)>,
    iterations: usize,
}

/// One optimization stage on a fixed objective: CG / steepest descent with
/// Armijo backtracking. Returns the final θ and why the stage stopped.
fn run_stage(
    problem: &IltProblem,
    config: &ILTConfig,
    mut theta: Array2<f64>,
    max_iterations: usize,
    trace: &mut IltTrace,
) -> (Array2<f64>, IltTermination) {
    const ARMIJO_C1: f64 = 1e-4;
    const MAX_BACKTRACKS: usize = 25;
    let (mut cost, mut grad) = problem.cost_and_gradient(&theta);
    trace.cost_history.push(cost);
    trace.gradient_norm_history.push(dot(&grad, &grad).sqrt());

    let use_cg = config.optimizer == IltOptimizer::ConjugateGradient
        && config.gradient == IltGradient::Adjoint;
    let mut direction = grad.mapv(|g| -g);
    let mut step = config.learning_rate;
    let mut small_decreases = 0usize;

    for _ in 0..max_iterations {
        let mut accepted: Option<(f64, Array2<f64>, f64)> = None;
        for attempt in 0..2 {
            let d_max = direction.iter().fold(0.0_f64, |a, v| a.max(v.abs()));
            if d_max == 0.0 {
                break;
            }
            let unit = direction.mapv(|v| v / d_max);
            let slope = dot(&grad, &unit);
            if slope >= 0.0 && config.gradient == IltGradient::Adjoint {
                // Not a descent direction: restart along −∇C.
                direction = grad.mapv(|g| -g);
                continue;
            }
            let mut s = step;
            for _ in 0..MAX_BACKTRACKS {
                let trial = &theta + &unit.mapv(|v| s * v);
                let c_trial = problem.cost(&trial);
                let ok = match config.gradient {
                    IltGradient::Adjoint => c_trial <= cost + ARMIJO_C1 * s * slope,
                    IltGradient::LocalProxy => c_trial < cost,
                };
                if ok && c_trial.is_finite() {
                    accepted = Some((c_trial, trial, s));
                    break;
                }
                s *= 0.5;
            }
            if accepted.is_some() || attempt == 1 || !use_cg {
                break;
            }
            // CG direction failed: retry once along steepest descent.
            direction = grad.mapv(|g| -g);
        }

        let Some((_, theta_new, s_used)) = accepted else {
            let stalled = trace.gradient_norm_history.last().copied().unwrap_or(0.0) > 0.0;
            return (
                theta,
                if stalled {
                    IltTermination::LineSearchFailed
                } else {
                    IltTermination::Converged
                },
            );
        };

        theta = theta_new;
        let (c_new, grad_new) = problem.cost_and_gradient(&theta);
        let prev_cost = cost;
        cost = c_new;
        trace.iterations += 1;
        trace.cost_history.push(cost);
        trace
            .gradient_norm_history
            .push(dot(&grad_new, &grad_new).sqrt());
        if config.snapshot_every > 0 && trace.iterations.is_multiple_of(config.snapshot_every) {
            trace
                .snapshots
                .push((trace.iterations, problem.mask(&theta)));
        }

        // Next direction.
        if use_cg {
            let gg = dot(&grad, &grad);
            let beta_pr = if gg > 0.0 {
                ((dot(&grad_new, &grad_new) - dot(&grad_new, &grad)) / gg).max(0.0)
            } else {
                0.0
            };
            direction = &grad_new.mapv(|g| -g) + &direction.mapv(|d| beta_pr * d);
        } else {
            direction = grad_new.mapv(|g| -g);
        }
        grad = grad_new;
        step = (s_used * 1.5).min(10.0 * config.learning_rate);

        let rel = if prev_cost > 0.0 {
            (prev_cost - cost) / prev_cost
        } else {
            0.0
        };
        if rel < config.convergence_tol {
            small_decreases += 1;
        } else {
            small_decreases = 0;
        }
        if small_decreases >= 3 || cost == 0.0 {
            return (theta, IltTermination::Converged);
        }
    }
    (theta, IltTermination::MaxIterations)
}

/// Run ILT optimization.
///
/// Builds an [`IltProblem`], starts from `config.init`, and iterates
/// (conjugate gradient or steepest descent) with an Armijo backtracking line
/// search on the direction normalized to unit max-norm, so each accepted step
/// strictly decreases the cost. With [`IltGradient::LocalProxy`] the search
/// direction is the proxy (steepest descent only) and a step is accepted on
/// plain decrease. A stage stops on convergence (relative decrease below
/// `convergence_tol` three times in a row), line-search failure, or its
/// iteration budget. With `binarization_weight > 0` and
/// `binarization_after = k > 0`, the first stage runs `k` iterations without
/// the binarization penalty and a second stage spends the remaining budget
/// with it (continuation).
pub fn optimize_ilt(engine: &AerialImageEngine, config: &ILTConfig) -> Result<ILTResult> {
    let mut problem = IltProblem::new(engine, config)?;
    let mut theta = problem.initial_theta(&config.init)?;
    let mut trace = IltTrace {
        cost_history: Vec::new(),
        gradient_norm_history: Vec::new(),
        snapshots: Vec::new(),
        iterations: 0,
    };
    if config.snapshot_every > 0 {
        trace.snapshots.push((0, problem.mask(&theta)));
    }

    let warmup = if config.binarization_weight > 0.0 {
        config.binarization_after.min(config.max_iterations)
    } else {
        0
    };
    let mut stage_starts = Vec::new();
    let mut termination = IltTermination::MaxIterations;
    if warmup > 0 {
        problem.bin_weight = 0.0;
        stage_starts.push(trace.cost_history.len());
        let (t, _) = run_stage(&problem, config, theta, warmup, &mut trace);
        theta = t;
        problem.bin_weight = config.binarization_weight;
    }
    let remaining = config.max_iterations - trace.iterations;
    if remaining > 0 || stage_starts.is_empty() {
        stage_starts.push(trace.cost_history.len());
        let (t, term) = run_stage(&problem, config, theta, remaining, &mut trace);
        theta = t;
        termination = term;
    }

    let m = problem.mask(&theta);
    if config.snapshot_every > 0 && trace.snapshots.last().map(|s| s.0) != Some(trace.iterations) {
        trace.snapshots.push((trace.iterations, m.clone()));
    }
    let binary_mask = m.mapv(|v| if v > 0.5 { 1.0 } else { 0.0 });
    let aerial_image = problem.image(&m, 0);
    let binary_aerial_image = problem.image(&binary_mask, 0);
    let dose0 = config.conditions[0].dose;
    let t_print = problem.print_threshold();
    let pattern_error =
        self::pattern_error(&aerial_image.mapv(|v| dose0 * v), &config.target, t_print);
    let binary_pattern_error = self::pattern_error(
        &binary_aerial_image.mapv(|v| dose0 * v),
        &config.target,
        t_print,
    );
    let final_cost = *trace.cost_history.last().unwrap_or(&f64::INFINITY);

    Ok(ILTResult {
        mask_transmittance: m,
        aerial_image,
        final_cost,
        cost_history: trace.cost_history,
        stage_starts,
        iterations: trace.iterations,
        converged: termination == IltTermination::Converged,
        termination,
        theta,
        binary_mask,
        binary_aerial_image,
        pattern_error,
        binary_pattern_error,
        gradient_norm_history: trace.gradient_norm_history,
        snapshots: trace.snapshots,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optics::ProjectionOptics;
    use crate::source::{IlluminationShape, VuvSource};

    /// F2 157.63 nm, NA 0.75 (2% flare), conventional σ = 0.6, 64 × 8 nm
    /// grid (512 nm field), 8 SOCS kernels.
    fn test_engine() -> AerialImageEngine {
        let source = VuvSource {
            illumination: IlluminationShape::Conventional { sigma: 0.6 },
            ..VuvSource::f2_laser(0.6).unwrap()
        };
        let optics = ProjectionOptics::new(0.75).unwrap();
        let grid = GridConfig {
            size: 64,
            pixel_nm: 8.0,
        };
        AerialImageEngine::new(&source, &optics, grid, 8).unwrap()
    }

    /// 2 × 2 array of 100 nm round contacts on a 200 nm pitch (k₁ ≈ 0.48).
    fn contact_target(grid: &GridConfig) -> Array2<f64> {
        create_target_contact_array(100.0, 200.0, 200.0, 2, 2, ContactShape::Round, grid)
    }

    const RESIST: IltCost = IltCost::ResistContour {
        threshold: 0.25,
        steepness: 50.0,
    };

    #[test]
    fn test_create_target() {
        let grid = GridConfig {
            size: 64,
            pixel_nm: 2.0,
        };
        let target = create_target_line_space(65.0, 180.0, &grid);
        assert_eq!(target.dim(), (64, 64));
        // Should have both 0 and 1 values
        assert!(target.iter().any(|&v| v > 0.5));
        assert!(target.iter().any(|&v| v < 0.5));
    }

    #[test]
    fn test_contact_target_area() {
        // Area-averaged targets integrate to the geometric hole area.
        let grid = GridConfig {
            size: 64,
            pixel_nm: 8.0,
        };
        let px_area = 64.0;
        let square =
            create_target_contact_array(96.0, 200.0, 200.0, 2, 2, ContactShape::Square, &grid);
        let round =
            create_target_contact_array(100.0, 200.0, 200.0, 2, 2, ContactShape::Round, &grid);
        let sq_area = square.sum() * px_area;
        let rd_area = round.sum() * px_area;
        assert!(
            (sq_area - 4.0 * 96.0 * 96.0).abs() / (4.0 * 96.0 * 96.0) < 0.01,
            "{sq_area}"
        );
        let disk = 4.0 * std::f64::consts::PI * 50.0 * 50.0;
        assert!((rd_area - disk).abs() / disk < 0.01, "{rd_area} vs {disk}");
    }

    #[test]
    fn test_pattern_error_counts_mismatches() {
        let target = Array2::from_shape_fn((4, 4), |(i, _)| if i < 2 { 1.0 } else { 0.0 });
        let mut image = target.mapv(|z| if z > 0.5 { 0.8 } else { 0.1 });
        assert_eq!(pattern_error(&image, &target, 0.5), 0);
        image[[0, 0]] = 0.2; // missing print
        image[[3, 3]] = 0.9; // extra print
        assert_eq!(pattern_error(&image, &target, 0.5), 2);
    }

    #[test]
    fn test_regularizer_fixtures() {
        // Vertical step m = 1 for j < 32: with periodic forward differences,
        // only columns 31 (dx = −1) and 63 (dx = +1, wrap) contribute
        // √(1 + ε²) − ε each; 2·64 such pixels over N = 4096.
        let engine = test_engine();
        let grid = engine.grid().clone();
        let config = ILTConfig {
            target: contact_target(&grid),
            regularization: 1.0,
            binarization_weight: 1.0,
            tv_epsilon: 0.01,
            ..Default::default()
        };
        let problem = IltProblem::new(&engine, &config).unwrap();
        let step = Array2::from_shape_fn((64, 64), |(_, j)| if j < 32 { 1.0 } else { 0.0 });
        let (cost, _) = problem.regularization(&step);
        let expected = 128.0 * ((1.0_f64 + 1e-4).sqrt() - 0.01) / 4096.0;
        assert!((cost - expected).abs() < 1e-12, "{cost} vs {expected}");
        // Uniform gray: no TV, binarization penalty 4·½·½ = 1.
        let gray = Array2::from_elem((64, 64), 0.5);
        let (cost, grad) = problem.regularization(&gray);
        assert!((cost - 1.0).abs() < 1e-12);
        assert!(grad.iter().all(|g| g.abs() < 1e-15));
    }

    #[test]
    fn test_forward_model_matches_engine() {
        // The ILT forward model (kernels + flare) must reproduce the engine
        // image exactly, in focus and defocused (contract C1 invariant).
        let engine = test_engine();
        let grid = engine.grid().clone();
        let config = ILTConfig {
            target: contact_target(&grid),
            conditions: vec![
                ProcessCondition::nominal(),
                ProcessCondition::new(120.0, 1.0),
            ],
            ..Default::default()
        };
        let problem = IltProblem::new(&engine, &config).unwrap();
        let m = Array2::from_shape_fn((64, 64), |(i, j)| {
            0.5 + 0.45 * ((i as f64 * 0.3).sin() * (j as f64 * 0.17).cos())
        });
        let t = m.mapv(|v| Complex64::new(v, 0.0));
        for (c, z) in [(0usize, 0.0), (1, 120.0)] {
            let ours = problem.image(&m, c);
            let theirs = engine.compute_from_transmittance(&t, z).data;
            let scale = theirs.iter().cloned().fold(0.0, f64::max);
            for (a, b) in ours.iter().zip(theirs.iter()) {
                assert!((a - b).abs() <= 1e-10 * scale, "z={z}: {a} vs {b}");
            }
        }
    }

    #[test]
    fn test_adjoint_gradient_matches_finite_differences() {
        // Central differences of the full cost (fidelity over two focus/dose
        // conditions + TV + binarization) against the adjoint gradient, for
        // both fidelity terms, a binary and an attenuated-PSM absorber, and
        // with/without the density filter.
        let engine = test_engine();
        let grid = engine.grid().clone();
        let target = contact_target(&grid);
        let att_psm = Complex64::from_polar(0.06_f64.sqrt(), std::f64::consts::PI);
        let cases = [
            (IltCost::ImageFidelity, Complex64::new(0.0, 0.0), 0.0),
            (RESIST, att_psm, 40.0),
        ];
        for (cost, absorber, min_feature_nm) in cases {
            let config = ILTConfig {
                target: target.clone(),
                cost,
                absorber,
                min_feature_nm,
                regularization: 0.05,
                binarization_weight: 0.02,
                conditions: vec![
                    ProcessCondition::nominal(),
                    ProcessCondition {
                        defocus_nm: 100.0,
                        dose: 1.05,
                        weight: 0.5,
                    },
                ],
                ..Default::default()
            };
            let problem = IltProblem::new(&engine, &config).unwrap();
            let base = problem.initial_theta(&IltInit::Target).unwrap();
            let theta = Array2::from_shape_fn(base.dim(), |(i, j)| {
                base[[i, j]] + 0.3 * (i as f64 * 0.37).sin() * (j as f64 * 0.23).cos()
            });
            let (_, grad) = problem.cost_and_gradient(&theta);
            let h = 1e-5;
            // Directional derivatives along three pseudo-random directions.
            for d in 0..3usize {
                let v = Array2::from_shape_fn(theta.dim(), |(i, j)| {
                    ((i * 31 + j * 17 + d * 7) % 13) as f64 / 13.0 - 0.5
                });
                let fd = (problem.cost(&(&theta + &v.mapv(|x| x * h)))
                    - problem.cost(&(&theta - &v.mapv(|x| x * h))))
                    / (2.0 * h);
                let an = dot(&grad, &v);
                let rel = (fd - an).abs() / an.abs();
                assert!(rel < 1e-6, "{cost:?}: directional rel err {rel:.2e}");
            }
            // Single pixels with the largest gradient.
            let mut idx: Vec<(usize, usize)> = grad.indexed_iter().map(|(ij, _)| ij).collect();
            idx.sort_by(|a, b| grad[*b].abs().total_cmp(&grad[*a].abs()));
            for &(i, j) in idx.iter().take(3) {
                let mut tp = theta.clone();
                tp[[i, j]] += h;
                let mut tm = theta.clone();
                tm[[i, j]] -= h;
                let fd = (problem.cost(&tp) - problem.cost(&tm)) / (2.0 * h);
                let rel = (fd - grad[[i, j]]).abs() / grad[[i, j]].abs();
                assert!(rel < 1e-5, "{cost:?}: pixel ({i},{j}) rel err {rel:.2e}");
            }
        }
    }

    #[test]
    fn test_cost_decreases_monotonically() {
        let engine = test_engine();
        let grid = engine.grid().clone();
        let config = ILTConfig {
            target: contact_target(&grid),
            cost: RESIST,
            max_iterations: 15,
            ..Default::default()
        };
        let result = optimize_ilt(&engine, &config).unwrap();
        assert_eq!(result.cost_history.len(), result.iterations + 1);
        assert_eq!(result.stage_starts, vec![0]);
        for w in result.cost_history.windows(2) {
            assert!(w[1] <= w[0], "cost increased: {} -> {}", w[0], w[1]);
        }
        assert!(
            result.final_cost < 0.3 * result.cost_history[0],
            "cost {} -> {}",
            result.cost_history[0],
            result.final_cost
        );
    }

    #[test]
    fn test_adjoint_converges_where_proxy_stalls() {
        // 2 × 2 array of 90 nm round contacts at 180 nm pitch (k₁ ≈ 0.43)
        // with a sigmoid-resist target: the target-shaped mask prints nothing
        // (peak intensity below threshold). The local proxy saturates the
        // holes at the target shape and stalls; the adjoint grows and
        // reshapes the openings and prints the array.
        let engine = test_engine();
        let grid = engine.grid().clone();
        let base = ILTConfig {
            target: create_target_contact_array(
                90.0,
                180.0,
                180.0,
                2,
                2,
                ContactShape::Round,
                &grid,
            ),
            cost: RESIST,
            max_iterations: 25,
            ..Default::default()
        };
        let adjoint = optimize_ilt(&engine, &base).unwrap();
        let proxy = optimize_ilt(
            &engine,
            &ILTConfig {
                gradient: IltGradient::LocalProxy,
                ..base.clone()
            },
        )
        .unwrap();
        // The proxy direction stops being a descent direction for the true
        // cost: its line search fails well inside the iteration budget.
        assert_eq!(proxy.termination, IltTermination::LineSearchFailed);
        assert!(proxy.iterations < base.max_iterations);
        assert!(
            adjoint.final_cost < 0.3 * proxy.final_cost,
            "adjoint {} vs proxy {}",
            adjoint.final_cost,
            proxy.final_cost
        );
        assert!(
            4 * adjoint.pattern_error < proxy.pattern_error,
            "pattern error adjoint {} vs proxy {}",
            adjoint.pattern_error,
            proxy.pattern_error
        );
    }

    #[test]
    fn test_binarization_continuation_gives_printable_binary_mask() {
        let engine = test_engine();
        let grid = engine.grid().clone();
        let target = contact_target(&grid);
        let config = ILTConfig {
            target: target.clone(),
            cost: RESIST,
            max_iterations: 30,
            binarization_weight: 0.2,
            binarization_after: 15,
            snapshot_every: 10,
            ..Default::default()
        };
        let result = optimize_ilt(&engine, &config).unwrap();
        assert_eq!(result.stage_starts.len(), 2);
        let gray = result
            .mask_transmittance
            .iter()
            .filter(|&&m| m > 0.1 && m < 0.9)
            .count();
        assert!(gray < 40, "{gray} gray pixels remain");
        // Every contact prints with the binary mask, and the printed pattern
        // differs from the target by little more than an edge-pixel ring.
        for &(cx, cy) in &[
            (-100.0, -100.0),
            (-100.0, 100.0),
            (100.0, -100.0),
            (100.0, 100.0),
        ] {
            let i = ((cy + 256.0) / 8.0) as usize;
            let j = ((cx + 256.0) / 8.0) as usize;
            assert!(
                result.binary_aerial_image[[i, j]] > 0.25,
                "contact at ({cx},{cy}) does not print"
            );
        }
        assert!(
            result.binary_pattern_error < 120,
            "{}",
            result.binary_pattern_error
        );
        // Snapshots every 10 iterations plus the final mask (the optimizer
        // may converge before the budget).
        let its: Vec<usize> = result.snapshots.iter().map(|s| s.0).collect();
        assert_eq!(&its[..3], &[0, 10, 20]);
        assert_eq!(*its.last().unwrap(), result.iterations);
    }

    #[test]
    fn test_process_window_ilt_improves_defocus_fidelity() {
        let engine = test_engine();
        let grid = engine.grid().clone();
        let nominal = ILTConfig {
            target: contact_target(&grid),
            cost: RESIST,
            max_iterations: 20,
            ..Default::default()
        };
        let pw = ILTConfig {
            conditions: vec![
                ProcessCondition::nominal(),
                ProcessCondition::new(100.0, 1.0),
            ],
            ..nominal.clone()
        };
        let r_nominal = optimize_ilt(&engine, &nominal).unwrap();
        let r_pw = optimize_ilt(&engine, &pw).unwrap();
        let at_defocus = IltProblem::new(
            &engine,
            &ILTConfig {
                conditions: vec![ProcessCondition::new(100.0, 1.0)],
                regularization: 0.0,
                ..nominal.clone()
            },
        )
        .unwrap();
        let c_nominal = at_defocus.cost(&r_nominal.theta);
        let c_pw = at_defocus.cost(&r_pw.theta);
        assert!(
            c_pw < 0.7 * c_nominal,
            "defocus cost pw {c_pw} vs nominal {c_nominal}"
        );
    }

    #[test]
    fn test_invalid_configs_rejected() {
        let engine = test_engine();
        let bad_target = ILTConfig {
            target: Array2::zeros((8, 8)),
            ..Default::default()
        };
        assert!(optimize_ilt(&engine, &bad_target).is_err());
        let grid = engine.grid().clone();
        let no_conditions = ILTConfig {
            target: contact_target(&grid),
            conditions: vec![],
            ..Default::default()
        };
        assert!(optimize_ilt(&engine, &no_conditions).is_err());
        let bad_init = ILTConfig {
            target: contact_target(&grid),
            init: IltInit::Uniform(1.5),
            ..Default::default()
        };
        assert!(optimize_ilt(&engine, &bad_init).is_err());
    }
}
