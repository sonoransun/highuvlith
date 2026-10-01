//! Python bindings for the quantum (N-photon) lithography research module
//! ([`highuvlith_core::quantum`], 🧪 Theoretical).
//!
//! Two physically different models are kept apart, as in the core:
//! classical N-photon absorption `I^N` (sharpening at the classical period,
//! [`n_photon_absorption_image`]) and the ideal entangled N00N models — the
//! analytic two-beam fringe at `λ/(2N sinθ)` ([`PyTwoBeamNPhoton`]) and the
//! `λ/N` ideal-limit image of a general mask
//! (`SimulationEngine.compute_noon_ideal_image`, built here by
//! [`noon_image_result`]).

use numpy::{IntoPyArray, PyArray1, PyArray2, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use highuvlith_core::quantum::{self, FluxBudget, NoonImage, TwoBeamNPhoton};

use crate::py_results::PyAerialImageResult;

fn value_err(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}

const STATUS: &str = "🧪";

/// The flux budget as a Python dict (keys = `FluxBudget` field names).
fn flux_dict<'py>(py: Python<'py>, f: &FluxBudget) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("num_photons", f.num_photons)?;
    d.set_item("photon_energy_ev", f.photon_energy_ev)?;
    d.set_item("hvm_photon_rate_per_s", f.hvm_photon_rate_per_s)?;
    d.set_item(
        "entangled_rate_ceiling_per_s",
        f.entangled_rate_ceiling_per_s,
    )?;
    d.set_item("relative_flux", f.relative_flux)?;
    d.set_item("exposure_time_ratio_bound", f.exposure_time_ratio_bound)?;
    d.set_item(
        "exposure_time_ratio_tightest_bound",
        f.exposure_time_ratio_tightest_bound,
    )?;
    d.set_item("exposure_time_ratio_claimed", f.exposure_time_ratio_claimed)?;
    Ok(d)
}

/// Classical N-photon absorption of a classical aerial image: `E = I^N`
/// (negative roundoff clamped to 0). Sharpening at the classical period —
/// `I^N` contains no spatial period the classical image lacks, so it does
/// not improve resolution. `classical` is a 2D float64 array; the result
/// has the same shape. 🧪 Theoretical.
#[pyfunction]
#[pyo3(signature = (classical, num_photons=2))]
fn n_photon_absorption_image<'py>(
    py: Python<'py>,
    classical: PyReadonlyArray2<f64>,
    num_photons: usize,
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let classical = classical.as_array().to_owned();
    let image = py
        .allow_threads(|| quantum::n_photon_absorption_image(&classical, num_photons))
        .map_err(value_err)?;
    Ok(image.into_pyarray(py))
}

/// Flux and exposure-time budget of entangled N-photon exposure at
/// `wavelength_nm` relative to an HVM-class classical exposure, derived from
/// the entangled-photon source constants. Returns a dict:
/// `num_photons`, `photon_energy_ev`, `hvm_photon_rate_per_s`,
/// `entangled_rate_ceiling_per_s` (order of the entangled-regime crossover
/// flux; bandwidth-dependent, 1e13 photons/s representative),
/// `relative_flux` (order-of-magnitude estimate), and the exposure-time
/// ratios `exposure_time_ratio_bound` (ETPA cross-section at its loosest
/// independent upper bound — a LOWER bound on the ratio),
/// `exposure_time_ratio_tightest_bound` and `exposure_time_ratio_claimed`
/// (largest disputed early claim; optimistic). N = 1 gives 1 everywhere.
#[pyfunction]
#[pyo3(signature = (wavelength_nm=157.63, num_photons=2))]
fn quantum_flux_budget(
    py: Python<'_>,
    wavelength_nm: f64,
    num_photons: usize,
) -> PyResult<Bound<'_, PyDict>> {
    if !(wavelength_nm.is_finite() && wavelength_nm > 0.0) {
        return Err(PyValueError::new_err(format!(
            "wavelength_nm must be positive and finite, got {wavelength_nm}"
        )));
    }
    if num_photons == 0 || num_photons > quantum::MAX_PHOTONS {
        return Err(PyValueError::new_err(format!(
            "num_photons must be in 1..={}, got {num_photons}",
            quantum::MAX_PHOTONS
        )));
    }
    flux_dict(py, &FluxBudget::new(wavelength_nm, num_photons))
}

/// N-photon exposure of a two-beam interference pattern in closed form
/// (🧪 Theoretical): the ideal N00N fringe `1 + cos(N(Kx + φ))` with period
/// `λ/(2N sinθ)`, the classical N-photon fringe `(1 + cos(Kx + φ))^N / m_N`
/// with the classical period `λ/(2 sinθ)`, and their fidelity mixture
/// `F·E_NOON + (1 − F)·E_cl` — all with unit mean. `half_angle_deg` is the
/// air-side half-angle of each beam (0 < θ < 90°); `phase_rad` the relative
/// single-photon phase φ. Positions are in nm.
#[pyclass(name = "TwoBeamNPhoton", frozen)]
pub struct PyTwoBeamNPhoton {
    inner: TwoBeamNPhoton,
}

