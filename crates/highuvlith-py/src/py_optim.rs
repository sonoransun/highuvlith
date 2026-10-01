//! PyO3 bindings for the optimization and patterning modules: true-adjoint
//! inverse lithography ([`highuvlith_core::ilt`]), fragment-based model OPC
//! ([`highuvlith_core::opc`]), SRAF insertion with a model print check and a
//! depth-of-focus comparison ([`highuvlith_core::sraf`]), and geometric
//! SADP/SAQP spacer patterning and LELE double patterning
//! ([`highuvlith_core::double_patterning`]), and analytic directed
//! self-assembly ([`highuvlith_core::dsa`], 🔶).
//!
//! Engine-based functions build the aerial-imaging engine from
//! `(source, optics, grid, max_kernels)` and release the GIL while they run.
//! An optional `illumination` tuple overrides the source's pupil fill —
//! `("conventional", sigma)`, `("annular", sigma_inner, sigma_outer)`,
//! `("dipole", sigma_center, sigma_radius, orientation_deg)` or
//! `("quadrupole", sigma_center, sigma_radius, opening_angle_deg)` — imaging
//! at the source's centre wavelength.

use numpy::{IntoPyArray, PyArray1, PyArray2, PyReadonlyArray2, ToPyArray};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyTuple;

use highuvlith_core::aerial::AerialImageEngine;
use highuvlith_core::double_patterning::{self, SpacerPatterningResult, SpacerTone};
use highuvlith_core::dsa;
use highuvlith_core::ilt::{
    self, ContactShape, ILTConfig, ILTResult, IltCost, IltGradient, IltInit, IltOptimizer,
    IltTermination,
};
use highuvlith_core::mask::{Mask, MaskFeature, MaskType};
use highuvlith_core::opc::{
    self, FragmentKind, FragmentOpcConfig, FragmentOpcResult, ProcessCondition,
};
use highuvlith_core::source::{LithographySource, VuvSource};
use highuvlith_core::sraf::{
    self, DofConfig, LineCut, PrintCheckConfig, SrafDofComparison, SrafResult, SrafRules,
};
use highuvlith_core::types::Complex64;

use crate::py_config::*;

fn value_err(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// Build the aerial engine, optionally overriding the pupil fill.
fn build_engine(
    py: Python<'_>,
    source: &PySourceConfig,
    optics: &PyOpticsConfig,
    grid: &PyGridConfig,
    max_kernels: usize,
    illumination: Option<&Bound<'_, PyTuple>>,
) -> PyResult<AerialImageEngine> {
    let override_source = match illumination {
        Some(spec) => {
            let shape = crate::py_config::parse_illumination_tuple(spec)?;
            Some(VuvSource {
                wavelength_nm: source.inner.wavelength_nm(),
                illumination: shape,
                ..VuvSource::f2_laser(0.5).map_err(value_err)?
            })
        }
        None => None,
    };
    let grid_cfg = grid.inner.clone();
    py.allow_threads(|| match &override_source {
        Some(s) => AerialImageEngine::new(s, optics.inner.as_dyn(), grid_cfg, max_kernels),
        None => AerialImageEngine::new(&source.inner, optics.inner.as_dyn(), grid_cfg, max_kernels),
    })
    .map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

fn parse_conditions(conditions: Option<Vec<(f64, f64, f64)>>) -> Vec<ProcessCondition> {
    match conditions {
        Some(list) => list
            .into_iter()
            .map(|(defocus_nm, dose, weight)| ProcessCondition {
                defocus_nm,
                dose,
                weight,
            })
            .collect(),
        None => vec![ProcessCondition::nominal()],
    }
}

// ---------------------------------------------------------------------------
// Masks
// ---------------------------------------------------------------------------

/// DEPRECATED — use `MaskConfig.from_features`, which accepts the same
/// `(x, y, w, h)` rectangles and vertex-list polygons (plus every other
/// feature type): `MaskConfig.from_features(rects + polygons,
/// dark_field=..., mask_type="att_psm", transmission=T, phase_deg=...)`.
/// Kept for compatibility; emits a `DeprecationWarning`. Behaviour is
/// unchanged: `rects` as `(x, y, w, h)` centre and size in nm, `polygons` as
/// vertex lists, clear on a dark-field mask and absorber on a bright-field
/// mask; with `attenuated_transmission` the absorber is an attenuated PSM
/// (`sqrt(T)·exp(i·phase)`).
#[pyfunction]
#[pyo3(signature = (rects=None, polygons=None, dark_field=false, attenuated_transmission=None, attenuated_phase_deg=180.0))]
fn mask_from_features(
    py: Python<'_>,
    rects: Option<Vec<(f64, f64, f64, f64)>>,
    polygons: Option<Vec<Vec<(f64, f64)>>>,
    dark_field: bool,
    attenuated_transmission: Option<f64>,
    attenuated_phase_deg: f64,
) -> PyResult<PyMaskConfig> {
    let category = py.get_type::<pyo3::exceptions::PyDeprecationWarning>();
    PyErr::warn(
        py,
        category.as_any(),
        c"mask_from_features is deprecated; use MaskConfig.from_features([(x, y, w, h), ...] \
          + polygons, dark_field=..., mask_type='att_psm', transmission=..., phase_deg=...)",
        1,
    )?;
    let mut features = Vec::new();
    for (x, y, w, h) in rects.unwrap_or_default() {
        if !(w > 0.0 && h > 0.0) {
            return Err(PyValueError::new_err(format!(
                "rect widths must be positive, got w={w}, h={h}"
            )));
        }
        features.push(MaskFeature::Rect { x, y, w, h });
    }
    for vertices in polygons.unwrap_or_default() {
        if vertices.len() < 3 {
            return Err(PyValueError::new_err("polygons need at least 3 vertices"));
        }
        features.push(MaskFeature::Polygon { vertices });
    }
    let mask_type = match attenuated_transmission {
        Some(t) if (0.0..=1.0).contains(&t) => MaskType::AttenuatedPSM {
            transmission: t,
            phase_deg: attenuated_phase_deg,
        },
        Some(t) => {
            return Err(PyValueError::new_err(format!(
                "attenuated_transmission must be in [0, 1], got {t}"
            )))
        }
        None => MaskType::Binary,
    };
    Ok(PyMaskConfig {
        inner: Mask {
            mask_type,
            features,
            dark_field,
        },
    })
}

// ---------------------------------------------------------------------------
// ILT
// ---------------------------------------------------------------------------

/// Result of `optimize_ilt`: optimized continuous mask, images, and
/// optimizer history.
#[pyclass(name = "IltResult")]
pub struct PyIltResult {
    inner: ILTResult,
    adjoint: bool,
}

#[pymethods]
impl PyIltResult {
    /// Optimized continuous mask transmission in (0, 1), shape `(n, n)`.
    #[getter]
    fn mask<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        self.inner.mask_transmittance.to_pyarray(py)
    }

    /// Aerial image of the optimized mask at the first condition's focus.
    #[getter]
    fn aerial_image<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        self.inner.aerial_image.to_pyarray(py)
    }

    /// The mask thresholded at 0.5 (0/1).
    #[getter]
    fn binary_mask<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        self.inner.binary_mask.to_pyarray(py)
    }

    /// Aerial image of the binary mask.
    #[getter]
    fn binary_aerial_image<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        self.inner.binary_aerial_image.to_pyarray(py)
    }

    /// Cost of the initial mask followed by the cost after each iteration.
    #[getter]
    fn cost_history<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        self.inner.cost_history.to_pyarray(py)
    }

    /// L2 norm of the gradient per recorded cost.
    #[getter]
    fn gradient_norm_history<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        self.inner.gradient_norm_history.to_pyarray(py)
    }

    /// Indices into `cost_history` where optimization stages start.
    #[getter]
    fn stage_starts(&self) -> Vec<usize> {
        self.inner.stage_starts.clone()
    }

    /// Optimizer iterations performed.
    #[getter]
    fn iterations(&self) -> usize {
        self.inner.iterations
    }

    /// Whether the optimizer converged.
    #[getter]
    fn converged(&self) -> bool {
        self.inner.converged
    }

    /// Why the optimizer stopped: "converged", "max_iterations" or
    /// "line_search_failed".
    #[getter]
    fn termination(&self) -> &'static str {
        match self.inner.termination {
            IltTermination::Converged => "converged",
            IltTermination::MaxIterations => "max_iterations",
            IltTermination::LineSearchFailed => "line_search_failed",
        }
    }

    /// Final cost.
    #[getter]
    fn final_cost(&self) -> f64 {
        self.inner.final_cost
    }

    /// Misprinted pixels of the continuous mask (first condition).
    #[getter]
    fn pattern_error(&self) -> usize {
        self.inner.pattern_error
    }

    /// Misprinted pixels of the binary mask (first condition).
    #[getter]
    fn binary_pattern_error(&self) -> usize {
        self.inner.binary_pattern_error
    }

    /// `(iteration, mask)` snapshots.
    #[getter]
    fn snapshots<'py>(&self, py: Python<'py>) -> Vec<(usize, Bound<'py, PyArray2<f64>>)> {
        self.inner
            .snapshots
            .iter()
            .map(|(i, m)| (*i, m.to_pyarray(py)))
            .collect()
    }

    /// Capability badge of the model behind this result (see
    /// docs/capability-matrix.md): "✅" for the exact adjoint gradient, "🔶"
    /// for the legacy local proxy gradient.
    #[getter]
    fn status(&self) -> &'static str {
        if self.adjoint {
            "✅"
        } else {
            "🔶"
        }
    }

    /// The model's stated approximations for this result.
    #[getter]
    fn notes(&self) -> Vec<&'static str> {
        let mut notes = vec![
            "constant-threshold sigmoid resist; continuous pixel mask (no MRC or polygon extraction)",
        ];
        if !self.adjoint {
            notes.push("legacy local proxy gradient (an approximation of the true adjoint)");
        }
        notes
    }

    fn __repr__(&self) -> String {
        format!(
            "IltResult(iterations={}, final_cost={:.4e}, termination={}, pattern_error={})",
            self.inner.iterations,
            self.inner.final_cost,
            self.termination(),
            self.inner.pattern_error
        )
    }
}

