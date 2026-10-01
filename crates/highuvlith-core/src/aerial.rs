//! Aerial image formation by Hopkins partially-coherent imaging.
//!
//! Computes the intensity image an optical system forms from a thin mask
//! under an extended (partially coherent) source. The source pupil fill is
//! sampled adaptively into discrete, mutually incoherent points `s` with
//! weights `w_s`; the transmission cross-coefficient (TCC) is then held in
//! exactly factorized form `TCC = A·Aᴴ`, with one column of `A` per source
//! point (or per source point × polarization field component in vector
//! mode). Its leading eigenpairs — the Sum-Of-Coherent-Systems (SOCS)
//! kernels — are obtained without ever forming a mask-frequency² matrix
//! when the source is the smaller dimension (Gram trick), with a dense
//! Hermitian eigensolver for small problems and randomized subspace
//! iteration for large ones (see [`crate::math::linalg`]).
//!
//! Defocus is applied where it physically acts, inside the pupil at every
//! source point (`P(f + s; z)`), so kernel sets are rebuilt per focus plane
//! and per wavelength and kept in a bounded, thread-safe cache. Mask orders
//! are kept out to `|f| ≤ (1 + σ_max)·NA/λ`, the full band that off-axis
//! source points can steer into the pupil.
//!
//! # Key equations
//!
//! ```text
//!   Abbe (reference):   I(x) = Σ_s w_s · |IFFT[ P((f+s)/f_c; z, λ) · M(f) ](x)|²
//!   Hopkins:            TCC(f, f') = Σ_s w_s P(f+s) P*(f'+s) = (A Aᴴ)_{f f'},
//!                       A[f, s] = √w_s · P((f+s)/f_c; z, λ),   f_c = NA/λ
//!   SOCS:               TCC = Σ_k λ_k u_k u_kᴴ  ⇒  I(x) = Σ_k λ_k |IFFT[u_k · M](x)|²
//!   Gram trick:         (AᴴA) v_k = λ_k v_k,   u_k = A v_k / √λ_k
//!   mask support:       |f| ≤ (1 + σ_max) · f_c
//!   captured energy:    Σ_{k<K} λ_k / tr(TCC),   tr(TCC) = ‖A‖_F²
//!   clear field:        I_clear = TCC(0,0) = Σ_s Σ_cols |A[f=0; s]|²
//!   relative intensity: I_rel = I / I_clear                (ImageNormalization::ClearField)
//!   flare:              I ← (1 − φ)·I + φ·⟨I⟩
//! ```
//! `M = FFT(t)` is the (unnormalized) mask spectrum and the inverse FFT
//! carries `1/N²`, so a clear mask gives the absolute intensity
//! `I_clear = Σ_s w_s |P(s)|²` (scalar; summed over field columns in vector
//! mode) relative to the illumination incident on the mask. Images are
//! reported as **relative intensity** by default — divided by the exact
//! `I_clear` of the same illumination, optics, wavelength and focus plane —
//! so a clear mask images to exactly 1 before flare for any pupil
//! (obscured, apodized, zone-plate efficiency, vector radiometric factor).
//! `ImageNormalization::Absolute` skips the division; the value itself is
//! [`AerialImageEngine::clear_field_intensity`]. The defocus phase inside `P`
//! comes from the optic: `(2π n/λ) z (1 − √(1 − (NAρ/n)²))` in the
//! image-space medium of index `n` for the in-tree optics.
//!
//! # Model status
//!
//! - **Exact** (up to the source discretization, which is reported and
//!   inspectable via [`AerialImageEngine::source_points`]): scalar Hopkins
//!   imaging of a thin (Kirchhoff) mask on the periodic simulation field,
//!   including partially coherent resolution out to `(1 + σ)·NA/λ`,
//!   defocus inside the pupil at every source point
//!   ([`DefocusModel::Exact`], default), and per-wavelength TCCs in
//!   [`AerialImageEngine::compute_multiwavelength`]. Keeping every kernel
//!   (`kernel_energy_fraction = 1`) reproduces the Abbe sum to rounding.
//! - **Source discretization** (default adaptive sampling, see
//!   [`sample_source`]): the pupil fill is integrated over the cells of a
//!   rotated (`tan θ = 1/φ`) square lattice of step ≤ 0.04 σ (≥ ~200 cells
//!   in the support), with exact area weights at hard source edges and
//!   peaked fills refined until no point carries more than ~1/200 of the
//!   intensity; no row of points is parallel to an axis, so results vary
//!   smoothly with σ and pitch instead of in sampling steps. Measured
//!   against fine-quadrature Abbe references
//!   (`tests/imaging_source_sampling.rs`): ≤ 0.0031 in contrast over a KrF
//!   σ sweep across the k₁ onset (0.011 with the earlier axis-aligned grid),
//!   ≤ 0.019 over near-coherent (σ 0.05, Gaussian or disk) pitch sweeps
//!   through the cutoff (0.11 / 0.056 before). The residual error is a
//!   discretization error of either sign (e.g. an OPC line-end bias of
//!   9.9 nm against 10.1 nm converged); `source_points_per_axis` refines it.
//! - **Approximations** (all opt-in or reported): SOCS truncation
//!   (`captured_energy_fraction` is returned with every kernel set);
//!   randomized eigensolver for problems larger than the dense limits;
//!   [`DefocusModel::KernelPhase`] (legacy on-axis phase per mask frequency —
//!   wrong for off-axis illumination through focus);
//!   [`AerialImageEngine::compute_polychromatic`] (focus shift only, TCC at
//!   the center wavelength — narrow band only); zeroth-order uniform flare.
//! - **Vector imaging** ([`ImagingModel::Vector`]) is plumbing: the physics
//!   is whatever [`crate::optics::vector`] provides. The engine sets
//!   `VectorSettings::reduction` from the optics, requires `image_index` to
//!   equal the optics' immersion index (inheriting it when left at 1), and
//!   validates the settings against the NA.
//! - **Normalization**: relative intensity by default (see above).
//!   Dark-field configurations — clear field below 1 % of the pupil's peak
//!   |P|², i.e. the pupil (e.g. a central obscuration) blocks the zero order
//!   for essentially the whole source — have no meaningful relative
//!   intensity; they stay absolute and [`KernelDiagnostics::normalized`] is
//!   `false`.
//! - Mask frequencies beyond the grid Nyquist `1/(2·pixel)` cannot be
//!   represented; pointwise intensities are exact only for
//!   `pixel ≤ λ / (2·NA·(1 + σ_max))` ([`KernelDiagnostics`] flags it).
//!
//! # References
//!
//! - H. H. Hopkins, "On the diffraction theory of optical images,"
//!   Proc. R. Soc. Lond. A 217, 408–432 (1953).
//! - N. Cobb, "Fast optical and process proximity correction algorithms for
//!   integrated circuit manufacturing," PhD thesis, UC Berkeley (1998) — SOCS.

use std::sync::{Arc, Mutex};

use ndarray::{Array2, Zip};
use serde::{Deserialize, Serialize};

use crate::compute::parallel::{for_each_chunk_mut, map_range};
use crate::error::{LithographyError, Result};
use crate::mask::Mask;
use crate::math::fft2d::Fft2D;
use crate::math::linalg::{self, ColMatrix};
use crate::mnsl::{MnslConfig, MnslEngine, MnslResult};
use crate::optics::vector::{self, VectorSettings};
use crate::optics::OpticalSystem;
use crate::source::LithographySource;
use crate::types::{Complex64, Grid2D, GridConfig};

// ---------------------------------------------------------------------------
// Public settings and result types
// ---------------------------------------------------------------------------

/// How defocus enters the imaging kernels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefocusModel {
    /// Defocus inside the pupil at every source point, `P(f + s; z)`; the
    /// kernel set is rebuilt (and cached) per focus plane. Exact.
    #[default]
    Exact,
    /// Legacy fast approximation: in-focus kernels multiplied by the on-axis
    /// paraxial phase `exp(iπ z λ |f|²)` of each mask frequency `f`. Exact
    /// only for coherent on-axis illumination; with off-axis / partially
    /// coherent sources it misstates the depth of focus (it cannot represent
    /// two-beam imaging).
    KernelPhase,
}

/// Scalar or vector (polarized) imaging.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImagingModel {
    /// Scalar diffraction (one `A` column per source point).
    #[default]
    Scalar,
    /// Vector imaging: each source point contributes
    /// [`vector::columns_per_source_point`] columns filled by
    /// [`vector::field_columns`].
    Vector(VectorSettings),
}

/// How aerial images are scaled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageNormalization {
    /// Relative intensity (lithography convention): divided by the exact
    /// clear-field intensity `TCC(0,0)` of the same illumination, optics,
    /// wavelength and focus plane, so a clear mask images to 1 before flare.
    #[default]
    ClearField,
    /// Absolute: intensity relative to the illumination incident on the
    /// mask, including pupil transmission, obscuration, apodization and the
    /// vector radiometric factor (a clear mask images to `TCC(0,0)`).
    Absolute,
}

/// Numerical settings of the imaging engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImagingSettings {
    /// Upper bound on the number of SOCS kernels per kernel set (≥ 1).
    pub max_kernels: usize,
    /// Stop adding kernels once they capture this fraction of the TCC trace
    /// (in (0, 1]; the default 1 keeps every numerically non-zero eigenpair
    /// up to `max_kernels`). The trace fraction is a loose proxy for image
    /// error: the clear-field error is bounded by `tr(TCC)·(1 − fraction)`.
    pub kernel_energy_fraction: f64,
    /// Source sampling density: `None` = adaptive (rotated lattice of step
    /// ≤ 0.04 σ with at least ~200 cells inside the source support,
    /// area-weighted at hard edges, peaked fills refined; see
    /// [`sample_source`]); `Some(n)` = a plain `n × n` midpoint grid across
    /// the larger side of the source's bounding box, weights = sampled
    /// intensity (`Some(1)` = a single point at the intensity centroid).
    pub source_points_per_axis: Option<usize>,
    /// How defocus enters the kernels.
    pub defocus_model: DefocusModel,
    /// Scalar or vector imaging.
    pub imaging_model: ImagingModel,
    /// Maximum number of kernel sets (focus planes × wavelengths) kept in
    /// the cache; 0 disables caching.
    pub kernel_cache_capacity: usize,
    /// Image scaling: relative (clear field = 1, default) or absolute.
    pub normalization: ImageNormalization,
}

impl Default for ImagingSettings {
    fn default() -> Self {
        Self {
            max_kernels: 48,
            kernel_energy_fraction: 1.0,
            source_points_per_axis: None,
            defocus_model: DefocusModel::Exact,
            imaging_model: ImagingModel::Scalar,
            kernel_cache_capacity: 32,
            normalization: ImageNormalization::ClearField,
        }
    }
}

/// A SOCS kernel set for one focus plane and wavelength.
///
/// Invariant (contract C1): for the engine's grid, the aerial image before
/// flare is `Σ_k eigenvalues[k] · |IFFT(kernels[k] ⊙ spectrum)|²`, with the
/// spectrum from [`Mask::spectrum`] and [`Fft2D::inverse`] (1/N²-normalized).
/// The eigenvalues already carry the engine's [`ImageNormalization`] (divided
/// by the clear-field intensity in the default mode).
#[derive(Debug, Clone)]
pub struct KernelSet {
    /// SOCS eigenvalues λ_k, decreasing.
    pub eigenvalues: Vec<f64>,
    /// Kernels in the frequency domain, same layout as `Fft2D::forward` of an
    /// n×n map; unit 2-norm; zero outside the mask-frequency support.
    pub kernels: Vec<Array2<Complex64>>,
    /// Focus plane (nm) the kernels were built for.
    pub defocus_nm: f64,
    /// Wavelength (nm) the kernels were built for.
    pub wavelength_nm: f64,
    /// `Σ_k λ_k / tr(TCC)` for the retained kernels.
    pub captured_energy_fraction: f64,
}

/// One discrete source point: pupil-fill coordinate in σ units (normalized
/// to NA/λ) and its normalized incoherent weight (weights sum to 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourcePoint {
    /// x coordinate in σ units.
    pub sx: f64,
    /// y coordinate in σ units.
    pub sy: f64,
    /// Normalized intensity weight.
    pub weight: f64,
}

/// Which eigensolver produced a kernel set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecompositionMethod {
    /// Dense Hermitian eigensolver on the (support × support) TCC.
    DenseTcc,
    /// Dense Hermitian eigensolver on the (columns × columns) Gram matrix AᴴA.
    DenseGram,
    /// Randomized subspace iteration on A (approximate; energy reported).
    Randomized,
}

/// Size and accuracy bookkeeping of one kernel set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KernelDiagnostics {
    /// Retained kernels.
    pub num_kernels: usize,
    /// `Σ_k λ_k / tr(TCC)`.
    pub captured_energy_fraction: f64,
    /// Mask frequencies in the kernel support (rows of `A`).
    pub num_frequencies: usize,
    /// Columns of `A` (source points × field columns).
    pub num_columns: usize,
    /// Eigensolver used.
    pub method: DecompositionMethod,
    /// True when `(1 + σ_max)·NA/λ` exceeds the grid Nyquist frequency, i.e.
    /// part of the physically transmitted band is not representable.
    pub support_exceeds_nyquist: bool,
    /// Absolute clear-field intensity `TCC(0,0)` (relative to the incident
    /// illumination).
    pub clear_field_intensity: f64,
    /// Whether images from this kernel set are divided by the clear field
    /// (false in `Absolute` mode and for dark-field configurations, where
    /// the clear field is below 1 % of the pupil's peak |P|²).
    pub normalized: bool,
}

