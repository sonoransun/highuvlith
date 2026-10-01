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
//! wire PyO3/CLI factories (see `docs/extending.md` for the full recipe).
//!
//! # Model status
//!
//! Spectral line shapes (Lorentzian, Gaussian, undulator sinc², tabulated),
//! pupil fills (including the graded `CoherentGaussian`), and the
//! pulse-energy/rep-rate/jitter metadata are live. Every family also
//! reports [`LithographySource::derived_quantities`] — physics derived from
//! its machine parameters (photon budgets, FEL gain lengths, coherent
//! fractions, power gaps); these are informational and never enter the
//! imaging pipeline. Polarization and time-domain pulse structure are not
//! modeled anywhere in the pipeline (documented-inert).
//!
//! `VuvSource` carries the whole optical-lithography heritage ladder: the
//! mercury-lamp g/h/i lines (NIST air wavelengths 435.8328 / 404.6563 /
//! 365.0153 nm, filtered few-nm lines, CW: `rep_rate_hz = 0`), the KrF
//! (248.3 nm) and ArF (193.368 nm) line-narrowed excimer lasers, and the
//! F₂ 157.63 nm laser — all with REPRESENTATIVE (hedged) bandwidth and
//! pulse parameters. The Ar₂ 126 nm preset is **hypothetical**: Ar₂* emits
//! a broad (~5–10 nm) second continuum, lasing was only ever demonstrated
//! with intense electron-beam pumping, and 126 nm lithography never
//! progressed beyond laboratory discussion.
//!
//! `LpaFelSource`: the wavelength is a stored set-point; when the optional
//! undulator is given, the resonance wavelength is derived and must agree
//! within 5 % ([`LpaFelSource::with_machine`]); with the optional beam
//! parameters the 1D/Ming-Xie FEL physics (Pierce parameter, gain length,
//! saturation power, energy-spread criterion) is derived, and the spectral
//! bandwidth becomes the SASE estimate `2 rho lambda` unless
//! `bandwidth_override_pm` is set. The demonstrated
//! LPA-FEL anchor is BELLA's 420 nm SASE lasing at 100 MeV and **1 Hz**;
//! the 25 nm / 500 MeV / 1 kHz preset is a design projection.

use serde::{Deserialize, Serialize};

use crate::source_models::physics;

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

    /// Time-averaged usable output power in watts — the source-side power
    /// delivered into the illuminator (for plasma sources: in-band power at
    /// intermediate focus). Derived from pulse energy and repetition rate
    /// when both are known; CW sources override directly. This is the
    /// input of the dose-limited throughput model
    /// (`source_models::throughput`).
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

    /// Physical quantities derived from the model's machine parameters
    /// (flux, gain length, coherent fraction, ...). Informational only —
    /// never consumed by the imaging pipeline. Default: none.
    fn derived_quantities(&self) -> Vec<DerivedQuantity> {
        Vec::new()
    }
}

/// A physical quantity a source model derives from its machine parameters
/// (e.g. an undulator's on-axis flux, an FEL's Pierce parameter, an ICS
/// source's Thomson-scattered photon yield), reported for inspection,
/// documentation and trade studies. Purely informational: the imaging
/// pipeline never reads these values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivedQuantity {
    /// Short snake_case identifier, e.g. `"pierce_parameter"`.
    pub name: String,
    /// Numerical value in `unit`.
    pub value: f64,
    /// Unit string, e.g. `"W"`, `"photons/s/0.1%BW"`, `"-"` for dimensionless.
    pub unit: String,
    /// One-line provenance / caveat, e.g. `"1D FEL theory; ignores emittance"`.
    pub note: String,
}

impl DerivedQuantity {
    /// Convenience constructor.
    pub fn new(name: &str, value: f64, unit: &str, note: &str) -> Self {
        Self {
            name: name.to_string(),
            value,
            unit: unit.to_string(),
            note: note.to_string(),
        }
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

/// Spectral line shape of a source (the `bandwidth_pm` it is paired with is
/// always the FWHM).
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
    /// Natural line of an N-period undulator harmonic (filament beam,
    /// on axis): `sinc^2(pi x)` with `x = SINC2_FWHM_X * d-lambda / FWHM`,
    /// i.e. `sinc^2(pi n N d-lambda / lambda)` for FWHM = 0.8859 lambda/(nN).
    /// First zeros at `d-lambda = +-lambda/(nN)`; the side lobes carry
    /// ~10 % of the line energy.
    SincSquared,
}

/// FWHM of `sinc^2(pi x)` in units of `x` (half maximum at x = ±0.44295).
pub const SINC2_FWHM_X: f64 = 0.885_892_941_378_904_7;

/// E95 / FWHM of a Gaussian line (95 % of the energy inside ±1.96 σ).
pub const E95_OVER_FWHM_GAUSSIAN: f64 = 1.664_640_139_849_238;
/// E95 / FWHM of a Lorentzian line, `tan(0.95 π / 2)` (heavy tails).
pub const E95_OVER_FWHM_LORENTZIAN: f64 = 12.706_204_736_174_705;
/// E95 / FWHM of the undulator `sinc^2` line (1/x² side-lobe tails).
pub const E95_OVER_FWHM_SINC2: f64 = 4.679_897_315_241_133;

/// E95 bandwidth in pm — the width holding the central 95 % of the line
/// energy (2.5 % cut from each tail), the convention lithography-laser
/// vendors specify — of a line of the given shape and FWHM `fwhm_pm`. For a
/// `Tabulated` spectrum it is computed from the table (quantiles of the
/// trapezoid-integrated spectrum) and `fwhm_pm` is ignored.
pub fn e95_bandwidth_pm(shape: &SpectralShape, fwhm_pm: f64) -> f64 {
    match shape {
        SpectralShape::Gaussian => E95_OVER_FWHM_GAUSSIAN * fwhm_pm,
        SpectralShape::Lorentzian => E95_OVER_FWHM_LORENTZIAN * fwhm_pm,
        SpectralShape::SincSquared => E95_OVER_FWHM_SINC2 * fwhm_pm,
        SpectralShape::Tabulated {
            wavelengths_nm,
            intensities,
        } => {
            let n = wavelengths_nm.len().min(intensities.len());
            if n < 2 {
                return 0.0;
            }
            let mut cum = vec![0.0; n];
            for i in 1..n {
                let dw = wavelengths_nm[i] - wavelengths_nm[i - 1];
                cum[i] = cum[i - 1] + 0.5 * (intensities[i] + intensities[i - 1]).max(0.0) * dw;
            }
            let total = cum[n - 1];
            if total <= 0.0 {
                return 0.0;
            }
            let quantile = |q: f64| -> f64 {
                let target = q * total;
                let j = cum.partition_point(|&c| c < target).clamp(1, n - 1);
                let (c0, c1) = (cum[j - 1], cum[j]);
                let t = if c1 > c0 {
                    (target - c0) / (c1 - c0)
                } else {
                    0.0
                };
                wavelengths_nm[j - 1] + t * (wavelengths_nm[j] - wavelengths_nm[j - 1])
            };
            (quantile(0.975) - quantile(0.025)) * 1e3
        }
    }
}

/// Mercury g-line, NIST air wavelength in nm (NIST Handbook of Basic Atomic
/// Spectroscopic Data, Hg I strong lines).
pub const HG_G_LINE_NM: f64 = 435.8328;
/// Mercury h-line, NIST air wavelength in nm.
pub const HG_H_LINE_NM: f64 = 404.6563;
/// Mercury i-line, NIST air wavelength in nm.
pub const HG_I_LINE_NM: f64 = 365.0153;
/// Nominal ArF excimer lithography wavelength in nm.
pub const ARF_WAVELENGTH_NM: f64 = 193.368;
/// Nominal KrF excimer lithography wavelength in nm.
pub const KRF_WAVELENGTH_NM: f64 = 248.3;

/// Optical (VUV / DUV / UV) source: an excimer laser or a filtered
/// mercury-lamp line. The presets span the lithography heritage ladder
/// (Hg g/h/i lines, KrF, ArF, F₂) plus the hypothetical Ar₂ laser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VuvSource {
    /// Center wavelength in nm (e.g. 157.63 for F2, 193.368 for ArF,
    /// 365.0153 for the Hg i-line).
    pub wavelength_nm: f64,
    /// Spectral bandwidth FWHM in pm (~1.1 pm for the F2 laser, ~3 nm =
    /// 3000 pm for a filtered Hg lamp line).
    pub bandwidth_pm: f64,
    /// Number of spectral sampling points for polychromatic simulation.
    pub spectral_samples: usize,
    /// Spectral line shape.
    pub spectral_shape: SpectralShape,
    /// Pulse energy in mJ (ignored for CW sources).
    pub pulse_energy_mj: f64,
    /// Repetition rate in Hz; `0` marks a CW source (mercury lamp), for
    /// which pulse metadata and `average_power_w` are `None`.
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

