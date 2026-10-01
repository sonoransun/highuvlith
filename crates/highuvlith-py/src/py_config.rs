use pyo3::prelude::*;

use highuvlith_core::mask::{LineOrientation, Mask, MaskFeature, MaskType, SpectrumMethod};
use highuvlith_core::optics::euv::EuvProjectionOptics;
use highuvlith_core::optics::schwarzschild::SchwarzschildObjective;
use highuvlith_core::optics::zone_plate::FresnelZonePlate;
use highuvlith_core::optics::{OpticalSystem, ProjectionOptics};
use highuvlith_core::resist::{DevelopmentModel, ResistParams};
use highuvlith_core::source::{
    BetatronSource, DppSource, SmithPurcellSource, SxrlScheme, SxrlSource, XrayAnode,
    XrayTubeSource,
};
use highuvlith_core::source::{
    EntangledPhotonSource, HhgGas, HhgSource, IcsSource, IlluminationShape, LithographySource,
    LpaFelSource, LppSource, SourceKind, SpectralShape, SsmbSource, SynchrotronSource, VuvSource,
    XfelSource,
};
use highuvlith_core::source_models::{
    ics::IcsCollision,
    lpp::LppDriveLaser,
    physics::{ElectronBeamParams, UndulatorParams},
    ssmb::SsmbRadiator,
    synchrotron::StorageRingBeam,
    throughput::{self, ThroughputParams},
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

    /// ArF excimer laser at 193.368 nm (line-narrowed immersion-scanner
    /// class). Representative values: 0.2 pm FWHM Gaussian line (E95 ~0.33
    /// pm), 15 mJ x 6 kHz = 90 W.
    #[staticmethod]
    #[pyo3(signature = (sigma=0.7))]
    fn arf_laser(sigma: f64) -> PyResult<Self> {
        let inner = VuvSource::arf_laser(sigma).map_err(src_core_error)?;
        Ok(Self {
            inner: SourceKind::Vuv(inner),
        })
    }

    /// KrF excimer laser at ~248.3 nm. Representative values: 0.6 pm FWHM
    /// Gaussian line (E95 ~1.0 pm), 10 mJ x 4 kHz = 40 W.
    #[staticmethod]
    #[pyo3(signature = (sigma=0.7))]
    fn krf_laser(sigma: f64) -> PyResult<Self> {
        let inner = VuvSource::krf_laser(sigma).map_err(src_core_error)?;
        Ok(Self {
            inner: SourceKind::Vuv(inner),
        })
    }

    /// Mercury i-line lamp, 365.0153 nm (NIST air wavelength): filtered
    /// ~3 nm line, CW (no pulse metadata), one spectral sample.
    #[staticmethod]
    #[pyo3(signature = (sigma=0.7))]
    fn hg_i_line(sigma: f64) -> PyResult<Self> {
        let inner = VuvSource::hg_i_line(sigma).map_err(src_core_error)?;
        Ok(Self {
            inner: SourceKind::Vuv(inner),
        })
    }

    /// Mercury h-line lamp, 404.6563 nm (NIST air wavelength), CW.
    #[staticmethod]
    #[pyo3(signature = (sigma=0.7))]
    fn hg_h_line(sigma: f64) -> PyResult<Self> {
        let inner = VuvSource::hg_h_line(sigma).map_err(src_core_error)?;
        Ok(Self {
            inner: SourceKind::Vuv(inner),
        })
    }

    /// Mercury g-line lamp, 435.8328 nm (NIST air wavelength), CW.
    #[staticmethod]
    #[pyo3(signature = (sigma=0.7))]
    fn hg_g_line(sigma: f64) -> PyResult<Self> {
        let inner = VuvSource::hg_g_line(sigma).map_err(src_core_error)?;
        Ok(Self {
            inner: SourceKind::Vuv(inner),
        })
    }

    /// Create an LPA-FEL source at the 25 nm / 500 MeV / 1 kHz DESIGN
    /// PROJECTION (an EUV study point). The demonstrated LPA-FEL anchor is
    /// LBNL BELLA's 420 nm SASE lasing at 100 MeV and 1 Hz (Kohrell et al.,
    /// Phys. Rev. Accel. Beams 29, 041301 (2026)). Its bandwidth is the
    /// derived SASE 2·rho·lambda of its illustrative beam (~266 pm).
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
    ///
    /// Optional machine parameters: `undulator_period_mm` + `undulator_k`
    /// attach an undulator whose resonance wavelength is DERIVED and must
    /// match `wavelength_nm` within 5 %; the beam parameters
    /// (`peak_current_a`, `norm_emittance_um`, `energy_spread_rel`,
    /// `beta_m`; missing ones take illustrative LPA-class defaults) add the
    /// derived 1D / Ming-Xie FEL physics to `derived_quantities()`.
    ///
    /// `bandwidth_pm` (FWHM) is an explicit override. When omitted the
    /// bandwidth is DERIVED as the SASE estimate 2·rho·lambda if the
    /// undulator and beam parameters are given, else 0.1 % of lambda.
    #[staticmethod]
    #[pyo3(signature = (
        wavelength_nm,
        sigma=0.7,
        electron_energy_mev=500.0,
        bandwidth_pm=None,
        pulse_duration_fs=10.0,
        rep_rate_hz=1000.0,
        pulse_energy_uj=None,
        undulator_period_mm=None,
        undulator_k=None,
        num_periods=200,
        peak_current_a=None,
        norm_emittance_um=None,
        energy_spread_rel=None,
        beta_m=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn lpa_fel(
        wavelength_nm: f64,
        sigma: f64,
        electron_energy_mev: f64,
        bandwidth_pm: Option<f64>,
        pulse_duration_fs: f64,
        rep_rate_hz: f64,
        pulse_energy_uj: Option<f64>,
        undulator_period_mm: Option<f64>,
        undulator_k: Option<f64>,
        num_periods: usize,
        peak_current_a: Option<f64>,
        norm_emittance_um: Option<f64>,
        energy_spread_rel: Option<f64>,
        beta_m: Option<f64>,
    ) -> PyResult<Self> {
        if let Some(bw) = bandwidth_pm {
            if !(bw.is_finite() && bw >= 0.0) {
                return Err(src_value_error(format!(
                    "bandwidth_pm must be finite and >= 0, got {bw}"
                )));
            }
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
        if let Some(bw) = bandwidth_pm {
            // Explicit: overrides the derived 2 rho lambda.
            inner.bandwidth_pm = bw;
            inner.bandwidth_override_pm = Some(bw);
        }
        inner.pulse_duration_fs = pulse_duration_fs;
        inner.rep_rate_hz = rep_rate_hz;
        if let Some(pe) = pulse_energy_uj {
            if !(pe.is_finite() && pe > 0.0) {
                return Err(src_value_error(format!(
                    "pulse_energy_uj must be positive, got {pe}"
                )));
            }
            inner.pulse_energy_uj = pe;
        }
        let any_beam = peak_current_a.is_some()
            || norm_emittance_um.is_some()
            || energy_spread_rel.is_some()
            || beta_m.is_some();
        match (undulator_period_mm, undulator_k) {
            (Some(period_mm), Some(k)) => {
                let und = UndulatorParams {
                    period_mm,
                    k,
                    num_periods,
                };
                let beam = any_beam.then(|| ElectronBeamParams {
                    peak_current_a: peak_current_a.unwrap_or(1000.0),
                    norm_emittance_um: norm_emittance_um.unwrap_or(0.5),
                    energy_spread_rel: energy_spread_rel.unwrap_or(0.01),
                    beta_m: beta_m.unwrap_or(1.0),
                });
                inner = inner.with_machine(und, beam).map_err(src_core_error)?;
            }
            (None, None) => {
                if any_beam {
                    return Err(src_value_error(
                        "LPA-FEL beam parameters need an undulator \
                         (undulator_period_mm and undulator_k)"
                            .to_string(),
                    ));
                }
            }
            _ => {
                return Err(src_value_error(
                    "an LPA-FEL undulator needs both undulator_period_mm and undulator_k"
                        .to_string(),
                ))
            }
        }
        Ok(Self {
            inner: SourceKind::LpaFel(inner),
        })
    }

    /// Sn laser-produced-plasma source at 13.5 nm (NXE-class parameters:
    /// ~250 W in-band at intermediate focus). `drive_laser` = "co2"
    /// (default), "solid_state_1um", or "thulium_2um" selects the drive
    /// technology and its conversion-efficiency default (reported for CO2
    /// and 1 um, a projection for 2 um) at the same 21.5 kW drive power.
    #[staticmethod]
    #[pyo3(signature = (sigma=0.9, drive_laser="co2"))]
    fn lpp_sn_13nm5(sigma: f64, drive_laser: &str) -> PyResult<Self> {
        let laser = match drive_laser {
            "co2" => LppDriveLaser::Co2,
            "solid_state_1um" | "1um" | "nd_yag" => LppDriveLaser::SolidState1um,
            "thulium_2um" | "2um" | "thulium" => LppDriveLaser::Thulium2um,
            other => {
                return Err(src_value_error(format!(
                    "unknown drive_laser '{other}' (expected co2 / solid_state_1um / thulium_2um)"
                )))
            }
        };
        let inner = LppSource::sn_with_drive_laser(sigma, laser).map_err(src_core_error)?;
        Ok(Self {
            inner: SourceKind::Lpp(inner),
        })
    }

    /// NXE:3800E-class Sn LPP: 500 W in-band at IF (ASML Investor Day
    /// 2024); the 43 kW CO2 drive behind it is an assumed scaling of the
    /// NXE:3400B chain (same CE, collection and 50 kHz).
    #[staticmethod]
    #[pyo3(signature = (sigma=0.9))]
    fn lpp_sn_13nm5_500w(sigma: f64) -> PyResult<Self> {
        let inner = LppSource::sn_13nm5_500w(sigma).map_err(src_core_error)?;
        Ok(Self {
            inner: SourceKind::Lpp(inner),
        })
    }

    /// Tb laser-produced-plasma source at 6.5 nm (beyond-EUV).
    #[staticmethod]
    #[pyo3(signature = (sigma=0.9))]
    fn lpp_tb_6nm5(sigma: f64) -> PyResult<Self> {
        let inner = LppSource::tb_6nm5(sigma).map_err(src_core_error)?;
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
    /// `ring_current_ma` sets the absolute flux/power; giving ring
    /// emittances (nm rad) DERIVES the transverse coherent fraction (and
    /// the pupil) instead of the legacy 0.2 constant.
    #[staticmethod]
    #[pyo3(signature = (
        electron_energy_gev=0.538,
        period_mm=20.0,
        k=1.0,
        num_periods=100,
        harmonic=1,
        ring_current_ma=200.0,
        emittance_x_nm_rad=None,
        emittance_y_nm_rad=None,
        energy_spread_rel=5e-4,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn synchrotron_undulator(
        electron_energy_gev: f64,
        period_mm: f64,
        k: f64,
        num_periods: usize,
        harmonic: usize,
        ring_current_ma: f64,
        emittance_x_nm_rad: Option<f64>,
        emittance_y_nm_rad: Option<f64>,
        energy_spread_rel: f64,
    ) -> PyResult<Self> {
        let mut inner =
            SynchrotronSource::undulator(electron_energy_gev, period_mm, k, num_periods, harmonic)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        if !(ring_current_ma.is_finite() && ring_current_ma >= 0.0) {
            return Err(src_value_error(format!(
                "ring_current_ma must be >= 0, got {ring_current_ma}"
            )));
        }
        inner.ring_current_ma = ring_current_ma;
        if emittance_x_nm_rad.is_some() || emittance_y_nm_rad.is_some() {
            let beam = StorageRingBeam {
                emittance_x_nm_rad: emittance_x_nm_rad.unwrap_or(10.0),
                emittance_y_nm_rad: emittance_y_nm_rad.unwrap_or(0.1),
                energy_spread_rel,
            };
            if beam.emittance_x_nm_rad < 0.0
                || beam.emittance_y_nm_rad < 0.0
                || beam.energy_spread_rel < 0.0
            {
                return Err(src_value_error(
                    "emittances and energy_spread_rel must be >= 0".to_string(),
                ));
            }
            inner = inner.with_ring_beam(beam);
        }
        Ok(Self {
            inner: SourceKind::Synchrotron(inner),
        })
    }

    /// Compact EUV undulator preset (538 MeV, 20 mm, K = 1, N = 100) on the
    /// illustrative compact-ring beam: coherent fraction DERIVED (≈ 0.089).
    #[staticmethod]
    fn synchrotron_compact_euv() -> PyResult<Self> {
        let inner = SynchrotronSource::compact_euv_undulator().map_err(src_core_error)?;
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
    ///
    /// The harmonic power is DERIVED as driver average power x an
    /// order-of-magnitude conversion-efficiency estimate (or the measured
    /// `conversion_efficiency`); `driver_average_power_w=None` keeps the
    /// ASSUMED 5 W default (~0.23 µW at 13.5 nm in Ne). A derived quantity
    /// `power_vs_measured_record` > 1 marks a PROJECTION beyond measured HHG.
    /// `full_comb=True` switches to the bookkeeping comb, optionally
    /// restricted to `comb_passband_nm`.
    #[staticmethod]
    #[pyo3(signature = (
        driver_wavelength_nm=800.0,
        gas="neon",
        driver_intensity_w_cm2=4e14,
        harmonic=59,
        monochromator_bandwidth_pm=15.0,
        full_comb=false,
        driver_average_power_w=None,
        conversion_efficiency=None,
        comb_passband_nm=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn hhg(
        driver_wavelength_nm: f64,
        gas: &str,
        driver_intensity_w_cm2: f64,
        harmonic: usize,
        monochromator_bandwidth_pm: f64,
        full_comb: bool,
        driver_average_power_w: Option<f64>,
        conversion_efficiency: Option<f64>,
        comb_passband_nm: Option<Vec<f64>>,
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
        let mut inner = HhgSource::new(
            driver_wavelength_nm,
            gas,
            driver_intensity_w_cm2,
            harmonic,
            monochromator_bandwidth_pm,
        )
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        if let Some(p) = driver_average_power_w {
            if !(p.is_finite() && p > 0.0) {
                return Err(src_value_error(format!(
                    "driver_average_power_w must be positive, got {p}"
                )));
            }
        }
        if let Some(ce) = conversion_efficiency {
            if !(ce > 0.0 && ce <= 1.0) {
                return Err(src_value_error(format!(
                    "conversion_efficiency must be in (0, 1], got {ce}"
                )));
            }
        }
        if driver_average_power_w.is_some() {
            inner.driver_average_power_w = driver_average_power_w;
        }
        inner.conversion_efficiency = conversion_efficiency;
        inner.comb_passband_nm = match comb_passband_nm.as_deref() {
            None => None,
            Some([lo, hi]) => Some([*lo, *hi]),
            Some(other) => {
                return Err(src_value_error(format!(
                    "comb_passband_nm must be (min_nm, max_nm), got {} values",
                    other.len()
                )))
            }
        };
        if full_comb {
            inner.monochromator = None;
        }
        Ok(Self {
            inner: SourceKind::Hhg(inner),
        })
    }

    /// Argon HHG preset: harmonic 27 of 800 nm -> 29.6 nm; an assumed 3 W
    /// driver derives 30 µW (tens of µW, below the measured record).
    #[staticmethod]
    fn hhg_ar_30nm() -> PyResult<Self> {
        let inner = HhgSource::ar_800nm_30nm().map_err(src_core_error)?;
        Ok(Self {
            inner: SourceKind::Hhg(inner),
        })
    }

    /// Neon HHG preset reaching 13.56 nm at harmonic 59; an assumed 20 W
    /// driver derives ~0.9 µW (measured 13.5 nm records: 0.43 to ~1 µW).
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

    /// PROJECTION: LCLS-II-like CW superconducting-linac FEL scaled to
    /// 13.5 nm (1 GeV, 1 MHz); pulse energy DERIVED (~135 uJ, ~135 W).
    #[staticmethod]
    fn xfel_cw_sc_13nm5() -> Self {
        Self {
            inner: SourceKind::Xfel(XfelSource::cw_sc_13nm5()),
        }
    }

    /// PROJECTION: ERL-driven FEL EUV-lithography design point (800 MeV,
    /// 162.5 MHz); pulse energy DERIVED (~65 uJ, ~10.5 kW average).
    #[staticmethod]
    fn xfel_erl_13nm5() -> Self {
        Self {
            inner: SourceKind::Xfel(XfelSource::erl_13nm5()),
        }
    }

    /// Inverse-Compton-scattering source tuned to the target wavelength
    /// (electron energy DERIVED from the Compton kinematics). Theoretical
    /// as a lithography source.
    ///
    /// Collision parameters (`bunch_charge_pc`, `laser_pulse_energy_mj`,
    /// `electron_spot_um`, `laser_spot_um`; missing ones take the
    /// aggressive design-point defaults) DERIVE the photon yield from the
    /// Thomson luminosity; the collection half-angle then defaults to the
    /// 2 % Mo/Si band.
    #[staticmethod]
    #[pyo3(signature = (
        target_wavelength_nm=13.5,
        laser_wavelength_nm=1030.0,
        laser_a0=0.1,
        bunch_charge_pc=None,
        laser_pulse_energy_mj=None,
        electron_spot_um=None,
        laser_spot_um=None,
        rep_rate_hz=None,
        collection_half_angle_mrad=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn ics(
        target_wavelength_nm: f64,
        laser_wavelength_nm: f64,
        laser_a0: f64,
        bunch_charge_pc: Option<f64>,
        laser_pulse_energy_mj: Option<f64>,
        electron_spot_um: Option<f64>,
        laser_spot_um: Option<f64>,
        rep_rate_hz: Option<f64>,
        collection_half_angle_mrad: Option<f64>,
    ) -> PyResult<Self> {
        let mut inner =
            IcsSource::for_wavelength(target_wavelength_nm, laser_wavelength_nm, laser_a0)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        if let Some(rr) = rep_rate_hz {
            if !(rr.is_finite() && rr > 0.0) {
                return Err(src_value_error(format!(
                    "rep_rate_hz must be positive, got {rr}"
                )));
            }
            inner.rep_rate_hz = rr;
        }
        if let Some(a) = collection_half_angle_mrad {
            if !(a.is_finite() && a > 0.0) {
                return Err(src_value_error(format!(
                    "collection_half_angle_mrad must be positive, got {a}"
                )));
            }
            inner.collection_half_angle_mrad = a;
        }
        if bunch_charge_pc.is_some()
            || laser_pulse_energy_mj.is_some()
            || electron_spot_um.is_some()
            || laser_spot_um.is_some()
        {
            let d = IcsCollision::high_average_power_design();
            let collision = IcsCollision {
                bunch_charge_pc: bunch_charge_pc.unwrap_or(d.bunch_charge_pc),
                laser_pulse_energy_mj: laser_pulse_energy_mj.unwrap_or(d.laser_pulse_energy_mj),
                electron_spot_um: electron_spot_um.unwrap_or(d.electron_spot_um),
                laser_spot_um: laser_spot_um.unwrap_or(d.laser_spot_um),
            };
            if collection_half_angle_mrad.is_none() {
                inner.collection_half_angle_mrad =
                    highuvlith_core::source_models::physics::ics_half_angle_for_bandwidth(
                        inner.gamma(),
                        inner.laser_a0,
                        0.02,
                    ) * 1e3;
            }
            inner = inner.with_collision(collision).map_err(src_core_error)?;
        }
        Ok(Self {
            inner: SourceKind::Ics(inner),
        })
    }

    /// Compact EUV ICS design point with the photon yield DERIVED from the
    /// collision (100 pC, 10 mJ, 10 um spots, 100 MHz, 2 % band): ~71 uW.
    #[staticmethod]
    fn ics_compact_euv_13nm5() -> PyResult<Self> {
        let inner = IcsSource::compact_euv_13nm5().map_err(src_core_error)?;
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

    /// General SSMB source. With `bunching_factor` (or any radiator
    /// field) the average power is DERIVED from coherent emission,
    /// `P = 2 pi^2 alpha hbar N Q_1(K) |b|^2 (I_avg/e)(I_pk/e)`; a missing
    /// `bunching_factor` defaults to the value `average_power_w` requires.
    /// Without radiator fields `average_power_w` is the stored projection.
    #[staticmethod]
    #[pyo3(signature = (
        ring_energy_mev=400.0,
        modulation_wavelength_nm=1053.0,
        target_wavelength_nm=13.5,
        average_power_w=1000.0,
        average_current_a=None,
        peak_current_a=None,
        bunching_factor=None,
        radiator_periods=None,
        radiator_k=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn ssmb(
        ring_energy_mev: f64,
        modulation_wavelength_nm: f64,
        target_wavelength_nm: f64,
        average_power_w: f64,
        average_current_a: Option<f64>,
        peak_current_a: Option<f64>,
        bunching_factor: Option<f64>,
        radiator_periods: Option<usize>,
        radiator_k: Option<f64>,
    ) -> PyResult<Self> {
        let mut inner = SsmbSource::new(
            ring_energy_mev,
            modulation_wavelength_nm,
            target_wavelength_nm,
            average_power_w,
        )
        .map_err(src_core_error)?;
        if average_current_a.is_some()
            || peak_current_a.is_some()
            || bunching_factor.is_some()
            || radiator_periods.is_some()
            || radiator_k.is_some()
        {
            let avg = average_current_a.unwrap_or(1.0);
            let mut radiator = SsmbRadiator {
                average_current_a: avg,
                peak_current_a: peak_current_a.unwrap_or(avg),
                bunching_factor: 1.0,
                num_periods: radiator_periods.unwrap_or(100),
                k: radiator_k.unwrap_or(1.6),
            };
            radiator.bunching_factor = bunching_factor.unwrap_or_else(|| {
                SsmbSource::bunching_required(&radiator, average_power_w).min(1.0)
            });
            inner = inner.with_radiator(radiator).map_err(src_core_error)?;
        }
        Ok(Self {
            inner: SourceKind::Ssmb(inner),
        })
    }

    /// N-photon entangled NOON-state source (entirely theoretical);
    /// bridges the quantum lithography research module.
    /// `pair_rate_hz` feeds the derived ETPA exposure-time estimates.
    #[staticmethod]
    #[pyo3(signature = (wavelength_nm=157.63, n=2, fidelity=1.0, pair_rate_hz=1e6))]
    fn entangled_noon(
        wavelength_nm: f64,
        n: usize,
        fidelity: f64,
        pair_rate_hz: f64,
    ) -> PyResult<Self> {
        let mut inner = EntangledPhotonSource::noon(wavelength_nm, n, fidelity)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        if !(pair_rate_hz.is_finite() && pair_rate_hz >= 0.0) {
            return Err(src_value_error(format!(
                "pair_rate_hz must be >= 0, got {pair_rate_hz}"
            )));
        }
        inner.pair_rate_hz = pair_rate_hz;
        Ok(Self {
            inner: SourceKind::Entangled(inner),
        })
    }

    /// Hard X-ray tube: Kramers bremsstrahlung + anode characteristic lines
    /// through a Be window. `kvp` / `current_ma` default to the anode's
    /// preset (W 60 kV/30 mA, Mo 50/40, Cu 40/40, Rh 50/40). Broadband and
    /// incoherent: for LIGA / proximity printing, not projection imaging.
    #[staticmethod]
    #[pyo3(signature = (anode="W", kvp=None, current_ma=None, be_window_um=250.0))]
    fn xray_tube(
        anode: &str,
        kvp: Option<f64>,
        current_ma: Option<f64>,
        be_window_um: f64,
    ) -> PyResult<Self> {
        let anode = XrayAnode::from_name(anode).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "unknown anode '{}' (expected W, Mo, Cu, or Rh)",
                anode
            ))
        })?;
        let mut inner = XrayTubeSource::preset(anode)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        if let Some(v) = kvp {
            inner.kvp = v;
        }
        if let Some(ma) = current_ma {
            inner.current_ma = ma;
        }
        inner.be_window_um = be_window_um;
        inner
            .validate()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: SourceKind::XrayTube(inner),
        })
    }

    /// Xe discharge-produced plasma at 13.5 nm (metrology class: 10 W in-band
    /// into 2π sr at the source, the reported Energetiq EQ-10 class; reported
    /// CE ~0.5%, pinch/collector parameters assumed).
    #[staticmethod]
    #[pyo3(signature = (sigma=0.9))]
    fn dpp_xe_13nm5(sigma: f64) -> PyResult<Self> {
        let inner = DppSource::xe_13nm5(sigma)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: SourceKind::Dpp(inner),
        })
    }

    /// Sn discharge (DPP / laser-assisted LDP) plasma at 13.5 nm (historical
    /// HVM candidate): 360 W in-band into 2π sr at the source (the reported
    /// continuous TRINITI level) → ≈34 W at intermediate focus; reported CE
    /// ~2%, pinch/collector parameters assumed.
    #[staticmethod]
    #[pyo3(signature = (sigma=0.9))]
    fn dpp_sn_13nm5(sigma: f64) -> PyResult<Self> {
        let inner = DppSource::sn_13nm5(sigma)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: SourceKind::Dpp(inner),
        })
    }

    /// Desk-top capillary-discharge Ne-like Ar soft-X-ray laser at 46.9 nm
    /// (13 µJ × 12 Hz = 0.16 mW, the reported desk-top output).
    #[staticmethod]
    fn sxrl_ar_46nm9() -> PyResult<Self> {
        let inner = SxrlSource::ar_46nm9()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: SourceKind::Sxrl(inner),
        })
    }

    /// Transient-collisional Ni-like Ag soft-X-ray laser at 13.9 nm (~0.1 mW,
    /// the reported order of magnitude).
    #[staticmethod]
    fn sxrl_ag_13nm9() -> PyResult<Self> {
        let inner = SxrlSource::ag_13nm9()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: SourceKind::Sxrl(inner),
        })
    }

    /// Plasma soft-X-ray laser by scheme ("ar_46nm9", "ag_13nm9",
    /// "cd_13nm2", "mo_18nm9"); unset parameters keep the scheme preset.
    #[staticmethod]
    #[pyo3(signature = (scheme="ag_13nm9", pulse_energy_uj=None, rep_rate_hz=None, pulse_duration_ps=None, rel_linewidth=None))]
    fn sxrl(
        scheme: &str,
        pulse_energy_uj: Option<f64>,
        rep_rate_hz: Option<f64>,
        pulse_duration_ps: Option<f64>,
        rel_linewidth: Option<f64>,
    ) -> PyResult<Self> {
        let scheme = SxrlScheme::from_name(scheme).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "unknown scheme '{}' (expected ar_46nm9, ag_13nm9, cd_13nm2, or mo_18nm9)",
                scheme
            ))
        })?;
        let mut inner = SxrlSource::preset(scheme)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        if let Some(v) = pulse_energy_uj {
            inner.pulse_energy_uj = v;
        }
        if let Some(v) = rep_rate_hz {
            inner.rep_rate_hz = v;
        }
        if let Some(v) = pulse_duration_ps {
            inner.pulse_duration_ps = v;
        }
        if let Some(v) = rel_linewidth {
            inner.rel_linewidth = v;
        }
        inner
            .validate()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: SourceKind::Sxrl(inner),
        })
    }

    /// Laser-wakefield betatron X-ray source (theoretical for lithography);
    /// K, the critical energy and the photon yield are DERIVED from the
    /// plasma and beam parameters. K < 1 (undulator regime) is rejected.
    #[staticmethod]
    #[pyo3(signature = (electron_energy_mev=200.0, plasma_density_cm3=1e19, betatron_amplitude_um=1.0, interaction_length_mm=3.0, bunch_charge_pc=50.0, rep_rate_hz=10.0))]
    fn betatron(
        electron_energy_mev: f64,
        plasma_density_cm3: f64,
        betatron_amplitude_um: f64,
        interaction_length_mm: f64,
        bunch_charge_pc: f64,
        rep_rate_hz: f64,
    ) -> PyResult<Self> {
        let inner = BetatronSource::new(
            electron_energy_mev,
            plasma_density_cm3,
            betatron_amplitude_um,
            interaction_length_mm,
            bunch_charge_pc,
            rep_rate_hz,
        )
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: SourceKind::Betatron(inner),
        })
    }

    /// Smith-Purcell free-electron grating source (theoretical). The
    /// wavelength follows lambda = (a/m)(1/beta - cos theta): give
    /// `grating_period_nm` to derive it, or omit it to derive the period for
    /// `target_wavelength_nm`. Power is an order-of-magnitude estimate with
    /// an assumed coupling efficiency.
    #[staticmethod]
    #[pyo3(signature = (target_wavelength_nm=13.5, electron_energy_kev=30.0, diffraction_order=1, observation_angle_deg=90.0, grating_period_nm=None, num_periods=100, beam_current_na=10.0, coupling_efficiency=1e-3, impact_height_nm=0.0))]
    #[allow(clippy::too_many_arguments)]
    fn smith_purcell(
        target_wavelength_nm: f64,
        electron_energy_kev: f64,
        diffraction_order: usize,
        observation_angle_deg: f64,
        grating_period_nm: Option<f64>,
        num_periods: usize,
        beam_current_na: f64,
        coupling_efficiency: f64,
        impact_height_nm: f64,
    ) -> PyResult<Self> {
        let mut inner = match grating_period_nm {
            Some(period) => SmithPurcellSource::new(
                electron_energy_kev,
                period,
                diffraction_order,
                observation_angle_deg,
            ),
            None => SmithPurcellSource::for_wavelength(
                target_wavelength_nm,
                electron_energy_kev,
                diffraction_order,
                observation_angle_deg,
            ),
        }
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        inner.num_periods = num_periods;
        inner.beam_current_na = beam_current_na;
        inner.coupling_efficiency = coupling_efficiency;
        inner.impact_height_nm = impact_height_nm;
        inner
            .validate()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: SourceKind::SmithPurcell(inner),
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
            SourceKind::Ics(s) => Some(s.electron_energy_mev),
            SourceKind::Ssmb(s) => Some(s.ring_energy_mev),
            SourceKind::Xfel(s) => s.machine.map(|m| m.electron_energy_mev),
            SourceKind::Synchrotron(s) => {
                Some((s.gamma() - 1.0) * highuvlith_core::source_models::physics::ELECTRON_REST_MEV)
            }
            SourceKind::Betatron(s) => Some(s.electron_energy_mev),
            SourceKind::SmithPurcell(s) => Some(s.electron_energy_kev * 1e-3),
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

    /// Physical quantities the source model derives from its machine
    /// parameters, as a list of `(name, value, unit, note)` tuples.
    /// Informational only — the imaging pipeline never reads them.
    fn derived_quantities(&self) -> Vec<(String, f64, String, String)> {
        self.inner
            .derived_quantities()
            .into_iter()
            .map(|q| (q.name, q.value, q.unit, q.note))
            .collect()
    }

    /// Value of one derived quantity by name, or `None` if this source
    /// does not report it.
    fn derived_quantity(&self, name: &str) -> Option<f64> {
        self.inner
            .derived_quantities()
            .into_iter()
            .find(|q| q.name == name)
            .map(|q| q.value)
    }

    /// Dose-limited wafer throughput (simplified scanner model, 🔶) from
    /// this source's `average_power_w`. `preset` selects ILLUSTRATIVE
    /// scanner assumptions: "euv" (default; 10 Mo/Si mirrors at R = 0.70
    /// between IF and wafer, mask 0.65, 300 mm wafer, 26 x 33 mm fields,
    /// 2 mm slit, 0.1 s/field + 10 s/wafer overheads), "beuv" (La/B mirrors
    /// and mask at 0.641) or "refractive" (optics 0.30, mask 0.90, 8 mm
    /// slit). Any keyword given overrides the preset; `n_mirrors` /
    /// `mirror_reflectivity` set `optics_transmission = R**n` (the other one
    /// from the preset's mirror train) unless `optics_transmission` is given.
    /// With `cd_nm`, also returns the photons per CD² square and their
    /// relative shot noise. Returns `None` if the source reports no average
    /// power (e.g. bending magnets). Not vendor data.
    #[pyo3(signature = (
        dose_mj_cm2=30.0,
        optics_transmission=None,
        n_mirrors=None,
        mirror_reflectivity=None,
        mask_efficiency=None,
        wafer_diameter_mm=None,
        field_width_mm=None,
        field_height_mm=None,
        slit_height_mm=None,
        fields_per_wafer=None,
        max_scan_speed_mm_s=None,
        field_overhead_s=None,
        wafer_overhead_s=None,
        cd_nm=None,
        preset="euv",
    ))]
    #[allow(clippy::too_many_arguments)]
    fn wafer_throughput(
        &self,
        py: Python<'_>,
        dose_mj_cm2: f64,
        optics_transmission: Option<f64>,
        n_mirrors: Option<u32>,
        mirror_reflectivity: Option<f64>,
        mask_efficiency: Option<f64>,
        wafer_diameter_mm: Option<f64>,
        field_width_mm: Option<f64>,
        field_height_mm: Option<f64>,
        slit_height_mm: Option<f64>,
        fields_per_wafer: Option<usize>,
        max_scan_speed_mm_s: Option<f64>,
        field_overhead_s: Option<f64>,
        wafer_overhead_s: Option<f64>,
        cd_nm: Option<f64>,
        preset: &str,
    ) -> PyResult<Option<Py<pyo3::types::PyDict>>> {
        use pyo3::types::PyDict;
        if !(dose_mj_cm2.is_finite() && dose_mj_cm2 > 0.0) {
            return Err(src_value_error(format!(
                "dose_mj_cm2 must be positive, got {dose_mj_cm2}"
            )));
        }
        let (base, mirror_train) = match preset {
            "euv" | "euv_hvm" => (
                ThroughputParams::euv_hvm_like(dose_mj_cm2),
                Some((10, 0.70)),
            ),
            "beuv" | "beuv_la_b" => (
                ThroughputParams::beuv_la_b_like(dose_mj_cm2),
                Some((10, 0.641)),
            ),
            "refractive" => (ThroughputParams::refractive_like(dose_mj_cm2), None),
            other => {
                return Err(src_value_error(format!(
                    "unknown throughput preset '{other}' (expected euv / beuv / refractive)"
                )))
            }
        };
        let transmission = match (optics_transmission, n_mirrors, mirror_reflectivity) {
            (Some(t), _, _) => t,
            (None, None, None) => base.optics_transmission,
            (None, n, r) => {
                let (n0, r0) = mirror_train.ok_or_else(|| {
                    src_value_error(
                        "the refractive preset has no mirror train: give optics_transmission"
                            .to_string(),
                    )
                })?;
                ThroughputParams::mirror_train(n.unwrap_or(n0), r.unwrap_or(r0))
            }
        };
        let params = ThroughputParams {
            optics_transmission: transmission,
            mask_efficiency: mask_efficiency.unwrap_or(base.mask_efficiency),
            dose_mj_cm2,
            wafer_diameter_mm: wafer_diameter_mm.unwrap_or(base.wafer_diameter_mm),
            field_width_mm: field_width_mm.unwrap_or(base.field_width_mm),
            field_height_mm: field_height_mm.unwrap_or(base.field_height_mm),
            slit_height_mm: slit_height_mm.unwrap_or(base.slit_height_mm),
            fields_per_wafer: fields_per_wafer.or(base.fields_per_wafer),
            max_scan_speed_mm_s: max_scan_speed_mm_s.or(base.max_scan_speed_mm_s),
            field_overhead_s: field_overhead_s.unwrap_or(base.field_overhead_s),
            wafer_overhead_s: wafer_overhead_s.unwrap_or(base.wafer_overhead_s),
        };
        for (name, v) in [
            ("optics_transmission", params.optics_transmission),
            ("mask_efficiency", params.mask_efficiency),
        ] {
            if !(0.0..=1.0).contains(&v) {
                return Err(src_value_error(format!(
                    "{name} must be in [0, 1], got {v}"
                )));
            }
        }
        let Some(r) = throughput::wafer_throughput(&self.inner, &params) else {
            return Ok(None);
        };
        let d = PyDict::new(py);
        d.set_item("preset", preset)?;
        d.set_item("source_power_w", r.source_power_w)?;
        d.set_item("power_at_wafer_w", r.power_at_wafer_w)?;
        d.set_item("optics_transmission", params.optics_transmission)?;
        d.set_item("mask_efficiency", params.mask_efficiency)?;
        d.set_item("dose_mj_cm2", dose_mj_cm2)?;
        d.set_item("fields_per_wafer", r.fields_per_wafer)?;
        d.set_item("exposed_area_cm2", r.exposed_area_cm2)?;
        d.set_item(
            "dose_limited_scan_speed_mm_s",
            r.dose_limited_scan_speed_mm_s,
        )?;
        d.set_item("scan_speed_mm_s", r.scan_speed_mm_s)?;
        d.set_item("stage_limited", r.stage_limited)?;
        d.set_item("exposure_time_per_wafer_s", r.exposure_time_per_wafer_s)?;
        d.set_item("total_time_per_wafer_s", r.total_time_per_wafer_s)?;
        d.set_item("wafers_per_hour", r.wafers_per_hour)?;
        d.set_item("pulses_per_point", r.pulses_per_point)?;
        if let Some(cd) = cd_nm {
            let wl = self.inner.wavelength_nm();
            d.set_item(
                "photons_per_cd_square",
                throughput::photons_per_square(dose_mj_cm2, wl, cd),
            )?;
            d.set_item(
                "relative_shot_noise",
                throughput::relative_shot_noise(dose_mj_cm2, wl, cd),
            )?;
        }
        Ok(Some(d.unbind()))
    }

    /// Relative photon spectrum `[(E_keV, photon fraction), ...]` (sums to 1)
    /// for broadband X-ray sources: the X-ray tube's filtered spectrum on
    /// `n_bins` equal bins over [1 keV, kVp], or the betatron's
    /// synchrotron-like spectrum on `n_bins` log bins over [0.1, 8] E_c.
    /// `None` for other source families.
    #[pyo3(signature = (n_bins=200))]
    fn xray_spectrum(&self, n_bins: usize) -> PyResult<Option<Vec<(f64, f64)>>> {
        if n_bins == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "n_bins must be >= 1",
            ));
        }
        let bins = match &self.inner {
            SourceKind::XrayTube(s) => s.binned_photon_rate(n_bins),
            SourceKind::Betatron(s) => s.photon_spectrum(n_bins),
            _ => return Ok(None),
        };
        let total: f64 = bins.iter().map(|(_, n)| n).sum();
        Ok(Some(
            bins.into_iter()
                .map(|(e, n)| (e, if total > 0.0 { n / total } else { 0.0 }))
                .collect(),
        ))
    }

    /// Absolute spectral photon flux density `[(E_keV, photons/s/mm^2/keV),
    /// ...]` at `distance_mm` (X-ray tube: isotropic point source; betatron:
    /// flat-top cone of half-angle K/gamma, averaged over the repetition
    /// rate). `None` for other source families.
    #[pyo3(signature = (distance_mm, n_bins=200))]
    fn spectral_flux_density(
        &self,
        distance_mm: f64,
        n_bins: usize,
    ) -> PyResult<Option<Vec<(f64, f64)>>> {
        if !(distance_mm.is_finite() && distance_mm > 0.0) {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "distance_mm must be positive, got {}",
                distance_mm
            )));
        }
        if n_bins == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "n_bins must be >= 1",
            ));
        }
        Ok(match &self.inner {
            SourceKind::XrayTube(s) => Some(s.spectral_flux_density(distance_mm, n_bins)),
            SourceKind::Betatron(s) => Some(s.spectral_flux_density(distance_mm, n_bins)),
            _ => None,
        })
    }

    // --- P1: pupil fill, spectrum and photon bookkeeping -------------------

    /// Illumination (pupil-fill) shape as a flat tuple, the same form
    /// `with_illumination` accepts: `("conventional", sigma)`,
    /// `("annular", sigma_inner, sigma_outer)`, `("dipole", sigma_center,
    /// sigma_radius, orientation_deg)`, `("quadrupole", sigma_center,
    /// sigma_radius, opening_angle_deg)` or `("coherent_gaussian", sigma)`.
    #[getter]
    fn illumination<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, pyo3::types::PyTuple>> {
        let (name, values) = illumination_parts(self.inner.illumination());
        let mut items: Vec<Bound<'py, PyAny>> = vec![name.into_pyobject(py)?.into_any()];
        for v in values {
            items.push(v.into_pyobject(py)?.into_any());
        }
        pyo3::types::PyTuple::new(py, items)
    }

    /// A copy of this source with its pupil fill replaced, e.g.
    /// `src.with_illumination("annular", 0.5, 0.8)` or
    /// `src.with_illumination(("dipole", 0.7, 0.15, 0.0))`. Shapes and
    /// parameters as in `illumination`; sigmas are in units of NA/λ and must
    /// keep the fill inside sigma <= 1. Only the pupil the imaging engine
    /// samples changes: spectrum, power and the coherence bookkeeping of
    /// `derived_quantities()` are the source model's.
    #[pyo3(signature = (shape, *params))]
    fn with_illumination(
        &self,
        shape: &Bound<'_, PyAny>,
        params: &Bound<'_, pyo3::types::PyTuple>,
    ) -> PyResult<Self> {
        let shape = if let Ok(spec) = shape.downcast::<pyo3::types::PyTuple>() {
            if !params.is_empty() {
                return Err(src_value_error(
                    "give either one illumination tuple or a shape name followed by numbers"
                        .to_string(),
                ));
            }
            parse_illumination_tuple(spec)?
        } else {
            let kind: String = shape.extract()?;
            let values: Vec<f64> = params
                .iter()
                .map(|v| v.extract::<f64>())
                .collect::<PyResult<_>>()?;
            parse_illumination(&kind, &values)?
        };
        let mut inner = self.inner.clone();
        *illumination_mut(&mut inner) = shape;
        Ok(Self { inner })
    }

    /// Pupil fill sampled on an `n × n` grid over sigma in
    /// `[-extent, extent]²` (units of NA/λ), indexed `[iy, ix]` with
    /// `sigma = numpy.linspace(-extent, extent, n)` on both axes; values are
    /// the relative source intensity the imaging engine samples.
    #[pyo3(signature = (n=101, extent=1.0))]
    fn pupil_fill<'py>(
        &self,
        py: Python<'py>,
        n: usize,
        extent: f64,
    ) -> PyResult<Bound<'py, numpy::PyArray2<f64>>> {
        if !(2..=2049).contains(&n) {
            return Err(src_value_error(format!("n must be in [2, 2049], got {n}")));
        }
        if !(extent.is_finite() && extent > 0.0) {
            return Err(src_value_error(format!(
                "extent must be positive, got {extent}"
            )));
        }
        let step = 2.0 * extent / (n - 1) as f64;
        let fill = ndarray::Array2::from_shape_fn((n, n), |(iy, ix)| {
            LithographySource::intensity_at(
                &self.inner,
                -extent + ix as f64 * step,
                -extent + iy as f64 * step,
            )
        });
        Ok(numpy::IntoPyArray::into_pyarray(fill, py))
    }

    /// Spectral samples `[(wavelength_nm, weight), ...]` the imaging engine
    /// uses for polychromatic images (weights sum to 1).
    fn spectrum(&self) -> Vec<(f64, f64)> {
        self.inner.spectral_weights()
    }

    /// Photon energy in eV at the center wavelength (hc/λ).
    #[getter]
    fn photon_energy_ev(&self) -> f64 {
        self.inner.photon_energy_ev()
    }

    /// Incident photons per nm² at 1 mJ/cm² (the stochastic shot-noise
    /// input; broadband X-ray sources use their mean photon energy).
    #[getter]
    fn photon_density_per_mj_cm2(&self) -> f64 {
        self.inner.photon_density_per_mj_cm2()
    }

    /// Pulse energy in J, or None for CW / untracked sources.
    #[getter]
    fn pulse_energy_j(&self) -> Option<f64> {
        self.inner.pulse_energy_j()
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

fn src_value_error(msg: String) -> PyErr {
    pyo3::exceptions::PyValueError::new_err(msg)
}

fn src_core_error(e: highuvlith_core::error::LithographyError) -> PyErr {
    pyo3::exceptions::PyValueError::new_err(e.to_string())
}

/// Mutable access to the pupil fill of whichever source family `kind` holds.
fn illumination_mut(kind: &mut SourceKind) -> &mut IlluminationShape {
    match kind {
        SourceKind::Vuv(s) => &mut s.illumination,
        SourceKind::LpaFel(s) => &mut s.illumination,
        SourceKind::Lpp(s) => &mut s.illumination,
        SourceKind::Synchrotron(s) => &mut s.illumination,
        SourceKind::Hhg(s) => &mut s.illumination,
        SourceKind::Xfel(s) => &mut s.illumination,
        SourceKind::Ics(s) => &mut s.illumination,
        SourceKind::Ssmb(s) => &mut s.illumination,
        SourceKind::Entangled(s) => &mut s.illumination,
        SourceKind::XrayTube(s) => &mut s.illumination,
        SourceKind::Dpp(s) => &mut s.illumination,
        SourceKind::Sxrl(s) => &mut s.illumination,
        SourceKind::Betatron(s) => &mut s.illumination,
        SourceKind::SmithPurcell(s) => &mut s.illumination,
    }
}

/// Name and parameters of an illumination shape (the flat-tuple form).
pub(crate) fn illumination_parts(shape: &IlluminationShape) -> (&'static str, Vec<f64>) {
    match *shape {
        IlluminationShape::Conventional { sigma } => ("conventional", vec![sigma]),
        IlluminationShape::Annular {
            sigma_inner,
            sigma_outer,
        } => ("annular", vec![sigma_inner, sigma_outer]),
        IlluminationShape::Dipole {
            sigma_center,
            sigma_radius,
            orientation_deg,
        } => ("dipole", vec![sigma_center, sigma_radius, orientation_deg]),
        IlluminationShape::Quadrupole {
            sigma_center,
            sigma_radius,
            opening_angle_deg,
        } => (
            "quadrupole",
            vec![sigma_center, sigma_radius, opening_angle_deg],
        ),
        IlluminationShape::CoherentGaussian { sigma } => ("coherent_gaussian", vec![sigma]),
    }
}

/// Validated illumination shape from its name and numbers: sigmas in units
/// of NA/λ with the fill inside sigma <= 1 (as the source presets require),
/// angles in degrees.
pub(crate) fn parse_illumination(kind: &str, values: &[f64]) -> PyResult<IlluminationShape> {
    let need = |n: usize| -> PyResult<()> {
        if values.len() == n {
            Ok(())
        } else {
            Err(src_value_error(format!(
                "illumination '{kind}' takes {n} numbers, got {}",
                values.len()
            )))
        }
    };
    let sigma_ok = |name: &str, v: f64, allow_zero: bool| -> PyResult<()> {
        let lower_ok = if allow_zero { v >= 0.0 } else { v > 0.0 };
        if v.is_finite() && lower_ok && v <= 1.0 {
            Ok(())
        } else {
            Err(src_value_error(format!(
                "illumination '{kind}': {name} must be in {}0, 1], got {v}",
                if allow_zero { "[" } else { "(" }
            )))
        }
    };
    let shape = match kind.to_ascii_lowercase().as_str() {
        "conventional" => {
            need(1)?;
            sigma_ok("sigma", values[0], false)?;
            IlluminationShape::Conventional { sigma: values[0] }
        }
        "annular" => {
            need(2)?;
            sigma_ok("sigma_inner", values[0], true)?;
            sigma_ok("sigma_outer", values[1], false)?;
            if values[0] >= values[1] {
                return Err(src_value_error(format!(
                    "illumination 'annular': sigma_inner ({}) must be below sigma_outer ({})",
                    values[0], values[1]
                )));
            }
            IlluminationShape::Annular {
                sigma_inner: values[0],
                sigma_outer: values[1],
            }
        }
        "dipole" | "quadrupole" => {
            need(3)?;
            sigma_ok("sigma_center", values[0], true)?;
            sigma_ok("sigma_radius", values[1], false)?;
            sigma_ok("sigma_center + sigma_radius", values[0] + values[1], false)?;
            if !values[2].is_finite() {
                return Err(src_value_error(format!(
                    "illumination '{kind}': angle must be finite, got {}",
                    values[2]
                )));
            }
            if kind.eq_ignore_ascii_case("dipole") {
                IlluminationShape::Dipole {
                    sigma_center: values[0],
                    sigma_radius: values[1],
                    orientation_deg: values[2],
                }
            } else {
                IlluminationShape::Quadrupole {
                    sigma_center: values[0],
                    sigma_radius: values[1],
                    opening_angle_deg: values[2],
                }
            }
        }
        "coherent_gaussian" | "gaussian" => {
            need(1)?;
            sigma_ok("sigma", values[0], false)?;
            IlluminationShape::CoherentGaussian { sigma: values[0] }
        }
        other => {
            return Err(src_value_error(format!(
                "unknown illumination '{other}' (use conventional, annular, dipole, \
                 quadrupole, coherent_gaussian)"
            )))
        }
    };
    Ok(shape)
}

/// Parse an illumination tuple `(name, numbers...)`.
pub(crate) fn parse_illumination_tuple(
    spec: &Bound<'_, pyo3::types::PyTuple>,
) -> PyResult<IlluminationShape> {
    let kind: String = spec
        .get_item(0)
        .map_err(|_| src_value_error("illumination tuple is empty".to_string()))?
        .extract()?;
    let values: Vec<f64> = spec
        .iter()
        .skip(1)
        .map(|v| v.extract::<f64>())
        .collect::<PyResult<_>>()?;
    parse_illumination(&kind, &values)
}

/// Type-erased optics carrier for the Python layer: refractive projection
/// lens (dry or immersion), Schwarzschild reflective objective, Fresnel zone
/// plate, or EUV scanner projection optics.
#[derive(Debug, Clone)]
pub enum PyOpticsInner {
    Refractive(ProjectionOptics),
    Schwarzschild(SchwarzschildObjective),
    ZonePlate(FresnelZonePlate),
    EuvProjection(EuvProjectionOptics),
}

impl PyOpticsInner {
    pub fn as_dyn(&self) -> &dyn OpticalSystem {
        match self {
            PyOpticsInner::Refractive(o) => o,
            PyOpticsInner::Schwarzschild(o) => o,
            PyOpticsInner::ZonePlate(o) => o,
            PyOpticsInner::EuvProjection(o) => o,
        }
    }

    pub fn kind_label(&self) -> &'static str {
        match self {
            PyOpticsInner::Refractive(_) => "refractive",
            PyOpticsInner::Schwarzschild(_) => "schwarzschild",
            PyOpticsInner::ZonePlate(_) => "zone_plate",
            PyOpticsInner::EuvProjection(_) => "euv_projection",
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
                paraxial_defocus: false,
                multilayer: None,
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

    /// Water-immersion (or other immersion-medium) refractive projection
    /// optics: `numerical_aperture = n·sin(θ_max)` may exceed 1, up to
    /// 0.95·`immersion_index`. The defocus phase is evaluated in the medium;
    /// chromatic aberration is not modeled (0 nm/pm).
    #[staticmethod]
    #[pyo3(signature = (numerical_aperture=1.35, immersion_index=1.437, reduction=4.0, flare_fraction=0.02))]
    fn immersion(
        numerical_aperture: f64,
        immersion_index: f64,
        reduction: f64,
        flare_fraction: f64,
    ) -> PyResult<Self> {
        if !(reduction.is_finite() && reduction > 0.0) {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "reduction must be positive, got {reduction}"
            )));
        }
        if !(0.0..=1.0).contains(&flare_fraction) {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "flare_fraction must be in [0, 1.0], got {flare_fraction}"
            )));
        }
        let base = ProjectionOptics::immersion(numerical_aperture, immersion_index)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: PyOpticsInner::Refractive(ProjectionOptics {
                reduction,
                flare_fraction,
                ..base
            }),
        })
    }

    /// ArF water-immersion preset ("193i"): NA 1.35, water n = 1.437, 4×.
    #[staticmethod]
    fn immersion_193i() -> Self {
        Self {
            inner: PyOpticsInner::Refractive(ProjectionOptics::immersion_193i()),
        }
    }

    /// EUV scanner projection optics: circular pupil of wafer-side
    /// `numerical_aperture` with an optional central obscuration (radius as a
    /// fraction of NA). Isotropic wafer-side model: anamorphic magnification
    /// and mask-3D effects are not modeled.
    #[staticmethod]
    #[pyo3(signature = (numerical_aperture=0.33, central_obscuration=0.0, reduction=4.0, flare=0.0))]
    fn euv_projection(
        numerical_aperture: f64,
        central_obscuration: f64,
        reduction: f64,
        flare: f64,
    ) -> PyResult<Self> {
        if !(reduction.is_finite() && reduction > 0.0) {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "reduction must be positive, got {reduction}"
            )));
        }
        if !(0.0..=1.0).contains(&flare) {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "flare must be in [0, 1.0], got {flare}"
            )));
        }
        let base = EuvProjectionOptics::new(numerical_aperture, central_obscuration)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: PyOpticsInner::EuvProjection(EuvProjectionOptics {
                reduction,
                flare,
                ..base
            }),
        })
    }

    /// 0.33-NA EUV projection optics (NXE-class, unobscured).
    #[staticmethod]
    fn euv_nxe() -> Self {
        Self {
            inner: PyOpticsInner::EuvProjection(EuvProjectionOptics::nxe_033()),
        }
    }

    /// 0.55-NA High-NA EUV projection optics (EXE-class) with a central
    /// obscuration of 0.2·NA (assumed representative value); isotropic
    /// wafer-side pupil — the anamorphic 4×/8× magnification is not modeled.
    #[staticmethod]
    fn euv_high_na() -> Self {
        Self {
            inner: PyOpticsInner::EuvProjection(EuvProjectionOptics::high_na_055()),
        }
    }

    /// Add a Zernike aberration coefficient (refractive and EUV projection optics).
    fn add_aberration(&mut self, fringe_index: usize, coefficient_waves: f64) -> PyResult<()> {
        match &mut self.inner {
            PyOpticsInner::Refractive(o) => {
                o.zernike_coefficients
                    .push((fringe_index, coefficient_waves));
                Ok(())
            }
            PyOpticsInner::EuvProjection(o) => {
                o.zernike_coefficients
                    .push((fringe_index, coefficient_waves));
                Ok(())
            }
            _ => Err(pyo3::exceptions::PyValueError::new_err(
                "Zernike aberrations are only modeled for refractive and EUV projection optics",
            )),
        }
    }

    /// Copy of these (Schwarzschild or EUV projection) optics with an
    /// angle-dependent multilayer mirror response across the pupil.
    ///
    /// `mirrors` gives one incidence-angle map per mirror as
    /// `(center_deg, tilt_deg, azimuth_deg, radial_deg)`:
    /// θ(p) = |center + tilt·(p·u) + radial·|p|²| with u at `azimuth_deg` and
    /// p the pupil coordinate normalized to NA. `coating` is "mo_si",
    /// "la_b4c" or "la_b" with `periods`, `period_nm`, `gamma` (heavy-layer
    /// fraction). The angle maps are user assumptions (real maps come from
    /// ray-tracing the design); with the default clear-field normalization
    /// the angle dependence shows up as pupil apodization/phase, not dose.
    #[pyo3(signature = (mirrors, coating="mo_si", periods=40, period_nm=6.9, gamma=0.4))]
    fn with_multilayer_pupil(
        &self,
        mirrors: Vec<(f64, f64, f64, f64)>,
        coating: &str,
        periods: usize,
        period_nm: f64,
        gamma: f64,
    ) -> PyResult<Self> {
        use highuvlith_core::materials::multilayer::MultilayerMirror;
        use highuvlith_core::optics::multilayer_pupil::{MirrorAngleMap, MultilayerPupil};
        let err = |e: highuvlith_core::error::LithographyError| {
            pyo3::exceptions::PyValueError::new_err(e.to_string())
        };
        let stack = match coating {
            "mo_si" => MultilayerMirror::mo_si(periods, period_nm, gamma),
            "la_b4c" => MultilayerMirror::la_b4c(periods, period_nm, gamma),
            "la_b" => MultilayerMirror::la_b(periods, period_nm, gamma),
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "coating must be 'mo_si', 'la_b4c' or 'la_b', got '{other}'"
                )))
            }
        }
        .map_err(err)?;
        let maps = mirrors
            .into_iter()
            .map(
                |(center_deg, tilt_deg, azimuth_deg, radial_deg)| MirrorAngleMap {
                    center_deg,
                    tilt_deg,
                    azimuth_deg,
                    radial_deg,
                },
            )
            .collect();
        let pupil = MultilayerPupil::new(stack, maps).map_err(err)?;
        let mut out = self.clone();
        match &mut out.inner {
            PyOpticsInner::Schwarzschild(o) => o.multilayer = Some(pupil),
            PyOpticsInner::EuvProjection(o) => o.multilayer = Some(pupil),
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "multilayer pupils apply to reflective optics (schwarzschild, euv_projection)",
                ))
            }
        }
        Ok(out)
    }

    /// Whether an angle-dependent multilayer pupil is attached.
    #[getter]
    fn has_multilayer_pupil(&self) -> bool {
        match &self.inner {
            PyOpticsInner::Schwarzschild(o) => o.multilayer.is_some(),
            PyOpticsInner::EuvProjection(o) => o.multilayer.is_some(),
            _ => false,
        }
    }

    /// Image-space (immersion) medium index; 1.0 for dry and reflective optics.
    #[getter]
    fn immersion_index(&self) -> f64 {
        self.inner.as_dyn().immersion_index()
    }

    /// Optics family: "refractive", "schwarzschild", "zone_plate", or "euv_projection".
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

    // --- P1: pupil inspection -------------------------------------------------

    /// Central obscuration radius as a fraction of the NA (Schwarzschild
    /// `obscuration_ratio`, EUV-projection `central_obscuration`; 0 for
    /// refractive optics and zone plates).
    #[getter]
    fn central_obscuration(&self) -> f64 {
        match &self.inner {
            PyOpticsInner::Schwarzschild(o) => o.obscuration_ratio,
            PyOpticsInner::EuvProjection(o) => o.central_obscuration,
            _ => 0.0,
        }
    }

    /// Whether the paraxial defocus phase π z NA² ρ²/(n λ) replaces the exact
    /// (2π n/λ) z (1 − √(1 − (NA ρ/n)²)).
    #[getter]
    fn paraxial_defocus(&self) -> bool {
        match &self.inner {
            PyOpticsInner::Refractive(o) => o.paraxial_defocus,
            PyOpticsInner::Schwarzschild(o) => o.paraxial_defocus,
            PyOpticsInner::ZonePlate(o) => o.paraxial_defocus,
            PyOpticsInner::EuvProjection(o) => o.paraxial_defocus,
        }
    }

    /// A copy using the paraxial (`True`) or exact (`False`, default) defocus
    /// phase — the paraxial form is the legacy approximation, kept for
    /// comparison (it overstates defocus at high NA).
    #[pyo3(signature = (enabled=true))]
    fn with_paraxial_defocus(&self, enabled: bool) -> Self {
        let mut inner = self.inner.clone();
        match &mut inner {
            PyOpticsInner::Refractive(o) => o.paraxial_defocus = enabled,
            PyOpticsInner::Schwarzschild(o) => o.paraxial_defocus = enabled,
            PyOpticsInner::ZonePlate(o) => o.paraxial_defocus = enabled,
            PyOpticsInner::EuvProjection(o) => o.paraxial_defocus = enabled,
        }
        Self { inner }
    }

    /// Complex pupil function P(ρ) at `wavelength_nm` and defocus `focus_nm`
    /// on an `n × n` grid over the normalized pupil coordinates
    /// `numpy.linspace(-1, 1, n)` (units of the cutoff NA/λ; zone plates:
    /// 1/(2 Δr_N)), indexed `[iy, ix]`; zero outside the pupil. Shows the
    /// obscuration, apodization, aberration and defocus phase the imaging
    /// engine uses.
    #[pyo3(signature = (wavelength_nm, n=65, focus_nm=0.0))]
    fn pupil_map<'py>(
        &self,
        py: Python<'py>,
        wavelength_nm: f64,
        n: usize,
        focus_nm: f64,
    ) -> PyResult<Bound<'py, numpy::PyArray2<Complex64>>> {
        if !(2..=2049).contains(&n) {
            return Err(value_err(format!("n must be in [2, 2049], got {n}")));
        }
        if !(wavelength_nm.is_finite() && wavelength_nm > 0.0) {
            return Err(value_err(format!(
                "wavelength_nm must be positive, got {wavelength_nm}"
            )));
        }
        let optics = self.inner.as_dyn();
        let step = 2.0 / (n - 1) as f64;
        let map = py.allow_threads(|| {
            ndarray::Array2::from_shape_fn((n, n), |(iy, ix)| {
                optics.pupil_function(
                    -1.0 + ix as f64 * step,
                    -1.0 + iy as f64 * step,
                    focus_nm,
                    wavelength_nm,
                )
            })
        });
        Ok(numpy::IntoPyArray::into_pyarray(map, py))
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