// ---------------------------------------------------------------------------
// Tuning constants
// ---------------------------------------------------------------------------

/// Memory budget for the factor `A` (complex f64 entries). Replaces the old
/// `MAX_PUPIL_SAMPLES` guard on the n_freq² TCC.
const MAX_FACTOR_BYTES: usize = 1 << 30;
/// Eigenproblems up to this size always use the dense solver (measured
/// crossover with the randomized path for 48 kernels: ~300 in release).
const DENSE_FAST_DIM: usize = 320;
/// Dense solver upper limit, used above `DENSE_FAST_DIM` only when the
/// requested kernels are a large share (≥ 1/3) of the spectrum.
const DENSE_MAX_DIM: usize = 1536;
/// Oversampling of the randomized range finder.
const RANDOMIZED_OVERSAMPLE: usize = 16;
/// Power iterations of the randomized range finder.
const RANDOMIZED_POWER_ITERS: usize = 3;
/// Fixed seed: results are deterministic.
const RANDOMIZED_SEED: u64 = 0x5EED_50C5;
/// Eigenvalues below this fraction of the largest are numerical zeros.
const EIGEN_ZERO_REL: f64 = 1e-12;
/// Eigenvalues closer than this (relative) form one degenerate cluster.
const DEGENERACY_REL: f64 = 1e-6;
/// Maximum number of kernel groups summed in parallel by the SOCS sum.
const SOCS_GROUPS: usize = 16;
/// Dark-field criterion: when the clear-field intensity is below this
/// fraction of the pupil's peak intensity |P|² (i.e. less than ~1 % of the
/// source's zero-order light reaches the image), imaging is dominated by
/// dark field, relative intensity is meaningless, and images stay absolute.
const DARK_FIELD_FRACTION: f64 = 1e-2;
/// Workspace budget (bytes) for the parallel SOCS sum's per-group buffers.
const SOCS_WORKSPACE_BYTES: usize = 256 << 20;

/// Source probe: half-width (σ units) of the level-0 search square.
const PROBE_HALF_WIDTH: f64 = 2.0;
/// Source probe: points per axis at level 0 (step 1/32 σ).
const PROBE_POINTS: usize = 129;
/// Source probe: points per axis at each zoom level.
const ZOOM_POINTS: usize = 129;
/// Source probe: maximum zoom levels.
const MAX_ZOOM_LEVELS: usize = 6;
/// Intensity (relative to the peak) that defines the source support; 1e-4
/// keeps a Gaussian fill out to 4.3 σ_g (99.99 % of its power).
const SUPPORT_THRESHOLD: f64 = 1e-4;
/// Zoom until the support spans at least this many probe cells per axis.
const RESOLVED_CELLS: f64 = 24.0;
/// Adaptive sampling: largest lattice step (cell side) in σ units.
const TARGET_STEP_SIGMA: f64 = 0.04;
/// Adaptive sampling: target minimum number of cells inside the support;
/// also the inverse of the largest intensity share a graded (non-uniform)
/// cell may carry before it is split (see [`lattice_cells`]), which bounds
/// the jump when one point's diffraction order crosses the pupil edge.
const MIN_SOURCE_POINTS: f64 = 200.0;
/// Adaptive sampling: each cell that is not uniformly lit (source edge,
/// graded fill, clipped by a symmetry axis) is integrated on this many
/// sub-samples per lattice axis.
const SUBCELLS: usize = 8;
/// Adaptive sampling: a lattice cell is split into at most this many
/// sub-cells per axis when it holds too large a share of the intensity.
const MAX_REFINE: usize = 8;
/// Adaptive sampling: maximum nesting of cell splits.
const MAX_REFINE_DEPTH: usize = 3;
/// Adaptive sampling: lattice rotation `tan θ = 1/φ` (golden ratio), as
/// `(cos θ, sin θ)`. No lattice row is parallel to the x or y axis or to a
/// diagonal (`tan(45° − θ) = 1/φ³` is irrational too), so a straight edge
/// (a pupil cutoff crossing the source, a source boundary) sweeps over the
/// points one mirror pair at a time instead of a whole grid column at once.
const LATTICE_COS: f64 = 0.850_650_808_352_04;
const LATTICE_SIN: f64 = 0.525_731_112_119_133_6;
/// Adaptive sampling: cap on lattice cells per axis of the support box.
const MAX_POINTS_PER_AXIS: usize = 128;
/// A source whose extent (in frequency) is below this fraction of the mask
/// frequency step is treated as a single coherent point.
const COHERENT_EXTENT_FRACTION: f64 = 1e-3;

// ---------------------------------------------------------------------------
// Source sampling
// ---------------------------------------------------------------------------

struct ProbeResult {
    bx0: f64,
    bx1: f64,
    by0: f64,
    by1: f64,
    step_x: f64,
    step_y: f64,
    count: usize,
    cx: f64,
    cy: f64,
}

/// Evaluate `f` on an `n × n` lattice spanning `[x0, x1] × [y0, y1]`
/// (inclusive) and locate its support. Level-0 probes pass `centered` to use
/// exactly sign-symmetric coordinates.
fn probe_lattice(
    f: &(dyn Fn(f64, f64) -> f64 + Sync),
    (x0, x1, y0, y1): (f64, f64, f64, f64),
    n: usize,
    centered: bool,
) -> Option<ProbeResult> {
    let step_x = (x1 - x0) / (n - 1) as f64;
    let step_y = (y1 - y0) / (n - 1) as f64;
    let half = (n - 1) as f64 / 2.0;
    let coord = |lo: f64, step: f64, i: usize| {
        if centered {
            (i as f64 - half) * step
        } else {
            lo + i as f64 * step
        }
    };
    let values: Vec<f64> = map_range(n * n, |k| {
        let (iy, ix) = (k / n, k % n);
        let v = f(coord(x0, step_x, ix), coord(y0, step_y, iy));
        if v.is_finite() && v > 0.0 {
            v
        } else {
            0.0
        }
    });
    let imax = values.iter().cloned().fold(0.0, f64::max);
    if imax <= 0.0 {
        return None;
    }
    let threshold = SUPPORT_THRESHOLD * imax;
    let (mut bx0, mut bx1, mut by0, mut by1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    let (mut count, mut wsum, mut wx, mut wy) = (0usize, 0.0, 0.0, 0.0);
    for (k, &v) in values.iter().enumerate() {
        if v <= 0.0 {
            continue;
        }
        let x = coord(x0, step_x, k % n);
        let y = coord(y0, step_y, k / n);
        wsum += v;
        wx += v * x;
        wy += v * y;
        if v > threshold {
            count += 1;
            bx0 = bx0.min(x);
            bx1 = bx1.max(x);
            by0 = by0.min(y);
            by1 = by1.max(y);
        }
    }
    Some(ProbeResult {
        bx0,
        bx1,
        by0,
        by1,
        step_x,
        step_y,
        count,
        cx: wx / wsum,
        cy: wy / wsum,
    })
}

/// Adaptive sampling of an arbitrary pupil-fill intensity map (σ units);
/// the algorithm is described on [`sample_source`].
fn sample_intensity(
    f: &(dyn Fn(f64, f64) -> f64 + Sync),
    points_per_axis: Option<usize>,
    df_sigma: f64,
) -> Result<Vec<SourcePoint>> {
    let no_light = LithographyError::InvalidParameter {
        name: "illumination",
        value: 0.0,
        reason: "source pupil fill has no intensity inside |sigma| <= 2",
    };
    let mut probe = probe_lattice(
        f,
        (
            -PROBE_HALF_WIDTH,
            PROBE_HALF_WIDTH,
            -PROBE_HALF_WIDTH,
            PROBE_HALF_WIDTH,
        ),
        PROBE_POINTS,
        true,
    )
    .ok_or(no_light)?;
    for _ in 0..MAX_ZOOM_LEVELS {
        let resolved_x = probe.bx1 - probe.bx0 >= RESOLVED_CELLS * probe.step_x;
        let resolved_y = probe.by1 - probe.by0 >= RESOLVED_CELLS * probe.step_y;
        if resolved_x && resolved_y {
            break;
        }
        let bounds = (
            probe.bx0 - probe.step_x,
            probe.bx1 + probe.step_x,
            probe.by0 - probe.step_y,
            probe.by1 + probe.step_y,
        );
        match probe_lattice(f, bounds, ZOOM_POINTS, false) {
            Some(p) => probe = p,
            None => break,
        }
    }

    // Final sampling box: support bounding box plus one probe cell per side,
    // made exactly sign-symmetric when the support is symmetric to within a
    // probe cell (keeps symmetric sources symmetric to the last bit).
    let sym_x = (probe.bx0 + probe.bx1).abs() <= 0.5 * probe.step_x;
    let sym_y = (probe.by0 + probe.by1).abs() <= 0.5 * probe.step_y;
    let (cx_box, w) = if sym_x {
        (
            0.0,
            2.0 * (probe.bx0.abs().max(probe.bx1.abs()) + probe.step_x),
        )
    } else {
        (
            0.5 * (probe.bx0 + probe.bx1),
            probe.bx1 - probe.bx0 + 2.0 * probe.step_x,
        )
    };
    let (cy_box, h) = if sym_y {
        (
            0.0,
            2.0 * (probe.by0.abs().max(probe.by1.abs()) + probe.step_y),
        )
    } else {
        (
            0.5 * (probe.by0 + probe.by1),
            probe.by1 - probe.by0 + 2.0 * probe.step_y,
        )
    };
    let (w, h) = if sym_x && sym_y && (w - h).abs() <= probe.step_x.max(probe.step_y) {
        let m = w.max(h);
        (m, m)
    } else {
        (w, h)
    };
    let extent = w.max(h);

    let single = |x: f64, y: f64| {
        vec![SourcePoint {
            sx: x,
            sy: y,
            weight: 1.0,
        }]
    };
    if points_per_axis == Some(1) || extent < COHERENT_EXTENT_FRACTION * df_sigma {
        let cx = if sym_x { 0.0 } else { probe.cx };
        let cy = if sym_y { 0.0 } else { probe.cy };
        return Ok(single(cx, cy));
    }

    if let Some(n) = points_per_axis {
        // Explicit density: plain n × n midpoint grid over the support box.
        let step = extent / n.max(1) as f64;
        let nx = ((w / step).round() as usize).max(1);
        let ny = ((h / step).round() as usize).max(1);
        let (hx, hy) = (w / nx as f64, h / ny as f64);
        let mut points = Vec::new();
        for iy in 0..ny {
            let y = cy_box + (iy as f64 + 0.5 - ny as f64 / 2.0) * hy;
            for ix in 0..nx {
                let x = cx_box + (ix as f64 + 0.5 - nx as f64 / 2.0) * hx;
                let v = f(x, y);
                if v.is_finite() && v > 0.0 {
                    points.push(SourcePoint {
                        sx: x,
                        sy: y,
                        weight: v,
                    });
                }
            }
        }
        return normalized_or_centroid(points, probe.cx, probe.cy);
    }

    let area = probe.count.max(1) as f64 * probe.step_x * probe.step_y;
    let step = TARGET_STEP_SIGMA
        .min((area / MIN_SOURCE_POINTS).sqrt())
        .max(extent / MAX_POINTS_PER_AXIS as f64);
    let symmetry = Symmetry {
        x: sym_x,
        y: sym_y,
        diagonal: sym_x && sym_y && w == h,
    };
    let points = lattice_cells(f, (cx_box, cy_box), (w, h), step, symmetry);
    normalized_or_centroid(points, probe.cx, probe.cy)
}

/// An element of the sampling box's symmetry group (a subgroup of the
/// square's dihedral group): `(x, y) ↦ (±a, ±b)` with
/// `(a, b) = (y, x)` if `swap`, else `(x, y)`, and the signs set by
/// `flip_x`/`flip_y`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Mirror {
    swap: bool,
    flip_x: bool,
    flip_y: bool,
}

impl Mirror {
    const IDENTITY: Self = Self::new(false, false, false);
    /// x → −x.
    const X: Self = Self::new(false, true, false);
    /// y → −y.
    const Y: Self = Self::new(false, false, true);
    /// x ↔ y.
    const DIAGONAL: Self = Self::new(true, false, false);

    const fn new(swap: bool, flip_x: bool, flip_y: bool) -> Self {
        Self {
            swap,
            flip_x,
            flip_y,
        }
    }

    fn apply(self, x: f64, y: f64) -> (f64, f64) {
        let (a, b) = if self.swap { (y, x) } else { (x, y) };
        (
            if self.flip_x { -a } else { a },
            if self.flip_y { -b } else { b },
        )
    }

    /// `self ∘ h` (apply `h` first).
    fn compose(self, h: Self) -> Self {
        let (hx, hy) = if self.swap {
            (h.flip_y, h.flip_x)
        } else {
            (h.flip_x, h.flip_y)
        };
        Self::new(self.swap ^ h.swap, self.flip_x ^ hx, self.flip_y ^ hy)
    }

    /// The group generated by `gens` (closure under composition).
    fn generate(gens: &[Self]) -> Vec<Self> {
        let mut group = vec![Self::IDENTITY];
        let mut k = 0;
        while k < group.len() {
            for &g in gens {
                let e = group[k].compose(g);
                if !group.contains(&e) {
                    group.push(e);
                }
            }
            k += 1;
        }
        group
    }
}

