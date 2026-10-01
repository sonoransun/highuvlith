//! Laser-wakefield (LWFA) betatron X-ray source.
//!
//! An intense laser pulse drives a plasma wake in the blow-out ("bubble")
//! regime; electrons trapped in the ion cavity are accelerated to hundreds of
//! MeV and, because the uniform ion column exerts a linear restoring force,
//! they oscillate transversely at the betatron frequency while they
//! accelerate. The plasma thereby acts as a mm-long, µm-amplitude wiggler
//! with a period of a few hundred µm, radiating a femtosecond,
//! synchrotron-like keV X-ray burst from a µm-size source in a ~10 mrad cone.
//! The source is compact and ultrafast but delivers ~10⁸–10⁹ photons per
//! shot at ≲10 Hz (100-TW-class drivers) — µW average power. Betatron X-rays
//! are keV, broadband, ~10 Hz sources used for (phase-contrast) imaging, not
//! lithography: for lithography this family is a research curiosity (🧪),
//! relevant at most to LIGA-style shadow printing via
//! [`BetatronSource::xray_spectrum`].
//!
//! # Key equations
//!
//! - Plasma frequency: `omega_p = sqrt(n_e e^2 / (eps0 m_e))`
//!   (`≈ 5.64e4 sqrt(n_e[cm^-3])` rad/s); `lambda_p = 2 pi c / omega_p`.
//! - Betatron frequency (ion channel, `gamma ≫ 1`):
//!   `omega_beta = omega_p / sqrt(2 gamma)`; `lambda_beta = lambda_p sqrt(2 gamma)`.
//! - Strength parameter: `K = gamma k_beta r_beta = gamma omega_beta r_beta / c`
//!   (`≈ 1.33e-10 sqrt(gamma n_e[cm^-3]) r_beta[um]`).
//! - Critical energy (wiggler regime `K ≫ 1`), from the synchrotron
//!   `omega_c = (3/2) gamma^3 c / rho` at the orbit's minimum curvature radius
//!   `1/rho = r_beta k_beta^2`:
//!   `hbar omega_c = (3/2) gamma^3 hbar omega_beta^2 r_beta / c
//!                 = (3/2) K gamma^2 hbar omega_beta`
//!   (`≈ 5.24e-24 gamma^2 n_e[cm^-3] r_beta[um]` keV). Note the `gamma^3`:
//!   the variant `(3/2) gamma^2 hbar omega_beta^2 r_beta / c` sometimes seen
//!   is dimensionally consistent but low by a factor `gamma`.
//! - Photons per period per electron (PLANAR orbit `x = r_beta sin(omega_beta t)`):
//!   the Larmor energy per period `W = e^2 gamma^2 K^2 omega_beta / (6 eps0 c)`
//!   divided by the mean synchrotron photon energy
//!   `(8 / (15 sqrt 3)) hbar omega_c` gives
//!   `N_gamma = (5 pi sqrt 3 / 6) alpha K ≈ 3.31e-2 K`. A helical (circular)
//!   orbit of the same amplitude would radiate twice the energy and photons
//!   per period.
//! - Spectrum (per shot): `dN/dE = N_shot S(E/E_c) / (E (5 pi / 3))` with the
//!   bending-magnet universal function `S(y) = y Int_y^inf K_{5/3}`
//!   ([`super::physics::bm_universal_flux`]); `Int S(y)/y dy = 5 pi / 3`.
//! - Divergence half-angle `theta ≈ K / gamma`.
//!
//! # Model status
//!
//! ✅ the formula layer (ω_p, ω_β, K, ħω_c, N_γ, the synchrotron-like
//! spectrum shape) is textbook and fixture-tested against an independent
//! NumPy evaluation. 🧪 as a lithography source. This is a **planar-orbit,
//! synchrotron-like (S(y)) model** with these assumptions: every electron
//! follows a planar sinusoidal orbit (the ensemble of randomly oriented orbit
//! planes makes the round flat-top cone of half-angle K/γ used for the flux
//! density; a helical orbit would double the photon count); a single electron
//! energy, a single betatron amplitude and a constant γ over the interaction
//! length (real betatron spectra are ensemble averages over energy, amplitude
//! and the acceleration history, usually fitted by an effective
//! synchrotron-like E_c — which is what this model provides); and the photon
//! count of the peak critical energy (the orbit-averaged spectrum is somewhat
//! softer). The undulator regime (K < 1) is rejected rather than
//! mis-modelled. Shot-to-shot jitter (default 30% rms) is an assumption
//! reflecting the tens-of-percent fluctuations typical of LWFA sources.
//!
//! [`LithographySource::wavelength_nm`] reports `hc / <E>` — the wavelength
//! at the mean photon energy `0.3079 E_c` (the photon-weighted mean
//! *wavelength* of a synchrotron spectrum diverges); `bandwidth_pm` is
//! nominal Δλ/λ = 1 (broadband).
//!
//! # References
//!
//! - S. Corde et al., "Femtosecond x rays from laser-plasma accelerators,"
//!   Rev. Mod. Phys. 85, 1 (2013) — review; the K, ħω_c and N_γ forms above.
//! - A. Rousse et al., "Production of a keV X-ray beam from synchrotron
//!   radiation in relativistic laser-plasma interaction," Phys. Rev. Lett.
//!   93, 135005 (2004) — first betatron X-rays from a laser-plasma
//!   accelerator (betatron radiation from an electron-beam-driven plasma had
//!   been reported earlier).
//! - E. Esarey et al., "Synchrotron radiation from electron beams in
//!   plasma-focusing channels," Phys. Rev. E 65, 056505 (2002).

