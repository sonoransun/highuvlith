//! Immersion optics, EUV projection presets, and clear-field normalization.
//!
//! Masks are explicit `MaskFeature::Rect` lists on fields that are an exact
//! multiple of the pitch, with edges on pixel boundaries (independent of the
//! `Mask::line_space` definition and of raster vs analytic spectra).

use highuvlith_core::aerial::{
    AerialImageEngine, ImageNormalization, ImagingModel, ImagingSettings,
};
use highuvlith_core::mask::{Mask, MaskFeature, MaskType};
use highuvlith_core::materials::multilayer::MultilayerMirror;
use highuvlith_core::math::fft2d::Fft2D;
use highuvlith_core::optics::euv::EuvProjectionOptics;
use highuvlith_core::optics::multilayer_pupil::{MirrorAngleMap, MultilayerPupil};
use highuvlith_core::optics::schwarzschild::SchwarzschildObjective;
use highuvlith_core::optics::vector::{IlluminationPolarization, VectorSettings};
use highuvlith_core::optics::zone_plate::FresnelZonePlate;
use highuvlith_core::optics::{
    defocus_phase_in_medium, Apodization, OpticalSystem, ProjectionOptics, WATER_INDEX_193NM,
};
use highuvlith_core::source::LithographySource;
use highuvlith_core::types::{Complex64, GridConfig};
use ndarray::Array2;

/// Monochromatic source defined by its pupil fill.
struct Fill {
    wavelength_nm: f64,
    fill: fn(f64, f64) -> f64,
}

impl LithographySource for Fill {
    fn wavelength_nm(&self) -> f64 {
        self.wavelength_nm
    }
    fn bandwidth_pm(&self) -> f64 {
        0.0
    }
    fn intensity_at(&self, x: f64, y: f64) -> f64 {
        (self.fill)(x, y)
    }
    fn spectral_weights(&self) -> Vec<(f64, f64)> {
        vec![(self.wavelength_nm, 1.0)]
    }
}

fn annular_07_09(x: f64, y: f64) -> f64 {
    let r = x.hypot(y);
    if (0.7..=0.9).contains(&r) {
        1.0
    } else {
        0.0
    }
}

fn annular_05_09(x: f64, y: f64) -> f64 {
    let r = x.hypot(y);
    if (0.5..=0.9).contains(&r) {
        1.0
    } else {
        0.0
    }
}

fn conventional_06(x: f64, y: f64) -> f64 {
    if x.hypot(y) <= 0.6 {
        1.0
    } else {
        0.0
    }
}

fn all_kernels() -> ImagingSettings {
    ImagingSettings {
        max_kernels: 100_000,
        kernel_energy_fraction: 1.0,
        ..Default::default()
    }
}

/// Opaque lines of width pitch/2 tiling a square field of `periods` pitches.
fn half_duty_lines(pitch: f64, periods: usize) -> Mask {
    let field = pitch * periods as f64;
    Mask {
        mask_type: MaskType::Binary,
        dark_field: false,
        features: (0..periods)
            .map(|k| MaskFeature::Rect {
                x: -field / 2.0 + (k as f64 + 0.5) * pitch,
                y: 0.0,
                w: pitch / 2.0,
                h: field,
            })
            .collect(),
    }
}

fn clear_mask() -> Mask {
    Mask {
        mask_type: MaskType::Binary,
        dark_field: false,
        features: vec![],
    }
}

fn contrast(img: &Array2<f64>) -> f64 {
    let max = img.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min = img.iter().cloned().fold(f64::INFINITY, f64::min);
    (max - min) / (max + min)
}

fn max_abs_diff(a: &Array2<f64>, b: &Array2<f64>) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

// ---------------------------------------------------------------------------
// Immersion
// ---------------------------------------------------------------------------

