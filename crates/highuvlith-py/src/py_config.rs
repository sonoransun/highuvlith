use pyo3::prelude::*;

use highuvlith_core::mask::Mask;
use highuvlith_core::optics::schwarzschild::SchwarzschildObjective;
use highuvlith_core::optics::zone_plate::FresnelZonePlate;
use highuvlith_core::optics::{OpticalSystem, ProjectionOptics};
use highuvlith_core::resist::{DevelopmentModel, ResistParams};
use highuvlith_core::source::{
    EntangledPhotonSource, HhgGas, HhgSource, IcsSource, IlluminationShape, LithographySource,
    LpaFelSource, LppSource, SourceKind, SpectralShape, SsmbSource, SynchrotronSource, VuvSource,
    XfelSource,
};
use highuvlith_core::thinfilm::{FilmLayer, FilmStack};
use highuvlith_core::types::{Complex64, GridConfig};

/// Illumination source configuration.
///
/// Wraps any concrete `LithographySource` variant (VUV excimer, LPA-FEL).
/// Use the static factory methods (`f2_laser`, `ar2_laser`,
/// `lpa_fel_bella_25nm`, `lpa_fel`) for preset configurations, or the
/// default constructor for a custom VUV source.
#[pyclass(name = "SourceConfig")]
#[derive(Debug, Clone)]
pub struct PySourceConfig {
    pub inner: SourceKind,
}

