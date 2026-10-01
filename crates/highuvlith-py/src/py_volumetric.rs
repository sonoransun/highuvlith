//! PyO3 bindings for the deep-layer (volumetric / z-resolved) lithography
//! modules: separable volumetric resist exposure and 3D development
//! ([`highuvlith_core::volumetric`]), LIGA deep-X-ray shadow printing
//! ([`highuvlith_core::deep_xray`]), grayscale 2.5D topography
//! ([`highuvlith_core::grayscale`]), multi-beam interference / two-photon
//! lattices ([`highuvlith_core::interference`]), and the theoretical quantum
//! N-photon aerial image ([`highuvlith_core::quantum`]). The volumetric
//! bindings cover exposure, the post-exposure bake (exact anisotropic
//! Gaussian and chemically amplified reaction–diffusion), and development by
//! fast marching or by the level-set moving boundary.
//!
//! 3D fields cross the boundary as numpy arrays with axis order
//! `(nz, ny, nx)` — depth first, matching `Grid3D::data`. `VolumetricResult`
//! and `HeightMapResult` are frozen and hand numpy read-only views of their
//! Rust buffers (no copy); results that contain volumes (LIGA, level set,
//! CAR bake, Talbot, EUV-IL) build those objects once and return the same
//! object on every access. Compute-heavy calls release the GIL with
//! `py.allow_threads`.

use numpy::{
    IntoPyArray, PyArray1, PyArray2, PyArray3, PyReadonlyArray1, PyReadonlyArray2,
    PyReadonlyArray3, ToPyArray,
};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

use highuvlith_core::aerial::AerialImageEngine;
use highuvlith_core::deep_xray;
use highuvlith_core::grayscale::{self, ContrastCurve, GrayscaleMap};
use highuvlith_core::interference::{self, ExposureKinetics, InterferenceSetup};
use highuvlith_core::mask::Mask;
use highuvlith_core::metrics;
use highuvlith_core::quantum::{compute_quantum_aerial_image, QuantumLithographyParams};
use highuvlith_core::resist::{CarParams, DiffusionBoundary, PebDiffusion};
use highuvlith_core::source::LithographySource;
use highuvlith_core::talbot::{
    self, ExposureMode as TalbotExposure, Grating, Propagation as TalbotPropagation, ResistMedium,
    Spectrum as TalbotSpectrum, TalbotSetup, TwoGratingInterference,
};
use highuvlith_core::types::{Complex64, Grid2D, Grid3D};
use highuvlith_core::volumetric::{
    self, DeveloperDepletion, DevelopmentOptions, LevelSetConfig, PebModel, SurfaceInhibition,
    VolumetricExposureConfig, VolumetricLatentImage,
};

use crate::py_config::*;

/// A z-resolved scalar field on a 3D grid (PAC concentration, absorbed dose,
/// or fast-marching arrival times, depending on how it was produced).
///
/// `values` is a numpy array of shape `(nz, ny, nx)`; the depth axis `z` runs
/// from the resist top downward. `x_nm`/`y_nm`/`z_nm` are the physical
/// cell-center coordinates along each axis. Frozen: `values` is a read-only
/// view of the Rust buffer (no copy).
#[pyclass(name = "VolumetricResult", frozen)]
#[derive(Clone)]
pub struct PyVolumetricResult {
    inner: Grid3D<f64>,
}

impl PyVolumetricResult {
    fn new(inner: Grid3D<f64>) -> Self {
        Self { inner }
    }

    /// A Python-owned result object, built once (nested results return the
    /// same object on every access).
    fn py_new(py: Python<'_>, inner: Grid3D<f64>) -> PyResult<Py<Self>> {
        Py::new(py, Self::new(inner))
    }
}

#[pymethods]
impl PyVolumetricResult {
    /// Wrap a `(nz, ny, nx)` float64 array (copied) as a volume with the
    /// given physical extents `(min, max)` in nm (z is depth from the resist
    /// top), e.g. to develop or bake a latent image computed elsewhere.
    #[staticmethod]
    #[pyo3(signature = (values, x_range_nm, y_range_nm, z_range_nm))]
    fn from_array(
        values: PyReadonlyArray3<f64>,
        x_range_nm: (f64, f64),
        y_range_nm: (f64, f64),
        z_range_nm: (f64, f64),
    ) -> PyResult<Self> {
        let data = values.as_array().to_owned();
        let (nz, ny, nx) = data.dim();
        let mut grid = Grid3D::<f64>::new(nx, ny, nz, x_range_nm, y_range_nm, z_range_nm)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        grid.data = data;
        Ok(Self::new(grid))
    }

    /// The field values as a `(nz, ny, nx)` numpy array: a read-only view
    /// of the result's buffer (no copy; `.copy()` for a writable array).
    #[getter]
    fn values<'py>(slf: &Bound<'py, Self>) -> Bound<'py, PyArray3<f64>> {
        crate::py_arrays::readonly_view(&slf.get().inner.data, slf.as_any())
    }

    /// x cell-center coordinates in nm (length nx).
    #[getter]
    fn x_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let x: Vec<f64> = (0..self.inner.nx()).map(|j| self.inner.x_at(j)).collect();
        x.into_pyarray(py)
    }

    /// y cell-center coordinates in nm (length ny).
    #[getter]
    fn y_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let y: Vec<f64> = (0..self.inner.ny()).map(|i| self.inner.y_at(i)).collect();
        y.into_pyarray(py)
    }

    /// z (depth-from-top) cell-center coordinates in nm (length nz).
    #[getter]
    fn z_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let z: Vec<f64> = (0..self.inner.nz()).map(|k| self.inner.z_at(k)).collect();
        z.into_pyarray(py)
    }

    /// Physical extents `((x_min, x_max), (y_min, y_max), (z_min, z_max))`
    /// in nm — the ranges `from_array` takes.
    #[getter]
    #[allow(clippy::type_complexity)]
    fn extent_nm(&self) -> ((f64, f64), (f64, f64), (f64, f64)) {
        (
            (self.inner.x_min_nm, self.inner.x_max_nm),
            (self.inner.y_min_nm, self.inner.y_max_nm),
            (self.inner.z_min_nm, self.inner.z_max_nm),
        )
    }

    /// The `(nz, ny, nx)` shape of the field.
    #[getter]
    fn shape(&self) -> (usize, usize, usize) {
        (self.inner.nz(), self.inner.ny(), self.inner.nx())
    }

    /// Depth-resolved critical dimension: the center-row CD in every z-slice
    /// at `threshold` (on the field value), top to bottom. Slices with no
    /// threshold crossing return `None`.
    fn cd_at_z(&self, py: Python<'_>, threshold: f64) -> Vec<Option<f64>> {
        py.allow_threads(|| metrics::cd_at_z(&self.inner, threshold))
    }

    /// Per-column development depth map (nm) by thresholding this volume as a
    /// PAC latent image: scan each column from the top and take the depth of
    /// the contiguous run of voxels with PAC below `threshold`.
    fn depth_map(&self, py: Python<'_>, threshold: f64) -> PyHeightMapResult {
        PyHeightMapResult::new(py.allow_threads(|| {
            let latent = VolumetricLatentImage {
                pac: self.inner.clone(),
            };
            volumetric::develop_depth_map(&latent, threshold)
        }))
    }

    fn __repr__(&self) -> String {
        format!(
            "VolumetricResult(shape=({}, {}, {}))",
            self.inner.nz(),
            self.inner.ny(),
            self.inner.nx()
        )
    }
}

/// A 2D height / depth map on a lateral grid (nm), with physical coordinates.
/// Frozen: `values` is a read-only view of the Rust buffer (no copy).
#[pyclass(name = "HeightMapResult", frozen)]
#[derive(Clone)]
pub struct PyHeightMapResult {
    inner: Grid2D<f64>,
}

impl PyHeightMapResult {
    fn new(inner: Grid2D<f64>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyHeightMapResult {
    /// The map values as a `(ny, nx)` numpy array: a read-only view of the
    /// result's buffer (no copy; `.copy()` for a writable array).
    #[getter]
    fn values<'py>(slf: &Bound<'py, Self>) -> Bound<'py, PyArray2<f64>> {
        crate::py_arrays::readonly_view(&slf.get().inner.data, slf.as_any())
    }

    /// x cell-center coordinates in nm (length nx).
    #[getter]
    fn x_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let x: Vec<f64> = (0..self.inner.nx()).map(|j| self.inner.x_at(j)).collect();
        x.into_pyarray(py)
    }

    /// y cell-center coordinates in nm (length ny).
    #[getter]
    fn y_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let y: Vec<f64> = (0..self.inner.ny()).map(|i| self.inner.y_at(i)).collect();
        y.into_pyarray(py)
    }

    /// The `(ny, nx)` shape of the map.
    #[getter]
    fn shape(&self) -> (usize, usize) {
        (self.inner.ny(), self.inner.nx())
    }

    fn __repr__(&self) -> String {
        format!(
            "HeightMapResult(shape=({}, {}))",
            self.inner.ny(),
            self.inner.nx()
        )
    }
}

