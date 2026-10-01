//! PyO3 bindings for the vector (polarized) pupil model
//! ([`highuvlith_core::optics::vector`]).
//!
//! Exposes the `VectorSettings` configuration class (the payload of the
//! aerial engine's vector imaging model) and pure-physics helpers: the
//! per-order image-side field, whole-pupil polarization maps, the radiometric
//! (obliquity) factor, and Fresnel film-entrance coefficients. The helpers
//! evaluate the pupil model directly; they do not build an imaging engine.

use ndarray::{Array2, Array4};
use numpy::{IntoPyArray, PyArray2, PyArray4};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyComplex, PyDict};

use highuvlith_core::optics::vector::{
    columns_per_source_point, field_columns, radiometric_factor, FilmInterface,
    FresnelCoefficients, IlluminationPolarization, VectorSettings,
};
use highuvlith_core::types::Complex64;

fn parse_polarization(name: &str, angle_deg: f64) -> PyResult<IlluminationPolarization> {
    use IlluminationPolarization as P;
    match name.to_ascii_lowercase().as_str() {
        "unpolarized" => Ok(P::Unpolarized),
        "x" => Ok(P::X),
        "y" => Ok(P::Y),
        "te" | "azimuthal" => Ok(P::Te),
        "tm" | "radial" => Ok(P::Tm),
        "linear" => Ok(P::Linear { angle_deg }),
        other => Err(PyValueError::new_err(format!(
            "unknown polarization '{other}'; expected one of: unpolarized, x, y, \
             te (azimuthal), tm (radial), linear"
        ))),
    }
}

fn polarization_name(p: IlluminationPolarization) -> &'static str {
    use IlluminationPolarization as P;
    match p {
        P::Unpolarized => "unpolarized",
        P::X => "x",
        P::Y => "y",
        P::Te => "te",
        P::Tm => "tm",
        P::Linear { .. } => "linear",
    }
}

fn value_error(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// Vector (polarized) imaging settings: illumination polarization, image
/// medium index, radiometric obliquity switch, reduction ratio, and an
/// optional film entrance interface (complex index `film_n + i film_k`).
#[pyclass(name = "VectorSettings")]
#[derive(Debug, Clone)]
pub struct PyVectorSettings {
    pub inner: VectorSettings,
}

#[pymethods]
impl PyVectorSettings {
    #[new]
    #[pyo3(signature = (
        polarization="unpolarized",
        angle_deg=0.0,
        image_index=1.0,
        obliquity=true,
        reduction=4.0,
        film_n=None,
        film_k=0.0,
    ))]
    fn new(
        polarization: &str,
        angle_deg: f64,
        image_index: f64,
        obliquity: bool,
        reduction: f64,
        film_n: Option<f64>,
        film_k: f64,
    ) -> PyResult<Self> {
        if !angle_deg.is_finite() {
            return Err(value_error(format!(
                "angle_deg must be finite, got {angle_deg}"
            )));
        }
        if !(image_index.is_finite() && image_index > 0.0) {
            return Err(value_error(format!(
                "image_index must be finite and positive, got {image_index}"
            )));
        }
        if reduction.is_nan() || reduction <= 0.0 {
            return Err(value_error(format!(
                "reduction must be positive, got {reduction}"
            )));
        }
        let film = match film_n {
            None => None,
            Some(n) => {
                if !(n.is_finite() && n > 0.0) {
                    return Err(value_error(format!(
                        "film_n must be finite and positive, got {n}"
                    )));
                }
                if !(film_k.is_finite() && film_k >= 0.0) {
                    return Err(value_error(format!(
                        "film_k must be finite and non-negative (k >= 0 is loss), got {film_k}"
                    )));
                }
                Some(FilmInterface { n, k: film_k })
            }
        };
        Ok(Self {
            inner: VectorSettings {
                polarization: parse_polarization(polarization, angle_deg)?,
                image_index,
                obliquity,
                reduction,
                film,
            },
        })
    }

    /// Polarization name: unpolarized, x, y, te, tm, or linear.
    #[getter]
    fn polarization(&self) -> &'static str {
        polarization_name(self.inner.polarization)
    }

    /// Linear polarization angle from the x axis (deg); None unless linear.
    #[getter]
    fn angle_deg(&self) -> Option<f64> {
        match self.inner.polarization {
            IlluminationPolarization::Linear { angle_deg } => Some(angle_deg),
            _ => None,
        }
    }

    /// Refractive index of the homogeneous image medium.
    #[getter]
    fn image_index(&self) -> f64 {
        self.inner.image_index
    }

    /// Whether the radiometric (obliquity) factor is applied.
    #[getter]
    fn obliquity(&self) -> bool {
        self.inner.obliquity
    }

    /// Projection reduction ratio used by the obliquity factor.
    #[getter]
    fn reduction(&self) -> f64 {
        self.inner.reduction
    }

    /// Real index of the film entrance interface; None without a film.
    #[getter]
    fn film_n(&self) -> Option<f64> {
        self.inner.film.map(|f| f.n)
    }

    /// Extinction coefficient of the film; None without a film.
    #[getter]
    fn film_k(&self) -> Option<f64> {
        self.inner.film.map(|f| f.k)
    }

    /// Check the settings against the image-side NA (raises ValueError).
    fn validate(&self, na: f64) -> PyResult<()> {
        self.inner.validate(na).map_err(value_error)
    }

    /// Imaging-matrix columns one source point contributes (3 × states).
    fn columns_per_source_point(&self) -> usize {
        columns_per_source_point(&self.inner)
    }

    fn __repr__(&self) -> String {
        let pol = match self.inner.polarization {
            IlluminationPolarization::Linear { angle_deg } => format!("linear {angle_deg}°"),
            p => polarization_name(p).to_string(),
        };
        let film = match self.inner.film {
            Some(f) => format!(", film={}+{}i", f.n, f.k),
            None => String::new(),
        };
        format!(
            "VectorSettings({pol}, image_index={}, obliquity={}, reduction={}{film})",
            self.inner.image_index, self.inner.obliquity, self.inner.reduction
        )
    }
}

