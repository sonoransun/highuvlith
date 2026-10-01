//! Discharge-produced plasma (DPP) and laser-assisted discharge plasma (LDP)
//! EUV sources at 13.5 nm, Xe and Sn fuels.
//!
//! A pulsed high-current discharge (a pinch, or a laser-triggered discharge
//! across a Sn-coated electrode gap) heats a small plasma column to tens of
//! eV; Xe¹⁰⁺ and Sn⁸⁺–Sn¹⁴⁺ ions radiate into the 2% band that Mo/Si
//! mirrors reflect. Electrical energy is converted *directly* into plasma
//! energy — no drive laser — which made DPP the leading low-cost alternative
//! to laser-produced plasma (LPP) during EUV development. It lost the
//! high-volume-manufacturing race to Sn LPP on power scaling (electrode
//! erosion and heat removal cap the input power at tens of kW) and on
//! étendue (a mm-scale pinch versus a ~0.1 mm LPP plasma). Discharge sources
//! live on as compact, lower-power sources for actinic (at-wavelength)
//! metrology, inspection, and resist/optics exposure studies.
//!
//! # Key equations
//!
//! - In-band power into 2π sr: `P_2pi = CE * P_elec`, with `CE` the
//!   conversion efficiency from electrical input to in-band (2% BW at
//!   13.5 nm) radiation into the 2π half-space.
//! - Collected power: `P_coll = P_2pi (Omega_c / 2 pi) eta_c` for a collector
//!   subtending `Omega_c` with throughput `eta_c` (mirror reflectivity ×
//!   debris-mitigation transmission).
//! - Source étendue: `G_src = A_proj Omega_c` with the side-on projected
//!   area of a cylindrical pinch `A_proj = d l` — the small-angle form
//!   `G = A Omega`, used here as a conservative upper estimate for a large
//!   collection solid angle.
//! - Étendue-limited usable fraction (uniform phase-space density):
//!   `f_G = min(1, G_ill / G_src)`.
//! - Power at intermediate focus: `P_IF = P_coll f_G`.
//! - In-band radiance: `L = P_2pi / (A_proj 2 pi)` [W mm⁻² sr⁻¹].
//!
//! # Model status
//!
//! Simplified (🔶). Live: the full power chain `P_elec → P_2π → P_coll →
//! P_IF` (including the étendue cut), the in-band Gaussian spectrum within
//! the mirror-selected 2% band, pulse energy from the repetition rate, and a
//! shot-to-shot energy jitter that feeds the stochastic module. Two power
//! conventions are kept apart: DPP outputs are customarily quoted **into
//! 2π sr at the source** (`in_band_power_2pi_w`), whereas scanner
//! requirements are **at intermediate focus** (`power_at_if_w`, several
//! times lower after collection and the étendue cut). The conversion
//! efficiencies are **reported order-of-magnitude values** (Xe ≈0.5%,
//! Sn ≈2%, both in-band into 2π sr) and the presets' 2π powers sit on the
//! real-machine anchors below; the electrical input split, pinch dimensions,
//! collector solid angle/throughput, jitter and the default illuminator
//! étendue are **assumptions** (labelled in each preset). Not modelled: discharge
//! circuit and pinch MHD dynamics, the Xe/Sn emission spectra outside the
//! mirror band (out-of-band power, Xe's stronger ~11 nm emission), debris
//! generation and electrode erosion lifetime, and the exact étendue integral
//! of an elongated emitter viewed over a large solid angle. Plasma emission
//! is spatially incoherent: `transverse_coherence()` reports 0.
//!
//! The `laser_assisted` flag (LDP: a laser pulse vaporizes Sn from a
//! rotating electrode to trigger and localize the discharge) is stored for
//! provenance only; its effect is represented through the preset parameters.
//!
//! # Real-machine anchors (reported, in-band at 13.5 nm)
//!
//! Into 2π sr AT THE SOURCE:
//! - Xe metrology DPP: Energetiq EQ-10 / EQ-10HP 10 → 20 W (product
//!   specification); an EQ-10HP delivered 27 W at the LBNL MET5 (Miyakawa,
//!   2024); Fraunhofer ILT FS5440 20–40 W in 2% bandwidth (Vieker, 2020).
//! - Sn DPP: Philips Extreme UV lamp "200 W/2π continuous" (2005, as
//!   reported by Banine & Moors, 2011); TRINITI rotating-disk-electrode Sn
//!   DPP 360 W continuous for ≥44 min (180 mJ × 2 kHz) and 1 kW for 10 s at
//!   63 kW electrical (Borisov et al., 2010) — an implied CE ≈ 1.6%.
//!
//! At intermediate focus: the XTREME/Ushio laser-assisted Sn DPP on the ASML
//! NXE:3100 delivered "20 W at 90% duty" (≈18 W time-averaged exposure
//! power; Banine & Moors, 2011). For scale, a 2013 LPP example quoted
//! "720 W in 2π → 176 W raw at IF" (Cymer).