/// A mirror line bounding the fundamental domain: its mirror and the test
/// "strictly on the domain side".
type Wall = (Mirror, fn(f64, f64) -> bool);

/// Mirror symmetries of the sampling box (not of the intensity: every
/// mirrored cell is integrated with its own intensity values).
#[derive(Clone, Copy)]
struct Symmetry {
    /// Box symmetric under x → −x.
    x: bool,
    /// Box symmetric under y → −y.
    y: bool,
    /// Square box symmetric under x ↔ y (requires `x` and `y`).
    diagonal: bool,
}

impl Symmetry {
    /// The mirror lines bounding the fundamental domain. With the diagonal
    /// the domain is the octant `0 < y < x` (bounded by y = 0 and y = x);
    /// otherwise the quadrant, half-plane or whole box.
    fn walls(self) -> Vec<Wall> {
        let mut walls: Vec<Wall> = Vec::new();
        if self.x && !self.diagonal {
            walls.push((Mirror::X, |x, _| x > 0.0));
        }
        if self.y {
            walls.push((Mirror::Y, |_, y| y > 0.0));
        }
        if self.diagonal {
            walls.push((Mirror::DIAGONAL, |x, y| y < x));
        }
        walls
    }

    /// The whole symmetry group: the images of the fundamental domain.
    fn group(self) -> Vec<Mirror> {
        let gens: Vec<Mirror> = self.walls().iter().map(|w| w.0).collect();
        Mirror::generate(&gens)
    }
}

/// Integrate `f` over the cells of a rotated square lattice (side `step`,
/// rotation [`LATTICE_COS`]/[`LATTICE_SIN`]) covering the box `w × h`
/// centred on `center`; one source point per cell (see
/// [`CellGeometry::integrate`]).
///
/// With box symmetries the lattice is laid out in the fundamental domain
/// and mirrored, so the point set of a symmetric source is exactly
/// symmetric. A cell that a mirror line crosses is clipped to the domain,
/// each piece a point at its own intensity centroid: the pieces tile the
/// box without overlap, and no column of points sits on a symmetry axis
/// (joining the pieces across the axis would put one there).
///
/// A graded cell (varying intensity, lit throughout) holding more than
/// `1/`[`MIN_SOURCE_POINTS`] of the total is split `r × r`
/// (`r = ⌈√(share·MIN_SOURCE_POINTS)⌉`, 2…[`MAX_REFINE`], recursively):
/// the core of a peaked fill (a Gaussian) is sampled as finely as its
/// weight demands while its tails keep the coarse step. Uniform and edge
/// cells are never split, so flat fills (disks, annuli, poles) are
/// unaffected.
fn lattice_cells(
    f: &(dyn Fn(f64, f64) -> f64 + Sync),
    center: (f64, f64),
    (w, h): (f64, f64),
    step: f64,
    symmetry: Symmetry,
) -> Vec<SourcePoint> {
    let geo = CellGeometry {
        f,
        center,
        step,
        half_box: (0.5 * w, 0.5 * h),
        walls: symmetry.walls(),
    };
    // Cells [i, i+1) × [j, j+1) whose centre lies within a cell diagonal
    // of the box and of the fundamental domain.
    let margin = step * std::f64::consts::SQRT_2;
    let reach = w.hypot(h) * 0.5 + margin;
    let n = (reach / step).ceil() as i64 + 1;
    let mut cells = Vec::new();
    for j in -n..n {
        for i in -n..n {
            let (x, y) = geo.to_xy(i as f64 + 0.5, j as f64 + 0.5);
            let near_domain = (!symmetry.x || symmetry.diagonal || x > -margin)
                && (!symmetry.y || y > -margin)
                && (!symmetry.diagonal || y < x + margin);
            let near_box = x.abs() <= 0.5 * w + margin && y.abs() <= 0.5 * h + margin;
            if near_box && near_domain {
                cells.push((i as f64, j as f64));
            }
        }
    }
    let group = symmetry.group();
    let n_img = group.len();
    let coarse = map_range(cells.len() * n_img, |k| {
        geo.integrate(cells[k / n_img], 1.0, group[k % n_img])
    });
    let total: f64 = coarse.iter().flatten().map(|c| c.point.weight).sum();
    let target = total / MIN_SOURCE_POINTS;
    let refined = map_range(coarse.len(), |k| {
        let mut out = Vec::new();
        if let Some(c) = &coarse[k] {
            geo.refine(cells[k / n_img], 1.0, c, target, 0, &mut out);
        }
        out
    });
    refined.into_iter().flatten().collect()
}

/// One integrated cell of [`lattice_cells`].
struct CellPoint {
    point: SourcePoint,
    /// Lit throughout with varying intensity (a candidate for refinement).
    graded: bool,
    /// The mirror image of the fundamental-domain cell this point stands for.
    image: Mirror,
}

/// Geometry shared by the lattice cells of [`lattice_cells`].
struct CellGeometry<'a> {
    f: &'a (dyn Fn(f64, f64) -> f64 + Sync),
    center: (f64, f64),
    step: f64,
    half_box: (f64, f64),
    walls: Vec<Wall>,
}

impl CellGeometry<'_> {
    /// Lattice coordinates (u, v) → box-centred σ coordinates.
    fn to_xy(&self, u: f64, v: f64) -> (f64, f64) {
        (
            self.step * (LATTICE_COS * u - LATTICE_SIN * v),
            self.step * (LATTICE_SIN * u + LATTICE_COS * v),
        )
    }

    fn in_domain(&self, x: f64, y: f64) -> bool {
        x.abs() <= self.half_box.0
            && y.abs() <= self.half_box.1
            && self.walls.iter().all(|(_, inside)| inside(x, y))
    }

    /// Intensity at the image `g` of a fundamental-domain point.
    fn eval(&self, g: Mirror, x: f64, y: f64) -> f64 {
        let (gx, gy) = g.apply(x, y);
        let v = (self.f)(self.center.0 + gx, self.center.1 + gy);
        if v.is_finite() && v > 0.0 {
            v
        } else {
            0.0
        }
    }

    fn point(&self, g: Mirror, (x, y): (f64, f64), weight: f64) -> SourcePoint {
        let (gx, gy) = g.apply(x, y);
        SourcePoint {
            sx: self.center.0 + gx,
            sy: self.center.1 + gy,
            weight,
        }
    }

    /// Emit `cell`'s point, or — for a graded cell heavier than `target` —
    /// split the cell `(u0, v0) + [0, du)²` into `r × r` sub-cells
    /// (`r = ⌈√(weight/target)⌉`, 2…[`MAX_REFINE`]) and recurse (at most
    /// [`MAX_REFINE_DEPTH`] levels; a cell clipped by a mirror line may put
    /// most of its weight in a few sub-cells).
    fn refine(
        &self,
        (u0, v0): (f64, f64),
        du: f64,
        cell: &CellPoint,
        target: f64,
        depth: usize,
        out: &mut Vec<SourcePoint>,
    ) {
        if !(cell.graded && cell.point.weight > target) || depth >= MAX_REFINE_DEPTH {
            out.push(cell.point);
            return;
        }
        let r = ((cell.point.weight / target).sqrt().ceil() as usize).clamp(2, MAX_REFINE);
        let dc = du / r as f64;
        for b in 0..r {
            for a in 0..r {
                let corner = (u0 + a as f64 * dc, v0 + b as f64 * dc);
                if let Some(child) = self.integrate(corner, dc, cell.image) {
                    self.refine(corner, dc, &child, target, depth + 1, out);
                }
            }
        }
    }

    /// One source point for the image `g` of the lattice cell
    /// `[u0, u0+du) × [v0, v0+du)` (lattice units), clipped to the
    /// fundamental domain; weight = mean intensity × `du²`:
    /// - uniform (inside the domain, equal intensity at the four corners and
    ///   the centre): the centre;
    /// - inside the domain and lit at all [`SUBCELLS`]² sub-sample midpoints
    ///   (a graded fill): the centre with its own intensity — the plain
    ///   lattice midpoint rule, spectrally accurate for smooth fills (a
    ///   point at the cell's mass centroid would drop the intra-cell
    ///   variance, `h²/6` of the second moment);
    /// - otherwise (a hard source edge, a mirror line or the box edge cuts
    ///   it): the lit sub-samples' integrated intensity at their intensity
    ///   centroid, i.e. exact area weights to `1/SUBCELLS²` of a cell.
    ///
    /// `graded` marks a cell lit wherever it lies inside the domain, with
    /// varying intensity (a cell clipped by a mirror line counts, one
    /// crossed by a hard source edge does not).
    fn integrate(&self, (u0, v0): (f64, f64), du: f64, g: Mirror) -> Option<CellPoint> {
        let area = du * du;
        let centre = self.to_xy(u0 + 0.5 * du, v0 + 0.5 * du);
        let corners =
            [(0.0, 0.0), (du, 0.0), (0.0, du), (du, du)].map(|(a, b)| self.to_xy(u0 + a, v0 + b));
        let interior = corners.iter().all(|&(x, y)| self.in_domain(x, y));
        let vc = self.eval(g, centre.0, centre.1);
        if interior && corners.iter().all(|&(x, y)| self.eval(g, x, y) == vc) {
            return (vc > 0.0).then(|| CellPoint {
                point: self.point(g, centre, vc * area),
                graded: false,
                image: g,
            });
        }
        let sub = SUBCELLS as f64;
        let (mut mass, mut mx, mut my) = (0.0, 0.0, 0.0);
        let (mut inside, mut lit) = (0usize, 0usize);
        let (mut vmin, mut vmax) = (f64::INFINITY, 0.0f64);
        for k in 0..SUBCELLS * SUBCELLS {
            let (a, b) = ((k % SUBCELLS) as f64, (k / SUBCELLS) as f64);
            let (x, y) = self.to_xy(u0 + (a + 0.5) / sub * du, v0 + (b + 0.5) / sub * du);
            if !self.in_domain(x, y) {
                continue;
            }
            inside += 1;
            let v = self.eval(g, x, y);
            vmin = vmin.min(v);
            vmax = vmax.max(v);
            if v > 0.0 {
                mass += v;
                mx += v * x;
                my += v * y;
                lit += 1;
            }
        }
        if mass <= 0.0 {
            return None;
        }
        let graded = lit == inside && vmin < vmax;
        let point = if interior && lit == SUBCELLS * SUBCELLS && vc > 0.0 {
            self.point(g, centre, vc * area)
        } else {
            self.point(g, (mx / mass, my / mass), mass / (sub * sub) * area)
        };
        Some(CellPoint {
            point,
            graded,
            image: g,
        })
    }
}

/// Normalize the weights to 1, or fall back to one point at the probe
/// centroid when the support is thinner than the sampling grid.
fn normalized_or_centroid(
    mut points: Vec<SourcePoint>,
    cx: f64,
    cy: f64,
) -> Result<Vec<SourcePoint>> {
    let total: f64 = points.iter().map(|p| p.weight).sum();
    if points.is_empty() || total.is_nan() || total <= 0.0 {
        return Ok(vec![SourcePoint {
            sx: cx,
            sy: cy,
            weight: 1.0,
        }]);
    }
    for p in &mut points {
        p.weight /= total;
    }
    Ok(points)
}

/// Sample a source's pupil fill the way the engine does (see
/// [`ImagingSettings::source_points_per_axis`]). `df_sigma` is the mask
/// frequency step in σ units, `λ / (NA · field_nm)`; it only decides when
/// a tiny source collapses to a single coherent point.
///
/// 1. Probe `[-2, 2]²` on a 129² lattice (step 1/32 σ), then zoom (129²
///    lattices) onto the support bounding box until it spans ≥ 24 probe
///    cells per axis — so a σ_g = 0.005 Gaussian is found as reliably as a
///    σ = 0.9 disk.
/// 2. Default (`points_per_axis = None`): cover the (one-probe-cell-
///    expanded, sign-symmetrized) box with the cells of a square lattice of
///    side `h = min(0.04, √(area/200))` σ rotated by `θ = atan(1/φ)`, so no
///    row of points is parallel to an axis or a diagonal and a straight
///    edge (a diffraction order's pupil cutoff) passes the points one
///    mirror pair at a time. The lattice is laid out in the box's
///    fundamental domain (octant, quadrant, half or whole box) and
///    mirrored, so a symmetric fill gets an exactly symmetric point set;
///    cells cut by a mirror line are clipped, each piece its own point (no
///    column of points sits on a symmetry axis). Each cell is one point: its
///    midpoint with the local intensity if it is uniform or lit throughout
///    (graded fills — the midpoint rule is spectrally accurate there); the
///    intensity centroid with the integrated intensity, on 8 × 8
///    sub-samples, if a hard source edge or a mirror line cuts it (exact
///    area weights). Graded cells holding more than 1/200 of the total are
///    split (up to 8 × 8, three levels), so the core of a narrow Gaussian
///    is resolved while its tails stay coarse. Typical sizes: σ = 0.7 disk
///    ≈ 1180 points, σ = 1 ≈ 2250, annulus 0.5–0.9 ≈ 1310, two-pole
///    dipole (0.7/0.15) ≈ 260, small disks ≈ 290, Gaussian σ_g ≤ 0.05 ≈ 760.
///    `Some(n)`: a plain `n × n` midpoint grid over the box (sampled
///    intensity as weight, no edge weighting) — kept as the explicit,
///    reproducible reference grid.
///    Weights are normalized to 1.
/// 3. Sources smaller than `1e-3` of the mask-frequency step (`df_sigma`, in
///    σ units), or `Some(1)`, collapse to one point at the intensity
///    centroid (the coherent limit).
pub fn sample_source(
    source: &(impl LithographySource + ?Sized),
    points_per_axis: Option<usize>,
    df_sigma: f64,
) -> Result<Vec<SourcePoint>> {
    sample_intensity(&|x, y| source.intensity_at(x, y), points_per_axis, df_sigma)
}

