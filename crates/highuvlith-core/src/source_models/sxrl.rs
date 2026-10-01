//! Plasma-based soft-X-ray lasers (SXRL): capillary-discharge Ne-like Ar at
//! 46.9 nm and transient-collisional Ni-like Ag / Cd / Mo at 13.9 / 13.2 /
//! 18.9 nm.
//!
//! Collisional-excitation X-ray lasers amplify spontaneous emission on a
//! J = 0 → 1 transition of a closed-shell-like ion in a hot, dense plasma
//! column: Ne-like ions (3p ¹S₀ → 3s ¹P₁) and Ni-like ions (4d ¹S₀ → 4p ¹P₁).
//! Electron collisions pump the upper level directly from the ground state;
//! the lower level empties radiatively, giving gain along the column. Two
//! table-top pumping schemes matter here:
//!
//! - **Capillary discharge** (Ne-like Ar⁸⁺, 46.9 nm): a fast current pulse
//!   through an Ar-filled capillary compresses a long, uniform plasma column;
//!   ns pulses at few-Hz to ~10 Hz rates.
//! - **Transient collisional / grazing-incidence pumping** (Ni-like Ag¹⁹⁺ at
//!   13.9 nm, Cd²⁰⁺ at 13.2 nm, Mo¹⁴⁺ at 18.9 nm): a ps laser pulse heats a
//!   pre-formed plasma from a solid target; ps pulses at ~10 Hz (up to
//!   ~100 Hz with diode-pumped drivers).
//!
//! Because the lasing wavelength is a fixed atomic transition, it is set by
//! the choice of ion — [`SxrlScheme`] — not by a tunable machine parameter.
//! These lasers are narrow-line and partially spatially coherent, which is
//! why table-top 46.9 nm capillary lasers have been used for EUV
//! interferometric (Lloyd's-mirror) and Talbot-effect lithography
//! demonstrations on nanoscale periodic patterns. Their reported average
//! powers (≈1 µW to a few mW, see below) are roughly five to eight orders of
//! magnitude below the ~250 W high-volume-manufacturing class.
//!
//! # Key equations
//!
//! - Photon energy `E = hc / lambda` (`hc = 1239.84193 eV nm`).
//! - Photons per pulse `N = E_pulse / E`; average power `P = E_pulse f`;
//!   peak power `P_peak = E_pulse / tau`.
//! - Temporal coherence length `l_c = lambda^2 / Delta-lambda =
//!   lambda / (Delta-lambda / lambda)` (order-unity line-shape factor
//!   omitted).
//! - Gaussian-Schell mode count `M ≈ 1 / zeta` for coherent fraction `zeta`
//!   (pupil fill via `sigma_from_coherence`).
//! - Two-beam interference: period `p = lambda / (2 sin theta)`, so the
//!   minimum half-pitch is `lambda / 4` (theta → 90°).
//!
//! # Model status
//!
//! ✅ spectral physics: the lasing wavelengths are fixed atomic lines and the
//! derived photon energy, photon number, coherence length and interference
//! limit are exact arithmetic on them. 🔶 everything else: the presets'
//! pulse energy × repetition rate reproduce the reported average powers below
//! (the Ar and Mo splits are the published ones; the Ag and Cd splits are
//! assumptions matched to reported orders of magnitude), while the linewidth
//! (Δλ/λ ≈ 1e-4 class), pulse duration, coherent fraction and shot-to-shot
//! jitter are **order-of-magnitude assumptions**, not the specification of
//! any one machine. Not modelled: the gain and saturation physics of the
//! amplifier (the output energy is an input), the refraction-limited beam
//! profile, the line shape (sampled as a Gaussian), and polarization.
//!
//! # Real-machine anchors (reported average laser output)
//!
//! - Capillary-discharge Ne-like Ar, 46.9 nm: 3.5 mW (0.88 mJ × 4 Hz;
//!   Macchietto et al., Opt. Lett. 24, 1115 (1999)); desk-top version
//!   0.16 mW (13 µJ × 12 Hz; Heinbuch et al., Opt. Express 13, 4050 (2005)).
//! - Diode-pumped transient Ni-like Mo, 18.9 nm: ~0.1 mW at 100 Hz (~1 µJ;
//!   Reagan et al., Opt. Express 21, 28380 (2013)).
//! - Ni-like Ag 13.9 nm ~0.1 mW, Ni-like Cd 13.2 nm ~1 µW, Ni-like Sn 11.9 nm
//!   ~20 µW: reported orders of magnitude, not independently verified here.

