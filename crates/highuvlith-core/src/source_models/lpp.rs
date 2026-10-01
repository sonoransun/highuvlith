//! Laser-produced plasma (LPP) sources: Sn at 13.5 nm, Gd/Tb at 6.7/6.5 nm.
//!
//! A high-power drive laser (CO2 for Sn, solid-state for Gd/Tb research
//! sources) vaporizes droplet targets into a dense plasma whose line
//! emission lands in the mirror band: Sn XIII-XV around 13.5 nm (the
//! 2% band Mo/Si multilayers reflect), Gd/Tb around 6.7/6.5 nm (the
//! ~0.6% band of La/B4C multilayers, "beyond-EUV").
//!
//! # Model status
//!
//! Implemented physics with real-machine parameters: the in-band spectrum
//! (Gaussian within the mirror-selected bandwidth), pupil fill, and the
//! power chain `P_IF = P_drive x CE x eta_(2pi -> IF) x f_etendue` are live;
//! `average_power_w()` reports the in-band power at intermediate focus
//! (IF), the industry hand-off point and the input of the throughput model.
//! Simplified (🔶): the drive-laser type only sets the conversion-efficiency
//! default (reported for CO2 and 1 µm, a projection for 2 µm) and the
//! reported critical density; the étendue check assumes uniform phase-space
//! density (`f_etendue = min(1, G_ill / G_src)`). Plasma hydrodynamics,
//! opacity, debris, and collector degradation are out of scope. Plasma
//! emission is spatially incoherent: `transverse_coherence()` reports 0.
//!
//! # Key equations
//!
//! - In-band emission into 2π sr: `P_2pi = P_drive CE`
//! - Power at intermediate focus: `P_IF = P_2pi eta_IF f_etendue`
//! - Source étendue (small isotropic source, projected disk):
//!   `G_src = (pi d^2 / 4) Omega_coll`; usable fraction `min(1, G_ill / G_src)`
//! - Drive-laser critical density: `n_c = eps0 m_e omega^2 / e^2`
//!   (1.1e21 cm^-3 at 1 µm, 9.9e18 cm^-3 at 10.6 µm)
//!
//! # References
//!
//! - Sn preset: the NXE:3400B source — 250 W in-band at IF from a 21.5 kW
//!   CO2 drive at 6 % conversion efficiency and 50 kHz (Fomenkov, 2017 EUV
//!   Source Workshop). ASML describes ~25 µm tin droplets at 70 m/s hit by
//!   a flattening pre-pulse and a main pulse 50,000 times per second
//!   (Versolato's review quotes ~30 µm droplets); the emission comes from
//!   Sn⁸⁺–Sn¹⁴⁺ unresolved transition arrays into the 2 % band (O. O.
//!   Versolato, Plasma Sources Sci. Technol. 28, 083001 (2019)). Current
//!   sources are higher: the NXE:3800E ships with a 500 W source (ASML
//!   Investor Day, 2024; preset [`LppSource::sn_13nm5_500w`], whose drive
//!   chain is an assumed scaling) and 1 kW was demonstrated in 2025 (ASML
//!   Annual Report 2025).
//! - Gd/Tb 6.x nm: the best peer-reviewed conversion efficiencies inside
//!   the La/B mirrors' ~0.6 % band are 0.54–0.8 % (measured 2011–2014; e.g.
//!   Higashiguchi et al., Appl. Phys. Lett. 99, 191502 (2011): 0.54 % in
//!   0.6 %, the same measurement as the often-quoted 1.8 % in 2 %). No
//!   watt-level in-band 6.x nm power has been demonstrated, so the Gd/Tb
//!   in-band powers here are PROJECTIONS. Emission peaks near 6.775 nm (Gd)
//!   and 6.515 nm (Tb), while La/B multilayers peak at 6.63–6.65 nm (boron
//!   K-edge at 6.595 nm): the 6.7 nm preset sits slightly off the mirror
//!   peak.

use serde::{Deserialize, Serialize};

use super::physics;
use crate::source::{
    evaluate_illumination, evaluate_spectral_weights, DerivedQuantity, IlluminationShape,
    LithographySource, SpectralShape,
};