// ---------------------------------------------------------------------------
// Kernel construction
// ---------------------------------------------------------------------------

/// Internal kernel set: values on the mask-frequency support only.
#[derive(Debug)]
struct SparseKernelSet {
    /// Flat row-major indices `i·n + j` of the support frequencies.
    support: Arc<Vec<usize>>,
    /// Distinct rows `i` present in `support` (for the pruned inverse FFT).
    nonzero_rows: Arc<Vec<usize>>,
    /// Raw (absolute) SOCS eigenvalues.
    eigenvalues: Vec<f64>,
    /// `values[k][t]`: kernel k at `support[t]`.
    values: Vec<Vec<Complex64>>,
    captured_energy_fraction: f64,
    /// Absolute clear-field intensity `TCC(0,0)`.
    clear_field: f64,
    /// Peak |P|² (summed over field columns) over all sampled pupil
    /// positions — the scale the dark-field criterion compares against.
    pupil_peak: f64,
    defocus_nm: f64,
    wavelength_nm: f64,
    diagnostics: KernelDiagnostics,
}

/// Signed FFT index → frequency multiple (`k` or `k − n`).
#[inline]
fn signed_index(k: usize, n: usize) -> f64 {
    if k < n / 2 {
        k as f64
    } else {
        k as f64 - n as f64
    }
}

/// Number of leading eigenpairs to keep: all numerically non-zero ones up to
/// `max_kernels`, fewer if `fraction < 1` is reached first. A cut never
/// splits a (near-)degenerate eigenspace — it would break the symmetry of
/// symmetric sources — so it moves to the cluster boundary: forward when the
/// energy criterion set the cut and the cluster fits under `max_kernels`,
/// backward otherwise (a leading cluster larger than `max_kernels` is kept
/// whole).
fn select_kernel_count(values: &[f64], trace: f64, max_kernels: usize, fraction: f64) -> usize {
    let lead = values.first().cloned().unwrap_or(0.0);
    if lead.is_nan() || lead <= 0.0 {
        return 0;
    }
    let tol = EIGEN_ZERO_REL * lead;
    let n_valid = values.iter().take_while(|&&v| v > tol).count();
    let cap = max_kernels.min(n_valid);
    let mut k = cap;
    if fraction < 1.0 {
        let mut acc = 0.0;
        for (i, &v) in values[..cap].iter().enumerate() {
            acc += v;
            if acc >= fraction * trace {
                k = i + 1;
                break;
            }
        }
    }
    let same = |a: f64, b: f64| (a - b).abs() <= DEGENERACY_REL * a.abs().max(b.abs());
    if k > 0 && k < n_valid && same(values[k - 1], values[k]) {
        let mut end = k;
        while end < n_valid && same(values[end - 1], values[end]) {
            end += 1;
        }
        if end <= max_kernels {
            k = end;
        } else {
            while k > 0 && same(values[k - 1], values[k]) {
                k -= 1;
            }
            if k == 0 {
                k = end;
            }
        }
    }
    k
}

struct KernelBuilder<'a> {
    optics: &'a dyn OpticalSystem,
    points: &'a [SourcePoint],
    grid: &'a GridConfig,
    settings: &'a ImagingSettings,
    /// Override the automatic eigensolver choice (tests cross-check paths).
    force_method: Option<DecompositionMethod>,
}

impl KernelBuilder<'_> {
    fn build(&self, defocus_nm: f64, wavelength_nm: f64) -> Result<SparseKernelSet> {
        let n = self.grid.size;
        let df = self.grid.freq_step();
        let cutoff = self.optics.cutoff_frequency(wavelength_nm);
        if !(cutoff.is_finite() && cutoff > 0.0) {
            return Err(LithographyError::InvalidParameter {
                name: "cutoff_frequency",
                value: cutoff,
                reason: "optics cutoff NA/lambda must be positive and finite",
            });
        }
        let na_eff = cutoff * wavelength_nm;
        let sigma_max = self
            .points
            .iter()
            .map(|p| p.sx.hypot(p.sy))
            .fold(0.0, f64::max);
        let radius = cutoff * (1.0 + sigma_max);
        let r2 = (radius / df).powi(2) * (1.0 + 1e-12);
        let support_exceeds_nyquist = radius > (n / 2) as f64 * df;

        // Candidate mask frequencies (flat index, f/f_c).
        let mut candidates: Vec<(usize, f64, f64)> = Vec::new();
        for i in 0..n {
            let ky = signed_index(i, n);
            if ky * ky > r2 {
                continue;
            }
            for j in 0..n {
                let kx = signed_index(j, n);
                if kx * kx + ky * ky <= r2 {
                    candidates.push((i * n + j, kx * df / cutoff, ky * df / cutoff));
                }
            }
        }
        let n_cand = candidates.len();
        let cols_per_point = match &self.settings.imaging_model {
            ImagingModel::Scalar => 1,
            ImagingModel::Vector(vs) => vector::columns_per_source_point(vs),
        };
        let n_pts = self.points.len();
        let n_cols = n_pts * cols_per_point;
        if n_cand == 0 || n_cols == 0 {
            return Err(LithographyError::NoDiffractionOrders);
        }
        let entry_bytes = std::mem::size_of::<Complex64>();
        if n_cand.saturating_mul(n_cols).saturating_mul(entry_bytes) > MAX_FACTOR_BYTES {
            return Err(LithographyError::PupilSamplingTooDense {
                samples: n_cand,
                max: MAX_FACTOR_BYTES / (entry_bytes * n_cols),
            });
        }

        // One A entry block: the `cols_per_point` columns of source point p
        // at candidate frequency (fxn, fyn).
        let model = &self.settings.imaging_model;
        let optics = self.optics;
        let eval = |fxn: f64, fyn: f64, p: &SourcePoint, out: &mut [Complex64]| {
            let px = fxn + p.sx;
            let py = fyn + p.sy;
            let pupil = optics.pupil_function(px, py, defocus_nm, wavelength_nm);
            let amp = p.weight.sqrt();
            match model {
                ImagingModel::Scalar => out[0] = pupil * amp,
                ImagingModel::Vector(vs) => {
                    if pupil == Complex64::new(0.0, 0.0) {
                        out.fill(Complex64::new(0.0, 0.0));
                    } else {
                        vector::field_columns(pupil, px, py, p.sx, p.sy, na_eff, vs, out);
                        out.iter_mut().for_each(|v| *v *= amp);
                    }
                }
            }
        };

        let max_k = self.settings.max_kernels;
        let fraction = self.settings.kernel_energy_fraction;
        let min_dim = n_cand.min(n_cols);
        let subspace = max_k.saturating_add(RANDOMIZED_OVERSAMPLE);
        let dense = min_dim <= DENSE_FAST_DIM
            || (min_dim <= DENSE_MAX_DIM && subspace.saturating_mul(3) >= min_dim);
        let method = self.force_method.unwrap_or(if !dense {
            DecompositionMethod::Randomized
        } else if n_cand <= n_cols {
            DecompositionMethod::DenseTcc
        } else {
            DecompositionMethod::DenseGram
        });

        let (support, eigenvalues, values, trace, clear_field, pupil_peak, n_cols_used) =
            match method {
                DecompositionMethod::DenseTcc => {
                    // Rows of conj(A): TCC = Gram of these vectors.
                    let mut rows = vec![Complex64::new(0.0, 0.0); n_cand * n_cols];
                    for_each_chunk_mut(&mut rows, n_cols, |r, row| {
                        let (_, fxn, fyn) = candidates[r];
                        for (pi, p) in self.points.iter().enumerate() {
                            let block = &mut row[pi * cols_per_point..(pi + 1) * cols_per_point];
                            eval(fxn, fyn, p, block);
                            block.iter_mut().for_each(|v| *v = v.conj());
                        }
                    });
                    // Clear field TCC(0,0): candidate 0 is the DC order.
                    let clear_field: f64 = rows[..n_cols].iter().map(|v| v.norm_sqr()).sum();
                    let pupil_peak = rows
                        .chunks_exact(cols_per_point)
                        .enumerate()
                        .map(|(b, block)| {
                            let w = self.points[b % n_pts].weight;
                            block.iter().map(|v| v.norm_sqr()).sum::<f64>() / w
                        })
                        .fold(0.0, f64::max);
                    // Drop all-zero columns (e.g. the TE/TM second state off axis)
                    // and mask frequencies no source point steers into the pupil.
                    let mut col_nonzero = vec![false; n_cols];
                    for row in rows.chunks_exact(n_cols) {
                        for (flag, v) in col_nonzero.iter_mut().zip(row) {
                            *flag |= v.re != 0.0 || v.im != 0.0;
                        }
                    }
                    let keep_cols: Vec<usize> = (0..n_cols).filter(|&c| col_nonzero[c]).collect();
                    let n_eff = keep_cols.len();
                    let mut kept = Vec::with_capacity(n_cand);
                    let mut compact = Vec::with_capacity(n_cand * n_eff);
                    for (r, row) in rows.chunks_exact(n_cols).enumerate() {
                        if keep_cols
                            .iter()
                            .any(|&c| row[c].re != 0.0 || row[c].im != 0.0)
                        {
                            compact.extend(keep_cols.iter().map(|&c| row[c]));
                            kept.push(candidates[r].0);
                        }
                    }
                    drop(rows);
                    let write = kept.len();
                    if write == 0 || n_eff == 0 {
                        return Err(LithographyError::NoDiffractionOrders);
                    }
                    let x = ColMatrix::from_col_major(n_eff, write, compact);
                    let trace = x.frobenius_norm_sq();
                    let eig = linalg::HermitianEigen::new(&x.gram());
                    let vals = eig.values();
                    let k = select_kernel_count(vals, trace, max_k, fraction);
                    let vecs = eig.vectors(k);
                    let values: Vec<Vec<Complex64>> = (0..k)
                        .map(|c| vecs.column(c).iter().cloned().collect())
                        .collect();
                    (
                        kept,
                        vals[..k].to_vec(),
                        values,
                        trace,
                        clear_field,
                        pupil_peak,
                        n_eff,
                    )
                }
                DecompositionMethod::DenseGram | DecompositionMethod::Randomized => {
                    // Columns of A, source point by source point.
                    let mut data = vec![Complex64::new(0.0, 0.0); n_cand * n_cols];
                    for_each_chunk_mut(&mut data, n_cand * cols_per_point, |pi, block| {
                        let p = &self.points[pi];
                        let mut buf = vec![Complex64::new(0.0, 0.0); cols_per_point];
                        for (r, &(_, fxn, fyn)) in candidates.iter().enumerate() {
                            eval(fxn, fyn, p, &mut buf);
                            for (c, v) in buf.iter().enumerate() {
                                block[c * n_cand + r] = *v;
                            }
                        }
                    });
                    // Clear field TCC(0,0) = Σ_cols |A[DC, col]|² (row 0 is DC).
                    let clear_field: f64 = data.iter().step_by(n_cand).map(|v| v.norm_sqr()).sum();
                    let point_len = cols_per_point * n_cand;
                    let pupil_peak = map_range(n_pts, |pi| {
                        let block = &data[pi * point_len..(pi + 1) * point_len];
                        let w = self.points[pi].weight;
                        (0..n_cand)
                            .map(|r| {
                                (0..cols_per_point)
                                    .map(|c| block[c * n_cand + r].norm_sqr())
                                    .sum::<f64>()
                                    / w
                            })
                            .fold(0.0, f64::max)
                    })
                    .into_iter()
                    .fold(0.0, f64::max);
                    // Drop all-zero columns (e.g. the TE/TM second state off axis).
                    let mut n_eff = 0;
                    for c in 0..n_cols {
                        let nonzero = data[c * n_cand..(c + 1) * n_cand]
                            .iter()
                            .any(|v| v.re != 0.0 || v.im != 0.0);
                        if nonzero {
                            if n_eff != c {
                                data.copy_within(c * n_cand..(c + 1) * n_cand, n_eff * n_cand);
                            }
                            n_eff += 1;
                        }
                    }
                    if n_eff == 0 {
                        return Err(LithographyError::NoDiffractionOrders);
                    }
                    data.truncate(n_eff * n_cand);
                    let a = ColMatrix::from_col_major(n_cand, n_eff, data);
                    let trace = a.frobenius_norm_sq();
                    let support: Vec<usize> = candidates.iter().map(|c| c.0).collect();
                    if method == DecompositionMethod::DenseGram {
                        let eig = linalg::HermitianEigen::new(&a.gram());
                        let vals = eig.values();
                        let k = select_kernel_count(vals, trace, max_k, fraction);
                        let v = eig.vectors(k);
                        let b =
                            nalgebra::DMatrix::from_fn(n_eff, k, |r, c| v[(r, c)] / vals[c].sqrt());
                        let u = a.mul(&b);
                        let values = (0..k).map(|c| u.col(c).to_vec()).collect();
                        (
                            support,
                            vals[..k].to_vec(),
                            values,
                            trace,
                            clear_field,
                            pupil_peak,
                            n_eff,
                        )
                    } else {
                        let l = subspace.min(n_cand.min(n_eff));
                        let (vals, u) = linalg::randomized_eigen(
                            &a,
                            l,
                            RANDOMIZED_POWER_ITERS,
                            RANDOMIZED_SEED,
                        );
                        let k = select_kernel_count(&vals, trace, max_k.min(l), fraction);
                        let values = (0..k).map(|c| u.col(c).to_vec()).collect();
                        (
                            support,
                            vals[..k].to_vec(),
                            values,
                            trace,
                            clear_field,
                            pupil_peak,
                            n_eff,
                        )
                    }
                }
            };

        if eigenvalues.is_empty() || trace.is_nan() || trace <= 0.0 {
            return Err(LithographyError::NoDiffractionOrders);
        }
        let captured = eigenvalues.iter().sum::<f64>() / trace;
        let mut rows_present: Vec<usize> = support.iter().map(|&idx| idx / n).collect();
        rows_present.dedup();
        let diagnostics = KernelDiagnostics {
            num_kernels: eigenvalues.len(),
            captured_energy_fraction: captured,
            num_frequencies: support.len(),
            num_columns: n_cols_used,
            method,
            support_exceeds_nyquist,
            clear_field_intensity: clear_field,
            normalized: false,
        };
        Ok(SparseKernelSet {
            support: Arc::new(support),
            nonzero_rows: Arc::new(rows_present),
            eigenvalues,
            values,
            captured_energy_fraction: captured,
            clear_field,
            pupil_peak,
            defocus_nm,
            wavelength_nm,
            diagnostics,
        })
    }
}