    /// Create a **hypothetical** line-narrowed Ar₂ excimer laser at 126 nm.
    ///
    /// Ar₂* radiates a broad (~5–10 nm FWHM) bound-free second continuum;
    /// laser action on it has only been demonstrated with intense
    /// relativistic electron-beam pumping of high-pressure argon, and 126 nm
    /// lithography never progressed beyond laboratory discussion. A 5 pm
    /// line-narrowed, kHz Ar₂ lithography laser (this preset) has never
    /// been built — treat it as a what-if (🧪), unlike the F₂ preset.
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

    /// ArF excimer laser at 193.368 nm, line-narrowed immersion-scanner
    /// class. REPRESENTATIVE values, not a vendor specification: a Gaussian
    /// line of 0.2 pm FWHM (E95 ≈ 0.33 pm; lithography lasers are specified
    /// by E95, of order 0.3 pm for ArF immersion) and 15 mJ × 6 kHz = 90 W
    /// (ArF immersion lasers span roughly 60–120 W).
    pub fn arf_laser(sigma: f64) -> crate::error::Result<Self> {
        validate_sigma(sigma)?;
        Ok(Self {
            wavelength_nm: ARF_WAVELENGTH_NM,
            bandwidth_pm: 0.2,
            spectral_samples: 5,
            spectral_shape: SpectralShape::Gaussian,
            pulse_energy_mj: 15.0,
            rep_rate_hz: 6000.0,
            illumination: IlluminationShape::Conventional { sigma },
        })
    }

    /// KrF excimer laser at ~248.3 nm, line-narrowed scanner class.
    /// REPRESENTATIVE values: a Gaussian line of 0.6 pm FWHM (E95 ≈ 1.0 pm;
    /// the first line-narrowed KrF litho lasers had few-pm bandwidths) and
    /// 10 mJ × 4 kHz = 40 W (KrF litho lasers span roughly 30–60 W).
    pub fn krf_laser(sigma: f64) -> crate::error::Result<Self> {
        validate_sigma(sigma)?;
        Ok(Self {
            wavelength_nm: KRF_WAVELENGTH_NM,
            bandwidth_pm: 0.6,
            spectral_samples: 5,
            spectral_shape: SpectralShape::Gaussian,
            pulse_energy_mj: 10.0,
            rep_rate_hz: 4000.0,
            illumination: IlluminationShape::Conventional { sigma },
        })
    }

    /// Filtered mercury-lamp line (CW, spatially incoherent): Gaussian
    /// filter passband of REPRESENTATIVE 3 nm FWHM (the i-line bandwidth
    /// quoted for i-line steppers), `rep_rate_hz = 0`. Defaults to ONE
    /// spectral sample: lamp-era lenses were colour-corrected over the
    /// filtered band, while the refractive optics model's axial-chromatic
    /// coefficient is an uncorrected CaF₂/157 nm figure — sampling a
    /// 3 nm band through it would add spurious defocus.
    fn hg_lamp_line(wavelength_nm: f64, sigma: f64) -> crate::error::Result<Self> {
        validate_sigma(sigma)?;
        Ok(Self {
            wavelength_nm,
            bandwidth_pm: 3000.0,
            spectral_samples: 1,
            spectral_shape: SpectralShape::Gaussian,
            pulse_energy_mj: 0.0,
            rep_rate_hz: 0.0,
            illumination: IlluminationShape::Conventional { sigma },
        })
    }

    /// Mercury i-line, 365.0153 nm (NIST air wavelength), filtered lamp
    /// line; see [`Self::hg_g_line`] for the conventions.
    pub fn hg_i_line(sigma: f64) -> crate::error::Result<Self> {
        Self::hg_lamp_line(HG_I_LINE_NM, sigma)
    }

    /// Mercury h-line, 404.6563 nm (NIST air wavelength), filtered lamp line.
    pub fn hg_h_line(sigma: f64) -> crate::error::Result<Self> {
        Self::hg_lamp_line(HG_H_LINE_NM, sigma)
    }

    /// Mercury g-line, 435.8328 nm (NIST air wavelength; NIST Handbook of
    /// Basic Atomic Spectroscopic Data, Hg I), isolated from a
    /// high-pressure mercury arc lamp by an interference filter: CW
    /// (`rep_rate_hz = 0`, no pulse metadata, `average_power_w() = None`),
    /// spatially incoherent, REPRESENTATIVE 3 nm FWHM passband, one
    /// spectral sample by default. Air wavelengths are used, as is
    /// conventional for lamp lines; the vacuum wavelengths are ~0.1 nm
    /// longer (a 3e-4 relative difference, immaterial here).
    pub fn hg_g_line(sigma: f64) -> crate::error::Result<Self> {
        Self::hg_lamp_line(HG_G_LINE_NM, sigma)
    }

    /// `true` for a CW source (`rep_rate_hz <= 0`, e.g. a mercury lamp).
    pub fn is_cw(&self) -> bool {
        self.rep_rate_hz <= 0.0
    }

    /// Identify the heritage line this source sits on (within 0.5 nm), for
    /// labelling derived quantities.
    pub fn line_label(&self) -> Option<&'static str> {
        let lambda = self.wavelength_nm;
        [
            (HG_G_LINE_NM, "Hg g-line (NIST air wavelength 435.8328 nm)"),
            (HG_H_LINE_NM, "Hg h-line (NIST air wavelength 404.6563 nm)"),
            (HG_I_LINE_NM, "Hg i-line (NIST air wavelength 365.0153 nm)"),
            (KRF_WAVELENGTH_NM, "KrF excimer laser"),
            (ARF_WAVELENGTH_NM, "ArF excimer laser"),
            (
                157.63,
                "F2 laser (157.63 nm line of the 157.52/157.63 nm doublet)",
            ),
            (
                126.0,
                "Ar2* second continuum (hypothetical line-narrowed laser)",
            ),
        ]
        .into_iter()
        .find(|(center, _)| (lambda - center).abs() <= 0.5)
        .map(|(_, label)| label)
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

    /// `None` for CW sources (`rep_rate_hz <= 0`).
    fn pulse_energy_j(&self) -> Option<f64> {
        (!self.is_cw()).then_some(self.pulse_energy_mj * 1e-3)
    }

    /// `None` for CW sources (`rep_rate_hz <= 0`).
    fn rep_rate_hz(&self) -> Option<f64> {
        (!self.is_cw()).then_some(self.rep_rate_hz)
    }

