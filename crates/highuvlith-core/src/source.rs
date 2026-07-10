//! Illumination sources and the `LithographySource` contract.
//!
//! The whole imaging pipeline is generic over [`LithographySource`]: a
//! source only influences results through its wavelength, spectral
//! weights, pupil intensity map, bandwidth, and photon density — plus
//! the pulse/coherence metadata methods consumed by the stochastic
//! module. Concrete implementations here are [`VuvSource`] (F2/Ar2
//! excimer) and [`LpaFelSource`] (laser-plasma-accelerator FEL); the
//! bleeding-edge families (LPP, synchrotron, HHG, XFEL, ICS, SSMB,
//! entangled-photon) live in [`crate::source_models`] and are
//! re-exported from this module.
//!
//! [`SourceKind`] is the serde-tagged, type-erased carrier used by the
//! PyO3/CLI/GUI layers; the `for_each_source!` macro keeps its dispatch
//! one line per method. To add a source family: new file in
//! `source_models/`, implement the trait (reusing
//! `evaluate_illumination` / `evaluate_spectral_weights`), add one enum
//! variant + one macro arm + one `kind_label` arm + a `From` impl, then
//! wire PyO3/CLI factories (see CLAUDE.md for the full recipe).
//!
//! # Model status
//!
//! Spectral line shapes, pupil fills (including the graded
//! `CoherentGaussian`), and the pulse-energy/rep-rate/jitter metadata
//! are live. Polarization and time-domain pulse structure are not
//! modeled anywhere in the pipeline (documented-inert).

use serde::{Deserialize, Serialize};

/// Trait for any illumination source.
pub trait LithographySource: Send + Sync {
    /// Center wavelength in nm.
    fn wavelength_nm(&self) -> f64;

    /// Photon energy in eV.
    fn photon_energy_ev(&self) -> f64 {
        1239.84193 / self.wavelength_nm()
    }

    /// Spectral bandwidth FWHM in pm.
    fn bandwidth_pm(&self) -> f64;

    /// Source intensity at normalized pupil coordinate.
    fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64;

    /// Spectral sampling weights for polychromatic simulation.
    fn spectral_weights(&self) -> Vec<(f64, f64)>;

    /// Photon density at dose=1 mJ/cm^2 (photons/nm^2).
    /// Computed from wavelength: higher energy photons = fewer photons per unit dose.
    fn photon_density_per_mj_cm2(&self) -> f64 {
        let e_photon_j = 6.62607015e-34 * 2.99792458e8 / (self.wavelength_nm() * 1e-9);
        // 1 mJ/cm^2 = 10 J/m^2; convert to photons/nm^2
        10.0 / e_photon_j * 1e-18
    }

    /// Energy of a single pulse in joules, if the source is pulsed.
    /// `None` for CW sources or when the model does not track pulse energy.
    fn pulse_energy_j(&self) -> Option<f64> {
        None
    }

    /// Pulse repetition rate in Hz, if the source is pulsed.
    fn rep_rate_hz(&self) -> Option<f64> {
        None
    }

    /// Pulse duration in seconds, if the source is pulsed.
    fn pulse_duration_s(&self) -> Option<f64> {
        None
    }

    /// Time-averaged output power in watts. Derived from pulse energy and
    /// repetition rate when both are known; CW sources override directly.
    fn average_power_w(&self) -> Option<f64> {
        match (self.pulse_energy_j(), self.rep_rate_hz()) {
            (Some(e), Some(r)) => Some(e * r),
            _ => None,
        }
    }

    /// Fraction of power in the dominant transverse coherent mode, in [0, 1].
    /// 0.0 means unknown or fully incoherent (multi-mode).
    fn transverse_coherence(&self) -> f64 {
        0.0
    }

    /// Relative rms shot-to-shot pulse-energy fluctuation.
    /// Consumed by `StochasticParams::from_source` as a dose-jitter term.
    fn shot_to_shot_rms(&self) -> f64 {
        0.0
    }
}

/// Spatial coherence / illumination shape of the VUV source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IlluminationShape {
    /// Conventional circular partial coherence.
    Conventional { sigma: f64 },
    /// Annular illumination.
    Annular { sigma_inner: f64, sigma_outer: f64 },
    /// Quadrupole illumination.
    Quadrupole {
        sigma_center: f64,
        sigma_radius: f64,
        opening_angle_deg: f64,
    },
    /// Dipole illumination.
    Dipole {
        sigma_center: f64,
        sigma_radius: f64,
        orientation_deg: f64,
    },
    /// Gaussian pupil fill from a highly coherent beam: intensity falls as
    /// exp(-rho^2 / (2 sigma^2)) inside the pupil. The first graded
    /// (non-binary) illumination shape; used by high-coherence sources
    /// (FEL, HHG, ICS) via `sigma_from_coherence`.
    CoherentGaussian { sigma: f64 },
}

/// Map a transverse coherence fraction to an effective Gaussian pupil sigma.
///
/// Gaussian-Schell mode-count heuristic: a beam with coherence fraction
/// zeta carries roughly M ~ 1/zeta transverse modes, and far-field
/// divergence (hence pupil fill) grows as sqrt(M), so
/// sigma = sigma_core / sqrt(zeta), clamped to [sigma_core, 1].
/// APPROXIMATE — a bookkeeping bridge from coherence to partial-coherence
/// imaging, not a rigorous coherent-mode decomposition.
pub fn sigma_from_coherence(coherence_fraction: f64, sigma_core: f64) -> f64 {
    if coherence_fraction <= 0.0 {
        return 1.0;
    }
    (sigma_core / coherence_fraction.sqrt()).clamp(sigma_core, 1.0)
}