/// Emit a Python `UserWarning` when a periodic mask is imaged on a field that
/// does not hold a whole number of its periods (`Mask::check_commensurate`):
/// the FFT then images a grating truncated at the field edge.
pub fn warn_if_incommensurate(mask: &Mask, grid: &GridConfig) -> PyResult<()> {
    if let Err(e) = mask.check_commensurate(grid) {
        let msg = std::ffi::CString::new(format!(
            "mask and grid are incommensurate, so the grating is truncated at the field edge \
             ({e}). Use mask.commensurate_grid(size, target_pixel_nm) or \
             GridConfig.commensurate(...)."
        ))
        .unwrap_or_default();
        Python::with_gil(|py| {
            let category = py.get_type::<pyo3::exceptions::PyUserWarning>();
            PyErr::warn(py, category.as_any(), &msg, 1)
        })?;
    }
    Ok(())
}

fn value_err(e: impl std::fmt::Display) -> PyErr {
    pyo3::exceptions::PyValueError::new_err(e.to_string())
}

/// A Python `complex` from a Rust complex number.
pub(crate) fn py_complex(py: Python<'_>, z: Complex64) -> Bound<'_, pyo3::types::PyComplex> {
    pyo3::types::PyComplex::from_doubles(py, z.re, z.im)
}

fn parse_orientation(s: &str) -> PyResult<LineOrientation> {
    match s.to_ascii_lowercase().as_str() {
        "vertical" | "v" => Ok(LineOrientation::Vertical),
        "horizontal" | "h" => Ok(LineOrientation::Horizontal),
        other => Err(value_err(format!(
            "orientation must be 'vertical' or 'horizontal', got '{other}'"
        ))),
    }
}

fn orientation_str(o: LineOrientation) -> &'static str {
    match o {
        LineOrientation::Vertical => "vertical",
        LineOrientation::Horizontal => "horizontal",
    }
}