/// Plasma fuel element, which fixes the emission band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LppFuel {
    /// Tin: 13.5 nm (Mo/Si multilayer band). The production EUV fuel.
    Sn,
    /// Gadolinium: 6.7 nm (La/B4C multilayer band), "beyond-EUV".
    Gd,
    /// Terbium: 6.5 nm, alternative BEUV fuel.
    Tb,
}

/// Drive-laser technology. Sets the conversion-efficiency default and the
/// plasma critical density the laser couples into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LppDriveLaser {
    /// CO2 at 10.6 µm with a pre-pulse (production EUV sources).
    Co2,
    /// ~1 µm solid-state (Nd:YAG / Yb class).
    SolidState1um,
    /// ~2 µm thulium solid-state (a research direction for Sn sources).
    Thulium2um,
}

impl LppDriveLaser {
    /// Drive wavelength in µm.
    pub fn wavelength_um(&self) -> f64 {
        match self {
            LppDriveLaser::Co2 => 10.6,
            LppDriveLaser::SolidState1um => 1.064,
            LppDriveLaser::Thulium2um => 2.0,
        }
    }

    /// Default in-band (2π sr, 2% band) conversion efficiency for a Sn
    /// target and its provenance label. CO2 + pre-pulse: 6 % (reported for
    /// the NXE:3400B source; ASML quotes "> 5.5 %"); 1 µm: few-percent
    /// class (reported, laboratory); 2 µm: 4.5 % (PROJECTION — laboratory
    /// values are lower; the appeal is solid-state wall-plug efficiency).
    /// Order-of-magnitude defaults: override `conversion_efficiency` with
    /// your own data.
    pub fn sn_conversion_efficiency(&self) -> (f64, &'static str) {
        match self {
            LppDriveLaser::Co2 => (0.06, "reported (NXE:3400B source, CO2 + pre-pulse)"),
            LppDriveLaser::SolidState1um => (0.03, "reported (laboratory, few-percent class)"),
            LppDriveLaser::Thulium2um => (0.045, "projection (laboratory values are lower)"),
        }
    }

    /// Short label.
    pub fn label(&self) -> &'static str {
        match self {
            LppDriveLaser::Co2 => "co2",
            LppDriveLaser::SolidState1um => "solid_state_1um",
            LppDriveLaser::Thulium2um => "thulium_2um",
        }
    }
}

/// Source/collector geometry for the étendue check.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LppGeometry {
    /// Emitting-plasma diameter (FWHM) in µm.
    pub source_diameter_um: f64,
    /// Collector solid angle in sr.
    pub collection_solid_angle_sr: f64,
    /// Étendue the illuminator can accept, in mm^2·sr.
    pub illuminator_etendue_mm2_sr: f64,
}

impl LppGeometry {
    /// ASSUMED NXE-like geometry: 200 µm emitting region, ~5 sr collector,
    /// 3.3 mm^2·sr illuminator étendue budget (a commonly quoted EUV
    /// scanner figure; the real budget depends on NA, field and pupil fill).
    pub fn nxe_like() -> Self {
        Self {
            source_diameter_um: 200.0,
            collection_solid_angle_sr: 5.0,
            illuminator_etendue_mm2_sr: 3.3,
        }
    }

    /// Source étendue `(pi d^2 / 4) Omega` in mm^2·sr.
    pub fn source_etendue_mm2_sr(&self) -> f64 {
        let d_mm = self.source_diameter_um * 1e-3;
        std::f64::consts::PI * d_mm * d_mm / 4.0 * self.collection_solid_angle_sr
    }

    /// Fraction of the collected power the illuminator can accept:
    /// `min(1, G_ill / G_src)` (uniform phase-space density assumed).
    pub fn etendue_limited_fraction(&self) -> f64 {
        let g = self.source_etendue_mm2_sr();
        if g <= 0.0 {
            1.0
        } else {
            (self.illuminator_etendue_mm2_sr / g).min(1.0)
        }
    }
}