use serde::{Deserialize, Serialize};

use super::physics::photon_energy_j;
use crate::source::{
    evaluate_illumination, evaluate_spectral_weights, DerivedQuantity, IlluminationShape,
    LithographySource, SpectralShape,
};

/// Reference in-band EUV power at intermediate focus for a
/// high-volume-manufacturing scanner (the ~250 W class of NXE-type Sn LPP
/// sources), used by the WP-D families' "power gap" derived quantities.
pub const HVM_REFERENCE_POWER_W: f64 = 250.0;

/// Discharge-plasma fuel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DppFuel {
    /// Xenon gas: clean (no condensable debris) but low in-band CE
    /// (~0.5%): xenon's strongest EUV emission lies near 11 nm, and the
    /// 13.5 nm band comes mainly from Xe¹⁰⁺ lines.
    Xe,
    /// Tin (vapour from a coated/rotating electrode): ~4x the in-band CE of
    /// Xe (~2%) at the price of condensable Sn debris.
    Sn,
}

/// Discharge-produced (or laser-assisted discharge) plasma EUV source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DppSource {
    /// Plasma fuel.
    pub fuel: DppFuel,
    /// Laser-assisted discharge (LDP) rather than a pure gas/vapour
    /// discharge. Stored (inert): provenance for the preset parameters.
    pub laser_assisted: bool,
    /// In-band center wavelength in nm (13.5: the Mo/Si mirror band).
    pub wavelength_nm: f64,
    /// Mirror-selected in-band FWHM in pm (2% of 13.5 nm = 270 pm).
    pub bandwidth_pm: f64,
    /// Electrical input power to the discharge in W (tens of kW at most:
    /// electrode heat removal is the scaling limit).
    pub electrical_power_w: f64,
    /// Conversion efficiency electrical → in-band (2% BW) radiation into
    /// 2π sr. Reported order of magnitude: Xe ≈ 0.005, Sn ≈ 0.02.
    pub conversion_efficiency: f64,
    /// Emitting-column (pinch) diameter in mm (FWHM-equivalent).
    pub source_diameter_mm: f64,
    /// Emitting-column length in mm along the discharge axis.
    pub source_length_mm: f64,
    /// Solid angle subtended by the collector in sr (≤ 2π).
    pub collector_solid_angle_sr: f64,
    /// Collector throughput in (0, 1]: mirror reflectivity × debris-
    /// mitigation (foil-trap) transmission.
    pub collector_efficiency: f64,
    /// Étendue the downstream illuminator accepts, in mm²·sr (depends on
    /// the scanner NA, field, and pupil fill — an input, not a source
    /// property).
    pub illuminator_etendue_mm2_sr: f64,
    /// Discharge repetition rate in Hz.
    pub rep_rate_hz: f64,
    /// Relative rms pulse-to-pulse in-band energy fluctuation (assumed).
    pub shot_to_shot_rms: f64,
    /// Number of spectral samples for polychromatic simulation.
    pub spectral_samples: usize,
    /// In-band line shape.
    pub spectral_shape: SpectralShape,
    /// Illumination pupil shape (plasma is incoherent: large sigma disk).
    pub illumination: IlluminationShape,
}

