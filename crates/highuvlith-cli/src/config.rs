use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct SimConfig {
    pub source: SourceConfig,
    pub optics: OpticsConfig,
    pub mask: MaskConfig,
    #[serde(default)]
    pub grid: GridConfig,
    #[serde(default)]
    pub process: ProcessConfig,
}

/// Source configuration section of the TOML.
///
/// Back-compatible: if `type` is absent, it defaults to `"vuv"`, so
/// existing configs that specify only `wavelength_nm`, `sigma`, and
/// `bandwidth_pm` continue to work unchanged. Available `type` tags:
/// `"vuv"`, `"lpa_fel"`, `"lpp"`, `"synchrotron"`, `"hhg"`, `"xfel"`,
/// `"ics"`, `"ssmb"`, `"entangled"`. Family-specific fields are all
/// optional and default to each family's reference-machine preset;
/// they are only consulted when the matching `type` is selected.
///
/// For sources whose wavelength is DERIVED from machine parameters
/// (synchrotron undulators), an explicitly given `wavelength_nm` that
/// disagrees with the derived value by more than 5% is rejected.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct SourceConfig {
    #[serde(rename = "type")]
    pub source_type: Option<String>,
    pub wavelength_nm: Option<f64>,
    pub sigma: Option<f64>,
    pub bandwidth_pm: Option<f64>,
    // --- LPA-FEL / XFEL shared pulse fields ---
    pub electron_energy_mev: Option<f64>,
    pub pulse_duration_fs: Option<f64>,
    pub rep_rate_hz: Option<f64>,
    pub pulse_energy_uj: Option<f64>,
    // --- LPP (type = "lpp") ---
    /// Plasma fuel: "sn" (13.5 nm), "gd" (6.7 nm), or "tb" (6.5 nm).
    pub fuel: Option<String>,
    pub drive_laser_power_w: Option<f64>,
    pub conversion_efficiency: Option<f64>,
    pub transport_efficiency: Option<f64>,
    // --- Synchrotron (type = "synchrotron") ---
    /// Beamline: "undulator" (default) or "bending_magnet".
    pub beamline: Option<String>,
    pub electron_energy_gev: Option<f64>,
    pub field_t: Option<f64>,
    pub period_mm: Option<f64>,
    pub undulator_k: Option<f64>,
    pub num_periods: Option<usize>,
    /// Odd harmonic (shared by synchrotron undulator and HHG).
    pub harmonic: Option<usize>,
    pub ring_current_ma: Option<f64>,
    // --- HHG (type = "hhg") ---
    /// Generation gas: "helium"/"he", "neon"/"ne", "argon"/"ar",
    /// "krypton"/"kr", or "xenon"/"xe".
    pub gas: Option<String>,
    pub driver_wavelength_nm: Option<f64>,
    pub driver_intensity_w_cm2: Option<f64>,
    pub monochromator_bandwidth_pm: Option<f64>,
    /// true = full-comb bookkeeping mode (no monochromator).
    pub full_comb: Option<bool>,
    pub pulse_energy_nj: Option<f64>,
    // --- XFEL (type = "xfel") ---
    /// Mode: "sase" (default) or "seeded".
    pub mode: Option<String>,
    pub pierce_parameter: Option<f64>,
    pub rel_bandwidth: Option<f64>,
    // --- ICS (type = "ics") ---
    pub laser_wavelength_nm: Option<f64>,
    pub laser_a0: Option<f64>,
    pub collection_half_angle_mrad: Option<f64>,
    pub electron_energy_spread_rel: Option<f64>,
    // --- SSMB (type = "ssmb") ---
    pub modulation_wavelength_nm: Option<f64>,
    pub average_power_w: Option<f64>,
    pub ring_energy_mev: Option<f64>,
    // --- Entangled (type = "entangled") ---
    pub num_photons: Option<usize>,
    pub fidelity: Option<f64>,
}

impl SourceConfig {
    pub fn source_type(&self) -> &str {
        self.source_type.as_deref().unwrap_or("vuv")
    }

