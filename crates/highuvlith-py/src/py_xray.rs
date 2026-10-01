//! PyO3 bindings for the X-ray / EUV materials and LIGA diffraction tools
//! (WP-E1): the shared LIGA configuration builder used by `simulate_liga`,
//! the fine 1D Fresnel edge profile ([`deep_xray::fresnel_edge_profile_1d`]),
//! periodic multilayer mirrors ([`multilayer::MultilayerMirror`]), and
//! Henke/CXRO optical constants ([`henke::Material`]).

use numpy::{IntoPyArray, PyArray1, PyArray2, ToPyArray};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyComplex, PyDict};

use highuvlith_core::deep_xray::{
    self, BeamFilter, BendingMagnetBeamline, DeepXrayConfig, EdgeProfile, ProximityModel,
    XraySpectrum,
};
use highuvlith_core::materials::attenuation::Compound;
use highuvlith_core::materials::henke;
use highuvlith_core::materials::multilayer::{self, MultilayerMirror};
use highuvlith_core::source::SourceKind;
use highuvlith_core::types::Polarization;

use crate::py_config::PySourceConfig;

/// A pair of 1D numpy arrays.
type ArrayPair<'py> = (Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>);

fn value_err(e: impl ToString) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// Parse a polarization tag: `"te"`/`"s"`, `"tm"`/`"p"`, `"unpolarized"`.
pub(crate) fn parse_polarization(tag: &str) -> PyResult<Polarization> {
    match tag.to_ascii_lowercase().as_str() {
        "te" | "s" => Ok(Polarization::TE),
        "tm" | "p" => Ok(Polarization::TM),
        "unpolarized" | "unpolarised" | "u" => Ok(Polarization::Unpolarized),
        other => Err(PyValueError::new_err(format!(
            "unknown polarization '{other}' (expected 'te'/'s', 'tm'/'p' or 'unpolarized')"
        ))),
    }
}

/// Parse a proximity-model tag: `"fresnel"` or `"gaussian"`.
pub(crate) fn parse_diffraction(tag: &str) -> PyResult<ProximityModel> {
    match tag.to_ascii_lowercase().as_str() {
        "fresnel" => Ok(ProximityModel::Fresnel),
        "gaussian" => Ok(ProximityModel::Gaussian),
        other => Err(PyValueError::new_err(format!(
            "unknown diffraction model '{other}' (expected 'fresnel' or 'gaussian')"
        ))),
    }
}

/// Everything that selects the LIGA exposure spectrum from Python arguments.
pub(crate) struct LigaSpectrumArgs<'a> {
    pub critical_energy_kev: f64,
    pub source: Option<&'a PySourceConfig>,
    pub source_distance_m: Option<f64>,
    pub horizontal_acceptance_mrad: f64,
    pub vertical_scan_mm: Option<f64>,
    pub flux_density: Option<Vec<(f64, f64)>>,
    pub spectrum_table: Option<Vec<(f64, f64)>>,
}

/// Energy bins of the X-ray tube / betatron spectra handed to the LIGA model
/// (the CLI `deep` command uses the same resolution).
const LAB_SOURCE_SPECTRUM_BINS: usize = 200;