/// Spectral line shape of the VUV laser.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum SpectralShape {
    /// Lorentzian (typical for excimer lasers).
    #[default]
    Lorentzian,
    /// Gaussian.
    Gaussian,
    /// Tabulated measured spectrum.
    Tabulated {
        wavelengths_nm: Vec<f64>,
        intensities: Vec<f64>,
    },
}

/// VUV laser source specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VuvSource {
    /// Center wavelength in nm (e.g., 157.63 for F2, 126.0 for Ar2).
    pub wavelength_nm: f64,
    /// Spectral bandwidth FWHM in pm (~1.1 pm for F2 laser).
    pub bandwidth_pm: f64,
    /// Number of spectral sampling points for polychromatic simulation.
    pub spectral_samples: usize,
    /// Spectral line shape.
    pub spectral_shape: SpectralShape,
    /// Pulse energy in mJ.
    pub pulse_energy_mj: f64,
    /// Repetition rate in Hz.
    pub rep_rate_hz: f64,
    /// Illumination pupil shape.
    pub illumination: IlluminationShape,
}

/// Validate sigma for conventional illumination. Shared across source types.
pub(crate) fn validate_sigma(sigma: f64) -> crate::error::Result<()> {
    if sigma.is_nan() || sigma <= 0.0 || sigma > 1.0 {
        return Err(crate::error::LithographyError::InvalidParameter {
            name: "sigma",
            value: if sigma.is_nan() { f64::NAN } else { sigma },
            reason: "must be in range (0, 1]",
        });
    }
    Ok(())
}

impl VuvSource {
    /// Create an F2 excimer laser source with default parameters.
    pub fn f2_laser(sigma: f64) -> crate::error::Result<Self> {
        validate_sigma(sigma)?;
        Ok(Self {
            wavelength_nm: 157.63,
            bandwidth_pm: 1.1,
            spectral_samples: 5,
            spectral_shape: SpectralShape::Lorentzian,
            pulse_energy_mj: 10.0,
            rep_rate_hz: 4000.0,
            illumination: IlluminationShape::Conventional { sigma },
        })
    }

    /// Create an Ar2 excimer laser source.
    pub fn ar2_laser(sigma: f64) -> crate::error::Result<Self> {
        validate_sigma(sigma)?;
        Ok(Self {
            wavelength_nm: 126.0,
            bandwidth_pm: 5.0,
            spectral_samples: 7,
            spectral_shape: SpectralShape::Lorentzian,
            pulse_energy_mj: 5.0,
            rep_rate_hz: 1000.0,
            illumination: IlluminationShape::Conventional { sigma },
        })
    }

    /// Evaluate the source intensity at a given pupil coordinate (fx, fy),
    /// normalized to the cutoff frequency NA/lambda.
    ///
    /// Returns the source intensity weight (0.0 if outside the source shape).
    pub fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64 {
        evaluate_illumination(&self.illumination, fx_norm, fy_norm)
    }

    /// Generate spectral sampling points (wavelength_nm, weight) for polychromatic simulation.
    pub fn spectral_weights(&self) -> Vec<(f64, f64)> {
        evaluate_spectral_weights(
            self.wavelength_nm,
            self.bandwidth_pm,
            self.spectral_samples,
            &self.spectral_shape,
        )
    }
}

impl LithographySource for VuvSource {
    fn wavelength_nm(&self) -> f64 {
        self.wavelength_nm
    }

    fn bandwidth_pm(&self) -> f64 {
        self.bandwidth_pm
    }

    fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64 {
        // Delegate to inherent method via UFCS
        VuvSource::intensity_at(self, fx_norm, fy_norm)
    }

    fn spectral_weights(&self) -> Vec<(f64, f64)> {
        VuvSource::spectral_weights(self)
    }

    fn pulse_energy_j(&self) -> Option<f64> {
        Some(self.pulse_energy_mj * 1e-3)
    }

    fn rep_rate_hz(&self) -> Option<f64> {
        Some(self.rep_rate_hz)
    }
}

fn interpolate_linear(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    if x <= xs[0] {
        return ys[0];
    }
    if x >= xs[xs.len() - 1] {
        return ys[ys.len() - 1];
    }
    let pos = xs.partition_point(|&v| v < x);
    if pos == 0 {
        return ys[0];
    }
    let t = (x - xs[pos - 1]) / (xs[pos] - xs[pos - 1]);
    ys[pos - 1] + t * (ys[pos] - ys[pos - 1])
}

impl Default for VuvSource {
    fn default() -> Self {
        Self::f2_laser(0.7).expect("default sigma 0.7 is valid")
    }
}