/// Parse one feature dict (`{"type": "rect", "x": ..., ...}`).
fn feature_from_dict(d: &Bound<'_, pyo3::types::PyDict>) -> PyResult<MaskFeature> {
    let kind: String = d
        .get_item("type")?
        .ok_or_else(|| value_err("every feature dict needs a 'type' key"))?
        .extract()?;
    let num = |key: &str| -> PyResult<f64> {
        d.get_item(key)?
            .ok_or_else(|| value_err(format!("feature '{kind}' needs '{key}'")))?
            .extract::<f64>()
    };
    let num_or = |key: &str, default: f64| -> PyResult<f64> {
        match d.get_item(key)? {
            Some(v) => v.extract::<f64>(),
            None => Ok(default),
        }
    };
    Ok(match kind.as_str() {
        "rect" => MaskFeature::Rect {
            x: num("x")?,
            y: num("y")?,
            w: num("w")?,
            h: num("h")?,
        },
        "gray_rect" => MaskFeature::GrayRect {
            x: num("x")?,
            y: num("y")?,
            w: num("w")?,
            h: num("h")?,
            transmittance: num("transmittance")?,
        },
        "polygon" => MaskFeature::Polygon {
            vertices: d
                .get_item("vertices")?
                .ok_or_else(|| value_err("feature 'polygon' needs 'vertices'"))?
                .extract::<Vec<(f64, f64)>>()?,
        },
        "line_space" => MaskFeature::LineSpace {
            cd: num("cd")?,
            pitch: num("pitch")?,
            orientation: match d.get_item("orientation")? {
                Some(o) => parse_orientation(&o.extract::<String>()?)?,
                None => LineOrientation::Vertical,
            },
            offset: num_or("offset", 0.0)?,
        },
        "rect_array" => MaskFeature::RectArray {
            w: num("w")?,
            h: num("h")?,
            pitch_x: num("pitch_x")?,
            pitch_y: num("pitch_y")?,
            offset_x: num_or("offset_x", 0.0)?,
            offset_y: num_or("offset_y", 0.0)?,
        },
        other => {
            return Err(value_err(format!(
                "unknown feature type '{other}' (expected rect, gray_rect, polygon, line_space, \
                 rect_array)"
            )))
        }
    })
}