impl LigaSpectrumArgs<'_> {
    /// Priority: absolute flux-density table, then a relative table, then a
    /// `source`, then `critical_energy_kev`. Sources: a synchrotron bending
    /// magnet (absolute when both `source_distance_m` and `vertical_scan_mm`
    /// are given), or an X-ray tube / laser-wakefield betatron (absolute
    /// photons s⁻¹ mm⁻² keV⁻¹ at the mask when `source_distance_m` is given —
    /// the source's `spectral_flux_density` through
    /// `XraySpectrum::from_flux_density` — else its relative spectrum).
    /// Returns the spectrum and a label naming the path taken.
    pub(crate) fn build(&self) -> PyResult<(XraySpectrum, &'static str)> {
        if let Some(table) = &self.flux_density {
            return Ok((
                XraySpectrum::from_flux_density(table).map_err(value_err)?,
                "flux_density",
            ));
        }
        if let Some(table) = &self.spectrum_table {
            if table.is_empty() {
                return Err(PyValueError::new_err("spectrum_table must not be empty"));
            }
            return Ok((
                XraySpectrum::Tabulated {
                    energies_kev: table.iter().map(|(e, _)| *e).collect(),
                    relative_flux: table.iter().map(|(_, w)| *w).collect(),
                },
                "tabulated",
            ));
        }
        if let Some(src) = self.source {
            let lab_source = |flux: &dyn Fn(f64) -> Vec<(f64, f64)>,
                              relative: XraySpectrum,
                              label: &'static str|
             -> PyResult<(XraySpectrum, &'static str)> {
                if self.vertical_scan_mm.is_some() {
                    return Err(PyValueError::new_err(format!(
                        "vertical_scan_mm applies to bending-magnet beamlines only, not to a {label} source"
                    )));
                }
                match self.source_distance_m {
                    Some(l) => {
                        if !(l.is_finite() && l > 0.0) {
                            return Err(PyValueError::new_err(format!(
                                "source_distance_m must be positive, got {l}"
                            )));
                        }
                        let table = flux(l * 1e3);
                        Ok((
                            XraySpectrum::from_flux_density(&table).map_err(value_err)?,
                            label,
                        ))
                    }
                    None => Ok((relative, label)),
                }
            };
            return match &src.inner {
                SourceKind::Synchrotron(sync) => {
                    match (self.source_distance_m, self.vertical_scan_mm) {
                        (Some(l), Some(h)) => {
                            let b = BendingMagnetBeamline::from_synchrotron(
                                sync,
                                l,
                                self.horizontal_acceptance_mrad,
                                h,
                            )
                            .ok_or_else(|| {
                                PyValueError::new_err(
                                    "undulator beamlines have no white-beam spectrum",
                                )
                            })?;
                            b.validate().map_err(value_err)?;
                            Ok((
                                XraySpectrum::BendingMagnetBeamline(b),
                                "bending_magnet_beamline",
                            ))
                        }
                        (None, None) => XraySpectrum::from_synchrotron(sync)
                            .map(|sp| (sp, "bending_magnet"))
                            .ok_or_else(|| {
                                PyValueError::new_err(
                                    "undulator beamlines have no white-beam spectrum",
                                )
                            }),
                        _ => Err(PyValueError::new_err(
                            "absolute bending-magnet exposure needs both source_distance_m and vertical_scan_mm",
                        )),
                    }
                }
                SourceKind::XrayTube(tube) => lab_source(
                    &|d_mm| tube.spectral_flux_density(d_mm, LAB_SOURCE_SPECTRUM_BINS),
                    tube.xray_spectrum(LAB_SOURCE_SPECTRUM_BINS),
                    "xray_tube",
                ),
                SourceKind::Betatron(b) => lab_source(
                    &|d_mm| b.spectral_flux_density(d_mm, LAB_SOURCE_SPECTRUM_BINS),
                    b.xray_spectrum(),
                    "betatron",
                ),
                _ => Err(PyValueError::new_err(
                    "LIGA needs a broadband X-ray source: a synchrotron bending magnet \
                     (SourceConfig.synchrotron_liga_bending_magnet()), an X-ray tube \
                     (SourceConfig.xray_tube()) or a betatron (SourceConfig.betatron())",
                )),
            };
        }
        if self.critical_energy_kev.is_nan() || self.critical_energy_kev <= 0.0 {
            return Err(PyValueError::new_err(
                "critical_energy_kev must be positive",
            ));
        }
        Ok((
            XraySpectrum::BendingMagnet {
                critical_energy_kev: self.critical_energy_kev,
            },
            "bending_magnet",
        ))
    }
}

/// Everything else that defines the LIGA stack and model from Python.
pub(crate) struct LigaStackArgs {
    pub resist_thickness_um: f64,
    pub proximity_gap_um: f64,
    pub energy_bins: usize,
    pub photoelectron_blur: bool,
    pub filters: Option<Vec<(String, f64)>>,
    pub absorber: String,
    pub absorber_thickness_um: f64,
    pub membrane: String,
    pub membrane_thickness_um: f64,
    pub target_bottom_dose_kj_cm3: f64,
    pub damage_dose_kj_cm3: f64,
    pub diffraction: String,
}

