//! PyO3 bindings for photon shot noise and stochastic line-edge / line-width
//! roughness ([`highuvlith_core::stochastic`], 🔶 simplified: Poisson photon
//! counting on the aerial image with a Gamma-distributed per-exposure dose
//! factor; no acid/quencher shot noise or secondary-electron blur), plus the
//! photon-counting helpers of the throughput model.

use numpy::{PyArray1, ToPyArray};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use highuvlith_core::source_models::throughput;
use highuvlith_core::stochastic::{self, LerResult, StochasticParams};

use crate::py_config::PySourceConfig;
use crate::py_results::PyAerialImageResult;

/// Monte-Carlo LER/LWR statistics of the feature straddling the field centre
/// (y = centre row) over the photon-noise realizations.
#[pyclass(name = "LerResult", frozen)]
pub struct PyLerResult {
    inner: LerResult,
    photon_density_per_mj_cm2: f64,
    dose_jitter_rms: f64,
    num_realizations: usize,
}

#[pymethods]
impl PyLerResult {
    /// Line-edge roughness, 3σ of the edge position (nm).
    #[getter]
    fn ler_3sigma_nm(&self) -> f64 {
        self.inner.ler_3sigma_nm
    }

    /// Line-width roughness, 3σ of the CD (nm).
    #[getter]
    fn lwr_3sigma_nm(&self) -> f64 {
        self.inner.lwr_3sigma_nm
    }

    /// Mean CD over the measured realizations (nm).
    #[getter]
    fn cd_mean_nm(&self) -> f64 {
        self.inner.cd_mean_nm
    }

    /// CD standard deviation (nm).
    #[getter]
    fn cd_sigma_nm(&self) -> f64 {
        self.inner.cd_sigma_nm
    }

    /// Left-edge positions (nm), one per realization where the feature printed.
    #[getter]
    fn left_edges_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        self.inner.left_edges.to_pyarray(py)
    }

    /// Right-edge positions (nm), one per realization where the feature printed.
    #[getter]
    fn right_edges_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        self.inner.right_edges.to_pyarray(py)
    }

    /// Realizations in which both edges were found (out of `num_realizations`).
    #[getter]
    fn measured_realizations(&self) -> usize {
        self.inner.left_edges.len()
    }

    /// Realizations simulated.
    #[getter]
    fn num_realizations(&self) -> usize {
        self.num_realizations
    }

    /// Photons per nm² per mJ/cm² used (incident, or absorbed when an
    /// absorbed fraction was given).
    #[getter]
    fn photon_density_per_mj_cm2(&self) -> f64 {
        self.photon_density_per_mj_cm2
    }

    /// Relative rms per-exposure dose jitter used.
    #[getter]
    fn dose_jitter_rms(&self) -> f64 {
        self.dose_jitter_rms
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
            "Poisson photon noise on one image row plus a Gamma-distributed per-exposure dose factor",
            "no resist chemistry, acid diffusion or secondary-electron blur in the loop",
        ]
    }

    fn __repr__(&self) -> String {
        format!(
            "LerResult(LER={:.3} nm, LWR={:.3} nm, CD={:.2} nm, {}/{} realizations)",
            self.inner.ler_3sigma_nm,
            self.inner.lwr_3sigma_nm,
            self.inner.cd_mean_nm,
            self.inner.left_edges.len(),
            self.num_realizations
        )
    }
}