/// Parse one `from_features` item: a feature dict, an `(x, y, w, h)`
/// rectangle, or a polygon vertex list.
fn feature_from_item(item: &Bound<'_, PyAny>) -> PyResult<MaskFeature> {
    if let Ok(d) = item.downcast::<pyo3::types::PyDict>() {
        return feature_from_dict(d);
    }
    if let Ok(v) = item.extract::<Vec<f64>>() {
        if let [x, y, w, h] = v[..] {
            return Ok(MaskFeature::Rect { x, y, w, h });
        }
        return Err(value_err(format!(
            "a rectangle shorthand needs 4 numbers (x, y, w, h), got {}",
            v.len()
        )));
    }
    if let Ok(vertices) = item.extract::<Vec<(f64, f64)>>() {
        if vertices.len() >= 3 {
            return Ok(MaskFeature::Polygon { vertices });
        }
        return Err(value_err(format!(
            "a polygon needs at least 3 (x, y) vertices, got {}",
            vertices.len()
        )));
    }
    Err(value_err(
        "each feature must be a dict, an (x, y, w, h) rectangle, or a list of (x, y) vertices",
    ))
}

fn feature_to_dict<'py>(
    py: Python<'py>,
    f: &MaskFeature,
) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
    let d = pyo3::types::PyDict::new(py);
    match f {
        MaskFeature::Rect { x, y, w, h } => {
            d.set_item("type", "rect")?;
            d.set_item("x", x)?;
            d.set_item("y", y)?;
            d.set_item("w", w)?;
            d.set_item("h", h)?;
        }
        MaskFeature::GrayRect {
            x,
            y,
            w,
            h,
            transmittance,
        } => {
            d.set_item("type", "gray_rect")?;
            d.set_item("x", x)?;
            d.set_item("y", y)?;
            d.set_item("w", w)?;
            d.set_item("h", h)?;
            d.set_item("transmittance", transmittance)?;
        }
        MaskFeature::Polygon { vertices } => {
            d.set_item("type", "polygon")?;
            d.set_item("vertices", vertices.clone())?;
        }
        MaskFeature::LineSpace {
            cd,
            pitch,
            orientation,
            offset,
        } => {
            d.set_item("type", "line_space")?;
            d.set_item("cd", cd)?;
            d.set_item("pitch", pitch)?;
            d.set_item("orientation", orientation_str(*orientation))?;
            d.set_item("offset", offset)?;
        }
        MaskFeature::RectArray {
            w,
            h,
            pitch_x,
            pitch_y,
            offset_x,
            offset_y,
        } => {
            d.set_item("type", "rect_array")?;
            d.set_item("w", w)?;
            d.set_item("h", h)?;
            d.set_item("pitch_x", pitch_x)?;
            d.set_item("pitch_y", pitch_y)?;
            d.set_item("offset_x", offset_x)?;
            d.set_item("offset_y", offset_y)?;
        }
    }
    Ok(d)
}