    /// Photon budget (per mJ, per pulse for lasers) plus spectral figures
    /// of merit, including the E95 width of the configured line shape. In
    /// the Ar₂* band (120–132 nm) the bandwidth note flags that a pm-class
    /// line-narrowed Ar₂ laser is hypothetical.
    fn derived_quantities(&self) -> Vec<DerivedQuantity> {
        let lambda = self.wavelength_nm;
        let e_ph = physics::photon_energy_j(lambda);
        let rel_bw = self.bandwidth_pm * 1e-3 / lambda;
        let in_ar2_band = (120.0..=132.0).contains(&lambda);
        let bw_note = if in_ar2_band {
            "Ar2* emits a ~5-10 nm wide second continuum; a pm-class line-narrowed \
             Ar2 laser has never been demonstrated (hypothetical)"
        } else {
            "FWHM / center wavelength"
        };
        let mut out = vec![
            DerivedQuantity::new(
                "photon_energy",
                physics::HC_EV_NM / lambda,
                "eV",
                self.line_label().unwrap_or("hc / lambda"),
            ),
            DerivedQuantity::new("relative_bandwidth", rel_bw, "-", bw_note),
            DerivedQuantity::new(
                "e95_bandwidth",
                e95_bandwidth_pm(&self.spectral_shape, self.bandwidth_pm),
                "pm",
                "width holding 95 % of the line energy for the configured shape \
                 (vendor convention; Gaussian 1.66 x FWHM, Lorentzian 12.7 x FWHM)",
            ),
            DerivedQuantity::new(
                "coherence_length",
                if self.bandwidth_pm > 0.0 {
                    lambda * lambda / (self.bandwidth_pm * 1e-3) * 1e-3
                } else {
                    f64::INFINITY
                },
                "um",
                "longitudinal coherence length lambda^2 / d-lambda",
            ),
            DerivedQuantity::new(
                "photons_per_mj",
                1e-3 / e_ph,
                "photons",
                "photons per millijoule, 1 mJ / (hc / lambda)",
            ),
            DerivedQuantity::new(
                "photon_density_per_dose",
                self.photon_density_per_mj_cm2(),
                "photons/nm^2 per mJ/cm^2",
                "incident photons per nm^2 for a 1 mJ/cm^2 dose",
            ),
            DerivedQuantity::new(
                "photons_vs_13nm5",
                lambda / 13.5,
                "-",
                "photons per unit dose relative to 13.5 nm EUV (= lambda / 13.5 nm)",
            ),
        ];
        if self.is_cw() {
            out.push(DerivedQuantity::new(
                "continuous_wave",
                1.0,
                "bool",
                "CW lamp line: no pulse metadata; lamp output power is not modeled",
            ));
        } else {
            let pulse_j = self.pulse_energy_mj * 1e-3;
            let power = pulse_j * self.rep_rate_hz;
            out.extend([
                DerivedQuantity::new(
                    "photons_per_pulse",
                    pulse_j / e_ph,
                    "photons",
                    "pulse energy / (hc / lambda)",
                ),
                DerivedQuantity::new(
                    "average_power",
                    power,
                    "W",
                    "pulse energy x repetition rate (laser output)",
                ),
                DerivedQuantity::new(
                    "photon_rate",
                    physics::watts_to_photon_rate(power, lambda),
                    "photons/s",
                    "average power / photon energy",
                ),
            ]);
        }
        if in_ar2_band {
            out.push(DerivedQuantity::new(
                "demonstrated",
                0.0,
                "bool",
                "0 = no line-narrowed Ar2 lithography laser exists; lasing on Ar2* was only \
                 shown with electron-beam pumping",
            ));
        }
        out
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
/// Models a compact FEL driven by a laser-wakefield accelerator. The
/// demonstrated anchor is LBNL BELLA (F. Kohrell et al., "Over 8 hours of
/// continuous operation of a free-electron laser driven by a laser-plasma
/// accelerator", Phys. Rev. Accel. Beams 29, 041301 (2026),
/// doi:10.1103/z2d3-bhyt): 100 MeV beams at **1 Hz** lasing (SASE) at
/// 420 nm for more than 8 h without operator input (~15,000 shots); only the
/// unamplified laser front end runs at 1 kHz. Other LPA-FEL results: 27 nm
/// lasing at ≤150 nJ/shot and ≤1 Hz (Wang et al., Nature 595, 516 (2021))
/// and seeded lasing at 269 nm (Nat. Photon. 17, 150 (2023)); none at
/// 13.5 nm. The 25 nm / 500 MeV / 1 kHz EUV preset is a DESIGN
/// PROJECTION used as a study point, not a published BELLA specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpaFelSource {
    /// Center wavelength in nm (420 nm demonstrated at 100 MeV; 20-30 nm at
    /// ~500 MeV is a projection).
    pub wavelength_nm: f64,
    /// Spectral bandwidth FWHM in pm used when no FEL estimate is available
    /// (no undulator + beam parameters) and no override is set; see
    /// [`LpaFelSource::effective_bandwidth_pm`]. [`LpaFelSource::new`]
    /// stores 0.1 % of lambda; the presets store their derived `2 rho lambda`.
    pub bandwidth_pm: f64,
    /// Electron beam kinetic energy in MeV (100 MeV demonstrated, 500 MeV
    /// projected).
    pub electron_energy_mev: f64,
    /// Bunch repetition rate in Hz (1 Hz demonstrated at BELLA; the 1 kHz of
    /// the 25 nm preset is a design projection).
    pub rep_rate_hz: f64,
    /// Pulse energy in µJ (FEL regime, distinct from mJ-class excimers).
    pub pulse_energy_uj: f64,
    /// Pulse duration in femtoseconds (FEL output is ultrashort).
    pub pulse_duration_fs: f64,
    /// Shot-to-shot intensity jitter as a fraction (e.g. 0.03 = 3 %).
    /// Representative (assumed) values; not a published BELLA figure.
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
    /// Optional undulator (period, K, number of periods). When present the
    /// on-axis resonance wavelength is DERIVED from `electron_energy_mev`
    /// and cross-checked against `wavelength_nm` (5 % rule, see
    /// [`LpaFelSource::with_machine`]). `None` in configs written before
    /// the field existed.
    #[serde(default)]
    pub undulator: Option<physics::UndulatorParams>,
    /// Optional electron-beam parameters in the undulator (peak current,
    /// normalized emittance, energy spread, beta function). Together with
    /// `undulator` they drive the derived 1D / Ming-Xie FEL physics.
    #[serde(default)]
    pub electron_beam: Option<physics::ElectronBeamParams>,
    /// Explicit bandwidth FWHM in pm. When set it always wins; when `None`
    /// the bandwidth is the SASE estimate `2 rho lambda` whenever undulator
    /// + beam parameters give an FEL estimate, else `bandwidth_pm`.
    #[serde(default)]
    pub bandwidth_override_pm: Option<f64>,
}

/// Illustrative LPA-class machine for the BELLA presets: a 20 mm-period
/// undulator of 200 periods (4 m) with K set by the resonance condition for
/// the preset wavelength, and a kA-class, sub-µm-emittance, percent-level
/// energy-spread beam typical of laser-wakefield accelerators. These are
/// ASSUMED order-of-magnitude values, not BELLA machine specifications.
fn illustrative_lpa_machine(
    electron_energy_mev: f64,
    wavelength_nm: f64,
) -> Option<(physics::UndulatorParams, physics::ElectronBeamParams)> {
    const PERIOD_MM: f64 = 20.0;
    let gamma = physics::gamma_from_mev(electron_energy_mev);
    let k = physics::undulator_k_for_wavelength(PERIOD_MM, gamma, wavelength_nm, 1)?;
    Some((
        physics::UndulatorParams {
            period_mm: PERIOD_MM,
            k,
            num_periods: 200,
        },
        physics::ElectronBeamParams {
            peak_current_a: 1000.0,
            norm_emittance_um: 0.5,
            energy_spread_rel: 0.01,
            beta_m: 1.0,
        },
    ))
}

/// Validate that a machine parameter is finite and strictly positive.
pub(crate) fn validate_positive(name: &'static str, value: f64) -> crate::error::Result<()> {
    if !value.is_finite() || value <= 0.0 {
        return Err(crate::error::LithographyError::InvalidParameter {
            name,
            value,
            reason: "must be finite and > 0",
        });
    }
    Ok(())
}

impl LpaFelSource {
    /// BELLA demonstrated configuration: 420 nm SASE at 100 MeV and 1 Hz
    /// (Kohrell et al. 2026, see the type docs). Retained as a reference
    /// fixture — this wavelength is NOT useful for EUV lithography; use
    /// `bella_target_25nm` for that (projected) regime. The 1 µJ pulse
    /// energy and 5 % jitter are ASSUMED placeholders. Carries the
    /// illustrative LPA-class undulator/beam (see `derived_quantities`),
    /// whose K reproduces 420 nm at 100 MeV.
    pub fn bella_baseline_100mev() -> crate::error::Result<Self> {
        validate_sigma(0.7)?;
        let machine = illustrative_lpa_machine(100.0, 420.0);
        let mut src = Self {
            wavelength_nm: 420.0,
            bandwidth_pm: 420.0,
            electron_energy_mev: 100.0,
            rep_rate_hz: 1.0,
            pulse_energy_uj: 1.0,
            pulse_duration_fs: 10.0,
            shot_to_shot_stability: 0.05,
            transverse_coherence_fraction: 0.85,
            spectral_samples: 5,
            spectral_shape: SpectralShape::Gaussian,
            illumination: IlluminationShape::Conventional { sigma: 0.7 },
            undulator: machine.map(|m| m.0),
            electron_beam: machine.map(|m| m.1),
            bandwidth_override_pm: None,
        };
        // Store the derived SASE bandwidth (2 rho lambda ~ 10.4 nm, 2.5 %).
        src.bandwidth_pm = src.effective_bandwidth_pm();
        Ok(src)
    }