/// Compute a z-resolved (volumetric) latent image through the resist depth.
///
/// Builds the partially-coherent aerial-imaging engine internally, then runs
/// the separable volumetric exposure: the lateral aerial image (refocused at
/// `n_defocus_planes` stepped depths and interpolated in z, or imaged exactly
/// at every slice centre when `n_defocus_planes >= nz`) is combined with
/// the exact transfer-matrix intensity through the film stack, and the Dill
/// exposure is integrated with `dose_steps` split-step bleaching.
///
/// The `film_stack` must contain the resist as layer `resist_layer`; layers
/// above it (e.g. a top coat) shift the resist in z and attenuate the field.
/// Returns the normalized PAC volume (`m = 1` unexposed, `m -> 0` fully
/// exposed).
#[pyfunction]
#[pyo3(signature = (
    source,
    optics,
    mask,
    film_stack,
    resist,
    grid,
    dose_mj_cm2=30.0,
    nz=64,
    n_defocus_planes=8,
    dose_steps=1,
    base_defocus_nm=0.0,
    resist_layer=0,
    max_kernels=20,
))]
#[allow(clippy::too_many_arguments)]
fn expose_volumetric(
    py: Python<'_>,
    source: PySourceConfig,
    optics: PyOpticsConfig,
    mask: PyMaskConfig,
    film_stack: PyFilmStackConfig,
    resist: PyResistConfig,
    grid: PyGridConfig,
    dose_mj_cm2: f64,
    nz: usize,
    n_defocus_planes: usize,
    dose_steps: usize,
    base_defocus_nm: f64,
    resist_layer: usize,
    max_kernels: usize,
) -> PyResult<PyVolumetricResult> {
    let wavelength_nm = source.inner.wavelength_nm();
    let grid_cfg = grid.inner.clone();
    let config = VolumetricExposureConfig {
        dose_mj_cm2,
        nz,
        n_defocus_planes,
        dose_steps,
        base_defocus_nm,
        resist_layer,
    };

    let latent = py
        .allow_threads(|| {
            let engine = AerialImageEngine::new(
                &source.inner,
                optics.inner.as_dyn(),
                grid_cfg,
                max_kernels,
            )?;
            volumetric::expose_volumetric(
                &engine,
                &mask.inner,
                &film_stack.inner,
                &resist.inner,
                wavelength_nm,
                &config,
            )
        })
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    Ok(PyVolumetricResult::new(latent.pac))
}

/// Tier-2 development: fast-marching solution of the etch front on a PAC
/// volume. Returns per-voxel arrival times in seconds (the developed region
/// after `t` seconds is `{times <= t}`, including lateral etching / undercut).
/// The development rate model comes from `resist`, optionally multiplied by
/// the surface-inhibition factor `1 − (1 − surface_rate_ratio)·exp(−d /
/// inhibition_depth_nm)` (d = depth below the original top). `lateral` is
/// `"reflecting"` (mirror edges, the historical behaviour) or `"periodic"`.
/// The rate field is static; use [`develop_level_set`] for developer
/// ageing / loading.
#[pyfunction]
#[pyo3(signature = (
    volume,
    resist,
    pixel_xy_nm,
    pixel_z_nm,
    surface_rate_ratio=1.0,
    inhibition_depth_nm=0.0,
    lateral="reflecting",
))]
#[allow(clippy::too_many_arguments)]
fn develop_fast_marching(
    py: Python<'_>,
    volume: &PyVolumetricResult,
    resist: PyResistConfig,
    pixel_xy_nm: f64,
    pixel_z_nm: f64,
    surface_rate_ratio: f64,
    inhibition_depth_nm: f64,
    lateral: &str,
) -> PyResult<PyVolumetricResult> {
    let options = DevelopmentOptions {
        surface_inhibition: parse_inhibition(surface_rate_ratio, inhibition_depth_nm)?,
        lateral_boundary: parse_lateral(lateral)?,
    };
    let latent = VolumetricLatentImage {
        pac: volume.inner.clone(),
    };
    let times = py
        .allow_threads(|| {
            volumetric::develop_fast_marching_with(
                &latent,
                &resist.inner,
                pixel_xy_nm,
                pixel_z_nm,
                &options,
            )
        })
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(PyVolumetricResult::new(times))
}

/// Remaining-height map (nm) after `dev_time_s` seconds of development, from a
/// fast-marching arrival-time volume (see [`develop_fast_marching`]).
#[pyfunction]
#[pyo3(signature = (times, dev_time_s))]
fn height_map_from_times(
    py: Python<'_>,
    times: &PyVolumetricResult,
    dev_time_s: f64,
) -> PyHeightMapResult {
    PyHeightMapResult::new(
        py.allow_threads(|| volumetric::height_map_from_times(&times.inner, dev_time_s)),
    )
}

// ============================================================================
// Level-set development and post-exposure bake
// ============================================================================

/// Parse a lateral boundary tag (`"periodic"` or `"reflecting"`).
fn parse_lateral(tag: &str) -> PyResult<DiffusionBoundary> {
    match tag {
        "periodic" => Ok(DiffusionBoundary::Periodic),
        "reflecting" => Ok(DiffusionBoundary::Reflecting),
        other => Err(PyValueError::new_err(format!(
            "unknown lateral boundary '{other}' (expected 'periodic' or 'reflecting')"
        ))),
    }
}

/// Surface inhibition from the keyword pair (`None` when it is a no-op).
fn parse_inhibition(
    surface_rate_ratio: f64,
    inhibition_depth_nm: f64,
) -> PyResult<Option<SurfaceInhibition>> {
    let inhibition = SurfaceInhibition::new(surface_rate_ratio, inhibition_depth_nm)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok((surface_rate_ratio != 1.0 && inhibition_depth_nm > 0.0).then_some(inhibition))
}

/// Developer depletion / ageing from its Python description.
fn parse_depletion(
    kind: &str,
    time_constant_s: Option<f64>,
    capacity_nm: Option<f64>,
    length_nm: Option<f64>,
) -> PyResult<DeveloperDepletion> {
    let need = |value: Option<f64>, name: &str| {
        value.ok_or_else(|| PyValueError::new_err(format!("depletion='{kind}' requires {name}")))
    };
    match kind {
        "none" => Ok(DeveloperDepletion::None),
        "exponential" => Ok(DeveloperDepletion::Exponential {
            time_constant_s: need(time_constant_s, "depletion_time_constant_s")?,
        }),
        "loading" => Ok(DeveloperDepletion::Loading {
            capacity_nm: need(capacity_nm, "loading_capacity_nm")?,
        }),
        "local_loading" => Ok(DeveloperDepletion::LocalLoading {
            capacity_nm: need(capacity_nm, "loading_capacity_nm")?,
            length_nm: need(length_nm, "loading_length_nm")?,
        }),
        other => Err(PyValueError::new_err(format!(
            "unknown depletion '{other}' (expected none / exponential / loading / local_loading)"
        ))),
    }
}

/// A new volume with the extents of `like` around `data`.
fn grid_like(like: &Grid3D<f64>, data: ndarray::Array3<f64>) -> Grid3D<f64> {
    Grid3D {
        data,
        x_min_nm: like.x_min_nm,
        x_max_nm: like.x_max_nm,
        y_min_nm: like.y_min_nm,
        y_max_nm: like.y_max_nm,
        z_min_nm: like.z_min_nm,
        z_max_nm: like.z_max_nm,
    }
}

/// Result of [`develop_level_set`]: front arrival times, the final level-set
/// function, and solver diagnostics. The volumes are built once; the getters
/// return the same objects (no copies).
#[pyclass(name = "LevelSetResult", frozen)]
pub struct PyLevelSetResult {
    arrival_times: Py<PyVolumetricResult>,
    phi: Py<PyVolumetricResult>,
    dev_time_s: f64,
    steps: usize,
    reinitializations: usize,
    dissolved_thickness_nm: f64,
    final_rate_factor: f64,
}

impl PyLevelSetResult {
    fn from_core(py: Python<'_>, r: volumetric::LevelSetResult) -> PyResult<Self> {
        Ok(Self {
            arrival_times: PyVolumetricResult::py_new(py, r.arrival_times)?,
            phi: PyVolumetricResult::py_new(py, r.phi)?,
            dev_time_s: r.dev_time_s,
            steps: r.steps,
            reinitializations: r.reinitializations,
            dissolved_thickness_nm: r.dissolved_thickness_nm,
            final_rate_factor: r.final_rate_factor,
        })
    }
}

#[pymethods]
impl PyLevelSetResult {
    /// Time (s) at which the front crossed each voxel centre (`inf` where it
    /// did not within `dev_time_s`) — comparable with fast-marching times.
    #[getter]
    fn arrival_times(&self, py: Python<'_>) -> Py<PyVolumetricResult> {
        self.arrival_times.clone_ref(py)
    }

    /// Final level-set function φ (nm): ≈ signed distance to the
    /// resist/developer boundary, negative where developed, clipped a few
    /// cells beyond the narrow band.
    #[getter]
    fn phi(&self, py: Python<'_>) -> Py<PyVolumetricResult> {
        self.phi.clone_ref(py)
    }

    /// Development time simulated (s).
    #[getter]
    fn dev_time_s(&self) -> f64 {
        self.dev_time_s
    }

    /// Number of time steps taken.
    #[getter]
    fn steps(&self) -> usize {
        self.steps
    }

    /// Number of signed-distance reinitializations (including the first).
    #[getter]
    fn reinitializations(&self) -> usize {
        self.reinitializations
    }