#[pymethods]
impl PyMaskConfig {
    /// Line/space grating: bright field, opaque lines of width `cd_nm` on a
    /// `pitch_nm` period, one line centred at `offset_nm` (default x = 0).
    #[staticmethod]
    #[pyo3(signature = (cd_nm, pitch_nm, orientation="vertical", offset_nm=0.0))]
    fn line_space(cd_nm: f64, pitch_nm: f64, orientation: &str, offset_nm: f64) -> PyResult<Self> {
        let inner =
            Mask::line_space_with(cd_nm, pitch_nm, parse_orientation(orientation)?, offset_nm)
                .map_err(value_err)?;
        Ok(Self { inner })
    }

    /// Contact-hole array: dark field, clear square holes of side
    /// `diameter_nm` on a periodic lattice (pitch_y defaults to pitch_x).
    #[staticmethod]
    #[pyo3(signature = (diameter_nm, pitch_x_nm, pitch_y_nm=None))]
    fn contact_hole(diameter_nm: f64, pitch_x_nm: f64, pitch_y_nm: Option<f64>) -> PyResult<Self> {
        let inner = Mask::contact_hole(diameter_nm, pitch_x_nm, pitch_y_nm.unwrap_or(pitch_x_nm))
            .map_err(value_err)?;
        Ok(Self { inner })
    }

