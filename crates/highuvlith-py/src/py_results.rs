use numpy::{IntoPyArray, PyArray1, PyArray2, ToPyArray};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use highuvlith_core::metrics::{self, FeatureTone};
use highuvlith_core::resist::ResistProfile;
use highuvlith_core::types::Grid2D;

/// Aerial image simulation result.
///
/// Frozen: the intensity is written once and never changes, so `intensity`
/// can hand numpy a read-only view of the Rust buffer (no copy).
#[pyclass(name = "AerialImageResult", frozen)]
#[derive(Debug, Clone)]
pub struct PyAerialImageResult {
    pub intensity: ndarray::Array2<f64>,
    pub x_min_nm: f64,
    pub x_max_nm: f64,
    pub y_min_nm: f64,
    pub y_max_nm: f64,
}

/// Parse a feature tone name: `"dark"` (feature below threshold, e.g. an
/// opaque line) or `"bright"` (feature above threshold, e.g. a space or
/// contact hole).
pub(crate) fn parse_tone(tone: &str) -> PyResult<FeatureTone> {
    match tone.to_ascii_lowercase().as_str() {
        "dark" => Ok(FeatureTone::Dark),
        "bright" => Ok(FeatureTone::Bright),
        other => Err(PyValueError::new_err(format!(
            "tone must be 'dark' or 'bright', got '{other}'"
        ))),
    }
}

/// Python-facing name of a feature tone.
pub(crate) fn tone_name(tone: FeatureTone) -> &'static str {
    match tone {
        FeatureTone::Dark => "dark",
        FeatureTone::Bright => "bright",
    }
}

impl PyAerialImageResult {
    pub fn from_grid2d(grid: Grid2D<f64>) -> Self {
        Self {
            intensity: grid.data,
            x_min_nm: grid.x_min_nm,
            x_max_nm: grid.x_max_nm,
            y_min_nm: grid.y_min_nm,
            y_max_nm: grid.y_max_nm,
        }
    }

    /// Pixel-centre x coordinates and the y = 0 intensity cross-section.
    fn y0_profile(&self) -> (Vec<f64>, Vec<f64>) {
        let x = metrics::pixel_centres(self.intensity.ncols(), self.x_min_nm, self.x_max_nm);
        (x, metrics::centre_profile(&self.intensity))
    }
}