/// Target for a centred `count_x × count_y` array of holes (1 inside, area
/// averaged at pixel edges) on `grid`; `shape` is "round" or "square".
#[pyfunction]
#[pyo3(signature = (size_nm, pitch_x_nm, pitch_y_nm, count_x, count_y, grid, shape="round"))]
#[allow(clippy::too_many_arguments)]
fn ilt_contact_target<'py>(
    py: Python<'py>,
    size_nm: f64,
    pitch_x_nm: f64,
    pitch_y_nm: f64,
    count_x: usize,
    count_y: usize,
    grid: PyGridConfig,
    shape: &str,
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let shape = match shape {
        "round" => ContactShape::Round,
        "square" => ContactShape::Square,
        other => {
            return Err(PyValueError::new_err(format!(
                "shape must be 'round' or 'square', got '{other}'"
            )))
        }
    };
    if size_nm.is_nan() || size_nm <= 0.0 {
        return Err(PyValueError::new_err("size_nm must be positive"));
    }
    Ok(ilt::create_target_contact_array(
        size_nm,
        pitch_x_nm,
        pitch_y_nm,
        count_x,
        count_y,
        shape,
        &grid.inner,
    )
    .into_pyarray(py))
}

/// Inverse lithography with the exact adjoint gradient through the SOCS
/// kernels (or the legacy local proxy with `gradient="proxy"`).
///
/// `target` is a `(n, n)` array on the grid (1 = bright/printed). `cost`
/// is "image" (mean-squared aerial-image error) or "resist" (sigmoid resist
/// contour with `threshold` and `steepness`). `conditions` is a list of
/// `(defocus_nm, dose, weight)` for process-window-aware ILT.
#[pyfunction]
#[pyo3(signature = (
    source, optics, grid, target, cost="image", threshold=0.3, steepness=50.0,
    gradient="adjoint", optimizer="cg", max_iterations=50, learning_rate=0.5,
    tv_weight=0.01, binarization_weight=0.0, binarization_after=0, min_feature_nm=0.0,
    sigmoid_steepness=4.0, conditions=None, absorber_transmission=0.0,
    absorber_phase_deg=180.0, init="target", init_level=0.5, initial_mask=None,
    snapshot_every=0, convergence_tol=1e-4, max_kernels=16, illumination=None,
))]
#[allow(clippy::too_many_arguments)]
fn optimize_ilt(
    py: Python<'_>,
    source: PySourceConfig,
    optics: PyOpticsConfig,
    grid: PyGridConfig,
    target: PyReadonlyArray2<f64>,
    cost: &str,
    threshold: f64,
    steepness: f64,
    gradient: &str,
    optimizer: &str,
    max_iterations: usize,
    learning_rate: f64,
    tv_weight: f64,
    binarization_weight: f64,
    binarization_after: usize,
    min_feature_nm: f64,
    sigmoid_steepness: f64,
    conditions: Option<Vec<(f64, f64, f64)>>,
    absorber_transmission: f64,
    absorber_phase_deg: f64,
    init: &str,
    init_level: f64,
    initial_mask: Option<PyReadonlyArray2<f64>>,
    snapshot_every: usize,
    convergence_tol: f64,
    max_kernels: usize,
    illumination: Option<Bound<'_, PyTuple>>,
) -> PyResult<PyIltResult> {
    let cost = match cost {
        "image" => IltCost::ImageFidelity,
        "resist" => IltCost::ResistContour {
            threshold,
            steepness,
        },
        other => {
            return Err(PyValueError::new_err(format!(
                "cost must be 'image' or 'resist', got '{other}'"
            )))
        }
    };
    let gradient = match gradient {
        "adjoint" => IltGradient::Adjoint,
        "proxy" => IltGradient::LocalProxy,
        other => {
            return Err(PyValueError::new_err(format!(
                "gradient must be 'adjoint' or 'proxy', got '{other}'"
            )))
        }
    };
    let optimizer = match optimizer {
        "cg" => IltOptimizer::ConjugateGradient,
        "sd" => IltOptimizer::SteepestDescent,
        other => {
            return Err(PyValueError::new_err(format!(
                "optimizer must be 'cg' or 'sd', got '{other}'"
            )))
        }
    };
    if !(0.0..=1.0).contains(&absorber_transmission) {
        return Err(PyValueError::new_err(
            "absorber_transmission must be in [0, 1]",
        ));
    }
    let init = match (initial_mask, init) {
        (Some(m), _) => IltInit::Mask(m.as_array().to_owned()),
        (None, "target") => IltInit::Target,
        (None, "uniform") => IltInit::Uniform(init_level),
        (None, other) => {
            return Err(PyValueError::new_err(format!(
                "init must be 'target' or 'uniform', got '{other}'"
            )))
        }
    };
    let config = ILTConfig {
        target: target.as_array().to_owned(),
        learning_rate,
        regularization: tv_weight,
        max_iterations,
        convergence_tol,
        min_feature_nm,
        gradient,
        optimizer,
        cost,
        conditions: parse_conditions(conditions),
        sigmoid_steepness,
        binarization_weight,
        binarization_after,
        absorber: Complex64::from_polar(
            absorber_transmission.sqrt(),
            absorber_phase_deg.to_radians(),
        ),
        init,
        snapshot_every,
        ..Default::default()
    };
    let engine = build_engine(
        py,
        &source,
        &optics,
        &grid,
        max_kernels,
        illumination.as_ref(),
    )?;
    let inner = py
        .allow_threads(|| ilt::optimize_ilt(&engine, &config))
        .map_err(value_err)?;
    Ok(PyIltResult {
        inner,
        adjoint: matches!(gradient, IltGradient::Adjoint),
    })
}