/// Laser-plasma driven free electron laser (LPA-FEL) source.
///
/// Models the compact LPA-FEL architecture demonstrated at LBNL BELLA
/// (Kohrell et al., Phys. Rev. Accel. Beams, 2026): laser wakefield
/// accelerator driving an undulator to produce coherent, narrow-band
/// radiation. The initial demo at 100 MeV reached 420 nm with 1 kHz
/// bunch rate and 8+ hours of feedback-stabilized operation. The
/// planned 500 MeV upgrade targets 20-30 nm — the regime this struct
/// is primarily intended to model for EUV lithography studies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpaFelSource {
    /// Center wavelength in nm (target 20-30 nm range at 500 MeV).
    pub wavelength_nm: f64,
    /// Spectral bandwidth FWHM in pm. FEL output is much narrower
    /// than excimer; SASE ~0.1 % relative, seeded/self-seeded tighter.
    pub bandwidth_pm: f64,
    /// Electron beam kinetic energy in MeV (100 MeV baseline, 500 MeV target).
    pub electron_energy_mev: f64,
    /// Bunch repetition rate in Hz (1000 Hz for BELLA architecture).
    pub rep_rate_hz: f64,
    /// Pulse energy in µJ (FEL regime, distinct from mJ-class excimers).
    pub pulse_energy_uj: f64,
    /// Pulse duration in femtoseconds (FEL output is ultrashort).
    pub pulse_duration_fs: f64,
    /// Shot-to-shot intensity jitter as a fraction (e.g. 0.03 = 3 %).
    /// Reflects feedback-stabilized stability demonstrated at BELLA.
    pub shot_to_shot_stability: f64,
    /// Transverse coherence fraction in [0, 1]. FEL is high-coherence
    /// relative to excimer; values >0.8 are typical for seeded modes.
    pub transverse_coherence_fraction: f64,
    /// Number of spectral sampling points for polychromatic simulation.
    pub spectral_samples: usize,
    /// Spectral line shape (Gaussian is a good default for seeded FEL).
    pub spectral_shape: SpectralShape,
    /// Illumination pupil shape.
    pub illumination: IlluminationShape,
}

impl LpaFelSource {
    /// BELLA baseline: ~420 nm at 100 MeV (current demonstrated config).
    /// Retained as a reference fixture — this wavelength is NOT useful
    /// for EUV lithography; use `bella_target_25nm` for that regime.
    pub fn bella_baseline_100mev() -> crate::error::Result<Self> {
        validate_sigma(0.7)?;
        Ok(Self {
            wavelength_nm: 420.0,
            bandwidth_pm: 420.0,
            electron_energy_mev: 100.0,
            rep_rate_hz: 1000.0,
            pulse_energy_uj: 1.0,
            pulse_duration_fs: 10.0,
            shot_to_shot_stability: 0.05,
            transverse_coherence_fraction: 0.85,
            spectral_samples: 5,
            spectral_shape: SpectralShape::Gaussian,
            illumination: IlluminationShape::Conventional { sigma: 0.7 },
        })
    }

    /// BELLA 500 MeV target config: ~25 nm EUV for lithography.
    /// Represents the projected performance after the funded electron
    /// beam upgrade to 500 MeV.
    pub fn bella_target_25nm(sigma: f64) -> crate::error::Result<Self> {
        validate_sigma(sigma)?;
        Ok(Self {
            wavelength_nm: 25.0,
            bandwidth_pm: 25.0,
            electron_energy_mev: 500.0,
            rep_rate_hz: 1000.0,
            pulse_energy_uj: 5.0,
            pulse_duration_fs: 10.0,
            shot_to_shot_stability: 0.03,
            transverse_coherence_fraction: 0.9,
            spectral_samples: 5,
            spectral_shape: SpectralShape::Gaussian,
            illumination: IlluminationShape::Conventional { sigma },
        })
    }

    /// Generic LPA-FEL constructor. Wavelength must be positive and
    /// typically in the 20-30 nm range for EUV lithography studies;
    /// values outside this are accepted but flagged via validation only.
    pub fn new(wavelength_nm: f64, sigma: f64) -> crate::error::Result<Self> {
        if wavelength_nm <= 0.0 || wavelength_nm.is_nan() {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "wavelength_nm",
                value: if wavelength_nm.is_nan() {
                    f64::NAN
                } else {
                    wavelength_nm
                },
                reason: "must be > 0",
            });
        }
        validate_sigma(sigma)?;
        Ok(Self {
            wavelength_nm,
            // SASE-class 0.1% relative bandwidth: d-lambda = 1e-3 * lambda,
            // converted nm -> pm (x1e3), i.e. bandwidth_pm = wavelength_nm.
            // (An earlier version wrote `wavelength_nm * 1e-3`, silently
            // producing a 1000x too-narrow 1e-6 relative bandwidth.)
            bandwidth_pm: wavelength_nm,
            electron_energy_mev: 500.0,
            rep_rate_hz: 1000.0,
            pulse_energy_uj: 5.0,
            pulse_duration_fs: 10.0,
            shot_to_shot_stability: 0.03,
            transverse_coherence_fraction: 0.9,
            spectral_samples: 5,
            spectral_shape: SpectralShape::Gaussian,
            illumination: IlluminationShape::Conventional { sigma },
        })
    }

    /// Pupil illumination intensity (shared logic with VuvSource).
    pub fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64 {
        evaluate_illumination(&self.illumination, fx_norm, fy_norm)
    }

    /// Polychromatic spectral sampling weights.
    pub fn spectral_weights(&self) -> Vec<(f64, f64)> {
        evaluate_spectral_weights(
            self.wavelength_nm,
            self.bandwidth_pm,
            self.spectral_samples,
            &self.spectral_shape,
        )
    }
}

impl LithographySource for LpaFelSource {
    fn wavelength_nm(&self) -> f64 {
        self.wavelength_nm
    }

    fn bandwidth_pm(&self) -> f64 {
        self.bandwidth_pm
    }

    fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64 {
        LpaFelSource::intensity_at(self, fx_norm, fy_norm)
    }

    fn spectral_weights(&self) -> Vec<(f64, f64)> {
        LpaFelSource::spectral_weights(self)
    }

    fn pulse_energy_j(&self) -> Option<f64> {
        Some(self.pulse_energy_uj * 1e-6)
    }

    fn rep_rate_hz(&self) -> Option<f64> {
        Some(self.rep_rate_hz)
    }

    fn pulse_duration_s(&self) -> Option<f64> {
        Some(self.pulse_duration_fs * 1e-15)
    }

    fn transverse_coherence(&self) -> f64 {
        self.transverse_coherence_fraction
    }