#[pymethods]
impl PySourceConfig {
    #[new]
    #[pyo3(signature = (wavelength_nm=157.63, sigma_outer=0.7, bandwidth_pm=1.1, spectral_samples=5))]
    fn new(
        wavelength_nm: f64,
        sigma_outer: f64,
        bandwidth_pm: f64,
        spectral_samples: usize,
    ) -> PyResult<Self> {
        if wavelength_nm <= 0.0 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "wavelength_nm must be positive, got {}",
                wavelength_nm
            )));
        }
        if sigma_outer <= 0.0 || sigma_outer > 1.0 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "sigma_outer must be in (0, 1.0], got {}",
                sigma_outer
            )));
        }
        if bandwidth_pm < 0.0 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "bandwidth_pm must be >= 0, got {}",
                bandwidth_pm
            )));
        }
        if spectral_samples < 1 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "spectral_samples must be >= 1, got {}",
                spectral_samples
            )));
        }
        Ok(Self {
            inner: SourceKind::Vuv(VuvSource {
                wavelength_nm,
                bandwidth_pm,
                spectral_samples,
                spectral_shape: SpectralShape::Lorentzian,
                pulse_energy_mj: 10.0,
                rep_rate_hz: 4000.0,
                illumination: IlluminationShape::Conventional { sigma: sigma_outer },
            }),
        })
    }

    /// Create F2 laser (157nm) source.
    #[staticmethod]
    #[pyo3(signature = (sigma=0.7))]
    fn f2_laser(sigma: f64) -> PyResult<Self> {
        let inner = VuvSource::f2_laser(sigma)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: SourceKind::Vuv(inner),
        })
    }

    /// Create Ar2 excimer (126nm) source.
    #[staticmethod]
    #[pyo3(signature = (sigma=0.7))]
    fn ar2_laser(sigma: f64) -> PyResult<Self> {
        let inner = VuvSource::ar2_laser(sigma)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: SourceKind::Vuv(inner),
        })
    }

    /// Create an LPA-FEL source with the BELLA 500 MeV target config (~25 nm).
    ///
    /// Models the projected performance of the laser-plasma driven FEL
    /// at LBNL BELLA (Kohrell et al., Phys. Rev. Accel. Beams, 2026)
    /// after the funded electron-beam upgrade to 500 MeV.
    #[staticmethod]
    #[pyo3(signature = (sigma=0.7))]
    fn lpa_fel_bella_25nm(sigma: f64) -> PyResult<Self> {
        let inner = LpaFelSource::bella_target_25nm(sigma)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: SourceKind::LpaFel(inner),
        })
    }

    /// Create a custom LPA-FEL source at the given wavelength (nm).
    /// Typically used for 20-30 nm EUV lithography studies; the
    /// remaining FEL-specific parameters default to values consistent
    /// with the BELLA architecture and can be overridden.
    #[staticmethod]
    #[pyo3(signature = (
        wavelength_nm,
        sigma=0.7,
        electron_energy_mev=500.0,
        bandwidth_pm=25.0,
        pulse_duration_fs=10.0,
        rep_rate_hz=1000.0,
    ))]
    fn lpa_fel(
        wavelength_nm: f64,
        sigma: f64,
        electron_energy_mev: f64,
        bandwidth_pm: f64,
        pulse_duration_fs: f64,
        rep_rate_hz: f64,
    ) -> PyResult<Self> {
        if bandwidth_pm < 0.0 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "bandwidth_pm must be >= 0, got {}",
                bandwidth_pm
            )));
        }
        if electron_energy_mev <= 0.0 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "electron_energy_mev must be positive, got {}",
                electron_energy_mev
            )));
        }
        if pulse_duration_fs <= 0.0 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "pulse_duration_fs must be positive, got {}",
                pulse_duration_fs
            )));
        }
        if rep_rate_hz <= 0.0 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "rep_rate_hz must be positive, got {}",
                rep_rate_hz
            )));
        }
        let mut inner = LpaFelSource::new(wavelength_nm, sigma)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        inner.electron_energy_mev = electron_energy_mev;
        inner.bandwidth_pm = bandwidth_pm;
        inner.pulse_duration_fs = pulse_duration_fs;
        inner.rep_rate_hz = rep_rate_hz;
        Ok(Self {
            inner: SourceKind::LpaFel(inner),
        })
    }

    /// Sn laser-produced-plasma source at 13.5 nm (NXE-class parameters).
    #[staticmethod]
    #[pyo3(signature = (sigma=0.9))]
    fn lpp_sn_13nm5(sigma: f64) -> PyResult<Self> {
        let inner = LppSource::sn_13nm5(sigma)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: SourceKind::Lpp(inner),
        })
    }

    /// Gd laser-produced-plasma source at 6.7 nm (beyond-EUV).
    #[staticmethod]
    #[pyo3(signature = (sigma=0.9))]
    fn lpp_gd_6nm7(sigma: f64) -> PyResult<Self> {
        let inner = LppSource::gd_6nm7(sigma)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: SourceKind::Lpp(inner),
        })
    }

    /// Synchrotron undulator beamline; the wavelength is DERIVED from
    /// the resonance condition lambda = lambda_u (1 + K^2/2) / (2 n gamma^2).
    #[staticmethod]
    #[pyo3(signature = (electron_energy_gev=0.538, period_mm=20.0, k=1.0, num_periods=100, harmonic=1))]
    fn synchrotron_undulator(
        electron_energy_gev: f64,
        period_mm: f64,
        k: f64,
        num_periods: usize,
        harmonic: usize,
    ) -> PyResult<Self> {
        let inner =
            SynchrotronSource::undulator(electron_energy_gev, period_mm, k, num_periods, harmonic)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: SourceKind::Synchrotron(inner),
        })
    }

    /// LIGA-class bending-magnet beamline (2.5 GeV, 1.5 T, E_c = 6.23 keV).
    #[staticmethod]
    fn synchrotron_liga_bending_magnet() -> Self {
        Self {
            inner: SourceKind::Synchrotron(SynchrotronSource::liga_bending_magnet()),
        }
    }

    /// High-harmonic generation source. Rejects even harmonics and
    /// harmonics beyond the three-step cutoff I_p + 3.17 U_p.
    #[staticmethod]
    #[pyo3(signature = (driver_wavelength_nm=800.0, gas="neon", driver_intensity_w_cm2=4e14, harmonic=59, monochromator_bandwidth_pm=15.0))]
    fn hhg(
        driver_wavelength_nm: f64,
        gas: &str,
        driver_intensity_w_cm2: f64,
        harmonic: usize,
        monochromator_bandwidth_pm: f64,
    ) -> PyResult<Self> {
        let gas = match gas {
            "helium" | "he" => HhgGas::Helium,
            "neon" | "ne" => HhgGas::Neon,
            "argon" | "ar" => HhgGas::Argon,
            "krypton" | "kr" => HhgGas::Krypton,
            "xenon" | "xe" => HhgGas::Xenon,
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown gas '{}' (expected helium/neon/argon/krypton/xenon)",
                    other
                )))
            }
        };
        let inner = HhgSource::new(
            driver_wavelength_nm,
            gas,
            driver_intensity_w_cm2,
            harmonic,
            monochromator_bandwidth_pm,
        )
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: SourceKind::Hhg(inner),
        })
    }

    /// Neon HHG preset reaching 13.56 nm at harmonic 59.
    #[staticmethod]
    fn hhg_ne_13nm5() -> PyResult<Self> {
        let inner = HhgSource::ne_800nm_13nm5()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: SourceKind::Hhg(inner),
        })
    }

    /// FLASH-class SASE XFEL at 13.5 nm.
    #[staticmethod]
    fn xfel_flash_13nm5() -> Self {
        Self {
            inner: SourceKind::Xfel(XfelSource::flash_13nm5()),
        }
    }

    /// FERMI-class seeded FEL near 13.5 nm (rel. bandwidth 5e-5).
    #[staticmethod]
    fn xfel_fermi_seeded() -> Self {
        Self {
            inner: SourceKind::Xfel(XfelSource::fermi_seeded_13nm5()),
        }
    }

    /// Inverse-Compton-scattering source tuned to the target wavelength
    /// (electron energy DERIVED from the Compton kinematics). Theoretical
    /// as a lithography source.
    #[staticmethod]
    #[pyo3(signature = (target_wavelength_nm=13.5, laser_wavelength_nm=1030.0, laser_a0=0.1))]
    fn ics(target_wavelength_nm: f64, laser_wavelength_nm: f64, laser_a0: f64) -> PyResult<Self> {
        let inner = IcsSource::for_wavelength(target_wavelength_nm, laser_wavelength_nm, laser_a0)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: SourceKind::Ics(inner),
        })
    }

    /// Steady-state-microbunching EUV design point (1053 nm modulation,
    /// harmonic 78 -> 13.5 nm, projected kW average power). Theoretical.
    #[staticmethod]
    fn ssmb_euv_13nm5() -> PyResult<Self> {
        let inner = SsmbSource::euv_1kw_13nm5()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: SourceKind::Ssmb(inner),
        })
    }

    /// N-photon entangled NOON-state source (entirely theoretical);
    /// bridges the quantum lithography research module.
    #[staticmethod]
    #[pyo3(signature = (wavelength_nm=157.63, n=2, fidelity=1.0))]
    fn entangled_noon(wavelength_nm: f64, n: usize, fidelity: f64) -> PyResult<Self> {
        let inner = EntangledPhotonSource::noon(wavelength_nm, n, fidelity)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: SourceKind::Entangled(inner),
        })
    }

    #[getter]
    fn wavelength_nm(&self) -> f64 {
        self.inner.wavelength_nm()
    }

    #[getter]
    fn bandwidth_pm(&self) -> f64 {
        self.inner.bandwidth_pm()
    }

    #[getter]
    fn sigma_outer(&self) -> f64 {
        self.inner.sigma_outer().unwrap_or(0.0)
    }

    #[getter]
    fn spectral_samples(&self) -> usize {
        self.inner.spectral_samples()
    }

    /// Short label identifying the source family ("vuv" or "lpa_fel").
    #[getter]
    fn kind(&self) -> &'static str {
        self.inner.kind_label()
    }

    /// Electron beam energy in MeV. Returns `None` for sources without
    /// an electron-beam stage.
    #[getter]
    fn electron_energy_mev(&self) -> Option<f64> {
        match &self.inner {
            SourceKind::LpaFel(s) => Some(s.electron_energy_mev),
            _ => None,
        }
    }

    /// Pulse duration in femtoseconds. Returns `None` when the source
    /// model does not track pulse duration.
    #[getter]
    fn pulse_duration_fs(&self) -> Option<f64> {
        self.inner.pulse_duration_s().map(|s| s * 1e15)
    }

    /// Bunch/pulse repetition rate in Hz. 0.0 for CW/untracked sources.
    #[getter]
    fn rep_rate_hz(&self) -> f64 {
        LithographySource::rep_rate_hz(&self.inner).unwrap_or(0.0)
    }

    /// Transverse coherence fraction in [0, 1]. Returns `None` for
    /// sources that do not model coherence (reported as 0 by the trait).
    #[getter]
    fn transverse_coherence_fraction(&self) -> Option<f64> {
        let coherence = self.inner.transverse_coherence();
        if coherence > 0.0 {
            Some(coherence)
        } else {
            None
        }
    }

    /// Time-averaged output power in watts (pulse energy × rep rate,
    /// or the model's own value for CW sources). `None` when untracked.
    #[getter]
    fn average_power_w(&self) -> Option<f64> {
        self.inner.average_power_w()
    }

    /// Relative rms shot-to-shot pulse-energy fluctuation (0 = stable).
    #[getter]
    fn shot_to_shot_rms(&self) -> f64 {
        self.inner.shot_to_shot_rms()
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            SourceKind::LpaFel(s) => format!(
                "SourceConfig(kind=lpa_fel, wavelength_nm={}, E_e={} MeV, tau={} fs)",
                s.wavelength_nm, s.electron_energy_mev, s.pulse_duration_fs
            ),
            // Generic form: stays total as new source families are added.
            _ => format!(
                "SourceConfig(kind={}, wavelength_nm={})",
                self.inner.kind_label(),
                self.inner.wavelength_nm()
            ),
        }
    }
}