// ---------------------------------------------------------------------------
// Fragment OPC
// ---------------------------------------------------------------------------

/// Result of `fragment_opc`.
#[pyclass(name = "OpcResult")]
pub struct PyOpcResult {
    inner: FragmentOpcResult,
}

#[pymethods]
impl PyOpcResult {
    /// Corrected mask (corrected polygons plus pass-through features).
    #[getter]
    fn mask(&self) -> PyMaskConfig {
        PyMaskConfig {
            inner: self.inner.mask.clone(),
        }
    }

    /// Corrected polygons as vertex lists.
    #[getter]
    fn polygons(&self) -> Vec<Vec<(f64, f64)>> {
        self.inner.polygons.clone()
    }

    /// Whether the rms and max EPE tolerances were met.
    #[getter]
    fn converged(&self) -> bool {
        self.inner.converged
    }

    /// Number of recorded iterations.
    #[getter]
    fn iterations(&self) -> usize {
        self.inner.history.len()
    }

    /// RMS EPE (nm) of controlled fragments per iteration.
    #[getter]
    fn epe_rms_history<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let v: Vec<f64> = self.inner.history.iter().map(|h| h.epe_rms_nm).collect();
        v.into_pyarray(py)
    }

    /// Max |EPE| (nm) of controlled fragments per iteration.
    #[getter]
    fn epe_max_history<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let v: Vec<f64> = self.inner.history.iter().map(|h| h.epe_max_nm).collect();
        v.into_pyarray(py)
    }

    /// RMS EPE (nm) re-measured on the output geometry.
    #[getter]
    fn final_epe_rms_nm(&self) -> f64 {
        self.inner.final_stats.epe_rms_nm
    }

    /// Max |EPE| (nm) re-measured on the output geometry.
    #[getter]
    fn final_epe_max_nm(&self) -> f64 {
        self.inner.final_stats.epe_max_nm
    }

    /// Aerial image of the corrected mask at the first condition's focus.
    #[getter]
    fn final_image<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        self.inner.final_image.to_pyarray(py)
    }

    /// Final fragment biases (nm, outward positive).
    #[getter]
    fn fragment_biases<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let v: Vec<f64> = self.inner.fragments.iter().map(|f| f.bias_nm).collect();
        v.into_pyarray(py)
    }

    /// Final fragment EPEs (nm; NaN for slaved corner fragments).
    #[getter]
    fn fragment_epes<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let v: Vec<f64> = self.inner.fragments.iter().map(|f| f.epe_nm).collect();
        v.into_pyarray(py)
    }

    /// Fragment roles: "edge", "corner" or "line_end".
    #[getter]
    fn fragment_kinds(&self) -> Vec<&'static str> {
        self.inner
            .fragments
            .iter()
            .map(|f| match f.kind {
                FragmentKind::Edge => "edge",
                FragmentKind::Corner => "corner",
                FragmentKind::LineEnd => "line_end",
            })
            .collect()
    }

    /// Fragment control points `(x, y)` on the target edges (nm).
    #[getter]
    fn fragment_points(&self) -> Vec<(f64, f64)> {
        self.inner
            .fragments
            .iter()
            .map(|f| f.control_point())
            .collect()
    }

    /// Capability badge of the model behind this result (see
    /// docs/capability-matrix.md): "✅" implemented, "🔶" simplified, "🧪"
    /// theoretical; split badges name exact and approximate parts.
    #[getter]
    fn status(&self) -> &'static str {
        "✅"
    }

    /// The model's stated approximations for this result.
    #[getter]
    fn notes(&self) -> Vec<&'static str> {
        vec![
            "per-fragment EPE feedback on the aerial image at a constant print threshold",
            "corners slaved (no corner-rounding correction); rectilinear features only",
            "no MRC beyond the bias clamp, jog cleanup and grid snap",
        ]
    }

    fn __repr__(&self) -> String {
        format!(
            "OpcResult(converged={}, iterations={}, final_epe_rms_nm={:.3})",
            self.inner.converged,
            self.inner.history.len(),
            self.inner.final_stats.epe_rms_nm
        )
    }
}