use serde::{Deserialize, Serialize};

use super::dpp::HVM_REFERENCE_POWER_W;
use super::physics::{
    bm_universal_flux, coherent_fraction_per_plane, gamma_from_mev, plasma_critical_density_cm3,
    ELECTRON_CHARGE_C, ELECTRON_MASS_KG, EPSILON_0_F_M, FINE_STRUCTURE, HBAR_J_S, HC_EV_NM,
    SPEED_OF_LIGHT_M_S,
};
use crate::deep_xray::XraySpectrum;
use crate::source::{evaluate_illumination, DerivedQuantity, IlluminationShape, LithographySource};

/// `hc` in keV·nm.
const HC_KEV_NM: f64 = HC_EV_NM * 1e-3;
/// Drive-laser wavelength (µm) assumed for the underdense check: a 0.8 µm
/// Ti:sapphire LWFA driver (critical density ≈ 1.74e21 cm⁻³).
const DRIVER_WAVELENGTH_UM: f64 = 0.8;
/// Mean photon energy of a synchrotron spectrum in units of E_c,
/// `8 / (15 sqrt 3)`.
pub const MEAN_PHOTON_ENERGY_OVER_EC: f64 = 0.307_920_143_567_800_4;
/// Spectral window (in units of E_c) used for the sampled spectra — the same
/// 0.1–8 E_c window the LIGA module uses for a bending magnet.
const SPECTRUM_Y_MIN: f64 = 0.1;
const SPECTRUM_Y_MAX: f64 = 8.0;

/// Plasma frequency in rad/s for an electron density in cm⁻³.
pub fn plasma_frequency_rad_s(density_cm3: f64) -> f64 {
    (density_cm3 * 1e6 * ELECTRON_CHARGE_C * ELECTRON_CHARGE_C / (EPSILON_0_F_M * ELECTRON_MASS_KG))
        .sqrt()
}

/// Betatron frequency `omega_p / sqrt(2 gamma)` in rad/s.
pub fn betatron_frequency_rad_s(density_cm3: f64, gamma: f64) -> f64 {
    plasma_frequency_rad_s(density_cm3) / (2.0 * gamma).sqrt()
}

/// Betatron strength parameter `K = gamma omega_beta r_beta / c`.
pub fn betatron_strength(density_cm3: f64, gamma: f64, amplitude_um: f64) -> f64 {
    gamma * betatron_frequency_rad_s(density_cm3, gamma) * amplitude_um * 1e-6 / SPEED_OF_LIGHT_M_S
}

/// Wiggler-regime critical photon energy in keV:
/// `(3/2) gamma^3 hbar omega_beta^2 r_beta / c`.
pub fn betatron_critical_energy_kev(density_cm3: f64, gamma: f64, amplitude_um: f64) -> f64 {
    let wb = betatron_frequency_rad_s(density_cm3, gamma);
    1.5 * gamma.powi(3) * HBAR_J_S * wb * wb * amplitude_um * 1e-6
        / SPEED_OF_LIGHT_M_S
        / ELECTRON_CHARGE_C
        * 1e-3
}

