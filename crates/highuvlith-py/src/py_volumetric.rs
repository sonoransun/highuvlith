//! PyO3 bindings for the deep-layer (volumetric / z-resolved) lithography
//! modules: separable volumetric resist exposure and 3D development
//! ([`highuvlith_core::volumetric`]), LIGA deep-X-ray shadow printing
//! ([`highuvlith_core::deep_xray`]), grayscale 2.5D topography
//! ([`highuvlith_core::grayscale`]), multi-beam interference / two-photon
//! lattices ([`highuvlith_core::interference`]), and the theoretical quantum
//! N-photon aerial image ([`highuvlith_core::quantum`]).
//!
//! 3D fields cross the boundary zero-copy as numpy arrays with axis order
//! `(nz, ny, nx)` — depth first, matching `Grid3D::data` — and compute-heavy
//! calls release the GIL with `py.allow_threads`.

use numpy::{PyArray1, PyArray2, PyArray3, PyReadonlyArray2, ToPyArray};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

use highuvlith_core::aerial::AerialImageEngine;
use highuvlith_core::deep_xray::{self, DeepXrayConfig, XraySpectrum};
use highuvlith_core::grayscale::{self, ContrastCurve, GrayscaleMap};
use highuvlith_core::interference::{self, ExposureKinetics, InterferenceSetup};
use highuvlith_core::mask::Mask;
use highuvlith_core::metrics;
use highuvlith_core::quantum::{compute_quantum_aerial_image, QuantumLithographyParams};
use highuvlith_core::source::LithographySource;
use highuvlith_core::types::{Complex64, Grid2D, Grid3D};
use highuvlith_core::volumetric::{self, VolumetricExposureConfig, VolumetricLatentImage};

use crate::py_config::*;

/// A z-resolved scalar field on a 3D grid (PAC concentration, absorbed dose,
/// or fast-marching arrival times, depending on how it was produced).
///
/// `values` is a numpy array of shape `(nz, ny, nx)`; the depth axis `z` runs
/// from the resist top downward. `x_nm`/`y_nm`/`z_nm` are the physical
/// cell-center coordinates along each axis.
#[pyclass(name = "VolumetricResult")]
#[derive(Clone)]
pub struct PyVolumetricResult {
    inner: Grid3D<f64>,
}

impl PyVolumetricResult {
    fn new(inner: Grid3D<f64>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVolumetricResult {
    /// The field values as a `(nz, ny, nx)` numpy array.
    #[getter]
    fn values<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray3<f64>> {
        self.inner.data.to_pyarray(py)
    }

    /// x cell-center coordinates in nm (length nx).
    #[getter]
    fn x_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let x: Vec<f64> = (0..self.inner.nx()).map(|j| self.inner.x_at(j)).collect();
        x.to_pyarray(py)
    }

    /// y cell-center coordinates in nm (length ny).
    #[getter]
    fn y_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let y: Vec<f64> = (0..self.inner.ny()).map(|i| self.inner.y_at(i)).collect();
        y.to_pyarray(py)
    }

    /// z (depth-from-top) cell-center coordinates in nm (length nz).
    #[getter]
    fn z_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let z: Vec<f64> = (0..self.inner.nz()).map(|k| self.inner.z_at(k)).collect();
        z.to_pyarray(py)
    }

    /// The `(nz, ny, nx)` shape of the field.
    #[getter]
    fn shape(&self) -> (usize, usize, usize) {
        (self.inner.nz(), self.inner.ny(), self.inner.nx())
    }

    /// Depth-resolved critical dimension: the center-row CD in every z-slice
    /// at `threshold` (on the field value), top to bottom. Slices with no
    /// threshold crossing return `None`.
    fn cd_at_z(&self, threshold: f64) -> Vec<Option<f64>> {
        metrics::cd_at_z(&self.inner, threshold)
    }

    /// Per-column development depth map (nm) by thresholding this volume as a
    /// PAC latent image: scan each column from the top and take the depth of
    /// the contiguous run of voxels with PAC below `threshold`.
    fn depth_map(&self, threshold: f64) -> PyHeightMapResult {
        let latent = VolumetricLatentImage {
            pac: self.inner.clone(),
        };
        PyHeightMapResult::new(volumetric::develop_depth_map(&latent, threshold))
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
#[pyclass(name = "HeightMapResult")]
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
    /// The map values as a `(ny, nx)` numpy array.
    #[getter]
    fn values<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray2<f64>> {
        self.inner.data.to_pyarray(py)
    }

    /// x cell-center coordinates in nm (length nx).
    #[getter]
    fn x_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let x: Vec<f64> = (0..self.inner.nx()).map(|j| self.inner.x_at(j)).collect();
        x.to_pyarray(py)
    }

    /// y cell-center coordinates in nm (length ny).
    #[getter]
    fn y_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let y: Vec<f64> = (0..self.inner.ny()).map(|i| self.inner.y_at(i)).collect();
        y.to_pyarray(py)
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
/// `n_defocus_planes` stepped depths and interpolated in z) is combined with
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
/// The development rate model comes from `resist`.
#[pyfunction]
#[pyo3(signature = (volume, resist, pixel_xy_nm, pixel_z_nm))]
fn develop_fast_marching(
    py: Python<'_>,
    volume: &PyVolumetricResult,
    resist: PyResistConfig,
    pixel_xy_nm: f64,
    pixel_z_nm: f64,
) -> PyVolumetricResult {
    let latent = VolumetricLatentImage {
        pac: volume.inner.clone(),
    };
    let times = py.allow_threads(|| {
        volumetric::develop_fast_marching(&latent, &resist.inner, pixel_xy_nm, pixel_z_nm)
    });
    PyVolumetricResult::new(times)
}