/// Type-erased optics carrier for the Python layer: refractive projection
/// lens, Schwarzschild reflective objective, or Fresnel zone plate.
#[derive(Debug, Clone)]
pub enum PyOpticsInner {
    Refractive(ProjectionOptics),
    Schwarzschild(SchwarzschildObjective),
    ZonePlate(FresnelZonePlate),
}

impl PyOpticsInner {
    pub fn as_dyn(&self) -> &dyn OpticalSystem {
        match self {
            PyOpticsInner::Refractive(o) => o,
            PyOpticsInner::Schwarzschild(o) => o,
            PyOpticsInner::ZonePlate(o) => o,
        }
    }

    pub fn kind_label(&self) -> &'static str {
        match self {
            PyOpticsInner::Refractive(_) => "refractive",
            PyOpticsInner::Schwarzschild(_) => "schwarzschild",
            PyOpticsInner::ZonePlate(_) => "zone_plate",
        }
    }
}

/// Optics configuration (refractive by default; Schwarzschild and zone
/// plate via the static factories — refractive lenses are physically
/// impossible below ~110 nm).
#[pyclass(name = "OpticsConfig")]
#[derive(Debug, Clone)]
pub struct PyOpticsConfig {
    pub inner: PyOpticsInner,
}

#[pymethods]
impl PyOpticsConfig {
    #[new]
    #[pyo3(signature = (numerical_aperture=0.75, reduction=4.0, flare_fraction=0.02))]
    fn new(numerical_aperture: f64, reduction: f64, flare_fraction: f64) -> PyResult<Self> {
        if numerical_aperture <= 0.0 || numerical_aperture > 1.0 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "numerical_aperture must be in (0, 1.0], got {}",
                numerical_aperture
            )));
        }
        if reduction <= 0.0 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "reduction must be positive, got {}",
                reduction
            )));
        }
        if !(0.0..=1.0).contains(&flare_fraction) {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "flare_fraction must be in [0, 1.0], got {}",
                flare_fraction
            )));
        }
        let base = ProjectionOptics::new(numerical_aperture)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: PyOpticsInner::Refractive(ProjectionOptics {
                na: numerical_aperture,
                reduction,
                flare_fraction,
                ..base
            }),
        })
    }

    /// Two-mirror Schwarzschild reflective objective — the physically
    /// correct optics for EUV/BEUV/soft-X-ray wavelengths where no
    /// transparent lens material exists. Defaults are the 13.5 nm
    /// Mo/Si-multilayer preset; pass `mirror_reflectivity=0.50`,
    /// `obscuration_ratio=0.3` for the 6.7 nm La/B4C band.
    #[staticmethod]
    #[pyo3(signature = (numerical_aperture=0.33, obscuration_ratio=0.25, reduction=4.0, mirror_reflectivity=0.67, flare=0.03))]
    fn schwarzschild(
        numerical_aperture: f64,
        obscuration_ratio: f64,
        reduction: f64,
        mirror_reflectivity: f64,
        flare: f64,
    ) -> PyResult<Self> {
        if numerical_aperture <= 0.0 || numerical_aperture >= 1.0 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "numerical_aperture must be in (0, 1), got {}",
                numerical_aperture
            )));
        }
        if !(0.0..1.0).contains(&obscuration_ratio) {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "obscuration_ratio must be in [0, 1), got {}",
                obscuration_ratio
            )));
        }
        if !(0.0..=1.0).contains(&mirror_reflectivity) {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "mirror_reflectivity must be in [0, 1], got {}",
                mirror_reflectivity
            )));
        }
        Ok(Self {
            inner: PyOpticsInner::Schwarzschild(SchwarzschildObjective {
                numerical_aperture,
                obscuration_ratio,
                reduction_ratio: reduction,
                mirror_reflectivity,
                flare,
            }),
        })
    }

    /// Fresnel zone plate (diffractive X-ray focusing): NA is set by the
    /// outermost zone width, NA = lambda / (2 * dr_N); strongly chromatic.
    #[staticmethod]
    fn zone_plate(outer_zone_width_nm: f64, design_wavelength_nm: f64) -> PyResult<Self> {
        let zp = FresnelZonePlate::new(outer_zone_width_nm, design_wavelength_nm)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: PyOpticsInner::ZonePlate(zp),
        })
    }

    /// Add a Zernike aberration coefficient (refractive optics only).
    fn add_aberration(&mut self, fringe_index: usize, coefficient_waves: f64) -> PyResult<()> {
        match &mut self.inner {
            PyOpticsInner::Refractive(o) => {
                o.zernike_coefficients
                    .push((fringe_index, coefficient_waves));
                Ok(())
            }
            _ => Err(pyo3::exceptions::PyValueError::new_err(
                "Zernike aberrations are only modeled for refractive optics",
            )),
        }
    }

    /// Optics family: "refractive", "schwarzschild", or "zone_plate".
    #[getter]
    fn kind(&self) -> &'static str {
        self.inner.kind_label()
    }

    #[getter]
    fn numerical_aperture(&self) -> f64 {
        self.inner.as_dyn().na()
    }

    #[getter]
    fn reduction(&self) -> f64 {
        self.inner.as_dyn().reduction()
    }

    #[getter]
    fn flare_fraction(&self) -> f64 {
        self.inner.as_dyn().flare_fraction()
    }

    fn rayleigh_resolution(&self, wavelength_nm: f64) -> f64 {
        self.inner.as_dyn().rayleigh_resolution(wavelength_nm)
    }

    fn __repr__(&self) -> String {
        format!(
            "OpticsConfig(kind={}, NA={}, reduction={}x)",
            self.inner.kind_label(),
            self.inner.as_dyn().na(),
            self.inner.as_dyn().reduction()
        )
    }
}