    /// Mean dissolved resist thickness at the end (nm).
    #[getter]
    fn dissolved_thickness_nm(&self) -> f64 {
        self.dissolved_thickness_nm
    }

    /// Developer depletion factor g at the end (1 without depletion).
    #[getter]
    fn final_rate_factor(&self) -> f64 {
        self.final_rate_factor
    }

    /// Remaining-height map (nm) at the end of development.
    fn height_map(&self, py: Python<'_>) -> PyHeightMapResult {
        let times = &self.arrival_times.get().inner;
        let t = self.dev_time_s;
        PyHeightMapResult::new(py.allow_threads(|| volumetric::height_map_from_times(times, t)))
    }

    /// Developed-region indicator volume (1 developed, 0 resist) at
    /// `time_s` (default: the end of development). Feed it to `cd_at_z(0.5)`
    /// for per-slice developed CD.
    #[pyo3(signature = (time_s=None))]
    fn developed(&self, py: Python<'_>, time_s: Option<f64>) -> PyVolumetricResult {
        let t = time_s.unwrap_or(self.dev_time_s);
        let times = &self.arrival_times.get().inner;
        PyVolumetricResult::new(py.allow_threads(|| volumetric::developed_indicator(times, t)))
    }

    /// Capability badge of the model behind this result (see
    /// docs/capability-matrix.md): "✅" implemented, "🔶" simplified, "🧪"
    /// theoretical; split badges name exact and approximate parts.
    #[getter]
    fn status(&self) -> &'static str {
        "✅/🔶"
    }

    /// The model's stated approximations for this result.
    #[getter]
    fn notes(&self) -> Vec<&'static str> {
        vec![
            "first-order level set validated against analytic fronts and fast marching",
            "surface inhibition and developer depletion constants are phenomenological",
        ]
    }

    fn __repr__(&self) -> String {
        format!(
            "LevelSetResult(dev_time={} s, steps={}, dissolved={:.2} nm)",
            self.dev_time_s, self.steps, self.dissolved_thickness_nm
        )
    }
}

/// Tier-3 development: level-set moving boundary with the dissolution rate
/// re-evaluated at the front every step,
/// `R = R_bulk(m) · f_inh(depth) · g(t)`.
///
/// `resist` supplies the bulk rate model (Mack / threshold). Surface
/// inhibition as in [`develop_fast_marching`]. `depletion` selects g:
/// `"none"`, `"exponential"` (g = exp(−t/τ), needs
/// `depletion_time_constant_s`), `"loading"` (g = 1 − h̄/h_cap over the
/// field, needs `loading_capacity_nm`), or `"local_loading"` (h̄ smoothed
/// over `loading_length_nm`). Grid spacings come from the volume extents.
/// For static rates, FMM gives the same fronts (within grid error) faster.
#[pyfunction]
#[pyo3(signature = (
    volume,
    resist,
    dev_time_s,
    surface_rate_ratio=1.0,
    inhibition_depth_nm=0.0,
    lateral="periodic",
    depletion="none",
    depletion_time_constant_s=None,
    loading_capacity_nm=None,
    loading_length_nm=None,
    cfl=0.5,
    reinit_interval=4,
    band_cells=6.0,
    max_steps=2_000_000,
))]
#[allow(clippy::too_many_arguments)]
fn develop_level_set(
    py: Python<'_>,
    volume: &PyVolumetricResult,
    resist: PyResistConfig,
    dev_time_s: f64,
    surface_rate_ratio: f64,
    inhibition_depth_nm: f64,
    lateral: &str,
    depletion: &str,
    depletion_time_constant_s: Option<f64>,
    loading_capacity_nm: Option<f64>,
    loading_length_nm: Option<f64>,
    cfl: f64,
    reinit_interval: usize,
    band_cells: f64,
    max_steps: usize,
) -> PyResult<PyLevelSetResult> {
    let options = DevelopmentOptions {
        surface_inhibition: parse_inhibition(surface_rate_ratio, inhibition_depth_nm)?,
        lateral_boundary: parse_lateral(lateral)?,
    };
    let config = LevelSetConfig {
        dev_time_s,
        depletion: parse_depletion(
            depletion,
            depletion_time_constant_s,
            loading_capacity_nm,
            loading_length_nm,
        )?,
        cfl,
        reinit_interval,
        band_cells,
        max_steps,
    };
    let latent = VolumetricLatentImage {
        pac: volume.inner.clone(),
    };
    let result = py
        .allow_threads(|| volumetric::develop_level_set(&latent, &resist.inner, &options, &config));
    match result {
        Ok(inner) => PyLevelSetResult::from_core(py, inner),
        Err(e @ highuvlith_core::error::LithographyError::NumericalError(_)) => {
            Err(PyRuntimeError::new_err(e.to_string()))
        }
        Err(e) => Err(PyValueError::new_err(e.to_string())),
    }
}

/// Static dissolution-rate volume (nm/s): the resist's Mack / threshold rate
/// of each voxel times the optional surface-inhibition factor.
#[pyfunction]
#[pyo3(signature = (volume, resist, surface_rate_ratio=1.0, inhibition_depth_nm=0.0))]
fn development_rate(
    py: Python<'_>,
    volume: &PyVolumetricResult,
    resist: PyResistConfig,
    surface_rate_ratio: f64,
    inhibition_depth_nm: f64,
) -> PyResult<PyVolumetricResult> {
    let inhibition = parse_inhibition(surface_rate_ratio, inhibition_depth_nm)?;
    py.allow_threads(|| {
        let latent = VolumetricLatentImage {
            pac: volume.inner.clone(),
        };
        volumetric::development_rate_volume(&latent, &resist.inner, inhibition.as_ref())
    })
    .map(PyVolumetricResult::new)
    .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Post-exposure bake by Fickian diffusion with exact Gaussian kernels:
/// lateral length `lateral_nm`, vertical length `vertical_nm` (default: the
/// lateral one), zero-flux top/bottom, `lateral` = `"periodic"` or
/// `"reflecting"`. A depth-dependent vertical diffusivity is given either as
/// `vertical_scale` (length-nz array of D_z(z)/D_ref) or parametrically as
/// `1 + (vertical_surface_ratio − 1)·exp(−z / vertical_decay_nm)`. Returns
/// the baked volume (the input is not modified).
#[pyfunction]
#[pyo3(signature = (
    volume,
    lateral_nm,
    vertical_nm=None,
    vertical_scale=None,
    vertical_surface_ratio=None,
    vertical_decay_nm=None,
    lateral="periodic",
))]
#[allow(clippy::too_many_arguments)]
fn peb_gaussian(
    py: Python<'_>,
    volume: &PyVolumetricResult,
    lateral_nm: f64,
    vertical_nm: Option<f64>,
    vertical_scale: Option<PyReadonlyArray1<f64>>,
    vertical_surface_ratio: Option<f64>,
    vertical_decay_nm: Option<f64>,
    lateral: &str,
) -> PyResult<PyVolumetricResult> {
    let mut peb = PebDiffusion::anisotropic(lateral_nm, vertical_nm.unwrap_or(lateral_nm));
    peb.lateral_boundary = parse_lateral(lateral)?;
    match (vertical_scale, vertical_surface_ratio, vertical_decay_nm) {
        (None, None, None) => {}
        (Some(scale), None, None) => {
            peb.vertical_diffusivity_scale = Some(scale.as_array().to_vec());
        }
        (None, Some(ratio), Some(decay)) => {
            if !(decay.is_finite() && decay > 0.0) {
                return Err(PyValueError::new_err(format!(
                    "vertical_decay_nm must be finite and > 0, got {decay}"
                )));
            }
            peb = peb.with_exponential_depth_profile(
                volume.inner.nz(),
                volume.inner.pixel_size_z(),
                ratio,
                decay,
            );
        }
        _ => {
            return Err(PyValueError::new_err(
                "give either vertical_scale, or both vertical_surface_ratio and \
                 vertical_decay_nm",
            ))
        }
    }
    let mut latent = VolumetricLatentImage {
        pac: volume.inner.clone(),
    };
    py.allow_threads(|| volumetric::peb_diffuse_3d_anisotropic(&mut latent, &peb))
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(PyVolumetricResult::new(latent.pac))
}

/// Result of [`peb_car`]: the chemically amplified resist state after the
/// bake (concentrations normalized to the initial PAG concentration).
#[pyclass(name = "CarPebResult", frozen)]
pub struct PyCarPebResult {
    protected: Py<PyVolumetricResult>,
    acid: Py<PyVolumetricResult>,
    quencher: Py<PyVolumetricResult>,
    neutralized_total: f64,
    steps: usize,
    time_step_s: f64,
}

#[pymethods]
impl PyCarPebResult {
    /// Protected (blocked) site fraction m — what the development rate
    /// consumes (1 protected, 0 fully deprotected). Pass it to
    /// `develop_fast_marching` / `develop_level_set`.
    #[getter]
    fn protected(&self, py: Python<'_>) -> Py<PyVolumetricResult> {
        self.protected.clone_ref(py)
    }

    /// Remaining acid h after the bake.
    #[getter]
    fn acid(&self, py: Python<'_>) -> Py<PyVolumetricResult> {
        self.acid.clone_ref(py)
    }