impl DppSource {
    /// Xe discharge (Z-pinch class) at 13.5 nm for actinic metrology /
    /// inspection: 10 W in-band into 2π sr at the source — the reported
    /// output class of commercial Xe metrology DPP sources (Energetiq EQ-10).
    /// Reported CE ≈ 0.5% (in-band, 2π sr); the 2 kW electrical input is
    /// then implied. ASSUMED: 1 mm × 3 mm pinch, 1.5 sr collector at 40%
    /// throughput, 2 kHz, 5% rms pulse-to-pulse jitter, 3.3 mm²·sr illuminator
    /// étendue.
    pub fn xe_13nm5(sigma: f64) -> crate::error::Result<Self> {
        crate::source::validate_sigma(sigma)?;
        let src = Self {
            fuel: DppFuel::Xe,
            laser_assisted: false,
            wavelength_nm: 13.5,
            bandwidth_pm: 270.0,
            electrical_power_w: 2_000.0,
            conversion_efficiency: 0.005,
            source_diameter_mm: 1.0,
            source_length_mm: 3.0,
            collector_solid_angle_sr: 1.5,
            collector_efficiency: 0.4,
            illuminator_etendue_mm2_sr: 3.3,
            rep_rate_hz: 2_000.0,
            shot_to_shot_rms: 0.05,
            spectral_samples: 5,
            spectral_shape: SpectralShape::Gaussian,
            illumination: IlluminationShape::Conventional { sigma },
        };
        src.validate()?;
        Ok(src)
    }

    /// Sn discharge (DPP / laser-assisted LDP class) at 13.5 nm — the
    /// historical HVM-candidate architecture — at the demonstrated
    /// *continuous* in-band level: 360 W into 2π sr at the source as 180 mJ
    /// × 2 kHz (TRINITI rotating-disk Sn DPP, Borisov et al. 2010), from
    /// 18 kW electrical at the reported CE ≈ 2%. The model then gives ≈34 W
    /// at intermediate focus, the same order as the ≈18–20 W the XTREME/Ushio
    /// LDP delivered on the NXE:3100 (2011). ASSUMED: 0.4 mm × 1 mm plasma,
    /// 1.5 sr collector at 40% throughput, 5% rms jitter, 3.3 mm²·sr
    /// illuminator étendue.
    pub fn sn_13nm5(sigma: f64) -> crate::error::Result<Self> {
        crate::source::validate_sigma(sigma)?;
        let src = Self {
            fuel: DppFuel::Sn,
            laser_assisted: true,
            wavelength_nm: 13.5,
            bandwidth_pm: 270.0,
            electrical_power_w: 18_000.0,
            conversion_efficiency: 0.02,
            source_diameter_mm: 0.4,
            source_length_mm: 1.0,
            collector_solid_angle_sr: 1.5,
            collector_efficiency: 0.4,
            illuminator_etendue_mm2_sr: 3.3,
            rep_rate_hz: 2_000.0,
            shot_to_shot_rms: 0.05,
            spectral_samples: 5,
            spectral_shape: SpectralShape::Gaussian,
            illumination: IlluminationShape::Conventional { sigma },
        };
        src.validate()?;
        Ok(src)
    }