/// Mask configuration.
#[pyclass(name = "MaskConfig")]
#[derive(Debug, Clone)]
pub struct PyMaskConfig {
    pub inner: Mask,
}

#[pymethods]
impl PyMaskConfig {
    /// Create a line/space pattern.
    #[staticmethod]
    fn line_space(cd_nm: f64, pitch_nm: f64) -> PyResult<Self> {
        let inner = Mask::line_space(cd_nm, pitch_nm)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Create a contact hole array.
    #[staticmethod]
    fn contact_hole(diameter_nm: f64, pitch_x_nm: f64, pitch_y_nm: f64) -> PyResult<Self> {
        let inner = Mask::contact_hole(diameter_nm, pitch_x_nm, pitch_y_nm)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    fn __repr__(&self) -> String {
        format!("MaskConfig({} features)", self.inner.features.len())
    }
}

/// Photoresist configuration.
#[pyclass(name = "ResistConfig")]
#[derive(Debug, Clone)]
pub struct PyResistConfig {
    pub inner: ResistParams,
}

#[pymethods]
impl PyResistConfig {
    #[new]
    #[pyo3(signature = (
        thickness_nm=150.0,
        dill_a=0.2,
        dill_b=0.45,
        dill_c=0.02,
        peb_diffusion_nm=30.0,
        model="mack"
    ))]
    fn new(
        thickness_nm: f64,
        dill_a: f64,
        dill_b: f64,
        dill_c: f64,
        peb_diffusion_nm: f64,
        model: &str,
    ) -> PyResult<Self> {
        if thickness_nm <= 0.0 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "thickness_nm must be positive, got {}",
                thickness_nm
            )));
        }
        if peb_diffusion_nm < 0.0 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "peb_diffusion_nm must be >= 0, got {}",
                peb_diffusion_nm
            )));
        }
        let development = match model {
            "threshold" => DevelopmentModel::Threshold { threshold: 0.5 },
            "mack" => DevelopmentModel::default(),
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Unknown development model: {}. Use 'threshold' or 'mack'",
                    model
                )))
            }
        };

        Ok(Self {
            inner: ResistParams {
                thickness_nm,
                dill_a,
                dill_b,
                dill_c,
                peb_diffusion_nm,
                development,
            },
        })
    }

    /// Create VUV fluoropolymer resist with default parameters.
    #[staticmethod]
    fn vuv_fluoropolymer() -> Self {
        Self {
            inner: ResistParams::vuv_fluoropolymer(),
        }
    }

    #[getter]
    fn thickness_nm(&self) -> f64 {
        self.inner.thickness_nm
    }

    #[getter]
    fn dill_a(&self) -> f64 {
        self.inner.dill_a
    }

    #[getter]
    fn dill_b(&self) -> f64 {
        self.inner.dill_b
    }

    #[getter]
    fn dill_c(&self) -> f64 {
        self.inner.dill_c
    }

    #[getter]
    fn peb_diffusion_nm(&self) -> f64 {
        self.inner.peb_diffusion_nm
    }

    fn __repr__(&self) -> String {
        format!(
            "ResistConfig(thickness={}nm, A={}, B={}, C={})",
            self.inner.thickness_nm, self.inner.dill_a, self.inner.dill_b, self.inner.dill_c
        )
    }
}

