//! Optics, imaging-settings, mask and grid view model.
//!
//! - Optics: automatic (refractive above 110 nm, reflective EUV projection
//!   optics below), dry or water-immersion refractive lenses (NA > 1), EUV
//!   projection optics with an optional central obscuration (NXE 0.33 /
//!   High-NA 0.55 presets), Schwarzschild and zone-plate objectives; the
//!   reflective EUV/Schwarzschild optics can carry the core's optional
//!   angle-dependent multilayer pupil (user-supplied incidence-angle maps).
//! - The engine's `ImagingSettings`: defocus model, scalar or vector
//!   imaging with the illumination polarization, normalization, kernels.
//! - The spectrum mode: one wavelength, the narrow-band focus-shift sum, or
//!   an exact TCC per spectral sample.
//! - The automatic commensurate grid: a field holding a whole number of
//!   pitches with a pixel fine enough to represent the whole band
//!   `(1 + σ)·NA/λ` that reaches the pupil.

use highuvlith_core::aerial::{DefocusModel, ImageNormalization, ImagingModel, ImagingSettings};
use highuvlith_core::mask::{Mask, MaskType};
use highuvlith_core::materials::multilayer::MultilayerMirror;
use highuvlith_core::optics::euv::EuvProjectionOptics;
use highuvlith_core::optics::multilayer_pupil::{MirrorAngleMap, MultilayerPupil};
use highuvlith_core::optics::schwarzschild::SchwarzschildObjective;
use highuvlith_core::optics::vector::{IlluminationPolarization, VectorSettings};
use highuvlith_core::optics::zone_plate::FresnelZonePlate;
use highuvlith_core::optics::{
    OpticalSystem, ProjectionOptics, IMMERSION_NA_MARGIN, WATER_INDEX_193NM,
};
use highuvlith_core::source::IlluminationShape;

/// Below this wavelength no transparent lens material exists (CaF₂/LiF
/// absorb), so the automatic choice switches to reflective optics.
pub const REFRACTIVE_MIN_WAVELENGTH_NM: f64 = 110.0;

/// Optics family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum OpticsKind {
    /// Refractive above 110 nm, EUV projection (reflective) below.
    #[default]
    Auto,
    /// Dry refractive projection lens (NA < 1).
    RefractiveDry,
    /// Refractive lens with an immersion fluid (NA up to 0.95·n).
    Immersion,
    /// Reflective EUV projection optics, optional central obscuration.
    EuvProjection,
    /// Two-mirror Schwarzschild objective (centrally obscured).
    Schwarzschild,
    /// Fresnel zone plate (NA = λ / (2 Δr_N)).
    ZonePlate,
}

impl OpticsKind {
    /// All kinds, in menu order.
    pub const ALL: [OpticsKind; 6] = [
        OpticsKind::Auto,
        OpticsKind::RefractiveDry,
        OpticsKind::Immersion,
        OpticsKind::EuvProjection,
        OpticsKind::Schwarzschild,
        OpticsKind::ZonePlate,
    ];

    /// Menu label.
    pub fn label(self) -> &'static str {
        match self {
            OpticsKind::Auto => "Auto (by wavelength)",
            OpticsKind::RefractiveDry => "Refractive lens (dry)",
            OpticsKind::Immersion => "Refractive lens + immersion",
            OpticsKind::EuvProjection => "EUV projection optics (reflective)",
            OpticsKind::Schwarzschild => "Schwarzschild objective",
            OpticsKind::ZonePlate => "Fresnel zone plate",
        }
    }

    /// The concrete kind `Auto` stands for at `wavelength_nm`.
    pub fn resolve(self, wavelength_nm: f64) -> OpticsKind {
        match self {
            OpticsKind::Auto if wavelength_nm >= REFRACTIVE_MIN_WAVELENGTH_NM => {
                OpticsKind::RefractiveDry
            }
            OpticsKind::Auto => OpticsKind::EuvProjection,
            other => other,
        }
    }

    /// Valid NA slider range of a resolved kind.
    pub fn na_range(self, immersion_index: f64) -> (f64, f64) {
        match self {
            OpticsKind::Auto | OpticsKind::RefractiveDry => (0.05, 0.95),
            OpticsKind::Immersion => (0.05, IMMERSION_NA_MARGIN * immersion_index.max(1.0)),
            OpticsKind::EuvProjection => (0.05, 0.75),
            OpticsKind::Schwarzschild => (0.05, 0.6),
            OpticsKind::ZonePlate => (0.01, 0.5),
        }
    }

    /// NA a resolved kind starts at.
    pub fn default_na(self) -> f64 {
        match self {
            OpticsKind::Auto | OpticsKind::RefractiveDry => 0.75,
            OpticsKind::Immersion => 1.35,
            OpticsKind::EuvProjection => 0.33,
            OpticsKind::Schwarzschild => 0.3,
            OpticsKind::ZonePlate => 0.1,
        }
    }

    /// Whether the kind has a central obscuration parameter.
    pub fn has_obscuration(self) -> bool {
        matches!(self, OpticsKind::EuvProjection | OpticsKind::Schwarzschild)
    }
}