    fn shot_to_shot_rms(&self) -> f64 {
        self.shot_to_shot_stability
    }
}

impl Default for LpaFelSource {
    fn default() -> Self {
        Self::bella_target_25nm(0.7).expect("default sigma 0.7 is valid")
    }
}

/// Evaluate illumination pupil intensity. Shared by all source implementations.
pub(crate) fn evaluate_illumination(shape: &IlluminationShape, fx_norm: f64, fy_norm: f64) -> f64 {
    let rho = (fx_norm * fx_norm + fy_norm * fy_norm).sqrt();
    match shape {
        IlluminationShape::Conventional { sigma } => {
            if rho <= *sigma {
                1.0
            } else {
                0.0
            }
        }
        IlluminationShape::Annular {
            sigma_inner,
            sigma_outer,
        } => {
            if rho >= *sigma_inner && rho <= *sigma_outer {
                1.0
            } else {
                0.0
            }
        }
        IlluminationShape::Quadrupole {
            sigma_center,
            sigma_radius,
            opening_angle_deg,
        } => {
            let angle = fy_norm.atan2(fx_norm).to_degrees();
            let half_open = opening_angle_deg / 2.0;
            let in_pole = |center_angle: f64| -> bool {
                let da = ((angle - center_angle + 180.0).rem_euclid(360.0)) - 180.0;
                da.abs() <= half_open
            };
            let dist_to_center = |center_angle: f64| -> f64 {
                let cx = sigma_center * center_angle.to_radians().cos();
                let cy = sigma_center * center_angle.to_radians().sin();
                ((fx_norm - cx).powi(2) + (fy_norm - cy).powi(2)).sqrt()
            };
            for &pole_angle in &[0.0, 90.0, 180.0, 270.0] {
                if in_pole(pole_angle) && dist_to_center(pole_angle) <= *sigma_radius {
                    return 1.0;
                }
            }
            0.0
        }
        IlluminationShape::Dipole {
            sigma_center,
            sigma_radius,
            orientation_deg,
        } => {
            let orient = orientation_deg.to_radians();
            for &sign in &[1.0_f64, -1.0] {
                let cx = sigma_center * (orient + sign * std::f64::consts::PI).cos();
                let cy = sigma_center * (orient + sign * std::f64::consts::PI).sin();
                let dist = ((fx_norm - cx).powi(2) + (fy_norm - cy).powi(2)).sqrt();
                if dist <= *sigma_radius {
                    return 1.0;
                }
            }
            0.0
        }
        IlluminationShape::CoherentGaussian { sigma } => {
            if rho <= 1.0 && *sigma > 0.0 {
                (-rho * rho / (2.0 * sigma * sigma)).exp()
            } else {
                0.0
            }
        }
    }
}

/// One line of a multi-line source spectrum (e.g. an HHG harmonic comb).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectralLine {
    /// Line center wavelength in nm.
    pub center_nm: f64,
    /// Line FWHM in pm.
    pub fwhm_pm: f64,
    /// Relative line intensity (any positive scale; normalized globally).
    pub relative_intensity: f64,
}

/// Sample one spectral line without normalization: (wavelength_nm, weight)
/// pairs over ±2.5×FWHM around the line center.
fn sample_line_unnormalized(
    wavelength_nm: f64,
    bandwidth_pm: f64,
    spectral_samples: usize,
    spectral_shape: &SpectralShape,
) -> Vec<(f64, f64)> {
    if spectral_samples <= 1 {
        return vec![(wavelength_nm, 1.0)];
    }

    let bw_nm = bandwidth_pm * 1e-3;
    let half_range = 2.5 * bw_nm;
    let step = 2.0 * half_range / (spectral_samples - 1) as f64;

    let mut weights: Vec<(f64, f64)> = Vec::with_capacity(spectral_samples);
    for i in 0..spectral_samples {
        let wl = wavelength_nm - half_range + i as f64 * step;
        let dw = wl - wavelength_nm;
        let w = match spectral_shape {
            SpectralShape::Lorentzian => {
                let gamma = bw_nm / 2.0;
                gamma * gamma / (dw * dw + gamma * gamma)
            }
            SpectralShape::Gaussian => {
                let sigma = bw_nm / (2.0 * (2.0_f64.ln()).sqrt());
                (-dw * dw / (2.0 * sigma * sigma)).exp()
            }
            SpectralShape::Tabulated {
                wavelengths_nm,
                intensities,
            } => interpolate_linear(wavelengths_nm, intensities, wl),
        };
        weights.push((wl, w));
    }
    weights
}

/// Normalize a set of (wavelength, weight) samples so weights sum to 1.
fn normalize_weights(mut weights: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    let total: f64 = weights.iter().map(|(_, w)| w).sum();
    if total > 0.0 {
        for (_, w) in &mut weights {
            *w /= total;
        }
    }
    weights
}

/// Evaluate spectral sampling weights for a single-line source.
/// Shared by all source implementations.
pub(crate) fn evaluate_spectral_weights(
    wavelength_nm: f64,
    bandwidth_pm: f64,
    spectral_samples: usize,
    spectral_shape: &SpectralShape,
) -> Vec<(f64, f64)> {
    normalize_weights(sample_line_unnormalized(
        wavelength_nm,
        bandwidth_pm,
        spectral_samples,
        spectral_shape,
    ))
}