#[test]
fn immersion_resolves_a_pitch_dry_optics_cannot() {
    // λ = 193 nm, annular σ 0.7–0.9, pitch 90 nm (< λ/1.0): the incoherent
    // limit λ/(NA(1+σ_max)) is 109 nm for dry NA 0.93 but 75 nm for water
    // immersion at NA 1.35 — only the immersion lens images the grating.
    let source = Fill {
        wavelength_nm: 193.0,
        fill: annular_07_09,
    };
    let grid = GridConfig {
        size: 64,
        pixel_nm: 11.25, // 720 nm field = 8 pitches, edges on pixel boundaries
    };
    let mask = half_duty_lines(90.0, 8);
    let mut dry = ProjectionOptics::new(0.93).unwrap();
    dry.flare_fraction = 0.0;
    let mut wet = ProjectionOptics::immersion_193i();
    wet.flare_fraction = 0.0;
    let c_dry = contrast(
        &AerialImageEngine::new(&source, &dry, grid.clone(), 32)
            .unwrap()
            .compute(&mask, 0.0)
            .data,
    );
    let wet_engine = AerialImageEngine::new(&source, &wet, grid, 32).unwrap();
    let c_wet = contrast(&wet_engine.compute(&mask, 0.0).data);
    assert!(c_dry < 1e-9, "dry NA 0.93 must not resolve 90 nm: {c_dry}");
    assert!(c_wet > 0.2, "NA 1.35 immersion must resolve 90 nm: {c_wet}");
    assert!(
        !wet_engine
            .kernel_diagnostics(0.0, 193.0)
            .unwrap()
            .support_exceeds_nyquist
    );
}

#[test]
fn immersion_defocus_uses_the_medium_index() {
    // Coherent three-beam grating image in closed form (see
    // analytical_validation.rs) with the defocus phase evaluated in water:
    // Φ = (2π n/λ) z (1 − √(1 − (λ/(n p))²)).
    let (wl, pitch, pixel, periods) = (193.0, 200.0, 10.0, 4);
    let n = (pitch * periods as f64 / pixel) as usize; // 80 px
    let grid = GridConfig {
        size: n,
        pixel_nm: pixel,
    };
    let mask = half_duty_lines(pitch, periods);
    let mut optics = ProjectionOptics::immersion_193i();
    optics.flare_fraction = 0.0;
    let engine = AerialImageEngine::with_settings(
        &Fill {
            wavelength_nm: wl,
            fill: conventional_06,
        },
        &optics,
        grid.clone(),
        ImagingSettings {
            source_points_per_axis: Some(1),
            ..Default::default()
        },
    )
    .unwrap();
    let spectrum = mask.spectrum(&grid, &Fft2D::new());
    let norm = (n * n) as f64;
    let (c0, c1, cm1) = (
        spectrum[[0, 0]] / norm,
        spectrum[[0, periods]] / norm,
        spectrum[[0, n - periods]] / norm,
    );
    for z in [0.0, 150.0, -400.0] {
        // Order ±1 sits at NA·ρ = λ/p in the pupil.
        let phi = defocus_phase_in_medium(z, wl / pitch, WATER_INDEX_193NM, wl, false);
        let tilt = Complex64::from_polar(1.0, phi);
        let img = engine.compute(&mask, z).data;
        for j in 0..n {
            let x = 2.0 * std::f64::consts::PI * j as f64 / (n / periods) as f64;
            let e = c0
                + (c1 * Complex64::from_polar(1.0, x) + cm1 * Complex64::from_polar(1.0, -x))
                    * tilt;
            assert!(
                (img[[0, j]] - e.norm_sqr()).abs() < 1e-12,
                "z = {z}, j = {j}"
            );
        }
    }
    // The medium phase differs from the vacuum phase at the same NA·ρ, so
    // the check above distinguishes them.
    let dry = defocus_phase_in_medium(400.0, wl / pitch, 1.0, wl, false);
    let wet = defocus_phase_in_medium(400.0, wl / pitch, WATER_INDEX_193NM, wl, false);
    assert!((dry - wet).abs() > 0.5);
}

#[test]
fn engine_rejects_na_above_the_medium_index() {
    // A struct literal can claim NA 1.35 without an immersion medium.
    let bogus = ProjectionOptics {
        na: 1.35,
        ..ProjectionOptics::new(0.5).unwrap()
    };
    let source = Fill {
        wavelength_nm: 193.0,
        fill: conventional_06,
    };
    let grid = GridConfig {
        size: 32,
        pixel_nm: 10.0,
    };
    assert!(AerialImageEngine::new(&source, &bogus, grid, 8).is_err());
}

