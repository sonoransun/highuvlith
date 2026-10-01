//! High-harmonic generation (HHG): table-top coherent EUV/soft-X-ray.
//!
//! An intense femtosecond driver (typically 800 nm Ti:Sapphire or
//! 1030 nm Yb) focused into a noble gas generates odd harmonics of the
//! driver frequency via the three-step model (tunnel ionization ->
//! field acceleration -> recombination). The comb extends up to the
//! cutoff `E_max = I_p + 3.17 U_p` where `U_p = 9.33e-14 I lambda^2` is
//! the ponderomotive energy. Photon flux is low (nJ-uJ per pulse) but
//! coherence is laser-like — HHG is the workhorse of table-top EUV
//! metrology and the candidate compact source for actinic mask
//! inspection.
//!
//! # Model status
//!
//! Comb spacing (2 driver photons), cutoff law, cutoff-violation
//! rejection, Keldysh parameter, and the plane-wave critical ionization
//! fraction (the phase-matching limit) are implemented and textbook-exact.
//! The plateau/rolloff intensity envelope is an empirical shape.
//! Simplified (🔶): the per-harmonic conversion efficiency is an
//! ORDER-OF-MAGNITUDE estimate — an assumed absorption-limited plateau
//! efficiency per gas for ~800 nm drivers, scaled by the measured
//! single-atom driver-wavelength law `(lambda / 800 nm)^-5.5` and the
//! cutoff envelope — used only for the derived harmonic power (when a
//! driver power is given) and the power-gap figures. It ignores
//! phase-matching geometry, pressure, and reabsorption details, and the
//! phase-matched cutoff lies below `I_p + 3.17 U_p` whenever the
//! ionization at the generating intensity exceeds the critical fraction.
//! Powers are DERIVED from a driver average power (ASSUMED 5 W by default,
//! 3 W / 20 W in the Ar / Ne presets) and compared with measured records
//! ([`measured_power_record_w`]): at 13.5 nm HHG has delivered ~60 nW (2024)
//! to ~0.43 µW (KMLabs/JILA, 2017) and ~1 µW (59th harmonic, Hyogo/RIKEN,
//! 2012); ~0.14 mW at 30 eV (2014); mW-class single harmonics only at
//! 21.7–26.5 eV (fiber-laser drivers). A derived power above the record at
//! its photon energy is labelled a PROJECTION.
//! The default (and lithographically honest) mode is monochromatized
//! single-harmonic imaging; full-comb mode (optionally restricted to a
//! filter/multilayer passband) is spectral bookkeeping for the
//! center-wavelength TCC and becomes an honest per-wavelength image only
//! through the exact multi-wavelength imaging path.
//!
//! # Key equations
//!
//! - `U_p[eV] = 9.33e-14 I[W/cm^2] lambda^2[um]`, `E_max = I_p + 3.17 U_p`
//! - Keldysh `gamma_K = sqrt(I_p / (2 U_p))` (< 1: tunnelling regime)
//! - Critical ionization `eta_cr = dn / (dn + N_atm r_e lambda^2 / 2 pi)`
//! - Efficiency estimate `eta_q = eta_gas(800 nm) (lambda / 800 nm)^-5.5 g(E_q / E_max)`
//! - Harmonic power `P_q = P_driver eta_q`, checked against the measured
//!   record at `E_q`

use serde::{Deserialize, Serialize};

use super::physics;
use crate::source::{
    evaluate_illumination, evaluate_multiline_weights, evaluate_spectral_weights,
    sigma_from_coherence, DerivedQuantity, IlluminationShape, LithographySource, SpectralLine,
    SpectralShape,
};

/// Planck constant x speed of light in eV·nm.
const HC_EV_NM: f64 = 1239.84193;

/// Exponent of the single-atom driver-wavelength scaling
/// `yield ~ lambda^-p` at constant intensity (measured values span
/// p ≈ 5–6.5; the midpoint is used).
pub const HHG_WAVELENGTH_SCALING_EXPONENT: f64 = 5.5;

/// ASSUMED driver average power (W) of the generic constructor: a
/// 5 W-class Ti:Sapphire (0.5 mJ at the default 10 kHz). With the
/// efficiency estimate it gives ~0.23 µW at 13.5 nm (Ne, q = 59) and 50 µW
/// at 29.6 nm (Ar, q = 27), both inside the measured records
/// ([`measured_power_record_w`]).
pub const DEFAULT_DRIVER_AVERAGE_POWER_W: f64 = 5.0;