/// Fragment-based model OPC of the rectangles / rectilinear polygons of
/// `mask`, at print threshold `threshold`. Unset options take the
/// resolution-scaled defaults of `FragmentOpcConfig::for_engine`.
#[pyfunction]
#[pyo3(signature = (
    source, optics, mask, grid, threshold, max_iterations=None, tolerance_nm=None,
    max_epe_tolerance_nm=None, feedback_gain=None, smoothing=None, max_bias_nm=None,
    max_fragment_nm=None, corner_fragment_nm=None, min_jog_nm=None, bias_grid_nm=0.0,
    correct_corners=false, conditions=None, max_kernels=16, illumination=None,
))]
#[allow(clippy::too_many_arguments)]
fn fragment_opc(
    py: Python<'_>,
    source: PySourceConfig,
    optics: PyOpticsConfig,
    mask: PyMaskConfig,
    grid: PyGridConfig,
    threshold: f64,
    max_iterations: Option<usize>,
    tolerance_nm: Option<f64>,
    max_epe_tolerance_nm: Option<f64>,
    feedback_gain: Option<f64>,
    smoothing: Option<f64>,
    max_bias_nm: Option<f64>,
    max_fragment_nm: Option<f64>,
    corner_fragment_nm: Option<f64>,
    min_jog_nm: Option<f64>,
    bias_grid_nm: f64,
    correct_corners: bool,
    conditions: Option<Vec<(f64, f64, f64)>>,
    max_kernels: usize,
    illumination: Option<Bound<'_, PyTuple>>,
) -> PyResult<PyOpcResult> {
    let engine = build_engine(
        py,
        &source,
        &optics,
        &grid,
        max_kernels,
        illumination.as_ref(),
    )?;
    let mut config = FragmentOpcConfig::for_engine(&engine, threshold);
    if let Some(v) = max_iterations {
        config.max_iterations = v;
    }
    if let Some(v) = tolerance_nm {
        config.tolerance_nm = v;
    }
    if let Some(v) = max_epe_tolerance_nm {
        config.max_epe_tolerance_nm = v;
    }
    if let Some(v) = feedback_gain {
        config.feedback_gain = v;
    }
    if let Some(v) = smoothing {
        config.smoothing = v;
    }
    if let Some(v) = max_bias_nm {
        config.max_bias_nm = v;
    }
    if let Some(v) = max_fragment_nm {
        config.max_fragment_nm = v;
    }
    if let Some(v) = corner_fragment_nm {
        config.corner_fragment_nm = v;
    }
    if let Some(v) = min_jog_nm {
        config.min_jog_nm = v;
    }
    config.bias_grid_nm = bias_grid_nm;
    config.correct_corners = correct_corners;
    config.conditions = parse_conditions(conditions);
    let inner = py
        .allow_threads(|| opc::fragment_opc(&mask.inner, &engine, &config))
        .map_err(value_err)?;
    Ok(PyOpcResult { inner })
}

// ---------------------------------------------------------------------------
// SRAF
// ---------------------------------------------------------------------------

/// Result of `insert_srafs`.
#[pyclass(name = "SrafResult")]
pub struct PySrafResult {
    inner: SrafResult,
}

#[pymethods]
impl PySrafResult {
    /// Main features plus surviving assists.
    #[getter]
    fn mask(&self) -> PyMaskConfig {
        PyMaskConfig {
            inner: self.inner.mask.clone(),
        }
    }

    /// Surviving assists as `(x, y, w, h)` rectangles (nm).
    #[getter]
    fn assists(&self) -> Vec<(f64, f64, f64, f64)> {
        self.inner
            .assists
            .iter()
            .filter_map(|a| match a {
                MaskFeature::Rect { x, y, w, h } => Some((*x, *y, *w, *h)),
                _ => None,
            })
            .collect()
    }

    /// Assists proposed by the rules.
    #[getter]
    fn placed(&self) -> usize {
        self.inner.placed
    }

    /// Assists removed by the print check.
    #[getter]
    fn removed(&self) -> usize {
        self.inner.removed
    }

    /// Shrink operations applied by the print check.
    #[getter]
    fn shrink_steps(&self) -> usize {
        self.inner.shrink_steps
    }

    /// Smallest print margin of the surviving assists (≥ the requested
    /// margin; +inf when there are none).
    #[getter]
    fn worst_margin(&self) -> f64 {
        self.inner
            .reports
            .iter()
            .map(|r| r.worst_margin)
            .fold(f64::INFINITY, f64::min)
    }

    /// Capability badge of the model behind this result (see
    /// docs/capability-matrix.md): "✅" implemented, "🔶" simplified, "🧪"
    /// theoretical; split badges name exact and approximate parts.
    #[getter]
    fn status(&self) -> &'static str {
        "🔶"
    }

    /// The model's stated approximations for this result.
    #[getter]
    fn notes(&self) -> Vec<&'static str> {
        vec![
            "heuristic, lambda/NA-scaled rule deck (parallel lines, isolated contacts)",
            "model print check at the focus/dose corners with a constant-threshold resist",
        ]
    }

    fn __repr__(&self) -> String {
        format!(
            "SrafResult(assists={}, placed={}, removed={}, shrink_steps={})",
            self.inner.assists.len(),
            self.inner.placed,
            self.inner.removed,
            self.inner.shrink_steps
        )
    }
}

/// Insert SRAFs with the heuristic rule deck of `SrafRules::for_engine`
/// (lines of width `line_cd_nm`, effective illumination centre
/// `sigma_center`), then shrink/remove every assist that prints at the
/// corners `{±defocus_nm, 0} × {1 ± dose_excursion, 1}` with safety
/// `margin` at print threshold `threshold`.
#[pyfunction]
#[pyo3(signature = (
    source, optics, mask, grid, line_cd_nm, sigma_center, threshold,
    defocus_nm=150.0, dose_excursion=0.08, margin=0.05, max_kernels=16, illumination=None,
))]
#[allow(clippy::too_many_arguments)]
fn insert_srafs(
    py: Python<'_>,
    source: PySourceConfig,
    optics: PyOpticsConfig,
    mask: PyMaskConfig,
    grid: PyGridConfig,
    line_cd_nm: f64,
    sigma_center: f64,
    threshold: f64,
    defocus_nm: f64,
    dose_excursion: f64,
    margin: f64,
    max_kernels: usize,
    illumination: Option<Bound<'_, PyTuple>>,
) -> PyResult<PySrafResult> {
    let engine = build_engine(
        py,
        &source,
        &optics,
        &grid,
        max_kernels,
        illumination.as_ref(),
    )?;
    let rules = SrafRules::for_engine(&engine, line_cd_nm, sigma_center);
    let check = PrintCheckConfig {
        margin,
        ..PrintCheckConfig::corners(threshold, defocus_nm, dose_excursion)
    };
    let inner = py
        .allow_threads(|| sraf::insert_srafs(&mask.inner, &engine, &rules, &check))
        .map_err(value_err)?;
    Ok(PySrafResult { inner })
}

