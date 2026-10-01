//! Validation of the Hopkins/SOCS imaging engine against an independent
//! direct Abbe summation and against closed-form imaging theory.
//!
//! Test masks are explicit `MaskFeature::Rect` lists on fields that are an
//! exact multiple of the pitch, with edges on pixel boundaries, so the
//! expectations do not depend on how `Mask::line_space` is defined or on
//! whether the mask spectrum is rasterized or analytic.

use highuvlith_core::aerial::{
    AerialImageEngine, DecompositionMethod, DefocusModel, ImagingModel, ImagingSettings,
};
use highuvlith_core::mask::{Mask, MaskFeature, MaskType};
use highuvlith_core::math::fft2d::Fft2D;
use highuvlith_core::optics::schwarzschild::SchwarzschildObjective;
use highuvlith_core::optics::vector::{IlluminationPolarization, VectorSettings};
use highuvlith_core::optics::{OpticalSystem, ProjectionOptics};
use highuvlith_core::source::{LithographySource, VuvSource};
use highuvlith_core::types::{Complex64, GridConfig};
use ndarray::Array2;

/// A source defined directly by its pupil fill and line spectrum.
struct PupilFill {
    wavelength_nm: f64,
    fill: fn(f64, f64) -> f64,
    spectrum: Vec<(f64, f64)>,
}

impl PupilFill {
    fn mono(wavelength_nm: f64, fill: fn(f64, f64) -> f64) -> Self {
        Self {
            wavelength_nm,
            fill,
            spectrum: vec![(wavelength_nm, 1.0)],
        }
    }
}

impl LithographySource for PupilFill {
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
        self.spectrum.clone()
    }
}