/// Photons emitted per betatron period per electron,
/// `(5 pi sqrt 3 / 6) alpha K ≈ 3.31e-2 K`.
pub fn photons_per_period_per_electron(k: f64) -> f64 {
    5.0 * std::f64::consts::PI * 3.0_f64.sqrt() / 6.0 * FINE_STRUCTURE * k
}

/// LWFA betatron X-ray source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetatronSource {
    /// Electron kinetic energy in MeV (sets gamma).
    pub electron_energy_mev: f64,
    /// Plasma electron density in cm⁻³ (10¹⁸–10¹⁹ typical).
    pub plasma_density_cm3: f64,
    /// Betatron oscillation amplitude r_β in µm (~0.5–2 µm).
    pub betatron_amplitude_um: f64,
    /// Length over which the electrons oscillate, in mm (sets the number of
    /// betatron periods).
    pub interaction_length_mm: f64,
    /// Radiating (trapped) bunch charge in pC.
    pub bunch_charge_pc: f64,
    /// Shot repetition rate in Hz (≲10 Hz for 100-TW-class drivers).
    pub rep_rate_hz: f64,
    /// X-ray pulse duration in fs (≈ electron bunch duration).
    pub pulse_duration_fs: f64,
    /// Relative rms shot-to-shot X-ray energy fluctuation (assumed).
    pub shot_to_shot_rms: f64,
    /// Number of spectral samples for `spectral_weights()`.
    pub spectral_samples: usize,
    /// Illumination pupil shape (not a projection-imaging source).
    pub illumination: IlluminationShape,
}

impl BetatronSource {
    /// Construct from the main machine parameters with default bunch
    /// timing: 10 fs pulses, 30% rms jitter, 16 spectral samples.
    pub fn new(
        electron_energy_mev: f64,
        plasma_density_cm3: f64,
        betatron_amplitude_um: f64,
        interaction_length_mm: f64,
        bunch_charge_pc: f64,
        rep_rate_hz: f64,
    ) -> crate::error::Result<Self> {
        let src = Self {
            electron_energy_mev,
            plasma_density_cm3,
            betatron_amplitude_um,
            interaction_length_mm,
            bunch_charge_pc,
            rep_rate_hz,
            pulse_duration_fs: 10.0,
            shot_to_shot_rms: 0.3,
            spectral_samples: 16,
            illumination: IlluminationShape::Conventional { sigma: 1.0 },
        };
        src.validate()?;
        Ok(src)
    }

    /// Illustrative 100-TW-class LWFA betatron point within published
    /// ranges: 200 MeV electrons, n_e = 1e19 cm⁻³, r_β = 1 µm, 3 mm, 50 pC,
    /// 10 Hz → K ≈ 8.3, E_c ≈ 8.1 keV, ≈9e8 photons/shot.
    pub fn lwfa_100tw() -> crate::error::Result<Self> {
        Self::new(200.0, 1e19, 1.0, 3.0, 50.0, 10.0)
    }

    /// Check every field (including the wiggler-regime assumption K ≥ 1);
    /// called by the constructors and available to frontends that override
    /// fields after construction.
    pub fn validate(&self) -> crate::error::Result<()> {
        use crate::error::LithographyError::InvalidParameter;
        if !(self.electron_energy_mev.is_finite() && self.electron_energy_mev >= 10.0) {
            return Err(InvalidParameter {
                name: "electron_energy_mev",
                value: self.electron_energy_mev,
                reason: "must be >= 10 MeV (omega_beta = omega_p/sqrt(2 gamma) assumes gamma >> 1)",
            });
        }
        if !(self.plasma_density_cm3 > 0.0
            && self.plasma_density_cm3 < plasma_critical_density_cm3(DRIVER_WAVELENGTH_UM))
        {
            return Err(InvalidParameter {
                name: "plasma_density_cm3",
                value: self.plasma_density_cm3,
                reason: "must be positive and underdense (< 1.74e21 cm^-3 for a 0.8 um driver)",
            });
        }
        for (name, value) in [
            ("betatron_amplitude_um", self.betatron_amplitude_um),
            ("interaction_length_mm", self.interaction_length_mm),
            ("bunch_charge_pc", self.bunch_charge_pc),
            ("rep_rate_hz", self.rep_rate_hz),
            ("pulse_duration_fs", self.pulse_duration_fs),
        ] {
            if !(value.is_finite() && value > 0.0) {
                return Err(InvalidParameter {
                    name,
                    value,
                    reason: "must be positive",
                });
            }
        }
        if !(self.shot_to_shot_rms >= 0.0 && self.shot_to_shot_rms < 1.0) {
            return Err(InvalidParameter {
                name: "shot_to_shot_rms",
                value: self.shot_to_shot_rms,
                reason: "must be in [0, 1)",
            });
        }
        if self.spectral_samples == 0 {
            return Err(InvalidParameter {
                name: "spectral_samples",
                value: 0.0,
                reason: "must be >= 1",
            });
        }
        let k = self.strength_k();
        if k < 1.0 {
            return Err(InvalidParameter {
                name: "betatron_strength_k",
                value: k,
                reason: "K < 1 is the undulator regime (narrow harmonics), not modelled; \
                         raise the amplitude, density or energy",
            });
        }
        Ok(())
    }