/// Evaluate spectral sampling weights for a multi-line spectrum
/// (e.g. an HHG harmonic comb): each line is sampled with the given
/// shape, scaled by its relative intensity, then all samples are
/// normalized globally to sum to 1.
///
/// Caveat for imaging use: `AerialImageEngine::compute_polychromatic`
/// only shifts focus per spectral sample (the TCC and pupil are built
/// at the center wavelength), which is accurate for Δλ/λ ≪ 1 but NOT
/// for a comb spanning a wide band. Multi-line weights are therefore
/// honest for spectral bookkeeping (dose, photon energy, depth-dose
/// integration) but approximate for imaging.
pub fn evaluate_multiline_weights(
    lines: &[SpectralLine],
    samples_per_line: usize,
    spectral_shape: &SpectralShape,
) -> Vec<(f64, f64)> {
    let mut all: Vec<(f64, f64)> = Vec::new();
    for line in lines {
        if line.relative_intensity <= 0.0 {
            continue;
        }
        let samples = sample_line_unnormalized(
            line.center_nm,
            line.fwhm_pm,
            samples_per_line,
            spectral_shape,
        );
        // Scale so each line contributes proportionally to its relative
        // intensity regardless of its per-line sample sum.
        let line_total: f64 = samples.iter().map(|(_, w)| w).sum();
        if line_total <= 0.0 {
            continue;
        }
        let scale = line.relative_intensity / line_total;
        all.extend(samples.into_iter().map(|(wl, w)| (wl, w * scale)));
    }
    normalize_weights(all)
}

// Re-export the bleeding-edge source families so downstream code keeps
// the flat `highuvlith_core::source::*` namespace.
pub use crate::source_models::entangled::EntangledPhotonSource;
pub use crate::source_models::hhg::{HarmonicSelection, HhgGas, HhgSource};
pub use crate::source_models::ics::IcsSource;
pub use crate::source_models::lpp::{LppFuel, LppSource};
pub use crate::source_models::ssmb::SsmbSource;
pub use crate::source_models::synchrotron::{SynchrotronBeamline, SynchrotronSource};
pub use crate::source_models::xfel::{XfelMode, XfelSource};

/// Type-erased source wrapper. Dispatches trait methods to the held
/// concrete variant. Used by the PyO3 bindings and CLI config to carry
/// any concrete source through APIs that only need the
/// `LithographySource` interface. The serde tag is the TOML/JSON
/// `type = "..."` discriminator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SourceKind {
    Vuv(VuvSource),
    LpaFel(LpaFelSource),
    Lpp(LppSource),
    Synchrotron(SynchrotronSource),
    Hhg(HhgSource),
    Xfel(XfelSource),
    Ics(IcsSource),
    Ssmb(SsmbSource),
    Entangled(EntangledPhotonSource),
}

/// Apply the same expression to whichever concrete source a `SourceKind`
/// holds. Adding a source family = one new arm here plus the enum variant;
/// every dispatched method then picks it up automatically.
macro_rules! for_each_source {
    ($self:expr, $s:ident => $body:expr) => {
        match $self {
            SourceKind::Vuv($s) => $body,
            SourceKind::LpaFel($s) => $body,
            SourceKind::Lpp($s) => $body,
            SourceKind::Synchrotron($s) => $body,
            SourceKind::Hhg($s) => $body,
            SourceKind::Xfel($s) => $body,
            SourceKind::Ics($s) => $body,
            SourceKind::Ssmb($s) => $body,
            SourceKind::Entangled($s) => $body,
        }
    };
}

impl SourceKind {
    /// Outer sigma of the illumination pupil, if it is a conventional
    /// circular or Gaussian shape. Returns `None` for quadrupole/dipole.
    pub fn sigma_outer(&self) -> Option<f64> {
        match self.illumination() {
            IlluminationShape::Conventional { sigma } => Some(*sigma),
            IlluminationShape::Annular { sigma_outer, .. } => Some(*sigma_outer),
            IlluminationShape::CoherentGaussian { sigma } => Some(*sigma),
            _ => None,
        }
    }

    /// Short label identifying the source family (for display/logging).
    /// Matches the serde `type` tag.
    pub fn kind_label(&self) -> &'static str {
        match self {
            SourceKind::Vuv(_) => "vuv",
            SourceKind::LpaFel(_) => "lpa_fel",
            SourceKind::Lpp(_) => "lpp",
            SourceKind::Synchrotron(_) => "synchrotron",
            SourceKind::Hhg(_) => "hhg",
            SourceKind::Xfel(_) => "xfel",
            SourceKind::Ics(_) => "ics",
            SourceKind::Ssmb(_) => "ssmb",
            SourceKind::Entangled(_) => "entangled",
        }
    }

    /// Number of spectral sampling points used for polychromatic simulation.
    pub fn spectral_samples(&self) -> usize {
        for_each_source!(self, s => s.spectral_samples)
    }

    /// Illumination pupil shape of the held source.
    pub fn illumination(&self) -> &IlluminationShape {
        for_each_source!(self, s => &s.illumination)
    }
}

impl LithographySource for SourceKind {
    fn wavelength_nm(&self) -> f64 {
        for_each_source!(self, s => s.wavelength_nm())
    }

    fn bandwidth_pm(&self) -> f64 {
        for_each_source!(self, s => s.bandwidth_pm())
    }

    fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64 {
        for_each_source!(self, s => LithographySource::intensity_at(s, fx_norm, fy_norm))
    }

    fn spectral_weights(&self) -> Vec<(f64, f64)> {
        for_each_source!(self, s => LithographySource::spectral_weights(s))
    }

    fn pulse_energy_j(&self) -> Option<f64> {
        for_each_source!(self, s => s.pulse_energy_j())
    }

    fn rep_rate_hz(&self) -> Option<f64> {
        for_each_source!(self, s => LithographySource::rep_rate_hz(s))
    }

