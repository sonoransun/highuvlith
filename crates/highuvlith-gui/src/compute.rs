//! The computations behind the views. Every function here is pure (inputs →
//! result or error message) and runs on a worker thread through
//! [`crate::jobs::Job`]; nothing here touches the UI.
//!
//! All projection imaging goes through the same `AerialImageEngine` as the
//! CLI and Python frontends. Engines are cached by everything that defines
//! them (source, optics, imaging settings, grid) so moving the focus or the
//! CD reuses the engine and only builds kernels for new focus planes.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use highuvlith_core::aerial::{
    AerialImageEngine, ImageNormalization, ImagingSettings, SourcePoint,
};
use highuvlith_core::deep_xray::{self, BeamFilter, DeepXrayConfig, ProximityModel, XraySpectrum};
use highuvlith_core::mask::Mask;
use highuvlith_core::materials::attenuation::Compound;
use highuvlith_core::materials::database::MaterialsDatabase;
use highuvlith_core::metrics::{self, FeatureTone};
use highuvlith_core::process::{ProcessRectangle, ProcessWindow};
use highuvlith_core::resist::{CarParams, PebDiffusion, ResistParams};
use highuvlith_core::source::{LithographySource, SourceKind};
use highuvlith_core::talbot::{Grating, TalbotSetup};
use highuvlith_core::volumetric::{
    self, DevelopmentOptions, LevelSetConfig, PebModel, VolumetricExposureConfig,
};
use ndarray::Array2;

use crate::imaging::{
    build_mask, build_optics, build_settings, choose_grid, nyquist_pixel_nm, sigma_extent,
    BuiltOptics, GridChoice, GridParams, ImagingParams, MaskParams, OpticsKind, OpticsParams,
    Pattern, SpectrumMode,
};
use crate::sources::{describe_illumination, Family, Route, SourceParams};
use crate::volume::{DatasetKind, VolumeData};

// ---------------------------------------------------------------------------
// Scene → engine
// ---------------------------------------------------------------------------

/// Everything that defines a projection-imaging run except focus and dose.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub source: SourceParams,
    pub optics: OpticsParams,
    pub imaging: ImagingParams,
    pub mask: MaskParams,
    pub grid: GridParams,
}

/// A scene resolved into core objects (cheap: no TCC yet).
pub struct Resolved {
    pub source: SourceKind,
    pub optics: BuiltOptics,
    pub mask: Mask,
    pub grid: GridChoice,
    pub settings: ImagingSettings,
    /// Radius of the pupil fill's support (σ units).
    pub sigma_extent: f64,
}

/// Resolve a scene. LIGA-route sources are rejected with a pointer to the
/// LIGA view.
pub fn resolve(scene: &Scene) -> Result<Resolved, String> {
    if scene.source.route() == Route::Liga {
        return Err(format!(
            "{} is a broadband LIGA / proximity source, not a projection-imaging source: \
             see the LIGA view",
            scene.source.family.label()
        ));
    }
    let source = scene.source.build(scene.imaging.spectral_samples)?;
    let lambda = source.wavelength_nm();
    let optics = build_optics(&scene.optics, lambda)?;
    let mask = build_mask(&scene.mask)?;
    // Per-wavelength imaging needs the band of the shortest sample.
    let lambda_min = if scene.imaging.spectrum == SpectrumMode::PerWavelength {
        source
            .spectral_weights()
            .iter()
            .map(|(l, _)| *l)
            .fold(lambda, f64::min)
    } else {
        lambda
    };
    let s_ext = sigma_extent(source.illumination());
    let cutoff = optics.optics.cutoff_frequency(lambda_min);
    let nyquist = nyquist_pixel_nm(lambda_min, cutoff * lambda_min, s_ext);
    let grid = choose_grid(&mask, &scene.grid, nyquist)?;
    let settings = build_settings(&scene.imaging);
    Ok(Resolved {
        source,
        optics,
        mask,
        grid,
        settings,
        sigma_extent: s_ext,
    })
}

/// Engine-defining part of a scene.
#[derive(Clone, Debug, PartialEq)]
struct EngineKey {
    source: SourceParams,
    optics: OpticsParams,
    imaging: ImagingParams,
    size: usize,
    pixel_bits: u64,
}

impl EngineKey {
    fn new(scene: &Scene, grid: &GridChoice) -> Self {
        let mut imaging = scene.imaging.clone();
        // The spectrum mode picks the compute call, not the engine.
        imaging.spectrum = SpectrumMode::Monochromatic;
        Self {
            source: scene.source.clone(),
            optics: scene.optics.clone(),
            imaging,
            size: grid.size,
            pixel_bits: grid.pixel_nm.to_bits(),
        }
    }
}

/// Small most-recently-used cache of engines shared by all jobs.
#[derive(Default)]
pub struct EngineCache {
    entries: Mutex<Vec<(EngineKey, Arc<AerialImageEngine>)>>,
}

const ENGINE_CACHE_CAPACITY: usize = 3;

impl EngineCache {
    /// The engine for `scene`/`resolved`, building it (outside the lock) on a
    /// miss. Returns the engine and whether it was reused.
    pub fn engine(
        &self,
        scene: &Scene,
        resolved: &Resolved,
    ) -> Result<(Arc<AerialImageEngine>, bool), String> {
        let key = EngineKey::new(scene, &resolved.grid);
        {
            let mut entries = self.entries.lock().expect("engine cache poisoned");
            if let Some(pos) = entries.iter().position(|(k, _)| *k == key) {
                let entry = entries.remove(pos);
                let engine = Arc::clone(&entry.1);
                entries.insert(0, entry);
                return Ok((engine, true));
            }
        }
        let engine = Arc::new(
            AerialImageEngine::with_settings(
                &resolved.source,
                resolved.optics.optics.as_ref(),
                resolved.grid.grid(),
                resolved.settings.clone(),
            )
            .map_err(|e| e.to_string())?,
        );
        let mut entries = self.entries.lock().expect("engine cache poisoned");
        entries.insert(0, (key, Arc::clone(&engine)));
        entries.truncate(ENGINE_CACHE_CAPACITY);
        Ok((engine, false))
    }

    /// Number of cached engines.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.lock().expect("engine cache poisoned").len()
    }
}

/// Image of `mask` at `focus_nm` with the scene's spectrum mode.
fn image_with_spectrum(
    engine: &AerialImageEngine,
    resolved: &Resolved,
    mode: SpectrumMode,
    focus_nm: f64,
) -> Result<highuvlith_core::types::Grid2D<f64>, String> {
    Ok(match mode {
        SpectrumMode::Monochromatic => engine.compute(&resolved.mask, focus_nm),
        SpectrumMode::NarrowBand => engine.compute_polychromatic(
            &resolved.mask,
            focus_nm,
            &resolved.source,
            resolved.optics.optics.as_ref(),
        ),
        SpectrumMode::PerWavelength => engine
            .compute_multiwavelength(&resolved.mask, focus_nm)
            .map_err(|e| e.to_string())?,
    })
}

// ---------------------------------------------------------------------------
// Aerial image
// ---------------------------------------------------------------------------