    /// Electron Lorentz factor.
    pub fn gamma(&self) -> f64 {
        gamma_from_mev(self.electron_energy_mev)
    }

    /// Plasma wavelength `2 pi c / omega_p` in µm.
    pub fn plasma_wavelength_um(&self) -> f64 {
        2.0 * std::f64::consts::PI * SPEED_OF_LIGHT_M_S
            / plasma_frequency_rad_s(self.plasma_density_cm3)
            * 1e6
    }

    /// Betatron frequency in rad/s.
    pub fn betatron_frequency_rad_s(&self) -> f64 {
        betatron_frequency_rad_s(self.plasma_density_cm3, self.gamma())
    }

    /// Betatron period (wavelength) `2 pi c / omega_beta` in µm.
    pub fn betatron_wavelength_um(&self) -> f64 {
        2.0 * std::f64::consts::PI * SPEED_OF_LIGHT_M_S / self.betatron_frequency_rad_s() * 1e6
    }

    /// Strength parameter K.
    pub fn strength_k(&self) -> f64 {
        betatron_strength(
            self.plasma_density_cm3,
            self.gamma(),
            self.betatron_amplitude_um,
        )
    }

    /// Critical photon energy in keV.
    pub fn critical_energy_kev(&self) -> f64 {
        betatron_critical_energy_kev(
            self.plasma_density_cm3,
            self.gamma(),
            self.betatron_amplitude_um,
        )
    }

    /// Number of betatron periods over the interaction length.
    pub fn num_periods(&self) -> f64 {
        self.interaction_length_mm * 1e3 / self.betatron_wavelength_um()
    }

    /// Radiating electrons per bunch, `Q / e`.
    pub fn electrons_per_bunch(&self) -> f64 {
        self.bunch_charge_pc * 1e-12 / ELECTRON_CHARGE_C
    }

    /// Photons per shot (all energies), `N_e N_beta N_gamma`.
    pub fn photons_per_shot(&self) -> f64 {
        self.electrons_per_bunch()
            * self.num_periods()
            * photons_per_period_per_electron(self.strength_k())
    }

    /// Mean photon energy `0.3079 E_c` in keV.
    pub fn mean_photon_energy_kev(&self) -> f64 {
        MEAN_PHOTON_ENERGY_OVER_EC * self.critical_energy_kev()
    }

    /// X-ray energy per shot in J.
    pub fn xray_energy_per_shot_j(&self) -> f64 {
        self.photons_per_shot() * self.mean_photon_energy_kev() * 1e3 * ELECTRON_CHARGE_C
    }

    /// Divergence half-angle `K / gamma` in mrad.
    pub fn divergence_mrad(&self) -> f64 {
        self.strength_k() / self.gamma() * 1e3
    }

    /// Photons per shot binned on `n_bins` log-spaced energy bins over
    /// `[0.1, 8] E_c`: `(bin-center keV, photons in bin)` with
    /// `N_shot S(y) Delta(ln y) / (5 pi / 3)` (midpoint in ln y).
    pub fn photon_spectrum(&self, n_bins: usize) -> Vec<(f64, f64)> {
        let n = n_bins.max(1);
        let e_c = self.critical_energy_kev();
        let n_shot = self.photons_per_shot();
        let norm = 5.0 * std::f64::consts::PI / 3.0;
        let dln = (SPECTRUM_Y_MAX / SPECTRUM_Y_MIN).ln() / n as f64;
        (0..n)
            .map(|j| {
                let y = SPECTRUM_Y_MIN * ((j as f64 + 0.5) * dln).exp();
                (y * e_c, n_shot * bm_universal_flux(y) * dln / norm)
            })
            .collect()
    }