    pub fn sigma(&self) -> f64 {
        self.sigma.unwrap_or(0.7)
    }
}

/// Optics configuration. The `type` tag selects the optical system:
/// `"refractive"` (default, CaF2 projection lens — VUV only),
/// `"schwarzschild"` (two-mirror reflective — EUV/BEUV/soft X-ray), or
/// `"zone_plate"` (Fresnel diffractive — X-ray). Refractive optics at
/// wavelengths below 50 nm are physically impossible (no transparent
/// lens material) and produce a warning.
#[derive(Debug, Deserialize)]
pub struct OpticsConfig {
    #[serde(rename = "type")]
    pub optics_type: Option<String>,
    #[serde(default = "default_na")]
    pub na: f64,
    #[serde(default = "default_flare")]
    pub flare_fraction: f64,
    /// Zone-plate outermost zone width (nm); defaults to lambda/(2 NA).
    pub outer_zone_width_nm: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct MaskConfig {
    #[serde(default = "default_cd")]
    pub cd_nm: f64,
    #[serde(default = "default_pitch")]
    pub pitch_nm: f64,
}

#[derive(Debug, Deserialize)]
pub struct GridConfig {
    #[serde(default = "default_grid_size")]
    pub size: usize,
    #[serde(default = "default_pixel")]
    pub pixel_nm: f64,
}

#[derive(Debug, Deserialize)]
pub struct ProcessConfig {
    #[serde(default = "default_dose")]
    pub dose_mj_cm2: f64,
    #[serde(default)]
    pub focus_nm: f64,
}

fn default_na() -> f64 {
    0.75
}
fn default_flare() -> f64 {
    0.02
}
fn default_cd() -> f64 {
    65.0
}
fn default_pitch() -> f64 {
    180.0
}
fn default_grid_size() -> usize {
    256
}
fn default_pixel() -> f64 {
    1.0
}
fn default_dose() -> f64 {
    30.0
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            size: default_grid_size(),
            pixel_nm: default_pixel(),
        }
    }
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            dose_mj_cm2: default_dose(),
            focus_nm: 0.0,
        }
    }
}