    /// Custom mask from features painted in order (later features win
    /// overlaps) — the canonical way to build an arbitrary mask. Each item is
    /// a feature dict (`{"type": "rect", "x": ..., "y": ..., "w": ...,
    /// "h": ...}`; types rect, gray_rect, polygon, line_space, rect_array —
    /// see `features()`), or a shorthand: an `(x, y, w, h)` rectangle (centre
    /// and size, nm) or a polygon given as a list of ≥ 3 `(x, y)` vertices.
    /// Features are absorber on a bright-field mask and clear on a dark-field
    /// one; `mask_type` is "binary", "att_psm" (absorber amplitude
    /// `sqrt(transmission)·exp(i·phase_deg)`) or "alt_psm".
    #[staticmethod]
    #[pyo3(signature = (features, dark_field=false, mask_type="binary", transmission=0.06, phase_deg=180.0))]
    fn from_features(
        features: Vec<Bound<'_, PyAny>>,
        dark_field: bool,
        mask_type: &str,
        transmission: f64,
        phase_deg: f64,
    ) -> PyResult<Self> {
        let mask_type = match mask_type.to_ascii_lowercase().as_str() {
            "binary" => MaskType::Binary,
            "att_psm" | "attenuated_psm" => MaskType::AttenuatedPSM {
                transmission,
                phase_deg,
            },
            "alt_psm" | "alternating_psm" => MaskType::AlternatingPSM,
            other => {
                return Err(value_err(format!(
                    "mask_type must be 'binary', 'att_psm' or 'alt_psm', got '{other}'"
                )))
            }
        };
        let features = features
            .iter()
            .map(feature_from_item)
            .collect::<PyResult<Vec<_>>>()?;
        let inner = Mask {
            mask_type,
            features,
            dark_field,
        };
        inner.validate().map_err(value_err)?;
        Ok(Self { inner })
    }