    /// DESIGN PROJECTION: a ~500 MeV, 1 kHz LPA-FEL at 25 nm as an EUV study
    /// point (LPA-FEL lasing has been reported at 27 nm, none at 13.5 nm, and
    /// the demonstrated BELLA FEL runs at 1 Hz). All pulse parameters are
    /// assumptions. Carries the
    /// illustrative LPA-class
    /// undulator/beam: its derived FEL physics shows that a percent-level
    /// energy spread exceeds the Pierce parameter (see
    /// `derived_quantities`) — the reason LPA-FEL schemes need
    /// decompression or transverse-gradient undulators.
    pub fn bella_target_25nm(sigma: f64) -> crate::error::Result<Self> {
        validate_sigma(sigma)?;
        let machine = illustrative_lpa_machine(500.0, 25.0);
        let mut src = Self {
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
            undulator: machine.map(|m| m.0),
            electron_beam: machine.map(|m| m.1),
            bandwidth_override_pm: None,
        };
        // Store the derived SASE bandwidth (2 rho lambda ~ 266 pm, 1.06 %),
        // consistent with the preset's own beam (an earlier version kept an
        // independent 25 pm = 0.1 % set-point).
        src.bandwidth_pm = src.effective_bandwidth_pm();
        Ok(src)
    }

    /// Attach an undulator (and optionally electron-beam parameters),
    /// rejecting machine parameters that are non-physical or whose derived
    /// resonance wavelength disagrees with `wavelength_nm` by more than 5 %
    /// (the same rule the CLI applies to synchrotron undulators).
    pub fn with_machine(
        mut self,
        undulator: physics::UndulatorParams,
        electron_beam: Option<physics::ElectronBeamParams>,
    ) -> crate::error::Result<Self> {
        validate_positive("period_mm", undulator.period_mm)?;
        validate_positive("undulator_k", undulator.k)?;
        validate_positive("num_periods", undulator.num_periods as f64)?;
        if let Some(beam) = &electron_beam {
            validate_positive("peak_current_a", beam.peak_current_a)?;
            validate_positive("norm_emittance_um", beam.norm_emittance_um)?;
            validate_positive("beta_m", beam.beta_m)?;
            if !beam.energy_spread_rel.is_finite() || beam.energy_spread_rel < 0.0 {
                return Err(crate::error::LithographyError::InvalidParameter {
                    name: "energy_spread_rel",
                    value: beam.energy_spread_rel,
                    reason: "must be finite and >= 0",
                });
            }
        }
        self.undulator = Some(undulator);
        self.electron_beam = electron_beam;
        self.check_resonance()?;
        Ok(self)
    }

    /// Electron Lorentz factor from `electron_energy_mev`.
    pub fn gamma(&self) -> f64 {
        physics::gamma_from_mev(self.electron_energy_mev)
    }

    /// On-axis fundamental resonance wavelength (nm) DERIVED from the
    /// undulator and electron energy; `None` without an undulator.
    pub fn resonant_wavelength_nm(&self) -> Option<f64> {
        self.undulator.map(|u| u.resonance_nm(self.gamma()))
    }

    /// Enforce the 5 % consistency rule between the stored wavelength
    /// set-point and the undulator resonance (no-op without an undulator).
    pub fn check_resonance(&self) -> crate::error::Result<()> {
        if let Some(derived) = self.resonant_wavelength_nm() {
            if ((self.wavelength_nm - derived) / derived).abs() > 0.05 {
                return Err(crate::error::LithographyError::InvalidParameter {
                    name: "wavelength_nm",
                    value: self.wavelength_nm,
                    reason: "disagrees by more than 5% with the undulator resonance \
                             lambda_u (1 + K^2/2) / (2 gamma^2) derived from \
                             electron_energy_mev / period / K",
                });
            }
        }
        Ok(())
    }

    /// 1D + Ming-Xie FEL estimate; `None` unless both the undulator and the
    /// electron-beam parameters are present.
    pub fn fel_estimate(&self) -> Option<physics::FelEstimate> {
        match (&self.undulator, &self.electron_beam) {
            (Some(u), Some(b)) => Some(physics::fel_estimate(self.gamma(), u, b)),
            _ => None,
        }
    }

    /// Bandwidth FWHM (pm) the source actually uses: `bandwidth_override_pm`
    /// if set; else the SASE estimate `2 rho lambda` when the undulator +
    /// beam parameters give an FEL estimate; else the stored `bandwidth_pm`.
    pub fn effective_bandwidth_pm(&self) -> f64 {
        match (self.bandwidth_override_pm, self.derived_sase_bandwidth_pm()) {
            (Some(bw), _) => bw,
            (None, Some(bw)) => bw,
            (None, None) => self.bandwidth_pm,
        }
    }

    /// SASE bandwidth `2 rho lambda` (pm) from the machine parameters;
    /// `None` without an FEL estimate or when it is not finite and > 0.
    fn derived_sase_bandwidth_pm(&self) -> Option<f64> {
        self.fel_estimate()
            .map(|est| physics::sase_bandwidth_pm(self.wavelength_nm, est.rho_1d))
            .filter(|bw| bw.is_finite() && *bw > 0.0)
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
            undulator: None,
            electron_beam: None,
            bandwidth_override_pm: None,
        })
    }

    /// Pupil illumination intensity (shared logic with VuvSource).
    pub fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64 {
        evaluate_illumination(&self.illumination, fx_norm, fy_norm)
    }

    /// Polychromatic spectral sampling weights over the effective bandwidth
    /// ([`LpaFelSource::effective_bandwidth_pm`]).
    pub fn spectral_weights(&self) -> Vec<(f64, f64)> {
        evaluate_spectral_weights(
            self.wavelength_nm,
            self.effective_bandwidth_pm(),
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
        self.effective_bandwidth_pm()
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

    /// Photon budget; with an undulator the derived resonance and its
    /// mismatch to the set-point; with beam parameters the 1D Pierce
    /// parameter, gain lengths (1D and Ming Xie 3D), saturation power and
    /// length, and the energy-spread / emittance criteria.
    fn derived_quantities(&self) -> Vec<DerivedQuantity> {
        let lambda = self.wavelength_nm;
        let pulse_j = self.pulse_energy_uj * 1e-6;
        let power = pulse_j * self.rep_rate_hz;
        let mut out = vec![
            DerivedQuantity::new(
                "photon_energy",
                physics::HC_EV_NM / lambda,
                "eV",
                "hc / lambda",
            ),
            DerivedQuantity::new(
                "photons_per_pulse",
                pulse_j / physics::photon_energy_j(lambda),
                "photons",
                "stored pulse energy / photon energy",
            ),
            DerivedQuantity::new(
                "average_power",
                power,
                "W",
                "stored pulse energy x repetition rate",
            ),
            DerivedQuantity::new(
                "hvm_power_ratio",
                power / crate::source_models::throughput::HVM_EUV_POWER_AT_IF_W,
                "-",
                "average power / 250 W (in-band power at IF of production 13.5 nm EUV)",
            ),
            DerivedQuantity::new("electron_gamma", self.gamma(), "-", "1 + E_kin / m_e c^2"),
            DerivedQuantity::new(
                "bandwidth_fwhm",
                self.effective_bandwidth_pm(),
                "pm",
                match (self.bandwidth_override_pm, self.derived_sase_bandwidth_pm()) {
                    (Some(_), _) => "explicit bandwidth_override_pm (overrides 2 rho lambda)",
                    (None, Some(_)) => "2 rho lambda, DERIVED from the undulator + beam",
                    (None, None) => "stored set-point (no undulator + beam to derive it)",
                },
            ),
        ];
        if let (Some(und), Some(derived)) = (&self.undulator, self.resonant_wavelength_nm()) {
            out.push(DerivedQuantity::new(
                "resonant_wavelength",
                derived,
                "nm",
                "lambda_u (1 + K^2/2) / (2 gamma^2), DERIVED from the undulator",
            ));
            out.push(DerivedQuantity::new(
                "resonance_mismatch",
                (lambda - derived) / derived,
                "-",
                "(set-point - resonance) / resonance; |value| > 0.05 is rejected at construction",
            ));
            out.push(DerivedQuantity::new(
                "undulator_field",
                physics::undulator_field_from_k(und.k, und.period_mm),
                "T",
                "B0 = 2 pi m_e c K / (e lambda_u)",
            ));
            out.push(DerivedQuantity::new(
                "undulator_length",
                und.length_m(),
                "m",
                "num_periods x period",
            ));
        }
        if let Some(est) = self.fel_estimate() {
            let tau = self.pulse_duration_fs * 1e-15;
            out.extend(fel_estimate_quantities(&est, tau, lambda));
        }
        out
    }
}

/// Shared derived-quantity list for an FEL estimate (used by the LPA-FEL
/// and XFEL models). `pulse_duration_s` converts saturation power to a
/// saturation pulse energy.
pub(crate) fn fel_estimate_quantities(
    est: &physics::FelEstimate,
    pulse_duration_s: f64,
    wavelength_nm: f64,
) -> Vec<DerivedQuantity> {
    let spread_note = if est.energy_spread_over_rho < 1.0 {
        "sigma_gamma/gamma over rho: < 1 satisfies the 1D gain criterion"
    } else {
        "sigma_gamma/gamma over rho: > 1 VIOLATES the gain criterion - the beam needs \
         decompression / a transverse-gradient undulator before it can lase"
    };
    vec![
        DerivedQuantity::new(
            "pierce_parameter_1d",
            est.rho_1d,
            "-",
            "[(1/16)(I/I_A) K^2 JJ^2 / (gamma^3 sigma_x^2 k_u^2)]^(1/3), 1D FEL theory",
        ),
        DerivedQuantity::new(
            "beam_size_rms",
            est.rms_beam_size_m * 1e6,
            "um",
            "sqrt(eps_n beta / gamma)",
        ),
        DerivedQuantity::new(
            "gain_length_1d",
            est.gain_length_1d_m,
            "m",
            "lambda_u / (4 pi sqrt3 rho)",
        ),
        DerivedQuantity::new(
            "ming_xie_lambda",
            est.xie.lambda,
            "-",
            "3D gain-length degradation (Ming Xie fit; >~5 means no practical gain)",
        ),
        DerivedQuantity::new(
            "gain_length_3d",
            est.gain_length_3d_m,
            "m",
            "L_g1D (1 + Lambda), Ming Xie fit",
        ),
        DerivedQuantity::new(
            "energy_spread_over_rho",
            est.energy_spread_over_rho,
            "-",
            spread_note,
        ),
        DerivedQuantity::new(
            "emittance_over_photon_emittance",
            est.emittance_over_photon_emittance,
            "-",
            "eps / (lambda / 4 pi); <~ 1 for a transversely coherent FEL mode",
        ),
        DerivedQuantity::new(
            "beam_power_peak",
            est.beam_power_w,
            "W",
            "gamma m_e c^2 I_peak / e",
        ),
        DerivedQuantity::new(
            "saturation_power_1d",
            est.saturation_power_1d_w,
            "W",
            "rho P_beam (1D)",
        ),
        DerivedQuantity::new(
            "saturation_power_3d",
            est.saturation_power_3d_w,
            "W",
            "1.6 rho (L_g1D / L_g3D)^2 P_beam (Ming Xie)",
        ),
        DerivedQuantity::new(
            "saturation_pulse_energy_3d",
            est.saturation_power_3d_w * pulse_duration_s,
            "J",
            "Ming Xie saturation power x pulse duration (flat-top estimate)",
        ),
        DerivedQuantity::new(
            "saturation_length_3d",
            est.saturation_length_3d_m,
            "m",
            "(lambda_u / rho)(1 + Lambda)",
        ),
        DerivedQuantity::new(
            "undulator_over_saturation_length",
            est.undulator_length_m / est.saturation_length_3d_m,
            "-",
            ">= 1: the undulator is long enough to reach saturation",
        ),
        DerivedQuantity::new(
            "sase_bandwidth_from_rho",
            physics::sase_bandwidth_pm(wavelength_nm, est.rho_1d),
            "pm",
            "2 rho lambda (SASE FWHM estimate)",
        ),
    ]
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
            // Two poles on the orientation axis, at `orient` and `orient + pi`.
            // (An earlier version placed them at `orient +- pi`, which is the
            // same point: every "dipole" was a single off-axis monopole.)
            let orient = orientation_deg.to_radians();
            for pole in [orient, orient + std::f64::consts::PI] {
                let cx = sigma_center * pole.cos();
                let cy = sigma_center * pole.sin();
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
            SpectralShape::SincSquared => {
                if bw_nm > 0.0 {
                    sinc_squared(std::f64::consts::PI * SINC2_FWHM_X * dw / bw_nm)
                } else {
                    1.0
                }
            }
        };
        weights.push((wl, w));
    }
    weights
}