/// Optics panel state.
#[derive(Clone, Debug, PartialEq)]
pub struct OpticsParams {
    pub kind: OpticsKind,
    pub na: f64,
    /// Immersion-fluid index (water at 193 nm: 1.437).
    pub immersion_index: f64,
    /// Central obscuration radius as a fraction of the NA.
    pub central_obscuration: f64,
    /// Uniform flare fraction.
    pub flare: f64,
    /// Optional angle-dependent multilayer pupil (reflective optics only).
    pub multilayer: MultilayerParams,
}

/// Multilayer coating of the angle-dependent pupil.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coating {
    MoSi,
    LaB4C,
    LaB,
}

impl Coating {
    /// All coatings, in menu order.
    pub const ALL: [Coating; 3] = [Coating::MoSi, Coating::LaB4C, Coating::LaB];

    /// Menu label.
    pub fn label(self) -> &'static str {
        match self {
            Coating::MoSi => "Mo/Si (13.5 nm)",
            Coating::LaB4C => "La/B\u{2084}C (6.x nm)",
            Coating::LaB => "La/B (6.6\u{2013}6.7 nm)",
        }
    }

    /// `(periods, period_nm, heavy-layer fraction)` of the core's ideal
    /// designs: Mo/Si 40 × 6.9 nm (peak 73 % at 13.48 nm); La/B₄C and La/B
    /// 200 × 3.37 / 3.33 nm (tuned to 6.7 / 6.65 nm).
    pub fn default_design(self) -> (usize, f64, f64) {
        match self {
            Coating::MoSi => (40, 6.9, 0.4),
            Coating::LaB4C => (200, 3.37, 0.4),
            Coating::LaB => (200, 3.329, 0.4),
        }
    }
}

/// Panel state of the optional multilayer pupil. Every mirror gets the
/// same incidence-angle map `θ(p) = |θc + t·(p·u) + g·|p|²|` — a user
/// ASSUMPTION standing in for a ray trace of a real design.
#[derive(Clone, Debug, PartialEq)]
pub struct MultilayerParams {
    pub enabled: bool,
    pub coating: Coating,
    pub periods: usize,
    pub period_nm: f64,
    pub gamma: f64,
    pub mirrors: usize,
    pub center_deg: f64,
    pub tilt_deg: f64,
    pub azimuth_deg: f64,
    pub radial_deg: f64,
}

impl Default for MultilayerParams {
    /// Off; when switched on: two Mo/Si mirrors whose incidence angle grows
    /// from 0° at the pupil centre to 15° at the rim (the core's
    /// illustrative test case).
    fn default() -> Self {
        let (periods, period_nm, gamma) = Coating::MoSi.default_design();
        Self {
            enabled: false,
            coating: Coating::MoSi,
            periods,
            period_nm,
            gamma,
            mirrors: 2,
            center_deg: 0.0,
            tilt_deg: 0.0,
            azimuth_deg: 0.0,
            radial_deg: 15.0,
        }
    }
}

impl MultilayerParams {
    /// Switch coating and load its default design.
    pub fn set_coating(&mut self, coating: Coating) {
        self.coating = coating;
        (self.periods, self.period_nm, self.gamma) = coating.default_design();
    }

    /// The coating stack.
    pub fn mirror(&self) -> Result<MultilayerMirror, String> {
        let (n, d, g) = (self.periods, self.period_nm, self.gamma);
        match self.coating {
            Coating::MoSi => MultilayerMirror::mo_si(n, d, g),
            Coating::LaB4C => MultilayerMirror::la_b4c(n, d, g),
            Coating::LaB => MultilayerMirror::la_b(n, d, g),
        }
        .map_err(|e| e.to_string())
    }

    /// The core pupil (`None` when disabled).
    pub fn build(&self) -> Result<Option<MultilayerPupil>, String> {
        if !self.enabled {
            return Ok(None);
        }
        if !(1..=12).contains(&self.mirrors) {
            return Err(format!(
                "multilayer mirror count must be 1-12 (got {})",
                self.mirrors
            ));
        }
        let map = MirrorAngleMap {
            center_deg: self.center_deg,
            tilt_deg: self.tilt_deg,
            azimuth_deg: self.azimuth_deg,
            radial_deg: self.radial_deg,
        };
        MultilayerPupil::new(self.mirror()?, vec![map; self.mirrors])
            .map(Some)
            .map_err(|e| e.to_string())
    }