#[pymethods]
impl PyTwoBeamNPhoton {
    #[new]
    #[pyo3(signature = (wavelength_nm, half_angle_deg, num_photons=2, fidelity=1.0, phase_rad=0.0))]
    fn new(
        wavelength_nm: f64,
        half_angle_deg: f64,
        num_photons: usize,
        fidelity: f64,
        phase_rad: f64,
    ) -> PyResult<Self> {
        let inner = TwoBeamNPhoton {
            wavelength_nm,
            half_angle_deg,
            num_photons,
            fidelity,
            phase_rad,
        };
        inner.validate().map_err(value_err)?;
        Ok(Self { inner })
    }

    #[getter]
    fn wavelength_nm(&self) -> f64 {
        self.inner.wavelength_nm
    }

    #[getter]
    fn half_angle_deg(&self) -> f64 {
        self.inner.half_angle_deg
    }

    #[getter]
    fn num_photons(&self) -> usize {
        self.inner.num_photons
    }

    #[getter]
    fn fidelity(&self) -> f64 {
        self.inner.fidelity
    }

    #[getter]
    fn phase_rad(&self) -> f64 {
        self.inner.phase_rad
    }

    /// Classical fringe wavenumber `K = 4π sinθ / λ` (rad/nm).
    fn fringe_wavenumber_per_nm(&self) -> f64 {
        self.inner.fringe_wavenumber_per_nm()
    }

    /// Classical fringe period `λ / (2 sinθ)` (nm) — also the fundamental
    /// period of classical N-photon absorption.
    fn classical_period_nm(&self) -> f64 {
        self.inner.classical_period_nm()
    }

    /// Ideal N00N fringe period `λ / (2N sinθ)` (nm).
    fn noon_period_nm(&self) -> f64 {
        self.inner.noon_period_nm()
    }

    /// Ideal N00N exposure `1 + cos(N(Kx + φ))` at `x_nm`.
    fn noon_exposure(&self, x_nm: f64) -> f64 {
        self.inner.noon_exposure(x_nm)
    }

    /// Classical N-photon exposure `(1 + cos(Kx + φ))^N / m_N` at `x_nm`.
    fn classical_exposure(&self, x_nm: f64) -> f64 {
        self.inner.classical_exposure(x_nm)
    }

    /// Mixed exposure `F·E_NOON + (1 − F)·E_cl` at `x_nm`.
    fn exposure(&self, x_nm: f64) -> f64 {
        self.inner.exposure(x_nm)
    }

    /// Mixed exposure at every position of the 1D float64 array `x_nm`.
    fn profile<'py>(
        &self,
        py: Python<'py>,
        x_nm: PyReadonlyArray1<f64>,
    ) -> PyResult<Bound<'py, PyArray1<f64>>> {
        let x: Vec<f64> = x_nm.as_array().to_vec();
        Ok(self.inner.profile(&x).into_pyarray(py))
    }

    /// Exact cosine amplitude of the `j`-th harmonic `cos(j(Kx + φ))` of the
    /// mixed exposure: 1 for j = 0, `F·[j = N] + (1 − F)·2C(2N, N−j)/C(2N, N)`
    /// for 1 ≤ j ≤ N, 0 above.
    fn harmonic_amplitude(&self, j: usize) -> f64 {
        self.inner.harmonic_amplitude(j)
    }

    /// Flux and exposure-time budget at this wavelength and N (see
    /// `quantum_flux_budget`).
    fn flux_budget<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        flux_dict(
            py,
            &FluxBudget::new(self.inner.wavelength_nm, self.inner.num_photons),
        )
    }

    /// Capability badge: "🧪" (theoretical research model).
    #[getter]
    fn status(&self) -> &'static str {
        STATUS
    }

    /// The model's stated approximations.
    #[getter]
    fn notes(&self) -> Vec<&'static str> {
        vec![
            "ideal path-entangled N00N state (Boto et al. 2000); no photon loss beyond the fidelity mixture",
            "the non-N00N fraction is classical N-photon absorption of the classical fringe",
            "no N-photon resist has recorded an entangled sub-Rayleigh pattern",
        ]
    }

    fn __repr__(&self) -> String {
        format!(
            "TwoBeamNPhoton(λ={} nm, θ={}°, N={}, F={}, classical_period={:.2} nm, noon_period={:.2} nm)",
            self.inner.wavelength_nm,
            self.inner.half_angle_deg,
            self.inner.num_photons,
            self.inner.fidelity,
            self.inner.classical_period_nm(),
            self.inner.noon_period_nm()
        )
    }
}