/// Best measured single-harmonic HHG average power (W) at a photon energy
/// (eV), from the curated records: 12.9 mW up to 26.5 eV (Klas et al.,
/// PhotoniX 2, 4 (2021)); 0.144 mW up to 30 eV (3e13 photons/s, Hädrich et
/// al., Nat. Photonics 8, 779 (2014)); from 30 eV to 92 eV (13.5 nm) a
/// log-linear interpolation down to ~1 µW (59th harmonic, Univ. of Hyogo /
/// RIKEN, 2012 EUVL Workshop; KMLabs/JILA reported 0.43 µW at 92 eV in
/// 2017) — the curated set has no records in between, so that segment is a
/// guide, not a measurement. `None` above 92 eV (no record).
pub fn measured_power_record_w(photon_ev: f64) -> Option<f64> {
    const KLAS_2021: (f64, f64) = (26.5, 1.29e-2);
    const HADRICH_2014: (f64, f64) = (30.0, 1.44e-4);
    const EUV_13NM5: (f64, f64) = (92.0, 1.0e-6);
    if !(photon_ev.is_finite() && photon_ev > 0.0) || photon_ev > EUV_13NM5.0 {
        return None;
    }
    Some(if photon_ev <= KLAS_2021.0 {
        KLAS_2021.1
    } else if photon_ev <= HADRICH_2014.0 {
        HADRICH_2014.1
    } else {
        let t = (photon_ev - HADRICH_2014.0) / (EUV_13NM5.0 - HADRICH_2014.0);
        HADRICH_2014.1 * (EUV_13NM5.1 / HADRICH_2014.1).powf(t)
    })
}

/// Noble generation gas, which fixes the ionization potential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HhgGas {
    Helium,
    Neon,
    Argon,
    Krypton,
    Xenon,
}

impl HhgGas {
    /// Ionization potential in eV (NIST atomic spectra database).
    pub fn ionization_potential_ev(&self) -> f64 {
        match self {
            HhgGas::Helium => 24.587,
            HhgGas::Neon => 21.565,
            HhgGas::Argon => 15.760,
            HhgGas::Krypton => 14.000,
            HhgGas::Xenon => 12.130,
        }
    }

    /// Refractivity `n - 1` at 0 °C and 1 atm (visible-wavelength values;
    /// within ~1–2 % at 800–1030 nm). Sets the critical ionization
    /// fraction for phase matching.
    pub fn refractivity_stp(&self) -> f64 {
        match self {
            HhgGas::Helium => 3.5e-5,
            HhgGas::Neon => 6.7e-5,
            HhgGas::Argon => 2.81e-4,
            HhgGas::Krypton => 4.27e-4,
            HhgGas::Xenon => 7.02e-4,
        }
    }

    /// ASSUMED order-of-magnitude plateau conversion efficiency per
    /// harmonic (driver energy -> one harmonic) for an ~800 nm driver in
    /// the absorption-limited, phase-matched regime: heavy gases ~1e-5,
    /// Ne ~1e-7, He ~1e-8. Representative of best-reported values, with
    /// roughly one decade of uncertainty either way; override via
    /// `HhgSource::conversion_efficiency` with measured data.
    pub fn plateau_efficiency_800nm(&self) -> f64 {
        match self {
            HhgGas::Helium => 1e-8,
            HhgGas::Neon => 1e-7,
            HhgGas::Argon => 1e-5,
            HhgGas::Krypton => 2e-5,
            HhgGas::Xenon => 3e-5,
        }
    }
}

/// Monochromator selection of a single harmonic for imaging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarmonicSelection {
    /// Odd harmonic order q; the output wavelength is lambda_driver / q.
    pub harmonic: usize,
    /// Post-monochromator bandwidth FWHM in pm.
    pub bandwidth_pm: f64,
}

/// High-harmonic generation source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HhgSource {
    /// Driver laser wavelength in nm (800 Ti:Sapphire, 1030 Yb).
    pub driver_wavelength_nm: f64,
    /// Generation gas.
    pub gas: HhgGas,
    /// Driver peak intensity in W/cm^2 (sets the cutoff via U_p).
    pub driver_intensity_w_cm2: f64,
    /// `Some` = monochromatized single-harmonic imaging mode (default,
    /// honest for imaging); `None` = full-comb bookkeeping mode.
    pub monochromator: Option<HarmonicSelection>,
    /// Pulse energy in nJ (in-band, after monochromator). Superseded by
    /// the derived harmonic energy when `driver_average_power_w` is set (the
    /// constructors store that derived value here for reference).
    pub pulse_energy_nj: f64,
    /// Pulse repetition rate in Hz (kHz-MHz for modern drivers).
    pub rep_rate_hz: f64,
    /// Pulse duration in fs (documentary; no time-domain physics yet).
    pub pulse_duration_fs: f64,
    /// Spectral samples per line (in full-comb mode 1 per harmonic is
    /// enough for per-wavelength imaging: lines are ~1 % wide).
    pub spectral_samples: usize,
    /// Illumination pupil (laser-like coherence: tight Gaussian).
    pub illumination: IlluminationShape,
    /// Transverse coherence fraction (laser-like, ~0.9).
    pub transverse_coherence_fraction: f64,
    /// Optional driver average power in W. When set, the harmonic power
    /// (and `pulse_energy_j`) is DERIVED as `P_driver x eta_q`; `None` uses
    /// the stored `pulse_energy_nj`. The constructors set
    /// [`DEFAULT_DRIVER_AVERAGE_POWER_W`] (presets: their own value).
    #[serde(default)]
    pub driver_average_power_w: Option<f64>,
    /// Optional per-harmonic conversion efficiency override (measured
    /// value); `None` uses the order-of-magnitude estimate.
    #[serde(default)]
    pub conversion_efficiency: Option<f64>,
    /// Optional passband `[min_nm, max_nm]` applied to the full comb (a
    /// metal filter or multilayer band); `None` keeps every harmonic.
    #[serde(default)]
    pub comb_passband_nm: Option<[f64; 2]>,
}