fn check_na(na: f64) -> PyResult<()> {
    if na.is_finite() && na > 0.0 {
        Ok(())
    } else {
        Err(value_error(format!(
            "na must be finite and positive, got {na}"
        )))
    }
}

/// Image-side field of one plane-wave order, shape `(n_states, 3)` complex
/// (`E_x, E_y, E_z` per incoherent illumination state), for a unit scalar
/// pupil. `px, py` are pupil coordinates in units of NA; `sx, sy` the source
/// point in σ (sets the TE/TM illumination basis).
#[pyfunction]
#[pyo3(signature = (settings, na, px, py, sx=0.0, sy=0.0))]
fn vector_pupil_field<'p>(
    python: Python<'p>,
    settings: &PyVectorSettings,
    na: f64,
    px: f64,
    py: f64,
    sx: f64,
    sy: f64,
) -> PyResult<Bound<'p, PyArray2<Complex64>>> {
    check_na(na)?;
    let cols = columns_per_source_point(&settings.inner);
    let mut out = vec![Complex64::new(0.0, 0.0); cols];
    field_columns(
        Complex64::new(1.0, 0.0),
        px,
        py,
        sx,
        sy,
        na,
        &settings.inner,
        &mut out,
    );
    let arr = Array2::from_shape_vec((cols / 3, 3), out).map_err(value_error)?;
    Ok(arr.into_pyarray(python))
}