/// Film stack configuration.
#[pyclass(name = "FilmStackConfig")]
#[derive(Debug, Clone)]
pub struct PyFilmStackConfig {
    pub inner: FilmStack,
}

#[pymethods]
impl PyFilmStackConfig {
    #[new]
    fn new() -> Self {
        Self {
            inner: FilmStack::default(),
        }
    }

    /// Add a layer to the stack.
    fn add_layer(&mut self, name: &str, thickness_nm: f64, n_real: f64, n_imag: f64) {
        self.inner.layers.push(FilmLayer {
            name: name.to_string(),
            thickness_nm,
            n: Complex64::new(n_real, n_imag),
        });
    }

    /// Set the substrate refractive index.
    fn set_substrate(&mut self, n_real: f64, n_imag: f64) {
        self.inner.substrate = Complex64::new(n_real, n_imag);
    }

    fn __repr__(&self) -> String {
        format!("FilmStackConfig({} layers)", self.inner.layers.len())
    }
}

/// Process parameters.
#[pyclass(name = "ProcessConfig")]
#[derive(Debug, Clone)]
pub struct PyProcessConfig {
    #[pyo3(get, set)]
    pub dose_mj_cm2: f64,
    #[pyo3(get, set)]
    pub focus_nm: f64,
    #[pyo3(get, set)]
    pub development_time_s: f64,
}