/// Build a [`DeepXrayConfig`] (thick PMMA) from the Python arguments, plus
/// the label of the spectrum path taken.
pub(crate) fn build_liga_config(
    spectrum: &LigaSpectrumArgs<'_>,
    stack: &LigaStackArgs,
) -> PyResult<(DeepXrayConfig, &'static str)> {
    let (spectrum, label) = spectrum.build()?;
    let mut config = DeepXrayConfig::pmma_default(spectrum);
    config.resist_thickness_um = stack.resist_thickness_um;
    config.proximity_gap_um = stack.proximity_gap_um;
    config.energy_bins = stack.energy_bins.max(1);
    config.photoelectron_blur = stack.photoelectron_blur;
    config.absorber = BeamFilter::new(
        Compound::from_spec(&stack.absorber).map_err(value_err)?,
        stack.absorber_thickness_um,
    );
    config.membrane = BeamFilter::new(
        Compound::from_spec(&stack.membrane).map_err(value_err)?,
        stack.membrane_thickness_um,
    );
    if let Some(filters) = &stack.filters {
        for (spec, t) in filters {
            if t.is_nan() || *t < 0.0 {
                return Err(PyValueError::new_err(format!(
                    "filter thickness must be >= 0, got {t} for {spec}"
                )));
            }
            config.filters.push(BeamFilter::new(
                Compound::from_spec(spec).map_err(value_err)?,
                *t,
            ));
        }
    }
    config.target_bottom_dose_kj_cm3 = stack.target_bottom_dose_kj_cm3;
    config.damage_dose_kj_cm3 = stack.damage_dose_kj_cm3;
    config.proximity_model = parse_diffraction(&stack.diffraction)?;
    Ok((config, label))
}

/// Dose across a single straight absorber edge (analytic Fresnel solution),
/// for LIGA sidewall analysis at fine sampling.
#[pyclass(name = "LigaEdgeProfile")]
pub struct PyLigaEdgeProfile {
    inner: EdgeProfile,
}

#[pymethods]
impl PyLigaEdgeProfile {
    /// Lateral positions in nm relative to the geometric edge (`x > 0` open).
    #[getter]
    fn x_nm<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        self.inner.x_nm.to_pyarray(py)
    }

    /// Depths in um below the resist surface.
    #[getter]
    fn z_um<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        self.inner.z_um.to_pyarray(py)
    }

    /// Absolute dose in kJ/cm^3 as a `(n_depths, n_x)` array.
    #[getter]
    fn dose_kj_cm3<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyArray2<f64>>> {
        let rows = self.inner.dose_kj_cm3.len();
        let cols = self.inner.x_nm.len();
        let flat: Vec<f64> = self.inner.dose_kj_cm3.iter().flatten().copied().collect();
        let arr = ndarray::Array2::from_shape_vec((rows, cols), flat)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(arr.into_pyarray(py))
    }

    /// Open-field (`x -> +inf`) dose at each depth in kJ/cm^3.
    #[getter]
    fn open_dose_kj_cm3<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        self.inner.open_dose_kj_cm3.to_pyarray(py)
    }

    /// Absorber-side (`x -> -inf`) dose at each depth in kJ/cm^3.
    #[getter]
    fn absorber_dose_kj_cm3<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        self.inner.absorber_dose_kj_cm3.to_pyarray(py)
    }

    /// Developed-edge position (nm) at each depth for a positive-tone
    /// threshold dose; `None` where the depth is not developed.
    fn edge_positions_nm(&self, threshold_kj_cm3: f64) -> Vec<Option<f64>> {
        self.inner.edge_positions_nm(threshold_kj_cm3)
    }

    /// Sidewall angle from vertical in degrees (least-squares fit of the edge
    /// positions over depth); positive = the opening narrows with depth.
    fn sidewall_angle_deg(&self, threshold_kj_cm3: f64) -> Option<f64> {
        self.inner.sidewall_angle_deg(threshold_kj_cm3)
    }

    /// Maximum `|dD/dx|` (kJ cm^-3 per nm) at each depth.
    fn max_gradient_kj_cm3_nm(&self) -> Vec<f64> {
        self.inner.max_gradient_kj_cm3_nm()
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
            "analytic paraxial Fresnel knife edge per energy bin, summed with the depth-dose weights",
            "thin-screen absorber; no photo- or secondary-electron transport",
        ]
    }

    fn __repr__(&self) -> String {
        format!(
            "LigaEdgeProfile(n_x={}, depths={:?} um)",
            self.inner.x_nm.len(),
            self.inner.z_um
        )
    }
}