impl SimConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Validate all configuration values for physical and computational correctness.
    pub fn validate(&self) -> anyhow::Result<()> {
        if let Some(w) = self.source.wavelength_nm {
            if w <= 0.0 {
                anyhow::bail!("wavelength_nm must be > 0, got {}", w);
            }
        }
        if self.optics.na <= 0.0 || self.optics.na > 1.0 {
            anyhow::bail!("NA must be in (0, 1.0], got {}", self.optics.na);
        }
        let sigma = self.source.sigma();
        if sigma <= 0.0 || sigma > 1.0 {
            anyhow::bail!("sigma must be in (0, 1.0], got {}", sigma);
        }
        // Building the source runs every per-family check (known type
        // tag, odd harmonics, HHG cutoff, SSMB harmonic consistency,
        // derived-wavelength cross-checks, ...).
        self.to_source()?;
        if self.mask.cd_nm <= 0.0 {
            anyhow::bail!("cd_nm must be > 0, got {}", self.mask.cd_nm);
        }
        if self.mask.pitch_nm <= 0.0 {
            anyhow::bail!("pitch_nm must be > 0, got {}", self.mask.pitch_nm);
        }
        if self.mask.cd_nm >= self.mask.pitch_nm {
            anyhow::bail!(
                "cd_nm ({}) must be < pitch_nm ({})",
                self.mask.cd_nm,
                self.mask.pitch_nm
            );
        }
        if self.grid.size == 0 || (self.grid.size & (self.grid.size - 1)) != 0 {
            anyhow::bail!("grid size must be a power of 2, got {}", self.grid.size);
        }
        if self.grid.pixel_nm <= 0.0 {
            anyhow::bail!("pixel_nm must be > 0, got {}", self.grid.pixel_nm);
        }
        Ok(())
    }

    pub fn to_source(&self) -> anyhow::Result<highuvlith_core::source::SourceKind> {
        use highuvlith_core::source::{
            EntangledPhotonSource, HhgGas, HhgSource, IcsSource, IlluminationShape,
            LithographySource, LpaFelSource, LppSource, SourceKind, SpectralShape, SsmbSource,
            SynchrotronBeamline, SynchrotronSource, VuvSource, XfelMode, XfelSource,
        };
        let s = &self.source;
        let sigma = s.sigma();
        let err = |e: highuvlith_core::error::LithographyError| anyhow::anyhow!("{}", e);

        match s.source_type() {
            "vuv" => Ok(SourceKind::Vuv(VuvSource {
                wavelength_nm: s.wavelength_nm.unwrap_or(157.63),
                bandwidth_pm: s.bandwidth_pm.unwrap_or(1.1),
                spectral_samples: 5,
                spectral_shape: SpectralShape::Lorentzian,
                pulse_energy_mj: 10.0,
                rep_rate_hz: s.rep_rate_hz.unwrap_or(4000.0),
                illumination: IlluminationShape::Conventional { sigma },
            })),
            "lpa_fel" => {
                let mut fel =
                    LpaFelSource::new(s.wavelength_nm.unwrap_or(25.0), sigma).map_err(err)?;
                if let Some(bw) = s.bandwidth_pm {
                    fel.bandwidth_pm = bw.max(0.0);
                }
                if let Some(e) = s.electron_energy_mev {
                    fel.electron_energy_mev = e;
                }
                if let Some(tau) = s.pulse_duration_fs {
                    fel.pulse_duration_fs = tau;
                }
                if let Some(rr) = s.rep_rate_hz {
                    fel.rep_rate_hz = rr;
                }
                if let Some(pe) = s.pulse_energy_uj {
                    fel.pulse_energy_uj = pe;
                }
                Ok(SourceKind::LpaFel(fel))
            }
            "lpp" => {
                let mut lpp = match s.fuel.as_deref().unwrap_or("sn") {
                    "sn" => LppSource::sn_13nm5(sigma).map_err(err)?,
                    "gd" => LppSource::gd_6nm7(sigma).map_err(err)?,
                    "tb" => LppSource::tb_6nm5(sigma).map_err(err)?,
                    other => anyhow::bail!(
                        "unknown lpp fuel '{}' (expected 'sn', 'gd', or 'tb')",
                        other
                    ),
                };
                if let Some(w) = s.wavelength_nm {
                    lpp.wavelength_nm = w;
                }
                if let Some(bw) = s.bandwidth_pm {
                    lpp.bandwidth_pm = bw;
                }
                if let Some(p) = s.drive_laser_power_w {
                    lpp.drive_laser_power_w = p;
                }
                if let Some(ce) = s.conversion_efficiency {
                    lpp.conversion_efficiency = ce;
                }
                if let Some(te) = s.transport_efficiency {
                    lpp.transport_efficiency = te;
                }
                if let Some(rr) = s.rep_rate_hz {
                    lpp.rep_rate_hz = rr;
                }
                Ok(SourceKind::Lpp(lpp))
            }
            "synchrotron" => match s.beamline.as_deref().unwrap_or("undulator") {
                "undulator" => {
                    let mut src = SynchrotronSource::undulator(
                        s.electron_energy_gev.unwrap_or(0.538),
                        s.period_mm.unwrap_or(20.0),
                        s.undulator_k.unwrap_or(1.0),
                        s.num_periods.unwrap_or(100),
                        s.harmonic.unwrap_or(1),
                    )
                    .map_err(err)?;
                    if let Some(ma) = s.ring_current_ma {
                        src.ring_current_ma = ma;
                    }
                    // Derived-wavelength cross-check: the resonance
                    // condition, not the config, sets the wavelength.
                    if let Some(w) = s.wavelength_nm {
                        let derived = src.wavelength_nm();
                        if ((w - derived) / derived).abs() > 0.05 {
                            anyhow::bail!(
                                "wavelength_nm = {w} nm disagrees with the undulator \
                                 resonance {derived:.3} nm derived from the machine \
                                 parameters by more than 5%; omit wavelength_nm or fix \
                                 electron_energy_gev / period_mm / undulator_k / harmonic"
                            );
                        }
                    }
                    Ok(SourceKind::Synchrotron(src))
                }
                "bending_magnet" => {
                    let src = SynchrotronSource {
                        beamline: SynchrotronBeamline::BendingMagnet {
                            electron_energy_gev: s.electron_energy_gev.unwrap_or(2.5),
                            field_t: s.field_t.unwrap_or(1.5),
                            selected_wavelength_nm: s.wavelength_nm.unwrap_or(0.2),
                            mono_bandwidth_pm: s.bandwidth_pm.unwrap_or(0.2),
                        },
                        ring_current_ma: s.ring_current_ma.unwrap_or(200.0),
                        spectral_samples: 5,
                        illumination: IlluminationShape::Conventional { sigma: 0.8 },
                        transverse_coherence_fraction: 0.0,
                    };
                    Ok(SourceKind::Synchrotron(src))
                }
                other => anyhow::bail!(
                    "unknown synchrotron beamline '{}' (expected 'undulator' or 'bending_magnet')",
                    other
                ),
            },
            "hhg" => {
                let gas = match s.gas.as_deref().unwrap_or("neon") {
                    "helium" | "he" => HhgGas::Helium,
                    "neon" | "ne" => HhgGas::Neon,
                    "argon" | "ar" => HhgGas::Argon,
                    "krypton" | "kr" => HhgGas::Krypton,
                    "xenon" | "xe" => HhgGas::Xenon,
                    other => anyhow::bail!(
                        "unknown hhg gas '{}' (expected helium/neon/argon/krypton/xenon)",
                        other
                    ),
                };
                let mut src = HhgSource::new(
                    s.driver_wavelength_nm.unwrap_or(800.0),
                    gas,
                    s.driver_intensity_w_cm2.unwrap_or(4e14),
                    s.harmonic.unwrap_or(59),
                    s.monochromator_bandwidth_pm.unwrap_or(15.0),
                )
                .map_err(err)?;
                if s.full_comb == Some(true) {
                    src.monochromator = None;
                }
                if let Some(pe) = s.pulse_energy_nj {
                    src.pulse_energy_nj = pe;
                }
                if let Some(rr) = s.rep_rate_hz {
                    src.rep_rate_hz = rr;
                }
                if let Some(tau) = s.pulse_duration_fs {
                    src.pulse_duration_fs = tau;
                }
                Ok(SourceKind::Hhg(src))
            }
            "xfel" => {
                let mut src = XfelSource::flash_13nm5();
                src.mode = match s.mode.as_deref().unwrap_or("sase") {
                    "sase" => XfelMode::Sase {
                        pierce_parameter: s.pierce_parameter.unwrap_or(3e-3),
                    },
                    "seeded" | "self_seeded" => XfelMode::SelfSeeded {
                        rel_bandwidth: s.rel_bandwidth.unwrap_or(5e-5),
                    },
                    other => anyhow::bail!(
                        "unknown xfel mode '{}' (expected 'sase' or 'seeded')",
                        other
                    ),
                };
                if let Some(w) = s.wavelength_nm {
                    src.wavelength_nm = w;
                }
                if let Some(pe) = s.pulse_energy_uj {
                    src.pulse_energy_uj = pe;
                }
                if let Some(tau) = s.pulse_duration_fs {
                    src.pulse_duration_fs = tau;
                }
                if let Some(rr) = s.rep_rate_hz {
                    src.rep_rate_hz = rr;
                }
                Ok(SourceKind::Xfel(src))
            }
            "ics" => {
                let mut src = IcsSource::for_wavelength(
                    s.wavelength_nm.unwrap_or(13.5),
                    s.laser_wavelength_nm.unwrap_or(1030.0),
                    s.laser_a0.unwrap_or(0.1),
                )
                .map_err(err)?;
                // The wavelength is the input here and the electron energy
                // is DERIVED from the Compton condition. If the config
                // also pins electron_energy_mev, it must satisfy
                // lambda_X = lambda_L (1 + a0^2/2) / (4 gamma^2) within 5%.
                if let Some(e_mev) = s.electron_energy_mev {
                    let derived = src.electron_energy_mev;
                    if ((e_mev - derived) / derived).abs() > 0.05 {
                        anyhow::bail!(
                            "electron_energy_mev = {e_mev} MeV is inconsistent with the \
                             Compton condition for the requested wavelength (needs \
                             ~{derived:.3} MeV); omit electron_energy_mev or fix \
                             wavelength_nm / laser_wavelength_nm / laser_a0"
                        );
                    }
                    src.electron_energy_mev = e_mev;
                }
                if let Some(a) = s.collection_half_angle_mrad {
                    src.collection_half_angle_mrad = a;
                }
                if let Some(es) = s.electron_energy_spread_rel {
                    src.electron_energy_spread_rel = es;
                }
                if let Some(pe) = s.pulse_energy_nj {
                    src.pulse_energy_nj = pe;
                }
                if let Some(rr) = s.rep_rate_hz {
                    src.rep_rate_hz = rr;
                }
                Ok(SourceKind::Ics(src))
            }
            "ssmb" => {
                let src = SsmbSource::new(
                    s.ring_energy_mev.unwrap_or(400.0),
                    s.modulation_wavelength_nm.unwrap_or(1053.0),
                    s.wavelength_nm.unwrap_or(13.5),
                    s.average_power_w.unwrap_or(1000.0),
                )
                .map_err(err)?;
                Ok(SourceKind::Ssmb(src))
            }
            "entangled" => {
                let src = EntangledPhotonSource::noon(
                    s.wavelength_nm.unwrap_or(157.63),
                    s.num_photons.unwrap_or(2),
                    s.fidelity.unwrap_or(1.0),
                )
                .map_err(err)?;
                Ok(SourceKind::Entangled(src))
            }
            other => anyhow::bail!(
                "unknown source type '{}' (expected one of: vuv, lpa_fel, lpp, synchrotron, \
                 hhg, xfel, ics, ssmb, entangled)",
                other
            ),
        }
    }

    pub fn to_optics(&self) -> anyhow::Result<Box<dyn highuvlith_core::optics::OpticalSystem>> {
        use highuvlith_core::optics::schwarzschild::SchwarzschildObjective;
        use highuvlith_core::optics::zone_plate::FresnelZonePlate;
        use highuvlith_core::source::LithographySource;

        let wavelength = self.to_source()?.wavelength_nm();
        match self.optics.optics_type.as_deref().unwrap_or("refractive") {
            "refractive" => {
                if wavelength < 50.0 {
                    eprintln!(
                        "warning: refractive CaF2 optics selected at {wavelength:.2} nm — no \
                         transparent lens material exists below ~110 nm; results are not \
                         physical. Set [optics] type = \"schwarzschild\" or \"zone_plate\"."
                    );
                }
                let mut optics = highuvlith_core::optics::ProjectionOptics::new(self.optics.na)
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                optics.flare_fraction = self.optics.flare_fraction;
                Ok(Box::new(optics))
            }
            "schwarzschild" => {
                // Start from the preset matching the wavelength band,
                // then apply the configured NA / flare.
                let mut objective = if wavelength < 3.0 {
                    SchwarzschildObjective::soft_xray(self.optics.na)
                        .map_err(|e| anyhow::anyhow!("{}", e))?
                } else if wavelength < 10.0 {
                    SchwarzschildObjective::beuv()
                } else {
                    SchwarzschildObjective::euv_standard()
                };
                objective.numerical_aperture = self.optics.na;
                objective.flare = self.optics.flare_fraction;
                Ok(Box::new(objective))
            }
            "zone_plate" => {
                let zone_width = self
                    .optics
                    .outer_zone_width_nm
                    .unwrap_or(wavelength / (2.0 * self.optics.na));
                let zp = FresnelZonePlate::new(zone_width, wavelength)
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                Ok(Box::new(zp))
            }
            other => anyhow::bail!(
                "unknown optics type '{}' (expected 'refractive', 'schwarzschild', or 'zone_plate')",
                other
            ),
        }
    }

    pub fn to_mask(&self) -> anyhow::Result<highuvlith_core::mask::Mask> {
        highuvlith_core::mask::Mask::line_space(self.mask.cd_nm, self.mask.pitch_nm)
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn to_grid(&self) -> anyhow::Result<highuvlith_core::types::GridConfig> {
        highuvlith_core::types::GridConfig::new(self.grid.size, self.grid.pixel_nm)
            .map_err(|e| anyhow::anyhow!("{}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_config() -> SimConfig {
        SimConfig {
            source: SourceConfig {
                source_type: Some("vuv".to_string()),
                wavelength_nm: Some(157.63),
                sigma: Some(0.7),
                bandwidth_pm: Some(1.1),
                ..SourceConfig::default()
            },
            optics: OpticsConfig {
                optics_type: None,
                na: 0.75,
                flare_fraction: 0.02,
                outer_zone_width_nm: None,
            },
            mask: MaskConfig {
                cd_nm: 65.0,
                pitch_nm: 180.0,
            },
            grid: GridConfig {
                size: 256,
                pixel_nm: 1.0,
            },
            process: ProcessConfig {
                dose_mj_cm2: 30.0,
                focus_nm: 0.0,
            },
        }
    }

    #[test]
    fn test_valid_config_passes() {
        valid_config().validate().unwrap();
    }

    #[test]
    fn test_zero_wavelength() {
        let mut cfg = valid_config();
        cfg.source.wavelength_nm = Some(0.0);
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_na_out_of_range() {
        let mut cfg = valid_config();
        cfg.optics.na = 1.5;
        assert!(cfg.validate().is_err());

        cfg.optics.na = 0.0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_sigma_out_of_range() {
        let mut cfg = valid_config();
        cfg.source.sigma = Some(0.0);
        assert!(cfg.validate().is_err());

        cfg.source.sigma = Some(1.1);
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_cd_gte_pitch() {
        let mut cfg = valid_config();
        cfg.mask.cd_nm = 200.0;
        cfg.mask.pitch_nm = 180.0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_grid_not_power_of_two() {
        let mut cfg = valid_config();
        cfg.grid.size = 100;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_zero_pixel() {
        let mut cfg = valid_config();
        cfg.grid.pixel_nm = 0.0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_unknown_source_type_rejected() {
        let mut cfg = valid_config();
        cfg.source.source_type = Some("tachyon".to_string());
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_synchrotron_source_type_accepted() {
        // Once a rejected placeholder — now a real source family.
        let mut cfg = valid_config();
        cfg.source.source_type = Some("synchrotron".to_string());
        cfg.source.wavelength_nm = None; // derived from machine parameters
        cfg.validate().unwrap();
        assert!(matches!(
            cfg.to_source().unwrap(),
            highuvlith_core::source::SourceKind::Synchrotron(_)
        ));
    }

    #[test]
    fn test_undulator_derived_wavelength_cross_check() {
        let mut cfg = valid_config();
        cfg.source.source_type = Some("synchrotron".to_string());
        // Machine params give ~13.5 nm; asking for 20 nm must fail.
        cfg.source.wavelength_nm = Some(20.0);
        assert!(cfg.validate().is_err());
        // Consistent request passes.
        cfg.source.wavelength_nm = Some(13.5);
        cfg.validate().unwrap();
    }

    #[test]
    fn test_lpa_fel_source_type_accepted() {
        let mut cfg = valid_config();
        cfg.source.source_type = Some("lpa_fel".to_string());
        cfg.source.wavelength_nm = Some(25.0);
        cfg.source.bandwidth_pm = Some(25.0);
        cfg.source.electron_energy_mev = Some(500.0);
        cfg.validate().unwrap();
        let src = cfg.to_source().unwrap();
        assert!(matches!(
            src,
            highuvlith_core::source::SourceKind::LpaFel(_)
        ));
    }

    #[test]
    fn test_all_new_source_types_build() {
        for tag in ["lpp", "hhg", "xfel", "ics", "ssmb", "entangled"] {
            let mut cfg = valid_config();
            cfg.source.source_type = Some(tag.to_string());
            cfg.source.wavelength_nm = None; // use family defaults
            cfg.validate()
                .unwrap_or_else(|e| panic!("{tag} should validate: {e}"));
            let src = cfg.to_source().unwrap();
            assert_eq!(src.kind_label(), tag);
        }
    }

    #[test]
    fn test_ics_electron_energy_cross_check() {
        let mut cfg = valid_config();
        cfg.source.source_type = Some("ics".to_string());
        cfg.source.wavelength_nm = Some(13.5);
        // 13.5 nm from a 1030 nm laser needs ~1.73 MeV; 10 MeV is
        // inconsistent with the Compton condition.
        cfg.source.electron_energy_mev = Some(10.0);
        assert!(cfg.validate().is_err());
        // A consistent value passes.
        cfg.source.electron_energy_mev = Some(1.73);
        cfg.validate().unwrap();
        // Omitting it derives the energy silently.
        cfg.source.electron_energy_mev = None;
        cfg.validate().unwrap();
    }

    #[test]
    fn test_hhg_cutoff_violation_rejected_from_config() {
        let mut cfg = valid_config();
        cfg.source.source_type = Some("hhg".to_string());
        cfg.source.wavelength_nm = None;
        cfg.source.gas = Some("argon".to_string());
        cfg.source.driver_intensity_w_cm2 = Some(2e14);
        cfg.source.harmonic = Some(59); // beyond the Ar cutoff at 2e14
        assert!(cfg.validate().is_err());
        cfg.source.harmonic = Some(27); // within cutoff
        cfg.validate().unwrap();
    }

    #[test]
    fn test_toml_hhg_parsing() {
        let toml_str = r#"
            [source]
            type = "hhg"
            gas = "ne"
            driver_wavelength_nm = 800.0
            driver_intensity_w_cm2 = 4.0e14
            harmonic = 59
            monochromator_bandwidth_pm = 15.0

            [optics]
            na = 0.3

            [mask]
            cd_nm = 30.0
            pitch_nm = 100.0
        "#;
        let cfg: SimConfig = toml::from_str(toml_str).unwrap();
        cfg.validate().unwrap();
        match cfg.to_source().unwrap() {
            highuvlith_core::source::SourceKind::Hhg(s) => {
                use highuvlith_core::source::LithographySource;
                assert!((s.wavelength_nm() - 800.0 / 59.0).abs() < 1e-9);
            }
            _ => panic!("expected Hhg variant"),
        }
    }

    #[test]
    fn test_toml_back_compat_without_type_tag() {
        let toml_str = r#"
            [source]
            wavelength_nm = 157.63
            sigma = 0.7
            bandwidth_pm = 1.1

            [optics]
            na = 0.75

            [mask]
            cd_nm = 65.0
            pitch_nm = 180.0
        "#;
        let cfg: SimConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.source.source_type(), "vuv");
        cfg.validate().unwrap();
        assert!(matches!(
            cfg.to_source().unwrap(),
            highuvlith_core::source::SourceKind::Vuv(_)
        ));
    }

    #[test]
    fn test_toml_lpa_fel_parsing() {
        let toml_str = r#"
            [source]
            type = "lpa_fel"
            wavelength_nm = 25.0
            sigma = 0.7
            bandwidth_pm = 25.0
            electron_energy_mev = 500.0
            pulse_duration_fs = 10.0
            rep_rate_hz = 1000.0

            [optics]
            na = 0.55

            [mask]
            cd_nm = 30.0
            pitch_nm = 100.0
        "#;
        let cfg: SimConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.source.source_type(), "lpa_fel");
        assert_eq!(cfg.source.electron_energy_mev, Some(500.0));
        cfg.validate().unwrap();
        match cfg.to_source().unwrap() {
            highuvlith_core::source::SourceKind::LpaFel(s) => {
                assert!((s.wavelength_nm - 25.0).abs() < 1e-9);
                assert!((s.electron_energy_mev - 500.0).abs() < 1e-9);
                assert!((s.pulse_duration_fs - 10.0).abs() < 1e-9);
            }
            _ => panic!("expected LpaFel variant"),
        }
    }
}
