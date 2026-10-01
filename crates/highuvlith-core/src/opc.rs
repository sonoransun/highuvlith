//! Optical proximity correction (OPC), plus the imaging and edge-measurement
//! helpers shared by the optimization modules ([`crate::ilt`], [`crate::sraf`]).
//!
//! Three correction paths pre-distort the mask so the printed contour matches
//! the target:
//!
//! - **Rule-based** ([`OpcRuleTable`]): each feature's width is looked up in a
//!   bias table and the feature grows or shrinks — rectangles by moving both
//!   edges, convex polygons by a miter offset of each vertex along its
//!   edge-bisector normal, [`MaskFeature::GrayRect`] footprints like rectangles,
//!   gratings and arrays through [`MaskFeature::with_edge_bias`].
//! - **Uniform model-based** ([`model_based_opc`]): one scalar edge bias for all
//!   features, driven by the centre-row CD through the aerial engine (a damped
//!   proportional controller). Kept for backwards compatibility.
//! - **Fragment-based model OPC** ([`fragment_opc`]): every rectangle or
//!   rectilinear polygon is cut into edge fragments (corner fragments, interior
//!   fragments, line ends); each controlled fragment carries its own bias,
//!   updated from its own edge-placement error (EPE) measured on the aerial
//!   image at a print threshold, with damping, per-step and total bias clamps,
//!   optional process-window averaging over (focus, dose) conditions, and jog
//!   cleanup / bias-grid snapping of the output geometry.
//!
//! Images come from [`AerialImageEngine::compute`], whose mask spectrum
//! ([`Mask::spectrum`]) is built from exact analytic Fourier coefficients, so
//! sub-pixel edge moves change the image continuously and the correction does
//! not depend on pixel quantization. [`socs_image`] states the engine's image
//! formula for an arbitrary spectrum (it reproduces
//! [`AerialImageEngine::compute_from_transmittance`], tested); ILT uses the
//! same kernel fields for its adjoint.
//!
//! # Key equations
//!
//! ```text
//!   FFT convention      M[k] = Σ_x t(x) e^{−2πi k·x/n}  (Fft2D::forward), F⁻¹ carries 1/N
//!   SOCS image          I = (1 − φ) Σ_k λ_k |F⁻¹(K_k ⊙ M)|² + φ ⟨Σ_k λ_k |F⁻¹(K_k ⊙ M)|²⟩
//!   edge placement      EPE = (s_printed − s_target), s measured along the outward normal n̂
//!   fragment update     b_j ← clamp(b_j + Δ_j, ±b_max),  Δ_j = clamp(−g·(S·EPE̅)_j, ±Δ_max)
//!                       S = (1 − s)·1 + s·[1, 2, 1]/4 along each edge (reflective ends),
//!                       EPE̅ = Σ_c w_c EPE_c / Σ_c w_c over (focus, dose) conditions
//! ```
//!
//! # Model status
//!
//! - The "resist" is a constant threshold on the aerial image (dose scales the
//!   image); no resist blur, acid diffusion, or etch bias enters the EPE.
//! - No sub-resolution assist features here — see [`crate::sraf`].
//! - Rule-based polygon offset is exact for convex polygons only; concave
//!   vertices get the same miter rule with no self-intersection cleanup.
//! - Fragment OPC handles rectangles and rectilinear (Manhattan) polygons;
//!   other features (gratings, arrays, gray rectangles, general polygons)
//!   pass through uncorrected. Corner fragments are slaved to their interior
//!   neighbour by default (corner rounding is a low-pass effect an edge bias
//!   cannot remove). There is no general mask-rule check: the bias clamp must
//!   stay below half the smallest width/space, otherwise corrected polygons
//!   can self-intersect.
//! - EPE is read from a Keys-cubic interpolation of the pixel image, accurate
//!   when the pixel is ≲ λ / (4·NA·(1 + σ)).

use ndarray::Array2;
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::aerial::{AerialImageEngine, KernelSet};
use crate::error::{LithographyError, Result};
use crate::mask::{Mask, MaskFeature};
use crate::math::fft2d::Fft2D;
use crate::metrics;
use crate::types::{Complex64, GridConfig};

// ---------------------------------------------------------------------------
// Rule-based OPC
// ---------------------------------------------------------------------------

/// OPC rule: bias to apply to features of a given width range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpcRule {
    /// Minimum feature width (nm) for this rule.
    pub min_width_nm: f64,
    /// Maximum feature width (nm) for this rule.
    pub max_width_nm: f64,
    /// Bias to add to each edge (nm). Positive = grow feature.
    pub bias_nm: f64,
}

/// Table of OPC rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpcRuleTable {
    pub rules: Vec<OpcRule>,
}

impl OpcRuleTable {
    /// Apply rule-based OPC to a mask.
    /// Returns a new mask with biased features.
    pub fn apply(&self, mask: &Mask) -> Mask {
        let new_features: Vec<MaskFeature> = mask
            .features
            .iter()
            .map(|feature| self.bias_feature(feature))
            .collect();

        Mask {
            mask_type: mask.mask_type.clone(),
            features: new_features,
            dark_field: mask.dark_field,
        }
    }

    fn bias_feature(&self, feature: &MaskFeature) -> MaskFeature {
        match feature {
            MaskFeature::Rect { x, y, w, h } => {
                let bias = self.find_bias(*w);
                MaskFeature::Rect {
                    x: *x,
                    y: *y,
                    w: w + 2.0 * bias, // bias applied to both edges
                    h: *h,
                }
            }
            MaskFeature::Polygon { vertices } => {
                // Determine characteristic width from bounding box
                let (min_x, max_x) = vertices
                    .iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), &(x, _)| {
                        (mn.min(x), mx.max(x))
                    });
                let bb_width = max_x - min_x;
                let bias = self.find_bias(bb_width);

                if bias.abs() < 1e-15 || vertices.len() < 3 {
                    return MaskFeature::Polygon {
                        vertices: vertices.clone(),
                    };
                }

                // Offset each vertex along the average outward normal of its
                // two adjacent edges. This is exact for convex polygons.
                let n = vertices.len();
                let mut new_verts = Vec::with_capacity(n);
                for i in 0..n {
                    let prev = vertices[(i + n - 1) % n];
                    let curr = vertices[i];
                    let next = vertices[(i + 1) % n];

                    // Edge vectors
                    let (dx1, dy1) = (curr.0 - prev.0, curr.1 - prev.1);
                    let (dx2, dy2) = (next.0 - curr.0, next.1 - curr.1);

                    // Outward normals (assuming CCW winding => outward is to the right)
                    let len1 = (dx1 * dx1 + dy1 * dy1).sqrt();
                    let len2 = (dx2 * dx2 + dy2 * dy2).sqrt();

                    if len1 < 1e-15 || len2 < 1e-15 {
                        new_verts.push(curr);
                        continue;
                    }

                    let (nx1, ny1) = (dy1 / len1, -dx1 / len1);
                    let (nx2, ny2) = (dy2 / len2, -dx2 / len2);

                    // Average outward normal at vertex
                    let avg_nx = nx1 + nx2;
                    let avg_ny = ny1 + ny2;
                    let avg_len = (avg_nx * avg_nx + avg_ny * avg_ny).sqrt();

                    if avg_len < 1e-15 {
                        new_verts.push(curr);
                        continue;
                    }

                    // Miter offset: to move each edge outward by `bias`, the
                    // vertex moves by bias/cos(half_angle) along the bisector.
                    // With avg = n1+n2, |avg| = 2*cos(half_angle), so the
                    // offset vector is avg * 2*bias / |avg|^2.
                    let avg_len_sq = avg_nx * avg_nx + avg_ny * avg_ny;
                    let factor = 2.0 * bias / avg_len_sq;
                    new_verts.push((curr.0 + avg_nx * factor, curr.1 + avg_ny * factor));
                }

                MaskFeature::Polygon {
                    vertices: new_verts,
                }
            }
            MaskFeature::GrayRect {
                x,
                y,
                w,
                h,
                transmittance,
            } => {
                // Grayscale features encode dose, not edge position; OPC
                // edge biasing still applies to their footprint.
                let bias = self.find_bias(*w);
                MaskFeature::GrayRect {
                    x: *x,
                    y: *y,
                    w: w + 2.0 * bias,
                    h: *h,
                    transmittance: *transmittance,
                }
            }
            // Periodic primitives (WP-B): bias the element width like a rectangle's.
            periodic @ (MaskFeature::LineSpace { .. } | MaskFeature::RectArray { .. }) => {
                periodic.with_edge_bias(self.find_bias(periodic.element_width_nm().unwrap_or(0.0)))
            }
            #[allow(unreachable_patterns)]
            other => other.clone(),
        }
    }

    fn find_bias(&self, width_nm: f64) -> f64 {
        for rule in &self.rules {
            if width_nm >= rule.min_width_nm && width_nm <= rule.max_width_nm {
                return rule.bias_nm;
            }
        }
        0.0 // no matching rule
    }
}

// ---------------------------------------------------------------------------
// Uniform-bias model-based OPC (legacy API)
// ---------------------------------------------------------------------------

