//! Accuracy of the engine's DEFAULT (adaptive) source sampling against
//! independent fine-quadrature references, in the two regimes where a
//! coarse point set shows up as artifacts: partially coherent imaging near
//! the k₁ onset (only a sliver of the source steers the first orders into
//! the pupil) and near-coherent fills (a diffraction order crossing the
//! pupil edge sweeps over the whole small source).
//!
//! Reference values: scalar Abbe imaging of the same pixelated 1:1 grating
//! (two periods, 64 px per period, numpy FFT of the pixel mask), uniform
//! circular pupil in focus, zeroth-order flare, contrast over the pixels;
//! the source integral is a 2001 × 2001 midpoint grid over the source's
//! bounding square (a 1201² grid agrees to ≤ 3e-4). Script:
//! `notes/A3.md` (WP-A3, `a3_reference.py`), independent of the engine.
//!
//! Before WP-A3 the default sampling (an axis-aligned grid, one point per
//! cell centre) gave a spurious +0.013 contrast blip at σ = 0.74 in the KrF
//! sweep (reference 0.0035, then 0 at σ = 0.75–0.76) and errors up to 0.14
//! with flat stair steps in the near-coherent pitch sweep.

use highuvlith_core::aerial::{AerialImageEngine, ImagingSettings};
use highuvlith_core::mask::Mask;
use highuvlith_core::metrics::image_contrast;
use highuvlith_core::optics::ProjectionOptics;
use highuvlith_core::source::{IlluminationShape, VuvSource};
use highuvlith_core::types::GridConfig;

/// Contrast of a 1:1 L/S grating of `pitch` on a two-period field
/// (128 px, 64 px per period) at the default source sampling.
fn ls_contrast(source: &VuvSource, optics: &ProjectionOptics, pitch: f64) -> f64 {
    let mask = Mask::line_space(pitch / 2.0, pitch).unwrap();
    let grid = GridConfig::new(128, pitch / 64.0).unwrap();
    let engine =
        AerialImageEngine::with_settings(source, optics, grid, ImagingSettings::default()).unwrap();
    image_contrast(&engine.compute(&mask, 0.0).data)
}

/// Assert `got` tracks `reference` within `tol`, never decreases where the
/// reference increases, and report the worst deviation.
fn check_sweep(label: &str, xs: &[f64], got: &[f64], reference: &[f64], tol: f64) {
    let mut worst = (0.0f64, 0.0);
    for ((&x, &g), &r) in xs.iter().zip(got).zip(reference) {
        if (g - r).abs() > worst.0 {
            worst = ((g - r).abs(), x);
        }
    }
    println!(
        "{label}: max |engine − reference| = {:.5} at {}",
        worst.0, worst.1
    );
    for (k, ((&x, &g), &r)) in xs.iter().zip(got).zip(reference).enumerate() {
        assert!(
            (g - r).abs() <= tol,
            "{label} at {x}: engine {g:.5} vs reference {r:.5} (tol {tol})"
        );
        if k > 0 {
            assert!(
                g >= got[k - 1] - 1e-9,
                "{label}: contrast decreases from {:.5} to {g:.5} at {x}",
                got[k - 1]
            );
        }
    }
}

/// KrF (248.3 nm), NA 0.8, 90/180 nm L/S (k₁ = 0.29), conventional σ swept
/// over 0.60–0.85: the first orders enter the pupil at σ = λ/(p·NA) − 1 =
/// 0.724, so the contrast must stay 0 below and rise smoothly above.
#[test]
fn krf_sigma_sweep_default_sampling_tracks_reference() {
    const REF: [f64; 26] = [
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.00080, 0.00350, 0.00714,
        0.01140, 0.01610, 0.02114, 0.02644, 0.03192, 0.03754, 0.04326, 0.04901, 0.05485, 0.06070,
    ];
    let optics = ProjectionOptics::new(0.80).unwrap();
    let sigmas: Vec<f64> = (0..26).map(|k| 0.60 + 0.01 * k as f64).collect();
    let got: Vec<f64> = sigmas
        .iter()
        .map(|&s| ls_contrast(&VuvSource::krf_laser(s).unwrap(), &optics, 180.0))
        .collect();
    check_sweep("KrF σ sweep", &sigmas, &got, &REF, 0.004);
    // Below the onset no source point may steer a first order into the pupil.
    for (s, c) in sigmas.iter().zip(&got).take(12) {
        assert!(*c < 1e-12, "σ {s}: contrast {c} below the k₁ onset");
    }
}

/// A source with an explicit pupil fill at 13.5 nm.
fn euv_fill(illumination: IlluminationShape) -> VuvSource {
    VuvSource {
        wavelength_nm: 13.5,
        illumination,
        ..VuvSource::f2_laser(0.5).unwrap()
    }
}

/// Near-coherent fills at 13.5 nm, NA 0.33, no flare: pitch swept over
/// 36–42 nm across the coherent cutoff λ/(NA(1 + σ)), where the first
/// order's pupil edge sweeps over the whole (small) source.
#[test]
fn near_coherent_pitch_sweeps_default_sampling_track_reference() {
    const GAUSS_REF: [f64; 25] = [
        0.00755, 0.01203, 0.01861, 0.02798, 0.04096, 0.05841, 0.08109, 0.10986, 0.14545, 0.18806,
        0.23767, 0.29389, 0.35594, 0.42298, 0.49358, 0.56631, 0.63990, 0.71259, 0.78333, 0.85071,
        0.90397, 0.91755, 0.92669, 0.93422, 0.94052,
    ];
    const DISK_REF: [f64; 25] = [
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.00452, 0.08625, 0.20506,
        0.33667, 0.46995, 0.59883, 0.72004, 0.83206, 0.91057, 0.92620, 0.93832, 0.94785, 0.95540,
    ];
    let mut optics = ProjectionOptics::new(0.33).unwrap();
    optics.flare_fraction = 0.0;
    let pitches: Vec<f64> = (0..25).map(|k| 36.0 + 0.25 * k as f64).collect();
    let sweep = |src: &VuvSource| -> Vec<f64> {
        pitches
            .iter()
            .map(|&p| ls_contrast(src, &optics, p))
            .collect()
    };
    // Gaussian fill σ_g = 0.05 (the HHG / FEL coherent-beam fills).
    let gauss = euv_fill(IlluminationShape::CoherentGaussian { sigma: 0.05 });
    check_sweep(
        "Gaussian σ_g 0.05",
        &pitches,
        &sweep(&gauss),
        &GAUSS_REF,
        0.025,
    );
    // Hard-edged disk σ = 0.05.
    let disk = euv_fill(IlluminationShape::Conventional { sigma: 0.05 });
    check_sweep("disk σ 0.05", &pitches, &sweep(&disk), &DISK_REF, 0.025);
}