#[pymethods]
impl PyAerialImageResult {
    /// Intensity `(ny, nx)` (relative to the clear field unless the engine
    /// uses `normalization="absolute"`), indexed `[y, x]`. A read-only numpy
    /// view of the result's buffer: no copy is made; use `.copy()` for a
    /// writable array.
    #[getter]
    fn intensity<'py>(slf: &Bound<'py, Self>) -> Bound<'py, PyArray2<f64>> {
        crate::py_arrays::readonly_view(&slf.get().intensity, slf.as_any())
    }

    /// Pixel-centre x coordinates in nm (length nx).
    #[getter]
    fn x_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let nx = self.intensity.ncols();
        let pixel = (self.x_max_nm - self.x_min_nm) / nx as f64;
        let x: Vec<f64> = (0..nx)
            .map(|j| self.x_min_nm + (j as f64 + 0.5) * pixel)
            .collect();
        x.into_pyarray(py)
    }

    /// Pixel-centre y coordinates in nm (length ny).
    #[getter]
    fn y_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let ny = self.intensity.nrows();
        let pixel = (self.y_max_nm - self.y_min_nm) / ny as f64;
        let y: Vec<f64> = (0..ny)
            .map(|i| self.y_min_nm + (i as f64 + 0.5) * pixel)
            .collect();
        y.into_pyarray(py)
    }

    /// Pixel size along x in nm.
    #[getter]
    fn pixel_nm(&self) -> f64 {
        (self.x_max_nm - self.x_min_nm) / self.intensity.ncols() as f64
    }

    /// Field edges `(x_min, x_max, y_min, y_max)` in nm — the `extent` for
    /// `matplotlib.pyplot.imshow(result.intensity, origin="lower", ...)`.
    #[getter]
    fn extent_nm(&self) -> (f64, f64, f64, f64) {
        (self.x_min_nm, self.x_max_nm, self.y_min_nm, self.y_max_nm)
    }

    /// `(ny, nx)` shape of the intensity grid.
    #[getter]
    fn shape(&self) -> (usize, usize) {
        self.intensity.dim()
    }

    /// Extract a 1D cross-section along x at y=y_nm.
    #[pyo3(signature = (y_nm=0.0))]
    fn cross_section<'py>(
        &self,
        py: Python<'py>,
        y_nm: f64,
    ) -> (Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>) {
        let ny = self.intensity.nrows();
        let nx = self.intensity.ncols();
        let pixel_y = (self.y_max_nm - self.y_min_nm) / ny as f64;
        let pixel_x = (self.x_max_nm - self.x_min_nm) / nx as f64;

        // Find nearest row
        let row_idx = ((y_nm - self.y_min_nm) / pixel_y - 0.5)
            .round()
            .clamp(0.0, (ny - 1) as f64) as usize;

        let x: Vec<f64> = (0..nx)
            .map(|j| self.x_min_nm + (j as f64 + 0.5) * pixel_x)
            .collect();
        let vals: Vec<f64> = (0..nx).map(|j| self.intensity[[row_idx, j]]).collect();

        (x.into_pyarray(py), vals.into_pyarray(py))
    }

    /// Compute image contrast: (Imax - Imin) / (Imax + Imin).
    fn image_contrast(&self) -> f64 {
        metrics::image_contrast(&self.intensity)
    }

    /// Compute NILS at the y = 0 cross-section (legacy: the feature straddling
    /// the field centre, whatever its tone; slope taken exactly at the
    /// threshold crossing). Prefer `nils_periodic`.
    #[pyo3(signature = (threshold=0.3))]
    fn nils(&self, threshold: f64) -> Option<f64> {
        let (x_nm, profile) = self.y0_profile();
        metrics::nils(&profile, &x_nm, threshold)
    }

    /// Printed CD (nm) at intensity `threshold`: width of the `tone` feature
    /// ("dark" = below threshold, e.g. a line; "bright" = above, e.g. a
    /// space/hole) nearest the field centre on the y = 0 cross-section,
    /// wrap-aware across the periodic field edge. None if nothing prints.
    #[pyo3(signature = (threshold=0.3, tone="dark"))]
    fn cd(&self, threshold: f64, tone: &str) -> PyResult<Option<f64>> {
        let tone = parse_tone(tone)?;
        let (x_nm, profile) = self.y0_profile();
        Ok(metrics::measure_cd_periodic(
            &profile, &x_nm, threshold, tone,
        ))
    }

    /// NILS = w·|dI/dx|/I_threshold of the `tone` feature nearest the field
    /// centre (slopes exactly at its two threshold crossings, averaged);
    /// `w` is the measured CD unless `width_nm` (e.g. the target CD) is given.
    #[pyo3(signature = (threshold=0.3, tone="dark", width_nm=None))]
    fn nils_periodic(
        &self,
        threshold: f64,
        tone: &str,
        width_nm: Option<f64>,
    ) -> PyResult<Option<f64>> {
        let tone = parse_tone(tone)?;
        let (x_nm, profile) = self.y0_profile();
        Ok(metrics::nils_periodic(
            &profile, &x_nm, threshold, tone, width_nm,
        ))
    }

    /// Image log-slope |d ln I/dx| (1/nm) at the edges of the `tone` feature
    /// nearest the field centre.
    #[pyo3(signature = (threshold=0.3, tone="dark"))]
    fn image_log_slope(&self, threshold: f64, tone: &str) -> PyResult<Option<f64>> {
        let tone = parse_tone(tone)?;
        let (x_nm, profile) = self.y0_profile();
        Ok(metrics::image_log_slope(&profile, &x_nm, threshold, tone))
    }

    /// All `tone` features on the y = 0 cross-section as
    /// `(left_nm, right_nm, width_nm, centre_nm)` tuples (wrap-aware; a
    /// feature crossing the field edge has `right_nm` beyond the field).
    #[pyo3(signature = (threshold=0.3, tone="dark"))]
    fn features(&self, threshold: f64, tone: &str) -> PyResult<Vec<(f64, f64, f64, f64)>> {
        let tone = parse_tone(tone)?;
        let (x_nm, profile) = self.y0_profile();
        Ok(metrics::periodic_features(&profile, &x_nm, threshold, tone)
            .into_iter()
            .map(|f| (f.left_nm, f.right_nm, f.width_nm, f.centre_nm))
            .collect())
    }

    fn __eq__(&self, other: &Self) -> bool {
        let bounds_eq = (self.x_min_nm - other.x_min_nm).abs() < 1e-9
            && (self.x_max_nm - other.x_max_nm).abs() < 1e-9
            && (self.y_min_nm - other.y_min_nm).abs() < 1e-9
            && (self.y_max_nm - other.y_max_nm).abs() < 1e-9;
        if !bounds_eq {
            return false;
        }
        if self.intensity.shape() != other.intensity.shape() {
            return false;
        }
        self.intensity
            .iter()
            .zip(other.intensity.iter())
            .all(|(a, b)| (a - b).abs() < 1e-12)
    }

    fn __repr__(&self) -> String {
        format!(
            "AerialImageResult({}x{}, x=[{:.1}, {:.1}]nm, y=[{:.1}, {:.1}]nm)",
            self.intensity.nrows(),
            self.intensity.ncols(),
            self.x_min_nm,
            self.x_max_nm,
            self.y_min_nm,
            self.y_max_nm,
        )
    }
}

/// Resist profile simulation result.
#[pyclass(name = "ResistProfileResult")]
#[derive(Debug, Clone)]
pub struct PyResistProfileResult {
    x_nm: Vec<f64>,
    height_nm: Vec<f64>,
    thickness_nm: f64,
}

impl PyResistProfileResult {
    pub fn from_profile(profile: ResistProfile) -> Self {
        Self {
            x_nm: profile.x_nm,
            height_nm: profile.height_nm,
            thickness_nm: profile.thickness_nm,
        }
    }
}

#[pymethods]
impl PyResistProfileResult {
    /// X positions in nm.
    #[getter]
    fn x_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        self.x_nm.to_pyarray(py)
    }

    /// Remaining resist height at each position in nm.
    #[getter]
    fn height_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        self.height_nm.to_pyarray(py)
    }

    /// Original resist thickness in nm.
    #[getter]
    fn thickness_nm(&self) -> f64 {
        self.thickness_nm
    }

    fn __eq__(&self, other: &Self) -> bool {
        if self.height_nm.len() != other.height_nm.len() {
            return false;
        }
        if (self.thickness_nm - other.thickness_nm).abs() > 1e-9 {
            return false;
        }
        self.height_nm
            .iter()
            .zip(other.height_nm.iter())
            .all(|(a, b)| (a - b).abs() < 1e-12)
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
            "depth-averaged Dill exposure, 2D Gaussian post-exposure bake",
            "centre-row vertical development only (no lateral front or sidewall)",
        ]
    }

    fn __repr__(&self) -> String {
        format!(
            "ResistProfileResult({} points, thickness={}nm)",
            self.x_nm.len(),
            self.thickness_nm
        )
    }
}