    /// Remaining base quencher q after the bake.
    #[getter]
    fn quencher(&self, py: Python<'_>) -> Py<PyVolumetricResult> {
        self.quencher.clone_ref(py)
    }

    /// Total neutralized acid summed over voxels (= neutralized quencher);
    /// `acid.sum() + neutralized_total == (1 − m_exposure).sum()`.
    #[getter]
    fn neutralized_total(&self) -> f64 {
        self.neutralized_total
    }

    /// Number of Strang-splitting steps.
    #[getter]
    fn steps(&self) -> usize {
        self.steps
    }

    /// Splitting time step (s).
    #[getter]
    fn time_step_s(&self) -> f64 {
        self.time_step_s
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
            "Mack/PROLITH-class acid/quencher reaction-diffusion normalized to the PAG",
            "no acid loss or evaporation; constant diffusivities; illustrative default constants",
        ]
    }

    fn __repr__(&self) -> String {
        format!(
            "CarPebResult(steps={}, dt={:.3} s, neutralized={:.4})",
            self.steps, self.time_step_s, self.neutralized_total
        )
    }
}

/// Chemically amplified resist post-exposure bake (acid/quencher
/// reaction–diffusion; standard Mack/PROLITH-class model, concentrations
/// normalized to the PAG):
/// `∂m/∂t = −k_amp·m·h`, `∂h/∂t = D_h∇²h − k_q·h·q`,
/// `∂q/∂t = D_q∇²q − k_q·h·q`, with `h(0) = 1 − volume` (the input is the
/// Dill latent image, read as the remaining PAG fraction), `q(0) =
/// quencher`, `m(0) = 1`. Diffusivities are lateral; `vertical_ratio` =
/// D_z/D_xy. Defaults are illustrative, not fitted to a resist.
#[pyfunction]
#[pyo3(signature = (
    volume,
    peb_time_s=60.0,
    k_amp=0.1,
    k_quench=10.0,
    quencher=0.15,
    acid_diffusivity_nm2_s=2.0,
    quencher_diffusivity_nm2_s=0.5,
    vertical_ratio=1.0,
    lateral="periodic",
    max_time_step_s=None,
))]
#[allow(clippy::too_many_arguments)]
fn peb_car(
    py: Python<'_>,
    volume: &PyVolumetricResult,
    peb_time_s: f64,
    k_amp: f64,
    k_quench: f64,
    quencher: f64,
    acid_diffusivity_nm2_s: f64,
    quencher_diffusivity_nm2_s: f64,
    vertical_ratio: f64,
    lateral: &str,
    max_time_step_s: Option<f64>,
) -> PyResult<PyCarPebResult> {
    let params = CarParams {
        peb_time_s,
        k_amp_per_s: k_amp,
        k_quench_per_s: k_quench,
        quencher_initial: quencher,
        acid_diffusivity_nm2_s,
        quencher_diffusivity_nm2_s,
        vertical_diffusivity_ratio: vertical_ratio,
        lateral_boundary: parse_lateral(lateral)?,
        max_time_step_s,
    };
    let mut latent = VolumetricLatentImage {
        pac: volume.inner.clone(),
    };
    let state = py
        .allow_threads(|| {
            volumetric::apply_peb(&mut latent, &PebModel::ChemicallyAmplified(params))
        })
        .map_err(|e| PyValueError::new_err(e.to_string()))?
        .ok_or_else(|| PyRuntimeError::new_err("CAR bake returned no state"))?;
    Ok(PyCarPebResult {
        acid: PyVolumetricResult::py_new(py, grid_like(&latent.pac, state.acid))?,
        quencher: PyVolumetricResult::py_new(py, grid_like(&latent.pac, state.quencher))?,
        protected: PyVolumetricResult::py_new(py, latent.pac)?,
        neutralized_total: state.neutralized_total,
        steps: state.steps,
        time_step_s: state.time_step_s,
    })
}

/// LIGA deep-X-ray exposure result: absolute depth dose, top/bottom contrast,
/// the volumetric absorbed-dose field, the developed-depth map, and (for
/// absolute spectra) dose rates and the exposure time.
#[pyclass(name = "LigaResult", frozen)]
pub struct PyLigaResult {
    exposure: deep_xray::LigaExposure,
    sampling: deep_xray::FresnelSampling,
    diffraction: String,
    spectrum: String,
    volume: Py<PyVolumetricResult>,
    developed: Py<PyHeightMapResult>,
}

#[pymethods]
impl PyLigaResult {
    /// Depth-dose profile as `(z_um, dose_kj_cm3)` numpy arrays, from the
    /// resist top (`z=0`) to the bottom.
    #[getter]
    fn depth_dose<'py>(
        &self,
        py: Python<'py>,
    ) -> (Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>) {
        (
            self.exposure.z_um.to_pyarray(py),
            self.exposure.dose_kj_cm3.to_pyarray(py),
        )
    }

    /// Top/bottom absorbed-dose ratio (>= 1; the process contrast).
    #[getter]
    fn dose_ratio(&self) -> f64 {
        self.exposure.dose_ratio
    }

    /// Absorbed-energy density at the resist top in kJ/cm^3.
    #[getter]
    fn top_dose_kj_cm3(&self) -> f64 {
        self.exposure.top_dose_kj_cm3
    }

    /// Absorbed-energy density at the resist bottom in kJ/cm^3.
    #[getter]
    fn bottom_dose_kj_cm3(&self) -> f64 {
        self.exposure.bottom_dose_kj_cm3
    }

    /// Whether the top dose exceeds the damage ceiling (foaming / T-topping).
    #[getter]
    fn exceeds_damage_ceiling(&self) -> bool {
        self.exposure.exceeds_damage_ceiling
    }

    /// Exposure time in s to reach the bottom dose (absolute spectra only).
    #[getter]
    fn exposure_time_s(&self) -> Option<f64> {
        self.exposure.exposure_time_estimate
    }

    /// Dose rate at the resist top in kJ cm^-3 s^-1 (absolute spectra only).
    #[getter]
    fn top_dose_rate_kj_cm3_s(&self) -> Option<f64> {
        self.exposure.top_dose_rate_kj_cm3_s
    }

    /// Dose rate at the resist bottom in kJ cm^-3 s^-1 (absolute spectra only).
    #[getter]
    fn bottom_dose_rate_kj_cm3_s(&self) -> Option<f64> {
        self.exposure.bottom_dose_rate_kj_cm3_s
    }

    /// Photon power density on the resist in W/mm^2 (absolute spectra only).
    #[getter]
    fn resist_power_density_w_mm2(&self) -> Option<f64> {
        self.exposure.resist_power_density_w_mm2
    }

    /// Ring current x exposure time in mA h (bending-magnet beamlines only).
    #[getter]
    fn exposure_charge_ma_h(&self) -> Option<f64> {
        self.exposure.exposure_charge_ma_h
    }

    /// Dose-weighted mean photon energy at the resist top in keV.
    #[getter]
    fn mean_energy_top_kev(&self) -> f64 {
        self.exposure.mean_energy_top_kev
    }

    /// Dose-weighted mean photon energy at the resist bottom in keV.
    #[getter]
    fn mean_energy_bottom_kev(&self) -> f64 {
        self.exposure.mean_energy_bottom_kev
    }

    /// Fraction of the transmitted beam power inside the sampled energy
    /// window (1 = nothing clipped; a warning is added below 0.99).
    #[getter]
    fn window_power_fraction(&self) -> f64 {
        self.exposure.window_power_fraction
    }

    /// Lateral proximity model used: `"fresnel"` or `"gaussian"`.
    #[getter]
    fn diffraction(&self) -> String {
        self.diffraction.clone()
    }

    /// Dose-weighted Fresnel scales `(top, bottom)` = `sqrt(lambda g)`,
    /// `sqrt(lambda (g + T))` in nm.
    #[getter]
    fn fresnel_scale_nm(&self) -> (f64, f64) {
        (
            self.sampling.fresnel_scale_top_nm,
            self.sampling.fresnel_scale_bottom_nm,
        )
    }

    /// Whether the lateral grid resolves the Fresnel scale (pixel <= half
    /// of it where it is finest).
    #[getter]
    fn sampling_resolved(&self) -> bool {
        self.sampling.resolved
    }

    /// Sampling and beamline-model warnings (empty when none apply).
    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.sampling
            .warnings
            .iter()
            .chain(self.exposure.warnings.iter())
            .cloned()
            .collect()
    }

    /// Volumetric absorbed-dose field `D(x,y,z)` in kJ/cm^3 (the same object
    /// on every access; `.values` is a view, no copy).
    #[getter]
    fn volume(&self, py: Python<'_>) -> Py<PyVolumetricResult> {
        self.volume.clone_ref(py)
    }

    /// Developed-depth map (nm) at the clearing (target bottom) dose.
    #[getter]
    fn developed_depth(&self, py: Python<'_>) -> Py<PyHeightMapResult> {
        self.developed.clone_ref(py)
    }

    /// Which spectrum drove the exposure: "flux_density" (absolute table),
    /// "tabulated" (relative lines), "bending_magnet" (relative, from E_c),
    /// "bending_magnet_beamline" (absolute ring/beamline), "xray_tube" or
    /// "betatron" (absolute at `source_distance_m`, else relative).
    #[getter]
    fn spectrum(&self) -> String {
        self.spectrum.clone()
    }

    /// Capability badge of the model behind this result (see
    /// docs/capability-matrix.md): "✅" implemented, "🔶" simplified, "🧪"
    /// theoretical; split badges name exact and approximate parts.
    #[getter]
    fn status(&self) -> &'static str {
        "✅/🔶"
    }

    /// The model's stated approximations for this result.
    #[getter]
    fn notes(&self) -> Vec<&'static str> {
        vec![
            "depth dose and exposure time: NIST mu/mu_en spectral depth dose (exact parts)",
            "lateral dose: scalar Fresnel propagation through a thin-screen absorber; no photo- or secondary-electron transport, no fluorescence",
            "collimated beam, uniform vertical scan, no beamline mirrors",
        ]
    }

    fn __repr__(&self) -> String {
        format!(
            "LigaResult(dose_ratio={:.2}, top={:.2} kJ/cm^3, exceeds_damage={}, diffraction={}, exposure_time_s={:?})",
            self.exposure.dose_ratio,
            self.exposure.top_dose_kj_cm3,
            self.exposure.exceeds_damage_ceiling,
            self.diffraction,
            self.exposure.exposure_time_estimate
        )
    }
}