/// Aerial-image request.
#[derive(Clone, Debug, PartialEq)]
pub struct AerialRequest {
    pub scene: Scene,
    pub focus_nm: f64,
    /// Intensity threshold for the printed CD (relative to the clear field).
    pub threshold: f64,
}

/// Aerial image plus everything the views report about it.
pub struct AerialResult {
    /// Image `[y][x]`, relative intensity (absolute in absolute mode).
    pub image: Array2<f64>,
    /// Field extents in nm (centred on 0).
    pub half_field_nm: f64,
    pub contrast: f64,
    pub i_min: f64,
    pub i_max: f64,
    /// y = 0 cross-section and its x coordinates (nm).
    pub profile_x: Vec<f64>,
    pub profile: Vec<f64>,
    pub tone: FeatureTone,
    pub cd_nm: Option<f64>,
    pub nils: Option<f64>,
    pub num_kernels: usize,
    pub captured_energy: f64,
    pub clear_field: f64,
    pub normalized: bool,
    pub grid: GridChoice,
    pub wavelength_nm: f64,
    pub na: f64,
    pub optics_label: String,
    pub illumination: String,
    pub sigma_extent: f64,
    pub spectral_samples: Vec<(f64, f64)>,
    pub source_points: Vec<SourcePoint>,
    pub central_obscuration: Option<f64>,
    /// Half-pitch k₁ = (pitch/2)·NA/λ.
    pub k1_half_pitch: f64,
    /// Smallest pitch whose first orders reach the pupil: λ/(NA(1+σ)).
    pub cutoff_pitch_nm: f64,
    pub warnings: Vec<String>,
    pub engine_reused: bool,
    pub engine_ms: f64,
    pub image_ms: f64,
}

/// Warnings about the model that apply to any result of `resolved`.
fn scene_warnings(
    scene: &Scene,
    resolved: &Resolved,
    engine: &AerialImageEngine,
    focus_nm: f64,
) -> Result<Vec<String>, String> {
    let mut w = resolved.optics.warnings.clone();
    let diag = engine
        .kernel_diagnostics(focus_nm, engine.wavelength_nm())
        .map_err(|e| e.to_string())?;
    if scene.imaging.normalization == ImageNormalization::ClearField && !diag.normalized {
        w.push(format!(
            "dark-field imaging: the pupil blocks the zero order for almost the whole source \
             (clear field {:.2e}); intensities are absolute, not relative to the clear field",
            diag.clear_field_intensity
        ));
    }
    if diag.support_exceeds_nyquist {
        w.push(format!(
            "pixel {:.3} nm is too coarse for the band (1+\u{3c3})NA/\u{3bb}: part of the image \
             spectrum is lost (Nyquist-safe pixel {:.3} nm) \u{2014} increase the grid size",
            resolved.grid.pixel_nm, resolved.grid.nyquist_pixel_nm
        ));
    }
    let src = &scene.source;
    if src.family == Family::Hhg
        && src.hhg.full_comb
        && scene.imaging.spectrum != SpectrumMode::PerWavelength
    {
        w.push(
            "full HHG comb: only the per-wavelength spectrum mode images every harmonic with its \
             own TCC; this mode treats the comb as centre-wavelength bookkeeping"
                .into(),
        );
    }
    if scene.imaging.spectrum == SpectrumMode::NarrowBand {
        let rel = resolved.source.bandwidth_pm() * 1e-3 / resolved.source.wavelength_nm();
        if rel > 0.01 {
            w.push(format!(
                "narrow-band mode assumes \u{394}\u{3bb}/\u{3bb} << 1 (here {:.1} %); use \
                 per-wavelength imaging",
                rel * 100.0
            ));
        }
    }
    if scene.imaging.spectrum != SpectrumMode::Monochromatic
        && resolved.optics.kind == OpticsKind::RefractiveDry
    {
        w.push(
            "the dry refractive lens carries the core's 157 nm CaF\u{2082} axial-chromatic \
             coefficient (15 nm/pm) whatever the wavelength"
                .into(),
        );
    }
    if src.family == Family::Entangled {
        w.push(
            "entangled-photon source imaged classically at its physical wavelength (the \u{3bb}/N \
             NOON sharpening is a separate research module, not applied here)"
                .into(),
        );
    }
    Ok(w)
}

/// Compute an aerial image.
pub fn run_aerial(cache: &EngineCache, req: &AerialRequest) -> Result<AerialResult, String> {
    let resolved = resolve(&req.scene)?;
    let t0 = Instant::now();
    let (engine, reused) = cache.engine(&req.scene, &resolved)?;
    let engine_ms = t0.elapsed().as_secs_f64() * 1e3;
    let t1 = Instant::now();
    let image = image_with_spectrum(&engine, &resolved, req.scene.imaging.spectrum, req.focus_nm)?;
    let image_ms = t1.elapsed().as_secs_f64() * 1e3;
    let mut warnings = scene_warnings(&req.scene, &resolved, &engine, req.focus_nm)?;

    let data = image.data;
    let contrast = metrics::image_contrast(&data);
    let i_max = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let i_min = data.iter().copied().fold(f64::INFINITY, f64::min);
    let profile = metrics::centre_profile(&data);
    let profile_x = metrics::pixel_centres(profile.len(), image.x_min_nm, image.x_max_nm);
    let tone = FeatureTone::of_mask(&resolved.mask);
    let cd_nm = metrics::measure_cd_periodic(&profile, &profile_x, req.threshold, tone);
    let nils = metrics::nils_periodic(&profile, &profile_x, req.threshold, tone, None);

    let lambda = engine.wavelength_nm();
    let na_eff = resolved.optics.optics.cutoff_frequency(lambda) * lambda;
    let pitch = req.scene.mask.pitch_nm;
    let cutoff_pitch_nm = lambda / (na_eff * (1.0 + resolved.sigma_extent));
    if req.scene.mask.pattern == Pattern::LineSpace && pitch < cutoff_pitch_nm && contrast < 1e-6 {
        warnings.push(format!(
            "pitch {pitch:.1} nm is below \u{3bb}/(NA(1+\u{3c3})) = {cutoff_pitch_nm:.1} nm: no \
             first diffraction order reaches the pupil, so the image is flat"
        ));
    }
    let central_obscuration = match resolved.optics.kind {
        OpticsKind::EuvProjection | OpticsKind::Schwarzschild => {
            Some(req.scene.optics.central_obscuration)
        }
        _ => None,
    };
    Ok(AerialResult {
        half_field_nm: image.x_max_nm,
        contrast,
        i_min,
        i_max,
        profile_x,
        profile,
        tone,
        cd_nm,
        nils,
        num_kernels: engine.num_kernels(),
        captured_energy: engine.captured_energy_fraction(),
        clear_field: engine.clear_field_intensity(req.focus_nm),
        normalized: engine
            .kernel_diagnostics(req.focus_nm, lambda)
            .map(|d| d.normalized)
            .unwrap_or(true),
        grid: resolved.grid.clone(),
        wavelength_nm: lambda,
        na: engine.na(),
        optics_label: resolved.optics.label.clone(),
        illumination: describe_illumination(resolved.source.illumination()),
        sigma_extent: resolved.sigma_extent,
        spectral_samples: engine.spectral_samples().to_vec(),
        source_points: engine.source_points().to_vec(),
        central_obscuration,
        k1_half_pitch: 0.5 * pitch * na_eff / lambda,
        cutoff_pitch_nm,
        warnings,
        engine_reused: reused,
        engine_ms,
        image_ms,
        image: data,
    })
}