impl HhgSource {
    /// Construct an HHG source with a selected harmonic, rejecting even
    /// harmonics and harmonics beyond the three-step-model cutoff for
    /// the given gas and intensity.
    pub fn new(
        driver_wavelength_nm: f64,
        gas: HhgGas,
        driver_intensity_w_cm2: f64,
        harmonic: usize,
        monochromator_bandwidth_pm: f64,
    ) -> crate::error::Result<Self> {
        if driver_wavelength_nm <= 0.0 || driver_wavelength_nm.is_nan() {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "driver_wavelength_nm",
                value: driver_wavelength_nm,
                reason: "must be positive",
            });
        }
        if driver_intensity_w_cm2 <= 0.0 || driver_intensity_w_cm2.is_nan() {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "driver_intensity_w_cm2",
                value: driver_intensity_w_cm2,
                reason: "must be positive",
            });
        }
        if harmonic == 0 || harmonic.is_multiple_of(2) {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "harmonic",
                value: harmonic as f64,
                reason: "HHG emits odd harmonics only (inversion symmetry of the gas)",
            });
        }

        let cutoff = Self::cutoff_energy_ev_for(gas, driver_intensity_w_cm2, driver_wavelength_nm);
        let photon_ev = harmonic as f64 * HC_EV_NM / driver_wavelength_nm;
        if photon_ev > cutoff {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "harmonic",
                value: harmonic as f64,
                reason: "harmonic energy exceeds the three-step-model cutoff \
                         I_p + 3.17 U_p for this gas and intensity",
            });
        }

        let coherence = 0.9;
        let mut src = Self {
            driver_wavelength_nm,
            gas,
            driver_intensity_w_cm2,
            monochromator: Some(HarmonicSelection {
                harmonic,
                bandwidth_pm: monochromator_bandwidth_pm,
            }),
            pulse_energy_nj: 0.0,
            rep_rate_hz: 10_000.0,
            pulse_duration_fs: 20.0,
            spectral_samples: 5,
            illumination: IlluminationShape::CoherentGaussian {
                sigma: sigma_from_coherence(coherence, 0.05),
            },
            transverse_coherence_fraction: coherence,
            // The power is DERIVED from an assumed driver (an earlier version
            // stored a flat 1 µW, and before that 1 mW, for every harmonic).
            driver_average_power_w: Some(DEFAULT_DRIVER_AVERAGE_POWER_W),
            conversion_efficiency: None,
            comb_passband_nm: None,
        };
        src.store_derived_pulse_energy();
        Ok(src)
    }

    /// Copy the derived harmonic pulse energy into the stored
    /// `pulse_energy_nj` (reference value; no-op without a driver power).
    fn store_derived_pulse_energy(&mut self) {
        if let (Some(p), true) = (self.derived_harmonic_power_w(), self.rep_rate_hz > 0.0) {
            self.pulse_energy_nj = p / self.rep_rate_hz * 1e9;
        }
    }

    /// Argon-driven 800 nm system, harmonic 27 -> 29.6 nm.
    /// At 2e14 W/cm^2 the cutoff is 53.6 eV, comfortably above the
    /// 41.8 eV of q = 27. An ASSUMED 3 W driver at 100 kHz derives
    /// 3 W x 1e-5 = 30 µW (0.3 nJ per pulse): tens of µW, below the measured
    /// record at this photon energy (~56 µW on the interpolated record
    /// curve; 0.14 mW demonstrated at 30 eV, Hädrich et al., Nat. Photonics
    /// 8, 779 (2014)); mW-class single-harmonic HHG has only been shown at
    /// 21.7–26.5 eV with fiber-laser drivers.
    pub fn ar_800nm_30nm() -> crate::error::Result<Self> {
        let mut src = Self::new(800.0, HhgGas::Argon, 2e14, 27, 30.0)?;
        src.rep_rate_hz = 100_000.0;
        src.driver_average_power_w = Some(3.0);
        src.store_derived_pulse_energy();
        Ok(src)
    }

    /// Neon-driven 800 nm system pushed to harmonic 59 -> 13.56 nm
    /// (the EUV mirror band). Requires 4e14 W/cm^2: cutoff 97.3 eV vs
    /// the 91.4 eV of q = 59. An ASSUMED 20 W driver at 10 kHz derives
    /// ~0.91 µW, between the measured 13.5 nm records (0.43 µW, KMLabs/JILA
    /// 2017; ~1 µW, 59th harmonic, Univ. of Hyogo / RIKEN, 2012).
    pub fn ne_800nm_13nm5() -> crate::error::Result<Self> {
        let mut src = Self::new(800.0, HhgGas::Neon, 4e14, 59, 15.0)?;
        src.rep_rate_hz = 10_000.0;
        src.driver_average_power_w = Some(20.0);
        src.store_derived_pulse_energy();
        Ok(src)
    }

    /// Three-step-model cutoff energy in eV for a gas / intensity /
    /// driver combination.
    pub fn cutoff_energy_ev_for(
        gas: HhgGas,
        intensity_w_cm2: f64,
        driver_wavelength_nm: f64,
    ) -> f64 {
        let up = physics::ponderomotive_ev(intensity_w_cm2, driver_wavelength_nm * 1e-3);
        physics::hhg_cutoff_ev(gas.ionization_potential_ev(), up)
    }

    /// Cutoff energy in eV for this source's configuration.
    pub fn cutoff_energy_ev(&self) -> f64 {
        Self::cutoff_energy_ev_for(
            self.gas,
            self.driver_intensity_w_cm2,
            self.driver_wavelength_nm,
        )
    }

    /// Ponderomotive energy U_p in eV.
    pub fn ponderomotive_energy_ev(&self) -> f64 {
        physics::ponderomotive_ev(
            self.driver_intensity_w_cm2,
            self.driver_wavelength_nm * 1e-3,
        )
    }

    /// Empirical plateau/rolloff envelope: flat up to 90 % of the cutoff,
    /// then an exponential rolloff with a 5 %-of-cutoff scale.
    fn envelope(photon_ev: f64, cutoff_ev: f64) -> f64 {
        let rolloff_start = 0.9 * cutoff_ev;
        if photon_ev <= rolloff_start {
            1.0
        } else {
            (-(photon_ev - rolloff_start) / (0.05 * cutoff_ev)).exp()
        }
    }

    /// The odd-harmonic comb from the plateau start (first harmonic above
    /// I_p) to the cutoff, with a flat plateau and an empirical
    /// exponential rolloff over the last ~10% below cutoff. Relative
    /// intensities are relative powers per harmonic. When
    /// `comb_passband_nm` is set, only harmonics inside it are kept.
    pub fn comb_lines(&self) -> Vec<SpectralLine> {
        let cutoff_ev = self.cutoff_energy_ev();
        let i_p = self.gas.ionization_potential_ev();
        let driver_ev = HC_EV_NM / self.driver_wavelength_nm;

        let mut lines = Vec::new();
        let mut q = 1;
        while (q as f64) * driver_ev <= cutoff_ev {
            let e_q = q as f64 * driver_ev;
            let center_nm = self.driver_wavelength_nm / q as f64;
            let in_band = match self.comb_passband_nm {
                Some([lo, hi]) => (lo.min(hi)..=lo.max(hi)).contains(&center_nm),
                None => true,
            };
            if e_q >= i_p && in_band {
                lines.push(SpectralLine {
                    center_nm,
                    // Individual harmonic width ~ driver bandwidth / q;
                    // use a fixed fraction of the line wavelength.
                    fwhm_pm: center_nm * 1e3 * 0.01,
                    relative_intensity: Self::envelope(e_q, cutoff_ev),
                });
            }
            q += 2;
        }
        lines
    }

    /// Selected harmonic order (`None` in full-comb mode).
    pub fn harmonic(&self) -> Option<usize> {
        self.monochromator.as_ref().map(|m| m.harmonic)
    }

    /// Estimated conversion efficiency (driver energy -> harmonic `q`):
    /// the override if set, else the gas's assumed 800 nm plateau
    /// efficiency x `(lambda_d / 800 nm)^-5.5` x the cutoff envelope.
    /// ORDER-OF-MAGNITUDE (see module docs).
    pub fn conversion_efficiency_for(&self, harmonic: usize) -> f64 {
        if let Some(ce) = self.conversion_efficiency {
            return ce;
        }
        let photon_ev = harmonic as f64 * HC_EV_NM / self.driver_wavelength_nm;
        self.gas.plateau_efficiency_800nm()
            * (self.driver_wavelength_nm / 800.0).powf(-HHG_WAVELENGTH_SCALING_EXPONENT)
            * Self::envelope(photon_ev, self.cutoff_energy_ev())
    }

    /// Derived in-band harmonic power in W (`P_driver x eta_q` for the
    /// selected harmonic); `None` without a driver power or in comb mode.
    pub fn derived_harmonic_power_w(&self) -> Option<f64> {
        let p = self.driver_average_power_w?;
        let q = self.harmonic()?;
        Some(p * self.conversion_efficiency_for(q))
    }
}