    /// Relative spectrum for the LIGA module: the synchrotron-like
    /// `XraySpectrum::BendingMagnet` at this source's critical energy.
    pub fn xray_spectrum(&self) -> XraySpectrum {
        XraySpectrum::BendingMagnet {
            critical_energy_kev: self.critical_energy_kev(),
        }
    }

    /// Absolute on-axis spectral photon flux density at `distance_mm`:
    /// `(E_keV, photons s⁻¹ mm⁻² keV⁻¹)` at the centers of `n_bins` equal
    /// bins over `[0.1, 8] E_c`, spreading each shot's photons uniformly over
    /// a flat-top cone of half-angle `K / gamma` (ASSUMPTION) and averaging
    /// over the repetition rate. Empty for a non-positive distance.
    pub fn spectral_flux_density(&self, distance_mm: f64, n_bins: usize) -> Vec<(f64, f64)> {
        if !(distance_mm.is_finite() && distance_mm > 0.0) {
            return Vec::new();
        }
        let n = n_bins.max(1);
        let e_c = self.critical_energy_kev();
        let spot_radius_mm = self.divergence_mrad() * 1e-3 * distance_mm;
        let spot_area_mm2 = std::f64::consts::PI * spot_radius_mm * spot_radius_mm;
        let rate = self.photons_per_shot() * self.rep_rate_hz;
        let norm = 5.0 * std::f64::consts::PI / 3.0;
        let (lo, hi) = (SPECTRUM_Y_MIN * e_c, SPECTRUM_Y_MAX * e_c);
        let de = (hi - lo) / n as f64;
        (0..n)
            .map(|j| {
                let e = lo + (j as f64 + 0.5) * de;
                let dn_de = rate * bm_universal_flux(e / e_c) / (e * norm);
                (e, dn_de / spot_area_mm2)
            })
            .collect()
    }

    /// Transverse coherent fraction: Kim's per-plane
    /// `eps_r / (eps + eps_r)` (`eps_r = lambda / 4 pi`,
    /// [`coherent_fraction_per_plane`]) at the mean photon wavelength, with the
    /// photon emittance `eps = r_beta theta` in both planes (randomly oriented
    /// orbit planes → round beam), multiplied over the two planes.
    pub fn coherent_fraction(&self) -> f64 {
        let lambda_nm = HC_KEV_NM / self.mean_photon_energy_kev();
        let emittance = self.betatron_amplitude_um * 1e-6 * self.divergence_mrad() * 1e-3;
        coherent_fraction_per_plane(emittance, lambda_nm).powi(2)
    }
}

impl Default for BetatronSource {
    fn default() -> Self {
        Self::lwfa_100tw().expect("100 TW betatron preset is valid")
    }
}

impl LithographySource for BetatronSource {
    /// `hc / <E>`: wavelength at the mean photon energy 0.3079 E_c.
    fn wavelength_nm(&self) -> f64 {
        HC_KEV_NM / self.mean_photon_energy_kev()
    }

    /// Nominal Δλ/λ = 1: the spectrum is synchrotron-like broadband.
    fn bandwidth_pm(&self) -> f64 {
        self.wavelength_nm() * 1e3
    }

    fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64 {
        evaluate_illumination(&self.illumination, fx_norm, fy_norm)
    }

    /// Photon-number weights of the synchrotron-like spectrum on
    /// `spectral_samples` log-spaced bins over [0.1, 8] E_c, ascending in
    /// wavelength — spectral bookkeeping, not an imaging sampling.
    fn spectral_weights(&self) -> Vec<(f64, f64)> {
        let bins = self.photon_spectrum(self.spectral_samples);
        let total: f64 = bins.iter().map(|(_, n)| n).sum();
        bins.into_iter()
            .rev()
            .map(|(e, n)| (HC_KEV_NM / e, if total > 0.0 { n / total } else { 0.0 }))
            .collect()
    }

    fn pulse_energy_j(&self) -> Option<f64> {
        Some(self.xray_energy_per_shot_j())
    }

    fn rep_rate_hz(&self) -> Option<f64> {
        Some(self.rep_rate_hz)
    }

    fn pulse_duration_s(&self) -> Option<f64> {
        Some(self.pulse_duration_fs * 1e-15)
    }

    fn shot_to_shot_rms(&self) -> f64 {
        self.shot_to_shot_rms
    }