    /// Unpolarized single-mirror reflectance at `wavelength_nm` and the
    /// pupil-centre angle (shows whether the coating is tuned to the source).
    pub fn center_reflectance(&self, wavelength_nm: f64) -> Result<f64, String> {
        use highuvlith_core::types::Polarization;
        let m = self.mirror()?;
        let theta = self.center_deg.to_radians();
        let rs = m
            .reflectance(wavelength_nm, theta, Polarization::TE)
            .map_err(|e| e.to_string())?;
        let rp = m
            .reflectance(wavelength_nm, theta, Polarization::TM)
            .map_err(|e| e.to_string())?;
        Ok(0.5 * (rs + rp))
    }
}

impl Default for OpticsParams {
    fn default() -> Self {
        Self {
            kind: OpticsKind::Auto,
            na: 0.75,
            immersion_index: WATER_INDEX_193NM,
            central_obscuration: 0.0,
            flare: 0.02,
            multilayer: MultilayerParams::default(),
        }
    }
}

/// Named optics presets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpticsPreset {
    /// ArF water immersion: NA 1.35, n = 1.437.
    Immersion193i,
    /// NXE-class EUV: NA 0.33, unobscured.
    Nxe033,
    /// High-NA EUV: NA 0.55, 0.2·NA obscuration (an ASSUMED value).
    HighNa055,
}

impl OpticsPreset {
    /// All presets, in menu order.
    pub const ALL: [OpticsPreset; 3] = [
        OpticsPreset::Immersion193i,
        OpticsPreset::Nxe033,
        OpticsPreset::HighNa055,
    ];

    /// Command-line name (`--optics`).
    pub fn cli_name(self) -> &'static str {
        match self {
            OpticsPreset::Immersion193i => "193i",
            OpticsPreset::Nxe033 => "nxe",
            OpticsPreset::HighNa055 => "high-na",
        }
    }

    /// Parse an `--optics` value.
    pub fn from_cli_name(name: &str) -> Option<OpticsPreset> {
        OpticsPreset::ALL.into_iter().find(|p| p.cli_name() == name)
    }

    /// Button label.
    pub fn label(self) -> &'static str {
        match self {
            OpticsPreset::Immersion193i => "193i (NA 1.35, water)",
            OpticsPreset::Nxe033 => "EUV NXE (NA 0.33)",
            OpticsPreset::HighNa055 => "High-NA (NA 0.55)",
        }
    }
}

impl OpticsParams {
    /// Apply a preset (values from the core presets).
    pub fn apply_preset(&mut self, preset: OpticsPreset) {
        match preset {
            OpticsPreset::Immersion193i => {
                let o = ProjectionOptics::immersion_193i();
                self.kind = OpticsKind::Immersion;
                self.na = o.na;
                self.immersion_index = o.immersion_index;
                self.flare = o.flare_fraction;
            }
            OpticsPreset::Nxe033 | OpticsPreset::HighNa055 => {
                let o = if preset == OpticsPreset::Nxe033 {
                    EuvProjectionOptics::nxe_033()
                } else {
                    EuvProjectionOptics::high_na_055()
                };
                self.kind = OpticsKind::EuvProjection;
                self.na = o.numerical_aperture;
                self.central_obscuration = o.central_obscuration;
                self.flare = o.flare;
            }
        }
    }

    /// Clamp the NA into the valid range of the kind resolved at
    /// `wavelength_nm`, resetting it to the kind's default when it falls
    /// outside (e.g. NA 1.35 left over from immersion on EUV optics).
    pub fn conform_to(&mut self, wavelength_nm: f64) {
        let kind = self.kind.resolve(wavelength_nm);
        let (lo, hi) = kind.na_range(self.immersion_index);
        if !(self.na >= lo && self.na <= hi) {
            self.na = kind.default_na().clamp(lo, hi);
        }
    }
}

/// Built optics plus what the panel reports about them.
pub struct BuiltOptics {
    pub optics: Box<dyn OpticalSystem>,
    /// Concrete kind (after `Auto`).
    pub kind: OpticsKind,
    /// One-line description.
    pub label: String,
    /// Honesty warnings about the choice.
    pub warnings: Vec<String>,
}