/// Result of `SimulationEngine.compute_noon_ideal_image` (🧪 Theoretical):
/// the classical image `I_λ`, its classical N-photon absorption `I_λ^N`
/// (sharpening, classical period), the ideal N00N limit `I_{λ/N}` (the same
/// source fill, optics, mask and grid imaged at `λ/N` — Boto et al.'s ideal
/// limit, not a simulation of state preparation) and the exposure
/// `F·I_{λ/N} + (1 − F)·I_λ^N`. All images are clear-field normalized
/// `AerialImageResult`s.
#[pyclass(name = "NoonImageResult", frozen)]
pub struct PyNoonImageResult {
    classical: Py<PyAerialImageResult>,
    n_photon_absorption: Py<PyAerialImageResult>,
    noon_limit: Py<PyAerialImageResult>,
    exposure: Py<PyAerialImageResult>,
    num_photons: usize,
    fidelity: f64,
    wavelength_nm: f64,
    effective_wavelength_nm: f64,
    classical_resolution_nm: f64,
    noon_limit_resolution_nm: f64,
    flux: FluxBudget,
}

/// Wrap a core [`NoonImage`] (the images become `AerialImageResult`s).
pub(crate) fn noon_image_result(py: Python<'_>, img: NoonImage) -> PyResult<PyNoonImageResult> {
    let wrap = |g| Py::new(py, PyAerialImageResult::from_grid2d(g));
    Ok(PyNoonImageResult {
        classical: wrap(img.classical)?,
        n_photon_absorption: wrap(img.n_photon_absorption)?,
        noon_limit: wrap(img.noon_limit)?,
        exposure: wrap(img.exposure)?,
        num_photons: img.num_photons,
        fidelity: img.fidelity,
        wavelength_nm: img.wavelength_nm,
        effective_wavelength_nm: img.effective_wavelength_nm,
        classical_resolution_nm: img.classical_resolution_nm,
        noon_limit_resolution_nm: img.noon_limit_resolution_nm,
        flux: img.flux,
    })
}

#[pymethods]
impl PyNoonImageResult {
    /// Classical aerial image `I_λ`.
    #[getter]
    fn classical(&self, py: Python<'_>) -> Py<PyAerialImageResult> {
        self.classical.clone_ref(py)
    }

    /// Classical N-photon absorption `I_λ^N` (same period as `classical`).
    #[getter]
    fn n_photon_absorption(&self, py: Python<'_>) -> Py<PyAerialImageResult> {
        self.n_photon_absorption.clone_ref(py)
    }

    /// Ideal N00N limit: the same system imaged at `λ/N`.
    #[getter]
    fn noon_limit(&self, py: Python<'_>) -> Py<PyAerialImageResult> {
        self.noon_limit.clone_ref(py)
    }

    /// Exposure `F·I_{λ/N} + (1 − F)·I_λ^N`.
    #[getter]
    fn exposure(&self, py: Python<'_>) -> Py<PyAerialImageResult> {
        self.exposure.clone_ref(py)
    }

    #[getter]
    fn num_photons(&self) -> usize {
        self.num_photons
    }

    #[getter]
    fn fidelity(&self) -> f64 {
        self.fidelity
    }

    /// Physical wavelength λ (nm).
    #[getter]
    fn wavelength_nm(&self) -> f64 {
        self.wavelength_nm
    }

    /// Effective wavelength `λ/N` of the ideal limit (nm).
    #[getter]
    fn effective_wavelength_nm(&self) -> f64 {
        self.effective_wavelength_nm
    }

    /// Classical Rayleigh resolution `0.61 λ/NA` (nm).
    #[getter]
    fn classical_resolution_nm(&self) -> f64 {
        self.classical_resolution_nm
    }

    /// Ideal-limit Rayleigh resolution `0.61 λ/(N NA)` (nm).
    #[getter]
    fn noon_limit_resolution_nm(&self) -> f64 {
        self.noon_limit_resolution_nm
    }

    /// Flux and exposure-time budget (dict; see `quantum_flux_budget`).
    #[getter]
    fn flux<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        flux_dict(py, &self.flux)
    }

    /// Capability badge: "🧪" (theoretical research model).
    #[getter]
    fn status(&self) -> &'static str {
        STATUS
    }

    /// The model's stated approximations.
    #[getter]
    fn notes(&self) -> Vec<&'static str> {
        vec![
            "ideal limit: assumes entangled states that write the pattern exist (no state preparation modelled)",
            "optics evaluated at lambda/N: wavelength-dependent coatings/materials are at the wrong wavelength",
            "decoherent fraction = classical N-photon absorption I^N; no photon loss model beyond F",
        ]
    }

    fn __repr__(&self) -> String {
        format!(
            "NoonImageResult(N={}, F={}, λ={} nm → λ/N={} nm, resolution {:.1} → {:.1} nm)",
            self.num_photons,
            self.fidelity,
            self.wavelength_nm,
            self.effective_wavelength_nm,
            self.classical_resolution_nm,
            self.noon_limit_resolution_nm
        )
    }
}

/// Register the quantum classes and functions.
pub fn register_quantum_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTwoBeamNPhoton>()?;
    m.add_class::<PyNoonImageResult>()?;
    m.add_function(wrap_pyfunction!(n_photon_absorption_image, m)?)?;
    m.add_function(wrap_pyfunction!(quantum_flux_budget, m)?)?;
    Ok(())
}