fn conventional_07(x: f64, y: f64) -> f64 {
    if x.hypot(y) <= 0.7 {
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

/// Symmetric x-dipole tuned to pitch 150 nm at λ = 100 nm, NA = 0.5:
/// σ_c = λ / (2·p·NA) = 2/3, pole radius 0.05.
fn dipole_150(x: f64, y: f64) -> f64 {
    let c = 100.0 / (2.0 * 150.0 * 0.5);
    if (x - c).hypot(y) <= 0.05 || (x + c).hypot(y) <= 0.05 {
        1.0
    } else {
        0.0
    }
}

fn optics_no_flare(na: f64) -> ProjectionOptics {
    let mut o = ProjectionOptics::new(na).unwrap();
    o.flare_fraction = 0.0;
    o
}

fn all_kernels() -> ImagingSettings {
    ImagingSettings {
        max_kernels: 100_000,
        kernel_energy_fraction: 1.0,
        ..Default::default()
    }
}

/// Bright-field L/S: opaque lines of width `width` on pitch `pitch`, tiling a
/// square field of side `field` (a multiple of the pitch).
fn line_space(pitch: f64, width: f64, field: f64) -> Mask {
    let n = (field / pitch).round() as usize;
    assert!(
        (n as f64 * pitch - field).abs() < 1e-9,
        "field must be a multiple of the pitch"
    );
    Mask {
        mask_type: MaskType::Binary,
        dark_field: false,
        features: (0..n)
            .map(|k| MaskFeature::Rect {
                x: -field / 2.0 + (k as f64 + 0.5) * pitch,
                y: 0.0,
                w: width,
                h: field,
            })
            .collect(),
    }
}

/// A 2D test pattern (clear features on a dark field): an L-shaped pair of
/// rectangles plus an isolated square, all edges on a 8 nm pixel lattice.
fn pattern_2d() -> Mask {
    Mask {
        mask_type: MaskType::Binary,
        dark_field: true,
        features: vec![
            MaskFeature::Rect {
                x: -64.0,
                y: 16.0,
                w: 96.0,
                h: 224.0,
            },
            MaskFeature::Rect {
                x: 24.0,
                y: -80.0,
                w: 176.0,
                h: 64.0,
            },
            MaskFeature::Rect {
                x: 120.0,
                y: 120.0,
                w: 48.0,
                h: 48.0,
            },
        ],
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

fn signed(k: usize, n: usize) -> f64 {
    if k < n / 2 {
        k as f64
    } else {
        k as f64 - n as f64
    }
}

/// Independent reference: direct Abbe summation over the engine's source
/// points, with the pupil evaluated at EVERY grid frequency (no support
/// truncation, no eigendecomposition):
///   I = Σ_s w_s |IFFT[ P((f + s·f_c)/f_c; z, λ) · M(f) ]|².
fn abbe_image(
    engine: &AerialImageEngine,
    optics: &dyn OpticalSystem,
    mask: &Mask,
    z: f64,
    wavelength_nm: f64,
) -> Array2<f64> {
    let grid = engine.grid();
    let n = grid.size;
    let df = grid.freq_step();
    let fft = Fft2D::new();
    let spectrum = mask.spectrum(grid, &fft);
    let fc = optics.cutoff_frequency(wavelength_nm);
    let mut total = Array2::<f64>::zeros((n, n));
    for p in engine.source_points() {
        let mut field = Array2::<Complex64>::zeros((n, n));
        for i in 0..n {
            for j in 0..n {
                let fx = signed(j, n) * df / fc + p.sx;
                let fy = signed(i, n) * df / fc + p.sy;
                field[[i, j]] = optics.pupil_function(fx, fy, z, wavelength_nm) * spectrum[[i, j]];
            }
        }
        fft.inverse(&mut field);
        total.zip_mut_with(&field, |t, e| *t += p.weight * e.norm_sqr());
    }
    total
}

fn assert_matches_abbe(
    engine: &AerialImageEngine,
    optics: &dyn OpticalSystem,
    mask: &Mask,
    z: f64,
) {
    // The engine reports relative intensity; the Abbe sum is absolute.
    let socs = &engine.compute(mask, z).data * engine.clear_field_intensity(z);
    let abbe = abbe_image(engine, optics, mask, z, engine.wavelength_nm());
    let peak = abbe.iter().cloned().fold(0.0, f64::max);
    let err = max_abs_diff(&socs, &abbe) / peak;
    eprintln!("SOCS vs Abbe: max relative error {err:.2e} at z = {z} nm");
    assert!(
        err <= 1e-8,
        "SOCS vs Abbe relative error {err:.3e} at z = {z}"
    );
}

// ---------------------------------------------------------------------------
// SOCS ≡ Abbe
// ---------------------------------------------------------------------------

#[test]
fn socs_equals_abbe_conventional_focus_and_defocus() {
    let source = PupilFill::mono(100.0, conventional_07);
    let optics = optics_no_flare(0.5);
    let grid = GridConfig {
        size: 64,
        pixel_nm: 8.0,
    };
    let settings = ImagingSettings {
        source_points_per_axis: Some(13),
        ..all_kernels()
    };
    let engine = AerialImageEngine::with_settings(&source, &optics, grid, settings).unwrap();
    let mask = pattern_2d();
    for z in [0.0, 250.0, -400.0] {
        assert_matches_abbe(&engine, &optics, &mask, z);
    }
}

#[test]
fn socs_equals_abbe_dipole_gram_path() {
    // Large field (many mask orders) and few source points: the Gram path.
    let source = PupilFill::mono(100.0, dipole_150);
    let optics = optics_no_flare(0.5);
    let grid = GridConfig {
        size: 128,
        pixel_nm: 9.375,
    };
    let settings = ImagingSettings {
        source_points_per_axis: Some(40),
        ..all_kernels()
    };
    let engine = AerialImageEngine::with_settings(&source, &optics, grid, settings).unwrap();
    let diag = engine.kernel_diagnostics(0.0, 100.0).unwrap();
    assert_eq!(diag.method, DecompositionMethod::DenseGram);
    assert!(diag.num_frequencies > diag.num_columns);
    let mask = line_space(150.0, 75.0, 1200.0);
    for z in [0.0, 300.0] {
        assert_matches_abbe(&engine, &optics, &mask, z);
    }
}

#[test]
fn socs_equals_abbe_annular_with_aberrations_and_psm() {
    let source = PupilFill::mono(100.0, annular_05_09);
    let mut optics = optics_no_flare(0.6);
    optics.zernike_coefficients = vec![(7, 0.05), (9, 0.03)]; // coma + spherical
    let grid = GridConfig {
        size: 64,
        pixel_nm: 8.0,
    };
    let settings = ImagingSettings {
        source_points_per_axis: Some(15),
        ..all_kernels()
    };
    let engine = AerialImageEngine::with_settings(&source, &optics, grid, settings).unwrap();
    let mut mask = pattern_2d();
    mask.mask_type = MaskType::AttenuatedPSM {
        transmission: 0.06,
        phase_deg: 180.0,
    };
    for z in [0.0, 150.0] {
        assert_matches_abbe(&engine, &optics, &mask, z);
    }
}

// ---------------------------------------------------------------------------
// Normalization, positivity, symmetry
// ---------------------------------------------------------------------------

#[test]
fn clear_field_is_unity_at_any_defocus() {
    let optics = optics_no_flare(0.5);
    let grid = GridConfig {
        size: 32,
        pixel_nm: 12.5,
    };
    let clear = Mask {
        mask_type: MaskType::Binary,
        dark_field: false,
        features: vec![],
    };
    for fill in [conventional_07, annular_05_09, dipole_150] {
        let source = PupilFill::mono(100.0, fill);
        let exact = AerialImageEngine::with_settings(&source, &optics, grid.clone(), all_kernels())
            .unwrap();
        let truncated = AerialImageEngine::new(&source, &optics, grid.clone(), 6).unwrap();
        for z in [0.0, 200.0, -1000.0] {
            for &v in exact.compute(&clear, z).data.iter() {
                assert!((v - 1.0).abs() < 1e-12, "clear field {v} at z = {z}");
            }
            // Truncation error on the clear field: 1 − Σ_{k<K} λ_k |u_k(0)|²
            // = Σ_{k≥K} λ_k |u_k(0)|² ≤ Σ_{k≥K} λ_k = tr(TCC)·(1 − captured).
            let ks = truncated.kernels(z);
            let kept: f64 = ks.eigenvalues.iter().sum();
            let dropped = kept / ks.captured_energy_fraction - kept;
            for &v in truncated.compute(&clear, z).data.iter() {
                assert!(
                    v <= 1.0 + 1e-12 && 1.0 - v <= dropped + 1e-12,
                    "{v}, bound {dropped}"
                );
            }
        }
    }
}

#[test]
fn images_are_non_negative() {
    let optics = optics_no_flare(0.6);
    let grid = GridConfig {
        size: 64,
        pixel_nm: 8.0,
    };
    let mut psm = pattern_2d();
    psm.mask_type = MaskType::AttenuatedPSM {
        transmission: 0.06,
        phase_deg: 180.0,
    };
    for fill in [conventional_07, annular_05_09, dipole_150] {
        let engine =
            AerialImageEngine::new(&PupilFill::mono(100.0, fill), &optics, grid.clone(), 24)
                .unwrap();
        for mask in [pattern_2d(), psm.clone()] {
            for z in [0.0, 300.0] {
                let min = engine
                    .compute(&mask, z)
                    .data
                    .iter()
                    .cloned()
                    .fold(f64::INFINITY, f64::min);
                assert!(min >= -1e-12, "negative intensity {min}");
            }
        }
    }
}

#[test]
fn symmetric_mask_and_source_give_symmetric_image() {
    // Mask symmetric under x → −x and y → −y (about the field center, a
    // pixel boundary); dipole and annular sources share both symmetries.
    let optics = optics_no_flare(0.5);
    let grid = GridConfig {
        size: 64,
        pixel_nm: 9.375,
    };
    let mask = Mask {
        mask_type: MaskType::Binary,
        dark_field: true,
        features: vec![
            MaskFeature::Rect {
                x: 0.0,
                y: 0.0,
                w: 75.0,
                h: 300.0,
            },
            MaskFeature::Rect {
                x: -150.0,
                y: 0.0,
                w: 37.5,
                h: 150.0,
            },
            MaskFeature::Rect {
                x: 150.0,
                y: 0.0,
                w: 37.5,
                h: 150.0,
            },
        ],
    };
    for fill in [annular_05_09, dipole_150] {
        let engine =
            AerialImageEngine::new(&PupilFill::mono(100.0, fill), &optics, grid.clone(), 32)
                .unwrap();
        for z in [0.0, 250.0] {
            let img = engine.compute(&mask, z).data;
            let n = img.nrows();
            let peak = img.iter().cloned().fold(0.0, f64::max);
            for i in 0..n {
                for j in 0..n {
                    let v = img[[i, j]];
                    assert!((v - img[[i, n - 1 - j]]).abs() < 1e-10 * peak);
                    assert!((v - img[[n - 1 - i, j]]).abs() < 1e-10 * peak);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Resolution and depth of focus (partial-coherence physics)
// ---------------------------------------------------------------------------

#[test]
fn partial_coherence_resolves_beyond_coherent_cutoff() {
    // λ = 100 nm, NA = 0.5, σ = 0.7: coherent cutoff pitch λ/NA = 200 nm,
    // incoherent-limit pitch λ/(NA(1+σ)) ≈ 117.6 nm.
    let optics = optics_no_flare(0.5);
    let source = PupilFill::mono(100.0, conventional_07);
    // Pitch 160 nm: first orders at 1.25·NA/λ — outside the coherent pupil,
    // inside (1+σ)·NA/λ. Field 640 nm = 4 pitches, edges on 10 nm pixels
    // (a commensurate periodic mask images the same for any period count).
    let grid_160 = GridConfig {
        size: 64,
        pixel_nm: 10.0,
    };
    let mask_160 = line_space(160.0, 80.0, 640.0);
    let engine = AerialImageEngine::new(&source, &optics, grid_160.clone(), 32).unwrap();
    let c160 = contrast(&engine.compute(&mask_160, 0.0).data);
    assert!(c160 > 0.3, "pitch 160 nm must be resolved, contrast {c160}");
    // Coherent on-axis illumination cannot resolve it: only the DC passes.
    let coherent = AerialImageEngine::with_settings(
        &source,
        &optics,
        grid_160,
        ImagingSettings {
            source_points_per_axis: Some(1),
            ..Default::default()
        },
    )
    .unwrap();
    let c_coh = contrast(&coherent.compute(&mask_160, 0.0).data);
    assert!(c_coh < 1e-9, "coherent contrast {c_coh}");
    // Pitch 100 nm < λ/(NA(1+σ)): no two orders ever share the pupil.
    let grid_100 = GridConfig {
        size: 64,
        pixel_nm: 12.5,
    };
    let engine = AerialImageEngine::new(&source, &optics, grid_100, 32).unwrap();
    let c100 = contrast(&engine.compute(&line_space(100.0, 50.0, 800.0), 0.0).data);
    assert!(
        c100 < 1e-9,
        "pitch 100 nm must be unresolved, contrast {c100}"
    );
}

#[test]
fn dipole_two_beam_imaging_is_focus_invariant() {
    // Symmetric dipole at σ_c = λ/(2·p·NA): orders 0 and ∓1 of each pole sit
    // symmetrically about the pupil center, so their defocus phases are equal
    // and the fringe contrast is (nearly) independent of focus.
    let optics = optics_no_flare(0.5);
    let grid = GridConfig {
        size: 64,
        pixel_nm: 9.375,
    };
    let mask = line_space(150.0, 75.0, 600.0);
    let dipole = AerialImageEngine::new(
        &PupilFill::mono(100.0, dipole_150),
        &optics,
        grid.clone(),
        32,
    )
    .unwrap();
    let conventional =
        AerialImageEngine::new(&PupilFill::mono(100.0, conventional_07), &optics, grid, 32)
            .unwrap();
    let d0 = contrast(&dipole.compute(&mask, 0.0).data);
    let d300 = contrast(&dipole.compute(&mask, 300.0).data);
    let c0 = contrast(&conventional.compute(&mask, 0.0).data);
    let c300 = contrast(&conventional.compute(&mask, 300.0).data);
    assert!(d0 > 0.8, "dipole in-focus contrast {d0}");
    assert!(d300 / d0 > 0.95, "dipole keeps contrast: {d0} -> {d300}");
    assert!(
        c300 / c0 < 0.5,
        "conventional loses contrast: {c0} -> {c300}"
    );
}

#[test]
fn legacy_kernel_phase_misstates_off_axis_depth_of_focus() {
    // The legacy per-frequency phase treats the tilted zero order as if it
    // were on axis, destroying two-beam imaging: at 100 nm defocus its dipole
    // contrast collapses while the exact model keeps it.
    let optics = optics_no_flare(0.5);
    let grid = GridConfig {
        size: 64,
        pixel_nm: 9.375,
    };
    let mask = line_space(150.0, 75.0, 600.0);
    let source = PupilFill::mono(100.0, dipole_150);
    let exact = AerialImageEngine::new(&source, &optics, grid.clone(), 32).unwrap();
    let legacy = AerialImageEngine::with_settings(
        &source,
        &optics,
        grid,
        ImagingSettings {
            max_kernels: 32,
            defocus_model: DefocusModel::KernelPhase,
            ..Default::default()
        },
    )
    .unwrap();
    let e0 = contrast(&exact.compute(&mask, 0.0).data);
    let l0 = contrast(&legacy.compute(&mask, 0.0).data);
    assert!((e0 - l0).abs() < 1e-12, "identical in focus");
    let e100 = contrast(&exact.compute(&mask, 100.0).data);
    let l100 = contrast(&legacy.compute(&mask, 100.0).data);
    assert!(e100 / e0 > 0.98);
    assert!(l100 / l0 < 0.5, "legacy model contrast {l0} -> {l100}");
}

#[test]
fn defocus_is_symmetric_for_aberration_free_optics() {
    let optics = optics_no_flare(0.5);
    let grid = GridConfig {
        size: 64,
        pixel_nm: 9.375,
    };
    let engine =
        AerialImageEngine::new(&PupilFill::mono(100.0, annular_05_09), &optics, grid, 24).unwrap();
    let mask = line_space(150.0, 75.0, 600.0);
    let plus = engine.compute(&mask, 220.0).data;
    let minus = engine.compute(&mask, -220.0).data;
    assert!(max_abs_diff(&plus, &minus) < 1e-10);
}

// ---------------------------------------------------------------------------
// Defocus phase: exact vs paraxial
// ---------------------------------------------------------------------------

#[test]
fn nonparaxial_defocus_reduces_to_paraxial_at_low_na() {
    let run = |na: f64| {
        let exact = optics_no_flare(na);
        let mut paraxial = exact.clone();
        paraxial.paraxial_defocus = true;
        let pitch = 1.5 * 100.0 / na;
        let pixel = pitch / 16.0;
        let grid = GridConfig {
            size: 32,
            pixel_nm: pixel,
        };
        let mask = line_space(pitch, pitch / 2.0, 32.0 * pixel);
        let source = PupilFill::mono(100.0, conventional_07);
        let e1 = AerialImageEngine::new(&source, &exact, grid.clone(), 24).unwrap();
        let e2 = AerialImageEngine::new(&source, &paraxial, grid, 24).unwrap();
        let z = 100.0 / (na * na); // two Rayleigh depths of focus
        max_abs_diff(&e1.compute(&mask, z).data, &e2.compute(&mask, z).data)
    };
    let low = run(0.1);
    let high = run(0.8);
    assert!(low < 3e-3, "NA 0.1: exact vs paraxial differ by {low}");
    assert!(
        high > 0.03,
        "NA 0.8: exact vs paraxial should differ, got {high}"
    );
}

// ---------------------------------------------------------------------------
// Spectral imaging
// ---------------------------------------------------------------------------

#[test]
fn multiwavelength_equals_weighted_sum_of_single_line_engines() {
    // Two widely separated lines through an achromatic reflective objective.
    let optics = SchwarzschildObjective::euv_standard();
    let grid = GridConfig {
        size: 64,
        pixel_nm: 2.0,
    };
    let mask = pattern_2d_scaled(0.25);
    let lines = [(13.5, 0.7), (20.0, 0.3)];
    let comb = PupilFill {
        wavelength_nm: 13.5,
        fill: conventional_07,
        spectrum: lines.to_vec(),
    };
    let engine = AerialImageEngine::new(&comb, &optics, grid.clone(), 24).unwrap();
    let multi = engine.compute_multiwavelength(&mask, 30.0).unwrap();
    // Relative intensity of the spectrum: Σ w_i I_abs,i / Σ w_i I_clear,i with
    // I_abs,i = (relative image of engine i) × I_clear,i. Flare is affine, so
    // it commutes with this normalized weighted sum.
    let mut expected = Array2::<f64>::zeros((64, 64));
    let mut clear_sum = 0.0;
    for (wl, w) in lines {
        let single = AerialImageEngine::new(
            &PupilFill::mono(wl, conventional_07),
            &optics,
            grid.clone(),
            24,
        )
        .unwrap();
        let clear = single.clear_field_intensity(30.0);
        expected.scaled_add(w * clear, &single.compute(&mask, 30.0).data);
        clear_sum += w * clear;
    }
    expected.mapv_inplace(|v| v / clear_sum);
    let peak = expected.iter().cloned().fold(0.0, f64::max);
    assert!(max_abs_diff(&multi.data, &expected) < 1e-12 * peak);
    // The two lines really do image differently (so the test has teeth).
    let only_first = engine.compute(&mask, 30.0).data;
    assert!(max_abs_diff(&only_first, &multi.data) > 1e-3 * peak);
}

#[test]
fn narrow_band_multiwavelength_matches_polychromatic() {
    // F2 excimer: 1.1 pm FWHM at 157.63 nm, 15 nm/pm axial chromatic lens.
    let source = VuvSource::f2_laser(0.6).unwrap();
    let optics = ProjectionOptics::new(0.75).unwrap();
    let grid = GridConfig {
        size: 64,
        pixel_nm: 5.0,
    };
    let mask = line_space(160.0, 80.0, 320.0);
    let engine = AerialImageEngine::new(&source, &optics, grid, 24).unwrap();
    for z in [0.0, 80.0] {
        let poly = engine.compute_polychromatic(&mask, z, &source, &optics);
        let multi = engine.compute_multiwavelength(&mask, z).unwrap();
        let d = max_abs_diff(&poly.data, &multi.data);
        assert!(
            d < 1e-4,
            "narrow band: polychromatic vs per-λ differ by {d}"
        );
        // Chromatic focus blur is actually present (±2.75 pm × 15 nm/pm).
        let mono = engine.compute(&mask, z);
        assert!(max_abs_diff(&mono.data, &poly.data) > 1e-4);
    }
}

fn pattern_2d_scaled(s: f64) -> Mask {
    let mut m = pattern_2d();
    for f in &mut m.features {
        if let MaskFeature::Rect { x, y, w, h } = f {
            *x *= s;
            *y *= s;
            *w *= s;
            *h *= s;
        }
    }
    m
}

// ---------------------------------------------------------------------------
// Contract C1: kernels(z) reproduce compute(z)
// ---------------------------------------------------------------------------

#[test]
fn kernels_reproduce_compute() {
    let optics = optics_no_flare(0.6);
    let grid = GridConfig {
        size: 64,
        pixel_nm: 8.0,
    };
    let mask = pattern_2d();
    for model in [DefocusModel::Exact, DefocusModel::KernelPhase] {
        let settings = ImagingSettings {
            max_kernels: 12,
            defocus_model: model,
            ..Default::default()
        };
        let engine = AerialImageEngine::with_settings(
            &PupilFill::mono(100.0, annular_05_09),
            &optics,
            grid.clone(),
            settings,
        )
        .unwrap();
        let fft = Fft2D::new();
        let spectrum = mask.spectrum(engine.grid(), &fft);
        for z in [0.0, 120.0] {
            let ks = engine.kernels(z);
            assert_eq!(ks.defocus_nm, z);
            assert_eq!(ks.wavelength_nm, 100.0);
            assert!(
                ks.captured_energy_fraction > 0.5 && ks.captured_energy_fraction <= 1.0 + 1e-12
            );
            let mut manual = Array2::<f64>::zeros((64, 64));
            for (lambda, k) in ks.eigenvalues.iter().zip(&ks.kernels) {
                let norm: f64 = k.iter().map(|v| v.norm_sqr()).sum();
                assert!((norm - 1.0).abs() < 1e-10, "kernels have unit norm");
                let mut field = k * &spectrum;
                fft.inverse(&mut field);
                manual.zip_mut_with(&field, |m, e| *m += lambda * e.norm_sqr());
            }
            let img = engine.compute(&mask, z).data;
            assert!(max_abs_diff(&img, &manual) < 1e-12);
        }
    }
}

#[test]
fn through_focus_batch_matches_single_calls() {
    let optics = ProjectionOptics::new(0.6).unwrap();
    let grid = GridConfig {
        size: 32,
        pixel_nm: 12.5,
    };
    let engine =
        AerialImageEngine::new(&PupilFill::mono(100.0, dipole_150), &optics, grid, 16).unwrap();
    let mask = pattern_2d_scaled(1.0);
    let zs = [-200.0, 0.0, 50.0, 50.0, 400.0];
    let batch = engine.compute_through_focus(&mask, &zs);
    assert_eq!(batch.len(), zs.len());
    for (img, &z) in batch.iter().zip(&zs) {
        assert_eq!(img.data, engine.compute(&mask, z).data);
    }
}

#[test]
fn concurrent_use_is_deterministic() {
    // Threads racing on new focus planes (including duplicates) must agree
    // with a sequential evaluation; nested Rayon use must not deadlock.
    let optics = ProjectionOptics::new(0.6).unwrap();
    let grid = GridConfig {
        size: 32,
        pixel_nm: 12.5,
    };
    let engine =
        AerialImageEngine::new(&PupilFill::mono(100.0, annular_05_09), &optics, grid, 16).unwrap();
    let mask = pattern_2d();
    let zs: Vec<f64> = [30.0, 60.0, 30.0, 90.0, 60.0, 30.0].to_vec();
    let parallel: Vec<Array2<f64>> = std::thread::scope(|s| {
        let handles: Vec<_> = zs
            .iter()
            .map(|&z| {
                let (engine, mask) = (&engine, &mask);
                s.spawn(move || engine.compute(mask, z).data)
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let batch = highuvlith_core::process::batch_defocus(&engine, &mask, &zs);
    for ((z, par), (bz, b)) in zs.iter().zip(&parallel).zip(&batch) {
        assert_eq!(z, bz);
        let seq = engine.compute(&mask, *z).data;
        assert_eq!(par, &seq);
        assert_eq!(b, &seq);
    }
}

// ---------------------------------------------------------------------------
// Vector plumbing (physics provided by optics::vector)
// ---------------------------------------------------------------------------

fn vector_engine(na: f64, polarization: IlluminationPolarization) -> AerialImageEngine {
    let settings = ImagingSettings {
        max_kernels: 1000,
        kernel_energy_fraction: 1.0,
        source_points_per_axis: Some(9),
        imaging_model: ImagingModel::Vector(VectorSettings {
            polarization,
            ..VectorSettings::default()
        }),
        ..Default::default()
    };
    AerialImageEngine::with_settings(
        &PupilFill::mono(100.0, conventional_07),
        &optics_no_flare(na),
        GridConfig {
            size: 32,
            pixel_nm: 8.0 * 0.5 / na,
        },
        settings,
    )
    .unwrap()
}

#[test]
fn vector_unpolarized_is_mean_of_orthogonal_states() {
    // Unpolarized light is the incoherent mean of any two orthogonal states,
    // whatever the (linear) vector physics.
    let mask = pattern_2d_scaled(0.25);
    let un = vector_engine(0.8, IlluminationPolarization::Unpolarized);
    let x = vector_engine(0.8, IlluminationPolarization::X);
    let y = vector_engine(0.8, IlluminationPolarization::Y);
    for z in [0.0, 40.0] {
        let iu = un.compute(&mask, z).data;
        let mean = (&x.compute(&mask, z).data + &y.compute(&mask, z).data) * 0.5;
        let peak = iu.iter().cloned().fold(0.0, f64::max);
        assert!(max_abs_diff(&iu, &mean) < 1e-10 * peak);
    }
}

#[test]
fn vector_imaging_approaches_scalar_at_low_na() {
    let na = 0.1;
    let mask = pattern_2d_scaled(0.5 / na);
    let vector = vector_engine(na, IlluminationPolarization::Unpolarized);
    let scalar = AerialImageEngine::with_settings(
        &PupilFill::mono(100.0, conventional_07),
        &optics_no_flare(na),
        vector.grid().clone(),
        ImagingSettings {
            max_kernels: 1000,
            kernel_energy_fraction: 1.0,
            source_points_per_axis: Some(9),
            ..Default::default()
        },
    )
    .unwrap();
    let a = vector.compute(&mask, 0.0).data;
    let b = scalar.compute(&mask, 0.0).data;
    // O(NA²) polarization / obliquity corrections at NA = 0.1.
    assert!(max_abs_diff(&a, &b) < 0.02);
}
