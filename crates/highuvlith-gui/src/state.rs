use highuvlith_core::aerial::AerialImageEngine;
use highuvlith_core::mask::Mask;
use highuvlith_core::metrics;
use highuvlith_core::optics::schwarzschild::SchwarzschildObjective;
use highuvlith_core::optics::{OpticalSystem, ProjectionOptics};
use highuvlith_core::source::{
    HhgGas, HhgSource, IlluminationShape, LithographySource, LpaFelSource, LppSource, SourceKind,
    SynchrotronSource, VuvSource, XfelSource,
};
use highuvlith_core::source_models::physics;
use highuvlith_core::types::GridConfig;
use ndarray::Array2;
use std::sync::{Arc, Mutex};
use std::thread;

/// Selects which concrete source type the GUI builds for simulation.
/// (ICS/SSMB/entangled are deliberately CLI/Python-only to keep the
/// panel usable.)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceType {
    Vuv,
    LpaFel,
    Lpp,
    Synchrotron,
    Hhg,
    Xfel,
}

#[derive(Clone)]
pub struct SimParams {
    pub source_type: SourceType,
    pub wavelength_nm: f64,
    pub sigma: f64,
    pub na: f64,
    pub cd_nm: f64,
    pub pitch_nm: f64,
    pub focus_nm: f64,
    pub grid_size: usize,
    /// Electron beam energy in MeV. Used only when `source_type == LpaFel`.
    pub electron_energy_mev: f64,
    /// Pulse duration in fs. Used only when `source_type == LpaFel`.
    pub pulse_duration_fs: f64,
    /// LPP fuel selector: false = Sn (13.5 nm), true = Gd (6.7 nm).
    pub lpp_fuel_gd: bool,
    /// Synchrotron ring energy in GeV (undulator beamline).
    pub electron_energy_gev: f64,
    /// Undulator period in mm.
    pub undulator_period_mm: f64,
    /// Undulator strength parameter K.
    pub undulator_k: f64,
    /// HHG harmonic order (odd).
    pub hhg_harmonic: usize,
    /// XFEL mode: false = SASE, true = seeded.
    pub xfel_seeded: bool,
}

impl Default for SimParams {
    fn default() -> Self {
        Self {
            source_type: SourceType::Vuv,
            wavelength_nm: 157.63,
            sigma: 0.7,
            na: 0.75,
            cd_nm: 65.0,
            pitch_nm: 180.0,
            focus_nm: 0.0,
            grid_size: 128,
            electron_energy_mev: 500.0,
            pulse_duration_fs: 10.0,
            lpp_fuel_gd: false,
            electron_energy_gev: 0.538,
            undulator_period_mm: 20.0,
            undulator_k: 1.0,
            hhg_harmonic: 59,
            xfel_seeded: false,
        }
    }
}

impl SimParams {
    /// Wavelength the simulation will actually use: the slider value for
    /// set-point sources, or the machine-DERIVED value for synchrotron
    /// undulators (resonance condition) and HHG (driver / harmonic).
    pub fn effective_wavelength_nm(&self) -> f64 {
        match self.source_type {
            SourceType::Synchrotron => {
                let gamma = physics::gamma_from_mev(self.electron_energy_gev * 1000.0);
                physics::undulator_resonance_nm(
                    self.undulator_period_mm,
                    self.undulator_k,
                    gamma,
                    1,
                )
            }
            SourceType::Hhg => 800.0 / self.hhg_harmonic as f64,
            _ => self.wavelength_nm,
        }
    }

    /// True when the wavelength is derived from machine parameters
    /// rather than set directly.
    pub fn wavelength_is_derived(&self) -> bool {
        matches!(self.source_type, SourceType::Synchrotron | SourceType::Hhg)
    }
}

pub struct SimResult {
    pub intensity: Array2<f64>,
    pub contrast: f64,
    pub i_max: f64,
    pub i_min: f64,
    pub num_kernels: usize,
    pub compute_ms: f64,
    pub cross_section_x: Vec<f64>,
    pub cross_section_i: Vec<f64>,
}

pub struct SimState {
    pub params: SimParams,
    pub result: Arc<Mutex<Option<SimResult>>>,
    pub computing: Arc<Mutex<bool>>,
    pub error_message: Arc<Mutex<Option<String>>>,
    dirty: bool,
}