/// Legacy [`DefocusModel::KernelPhase`]: multiply in-focus kernels by the
/// on-axis paraxial phase `π z λ |f|²` of each mask frequency.
fn apply_kernel_phase(
    base: &SparseKernelSet,
    grid: &GridConfig,
    defocus_nm: f64,
) -> SparseKernelSet {
    let n = grid.size;
    let df = grid.freq_step();
    let lambda = base.wavelength_nm;
    let phases: Vec<Complex64> = base
        .support
        .iter()
        .map(|&idx| {
            let fy = signed_index(idx / n, n) * df;
            let fx = signed_index(idx % n, n) * df;
            let phi = std::f64::consts::PI * defocus_nm * lambda * (fx * fx + fy * fy);
            Complex64::from_polar(1.0, phi)
        })
        .collect();
    let values = base
        .values
        .iter()
        .map(|kv| kv.iter().zip(&phases).map(|(v, p)| v * p).collect())
        .collect();
    SparseKernelSet {
        support: Arc::clone(&base.support),
        nonzero_rows: Arc::clone(&base.nonzero_rows),
        eigenvalues: base.eigenvalues.clone(),
        values,
        captured_energy_fraction: base.captured_energy_fraction,
        clear_field: base.clear_field,
        pupil_peak: base.pupil_peak,
        defocus_nm,
        wavelength_nm: lambda,
        diagnostics: base.diagnostics,
    }
}

// ---------------------------------------------------------------------------
// Kernel cache
// ---------------------------------------------------------------------------

type CacheKey = (u64, u64);

fn cache_key(defocus_nm: f64, wavelength_nm: f64) -> CacheKey {
    // +0.0 and -0.0 are the same focus plane.
    let z = if defocus_nm == 0.0 { 0.0 } else { defocus_nm };
    (z.to_bits(), wavelength_nm.to_bits())
}

struct CacheEntry {
    key: CacheKey,
    set: Arc<SparseKernelSet>,
    dense: Option<Arc<KernelSet>>,
    last_used: u64,
}

/// Bounded LRU cache of kernel sets keyed by (defocus, wavelength). Builds
/// happen outside the lock: two threads racing on a new key may both build
/// it (the first insert wins); nobody ever blocks on another thread's build,
/// which keeps nested Rayon parallelism deadlock-free.
struct KernelCache {
    capacity: usize,
    state: Mutex<(Vec<CacheEntry>, u64)>,
}

impl KernelCache {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            state: Mutex::new((Vec::new(), 0)),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, (Vec<CacheEntry>, u64)> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn get(&self, key: CacheKey) -> Option<Arc<SparseKernelSet>> {
        let mut guard = self.lock();
        let (entries, clock) = &mut *guard;
        *clock += 1;
        let now = *clock;
        entries.iter_mut().find(|e| e.key == key).map(|e| {
            e.last_used = now;
            Arc::clone(&e.set)
        })
    }

    fn get_dense(&self, key: CacheKey) -> Option<Arc<KernelSet>> {
        let mut guard = self.lock();
        let (entries, clock) = &mut *guard;
        *clock += 1;
        let now = *clock;
        entries.iter_mut().find(|e| e.key == key).and_then(|e| {
            e.last_used = now;
            e.dense.clone()
        })
    }

    /// Insert (or keep the existing) entry; optionally attach a dense copy.
    fn insert(
        &self,
        key: CacheKey,
        set: Arc<SparseKernelSet>,
        dense: Option<Arc<KernelSet>>,
    ) -> Arc<SparseKernelSet> {
        if self.capacity == 0 {
            return set;
        }
        let mut guard = self.lock();
        let (entries, clock) = &mut *guard;
        *clock += 1;
        let now = *clock;
        if let Some(e) = entries.iter_mut().find(|e| e.key == key) {
            e.last_used = now;
            if dense.is_some() {
                e.dense = dense;
            }
            return Arc::clone(&e.set);
        }
        if entries.len() >= self.capacity {
            if let Some(oldest) = entries
                .iter()
                .enumerate()
                .min_by_key(|(_, e)| e.last_used)
                .map(|(i, _)| i)
            {
                entries.swap_remove(oldest);
            }
        }
        entries.push(CacheEntry {
            key,
            set: Arc::clone(&set),
            dense,
            last_used: now,
        });
        set
    }

    fn len(&self) -> usize {
        self.lock().0.len()
    }
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Aerial image computation engine using Hopkins partially-coherent imaging
/// with an exactly factorized TCC and SOCS kernels (see the module docs).
pub struct AerialImageEngine {
    grid: GridConfig,
    settings: ImagingSettings,
    wavelength_nm: f64,
    na: f64,
    flare_fraction: f64,
    optics: Box<dyn OpticalSystem>,
    source_points: Vec<SourcePoint>,
    spectral: Vec<(f64, f64)>,
    /// In-focus kernel set at the center wavelength (built at construction).
    focus_set: Arc<SparseKernelSet>,
    cache: KernelCache,
    fft: Fft2D,
}

impl AerialImageEngine {
    /// Create an engine with default [`ImagingSettings`] and at most
    /// `max_kernels` SOCS kernels per kernel set.
    ///
    /// This is the expensive initialization step (source sampling and the
    /// in-focus kernel set). Once created, the engine computes aerial images
    /// for different masks and focus planes; kernel sets for new focus
    /// planes are built on demand and cached.
    pub fn new(
        source: &(impl LithographySource + ?Sized),
        optics: &(impl OpticalSystem + ?Sized),
        grid: GridConfig,
        max_kernels: usize,
    ) -> Result<Self> {
        Self::with_settings(
            source,
            optics,
            grid,
            ImagingSettings {
                max_kernels,
                ..ImagingSettings::default()
            },
        )
    }

    /// Create an engine with explicit [`ImagingSettings`].
    ///
    /// For [`ImagingModel::Vector`], `VectorSettings::reduction` is replaced
    /// by `optics.reduction()`, `image_index` inherits
    /// `optics.immersion_index()` when left at its default 1.0 (any other
    /// value must equal it — there is one image-space medium, used by both
    /// the defocus phase and the vector fields), and the settings are
    /// validated against `optics.na()`; [`Self::settings`] reports the
    /// adjusted values.
    pub fn with_settings(
        source: &(impl LithographySource + ?Sized),
        optics: &(impl OpticalSystem + ?Sized),
        grid: GridConfig,
        settings: ImagingSettings,
    ) -> Result<Self> {
        let mut settings = settings;
        if settings.max_kernels == 0 {
            return Err(LithographyError::InvalidParameter {
                name: "max_kernels",
                value: 0.0,
                reason: "must be at least 1",
            });
        }
        let frac = settings.kernel_energy_fraction;
        if !(frac > 0.0 && frac <= 1.0) {
            return Err(LithographyError::InvalidParameter {
                name: "kernel_energy_fraction",
                value: frac,
                reason: "must be in (0, 1]",
            });
        }
        if settings.source_points_per_axis == Some(0) {
            return Err(LithographyError::InvalidParameter {
                name: "source_points_per_axis",
                value: 0.0,
                reason: "must be at least 1 (or None for adaptive sampling)",
            });
        }
        if grid.size < 2 || !(grid.pixel_nm.is_finite() && grid.pixel_nm > 0.0) {
            return Err(LithographyError::InvalidParameter {
                name: "grid",
                value: grid.pixel_nm,
                reason: "grid needs size >= 2 and a positive finite pixel_nm",
            });
        }
        let wavelength_nm = source.wavelength_nm();
        if !(wavelength_nm.is_finite() && wavelength_nm > 0.0) {
            return Err(LithographyError::InvalidParameter {
                name: "wavelength_nm",
                value: wavelength_nm,
                reason: "must be positive and finite",
            });
        }
        let optics_box = optics.clone_box();
        let (na, medium) = (optics_box.na(), optics_box.immersion_index());
        if !(medium.is_finite() && medium >= 1.0 && na > 0.0 && na < medium) {
            return Err(LithographyError::InvalidParameter {
                name: "numerical_aperture",
                value: na,
                reason: "must be positive and below the image-space (immersion) index",
            });
        }
        if let ImagingModel::Vector(vs) = &mut settings.imaging_model {
            // One image-space medium: the optics' defocus phase and the vector
            // fields must use the same index.
            if vs.image_index == 1.0 {
                vs.image_index = medium;
            } else if vs.image_index != medium {
                return Err(LithographyError::InvalidParameter {
                    name: "image_index",
                    value: vs.image_index,
                    reason: "must equal the optics' immersion index (set the medium on the \
                             optics, e.g. ProjectionOptics::immersion) or be left at 1.0 to \
                             inherit it",
                });
            }
            vs.reduction = optics_box.reduction();
            vs.validate(na)?;
        }
        let cutoff = optics_box.cutoff_frequency(wavelength_nm);
        if !(cutoff.is_finite() && cutoff > 0.0) {
            return Err(LithographyError::InvalidParameter {
                name: "cutoff_frequency",
                value: cutoff,
                reason: "optics cutoff NA/lambda must be positive and finite",
            });
        }
        optics_box.prepare(wavelength_nm)?;
        let df_sigma = grid.freq_step() / cutoff;
        let source_points = sample_source(source, settings.source_points_per_axis, df_sigma)?;

        let mut spectral: Vec<(f64, f64)> = source
            .spectral_weights()
            .into_iter()
            .filter(|(wl, w)| wl.is_finite() && *wl > 0.0 && w.is_finite() && *w > 0.0)
            .collect();
        let total: f64 = spectral.iter().map(|(_, w)| w).sum();
        if spectral.is_empty() || total.is_nan() || total <= 0.0 {
            spectral = vec![(wavelength_nm, 1.0)];
        } else {
            spectral.iter_mut().for_each(|(_, w)| *w /= total);
        }

        let focus_set = KernelBuilder {
            optics: optics_box.as_ref(),
            points: &source_points,
            grid: &grid,
            settings: &settings,
            force_method: None,
        }
        .build(0.0, wavelength_nm)?;

        Ok(Self {
            na: optics_box.na(),
            flare_fraction: optics_box.flare_fraction(),
            cache: KernelCache::new(settings.kernel_cache_capacity),
            grid,
            settings,
            wavelength_nm,
            optics: optics_box,
            source_points,
            spectral,
            focus_set: Arc::new(focus_set),
            fft: Fft2D::new(),
        })
    }

    fn builder(&self) -> KernelBuilder<'_> {
        KernelBuilder {
            optics: self.optics.as_ref(),
            points: &self.source_points,
            grid: &self.grid,
            settings: &self.settings,
            force_method: None,
        }
    }

    /// Kernel set for (defocus, wavelength): in-focus set, cache, or build.
    fn kernel_set(&self, defocus_nm: f64, wavelength_nm: f64) -> Result<Arc<SparseKernelSet>> {
        let key = cache_key(defocus_nm, wavelength_nm);
        if key == cache_key(0.0, self.wavelength_nm) {
            return Ok(Arc::clone(&self.focus_set));
        }
        if let Some(set) = self.cache.get(key) {
            return Ok(set);
        }
        let set = if defocus_nm == 0.0 || self.settings.defocus_model == DefocusModel::Exact {
            self.optics.prepare(wavelength_nm)?;
            self.builder().build(defocus_nm, wavelength_nm)?
        } else {
            let base = self.kernel_set(0.0, wavelength_nm)?;
            apply_kernel_phase(&base, &self.grid, defocus_nm)
        };
        Ok(self.cache.insert(key, Arc::new(set), None))
    }

    /// Kernel set at the center wavelength; kernel rebuilds at a new focus
    /// plane reuse the source/support sizes that already succeeded at
    /// construction, so failure here is an internal error.
    fn center_set(&self, defocus_nm: f64) -> Arc<SparseKernelSet> {
        self.kernel_set(defocus_nm, self.wavelength_nm)
            .unwrap_or_else(|e| panic!("kernel rebuild at defocus {defocus_nm} nm failed: {e}"))
    }