/// Simulate a LIGA deep-X-ray shadow exposure of thick PMMA.
///
/// Spectrum (first that applies): `flux_density` (absolute photons s^-1
/// mm^-2 keV^-1 table at the mask), `spectrum_table` (relative lines),
/// `source` — a synchrotron bending magnet (absolute when `source_distance_m`
/// and `vertical_scan_mm` are both given), an X-ray tube or a betatron
/// (absolute when `source_distance_m` is given: the source's spectral flux
/// density at the mask) — else `critical_energy_kev` (relative).
/// `LigaResult.spectrum` names the path taken. The stack defaults to 20 um Au on 2 um Ti over PMMA with a
/// 100 um gap; `filters` is a list of `(material, thickness_um)` with a
/// preset name (`"Be"`, `"Al"`, `"Kapton"`, ...) or `"<formula>@<density>"`.
/// `diffraction` selects Fresnel propagation (default) or the legacy
/// Gaussian blur; `strict_sampling=True` raises when the grid cannot resolve
/// the Fresnel scale. Returns depth dose, the volumetric dose field, the
/// developed depth at the clearing dose, dose rates / exposure time when
/// absolute, and sampling warnings.
#[pyfunction]
#[pyo3(signature = (
    critical_energy_kev=6.23,
    resist_thickness_um=500.0,
    cd_nm=5000.0,
    pitch_nm=10000.0,
    grid_size=128,
    pixel_nm=200.0,
    nz=64,
    diffraction="fresnel",
    proximity_gap_um=100.0,
    energy_bins=100,
    photoelectron_blur=false,
    filters=None,
    absorber="Au",
    absorber_thickness_um=20.0,
    membrane="Ti",
    membrane_thickness_um=2.0,
    target_bottom_dose_kj_cm3=3.0,
    damage_dose_kj_cm3=20.0,
    source=None,
    source_distance_m=None,
    horizontal_acceptance_mrad=5.0,
    vertical_scan_mm=None,
    flux_density=None,
    spectrum_table=None,
    strict_sampling=false,
))]
#[allow(clippy::too_many_arguments)]
fn simulate_liga(
    py: Python<'_>,
    critical_energy_kev: f64,
    resist_thickness_um: f64,
    cd_nm: f64,
    pitch_nm: f64,
    grid_size: usize,
    pixel_nm: f64,
    nz: usize,
    diffraction: &str,
    proximity_gap_um: f64,
    energy_bins: usize,
    photoelectron_blur: bool,
    filters: Option<Vec<(String, f64)>>,
    absorber: &str,
    absorber_thickness_um: f64,
    membrane: &str,
    membrane_thickness_um: f64,
    target_bottom_dose_kj_cm3: f64,
    damage_dose_kj_cm3: f64,
    source: Option<PyRef<'_, PySourceConfig>>,
    source_distance_m: Option<f64>,
    horizontal_acceptance_mrad: f64,
    vertical_scan_mm: Option<f64>,
    flux_density: Option<Vec<(f64, f64)>>,
    spectrum_table: Option<Vec<(f64, f64)>>,
    strict_sampling: bool,
) -> PyResult<PyLigaResult> {
    let grid = highuvlith_core::types::GridConfig::new(grid_size, pixel_nm)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    let mask =
        Mask::line_space(cd_nm, pitch_nm).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let spectrum = crate::py_xray::LigaSpectrumArgs {
        critical_energy_kev,
        source: source.as_deref(),
        source_distance_m,
        horizontal_acceptance_mrad,
        vertical_scan_mm,
        flux_density,
        spectrum_table,
    };
    let stack = crate::py_xray::LigaStackArgs {
        resist_thickness_um,
        proximity_gap_um,
        energy_bins,
        photoelectron_blur,
        filters,
        absorber: absorber.to_string(),
        absorber_thickness_um,
        membrane: membrane.to_string(),
        membrane_thickness_um,
        target_bottom_dose_kj_cm3,
        damage_dose_kj_cm3,
        diffraction: diffraction.to_string(),
    };
    let (config, spectrum_label) = crate::py_xray::build_liga_config(&spectrum, &stack)?;
    let sampling = deep_xray::fresnel_sampling(&config, &grid);
    if strict_sampling {
        sampling
            .require_resolved()
            .map_err(|e| PyValueError::new_err(format!("{e}: {}", sampling.warnings.join("; "))))?;
    }
    let threshold = config.target_bottom_dose_kj_cm3;

    let (exposure, volume, developed) = py
        .allow_threads(|| -> highuvlith_core::error::Result<_> {
            let exposure = deep_xray::expose_depth(&config);
            let volume = deep_xray::expose_volumetric(&config, &mask, &grid, nz)?;
            let developed_data = deep_xray::develop_depth(&volume, threshold);
            let developed = Grid2D {
                data: developed_data,
                x_min_nm: volume.x_min_nm,
                x_max_nm: volume.x_max_nm,
                y_min_nm: volume.y_min_nm,
                y_max_nm: volume.y_max_nm,
            };
            Ok((exposure, volume, developed))
        })
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    Ok(PyLigaResult {
        exposure,
        sampling,
        diffraction: diffraction.to_ascii_lowercase(),
        spectrum: spectrum_label.to_string(),
        volume: PyVolumetricResult::py_new(py, volume)?,
        developed: Py::new(py, PyHeightMapResult::new(developed))?,
    })
}

/// Grayscale 2.5D height map from a continuous intensity-transmittance mask.
///
/// Builds the aerial-imaging engine, images the `transmittance` array (values
/// in [0, 1], amplitude `sqrt(T)`), and maps the local dose through the
/// log-linear contrast curve fixed by `d_th`/`d_clear` into a remaining-height
/// map. `transmittance` must be a square `(n, n)` float64 array matching the
/// grid size.
#[pyfunction]
#[pyo3(signature = (
    source,
    optics,
    grid,
    transmittance,
    dose_mj_cm2,
    d_th,
    d_clear,
    thickness_nm,
    max_kernels=20,
))]
#[allow(clippy::too_many_arguments)]
fn grayscale_height_map(
    py: Python<'_>,
    source: PySourceConfig,
    optics: PyOpticsConfig,
    grid: PyGridConfig,
    transmittance: PyReadonlyArray2<f64>,
    dose_mj_cm2: f64,
    d_th: f64,
    d_clear: f64,
    thickness_nm: f64,
    max_kernels: usize,
) -> PyResult<PyHeightMapResult> {
    let n = grid.inner.size;
    let t = transmittance.as_array();
    if t.dim() != (n, n) {
        return Err(PyValueError::new_err(format!(
            "transmittance must be ({n}, {n}) to match the grid, got {:?}",
            t.dim()
        )));
    }
    let amplitude = t.mapv(|v| Complex64::new(v.clamp(0.0, 1.0).sqrt(), 0.0));

    let curve =
        ContrastCurve::new(d_th, d_clear).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let grid_cfg = grid.inner.clone();

    let height = py
        .allow_threads(|| -> highuvlith_core::error::Result<Grid2D<f64>> {
            let engine = AerialImageEngine::new(
                &source.inner,
                optics.inner.as_dyn(),
                grid_cfg,
                max_kernels,
            )?;
            let aerial = engine.compute_from_transmittance(&amplitude, 0.0);
            Ok(grayscale::height_map(
                &aerial,
                dose_mj_cm2,
                &curve,
                thickness_nm,
            ))
        })
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    Ok(PyHeightMapResult::new(height))
}

/// Target surface-height map (nm) for a blazed (sawtooth) grating, as a
/// `(n, n)` numpy array. The relief runs along x with period `period_px`.
#[pyfunction]
#[pyo3(signature = (n, period_px, depth_nm, thickness_nm))]
fn blazed_grating<'py>(
    py: Python<'py>,
    n: usize,
    period_px: usize,
    depth_nm: f64,
    thickness_nm: f64,
) -> Bound<'py, PyArray2<f64>> {
    grayscale::blazed_grating(n, period_px, depth_nm, thickness_nm).into_pyarray(py)
}