// ---------------------------------------------------------------------------
// Process window
// ---------------------------------------------------------------------------

/// Process-window sweep settings.
#[derive(Clone, Debug, PartialEq)]
pub struct PwParams {
    /// Nominal dose (mJ/cm²); the threshold is the printed-edge intensity there.
    pub dose_nominal: f64,
    /// Dose sweep ±% around nominal.
    pub dose_span_pct: f64,
    pub n_doses: usize,
    /// Focus sweep ± (nm), unless automatic.
    pub focus_span_nm: f64,
    /// Sweep ±2 Rayleigh depths of focus n·λ/(2NA²).
    pub focus_auto: bool,
    pub n_focus: usize,
    /// Intensity threshold at the nominal dose.
    pub threshold: f64,
    /// Target CD (`None` = the mask CD).
    pub target_cd_nm: Option<f64>,
    /// ±% CD tolerance.
    pub tolerance_pct: f64,
}

impl Default for PwParams {
    fn default() -> Self {
        Self {
            dose_nominal: 30.0,
            dose_span_pct: 20.0,
            n_doses: 9,
            focus_span_nm: 150.0,
            focus_auto: true,
            n_focus: 13,
            threshold: 0.3,
            target_cd_nm: None,
            tolerance_pct: 10.0,
        }
    }
}

/// Process-window request.
#[derive(Clone, Debug, PartialEq)]
pub struct PwRequest {
    pub scene: Scene,
    pub pw: PwParams,
}

/// Process-window result.
pub struct PwResult {
    pub window: ProcessWindow,
    pub best_focus_nm: f64,
    pub dof_nm: f64,
    pub el_pct: f64,
    pub dose_to_size: Option<f64>,
    pub max_area: Option<ProcessRectangle>,
    /// DOF at 5 % exposure latitude.
    pub dof_at_5pct: Option<ProcessRectangle>,
    pub el_vs_dof: Vec<(f64, f64)>,
    pub rayleigh_dof_nm: f64,
    pub warnings: Vec<String>,
}

/// Linearly spaced samples `centre ± span`.
pub fn sweep(centre: f64, span: f64, n: usize) -> Vec<f64> {
    let n = n.max(1);
    if n == 1 {
        return vec![centre];
    }
    (0..n)
        .map(|i| centre - span + 2.0 * span * i as f64 / (n - 1) as f64)
        .collect()
}

/// Rayleigh depth of focus `n·λ/(2·NA²)` (nm).
pub fn rayleigh_dof_nm(wavelength_nm: f64, na: f64, medium_index: f64) -> f64 {
    medium_index * wavelength_nm / (2.0 * na * na)
}

/// Sweep dose and focus and analyse the window (constant-threshold resist,
/// one aerial image per focus plane).
pub fn run_process_window(cache: &EngineCache, req: &PwRequest) -> Result<PwResult, String> {
    let resolved = resolve(&req.scene)?;
    let (engine, _) = cache.engine(&req.scene, &resolved)?;
    let lambda = engine.wavelength_nm();
    let rdof = rayleigh_dof_nm(
        lambda,
        engine.na(),
        resolved.optics.optics.immersion_index(),
    );
    let p = &req.pw;
    let focus_span = if p.focus_auto {
        2.0 * rdof
    } else {
        p.focus_span_nm
    };
    let doses = sweep(
        p.dose_nominal,
        p.dose_nominal * p.dose_span_pct / 100.0,
        p.n_doses,
    );
    let focuses = sweep(0.0, focus_span, p.n_focus);
    let target = p.target_cd_nm.unwrap_or(req.scene.mask.cd_nm);
    let mut warnings = Vec::new();
    if req.scene.imaging.spectrum != SpectrumMode::Monochromatic {
        warnings.push(
            "the process window uses the centre-wavelength image at every focus (spectrum mode \
             applies to the aerial-image view only)"
                .into(),
        );
    }
    let window = ProcessWindow::compute(
        &engine,
        &resolved.mask,
        &doses,
        &focuses,
        p.threshold,
        target,
        p.tolerance_pct,
    )
    .map_err(|e| e.to_string())?;
    if window.cd_matrix.iter().all(|c| !c.is_finite()) {
        warnings.push(format!(
            "the feature prints at no swept (dose, focus): check the threshold {:.2} against the \
             image range",
            p.threshold
        ));
    }
    Ok(PwResult {
        best_focus_nm: window.best_focus(),
        dof_nm: window.depth_of_focus(),
        el_pct: window.exposure_latitude(),
        dose_to_size: window.dose_to_size(),
        max_area: window.max_area_rectangle(),
        dof_at_5pct: window.dof_at_el(5.0),
        el_vs_dof: window.el_vs_dof(41),
        rayleigh_dof_nm: rdof,
        warnings,
        window,
    })
}

// ---------------------------------------------------------------------------
// Volumetric resist exposure and development
// ---------------------------------------------------------------------------

/// Post-exposure bake model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PebChoice {
    #[default]
    None,
    /// Exact anisotropic Gaussian diffusion.
    Gaussian,
    /// Chemically amplified acid/quencher reaction–diffusion (default constants).
    Car,
}

/// Development model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DevelopChoice {
    /// Latent image only.
    None,
    /// Fast-marching arrival times (static rate).
    #[default]
    FastMarching,
    /// Level-set moving boundary.
    LevelSet,
}

/// Volumetric run settings.
#[derive(Clone, Debug, PartialEq)]
pub struct VolumetricParams {
    pub resist_thickness_nm: f64,
    pub dose_mj_cm2: f64,
    pub nz: usize,
    pub n_planes: usize,
    pub peb: PebChoice,
    pub peb_lateral_nm: f64,
    pub peb_vertical_nm: f64,
    pub develop: DevelopChoice,
    pub dev_time_s: f64,
}

/// Defaults chosen so the default scene (F2, 65/180 nm lines) develops into
/// a recognisable line/space profile with the core's illustrative default
/// resist (`ResistParams::default()`, Mack n = 3):
/// - 150 nm resist = the core default resist / film-stack thickness;
/// - a Gaussian bake with σz = 20 nm: on the reflective Si substrate the
///   standing-wave period is λ/(2 n_r) ≈ 48 nm and the bake damps its
///   modulation by exp(−2π²σz²/P²) ≈ 0.03 (σz = 5 nm: 0.81). Without a bake
///   the dissolution front stalls at the first standing-wave node
///   (|E|² ≈ 0.12 of the incident there), which is real no-bake physics
///   but made the default look undeveloped;
/// - 15 mJ/cm² and 30 s: the low-contrast default resist on a contrast-0.46
///   image has a narrow window (at 15 mJ/cm² spaces reach the substrate after
///   ≈ 25 s and the lines are gone by ≈ 60 s).
impl Default for VolumetricParams {
    fn default() -> Self {
        Self {
            resist_thickness_nm: 150.0,
            dose_mj_cm2: 15.0,
            nz: 32,
            n_planes: 8,
            peb: PebChoice::Gaussian,
            peb_lateral_nm: 10.0,
            peb_vertical_nm: 20.0,
            develop: DevelopChoice::FastMarching,
            dev_time_s: 30.0,
        }
    }
}