    /// Σ_k λ_k |IFFT(u_k ⊙ M)|² (no flare).
    ///
    /// Kernels are split into a fixed number of contiguous groups (a function
    /// of the kernel count and grid size only), each summed sequentially in
    /// parallel, and the group images are added in index order — so results
    /// are bit-reproducible regardless of thread scheduling.
    fn socs_intensity(&self, set: &SparseKernelSet, spectrum: &Array2<Complex64>) -> Array2<f64> {
        let n = self.grid.size;
        let m_s: Vec<Complex64> = set
            .support
            .iter()
            .map(|&idx| spectrum[[idx / n, idx % n]])
            .collect();
        let nk = set.values.len();
        let bytes_per_group =
            n * n * (std::mem::size_of::<f64>() + std::mem::size_of::<Complex64>());
        let max_groups = (SOCS_WORKSPACE_BYTES / bytes_per_group.max(1)).clamp(1, SOCS_GROUPS);
        let per_group = nk.div_ceil(nk.clamp(1, max_groups)).max(1);
        let groups = nk.div_ceil(per_group);
        let fft = &self.fft;
        let partials = map_range(groups, |g| {
            let mut acc = Array2::<f64>::zeros((n, n));
            let mut buf = Array2::<Complex64>::zeros((n, n));
            for k in g * per_group..((g + 1) * per_group).min(nk) {
                buf.fill(Complex64::new(0.0, 0.0));
                {
                    let b = buf.as_slice_mut().expect("fresh array is contiguous");
                    for ((&idx, v), m) in set.support.iter().zip(&set.values[k]).zip(&m_s) {
                        b[idx] = v * m;
                    }
                }
                fft.inverse_pruned_rows(&mut buf, &set.nonzero_rows);
                let lambda = set.eigenvalues[k];
                Zip::from(&mut acc)
                    .and(&buf)
                    .for_each(|a, v| *a += lambda * v.norm_sqr());
            }
            acc
        });
        let mut iter = partials.into_iter();
        let mut total = iter.next().unwrap_or_else(|| Array2::zeros((n, n)));
        for p in iter {
            total += &p;
        }
        total
    }

    /// Divisor that turns an absolute image into the configured output:
    /// the clear-field intensity in `ClearField` mode unless the
    /// configuration is dark field (`clear_field < 1 % · pupil_peak`),
    /// `None` (no scaling) otherwise.
    fn normalization_divisor(&self, clear_field: f64, pupil_peak: f64) -> Option<f64> {
        let bright = clear_field > 0.0 && clear_field >= DARK_FIELD_FRACTION * pupil_peak;
        match self.settings.normalization {
            ImageNormalization::ClearField if bright => Some(clear_field),
            _ => None,
        }
    }

    fn normalize(&self, image: &mut Array2<f64>, clear_field: f64, pupil_peak: f64) {
        if let Some(c) = self.normalization_divisor(clear_field, pupil_peak) {
            image.mapv_inplace(|v| v / c);
        }
    }

    fn apply_flare(&self, image: &mut Array2<f64>) {
        if self.flare_fraction > 0.0 {
            let mean = image.iter().sum::<f64>() / image.len() as f64;
            let flare = self.flare_fraction * mean;
            let keep = 1.0 - self.flare_fraction;
            image.mapv_inplace(|v| v * keep + flare);
        }
    }

    fn wrap(&self, data: Array2<f64>) -> Grid2D<f64> {
        let half = self.grid.field_size_nm() / 2.0;
        Grid2D {
            data,
            x_min_nm: -half,
            x_max_nm: half,
            y_min_nm: -half,
            y_max_nm: half,
        }
    }

    fn check_dims(&self, a: &Array2<Complex64>, what: &str) {
        let n = self.grid.size;
        assert_eq!(
            a.dim(),
            (n, n),
            "{what} must match the engine grid ({n}x{n})"
        );
    }

    /// Compute the aerial image for a given mask and defocus.
    ///
    /// The mask spectrum comes from [`Mask::spectrum`]; the image is
    /// `Σ_k λ_k |IFFT(K_k(z) · M)|²` with the kernel set of focus plane `z`
    /// (see [`Self::kernels`]), followed by flare.
    pub fn compute(&self, mask: &Mask, defocus_nm: f64) -> Grid2D<f64> {
        let spectrum = mask.spectrum(&self.grid, &self.fft);
        self.compute_from_spectrum(&spectrum, defocus_nm)
    }

    /// Compute the aerial image from a raw complex transmittance map in
    /// the space domain (same grid as the engine).
    ///
    /// This is the entry point for continuous-transmittance patterns that
    /// are not geometric [`Mask`] features — e.g. grayscale lithography
    /// masks or externally generated transmittance maps.
    ///
    /// # Panics
    /// Panics if `transmittance` does not match the engine grid size.
    pub fn compute_from_transmittance(
        &self,
        transmittance: &Array2<Complex64>,
        defocus_nm: f64,
    ) -> Grid2D<f64> {
        self.check_dims(transmittance, "transmittance map");
        let mut spectrum = transmittance.clone();
        self.fft.forward(&mut spectrum);
        self.compute_from_spectrum(&spectrum, defocus_nm)
    }

    /// Compute the aerial image from a mask spectrum (`Fft2D::forward`
    /// layout and scale, n×n).
    ///
    /// # Panics
    /// Panics if `spectrum` does not match the engine grid size.
    pub fn compute_from_spectrum(
        &self,
        spectrum: &Array2<Complex64>,
        defocus_nm: f64,
    ) -> Grid2D<f64> {
        self.check_dims(spectrum, "mask spectrum");
        let set = self.center_set(defocus_nm);
        let mut image = self.socs_intensity(&set, spectrum);
        self.normalize(&mut image, set.clear_field, set.pupil_peak);
        self.apply_flare(&mut image);
        self.wrap(image)
    }

    /// Aerial images at several focus planes (spectrum computed once, planes
    /// in parallel). Equivalent to calling [`Self::compute`] per plane.
    pub fn compute_through_focus(&self, mask: &Mask, defocus_nm: &[f64]) -> Vec<Grid2D<f64>> {
        let spectrum = mask.spectrum(&self.grid, &self.fft);
        map_range(defocus_nm.len(), |i| {
            self.compute_from_spectrum(&spectrum, defocus_nm[i])
        })
    }

    /// [`Self::compute_through_focus`] for a raw transmittance map.
    ///
    /// # Panics
    /// Panics if `transmittance` does not match the engine grid size.
    pub fn compute_through_focus_from_transmittance(
        &self,
        transmittance: &Array2<Complex64>,
        defocus_nm: &[f64],
    ) -> Vec<Grid2D<f64>> {
        self.check_dims(transmittance, "transmittance map");
        let mut spectrum = transmittance.clone();
        self.fft.forward(&mut spectrum);
        map_range(defocus_nm.len(), |i| {
            self.compute_from_spectrum(&spectrum, defocus_nm[i])
        })
    }

    /// Polychromatic image by the narrow-band focus-shift approximation:
    /// each spectral sample of `source` is imaged with the center-wavelength
    /// kernels at defocus `z + optics.chromatic_defocus(Δλ)`, and the images
    /// are summed with the spectral weights.
    ///
    /// Valid only for Δλ/λ ≪ 1 (excimer, FEL, monochromatized beamlines);
    /// for broad or multi-line spectra use [`Self::compute_multiwavelength`],
    /// which rebuilds the TCC at every wavelength.
    pub fn compute_polychromatic(
        &self,
        mask: &Mask,
        defocus_nm: f64,
        source: &(impl LithographySource + ?Sized),
        optics: &(impl OpticalSystem + ?Sized),
    ) -> Grid2D<f64> {
        let spectral_weights = source.spectral_weights();
        if spectral_weights.len() <= 1 {
            return self.compute(mask, defocus_nm);
        }
        let spectrum = mask.spectrum(&self.grid, &self.fft);
        let n = self.grid.size;
        let mut total = Array2::zeros((n, n));
        let (mut clear, mut peak) = (0.0, 0.0f64);
        for (wl, weight) in &spectral_weights {
            let delta_pm = (wl - source.wavelength_nm()) * 1000.0;
            let z = defocus_nm + optics.chromatic_defocus(delta_pm);
            let set = self.center_set(z);
            total.scaled_add(*weight, &self.socs_intensity(&set, &spectrum));
            clear += weight * set.clear_field;
            peak = peak.max(set.pupil_peak);
        }
        let weight_sum: f64 = spectral_weights.iter().map(|(_, w)| w).sum();
        self.normalize(&mut total, clear / weight_sum, peak);
        if weight_sum > 0.0 {
            total.mapv_inplace(|v| v / weight_sum);
        }
        self.apply_flare(&mut total);
        self.wrap(total)
    }

    /// Exact incoherent per-wavelength sum: at every spectral sample `λ_i`
    /// of the source (weights `w_i`) the kernels are rebuilt with the
    /// cutoff `NA/λ_i`, source points at `σ·NA/λ_i`, the pupil evaluated at
    /// `λ_i`, and focus `z + optics.chromatic_defocus(λ_i − λ_0)`;
    /// `I = Σ_i w_i I_i` (relative mode: divided by `Σ_i w_i I_clear,i`, so
    /// wavelengths that transmit more weigh more), then flare. Kernel sets
    /// are cached per (z, λ).
    pub fn compute_multiwavelength(&self, mask: &Mask, defocus_nm: f64) -> Result<Grid2D<f64>> {
        let spectrum = mask.spectrum(&self.grid, &self.fft);
        let n = self.grid.size;
        let mut total = Array2::zeros((n, n));
        let (mut clear, mut peak) = (0.0, 0.0f64);
        for &(wl, weight) in &self.spectral {
            let delta_pm = (wl - self.wavelength_nm) * 1000.0;
            let z = defocus_nm + self.optics.chromatic_defocus(delta_pm);
            let set = self.kernel_set(z, wl)?;
            total.scaled_add(weight, &self.socs_intensity(&set, &spectrum));
            clear += weight * set.clear_field;
            peak = peak.max(set.pupil_peak);
        }
        // Relative intensity of the whole spectrum: Σ w_i I_i / Σ w_i I_clear,i.
        self.normalize(&mut total, clear, peak);
        self.apply_flare(&mut total);
        Ok(self.wrap(total))
    }

    /// SOCS kernel set (dense n×n kernels) for focus plane `defocus_nm` at
    /// the center wavelength. Cached; see [`KernelSet`] for the invariant.
    pub fn kernels(&self, defocus_nm: f64) -> Arc<KernelSet> {
        let key = cache_key(defocus_nm, self.wavelength_nm);
        if let Some(dense) = self.cache.get_dense(key) {
            return dense;
        }
        let set = self.center_set(defocus_nm);
        let n = self.grid.size;
        let kernels = set
            .values
            .iter()
            .map(|kv| {
                let mut k = Array2::zeros((n, n));
                for (&idx, v) in set.support.iter().zip(kv) {
                    k[[idx / n, idx % n]] = *v;
                }
                k
            })
            .collect();
        let divisor = self
            .normalization_divisor(set.clear_field, set.pupil_peak)
            .unwrap_or(1.0);
        let dense = Arc::new(KernelSet {
            eigenvalues: set.eigenvalues.iter().map(|l| l / divisor).collect(),
            kernels,
            defocus_nm: set.defocus_nm,
            wavelength_nm: set.wavelength_nm,
            captured_energy_fraction: set.captured_energy_fraction,
        });
        self.cache.insert(key, set, Some(Arc::clone(&dense)));
        dense
    }

    /// Size/accuracy bookkeeping of the kernel set at (`defocus_nm`,
    /// `wavelength_nm`) — builds (and caches) it if needed.
    pub fn kernel_diagnostics(
        &self,
        defocus_nm: f64,
        wavelength_nm: f64,
    ) -> Result<KernelDiagnostics> {
        let set = self.kernel_set(defocus_nm, wavelength_nm)?;
        Ok(KernelDiagnostics {
            normalized: self
                .normalization_divisor(set.clear_field, set.pupil_peak)
                .is_some(),
            ..set.diagnostics
        })
    }

    /// Absolute clear-field intensity `TCC(0,0)` at focus plane `defocus_nm`
    /// and the center wavelength, relative to the illumination incident on
    /// the mask: `Σ_s w_s |P(s)|²` (summed over field columns in vector
    /// mode). Default-mode images are divided by this value; multiply them
    /// by it to recover absolute intensity.
    pub fn clear_field_intensity(&self, defocus_nm: f64) -> f64 {
        self.center_set(defocus_nm).clear_field
    }

    /// Number of SOCS kernels in the in-focus kernel set.
    pub fn num_kernels(&self) -> usize {
        self.focus_set.values.len()
    }

    /// Captured TCC energy fraction of the in-focus kernel set.
    pub fn captured_energy_fraction(&self) -> f64 {
        self.focus_set.captured_energy_fraction
    }

    /// Grid configuration.
    pub fn grid(&self) -> &GridConfig {
        &self.grid
    }

    /// Engine settings.
    pub fn settings(&self) -> &ImagingSettings {
        &self.settings
    }

    /// Flare fraction applied after the SOCS sum.
    pub fn flare_fraction(&self) -> f64 {
        self.flare_fraction
    }

    /// Center wavelength in nm.
    pub fn wavelength_nm(&self) -> f64 {
        self.wavelength_nm
    }

    /// Image-side numerical aperture.
    pub fn na(&self) -> f64 {
        self.na
    }