use serde::{Deserialize, Serialize};

use super::dpp::HVM_REFERENCE_POWER_W;
use super::physics::ELECTRON_CHARGE_C;
use crate::source::{
    evaluate_illumination, evaluate_spectral_weights, sigma_from_coherence, DerivedQuantity,
    IlluminationShape, LithographySource, SpectralShape,
};

/// Core Gaussian pupil sigma for a fully coherent beam (shared convention
/// with the other coherent families).
const SIGMA_CORE: f64 = 0.05;

/// Lasing scheme: fixes the ion, the transition, and hence the wavelength.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SxrlScheme {
    /// Capillary-discharge Ne-like Ar (Ar⁸⁺ 3p ¹S₀ → 3s ¹P₁), 46.9 nm.
    CapillaryNeLikeAr,
    /// Transient-collisional Ni-like Ag (Ag¹⁹⁺ 4d ¹S₀ → 4p ¹P₁), 13.9 nm.
    NiLikeAg,
    /// Transient-collisional Ni-like Cd (Cd²⁰⁺ 4d ¹S₀ → 4p ¹P₁), 13.2 nm.
    NiLikeCd,
    /// Transient-collisional Ni-like Mo (Mo¹⁴⁺ 4d ¹S₀ → 4p ¹P₁), 18.9 nm.
    NiLikeMo,
}

impl SxrlScheme {
    /// Lasing wavelength in nm (fixed atomic transition).
    pub fn wavelength_nm(self) -> f64 {
        match self {
            Self::CapillaryNeLikeAr => 46.9,
            Self::NiLikeAg => 13.9,
            Self::NiLikeCd => 13.2,
            Self::NiLikeMo => 18.9,
        }
    }

    /// Lasing ion and transition, for display.
    pub fn transition(self) -> &'static str {
        match self {
            Self::CapillaryNeLikeAr => "Ne-like Ar8+ 3p 1S0 -> 3s 1P1",
            Self::NiLikeAg => "Ni-like Ag19+ 4d 1S0 -> 4p 1P1",
            Self::NiLikeCd => "Ni-like Cd20+ 4d 1S0 -> 4p 1P1",
            Self::NiLikeMo => "Ni-like Mo14+ 4d 1S0 -> 4p 1P1",
        }
    }

    /// Reported average output of real lasers of this scheme (hedged
    /// real-machine anchor for the derived-quantity notes).
    pub fn reported_average_power(self) -> &'static str {
        match self {
            Self::CapillaryNeLikeAr => {
                "reported: 0.16 mW desk-top (Heinbuch et al. 2005), 3.5 mW (Macchietto et al. 1999)"
            }
            Self::NiLikeAg => "reported: ~0.1 mW (order of magnitude)",
            Self::NiLikeCd => "reported: ~1 uW (order of magnitude)",
            Self::NiLikeMo => "reported: ~0.1 mW at 100 Hz, diode-pumped (Reagan et al. 2013)",
        }
    }

    /// Parse a scheme tag: the serde names (`"capillary_ne_like_ar"`,
    /// `"ni_like_ag"`, ...) or the short preset names (`"ar_46nm9"`,
    /// `"ag_13nm9"`, `"cd_13nm2"`, `"mo_18nm9"`).
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "ar_46nm9" | "capillary_ne_like_ar" | "ne_like_ar" | "ar" => {
                Some(Self::CapillaryNeLikeAr)
            }
            "ag_13nm9" | "ni_like_ag" | "ag" => Some(Self::NiLikeAg),
            "cd_13nm2" | "ni_like_cd" | "cd" => Some(Self::NiLikeCd),
            "mo_18nm9" | "ni_like_mo" | "mo" => Some(Self::NiLikeMo),
            _ => None,
        }
    }
}