/// Fine 1D Fresnel dose profile across a single straight absorber edge at
/// several depths (analytic knife-edge solution per energy bin, incoherently
/// summed with the depth-dose weights; the open side at the resist bottom
/// reaches the target dose). `depths_um=None` samples 5 depths from the top
/// to the bottom of the resist.
#[pyfunction]
#[pyo3(signature = (
    critical_energy_kev=6.23,
    resist_thickness_um=500.0,
    proximity_gap_um=100.0,
    x_min_nm=-3000.0,
    x_max_nm=3000.0,
    dx_nm=10.0,
    depths_um=None,
    energy_bins=100,
    photoelectron_blur=false,
    filters=None,
    absorber="Au",
    absorber_thickness_um=20.0,
    membrane="Ti",
    membrane_thickness_um=2.0,
    target_bottom_dose_kj_cm3=3.0,
    source=None,
    spectrum_table=None,
    flux_density=None,
))]
#[allow(clippy::too_many_arguments)]
fn liga_edge_profile(
    py: Python<'_>,
    critical_energy_kev: f64,
    resist_thickness_um: f64,
    proximity_gap_um: f64,
    x_min_nm: f64,
    x_max_nm: f64,
    dx_nm: f64,
    depths_um: Option<Vec<f64>>,
    energy_bins: usize,
    photoelectron_blur: bool,
    filters: Option<Vec<(String, f64)>>,
    absorber: &str,
    absorber_thickness_um: f64,
    membrane: &str,
    membrane_thickness_um: f64,
    target_bottom_dose_kj_cm3: f64,
    source: Option<PyRef<'_, PySourceConfig>>,
    spectrum_table: Option<Vec<(f64, f64)>>,
    flux_density: Option<Vec<(f64, f64)>>,
) -> PyResult<PyLigaEdgeProfile> {
    let spectrum = LigaSpectrumArgs {
        critical_energy_kev,
        source: source.as_deref(),
        source_distance_m: None,
        horizontal_acceptance_mrad: 5.0,
        vertical_scan_mm: None,
        flux_density,
        spectrum_table,
    };
    let stack = LigaStackArgs {
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
        damage_dose_kj_cm3: 20.0,
        diffraction: "fresnel".to_string(),
    };
    let (config, _) = build_liga_config(&spectrum, &stack)?;
    let depths = depths_um.unwrap_or_else(|| {
        (0..5)
            .map(|k| k as f64 / 4.0 * resist_thickness_um)
            .collect()
    });
    let profile = py
        .allow_threads(|| {
            deep_xray::fresnel_edge_profile_1d(&config, x_min_nm, x_max_nm, dx_nm, &depths)
        })
        .map_err(value_err)?;
    Ok(PyLigaEdgeProfile { inner: profile })
}

/// Periodic EUV/BEUV multilayer mirror (Parratt recursion with Nevot-Croce
/// roughness, Henke optical constants). Angles are in degrees from the
/// surface normal; polarization is `'te'`/`'s'`, `'tm'`/`'p'` or
/// `'unpolarized'`.
#[pyclass(name = "MultilayerMirror")]
#[derive(Clone)]
pub struct PyMultilayerMirror {
    inner: MultilayerMirror,
}

#[pymethods]
impl PyMultilayerMirror {
    /// Mo/Si for 13.5 nm: `periods` x (Si on Mo), Mo fraction `gamma`, on
    /// SiO2, CXRO bulk densities.
    #[staticmethod]
    #[pyo3(signature = (periods=40, period_nm=6.9, gamma=0.4))]
    fn mo_si(periods: usize, period_nm: f64, gamma: f64) -> PyResult<Self> {
        Ok(Self {
            inner: MultilayerMirror::mo_si(periods, period_nm, gamma).map_err(value_err)?,
        })
    }

    /// La/B4C for 6.x nm: `periods` x (B4C on La), La fraction `gamma`, on
    /// Si, CXRO bulk densities (`period_nm=3.37` puts the peak near 6.7 nm).
    #[staticmethod]
    #[pyo3(signature = (periods=200, period_nm=3.37, gamma=0.4))]
    fn la_b4c(periods: usize, period_nm: f64, gamma: f64) -> PyResult<Self> {
        Ok(Self {
            inner: MultilayerMirror::la_b4c(periods, period_nm, gamma).map_err(value_err)?,
        })
    }