impl SimState {
    pub fn new() -> Self {
        Self {
            params: SimParams::default(),
            result: Arc::new(Mutex::new(None)),
            computing: Arc::new(Mutex::new(false)),
            error_message: Arc::new(Mutex::new(None)),
            dirty: true,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn try_compute(&mut self) {
        if !self.dirty {
            return;
        }
        let is_computing = *self.computing.lock().unwrap();
        if is_computing {
            return;
        }
        self.dirty = false;
        *self.computing.lock().unwrap() = true;

        let params = self.params.clone();
        let result_ref = Arc::clone(&self.result);
        let computing_ref = Arc::clone(&self.computing);
        let error_ref = Arc::clone(&self.error_message);

        // Clear previous error when starting a new computation
        *error_ref.lock().unwrap() = None;

        thread::spawn(move || {
            macro_rules! fail {
                ($msg:expr) => {{
                    *error_ref.lock().unwrap() = Some($msg);
                    *computing_ref.lock().unwrap() = false;
                    return;
                }};
            }

            let source: SourceKind = match params.source_type {
                SourceType::Vuv => SourceKind::Vuv(VuvSource {
                    wavelength_nm: params.wavelength_nm,
                    bandwidth_pm: 1.1,
                    spectral_samples: 1,
                    spectral_shape: highuvlith_core::source::SpectralShape::Lorentzian,
                    pulse_energy_mj: 10.0,
                    rep_rate_hz: 4000.0,
                    illumination: IlluminationShape::Conventional {
                        sigma: params.sigma,
                    },
                }),
                SourceType::LpaFel => {
                    let mut fel = match LpaFelSource::new(params.wavelength_nm, params.sigma) {
                        Ok(s) => s,
                        Err(e) => fail!(format!("LPA-FEL parameter error: {}", e)),
                    };
                    fel.electron_energy_mev = params.electron_energy_mev;
                    fel.pulse_duration_fs = params.pulse_duration_fs;
                    fel.spectral_samples = 1;
                    SourceKind::LpaFel(fel)
                }
                SourceType::Lpp => {
                    let preset = if params.lpp_fuel_gd {
                        LppSource::gd_6nm7(params.sigma)
                    } else {
                        LppSource::sn_13nm5(params.sigma)
                    };
                    let mut lpp = match preset {
                        Ok(s) => s,
                        Err(e) => fail!(format!("LPP parameter error: {}", e)),
                    };
                    lpp.spectral_samples = 1;
                    SourceKind::Lpp(lpp)
                }
                SourceType::Synchrotron => {
                    match SynchrotronSource::undulator(
                        params.electron_energy_gev,
                        params.undulator_period_mm,
                        params.undulator_k,
                        100,
                        1,
                    ) {
                        Ok(mut s) => {
                            s.spectral_samples = 1;
                            SourceKind::Synchrotron(s)
                        }
                        Err(e) => fail!(format!("Synchrotron parameter error: {}", e)),
                    }
                }
                SourceType::Hhg => {
                    match HhgSource::new(800.0, HhgGas::Neon, 4e14, params.hhg_harmonic, 15.0) {
                        Ok(mut s) => {
                            s.spectral_samples = 1;
                            SourceKind::Hhg(s)
                        }
                        Err(e) => fail!(format!("HHG parameter error: {}", e)),
                    }
                }
                SourceType::Xfel => {
                    let mut xfel = if params.xfel_seeded {
                        XfelSource::fermi_seeded_13nm5()
                    } else {
                        XfelSource::flash_13nm5()
                    };
                    xfel.wavelength_nm = params.wavelength_nm;
                    xfel.spectral_samples = 1;
                    SourceKind::Xfel(xfel)
                }
            };

            // Optics honesty: no refractive lens material exists below
            // ~110 nm, so auto-select a Schwarzschild reflective
            // objective for short wavelengths.
            let wavelength = source.wavelength_nm();
            let optics: Box<dyn OpticalSystem> = if wavelength < 50.0 {
                let mut objective = if wavelength < 10.0 {
                    SchwarzschildObjective::beuv()
                } else {
                    SchwarzschildObjective::euv_standard()
                };
                objective.numerical_aperture = params.na.min(0.6);
                Box::new(objective)
            } else {
                match ProjectionOptics::new(params.na) {
                    Ok(o) => Box::new(o),
                    Err(e) => fail!(format!("Optics error: {}", e)),
                }
            };
            let grid = GridConfig {
                size: params.grid_size,
                pixel_nm: 2.0,
            };
            let mask = match Mask::line_space(params.cd_nm, params.pitch_nm) {
                Ok(m) => m,
                Err(e) => {
                    *error_ref.lock().unwrap() = Some(format!("Mask error: {}", e));
                    *computing_ref.lock().unwrap() = false;
                    return;
                }
            };

            let start = std::time::Instant::now();
            match AerialImageEngine::new(&source, optics.as_ref(), grid, 15) {
                Ok(engine) => {
                    let aerial = engine.compute(&mask, params.focus_nm);
                    let elapsed = start.elapsed();

                    let contrast = metrics::image_contrast(&aerial.data);
                    let i_max = aerial
                        .data
                        .iter()
                        .cloned()
                        .fold(f64::NEG_INFINITY, f64::max);
                    let i_min = aerial.data.iter().cloned().fold(f64::INFINITY, f64::min);

                    let n = aerial.data.ncols();
                    let center_row = aerial.data.nrows() / 2;
                    let field = params.grid_size as f64 * 2.0;
                    let half = field / 2.0;
                    let pixel = field / n as f64;
                    let cross_section_x: Vec<f64> =
                        (0..n).map(|j| -half + (j as f64 + 0.5) * pixel).collect();
                    let cross_section_i: Vec<f64> =
                        (0..n).map(|j| aerial.data[[center_row, j]]).collect();

                    let sim_result = SimResult {
                        intensity: aerial.data,
                        contrast,
                        i_max,
                        i_min,
                        num_kernels: engine.num_kernels(),
                        compute_ms: elapsed.as_secs_f64() * 1000.0,
                        cross_section_x,
                        cross_section_i,
                    };
                    *result_ref.lock().unwrap() = Some(sim_result);
                }
                Err(e) => {
                    *error_ref.lock().unwrap() = Some(format!("Engine error: {}", e));
                }
            }
            *computing_ref.lock().unwrap() = false;
        });
    }
}