impl Default for HhgSource {
    fn default() -> Self {
        Self::ar_800nm_30nm().expect("Ar preset is valid")
    }
}

impl LithographySource for HhgSource {
    /// Selected-harmonic wavelength in monochromatized mode; the median
    /// plateau harmonic's wavelength in full-comb mode.
    fn wavelength_nm(&self) -> f64 {
        match &self.monochromator {
            Some(sel) => self.driver_wavelength_nm / sel.harmonic as f64,
            None => {
                let lines = self.comb_lines();
                if lines.is_empty() {
                    self.driver_wavelength_nm
                } else {
                    lines[lines.len() / 2].center_nm
                }
            }
        }
    }

    fn bandwidth_pm(&self) -> f64 {
        match &self.monochromator {
            Some(sel) => sel.bandwidth_pm,
            None => {
                // Full comb: report the span from longest to shortest line.
                let lines = self.comb_lines();
                match (lines.first(), lines.last()) {
                    (Some(a), Some(b)) => (a.center_nm - b.center_nm).abs() * 1e3,
                    _ => 0.0,
                }
            }
        }
    }

    fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64 {
        evaluate_illumination(&self.illumination, fx_norm, fy_norm)
    }

    fn spectral_weights(&self) -> Vec<(f64, f64)> {
        match &self.monochromator {
            Some(sel) => evaluate_spectral_weights(
                self.driver_wavelength_nm / sel.harmonic as f64,
                sel.bandwidth_pm,
                self.spectral_samples,
                &SpectralShape::Gaussian,
            ),
            None => evaluate_multiline_weights(
                &self.comb_lines(),
                self.spectral_samples,
                &SpectralShape::Gaussian,
            ),
        }
    }