    /// Feature dicts (the inverse of `from_features`).
    fn features<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, pyo3::types::PyDict>>> {
        self.inner
            .features
            .iter()
            .map(|f| feature_to_dict(py, f))
            .collect()
    }

    #[getter]
    fn dark_field(&self) -> bool {
        self.inner.dark_field
    }

    /// Periods the square field must hold a whole number of:
    /// `(p1, p2 or None)`, or None if the mask has no periodic primitive.
    fn periodicity(&self) -> Option<(f64, Option<f64>)> {
        self.inner.periodicity()
    }

    /// Raise ValueError (with a suggested fix) unless the grid's field holds
    /// a whole number of periods of every periodic primitive.
    fn check_commensurate(&self, grid: &PyGridConfig) -> PyResult<()> {
        self.inner
            .check_commensurate(&grid.inner)
            .map_err(value_err)
    }

    /// Grid of `size` pixels commensurate with this mask, pixel as close to
    /// (and not coarser than) `target_pixel_nm` as possible.
    #[pyo3(signature = (size=256, target_pixel_nm=1.0))]
    fn commensurate_grid(&self, size: usize, target_pixel_nm: f64) -> PyResult<PyGridConfig> {
        let inner = self
            .inner
            .commensurate_grid(size, target_pixel_nm)
            .map_err(value_err)?;
        Ok(PyGridConfig { inner })
    }