#[test]
fn vector_imaging_inherits_the_immersion_index() {
    let source = Fill {
        wavelength_nm: 193.0,
        fill: conventional_06,
    };
    let grid = GridConfig {
        size: 32,
        pixel_nm: 10.0,
    };
    let optics = ProjectionOptics::immersion_193i();
    let vector = |vs: VectorSettings| ImagingSettings {
        max_kernels: 16,
        source_points_per_axis: Some(7),
        imaging_model: ImagingModel::Vector(vs),
        ..Default::default()
    };
    let engine = AerialImageEngine::with_settings(
        &source,
        &optics,
        grid.clone(),
        vector(VectorSettings::default()),
    )
    .unwrap();
    match &engine.settings().imaging_model {
        ImagingModel::Vector(vs) => {
            assert_eq!(vs.image_index, WATER_INDEX_193NM);
            assert_eq!(vs.reduction, optics.reduction());
        }
        ImagingModel::Scalar => panic!("vector settings lost"),
    }
    // One image-space medium: an explicit index equal to the optics' index is
    // accepted, a different one (which would make the defocus phase and the
    // vector fields use different media) is rejected.
    let explicit = AerialImageEngine::with_settings(
        &source,
        &optics,
        grid.clone(),
        vector(VectorSettings {
            image_index: WATER_INDEX_193NM,
            ..VectorSettings::default()
        }),
    );
    assert!(explicit.is_ok());
    for bad in [1.7, 1.2] {
        assert!(AerialImageEngine::with_settings(
            &source,
            &optics,
            grid.clone(),
            vector(VectorSettings {
                image_index: bad,
                ..VectorSettings::default()
            }),
        )
        .is_err());
    }
    // Dry optics with an explicit immersion index in the vector settings is
    // rejected too (set the medium on the optics instead).
    assert!(AerialImageEngine::with_settings(
        &source,
        &ProjectionOptics::new(0.9).unwrap(),
        grid,
        vector(VectorSettings {
            image_index: 1.44,
            ..VectorSettings::default()
        }),
    )
    .is_err());
}

#[test]
fn te_tm_zero_columns_are_dropped() {
    // Off-axis source points carry only one TE (or TM) state; the second
    // state's three columns are identically zero and must not reach the
    // eigensolver.
    let source = Fill {
        wavelength_nm: 193.0,
        fill: annular_07_09,
    };
    let grid = GridConfig {
        size: 32,
        pixel_nm: 10.0,
    };
    let engine = AerialImageEngine::with_settings(
        &source,
        &ProjectionOptics::new(0.9).unwrap(),
        grid,
        ImagingSettings {
            max_kernels: 16,
            source_points_per_axis: Some(9),
            imaging_model: ImagingModel::Vector(VectorSettings {
                polarization: IlluminationPolarization::Te,
                ..VectorSettings::default()
            }),
            ..Default::default()
        },
    )
    .unwrap();
    let points = engine.source_points().len();
    let d = engine.kernel_diagnostics(0.0, 193.0).unwrap();
    // No point of the annulus is on axis: exactly 3 live columns per point.
    assert_eq!(d.num_columns, 3 * points);
}

// ---------------------------------------------------------------------------
// EUV projection presets
// ---------------------------------------------------------------------------

#[test]
fn high_na_resolves_a_pitch_below_the_nxe_limit() {
    // λ = 13.5 nm, annular σ 0.5–0.9, pitch 18 nm: below the NA 0.33 limit
    // λ/(NA(1+σ)) = 21.5 nm, above the NA 0.55 limit (12.9 nm). The annulus
    // clears the High-NA central obscuration (0.2 NA).
    let source = Fill {
        wavelength_nm: 13.5,
        fill: annular_05_09,
    };
    let grid = GridConfig {
        size: 64,
        pixel_nm: 2.25, // 144 nm field = 8 pitches
    };
    let mask = half_duty_lines(18.0, 8);
    let nxe =
        AerialImageEngine::new(&source, &EuvProjectionOptics::nxe_033(), grid.clone(), 32).unwrap();
    let hna =
        AerialImageEngine::new(&source, &EuvProjectionOptics::high_na_055(), grid, 32).unwrap();
    let c_nxe = contrast(&nxe.compute(&mask, 0.0).data);
    let c_hna = contrast(&hna.compute(&mask, 0.0).data);
    assert!(c_nxe < 1e-9, "NA 0.33 must not resolve 18 nm: {c_nxe}");
    assert!(c_hna > 0.2, "NA 0.55 must resolve 18 nm: {c_hna}");
}