/// `(sin u / u)^2` with the removable singularity at 0.
fn sinc_squared(u: f64) -> f64 {
    if u.abs() < 1e-8 {
        1.0
    } else {
        let s = u.sin() / u;
        s * s
    }
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

// Lab / compact families: hard X-ray tube, discharge plasma, soft-X-ray
// laser, laser-wakefield betatron, Smith-Purcell free-electron grating.
pub use crate::source_models::betatron::BetatronSource;
pub use crate::source_models::dpp::{DppFuel, DppSource};
pub use crate::source_models::smith_purcell::SmithPurcellSource;
pub use crate::source_models::sxrl::{SxrlScheme, SxrlSource};
pub use crate::source_models::xray_tube::{XrayAnode, XrayTubeSource};

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
    XrayTube(XrayTubeSource),
    Dpp(DppSource),
    Sxrl(SxrlSource),
    Betatron(BetatronSource),
    SmithPurcell(SmithPurcellSource),
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
            SourceKind::XrayTube($s) => $body,
            SourceKind::Dpp($s) => $body,
            SourceKind::Sxrl($s) => $body,
            SourceKind::Betatron($s) => $body,
            SourceKind::SmithPurcell($s) => $body,
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
            SourceKind::XrayTube(_) => "xray_tube",
            SourceKind::Dpp(_) => "dpp",
            SourceKind::Sxrl(_) => "sxrl",
            SourceKind::Betatron(_) => "betatron",
            SourceKind::SmithPurcell(_) => "smith_purcell",
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

// Every trait method is forwarded, including the ones with default bodies:
// a missing arm silently replaces a family's override with the trait default
// (the X-ray tube's mean-photon-energy photon density was lost that way).
// `test_source_kind_forwards_every_trait_method` checks all of them.
impl LithographySource for SourceKind {
    fn wavelength_nm(&self) -> f64 {
        for_each_source!(self, s => s.wavelength_nm())
    }

    fn photon_energy_ev(&self) -> f64 {
        for_each_source!(self, s => LithographySource::photon_energy_ev(s))
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

    fn photon_density_per_mj_cm2(&self) -> f64 {
        for_each_source!(self, s => LithographySource::photon_density_per_mj_cm2(s))
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

    fn derived_quantities(&self) -> Vec<DerivedQuantity> {
        for_each_source!(self, s => s.derived_quantities())
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

impl From<XrayTubeSource> for SourceKind {
    fn from(s: XrayTubeSource) -> Self {
        SourceKind::XrayTube(s)
    }
}

impl From<DppSource> for SourceKind {
    fn from(s: DppSource) -> Self {
        SourceKind::Dpp(s)
    }
}

impl From<SxrlSource> for SourceKind {
    fn from(s: SxrlSource) -> Self {
        SourceKind::Sxrl(s)
    }
}

impl From<BetatronSource> for SourceKind {
    fn from(s: BetatronSource) -> Self {
        SourceKind::Betatron(s)
    }
}

impl From<SmithPurcellSource> for SourceKind {
    fn from(s: SmithPurcellSource) -> Self {
        SourceKind::SmithPurcell(s)
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
            undulator: None,
            electron_beam: None,
            bandwidth_override_pm: None,
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
    fn test_lpa_fel_bandwidth_follows_beam() {
        // With undulator + beam the bandwidth is the SASE 2 rho lambda of the
        // machine (rho = 5.3239e-3 at 25 nm -> 266.2 pm, 1.06 %), not an
        // independent set-point.
        let fel = LpaFelSource::bella_target_25nm(0.7).unwrap();
        let rho = fel.fel_estimate().unwrap().rho_1d;
        let expected = 2.0 * rho * 25.0 * 1e3;
        assert_relative_eq!(expected, 266.196_827_6, max_relative = 1e-5);
        assert_relative_eq!(fel.bandwidth_pm(), expected, max_relative = 1e-12);
        assert_relative_eq!(fel.bandwidth_pm, expected, max_relative = 1e-12);
        assert_relative_eq!(
            derived(&fel, "bandwidth_fwhm"),
            expected,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            derived(&fel, "sase_bandwidth_from_rho"),
            expected,
            max_relative = 1e-12
        );
        // The spectral samples span the derived line: +-2.5 FWHM.
        let w = fel.spectral_weights();
        let span_pm = (w.last().unwrap().0 - w[0].0) * 1e3;
        assert_relative_eq!(span_pm, 5.0 * expected, max_relative = 1e-9);
        // Changing the beam changes the bandwidth.
        let mut dense = fel.clone();
        dense.electron_beam = Some(physics::ElectronBeamParams {
            peak_current_a: 8000.0,
            ..fel.electron_beam.unwrap()
        });
        assert_relative_eq!(dense.bandwidth_pm(), 2.0 * expected, max_relative = 1e-9);
        // The 420 nm demonstrated-energy preset: rho ~ 1.24e-2 -> ~10.4 nm.
        let base = LpaFelSource::bella_baseline_100mev().unwrap();
        let rho0 = base.fel_estimate().unwrap().rho_1d;
        assert_relative_eq!(
            base.bandwidth_pm(),
            2.0 * rho0 * 420.0 * 1e3,
            max_relative = 1e-12
        );
        assert!((10_000.0..11_000.0).contains(&base.bandwidth_pm()));
        // An explicit override always wins.
        let mut seeded = fel.clone();
        seeded.bandwidth_override_pm = Some(5.0);
        assert_relative_eq!(seeded.bandwidth_pm(), 5.0);
        let w = seeded.spectral_weights();
        assert_relative_eq!(
            (w.last().unwrap().0 - w[0].0) * 1e3,
            25.0,
            max_relative = 1e-9
        );
        // Undulator without beam parameters: no FEL estimate, stored value.
        let und_only = LpaFelSource::new(25.0, 0.7)
            .unwrap()
            .with_machine(fel.undulator.unwrap(), None)
            .unwrap();
        assert_relative_eq!(und_only.bandwidth_pm(), 25.0);
        // Configs without the override field still load (TOML omits the
        // `None` field, exercising `#[serde(default)]`).
        let toml_str = toml::to_string(&fel).unwrap();
        assert!(!toml_str.contains("bandwidth_override_pm"));
        let back: LpaFelSource = toml::from_str(&toml_str).unwrap();
        assert!(back.bandwidth_override_pm.is_none());
        assert_relative_eq!(back.bandwidth_pm(), expected, max_relative = 1e-12);
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

    fn derived(src: &impl LithographySource, name: &str) -> f64 {
        src.derived_quantities()
            .into_iter()
            .find(|q| q.name == name)
            .unwrap_or_else(|| panic!("missing derived quantity {name}"))
            .value
    }

    #[test]
    fn test_vuv_derived_quantities() {
        let f2 = VuvSource::f2_laser(0.7).unwrap();
        // 10 mJ / (hc / 157.63 nm) = 7.935e15 photons per pulse (scipy fixture)
        assert_relative_eq!(
            derived(&f2, "photons_per_pulse"),
            7.935_278_293_155_081e15,
            max_relative = 1e-7
        );
        assert_relative_eq!(derived(&f2, "average_power"), 40.0, max_relative = 1e-12);
        assert_relative_eq!(
            derived(&f2, "photon_rate"),
            3.174_111_317_262_032e19,
            max_relative = 1e-7
        );
        // lambda^2 / d-lambda = 157.63^2 / 1.1e-3 nm = 2.26 cm
        assert_relative_eq!(
            derived(&f2, "coherence_length"),
            22_588.379,
            max_relative = 1e-6
        );
        // F2 is a real laser: no "demonstrated = 0" flag.
        assert!(f2
            .derived_quantities()
            .iter()
            .all(|q| q.name != "demonstrated"));
        // The Ar2 preset is flagged as hypothetical.
        let ar2 = VuvSource::ar2_laser(0.7).unwrap();
        assert_eq!(derived(&ar2, "demonstrated"), 0.0);
        let bw_note = ar2
            .derived_quantities()
            .into_iter()
            .find(|q| q.name == "relative_bandwidth")
            .unwrap()
            .note;
        assert!(bw_note.contains("never been demonstrated"));
    }

    #[test]
    fn test_heritage_presets_photon_budget() {
        // (preset, photon energy eV, photons per mJ) — mpmath fixtures.
        let cases = [
            (
                VuvSource::arf_laser(0.7).unwrap(),
                6.411_825_793_31,
                9.734_370_950_9e14,
            ),
            (
                VuvSource::krf_laser(0.7).unwrap(),
                4.993_322_311_72,
                1.249_971_198_5e15,
            ),
            (
                VuvSource::hg_i_line(0.7).unwrap(),
                3.396_684_823_9,
                1.837_529_649_66e15,
            ),
            (
                VuvSource::hg_h_line(0.7).unwrap(),
                3.063_938_285_4,
                2.037_087_073_26e15,
            ),
            (
                VuvSource::hg_g_line(0.7).unwrap(),
                2.844_765_079_64,
                2.194_033_215_3e15,
            ),
        ];
        for (src, e_ev, per_mj) in cases {
            assert_relative_eq!(derived(&src, "photon_energy"), e_ev, max_relative = 1e-9);
            // HC_EV_NM is the rounded CODATA value: 4.4e-8 tolerance.
            assert_relative_eq!(derived(&src, "photons_per_mj"), per_mj, max_relative = 1e-7);
            assert_relative_eq!(
                derived(&src, "photon_density_per_dose"),
                per_mj * 1e-14,
                max_relative = 1e-7
            );
            assert!(src.line_label().is_some());
        }
        // NIST air wavelengths of the Hg lines.
        assert_eq!(VuvSource::hg_g_line(0.7).unwrap().wavelength_nm, 435.8328);
        assert_eq!(VuvSource::hg_h_line(0.7).unwrap().wavelength_nm, 404.6563);
        assert_eq!(VuvSource::hg_i_line(0.7).unwrap().wavelength_nm, 365.0153);
        // ArF carries 14.3x the photons of 13.5 nm EUV per unit dose.
        let arf = VuvSource::arf_laser(0.7).unwrap();
        assert_relative_eq!(
            derived(&arf, "photons_vs_13nm5"),
            193.368 / 13.5,
            max_relative = 1e-12
        );
        // Representative laser budgets: 15 mJ x 6 kHz = 90 W; 10 mJ x 4 kHz = 40 W.
        assert_relative_eq!(arf.average_power_w().unwrap(), 90.0, max_relative = 1e-12);
        assert_relative_eq!(
            derived(&arf, "photons_per_pulse"),
            15.0 * 9.734_370_950_9e14,
            max_relative = 1e-7
        );
        let krf = VuvSource::krf_laser(0.7).unwrap();
        assert_relative_eq!(krf.average_power_w().unwrap(), 40.0, max_relative = 1e-12);
        assert!(VuvSource::arf_laser(0.0).is_err());
        assert!(VuvSource::hg_i_line(1.5).is_err());
    }

    #[test]
    fn test_hg_lamps_are_cw_single_sample() {
        let lamp = VuvSource::hg_i_line(0.6).unwrap();
        assert!(lamp.is_cw());
        assert_eq!(lamp.pulse_energy_j(), None);
        assert_eq!(LithographySource::rep_rate_hz(&lamp), None);
        assert_eq!(lamp.average_power_w(), None);
        // One sample at the line centre (achromatic lamp-era lens).
        let w = lamp.spectral_weights();
        assert_eq!(w.len(), 1);
        assert_relative_eq!(w[0].0, 365.0153);
        assert_relative_eq!(w[0].1, 1.0);
        assert_eq!(derived(&lamp, "continuous_wave"), 1.0);
        assert!(lamp
            .derived_quantities()
            .iter()
            .all(|q| q.name != "photons_per_pulse"));
        // 3 nm FWHM Gaussian filter band: E95 = 1.6646 x 3000 pm.
        assert_relative_eq!(
            derived(&lamp, "e95_bandwidth"),
            1.664_640_139_849_238 * 3000.0,
            max_relative = 1e-12
        );
        // Pulsed presets keep their pulse metadata.
        let f2 = VuvSource::f2_laser(0.7).unwrap();
        assert!(!f2.is_cw());
        assert_eq!(LithographySource::rep_rate_hz(&f2), Some(4000.0));
    }

    #[test]
    fn test_e95_bandwidth_conventions() {
        // Analytic ratios (mpmath fixtures).
        assert_relative_eq!(
            e95_bandwidth_pm(&SpectralShape::Gaussian, 0.2),
            0.332_928_027_969_847_6,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            e95_bandwidth_pm(&SpectralShape::Lorentzian, 1.0),
            12.706_204_736_174_705,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            e95_bandwidth_pm(&SpectralShape::SincSquared, 1.0),
            4.679_897_315_241_133,
            max_relative = 1e-12
        );
        // Tabulated: a finely sampled symmetric triangle of half-width 1 nm
        // has E95 = 2 (1 - sqrt(0.05)) nm = 1.5528 nm.
        let xs: Vec<f64> = (0..=2000).map(|i| -1.0 + i as f64 * 0.001).collect();
        let ys: Vec<f64> = xs.iter().map(|x| 1.0 - x.abs()).collect();
        let tab = SpectralShape::Tabulated {
            wavelengths_nm: xs,
            intensities: ys,
        };
        let expected = 2.0 * (1.0 - 0.05_f64.sqrt()) * 1e3;
        assert_relative_eq!(e95_bandwidth_pm(&tab, 0.0), expected, max_relative = 1e-4);
        // Degenerate tables give 0, no NaN.
        let empty = SpectralShape::Tabulated {
            wavelengths_nm: vec![1.0],
            intensities: vec![1.0],
        };
        assert_eq!(e95_bandwidth_pm(&empty, 1.0), 0.0);
    }

    #[test]
    fn test_sinc_squared_line_shape() {
        // FWHM convention: half maximum at +-FWHM/2, first zero at FWHM / 0.8859.
        let fwhm_pm = 100.0;
        let w_half = sample_line_unnormalized(13.5, fwhm_pm, 5, &SpectralShape::SincSquared);
        // Samples at -2.5, -1.25, 0, +1.25, +2.5 FWHM; centre = 1.
        assert_relative_eq!(w_half[2].1, 1.0, epsilon = 1e-15);
        // sinc^2(pi * 0.88589 * 1.25) = sinc^2(3.4790) = 0.009022
        let u = std::f64::consts::PI * SINC2_FWHM_X * 1.25;
        assert_relative_eq!(w_half[3].1, (u.sin() / u).powi(2), max_relative = 1e-12);
        assert_relative_eq!(w_half[1].1, w_half[3].1, max_relative = 1e-12);
        // Exactly at +-FWHM/2 the weight is 1/2.
        let x = std::f64::consts::PI * SINC2_FWHM_X * 0.5;
        assert_relative_eq!((x.sin() / x).powi(2), 0.5, max_relative = 1e-12);
        assert_relative_eq!(sinc_squared(0.0), 1.0);
        // Zero bandwidth degenerates to a single unit weight, no NaN.
        let w0 = evaluate_spectral_weights(13.5, 0.0, 3, &SpectralShape::SincSquared);
        assert!(w0.iter().all(|(_, w)| w.is_finite()));
    }

    #[test]
    fn test_lpa_fel_machine_derivation() {
        let fel = LpaFelSource::bella_target_25nm(0.7).unwrap();
        // K chosen by the resonance condition: derived lambda == set-point.
        assert_relative_eq!(
            fel.resonant_wavelength_nm().unwrap(),
            25.0,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            fel.undulator.unwrap().k,
            1.672_382_185_524_561,
            max_relative = 1e-9
        );
        // 1D Pierce parameter for 1 kA, 0.5 um, beta 1 m at 500 MeV: 5.32e-3
        // (scipy fixture; beta*gamma vs gamma emittance convention < 1e-6).
        let est = fel.fel_estimate().unwrap();
        assert_relative_eq!(est.rho_1d, 0.005_323_936_552_564_808, max_relative = 1e-5);
        assert_relative_eq!(
            est.gain_length_1d_m,
            0.172_594_373_223_133_56,
            max_relative = 1e-5
        );
        // A 1 % energy spread is ~1.9 rho: the gain criterion is violated and
        // the Ming Xie degradation is enormous (Lambda ~ 49).
        assert!(derived(&fel, "energy_spread_over_rho") > 1.8);
        assert!(est.xie.lambda > 40.0);
        assert!(derived(&fel, "undulator_over_saturation_length") < 0.1);
        // Reducing the spread below rho restores a sane gain length.
        let cool = fel
            .clone()
            .with_machine(
                fel.undulator.unwrap(),
                Some(physics::ElectronBeamParams {
                    energy_spread_rel: 1e-3,
                    ..fel.electron_beam.unwrap()
                }),
            )
            .unwrap();
        assert!(cool.fel_estimate().unwrap().xie.lambda < 5.0);
    }

    #[test]
    fn test_lpa_fel_resonance_five_percent_rule() {
        let base = LpaFelSource::new(25.0, 0.7).unwrap();
        assert!(base.undulator.is_none());
        let gamma = physics::gamma_from_mev(500.0);
        let k = physics::undulator_k_for_wavelength(20.0, gamma, 25.0, 1).unwrap();
        let und = physics::UndulatorParams {
            period_mm: 20.0,
            k,
            num_periods: 200,
        };
        assert!(base.clone().with_machine(und, None).is_ok());
        // 4 % off: accepted; 10 % off: rejected.
        let mut off4 = base.clone();
        off4.wavelength_nm = 26.0;
        assert!(off4.with_machine(und, None).is_ok());
        let mut off10 = base.clone();
        off10.wavelength_nm = 27.5;
        assert!(off10.with_machine(und, None).is_err());
        // Non-physical machine parameters are rejected.
        let bad = physics::UndulatorParams { k: 0.0, ..und };
        assert!(base.with_machine(bad, None).is_err());
    }

    #[test]
    fn test_new_presets_toml_roundtrip() {
        use crate::source_models::lpp::{LppDriveLaser, LppSource};
        use crate::source_models::xfel::XfelSource;
        let kinds: Vec<SourceKind> = vec![
            LpaFelSource::bella_target_25nm(0.7).unwrap().into(),
            XfelSource::cw_sc_13nm5().into(),
            XfelSource::erl_13nm5().into(),
            LppSource::sn_with_drive_laser(0.9, LppDriveLaser::Thulium2um)
                .unwrap()
                .into(),
        ];
        for kind in kinds {
            let toml_str = toml::to_string(&kind).unwrap();
            let parsed: SourceKind = toml::from_str(&toml_str).unwrap();
            assert_eq!(parsed.kind_label(), kind.kind_label());
            assert_relative_eq!(
                parsed.average_power_w().unwrap(),
                kind.average_power_w().unwrap(),
                max_relative = 1e-12
            );
            assert_eq!(
                parsed.derived_quantities().len(),
                kind.derived_quantities().len()
            );
        }
    }

    #[test]
    fn test_every_family_reports_derived_quantities() {
        use crate::source_models::{
            hhg::HhgSource, ics::IcsSource, lpp::LppSource, ssmb::SsmbSource,
            synchrotron::SynchrotronSource, xfel::XfelSource,
        };
        let kinds: Vec<SourceKind> = vec![
            VuvSource::f2_laser(0.7).unwrap().into(),
            LpaFelSource::bella_target_25nm(0.7).unwrap().into(),
            LppSource::sn_13nm5(0.9).unwrap().into(),
            SynchrotronSource::compact_euv_undulator().unwrap().into(),
            SynchrotronSource::liga_bending_magnet().into(),
            HhgSource::ne_800nm_13nm5().unwrap().into(),
            XfelSource::flash_13nm5().into(),
            IcsSource::compact_euv_13nm5().unwrap().into(),
            SsmbSource::euv_1kw_13nm5().unwrap().into(),
            EntangledPhotonSource::default().into(),
        ];
        for kind in kinds {
            let dq = kind.derived_quantities();
            assert!(
                dq.len() >= 3,
                "{} reports too few quantities",
                kind.kind_label()
            );
            for q in &dq {
                assert!(!q.name.is_empty() && !q.unit.is_empty() && !q.note.is_empty());
                assert!(
                    !q.value.is_nan(),
                    "{}: {} is NaN",
                    kind.kind_label(),
                    q.name
                );
            }
            let photon_energy = dq.iter().find(|q| q.name == "photon_energy");
            if let Some(q) = photon_energy {
                assert_relative_eq!(q.value, kind.photon_energy_ev(), max_relative = 1e-9);
            }
        }
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

    /// Every `LithographySource` method called through `SourceKind` must
    /// return exactly what the concrete type returns.
    fn assert_forwarded<T: LithographySource + Clone + Into<SourceKind>>(src: T) {
        let kind: SourceKind = src.clone().into();
        let label = kind.kind_label();
        let bits = |v: f64| v.to_bits();
        let opt_bits = |v: Option<f64>| v.map(f64::to_bits);
        assert_eq!(
            bits(kind.wavelength_nm()),
            bits(src.wavelength_nm()),
            "{label}"
        );
        assert_eq!(
            bits(kind.photon_energy_ev()),
            bits(src.photon_energy_ev()),
            "{label} photon_energy_ev"
        );
        assert_eq!(
            bits(kind.bandwidth_pm()),
            bits(src.bandwidth_pm()),
            "{label}"
        );
        for (fx, fy) in [(0.0, 0.0), (0.1, 0.05), (0.6, -0.3)] {
            assert_eq!(
                bits(kind.intensity_at(fx, fy)),
                bits(src.intensity_at(fx, fy)),
                "{label}"
            );
        }
        let (a, b) = (kind.spectral_weights(), src.spectral_weights());
        assert_eq!(a.len(), b.len(), "{label}");
        for ((wa, pa), (wb, pb)) in a.iter().zip(&b) {
            assert_eq!((bits(*wa), bits(*pa)), (bits(*wb), bits(*pb)), "{label}");
        }
        assert_eq!(
            bits(kind.photon_density_per_mj_cm2()),
            bits(src.photon_density_per_mj_cm2()),
            "{label} photon_density_per_mj_cm2"
        );
        assert_eq!(
            opt_bits(kind.pulse_energy_j()),
            opt_bits(src.pulse_energy_j()),
            "{label}"
        );
        assert_eq!(
            opt_bits(kind.rep_rate_hz()),
            opt_bits(src.rep_rate_hz()),
            "{label}"
        );
        assert_eq!(
            opt_bits(kind.pulse_duration_s()),
            opt_bits(src.pulse_duration_s()),
            "{label}"
        );
        assert_eq!(
            opt_bits(kind.average_power_w()),
            opt_bits(src.average_power_w()),
            "{label}"
        );
        assert_eq!(
            bits(kind.transverse_coherence()),
            bits(src.transverse_coherence()),
            "{label}"
        );
        assert_eq!(
            bits(kind.shot_to_shot_rms()),
            bits(src.shot_to_shot_rms()),
            "{label}"
        );
        let (da, db) = (kind.derived_quantities(), src.derived_quantities());
        assert_eq!(da.len(), db.len(), "{label}");
        for (qa, qb) in da.iter().zip(&db) {
            assert_eq!(
                (&qa.name, bits(qa.value)),
                (&qb.name, bits(qb.value)),
                "{label}"
            );
        }
    }

    #[test]
    fn test_source_kind_forwards_every_trait_method() {
        use crate::source_models::{
            betatron::BetatronSource, dpp::DppSource, hhg::HhgSource, ics::IcsSource,
            lpp::LppSource, smith_purcell::SmithPurcellSource, ssmb::SsmbSource, sxrl::SxrlSource,
            synchrotron::SynchrotronSource, xfel::XfelSource, xray_tube::XrayTubeSource,
        };
        assert_forwarded(VuvSource::f2_laser(0.7).unwrap());
        assert_forwarded(VuvSource::hg_i_line(0.7).unwrap());
        assert_forwarded(LpaFelSource::bella_target_25nm(0.7).unwrap());
        assert_forwarded(LppSource::sn_13nm5(0.9).unwrap());
        assert_forwarded(SynchrotronSource::compact_euv_undulator().unwrap());
        assert_forwarded(SynchrotronSource::liga_bending_magnet());
        assert_forwarded(HhgSource::ne_800nm_13nm5().unwrap());
        assert_forwarded(XfelSource::flash_13nm5());
        assert_forwarded(IcsSource::compact_euv_13nm5().unwrap());
        assert_forwarded(SsmbSource::euv_1kw_13nm5().unwrap());
        assert_forwarded(EntangledPhotonSource::default());
        assert_forwarded(XrayTubeSource::w_60kv().unwrap());
        assert_forwarded(DppSource::default());
        assert_forwarded(SxrlSource::default());
        assert_forwarded(BetatronSource::default());
        assert_forwarded(SmithPurcellSource::default());
    }

    #[test]
    fn test_source_kind_keeps_xray_tube_photon_density() {
        // Regression: SourceKind used the trait-default photon density
        // (hc / mean wavelength), dropping the tube's mean-photon-energy
        // override and overcounting photons per dose by <E><1/E>.
        use crate::source_models::xray_tube::XrayTubeSource;
        let tube = XrayTubeSource::w_60kv().unwrap();
        let kind: SourceKind = tube.clone().into();
        let own = tube.photon_density_per_mj_cm2();
        let trait_default = 10.0 / physics::photon_energy_j(tube.wavelength_nm()) * 1e-18;
        assert!(trait_default / own > 1.3, "{}", trait_default / own);
        assert_relative_eq!(kind.photon_density_per_mj_cm2(), own, max_relative = 1e-15);
        assert_relative_eq!(
            kind.photon_energy_ev(),
            tube.photon_energy_ev(),
            max_relative = 1e-15
        );
        // The stochastic path reads the source through SourceKind.
        let params = crate::stochastic::StochasticParams::from_source(&kind);
        assert_relative_eq!(params.photon_density_per_mj_cm2, own, max_relative = 1e-15);
    }

    #[test]
    fn test_dipole_has_two_symmetric_poles() {
        // Regression: both poles used to land on the same point.
        let x = IlluminationShape::Dipole {
            sigma_center: 0.7,
            sigma_radius: 0.15,
            orientation_deg: 0.0,
        };
        assert_eq!(evaluate_illumination(&x, 0.7, 0.0), 1.0);
        assert_eq!(evaluate_illumination(&x, -0.7, 0.0), 1.0);
        assert_eq!(evaluate_illumination(&x, 0.0, 0.7), 0.0);
        assert_eq!(evaluate_illumination(&x, 0.0, 0.0), 0.0);
        // Orientation 90 deg puts the poles on the y axis.
        let y = IlluminationShape::Dipole {
            sigma_center: 0.7,
            sigma_radius: 0.15,
            orientation_deg: 90.0,
        };
        assert_eq!(evaluate_illumination(&y, 0.0, 0.7), 1.0);
        assert_eq!(evaluate_illumination(&y, 0.0, -0.7), 1.0);
        assert_eq!(evaluate_illumination(&y, 0.7, 0.0), 0.0);
        // Symmetric under a 180-degree rotation for any orientation, and the
        // two poles carry equal area.
        let tilted = IlluminationShape::Dipole {
            sigma_center: 0.6,
            sigma_radius: 0.2,
            orientation_deg: 30.0,
        };
        let (mut plus, mut minus) = (0usize, 0usize);
        for i in 0..81 {
            for j in 0..81 {
                let fx = -1.0 + i as f64 * 0.025;
                let fy = -1.0 + j as f64 * 0.025;
                let a = evaluate_illumination(&tilted, fx, fy);
                let b = evaluate_illumination(&tilted, -fx, -fy);
                assert_eq!(a, b, "not 180-degree symmetric at ({fx}, {fy})");
                if a > 0.0 {
                    let along = fx * 30.0_f64.to_radians().cos() + fy * 30.0_f64.to_radians().sin();
                    if along > 0.0 {
                        plus += 1;
                    } else {
                        minus += 1;
                    }
                }
            }
        }
        assert!(plus > 0 && plus == minus, "pole areas {plus} vs {minus}");
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

    #[test]
    fn test_lab_compact_families_toml_roundtrip() {
        // X-ray tube, DPP/LDP, soft-X-ray laser, betatron, Smith-Purcell:
        // tag serialization, wavelength round trip, trait invariants, and
        // non-empty derived quantities through SourceKind dispatch.
        let kinds: Vec<(SourceKind, &str)> = vec![
            (XrayTubeSource::w_60kv().unwrap().into(), "xray_tube"),
            (XrayTubeSource::cu_40kv().unwrap().into(), "xray_tube"),
            (DppSource::xe_13nm5(0.9).unwrap().into(), "dpp"),
            (DppSource::sn_13nm5(0.9).unwrap().into(), "dpp"),
            (SxrlSource::ar_46nm9().unwrap().into(), "sxrl"),
            (SxrlSource::ag_13nm9().unwrap().into(), "sxrl"),
            (BetatronSource::lwfa_100tw().unwrap().into(), "betatron"),
            (
                SmithPurcellSource::euv_13nm5().unwrap().into(),
                "smith_purcell",
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
            assert_relative_eq!(parsed.wavelength_nm(), kind.wavelength_nm(), epsilon = 1e-9);
            assert_relative_eq!(
                parsed.bandwidth_pm(),
                kind.bandwidth_pm(),
                max_relative = 1e-9
            );
            let sum: f64 = parsed.spectral_weights().iter().map(|(_, w)| w).sum();
            assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
            assert_eq!(parsed.spectral_samples(), kind.spectral_samples());
            let dq = parsed.derived_quantities();
            assert!(!dq.is_empty(), "{tag}: no derived quantities");
            assert!(
                dq.iter().all(|q| q.value.is_finite()),
                "{tag}: non-finite derived value"
            );
            assert!(parsed.average_power_w().unwrap() > 0.0);
        }
    }
}