/// Build the optics for a source at `wavelength_nm`.
pub fn build_optics(p: &OpticsParams, wavelength_nm: f64) -> Result<BuiltOptics, String> {
    let kind = p.kind.resolve(wavelength_nm);
    let mut warnings = Vec::new();
    if !(p.flare.is_finite() && (0.0..1.0).contains(&p.flare)) {
        return Err(format!("flare must be in [0, 1) (got {})", p.flare));
    }
    let obsc = p.central_obscuration;
    let (optics, label): (Box<dyn OpticalSystem>, String) = match kind {
        OpticsKind::RefractiveDry | OpticsKind::Auto => {
            let mut o = ProjectionOptics::new(p.na).map_err(|e| e.to_string())?;
            o.flare_fraction = p.flare;
            if wavelength_nm < REFRACTIVE_MIN_WAVELENGTH_NM {
                warnings.push(format!(
                    "refractive optics at {wavelength_nm:.2} nm: no transparent lens material \
                     exists below ~110 nm, so this result is not physical"
                ));
            }
            (
                Box::new(o),
                format!("refractive lens, NA {:.2} (dry)", p.na),
            )
        }
        OpticsKind::Immersion => {
            let mut o =
                ProjectionOptics::immersion(p.na, p.immersion_index).map_err(|e| e.to_string())?;
            o.flare_fraction = p.flare;
            if wavelength_nm < REFRACTIVE_MIN_WAVELENGTH_NM {
                warnings.push(format!(
                    "immersion lens at {wavelength_nm:.2} nm: no transparent lens or fluid \
                     exists at this wavelength"
                ));
            }
            (
                Box::new(o),
                format!(
                    "immersion lens, NA {:.2} in n = {:.3} (sin\u{3b8} = {:.3})",
                    p.na,
                    p.immersion_index,
                    p.na / p.immersion_index
                ),
            )
        }
        OpticsKind::EuvProjection => {
            let mut o = EuvProjectionOptics::new(p.na, obsc).map_err(|e| e.to_string())?;
            o.flare = p.flare;
            o.multilayer = p.multilayer.build()?;
            (
                Box::new(o),
                format!(
                    "EUV projection optics, NA {:.2}, obscuration {:.2}\u{b7}NA (isotropic \
                     wafer-side pupil){}",
                    p.na,
                    obsc,
                    multilayer_suffix(&p.multilayer)
                ),
            )
        }
        OpticsKind::Schwarzschild => {
            let mut o = if wavelength_nm < 3.0 {
                SchwarzschildObjective::soft_xray(p.na).map_err(|e| e.to_string())?
            } else if wavelength_nm < 10.0 {
                SchwarzschildObjective::beuv()
            } else {
                SchwarzschildObjective::euv_standard()
            };
            if !(p.na > 0.0 && p.na < 1.0) {
                return Err(format!("NA must be in (0, 1) (got {})", p.na));
            }
            if !(0.0..1.0).contains(&obsc) {
                return Err(format!("obscuration must be in [0, 1) (got {obsc})"));
            }
            o.numerical_aperture = p.na;
            o.obscuration_ratio = obsc;
            o.flare = p.flare;
            o.multilayer = p.multilayer.build()?;
            (
                Box::new(o),
                format!(
                    "Schwarzschild objective, NA {:.2}, obscuration {:.2}{}",
                    p.na,
                    obsc,
                    multilayer_suffix(&p.multilayer)
                ),
            )
        }
        OpticsKind::ZonePlate => {
            if !(p.na > 0.0 && p.na < 1.0) {
                return Err(format!("NA must be in (0, 1) (got {})", p.na));
            }
            let zone_nm = wavelength_nm / (2.0 * p.na);
            let o = FresnelZonePlate::new(zone_nm, wavelength_nm).map_err(|e| e.to_string())?;
            (
                Box::new(o),
                format!(
                    "zone plate, outer zone {zone_nm:.2} nm (NA {:.2}, efficiency-limited)",
                    p.na
                ),
            )
        }
    };
    Ok(BuiltOptics {
        optics,
        kind,
        label,
        warnings,
    })
}

fn multilayer_suffix(m: &MultilayerParams) -> String {
    if m.enabled {
        format!(
            ", {} \u{d7} {} multilayer pupil",
            m.mirrors,
            m.coating.label()
        )
    } else {
        String::new()
    }
}

/// Spectrum treatment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SpectrumMode {
    /// Centre wavelength only.
    #[default]
    Monochromatic,
    /// Centre-wavelength kernels with a chromatic focus shift per sample
    /// (`compute_polychromatic`; Δλ/λ ≪ 1 only).
    NarrowBand,
    /// TCC rebuilt at every spectral sample (`compute_multiwavelength`).
    PerWavelength,
}

impl SpectrumMode {
    /// All modes, in menu order.
    pub const ALL: [SpectrumMode; 3] = [
        SpectrumMode::Monochromatic,
        SpectrumMode::NarrowBand,
        SpectrumMode::PerWavelength,
    ];

    /// Menu label.
    pub fn label(self) -> &'static str {
        match self {
            SpectrumMode::Monochromatic => "Monochromatic (centre \u{3bb})",
            SpectrumMode::NarrowBand => "Narrow-band (focus shift per sample)",
            SpectrumMode::PerWavelength => "Per-wavelength (exact TCC per sample)",
        }
    }
}

/// Illumination polarization for vector imaging.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PolarizationChoice {
    #[default]
    Unpolarized,
    X,
    Y,
    Te,
    Tm,
}