#[test]
fn obscured_pupil_with_on_axis_light_is_dark_field() {
    // A coherent on-axis point sends the zero order into the High-NA central
    // obscuration: the clear field vanishes, relative intensity is undefined
    // and the engine reports the absolute (dark-field) image instead.
    let source = Fill {
        wavelength_nm: 13.5,
        fill: conventional_06,
    };
    let grid = GridConfig {
        size: 64,
        pixel_nm: 2.5, // 160 nm field = 4 pitches of 40 nm
    };
    let settings = |normalization| ImagingSettings {
        source_points_per_axis: Some(1),
        normalization,
        ..Default::default()
    };
    let optics = EuvProjectionOptics::high_na_055();
    let relative = AerialImageEngine::with_settings(
        &source,
        &optics,
        grid.clone(),
        settings(ImageNormalization::ClearField),
    )
    .unwrap();
    let absolute = AerialImageEngine::with_settings(
        &source,
        &optics,
        grid,
        settings(ImageNormalization::Absolute),
    )
    .unwrap();
    assert_eq!(relative.clear_field_intensity(0.0), 0.0);
    assert!(!relative.kernel_diagnostics(0.0, 13.5).unwrap().normalized);
    let mask = half_duty_lines(40.0, 4);
    let a = relative.compute(&mask, 0.0).data;
    let b = absolute.compute(&mask, 0.0).data;
    assert_eq!(a, b);
    // Only the ±1 orders pass: a frequency-doubled, non-trivial fringe.
    assert!(a.iter().cloned().fold(0.0, f64::max) > 1e-3);
}

fn gaussian_005(x: f64, y: f64) -> f64 {
    let r2 = x * x + y * y;
    if r2 <= 1.0 {
        (-r2 / (2.0 * 0.05 * 0.05)).exp()
    } else {
        0.0
    }
}

#[test]
fn near_coherent_beam_inside_an_obscuration_is_dark_field() {
    // A σ_g = 0.05 Gaussian beam through a Schwarzschild objective with a
    // 0.25 central obscuration: only the far Gaussian tail (~1e-6 of the
    // power) reaches the pupil's clear annulus. The clear field is tiny but
    // non-zero; relative intensity would blow up by ~1e6, so the engine
    // treats it as dark field and reports absolute intensity.
    let source = Fill {
        wavelength_nm: 13.5,
        fill: gaussian_005,
    };
    let optics = SchwarzschildObjective {
        flare: 0.0,
        ..SchwarzschildObjective::euv_standard()
    };
    let grid = GridConfig {
        size: 64,
        pixel_nm: 2.0,
    };
    let engine = AerialImageEngine::new(&source, &optics, grid.clone(), 16).unwrap();
    let absolute = AerialImageEngine::with_settings(
        &source,
        &optics,
        grid,
        ImagingSettings {
            max_kernels: 16,
            normalization: ImageNormalization::Absolute,
            ..Default::default()
        },
    )
    .unwrap();
    let d = engine.kernel_diagnostics(0.0, 13.5).unwrap();
    assert!(
        d.clear_field_intensity < 1e-3,
        "clear {}",
        d.clear_field_intensity
    );
    assert!(!d.normalized);
    let mask = half_duty_lines(32.0, 4);
    let a = engine.compute(&mask, 0.0).data;
    let b = absolute.compute(&mask, 0.0).data;
    assert_eq!(a, b);
    let peak = a.iter().cloned().fold(0.0, f64::max);
    assert!(
        peak < 1.0,
        "absolute dark-field image stays below the incident level: {peak}"
    );
}

// ---------------------------------------------------------------------------
// Multilayer pupil (optional angle-dependent mirror response)
// ---------------------------------------------------------------------------

fn conventional_05(x: f64, y: f64) -> f64 {
    if x.hypot(y) <= 0.5 {
        1.0
    } else {
        0.0
    }
}

fn two_mirror_pupil(map: MirrorAngleMap) -> MultilayerPupil {
    MultilayerPupil::new(MultilayerMirror::mo_si(40, 6.9, 0.4).unwrap(), vec![map; 2]).unwrap()
}