/// Model-based OPC: iteratively adjust mask edges to match target CD.
///
/// Uses the aerial image engine to simulate the mask at each iteration
/// and adjusts feature edges based on the error between measured and target CD.
/// One uniform bias is applied to every rectangle (a scalar proportional
/// controller on the centre-row CD); see [`fragment_opc`] for per-fragment
/// correction.
pub fn model_based_opc(
    mask: &Mask,
    engine: &AerialImageEngine,
    target_cd_nm: f64,
    cd_threshold: f64,
    max_iterations: usize,
    convergence_tol_nm: f64,
) -> crate::error::Result<(Mask, OpcConvergence)> {
    let mut current_mask = mask.clone();
    let mut convergence = OpcConvergence {
        iterations: Vec::new(),
    };

    let grid = engine.grid();
    let field = grid.field_size_nm();
    let half = field / 2.0;

    for iter in 0..max_iterations {
        // Simulate current mask
        let aerial = engine.compute(&current_mask, 0.0);
        let measured_cd =
            metrics::measure_cd_2d(&aerial.data, -half, half, cd_threshold).unwrap_or(0.0);

        let error = measured_cd - target_cd_nm;

        convergence.iterations.push(OpcIteration {
            iteration: iter,
            cd_nm: measured_cd,
            error_nm: error,
        });

        // Check convergence
        if error.abs() < convergence_tol_nm {
            return Ok((current_mask, convergence));
        }

        // Adjust mask features: simple proportional correction
        let correction = -error * 0.5; // damped correction factor
        current_mask = bias_mask_features(&current_mask, correction);
    }

    let last_error = convergence
        .iterations
        .last()
        .map(|it| it.error_nm.abs())
        .unwrap_or(f64::INFINITY);

    if last_error > convergence_tol_nm {
        Err(crate::error::LithographyError::ConvergenceFailure {
            iterations: max_iterations,
            residual: last_error,
        })
    } else {
        Ok((current_mask, convergence))
    }
}

/// Apply a uniform edge bias to every feature of a mask
/// ([`MaskFeature::with_edge_bias`]: rectangles and gray rectangles grow in
/// width, grating lines in CD, array elements in both sides; polygons are
/// unchanged).
fn bias_mask_features(mask: &Mask, bias_nm: f64) -> Mask {
    Mask {
        mask_type: mask.mask_type.clone(),
        features: mask
            .features
            .iter()
            .map(|feature| feature.with_edge_bias(bias_nm))
            .collect(),
        dark_field: mask.dark_field,
    }
}

/// OPC convergence history.
#[derive(Debug, Clone)]
pub struct OpcConvergence {
    pub iterations: Vec<OpcIteration>,
}

/// Single OPC iteration result.
#[derive(Debug, Clone)]
pub struct OpcIteration {
    pub iteration: usize,
    pub cd_nm: f64,
    pub error_nm: f64,
}

// ---------------------------------------------------------------------------
// Shared exact-imaging helpers (OPC, SRAF, ILT)
// ---------------------------------------------------------------------------

/// One process condition — focus, relative dose, and a weight — used by
/// process-window-aware ILT and OPC and by the SRAF print check.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProcessCondition {
    /// Defocus (nm).
    pub defocus_nm: f64,
    /// Relative dose (1 = nominal); the constant-threshold resist sees `dose × I`.
    pub dose: f64,
    /// Weight of this condition in a cost or EPE average.
    pub weight: f64,
}

impl ProcessCondition {
    /// A condition at the given defocus and relative dose, with weight 1.
    pub fn new(defocus_nm: f64, dose: f64) -> Self {
        Self {
            defocus_nm,
            dose,
            weight: 1.0,
        }
    }

    /// Nominal condition: best focus, nominal dose, weight 1.
    pub fn nominal() -> Self {
        Self::new(0.0, 1.0)
    }

    /// The same condition with a different weight.
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

impl Default for ProcessCondition {
    fn default() -> Self {
        Self::nominal()
    }
}

/// Validate a list of process conditions (non-empty, finite, dose > 0,
/// weight ≥ 0 with a positive sum).
pub(crate) fn validate_conditions(conditions: &[ProcessCondition]) -> Result<()> {
    if conditions.is_empty() {
        return Err(LithographyError::InvalidParameter {
            name: "conditions",
            value: 0.0,
            reason: "at least one process condition is required",
        });
    }
    let mut weight_sum = 0.0;
    for c in conditions {
        if !c.defocus_nm.is_finite() {
            return Err(LithographyError::InvalidParameter {
                name: "defocus_nm",
                value: c.defocus_nm,
                reason: "must be finite",
            });
        }
        if !(c.dose.is_finite() && c.dose > 0.0) {
            return Err(LithographyError::InvalidParameter {
                name: "dose",
                value: c.dose,
                reason: "must be positive and finite",
            });
        }
        if !(c.weight.is_finite() && c.weight >= 0.0) {
            return Err(LithographyError::InvalidParameter {
                name: "weight",
                value: c.weight,
                reason: "must be non-negative and finite",
            });
        }
        weight_sum += c.weight;
    }
    if weight_sum <= 0.0 {
        return Err(LithographyError::InvalidParameter {
            name: "weight",
            value: weight_sum,
            reason: "condition weights must not all be zero",
        });
    }
    Ok(())
}

/// Signed frequency (cycles/nm) of FFT index `index` on an `n`-point axis
/// with frequency step `df` — the same convention as the aerial engine
/// (indices ≥ n/2 are negative frequencies).
pub(crate) fn fft_freq(index: usize, n: usize, df: f64) -> f64 {
    if index < n / 2 {
        index as f64 * df
    } else {
        (index as f64 - n as f64) * df
    }
}

/// Signed (shoelace) area of a polygon; positive for counter-clockwise.
pub(crate) fn signed_area(vertices: &[(f64, f64)]) -> f64 {
    let n = vertices.len();
    let mut a = 0.0;
    for i in 0..n {
        let (x0, y0) = vertices[i];
        let (x1, y1) = vertices[(i + 1) % n];
        a += x0 * y1 - x1 * y0;
    }
    0.5 * a
}

/// Coherent fields `A_k = F⁻¹(K_k ⊙ M)` for every kernel of `kset`.
pub(crate) fn kernel_fields(
    kset: &KernelSet,
    spectrum: &Array2<Complex64>,
    fft: &Fft2D,
) -> Vec<Array2<Complex64>> {
    let field = |kernel: &Array2<Complex64>| {
        let mut product = kernel * spectrum;
        fft.inverse(&mut product);
        product
    };
    #[cfg(feature = "parallel")]
    {
        kset.kernels.par_iter().map(field).collect()
    }
    #[cfg(not(feature = "parallel"))]
    {
        kset.kernels.iter().map(field).collect()
    }
}

/// SOCS intensity before flare, `Σ_k λ_k |A_k|²`, from precomputed fields.
pub(crate) fn intensity_from_fields(kset: &KernelSet, fields: &[Array2<Complex64>]) -> Array2<f64> {
    let dim = fields.first().map(|f| f.dim()).unwrap_or((0, 0));
    let mut total = Array2::zeros(dim);
    for (lambda, a) in kset.eigenvalues.iter().zip(fields) {
        total.zip_mut_with(a, |t, v| *t += lambda * v.norm_sqr());
    }
    total
}

/// Uniform flare as applied by the aerial engine:
/// `I = (1 − φ) I₀ + φ ⟨I₀⟩`.
pub fn apply_flare(image: &mut Array2<f64>, flare_fraction: f64) {
    if flare_fraction > 0.0 && !image.is_empty() {
        let mean = image.iter().sum::<f64>() / image.len() as f64;
        let flare = flare_fraction * mean;
        image.mapv_inplace(|v| v * (1.0 - flare_fraction) + flare);
    }
}

/// Aerial image from a mask spectrum through a kernel set, including the
/// engine's uniform flare: `I = (1 − φ) Σ_k λ_k |F⁻¹(K_k ⊙ M)|² + φ⟨·⟩`.
///
/// With `spectrum = FFT(t)` this equals
/// `engine.compute_from_transmittance(t, kset.defocus_nm)` (contract C1).
pub fn socs_image(
    kset: &KernelSet,
    spectrum: &Array2<Complex64>,
    flare_fraction: f64,
    fft: &Fft2D,
) -> Array2<f64> {
    let fields = kernel_fields(kset, spectrum, fft);
    let mut image = intensity_from_fields(kset, &fields);
    apply_flare(&mut image, flare_fraction);
    image
}

/// Keys cubic-convolution weights (a = −0.5) for offsets −1, 0, 1, 2 at
/// fractional position `t ∈ [0, 1)`.
fn keys_weights(t: f64) -> [f64; 4] {
    let a = -0.5;
    let far = |x: f64| a * x * x * x - 5.0 * a * x * x + 8.0 * a * x - 4.0 * a;
    let near = |x: f64| (a + 2.0) * x * x * x - (a + 3.0) * x * x + 1.0;
    [far(1.0 + t), near(t), near(1.0 - t), far(2.0 - t)]
}

/// Sample a periodic image at a physical point `(x_nm, y_nm)` with Keys
/// cubic convolution. Pixel `(i, j)` sits at
/// `(−L/2 + (j + ½)p, −L/2 + (i + ½)p)` (the engine's convention).
pub fn sample_image(image: &Array2<f64>, grid: &GridConfig, x_nm: f64, y_nm: f64) -> f64 {
    let (ny, nx) = image.dim();
    let half = grid.field_size_nm() / 2.0;
    let u = (x_nm + half) / grid.pixel_nm - 0.5;
    let v = (y_nm + half) / grid.pixel_nm - 0.5;
    let (u0, v0) = (u.floor(), v.floor());
    let wu = keys_weights(u - u0);
    let wv = keys_weights(v - v0);
    let mut acc = 0.0;
    for (a, wy) in wv.iter().enumerate() {
        let i = (v0 as i64 - 1 + a as i64).rem_euclid(ny as i64) as usize;
        let mut row = 0.0;
        for (b, wx) in wu.iter().enumerate() {
            let j = (u0 as i64 - 1 + b as i64).rem_euclid(nx as i64) as usize;
            row += wx * image[[i, j]];
        }
        acc += wy * row;
    }
    acc
}

/// Edge-placement error at a target edge point.
///
/// Walks the line `point + s·normal`, `s ∈ [−range, range]`, and returns the
/// signed position `s` of the printed edge nearest `s = 0` — the crossing
/// from "printed" to "not printed" as `s` increases — refined by bisection on
/// the interpolated image. "Printed" means `I > threshold` for a clear
/// feature and `I < threshold` for an opaque one. Returns `(epe, found)`;
/// when no crossing lies in range the EPE saturates at `±range` (sign from
/// whether the inner end prints) and `found` is `false`.
pub fn measure_epe(
    image: &Array2<f64>,
    grid: &GridConfig,
    point: (f64, f64),
    normal: (f64, f64),
    threshold: f64,
    clear_feature: bool,
    range_nm: f64,
) -> (f64, bool) {
    let sign = if clear_feature { 1.0 } else { -1.0 };
    let g = |s: f64| {
        let v = sample_image(image, grid, point.0 + s * normal.0, point.1 + s * normal.1);
        sign * (v - threshold)
    };
    let step = (grid.pixel_nm / 4.0).min(1.0);
    let steps = ((2.0 * range_nm) / step).ceil().max(2.0) as usize;
    let ds = 2.0 * range_nm / steps as f64;
    let mut best: Option<f64> = None;
    let mut s_prev = -range_nm;
    let mut g_prev = g(s_prev);
    let g_start = g_prev;
    for k in 1..=steps {
        let s = -range_nm + k as f64 * ds;
        let gs = g(s);
        if g_prev > 0.0 && gs <= 0.0 {
            // Bisection on [s_prev, s].
            let (mut lo, mut hi) = (s_prev, s);
            for _ in 0..30 {
                let mid = 0.5 * (lo + hi);
                if g(mid) > 0.0 {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            let root = 0.5 * (lo + hi);
            if best.is_none_or(|b| root.abs() < b.abs()) {
                best = Some(root);
            }
        }
        s_prev = s;
        g_prev = gs;
    }
    match best {
        Some(s) => (s, true),
        None if g_start > 0.0 => (range_nm, false),
        None => (-range_nm, false),
    }
}

// ---------------------------------------------------------------------------
// Fragment-based model OPC
// ---------------------------------------------------------------------------

/// Role of an edge fragment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FragmentKind {
    /// Interior edge fragment (controlled).
    Edge,
    /// Fragment touching a polygon corner. Slaved to its interior neighbour
    /// unless [`FragmentOpcConfig::correct_corners`] is set.
    Corner,
    /// Interior fragment of a line end: an edge between two convex corners
    /// that is shorter than both adjacent edges (controlled).
    LineEnd,
}

/// One edge fragment of a corrected polygon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fragment {
    /// Index of the corrected polygon (into [`FragmentOpcResult::polygons`]).
    pub polygon: usize,
    /// Edge index within the polygon's normalized (CCW) vertex list.
    pub edge: usize,
    /// Fragment start on the target edge (nm).
    pub start: (f64, f64),
    /// Fragment end on the target edge (nm).
    pub end: (f64, f64),
    /// Outward unit normal of the target edge.
    pub normal: (f64, f64),
    /// Fragment role.
    pub kind: FragmentKind,
    /// Current outward edge bias (nm).
    pub bias_nm: f64,
    /// Last measured condition-averaged EPE at the fragment midpoint (nm);
    /// `NaN` for slaved corner fragments.
    pub epe_nm: f64,
    /// Whether a printed edge was found within the search range at every
    /// condition in the last measurement.
    pub found: bool,
}

impl Fragment {
    /// Fragment midpoint on the target edge — the EPE control point.
    pub fn control_point(&self) -> (f64, f64) {
        (
            0.5 * (self.start.0 + self.end.0),
            0.5 * (self.start.1 + self.end.1),
        )
    }