/// Whole-pupil polarization map for one source point: complex array of shape
/// `(n_states, 3, n, n)` indexed `[state, component, iy, ix]`, with
/// `p_x = -1 + 2·ix/(n-1)` and `p_y = -1 + 2·iy/(n-1)` (units of NA). Points
/// outside the pupil are zero. Unit scalar pupil (no aberrations).
#[pyfunction]
#[pyo3(signature = (settings, na, n=65, sx=0.0, sy=0.0))]
fn vector_pupil_map<'p>(
    python: Python<'p>,
    settings: &PyVectorSettings,
    na: f64,
    n: usize,
    sx: f64,
    sy: f64,
) -> PyResult<Bound<'p, PyArray4<Complex64>>> {
    settings.inner.validate(na).map_err(value_error)?;
    if !(2..=2049).contains(&n) {
        return Err(value_error(format!("n must be in [2, 2049], got {n}")));
    }
    let cols = columns_per_source_point(&settings.inner);
    let states = cols / 3;
    let vs = &settings.inner;
    let map = python.allow_threads(|| {
        let mut map = Array4::from_elem((states, 3, n, n), Complex64::new(0.0, 0.0));
        let mut out = vec![Complex64::new(0.0, 0.0); cols];
        let coord = |i: usize| -1.0 + 2.0 * i as f64 / (n - 1) as f64;
        for iy in 0..n {
            for ix in 0..n {
                field_columns(
                    Complex64::new(1.0, 0.0),
                    coord(ix),
                    coord(iy),
                    sx,
                    sy,
                    na,
                    vs,
                    &mut out,
                );
                for k in 0..states {
                    for c in 0..3 {
                        map[[k, c, iy, ix]] = out[3 * k + c];
                    }
                }
            }
        }
        map
    });
    Ok(map.into_pyarray(python))
}

/// Radiometric (obliquity) amplitude factor `(cos θ_obj / cos θ_img)^(1/2)`
/// for an order at `pupil_na = NA·|p|`.
#[pyfunction]
#[pyo3(name = "radiometric_factor", signature = (pupil_na, image_index=1.0, reduction=4.0))]
fn py_radiometric_factor(pupil_na: f64, image_index: f64, reduction: f64) -> f64 {
    radiometric_factor(pupil_na, image_index, reduction)
}

/// Fresnel coefficients of one interface from a real medium `n1` into a
/// medium of complex index `n2_real + i n2_imag` at incidence `sin θ₁`.
/// Returns a dict with complex `r_s, r_p, t_s, t_p, sin_theta2, cos_theta2`
/// and real `R_s, R_p, T_s, T_p`.
#[pyfunction]
#[pyo3(signature = (n1, n2_real, n2_imag, sin_theta1))]
fn fresnel_coefficients<'p>(
    python: Python<'p>,
    n1: f64,
    n2_real: f64,
    n2_imag: f64,
    sin_theta1: f64,
) -> PyResult<Bound<'p, PyDict>> {
    if !(n1.is_finite() && n1 > 0.0) {
        return Err(value_error(format!(
            "n1 must be finite and positive, got {n1}"
        )));
    }
    if !(n2_real.is_finite() && n2_real > 0.0 && n2_imag.is_finite() && n2_imag >= 0.0) {
        return Err(value_error(format!(
            "n2 must have n2_real > 0 and n2_imag >= 0, got {n2_real} + {n2_imag}i"
        )));
    }
    if !(0.0..1.0).contains(&sin_theta1) {
        return Err(value_error(format!(
            "sin_theta1 must be in [0, 1), got {sin_theta1}"
        )));
    }
    let f = FresnelCoefficients::new(n1, Complex64::new(n2_real, n2_imag), sin_theta1);
    let d = PyDict::new(python);
    let cplx = |z: Complex64| PyComplex::from_doubles(python, z.re, z.im);
    d.set_item("r_s", cplx(f.r_s))?;
    d.set_item("r_p", cplx(f.r_p))?;
    d.set_item("t_s", cplx(f.t_s))?;
    d.set_item("t_p", cplx(f.t_p))?;
    d.set_item("sin_theta2", cplx(f.sin_theta2))?;
    d.set_item("cos_theta2", cplx(f.cos_theta2))?;
    d.set_item("R_s", f.reflectance_s())?;
    d.set_item("R_p", f.reflectance_p())?;
    d.set_item("T_s", f.transmittance_s())?;
    d.set_item("T_p", f.transmittance_p())?;
    Ok(d)
}

/// Register the vector-pupil class and helpers on the native module.
pub fn register_vector_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyVectorSettings>()?;
    m.add_function(wrap_pyfunction!(vector_pupil_field, m)?)?;
    m.add_function(wrap_pyfunction!(vector_pupil_map, m)?)?;
    m.add_function(wrap_pyfunction!(py_radiometric_factor, m)?)?;
    m.add_function(wrap_pyfunction!(fresnel_coefficients, m)?)?;
    Ok(())
}