/// Volumetric request.
#[derive(Clone, Debug, PartialEq)]
pub struct VolumetricRequest {
    pub scene: Scene,
    pub focus_nm: f64,
    pub vol: VolumetricParams,
}

/// Volumetric result.
pub struct VolumetricResult {
    pub latent: Arc<VolumeData>,
    pub developed: Option<Arc<VolumeData>>,
    /// `(label, value)` rows for the summary table.
    pub summary: Vec<(String, String)>,
    pub notes: Vec<String>,
}

fn fmt_cd(cd: Option<f64>) -> String {
    cd.map_or_else(|| "n/a".to_string(), |v| format!("{v:.1} nm"))
}

/// Expose the resist volumetrically, bake, and develop (same model and
/// default film stack / resist as `highuvlith deep` volumetric mode).
pub fn run_volumetric(
    cache: &EngineCache,
    req: &VolumetricRequest,
) -> Result<VolumetricResult, String> {
    let resolved = resolve(&req.scene)?;
    let (engine, _) = cache.engine(&req.scene, &resolved)?;
    let v = &req.vol;
    if !(v.resist_thickness_nm > 0.0 && v.dose_mj_cm2 > 0.0) {
        return Err("resist thickness and dose must be positive".into());
    }
    // Resist-on-Si stack at the source wavelength from the materials
    // database (errors where it has no resist data, e.g. 193 nm).
    let stack = MaterialsDatabase::new()
        .resist_on_silicon_stack(v.resist_thickness_nm, resolved.source.wavelength_nm())
        .map_err(|e| e.to_string())?;
    let resist = ResistParams {
        thickness_nm: v.resist_thickness_nm,
        ..ResistParams::default()
    };
    let config = VolumetricExposureConfig {
        dose_mj_cm2: v.dose_mj_cm2,
        nz: v.nz.max(2),
        n_defocus_planes: v.n_planes.max(1),
        dose_steps: 1,
        base_defocus_nm: req.focus_nm,
        resist_layer: 0,
    };
    let wavelength = resolved.source.wavelength_nm();
    let mut latent = volumetric::expose_volumetric(
        &engine,
        &resolved.mask,
        &stack,
        &resist,
        wavelength,
        &config,
    )
    .map_err(|e| e.to_string())?;
    let peb = match v.peb {
        PebChoice::None => PebModel::None,
        PebChoice::Gaussian => PebModel::Gaussian(PebDiffusion::anisotropic(
            v.peb_lateral_nm,
            v.peb_vertical_nm,
        )),
        PebChoice::Car => PebModel::ChemicallyAmplified(CarParams::default()),
    };
    volumetric::apply_peb(&mut latent, &peb).map_err(|e| e.to_string())?;

    let z_label = "depth below the resist top";
    let mut summary = vec![
        ("Wavelength".to_string(), format!("{wavelength:.3} nm")),
        (
            "Resist / dose".to_string(),
            format!(
                "{:.0} nm, {:.1} mJ/cm\u{b2}",
                v.resist_thickness_nm, v.dose_mj_cm2
            ),
        ),
        (
            "Grid".to_string(),
            format!(
                "{0}\u{d7}{0}\u{d7}{1} (pixel {2:.2} nm, \u{394}z {3:.2} nm)",
                resolved.grid.size,
                config.nz,
                resolved.grid.pixel_nm,
                v.resist_thickness_nm / config.nz as f64
            ),
        ),
    ];
    let cds = metrics::cd_at_z(&latent.pac, 0.5);
    summary.push((
        "Latent CD top / mid / bottom (m = 0.5)".to_string(),
        format!(
            "{} / {} / {}",
            fmt_cd(cds.first().copied().flatten()),
            fmt_cd(cds.get(cds.len() / 2).copied().flatten()),
            fmt_cd(cds.last().copied().flatten())
        ),
    ));
    let latent_data = VolumeData::from_grid3d(
        DatasetKind::Latent,
        &latent.pac,
        "PAC m (1 = unexposed)",
        "",
        z_label,
    );

    let options = DevelopmentOptions::default();
    let dz = latent.pac.pixel_size_z();
    let dx = latent.pac.pixel_size_x();
    let times = match v.develop {
        DevelopChoice::None => None,
        DevelopChoice::FastMarching => Some(
            volumetric::develop_fast_marching_with(&latent, &resist, dx, dz, &options)
                .map_err(|e| e.to_string())?,
        ),
        DevelopChoice::LevelSet => {
            let result = volumetric::develop_level_set(
                &latent,
                &resist,
                &options,
                &LevelSetConfig::new(v.dev_time_s),
            )
            .map_err(|e| e.to_string())?;
            summary.push((
                "Level set".to_string(),
                format!(
                    "{} steps, dissolved {:.1} nm",
                    result.steps, result.dissolved_thickness_nm
                ),
            ));
            Some(result.arrival_times)
        }
    };
    let developed = times.map(|t| {
        let dev = volumetric::developed_indicator(&t, v.dev_time_s);
        let dcds = metrics::cd_at_z(&dev, 0.5);
        let (top, bottom) = (
            dcds.first().copied().flatten(),
            dcds.last().copied().flatten(),
        );
        summary.push((
            format!("Developed CD top / mid / bottom ({:.0} s)", v.dev_time_s),
            format!(
                "{} / {} / {}",
                fmt_cd(top),
                fmt_cd(dcds.get(dcds.len() / 2).copied().flatten()),
                fmt_cd(bottom)
            ),
        ));
        if let (Some(a), Some(b)) = (top, bottom) {
            summary.push((
                "Developed sidewall".to_string(),
                format!(
                    "{:.1}\u{b0}",
                    metrics::sidewall_angle_deg(a, b, v.resist_thickness_nm)
                ),
            ));
        }
        VolumeData::from_grid3d(
            DatasetKind::Developed,
            &dev,
            "developed (1 = dissolved)",
            "",
            z_label,
        )
    });
    let notes = vec![
        "Separable model I(x,y; z) = I_aer(x,y; d\u{2080} + z/n)\u{b7}S(z) with the exact \
         thin-film standing wave S(z) (status: Simplified, see docs/processes/volumetric-exposure.md)."
            .to_string(),
        "Film stack and resist are the core defaults used by `highuvlith deep` (fluoropolymer \
         resist n = 1.65 + 0.015i on Si, 157 nm optical constants), whatever the wavelength."
            .to_string(),
    ];
    Ok(VolumetricResult {
        latent: Arc::new(latent_data),
        developed: developed.map(Arc::new),
        summary,
        notes,
    })
}

// ---------------------------------------------------------------------------
// LIGA (deep X-ray shadow printing)
// ---------------------------------------------------------------------------

/// Upstream beam filter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LigaFilter {
    #[default]
    None,
    Aluminum,
    Beryllium,
    Kapton,
}

impl LigaFilter {
    /// All filters, in menu order.
    pub const ALL: [LigaFilter; 4] = [
        LigaFilter::None,
        LigaFilter::Aluminum,
        LigaFilter::Beryllium,
        LigaFilter::Kapton,
    ];