impl PolarizationChoice {
    /// All states, in menu order.
    pub const ALL: [PolarizationChoice; 5] = [
        PolarizationChoice::Unpolarized,
        PolarizationChoice::X,
        PolarizationChoice::Y,
        PolarizationChoice::Te,
        PolarizationChoice::Tm,
    ];

    /// Menu label.
    pub fn label(self) -> &'static str {
        match self {
            PolarizationChoice::Unpolarized => "Unpolarized",
            PolarizationChoice::X => "Linear X",
            PolarizationChoice::Y => "Linear Y",
            PolarizationChoice::Te => "TE (azimuthal)",
            PolarizationChoice::Tm => "TM (radial)",
        }
    }

    fn to_core(self) -> IlluminationPolarization {
        match self {
            PolarizationChoice::Unpolarized => IlluminationPolarization::Unpolarized,
            PolarizationChoice::X => IlluminationPolarization::X,
            PolarizationChoice::Y => IlluminationPolarization::Y,
            PolarizationChoice::Te => IlluminationPolarization::Te,
            PolarizationChoice::Tm => IlluminationPolarization::Tm,
        }
    }
}

/// Imaging-settings panel state.
#[derive(Clone, Debug, PartialEq)]
pub struct ImagingParams {
    pub defocus_model: DefocusModel,
    /// Vector (polarized) instead of scalar imaging.
    pub vector: bool,
    pub polarization: PolarizationChoice,
    /// Radiometric (obliquity) factor in vector mode.
    pub obliquity: bool,
    pub normalization: ImageNormalization,
    pub spectrum: SpectrumMode,
    /// Spectral samples (`None` = the source's own sampling).
    pub spectral_samples: Option<usize>,
    pub max_kernels: usize,
}

impl Default for ImagingParams {
    fn default() -> Self {
        Self {
            defocus_model: DefocusModel::Exact,
            vector: false,
            polarization: PolarizationChoice::Unpolarized,
            obliquity: true,
            normalization: ImageNormalization::ClearField,
            spectrum: SpectrumMode::Monochromatic,
            spectral_samples: None,
            max_kernels: 32,
        }
    }
}

/// Engine settings for the panel state. The vector image index is left at
/// 1 so the engine inherits the optics' immersion index (one image-space
/// medium), and `reduction` is replaced by the optics' value.
pub fn build_settings(p: &ImagingParams) -> ImagingSettings {
    let imaging_model = if p.vector {
        ImagingModel::Vector(VectorSettings {
            polarization: p.polarization.to_core(),
            obliquity: p.obliquity,
            ..VectorSettings::default()
        })
    } else {
        ImagingModel::Scalar
    };
    ImagingSettings {
        max_kernels: p.max_kernels.max(1),
        defocus_model: p.defocus_model,
        imaging_model,
        normalization: p.normalization,
        kernel_cache_capacity: 64,
        ..ImagingSettings::default()
    }
}

/// Mask pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Pattern {
    /// Opaque lines of width CD on a bright field (repeating along x).
    #[default]
    LineSpace,
    /// Square clear holes of side CD on a dark field, square lattice.
    ContactArray,
}

/// Mask panel state.
#[derive(Clone, Debug, PartialEq)]
pub struct MaskParams {
    pub pattern: Pattern,
    pub cd_nm: f64,
    pub pitch_nm: f64,
    /// Reverse the field tone (clear lines / opaque holes).
    pub reverse_tone: bool,
    /// Attenuated phase-shift absorber (6 %, 180°) instead of binary.
    pub att_psm: bool,
}

impl Default for MaskParams {
    fn default() -> Self {
        Self {
            pattern: Pattern::LineSpace,
            cd_nm: 65.0,
            pitch_nm: 180.0,
            reverse_tone: false,
            att_psm: false,
        }
    }
}

/// Build the mask.
pub fn build_mask(p: &MaskParams) -> Result<Mask, String> {
    let mut mask = match p.pattern {
        Pattern::LineSpace => Mask::line_space(p.cd_nm, p.pitch_nm),
        Pattern::ContactArray => Mask::contact_hole(p.cd_nm, p.pitch_nm, p.pitch_nm),
    }
    .map_err(|e| e.to_string())?;
    if p.reverse_tone {
        mask.dark_field = !mask.dark_field;
    }
    if p.att_psm {
        mask.mask_type = MaskType::AttenuatedPSM {
            transmission: 0.06,
            phase_deg: 180.0,
        };
    }
    Ok(mask)
}

