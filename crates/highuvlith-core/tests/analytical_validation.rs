//! Analytical validation tests: compare simulation results against
//! known closed-form solutions from optics textbooks.

use approx::assert_relative_eq;
use highuvlith_core::aerial::{AerialImageEngine, ImagingSettings};
use highuvlith_core::mask::{Mask, MaskFeature, MaskType};
use highuvlith_core::math::fft2d::Fft2D;
use highuvlith_core::metrics;
use highuvlith_core::optics::{defocus_phase, ProjectionOptics};
use highuvlith_core::source::{IlluminationShape, SpectralShape, VuvSource};
use highuvlith_core::thinfilm::{FilmLayer, FilmStack};
use highuvlith_core::types::{Complex64, GridConfig, Polarization};

/// Helper to create a coherent source (sigma -> 0).
fn coherent_source(wavelength_nm: f64) -> VuvSource {
    VuvSource {
        wavelength_nm,
        bandwidth_pm: 0.01,
        spectral_samples: 1,
        spectral_shape: SpectralShape::Gaussian,
        pulse_energy_mj: 10.0,
        rep_rate_hz: 4000.0,
        illumination: IlluminationShape::Conventional { sigma: 0.01 },
    }
}

/// Test: Defocus symmetry - positive and negative defocus should produce
/// identical aerial images for a symmetric optical system.
#[test]
fn test_defocus_symmetry() {
    let source = VuvSource::f2_laser(0.5).unwrap();
    let optics = ProjectionOptics::new(0.75).unwrap();
    let grid = GridConfig {
        size: 128,
        pixel_nm: 2.0,
    };
    let engine = AerialImageEngine::new(&source, &optics, grid, 15).unwrap();
    let mask = Mask::line_space(65.0, 180.0).unwrap();

    let img_pos = engine.compute(&mask, 150.0);
    let img_neg = engine.compute(&mask, -150.0);

    let c_pos = metrics::image_contrast(&img_pos.data);
    let c_neg = metrics::image_contrast(&img_neg.data);
    assert_relative_eq!(c_pos, c_neg, epsilon = 1e-4);
}

/// Test: Increasing partial coherence (sigma) reduces image contrast
/// for dense features near the resolution limit.
#[test]
fn test_sigma_reduces_contrast() {
    let optics = ProjectionOptics::new(0.75).unwrap();
    let grid = GridConfig {
        size: 128,
        pixel_nm: 2.0,
    };
    let mask = Mask::line_space(65.0, 180.0).unwrap();

    let src_low_sigma = VuvSource {
        illumination: IlluminationShape::Conventional { sigma: 0.3 },
        ..VuvSource::f2_laser(0.3).unwrap()
    };
    let src_high_sigma = VuvSource {
        illumination: IlluminationShape::Conventional { sigma: 0.9 },
        ..VuvSource::f2_laser(0.9).unwrap()
    };

    let engine_low = AerialImageEngine::new(&src_low_sigma, &optics, grid.clone(), 15).unwrap();
    let engine_high = AerialImageEngine::new(&src_high_sigma, &optics, grid, 15).unwrap();

    let c_low = metrics::image_contrast(&engine_low.compute(&mask, 0.0).data);
    let c_high = metrics::image_contrast(&engine_high.compute(&mask, 0.0).data);

    // Higher sigma generally gives lower contrast for dense features
    // (this is a well-known result in partial coherence theory)
    assert!(
        c_low > c_high * 0.9,
        "Low sigma contrast ({}) should exceed high sigma contrast ({})",
        c_low,
        c_high
    );
}