/// Target surface-height map (nm) for a square array of parabolic microlenses,
/// as a `(n, n)` numpy array with cell pitch `pitch_px` and lens sag `sag_nm`.
#[pyfunction]
#[pyo3(signature = (n, pitch_px, sag_nm, thickness_nm))]
fn microlens_array<'py>(
    py: Python<'py>,
    n: usize,
    pitch_px: usize,
    sag_nm: f64,
    thickness_nm: f64,
) -> Bound<'py, PyArray2<f64>> {
    grayscale::microlens_array(n, pitch_px, sag_nm, thickness_nm).into_pyarray(py)
}

/// Synthesize the intensity-transmittance mask (`(ny, nx)` numpy array) that
/// prints `target_height` (nm) with a single exposure of `exposure_dose`,
/// through the contrast curve fixed by `d_th`/`d_clear`. Raises if any pixel
/// needs more dose than the exposure delivers.
#[pyfunction]
#[pyo3(signature = (target_height, thickness_nm, d_th, d_clear, exposure_dose))]
fn grayscale_transmittance_for_target<'py>(
    py: Python<'py>,
    target_height: PyReadonlyArray2<f64>,
    thickness_nm: f64,
    d_th: f64,
    d_clear: f64,
    exposure_dose: f64,
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let curve =
        ContrastCurve::new(d_th, d_clear).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let target = target_height.as_array().to_owned();
    let map = py
        .allow_threads(|| {
            GrayscaleMap::from_target_height(&target, thickness_nm, &curve, exposure_dose)
        })
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    Ok(map.transmittance.into_pyarray(py))
}

/// Multi-beam interference (holographic) lithography: expose a periodic
/// standing-wave lattice into a PAC volume.
///
/// `preset` selects the beam geometry (`"two_beam"` -> 1D grating,
/// `"three_beam_hex"` -> 2D hexagonal, `"four_beam_umbrella"` -> FCC-like 3D).
/// The fringe period is set by the air-side `half_angle_deg` and is invariant
/// under `n_medium`. Set `two_photon=True` for `I^2` (multiphoton) kinetics.
/// Returns the PAC volume (`m = 1` unexposed, `m -> 0` fully exposed).
#[pyfunction]
#[pyo3(signature = (
    preset,
    wavelength_nm,
    n_medium,
    half_angle_deg,
    nx,
    ny,
    nz,
    x_span_nm,
    y_span_nm,
    z_span_nm,
    dose_scale=1.0,
    dill_c=0.02,
    two_photon=false,
))]
#[allow(clippy::too_many_arguments)]
fn simulate_interference(
    py: Python<'_>,
    preset: &str,
    wavelength_nm: f64,
    n_medium: f64,
    half_angle_deg: f64,
    nx: usize,
    ny: usize,
    nz: usize,
    x_span_nm: f64,
    y_span_nm: f64,
    z_span_nm: f64,
    dose_scale: f64,
    dill_c: f64,
    two_photon: bool,
) -> PyResult<PyVolumetricResult> {
    let setup = match preset {
        "two_beam" => InterferenceSetup::two_beam(wavelength_nm, n_medium, half_angle_deg),
        "three_beam_hex" => {
            InterferenceSetup::three_beam_hex(wavelength_nm, n_medium, half_angle_deg)
        }
        "four_beam_umbrella" => {
            InterferenceSetup::four_beam_umbrella(wavelength_nm, n_medium, half_angle_deg)
        }
        other => {
            return Err(PyValueError::new_err(format!(
                "unknown preset '{other}' (expected two_beam / three_beam_hex / four_beam_umbrella)"
            )))
        }
    }
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let mut grid = Grid3D::<f64>::new(
        nx,
        ny,
        nz,
        (0.0, x_span_nm),
        (0.0, y_span_nm),
        (0.0, z_span_nm),
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let kinetics = if two_photon {
        ExposureKinetics::TwoPhoton
    } else {
        ExposureKinetics::OnePhoton
    };

    let pac = py.allow_threads(|| {
        setup.intensity(&mut grid);
        interference::expose(&grid, dose_scale, dill_c, kinetics)
    });

    Ok(PyVolumetricResult::new(pac))
}

/// Deprecated alias of `n_photon_absorption_image`: classical N-photon
/// absorption `I^N` of a classical intensity image (sharpening at the
/// classical period — no resolution gain). `fidelity`, `wavelength_nm` and
/// `na` are validated but do not change the result (classical N-photon
/// absorption involves no entanglement; the old `F·I^N + (1 − F)·I` mix had
/// no physical basis). For the entangled N00N model use `TwoBeamNPhoton` or
/// `SimulationEngine.compute_noon_ideal_image`. `classical` is a 2D float64
/// array; the return has the same shape. 🧪 Theoretical.
#[pyfunction]
#[pyo3(signature = (classical, n=2, wavelength_nm=157.63, na=0.75, fidelity=1.0))]
fn quantum_aerial_image<'py>(
    py: Python<'py>,
    classical: PyReadonlyArray2<f64>,
    n: usize,
    wavelength_nm: f64,
    na: f64,
    fidelity: f64,
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let params = QuantumLithographyParams {
        num_entangled_photons: n,
        wavelength_nm,
        na,
        fidelity,
    };
    let classical = classical.as_array().to_owned();
    let quantum = py
        .allow_threads(|| compute_quantum_aerial_image(&classical, &params))
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(quantum.into_pyarray(py))
}

// ============================================================================
// Talbot / EUV-IL
// ============================================================================

/// Build a Talbot grating from its Python description.
fn talbot_grating(
    grating: &str,
    period_nm: f64,
    duty_cycle: f64,
    phase_rad: f64,
    hole_diameter_nm: Option<f64>,
    max_order: usize,
) -> PyResult<Grating> {
    let hole = hole_diameter_nm.unwrap_or(0.5 * period_nm);
    match grating {
        "amplitude" => Grating::binary_amplitude(period_nm, duty_cycle, max_order),
        "phase" => Grating::binary_phase(period_nm, duty_cycle, phase_rad, max_order),
        "sinusoidal" => Grating::sinusoidal_amplitude(period_nm, 0.5, 0.5),
        "holes_square" => Grating::hole_array_square(period_nm, hole, max_order),
        "holes_hex" => Grating::hole_array_hexagonal(period_nm, hole, max_order),
        other => {
            return Err(PyValueError::new_err(format!(
                "unknown grating '{other}' (expected amplitude / phase / sinusoidal / \
                 holes_square / holes_hex)"
            )))
        }
    }
    .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Result of [`simulate_talbot`]: the Talbot carpet, wafer-plane images for
/// the coherent / displacement (DTL) / achromatic (ATL) exposure modes, the
/// characteristic lengths, and optional resist intensity / PAC volumes.
#[pyclass(name = "TalbotResult", frozen)]
pub struct PyTalbotResult {
    carpet: Grid2D<f64>,
    image_grid: Grid2D<f64>,
    coherent: ndarray::Array2<f64>,
    stationary: ndarray::Array2<f64>,
    dtl: ndarray::Array2<f64>,
    atl: Option<ndarray::Array2<f64>>,
    talbot_length_nm: f64,
    talbot_length_exact_nm: Option<f64>,
    achromatic_distance_nm: Option<f64>,
    period_nm: f64,
    wavelength_nm: f64,
    gap_nm: f64,
    scan_length_nm: f64,
    efficiencies: Vec<(i32, i32, f64)>,
    intensity_volume: Option<Py<PyVolumetricResult>>,
    pac_volume: Option<Py<PyVolumetricResult>>,
}

#[pymethods]
impl PyTalbotResult {
    /// Coherent Talbot carpet `I(x, z)` along `y = 0` as a `(nz, nx)` array
    /// (rows = propagation distance behind the grating).
    #[getter]
    fn carpet<'py>(slf: &Bound<'py, Self>) -> Bound<'py, PyArray2<f64>> {
        crate::py_arrays::readonly_view(&slf.get().carpet.data, slf.as_any())
    }

    /// x cell-centre coordinates (nm) of the carpet columns.
    #[getter]
    fn carpet_x_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let x: Vec<f64> = (0..self.carpet.nx()).map(|j| self.carpet.x_at(j)).collect();
        x.into_pyarray(py)
    }

    /// Propagation distances z (nm) of the carpet rows.
    #[getter]
    fn carpet_z_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let z: Vec<f64> = (0..self.carpet.ny()).map(|i| self.carpet.y_at(i)).collect();
        z.into_pyarray(py)
    }

    /// x cell-centre coordinates (nm) of the wafer-plane images.
    #[getter]
    fn x_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let x: Vec<f64> = (0..self.image_grid.nx())
            .map(|j| self.image_grid.x_at(j))
            .collect();
        x.into_pyarray(py)
    }

    /// y cell-centre coordinates (nm) of the wafer-plane images.
    #[getter]
    fn y_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let y: Vec<f64> = (0..self.image_grid.ny())
            .map(|i| self.image_grid.y_at(i))
            .collect();
        y.into_pyarray(py)
    }

    /// Coherent (single-wavelength) intensity at the gap, `(ny, nx)`.
    #[getter]
    fn coherent_image<'py>(slf: &Bound<'py, Self>) -> Bound<'py, PyArray2<f64>> {
        crate::py_arrays::readonly_view(&slf.get().coherent, slf.as_any())
    }

    /// Infinite-scan DTL (stationary) image, `(ny, nx)`: gap independent.
    #[getter]
    fn stationary_image<'py>(slf: &Bound<'py, Self>) -> Bound<'py, PyArray2<f64>> {
        crate::py_arrays::readonly_view(&slf.get().stationary, slf.as_any())
    }

    /// DTL image averaged over gaps `[gap, gap + scan_length]`, `(ny, nx)`.
    #[getter]
    fn dtl_image<'py>(slf: &Bound<'py, Self>) -> Bound<'py, PyArray2<f64>> {
        crate::py_arrays::readonly_view(&slf.get().dtl, slf.as_any())
    }

    /// ATL (spectrally averaged) image at the gap, `(ny, nx)`, or `None` when
    /// `bandwidth_nm = 0`.
    #[getter]
    fn atl_image<'py>(slf: &Bound<'py, Self>) -> Option<Bound<'py, PyArray2<f64>>> {
        slf.get()
            .atl
            .as_ref()
            .map(|a| crate::py_arrays::readonly_view(a, slf.as_any()))
    }

    /// Paraxial Talbot length `2p²/λ` (nm).
    #[getter]
    fn talbot_length_nm(&self) -> f64 {
        self.talbot_length_nm
    }

    /// Exact first-order-pair Talbot length `λ/(1 − sqrt(1 − λ²/p²))` (nm), or
    /// `None` if the first order is evanescent.
    #[getter]
    fn talbot_length_exact_nm(&self) -> Option<f64> {
        self.talbot_length_exact_nm
    }

    /// Achromatic distance `2p²/Δλ` (nm), or `None` for monochromatic runs.
    #[getter]
    fn achromatic_distance_nm(&self) -> Option<f64> {
        self.achromatic_distance_nm
    }

    /// Grating period along x (nm).
    #[getter]
    fn period_nm(&self) -> f64 {
        self.period_nm
    }

    /// Vacuum (centre) wavelength (nm).
    #[getter]
    fn wavelength_nm(&self) -> f64 {
        self.wavelength_nm
    }

    /// Mask-to-wafer gap used for the coherent / ATL images (nm).
    #[getter]
    fn gap_nm(&self) -> f64 {
        self.gap_nm
    }

    /// Gap-scan length of the DTL image (nm).
    #[getter]
    fn scan_length_nm(&self) -> f64 {
        self.scan_length_nm
    }

    /// Diffraction efficiencies `(n, m, |c_nm|²)` of the retained orders.
    #[getter]
    fn efficiencies(&self) -> Vec<(i32, i32, f64)> {
        self.efficiencies.clone()
    }

    /// Intensity volume in the resist for the requested exposure mode, or
    /// `None` when `resist_thickness_nm = 0`.
    #[getter]
    fn intensity_volume(&self, py: Python<'_>) -> Option<Py<PyVolumetricResult>> {
        self.intensity_volume.as_ref().map(|v| v.clone_ref(py))
    }

    /// PAC volume `m = exp(−C·dose·I)`, or `None` when `dill_c = 0` or there is
    /// no resist volume.
    #[getter]
    fn pac_volume(&self, py: Python<'_>) -> Option<Py<PyVolumetricResult>> {
        self.pac_volume.as_ref().map(|v| v.clone_ref(py))
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
            "scalar thin-mask grating orders with exact angular-spectrum propagation",
            "coherent normal illumination; wavelength-independent grating coefficients; no Fresnel coefficients at the resist",
        ]
    }

    fn __repr__(&self) -> String {
        format!(
            "TalbotResult(period={} nm, λ={} nm, z_T={:.1} nm, gap={:.1} nm)",
            self.period_nm, self.wavelength_nm, self.talbot_length_nm, self.gap_nm
        )
    }
}