fn line_cut(
    x_center_nm: f64,
    y_nm: f64,
    half_width_nm: Option<f64>,
    target_cd_nm: f64,
    dark_line: bool,
) -> LineCut {
    LineCut {
        x_center_nm,
        y_nm,
        half_width_nm: half_width_nm.unwrap_or(1.5 * target_cd_nm),
        dark_line,
    }
}

/// Dose-to-size print threshold: the threshold at which the line centred at
/// `x_center_nm` (cut at `y_nm`) prints at `target_cd_nm` in focus.
#[pyfunction]
#[pyo3(signature = (
    source, optics, mask, grid, target_cd_nm, x_center_nm=0.0, y_nm=0.0,
    half_width_nm=None, dark_line=true, max_kernels=16, illumination=None,
))]
#[allow(clippy::too_many_arguments)]
fn dose_to_size_threshold(
    py: Python<'_>,
    source: PySourceConfig,
    optics: PyOpticsConfig,
    mask: PyMaskConfig,
    grid: PyGridConfig,
    target_cd_nm: f64,
    x_center_nm: f64,
    y_nm: f64,
    half_width_nm: Option<f64>,
    dark_line: bool,
    max_kernels: usize,
    illumination: Option<Bound<'_, PyTuple>>,
) -> PyResult<f64> {
    let engine = build_engine(
        py,
        &source,
        &optics,
        &grid,
        max_kernels,
        illumination.as_ref(),
    )?;
    let cut = line_cut(x_center_nm, y_nm, half_width_nm, target_cd_nm, dark_line);
    py.allow_threads(|| sraf::dose_to_size_threshold(&engine, &mask.inner, &cut, target_cd_nm))
        .map_err(value_err)
}

/// Depth of focus of a line with and without assists.
#[pyclass(name = "DofComparison")]
pub struct PyDofComparison {
    inner: SrafDofComparison,
}

fn cds_array<'py>(py: Python<'py>, cds: &[Option<f64>]) -> Bound<'py, PyArray1<f64>> {
    let v: Vec<f64> = cds.iter().map(|c| c.unwrap_or(f64::NAN)).collect();
    v.into_pyarray(py)
}

#[pymethods]
impl PyDofComparison {
    /// DOF (nm) of the mask without assists.
    #[getter]
    fn dof_without_nm(&self) -> f64 {
        self.inner.without.dof_nm
    }

    /// DOF (nm) of the mask with assists.
    #[getter]
    fn dof_with_nm(&self) -> f64 {
        self.inner.with.dof_nm
    }

    /// DOF ratio with / without.
    #[getter]
    fn gain(&self) -> f64 {
        self.inner.gain()
    }

    /// Focus samples (nm).
    #[getter]
    fn focus_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        self.inner.without.focus_nm.to_pyarray(py)
    }

    /// Nominal-dose CD (nm, NaN = not printed) without assists.
    #[getter]
    fn cd_without_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        cds_array(py, &self.inner.without.cd_nm)
    }

    /// Nominal-dose CD (nm, NaN = not printed) with assists.
    #[getter]
    fn cd_with_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        cds_array(py, &self.inner.with.cd_nm)
    }

    /// Dose-to-size thresholds `(without, with)`.
    #[getter]
    fn thresholds(&self) -> (f64, f64) {
        (self.inner.without.threshold, self.inner.with.threshold)
    }

    /// In-spec focus windows `((lo, hi) without, (lo, hi) with)` (nm).
    #[getter]
    fn windows_nm(&self) -> ((f64, f64), (f64, f64)) {
        (self.inner.without.window_nm, self.inner.with.window_nm)
    }

    /// Capability badge of the model behind this result (see
    /// docs/capability-matrix.md): "✅" implemented, "🔶" simplified, "🧪"
    /// theoretical; split badges name exact and approximate parts.
    #[getter]
    fn status(&self) -> &'static str {
        "🔶"
    }

    /// The model's stated approximations for this result.
    #[getter]
    fn notes(&self) -> Vec<&'static str> {
        vec![
            "constant-threshold resist; each mask at its own in-focus dose-to-size threshold",
            "CD measured on one cut line",
        ]
    }

    fn __repr__(&self) -> String {
        format!(
            "DofComparison(dof_without_nm={:.1}, dof_with_nm={:.1}, gain={:.3})",
            self.inner.without.dof_nm,
            self.inner.with.dof_nm,
            self.inner.gain()
        )
    }
}

/// Compare the depth of focus of a line with (`mask_with`) and without
/// (`mask_without`) assists, each at its own dose-to-size threshold. The
/// result depends on the engine's defocus model (see the Rust docs of
/// `compare_sraf_dof`).
#[pyfunction]
#[pyo3(signature = (
    source, optics, mask_without, mask_with, grid, target_cd_nm, x_center_nm=0.0, y_nm=0.0,
    half_width_nm=None, dark_line=true, cd_tolerance_pct=10.0, focus_range_nm=300.0,
    focus_steps=13, exposure_latitude_pct=0.0, max_kernels=16, illumination=None,
))]
#[allow(clippy::too_many_arguments)]
fn compare_sraf_dof(
    py: Python<'_>,
    source: PySourceConfig,
    optics: PyOpticsConfig,
    mask_without: PyMaskConfig,
    mask_with: PyMaskConfig,
    grid: PyGridConfig,
    target_cd_nm: f64,
    x_center_nm: f64,
    y_nm: f64,
    half_width_nm: Option<f64>,
    dark_line: bool,
    cd_tolerance_pct: f64,
    focus_range_nm: f64,
    focus_steps: usize,
    exposure_latitude_pct: f64,
    max_kernels: usize,
    illumination: Option<Bound<'_, PyTuple>>,
) -> PyResult<PyDofComparison> {
    let engine = build_engine(
        py,
        &source,
        &optics,
        &grid,
        max_kernels,
        illumination.as_ref(),
    )?;
    let cut = line_cut(x_center_nm, y_nm, half_width_nm, target_cd_nm, dark_line);
    let config = DofConfig {
        target_cd_nm,
        cd_tolerance_pct,
        focus_range_nm,
        focus_steps,
        exposure_latitude_pct,
    };
    let inner = py
        .allow_threads(|| {
            sraf::compare_sraf_dof(
                &engine,
                &mask_without.inner,
                &mask_with.inner,
                &cut,
                &config,
            )
        })
        .map_err(value_err)?;
    Ok(PyDofComparison { inner })
}