#[pymethods]
impl PyProcessConfig {
    #[new]
    #[pyo3(signature = (dose_mj_cm2=30.0, focus_nm=0.0, development_time_s=60.0))]
    fn new(dose_mj_cm2: f64, focus_nm: f64, development_time_s: f64) -> Self {
        Self {
            dose_mj_cm2,
            focus_nm,
            development_time_s,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ProcessConfig(dose={}mJ/cm², focus={}nm)",
            self.dose_mj_cm2, self.focus_nm
        )
    }
}

/// Grid configuration.
#[pyclass(name = "GridConfig")]
#[derive(Debug, Clone)]
pub struct PyGridConfig {
    pub inner: GridConfig,
}

#[pymethods]
impl PyGridConfig {
    #[new]
    #[pyo3(signature = (size=512, pixel_nm=1.0))]
    fn new(size: usize, pixel_nm: f64) -> PyResult<Self> {
        let inner = GridConfig::new(size, pixel_nm)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    #[getter]
    fn size(&self) -> usize {
        self.inner.size
    }

    #[getter]
    fn pixel_nm(&self) -> f64 {
        self.inner.pixel_nm
    }

    fn field_size_nm(&self) -> f64 {
        self.inner.field_size_nm()
    }

    fn __repr__(&self) -> String {
        format!(
            "GridConfig({}x{}, pixel={}nm, field={}nm)",
            self.inner.size,
            self.inner.size,
            self.inner.pixel_nm,
            self.inner.field_size_nm()
        )
    }
}