    fn pulse_duration_s(&self) -> Option<f64> {
        for_each_source!(self, s => s.pulse_duration_s())
    }

    fn average_power_w(&self) -> Option<f64> {
        for_each_source!(self, s => s.average_power_w())
    }

    fn transverse_coherence(&self) -> f64 {
        for_each_source!(self, s => s.transverse_coherence())
    }

    fn shot_to_shot_rms(&self) -> f64 {
        for_each_source!(self, s => s.shot_to_shot_rms())
    }
}

impl Default for SourceKind {
    fn default() -> Self {
        SourceKind::Vuv(VuvSource::default())
    }
}

impl From<VuvSource> for SourceKind {
    fn from(s: VuvSource) -> Self {
        SourceKind::Vuv(s)
    }
}

impl From<LpaFelSource> for SourceKind {
    fn from(s: LpaFelSource) -> Self {
        SourceKind::LpaFel(s)
    }
}

impl From<LppSource> for SourceKind {
    fn from(s: LppSource) -> Self {
        SourceKind::Lpp(s)
    }
}

impl From<SynchrotronSource> for SourceKind {
    fn from(s: SynchrotronSource) -> Self {
        SourceKind::Synchrotron(s)
    }
}

impl From<HhgSource> for SourceKind {
    fn from(s: HhgSource) -> Self {
        SourceKind::Hhg(s)
    }
}

impl From<XfelSource> for SourceKind {
    fn from(s: XfelSource) -> Self {
        SourceKind::Xfel(s)
    }
}

impl From<IcsSource> for SourceKind {
    fn from(s: IcsSource) -> Self {
        SourceKind::Ics(s)
    }
}

impl From<SsmbSource> for SourceKind {
    fn from(s: SsmbSource) -> Self {
        SourceKind::Ssmb(s)
    }
}