    /// La/B for 6.6-6.7 nm: `periods` x (B on La), La fraction `gamma`, on
    /// Si, CXRO bulk densities (`period_nm=3.33` puts the peak near 6.65 nm).
    #[staticmethod]
    #[pyo3(signature = (periods=200, period_nm=3.33, gamma=0.4))]
    fn la_b(periods: usize, period_nm: f64, gamma: f64) -> PyResult<Self> {
        Ok(Self {
            inner: MultilayerMirror::la_b(periods, period_nm, gamma).map_err(value_err)?,
        })
    }

    /// A copy with a capping layer of `formula` (density `density_g_cm3`, or
    /// the CXRO default) on top, e.g. `with_capping("Ru", 2.0)`.
    #[pyo3(signature = (formula, thickness_nm, density_g_cm3=None))]
    fn with_capping(
        &self,
        formula: &str,
        thickness_nm: f64,
        density_g_cm3: Option<f64>,
    ) -> PyResult<Self> {
        let material = match density_g_cm3 {
            Some(rho) => henke::Material::new(formula, rho),
            None => henke::Material::with_default_density(formula),
        }
        .map_err(value_err)?;
        Ok(Self {
            inner: self.inner.clone().with_capping(material, thickness_nm),
        })
    }

    /// A copy with RMS interface roughness `sigma_nm` on every interface.
    fn with_roughness(&self, sigma_nm: f64) -> Self {
        Self {
            inner: self.inner.clone().with_roughness(sigma_nm),
        }
    }

    /// Period thickness in nm.
    #[getter]
    fn period_nm(&self) -> f64 {
        self.inner.period_nm()
    }

    /// Number of periods.
    #[getter]
    fn periods(&self) -> usize {
        self.inner.periods
    }

    /// Reflectance at one wavelength (nm) and angle (deg from normal).
    #[pyo3(signature = (wavelength_nm, angle_deg=0.0, polarization="unpolarized"))]
    fn reflectance(&self, wavelength_nm: f64, angle_deg: f64, polarization: &str) -> PyResult<f64> {
        self.inner
            .reflectance(
                wavelength_nm,
                angle_deg.to_radians(),
                parse_polarization(polarization)?,
            )
            .map_err(value_err)
    }