// ---------------------------------------------------------------------------
// SADP / SAQP
// ---------------------------------------------------------------------------

/// Geometric spacer-patterning result (one mandrel period).
#[pyclass(name = "SpacerPatterningResult")]
pub struct PySpacerPatterningResult {
    inner: SpacerPatterningResult,
}

#[pymethods]
impl PySpacerPatterningResult {
    /// Final line intervals `(x0, x1)` (nm) in one mandrel period.
    #[getter]
    fn lines(&self) -> Vec<(f64, f64)> {
        self.inner.pattern.lines.clone()
    }

    /// Mandrel period (nm).
    #[getter]
    fn period_nm(&self) -> f64 {
        self.inner.pattern.period_nm
    }

    /// Final line CDs (nm).
    #[getter]
    fn line_cds_nm(&self) -> Vec<f64> {
        self.inner.line_cds_nm.clone()
    }

    /// Final spaces (nm).
    #[getter]
    fn spaces_nm(&self) -> Vec<f64> {
        self.inner.spaces_nm.clone()
    }

    /// Centre-to-centre pitches (nm).
    #[getter]
    fn pitches_nm(&self) -> Vec<f64> {
        self.inner.pitches_nm.clone()
    }

    /// Nominal pitch (mandrel pitch / 2 or / 4) (nm).
    #[getter]
    fn nominal_pitch_nm(&self) -> f64 {
        self.inner.nominal_pitch_nm
    }

    /// Largest minus smallest pitch (nm).
    #[getter]
    fn pitch_walk_nm(&self) -> f64 {
        self.inner.pitch_walk_nm
    }

    /// Largest minus smallest line CD (nm).
    #[getter]
    fn cd_range_nm(&self) -> f64 {
        self.inner.cd_range_nm
    }

    /// Capability badge of the model behind this result (see
    /// docs/capability-matrix.md): "✅" implemented, "🔶" simplified, "🧪"
    /// theoretical; split badges name exact and approximate parts.
    #[getter]
    fn status(&self) -> &'static str {
        "🔶"
    }

    /// The model's stated approximations for this result.
    #[getter]
    fn notes(&self) -> Vec<&'static str> {
        vec!["purely geometric SADP/SAQP: no deposition, etch or lithography physics"]
    }

    fn __repr__(&self) -> String {
        format!(
            "SpacerPatterningResult(lines={}, nominal_pitch_nm={:.2}, pitch_walk_nm={:.3})",
            self.inner.pattern.len(),
            self.inner.nominal_pitch_nm,
            self.inner.pitch_walk_nm
        )
    }
}

fn parse_tone(tone: &str) -> PyResult<SpacerTone> {
    match tone {
        "spacer_is_line" => Ok(SpacerTone::SpacerIsLine),
        "spacer_is_dielectric" => Ok(SpacerTone::SpacerIsDielectric),
        other => Err(PyValueError::new_err(format!(
            "tone must be 'spacer_is_line' or 'spacer_is_dielectric', got '{other}'"
        ))),
    }
}

/// Geometric SADP: mandrel pitch, printed mandrel CD, spacer thickness.
#[pyfunction]
#[pyo3(signature = (mandrel_pitch_nm, mandrel_cd_nm, spacer_nm, tone="spacer_is_line"))]
fn sadp(
    mandrel_pitch_nm: f64,
    mandrel_cd_nm: f64,
    spacer_nm: f64,
    tone: &str,
) -> PyResult<PySpacerPatterningResult> {
    let inner = double_patterning::sadp(
        mandrel_pitch_nm,
        mandrel_cd_nm,
        spacer_nm,
        parse_tone(tone)?,
    )
    .map_err(value_err)?;
    Ok(PySpacerPatterningResult { inner })
}

/// Geometric SAQP: mandrel pitch, mandrel CD, first and second spacer.
#[pyfunction]
#[pyo3(signature = (mandrel_pitch_nm, mandrel_cd_nm, spacer1_nm, spacer2_nm, tone="spacer_is_line"))]
fn saqp(
    mandrel_pitch_nm: f64,
    mandrel_cd_nm: f64,
    spacer1_nm: f64,
    spacer2_nm: f64,
    tone: &str,
) -> PyResult<PySpacerPatterningResult> {
    let inner = double_patterning::saqp(
        mandrel_pitch_nm,
        mandrel_cd_nm,
        spacer1_nm,
        spacer2_nm,
        parse_tone(tone)?,
    )
    .map_err(value_err)?;
    Ok(PySpacerPatterningResult { inner })
}

// ---------------------------------------------------------------------------
// LELE double patterning
// ---------------------------------------------------------------------------

/// LELE (litho-etch-litho-etch) double patterning: two exposures of
/// complementary 2×-pitch gratings, the second shifted by the overlay error.
#[pyclass(name = "LeleResult", frozen)]
pub struct PyLeleResult {
    inner: double_patterning::DoublePatterningResult,
    mask1: Mask,
    mask2: Mask,
}