impl From<EntangledPhotonSource> for SourceKind {
    fn from(s: EntangledPhotonSource) -> Self {
        SourceKind::Entangled(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_conventional_source_inside() {
        let src = VuvSource::f2_laser(0.5).unwrap();
        assert_relative_eq!(src.intensity_at(0.0, 0.0), 1.0);
        assert_relative_eq!(src.intensity_at(0.3, 0.3), 1.0);
    }

    #[test]
    fn test_conventional_source_outside() {
        let src = VuvSource::f2_laser(0.5).unwrap();
        assert_relative_eq!(src.intensity_at(0.6, 0.0), 0.0);
    }

    #[test]
    fn test_spectral_weights_sum_to_one() {
        let src = VuvSource::f2_laser(0.7).unwrap();
        let weights = src.spectral_weights();
        let sum: f64 = weights.iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_spectral_weights_centered() {
        let src = VuvSource::f2_laser(0.7).unwrap();
        let weights = src.spectral_weights();
        // Center weight should be the largest
        let center_idx = weights.len() / 2;
        let center_w = weights[center_idx].1;
        for (i, (_, w)) in weights.iter().enumerate() {
            if i != center_idx {
                assert!(center_w >= *w);
            }
        }
    }

    #[test]
    fn test_f2_laser_wavelength() {
        let src = VuvSource::f2_laser(0.5).unwrap();
        assert_relative_eq!(src.wavelength_nm, 157.63, epsilon = 0.01);
    }

    #[test]
    fn test_ar2_laser_wavelength() {
        let src = VuvSource::ar2_laser(0.5).unwrap();
        assert_relative_eq!(src.wavelength_nm, 126.0, epsilon = 0.1);
    }

    #[test]
    fn test_photon_energy_ev() {
        let src = VuvSource::f2_laser(0.5).unwrap();
        let energy = src.photon_energy_ev();
        // hc/lambda = 1239.84193 / 157.63 ~ 7.866 eV
        assert_relative_eq!(energy, 1239.84193 / 157.63, epsilon = 0.01);
        assert!(
            energy > 7.8 && energy < 8.0,
            "F2 photon energy should be ~7.9 eV, got {}",
            energy
        );
    }

    #[test]
    fn test_invalid_sigma_rejected() {
        assert!(VuvSource::f2_laser(0.0).is_err());
        assert!(VuvSource::f2_laser(-1.0).is_err());
        assert!(VuvSource::f2_laser(f64::NAN).is_err());
    }

    #[test]
    fn test_lpa_fel_wavelength_in_target_range() {
        let src = LpaFelSource::bella_target_25nm(0.7).unwrap();
        assert!(
            (20.0..=30.0).contains(&src.wavelength_nm),
            "bella_target_25nm should be in 20-30 nm range, got {}",
            src.wavelength_nm
        );
    }

    #[test]
    fn test_lpa_fel_photon_energy_at_25nm() {
        let src = LpaFelSource::bella_target_25nm(0.7).unwrap();
        let energy = src.photon_energy_ev();
        assert_relative_eq!(energy, 1239.84193 / 25.0, epsilon = 0.01);
        assert!(
            (49.0..=50.0).contains(&energy),
            "25 nm photon energy should be ~49.6 eV, got {}",
            energy
        );
    }

    #[test]
    fn test_lpa_fel_spectral_weights_sum_to_one() {
        let src = LpaFelSource::bella_target_25nm(0.7).unwrap();
        let weights = src.spectral_weights();
        let sum: f64 = weights.iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_lpa_fel_narrow_bandwidth_finite() {
        // Seeded FEL regime: very tight bandwidth. Guards against the
        // normalization blowing up or producing NaN weights.
        let src = LpaFelSource {
            wavelength_nm: 25.0,
            bandwidth_pm: 0.01,
            electron_energy_mev: 500.0,
            rep_rate_hz: 1000.0,
            pulse_energy_uj: 5.0,
            pulse_duration_fs: 10.0,
            shot_to_shot_stability: 0.02,
            transverse_coherence_fraction: 0.95,
            spectral_samples: 7,
            spectral_shape: SpectralShape::Gaussian,
            illumination: IlluminationShape::Conventional { sigma: 0.7 },
        };
        let weights = src.spectral_weights();
        assert_eq!(weights.len(), 7);
        for (wl, w) in &weights {
            assert!(wl.is_finite(), "wavelength not finite: {}", wl);
            assert!(w.is_finite(), "weight not finite: {}", w);
            assert!(*w >= 0.0, "weight negative: {}", w);
        }
        let sum: f64 = weights.iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_lpa_fel_new_default_bandwidth_is_sase_class() {
        // Regression: the generic constructor's default bandwidth must be
        // the documented ~0.1% relative (25 pm at 25 nm), matching the
        // bella_target_25nm preset — not the 1000x-narrower 1e-6 an
        // earlier version produced.
        let src = LpaFelSource::new(25.0, 0.7).unwrap();
        assert_relative_eq!(src.bandwidth_pm, 25.0, epsilon = 1e-9);
        let rel = src.bandwidth_pm * 1e-3 / src.wavelength_nm;
        assert_relative_eq!(rel, 1e-3, epsilon = 1e-12);
    }

    #[test]
    fn test_lpa_fel_invalid_sigma_rejected() {
        assert!(LpaFelSource::bella_target_25nm(0.0).is_err());
        assert!(LpaFelSource::bella_target_25nm(-0.5).is_err());
        assert!(LpaFelSource::bella_target_25nm(f64::NAN).is_err());
        assert!(LpaFelSource::new(25.0, 1.5).is_err());
        assert!(LpaFelSource::new(-10.0, 0.7).is_err());
        assert!(LpaFelSource::new(f64::NAN, 0.7).is_err());
    }

    #[test]
    fn test_source_kind_trait_dispatch() {
        let vuv: SourceKind = VuvSource::f2_laser(0.7).unwrap().into();
        let fel: SourceKind = LpaFelSource::bella_target_25nm(0.7).unwrap().into();

        assert_relative_eq!(vuv.wavelength_nm(), 157.63, epsilon = 0.01);
        assert_relative_eq!(fel.wavelength_nm(), 25.0, epsilon = 0.01);

        // Both dispatch pupil evaluation through the trait
        assert_relative_eq!(vuv.intensity_at(0.0, 0.0), 1.0);
        assert_relative_eq!(fel.intensity_at(0.0, 0.0), 1.0);
        assert_relative_eq!(fel.intensity_at(0.9, 0.0), 0.0);

        // Spectral weights normalize to 1 through the trait
        let fel_sum: f64 = fel.spectral_weights().iter().map(|(_, w)| w).sum();
        assert_relative_eq!(fel_sum, 1.0, epsilon = 1e-12);

        // Labels and sigma_outer helpers
        assert_eq!(vuv.kind_label(), "vuv");
        assert_eq!(fel.kind_label(), "lpa_fel");
        assert_eq!(vuv.sigma_outer(), Some(0.7));
        assert_eq!(fel.sigma_outer(), Some(0.7));
    }

    #[test]
    fn test_pulse_metadata_live_through_trait() {
        // VUV excimer: 10 mJ at 4 kHz -> 40 W average power.
        let vuv = VuvSource::f2_laser(0.7).unwrap();
        assert_relative_eq!(vuv.pulse_energy_j().unwrap(), 0.010, epsilon = 1e-12);
        assert_relative_eq!(
            LithographySource::rep_rate_hz(&vuv).unwrap(),
            4000.0,
            epsilon = 1e-9
        );
        assert_relative_eq!(vuv.average_power_w().unwrap(), 40.0, epsilon = 1e-9);
        assert_relative_eq!(vuv.transverse_coherence(), 0.0);

        // LPA-FEL: 5 uJ at 1 kHz -> 5 mW; fs pulse; coherence + jitter live.
        let fel = LpaFelSource::bella_target_25nm(0.7).unwrap();
        assert_relative_eq!(fel.pulse_energy_j().unwrap(), 5.0e-6, epsilon = 1e-15);
        assert_relative_eq!(fel.average_power_w().unwrap(), 5.0e-3, epsilon = 1e-12);
        assert_relative_eq!(fel.pulse_duration_s().unwrap(), 10.0e-15, epsilon = 1e-24);
        assert_relative_eq!(fel.transverse_coherence(), 0.9);
        assert_relative_eq!(fel.shot_to_shot_rms(), 0.03);

        // Dispatch through SourceKind matches the concrete source.
        let kind: SourceKind = fel.clone().into();
        assert_eq!(kind.average_power_w(), fel.average_power_w());
        assert_relative_eq!(kind.shot_to_shot_rms(), 0.03);
    }

    #[test]
    fn test_coherent_gaussian_pupil() {
        let shape = IlluminationShape::CoherentGaussian { sigma: 0.2 };
        let center = evaluate_illumination(&shape, 0.0, 0.0);
        let at_sigma = evaluate_illumination(&shape, 0.2, 0.0);
        let outside_pupil = evaluate_illumination(&shape, 1.1, 0.0);
        assert_relative_eq!(center, 1.0, epsilon = 1e-12);
        // exp(-1/2) at rho = sigma
        assert_relative_eq!(at_sigma, (-0.5_f64).exp(), epsilon = 1e-12);
        assert_relative_eq!(outside_pupil, 0.0);
        // Graded: strictly decreasing with rho
        let mid = evaluate_illumination(&shape, 0.1, 0.0);
        assert!(center > mid && mid > at_sigma);
    }

    #[test]
    fn test_sigma_from_coherence_mapping() {
        // Full coherence -> the core sigma itself
        assert_relative_eq!(sigma_from_coherence(1.0, 0.05), 0.05, epsilon = 1e-12);
        // Quarter coherence -> doubled sigma
        assert_relative_eq!(sigma_from_coherence(0.25, 0.05), 0.10, epsilon = 1e-12);
        // Very low coherence clamps at 1 (fully incoherent fill)
        assert_relative_eq!(sigma_from_coherence(1e-6, 0.05), 1.0, epsilon = 1e-12);
        // Zero/negative coherence means unknown -> incoherent fill
        assert_relative_eq!(sigma_from_coherence(0.0, 0.05), 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_multiline_weights_normalize_and_scale() {
        let lines = vec![
            SpectralLine {
                center_nm: 30.0,
                fwhm_pm: 10.0,
                relative_intensity: 3.0,
            },
            SpectralLine {
                center_nm: 28.0,
                fwhm_pm: 10.0,
                relative_intensity: 1.0,
            },
        ];
        let weights = evaluate_multiline_weights(&lines, 5, &SpectralShape::Gaussian);
        assert_eq!(weights.len(), 10);

        let sum: f64 = weights.iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-12);

        // Line at 30 nm carries 3x the integrated weight of the line at 28 nm.
        let w30: f64 = weights
            .iter()
            .filter(|(wl, _)| (*wl - 30.0).abs() < 1.0)
            .map(|(_, w)| w)
            .sum();
        let w28: f64 = weights
            .iter()
            .filter(|(wl, _)| (*wl - 28.0).abs() < 1.0)
            .map(|(_, w)| w)
            .sum();
        assert_relative_eq!(w30 / w28, 3.0, epsilon = 1e-9);
    }

    #[test]
    fn test_multiline_skips_nonpositive_lines() {
        let lines = vec![
            SpectralLine {
                center_nm: 30.0,
                fwhm_pm: 10.0,
                relative_intensity: 1.0,
            },
            SpectralLine {
                center_nm: 28.0,
                fwhm_pm: 10.0,
                relative_intensity: 0.0,
            },
        ];
        let weights = evaluate_multiline_weights(&lines, 5, &SpectralShape::Gaussian);
        assert_eq!(weights.len(), 5);
        assert!(weights.iter().all(|(wl, _)| (*wl - 30.0).abs() < 1.0));
    }

    #[test]
    fn test_source_kind_toml_roundtrip() {
        let fel: SourceKind = LpaFelSource::bella_target_25nm(0.6).unwrap().into();
        let toml_str = toml::to_string(&fel).unwrap();
        assert!(
            toml_str.contains("type = \"lpa_fel\""),
            "serialized TOML missing tag: {}",
            toml_str
        );
        let parsed: SourceKind = toml::from_str(&toml_str).unwrap();
        match parsed {
            SourceKind::LpaFel(s) => {
                assert_relative_eq!(s.wavelength_nm, 25.0, epsilon = 0.01);
                assert_relative_eq!(s.electron_energy_mev, 500.0, epsilon = 0.01);
            }
            _ => panic!("expected LpaFel variant"),
        }

        let vuv: SourceKind = VuvSource::f2_laser(0.7).unwrap().into();
        let vuv_toml = toml::to_string(&vuv).unwrap();
        assert!(
            vuv_toml.contains("type = \"vuv\""),
            "VUV TOML missing tag: {}",
            vuv_toml
        );
        let reparsed: SourceKind = toml::from_str(&vuv_toml).unwrap();
        assert!(matches!(reparsed, SourceKind::Vuv(_)));
    }

    #[test]
    fn test_new_source_families_toml_roundtrip() {
        use crate::source_models::hhg::HhgSource;
        use crate::source_models::ics::IcsSource;
        use crate::source_models::lpp::LppSource;
        use crate::source_models::ssmb::SsmbSource;
        use crate::source_models::synchrotron::SynchrotronSource;
        use crate::source_models::xfel::XfelSource;

        let kinds: Vec<(SourceKind, &str)> = vec![
            (LppSource::sn_13nm5(0.9).unwrap().into(), "lpp"),
            (
                SynchrotronSource::compact_euv_undulator().unwrap().into(),
                "synchrotron",
            ),
            (
                SynchrotronSource::liga_bending_magnet().into(),
                "synchrotron",
            ),
            (HhgSource::ar_800nm_30nm().unwrap().into(), "hhg"),
            (XfelSource::flash_13nm5().into(), "xfel"),
            (IcsSource::compact_euv_13nm5().unwrap().into(), "ics"),
            (SsmbSource::euv_1kw_13nm5().unwrap().into(), "ssmb"),
            (
                EntangledPhotonSource::noon(157.63, 2, 1.0).unwrap().into(),
                "entangled",
            ),
        ];

        for (kind, tag) in kinds {
            let toml_str = toml::to_string(&kind).unwrap();
            assert!(
                toml_str.contains(&format!("type = \"{tag}\"")),
                "serialized TOML missing tag {tag}: {toml_str}"
            );
            let parsed: SourceKind = toml::from_str(&toml_str).unwrap();
            assert_eq!(parsed.kind_label(), tag);
            // Wavelength survives the round trip (derived or stored).
            assert_relative_eq!(parsed.wavelength_nm(), kind.wavelength_nm(), epsilon = 1e-9);
            // Every family satisfies the trait invariants.
            let sum: f64 = parsed.spectral_weights().iter().map(|(_, w)| w).sum();
            assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
        }
    }
}
