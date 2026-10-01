use numpy::{IntoPyArray, PyArray1, PyReadonlyArray2, ToPyArray};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use highuvlith_core::aerial::AerialImageEngine;
use highuvlith_core::process::{ProcessRectangle, ProcessWindow, ThresholdResist};
use highuvlith_core::types::GridConfig;

use crate::py_config::*;
use crate::py_results::{parse_tone, tone_name};
use crate::py_vector::PyVectorSettings;

/// Batch process-window / focus sweeps over one imaging engine. Every
/// method (including construction) releases the GIL; focus planes run in
/// parallel inside the engine.
#[pyclass(name = "BatchSimulator")]
pub struct PyBatchSimulator {
    engine: AerialImageEngine,
    mask: highuvlith_core::mask::Mask,
    _grid: GridConfig,
}

#[pymethods]
impl PyBatchSimulator {
    #[new]
    #[pyo3(signature = (
        source, optics, mask, grid=None, max_kernels=30, defocus_model="exact",
        kernel_energy_fraction=1.0, source_points_per_axis=None, vector=None,
        normalization="clear_field", kernel_cache_capacity=32
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        py: Python<'_>,
        source: PySourceConfig,
        optics: PyOpticsConfig,
        mask: PyMaskConfig,
        grid: Option<PyGridConfig>,
        max_kernels: usize,
        defocus_model: &str,
        kernel_energy_fraction: f64,
        source_points_per_axis: Option<usize>,
        vector: Option<PyVectorSettings>,
        normalization: &str,
        kernel_cache_capacity: usize,
    ) -> PyResult<Self> {
        let grid_config = grid.map(|g| g.inner).unwrap_or_default();
        crate::py_config::warn_if_incommensurate(&mask.inner, &grid_config)?;
        let settings = crate::py_simulation::imaging_settings(
            max_kernels,
            defocus_model,
            kernel_energy_fraction,
            source_points_per_axis,
            vector.as_ref(),
            normalization,
            kernel_cache_capacity,
        )?;
        let engine = crate::py_simulation::build_engine_nogil(
            py,
            &source,
            &optics,
            grid_config.clone(),
            settings,
        )?;

        Ok(Self {
            engine,
            mask: mask.inner,
            _grid: grid_config,
        })
    }

    /// Compute the process window: CD vs dose and focus.
    ///
    /// Dose-aware constant-threshold resist: `cd_threshold` is the intensity
    /// threshold at the nominal dose (the median of `doses`), so at dose `d`
    /// the printed edge is the contour `cd_threshold · d_nom / d`.
    #[pyo3(signature = (
        doses,
        focuses,
        cd_threshold=0.3,
        cd_target_nm=65.0,
        cd_tolerance_pct=10.0
    ))]
    fn process_window<'py>(
        &self,
        py: Python<'py>,
        doses: Vec<f64>,
        focuses: Vec<f64>,
        cd_threshold: f64,
        cd_target_nm: f64,
        cd_tolerance_pct: f64,
    ) -> PyResult<PyProcessWindowResult> {
        let pw = py
            .allow_threads(|| {
                ProcessWindow::compute(
                    &self.engine,
                    &self.mask,
                    &doses,
                    &focuses,
                    cd_threshold,
                    cd_target_nm,
                    cd_tolerance_pct,
                )
            })
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(PyProcessWindowResult { inner: pw })
    }

    /// Compute the process window with an explicit constant-threshold resist:
    /// a point clears where `dose · I ≥ dose_to_clear_mj_cm2` (I normalized
    /// to the clear field).
    #[pyo3(signature = (
        doses,
        focuses,
        dose_to_clear_mj_cm2,
        cd_target_nm=65.0,
        cd_tolerance_pct=10.0
    ))]
    fn process_window_threshold<'py>(
        &self,
        py: Python<'py>,
        doses: Vec<f64>,
        focuses: Vec<f64>,
        dose_to_clear_mj_cm2: f64,
        cd_target_nm: f64,
        cd_tolerance_pct: f64,
    ) -> PyResult<PyProcessWindowResult> {
        let resist = ThresholdResist::new(dose_to_clear_mj_cm2)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let pw = py
            .allow_threads(|| {
                ProcessWindow::compute_threshold_model(
                    &self.engine,
                    &self.mask,
                    &doses,
                    &focuses,
                    &resist,
                    cd_target_nm,
                    cd_tolerance_pct,
                )
            })
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(PyProcessWindowResult { inner: pw })
    }

    /// Aerial images at several focus values (nm), as a list of
    /// `(focus_nm, intensity)` tuples; each array is handed to numpy without
    /// a copy.
    fn batch_defocus<'py>(
        &self,
        py: Python<'py>,
        focuses: Vec<f64>,
    ) -> PyResult<Vec<(f64, Bound<'py, numpy::PyArray2<f64>>)>> {
        let results = py.allow_threads(|| {
            highuvlith_core::process::batch_defocus(&self.engine, &self.mask, &focuses)
        });

        Ok(results
            .into_iter()
            .map(|(f, arr)| (f, arr.into_pyarray(py)))
            .collect())
    }
}