/// Stochastic LER/LWR of an aerial image (🔶): each of `num_realizations`
/// Monte-Carlo realizations Poisson-samples the photons per pixel at
/// `dose_mj_cm2` (Gaussian above 1000 photons), scales by a Gamma-distributed
/// per-exposure dose factor of rms `dose_jitter_rms`, and finds the two
/// `threshold` crossings around the field centre on the centre row; LER is
/// 3σ of the edge positions, LWR 3σ of the CD. Fixed seed (reproducible).
///
/// Photon density and jitter come from `source` (its photon energy and
/// `shot_to_shot_rms`, divided by sqrt(`pulses_per_exposure`) when given —
/// e.g. `wafer_throughput()["pulses_per_point"]`) unless given explicitly;
/// `absorbed_fraction` (e.g. `resist_absorbed_fraction(35, 4.8)`) counts only
/// absorbed photons. Give `source` or `photon_density_per_mj_cm2`.
#[pyfunction]
#[pyo3(signature = (
    aerial, dose_mj_cm2, threshold=0.3, source=None, photon_density_per_mj_cm2=None,
    dose_jitter_rms=None, pulses_per_exposure=None, absorbed_fraction=None,
    num_realizations=100,
))]
#[allow(clippy::too_many_arguments)]
fn compute_ler_lwr(
    py: Python<'_>,
    aerial: &PyAerialImageResult,
    dose_mj_cm2: f64,
    threshold: f64,
    source: Option<PySourceConfig>,
    photon_density_per_mj_cm2: Option<f64>,
    dose_jitter_rms: Option<f64>,
    pulses_per_exposure: Option<f64>,
    absorbed_fraction: Option<f64>,
    num_realizations: usize,
) -> PyResult<PyLerResult> {
    if !(dose_mj_cm2.is_finite() && dose_mj_cm2 > 0.0) {
        return Err(PyValueError::new_err(format!(
            "dose_mj_cm2 must be positive, got {dose_mj_cm2}"
        )));
    }
    if num_realizations < 2 {
        return Err(PyValueError::new_err(format!(
            "num_realizations must be >= 2, got {num_realizations}"
        )));
    }
    let mut params = match (&source, pulses_per_exposure) {
        (Some(s), Some(n)) => StochasticParams::from_source_multi_pulse(&s.inner, n),
        (Some(s), None) => StochasticParams::from_source(&s.inner),
        (None, _) => {
            if photon_density_per_mj_cm2.is_none() {
                return Err(PyValueError::new_err(
                    "give a source (photon energy and jitter) or photon_density_per_mj_cm2",
                ));
            }
            StochasticParams::default()
        }
    };
    if let Some(d) = photon_density_per_mj_cm2 {
        if !(d.is_finite() && d > 0.0) {
            return Err(PyValueError::new_err(format!(
                "photon_density_per_mj_cm2 must be positive, got {d}"
            )));
        }
        params.photon_density_per_mj_cm2 = d;
    }
    if let Some(j) = dose_jitter_rms {
        if !(j.is_finite() && j >= 0.0) {
            return Err(PyValueError::new_err(format!(
                "dose_jitter_rms must be >= 0, got {j}"
            )));
        }
        params.dose_jitter_rms = j;
    } else if source.is_none() {
        params.dose_jitter_rms = 0.0;
    }
    if let Some(f) = absorbed_fraction {
        if !(f > 0.0 && f <= 1.0) {
            return Err(PyValueError::new_err(format!(
                "absorbed_fraction must be in (0, 1], got {f}"
            )));
        }
        params = params.with_absorbed_fraction(f);
    }
    params.num_realizations = num_realizations;
    let pixel_nm = (aerial.x_max_nm - aerial.x_min_nm) / aerial.intensity.ncols() as f64;
    let inner = py.allow_threads(|| {
        stochastic::compute_ler_lwr(
            &aerial.intensity,
            aerial.x_min_nm,
            aerial.x_max_nm,
            dose_mj_cm2,
            pixel_nm,
            threshold,
            &params,
        )
    });
    Ok(PyLerResult {
        inner,
        photon_density_per_mj_cm2: params.photon_density_per_mj_cm2,
        dose_jitter_rms: params.dose_jitter_rms,
        num_realizations,
    })
}

/// Fraction of the incident photons a resist film absorbs (Beer–Lambert):
/// `1 − exp(−α d)` with `absorption_per_um` α in 1/µm and `thickness_nm` d.
#[pyfunction]
fn resist_absorbed_fraction(thickness_nm: f64, absorption_per_um: f64) -> f64 {
    stochastic::resist_absorbed_fraction(thickness_nm, absorption_per_um)
}

/// Mean photons delivered into a square of side `side_nm` by `dose_mj_cm2`
/// at `wavelength_nm`: `N = D a² / (h c / λ)`.
#[pyfunction]
fn photons_per_square(dose_mj_cm2: f64, wavelength_nm: f64, side_nm: f64) -> f64 {
    throughput::photons_per_square(dose_mj_cm2, wavelength_nm, side_nm)
}

/// Relative Poisson shot noise `1/sqrt(N)` of the photons in a square of
/// side `side_nm` (see `photons_per_square`).
#[pyfunction]
fn relative_shot_noise(dose_mj_cm2: f64, wavelength_nm: f64, side_nm: f64) -> f64 {
    throughput::relative_shot_noise(dose_mj_cm2, wavelength_nm, side_nm)
}

/// Register the stochastic / photon-counting functions.
pub fn register_stochastic_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyLerResult>()?;
    m.add_function(wrap_pyfunction!(compute_ler_lwr, m)?)?;
    m.add_function(wrap_pyfunction!(resist_absorbed_fraction, m)?)?;
    m.add_function(wrap_pyfunction!(photons_per_square, m)?)?;
    m.add_function(wrap_pyfunction!(relative_shot_noise, m)?)?;
    Ok(())
}