    /// Fragment length (nm).
    pub fn length(&self) -> f64 {
        ((self.end.0 - self.start.0).powi(2) + (self.end.1 - self.start.1).powi(2)).sqrt()
    }

    fn controlled(&self, correct_corners: bool) -> bool {
        correct_corners || self.kind != FragmentKind::Corner
    }
}

/// Fragment-based model OPC settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FragmentOpcConfig {
    /// Print threshold on the aerial image (constant-threshold resist).
    pub threshold: f64,
    /// Maximum length of an interior fragment (nm).
    pub max_fragment_nm: f64,
    /// Length of the fragments adjacent to each corner (nm). Edges shorter
    /// than `2·corner + corner/2` become a single controlled fragment.
    pub corner_fragment_nm: f64,
    /// Feedback gain `g` in `Δb = −g·(S·EPE)` (damping; the uniform mode is
    /// stable for `g·MEEF < 2`).
    pub feedback_gain: f64,
    /// Along-edge feedback smoothing `s ∈ [0, 1]`:
    /// `S = (1 − s)·1 + s·H`, with `H` the `[1, 2, 1]/4` filter over the
    /// neighbouring controlled fragments of one edge (reflective ends).
    /// Bias patterns that alternate faster than the optics resolve are
    /// invisible in the image (a near-null space of the EPE response); an
    /// unfiltered loop integrates any EPE component in that space up to the
    /// bias clamp, whereas `H` removes the alternating component from the
    /// feedback and passes the smooth (resolvable) components. 0 disables.
    pub smoothing: f64,
    /// Maximum bias change per iteration (nm).
    pub max_step_nm: f64,
    /// Maximum |total bias| of any fragment (nm). Keep below half the
    /// smallest feature width/space.
    pub max_bias_nm: f64,
    /// EPE search half-range along the edge normal (nm).
    pub search_range_nm: f64,
    /// Maximum number of correction iterations.
    pub max_iterations: usize,
    /// Convergence tolerance on the rms EPE of controlled fragments (nm).
    pub tolerance_nm: f64,
    /// Convergence tolerance on the max |EPE| of controlled fragments (nm);
    /// both this and `tolerance_nm` must be met.
    pub max_epe_tolerance_nm: f64,
    /// Jog cleanup of the output: neighbouring fragments on an edge whose
    /// biases differ by less than this are merged to their length-weighted
    /// mean (nm; 0 disables).
    pub min_jog_nm: f64,
    /// Snap output biases to this grid (nm; 0 disables), e.g. a mask-writer
    /// address unit.
    pub bias_grid_nm: f64,
    /// Control corner fragments independently instead of slaving them.
    pub correct_corners: bool,
    /// (focus, dose, weight) conditions; the controller drives the weighted
    /// mean EPE to zero. Default: nominal only.
    pub conditions: Vec<ProcessCondition>,
}

impl Default for FragmentOpcConfig {
    /// Absolute defaults sized for a resolution unit `λ/NA ≈ 200 nm`
    /// (VUV/ArF); use [`FragmentOpcConfig::for_engine`] for other systems.
    fn default() -> Self {
        Self {
            threshold: 0.3,
            max_fragment_nm: 60.0,
            corner_fragment_nm: 50.0,
            feedback_gain: 0.6,
            smoothing: 1.0,
            max_step_nm: 6.0,
            max_bias_nm: 50.0,
            search_range_nm: 75.0,
            max_iterations: 40,
            tolerance_nm: 1.0,
            max_epe_tolerance_nm: 2.0,
            min_jog_nm: 1.0,
            bias_grid_nm: 0.0,
            correct_corners: false,
            conditions: vec![ProcessCondition::nominal()],
        }
    }
}

impl FragmentOpcConfig {
    /// Settings scaled to the engine's resolution unit `R = λ/NA`:
    /// interior fragments `0.3 R`, corner (slaved, unmeasured) fragments
    /// `0.25 R`, EPE search range `0.35 R`, bias clamp `0.25 R`, step clamp
    /// `0.03 R`, rms tolerance and minimum jog `0.005 R`, max-|EPE|
    /// tolerance `0.01 R` — e.g. 63 / 53 / 74 / 53 / 6.3 / 1.05 / 2.1 nm at
    /// 157.63 nm, NA 0.75. Starting points, not calibrated recipes.
    pub fn for_engine(engine: &AerialImageEngine, threshold: f64) -> Self {
        let r = engine.wavelength_nm() / engine.na();
        Self {
            threshold,
            max_fragment_nm: 0.3 * r,
            corner_fragment_nm: 0.25 * r,
            search_range_nm: 0.35 * r,
            max_bias_nm: 0.25 * r,
            max_step_nm: 0.03 * r,
            tolerance_nm: 0.005 * r,
            max_epe_tolerance_nm: 0.01 * r,
            min_jog_nm: 0.005 * r,
            ..Self::default()
        }
    }
}