/// Laser-produced plasma source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LppSource {
    /// Plasma fuel (fixes the emission band).
    pub fuel: LppFuel,
    /// In-band center wavelength in nm (13.5 for Sn, 6.7 for Gd, 6.5 for Tb).
    pub wavelength_nm: f64,
    /// Mirror-selected in-band width, FWHM in pm (the multilayer stack,
    /// not the plasma, sets the usable bandwidth).
    pub bandwidth_pm: f64,
    /// Drive laser average power in W (CO2 for Sn: tens of kW).
    pub drive_laser_power_w: f64,
    /// Conversion efficiency from drive power to in-band radiation into
    /// 2π sr (Sn: ~0.06 in production; Gd/Tb: 0.0054–0.008 in the 0.6 %
    /// band, laboratory).
    pub conversion_efficiency: f64,
    /// Lumped efficiency from in-band emission into 2π sr to intermediate
    /// focus (collector solid angle x collector reflectance x debris /
    /// buffer-gas / spectral-purity losses). The presets use
    /// 250 / (21.5 kW x 0.06) ≈ 0.194, calibrated so the Sn preset
    /// reproduces the 250 W at IF of the NXE:3400B source. Earlier versions
    /// documented this field as a
    /// collector-to-wafer transport (preset 0.05); wafer-side optics losses
    /// now belong to the throughput model (`source_models::throughput`).
    pub transport_efficiency: f64,
    /// Droplet / pulse repetition rate in Hz (Sn production: ~50 kHz).
    pub rep_rate_hz: f64,
    /// Number of spectral samples for polychromatic simulation.
    pub spectral_samples: usize,
    /// Spectral line shape of the in-band emission.
    pub spectral_shape: SpectralShape,
    /// Illumination pupil shape (plasma is incoherent: large sigma disk).
    pub illumination: IlluminationShape,
    /// Drive-laser technology (`None` in configs written before the field
    /// existed: laser-specific derived quantities are then omitted).
    #[serde(default)]
    pub drive_laser: Option<LppDriveLaser>,
    /// Source/collector geometry for the étendue check (`None`: no étendue
    /// limit applied).
    #[serde(default)]
    pub geometry: Option<LppGeometry>,
}

/// NXE:3400B source drive power (W): 21.5 kW CO2 (Fomenkov 2017).
const NXE3400B_DRIVE_POWER_W: f64 = 21_500.0;

/// Lumped 2π -> IF efficiency of the presets (see `transport_efficiency`):
/// calibrated so 21.5 kW x 6 % CE lands on the NXE:3400B's 250 W at IF.
const PRESET_TRANSPORT_TO_IF: f64 = 250.0 / (NXE3400B_DRIVE_POWER_W * 0.06);

impl LppSource {
    /// Production-class Sn LPP at 13.5 nm — the NXE:3400B source: 21.5 kW
    /// CO2 drive, 6 % CE, 2 % mirror band (270 pm), 50 kHz droplets,
    /// ≈0.194 collection to IF → 250 W in-band at IF (~5 mJ per pulse).
    pub fn sn_13nm5(sigma: f64) -> crate::error::Result<Self> {
        Self::sn_with_drive_laser(sigma, LppDriveLaser::Co2)
    }

    /// NXE:3800E-class Sn LPP: 500 W in-band at IF, the source power ASML
    /// states for the NXE:3800E (ASML Investor Day, 14 Nov 2024). Only the
    /// 500 W is sourced: the 43 kW CO2 drive (twice the NXE:3400B chain at
    /// the same 6 % CE and collection) is an ASSUMED scaling, and the 50 kHz
    /// droplet rate is the NXE:3400B value (so ~10 mJ per pulse at IF).
    pub fn sn_13nm5_500w(sigma: f64) -> crate::error::Result<Self> {
        let mut src = Self::sn_13nm5(sigma)?;
        src.drive_laser_power_w = 2.0 * NXE3400B_DRIVE_POWER_W;
        Ok(src)
    }