    // transverse_coherence(): the coherent fraction is ~1e-6 (reported in
    // derived_quantities); keep the trait default 0 so no coherent pupil is
    // implied.

    fn derived_quantities(&self) -> Vec<DerivedQuantity> {
        let p_avg = self.xray_energy_per_shot_j() * self.rep_rate_hz;
        vec![
            DerivedQuantity::new(
                "plasma_frequency_rad_s",
                plasma_frequency_rad_s(self.plasma_density_cm3),
                "rad/s",
                "omega_p = sqrt(n_e e^2 / (eps0 m_e))",
            ),
            DerivedQuantity::new(
                "plasma_wavelength_um",
                self.plasma_wavelength_um(),
                "um",
                "lambda_p = 2 pi c / omega_p",
            ),
            DerivedQuantity::new(
                "betatron_wavelength_um",
                self.betatron_wavelength_um(),
                "um",
                "lambda_beta = lambda_p sqrt(2 gamma) (ion channel)",
            ),
            DerivedQuantity::new(
                "betatron_strength_k",
                self.strength_k(),
                "-",
                "K = gamma k_beta r_beta; K >> 1 is the wiggler regime",
            ),
            DerivedQuantity::new(
                "critical_energy_kev",
                self.critical_energy_kev(),
                "keV",
                "(3/2) gamma^3 hbar omega_beta^2 r_beta / c = (3/2) K gamma^2 hbar omega_beta",
            ),
            DerivedQuantity::new(
                "photons_per_electron_per_period",
                photons_per_period_per_electron(self.strength_k()),
                "photons",
                "(5 pi sqrt3 / 6) alpha K ~ 3.31e-2 K",
            ),
            DerivedQuantity::new(
                "betatron_periods",
                self.num_periods(),
                "-",
                "interaction length / lambda_beta (constant gamma assumed)",
            ),
            DerivedQuantity::new(
                "photons_per_shot",
                self.photons_per_shot(),
                "photons",
                "N_e x N_beta x N_gamma, all energies (single-energy, single-amplitude bunch)",
            ),
            DerivedQuantity::new(
                "mean_photon_energy_kev",
                self.mean_photon_energy_kev(),
                "keV",
                "8/(15 sqrt3) E_c = 0.3079 E_c",
            ),
            DerivedQuantity::new(
                "xray_energy_per_shot_uj",
                self.xray_energy_per_shot_j() * 1e6,
                "uJ",
                "photons per shot x mean photon energy",
            ),
            DerivedQuantity::new(
                "average_power_w",
                p_avg,
                "W",
                "X-ray energy per shot x repetition rate",
            ),
            DerivedQuantity::new(
                "divergence_half_angle_mrad",
                self.divergence_mrad(),
                "mrad",
                "theta ~ K / gamma",
            ),
            DerivedQuantity::new(
                "coherent_fraction",
                self.coherent_fraction(),
                "-",
                "Kim eps_r/(eps+eps_r) per plane, eps = r_beta*theta, both planes (random orbit planes)",
            ),
            DerivedQuantity::new(
                "hvm_power_gap",
                HVM_REFERENCE_POWER_W / p_avg,
                "x",
                "250 W HVM-class EUV power / this source's average X-ray power (context only)",
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // Fixture numbers computed independently with NumPy (SI constants,
    // K_nu from its integral representation).

    #[test]
    fn test_plasma_frequency_fixture() {
        // n_e = 1e19 cm^-3: omega_p = 1.78399e14 rad/s (~5.64e4 sqrt(n)),
        // lambda_p = 10.5587 um.
        assert_relative_eq!(
            plasma_frequency_rad_s(1e19),
            1.783_99e14,
            max_relative = 1e-5
        );
        assert_relative_eq!(
            plasma_frequency_rad_s(1e19),
            5.64e4 * 1e19_f64.sqrt(),
            max_relative = 1e-3
        );
        let src = BetatronSource::lwfa_100tw().unwrap();
        assert_relative_eq!(src.plasma_wavelength_um(), 10.5587, max_relative = 1e-5);
        // omega_p ∝ sqrt(n): 4x density doubles it.
        assert_relative_eq!(
            plasma_frequency_rad_s(4e19) / plasma_frequency_rad_s(1e19),
            2.0,
            max_relative = 1e-12
        );
    }

    #[test]
    fn test_betatron_frequency_and_strength_fixture() {
        let src = BetatronSource::lwfa_100tw().unwrap();
        assert_relative_eq!(src.gamma(), 392.390_236_711_836_7, max_relative = 1e-12);
        assert_relative_eq!(
            src.betatron_frequency_rad_s(),
            6.368_21e12,
            max_relative = 1e-5
        );
        assert_relative_eq!(src.betatron_wavelength_um(), 295.790, max_relative = 1e-5);
        assert_relative_eq!(src.strength_k(), 8.3352, max_relative = 1e-4);
        // Engineering form K = 1.33e-10 sqrt(gamma n) r_beta[um].
        assert_relative_eq!(
            src.strength_k(),
            1.33e-10 * (src.gamma() * 1e19).sqrt(),
            max_relative = 2e-3
        );
        assert_relative_eq!(src.num_periods(), 10.1423, max_relative = 1e-4);
    }

    #[test]
    fn test_critical_energy_fixture_and_forms() {
        let src = BetatronSource::lwfa_100tw().unwrap();
        assert_relative_eq!(src.critical_energy_kev(), 8.069_11, max_relative = 1e-5);
        // Same value from (3/2) K gamma^2 hbar omega_beta.
        let alt = 1.5
            * src.strength_k()
            * src.gamma().powi(2)
            * HBAR_J_S
            * src.betatron_frequency_rad_s()
            / ELECTRON_CHARGE_C
            * 1e-3;
        assert_relative_eq!(src.critical_energy_kev(), alt, max_relative = 1e-12);
        // Engineering form 5.24e-24 gamma^2 n r_beta keV.
        assert_relative_eq!(
            src.critical_energy_kev(),
            5.24e-24 * src.gamma().powi(2) * 1e19,
            max_relative = 2e-3
        );
    }

    #[test]
    fn test_critical_energy_scaling_laws() {
        let e_c =
            |mev: f64, n: f64, r: f64| betatron_critical_energy_kev(n, gamma_from_mev(mev), r);
        let base = e_c(200.0, 1e19, 1.0);
        // E_c ∝ gamma^3 omega_beta^2 ∝ gamma^2 at fixed n, r_beta.
        let g1 = gamma_from_mev(200.0);
        let g2 = 2.0 * g1;
        let doubled_gamma = betatron_critical_energy_kev(1e19, g2, 1.0);
        assert_relative_eq!(doubled_gamma / base, 4.0, max_relative = 1e-12);
        // Linear in density and in amplitude.
        assert_relative_eq!(e_c(200.0, 2e19, 1.0) / base, 2.0, max_relative = 1e-12);
        assert_relative_eq!(e_c(200.0, 1e19, 2.0) / base, 2.0, max_relative = 1e-12);
    }

    #[test]
    fn test_photon_yield_fixture() {
        let src = BetatronSource::lwfa_100tw().unwrap();
        // N_gamma = (5 pi sqrt3/6) alpha K = 0.27581 (≈ 3.31e-2 K)
        let ng = photons_per_period_per_electron(src.strength_k());
        assert_relative_eq!(ng, 0.275_81, max_relative = 1e-4);
        assert_relative_eq!(ng / src.strength_k(), 3.31e-2, max_relative = 1e-3);
        // photons/shot = 3.12075e8 e x 10.1423 periods x 0.27581
        assert_relative_eq!(src.electrons_per_bunch(), 3.120_75e8, max_relative = 1e-5);
        assert_relative_eq!(src.photons_per_shot(), 8.729_86e8, max_relative = 1e-5);
        assert_relative_eq!(src.mean_photon_energy_kev(), 2.484_64, max_relative = 1e-5);
        assert_relative_eq!(
            src.xray_energy_per_shot_j(),
            3.475_21e-7,
            max_relative = 1e-5
        );
        assert_relative_eq!(
            src.average_power_w().unwrap(),
            3.475_21e-6,
            max_relative = 1e-5
        );
        assert_relative_eq!(src.divergence_mrad(), 21.2421, max_relative = 1e-5);
    }

    #[test]
    fn test_trait_wavelength_is_mean_photon_energy() {
        let src = BetatronSource::lwfa_100tw().unwrap();
        // hc / (0.30792 x 8.06911 keV) = 0.499002 nm
        assert_relative_eq!(src.wavelength_nm(), 0.499_002, max_relative = 1e-5);
        assert_relative_eq!(src.bandwidth_pm(), src.wavelength_nm() * 1e3);
    }

    #[test]
    fn test_spectrum_normalization_and_shape() {
        let src = BetatronSource::lwfa_100tw().unwrap();
        // The [0.1, 8] E_c window holds the photon fraction
        // Int_0.1^8 S(y)/y dy / (5 pi/3) = 0.462807 (independent quadrature;
        // 53% of synchrotron-like photons lie below 0.1 E_c).
        let fine = src.photon_spectrum(2000);
        let in_window: f64 = fine.iter().map(|(_, n)| n).sum();
        let frac = in_window / src.photons_per_shot();
        assert_relative_eq!(frac, 0.462_807, max_relative = 1e-4);
        let w = src.spectral_weights();
        assert_eq!(w.len(), 16);
        let sum: f64 = w.iter().map(|(_, x)| x).sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
        assert!(w.windows(2).all(|p| p[1].0 > p[0].0));
        match src.xray_spectrum() {
            XraySpectrum::BendingMagnet {
                critical_energy_kev,
            } => {
                assert_relative_eq!(critical_energy_kev, src.critical_energy_kev())
            }
            other => panic!("expected BendingMagnet, got {other:?}"),
        }
    }

    #[test]
    fn test_spectral_flux_density_scaling() {
        let src = BetatronSource::lwfa_100tw().unwrap();
        let near = src.spectral_flux_density(500.0, 32);
        let far = src.spectral_flux_density(1000.0, 32);
        assert_eq!(near.len(), 32);
        for ((e1, p1), (e2, p2)) in near.iter().zip(far.iter()) {
            assert_relative_eq!(e1, e2);
            assert_relative_eq!(p1 / p2, 4.0, max_relative = 1e-12);
        }
        // Spot area pi (K/gamma d)^2 at 1 m: pi (21.2421 mm)^2.
        let e = near[5].0;
        let expected = src.photons_per_shot()
            * src.rep_rate_hz
            * bm_universal_flux(e / src.critical_energy_kev())
            / (e * 5.0 * std::f64::consts::PI / 3.0)
            / (std::f64::consts::PI * (src.divergence_mrad() * 1e-3 * 500.0).powi(2));
        assert_relative_eq!(near[5].1, expected, max_relative = 1e-12);
        assert!(src.spectral_flux_density(-1.0, 8).is_empty());
    }

    #[test]
    fn test_validation() {
        // K < 1 (undulator regime) rejected: r_beta = 0.1 um gives K ≈ 0.83.
        assert!(BetatronSource::new(200.0, 1e19, 0.1, 3.0, 50.0, 10.0).is_err());
        assert!(BetatronSource::new(5.0, 1e19, 1.0, 3.0, 50.0, 10.0).is_err());
        assert!(BetatronSource::new(200.0, 2e21, 1.0, 3.0, 50.0, 10.0).is_err());
        assert!(BetatronSource::new(200.0, 0.0, 1.0, 3.0, 50.0, 10.0).is_err());
        assert!(BetatronSource::new(200.0, 1e19, 1.0, 0.0, 50.0, 10.0).is_err());
        assert!(BetatronSource::new(200.0, 1e19, 1.0, 3.0, -5.0, 10.0).is_err());
        assert!(BetatronSource::new(200.0, 1e19, 1.0, 3.0, 50.0, f64::NAN).is_err());
        let mut s = BetatronSource::lwfa_100tw().unwrap();
        s.shot_to_shot_rms = 1.2;
        assert!(s.validate().is_err());
    }

    #[test]
    fn test_derived_quantities_and_metadata() {
        let src = BetatronSource::lwfa_100tw().unwrap();
        let dq = src.derived_quantities();
        let get = |name: &str| dq.iter().find(|q| q.name == name).unwrap().value;
        assert_relative_eq!(get("critical_energy_kev"), 8.069_11, max_relative = 1e-5);
        assert_relative_eq!(get("betatron_strength_k"), 8.3352, max_relative = 1e-4);
        // Kim's form, both planes: (3.970933e-11 / (2.124207e-8 + 3.970933e-11))^2
        assert_relative_eq!(get("coherent_fraction"), 3.481_525e-6, max_relative = 1e-5);
        assert!(get("hvm_power_gap") > 1e7);
        assert!(dq.iter().all(|q| q.value.is_finite()));
        assert_relative_eq!(src.pulse_duration_s().unwrap(), 1e-14, max_relative = 1e-12);
        assert_relative_eq!(src.shot_to_shot_rms(), 0.3);
        assert_eq!(src.transverse_coherence(), 0.0);
    }
}