    /// Menu label.
    pub fn label(self) -> &'static str {
        match self {
            LigaFilter::None => "none",
            LigaFilter::Aluminum => "Al",
            LigaFilter::Beryllium => "Be",
            LigaFilter::Kapton => "Kapton",
        }
    }

    fn spec(self) -> Option<&'static str> {
        match self {
            LigaFilter::None => None,
            LigaFilter::Aluminum => Some("Al"),
            LigaFilter::Beryllium => Some("Be"),
            LigaFilter::Kapton => Some("Kapton"),
        }
    }
}

/// LIGA exposure settings.
#[derive(Clone, Debug, PartialEq)]
pub struct LigaParams {
    pub resist_thickness_um: f64,
    pub gap_um: f64,
    /// Au absorber thickness.
    pub absorber_um: f64,
    /// Ti membrane thickness.
    pub membrane_um: f64,
    pub filter: LigaFilter,
    pub filter_um: f64,
    pub target_bottom_dose: f64,
    pub damage_dose: f64,
    /// Fresnel angular-spectrum propagation (else the legacy Gaussian blur).
    pub fresnel: bool,
    pub energy_bins: usize,
    /// Absolute flux → exposure time (tube/betatron: point source at
    /// `distance_mm`; bending magnet: beamline geometry).
    pub absolute: bool,
    pub distance_mm: f64,
    pub bm_distance_m: f64,
    pub bm_acceptance_mrad: f64,
    pub bm_scan_mm: f64,
    /// Mask bars (µm).
    pub cd_um: f64,
    pub pitch_um: f64,
    pub grid_size: usize,
    pub nz: usize,
    /// Developer threshold (kJ/cm³) for the developed-depth map.
    pub develop_threshold: f64,
}

impl Default for LigaParams {
    fn default() -> Self {
        Self {
            resist_thickness_um: 500.0,
            gap_um: 100.0,
            absorber_um: 20.0,
            membrane_um: 2.0,
            filter: LigaFilter::Aluminum,
            filter_um: 20.0,
            target_bottom_dose: 3.0,
            damage_dose: 20.0,
            fresnel: true,
            energy_bins: 60,
            absolute: true,
            distance_mm: 100.0,
            bm_distance_m: 15.0,
            bm_acceptance_mrad: 5.0,
            bm_scan_mm: 50.0,
            cd_um: 20.0,
            pitch_um: 40.0,
            grid_size: 64,
            nz: 32,
            develop_threshold: 2.5,
        }
    }
}

/// LIGA request.
#[derive(Clone, Debug, PartialEq)]
pub struct LigaRequest {
    pub source: SourceParams,
    pub liga: LigaParams,
}

/// LIGA result.
pub struct LigaResult {
    /// Depth (µm) and absorbed dose (kJ/cm³) in open areas.
    pub z_um: Vec<f64>,
    pub dose_kj_cm3: Vec<f64>,
    pub target_bottom_dose: f64,
    pub damage_dose: f64,
    pub volume: Arc<VolumeData>,
    pub summary: Vec<(String, String)>,
    pub warnings: Vec<String>,
}

/// Number of energy bins of the tabulated tube / betatron spectra.
const LIGA_SOURCE_BINS: usize = 200;

/// The LIGA spectrum of a LIGA-route source and a description of it.
fn liga_spectrum(source: &SourceParams, p: &LigaParams) -> Result<(XraySpectrum, String), String> {
    let kind = source.build(None)?;
    match kind {
        SourceKind::XrayTube(tube) => {
            if p.absolute {
                let table = tube.spectral_flux_density(p.distance_mm, LIGA_SOURCE_BINS);
                Ok((
                    XraySpectrum::from_flux_density(&table).map_err(|e| e.to_string())?,
                    format!(
                        "{} tube {:.0} kV / {:.0} mA, absolute flux at {:.0} mm (isotropic point \
                         source)",
                        tube.anode.symbol(),
                        tube.kvp,
                        tube.current_ma,
                        p.distance_mm
                    ),
                ))
            } else {
                Ok((
                    tube.xray_spectrum(LIGA_SOURCE_BINS),
                    format!(
                        "{} tube {:.0} kV, relative spectrum",
                        tube.anode.symbol(),
                        tube.kvp
                    ),
                ))
            }
        }
        SourceKind::Betatron(b) => {
            if p.absolute {
                let table = b.spectral_flux_density(p.distance_mm, LIGA_SOURCE_BINS);
                Ok((
                    XraySpectrum::from_flux_density(&table).map_err(|e| e.to_string())?,
                    format!(
                        "betatron E_c {:.2} keV, absolute flux at {:.0} mm (flat-top cone)",
                        b.critical_energy_kev(),
                        p.distance_mm
                    ),
                ))
            } else {
                Ok((
                    b.xray_spectrum(),
                    format!(
                        "betatron E_c {:.2} keV, relative spectrum",
                        b.critical_energy_kev()
                    ),
                ))
            }
        }
        SourceKind::Synchrotron(s) => {
            let e_c = s
                .critical_energy_kev()
                .ok_or("an undulator is quasi-monochromatic: LIGA needs a bending magnet")?;
            if p.absolute {
                let spectrum = XraySpectrum::from_synchrotron_beamline(
                    &s,
                    p.bm_distance_m,
                    p.bm_acceptance_mrad,
                    p.bm_scan_mm,
                )
                .ok_or("not a bending-magnet beamline")?;
                if let XraySpectrum::BendingMagnetBeamline(b) = &spectrum {
                    b.validate().map_err(|e| e.to_string())?;
                }
                Ok((
                    spectrum,
                    format!(
                        "bending magnet E_c {e_c:.2} keV, {:.0} mA, absolute at {:.0} m",
                        s.ring_current_ma, p.bm_distance_m
                    ),
                ))
            } else {
                Ok((
                    XraySpectrum::from_synchrotron(&s).ok_or("not a bending magnet")?,
                    format!("bending magnet E_c {e_c:.2} keV, relative spectrum"),
                ))
            }
        }
        other => Err(format!(
            "{} is not a LIGA source (X-ray tube, betatron or bending magnet)",
            other.kind_label()
        )),
    }
}