/// Plasma-based soft-X-ray laser source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SxrlSource {
    /// Lasing scheme (fixes the wavelength).
    pub scheme: SxrlScheme,
    /// Relative linewidth Δλ/λ (FWHM). Assumed ~1e-4 class (Doppler/
    /// collisional broadening narrowed by gain).
    pub rel_linewidth: f64,
    /// Output energy per pulse in µJ (order of magnitude).
    pub pulse_energy_uj: f64,
    /// Repetition rate in Hz.
    pub rep_rate_hz: f64,
    /// Pulse duration in ps (capillary: ~1 ns; transient: a few ps).
    pub pulse_duration_ps: f64,
    /// Transverse coherent fraction in (0, 1] (partial spatial coherence;
    /// improves with plasma-column length).
    pub transverse_coherence_fraction: f64,
    /// Relative rms shot-to-shot pulse-energy fluctuation (assumed).
    pub shot_to_shot_rms: f64,
    /// Number of spectral samples.
    pub spectral_samples: usize,
    /// Illumination pupil shape (Gaussian from the coherent fraction).
    pub illumination: IlluminationShape,
}

impl SxrlSource {
    /// Construct a scheme with explicit (order-of-magnitude) output
    /// parameters; the pupil is derived from the coherent fraction.
    pub fn new(
        scheme: SxrlScheme,
        pulse_energy_uj: f64,
        rep_rate_hz: f64,
        pulse_duration_ps: f64,
        transverse_coherence_fraction: f64,
    ) -> crate::error::Result<Self> {
        let src = Self {
            scheme,
            rel_linewidth: 1e-4,
            pulse_energy_uj,
            rep_rate_hz,
            pulse_duration_ps,
            transverse_coherence_fraction,
            shot_to_shot_rms: 0.1,
            spectral_samples: 5,
            illumination: IlluminationShape::CoherentGaussian {
                sigma: sigma_from_coherence(transverse_coherence_fraction, SIGMA_CORE),
            },
        };
        src.validate()?;
        Ok(src)
    }

    /// Desk-top capillary-discharge Ne-like Ar laser at 46.9 nm: 13 µJ ×
    /// 12 Hz = 0.16 mW, the reported desk-top output (Heinbuch et al., 2005).
    /// ASSUMED: ~1.2 ns pulses (order of magnitude), coherent fraction 0.3.
    pub fn ar_46nm9() -> crate::error::Result<Self> {
        Self::new(SxrlScheme::CapillaryNeLikeAr, 13.0, 12.0, 1200.0, 0.3)
    }

    /// Transient-collisional Ni-like Ag laser at 13.9 nm: 10 µJ × 10 Hz =
    /// 0.1 mW, matching the reported ~0.1 mW scale (the energy/rate split is
    /// an assumption). ASSUMED: ~5 ps, coherent fraction 0.2.
    pub fn ag_13nm9() -> crate::error::Result<Self> {
        Self::new(SxrlScheme::NiLikeAg, 10.0, 10.0, 5.0, 0.2)
    }

    /// Transient-collisional Ni-like Cd laser at 13.2 nm: 0.1 µJ × 10 Hz =
    /// 1 µW, matching the reported ~1 µW scale (the energy/rate split is an
    /// assumption). ASSUMED: ~5 ps, coherent fraction 0.2.
    pub fn cd_13nm2() -> crate::error::Result<Self> {
        Self::new(SxrlScheme::NiLikeCd, 0.1, 10.0, 5.0, 0.2)
    }

    /// Diode-pumped transient-collisional Ni-like Mo laser at 18.9 nm:
    /// 1 µJ × 100 Hz = 0.1 mW (Reagan et al., 2013). ASSUMED: ~5 ps,
    /// coherent fraction 0.2.
    pub fn mo_18nm9() -> crate::error::Result<Self> {
        Self::new(SxrlScheme::NiLikeMo, 1.0, 100.0, 5.0, 0.2)
    }