#[test]
fn uniform_multilayer_reflectance_is_only_a_dose_change() {
    // A constant incidence angle gives the same complex reflectance at every
    // pupil point: relative images are unchanged, only the absolute clear
    // field scales, by |r(0°)|⁴ for two mirrors.
    let source = Fill {
        wavelength_nm: 13.5,
        fill: conventional_05,
    };
    let grid = GridConfig {
        size: 64,
        pixel_nm: 45.0 / 16.0,
    };
    let plain = EuvProjectionOptics::nxe_033();
    let mut coated = plain.clone();
    coated.multilayer = Some(two_mirror_pupil(MirrorAngleMap::constant(0.0)));
    let e0 = AerialImageEngine::new(&source, &plain, grid.clone(), 24).unwrap();
    let e1 = AerialImageEngine::new(&source, &coated, grid, 24).unwrap();
    let mask = half_duty_lines(45.0, 4);
    for z in [0.0, 60.0] {
        assert!(max_abs_diff(&e0.compute(&mask, z).data, &e1.compute(&mask, z).data) < 1e-12);
    }
    let r0 = MultilayerMirror::mo_si(40, 6.9, 0.4)
        .unwrap()
        .reflectance(13.5, 0.0, highuvlith_core::types::Polarization::TE)
        .unwrap();
    assert!((e1.clear_field_intensity(0.0) - r0 * r0).abs() < 1e-9);
    assert!((e0.clear_field_intensity(0.0) - 1.0).abs() < 1e-12);
}

#[test]
fn angle_dependent_multilayer_apodizes_and_shifts_focus() {
    // Incidence 0° at the pupil centre rising to 15° at the rim on each of
    // two Mo/Si mirrors: the rim is darkened (apodization) and the
    // angle-dependent reflection phase acts as a pupil aberration that
    // moves best focus. The clear field stays exactly 1 (relative
    // intensity), so neither effect is a dose change.
    let source = Fill {
        wavelength_nm: 13.5,
        fill: conventional_05,
    };
    // Two periods of a 60 nm grating (periodic field: same image as more).
    let grid = GridConfig {
        size: 32,
        pixel_nm: 60.0 / 16.0,
    };
    let plain = EuvProjectionOptics::nxe_033();
    let mut coated = plain.clone();
    coated.multilayer = Some(two_mirror_pupil(MirrorAngleMap {
        radial_deg: 15.0,
        ..MirrorAngleMap::constant(0.0)
    }));
    let e0 =
        AerialImageEngine::with_settings(&source, &plain, grid.clone(), all_kernels()).unwrap();
    let e1 = AerialImageEngine::with_settings(&source, &coated, grid, all_kernels()).unwrap();
    for &v in e1.compute(&clear_mask(), 0.0).data.iter() {
        assert!((v - 1.0).abs() < 1e-12, "clear field {v}");
    }
    let mask = half_duty_lines(60.0, 2);
    let (p0, p60) = (
        contrast(&e0.compute(&mask, 0.0).data),
        contrast(&e0.compute(&mask, 60.0).data),
    );
    let (m0, m60) = (
        contrast(&e1.compute(&mask, 0.0).data),
        contrast(&e1.compute(&mask, 60.0).data),
    );
    // Uniform pupil: best focus at z = 0. Coated pupil: contrast lower at
    // z = 0 and higher at z = +60 nm (best focus moved).
    assert!(p60 < p0, "plain: {p0} -> {p60}");
    assert!(m0 < p0 - 0.1, "apodized/aberrated in focus: {m0} vs {p0}");
    assert!(m60 > m0 + 0.1, "coated pupil refocuses: {m0} -> {m60}");
}

#[test]
fn multilayer_pupil_rejects_wavelengths_without_optical_constants() {
    let mut optics = SchwarzschildObjective::euv_standard();
    optics.multilayer = Some(two_mirror_pupil(MirrorAngleMap::constant(0.0)));
    let source = Fill {
        wavelength_nm: 2000.0,
        fill: conventional_05,
    };
    let grid = GridConfig {
        size: 32,
        pixel_nm: 500.0,
    };
    assert!(AerialImageEngine::new(&source, &optics, grid, 8).is_err());
}

// ---------------------------------------------------------------------------
// Clear-field normalization
// ---------------------------------------------------------------------------