/// Talbot / displacement-Talbot (DTL) / achromatic-Talbot (ATL) lithography
/// of a transmission grating under a normally incident plane wave.
///
/// Computes the coherent Talbot carpet, the wafer-plane images at `gap_nm`
/// (coherent, infinite-scan stationary DTL, finite-scan DTL over
/// `scan_length_nm`, and ATL when `bandwidth_nm > 0`), and, when
/// `resist_thickness_nm > 0`, the intensity (and PAC) volume inside the resist
/// for the `exposure` mode (`coherent` / `dtl` / `stationary` / `atl`).
/// Defaults: gap = 2·z_A with a bandwidth, else z_T; scan = z_T; carpet depth
/// 2·z_T.
#[pyfunction]
#[pyo3(signature = (
    wavelength_nm=13.5,
    period_nm=100.0,
    grating="amplitude",
    duty_cycle=0.5,
    phase_rad=None,
    hole_diameter_nm=None,
    max_order=10,
    propagation="exact",
    gap_nm=None,
    n_periods=2,
    nx=128,
    ny=None,
    carpet_z_max_nm=None,
    carpet_nz=128,
    scan_length_nm=None,
    bandwidth_nm=0.0,
    spectrum="gaussian",
    spectral_bins=256,
    resist_thickness_nm=0.0,
    resist_index=1.0,
    absorption_per_nm=0.0,
    nz=16,
    exposure="dtl",
    dose_scale=1.0,
    dill_c=0.0,
))]
#[allow(clippy::too_many_arguments)]
fn simulate_talbot(
    py: Python<'_>,
    wavelength_nm: f64,
    period_nm: f64,
    grating: &str,
    duty_cycle: f64,
    phase_rad: Option<f64>,
    hole_diameter_nm: Option<f64>,
    max_order: usize,
    propagation: &str,
    gap_nm: Option<f64>,
    n_periods: usize,
    nx: usize,
    ny: Option<usize>,
    carpet_z_max_nm: Option<f64>,
    carpet_nz: usize,
    scan_length_nm: Option<f64>,
    bandwidth_nm: f64,
    spectrum: &str,
    spectral_bins: usize,
    resist_thickness_nm: f64,
    resist_index: f64,
    absorption_per_nm: f64,
    nz: usize,
    exposure: &str,
    dose_scale: f64,
    dill_c: f64,
) -> PyResult<PyTalbotResult> {
    let val = |e: highuvlith_core::error::LithographyError| PyValueError::new_err(e.to_string());
    let g = talbot_grating(
        grating,
        period_nm,
        duty_cycle,
        phase_rad.unwrap_or(std::f64::consts::PI),
        hole_diameter_nm,
        max_order,
    )?;
    let mut setup = TalbotSetup::new(g, wavelength_nm).map_err(val)?;
    setup.propagation = match propagation {
        "exact" => TalbotPropagation::AngularSpectrum,
        "paraxial" => TalbotPropagation::Paraxial,
        other => {
            return Err(PyValueError::new_err(format!(
                "unknown propagation '{other}' (expected exact / paraxial)"
            )))
        }
    };
    if bandwidth_nm.is_nan() || bandwidth_nm < 0.0 {
        return Err(PyValueError::new_err("bandwidth_nm must be >= 0"));
    }
    let spec = if bandwidth_nm > 0.0 {
        Some(match spectrum {
            "gaussian" => TalbotSpectrum::Gaussian {
                fwhm_nm: bandwidth_nm,
                bins: spectral_bins,
            },
            "flat" => TalbotSpectrum::FlatTop {
                bandwidth_nm,
                bins: spectral_bins,
            },
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown spectrum '{other}' (expected gaussian / flat)"
                )))
            }
        })
    } else {
        None
    };

    let z_t = setup.talbot_length_nm();
    let z_a = (bandwidth_nm > 0.0).then(|| talbot::achromatic_distance_nm(period_nm, bandwidth_nm));
    let gap = gap_nm.unwrap_or_else(|| z_a.map_or(z_t, |z| 2.0 * z));
    let scan = scan_length_nm.unwrap_or(z_t);
    let n_periods = n_periods.max(1) as f64;
    let span_x = n_periods * setup.grating.period_x_nm;
    let (ny, y_range) = match setup.grating.period_y_nm {
        Some(py_nm) => (ny.unwrap_or(nx), (0.0, n_periods * py_nm)),
        None => (ny.unwrap_or(1), (-0.5, 0.5)),
    };
    let z_max = carpet_z_max_nm.unwrap_or(2.0 * z_t);
    let volume_mode = match exposure {
        "coherent" => TalbotExposure::Coherent,
        "dtl" => TalbotExposure::Displacement {
            scan_length_nm: scan,
        },
        "stationary" => TalbotExposure::DisplacementStationary,
        "atl" => TalbotExposure::Achromatic(
            spec.clone()
                .ok_or_else(|| PyValueError::new_err("exposure = 'atl' needs bandwidth_nm > 0"))?,
        ),
        other => {
            return Err(PyValueError::new_err(format!(
                "unknown exposure '{other}' (expected coherent / dtl / stationary / atl)"
            )))
        }
    };
    let efficiencies = setup
        .grating
        .orders
        .iter()
        .map(|o| (o.n, o.m, o.efficiency()))
        .collect();

    let computed = py
        .allow_threads(|| -> highuvlith_core::error::Result<_> {
            let carpet = setup.carpet((0.0, span_x), nx, (0.0, z_max), carpet_nz, None)?;
            let mut image = Grid2D::<f64>::new(nx, ny, (0.0, span_x), y_range)?;
            setup.exposure_image(&mut image, gap, &TalbotExposure::Coherent)?;
            let coherent = image.data.clone();
            setup.exposure_image(&mut image, gap, &TalbotExposure::DisplacementStationary)?;
            let stationary = image.data.clone();
            setup.exposure_image(
                &mut image,
                gap,
                &TalbotExposure::Displacement {
                    scan_length_nm: scan,
                },
            )?;
            let dtl = image.data.clone();
            let atl = match &spec {
                Some(s) => {
                    setup.exposure_image(
                        &mut image,
                        gap,
                        &TalbotExposure::Achromatic(s.clone()),
                    )?;
                    Some(image.data.clone())
                }
                None => None,
            };
            let (volume, pac) = if resist_thickness_nm > 0.0 {
                let mut vol = Grid3D::<f64>::new(
                    nx,
                    ny,
                    nz,
                    (0.0, span_x),
                    y_range,
                    (0.0, resist_thickness_nm),
                )?;
                let medium = ResistMedium {
                    index: resist_index,
                    absorption_per_nm,
                };
                setup.resist_volume(&mut vol, gap, &medium, &volume_mode)?;
                let pac = (dill_c > 0.0).then(|| {
                    interference::expose(&vol, dose_scale, dill_c, ExposureKinetics::OnePhoton)
                });
                (Some(vol), pac)
            } else {
                (None, None)
            };
            Ok((carpet, image, coherent, stationary, dtl, atl, volume, pac))
        })
        .map_err(val)?;
    let (carpet, image_grid, coherent, stationary, dtl, atl, intensity_volume, pac_volume) =
        computed;

    Ok(PyTalbotResult {
        carpet,
        image_grid,
        coherent,
        stationary,
        dtl,
        atl,
        talbot_length_nm: z_t,
        talbot_length_exact_nm: setup.talbot_length_exact_nm(),
        achromatic_distance_nm: z_a,
        period_nm: setup.grating.period_x_nm,
        wavelength_nm,
        gap_nm: gap,
        scan_length_nm: scan,
        efficiencies,
        intensity_volume: intensity_volume
            .map(|v| PyVolumetricResult::py_new(py, v))
            .transpose()?,
        pac_volume: pac_volume
            .map(|v| PyVolumetricResult::py_new(py, v))
            .transpose()?,
    })
}