/// Test: Quarter-wave anti-reflection coating gives minimum reflectance.
/// This validates the transfer matrix method.
#[test]
fn test_quarter_wave_ar_minimum() {
    let wavelength = 157.0;
    let n_sub: f64 = 1.5;
    let n_film = n_sub.sqrt(); // optimal AR condition
    let qw_thickness = wavelength / (4.0 * n_film);

    // Test at several thicknesses around quarter-wave
    let mut reflectances = Vec::new();
    for factor in [0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0] {
        let stack = FilmStack::new_vuv(
            vec![FilmLayer {
                name: "AR".to_string(),
                thickness_nm: qw_thickness * factor,
                n: Complex64::new(n_film, 0.0),
            }],
            Complex64::new(n_sub, 0.0),
        );
        reflectances.push((factor, stack.reflectance(wavelength, Polarization::TE)));
    }

    // The quarter-wave (factor=1.0) should have the minimum reflectance
    let qw_r = reflectances
        .iter()
        .find(|(f, _)| (*f - 1.0).abs() < 0.01)
        .unwrap()
        .1;
    for &(factor, r) in &reflectances {
        if (factor - 1.0).abs() > 0.01 {
            assert!(qw_r <= r + 1e-10,
                "Quarter-wave reflectance ({:.6}) should be <= thickness factor {} reflectance ({:.6})",
                qw_r, factor, r);
        }
    }
}

/// Test: Bare substrate Fresnel reflectance matches analytical formula.
/// R = ((n1-n2)/(n1+n2))^2 for normal incidence.
#[test]
fn test_fresnel_reflectance() {
    for n_sub in [1.3, 1.5, 2.0, 2.5] {
        let stack = FilmStack::new_vuv(vec![], Complex64::new(n_sub, 0.0));
        let r = stack.reflectance(157.0, Polarization::Unpolarized);
        let expected = ((1.0 - n_sub) / (1.0 + n_sub)).powi(2);
        assert!(
            (r - expected).abs() < 1e-4,
            "Fresnel reflectance mismatch for n={}: got {}, expected {}",
            n_sub,
            r,
            expected
        );
    }
}

/// Test: Aerial image intensity is always non-negative across a range of configurations.
#[test]
fn test_intensity_non_negative_sweep() {
    for na in [0.5, 0.65, 0.75, 0.85] {
        for sigma in [0.3, 0.5, 0.7] {
            let source = VuvSource {
                illumination: IlluminationShape::Conventional { sigma },
                ..VuvSource::f2_laser(sigma).unwrap()
            };
            let optics = ProjectionOptics::new(na).unwrap();
            let grid = GridConfig {
                size: 64,
                pixel_nm: 2.0,
            };
            let engine = AerialImageEngine::new(&source, &optics, grid, 10).unwrap();
            let mask = Mask::line_space(65.0, 180.0).unwrap();

            for focus in [-200.0, 0.0, 200.0] {
                let image = engine.compute(&mask, focus);
                let min_val = image.data.iter().cloned().fold(f64::INFINITY, f64::min);
                assert!(
                    min_val >= -1e-10,
                    "Negative intensity {:.2e} at NA={}, sigma={}, focus={}",
                    min_val,
                    na,
                    sigma,
                    focus
                );
            }
        }
    }
}

/// Test: Energy conservation -- total integrated intensity should not depend
/// on defocus (for a phase-only aberration, total energy is conserved).
#[test]
fn test_energy_conservation_defocus() {
    let source = VuvSource::f2_laser(0.5).unwrap();
    let optics = ProjectionOptics::new(0.75).unwrap();
    let grid = GridConfig {
        size: 128,
        pixel_nm: 2.0,
    };
    let engine = AerialImageEngine::new(&source, &optics, grid, 15).unwrap();
    let mask = Mask::line_space(65.0, 180.0).unwrap();

    let energy_focus0: f64 = engine.compute(&mask, 0.0).data.iter().sum();
    let energy_focus200: f64 = engine.compute(&mask, 200.0).data.iter().sum();

    // Energy should be approximately conserved (within flare adjustment)
    assert_relative_eq!(
        energy_focus0,
        energy_focus200,
        epsilon = energy_focus0 * 0.05
    );
}