    /// Reflectance for a sequence of wavelengths (nm) as a numpy array.
    #[pyo3(signature = (wavelengths_nm, angle_deg=0.0, polarization="unpolarized"))]
    fn reflectance_curve<'py>(
        &self,
        py: Python<'py>,
        wavelengths_nm: Vec<f64>,
        angle_deg: f64,
        polarization: &str,
    ) -> PyResult<Bound<'py, PyArray1<f64>>> {
        let pol = parse_polarization(polarization)?;
        let th = angle_deg.to_radians();
        let r = py
            .allow_threads(|| {
                wavelengths_nm
                    .iter()
                    .map(|l| self.inner.reflectance(*l, th, pol))
                    .collect::<highuvlith_core::error::Result<Vec<f64>>>()
            })
            .map_err(value_err)?;
        Ok(r.into_pyarray(py))
    }

    /// Reflectance versus incidence angle (deg) at one wavelength.
    #[pyo3(signature = (wavelength_nm, angles_deg, polarization="unpolarized"))]
    fn reflectance_vs_angle<'py>(
        &self,
        py: Python<'py>,
        wavelength_nm: f64,
        angles_deg: Vec<f64>,
        polarization: &str,
    ) -> PyResult<Bound<'py, PyArray1<f64>>> {
        let pol = parse_polarization(polarization)?;
        let r = py
            .allow_threads(|| {
                angles_deg
                    .iter()
                    .map(|a| self.inner.reflectance(wavelength_nm, a.to_radians(), pol))
                    .collect::<highuvlith_core::error::Result<Vec<f64>>>()
            })
            .map_err(value_err)?;
        Ok(r.into_pyarray(py))
    }

    /// Complex amplitude reflection coefficient (`'te'` or `'tm'`; physics
    /// convention `exp(i(kz - wt))`, top-surface phase reference).
    #[pyo3(signature = (wavelength_nm, angle_deg=0.0, polarization="te"))]
    fn amplitude<'py>(
        &self,
        py: Python<'py>,
        wavelength_nm: f64,
        angle_deg: f64,
        polarization: &str,
    ) -> PyResult<Bound<'py, PyComplex>> {
        let r = self
            .inner
            .amplitude(
                wavelength_nm,
                angle_deg.to_radians(),
                parse_polarization(polarization)?,
            )
            .map_err(value_err)?;
        Ok(PyComplex::from_doubles(py, r.re, r.im))
    }

    /// `(peak_wavelength_nm, peak_reflectance)` within `[lambda_min_nm,
    /// lambda_max_nm]`.
    #[pyo3(signature = (lambda_min_nm, lambda_max_nm, angle_deg=0.0, polarization="unpolarized"))]
    fn peak(
        &self,
        py: Python<'_>,
        lambda_min_nm: f64,
        lambda_max_nm: f64,
        angle_deg: f64,
        polarization: &str,
    ) -> PyResult<(f64, f64)> {
        let pol = parse_polarization(polarization)?;
        py.allow_threads(|| {
            self.inner
                .peak(angle_deg.to_radians(), pol, lambda_min_nm, lambda_max_nm)
        })
        .map_err(value_err)
    }

    /// FWHM (nm) of the reflectance peak within `[lambda_min_nm,
    /// lambda_max_nm]`.
    #[pyo3(signature = (lambda_min_nm, lambda_max_nm, angle_deg=0.0, polarization="unpolarized"))]
    fn bandwidth_fwhm_nm(
        &self,
        py: Python<'_>,
        lambda_min_nm: f64,
        lambda_max_nm: f64,
        angle_deg: f64,
        polarization: &str,
    ) -> PyResult<f64> {
        let pol = parse_polarization(polarization)?;
        py.allow_threads(|| {
            self.inner
                .bandwidth_fwhm_nm(angle_deg.to_radians(), pol, lambda_min_nm, lambda_max_nm)
        })
        .map_err(value_err)
    }

    /// Angular acceptance: FWHM (deg) of reflectance versus incidence angle
    /// at `wavelength_nm` (symmetric full width when the maximum is at
    /// normal incidence).
    #[pyo3(signature = (wavelength_nm, polarization="unpolarized"))]
    fn angular_acceptance_fwhm_deg(
        &self,
        py: Python<'_>,
        wavelength_nm: f64,
        polarization: &str,
    ) -> PyResult<f64> {
        let pol = parse_polarization(polarization)?;
        py.allow_threads(|| self.inner.angular_acceptance_fwhm_deg(wavelength_nm, pol))
            .map_err(value_err)
    }

    /// s/p amplitudes, unpolarized reflectance and phases at one wavelength
    /// and incidence angle - what a reflective objective samples per ray.
    fn pupil_response<'py>(
        &self,
        py: Python<'py>,
        wavelength_nm: f64,
        angle_deg: f64,
    ) -> PyResult<Bound<'py, PyDict>> {
        let r = self
            .inner
            .pupil_response(wavelength_nm, angle_deg.to_radians())
            .map_err(value_err)?;
        let d = PyDict::new(py);
        d.set_item("rs", PyComplex::from_doubles(py, r.rs.re, r.rs.im))?;
        d.set_item("rp", PyComplex::from_doubles(py, r.rp.re, r.rp.im))?;
        d.set_item("reflectance_unpolarized", r.reflectance_unpolarized)?;
        d.set_item("phase_s_rad", r.phase_s_rad)?;
        d.set_item("phase_p_rad", r.phase_p_rad)?;
        Ok(d)
    }

    fn __repr__(&self) -> String {
        format!(
            "MultilayerMirror(periods={}, period_nm={:.4}, layers={:?}, capping={}, roughness_nm={})",
            self.inner.periods,
            self.inner.period_nm(),
            self.inner
                .period
                .iter()
                .map(|l| l.index.label())
                .collect::<Vec<_>>(),
            self.inner.capping.len(),
            self.inner.roughness_nm
        )
    }
}