    /// Stored in-band pulse energy, or the derived harmonic power divided
    /// by the repetition rate when a driver power is given.
    fn pulse_energy_j(&self) -> Option<f64> {
        match self.derived_harmonic_power_w() {
            Some(p) if self.rep_rate_hz > 0.0 => Some(p / self.rep_rate_hz),
            _ => Some(self.pulse_energy_nj * 1e-9),
        }
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

    fn derived_quantities(&self) -> Vec<DerivedQuantity> {
        let up = self.ponderomotive_energy_ev();
        let cutoff = self.cutoff_energy_ev();
        let i_p = self.gas.ionization_potential_ev();
        let lambda = self.wavelength_nm();
        let mut out = vec![
            DerivedQuantity::new(
                "photon_energy",
                HC_EV_NM / lambda,
                "eV",
                "hc / lambda (selected harmonic; median comb line in comb mode)",
            ),
            DerivedQuantity::new("ponderomotive_energy", up, "eV", "9.33e-14 I lambda^2"),
            DerivedQuantity::new(
                "cutoff_energy",
                cutoff,
                "eV",
                "single-atom cutoff I_p + 3.17 U_p",
            ),
            DerivedQuantity::new(
                "keldysh_parameter",
                physics::keldysh_parameter(i_p, up),
                "-",
                "sqrt(I_p / 2 U_p); < 1 = tunnelling regime of the three-step model",
            ),
            DerivedQuantity::new(
                "critical_ionization",
                physics::hhg_critical_ionization(
                    self.gas.refractivity_stp(),
                    self.driver_wavelength_nm,
                ),
                "-",
                "phase matching is lost above this ionization fraction: the phase-matched \
                 cutoff lies below I_p + 3.17 U_p when the generating intensity ionizes more",
            ),
            DerivedQuantity::new(
                "wavelength_scaling_factor",
                (self.driver_wavelength_nm / 800.0).powf(-HHG_WAVELENGTH_SCALING_EXPONENT),
                "-",
                "single-atom yield relative to an 800 nm driver, (lambda/800 nm)^-5.5",
            ),
        ];
        let power = self.average_power_w().unwrap_or(0.0);
        if let Some(q) = self.harmonic() {
            let ce = self.conversion_efficiency_for(q);
            let ce_note = if self.conversion_efficiency.is_some() {
                "user-supplied conversion efficiency"
            } else {
                "ORDER-OF-MAGNITUDE estimate: assumed 800 nm plateau efficiency x \
                 (lambda/800)^-5.5 x cutoff envelope (+-1 decade)"
            };
            out.push(DerivedQuantity::new(
                "conversion_efficiency",
                ce,
                "-",
                ce_note,
            ));
            if let Some(p) = self.derived_harmonic_power_w() {
                out.push(DerivedQuantity::new(
                    "derived_harmonic_power",
                    p,
                    "W",
                    "driver average power x conversion efficiency (= average_power_w)",
                ));
            } else if ce > 0.0 {
                out.push(DerivedQuantity::new(
                    "implied_driver_power",
                    power / ce,
                    "W",
                    "driver average power the stored XUV pulse energy implies at this efficiency",
                ));
            }
            if ce > 0.0 {
                out.push(DerivedQuantity::new(
                    "driver_power_for_hvm",
                    super::throughput::HVM_EUV_POWER_AT_IF_W / ce,
                    "W",
                    "driver average power needed for 250 W in this harmonic",
                ));
            }
            if let Some(record) = measured_power_record_w(HC_EV_NM / lambda) {
                out.push(DerivedQuantity::new(
                    "measured_power_record",
                    record,
                    "W",
                    "best measured single-harmonic HHG power at this photon energy: 12.9 mW \
                     <= 26.5 eV (Klas 2021), 0.144 mW <= 30 eV (Hadrich 2014), log-interpolated \
                     (no records between) to ~1 uW at 92 eV / 13.5 nm (Hyogo/RIKEN 2012)",
                ));
                let ratio = power / record;
                out.push(DerivedQuantity::new(
                    "power_vs_measured_record",
                    ratio,
                    "-",
                    if ratio > 1.0 {
                        "> 1: PROJECTION - above the measured HHG record at this photon energy"
                    } else {
                        "<= 1: within demonstrated HHG power at this photon energy"
                    },
                ));
            }
        } else {
            out.push(DerivedQuantity::new(
                "comb_line_count",
                self.comb_lines().len() as f64,
                "-",
                "odd harmonics between I_p and the cutoff (inside the passband, if set)",
            ));
        }
        out.push(DerivedQuantity::new(
            "average_power",
            power,
            "W",
            if self.derived_harmonic_power_w().is_some() {
                "DERIVED driver power x conversion efficiency (measured 13.5 nm HHG: \
                 ~60 nW to ~1 uW; mW-class only at 21.7-26.5 eV)"
            } else {
                "stored pulse energy x repetition rate (measured 13.5 nm HHG: ~60 nW to \
                 ~1 uW; mW-class only at 21.7-26.5 eV)"
            },
        ));
        out.push(DerivedQuantity::new(
            "hvm_power_ratio",
            power / super::throughput::HVM_EUV_POWER_AT_IF_W,
            "-",
            "average power / 250 W (production 13.5 nm HVM at IF)",
        ));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn dq(src: &HhgSource, name: &str) -> f64 {
        src.derived_quantities()
            .into_iter()
            .find(|q| q.name == name)
            .unwrap_or_else(|| panic!("missing derived quantity {name}"))
            .value
    }

    #[test]
    fn test_comb_spacing_is_two_driver_photons() {
        let src = HhgSource {
            monochromator: None,
            ..HhgSource::ar_800nm_30nm().unwrap()
        };
        let lines = src.comb_lines();
        assert!(
            lines.len() >= 3,
            "Ar at 2e14 should have several plateau harmonics"
        );
        let driver_ev = HC_EV_NM / 800.0;
        for pair in lines.windows(2) {
            let e0 = HC_EV_NM / pair[0].center_nm;
            let e1 = HC_EV_NM / pair[1].center_nm;
            assert_relative_eq!(e1 - e0, 2.0 * driver_ev, epsilon = 1e-9);
        }
    }

    #[test]
    fn test_cutoff_rejection() {
        // Ar at 2e14 W/cm^2: cutoff 53.6 eV; q = 59 (91.4 eV) must be rejected.
        assert!(HhgSource::new(800.0, HhgGas::Argon, 2e14, 59, 15.0).is_err());
        // Ne at 4e14 (cutoff 97.3 eV) accepts q = 59.
        assert!(HhgSource::new(800.0, HhgGas::Neon, 4e14, 59, 15.0).is_ok());
    }

    #[test]
    fn test_even_harmonic_rejected() {
        assert!(HhgSource::new(800.0, HhgGas::Argon, 2e14, 26, 15.0).is_err());
        assert!(HhgSource::new(800.0, HhgGas::Argon, 2e14, 0, 15.0).is_err());
    }

    #[test]
    fn test_preset_wavelengths() {
        let ar = HhgSource::ar_800nm_30nm().unwrap();
        assert_relative_eq!(ar.wavelength_nm(), 800.0 / 27.0, epsilon = 1e-9);
        assert!((29.0..30.0).contains(&ar.wavelength_nm()));

        let ne = HhgSource::ne_800nm_13nm5().unwrap();
        assert_relative_eq!(ne.wavelength_nm(), 800.0 / 59.0, epsilon = 1e-9);
        assert!((13.4..13.7).contains(&ne.wavelength_nm()));
    }

    #[test]
    fn test_weights_sum_to_one_in_both_modes() {
        let mono = HhgSource::ar_800nm_30nm().unwrap();
        let mono_weights = mono.spectral_weights();
        let sum: f64 = mono_weights.iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-12);

        let comb = HhgSource {
            monochromator: None,
            ..mono
        };
        let comb_weights = comb.spectral_weights();
        let sum: f64 = comb_weights.iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
        // Comb mode spans many lines.
        assert!(comb_weights.len() > mono_weights.len());
    }

    #[test]
    fn test_plateau_starts_above_ionization_potential() {
        let src = HhgSource {
            monochromator: None,
            ..HhgSource::ar_800nm_30nm().unwrap()
        };
        let i_p = src.gas.ionization_potential_ev();
        for line in src.comb_lines() {
            assert!(HC_EV_NM / line.center_nm >= i_p);
        }
    }

    #[test]
    fn test_comb_passband_filter() {
        // Ar comb spans q = 11..33 (72.7 -> 24.2 nm); a 25-35 nm filter
        // keeps q = 23..31 (34.8 .. 25.8 nm); q = 21 (38.1 nm) and
        // q = 33 (24.2 nm) fall outside.
        let src = HhgSource {
            monochromator: None,
            comb_passband_nm: Some([25.0, 35.0]),
            spectral_samples: 1,
            ..HhgSource::ar_800nm_30nm().unwrap()
        };
        let lines = src.comb_lines();
        assert!(!lines.is_empty());
        for l in &lines {
            assert!((25.0..=35.0).contains(&l.center_nm), "{}", l.center_nm);
        }
        let orders: Vec<usize> = lines
            .iter()
            .map(|l| (800.0 / l.center_nm).round() as usize)
            .collect();
        assert_eq!(orders, vec![23, 25, 27, 29, 31]);
        // One sample per harmonic: one TCC per line in per-wavelength imaging.
        assert_eq!(src.spectral_weights().len(), lines.len());
        assert_relative_eq!(dq(&src, "comb_line_count"), 5.0);
    }

    #[test]
    fn test_pulse_metadata_live() {
        let src = HhgSource::ar_800nm_30nm().unwrap();
        // 3 W x 1e-5 = 30 uW; / 100 kHz = 0.3 nJ per pulse (also stored).
        assert_relative_eq!(src.average_power_w().unwrap(), 3.0e-5, max_relative = 1e-12);
        assert_relative_eq!(src.pulse_energy_j().unwrap(), 3.0e-10, max_relative = 1e-12);
        assert_relative_eq!(src.pulse_energy_nj, 0.3, max_relative = 1e-12);
        // 13.5 nm preset: 20 W x 4.5e-8 = 0.91 uW, between the measured
        // 0.43 uW and ~1 uW records.
        let ne = HhgSource::ne_800nm_13nm5().unwrap();
        let p_ne = ne.average_power_w().unwrap();
        assert_relative_eq!(
            p_ne,
            20.0 * ne.conversion_efficiency_for(59),
            max_relative = 1e-12
        );
        assert!((4.3e-7..=1.0e-6).contains(&p_ne), "{p_ne}");
        // Generic constructor: the default 5 W driver -> ~0.23 uW at q = 59.
        let generic = HhgSource::new(800.0, HhgGas::Neon, 4e14, 59, 15.0).unwrap();
        assert_relative_eq!(
            generic.average_power_w().unwrap(),
            p_ne * DEFAULT_DRIVER_AVERAGE_POWER_W / 20.0,
            max_relative = 1e-12
        );
        assert_relative_eq!(src.transverse_coherence(), 0.9);
    }

    #[test]
    fn test_power_anchored_to_measured_records() {
        // Record curve: 12.9 mW <= 26.5 eV, 0.144 mW to 30 eV, log-linear to
        // 1 uW at 92 eV, nothing above.
        assert_relative_eq!(measured_power_record_w(21.7).unwrap(), 1.29e-2);
        assert_relative_eq!(measured_power_record_w(30.0).unwrap(), 1.44e-4);
        assert_relative_eq!(
            measured_power_record_w(92.0).unwrap(),
            1.0e-6,
            max_relative = 1e-12
        );
        assert!(measured_power_record_w(92.5).is_none());
        assert_relative_eq!(
            measured_power_record_w(45.0).unwrap(),
            1.44e-4 * (1e-6 / 1.44e-4_f64).powf(15.0 / 62.0),
            max_relative = 1e-12
        );
        // The presets and the generic defaults sit at or below the record.
        for src in [
            HhgSource::ar_800nm_30nm().unwrap(),
            HhgSource::ne_800nm_13nm5().unwrap(),
            HhgSource::new(800.0, HhgGas::Neon, 4e14, 59, 15.0).unwrap(),
            HhgSource::new(800.0, HhgGas::Argon, 2e14, 27, 30.0).unwrap(),
        ] {
            assert!(dq(&src, "power_vs_measured_record") <= 1.0, "{:?}", src.gas);
        }
        // No default 800 nm configuration returns mW-class power for a
        // sub-45 nm harmonic, in any gas (the maximum is Xe on the plateau:
        // 5 W x 3e-5 = 0.15 mW).
        for gas in [
            HhgGas::Helium,
            HhgGas::Neon,
            HhgGas::Argon,
            HhgGas::Krypton,
            HhgGas::Xenon,
        ] {
            for intensity in [1e14, 2e14, 4e14, 8e14] {
                for q in (19..200).step_by(2) {
                    if let Ok(src) = HhgSource::new(800.0, gas, intensity, q, 15.0) {
                        let p = src.average_power_w().unwrap();
                        assert!(p < 2e-4, "{gas:?} q = {q}: {p} W");
                    }
                }
            }
        }
        // An explicit 50 W Ar driver derives 0.5 mW at 41.8 eV: a projection.
        let mut big = HhgSource::ar_800nm_30nm().unwrap();
        big.driver_average_power_w = Some(50.0);
        let flag = big
            .derived_quantities()
            .into_iter()
            .find(|q| q.name == "power_vs_measured_record")
            .unwrap();
        assert!(flag.value > 1.0 && flag.note.contains("PROJECTION"));
    }

    #[test]
    fn test_keldysh_and_critical_ionization() {
        let ar = HhgSource::ar_800nm_30nm().unwrap();
        assert_relative_eq!(
            dq(&ar, "keldysh_parameter"),
            0.812_301_587_613_254_5,
            max_relative = 1e-9
        );
        assert_relative_eq!(
            dq(&ar, "critical_ionization"),
            0.035_155_982_993_349_26,
            max_relative = 1e-6
        );
        let ne = HhgSource::ne_800nm_13nm5().unwrap();
        // U_p = 23.88 eV -> gamma_K = sqrt(21.565 / 47.77) = 0.672
        assert_relative_eq!(
            dq(&ne, "keldysh_parameter"),
            0.671_891_162_658_607_4,
            max_relative = 1e-9
        );
        // Neon tolerates ~4x less ionization than argon.
        assert!(dq(&ne, "critical_ionization") < 0.3 * dq(&ar, "critical_ionization"));
    }

    #[test]
    fn test_efficiency_estimate_and_scaling() {
        let ar = HhgSource::ar_800nm_30nm().unwrap();
        // q = 27 (41.8 eV) sits on the plateau (< 0.9 x 53.6 eV): anchor 1e-5.
        assert_relative_eq!(ar.conversion_efficiency_for(27), 1e-5, max_relative = 1e-12);
        // The 3 W driver derives 30 uW; 250 W needs 25 MW of driver.
        assert_relative_eq!(dq(&ar, "derived_harmonic_power"), 3e-5, max_relative = 1e-9);
        assert_relative_eq!(dq(&ar, "driver_power_for_hvm"), 2.5e7, max_relative = 1e-9);
        // Without a driver power the stored pulse energy is used and the
        // driver power it implies is reported: 1 nJ x 100 kHz -> 10 W.
        let stored = HhgSource {
            driver_average_power_w: None,
            pulse_energy_nj: 1.0,
            ..ar.clone()
        };
        assert_relative_eq!(
            stored.average_power_w().unwrap(),
            1e-4,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            dq(&stored, "implied_driver_power"),
            10.0,
            max_relative = 1e-9
        );

        // Driver-wavelength law: a 1030 nm driver at the same photon energy
        // pays (1030/800)^-5.5 = 0.2491.
        let yb = HhgSource {
            driver_wavelength_nm: 1030.0,
            ..ar.clone()
        };
        assert_relative_eq!(
            dq(&yb, "wavelength_scaling_factor"),
            0.249_109_444_097_418_8,
            max_relative = 1e-9
        );

        // Ne q = 59 sits in the rolloff: 91.4 eV vs 0.9 x 97.28 eV cutoff.
        let ne = HhgSource::ne_800nm_13nm5().unwrap();
        let e_q = 59.0 * HC_EV_NM / 800.0;
        let cutoff = ne.cutoff_energy_ev();
        let env = (-(e_q - 0.9 * cutoff) / (0.05 * cutoff)).exp();
        assert_relative_eq!(
            ne.conversion_efficiency_for(59),
            1e-7 * env,
            max_relative = 1e-12
        );
        assert!(env < 0.5 && env > 0.4);
    }

    #[test]
    fn test_derived_power_from_driver() {
        let mut src = HhgSource::ar_800nm_30nm().unwrap();
        src.driver_average_power_w = Some(50.0);
        // 50 W x 1e-5 = 0.5 mW; pulse energy = 0.5 mW / 100 kHz = 5 nJ
        assert_relative_eq!(src.average_power_w().unwrap(), 5e-4, max_relative = 1e-12);
        assert_relative_eq!(src.pulse_energy_j().unwrap(), 5e-9, max_relative = 1e-12);
        assert_relative_eq!(
            dq(&src, "derived_harmonic_power"),
            5e-4,
            max_relative = 1e-12
        );
        // A measured efficiency overrides the estimate.
        src.conversion_efficiency = Some(2e-6);
        assert_relative_eq!(src.average_power_w().unwrap(), 1e-4, max_relative = 1e-12);
    }
}
