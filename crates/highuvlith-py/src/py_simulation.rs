use numpy::{IntoPyArray, PyArray1, PyArray3, PyReadonlyArray2};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use highuvlith_core::aerial::{
    AerialImageEngine, DecompositionMethod, DefocusModel, ImageNormalization, ImagingModel,
    ImagingSettings,
};
use highuvlith_core::mask::Mask;
use highuvlith_core::metrics;
use highuvlith_core::resist;
use highuvlith_core::source::LithographySource;
use highuvlith_core::types::Complex64;

use crate::py_config::*;
use crate::py_results::*;
use crate::py_vector::PyVectorSettings;

fn runtime_err(e: impl std::fmt::Display) -> PyErr {
    pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
}

/// Parse the `defocus_model` keyword ("exact" | "kernel_phase").
pub(crate) fn parse_defocus_model(name: &str) -> PyResult<DefocusModel> {
    match name {
        "exact" => Ok(DefocusModel::Exact),
        "kernel_phase" => Ok(DefocusModel::KernelPhase),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "defocus_model must be 'exact' or 'kernel_phase', got '{other}'"
        ))),
    }
}

/// Parse the `normalization` keyword ("clear_field" | "absolute").
pub(crate) fn parse_normalization(name: &str) -> PyResult<ImageNormalization> {
    match name {
        "clear_field" => Ok(ImageNormalization::ClearField),
        "absolute" => Ok(ImageNormalization::Absolute),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "normalization must be 'clear_field' or 'absolute', got '{other}'"
        ))),
    }
}

/// Imaging settings from the keyword arguments shared by `SimulationEngine`
/// and `BatchSimulator`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn imaging_settings(
    max_kernels: usize,
    defocus_model: &str,
    kernel_energy_fraction: f64,
    source_points_per_axis: Option<usize>,
    vector: Option<&PyVectorSettings>,
    normalization: &str,
    kernel_cache_capacity: usize,
) -> PyResult<ImagingSettings> {
    Ok(ImagingSettings {
        max_kernels,
        kernel_energy_fraction,
        source_points_per_axis,
        defocus_model: parse_defocus_model(defocus_model)?,
        imaging_model: match vector {
            Some(v) => ImagingModel::Vector(v.inner.clone()),
            None => ImagingModel::Scalar,
        },
        normalization: parse_normalization(normalization)?,
        kernel_cache_capacity,
    })
}

/// Build an imaging engine with the GIL released (the TCC decomposition of
/// the in-focus kernel set is the expensive part).
pub(crate) fn build_engine_nogil(
    py: Python<'_>,
    source: &PySourceConfig,
    optics: &PyOpticsConfig,
    grid: highuvlith_core::types::GridConfig,
    settings: ImagingSettings,
) -> PyResult<AerialImageEngine> {
    py.allow_threads(|| {
        AerialImageEngine::with_settings(&source.inner, optics.inner.as_dyn(), grid, settings)
    })
    .map_err(runtime_err)
}

/// Core simulation engine: builds the partially coherent imaging model
/// (factorized Hopkins TCC → SOCS kernels, cached per focus plane) once and
/// evaluates many conditions cheaply. Every compute method releases the GIL.
#[pyclass(name = "SimulationEngine")]
pub struct PySimulationEngine {
    engine: AerialImageEngine,
    source: PySourceConfig,
    optics: PyOpticsConfig,
    mask: PyMaskConfig,
    resist: PyResistConfig,
    grid: PyGridConfig,
}

impl PySimulationEngine {
    /// The mask to image: the override if given (after the commensurability
    /// warning), else the engine's own mask.
    fn pick_mask<'a>(&'a self, mask: Option<&'a PyMaskConfig>) -> PyResult<&'a Mask> {
        match mask {
            Some(m) => {
                crate::py_config::warn_if_incommensurate(&m.inner, &self.grid.inner)?;
                Ok(&m.inner)
            }
            None => Ok(&self.mask.inner),
        }
    }
}