/// Radius (σ units) of the pupil fill's support: the largest |s| carrying
/// light. A coherent Gaussian is cut where it falls below 1e-4 of its peak
/// (the engine's source-sampling threshold), `σ·√(2 ln 10⁴)`.
pub fn sigma_extent(shape: &IlluminationShape) -> f64 {
    let extent = match shape {
        IlluminationShape::Conventional { sigma } => *sigma,
        IlluminationShape::Annular { sigma_outer, .. } => *sigma_outer,
        IlluminationShape::Dipole {
            sigma_center,
            sigma_radius,
            ..
        }
        | IlluminationShape::Quadrupole {
            sigma_center,
            sigma_radius,
            ..
        } => sigma_center + sigma_radius,
        IlluminationShape::CoherentGaussian { sigma } => sigma * (2.0 * 1e4_f64.ln()).sqrt(),
    };
    extent.clamp(0.0, 1.0)
}

/// Largest pixel that represents the full band `(1 + σ)·NA/λ` reaching the
/// pupil: `λ / (2·NA·(1 + σ))`.
pub fn nyquist_pixel_nm(wavelength_nm: f64, na: f64, sigma_extent: f64) -> f64 {
    wavelength_nm / (2.0 * na * (1.0 + sigma_extent))
}

/// Grid panel state.
#[derive(Clone, Debug, PartialEq)]
pub struct GridParams {
    /// Pixels per side (power of two).
    pub size: usize,
    /// Pitches the field should hold (automatic pixel only).
    pub periods: usize,
    /// Choose the pixel automatically (Nyquist-safe, `periods` pitches).
    pub auto_pixel: bool,
    /// Target pixel (nm) when not automatic.
    pub pixel_nm: f64,
}

impl Default for GridParams {
    fn default() -> Self {
        Self {
            size: 128,
            periods: 2,
            auto_pixel: true,
            pixel_nm: 2.0,
        }
    }
}

/// The chosen grid and what to report about it.
#[derive(Clone, Debug, PartialEq)]
pub struct GridChoice {
    pub size: usize,
    pub pixel_nm: f64,
    pub field_nm: f64,
    /// Pitches in the field (a whole number).
    pub periods: f64,
    /// Pixel the commensurate rule started from.
    pub target_pixel_nm: f64,
    /// Nyquist-safe pixel `λ/(2·NA·(1+σ))`.
    pub nyquist_pixel_nm: f64,
    /// Whether the chosen pixel represents the whole transmitted band.
    pub resolves_band: bool,
}

impl GridChoice {
    /// The core grid.
    pub fn grid(&self) -> highuvlith_core::types::GridConfig {
        highuvlith_core::types::GridConfig {
            size: self.size,
            pixel_nm: self.pixel_nm,
        }
    }
}