    /// Area-averaged complex transmittance on `grid` (complex128, [y, x]).
    fn rasterize<'py>(
        &self,
        py: Python<'py>,
        grid: &PyGridConfig,
    ) -> Bound<'py, numpy::PyArray2<Complex64>> {
        let raster = py.allow_threads(|| self.inner.rasterize(&grid.inner));
        numpy::IntoPyArray::into_pyarray(raster, py)
    }

    /// Area-averaged intensity transmittance <|t|^2> on `grid` (float64).
    fn rasterize_intensity<'py>(
        &self,
        py: Python<'py>,
        grid: &PyGridConfig,
    ) -> Bound<'py, numpy::PyArray2<f64>> {
        let raster = py.allow_threads(|| self.inner.rasterize_intensity(&grid.inner));
        numpy::IntoPyArray::into_pyarray(raster, py)
    }

    /// Exact mask spectrum on `grid` in numpy.fft.fft2 layout and scale.
    fn spectrum<'py>(
        &self,
        py: Python<'py>,
        grid: &PyGridConfig,
    ) -> Bound<'py, numpy::PyArray2<Complex64>> {
        let spectrum = py.allow_threads(|| {
            let fft = highuvlith_core::math::fft2d::Fft2D::new();
            self.inner.spectrum(&grid.inner, &fft)
        });
        numpy::IntoPyArray::into_pyarray(spectrum, py)
    }

    /// How `spectrum` evaluates this mask on `grid`: "analytic", "mixed" or
    /// "raster".
    fn spectrum_method(&self, grid: &PyGridConfig) -> &'static str {
        match self.inner.spectrum_method(&grid.inner) {
            SpectrumMethod::Analytic => "analytic",
            SpectrumMethod::Mixed => "mixed",
            SpectrumMethod::Raster => "raster",
        }
    }

    /// Raise ValueError on invalid feature parameters.
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(value_err)
    }

    fn __repr__(&self) -> String {
        format!("MaskConfig({})", self.inner.summary())
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

/// Thin-film stack (layers top to bottom over a semi-infinite substrate,
/// below a superstrate), used by `expose_volumetric` and for reflectance /
/// standing-wave analysis with the exact characteristic-matrix method.
#[pyclass(name = "FilmStackConfig")]
#[derive(Debug, Clone)]
pub struct PyFilmStackConfig {
    pub inner: FilmStack,
}

#[pymethods]
impl PyFilmStackConfig {
    /// Without arguments: the default VUV stack (150 nm resist n = 1.65 +
    /// 0.015i on Si n = 0.88 + 2.10i, vacuum superstrate). With `layers` (a
    /// list of `(name, thickness_nm, n_real, n_imag)`, top to bottom) the
    /// stack is built from them on the given substrate (default Si at 157 nm)
    /// under a superstrate of index `superstrate_n` (1.0 = vacuum/air).
    #[new]
    #[pyo3(signature = (layers=None, substrate_n=None, substrate_k=None, superstrate_n=1.0))]
    fn new(
        layers: Option<Vec<(String, f64, f64, f64)>>,
        substrate_n: Option<f64>,
        substrate_k: Option<f64>,
        superstrate_n: f64,
    ) -> PyResult<Self> {
        let mut inner = FilmStack::default();
        if let Some(layers) = layers {
            inner.layers = layers
                .into_iter()
                .map(|(name, thickness_nm, n, k)| {
                    if !(thickness_nm.is_finite() && thickness_nm >= 0.0) {
                        return Err(value_err(format!(
                            "layer '{name}': thickness_nm must be >= 0, got {thickness_nm}"
                        )));
                    }
                    Ok(FilmLayer {
                        name,
                        thickness_nm,
                        n: Complex64::new(n, k),
                    })
                })
                .collect::<PyResult<_>>()?;
        }
        if substrate_n.is_some() || substrate_k.is_some() {
            inner.substrate = Complex64::new(
                substrate_n.unwrap_or(inner.substrate.re),
                substrate_k.unwrap_or(inner.substrate.im),
            );
        }
        if !(superstrate_n.is_finite() && superstrate_n > 0.0) {
            return Err(value_err(format!(
                "superstrate_n must be positive, got {superstrate_n}"
            )));
        }
        inner.superstrate = Complex64::new(superstrate_n, 0.0);
        Ok(Self { inner })
    }

    /// Append a layer below the existing ones (top to bottom).
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

    /// Set the (real) superstrate index, e.g. 1.437 for water immersion.
    fn set_superstrate(&mut self, n_real: f64) {
        self.inner.superstrate = Complex64::new(n_real, 0.0);
    }

    /// Layers as `(name, thickness_nm, n_real, n_imag)`, top to bottom.
    #[getter]
    fn layers(&self) -> Vec<(String, f64, f64, f64)> {
        self.inner
            .layers
            .iter()
            .map(|l| (l.name.clone(), l.thickness_nm, l.n.re, l.n.im))
            .collect()
    }

    /// Substrate index `n + ik` (complex).
    #[getter]
    fn substrate<'py>(&self, py: Python<'py>) -> Bound<'py, pyo3::types::PyComplex> {
        py_complex(py, self.inner.substrate)
    }

    /// Superstrate index `n + ik` (complex).
    #[getter]
    fn superstrate<'py>(&self, py: Python<'py>) -> Bound<'py, pyo3::types::PyComplex> {
        py_complex(py, self.inner.superstrate)
    }

    /// Power reflectance at `wavelength_nm`, incidence `angle_deg` (from the
    /// normal) and `polarization` ("te"/"s", "tm"/"p", "unpolarized").
    #[pyo3(signature = (wavelength_nm, angle_deg=0.0, polarization="unpolarized"))]
    fn reflectance(&self, wavelength_nm: f64, angle_deg: f64, polarization: &str) -> PyResult<f64> {
        let pol = crate::py_xray::parse_polarization(polarization)?;
        Ok(self
            .inner
            .reflectance_at_angle(wavelength_nm, angle_deg.to_radians(), pol))
    }

    /// Power transmittance into the substrate (1 − R − T is absorbed in the
    /// layers).
    #[pyo3(signature = (wavelength_nm, angle_deg=0.0, polarization="unpolarized"))]
    fn transmittance(
        &self,
        wavelength_nm: f64,
        angle_deg: f64,
        polarization: &str,
    ) -> PyResult<f64> {
        let pol = crate::py_xray::parse_polarization(polarization)?;
        Ok(self
            .inner
            .transmittance_at_angle(wavelength_nm, angle_deg.to_radians(), pol))
    }

    /// Complex amplitude `(r, t)` for "te" or "tm" (physics convention
    /// `n + ik`, phase reference at the top surface).
    #[pyo3(signature = (wavelength_nm, angle_deg=0.0, polarization="te"))]
    #[allow(clippy::type_complexity)]
    fn amplitude_coefficients<'py>(
        &self,
        py: Python<'py>,
        wavelength_nm: f64,
        angle_deg: f64,
        polarization: &str,
    ) -> PyResult<(
        Bound<'py, pyo3::types::PyComplex>,
        Bound<'py, pyo3::types::PyComplex>,
    )> {
        let pol = crate::py_xray::parse_polarization(polarization)?;
        let (r, t) = self
            .inner
            .amplitude_coefficients(wavelength_nm, angle_deg.to_radians(), pol)
            .map_err(value_err)?;
        Ok((py_complex(py, r), py_complex(py, t)))
    }

    /// Exact intensity |E(z)|² inside the stack (unit incident intensity)
    /// at depths `z_nm` from the stack top (positive downward): standing
    /// waves, absorption and every interface.
    #[pyo3(signature = (wavelength_nm, z_nm, angle_deg=0.0, polarization="te"))]
    fn intensity_profile<'py>(
        &self,
        py: Python<'py>,
        wavelength_nm: f64,
        z_nm: Vec<f64>,
        angle_deg: f64,
        polarization: &str,
    ) -> PyResult<Bound<'py, numpy::PyArray1<f64>>> {
        let pol = crate::py_xray::parse_polarization(polarization)?;
        let profile = py.allow_threads(|| {
            self.inner
                .intensity_profile(wavelength_nm, angle_deg.to_radians(), pol, &z_nm)
        });
        Ok(numpy::IntoPyArray::into_pyarray(profile, py))
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

    /// Grid whose square field is a whole multiple of the pitch(es), with
    /// the pixel as close to (and not coarser than) `target_pixel_nm` as
    /// possible.
    #[staticmethod]
    #[pyo3(signature = (pitch_x_nm, pitch_y_nm=None, size=256, target_pixel_nm=1.0))]
    fn commensurate(
        pitch_x_nm: f64,
        pitch_y_nm: Option<f64>,
        size: usize,
        target_pixel_nm: f64,
    ) -> PyResult<Self> {
        let inner = GridConfig::commensurate(pitch_x_nm, pitch_y_nm, size, target_pixel_nm)
            .map_err(value_err)?;
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

    /// Number of periods of `pitch_nm` in the field.
    fn periods_in_field(&self, pitch_nm: f64) -> f64 {
        self.inner.periods_in_field(pitch_nm)
    }

    /// True if the field holds a whole number of periods of `pitch_nm`.
    fn is_commensurate_with(&self, pitch_nm: f64) -> bool {
        self.inner.is_commensurate_with(pitch_nm)
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