#[pymethods]
impl PyLeleResult {
    /// Aerial image of exposure 1 (relative intensity).
    #[getter]
    fn aerial1<'py>(slf: &Bound<'py, Self>) -> Bound<'py, PyArray2<f64>> {
        crate::py_arrays::readonly_view(&slf.get().inner.aerial1.data, slf.as_any())
    }

    /// Aerial image of exposure 2 as imaged (before the overlay shift).
    #[getter]
    fn aerial2<'py>(slf: &Bound<'py, Self>) -> Bound<'py, PyArray2<f64>> {
        crate::py_arrays::readonly_view(&slf.get().inner.aerial2.data, slf.as_any())
    }

    /// Aerial image of exposure 2 translated by the overlay error (Fourier
    /// shift, sub-pixel exact).
    #[getter]
    fn aerial2_overlay<'py>(slf: &Bound<'py, Self>) -> Bound<'py, PyArray2<f64>> {
        crate::py_arrays::readonly_view(&slf.get().inner.aerial2_overlay.data, slf.as_any())
    }

    /// Double-exposure composite `(d₁ I₁ + d₂ I₂′)/(d₁ + d₂)` — what ONE
    /// resist exposed twice would see; NOT the LELE result (see
    /// `printed_pattern`).
    #[getter]
    fn combined_aerial<'py>(slf: &Bound<'py, Self>) -> Bound<'py, PyArray2<f64>> {
        crate::py_arrays::readonly_view(&slf.get().inner.combined_aerial.data, slf.as_any())
    }

    /// Contrast of the double-exposure composite.
    #[getter]
    fn combined_contrast(&self) -> f64 {
        self.inner.combined_contrast
    }

    /// Exposure doses `(d1, d2)` in mJ/cm².
    #[getter]
    fn doses_mj_cm2(&self) -> (f64, f64) {
        self.inner.doses_mj_cm2
    }

    /// Pixel-centre x coordinates (nm) of the images.
    #[getter]
    fn x_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let g = &self.inner.aerial1;
        let x: Vec<f64> = (0..g.nx()).map(|j| g.x_at(j)).collect();
        x.into_pyarray(py)
    }

    /// The two masks (every other line of the target grating).
    #[getter]
    fn masks(&self) -> (PyMaskConfig, PyMaskConfig) {
        (
            PyMaskConfig {
                inner: self.mask1.clone(),
            },
            PyMaskConfig {
                inner: self.mask2.clone(),
            },
        )
    }

    /// Printed LELE pattern (1 = resist line remains) for a resist clearing
    /// energy `threshold_mj_cm2` (E_th): a line remains where
    /// `d₁·I₁ < E_th` or `d₂·I₂′ < E_th`.
    fn printed_pattern<'py>(
        &self,
        py: Python<'py>,
        threshold_mj_cm2: f64,
    ) -> Bound<'py, PyArray2<f64>> {
        double_patterning::lele_printed_pattern(&self.inner, threshold_mj_cm2).into_pyarray(py)
    }

    /// Printed line intervals `(x0_nm, x1_nm)` along image row `row` (default:
    /// the centre row), sub-pixel edges; lines cut by the field edge are
    /// dropped.
    #[pyo3(signature = (threshold_mj_cm2, row=None))]
    fn lines(&self, threshold_mj_cm2: f64, row: Option<usize>) -> PyResult<Vec<(f64, f64)>> {
        let ny = self.inner.aerial1.data.nrows();
        let row = row.unwrap_or(ny / 2);
        if row >= ny {
            return Err(value_err(format!("row must be < {ny}, got {row}")));
        }
        Ok(double_patterning::lele_cut(
            &self.inner,
            threshold_mj_cm2,
            row,
        ))
    }

    /// Capability badge of the model behind this result (see
    /// docs/capability-matrix.md): "✅" implemented, "🔶" simplified, "🧪"
    /// theoretical; split badges name exact and approximate parts.
    #[getter]
    fn status(&self) -> &'static str {
        "🔶"
    }

    /// The model's stated approximations for this result.
    #[getter]
    fn notes(&self) -> Vec<&'static str> {
        vec![
            "two independent exposures with per-exposure thresholds; printed lines are the union",
            "no freeze/etch step or resist interaction between the exposures",
        ]
    }

    fn __repr__(&self) -> String {
        format!(
            "LeleResult(doses={:?} mJ/cm^2, combined_contrast={:.3})",
            self.inner.doses_mj_cm2, self.inner.combined_contrast
        )
    }
}

/// LELE double patterning of opaque lines of width `cd_nm` at `pitch_nm`:
/// each mask carries every other line (a `2·pitch_nm` grating, mask 2 offset
/// by `pitch_nm`); both are imaged on `grid` (commensurate with
/// `2·pitch_nm`), exposure 2 is shifted by `(overlay_x_nm, overlay_y_nm)`.
/// Two independent resist exposures (per-exposure threshold, union of the
/// printed lines); no freeze/etch or resist interaction (🔶).
#[pyfunction]
#[pyo3(signature = (
    source, optics, grid, cd_nm, pitch_nm, overlay_x_nm=0.0, overlay_y_nm=0.0,
    dose1_mj_cm2=30.0, dose2_mj_cm2=30.0, focus1_nm=0.0, focus2_nm=0.0, max_kernels=16,
    illumination=None,
))]
#[allow(clippy::too_many_arguments)]
fn simulate_lele(
    py: Python<'_>,
    source: PySourceConfig,
    optics: PyOpticsConfig,
    grid: PyGridConfig,
    cd_nm: f64,
    pitch_nm: f64,
    overlay_x_nm: f64,
    overlay_y_nm: f64,
    dose1_mj_cm2: f64,
    dose2_mj_cm2: f64,
    focus1_nm: f64,
    focus2_nm: f64,
    max_kernels: usize,
    illumination: Option<Bound<'_, PyTuple>>,
) -> PyResult<PyLeleResult> {
    let (mask1, mask2) = double_patterning::split_mask_lele(cd_nm, pitch_nm).map_err(value_err)?;
    crate::py_config::warn_if_incommensurate(&mask1, &grid.inner)?;
    let engine = build_engine(
        py,
        &source,
        &optics,
        &grid,
        max_kernels,
        illumination.as_ref(),
    )?;
    let config = double_patterning::DoublePatterningConfig {
        overlay_x_nm,
        overlay_y_nm,
        dose1_mj_cm2,
        dose2_mj_cm2,
        focus1_nm,
        focus2_nm,
    };
    let inner = py.allow_threads(|| {
        double_patterning::simulate_double_patterning(&engine, &mask1, &mask2, &config)
    });
    Ok(PyLeleResult {
        inner,
        mask1,
        mask2,
    })
}

// ---------------------------------------------------------------------------
// Directed self-assembly (analytic, 🔶)
// ---------------------------------------------------------------------------

/// Directed self-assembly result (analytic morphology, not SCFT).
#[pyclass(name = "DsaResult", frozen)]
pub struct PyDsaResult {
    inner: dsa::DSAResult,
}

fn commensurability_dict<'py>(
    py: Python<'py>,
    r: &dsa::CommensurabilityReport,
) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
    let d = pyo3::types::PyDict::new(py);
    d.set_item("confinement_nm", r.confinement_nm)?;
    d.set_item("periods", r.periods)?;
    d.set_item("period_nm", r.period_nm)?;
    d.set_item("strain", r.strain)?;
    d.set_item("free_energy_ratio", r.free_energy_ratio)?;
    Ok(d)
}