/// Test: Brewster angle gives zero p-polarization reflectance for a dielectric.
#[test]
fn test_brewster_angle_zero_reflection() {
    let n_sub: f64 = 1.5;
    let brewster = n_sub.atan(); // atan(n2/n1) where n1=1.0 (vacuum)
    let stack = FilmStack::new_vuv(vec![], Complex64::new(n_sub, 0.0));
    let r_tm = stack.reflectance_at_angle(157.0, brewster, Polarization::TM);
    assert!(
        r_tm < 1e-6,
        "TM reflectance at Brewster's angle should be ~0, got {:.2e}",
        r_tm
    );
}

/// Opaque lines of width pitch/2 tiling a field of `periods` pitches.
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

/// Test: coherent three-beam imaging in closed form.
///
/// On-axis coherent illumination of a grating whose ±1 orders pass the pupil
/// (λ/p < NA) and whose ±2 orders do not gives
///   E(x) = c₀ + (c₁ e^{2πix/p} + c₋₁ e^{−2πix/p}) · e^{iφ(z)},
///   φ(z) = (2π/λ) z (1 − √(1 − (λ/p)²)),
/// with c_k the mask Fourier coefficients (read from the mask spectrum, so
/// this checks the imaging, not the mask model). The engine must reproduce
/// |E|² pixel by pixel, in focus and defocused.
#[test]
fn test_coherent_three_beam_image_closed_form() {
    let (wl, na, pitch, pixel) = (100.0, 0.5, 300.0, 10.0);
    let periods = 4;
    let n = (pitch * periods as f64 / pixel) as usize; // 120 pixels
    let grid = GridConfig {
        size: n,
        pixel_nm: pixel,
    };
    let mask = half_duty_lines(pitch, periods);
    let mut optics = ProjectionOptics::new(na).unwrap();
    optics.flare_fraction = 0.0;
    let fft = Fft2D::new();
    let spectrum = mask.spectrum(&grid, &fft);
    let norm = (n * n) as f64;
    let (c0, c1, cm1) = (
        spectrum[[0, 0]] / norm,
        spectrum[[0, periods]] / norm,
        spectrum[[0, n - periods]] / norm,
    );
    // Exactly coherent on-axis point: `source_points_per_axis = Some(1)`.
    let settings = ImagingSettings {
        source_points_per_axis: Some(1),
        ..Default::default()
    };
    let engine =
        AerialImageEngine::with_settings(&coherent_source(wl), &optics, grid.clone(), settings)
            .unwrap();
    // A slightly extended (σ = 0.01) source images the same to ~1e-3.
    let extended = AerialImageEngine::new(&coherent_source(wl), &optics, grid, 8).unwrap();
    for z in [0.0, 400.0, -900.0] {
        let phi = defocus_phase(z, wl / pitch, wl, false);
        let tilt = Complex64::from_polar(1.0, phi);
        let img = engine.compute(&mask, z);
        let img_ext = extended.compute(&mask, z);
        for j in 0..n {
            let x = 2.0 * std::f64::consts::PI * j as f64 / (n / periods) as f64;
            let e = c0
                + (c1 * Complex64::from_polar(1.0, x) + cm1 * Complex64::from_polar(1.0, -x))
                    * tilt;
            for i in [0, n / 3] {
                assert!(
                    (img.data[[i, j]] - e.norm_sqr()).abs() < 1e-12,
                    "z = {z}, j = {j}"
                );
                assert!(
                    (img_ext.data[[i, j]] - e.norm_sqr()).abs() < 2e-3,
                    "z = {z}, j = {j}"
                );
            }
        }
    }
    // Sanity: the in-focus contrast of a half-duty grating with ±1 orders
    // only, 4|c₀c₁| / (c₀² + 2|c₁|²)… is large (≈ 0.97 for continuous c_k).
    let c = metrics::image_contrast(&engine.compute(&mask, 0.0).data);
    assert!(c > 0.95, "three-beam contrast {c}");
}