/// Depth dose plus the 3D dose volume of a LIGA exposure (same model as
/// `highuvlith deep` LIGA mode).
pub fn run_liga(req: &LigaRequest) -> Result<LigaResult, String> {
    let p = &req.liga;
    let (spectrum, spectrum_label) = liga_spectrum(&req.source, p)?;
    let mut dx = DeepXrayConfig::pmma_default(spectrum);
    dx.resist_thickness_um = p.resist_thickness_um;
    dx.proximity_gap_um = p.gap_um;
    dx.absorber = BeamFilter::new(Compound::gold(19.3), p.absorber_um);
    dx.membrane = BeamFilter::new(Compound::titanium(4.51), p.membrane_um);
    if let Some(spec) = p.filter.spec() {
        dx.filters.push(BeamFilter::new(
            Compound::from_spec(spec).map_err(|e| e.to_string())?,
            p.filter_um,
        ));
    }
    dx.target_bottom_dose_kj_cm3 = p.target_bottom_dose;
    dx.damage_dose_kj_cm3 = p.damage_dose;
    dx.energy_bins = p.energy_bins.max(4);
    dx.proximity_model = if p.fresnel {
        ProximityModel::Fresnel
    } else {
        ProximityModel::Gaussian
    };
    let exposure = deep_xray::expose_depth(&dx);

    let mask = Mask::line_space(p.cd_um * 1e3, p.pitch_um * 1e3).map_err(|e| e.to_string())?;
    let target_pixel = 3.0 * p.pitch_um * 1e3 / p.grid_size as f64;
    let grid = mask
        .commensurate_grid(p.grid_size, target_pixel)
        .map_err(|e| e.to_string())?;
    let sampling = deep_xray::fresnel_sampling(&dx, &grid);
    let volume =
        deep_xray::expose_volumetric(&dx, &mask, &grid, p.nz.max(2)).map_err(|e| e.to_string())?;
    let depth = deep_xray::develop_depth(&volume, p.develop_threshold);
    let (d_min, d_max) = depth
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &d| {
            (a.min(d), b.max(d))
        });

    let mut summary = vec![
        ("Spectrum".to_string(), spectrum_label),
        (
            "Top / bottom dose".to_string(),
            format!(
                "{:.2} / {:.2} kJ/cm\u{b3} (ratio {:.2})",
                exposure.top_dose_kj_cm3, exposure.bottom_dose_kj_cm3, exposure.dose_ratio
            ),
        ),
        (
            "Damage ceiling".to_string(),
            format!(
                "{:.1} kJ/cm\u{b3} \u{2014} {}",
                p.damage_dose,
                if exposure.exceeds_damage_ceiling {
                    "EXCEEDED at the top"
                } else {
                    "ok"
                }
            ),
        ),
        (
            "Mean photon energy top / bottom".to_string(),
            format!(
                "{:.2} / {:.2} keV",
                exposure.mean_energy_top_kev, exposure.mean_energy_bottom_kev
            ),
        ),
        (
            "Exposure time".to_string(),
            match exposure.exposure_time_estimate {
                Some(t) => format!("{t:.1} s ({:.2} h)", t / 3600.0),
                None => "n/a (relative spectrum)".to_string(),
            },
        ),
        (
            "Developed depth".to_string(),
            format!(
                "{:.1}\u{2013}{:.1} \u{b5}m (threshold {:.2} kJ/cm\u{b3})",
                d_min / 1e3,
                d_max / 1e3,
                p.develop_threshold
            ),
        ),
        (
            "Grid".to_string(),
            format!(
                "{0}\u{d7}{0}\u{d7}{1}, pixel {2:.2} \u{b5}m, field {3:.1} \u{b5}m",
                grid.size,
                p.nz.max(2),
                grid.pixel_nm / 1e3,
                grid.field_size_nm() / 1e3
            ),
        ),
        (
            "Fresnel scale top / bottom".to_string(),
            format!(
                "{:.0} / {:.0} nm ({})",
                sampling.fresnel_scale_top_nm,
                sampling.fresnel_scale_bottom_nm,
                if sampling.resolved {
                    "resolved"
                } else {
                    "under-resolved by the grid"
                }
            ),
        ),
    ];
    if let Some(q) = exposure.exposure_charge_ma_h {
        summary.push(("Exposure charge".to_string(), format!("{q:.2} mA\u{b7}h")));
    }
    let mut warnings = sampling.warnings.clone();
    warnings.extend(exposure.warnings.iter().cloned());
    Ok(LigaResult {
        z_um: exposure.z_um.clone(),
        dose_kj_cm3: exposure.dose_kj_cm3.clone(),
        target_bottom_dose: p.target_bottom_dose,
        damage_dose: p.damage_dose,
        volume: Arc::new(VolumeData::from_grid3d(
            DatasetKind::LigaDose,
            &volume,
            "absorbed dose",
            "kJ/cm\u{b3}",
            "depth below the PMMA top",
        )),
        summary,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Talbot carpet
// ---------------------------------------------------------------------------

/// Talbot-carpet settings.
#[derive(Clone, Debug, PartialEq)]
pub struct TalbotParams {
    /// Binary phase grating (π) instead of a binary amplitude grating.
    pub phase_grating: bool,
    pub max_order: usize,
    /// Propagation range in Talbot lengths.
    pub z_max_talbot: f64,
    /// Periods across the x window.
    pub periods: usize,
    pub nx: usize,
    pub nz: usize,
}

impl Default for TalbotParams {
    fn default() -> Self {
        Self {
            phase_grating: false,
            max_order: 10,
            z_max_talbot: 2.0,
            periods: 2,
            nx: 256,
            nz: 256,
        }
    }
}

/// Talbot request (wavelength from the source, period and duty from the mask).
#[derive(Clone, Debug, PartialEq)]
pub struct TalbotRequest {
    pub source: SourceParams,
    pub mask: MaskParams,
    pub talbot: TalbotParams,
}

/// Talbot result.
pub struct TalbotResult {
    pub carpet: Arc<VolumeData>,
    pub wavelength_nm: f64,
    pub period_nm: f64,
    pub talbot_length_nm: f64,
    pub talbot_length_exact_nm: Option<f64>,
}

/// Coherent Talbot carpet I(x, z) of the mask grating at the source wavelength.
pub fn run_talbot(req: &TalbotRequest) -> Result<TalbotResult, String> {
    if req.source.route() == Route::Liga {
        return Err("the Talbot carpet needs a quasi-monochromatic projection-route source".into());
    }
    let lambda = req.source.build(None)?.wavelength_nm();
    let p = &req.talbot;
    let period = req.mask.pitch_nm;
    let duty = req.mask.cd_nm / period;
    let grating = if p.phase_grating {
        Grating::binary_phase(period, duty, std::f64::consts::PI, p.max_order)
    } else {
        Grating::binary_amplitude(period, 1.0 - duty, p.max_order)
    }
    .map_err(|e| e.to_string())?;
    let setup = TalbotSetup::new(grating, lambda).map_err(|e| e.to_string())?;
    let z_t = setup.talbot_length_nm();
    let half = 0.5 * p.periods.max(1) as f64 * period;
    let carpet = setup
        .carpet(
            (-half, half),
            p.nx.max(8),
            (0.0, p.z_max_talbot.max(0.05) * z_t),
            p.nz.max(8),
            None,
        )
        .map_err(|e| e.to_string())?;
    Ok(TalbotResult {
        carpet: Arc::new(VolumeData::from_xz_plane(
            DatasetKind::TalbotCarpet,
            &carpet,
            "intensity",
            "",
            "distance behind the grating",
        )),
        wavelength_nm: lambda,
        period_nm: period,
        talbot_length_nm: z_t,
        talbot_length_exact_nm: setup.talbot_length_exact_nm(),
    })
}

/// X-ray photon spectrum of a LIGA-route source for the Source view:
/// `(E keV, value)` and the value's label.
pub fn xray_source_spectrum(source: &SourceKind) -> Option<(Vec<(f64, f64)>, &'static str)> {
    match source {
        SourceKind::XrayTube(t) => {
            Some((t.binned_photon_rate(120), "photons/s per bin (4\u{3c0})"))
        }
        SourceKind::Betatron(b) => Some((b.photon_spectrum(80), "photons/shot per bin")),
        SourceKind::Synchrotron(s) => {
            let e_c = s.critical_energy_kev()?;
            let pts = (0..120)
                .map(|i| {
                    let e = e_c * 10f64.powf(-2.0 + 3.0 * i as f64 / 119.0);
                    (e, s.bm_flux_per_mrad(e))
                })
                .collect();
            Some((pts, "photons/s/mrad/0.1%BW"))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imaging::{OpticsPreset, PolarizationChoice};
    use crate::sources::PresetId;

    /// VUV F2, NA 0.75, σ 0.7, 100/300 nm lines on a small grid.
    fn small_scene() -> Scene {
        Scene {
            source: SourceParams::default(),
            optics: OpticsParams::default(),
            imaging: ImagingParams {
                max_kernels: 16,
                ..ImagingParams::default()
            },
            mask: MaskParams {
                cd_nm: 100.0,
                pitch_nm: 300.0,
                ..MaskParams::default()
            },
            grid: GridParams {
                size: 64,
                periods: 1,
                ..GridParams::default()
            },
        }
    }

    #[test]
    fn aerial_image_of_a_resolved_pitch_has_contrast_and_a_cd() {
        let cache = EngineCache::default();
        let req = AerialRequest {
            scene: small_scene(),
            focus_nm: 0.0,
            threshold: 0.3,
        };
        let r = run_aerial(&cache, &req).unwrap();
        assert_eq!(r.image.dim(), (64, 64));
        assert!(r.contrast > 0.5, "contrast {}", r.contrast);
        // Clear-field normalized: the bright space approaches 1.
        assert!(r.i_max > 0.8 && r.i_max < 1.3, "I_max {}", r.i_max);
        let cd = r.cd_nm.expect("the line prints at 0.3");
        assert!((cd - 100.0).abs() < 25.0, "CD {cd}");
        assert!(r.nils.unwrap() > 0.5);
        assert!(r.grid.resolves_band);
        assert!(!r.engine_reused);
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        assert!((r.half_field_nm - 150.0).abs() < 1e-9);
        // A focus change reuses the engine.
        let defocused = run_aerial(
            &cache,
            &AerialRequest {
                focus_nm: 300.0,
                ..req.clone()
            },
        )
        .unwrap();
        assert!(defocused.engine_reused);
        assert!(defocused.contrast < r.contrast);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn pitch_below_the_cutoff_images_flat_and_says_why() {
        let mut scene = small_scene();
        scene.mask = MaskParams {
            cd_nm: 30.0,
            pitch_nm: 80.0, // λ/(NA(1+σ)) ≈ 124 nm
            ..MaskParams::default()
        };
        let r = run_aerial(
            &EngineCache::default(),
            &AerialRequest {
                scene,
                focus_nm: 0.0,
                threshold: 0.3,
            },
        )
        .unwrap();
        assert!(r.contrast < 1e-6);
        assert!(r.cutoff_pitch_nm > 120.0);
        assert!(
            r.warnings.iter().any(|w| w.contains("flat")),
            "{:?}",
            r.warnings
        );
    }

    #[test]
    fn spectrum_modes_and_vector_imaging_run() {
        let cache = EngineCache::default();
        let mut scene = small_scene();
        scene.imaging.spectral_samples = Some(3);
        let mut images = Vec::new();
        for mode in SpectrumMode::ALL {
            scene.imaging.spectrum = mode;
            let r = run_aerial(
                &cache,
                &AerialRequest {
                    scene: scene.clone(),
                    focus_nm: 0.0,
                    threshold: 0.3,
                },
            )
            .unwrap();
            assert_eq!(r.spectral_samples.len(), 3);
            images.push(r.contrast);
        }
        // All three modes share one engine (the mode is not part of the key).
        assert_eq!(cache.len(), 1);
        assert!(images.iter().all(|c| *c > 0.5));
        // Vector TE vs TM at NA 0.75: TE keeps more contrast.
        scene.imaging.spectrum = SpectrumMode::Monochromatic;
        scene.imaging.vector = true;
        let mut contrast = |pol| {
            scene.imaging.polarization = pol;
            run_aerial(
                &cache,
                &AerialRequest {
                    scene: scene.clone(),
                    focus_nm: 0.0,
                    threshold: 0.3,
                },
            )
            .unwrap()
            .contrast
        };
        let (te, tm) = (
            contrast(PolarizationChoice::Te),
            contrast(PolarizationChoice::Tm),
        );
        assert!(te > tm, "TE {te} vs TM {tm}");
    }

    #[test]
    fn immersion_and_high_na_presets_image() {
        let cache = EngineCache::default();
        let mut scene = small_scene();
        scene.source.apply_preset(PresetId::ArF).unwrap();
        scene.optics.apply_preset(OpticsPreset::Immersion193i);
        scene.mask = MaskParams {
            cd_nm: 45.0,
            pitch_nm: 90.0,
            ..MaskParams::default()
        };
        scene.source.illumination.mode = crate::sources::IllumMode::Annular;
        scene.source.illumination.sigma_inner = 0.7;
        scene.source.illumination.sigma_outer = 0.9;
        let r = run_aerial(
            &cache,
            &AerialRequest {
                scene: scene.clone(),
                focus_nm: 0.0,
                threshold: 0.3,
            },
        )
        .unwrap();
        assert!((r.na - 1.35).abs() < 1e-12);
        assert!(r.contrast > 0.2, "193i contrast {}", r.contrast);

        // EUV High-NA preset with the obscured pupil.
        scene.source.apply_preset(PresetId::LppSnCo2).unwrap();
        scene.optics.apply_preset(OpticsPreset::HighNa055);
        scene.mask = MaskParams {
            cd_nm: 12.0,
            pitch_nm: 24.0,
            ..MaskParams::default()
        };
        let r = run_aerial(
            &cache,
            &AerialRequest {
                scene,
                focus_nm: 0.0,
                threshold: 0.3,
            },
        )
        .unwrap();
        assert_eq!(r.central_obscuration, Some(0.2));
        assert!(r.contrast > 0.1, "High-NA contrast {}", r.contrast);
    }

    #[test]
    fn liga_sources_are_refused_by_projection_runs() {
        let mut scene = small_scene();
        scene.source.apply_preset(PresetId::TubeW60).unwrap();
        let err = run_aerial(
            &EngineCache::default(),
            &AerialRequest {
                scene,
                focus_nm: 0.0,
                threshold: 0.3,
            },
        )
        .err()
        .unwrap();
        assert!(err.contains("LIGA"), "{err}");
    }

    #[test]
    fn process_window_has_bossung_curves_and_a_window() {
        let cache = EngineCache::default();
        let req = PwRequest {
            scene: small_scene(),
            pw: PwParams {
                n_doses: 5,
                n_focus: 7,
                ..PwParams::default()
            },
        };
        let r = run_process_window(&cache, &req).unwrap();
        assert_eq!(r.window.doses.len(), 5);
        assert_eq!(r.window.focuses.len(), 7);
        assert_eq!(r.window.bossung_curves().len(), 5);
        // Auto focus range: ±2 Rayleigh DOF = ±2 λ/(2NA²).
        let rdof = 157.63 / (2.0 * 0.75 * 0.75);
        assert!((r.rayleigh_dof_nm - rdof).abs() < 1e-9);
        assert!((r.window.focuses[6] - 2.0 * rdof).abs() < 1e-9);
        assert!(r.dof_nm > 0.0, "DOF {}", r.dof_nm);
        assert!(r.el_pct > 0.0, "EL {}", r.el_pct);
        assert!(r.max_area.is_some());
        assert!(!r.el_vs_dof.is_empty());
    }

    #[test]
    fn sweep_is_symmetric() {
        assert_eq!(sweep(30.0, 6.0, 5), vec![24.0, 27.0, 30.0, 33.0, 36.0]);
        assert_eq!(sweep(0.0, 10.0, 1), vec![0.0]);
    }

    #[test]
    fn volumetric_exposure_and_development() {
        let cache = EngineCache::default();
        let mut scene = small_scene();
        scene.grid.size = 32;
        let req = VolumetricRequest {
            scene,
            focus_nm: 0.0,
            vol: VolumetricParams {
                nz: 8,
                n_planes: 2,
                peb: PebChoice::Gaussian,
                ..VolumetricParams::default()
            },
        };
        let r = run_volumetric(&cache, &req).unwrap();
        assert_eq!(r.latent.dims(), (8, 32, 32));
        let (lo, hi) = r.latent.range;
        assert!((0.0..=1.0).contains(&lo) && hi <= 1.0 + 1e-12 && hi > lo);
        let dev = r.developed.expect("fast marching ran");
        assert_eq!(dev.dims(), (8, 32, 32));
        assert!(dev.data.iter().all(|v| *v == 0.0 || *v == 1.0));
        assert!(r.summary.iter().any(|(k, _)| k.starts_with("Developed CD")));
    }

    /// The default Volume tab develops a line/space profile: spaces reach the
    /// substrate while lines stand at the default 30 s, the substrate is not
    /// reached at 20 s, and the lines are gone at 60 s (the narrow window of
    /// the illustrative n = 3 default resist). Without the bake the front
    /// stalls at a standing-wave node well above the substrate.
    #[test]
    fn default_volume_develops_lines_and_spaces() {
        let cache = EngineCache::default();
        let scene = Scene {
            source: SourceParams::default(),
            optics: OpticsParams::default(),
            imaging: ImagingParams::default(),
            mask: MaskParams::default(),
            grid: GridParams::default(),
        };
        let dissolved = |vol: VolumetricParams| {
            let req = VolumetricRequest {
                scene: scene.clone(),
                focus_nm: 0.0,
                vol,
            };
            let d = run_volumetric(&cache, &req).unwrap().developed.unwrap();
            let nz = d.data.dim().0;
            let frac = |k: usize| d.data.index_axis(ndarray::Axis(0), k).mean().unwrap();
            let depth = (0..nz).take_while(|&k| frac(k) > 0.0).count();
            (frac(nz - 1), depth, nz)
        };
        let (bottom, _, _) = dissolved(VolumetricParams::default());
        assert!(
            bottom > 0.2 && bottom < 0.9,
            "bottom dissolved fraction {bottom}"
        );
        let at = |t: f64| VolumetricParams {
            dev_time_s: t,
            ..VolumetricParams::default()
        };
        assert_eq!(dissolved(at(20.0)).0, 0.0);
        assert_eq!(dissolved(at(60.0)).0, 1.0);
        let (bottom, depth, nz) = dissolved(VolumetricParams {
            resist_thickness_nm: 300.0,
            dose_mj_cm2: 30.0,
            peb: PebChoice::None,
            dev_time_s: 60.0,
            ..VolumetricParams::default()
        });
        assert_eq!(bottom, 0.0);
        assert!(
            depth < nz / 2,
            "no-bake front reached slice {depth} of {nz}"
        );
    }

    #[test]
    fn liga_depth_dose_and_volume() {
        let mut source = SourceParams::default();
        source.apply_preset(PresetId::BendingMagnetLiga).unwrap();
        let req = LigaRequest {
            source,
            liga: LigaParams {
                energy_bins: 20,
                grid_size: 16,
                nz: 6,
                fresnel: false,
                ..LigaParams::default()
            },
        };
        let r = run_liga(&req).unwrap();
        assert!(r.z_um.len() > 2 && r.z_um.len() == r.dose_kj_cm3.len());
        // The beam hardens and attenuates: the top receives more dose.
        assert!(r.dose_kj_cm3[0] > *r.dose_kj_cm3.last().unwrap());
        assert_eq!(r.volume.dims(), (6, 16, 16));
        assert_eq!(r.volume.display_unit, "\u{b5}m");
        assert!(r
            .summary
            .iter()
            .any(|(k, v)| k == "Exposure time" && v.ends_with("h)")));
        // X-ray tube, relative spectrum.
        let mut tube = SourceParams::default();
        tube.apply_preset(PresetId::TubeCu40).unwrap();
        let r = run_liga(&LigaRequest {
            source: tube,
            liga: LigaParams {
                absolute: false,
                energy_bins: 20,
                grid_size: 16,
                nz: 4,
                fresnel: false,
                ..LigaParams::default()
            },
        })
        .unwrap();
        assert!(r
            .summary
            .iter()
            .any(|(k, v)| k == "Exposure time" && v.starts_with("n/a")));
        // Projection sources are refused.
        assert!(run_liga(&LigaRequest {
            source: SourceParams::default(),
            liga: LigaParams::default(),
        })
        .is_err());
    }

    #[test]
    fn talbot_carpet_dimensions_and_length() {
        let mut source = SourceParams::default();
        source.apply_preset(PresetId::LppSnCo2).unwrap();
        let req = TalbotRequest {
            source,
            mask: MaskParams {
                cd_nm: 50.0,
                pitch_nm: 100.0,
                ..MaskParams::default()
            },
            talbot: TalbotParams {
                nx: 64,
                nz: 32,
                ..TalbotParams::default()
            },
        };
        let r = run_talbot(&req).unwrap();
        // Paraxial Talbot length 2p²/λ.
        assert!((r.talbot_length_nm - 2.0 * 100.0 * 100.0 / 13.5).abs() < 1e-6);
        assert_eq!(r.carpet.dims(), (32, 1, 64));
        assert!(r.carpet.is_xz_plane());
        assert!((r.carpet.z_nm.1 - 2.0 * r.talbot_length_nm).abs() < 1e-6);
    }

    #[test]
    fn xray_spectra_for_the_source_view() {
        for id in [
            PresetId::TubeW60,
            PresetId::BetatronLwfa,
            PresetId::BendingMagnetLiga,
        ] {
            let mut p = SourceParams::default();
            p.apply_preset(id).unwrap();
            let (pts, _) = xray_source_spectrum(&p.build(None).unwrap()).unwrap();
            assert!(pts.len() > 50);
            assert!(pts
                .iter()
                .all(|(e, v)| *e > 0.0 && v.is_finite() && *v >= 0.0));
        }
        assert!(xray_source_spectrum(&SourceParams::default().build(None).unwrap()).is_none());
    }
}