/// Independent clear-field value Σ_s w_s |P(s; z)|² from the engine's own
/// source points and the optics' pupil.
fn clear_field_reference(engine: &AerialImageEngine, optics: &dyn OpticalSystem, z: f64) -> f64 {
    engine
        .source_points()
        .iter()
        .map(|p| {
            p.weight
                * optics
                    .pupil_function(p.sx, p.sy, z, engine.wavelength_nm())
                    .norm_sqr()
        })
        .sum()
}

#[test]
fn lossy_pupils_image_a_clear_field_to_exactly_one() {
    let mut apodized = ProjectionOptics::new(0.6).unwrap();
    apodized.apodization = Apodization::Gaussian { alpha: 0.7 };
    let cases: Vec<(Box<dyn OpticalSystem>, f64, fn(f64, f64) -> f64)> = vec![
        (Box::new(apodized), 193.0, conventional_06),
        (
            Box::new(SchwarzschildObjective::euv_standard()),
            13.5,
            annular_05_09,
        ),
        // Part of the σ 0.6 disk falls inside the 0.2-NA central obscuration.
        (
            Box::new(EuvProjectionOptics::high_na_055()),
            13.5,
            conventional_06,
        ),
        (
            Box::new(FresnelZonePlate::new(25.0, 1.0).unwrap()),
            1.0,
            conventional_06,
        ),
    ];
    for (optics, wl, fill) in cases {
        let source = Fill {
            wavelength_nm: wl,
            fill,
        };
        // ~3 pupil radii of mask frequencies per axis on a 32-pixel field.
        let field = 3.0 * wl / optics.na();
        let grid = GridConfig {
            size: 32,
            pixel_nm: field / 32.0,
        };
        let engine =
            AerialImageEngine::with_settings(&source, optics.as_ref(), grid, all_kernels())
                .unwrap();
        for z in [0.0, 3.0 * wl] {
            let reference = clear_field_reference(&engine, optics.as_ref(), z);
            let clear = engine.clear_field_intensity(z);
            assert!(
                reference > 0.01 && reference < 1.0 - 1e-3,
                "lossy: {reference}"
            );
            assert!(
                (clear - reference).abs() < 1e-12 * reference,
                "{clear} vs {reference}"
            );
            let flare = optics.flare_fraction();
            for &v in engine.compute(&clear_mask(), z).data.iter() {
                // Flare blends toward the mean, which is also 1.
                assert!((v - 1.0).abs() < 1e-12, "clear field {v} (flare {flare})");
            }
        }
    }
}

#[test]
fn absolute_mode_is_relative_times_clear_field() {
    let source = Fill {
        wavelength_nm: 13.5,
        fill: annular_05_09,
    };
    let optics = SchwarzschildObjective {
        flare: 0.0,
        ..SchwarzschildObjective::euv_standard()
    };
    let grid = GridConfig {
        size: 32,
        pixel_nm: 2.5,
    };
    let make = |normalization| {
        AerialImageEngine::with_settings(
            &source,
            &optics,
            grid.clone(),
            ImagingSettings {
                max_kernels: 12,
                normalization,
                ..Default::default()
            },
        )
        .unwrap()
    };
    let rel = make(ImageNormalization::ClearField);
    let abs = make(ImageNormalization::Absolute);
    let mask = half_duty_lines(40.0, 2);
    for z in [0.0, 25.0] {
        let c = rel.clear_field_intensity(z);
        assert!(c > 0.2 && c < 0.45, "R² × unobscured fraction: {c}");
        let r = rel.compute(&mask, z).data;
        let a = abs.compute(&mask, z).data;
        let scaled = &r * c;
        assert!(max_abs_diff(&scaled, &a) < 1e-12);
        // The C1 identity holds in both modes with each engine's kernels.
        let fft = Fft2D::new();
        let spectrum = mask.spectrum(rel.grid(), &fft);
        for (engine, image) in [(&rel, &r), (&abs, &a)] {
            let ks = engine.kernels(z);
            let mut manual = Array2::<f64>::zeros(image.dim());
            for (l, k) in ks.eigenvalues.iter().zip(&ks.kernels) {
                let mut field = k * &spectrum;
                fft.inverse(&mut field);
                manual.zip_mut_with(&field, |m, e| *m += l * e.norm_sqr());
            }
            assert!(max_abs_diff(&manual, image) < 1e-12);
        }
    }
}