    /// The engine's copy of the optics.
    pub fn optics(&self) -> &dyn OpticalSystem {
        self.optics.as_ref()
    }

    /// Discrete source points (σ units, weights summing to 1).
    pub fn source_points(&self) -> &[SourcePoint] {
        &self.source_points
    }

    /// Normalized spectral samples `(λ_nm, weight)` used by
    /// [`Self::compute_multiwavelength`].
    pub fn spectral_samples(&self) -> &[(f64, f64)] {
        &self.spectral
    }

    /// Number of kernel sets currently cached.
    pub fn cached_kernel_sets(&self) -> usize {
        self.cache.len()
    }

    /// Compute MNSL (Moiré Nanosphere Lithographic Reflection) emission pattern.
    ///
    /// Integrates MNSL simulation with the aerial image engine's existing
    /// capabilities, leveraging the engine's grid configuration and optical setup.
    pub fn compute_mnsl(&self, config: &MnslConfig) -> MnslResult {
        // Create MNSL engine with our grid configuration
        let engine = MnslEngine::new(config.clone(), self.grid.clone());

        // Compute the MNSL emission pattern
        engine.compute_emission()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optics::ProjectionOptics;
    use crate::source::{IlluminationShape, VuvSource};
    use approx::assert_relative_eq;

    fn make_test_engine(sigma: f64, na: f64) -> AerialImageEngine {
        let source = VuvSource {
            illumination: IlluminationShape::Conventional { sigma },
            ..VuvSource::f2_laser(sigma).unwrap()
        };
        let optics = ProjectionOptics::new(na).unwrap();
        let grid = GridConfig {
            size: 128,
            pixel_nm: 2.0,
        };
        AerialImageEngine::new(&source, &optics, grid, 20).unwrap()
    }

    fn contrast(img: &Array2<f64>) -> f64 {
        let max = img.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min = img.iter().cloned().fold(f64::INFINITY, f64::min);
        (max - min) / (max + min)
    }

    #[test]
    fn test_engine_creation() {
        let engine = make_test_engine(0.5, 0.75);
        assert!(engine.num_kernels() > 0);
        assert!(engine.num_kernels() <= 20);
        assert!(engine.captured_energy_fraction() > 0.9);
    }

    #[test]
    fn test_aerial_image_non_negative() {
        let engine = make_test_engine(0.5, 0.75);
        let mask = Mask::line_space(65.0, 180.0).unwrap();
        for z in [0.0, 150.0] {
            let image = engine.compute(&mask, z);
            for &v in image.data.iter() {
                assert!(v >= -1e-10, "Intensity must be non-negative, got {}", v);
            }
        }
    }

    #[test]
    fn test_symmetric_mask_symmetric_image() {
        let engine = make_test_engine(0.5, 0.75);
        // Centered rectangular feature should give symmetric image
        let mask = Mask {
            mask_type: crate::mask::MaskType::Binary,
            features: vec![crate::mask::MaskFeature::Rect {
                x: 0.0,
                y: 0.0,
                w: 80.0,
                h: 256.0,
            }],
            dark_field: true,
        };
        for z in [0.0, 120.0] {
            let image = engine.compute(&mask, z);
            let n = image.data.ncols();
            let center_row = image.data.nrows() / 2;
            // Left-right symmetry about the field center (pixel boundary n/2).
            for j in 0..n / 2 {
                let left = image.data[[center_row, j]];
                let right = image.data[[center_row, n - 1 - j]];
                assert_relative_eq!(left, right, epsilon = 1e-9);
            }
        }
    }

    #[test]
    fn test_compute_uses_mask_spectrum() {
        // compute() must image Mask::spectrum; compute_from_transmittance of
        // the inverse transform of that spectrum is the same image.
        let engine = make_test_engine(0.5, 0.75);
        let mask = Mask::line_space(65.0, 180.0).unwrap();
        let fft = Fft2D::new();
        let spectrum = mask.spectrum(engine.grid(), &fft);
        let mut t = spectrum.clone();
        fft.inverse(&mut t);
        let via_mask = engine.compute(&mask, 50.0);
        let via_map = engine.compute_from_transmittance(&t, 50.0);
        let via_spec = engine.compute_from_spectrum(&spectrum, 50.0);
        for ((a, b), c) in via_mask
            .data
            .iter()
            .zip(via_map.data.iter())
            .zip(via_spec.data.iter())
        {
            assert!((a - b).abs() < 1e-12);
            assert_eq!(a, c);
        }
    }

    #[test]
    fn test_compute_from_transmittance_close_to_raster() {
        // The raster path and the (possibly analytic) spectrum path image
        // the same geometry; they may differ only by edge sampling.
        let engine = make_test_engine(0.5, 0.75);
        let mask = Mask::line_space(64.0, 128.0).unwrap();
        let via_mask = engine.compute(&mask, 0.0);
        let via_raster = engine.compute_from_transmittance(&mask.rasterize(engine.grid()), 0.0);
        for (a, b) in via_mask.data.iter().zip(via_raster.data.iter()) {
            assert!((a - b).abs() < 0.05, "{a} vs {b}");
        }
    }

    #[test]
    fn test_pupil_sampling_guard_rejects_dense_pupil() {
        // Soft-X-ray wavelength on a fine, large grid: the factor A would
        // hold ~6.7e5 mask frequencies x ~1e3 source columns (>> 1 GiB).
        let source = VuvSource {
            wavelength_nm: 1.0,
            ..VuvSource::f2_laser(0.5).unwrap()
        };
        let optics = ProjectionOptics::new(0.3).unwrap();
        let grid = GridConfig {
            size: 1024,
            pixel_nm: 1.0,
        };
        let result = AerialImageEngine::new(&source, &optics, grid, 20);
        match result {
            Err(LithographyError::PupilSamplingTooDense { samples, max }) => {
                assert!(samples > max);
                assert!(samples > 600_000, "support size {samples}");
            }
            Err(e) => panic!("expected PupilSamplingTooDense, got {e}"),
            Ok(_) => panic!("expected PupilSamplingTooDense, got an engine"),
        }
    }

    #[test]
    fn test_defocus_reduces_contrast() {
        let engine = make_test_engine(0.5, 0.75);
        let mask = Mask::line_space(65.0, 180.0).unwrap();
        let c0 = contrast(&engine.compute(&mask, 0.0).data);
        let c1 = contrast(&engine.compute(&mask, 200.0).data);
        assert!(
            c0 > c1,
            "In-focus contrast {} should exceed defocused contrast {}",
            c0,
            c1
        );
    }

    #[test]
    fn test_settings_validation() {
        let source = VuvSource::f2_laser(0.5).unwrap();
        let optics = ProjectionOptics::new(0.75).unwrap();
        let grid = GridConfig {
            size: 32,
            pixel_nm: 4.0,
        };
        for bad in [
            ImagingSettings {
                max_kernels: 0,
                ..Default::default()
            },
            ImagingSettings {
                kernel_energy_fraction: 0.0,
                ..Default::default()
            },
            ImagingSettings {
                kernel_energy_fraction: f64::NAN,
                ..Default::default()
            },
            ImagingSettings {
                kernel_energy_fraction: 1.5,
                ..Default::default()
            },
            ImagingSettings {
                source_points_per_axis: Some(0),
                ..Default::default()
            },
        ] {
            assert!(AerialImageEngine::with_settings(&source, &optics, grid.clone(), bad).is_err());
        }
    }

    #[test]
    fn test_source_sampling_conventional_is_symmetric_and_normalized() {
        let src = VuvSource::f2_laser(0.6).unwrap();
        let pts = sample_source(&src, None, 0.5).unwrap();
        let wsum: f64 = pts.iter().map(|p| p.weight).sum();
        assert_relative_eq!(wsum, 1.0, epsilon = 1e-12);
        assert!(
            pts.len() > 500,
            "adaptive grid should be dense: {}",
            pts.len()
        );
        let smax = pts.iter().map(|p| p.sx.hypot(p.sy)).fold(0.0, f64::max);
        assert!(smax <= 0.6 && smax > 0.57, "sigma_max {smax}");
        // Point set is closed under x -> -x, y -> -y and x <-> y.
        let has = |x: f64, y: f64| pts.iter().any(|p| p.sx == x && p.sy == y);
        for p in &pts {
            assert!(has(-p.sx, p.sy) && has(p.sx, -p.sy) && has(p.sy, p.sx));
        }
    }

    #[test]
    fn test_source_sampling_small_gaussian_is_resolved() {
        let shape = IlluminationShape::CoherentGaussian { sigma: 0.02 };
        let f = |x: f64, y: f64| crate::source::evaluate_illumination(&shape, x, y);
        let pts = sample_intensity(&f, None, 0.5).unwrap();
        assert!(pts.len() >= 150, "{} points", pts.len());
        // Second moment of the sampled weights ≈ 2 σ² (2D Gaussian).
        let m2: f64 = pts
            .iter()
            .map(|p| p.weight * (p.sx * p.sx + p.sy * p.sy))
            .sum();
        assert_relative_eq!(m2, 2.0 * 0.02 * 0.02, max_relative = 0.01);
        let smax = pts.iter().map(|p| p.sx.hypot(p.sy)).fold(0.0, f64::max);
        assert!(smax < 0.2, "support should hug the Gaussian: {smax}");
    }

    #[test]
    fn test_source_sampling_coherent_limit_and_override() {
        // A tiny source (extent << mask frequency step) is one on-axis point.
        let tiny = |x: f64, y: f64| if x.hypot(y) <= 1e-7 { 1.0 } else { 0.0 };
        let pts = sample_intensity(&tiny, None, 0.5).unwrap();
        assert_eq!(pts.len(), 1);
        assert_eq!((pts[0].sx, pts[0].sy, pts[0].weight), (0.0, 0.0, 1.0));
        // Some(1) collapses any source to its centroid.
        let src = VuvSource::f2_laser(0.7).unwrap();
        let one = sample_source(&src, Some(1), 0.5).unwrap();
        assert_eq!(one.len(), 1);
        assert!(one[0].sx.abs() < 1e-12 && one[0].sy.abs() < 1e-12);
        // Some(n) puts n points across the support box.
        let pts = sample_source(&src, Some(11), 0.5).unwrap();
        let xs: std::collections::BTreeSet<u64> = pts.iter().map(|p| p.sx.to_bits()).collect();
        assert_eq!(xs.len(), 11);
    }

    #[test]
    fn test_source_sampling_dipole_and_empty() {
        let shape = IlluminationShape::Dipole {
            sigma_center: 0.7,
            sigma_radius: 0.08,
            orientation_deg: 0.0,
        };
        let f = |x: f64, y: f64| crate::source::evaluate_illumination(&shape, x, y);
        let pts = sample_intensity(&f, None, 0.5).unwrap();
        assert!(pts.len() >= 150, "{} points", pts.len());
        for p in &pts {
            assert!(((p.sx.abs() - 0.7).hypot(p.sy)) <= 0.08 + 1e-12);
        }
        let dark = |_: f64, _: f64| 0.0;
        assert!(sample_intensity(&dark, None, 0.5).is_err());
    }

    /// Every point's mirror images (x → −x, y → −y, x ↔ y) are in the set,
    /// bit for bit.
    fn assert_d4_closed(pts: &[SourcePoint]) {
        let has = |x: f64, y: f64| pts.iter().any(|p| p.sx == x && p.sy == y);
        for p in pts {
            assert!(
                has(-p.sx, p.sy) && has(p.sx, -p.sy) && has(p.sy, p.sx),
                "missing mirror image of ({}, {})",
                p.sx,
                p.sy
            );
        }
    }

    fn fill(shape: IlluminationShape) -> impl Fn(f64, f64) -> f64 + Sync {
        move |x, y| crate::source::evaluate_illumination(&shape, x, y)
    }

    #[test]
    fn test_source_sampling_symmetric_fills_are_exactly_symmetric() {
        for shape in [
            IlluminationShape::Annular {
                sigma_inner: 0.5,
                sigma_outer: 0.9,
            },
            IlluminationShape::Quadrupole {
                sigma_center: 0.7,
                sigma_radius: 0.2,
                opening_angle_deg: 30.0,
            },
            IlluminationShape::CoherentGaussian { sigma: 0.05 },
            IlluminationShape::Conventional { sigma: 0.05 },
        ] {
            let pts = sample_intensity(&fill(shape.clone()), None, 0.5).unwrap();
            assert_d4_closed(&pts);
            let wsum: f64 = pts.iter().map(|p| p.weight).sum();
            assert_relative_eq!(wsum, 1.0, epsilon = 1e-12);
        }
        // A dipole box is symmetric in x and y but not square: D2 only.
        let dipole = IlluminationShape::Dipole {
            sigma_center: 0.7,
            sigma_radius: 0.15,
            orientation_deg: 0.0,
        };
        let pts = sample_intensity(&fill(dipole), None, 0.5).unwrap();
        let has = |x: f64, y: f64| pts.iter().any(|p| p.sx == x && p.sy == y);
        for p in &pts {
            assert!(has(-p.sx, p.sy) && has(p.sx, -p.sy));
        }
    }

    /// Fraction of the sampled weight beyond a straight edge `x > a` (what
    /// a pupil cutoff crossing the source passes) against the exact value,
    /// over a fine sweep of `a`: the area-weighted, rotated-lattice points
    /// follow it to about one point's weight, without the column steps of
    /// an axis-aligned grid.
    #[test]
    fn test_source_sampling_tracks_a_moving_straight_edge() {
        let beyond = |pts: &[SourcePoint], a: f64| -> f64 {
            pts.iter().filter(|p| p.sx > a).map(|p| p.weight).sum()
        };
        // Disk σ = 0.75: circular-segment area fraction.
        let s = 0.75;
        let pts = sample_intensity(
            &fill(IlluminationShape::Conventional { sigma: s }),
            None,
            0.5,
        )
        .unwrap();
        let mut worst: f64 = 0.0;
        for k in 0..=60 {
            let a = 0.60 + 0.0025 * k as f64;
            let exact = (s * s * (a / s).acos() - a * (s * s - a * a).sqrt())
                / (std::f64::consts::PI * s * s);
            worst = worst.max((beyond(&pts, a) - exact).abs());
        }
        // (The axis-aligned grid used before WP-A3 was off by up to 0.0099
        // here and by 0.089 in the Gaussian sweep below.)
        assert!(worst < 2.5e-3, "disk: max error {worst}");
        // Gaussian σ_g = 0.05: tail fraction ½·erfc(a/(√2 σ_g)), by Simpson.
        let sg = 0.05;
        let pts = sample_intensity(
            &fill(IlluminationShape::CoherentGaussian { sigma: sg }),
            None,
            0.5,
        )
        .unwrap();
        let tail = |a: f64| -> f64 {
            let (b, n) = (a + 12.0 * sg, 2000);
            let h = (b - a) / n as f64;
            let g = |x: f64| (-x * x / (2.0 * sg * sg)).exp();
            let simpson: f64 = (0..=n)
                .map(|i| {
                    let c = if i == 0 || i == n {
                        1.0
                    } else if i % 2 == 1 {
                        4.0
                    } else {
                        2.0
                    };
                    c * g(a + i as f64 * h)
                })
                .sum::<f64>()
                * h
                / 3.0;
            simpson / ((2.0 * std::f64::consts::PI).sqrt() * sg)
        };
        let mut worst: f64 = 0.0;
        for k in 0..=60 {
            let a = -0.15 + 0.005 * k as f64;
            worst = worst.max((beyond(&pts, a) - tail(a)).abs());
        }
        assert!(worst < 1e-2, "Gaussian: max error {worst}");
    }

    #[test]
    fn test_source_sampling_bounds_point_weight_of_peaked_fills() {
        // Graded cells are split until no point carries much more than
        // 1/MIN_SOURCE_POINTS of a peaked fill.
        for sg in [0.02, 0.05, 0.3] {
            let pts = sample_intensity(
                &fill(IlluminationShape::CoherentGaussian { sigma: sg }),
                None,
                0.5,
            )
            .unwrap();
            let wmax = pts.iter().map(|p| p.weight).fold(0.0, f64::max);
            assert!(
                wmax <= 1.0 / MIN_SOURCE_POINTS,
                "σ_g {sg}: max weight {wmax}"
            );
            // Second moment of the fill (a 2-D Gaussian truncated at ρ = 1):
            // 2σ²·[1 − u·e^(−u)/(1 − e^(−u))], u = 1/(2σ²).
            let u = 1.0 / (2.0 * sg * sg);
            let exact = 2.0 * sg * sg * (1.0 - u * (-u).exp() / (1.0 - (-u).exp()));
            let m2: f64 = pts
                .iter()
                .map(|p| p.weight * (p.sx * p.sx + p.sy * p.sy))
                .sum();
            assert_relative_eq!(m2, exact, max_relative = 0.01);
        }
        // Flat fills are not refined: about MIN_SOURCE_POINTS cells (plus
        // the pieces of cells clipped at the symmetry axes) for a small
        // disk, and the exact second moment σ²/2 from the edge weights.
        let pts = sample_intensity(
            &fill(IlluminationShape::Conventional { sigma: 0.05 }),
            None,
            0.5,
        )
        .unwrap();
        assert!((180..=320).contains(&pts.len()), "{} points", pts.len());
        let m2: f64 = pts
            .iter()
            .map(|p| p.weight * (p.sx * p.sx + p.sy * p.sy))
            .sum();
        assert_relative_eq!(m2, 0.05 * 0.05 / 2.0, max_relative = 0.01);
    }

    #[test]
    fn test_frequency_support_extends_to_one_plus_sigma() {
        // Pitch 160 nm at λ=100 nm, NA=0.5: first order at 1.25 NA/λ lies
        // outside the coherent cutoff but inside (1+σ)NA/λ for σ=0.7.
        let source = VuvSource {
            wavelength_nm: 100.0,
            spectral_samples: 1,
            illumination: IlluminationShape::Conventional { sigma: 0.7 },
            ..VuvSource::f2_laser(0.7).unwrap()
        };
        let optics = ProjectionOptics::new(0.5).unwrap();
        let grid = GridConfig {
            size: 64,
            pixel_nm: 10.0,
        };
        let engine = AerialImageEngine::new(&source, &optics, grid, 40).unwrap();
        let ks = engine.kernels(0.0);
        let n = 64;
        let k_first = 4; // field 640 nm / pitch 160 nm
        let mut has_first_order = false;
        for k in &ks.kernels {
            if k[[0, k_first]].norm() > 1e-6 || k[[0, n - k_first]].norm() > 1e-6 {
                has_first_order = true;
            }
        }
        assert!(has_first_order);
    }

    #[test]
    fn test_kernel_cache_is_bounded_and_reused() {
        let source = VuvSource::f2_laser(0.5).unwrap();
        let optics = ProjectionOptics::new(0.75).unwrap();
        let grid = GridConfig {
            size: 32,
            pixel_nm: 4.0,
        };
        let settings = ImagingSettings {
            kernel_cache_capacity: 3,
            ..Default::default()
        };
        let engine = AerialImageEngine::with_settings(&source, &optics, grid, settings).unwrap();
        let mask = Mask::line_space(40.0, 64.0).unwrap();
        for z in [10.0, 20.0, 30.0, 40.0, 50.0] {
            engine.compute(&mask, z);
        }
        assert_eq!(engine.cached_kernel_sets(), 3);
        // Re-requesting a cached plane returns the identical kernel set.
        let a = engine.kernels(50.0);
        let b = engine.kernels(50.0);
        assert!(Arc::ptr_eq(&a, &b));
        // Capacity 0 disables caching but still works.
        let settings = ImagingSettings {
            kernel_cache_capacity: 0,
            ..Default::default()
        };
        let grid = GridConfig {
            size: 32,
            pixel_nm: 4.0,
        };
        let engine = AerialImageEngine::with_settings(&source, &optics, grid, settings).unwrap();
        engine.compute(&mask, 25.0);
        assert_eq!(engine.cached_kernel_sets(), 0);
    }

    #[test]
    fn test_kernel_phase_model_matches_legacy_formula() {
        let source = VuvSource::f2_laser(0.5).unwrap();
        let optics = ProjectionOptics::new(0.75).unwrap();
        let grid = GridConfig {
            size: 32,
            pixel_nm: 4.0,
        };
        let settings = ImagingSettings {
            defocus_model: DefocusModel::KernelPhase,
            ..Default::default()
        };
        let engine = AerialImageEngine::with_settings(&source, &optics, grid, settings).unwrap();
        let k0 = engine.kernels(0.0);
        let kz = engine.kernels(120.0);
        let (n, df) = (32usize, 1.0 / 128.0);
        for (a, b) in k0.kernels.iter().zip(&kz.kernels) {
            for i in 0..n {
                for j in 0..n {
                    let f2 = (signed_index(i, n) * df).powi(2) + (signed_index(j, n) * df).powi(2);
                    let phase = std::f64::consts::PI * 120.0 * 157.63 * f2;
                    let expected = a[[i, j]] * Complex64::from_polar(1.0, phase);
                    assert!((b[[i, j]] - expected).norm() < 1e-14);
                }
            }
        }
    }

    #[test]
    fn test_select_kernel_count() {
        let vals = [5.0, 3.0, 1.0, 0.5, 1e-14];
        let trace = 9.5;
        assert_eq!(select_kernel_count(&vals, trace, 10, 1.0), 4);
        assert_eq!(select_kernel_count(&vals, trace, 2, 1.0), 2);
        assert_eq!(select_kernel_count(&vals, trace, 10, 0.8), 2);
        assert_eq!(select_kernel_count(&vals, trace, 10, 0.95), 4);
        assert_eq!(select_kernel_count(&[], 0.0, 10, 1.0), 0);
        // Degenerate clusters are never split.
        let pair = [4.0, 2.0, 2.0 * (1.0 + 1e-9), 1.0];
        assert_eq!(select_kernel_count(&pair, 9.0, 2, 1.0), 1); // cap: back off
        assert_eq!(select_kernel_count(&pair, 9.0, 10, 0.6), 3); // energy: complete
        assert_eq!(select_kernel_count(&[3.0, 3.0, 1.0], 7.0, 1, 1.0), 2); // leading
    }

    /// Build the same kernel set with every eigensolver path.
    fn build_with(
        method: DecompositionMethod,
        max_kernels: usize,
        fraction: f64,
        z: f64,
    ) -> (SparseKernelSet, AerialImageEngine) {
        let source = VuvSource {
            wavelength_nm: 100.0,
            spectral_samples: 1,
            illumination: IlluminationShape::Annular {
                sigma_inner: 0.3,
                sigma_outer: 0.8,
            },
            ..VuvSource::f2_laser(0.8).unwrap()
        };
        let mut optics = ProjectionOptics::new(0.5).unwrap();
        optics.flare_fraction = 0.0;
        let grid = GridConfig {
            size: 32,
            pixel_nm: 12.5,
        };
        let settings = ImagingSettings {
            max_kernels,
            kernel_energy_fraction: fraction,
            source_points_per_axis: Some(9),
            ..Default::default()
        };
        let engine =
            AerialImageEngine::with_settings(&source, &optics, grid, settings.clone()).unwrap();
        let set = KernelBuilder {
            optics: engine.optics(),
            points: engine.source_points(),
            grid: engine.grid(),
            settings: &settings,
            force_method: Some(method),
        }
        .build(z, 100.0)
        .unwrap();
        (set, engine)
    }

    fn image_of(engine: &AerialImageEngine, set: &SparseKernelSet) -> Array2<f64> {
        let mask = Mask {
            mask_type: crate::mask::MaskType::Binary,
            features: vec![
                crate::mask::MaskFeature::Rect {
                    x: -40.0,
                    y: 10.0,
                    w: 75.0,
                    h: 150.0,
                },
                crate::mask::MaskFeature::Rect {
                    x: 60.0,
                    y: -50.0,
                    w: 50.0,
                    h: 50.0,
                },
            ],
            dark_field: true,
        };
        let spectrum = mask.spectrum(engine.grid(), &Fft2D::new());
        engine.socs_intensity(set, &spectrum)
    }

    #[test]
    fn test_all_decomposition_paths_agree() {
        for z in [0.0, 180.0] {
            let (tcc, engine) = build_with(DecompositionMethod::DenseTcc, 1000, 1.0, z);
            let (gram, _) = build_with(DecompositionMethod::DenseGram, 1000, 1.0, z);
            let (rand, _) = build_with(DecompositionMethod::Randomized, 1000, 1.0, z);
            assert!(tcc.diagnostics.num_columns > tcc.diagnostics.num_frequencies);
            let rank = tcc.eigenvalues.len();
            assert_eq!(gram.eigenvalues.len(), rank);
            assert_eq!(rand.eigenvalues.len(), rank);
            for k in 0..rank {
                let l0 = tcc.eigenvalues[0];
                assert!((tcc.eigenvalues[k] - gram.eigenvalues[k]).abs() < 1e-12 * l0);
                assert!((tcc.eigenvalues[k] - rand.eigenvalues[k]).abs() < 1e-10 * l0);
            }
            assert_relative_eq!(tcc.captured_energy_fraction, 1.0, epsilon = 1e-10);
            let reference = image_of(&engine, &tcc);
            for other in [&gram, &rand] {
                let img = image_of(&engine, other);
                for (a, b) in img.iter().zip(reference.iter()) {
                    assert!((a - b).abs() < 1e-10, "{a} vs {b}");
                }
            }
        }
    }

    #[test]
    fn test_truncated_randomized_matches_dense_leading_pairs() {
        let (dense, engine) = build_with(DecompositionMethod::DenseTcc, 6, 1.0, 90.0);
        let (rand, _) = build_with(DecompositionMethod::Randomized, 6, 1.0, 90.0);
        assert_eq!(rand.eigenvalues.len(), 6);
        // Randomized subspace iteration: relative eigenvalue error is bounded
        // by ~(λ_{l+1}/λ_k)^(2q+1), l = 6 + oversampling, q = power iterations.
        let (all, _) = build_with(DecompositionMethod::DenseTcc, 1000, 1.0, 90.0);
        let l = 6 + RANDOMIZED_OVERSAMPLE;
        for k in 0..6 {
            let bound = (all.eigenvalues[l] / all.eigenvalues[k])
                .powi(2 * RANDOMIZED_POWER_ITERS as i32 + 1)
                .max(1e-13);
            let rel = (rand.eigenvalues[k] / dense.eigenvalues[k] - 1.0).abs();
            assert!(
                rel <= 10.0 * bound,
                "k={k}: rel {rel:.2e} bound {bound:.2e}"
            );
        }
        let a = image_of(&engine, &dense);
        let b = image_of(&engine, &rand);
        let d = a
            .iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).abs())
            .fold(0.0, f64::max);
        assert!(d < 1e-5, "image difference {d:.3e}");
    }
}