    /// The preset for a scheme.
    pub fn preset(scheme: SxrlScheme) -> crate::error::Result<Self> {
        match scheme {
            SxrlScheme::CapillaryNeLikeAr => Self::ar_46nm9(),
            SxrlScheme::NiLikeAg => Self::ag_13nm9(),
            SxrlScheme::NiLikeCd => Self::cd_13nm2(),
            SxrlScheme::NiLikeMo => Self::mo_18nm9(),
        }
    }

    /// Check every field; called by the constructors and available to
    /// frontends that override fields after construction.
    pub fn validate(&self) -> crate::error::Result<()> {
        use crate::error::LithographyError::InvalidParameter;
        if !(self.rel_linewidth > 0.0 && self.rel_linewidth <= 1e-2) {
            return Err(InvalidParameter {
                name: "rel_linewidth",
                value: self.rel_linewidth,
                reason: "must be in (0, 1e-2]: plasma X-ray lasers are narrow-line (~1e-4)",
            });
        }
        for (name, value) in [
            ("pulse_energy_uj", self.pulse_energy_uj),
            ("rep_rate_hz", self.rep_rate_hz),
            ("pulse_duration_ps", self.pulse_duration_ps),
        ] {
            if !(value.is_finite() && value > 0.0) {
                return Err(InvalidParameter {
                    name,
                    value,
                    reason: "must be positive",
                });
            }
        }
        if !(self.transverse_coherence_fraction > 0.0 && self.transverse_coherence_fraction <= 1.0)
        {
            return Err(InvalidParameter {
                name: "transverse_coherence_fraction",
                value: self.transverse_coherence_fraction,
                reason: "must be in (0, 1]",
            });
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
        Ok(())
    }

    /// Photons per pulse, `E_pulse / (hc / lambda)`.
    pub fn photons_per_pulse(&self) -> f64 {
        self.pulse_energy_uj * 1e-6 / (self.photon_energy_ev() * ELECTRON_CHARGE_C)
    }

    /// Peak power `E_pulse / tau` in W.
    pub fn peak_power_w(&self) -> f64 {
        self.pulse_energy_uj * 1e-6 / (self.pulse_duration_ps * 1e-12)
    }

    /// Temporal coherence length `lambda^2 / Delta-lambda` in µm.
    pub fn coherence_length_um(&self) -> f64 {
        self.scheme.wavelength_nm() / self.rel_linewidth * 1e-3
    }

    /// Minimum two-beam-interference half-pitch `lambda / 4` in nm.
    pub fn interference_min_half_pitch_nm(&self) -> f64 {
        self.scheme.wavelength_nm() / 4.0
    }
}

impl Default for SxrlSource {
    fn default() -> Self {
        Self::ar_46nm9().expect("Ar 46.9 nm preset is valid")
    }
}

impl LithographySource for SxrlSource {
    /// Fixed by the lasing transition of the chosen ion.
    fn wavelength_nm(&self) -> f64 {
        self.scheme.wavelength_nm()
    }

    fn bandwidth_pm(&self) -> f64 {
        self.rel_linewidth * self.scheme.wavelength_nm() * 1e3
    }

    fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64 {
        evaluate_illumination(&self.illumination, fx_norm, fy_norm)
    }

    fn spectral_weights(&self) -> Vec<(f64, f64)> {
        evaluate_spectral_weights(
            self.scheme.wavelength_nm(),
            self.bandwidth_pm(),
            self.spectral_samples,
            &SpectralShape::Gaussian,
        )
    }

    fn pulse_energy_j(&self) -> Option<f64> {
        Some(self.pulse_energy_uj * 1e-6)
    }

    fn rep_rate_hz(&self) -> Option<f64> {
        Some(self.rep_rate_hz)
    }

    fn pulse_duration_s(&self) -> Option<f64> {
        Some(self.pulse_duration_ps * 1e-12)
    }

    fn transverse_coherence(&self) -> f64 {
        self.transverse_coherence_fraction
    }

    fn shot_to_shot_rms(&self) -> f64 {
        self.shot_to_shot_rms
    }