#[pymethods]
impl PyDsaResult {
    /// A-block fraction map `(ny, nx)` (1 = A, 0 = B or guiding wall).
    #[getter]
    fn pattern<'py>(slf: &Bound<'py, Self>) -> Bound<'py, PyArray2<f64>> {
        crate::py_arrays::readonly_view(&slf.get().inner.pattern, slf.as_any())
    }

    /// ILLUSTRATIVE heuristic defect index `100·min(10·max|W/L₀ − n|, 1)`
    /// of the worst trench — not calibrated to measured defectivity; NaN when
    /// no commensurability was evaluated (cylinders, spheres, free lamellae).
    #[getter]
    fn heuristic_defect_index(&self) -> f64 {
        self.inner.defect_density
    }

    /// `heuristic_defect_index < 1` (False when not evaluated).
    #[getter]
    fn is_defect_free(&self) -> bool {
        self.inner.is_defect_free
    }

    /// Lamellae: A-domain width f·L of the first trench; cylinders/spheres:
    /// domain diameter (nm).
    #[getter]
    fn assembled_cd_nm(&self) -> f64 {
        self.inner.assembled_cd_nm
    }

    /// One dict per distinct confining trench (lamellae only):
    /// confinement_nm, periods, period_nm, strain, free_energy_ratio.
    fn commensurability<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Vec<Bound<'py, pyo3::types::PyDict>>> {
        self.inner
            .commensurability
            .iter()
            .map(|r| commensurability_dict(py, r))
            .collect()
    }

    /// Capability badge of the model behind this result (see
    /// docs/capability-matrix.md): "✅" implemented, "🔶" simplified, "🧪"
    /// theoretical; split badges name exact and approximate parts.
    #[getter]
    fn status(&self) -> &'static str {
        "🔶"
    }

    /// The model's stated approximations for this result.
    #[getter]
    fn notes(&self) -> Vec<&'static str> {
        vec![
            "analytic morphologies with strong-segregation commensurability, not SCFT",
            "heuristic_defect_index is an uncalibrated heuristic",
        ]
    }

    fn __repr__(&self) -> String {
        format!(
            "DsaResult(assembled_cd_nm={:.2}, heuristic_defect_index={:.3})",
            self.inner.assembled_cd_nm, self.inner.defect_density
        )
    }
}

/// Analytic directed self-assembly on a template (`(ny, nx)` array, > 0.5 =
/// open trench, else guiding wall) with pixel `pixel_nm`: lamellae (stripes
/// along y) registered to each row's trench walls with the free-energy-
/// optimal period count; cylinders (hexagonal) and spheres (simple cubic) as
/// free-running lattices clipped to the open region. `interface_width_nm`
/// defaults to L₀/10. χN must exceed the order–disorder transition 10.495.
#[pyfunction]
#[pyo3(signature = (
    template, pixel_nm, l0_nm=28.0, chi_n=20.0, volume_fraction=0.5,
    morphology="lamellar", interface_width_nm=None,
))]
#[allow(clippy::too_many_arguments)]
fn simulate_dsa(
    py: Python<'_>,
    template: PyReadonlyArray2<'_, f64>,
    pixel_nm: f64,
    l0_nm: f64,
    chi_n: f64,
    volume_fraction: f64,
    morphology: &str,
    interface_width_nm: Option<f64>,
) -> PyResult<PyDsaResult> {
    if !(pixel_nm.is_finite() && pixel_nm > 0.0) {
        return Err(value_err(format!(
            "pixel_nm must be positive, got {pixel_nm}"
        )));
    }
    let morphology = match morphology {
        "lamellar" => dsa::DSAMorphology::Lamellar,
        "cylindrical" | "cylinders" => dsa::DSAMorphology::Cylindrical,
        "spherical" | "spheres" => dsa::DSAMorphology::Spherical,
        other => {
            return Err(value_err(format!(
                "morphology must be 'lamellar', 'cylindrical' or 'spherical', got '{other}'"
            )))
        }
    };
    let params = dsa::DSAParams {
        l0_nm,
        chi_n,
        volume_fraction,
        morphology,
        interface_width_nm: interface_width_nm.unwrap_or(l0_nm / 10.0),
    };
    params.validate().map_err(value_err)?;
    let template = template.as_array().to_owned();
    let inner = py.allow_threads(|| dsa::simulate_dsa_2d(&template, pixel_nm, &params));
    Ok(PyDsaResult { inner })
}

/// Lamellae confined in one trench of width `trench_nm`: the free-energy-
/// optimal number of periods and its strain, as a dict (confinement_nm,
/// periods, period_nm, strain, free_energy_ratio =
/// F(L)/F(L₀) = (λ² + 2/λ)/3 with λ = L/L₀; strong-segregation theory).
#[pyfunction]
fn dsa_confined_lamellae<'py>(
    py: Python<'py>,
    trench_nm: f64,
    l0_nm: f64,
) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
    if !(trench_nm.is_finite() && trench_nm > 0.0 && l0_nm.is_finite() && l0_nm > 0.0) {
        return Err(value_err(format!(
            "trench_nm and l0_nm must be positive, got {trench_nm}, {l0_nm}"
        )));
    }
    commensurability_dict(py, &dsa::confined_lamellae(trench_nm, l0_nm))
}

/// Register the optimization / patterning classes and functions.
pub fn register_optim_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyIltResult>()?;
    m.add_class::<PyOpcResult>()?;
    m.add_class::<PySrafResult>()?;
    m.add_class::<PyDofComparison>()?;
    m.add_class::<PySpacerPatterningResult>()?;

    m.add_function(wrap_pyfunction!(mask_from_features, m)?)?;
    m.add_function(wrap_pyfunction!(ilt_contact_target, m)?)?;
    m.add_function(wrap_pyfunction!(optimize_ilt, m)?)?;
    m.add_function(wrap_pyfunction!(fragment_opc, m)?)?;
    m.add_function(wrap_pyfunction!(insert_srafs, m)?)?;
    m.add_function(wrap_pyfunction!(dose_to_size_threshold, m)?)?;
    m.add_function(wrap_pyfunction!(compare_sraf_dof, m)?)?;
    m.add_function(wrap_pyfunction!(sadp, m)?)?;
    m.add_function(wrap_pyfunction!(saqp, m)?)?;

    m.add_class::<PyLeleResult>()?;
    m.add_class::<PyDsaResult>()?;
    m.add_function(wrap_pyfunction!(simulate_lele, m)?)?;
    m.add_function(wrap_pyfunction!(simulate_dsa, m)?)?;
    m.add_function(wrap_pyfunction!(dsa_confined_lamellae, m)?)?;
    Ok(())
}