/// Remaining-height map (nm) after `dev_time_s` seconds of development, from a
/// fast-marching arrival-time volume (see [`develop_fast_marching`]).
#[pyfunction]
#[pyo3(signature = (times, dev_time_s))]
fn height_map_from_times(times: &PyVolumetricResult, dev_time_s: f64) -> PyHeightMapResult {
    PyHeightMapResult::new(volumetric::height_map_from_times(&times.inner, dev_time_s))
}

/// LIGA deep-X-ray exposure result: absolute depth dose, top/bottom contrast,
/// the volumetric absorbed-dose field, and the developed-depth map.
#[pyclass(name = "LigaResult")]
pub struct PyLigaResult {
    z_um: Vec<f64>,
    dose_kj_cm3: Vec<f64>,
    dose_ratio: f64,
    top_dose_kj_cm3: f64,
    bottom_dose_kj_cm3: f64,
    exceeds_damage_ceiling: bool,
    volume: Grid3D<f64>,
    developed: Grid2D<f64>,
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
        (self.z_um.to_pyarray(py), self.dose_kj_cm3.to_pyarray(py))
    }

    /// Top/bottom absorbed-dose ratio (>= 1; the process contrast).
    #[getter]
    fn dose_ratio(&self) -> f64 {
        self.dose_ratio
    }

    /// Absorbed-energy density at the resist top in kJ/cm^3.
    #[getter]
    fn top_dose_kj_cm3(&self) -> f64 {
        self.top_dose_kj_cm3
    }

    /// Absorbed-energy density at the resist bottom in kJ/cm^3.
    #[getter]
    fn bottom_dose_kj_cm3(&self) -> f64 {
        self.bottom_dose_kj_cm3
    }

    /// Whether the top dose exceeds the damage ceiling (foaming / T-topping).
    #[getter]
    fn exceeds_damage_ceiling(&self) -> bool {
        self.exceeds_damage_ceiling
    }

    /// Volumetric absorbed-dose field `D(x,y,z)` in kJ/cm^3.
    #[getter]
    fn volume(&self) -> PyVolumetricResult {
        PyVolumetricResult::new(self.volume.clone())
    }

    /// Developed-depth map (nm) at the clearing (target bottom) dose.
    #[getter]
    fn developed_depth(&self) -> PyHeightMapResult {
        PyHeightMapResult::new(self.developed.clone())
    }

    fn __repr__(&self) -> String {
        format!(
            "LigaResult(dose_ratio={:.2}, top={:.2} kJ/cm^3, exceeds_damage={})",
            self.dose_ratio, self.top_dose_kj_cm3, self.exceeds_damage_ceiling
        )
    }
}

/// Simulate a LIGA deep-X-ray shadow exposure of thick PMMA under a
/// bending-magnet white beam.
///
/// Uses the standard thick-PMMA preset (20 um Au absorber on a 2 um Ti
/// membrane, 100 um proximity gap, 3 kJ/cm^3 bottom clearing dose) with the
/// requested critical energy and resist thickness, over a line/space mask.
/// Returns depth dose, the volumetric dose field, and the developed depth at
/// the clearing dose.
#[pyfunction]
#[pyo3(signature = (
    critical_energy_kev=6.23,
    resist_thickness_um=500.0,
    cd_nm=5000.0,
    pitch_nm=10000.0,
    grid_size=128,
    pixel_nm=200.0,
    nz=64,
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
) -> PyResult<PyLigaResult> {
    let grid = highuvlith_core::types::GridConfig::new(grid_size, pixel_nm)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    let mask =
        Mask::line_space(cd_nm, pitch_nm).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let mut config = DeepXrayConfig::pmma_default(XraySpectrum::BendingMagnet {
        critical_energy_kev,
    });
    config.resist_thickness_um = resist_thickness_um;
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
        z_um: exposure.z_um,
        dose_kj_cm3: exposure.dose_kj_cm3,
        dose_ratio: exposure.dose_ratio,
        top_dose_kj_cm3: exposure.top_dose_kj_cm3,
        bottom_dose_kj_cm3: exposure.bottom_dose_kj_cm3,
        exceeds_damage_ceiling: exposure.exceeds_damage_ceiling,
        volume,
        developed,
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
    grayscale::blazed_grating(n, period_px, depth_nm, thickness_nm).to_pyarray(py)
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
    grayscale::microlens_array(n, pitch_px, sag_nm, thickness_nm).to_pyarray(py)
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
    let map = GrayscaleMap::from_target_height(&target, thickness_nm, &curve, exposure_dose)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    Ok(map.transmittance.to_pyarray(py))
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

/// Quantum N-photon aerial image: `E_N = fidelity * I^N + (1 - fidelity) * I`
/// applied to a classical intensity image (theoretical NOON-state lithography;
/// sharper features than classical `|E|^2` imaging). `classical` is a 2D
/// float64 array; the return has the same shape.
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
    let quantum = compute_quantum_aerial_image(&classical, &params)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(quantum.to_pyarray(py))
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

    Ok(())
}