/// EPE statistics of one fragment-OPC iteration (measured before that
/// iteration's bias update).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FragmentOpcIteration {
    /// Iteration index (0 = uncorrected mask).
    pub iteration: usize,
    /// RMS EPE over controlled fragments (nm).
    pub epe_rms_nm: f64,
    /// Max |EPE| over controlled fragments (nm).
    pub epe_max_nm: f64,
    /// Controlled fragments whose printed edge was not found in range.
    pub not_found: usize,
}

/// Result of [`fragment_opc`].
#[derive(Debug, Clone)]
pub struct FragmentOpcResult {
    /// Corrected mask: corrected polygons (as `Polygon` features) followed by
    /// the pass-through features, same mask type and field tone.
    pub mask: Mask,
    /// Corrected polygons (CCW vertex lists, after jog cleanup and snapping).
    pub polygons: Vec<Vec<(f64, f64)>>,
    /// Fragments with final biases and the EPE re-measured on the output
    /// geometry.
    pub fragments: Vec<Fragment>,
    /// Per-iteration EPE statistics during correction.
    pub history: Vec<FragmentOpcIteration>,
    /// EPE statistics re-measured on the output geometry (after cleanup).
    pub final_stats: FragmentOpcIteration,
    /// Whether the rms and max EPE fell below their tolerances during
    /// correction.
    pub converged: bool,
    /// Aerial image of the output mask at the first condition's focus.
    pub final_image: Array2<f64>,
}

impl FragmentOpcResult {
    /// Final rms EPE of controlled fragments (nm).
    pub fn epe_rms_nm(&self) -> f64 {
        self.final_stats.epe_rms_nm
    }
}

/// Internal: one normalized rectilinear polygon and its fragmentation.
struct OpcPolygon {
    /// CCW vertices, rectilinear, no duplicate or collinear points.
    vertices: Vec<(f64, f64)>,
    /// Per edge: outward normal, breakpoints (q + 1 points from V_i to
    /// V_{i+1}), and the global indices of its q fragments.
    edges: Vec<OpcEdge>,
}

struct OpcEdge {
    normal: (f64, f64),
    breakpoints: Vec<(f64, f64)>,
    fragments: Vec<usize>,
}

/// Remove duplicate and collinear vertices; `None` if fewer than 3 remain.
fn simplify_polygon(vertices: &[(f64, f64)]) -> Option<Vec<(f64, f64)>> {
    let mut v: Vec<(f64, f64)> = vertices.to_vec();
    loop {
        let n = v.len();
        if n < 3 {
            return None;
        }
        let mut changed = false;
        let mut out: Vec<(f64, f64)> = Vec::with_capacity(n);
        for i in 0..n {
            let prev = if out.is_empty() {
                v[(i + n - 1) % n]
            } else {
                out[out.len() - 1]
            };
            let cur = v[i];
            let next = v[(i + 1) % n];
            let (ax, ay) = (cur.0 - prev.0, cur.1 - prev.1);
            let (bx, by) = (next.0 - cur.0, next.1 - cur.1);
            let dup = ax.abs() < 1e-9 && ay.abs() < 1e-9;
            let collinear = (ax * by - ay * bx).abs() < 1e-9 * (1.0 + ax.abs() + ay.abs());
            if dup || collinear {
                changed = true;
                continue;
            }
            out.push(cur);
        }
        v = out;
        if !changed {
            return if v.len() >= 3 { Some(v) } else { None };
        }
    }
}

/// Normalize a rectilinear polygon to CCW with no redundant vertices;
/// `None` if it is not rectilinear or degenerate.
fn normalize_rectilinear(vertices: &[(f64, f64)]) -> Option<Vec<(f64, f64)>> {
    let mut v = simplify_polygon(vertices)?;
    if v.len() < 4 {
        return None;
    }
    let n = v.len();
    for i in 0..n {
        let a = v[i];
        let b = v[(i + 1) % n];
        if (a.0 - b.0).abs() > 1e-6 && (a.1 - b.1).abs() > 1e-6 {
            return None;
        }
    }
    let area = signed_area(&v);
    if area.abs() < 1e-9 {
        return None;
    }
    if area < 0.0 {
        v.reverse();
    }
    Some(v)
}

/// Fragment one normalized polygon; appends to `fragments`.
fn fragment_polygon(
    vertices: Vec<(f64, f64)>,
    polygon_index: usize,
    config: &FragmentOpcConfig,
    fragments: &mut Vec<Fragment>,
) -> OpcPolygon {
    let m = vertices.len();
    let corner = config.corner_fragment_nm.max(0.0);
    let max_frag = config.max_fragment_nm.max(1e-3);
    let edge_len = |k: usize| {
        let a = vertices[k % m];
        let b = vertices[(k + 1) % m];
        ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
    };
    // Convex vertex (CCW): left turn.
    let convex = |k: usize| {
        let prev = vertices[(k + m - 1) % m];
        let cur = vertices[k % m];
        let next = vertices[(k + 1) % m];
        (cur.0 - prev.0) * (next.1 - cur.1) - (cur.1 - prev.1) * (next.0 - cur.0) > 0.0
    };

    let mut edges = Vec::with_capacity(m);
    for k in 0..m {
        let a = vertices[k];
        let b = vertices[(k + 1) % m];
        let len = edge_len(k);
        let (ux, uy) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
        let normal = (uy, -ux);
        let at = |s: f64| (a.0 + s * ux, a.1 + s * uy);

        let line_end =
            convex(k) && convex(k + 1) && len < edge_len(k + m - 1) && len < edge_len(k + 1);
        let interior_kind = if line_end {
            FragmentKind::LineEnd
        } else {
            FragmentKind::Edge
        };

        let mut cuts: Vec<(f64, FragmentKind)> = Vec::new(); // (end position, kind)
        if len <= 2.0 * corner + 0.5 * corner || corner <= 0.0 {
            let parts = if corner <= 0.0 {
                (len / max_frag).ceil().max(1.0) as usize
            } else {
                1
            };
            for q in 1..=parts {
                cuts.push((len * q as f64 / parts as f64, interior_kind));
            }
        } else {
            cuts.push((corner, FragmentKind::Corner));
            let interior = len - 2.0 * corner;
            let parts = (interior / max_frag).ceil().max(1.0) as usize;
            for q in 1..=parts {
                cuts.push((corner + interior * q as f64 / parts as f64, interior_kind));
            }
            cuts.push((len, FragmentKind::Corner));
        }

        let mut breakpoints = vec![a];
        let mut ids = Vec::with_capacity(cuts.len());
        let mut s_prev = 0.0;
        for &(s, kind) in &cuts {
            let start = at(s_prev);
            let end = if (s - len).abs() < 1e-12 { b } else { at(s) };
            ids.push(fragments.len());
            fragments.push(Fragment {
                polygon: polygon_index,
                edge: k,
                start,
                end,
                normal,
                kind,
                bias_nm: 0.0,
                epe_nm: f64::NAN,
                found: true,
            });
            breakpoints.push(end);
            s_prev = s;
        }
        edges.push(OpcEdge {
            normal,
            breakpoints,
            fragments: ids,
        });
    }
    OpcPolygon { vertices, edges }
}

/// Build the corrected polygon from fragment biases: offset each fragment
/// along its normal, join fragments of one edge with jogs, and intersect the
/// offset lines of perpendicular edges at corners.
fn build_corrected_polygon(poly: &OpcPolygon, fragments: &[Fragment]) -> Vec<(f64, f64)> {
    let m = poly.vertices.len();
    let mut out = Vec::new();
    for e in 0..m {
        let prev = (e + m - 1) % m;
        let edge = &poly.edges[e];
        let prev_edge = &poly.edges[prev];
        let b_prev = fragments[*prev_edge.fragments.last().expect("edge has fragments")].bias_nm;
        let b_first = fragments[edge.fragments[0]].bias_nm;
        let v = poly.vertices[e];
        out.push((
            v.0 + b_prev * prev_edge.normal.0 + b_first * edge.normal.0,
            v.1 + b_prev * prev_edge.normal.1 + b_first * edge.normal.1,
        ));
        let q = edge.fragments.len();
        for j in 1..q {
            let bp = edge.breakpoints[j];
            let b_left = fragments[edge.fragments[j - 1]].bias_nm;
            let b_right = fragments[edge.fragments[j]].bias_nm;
            out.push((bp.0 + b_left * edge.normal.0, bp.1 + b_left * edge.normal.1));
            out.push((
                bp.0 + b_right * edge.normal.0,
                bp.1 + b_right * edge.normal.1,
            ));
        }
    }
    simplify_polygon(&out).unwrap_or(out)
}