#[pymethods]
impl PySimulationEngine {
    #[new]
    #[pyo3(signature = (
        source, optics, mask, resist=None, grid=None, max_kernels=30,
        defocus_model="exact", kernel_energy_fraction=1.0, source_points_per_axis=None,
        vector=None, normalization="clear_field", kernel_cache_capacity=32
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        py: Python<'_>,
        source: PySourceConfig,
        optics: PyOpticsConfig,
        mask: PyMaskConfig,
        resist: Option<PyResistConfig>,
        grid: Option<PyGridConfig>,
        max_kernels: usize,
        defocus_model: &str,
        kernel_energy_fraction: f64,
        source_points_per_axis: Option<usize>,
        vector: Option<PyVectorSettings>,
        normalization: &str,
        kernel_cache_capacity: usize,
    ) -> PyResult<Self> {
        let settings = imaging_settings(
            max_kernels,
            defocus_model,
            kernel_energy_fraction,
            source_points_per_axis,
            vector.as_ref(),
            normalization,
            kernel_cache_capacity,
        )?;
        let grid = grid.unwrap_or_else(|| PyGridConfig {
            inner: highuvlith_core::types::GridConfig::default(),
        });
        crate::py_config::warn_if_incommensurate(&mask.inner, &grid.inner)?;
        let resist = resist.unwrap_or_else(|| PyResistConfig {
            inner: highuvlith_core::resist::ResistParams::vuv_fluoropolymer(),
        });

        let engine = build_engine_nogil(py, &source, &optics, grid.inner.clone(), settings)?;

        Ok(Self {
            engine,
            source,
            optics,
            mask,
            resist,
            grid,
        })
    }

    /// Monochromatic aerial image at defocus `focus_nm` (nm). `mask`
    /// optionally images another mask with the same kernels (the TCC does not
    /// depend on the mask).
    #[pyo3(signature = (focus_nm=0.0, mask=None))]
    fn compute_aerial_image(
        &self,
        py: Python<'_>,
        focus_nm: f64,
        mask: Option<PyRef<'_, PyMaskConfig>>,
    ) -> PyResult<PyAerialImageResult> {
        let mask = self.pick_mask(mask.as_deref())?;
        let grid = py.allow_threads(|| self.engine.compute(mask, focus_nm));
        Ok(PyAerialImageResult::from_grid2d(grid))
    }

    /// Narrow-band polychromatic image: every spectral sample is imaged with
    /// the center-wavelength kernels at the focus shifted by the optics'
    /// chromatic aberration. Valid for Δλ/λ ≪ 1; use
    /// `compute_multiwavelength` for broad or multi-line spectra.
    #[pyo3(signature = (focus_nm=0.0, mask=None))]
    fn compute_polychromatic(
        &self,
        py: Python<'_>,
        focus_nm: f64,
        mask: Option<PyRef<'_, PyMaskConfig>>,
    ) -> PyResult<PyAerialImageResult> {
        let mask = self.pick_mask(mask.as_deref())?;
        let grid = py.allow_threads(|| {
            self.engine.compute_polychromatic(
                mask,
                focus_nm,
                &self.source.inner,
                self.optics.inner.as_dyn(),
            )
        });
        Ok(PyAerialImageResult::from_grid2d(grid))
    }

    /// Measure the printed CD (nm) at the given dose and focus.
    ///
    /// The feature tone follows the mask (bright-field → dark lines,
    /// dark-field → bright holes); the CD is the width of the feature nearest
    /// the field centre on the y = 0 cross-section (wrap-aware). Without
    /// `dose_to_clear_mj_cm2`, `threshold` is the intensity threshold and
    /// `dose_mj_cm2` does not enter. With it, a constant-threshold resist
    /// sets the threshold to `dose_to_clear_mj_cm2 / dose_mj_cm2` (and
    /// `threshold` is ignored).
    #[pyo3(signature = (dose_mj_cm2=30.0, focus_nm=0.0, threshold=0.3, dose_to_clear_mj_cm2=None, mask=None))]
    fn measure_cd(
        &self,
        py: Python<'_>,
        dose_mj_cm2: f64,
        focus_nm: f64,
        threshold: f64,
        dose_to_clear_mj_cm2: Option<f64>,
        mask: Option<PyRef<'_, PyMaskConfig>>,
    ) -> PyResult<f64> {
        let threshold = match dose_to_clear_mj_cm2 {
            Some(e_th) => {
                let resist = highuvlith_core::process::ThresholdResist::new(e_th)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
                if !(dose_mj_cm2.is_finite() && dose_mj_cm2 > 0.0) {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "dose_mj_cm2 must be positive, got {dose_mj_cm2}"
                    )));
                }
                resist.intensity_threshold(dose_mj_cm2)
            }
            None => threshold,
        };
        let mask = self.pick_mask(mask.as_deref())?;
        let tone = metrics::FeatureTone::of_mask(mask);
        py.allow_threads(|| {
            let aerial = self.engine.compute(mask, focus_nm);
            let x_nm = metrics::pixel_centres(aerial.nx(), aerial.x_min_nm, aerial.x_max_nm);
            let profile = metrics::centre_profile(&aerial.data);
            metrics::measure_cd_periodic(&profile, &x_nm, threshold, tone)
        })
        .ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Could not measure CD: no {} feature crosses the intensity threshold {threshold:.4}",
                crate::py_results::tone_name(tone)
            ))
        })
    }

    /// Resist profile after exposure at `dose_mj_cm2`, the 2D Gaussian
    /// post-exposure bake (`resist.peb_diffusion_nm`), and development for
    /// `dev_time_s` (Dill exposure + Mack/threshold development; 🔶
    /// simplified resist chemistry).
    #[pyo3(signature = (dose_mj_cm2=30.0, focus_nm=0.0, dev_time_s=60.0))]
    fn compute_resist_profile(
        &self,
        py: Python<'_>,
        dose_mj_cm2: f64,
        focus_nm: f64,
        dev_time_s: f64,
    ) -> PyResult<PyResistProfileResult> {
        let pixel_nm = self.grid.inner.pixel_nm;
        let params = &self.resist.inner;
        let mask = &self.mask.inner;
        let profile = py.allow_threads(|| {
            let aerial = self.engine.compute(mask, focus_nm);
            let mut latent = resist::expose(&aerial.data, dose_mj_cm2, params);
            resist::peb_diffuse(&mut latent, params.peb_diffusion_nm, pixel_nm);
            resist::develop(&latent, params, dev_time_s, pixel_nm)
        });
        Ok(PyResistProfileResult::from_profile(profile))
    }

    /// Image contrast (I_max − I_min)/(I_max + I_min) at `focus_nm`.
    #[pyo3(signature = (focus_nm=0.0, mask=None))]
    fn image_contrast(
        &self,
        py: Python<'_>,
        focus_nm: f64,
        mask: Option<PyRef<'_, PyMaskConfig>>,
    ) -> PyResult<f64> {
        let mask = self.pick_mask(mask.as_deref())?;
        Ok(py.allow_threads(|| {
            let aerial = self.engine.compute(mask, focus_nm);
            metrics::image_contrast(&aerial.data)
        }))
    }

    /// Number of SOCS kernels in the in-focus decomposition.
    fn num_kernels(&self) -> usize {
        self.engine.num_kernels()
    }

    /// Exact per-wavelength polychromatic aerial image: the imaging kernels
    /// are rebuilt at every spectral sample of the source (cutoff NA/λ_i,
    /// pupil at λ_i, chromatic focus shift) and the images summed with the
    /// spectral weights. Use this for broad or multi-line spectra (e.g. HHG
    /// harmonic combs); `compute_polychromatic` is the narrow-band
    /// approximation.
    #[pyo3(signature = (focus_nm=0.0, mask=None))]
    fn compute_multiwavelength(
        &self,
        py: Python<'_>,
        focus_nm: f64,
        mask: Option<PyRef<'_, PyMaskConfig>>,
    ) -> PyResult<PyAerialImageResult> {
        let mask = self.pick_mask(mask.as_deref())?;
        let grid = py
            .allow_threads(|| self.engine.compute_multiwavelength(mask, focus_nm))
            .map_err(runtime_err)?;
        Ok(PyAerialImageResult::from_grid2d(grid))
    }

    /// Aerial images at several focus planes (nm): one mask spectrum, planes
    /// in parallel, kernels rebuilt and cached per plane.
    #[pyo3(signature = (focus_nm, mask=None))]
    fn compute_through_focus(
        &self,
        py: Python<'_>,
        focus_nm: Vec<f64>,
        mask: Option<PyRef<'_, PyMaskConfig>>,
    ) -> PyResult<Vec<PyAerialImageResult>> {
        let mask = self.pick_mask(mask.as_deref())?;
        Ok(py
            .allow_threads(|| self.engine.compute_through_focus(mask, &focus_nm))
            .into_iter()
            .map(PyAerialImageResult::from_grid2d)
            .collect())
    }

    /// Ideal N00N-limit exposure of the mask (🧪 Theoretical; Boto et al.
    /// 2000): the same source fill, optics, mask and grid imaged at `λ/N`,
    /// mixed with classical N-photon absorption of the classical image,
    /// `E = F·I_{λ/N} + (1 − F)·I_λ^N`. An idealization (assumes the
    /// entangled states that write the pattern exist), not a simulation of
    /// state preparation. Needs `normalization="clear_field"` and a pixel no
    /// coarser than `λ/(4·N·NA)`; otherwise raises `ValueError`. Builds two
    /// imaging engines (at λ and λ/N) with the GIL released.
    #[pyo3(signature = (num_photons=2, fidelity=1.0, focus_nm=0.0, mask=None))]
    fn compute_noon_ideal_image(
        &self,
        py: Python<'_>,
        num_photons: usize,
        fidelity: f64,
        focus_nm: f64,
        mask: Option<PyRef<'_, PyMaskConfig>>,
    ) -> PyResult<crate::py_quantum::PyNoonImageResult> {
        let mask = self.pick_mask(mask.as_deref())?;
        let exposure = highuvlith_core::quantum::NoonExposure {
            num_photons,
            fidelity,
        };
        let image = py
            .allow_threads(|| {
                highuvlith_core::quantum::noon_ideal_image(
                    &self.source.inner,
                    self.optics.inner.as_dyn(),
                    self.grid.inner.clone(),
                    self.engine.settings().clone(),
                    mask,
                    focus_nm,
                    &exposure,
                )
            })
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        crate::py_quantum::noon_image_result(py, image)
    }

    /// Aerial image of an arbitrary complex amplitude transmittance
    /// `transmittance` (complex128 `(n, n)` array on the engine grid, indexed
    /// `[y, x]`), e.g. an ILT mask. The spectrum is the FFT of the samples
    /// (no sub-pixel geometry).
    #[pyo3(signature = (transmittance, focus_nm=0.0))]
    fn compute_from_transmittance(
        &self,
        py: Python<'_>,
        transmittance: PyReadonlyArray2<'_, Complex64>,
        focus_nm: f64,
    ) -> PyResult<PyAerialImageResult> {
        let n = self.grid.inner.size;
        let t = transmittance.as_array();
        if t.dim() != (n, n) {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "transmittance must be ({n}, {n}) to match the grid, got {:?}",
                t.dim()
            )));
        }
        let t = t.to_owned();
        let grid = py.allow_threads(|| self.engine.compute_from_transmittance(&t, focus_nm));
        Ok(PyAerialImageResult::from_grid2d(grid))
    }

    /// Absolute clear-field intensity TCC(0,0) at `focus_nm` (relative to the
    /// illumination incident on the mask). Images are divided by it unless
    /// `normalization="absolute"`; multiply to recover absolute intensity.
    #[pyo3(signature = (focus_nm=0.0))]
    fn clear_field_intensity(&self, py: Python<'_>, focus_nm: f64) -> f64 {
        py.allow_threads(|| self.engine.clear_field_intensity(focus_nm))
    }

    /// Imaging-engine bookkeeping for the kernel set at `focus_nm` (center
    /// wavelength): num_kernels, captured_energy_fraction, num_frequencies,
    /// num_columns, method ('dense_tcc' | 'dense_gram' | 'randomized'),
    /// support_exceeds_nyquist, clear_field_intensity, normalized,
    /// imaging_model ('scalar' | 'vector'), num_source_points, defocus_model,
    /// normalization ('clear_field' | 'absolute').
    #[pyo3(signature = (focus_nm=0.0))]
    fn imaging_diagnostics<'py>(
        &self,
        py: Python<'py>,
        focus_nm: f64,
    ) -> PyResult<Bound<'py, PyDict>> {
        let wavelength = self.engine.wavelength_nm();
        let d = py
            .allow_threads(|| self.engine.kernel_diagnostics(focus_nm, wavelength))
            .map_err(runtime_err)?;
        let dict = PyDict::new(py);
        dict.set_item("num_kernels", d.num_kernels)?;
        dict.set_item("captured_energy_fraction", d.captured_energy_fraction)?;
        dict.set_item("num_frequencies", d.num_frequencies)?;
        dict.set_item("num_columns", d.num_columns)?;
        dict.set_item(
            "method",
            match d.method {
                DecompositionMethod::DenseTcc => "dense_tcc",
                DecompositionMethod::DenseGram => "dense_gram",
                DecompositionMethod::Randomized => "randomized",
            },
        )?;
        dict.set_item("support_exceeds_nyquist", d.support_exceeds_nyquist)?;
        dict.set_item("clear_field_intensity", d.clear_field_intensity)?;
        dict.set_item("normalized", d.normalized)?;
        dict.set_item(
            "imaging_model",
            match self.engine.settings().imaging_model {
                ImagingModel::Scalar => "scalar",
                ImagingModel::Vector(_) => "vector",
            },
        )?;
        dict.set_item("num_source_points", self.engine.source_points().len())?;
        dict.set_item(
            "defocus_model",
            match self.engine.settings().defocus_model {
                DefocusModel::Exact => "exact",
                DefocusModel::KernelPhase => "kernel_phase",
            },
        )?;
        dict.set_item(
            "normalization",
            match self.engine.settings().normalization {
                ImageNormalization::ClearField => "clear_field",
                ImageNormalization::Absolute => "absolute",
            },
        )?;
        Ok(dict)
    }

    /// The engine's discrete source sampling: `(sx, sy, weight)` numpy
    /// arrays, pupil-fill coordinates in σ units (NA/λ) and normalized
    /// incoherent weights (sum 1).
    #[allow(clippy::type_complexity)]
    fn source_points<'py>(
        &self,
        py: Python<'py>,
    ) -> (
        Bound<'py, PyArray1<f64>>,
        Bound<'py, PyArray1<f64>>,
        Bound<'py, PyArray1<f64>>,
    ) {
        let pts = self.engine.source_points();
        let sx: Vec<f64> = pts.iter().map(|p| p.sx).collect();
        let sy: Vec<f64> = pts.iter().map(|p| p.sy).collect();
        let w: Vec<f64> = pts.iter().map(|p| p.weight).collect();
        (sx.into_pyarray(py), sy.into_pyarray(py), w.into_pyarray(py))
    }

    /// The source's spectral samples `[(wavelength_nm, weight), ...]` used by
    /// the polychromatic and per-wavelength images.
    fn spectral_samples(&self) -> Vec<(f64, f64)> {
        self.engine.spectral_samples().to_vec()
    }

    /// SOCS eigenvalues (decreasing) of the kernel set at `focus_nm`; they
    /// carry the image normalization (divided by the clear field by default).
    #[pyo3(signature = (focus_nm=0.0))]
    fn kernel_eigenvalues<'py>(&self, py: Python<'py>, focus_nm: f64) -> Bound<'py, PyArray1<f64>> {
        let set = py.allow_threads(|| self.engine.kernels(focus_nm));
        set.eigenvalues.clone().into_pyarray(py)
    }

    /// The SOCS kernel set at `focus_nm`: `(eigenvalues, kernels)` with
    /// `kernels` complex `(k, n, n)` in numpy.fft.fft2 frequency layout, unit
    /// 2-norm each; `I = Σ_k λ_k |ifft2(K_k · spectrum)|²` before flare.
    #[pyo3(signature = (focus_nm=0.0))]
    #[allow(clippy::type_complexity)]
    fn kernels<'py>(
        &self,
        py: Python<'py>,
        focus_nm: f64,
    ) -> PyResult<(Bound<'py, PyArray1<f64>>, Bound<'py, PyArray3<Complex64>>)> {
        let set = py.allow_threads(|| self.engine.kernels(focus_nm));
        let n = self.grid.inner.size;
        let k = set.kernels.len();
        let mut stacked = ndarray::Array3::<Complex64>::zeros((k, n, n));
        for (i, kernel) in set.kernels.iter().enumerate() {
            stacked.index_axis_mut(ndarray::Axis(0), i).assign(kernel);
        }
        Ok((
            set.eigenvalues.clone().into_pyarray(py),
            stacked.into_pyarray(py),
        ))
    }

    /// Captured TCC energy fraction Σλ_k/tr(TCC) of the in-focus kernel set.
    fn captured_energy_fraction(&self) -> f64 {
        self.engine.captured_energy_fraction()
    }

    /// Center wavelength (nm) the engine images at.
    #[getter]
    fn wavelength_nm(&self) -> f64 {
        self.engine.wavelength_nm()
    }

    /// Image-side numerical aperture of the optics.
    #[getter]
    fn numerical_aperture(&self) -> f64 {
        self.engine.na()
    }

    #[getter]
    fn source(&self) -> PySourceConfig {
        self.source.clone()
    }

    #[getter]
    fn optics(&self) -> PyOpticsConfig {
        self.optics.clone()
    }

    #[getter]
    fn mask(&self) -> PyMaskConfig {
        self.mask.clone()
    }

    #[getter]
    fn resist(&self) -> PyResistConfig {
        self.resist.clone()
    }

    #[getter]
    fn grid(&self) -> PyGridConfig {
        self.grid.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "SimulationEngine(λ={}nm, NA={}, kernels={})",
            self.source.inner.wavelength_nm(),
            self.optics.inner.as_dyn().na(),
            self.engine.num_kernels()
        )
    }
}