    /// Sn LPP at 13.5 nm driven by the given laser technology at the same
    /// 21.5 kW drive power; the conversion efficiency defaults to
    /// [`LppDriveLaser::sn_conversion_efficiency`] (reported for CO2 and
    /// 1 µm, a projection for 2 µm).
    pub fn sn_with_drive_laser(
        sigma: f64,
        drive_laser: LppDriveLaser,
    ) -> crate::error::Result<Self> {
        crate::source::validate_sigma(sigma)?;
        Ok(Self {
            fuel: LppFuel::Sn,
            wavelength_nm: 13.5,
            bandwidth_pm: 270.0, // 2% of 13.5 nm
            drive_laser_power_w: NXE3400B_DRIVE_POWER_W,
            conversion_efficiency: drive_laser.sn_conversion_efficiency().0,
            transport_efficiency: PRESET_TRANSPORT_TO_IF,
            rep_rate_hz: 50_000.0,
            spectral_samples: 5,
            spectral_shape: SpectralShape::Gaussian,
            illumination: IlluminationShape::Conventional { sigma },
            drive_laser: Some(drive_laser),
            geometry: Some(LppGeometry::nxe_like()),
        })
    }

    /// Gd LPP at 6.7 nm (beyond-EUV): La/B4C mirror band ~0.6% (40 pm),
    /// conversion efficiency 0.7 % (inside the 0.54–0.8 % reported in the
    /// 0.6 % band), 10 kW 1 µm solid-state drive (assumed). Uses the
    /// Sn-calibrated collection to IF — optimistic, since La/B collectors
    /// reflect less than Mo/Si. The resulting in-band power (~13.6 W at IF)
    /// is a PROJECTION: no watt-level 6.x nm source has been demonstrated.
    pub fn gd_6nm7(sigma: f64) -> crate::error::Result<Self> {
        crate::source::validate_sigma(sigma)?;
        Ok(Self {
            fuel: LppFuel::Gd,
            wavelength_nm: 6.7,
            bandwidth_pm: 40.0,
            drive_laser_power_w: 10_000.0,
            conversion_efficiency: 0.007,
            transport_efficiency: PRESET_TRANSPORT_TO_IF,
            rep_rate_hz: 10_000.0,
            spectral_samples: 5,
            spectral_shape: SpectralShape::Gaussian,
            illumination: IlluminationShape::Conventional { sigma },
            drive_laser: Some(LppDriveLaser::SolidState1um),
            geometry: Some(LppGeometry::nxe_like()),
        })
    }

    /// Tb LPP at 6.5 nm, alternative BEUV fuel with similar (assumed)
    /// parameters to Gd; its in-band power is likewise a PROJECTION.
    pub fn tb_6nm5(sigma: f64) -> crate::error::Result<Self> {
        crate::source::validate_sigma(sigma)?;
        Ok(Self {
            fuel: LppFuel::Tb,
            wavelength_nm: 6.5,
            bandwidth_pm: 39.0,
            drive_laser_power_w: 10_000.0,
            conversion_efficiency: 0.006,
            transport_efficiency: PRESET_TRANSPORT_TO_IF,
            rep_rate_hz: 10_000.0,
            spectral_samples: 5,
            spectral_shape: SpectralShape::Gaussian,
            illumination: IlluminationShape::Conventional { sigma },
            drive_laser: Some(LppDriveLaser::SolidState1um),
            geometry: Some(LppGeometry::nxe_like()),
        })
    }

    /// In-band power emitted into 2π sr, in W: drive power x CE.
    pub fn in_band_emission_w(&self) -> f64 {
        self.drive_laser_power_w * self.conversion_efficiency
    }

    /// Étendue-limited usable fraction (1 without a geometry).
    pub fn etendue_limited_fraction(&self) -> f64 {
        self.geometry
            .map(|g| g.etendue_limited_fraction())
            .unwrap_or(1.0)
    }

    /// In-band power at intermediate focus, in W:
    /// drive power x CE x (2π -> IF efficiency) x étendue-limited fraction.
    pub fn in_band_power_w(&self) -> f64 {
        self.in_band_emission_w() * self.transport_efficiency * self.etendue_limited_fraction()
    }
}