    /// Check every field; called by the presets and available to frontends
    /// that override fields after construction.
    pub fn validate(&self) -> crate::error::Result<()> {
        use crate::error::LithographyError::InvalidParameter;
        let positive = [
            ("wavelength_nm", self.wavelength_nm),
            ("electrical_power_w", self.electrical_power_w),
            ("source_diameter_mm", self.source_diameter_mm),
            ("source_length_mm", self.source_length_mm),
            (
                "illuminator_etendue_mm2_sr",
                self.illuminator_etendue_mm2_sr,
            ),
            ("rep_rate_hz", self.rep_rate_hz),
        ];
        for (name, value) in positive {
            if !(value.is_finite() && value > 0.0) {
                return Err(InvalidParameter {
                    name,
                    value,
                    reason: "must be positive",
                });
            }
        }
        if !(self.bandwidth_pm.is_finite() && self.bandwidth_pm >= 0.0) {
            return Err(InvalidParameter {
                name: "bandwidth_pm",
                value: self.bandwidth_pm,
                reason: "must be >= 0",
            });
        }
        if !(self.conversion_efficiency > 0.0 && self.conversion_efficiency < 1.0) {
            return Err(InvalidParameter {
                name: "conversion_efficiency",
                value: self.conversion_efficiency,
                reason: "must be in (0, 1)",
            });
        }
        let two_pi = 2.0 * std::f64::consts::PI;
        if !(self.collector_solid_angle_sr > 0.0 && self.collector_solid_angle_sr <= two_pi) {
            return Err(InvalidParameter {
                name: "collector_solid_angle_sr",
                value: self.collector_solid_angle_sr,
                reason: "must be in (0, 2 pi] sr (the conversion efficiency is defined into 2 pi)",
            });
        }
        if !(self.collector_efficiency > 0.0 && self.collector_efficiency <= 1.0) {
            return Err(InvalidParameter {
                name: "collector_efficiency",
                value: self.collector_efficiency,
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

    /// In-band power into 2π sr at the plasma, `CE * P_elec`, in W.
    pub fn in_band_power_2pi_w(&self) -> f64 {
        self.conversion_efficiency * self.electrical_power_w
    }

    /// In-band power collected and relayed by the collector, in W:
    /// `P_2pi (Omega_c / 2 pi) eta_c`.
    pub fn collected_power_w(&self) -> f64 {
        self.in_band_power_2pi_w() * self.collector_solid_angle_sr / (2.0 * std::f64::consts::PI)
            * self.collector_efficiency
    }

    /// Side-on projected area of the emitting column, `d l`, in mm².
    pub fn source_area_mm2(&self) -> f64 {
        self.source_diameter_mm * self.source_length_mm
    }

    /// Source étendue into the collector, `A_proj Omega_c`, in mm²·sr.
    pub fn source_etendue_mm2_sr(&self) -> f64 {
        self.source_area_mm2() * self.collector_solid_angle_sr
    }

    /// Fraction of the collected light that fits the illuminator étendue,
    /// `min(1, G_ill / G_src)` (uniform phase-space density).
    pub fn etendue_usable_fraction(&self) -> f64 {
        (self.illuminator_etendue_mm2_sr / self.source_etendue_mm2_sr()).min(1.0)
    }

    /// Usable in-band power at intermediate focus, `P_coll f_G`, in W.
    pub fn power_at_if_w(&self) -> f64 {
        self.collected_power_w() * self.etendue_usable_fraction()
    }

    /// In-band radiance `P_2pi / (A_proj 2 pi)` in W mm⁻² sr⁻¹ — the
    /// figure of merit for metrology/inspection (brightness, not power).
    pub fn in_band_radiance_w_mm2_sr(&self) -> f64 {
        self.in_band_power_2pi_w() / (self.source_area_mm2() * 2.0 * std::f64::consts::PI)
    }
}

impl Default for DppSource {
    fn default() -> Self {
        Self::sn_13nm5(0.9).expect("Sn LDP preset is valid")
    }
}

impl LithographySource for DppSource {
    fn wavelength_nm(&self) -> f64 {
        self.wavelength_nm
    }

    fn bandwidth_pm(&self) -> f64 {
        self.bandwidth_pm
    }

    fn intensity_at(&self, fx_norm: f64, fy_norm: f64) -> f64 {
        evaluate_illumination(&self.illumination, fx_norm, fy_norm)
    }

    fn spectral_weights(&self) -> Vec<(f64, f64)> {
        evaluate_spectral_weights(
            self.wavelength_nm,
            self.bandwidth_pm,
            self.spectral_samples,
            &self.spectral_shape,
        )
    }

    /// Usable in-band energy per discharge at intermediate focus.
    fn pulse_energy_j(&self) -> Option<f64> {
        Some(self.power_at_if_w() / self.rep_rate_hz)
    }

    fn rep_rate_hz(&self) -> Option<f64> {
        Some(self.rep_rate_hz)
    }

    /// Usable in-band power at intermediate focus (after collection and the
    /// étendue cut) — comparable to the LPP family's power chain.
    fn average_power_w(&self) -> Option<f64> {
        Some(self.power_at_if_w())
    }

    fn shot_to_shot_rms(&self) -> f64 {
        self.shot_to_shot_rms
    }

    // Plasma emission is spatially incoherent: keep the trait default
    // transverse_coherence() = 0.

    fn derived_quantities(&self) -> Vec<DerivedQuantity> {
        let p_if = self.power_at_if_w();
        let photon_j = photon_energy_j(self.wavelength_nm);
        vec![
            DerivedQuantity::new(
                "in_band_power_2pi_w",
                self.in_band_power_2pi_w(),
                "W",
                "AT THE SOURCE into 2pi sr (the DPP convention): CE x electrical power, CE reported order of magnitude",
            ),
            DerivedQuantity::new(
                "collected_power_w",
                self.collected_power_w(),
                "W",
                "P_2pi x (Omega_c / 2 pi) x collector throughput (assumed)",
            ),
            DerivedQuantity::new(
                "source_etendue_mm2_sr",
                self.source_etendue_mm2_sr(),
                "mm^2 sr",
                "side-on area d*l x collector solid angle (conservative upper estimate)",
            ),
            DerivedQuantity::new(
                "etendue_usable_fraction",
                self.etendue_usable_fraction(),
                "-",
                "min(1, G_ill / G_src), uniform phase-space density assumed",
            ),
            DerivedQuantity::new(
                "power_at_if_w",
                p_if,
                "W",
                "AT INTERMEDIATE FOCUS (the scanner convention): after collector and etendue cut",
            ),
            DerivedQuantity::new(
                "photon_rate_at_if",
                p_if / photon_j,
                "photons/s",
                "in-band photons at intermediate focus",
            ),
            DerivedQuantity::new(
                "in_band_radiance_w_mm2_sr",
                self.in_band_radiance_w_mm2_sr(),
                "W/mm^2/sr",
                "brightness figure of merit for metrology / inspection",
            ),
            DerivedQuantity::new(
                "in_band_pulse_energy_2pi_mj",
                self.in_band_power_2pi_w() / self.rep_rate_hz * 1e3,
                "mJ",
                "in-band energy per discharge into 2pi sr",
            ),
            DerivedQuantity::new(
                "electrode_heat_load_w",
                self.electrical_power_w * (1.0 - self.conversion_efficiency),
                "W",
                "electrical input not radiated in band: the power-scaling limit",
            ),
            DerivedQuantity::new(
                "hvm_power_gap",
                HVM_REFERENCE_POWER_W / p_if,
                "x",
                "250 W HVM-class IF power / this source's IF power (not its 2pi power)",
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use std::f64::consts::PI;

    #[test]
    fn test_xe_power_chain_fixture() {
        let src = DppSource::xe_13nm5(0.9).unwrap();
        // 2 kW x 0.5% = 10 W into 2 pi sr
        assert_relative_eq!(src.in_band_power_2pi_w(), 10.0, max_relative = 1e-12);
        // 10 W x 1.5/(2 pi) x 0.4 = 0.954930 W collected
        assert_relative_eq!(
            src.collected_power_w(),
            6.0 / (2.0 * PI),
            max_relative = 1e-12
        );
        assert_relative_eq!(
            src.collected_power_w(),
            0.954_929_658_551_372,
            max_relative = 1e-12
        );
        // G_src = 1 mm x 3 mm x 1.5 sr = 4.5 mm^2 sr > 3.3: etendue-limited
        assert_relative_eq!(src.source_etendue_mm2_sr(), 4.5, max_relative = 1e-12);
        assert_relative_eq!(
            src.etendue_usable_fraction(),
            3.3 / 4.5,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            src.power_at_if_w(),
            0.954_929_658_551_372 * 3.3 / 4.5,
            max_relative = 1e-12
        );
        // Radiance 10 W / (3 mm^2 x 2 pi) = 0.530516 W/mm^2/sr
        assert_relative_eq!(
            src.in_band_radiance_w_mm2_sr(),
            10.0 / (6.0 * PI),
            max_relative = 1e-12
        );
    }

    #[test]
    fn test_sn_power_chain_fixture_and_hvm_gap() {
        let src = DppSource::sn_13nm5(0.9).unwrap();
        // 18 kW x 2% = 360 W into 2 pi; x 1.5/(2 pi) x 0.4 = 34.3775 W
        assert_relative_eq!(src.in_band_power_2pi_w(), 360.0, max_relative = 1e-12);
        assert_relative_eq!(
            src.collected_power_w(),
            34.377_467_707_849_4,
            max_relative = 1e-12
        );
        // G_src = 0.4 x 1.0 x 1.5 = 0.6 < 3.3: not etendue-limited
        assert_relative_eq!(src.etendue_usable_fraction(), 1.0);
        assert_relative_eq!(
            src.average_power_w().unwrap(),
            34.377_467_707_849_4,
            max_relative = 1e-12
        );
        // 250 W / 34.3775 W = 7.27221x short of HVM
        let gap = src
            .derived_quantities()
            .into_iter()
            .find(|q| q.name == "hvm_power_gap")
            .unwrap()
            .value;
        assert_relative_eq!(gap, 7.272_205_216_643_039, max_relative = 1e-12);
        // Per-discharge energy at IF: 34.3775 W / 2 kHz = 17.1887 mJ, and the
        // 180 mJ into 2 pi of the TRINITI anchor.
        assert_relative_eq!(
            src.pulse_energy_j().unwrap(),
            1.718_873_385_392_47e-2,
            max_relative = 1e-12
        );
        let e2pi = src
            .derived_quantities()
            .into_iter()
            .find(|q| q.name == "in_band_pulse_energy_2pi_mj")
            .unwrap()
            .value;
        assert_relative_eq!(e2pi, 180.0, max_relative = 1e-12);
    }

    #[test]
    fn test_etendue_scaling() {
        let mut src = DppSource::xe_13nm5(0.9).unwrap();
        let g0 = src.source_etendue_mm2_sr();
        let f0 = src.etendue_usable_fraction();
        src.source_diameter_mm *= 2.0;
        // Doubling the pinch diameter doubles the etendue and, when limited,
        // halves the usable fraction.
        assert_relative_eq!(src.source_etendue_mm2_sr(), 2.0 * g0, max_relative = 1e-12);
        assert_relative_eq!(
            src.etendue_usable_fraction(),
            0.5 * f0,
            max_relative = 1e-12
        );
        // A generous illuminator removes the cut.
        src.illuminator_etendue_mm2_sr = 100.0;
        assert_relative_eq!(src.etendue_usable_fraction(), 1.0);
        assert_relative_eq!(
            src.power_at_if_w(),
            src.collected_power_w(),
            max_relative = 1e-12
        );
    }

    #[test]
    fn test_fuel_ordering_and_band() {
        let xe = DppSource::xe_13nm5(0.9).unwrap();
        let sn = DppSource::sn_13nm5(0.9).unwrap();
        assert!(sn.conversion_efficiency > xe.conversion_efficiency);
        // 13.5 nm -> 91.84 eV; 2% mirror band.
        assert_relative_eq!(sn.photon_energy_ev(), 91.84, epsilon = 0.01);
        assert_relative_eq!(
            sn.bandwidth_pm / (sn.wavelength_nm * 1e3),
            0.02,
            max_relative = 1e-12
        );
        for src in [xe, sn] {
            let sum: f64 = src.spectral_weights().iter().map(|(_, w)| w).sum();
            assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
            assert_eq!(src.transverse_coherence(), 0.0);
            assert_relative_eq!(src.shot_to_shot_rms(), 0.05);
        }
    }

    #[test]
    fn test_validation() {
        assert!(DppSource::xe_13nm5(0.0).is_err());
        assert!(DppSource::sn_13nm5(1.5).is_err());
        let base = DppSource::sn_13nm5(0.9).unwrap();
        let mut s = base.clone();
        s.conversion_efficiency = 0.0;
        assert!(s.validate().is_err());
        let mut s = base.clone();
        s.collector_solid_angle_sr = 7.0; // > 2 pi
        assert!(s.validate().is_err());
        let mut s = base.clone();
        s.collector_efficiency = 1.2;
        assert!(s.validate().is_err());
        let mut s = base.clone();
        s.source_diameter_mm = -0.1;
        assert!(s.validate().is_err());
        let mut s = base.clone();
        s.electrical_power_w = f64::NAN;
        assert!(s.validate().is_err());
        let mut s = base.clone();
        s.rep_rate_hz = 0.0;
        assert!(s.validate().is_err());
        let mut s = base;
        s.shot_to_shot_rms = 1.0;
        assert!(s.validate().is_err());
    }

    #[test]
    fn test_derived_quantities_finite() {
        for src in [
            DppSource::xe_13nm5(0.9).unwrap(),
            DppSource::sn_13nm5(0.9).unwrap(),
        ] {
            let dq = src.derived_quantities();
            assert!(dq.len() >= 8);
            assert!(dq.iter().all(|q| q.value.is_finite() && q.value >= 0.0));
            let heat = dq
                .iter()
                .find(|q| q.name == "electrode_heat_load_w")
                .unwrap()
                .value;
            assert!(heat > 0.97 * src.electrical_power_w);
        }
    }
}