/// Result of [`simulate_euv_il`]: two-grating interference fringes.
#[pyclass(name = "EuvIlResult", frozen)]
pub struct PyEuvIlResult {
    il: TwoGratingInterference,
    intensity: Py<PyVolumetricResult>,
    pac: Py<PyVolumetricResult>,
}

#[pymethods]
impl PyEuvIlResult {
    /// Fringe period `p/(2m)` (nm) — independent of the wavelength.
    #[getter]
    fn fringe_period_nm(&self) -> f64 {
        self.il.fringe_period_nm()
    }

    /// Diffraction half-angle θ (deg), `sin θ = mλ/p`.
    #[getter]
    fn diffraction_angle_deg(&self) -> f64 {
        self.il.diffraction_angle_deg()
    }

    /// Fringe visibility `2 sqrt(I₁I₂)/(I₁ + I₂)`.
    #[getter]
    fn visibility(&self) -> f64 {
        self.il.visibility()
    }

    /// Relative beam intensities `(I₁, I₂)` (grating efficiencies × balance).
    #[getter]
    fn beam_intensities(&self) -> (f64, f64) {
        (self.il.beam_intensity_1, self.il.beam_intensity_2)
    }

    /// Intensity volume `(nz, ny, nx)` in the resist.
    #[getter]
    fn intensity(&self, py: Python<'_>) -> Py<PyVolumetricResult> {
        self.intensity.clone_ref(py)
    }

    /// PAC volume `m = exp(−C·dose·I)`.
    #[getter]
    fn pac(&self, py: Python<'_>) -> Py<PyVolumetricResult> {
        self.pac.clone_ref(py)
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
            "fringe period p/(2m) and visibility exact for ideal TE beams from thin-mask orders",
            "no zero-order background, partial coherence or mask 3D",
        ]
    }

    fn __repr__(&self) -> String {
        format!(
            "EuvIlResult(fringe_period={:.2} nm, θ={:.3}°, visibility={:.3})",
            self.il.fringe_period_nm(),
            self.il.diffraction_angle_deg(),
            self.il.visibility()
        )
    }
}

/// Two-grating EUV interference lithography: the +m / −m diffraction orders of
/// two transmission gratings of period `grating_period_nm` interfere at the
/// wafer, printing fringes of period `p/(2m)` independent of the wavelength.
///
/// Beam intensities are the grating's m-th-order efficiencies (`grating` /
/// `duty_cycle` / `phase_rad` as in `simulate_talbot`); `intensity_ratio`
/// scales beam 2 to model imbalance. The intensity volume spans `n_fringes`
/// fringe periods in x and `z_span_nm` of resist (index `n_medium`, absorption
/// `absorption_per_nm`).
#[pyfunction]
#[pyo3(signature = (
    grating_period_nm=100.0,
    wavelength_nm=13.5,
    order=1,
    grating="amplitude",
    duty_cycle=0.5,
    phase_rad=None,
    intensity_ratio=1.0,
    n_medium=1.0,
    absorption_per_nm=0.0,
    n_fringes=8,
    nx=128,
    ny=4,
    nz=16,
    z_span_nm=50.0,
    dose_scale=1.0,
    dill_c=0.02,
))]
#[allow(clippy::too_many_arguments)]
fn simulate_euv_il(
    py: Python<'_>,
    grating_period_nm: f64,
    wavelength_nm: f64,
    order: u32,
    grating: &str,
    duty_cycle: f64,
    phase_rad: Option<f64>,
    intensity_ratio: f64,
    n_medium: f64,
    absorption_per_nm: f64,
    n_fringes: usize,
    nx: usize,
    ny: usize,
    nz: usize,
    z_span_nm: f64,
    dose_scale: f64,
    dill_c: f64,
) -> PyResult<PyEuvIlResult> {
    let val = |e: highuvlith_core::error::LithographyError| PyValueError::new_err(e.to_string());
    if intensity_ratio.is_nan() || intensity_ratio < 0.0 {
        return Err(PyValueError::new_err("intensity_ratio must be >= 0"));
    }
    let g = talbot_grating(
        grating,
        grating_period_nm,
        duty_cycle,
        phase_rad.unwrap_or(std::f64::consts::PI),
        None,
        (order as usize).max(1),
    )?;
    if !g.is_1d() {
        return Err(PyValueError::new_err(
            "simulate_euv_il needs a 1D grating (amplitude / phase / sinusoidal)",
        ));
    }
    let mut il = TwoGratingInterference::new(&g, wavelength_nm, order).map_err(val)?;
    il.beam_intensity_2 *= intensity_ratio;
    let setup = il
        .to_interference_setup(n_medium, absorption_per_nm)
        .map_err(val)?;
    let span = n_fringes.max(1) as f64 * il.fringe_period_nm();
    let mut grid = Grid3D::<f64>::new(
        nx,
        ny,
        nz,
        (0.0, span),
        (0.0, il.fringe_period_nm()),
        (0.0, z_span_nm),
    )
    .map_err(val)?;
    let pac = py.allow_threads(|| {
        setup.intensity(&mut grid);
        interference::expose(&grid, dose_scale, dill_c, ExposureKinetics::OnePhoton)
    });
    Ok(PyEuvIlResult {
        il,
        intensity: PyVolumetricResult::py_new(py, grid)?,
        pac: PyVolumetricResult::py_new(py, pac)?,
    })
}

/// Register the volumetric / deep-layer classes and functions on the module.
pub fn register_volumetric_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyVolumetricResult>()?;
    m.add_class::<PyHeightMapResult>()?;
    m.add_class::<PyLigaResult>()?;

    m.add_function(wrap_pyfunction!(expose_volumetric, m)?)?;
    m.add_function(wrap_pyfunction!(develop_fast_marching, m)?)?;
    m.add_function(wrap_pyfunction!(height_map_from_times, m)?)?;
    m.add_function(wrap_pyfunction!(simulate_liga, m)?)?;
    m.add_function(wrap_pyfunction!(grayscale_height_map, m)?)?;
    m.add_function(wrap_pyfunction!(blazed_grating, m)?)?;
    m.add_function(wrap_pyfunction!(microlens_array, m)?)?;
    m.add_function(wrap_pyfunction!(grayscale_transmittance_for_target, m)?)?;
    m.add_function(wrap_pyfunction!(simulate_interference, m)?)?;
    m.add_function(wrap_pyfunction!(quantum_aerial_image, m)?)?;

    // Level-set development and post-exposure bake
    m.add_class::<PyLevelSetResult>()?;
    m.add_class::<PyCarPebResult>()?;
    m.add_function(wrap_pyfunction!(develop_level_set, m)?)?;
    m.add_function(wrap_pyfunction!(development_rate, m)?)?;
    m.add_function(wrap_pyfunction!(peb_gaussian, m)?)?;
    m.add_function(wrap_pyfunction!(peb_car, m)?)?;

    // Talbot / EUV-IL
    m.add_class::<PyTalbotResult>()?;
    m.add_class::<PyEuvIlResult>()?;
    m.add_function(wrap_pyfunction!(simulate_talbot, m)?)?;
    m.add_function(wrap_pyfunction!(simulate_euv_il, m)?)?;

    Ok(())
}