/// Choose a commensurate grid for `mask`: automatic target pixel
/// `min(Nyquist pixel, periods·pitch/size)` (or the user's pixel), then the
/// largest pixel ≤ target whose field holds whole pitches
/// ([`Mask::commensurate_grid`]). When one pitch does not fit at the target
/// pixel the pixel coarsens to `pitch/size` and `resolves_band` reports
/// whether it still meets the Nyquist limit.
pub fn choose_grid(
    mask: &Mask,
    params: &GridParams,
    nyquist_pixel_nm: f64,
) -> Result<GridChoice, String> {
    if !params.size.is_power_of_two() || params.size < 8 {
        return Err(format!(
            "grid size must be a power of two \u{2265} 8 (got {})",
            params.size
        ));
    }
    let pitch = mask
        .periodicity()
        .map(|(p, q)| q.map_or(p, |q| p.max(q)))
        .ok_or("the mask has no periodic pattern")?;
    let target = if params.auto_pixel {
        let periods = params.periods.max(1) as f64;
        nyquist_pixel_nm.min(periods * pitch / params.size as f64)
    } else {
        params.pixel_nm
    };
    if !(target.is_finite() && target > 0.0) {
        return Err(format!("pixel must be positive (got {target})"));
    }
    let grid = mask
        .commensurate_grid(params.size, target)
        .map_err(|e| e.to_string())?;
    let field = grid.field_size_nm();
    Ok(GridChoice {
        size: grid.size,
        pixel_nm: grid.pixel_nm,
        field_nm: field,
        periods: (field / pitch).round(),
        target_pixel_nm: target,
        nyquist_pixel_nm,
        resolves_band: grid.pixel_nm <= nyquist_pixel_nm * (1.0 + 1e-9),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_optics_switches_to_reflective_below_110_nm() {
        assert_eq!(OpticsKind::Auto.resolve(193.368), OpticsKind::RefractiveDry);
        assert_eq!(OpticsKind::Auto.resolve(157.63), OpticsKind::RefractiveDry);
        assert_eq!(OpticsKind::Auto.resolve(13.5), OpticsKind::EuvProjection);
        assert_eq!(OpticsKind::Immersion.resolve(13.5), OpticsKind::Immersion);
        let p = OpticsParams {
            na: 0.33,
            ..OpticsParams::default()
        };
        let built = build_optics(&p, 13.5).unwrap();
        assert_eq!(built.kind, OpticsKind::EuvProjection);
        assert!(built.warnings.is_empty());
        // A refractive lens forced below 110 nm is flagged.
        let forced = OpticsParams {
            kind: OpticsKind::RefractiveDry,
            na: 0.3,
            ..OpticsParams::default()
        };
        assert_eq!(build_optics(&forced, 13.5).unwrap().warnings.len(), 1);
    }

    #[test]
    fn immersion_allows_na_above_one_dry_does_not() {
        let mut p = OpticsParams::default();
        p.apply_preset(OpticsPreset::Immersion193i);
        assert_eq!(p.kind, OpticsKind::Immersion);
        let built = build_optics(&p, 193.368).unwrap();
        assert!((built.optics.na() - 1.35).abs() < 1e-12);
        assert!((built.optics.immersion_index() - WATER_INDEX_193NM).abs() < 1e-12);
        // The same NA without the fluid is rejected by the core.
        p.kind = OpticsKind::RefractiveDry;
        assert!(build_optics(&p, 193.368).is_err());
        // NA beyond 0.95·n is rejected.
        p.kind = OpticsKind::Immersion;
        p.na = 1.40;
        assert!(build_optics(&p, 193.368).is_err());
        // Switching to EUV optics conforms the leftover NA to its default.
        p.kind = OpticsKind::EuvProjection;
        p.conform_to(13.5);
        assert!((p.na - 0.33).abs() < 1e-12);
    }

    #[test]
    fn euv_presets_carry_the_core_obscuration() {
        let mut p = OpticsParams::default();
        p.apply_preset(OpticsPreset::HighNa055);
        assert_eq!((p.na, p.central_obscuration), (0.55, 0.2));
        assert!(build_optics(&p, 13.5).is_ok());
        p.apply_preset(OpticsPreset::Nxe033);
        assert_eq!((p.na, p.central_obscuration), (0.33, 0.0));
        p.central_obscuration = 1.2;
        assert!(build_optics(&p, 13.5).is_err());
    }

    #[test]
    fn settings_map_the_panel_state() {
        let mut p = ImagingParams::default();
        let s = build_settings(&p);
        assert_eq!(s.defocus_model, DefocusModel::Exact);
        assert_eq!(s.imaging_model, ImagingModel::Scalar);
        assert_eq!(s.normalization, ImageNormalization::ClearField);
        p.vector = true;
        p.polarization = PolarizationChoice::Tm;
        p.obliquity = false;
        p.defocus_model = DefocusModel::KernelPhase;
        p.normalization = ImageNormalization::Absolute;
        let s = build_settings(&p);
        match &s.imaging_model {
            ImagingModel::Vector(v) => {
                assert_eq!(v.polarization, IlluminationPolarization::Tm);
                assert!(!v.obliquity);
                // Left at 1 so the engine inherits the optics' medium.
                assert_eq!(v.image_index, 1.0);
            }
            other => panic!("expected vector imaging, got {other:?}"),
        }
        assert_eq!(s.defocus_model, DefocusModel::KernelPhase);
        assert_eq!(s.normalization, ImageNormalization::Absolute);
    }

    #[test]
    fn sigma_extent_of_each_fill() {
        assert_eq!(
            sigma_extent(&IlluminationShape::Conventional { sigma: 0.7 }),
            0.7
        );
        assert_eq!(
            sigma_extent(&IlluminationShape::Annular {
                sigma_inner: 0.5,
                sigma_outer: 0.9
            }),
            0.9
        );
        let d = sigma_extent(&IlluminationShape::Dipole {
            sigma_center: 0.6,
            sigma_radius: 0.2,
            orientation_deg: 0.0,
        });
        assert!((d - 0.8).abs() < 1e-12);
        // Gaussian cut at 1e-4 of the peak: σ·√(2 ln 1e4) = 4.2919·σ.
        let g = sigma_extent(&IlluminationShape::CoherentGaussian { sigma: 0.1 });
        assert!((g - 0.429_193_0).abs() < 1e-6, "{g}");
        assert_eq!(
            sigma_extent(&IlluminationShape::CoherentGaussian { sigma: 0.5 }),
            1.0
        );
    }

    #[test]
    fn auto_grid_is_commensurate_and_nyquist_safe() {
        let mask = build_mask(&MaskParams::default()).unwrap(); // 65 / 180 nm
        let nyq = nyquist_pixel_nm(157.63, 0.75, 0.7);
        assert!((nyq - 61.8157).abs() < 1e-3);
        let g = choose_grid(&mask, &GridParams::default(), nyq).unwrap();
        // Two pitches in a 128-pixel field.
        assert_eq!(g.periods, 2.0);
        assert!((g.field_nm - 360.0).abs() < 1e-9);
        assert!((g.pixel_nm - 360.0 / 128.0).abs() < 1e-12);
        assert!(g.resolves_band);
        mask.check_commensurate(&g.grid()).unwrap();

        // EUV: a 64-pixel grid cannot hold a 500 nm pitch below the
        // 6.46 nm Nyquist pixel of NA 0.55 / σ 0.9 — flagged.
        let coarse = build_mask(&MaskParams {
            cd_nm: 200.0,
            pitch_nm: 500.0,
            ..MaskParams::default()
        })
        .unwrap();
        let nyq_euv = nyquist_pixel_nm(13.5, 0.55, 0.9);
        let params = GridParams {
            size: 64,
            ..GridParams::default()
        };
        let g = choose_grid(&coarse, &params, nyq_euv).unwrap();
        assert_eq!(g.periods, 1.0);
        assert!((g.pixel_nm - 500.0 / 64.0).abs() < 1e-12);
        assert!(!g.resolves_band);
        // 128 pixels fit one pitch below the limit.
        let params = GridParams {
            size: 128,
            ..GridParams::default()
        };
        let g = choose_grid(&coarse, &params, nyq_euv).unwrap();
        assert!(g.resolves_band);

        // Manual pixel: never coarsened while one pitch fits (2 nm → 256 nm
        // field → 1 pitch at 180/128 nm) …
        let mut manual = GridParams {
            auto_pixel: false,
            pixel_nm: 2.0,
            ..GridParams::default()
        };
        let g = choose_grid(&mask, &manual, nyq).unwrap();
        assert!((g.pixel_nm - 180.0 / 128.0).abs() < 1e-12);
        assert_eq!(g.periods, 1.0);
        mask.check_commensurate(&g.grid()).unwrap();
        // … but coarsened to pitch/size when the field would be smaller
        // than one pitch (1 nm × 128 = 128 nm < 180 nm).
        manual.pixel_nm = 1.0;
        let g = choose_grid(&mask, &manual, nyq).unwrap();
        assert!((g.pixel_nm - 180.0 / 128.0).abs() < 1e-12);
        // Non-power-of-two sizes are rejected.
        let bad = GridParams {
            size: 100,
            ..GridParams::default()
        };
        assert!(choose_grid(&mask, &bad, nyq).is_err());
    }

    #[test]
    fn mask_panel_builds_tones_and_psm() {
        let lines = build_mask(&MaskParams::default()).unwrap();
        assert!(!lines.dark_field);
        let contacts = build_mask(&MaskParams {
            pattern: Pattern::ContactArray,
            cd_nm: 60.0,
            pitch_nm: 200.0,
            ..MaskParams::default()
        })
        .unwrap();
        assert!(contacts.dark_field);
        let reversed = build_mask(&MaskParams {
            reverse_tone: true,
            att_psm: true,
            ..MaskParams::default()
        })
        .unwrap();
        assert!(reversed.dark_field);
        assert!(matches!(
            reversed.mask_type,
            MaskType::AttenuatedPSM { transmission, .. } if transmission == 0.06
        ));
        // CD must be below the pitch.
        assert!(build_mask(&MaskParams {
            cd_nm: 200.0,
            pitch_nm: 180.0,
            ..MaskParams::default()
        })
        .is_err());
    }

    #[test]
    fn multilayer_pupil_is_opt_in_and_attaches_to_reflective_optics() {
        let mut p = OpticsParams::default();
        p.apply_preset(OpticsPreset::Nxe033);
        assert!(p.multilayer.build().unwrap().is_none());
        assert!(!build_optics(&p, 13.5).unwrap().label.contains("multilayer"));
        p.multilayer.enabled = true;
        let pupil = p.multilayer.build().unwrap().unwrap();
        assert_eq!(pupil.mirrors.len(), 2);
        assert_eq!(pupil.mirrors[0].radial_deg, 15.0);
        assert!(build_optics(&p, 13.5).unwrap().label.contains("multilayer"));
        // Ideal Mo/Si 40 × 6.9 nm at normal incidence: 72.9 % at 13.5 nm
        // (E1's CXRO-checked scan: 0.729), far off-peak at 6.7 nm.
        let r = p.multilayer.center_reflectance(13.5).unwrap();
        assert!((r - 0.729).abs() < 2e-3, "{r}");
        assert!(p.multilayer.center_reflectance(6.7).unwrap() < 0.05);
        // La/B4C 200 × 3.37 nm tuned to 6.7 nm: 68.9 % (ideal).
        p.multilayer.set_coating(Coating::LaB4C);
        let r = p.multilayer.center_reflectance(6.7).unwrap();
        assert!((r - 0.689).abs() < 5e-3, "{r}");
        p.multilayer.mirrors = 0;
        assert!(build_optics(&p, 6.7).is_err());
    }
}