/// Process window analysis result.
#[pyclass(name = "ProcessWindowResult")]
pub struct PyProcessWindowResult {
    inner: ProcessWindow,
}

/// A process-window rectangle as a Python dict.
fn rectangle_dict<'py>(py: Python<'py>, r: &ProcessRectangle) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("focus_min_nm", r.focus_min_nm)?;
    d.set_item("focus_max_nm", r.focus_max_nm)?;
    d.set_item("dose_min_mj_cm2", r.dose_min_mj_cm2)?;
    d.set_item("dose_max_mj_cm2", r.dose_max_mj_cm2)?;
    d.set_item("dof_nm", r.dof_nm())?;
    d.set_item("exposure_latitude_pct", r.exposure_latitude_pct())?;
    d.set_item("focus_centre_nm", r.focus_centre_nm())?;
    d.set_item("dose_centre_mj_cm2", r.dose_centre_mj_cm2())?;
    Ok(d)
}

fn rectangle_or_none<'py>(
    py: Python<'py>,
    r: Option<ProcessRectangle>,
) -> PyResult<Option<Bound<'py, PyDict>>> {
    r.map(|r| rectangle_dict(py, &r)).transpose()
}

#[pymethods]
impl PyProcessWindowResult {
    /// Build a result from a tabulated focus–exposure matrix
    /// (`cd_matrix[dose_index, focus_index]`, NaN = not measurable), e.g.
    /// measured wafer data; CD is interpolated linearly in dose.
    #[staticmethod]
    #[pyo3(signature = (doses, focuses, cd_matrix, cd_target_nm=65.0, cd_tolerance_pct=10.0))]
    fn from_cd_matrix(
        doses: Vec<f64>,
        focuses: Vec<f64>,
        cd_matrix: PyReadonlyArray2<f64>,
        cd_target_nm: f64,
        cd_tolerance_pct: f64,
    ) -> PyResult<Self> {
        let m = cd_matrix.as_array().to_owned();
        let inner =
            ProcessWindow::from_cd_matrix(&doses, &focuses, m, cd_target_nm, cd_tolerance_pct)
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Build a result from precomputed y = 0 intensity cross-sections
    /// (`profiles[focus_index, pixel]`, one full field period each, at the
    /// uniform pixel centres `x_nm`) with a constant-threshold resist.
    #[staticmethod]
    #[pyo3(signature = (
        x_nm,
        focuses,
        profiles,
        doses,
        dose_to_clear_mj_cm2,
        tone="dark",
        cd_target_nm=65.0,
        cd_tolerance_pct=10.0
    ))]
    #[allow(clippy::too_many_arguments)]
    fn from_profiles(
        x_nm: Vec<f64>,
        focuses: Vec<f64>,
        profiles: PyReadonlyArray2<f64>,
        doses: Vec<f64>,
        dose_to_clear_mj_cm2: f64,
        tone: &str,
        cd_target_nm: f64,
        cd_tolerance_pct: f64,
    ) -> PyResult<Self> {
        let tone = parse_tone(tone)?;
        let resist = ThresholdResist::new(dose_to_clear_mj_cm2)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let rows: Vec<Vec<f64>> = profiles
            .as_array()
            .rows()
            .into_iter()
            .map(|r| r.to_vec())
            .collect();
        let inner = ProcessWindow::from_profiles(
            x_nm,
            &focuses,
            rows,
            &doses,
            resist,
            tone,
            cd_target_nm,
            cd_tolerance_pct,
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Depth of focus (nm) at the dose-to-size around best focus (in-spec
    /// focus range, spec crossings interpolated between focus samples).
    fn depth_of_focus(&self) -> f64 {
        self.inner.depth_of_focus()
    }

    /// Exposure latitude (%) at best focus (continuous in dose).
    fn exposure_latitude(&self) -> f64 {
        self.inner.exposure_latitude()
    }

    /// Get CD matrix as numpy array (doses x focuses); NaN where nothing
    /// prints.
    #[getter]
    fn cd_matrix<'py>(&self, py: Python<'py>) -> Bound<'py, numpy::PyArray2<f64>> {
        self.inner.cd_matrix.to_pyarray(py)
    }

    /// Get dose values.
    #[getter]
    fn doses<'py>(&self, py: Python<'py>) -> Bound<'py, numpy::PyArray1<f64>> {
        self.inner.doses.to_pyarray(py)
    }

    /// Get focus values.
    #[getter]
    fn focuses<'py>(&self, py: Python<'py>) -> Bound<'py, numpy::PyArray1<f64>> {
        self.inner.focuses.to_pyarray(py)
    }

    /// Per focus: in-spec dose interval `(dose_min, dose_max)` in mJ/cm², or
    /// None where the CD is in spec at no dose.
    #[getter]
    fn dose_limits(&self) -> Vec<Option<(f64, f64)>> {
        self.inner.dose_limits.clone()
    }

    /// Measured feature tone ("dark"/"bright"), None for tabulated data.
    #[getter]
    fn tone(&self) -> Option<&'static str> {
        self.inner.tone.map(tone_name)
    }

    /// Open-frame dose to clear E_th (mJ/cm²) of the constant-threshold
    /// resist, None for tabulated data.
    #[getter]
    fn dose_to_clear_mj_cm2(&self) -> Option<f64> {
        self.inner.resist.map(|r| r.dose_to_clear_mj_cm2)
    }

    /// Target CD (nm).
    #[getter]
    fn cd_target_nm(&self) -> f64 {
        self.inner.cd_target_nm
    }

    /// CD tolerance (± %).
    #[getter]
    fn cd_tolerance_pct(&self) -> f64 {
        self.inner.cd_tolerance_pct
    }

    /// Nominal dose: median of the swept doses (mJ/cm²).
    fn nominal_dose(&self) -> f64 {
        self.inner.nominal_dose()
    }

    /// Best focus (nm): zero-slope point of the Bossung curve at the
    /// dose-to-size.
    fn best_focus(&self) -> f64 {
        self.inner.best_focus()
    }

    /// Dose-to-size (mJ/cm²) at best focus.
    fn dose_to_size(&self) -> Option<f64> {
        self.inner.dose_to_size()
    }

    /// Iso-focal dose (mJ/cm²): least focus-sensitive CD in the swept range.
    fn iso_focal_dose(&self) -> Option<f64> {
        self.inner.iso_focal_dose()
    }

    /// CD (nm) at any dose and focus (exact in dose for model data, linear
    /// between focus samples).
    fn cd_at(&self, dose_mj_cm2: f64, focus_nm: f64) -> Option<f64> {
        self.inner.cd_at(dose_mj_cm2, focus_nm)
    }

    /// Exposure–defocus curve: `(dof_nm, max_el_pct)` arrays over
    /// `n_points` DOF values from 0 to the widest in-spec focus run.
    #[pyo3(signature = (n_points=51))]
    fn el_vs_dof<'py>(
        &self,
        py: Python<'py>,
        n_points: usize,
    ) -> (Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>) {
        let curve = self.inner.el_vs_dof(n_points);
        let dof: Vec<f64> = curve.iter().map(|c| c.0).collect();
        let el: Vec<f64> = curve.iter().map(|c| c.1).collect();
        (dof.into_pyarray(py), el.into_pyarray(py))
    }

    /// Largest-DOF process-window rectangle with exposure latitude ≥ `el_pct`
    /// (e.g. DOF at 5 % EL) as a dict, or None.
    #[pyo3(signature = (el_pct=5.0))]
    fn dof_at_el<'py>(&self, py: Python<'py>, el_pct: f64) -> PyResult<Option<Bound<'py, PyDict>>> {
        rectangle_or_none(py, self.inner.dof_at_el(el_pct))
    }

    /// Largest-EL process-window rectangle of focus extent `dof_nm` as a
    /// dict, or None.
    fn el_at_dof<'py>(&self, py: Python<'py>, dof_nm: f64) -> PyResult<Option<Bound<'py, PyDict>>> {
        rectangle_or_none(py, self.inner.el_at_dof(dof_nm))
    }

    /// Process-window rectangle maximizing DOF × EL, as a dict, or None.
    fn max_area_rectangle<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyDict>>> {
        rectangle_or_none(py, self.inner.max_area_rectangle())
    }

    /// Lower and upper CD spec limits `(cd_min_nm, cd_max_nm)`.
    fn spec_limits_nm(&self) -> (f64, f64) {
        self.inner.spec_limits_nm()
    }

    /// Dose-to-size (mJ/cm²) at any focus inside the sampled range (linear
    /// between focus samples); None where the target CD is not reached.
    fn dose_to_size_at(&self, focus_nm: f64) -> Option<f64> {
        self.inner.dose_to_size_at(focus_nm)
    }

    /// In-spec dose interval `(dose_min, dose_max)` (mJ/cm²) at any focus
    /// inside the sampled range, or None.
    fn dose_limits_at(&self, focus_nm: f64) -> Option<(f64, f64)> {
        self.inner.dose_limits_at(focus_nm)
    }

    /// Bossung curves: one list of `(focus_nm, cd_nm)` per sampled dose
    /// (NaN = not printed), as `[(dose_mj_cm2, [(focus_nm, cd_nm), ...]),
    /// ...]`.
    #[allow(clippy::type_complexity)]
    fn bossung_curves(&self) -> Vec<(f64, Vec<(f64, f64)>)> {
        self.inner
            .bossung_curves()
            .into_iter()
            .map(|curve| {
                let dose = curve.first().map_or(f64::NAN, |p| p.dose_mj_cm2);
                (dose, curve.iter().map(|p| (p.focus_nm, p.cd_nm)).collect())
            })
            .collect()
    }

    /// Key process-window numbers in one dict.
    fn summary<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        d.set_item("nominal_dose_mj_cm2", self.inner.nominal_dose())?;
        d.set_item("dose_to_clear_mj_cm2", self.dose_to_clear_mj_cm2())?;
        d.set_item("tone", self.tone())?;
        d.set_item("cd_target_nm", self.inner.cd_target_nm)?;
        d.set_item("cd_tolerance_pct", self.inner.cd_tolerance_pct)?;
        d.set_item("best_focus_nm", self.inner.best_focus())?;
        d.set_item("dose_to_size_mj_cm2", self.inner.dose_to_size())?;
        d.set_item("depth_of_focus_nm", self.inner.depth_of_focus())?;
        d.set_item("exposure_latitude_pct", self.inner.exposure_latitude())?;
        d.set_item("iso_focal_dose_mj_cm2", self.inner.iso_focal_dose())?;
        d.set_item(
            "dof_at_5pct_el",
            rectangle_or_none(py, self.inner.dof_at_el(5.0))?,
        )?;
        Ok(d)
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
            "constant-threshold resist: a point clears where dose x intensity >= E_th; no blur, diffusion or development",
            "rectangular process windows only (no ellipse); CD on the y = 0 cut",
        ]
    }

    fn __repr__(&self) -> String {
        format!(
            "ProcessWindowResult(DOF={:.1}nm, EL={:.1}%, best_focus={:.1}nm)",
            self.inner.depth_of_focus(),
            self.inner.exposure_latitude(),
            self.inner.best_focus()
        )
    }
}