impl Default for LppSource {
    fn default() -> Self {
        Self::sn_13nm5(0.9).expect("default sigma 0.9 is valid")
    }
}

impl LithographySource for LppSource {
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

    fn pulse_energy_j(&self) -> Option<f64> {
        if self.rep_rate_hz > 0.0 {
            Some(self.in_band_power_w() / self.rep_rate_hz)
        } else {
            None
        }
    }

    fn rep_rate_hz(&self) -> Option<f64> {
        Some(self.rep_rate_hz)
    }

    fn average_power_w(&self) -> Option<f64> {
        Some(self.in_band_power_w())
    }

    // Plasma emission is spatially incoherent: keep the trait default
    // transverse_coherence() = 0.

    fn derived_quantities(&self) -> Vec<DerivedQuantity> {
        let lambda = self.wavelength_nm;
        let p_if = self.in_band_power_w();
        let mut out = vec![
            DerivedQuantity::new(
                "photon_energy",
                physics::HC_EV_NM / lambda,
                "eV",
                "hc / lambda",
            ),
            DerivedQuantity::new(
                "in_band_emission_2pi",
                self.in_band_emission_w(),
                "W",
                "drive power x conversion efficiency (in-band, 2pi sr)",
            ),
        ];
        if let Some(laser) = self.drive_laser {
            let (ce_default, provenance) = laser.sn_conversion_efficiency();
            out.push(DerivedQuantity::new(
                "drive_laser_wavelength",
                laser.wavelength_um(),
                "um",
                laser.label(),
            ));
            out.push(DerivedQuantity::new(
                "critical_density",
                physics::plasma_critical_density_cm3(laser.wavelength_um()),
                "cm^-3",
                "eps0 m_e omega^2 / e^2: longer drive wavelengths deposit energy in \
                 lower-density, optically thinner plasma (less opacity broadening of the \
                 Sn UTA)",
            ));
            if self.fuel == LppFuel::Sn {
                out.push(DerivedQuantity::new(
                    "conversion_efficiency_default",
                    ce_default,
                    "-",
                    provenance,
                ));
            }
        }
        if let Some(g) = self.geometry {
            let g_src = g.source_etendue_mm2_sr();
            out.push(DerivedQuantity::new(
                "source_etendue",
                g_src,
                "mm^2 sr",
                "(pi d^2 / 4) x collector solid angle (assumed geometry)",
            ));
            out.push(DerivedQuantity::new(
                "etendue_margin",
                g.illuminator_etendue_mm2_sr / g_src,
                "-",
                "illuminator etendue / source etendue (> 1: not etendue-limited)",
            ));
            out.push(DerivedQuantity::new(
                "etendue_limited_fraction",
                g.etendue_limited_fraction(),
                "-",
                "min(1, G_ill / G_src), uniform phase-space density assumed",
            ));
        }
        out.push(DerivedQuantity::new(
            "in_band_power_at_if",
            p_if,
            "W",
            if self.fuel == LppFuel::Sn {
                "P_2pi x 2pi->IF efficiency x etendue fraction (= average_power_w)"
            } else {
                "PROJECTION (no watt-level 6.x nm in-band source demonstrated): \
                 P_2pi x Sn-calibrated 2pi->IF efficiency x etendue fraction"
            },
        ));
        if self.rep_rate_hz > 0.0 {
            out.push(DerivedQuantity::new(
                "pulse_energy_at_if",
                p_if / self.rep_rate_hz,
                "J",
                "in-band energy per droplet at IF",
            ));
        }
        out.push(DerivedQuantity::new(
            "photon_rate_at_if",
            physics::watts_to_photon_rate(p_if, lambda),
            "photons/s",
            "in-band photons per second at IF",
        ));
        if p_if > 0.0 {
            out.push(DerivedQuantity::new(
                "drive_watts_per_if_watt",
                self.drive_laser_power_w / p_if,
                "-",
                "drive-laser power needed per in-band watt at IF",
            ));
        }
        out.push(DerivedQuantity::new(
            "hvm_power_ratio",
            p_if / super::throughput::HVM_EUV_POWER_AT_IF_W,
            "-",
            "in-band power at IF / 250 W (production 13.5 nm HVM)",
        ));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn dq(src: &LppSource, name: &str) -> f64 {
        src.derived_quantities()
            .into_iter()
            .find(|q| q.name == name)
            .unwrap_or_else(|| panic!("missing derived quantity {name}"))
            .value
    }

    #[test]
    fn test_sn_preset_band_and_energy() {
        let src = LppSource::sn_13nm5(0.9).unwrap();
        assert_relative_eq!(src.wavelength_nm(), 13.5);
        // 13.5 nm photon: hc/lambda = 91.84 eV
        assert_relative_eq!(src.photon_energy_ev(), 91.84, epsilon = 0.01);
        // 2% band
        assert_relative_eq!(
            src.bandwidth_pm / (src.wavelength_nm * 1e3),
            0.02,
            epsilon = 1e-9
        );
    }

    #[test]
    fn test_power_chain_is_live() {
        let src = LppSource::sn_13nm5(0.9).unwrap();
        // NXE:3400B: 21.5 kW x 6 % = 1290 W into 2pi; calibrated collection
        // -> 250 W at IF; etendue margin > 1 so no further loss.
        assert_relative_eq!(src.in_band_emission_w(), 1290.0, epsilon = 1e-9);
        assert_relative_eq!(src.in_band_power_w(), 250.0, epsilon = 1e-9);
        assert_relative_eq!(src.average_power_w().unwrap(), 250.0, epsilon = 1e-9);
        assert_relative_eq!(src.transport_efficiency, 250.0 / 1290.0, epsilon = 1e-15);
        // Per-droplet in-band energy at IF: 250 W / 50 kHz = 5 mJ
        assert_relative_eq!(src.pulse_energy_j().unwrap(), 5.0e-3, epsilon = 1e-12);
        // Photon rate at IF: 250 W / 91.84 eV = 1.699e19 photons/s
        assert_relative_eq!(
            dq(&src, "photon_rate_at_if"),
            250.0 / (1239.84193 / 13.5 * 1.602176634e-19),
            max_relative = 1e-12
        );
        assert_relative_eq!(dq(&src, "hvm_power_ratio"), 1.0, epsilon = 1e-12);
        // NXE:3800E class: 500 W at IF (assumed 43 kW drive, same chain).
        let e = LppSource::sn_13nm5_500w(0.9).unwrap();
        assert_relative_eq!(e.average_power_w().unwrap(), 500.0, max_relative = 1e-12);
        assert_relative_eq!(e.pulse_energy_j().unwrap(), 1.0e-2, max_relative = 1e-12);
        assert_relative_eq!(dq(&e, "hvm_power_ratio"), 2.0, max_relative = 1e-12);
    }

    #[test]
    fn test_etendue_check() {
        // 200 um, 5 sr: G = pi (0.2 mm)^2 / 4 x 5 = 0.1571 mm^2 sr; margin 21.
        let src = LppSource::sn_13nm5(0.9).unwrap();
        assert_relative_eq!(
            dq(&src, "source_etendue"),
            0.157_079_632_679_489_66,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            dq(&src, "etendue_margin"),
            21.008_452_488_130_185,
            max_relative = 1e-9
        );
        assert_relative_eq!(src.etendue_limited_fraction(), 1.0);
        // A DPP-like 1 mm source overfills the 3.3 mm^2 sr budget: 84 %.
        let big = LppSource {
            geometry: Some(LppGeometry {
                source_diameter_um: 1000.0,
                ..LppGeometry::nxe_like()
            }),
            ..src.clone()
        };
        assert_relative_eq!(
            big.etendue_limited_fraction(),
            0.840_338_099_525_207_3,
            max_relative = 1e-9
        );
        assert_relative_eq!(
            big.average_power_w().unwrap(),
            250.0 * 0.840_338_099_525_207_3,
            max_relative = 1e-9
        );
        // No geometry: no etendue limit (legacy configs).
        let legacy = LppSource {
            geometry: None,
            ..src
        };
        assert_relative_eq!(legacy.in_band_power_w(), 250.0, epsilon = 1e-9);
    }

    #[test]
    fn test_drive_laser_options() {
        let co2 = LppSource::sn_with_drive_laser(0.9, LppDriveLaser::Co2).unwrap();
        let yag = LppSource::sn_with_drive_laser(0.9, LppDriveLaser::SolidState1um).unwrap();
        let tm = LppSource::sn_with_drive_laser(0.9, LppDriveLaser::Thulium2um).unwrap();
        // Same drive power: CE ordering sets IF power ordering.
        assert!(co2.average_power_w() > tm.average_power_w());
        assert!(tm.average_power_w() > yag.average_power_w());
        // Critical densities: 9.9e18 (CO2) vs 1.1e21 / 1.064^2 (1 um class).
        assert_relative_eq!(
            dq(&co2, "critical_density"),
            9.922_162_835_276_704e18,
            max_relative = 1e-9
        );
        assert_relative_eq!(
            dq(&yag, "critical_density"),
            1.114_854_216_171_690_5e21 / (1.064 * 1.064),
            max_relative = 1e-9
        );
        assert_eq!(
            LppSource::sn_13nm5(0.9).unwrap().drive_laser,
            Some(LppDriveLaser::Co2)
        );
    }

    #[test]
    fn test_beuv_presets_in_band() {
        let gd = LppSource::gd_6nm7(0.9).unwrap();
        assert_relative_eq!(gd.wavelength_nm(), 6.7);
        assert!((184.0..186.0).contains(&gd.photon_energy_ev()));
        // 10 kW x 0.7 % x 0.1938 = 13.57 W at IF (a projection): ~18x short
        // of the 250 W Sn benchmark.
        assert_relative_eq!(
            gd.average_power_w().unwrap(),
            70.0 * 250.0 / 1290.0,
            max_relative = 1e-12
        );
        let note = gd
            .derived_quantities()
            .into_iter()
            .find(|q| q.name == "in_band_power_at_if")
            .unwrap()
            .note;
        assert!(note.contains("PROJECTION"));

        let tb = LppSource::tb_6nm5(0.9).unwrap();
        assert_relative_eq!(tb.wavelength_nm(), 6.5);
        assert_relative_eq!(
            tb.average_power_w().unwrap(),
            60.0 * 250.0 / 1290.0,
            max_relative = 1e-12
        );
    }

    #[test]
    fn test_spectral_weights_sum_to_one() {
        for src in [
            LppSource::sn_13nm5(0.9).unwrap(),
            LppSource::gd_6nm7(0.9).unwrap(),
        ] {
            let sum: f64 = src.spectral_weights().iter().map(|(_, w)| w).sum();
            assert_relative_eq!(sum, 1.0, epsilon = 1e-12);
        }
    }

    #[test]
    fn test_incoherent_plasma() {
        let src = LppSource::sn_13nm5(0.9).unwrap();
        assert_eq!(src.transverse_coherence(), 0.0);
    }

    #[test]
    fn test_invalid_sigma_rejected() {
        assert!(LppSource::sn_13nm5(0.0).is_err());
        assert!(LppSource::gd_6nm7(1.5).is_err());
    }

    #[test]
    fn test_legacy_toml_without_new_fields_parses() {
        // A pre-WP-C serialized LppSource (no drive_laser / geometry).
        let toml_str = r#"
            fuel = "sn"
            wavelength_nm = 13.5
            bandwidth_pm = 270.0
            drive_laser_power_w = 25000.0
            conversion_efficiency = 0.055
            transport_efficiency = 0.05
            rep_rate_hz = 50000.0
            spectral_samples = 5
            spectral_shape = "Gaussian"
            [illumination.Conventional]
            sigma = 0.9
        "#;
        let src: LppSource = toml::from_str(toml_str).unwrap();
        assert!(src.drive_laser.is_none() && src.geometry.is_none());
        assert_relative_eq!(src.average_power_w().unwrap(), 68.75, epsilon = 1e-9);
    }
}