/// Set every corner fragment's bias to that of its nearest interior
/// neighbour on the same edge.
fn slave_corner_fragments(polys: &[OpcPolygon], fragments: &mut [Fragment]) {
    for poly in polys {
        for edge in &poly.edges {
            let ids = &edge.fragments;
            if ids.len() < 3 {
                continue;
            }
            let first_interior = fragments[ids[1]].bias_nm;
            let last_interior = fragments[ids[ids.len() - 2]].bias_nm;
            if fragments[ids[0]].kind == FragmentKind::Corner {
                fragments[ids[0]].bias_nm = first_interior;
            }
            let last = ids[ids.len() - 1];
            if fragments[last].kind == FragmentKind::Corner {
                fragments[last].bias_nm = last_interior;
            }
        }
    }
}

/// Jog cleanup: merge runs of neighbouring fragments on an edge whose biases
/// differ by less than `min_jog` into their length-weighted mean.
fn jog_cleanup(polys: &[OpcPolygon], fragments: &mut [Fragment], min_jog: f64) {
    if min_jog <= 0.0 {
        return;
    }
    for poly in polys {
        for edge in &poly.edges {
            let ids = &edge.fragments;
            let mut start = 0;
            while start < ids.len() {
                let mut end = start + 1;
                let mut w_sum = fragments[ids[start]].length();
                let mut b_sum = fragments[ids[start]].bias_nm * w_sum;
                while end < ids.len() {
                    let mean = b_sum / w_sum;
                    let f = &fragments[ids[end]];
                    if (f.bias_nm - mean).abs() < min_jog {
                        w_sum += f.length();
                        b_sum += f.bias_nm * f.length();
                        end += 1;
                    } else {
                        break;
                    }
                }
                let mean = b_sum / w_sum;
                for &id in &ids[start..end] {
                    fragments[id].bias_nm = mean;
                }
                start = end;
            }
        }
    }
}

/// Assemble a mask from corrected polygons plus pass-through features.
fn assemble_mask(
    template: &Mask,
    polygons: &[Vec<(f64, f64)>],
    passthrough: &[MaskFeature],
) -> Mask {
    let mut features: Vec<MaskFeature> = polygons
        .iter()
        .map(|v| MaskFeature::Polygon {
            vertices: v.clone(),
        })
        .collect();
    features.extend(passthrough.iter().cloned());
    Mask {
        mask_type: template.mask_type.clone(),
        features,
        dark_field: template.dark_field,
    }
}

/// Measure the condition-averaged EPE of every controlled fragment; returns
/// per-iteration statistics and updates `epe_nm` / `found`.
#[allow(clippy::too_many_arguments)]
fn measure_all_epe(
    fragments: &mut [Fragment],
    images: &[Array2<f64>],
    conditions: &[ProcessCondition],
    grid: &GridConfig,
    config: &FragmentOpcConfig,
    clear_feature: bool,
    iteration: usize,
) -> FragmentOpcIteration {
    let w_sum: f64 = conditions.iter().map(|c| c.weight).sum();
    let mut sq = 0.0;
    let mut max_abs: f64 = 0.0;
    let mut count = 0usize;
    let mut not_found = 0usize;
    for frag in fragments.iter_mut() {
        if !frag.controlled(config.correct_corners) {
            frag.epe_nm = f64::NAN;
            continue;
        }
        let point = frag.control_point();
        let mut epe = 0.0;
        let mut found = true;
        for (image, cond) in images.iter().zip(conditions) {
            let (e, ok) = measure_epe(
                image,
                grid,
                point,
                frag.normal,
                config.threshold / cond.dose,
                clear_feature,
                config.search_range_nm,
            );
            epe += cond.weight * e;
            found &= ok;
        }
        epe /= w_sum;
        frag.epe_nm = epe;
        frag.found = found;
        if !found {
            not_found += 1;
        }
        sq += epe * epe;
        max_abs = max_abs.max(epe.abs());
        count += 1;
    }
    FragmentOpcIteration {
        iteration,
        epe_rms_nm: if count > 0 {
            (sq / count as f64).sqrt()
        } else {
            0.0
        },
        epe_max_nm: max_abs,
        not_found,
    }
}

/// Damped, filtered feedback step of every fragment (0 for slaved ones):
/// `Δb_j = clamp(−g·(S·EPE)_j, ±Δ_max)` with `S = (1 − s)·1 + s·H` and `H`
/// the reflective `[1, 2, 1]/4` filter over the controlled fragments of one
/// edge.
fn controller_steps(
    polys: &[OpcPolygon],
    fragments: &[Fragment],
    config: &FragmentOpcConfig,
) -> Vec<f64> {
    let mut steps = vec![0.0; fragments.len()];
    let s = config.smoothing;
    for poly in polys {
        for edge in &poly.edges {
            let controlled: Vec<usize> = edge
                .fragments
                .iter()
                .copied()
                .filter(|&id| fragments[id].controlled(config.correct_corners))
                .collect();
            let q = controlled.len();
            for (pos, &id) in controlled.iter().enumerate() {
                let e = |k: usize| fragments[controlled[k]].epe_nm;
                let left = e(pos.saturating_sub(1));
                let right = e((pos + 1).min(q - 1));
                let filtered = (left + 2.0 * e(pos) + right) / 4.0;
                let feedback = (1.0 - s) * e(pos) + s * filtered;
                steps[id] = (-config.feedback_gain * feedback)
                    .clamp(-config.max_step_nm, config.max_step_nm);
            }
        }
    }
    steps
}

fn validate_fragment_config(config: &FragmentOpcConfig) -> Result<()> {
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
    positive("threshold", config.threshold)?;
    positive("max_fragment_nm", config.max_fragment_nm)?;
    positive("feedback_gain", config.feedback_gain)?;
    if !(0.0..=1.0).contains(&config.smoothing) {
        return Err(LithographyError::InvalidParameter {
            name: "smoothing",
            value: config.smoothing,
            reason: "must be in [0, 1]",
        });
    }
    positive("max_step_nm", config.max_step_nm)?;
    positive("max_bias_nm", config.max_bias_nm)?;
    positive("search_range_nm", config.search_range_nm)?;
    positive("tolerance_nm", config.tolerance_nm)?;
    positive("max_epe_tolerance_nm", config.max_epe_tolerance_nm)?;
    if !(config.corner_fragment_nm.is_finite() && config.corner_fragment_nm >= 0.0) {
        return Err(LithographyError::InvalidParameter {
            name: "corner_fragment_nm",
            value: config.corner_fragment_nm,
            reason: "must be non-negative and finite",
        });
    }
    validate_conditions(&config.conditions)
}