/// Tune the period (nm) of a `"mo_si"`, `"la_b4c"` or `"la_b"` mirror so its
/// reflectance peak sits at `target_nm`.
#[pyfunction]
#[pyo3(signature = (family, target_nm, periods, gamma=0.4, angle_deg=0.0, polarization="te"))]
fn tune_multilayer_period(
    py: Python<'_>,
    family: &str,
    target_nm: f64,
    periods: usize,
    gamma: f64,
    angle_deg: f64,
    polarization: &str,
) -> PyResult<f64> {
    let pol = parse_polarization(polarization)?;
    let th = angle_deg.to_radians();
    type Build = fn(usize, f64, f64) -> highuvlith_core::error::Result<MultilayerMirror>;
    // (constructor, mean δ for the Bragg start value)
    let (build, delta_mean): (Build, f64) = match family.to_ascii_lowercase().as_str() {
        "mo_si" | "mosi" => (MultilayerMirror::mo_si, 0.031),
        "la_b4c" | "lab4c" => (MultilayerMirror::la_b4c, 0.007),
        "la_b" | "lab" => (MultilayerMirror::la_b, 0.004),
        other => {
            return Err(PyValueError::new_err(format!(
                "unknown multilayer family '{other}' (expected 'mo_si', 'la_b4c' or 'la_b')"
            )))
        }
    };
    py.allow_threads(|| {
        multilayer::tune_period_nm(
            |d| build(periods, d, gamma),
            target_nm,
            th,
            pol,
            multilayer::bragg_period_nm(target_nm, th, delta_mean, 1),
        )
    })
    .map_err(value_err)
}

/// Henke/CXRO optical constants `(delta, beta)` of `formula` at the given
/// wavelengths (nm, 0.0413-41.3), at `density_g_cm3` or the CXRO default
/// density; `n = 1 - delta + i beta`.
#[pyfunction]
#[pyo3(signature = (formula, wavelengths_nm, density_g_cm3=None))]
fn xray_optical_constants<'py>(
    py: Python<'py>,
    formula: &str,
    wavelengths_nm: Vec<f64>,
    density_g_cm3: Option<f64>,
) -> PyResult<ArrayPair<'py>> {
    let m = match density_g_cm3 {
        Some(rho) => henke::Material::new(formula, rho),
        None => henke::Material::with_default_density(formula),
    }
    .map_err(value_err)?;
    let mut delta = Vec::with_capacity(wavelengths_nm.len());
    let mut beta = Vec::with_capacity(wavelengths_nm.len());
    for l in &wavelengths_nm {
        let (d, b) = m.delta_beta(*l).map_err(value_err)?;
        delta.push(d);
        beta.push(b);
    }
    Ok((delta.into_pyarray(py), beta.into_pyarray(py)))
}

/// Complex refractive index `n + ik` of a named material at `wavelength_nm`
/// from the materials database: VUV/DUV Sellmeier and tabulated entries
/// (e.g. "CaF2", "SiO2", "Cr"), Henke-computed EUV/BEUV/X-ray entries
/// (0.0413–41.3 nm, e.g. "Mo_euv", "Ta_euv", "MOx_resist"), or any formula as
/// `"henke:<formula>[@density]"`. See `material_names()`. Lookups never
/// extrapolate: outside an entry's data range "Si", "Cr" and "SiO2" fall back
/// to their Henke values (0.0413–41.3 nm); anything else raises ValueError
/// naming the range and the alternative key.
#[pyfunction]
fn refractive_index<'py>(
    py: Python<'py>,
    material: &str,
    wavelength_nm: f64,
) -> PyResult<Bound<'py, PyComplex>> {
    let db = highuvlith_core::materials::database::MaterialsDatabase::new();
    let n = db
        .refractive_index(material, wavelength_nm)
        .map_err(value_err)?;
    Ok(PyComplex::from_doubles(py, n.re, n.im))
}

/// Names of the built-in materials (sorted); `"henke:<formula>[@density]"`
/// lookups work in addition.
#[pyfunction]
fn material_names() -> Vec<String> {
    highuvlith_core::materials::database::MaterialsDatabase::new().material_names()
}

/// Register the X-ray materials / LIGA-diffraction classes and functions.
pub fn register_xray_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyLigaEdgeProfile>()?;
    m.add_class::<PyMultilayerMirror>()?;
    m.add_function(wrap_pyfunction!(liga_edge_profile, m)?)?;
    m.add_function(wrap_pyfunction!(tune_multilayer_period, m)?)?;
    m.add_function(wrap_pyfunction!(xray_optical_constants, m)?)?;
    m.add_function(wrap_pyfunction!(refractive_index, m)?)?;
    m.add_function(wrap_pyfunction!(material_names, m)?)?;
    Ok(())
}