    fn derived_quantities(&self) -> Vec<DerivedQuantity> {
        let p_avg = self.pulse_energy_uj * 1e-6 * self.rep_rate_hz;
        vec![
            DerivedQuantity::new(
                "photon_energy_ev",
                self.photon_energy_ev(),
                "eV",
                self.scheme.transition(),
            ),
            DerivedQuantity::new(
                "photons_per_pulse",
                self.photons_per_pulse(),
                "photons",
                "pulse energy / photon energy (pulse energy: order-of-magnitude input)",
            ),
            DerivedQuantity::new(
                "average_power_w",
                p_avg,
                "W",
                &format!(
                    "pulse energy x repetition rate; {}",
                    self.scheme.reported_average_power()
                ),
            ),
            DerivedQuantity::new(
                "peak_power_w",
                self.peak_power_w(),
                "W",
                "pulse energy / pulse duration",
            ),
            DerivedQuantity::new(
                "coherence_length_um",
                self.coherence_length_um(),
                "um",
                "lambda^2 / d-lambda at the assumed relative linewidth",
            ),
            DerivedQuantity::new(
                "interference_min_half_pitch_nm",
                self.interference_min_half_pitch_nm(),
                "nm",
                "two-beam interference limit lambda/4 (Lloyd's mirror / IL)",
            ),
            DerivedQuantity::new(
                "coherent_mode_count",
                1.0 / self.transverse_coherence_fraction,
                "-",
                "Gaussian-Schell estimate M ~ 1/zeta (zeta assumed)",
            ),
            DerivedQuantity::new(
                "hvm_power_gap",
                HVM_REFERENCE_POWER_W / p_avg,
                "x",
                "250 W HVM-class EUV power / this laser's average power",
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_fixed_atomic_lines() {
        let cases = [
            (SxrlSource::ar_46nm9().unwrap(), 46.9, 26.435_862),
            (SxrlSource::ag_13nm9().unwrap(), 13.9, 89.197_261),
            (SxrlSource::cd_13nm2().unwrap(), 13.2, 93.927_419),
            (SxrlSource::mo_18nm9().unwrap(), 18.9, 65.600_102),
        ];
        for (src, lambda, ev) in cases {
            assert_relative_eq!(src.wavelength_nm(), lambda);
            assert_relative_eq!(src.photon_energy_ev(), ev, max_relative = 1e-7);
        }
    }

    #[test]
    fn test_photon_number_and_power_fixture() {
        let ar = SxrlSource::ar_46nm9().unwrap();
        // 13 uJ / (26.435862 eV * 1.602176634e-19 J/eV) = 3.0693010e12
        assert_relative_eq!(
            ar.photons_per_pulse(),
            3.069_301_005_7e12,
            max_relative = 1e-9
        );
        // 13 uJ x 12 Hz = 156 uW (desk-top anchor 0.16 mW); 13 uJ / 1.2 ns
        assert_relative_eq!(ar.average_power_w().unwrap(), 1.56e-4, max_relative = 1e-12);
        assert_relative_eq!(
            ar.peak_power_w(),
            10_833.333_333_333_334,
            max_relative = 1e-12
        );
        assert_relative_eq!(ar.pulse_duration_s().unwrap(), 1.2e-9, max_relative = 1e-12);
        let gap = ar
            .derived_quantities()
            .into_iter()
            .find(|q| q.name == "hvm_power_gap")
            .unwrap()
            .value;
        assert_relative_eq!(gap, 1.602_564_1e6, max_relative = 1e-7);
        // Reported-power anchors are carried in the note.
        let note = ar
            .derived_quantities()
            .into_iter()
            .find(|q| q.name == "average_power_w")
            .unwrap()
            .note;
        assert!(note.contains("Heinbuch"), "{note}");
        // Mo preset: 1 uJ x 100 Hz = 0.1 mW (Reagan et al. 2013 anchor).
        let mo = SxrlSource::mo_18nm9().unwrap();
        assert_relative_eq!(mo.average_power_w().unwrap(), 1e-4, max_relative = 1e-12);
        assert_relative_eq!(
            mo.photons_per_pulse(),
            9.514_480_729_6e10,
            max_relative = 1e-9
        );
        // Cd preset: ~1 uW scale.
        let cd = SxrlSource::cd_13nm2().unwrap();
        assert_relative_eq!(cd.average_power_w().unwrap(), 1e-6, max_relative = 1e-12);
    }

    #[test]
    fn test_narrow_line_and_coherence_length() {
        let ar = SxrlSource::ar_46nm9().unwrap();
        // d-lambda = 1e-4 x 46.9 nm = 4.69 pm
        assert_relative_eq!(ar.bandwidth_pm(), 4.69, max_relative = 1e-12);
        // l_c = lambda / (d-lambda/lambda) = 46.9 nm / 1e-4 = 469 um
        assert_relative_eq!(ar.coherence_length_um(), 469.0, max_relative = 1e-12);
        // Halving the linewidth doubles the coherence length.
        let mut narrow = ar.clone();
        narrow.rel_linewidth = 5e-5;
        assert_relative_eq!(narrow.coherence_length_um(), 938.0, max_relative = 1e-12);
        // lambda/4 interference half-pitch
        assert_relative_eq!(
            ar.interference_min_half_pitch_nm(),
            11.725,
            max_relative = 1e-12
        );
        let ag = SxrlSource::ag_13nm9().unwrap();
        assert_relative_eq!(
            ag.interference_min_half_pitch_nm(),
            3.475,
            max_relative = 1e-12
        );
    }

    #[test]
    fn test_partial_coherence_sets_gaussian_pupil() {
        let ar = SxrlSource::ar_46nm9().unwrap();
        assert_relative_eq!(ar.transverse_coherence(), 0.3);
        match ar.illumination {
            IlluminationShape::CoherentGaussian { sigma } => {
                assert_relative_eq!(sigma, 0.05 / 0.3_f64.sqrt(), max_relative = 1e-12)
            }
            ref other => panic!("expected CoherentGaussian, got {other:?}"),
        }
        assert!(ar.intensity_at(0.0, 0.0) > ar.intensity_at(0.1, 0.0));
    }

    #[test]
    fn test_spectral_weights_sum_to_one() {
        for scheme in [
            SxrlScheme::CapillaryNeLikeAr,
            SxrlScheme::NiLikeAg,
            SxrlScheme::NiLikeCd,
            SxrlScheme::NiLikeMo,
        ] {
            let src = SxrlSource::preset(scheme).unwrap();
            let w = src.spectral_weights();
            let sum: f64 = w.iter().map(|(_, x)| x).sum();
            assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
            // All samples within +-2.5 FWHM of the line.
            let span = 2.5 * src.bandwidth_pm() * 1e-3 + 1e-12;
            assert!(w
                .iter()
                .all(|(l, _)| (l - src.wavelength_nm()).abs() <= span));
        }
    }

    #[test]
    fn test_validation_and_names() {
        assert!(SxrlSource::new(SxrlScheme::NiLikeAg, 0.0, 10.0, 5.0, 0.2).is_err());
        assert!(SxrlSource::new(SxrlScheme::NiLikeAg, 1.0, -1.0, 5.0, 0.2).is_err());
        assert!(SxrlSource::new(SxrlScheme::NiLikeAg, 1.0, 10.0, 0.0, 0.2).is_err());
        assert!(SxrlSource::new(SxrlScheme::NiLikeAg, 1.0, 10.0, 5.0, 0.0).is_err());
        assert!(SxrlSource::new(SxrlScheme::NiLikeAg, 1.0, 10.0, 5.0, 1.5).is_err());
        let mut s = SxrlSource::ag_13nm9().unwrap();
        s.rel_linewidth = 0.05;
        assert!(s.validate().is_err());
        s.rel_linewidth = 1e-4;
        s.shot_to_shot_rms = -0.1;
        assert!(s.validate().is_err());
        assert_eq!(
            SxrlScheme::from_name("ag_13nm9"),
            Some(SxrlScheme::NiLikeAg)
        );
        assert_eq!(
            SxrlScheme::from_name("ni_like_mo"),
            Some(SxrlScheme::NiLikeMo)
        );
        assert_eq!(
            SxrlScheme::from_name("AR_46NM9"),
            Some(SxrlScheme::CapillaryNeLikeAr)
        );
        assert_eq!(SxrlScheme::from_name("krypton"), None);
    }
}