/// Fragment-based model OPC.
///
/// Rectangles and rectilinear polygons of `mask` are fragmented and
/// corrected; other features pass through unchanged (but are imaged). Each
/// iteration images the current geometry at every condition (exact spectrum
/// → SOCS kernels at that focus), measures each controlled fragment's EPE at
/// its midpoint (threshold `config.threshold / dose`), and applies the damped,
/// clamped update `Δb = −gain·EPE̅`. Corner fragments follow their interior
/// neighbour unless `correct_corners`. After convergence (rms EPE below
/// the tolerances) or the iteration budget, jog cleanup and bias-grid
/// snapping are applied and the EPE is re-measured on the output geometry.
/// Convergence requires both the rms EPE below `tolerance_nm` and the max
/// |EPE| below `max_epe_tolerance_nm`.
///
/// Tone: for a dark-field mask the features are clear and "printed" means
/// `I > threshold`; for a bright-field mask the features are absorber and
/// "printed" means `I < threshold`.
pub fn fragment_opc(
    mask: &Mask,
    engine: &AerialImageEngine,
    config: &FragmentOpcConfig,
) -> Result<FragmentOpcResult> {
    validate_fragment_config(config)?;
    let grid = engine.grid().clone();
    let clear_feature = mask.dark_field;

    let mut fragments: Vec<Fragment> = Vec::new();
    let mut polys: Vec<OpcPolygon> = Vec::new();
    let mut passthrough: Vec<MaskFeature> = Vec::new();
    for feature in &mask.features {
        let verts = match feature {
            MaskFeature::Rect { x, y, w, h } => normalize_rectilinear(&[
                (x - w / 2.0, y - h / 2.0),
                (x + w / 2.0, y - h / 2.0),
                (x + w / 2.0, y + h / 2.0),
                (x - w / 2.0, y + h / 2.0),
            ]),
            MaskFeature::Polygon { vertices } => normalize_rectilinear(vertices),
            _ => None,
        };
        match verts {
            Some(v) => {
                let index = polys.len();
                polys.push(fragment_polygon(v, index, config, &mut fragments));
            }
            None => passthrough.push(feature.clone()),
        }
    }
    if polys.is_empty() {
        return Err(LithographyError::InvalidParameter {
            name: "mask",
            value: mask.features.len() as f64,
            reason: "no rectangle or rectilinear polygon features to correct",
        });
    }

    let render = |fragments: &[Fragment]| {
        let polygons: Vec<Vec<(f64, f64)>> = polys
            .iter()
            .map(|p| build_corrected_polygon(p, fragments))
            .collect();
        let current = assemble_mask(mask, &polygons, &passthrough);
        let images: Vec<Array2<f64>> = config
            .conditions
            .iter()
            .map(|c| engine.compute(&current, c.defocus_nm).data)
            .collect();
        (polygons, current, images)
    };

    let mut history = Vec::new();
    let mut converged = false;
    for iteration in 0..=config.max_iterations {
        let (_, _, images) = render(&fragments);
        let stats = measure_all_epe(
            &mut fragments,
            &images,
            &config.conditions,
            &grid,
            config,
            clear_feature,
            iteration,
        );
        let done = stats.epe_rms_nm < config.tolerance_nm
            && stats.epe_max_nm < config.max_epe_tolerance_nm;
        history.push(stats);
        if done {
            converged = true;
            break;
        }
        if iteration == config.max_iterations {
            break;
        }
        let steps = controller_steps(&polys, &fragments, config);
        for (frag, step) in fragments.iter_mut().zip(steps) {
            frag.bias_nm = (frag.bias_nm + step).clamp(-config.max_bias_nm, config.max_bias_nm);
        }
        if !config.correct_corners {
            slave_corner_fragments(&polys, &mut fragments);
        }
    }

    // Output geometry: jog cleanup, grid snapping, re-slaved corners.
    jog_cleanup(&polys, &mut fragments, config.min_jog_nm);
    if config.bias_grid_nm > 0.0 {
        for frag in fragments.iter_mut() {
            frag.bias_nm = (frag.bias_nm / config.bias_grid_nm).round() * config.bias_grid_nm;
        }
    }
    if !config.correct_corners {
        slave_corner_fragments(&polys, &mut fragments);
    }
    let (polygons, out_mask, images) = render(&fragments);
    let final_stats = measure_all_epe(
        &mut fragments,
        &images,
        &config.conditions,
        &grid,
        config,
        clear_feature,
        history.len(),
    );
    let final_image = images.into_iter().next().unwrap_or_default();

    Ok(FragmentOpcResult {
        mask: out_mask,
        polygons,
        fragments,
        history,
        final_stats,
        converged,
        final_image,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mask::MaskType;
    use crate::optics::ProjectionOptics;
    use crate::source::{IlluminationShape, VuvSource};

    #[test]
    fn test_rule_based_opc_applies_bias() {
        let rules = OpcRuleTable {
            rules: vec![OpcRule {
                min_width_nm: 50.0,
                max_width_nm: 100.0,
                bias_nm: 5.0,
            }],
        };

        let mask = Mask {
            mask_type: MaskType::Binary,
            features: vec![MaskFeature::Rect {
                x: 0.0,
                y: 0.0,
                w: 65.0,
                h: 500.0,
            }],
            dark_field: false,
        };

        let biased = rules.apply(&mask);
        match &biased.features[0] {
            MaskFeature::Rect { w, .. } => {
                assert!(
                    (w - 75.0).abs() < 1e-10,
                    "Expected 65 + 2*5 = 75, got {}",
                    w
                );
            }
            _ => panic!("Expected Rect"),
        }
    }

    #[test]
    fn test_rule_no_match_zero_bias() {
        let rules = OpcRuleTable {
            rules: vec![OpcRule {
                min_width_nm: 50.0,
                max_width_nm: 100.0,
                bias_nm: 5.0,
            }],
        };

        let mask = Mask {
            mask_type: MaskType::Binary,
            features: vec![MaskFeature::Rect {
                x: 0.0,
                y: 0.0,
                w: 200.0, // outside rule range
                h: 500.0,
            }],
            dark_field: false,
        };

        let biased = rules.apply(&mask);
        match &biased.features[0] {
            MaskFeature::Rect { w, .. } => {
                assert!((w - 200.0).abs() < 1e-10, "No rule should match");
            }
            _ => panic!("Expected Rect"),
        }
    }

    #[test]
    fn test_polygon_opc_applies_bias() {
        let rules = OpcRuleTable {
            rules: vec![OpcRule {
                min_width_nm: 50.0,
                max_width_nm: 200.0,
                bias_nm: 5.0,
            }],
        };

        // Square polygon centered at origin, 100 nm wide (CCW winding)
        let mask = Mask {
            mask_type: MaskType::Binary,
            features: vec![MaskFeature::Polygon {
                vertices: vec![(-50.0, -50.0), (50.0, -50.0), (50.0, 50.0), (-50.0, 50.0)],
            }],
            dark_field: false,
        };

        let biased = rules.apply(&mask);
        match &biased.features[0] {
            MaskFeature::Polygon { vertices } => {
                assert_eq!(vertices.len(), 4);
                // Each vertex of the square should move outward by ~5 nm
                // along the diagonal (bias applied to each edge).
                // The bounding box width of the biased polygon should be
                // approximately 100 + 2*5 = 110 nm.
                let (min_x, max_x) = vertices
                    .iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), &(x, _)| {
                        (mn.min(x), mx.max(x))
                    });
                let biased_width = max_x - min_x;
                assert!(
                    (biased_width - 110.0).abs() < 1.0,
                    "Expected biased width ~110, got {}",
                    biased_width
                );
            }
            _ => panic!("Expected Polygon"),
        }
    }

    #[test]
    fn test_polygon_no_bias_when_no_rule() {
        let rules = OpcRuleTable {
            rules: vec![OpcRule {
                min_width_nm: 200.0,
                max_width_nm: 300.0,
                bias_nm: 5.0,
            }],
        };

        let mask = Mask {
            mask_type: MaskType::Binary,
            features: vec![MaskFeature::Polygon {
                vertices: vec![(-50.0, -50.0), (50.0, -50.0), (50.0, 50.0), (-50.0, 50.0)],
            }],
            dark_field: false,
        };

        let biased = rules.apply(&mask);
        match &biased.features[0] {
            MaskFeature::Polygon { vertices } => {
                // No matching rule for width=100, vertices should be unchanged
                assert!((vertices[0].0 - (-50.0)).abs() < 1e-10);
                assert!((vertices[1].0 - 50.0).abs() < 1e-10);
            }
            _ => panic!("Expected Polygon"),
        }
    }

    #[test]
    fn test_bias_mask_positive() {
        let mask = Mask {
            mask_type: MaskType::Binary,
            features: vec![MaskFeature::Rect {
                x: 0.0,
                y: 0.0,
                w: 65.0,
                h: 500.0,
            }],
            dark_field: false,
        };

        let biased = bias_mask_features(&mask, 3.0);
        match &biased.features[0] {
            MaskFeature::Rect { w, .. } => {
                assert!((w - 71.0).abs() < 1e-10);
            }
            _ => panic!("Expected Rect"),
        }
    }

    // ---------------------------------------------------------------------
    // Exact-imaging helpers
    // ---------------------------------------------------------------------

    fn test_engine(size: usize, pixel_nm: f64, max_kernels: usize) -> AerialImageEngine {
        let source = VuvSource {
            illumination: IlluminationShape::Conventional { sigma: 0.6 },
            ..VuvSource::f2_laser(0.6).unwrap()
        };
        let optics = ProjectionOptics::new(0.75).unwrap();
        AerialImageEngine::new(&source, &optics, GridConfig { size, pixel_nm }, max_kernels)
            .unwrap()
    }

    fn single(feature: MaskFeature, dark_field: bool) -> Mask {
        Mask {
            mask_type: MaskType::Binary,
            features: vec![feature],
            dark_field,
        }
    }

    #[test]
    fn test_mask_spectrum_matches_numerical_integration() {
        // Independent check of the spectrum convention OPC relies on
        // (`Mask::spectrum`, exact for simple polygons): right triangle
        // (0,0)-(90,0)-(0,70), shifted, against a 400×400 midpoint-rule
        // integral of e^{−2πi f·r} (DFT scale, first-pixel-centre phase).
        let grid = GridConfig {
            size: 32,
            pixel_nm: 8.0,
        };
        let (ox, oy) = (-23.0, 11.0);
        let tri = vec![(ox, oy), (ox + 90.0, oy), (ox, oy + 70.0)];
        let spec =
            single(MaskFeature::Polygon { vertices: tri }, true).spectrum(&grid, &Fft2D::new());
        let n = grid.size;
        let df = grid.freq_step();
        let x_first = -grid.field_size_nm() / 2.0 + 0.5 * grid.pixel_nm;
        let m = 400;
        for &(i, j) in &[(0usize, 1usize), (1, 0), (2, 3), (31, 2), (5, 29)] {
            let (fx, fy) = (fft_freq(j, n, df), fft_freq(i, n, df));
            let mut acc = Complex64::new(0.0, 0.0);
            let (hx, hy) = (90.0 / m as f64, 70.0 / m as f64);
            for a in 0..m {
                let u = (a as f64 + 0.5) * hx;
                for b in 0..m {
                    let v = (b as f64 + 0.5) * hy;
                    if u / 90.0 + v / 70.0 <= 1.0 {
                        let (x, y) = (ox + u - x_first, oy + v - x_first);
                        acc += Complex64::from_polar(
                            hx * hy,
                            -2.0 * std::f64::consts::PI * (fx * x + fy * y),
                        );
                    }
                }
            }
            let expected = acc / (grid.pixel_nm * grid.pixel_nm);
            let got = spec[[i, j]];
            // Midpoint rule on a staircase boundary: ~1e-3 relative accuracy.
            assert!(
                (got - expected).norm() < 3e-3 * spec[[0, 0]].norm(),
                "({i},{j}): {got} vs {expected}"
            );
        }
    }

    #[test]
    fn test_socs_image_matches_engine() {
        let engine = test_engine(64, 8.0, 8);
        let t = Array2::from_shape_fn((64, 64), |(i, j)| {
            Complex64::new(
                0.5 + 0.4 * (i as f64 * 0.21).sin() * (j as f64 * 0.13).cos(),
                0.0,
            )
        });
        let mut spec = t.clone();
        let fft = Fft2D::new();
        fft.forward(&mut spec);
        for z in [0.0, 80.0] {
            let ours = socs_image(&engine.kernels(z), &spec, engine.flare_fraction(), &fft);
            let theirs = engine.compute_from_transmittance(&t, z).data;
            for (a, b) in ours.iter().zip(theirs.iter()) {
                assert!((a - b).abs() < 1e-12 * b.abs().max(1e-3));
            }
        }
    }

    #[test]
    fn test_sample_image_reproduces_band_limited_field() {
        // cos pattern with 5 × 4 periods over a 512 nm field on 8 nm pixels
        // (≥ 12 samples per period): Keys interpolation error ≲ 1e-3.
        let grid = GridConfig {
            size: 64,
            pixel_nm: 8.0,
        };
        let f = |x: f64, y: f64| {
            let two_pi = 2.0 * std::f64::consts::PI;
            1.0 + 0.5 * (two_pi * (5.0 * x + 4.0 * y) / 512.0).cos()
        };
        let image = Array2::from_shape_fn((64, 64), |(i, j)| {
            f(
                -256.0 + (j as f64 + 0.5) * 8.0,
                -256.0 + (i as f64 + 0.5) * 8.0,
            )
        });
        for k in 0..50 {
            let x = -250.0 + 9.73 * k as f64;
            let y = 200.0 - 7.31 * k as f64;
            assert!((sample_image(&image, &grid, x, y) - f(x, y)).abs() < 2e-3);
        }
    }

    #[test]
    fn test_measure_epe_on_synthetic_edge() {
        // Bright for x < x_edge (clear feature on the left): the printed edge
        // at threshold 0.5 sits exactly at x_edge.
        let grid = GridConfig {
            size: 64,
            pixel_nm: 8.0,
        };
        let x_edge = 13.7;
        let image = Array2::from_shape_fn((64, 64), |(_, j)| {
            let x = -256.0 + (j as f64 + 0.5) * 8.0;
            0.5 + 0.5 * ((x_edge - x) / 40.0).tanh()
        });
        // Target edge at x = 10 with outward normal +x: EPE = +3.7 nm.
        let (epe, found) = measure_epe(&image, &grid, (10.0, 0.0), (1.0, 0.0), 0.5, true, 40.0);
        assert!(found);
        assert!((epe - 3.7).abs() < 0.05, "{epe}");
        // Opaque feature on the right (printed where I < 0.5), target edge
        // at x = 20 with outward normal −x: printed edge 6.3 nm outside.
        let (epe, found) = measure_epe(&image, &grid, (20.0, 5.0), (-1.0, 0.0), 0.5, false, 40.0);
        assert!(found);
        assert!((epe - 6.3).abs() < 0.05, "{epe}");
        // Out of range: saturates with the sign of the inner end.
        let (epe, found) = measure_epe(&image, &grid, (-100.0, 0.0), (1.0, 0.0), 0.5, true, 40.0);
        assert!(!found);
        assert_eq!(epe, 40.0);
    }

    // ---------------------------------------------------------------------
    // Fragmentation geometry
    // ---------------------------------------------------------------------

    fn fragmented_rect(w: f64, h: f64, config: &FragmentOpcConfig) -> (OpcPolygon, Vec<Fragment>) {
        let verts = normalize_rectilinear(&[
            (-w / 2.0, -h / 2.0),
            (w / 2.0, -h / 2.0),
            (w / 2.0, h / 2.0),
            (-w / 2.0, h / 2.0),
        ])
        .unwrap();
        let mut fragments = Vec::new();
        let poly = fragment_polygon(verts, 0, config, &mut fragments);
        (poly, fragments)
    }

    fn opc_config() -> FragmentOpcConfig {
        FragmentOpcConfig {
            max_fragment_nm: 63.0,
            corner_fragment_nm: 52.0,
            ..Default::default()
        }
    }

    #[test]
    fn test_fragmentation_counts_and_kinds() {
        // 110 × 500 line: each 110 nm end (< 2.5 × 52) is one line-end
        // fragment; each 500 nm side is 52 + 7 × 56.57 + 52.
        let (_, fragments) = fragmented_rect(110.0, 500.0, &opc_config());
        assert_eq!(fragments.len(), 20);
        let count = |k: FragmentKind| fragments.iter().filter(|f| f.kind == k).count();
        assert_eq!(count(FragmentKind::LineEnd), 2);
        assert_eq!(count(FragmentKind::Corner), 4);
        assert_eq!(count(FragmentKind::Edge), 14);
        let total: f64 = fragments.iter().map(|f| f.length()).sum();
        assert!((total - 2.0 * (110.0 + 500.0)).abs() < 1e-9);
        // Outward normals point away from the centre.
        for f in &fragments {
            let (cx, cy) = f.control_point();
            assert!(cx * f.normal.0 + cy * f.normal.1 > 0.0);
        }
    }

    #[test]
    fn test_corrected_polygon_geometry() {
        let (poly, mut fragments) = fragmented_rect(110.0, 500.0, &opc_config());
        let area = |v: &[(f64, f64)]| signed_area(v);
        // Zero bias reproduces the target.
        let out = build_corrected_polygon(&poly, &fragments);
        assert_eq!(out.len(), 4);
        assert!((area(&out) - 110.0 * 500.0).abs() < 1e-9);
        // Uniform bias b grows the rectangle by b on every side.
        for f in fragments.iter_mut() {
            f.bias_nm = 5.0;
        }
        let out = build_corrected_polygon(&poly, &fragments);
        assert_eq!(out.len(), 4);
        assert!((area(&out) - 120.0 * 510.0).abs() < 1e-9);
        // One interior fragment pushed out by 4 nm adds 4 × its length and
        // two jogs (four extra vertices).
        for f in fragments.iter_mut() {
            f.bias_nm = 0.0;
        }
        let id = fragments
            .iter()
            .position(|f| f.kind == FragmentKind::Edge)
            .unwrap()
            + 2;
        fragments[id].bias_nm = 4.0;
        let out = build_corrected_polygon(&poly, &fragments);
        assert_eq!(out.len(), 8);
        let expected = 110.0 * 500.0 + 4.0 * fragments[id].length();
        assert!((area(&out) - expected).abs() < 1e-9);
    }

    #[test]
    fn test_jog_cleanup_merges_small_jogs() {
        let (poly, mut fragments) = fragmented_rect(110.0, 500.0, &opc_config());
        let side: Vec<usize> = poly
            .edges
            .iter()
            .find(|e| e.fragments.len() == 9)
            .unwrap()
            .fragments
            .clone();
        // Interior fragments 1..=3 have equal lengths.
        fragments[side[1]].bias_nm = 3.0;
        fragments[side[2]].bias_nm = 3.4;
        fragments[side[3]].bias_nm = 8.0;
        jog_cleanup(&[poly], &mut fragments, 1.0);
        assert!((fragments[side[1]].bias_nm - 3.2).abs() < 1e-9);
        assert!((fragments[side[2]].bias_nm - 3.2).abs() < 1e-9);
        assert!((fragments[side[3]].bias_nm - 8.0).abs() < 1e-9);
    }

    #[test]
    fn test_normalize_rectilinear() {
        // Clockwise with a redundant collinear vertex → CCW, 4 vertices.
        let cw = [
            (0.0, 0.0),
            (0.0, 10.0),
            (5.0, 10.0),
            (10.0, 10.0),
            (10.0, 0.0),
        ];
        let v = normalize_rectilinear(&cw).unwrap();
        assert_eq!(v.len(), 4);
        assert!(signed_area(&v) > 0.0);
        // Diagonal edges are not rectilinear.
        assert!(normalize_rectilinear(&[(0.0, 0.0), (10.0, 0.0), (0.0, 10.0)]).is_none());
    }

    // ---------------------------------------------------------------------
    // Fragment OPC convergence
    // ---------------------------------------------------------------------

    /// Line-end pattern: a 90 × 500 nm line (k₁ ≈ 0.43) in a 768 nm field.
    fn line_mask(dark_field: bool) -> Mask {
        single(
            MaskFeature::Rect {
                x: 0.0,
                y: 0.0,
                w: 90.0,
                h: 500.0,
            },
            dark_field,
        )
    }

    /// T-junction: a 500 × 100 nm bar on a 100 nm wide, 380 nm long stem
    /// (k₁ ≈ 0.48).
    fn tee_mask(dark_field: bool) -> Mask {
        single(
            MaskFeature::Polygon {
                vertices: vec![
                    (-250.0, 80.0),
                    (-250.0, 180.0),
                    (250.0, 180.0),
                    (250.0, 80.0),
                    (50.0, 80.0),
                    (50.0, -300.0),
                    (-50.0, -300.0),
                    (-50.0, 80.0),
                ],
            },
            dark_field,
        )
    }

    /// Dose-to-size print threshold: the vertical line (or stem) centred at
    /// x = 0 prints `cd_nm` wide on the cut at `y_nm`.
    fn sized_threshold(engine: &AerialImageEngine, mask: &Mask, y_nm: f64, cd_nm: f64) -> f64 {
        let cut = crate::sraf::LineCut {
            x_center_nm: 0.0,
            y_nm,
            half_width_nm: 1.5 * cd_nm,
            dark_line: !mask.dark_field,
        };
        crate::sraf::dose_to_size_threshold(engine, mask, &cut, cd_nm).unwrap()
    }

    fn rms_epe_at(
        engine: &AerialImageEngine,
        result: &FragmentOpcResult,
        defocus_nm: f64,
        threshold: f64,
        clear_feature: bool,
    ) -> f64 {
        let image = engine.compute(&result.mask, defocus_nm).data;
        let e: Vec<f64> = result
            .fragments
            .iter()
            .filter(|f| f.kind != FragmentKind::Corner)
            .map(|f| {
                measure_epe(
                    &image,
                    engine.grid(),
                    f.control_point(),
                    f.normal,
                    threshold,
                    clear_feature,
                    80.0,
                )
                .0
            })
            .collect();
        (e.iter().map(|v| v * v).sum::<f64>() / e.len() as f64).sqrt()
    }

    #[test]
    fn test_fragment_opc_line_end_converges() {
        // Exposed at dose-to-size (the line centre prints 90 nm wide), the
        // uncorrected line ends pull back by more than 20 nm in both tones;
        // OPC extends them and brings every controlled EPE within tolerance.
        let engine = test_engine(64, 12.0, 12);
        for dark_field in [false, true] {
            let mask = line_mask(dark_field);
            let threshold = sized_threshold(&engine, &mask, 0.0, 90.0);
            let config = FragmentOpcConfig {
                tolerance_nm: 0.5,
                max_epe_tolerance_nm: 1.0,
                ..FragmentOpcConfig::for_engine(&engine, threshold)
            };
            let result = fragment_opc(&mask, &engine, &config).unwrap();
            assert!(
                result.converged,
                "dark_field={dark_field}: {:?}",
                result.history.last()
            );
            assert!(result.final_stats.epe_rms_nm < 1.5 * config.tolerance_nm);
            assert!(result.history[0].epe_rms_nm > 5.0 * result.final_stats.epe_rms_nm);
            let uncorrected = engine.compute(&mask, 0.0).data;
            let ends: Vec<&Fragment> = result
                .fragments
                .iter()
                .filter(|f| f.kind == FragmentKind::LineEnd)
                .collect();
            assert_eq!(ends.len(), 2);
            for f in ends {
                let (before, _) = measure_epe(
                    &uncorrected,
                    engine.grid(),
                    f.control_point(),
                    f.normal,
                    threshold,
                    dark_field,
                    config.search_range_nm,
                );
                assert!(before < -20.0, "uncorrected line-end EPE {before}");
                assert!(f.bias_nm > 15.0, "line end bias {}", f.bias_nm);
                assert!(
                    f.epe_nm.abs() < 1.5,
                    "line end EPE {} (was {before})",
                    f.epe_nm
                );
            }
        }
    }

    #[test]
    fn test_fragment_opc_t_junction_converges() {
        // T with 100 nm arms at the stem's dose-to-size threshold: junction
        // fill-in and line-end pull-back leave > 15 nm max EPE uncorrected;
        // the default (λ/NA-scaled) settings converge.
        let engine = test_engine(64, 12.0, 12);
        for dark_field in [false, true] {
            let mask = tee_mask(dark_field);
            let threshold = sized_threshold(&engine, &mask, -150.0, 100.0);
            let config = FragmentOpcConfig::for_engine(&engine, threshold);
            let result = fragment_opc(&mask, &engine, &config).unwrap();
            let first = &result.history[0];
            assert!(
                first.epe_max_nm > 15.0,
                "uncorrected max EPE {}",
                first.epe_max_nm
            );
            assert!(
                result.converged,
                "dark_field={dark_field}: {:?}",
                result.history.last()
            );
            assert!(result.final_stats.epe_rms_nm < 1.5 * config.tolerance_nm);
            assert!(first.epe_rms_nm > 5.0 * result.final_stats.epe_rms_nm);
            assert_eq!(result.polygons.len(), 1);
            assert!(signed_area(&result.polygons[0]) > 0.0);
        }
    }

    #[test]
    fn test_fragment_opc_process_window_balances_defocus() {
        // Nominal-only correction leaves a large EPE at 120 nm defocus; the
        // (focus, defocus) weighted correction trades nominal EPE for it.
        let engine = test_engine(64, 12.0, 12);
        let mask = line_mask(false);
        let threshold = sized_threshold(&engine, &mask, 0.0, 90.0);
        let nominal = FragmentOpcConfig::for_engine(&engine, threshold);
        let pw = FragmentOpcConfig {
            conditions: vec![
                ProcessCondition::nominal(),
                ProcessCondition::new(120.0, 1.0),
            ],
            ..nominal.clone()
        };
        let r_nominal = fragment_opc(&mask, &engine, &nominal).unwrap();
        let r_pw = fragment_opc(&mask, &engine, &pw).unwrap();
        assert!(r_pw.converged);
        let (d_nominal, d_pw) = (
            rms_epe_at(&engine, &r_nominal, 120.0, threshold, false),
            rms_epe_at(&engine, &r_pw, 120.0, threshold, false),
        );
        assert!(
            d_pw < 0.6 * d_nominal,
            "defocus EPE pw {d_pw} vs nominal {d_nominal}"
        );
    }

    #[test]
    fn test_model_based_opc_converges_on_periodic_line_space() {
        // `Mask::line_space` is one infinite grating feature (opaque lines of
        // width cd); the uniform bias reaches it through `with_edge_bias`.
        // On a commensurate field the centre-row CD is driven to target.
        let source = VuvSource {
            illumination: IlluminationShape::Conventional { sigma: 0.6 },
            ..VuvSource::f2_laser(0.6).unwrap()
        };
        let optics = ProjectionOptics::new(0.75).unwrap();
        let grid = GridConfig::commensurate(180.0, None, 64, 8.0).unwrap();
        let engine = AerialImageEngine::new(&source, &optics, grid, 12).unwrap();
        let mask = Mask::line_space(90.0, 180.0).unwrap();
        // Drawn 90 nm lines print ≈ 90 nm at t = 0.3; size them to 80 nm.
        let (threshold, target) = (0.3, 80.0);
        let half = engine.grid().field_size_nm() / 2.0;
        let uncorrected =
            metrics::measure_cd_2d(&engine.compute(&mask, 0.0).data, -half, half, threshold)
                .unwrap();
        assert!(
            (uncorrected - target).abs() > 2.0,
            "no correction needed: {uncorrected}"
        );
        let (corrected, history) =
            model_based_opc(&mask, &engine, target, threshold, 20, 0.25).unwrap();
        let last = history.iterations.last().unwrap();
        assert!(last.error_nm.abs() < 0.25, "{:?}", history.iterations);
        match corrected.features.as_slice() {
            [MaskFeature::LineSpace { cd, pitch, .. }] => {
                assert!((cd - 90.0).abs() > 1.0, "grating CD not biased: {cd}");
                assert_eq!(*pitch, 180.0);
            }
            other => panic!("expected one grating, got {other:?}"),
        }
    }

    #[test]
    fn test_fragment_opc_passthrough_snapping_and_errors() {
        let engine = test_engine(64, 12.0, 12);
        let gray = MaskFeature::GrayRect {
            x: 250.0,
            y: 250.0,
            w: 40.0,
            h: 40.0,
            transmittance: 0.5,
        };
        assert!(fragment_opc(
            &single(gray.clone(), false),
            &engine,
            &FragmentOpcConfig::default()
        )
        .is_err());
        let mut mask = line_mask(false);
        mask.features.push(gray);
        let config = FragmentOpcConfig {
            bias_grid_nm: 0.5,
            ..FragmentOpcConfig::for_engine(&engine, 0.35)
        };
        let result = fragment_opc(&mask, &engine, &config).unwrap();
        assert_eq!(result.mask.features.len(), 2);
        assert!(matches!(
            result.mask.features[1],
            MaskFeature::GrayRect { .. }
        ));
        for f in &result.fragments {
            let q = f.bias_nm / 0.5;
            assert!(
                (q - q.round()).abs() < 1e-9,
                "bias {} not on grid",
                f.bias_nm
            );
        }
        let bad = FragmentOpcConfig {
            smoothing: 1.5,
            ..FragmentOpcConfig::default()
        };
        assert!(fragment_opc(&line_mask(false), &engine, &bad).is_err());
    }
}
