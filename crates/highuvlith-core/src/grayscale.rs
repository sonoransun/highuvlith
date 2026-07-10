//! Grayscale lithography: continuous dose modulation into continuous 3D
//! resist topography.
//!
//! Unlike binary patterning, grayscale exposure varies the local dose across
//! the field so that a positive resist clears to a *depth* that follows the
//! dose. The mapping from dose to removed depth is the resist's contrast
//! (H–D) curve; over the log-linear regime it is fixed by two doses — the
//! threshold dose `D_th` (below which nothing clears) and the clearing dose
//! `D_clear` (at/above which the resist clears to the substrate). A single
//! exposure with a spatially varying transmittance mask then sculpts blazed
//! gratings, microlens arrays, and staircase phase elements.
//!
//! This module provides the fast 2.5D height map: each pixel's remaining
//! resist height is a closed-form function of the local dose through the
//! contrast curve, with no standing-wave or lateral-development physics.
//! It builds continuous-transmittance masks that feed
//! [`crate::aerial::AerialImageEngine::compute_from_transmittance`], and the
//! inverse map ([`GrayscaleMap::from_target_height`]) that turns a desired
//! surface relief into the transmittance needed to print it.
//!
//! # Key equations
//!
//! ```text
//!   contrast slope  γ = 1 / log10(D_clear / D_th)
//!   removed depth   f(D) = clamp(γ · log10(D / D_th), 0, 1)   (fraction)
//!   height          h(D) = thickness · (1 − f(D))
//!   inverse         D(f) = D_th · 10^(f / γ)
//! ```
//!
//! # Model status
//!
//! Analytical 2.5D only: the contrast curve is depth-independent, so the
//! remaining height is a per-pixel function of the surface dose with no
//! standing-wave modulation, no PEB diffusion, and no sidewall angle. The
//! full standing-wave- and diffusion-aware topography goes through the
//! separate volumetric development path in [`crate::volumetric`]; use this
//! module for first-order design and fast mask synthesis.
//!
//! # References
//!
//! - Dill et al., IEEE Trans. Electron Devices (1975) — exposure kinetics.
//! - C. M. Waits et al., "Investigation of gray-scale technology for large
//!   area 3D silicon MEMS structures," J. Micromech. Microeng. (2003).

use ndarray::Array2;
use serde::{Deserialize, Serialize};

use crate::error::{LithographyError, Result};
use crate::mask::{Mask, MaskFeature, MaskType};
use crate::types::{Complex64, Grid2D};

/// Positive-resist log-linear contrast (H–D) curve.
///
/// Fixed by two doses: `D_th` (threshold, below which nothing clears) and
/// `D_clear` (at/above which the resist clears to the substrate). The
/// contrast slope `γ = 1 / log10(D_clear / D_th)` maps dose to the fraction
/// of the film thickness removed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ContrastCurve {
    /// Contrast slope γ = 1 / log10(D_clear / D_th).
    pub gamma: f64,
    /// Threshold dose in mJ/cm² (below this, no resist clears).
    pub dose_threshold_mj_cm2: f64,
    /// Clearing dose in mJ/cm² (at/above this, resist clears fully).
    pub dose_clear_mj_cm2: f64,
}

impl ContrastCurve {
    /// Build a contrast curve from the threshold and clearing doses.
    ///
    /// Requires `0 < d_th < d_clear`. The contrast slope is derived as
    /// `gamma = 1 / log10(d_clear / d_th)`.
    pub fn new(d_th: f64, d_clear: f64) -> Result<Self> {
        if d_th.is_nan() || d_th <= 0.0 {
            return Err(LithographyError::InvalidParameter {
                name: "d_th",
                value: d_th,
                reason: "threshold dose must be positive",
            });
        }
        if d_clear.is_nan() || d_clear <= d_th {
            return Err(LithographyError::InvalidParameter {
                name: "d_clear",
                value: d_clear,
                reason: "clearing dose must exceed the threshold dose",
            });
        }
        let gamma = 1.0 / (d_clear / d_th).log10();
        Ok(Self {
            gamma,
            dose_threshold_mj_cm2: d_th,
            dose_clear_mj_cm2: d_clear,
        })
    }

    /// Fraction of the film thickness removed at the given dose, in [0, 1].
    ///
    /// Zero at or below the threshold dose, one at or above the clearing
    /// dose, and `γ · log10(dose / D_th)` in between.
    pub fn removed_depth_fraction(&self, dose: f64) -> f64 {
        if dose <= 0.0 {
            return 0.0;
        }
        (self.gamma * (dose / self.dose_threshold_mj_cm2).log10()).clamp(0.0, 1.0)
    }

    /// Removed depth in nm at the given dose for a film of `thickness_nm`.
    pub fn depth_nm(&self, dose: f64, thickness_nm: f64) -> f64 {
        thickness_nm * self.removed_depth_fraction(dose)
    }

    /// Dose (mJ/cm²) required to remove `depth_nm` from a `thickness_nm` film.
    ///
    /// Exact inverse of [`Self::depth_nm`] over the log-linear regime:
    /// `D = D_th · 10^(f / γ)` where `f = clamp(depth / thickness, 0, 1)`.
    /// A required fraction of 0 maps to the threshold dose `D_th` (any dose
    /// at or below `D_th` leaves the full thickness), and a fraction of 1
    /// maps to the clearing dose `D_clear`.
    pub fn dose_for_depth(&self, depth_nm: f64, thickness_nm: f64) -> f64 {
        let fraction = (depth_nm / thickness_nm).clamp(0.0, 1.0);
        self.dose_threshold_mj_cm2 * 10.0_f64.powf(fraction / self.gamma)
    }
}

/// A continuous-transmittance grayscale mask: intensity transmittance `T` in
/// [0, 1] at each grid pixel.
#[derive(Debug, Clone)]
pub struct GrayscaleMap {
    /// Intensity transmittance `T` in [0, 1] at each `(y, x)` grid pixel.
    pub transmittance: Array2<f64>,
}

impl GrayscaleMap {
    /// Complex amplitude transmittance map (amplitude `sqrt(T)`, zero phase)
    /// suitable for
    /// [`crate::aerial::AerialImageEngine::compute_from_transmittance`].
    pub fn to_complex(&self) -> Array2<Complex64> {
        self.transmittance
            .mapv(|t| Complex64::new(t.clamp(0.0, 1.0).sqrt(), 0.0))
    }

    /// Synthesize the transmittance that prints a target surface relief with a
    /// single exposure of `exposure_dose_mj_cm2`.
    ///
    /// For each pixel the required removed depth is
    /// `thickness − target_height`; the dose to reach it is
    /// [`ContrastCurve::dose_for_depth`]; and the transmittance is that dose
    /// divided by the exposure dose. Returns an error if any pixel needs more
    /// dose than the exposure delivers (i.e. `T > 1`), reporting the maximum
    /// required dose.
    pub fn from_target_height(
        target_height_nm: &Array2<f64>,
        thickness_nm: f64,
        curve: &ContrastCurve,
        exposure_dose_mj_cm2: f64,
    ) -> Result<Self> {
        if thickness_nm.is_nan() || thickness_nm <= 0.0 {
            return Err(LithographyError::InvalidParameter {
                name: "thickness_nm",
                value: thickness_nm,
                reason: "must be positive",
            });
        }
        if exposure_dose_mj_cm2.is_nan() || exposure_dose_mj_cm2 <= 0.0 {
            return Err(LithographyError::InvalidParameter {
                name: "exposure_dose_mj_cm2",
                value: exposure_dose_mj_cm2,
                reason: "must be positive",
            });
        }

        let mut max_required = 0.0_f64;
        let transmittance = target_height_nm.mapv(|h| {
            let depth = thickness_nm - h;
            let required = curve.dose_for_depth(depth, thickness_nm);
            if required > max_required {
                max_required = required;
            }
            required / exposure_dose_mj_cm2
        });

        if max_required > exposure_dose_mj_cm2 {
            return Err(LithographyError::NumericalError(format!(
                "exposure dose {exposure_dose_mj_cm2} mJ/cm^2 is too low to print the target: \
                 a pixel requires up to {max_required} mJ/cm^2 (transmittance would exceed 1); \
                 raise the exposure dose to at least this value"
            )));
        }

        Ok(Self { transmittance })
    }
}

/// Target surface height (nm) for a blazed (sawtooth) grating.
///
/// The relief runs along x: within each `period_px`-wide period the height
/// ramps from `thickness_nm` down toward `thickness_nm − depth_nm`, giving a
/// blaze that steers a diffraction order. The height at pixel `x` is
/// `thickness − depth · frac(x / period)` with `frac(x/period) = (x mod
/// period) / period`, so the maximum height is `thickness` (at the start of
/// each period) and the minimum is `thickness − depth·(period−1)/period`.
pub fn blazed_grating(n: usize, period_px: usize, depth_nm: f64, thickness_nm: f64) -> Array2<f64> {
    let period = period_px.max(1);
    let mut height = Array2::from_elem((n, n), thickness_nm);
    for j in 0..n {
        let frac = (j % period) as f64 / period as f64;
        let h = thickness_nm - depth_nm * frac;
        for i in 0..n {
            height[[i, j]] = h;
        }
    }
    height
}

/// Target surface height (nm) for a square array of parabolic microlenses.
///
/// The field is tiled into `pitch_px × pitch_px` cells; within each cell the
/// height follows a downward paraboloid `thickness − sag·(1 − p)`, where
/// `p = clamp(1 − (r/R)², 0, 1)` is the (clamped) lens profile, `r` is the
/// distance from the cell center, and `R = pitch_px/2`. Each cell is a convex
/// bump of height `sag_nm` (peak `thickness_nm` at the center) sitting on a
/// base plane at `thickness_nm − sag_nm`, so every height lies in
/// `[thickness − sag, thickness]`.
pub fn microlens_array(n: usize, pitch_px: usize, sag_nm: f64, thickness_nm: f64) -> Array2<f64> {
    let pitch = pitch_px.max(1);
    let radius = pitch as f64 / 2.0;
    let mut height = Array2::from_elem((n, n), thickness_nm - sag_nm);
    for i in 0..n {
        let dy = (i % pitch) as f64 + 0.5 - radius;
        for j in 0..n {
            let dx = (j % pitch) as f64 + 0.5 - radius;
            let r2_norm = (dx * dx + dy * dy) / (radius * radius);
            let profile = (1.0 - r2_norm).clamp(0.0, 1.0);
            height[[i, j]] = thickness_nm - sag_nm * (1.0 - profile);
        }
    }
    height
}

/// Fast 2.5D height map: per-pixel remaining resist height from an aerial
/// image and a single exposure dose.
///
/// Each pixel's remaining height is
/// `thickness − curve.depth_nm(dose · I, thickness)`, where `I` is the local
/// (normalized) aerial intensity. This is the depth-independent path; the
/// standing-wave- and diffusion-aware topography goes through
/// [`crate::volumetric`].
pub fn height_map(
    aerial: &Grid2D<f64>,
    dose_mj_cm2: f64,
    curve: &ContrastCurve,
    thickness_nm: f64,
) -> Grid2D<f64> {
    let data = aerial
        .data
        .mapv(|intensity| thickness_nm - curve.depth_nm(dose_mj_cm2 * intensity, thickness_nm));
    Grid2D {
        data,
        x_min_nm: aerial.x_min_nm,
        x_max_nm: aerial.x_max_nm,
        y_min_nm: aerial.y_min_nm,
        y_max_nm: aerial.y_max_nm,
    }
}

/// Build a staircase mask of adjacent [`MaskFeature::GrayRect`] steps, one per
/// transmittance level, tiled left-to-right along x and centered on the field.
///
/// A convenience helper for tests and examples: `n_levels` steps each
/// `level_width_nm` wide and `field_h_nm` tall carry the transmittances in
/// `transmittances` (using at most `n_levels` of them). `GrayRect` ignores the
/// mask type and dark-field flag, so the returned [`Mask`] carries its
/// transmission per step directly.
pub fn staircase_mask(
    n_levels: usize,
    level_width_nm: f64,
    transmittances: &[f64],
    field_h_nm: f64,
) -> Mask {
    let count = n_levels.min(transmittances.len());
    let total_width = count as f64 * level_width_nm;
    let start = -total_width / 2.0;
    let mut features = Vec::with_capacity(count);
    for (i, &t) in transmittances.iter().take(count).enumerate() {
        features.push(MaskFeature::GrayRect {
            x: start + (i as f64 + 0.5) * level_width_nm,
            y: 0.0,
            w: level_width_nm,
            h: field_h_nm,
            transmittance: t,
        });
    }
    Mask {
        mask_type: MaskType::Binary,
        features,
        dark_field: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_gamma_fixture() {
        // D_th = 10, D_clear = 100 → log10(10) = 1 → gamma = 1.
        let curve = ContrastCurve::new(10.0, 100.0).unwrap();
        assert_relative_eq!(curve.gamma, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_contrast_curve_round_trip() {
        let curve = ContrastCurve::new(10.0, 100.0).unwrap();
        let thickness = 200.0;
        // Doses strictly inside (D_th, D_clear) stay in the open interval,
        // so depth→dose is exact with no clamping.
        for &dose in &[15.0, 25.0, 50.0, 75.0, 99.0] {
            let depth = curve.depth_nm(dose, thickness);
            let recovered = curve.dose_for_depth(depth, thickness);
            assert_relative_eq!(recovered, dose, epsilon = 1e-9);
        }
    }

    #[test]
    fn test_removed_fraction_bounds() {
        let curve = ContrastCurve::new(10.0, 100.0).unwrap();
        // Below threshold: nothing removed.
        assert_relative_eq!(curve.removed_depth_fraction(5.0), 0.0, epsilon = 1e-12);
        assert_relative_eq!(curve.removed_depth_fraction(10.0), 0.0, epsilon = 1e-12);
        // At/above clearing: fully removed.
        assert_relative_eq!(curve.removed_depth_fraction(100.0), 1.0, epsilon = 1e-12);
        assert_relative_eq!(curve.removed_depth_fraction(500.0), 1.0, epsilon = 1e-12);
        // fraction = 0 inverts to the threshold dose.
        assert_relative_eq!(curve.dose_for_depth(0.0, 200.0), 10.0, epsilon = 1e-12);
    }

    #[test]
    fn test_invalid_contrast_curve() {
        assert!(ContrastCurve::new(0.0, 100.0).is_err());
        assert!(ContrastCurve::new(-1.0, 100.0).is_err());
        assert!(ContrastCurve::new(100.0, 100.0).is_err());
        assert!(ContrastCurve::new(100.0, 50.0).is_err());
    }

    #[test]
    fn test_blazed_grating_min_max() {
        let n = 32;
        let period = 8;
        let depth = 60.0;
        let thickness = 100.0;
        let grating = blazed_grating(n, period, depth, thickness);

        let max = grating.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min = grating.iter().cloned().fold(f64::INFINITY, f64::min);

        // Max height is the full thickness at the start of each period.
        assert_relative_eq!(max, thickness, epsilon = 1e-12);
        // Min height is at frac = (period-1)/period.
        let expected_min = thickness - depth * (period as f64 - 1.0) / period as f64;
        assert_relative_eq!(min, expected_min, epsilon = 1e-12);
        // Every height stays within the sawtooth envelope.
        for &h in grating.iter() {
            assert!(h >= thickness - depth - 1e-12 && h <= thickness + 1e-12);
        }
    }

    #[test]
    fn test_microlens_sag_bounds() {
        let n = 18;
        let pitch = 9; // odd → a pixel lands exactly on each cell center
        let sag = 40.0;
        let thickness = 120.0;
        let lenses = microlens_array(n, pitch, sag, thickness);

        for &h in lenses.iter() {
            assert!(
                h >= thickness - sag - 1e-9 && h <= thickness + 1e-9,
                "height {h} outside [thickness - sag, thickness]"
            );
        }
        // The odd pitch puts a sample on the paraboloid peak (r = 0), which
        // reaches the full thickness.
        let max = lenses.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert_relative_eq!(max, thickness, epsilon = 1e-12);
        // And the pattern actually varies (bumps above the base plane).
        let min = lenses.iter().cloned().fold(f64::INFINITY, f64::min);
        assert!(min < thickness - 1e-6);
    }

    #[test]
    fn test_staircase_through_height_map() {
        // gamma = 1, D_th = 10, D_clear = 100.
        let curve = ContrastCurve::new(10.0, 100.0).unwrap();
        let thickness = 200.0;
        let dose = 100.0;

        // Four regions with intensities chosen for clean closed-form depths.
        let intensities = [0.1, 0.1_f64.sqrt(), 0.5, 1.0];
        let mut aerial = Grid2D::<f64>::new(4, 2, (-4.0, 4.0), (-1.0, 1.0)).unwrap();
        for (j, &intensity) in intensities.iter().enumerate() {
            for i in 0..aerial.ny() {
                aerial.data[[i, j]] = intensity;
            }
        }

        let heights = height_map(&aerial, dose, &curve, thickness);
        for (j, &intensity) in intensities.iter().enumerate() {
            let frac = (curve.gamma * (dose * intensity / 10.0).log10()).clamp(0.0, 1.0);
            let expected = thickness - thickness * frac;
            for i in 0..heights.ny() {
                assert_relative_eq!(heights.data[[i, j]], expected, epsilon = 1e-9);
            }
        }
    }

    #[test]
    fn test_from_target_height_errors_when_dose_insufficient() {
        let curve = ContrastCurve::new(10.0, 100.0).unwrap();
        let thickness = 100.0;
        // A pixel that must clear fully needs the clearing dose (100); an
        // exposure of only 50 cannot reach it (T would exceed 1).
        let mut target = Array2::from_elem((4, 4), thickness);
        target[[1, 1]] = 0.0; // clear to substrate here
        let result = GrayscaleMap::from_target_height(&target, thickness, &curve, 50.0);
        assert!(matches!(result, Err(LithographyError::NumericalError(_))));

        // With enough exposure dose it succeeds and stays within [0, 1].
        let ok = GrayscaleMap::from_target_height(&target, thickness, &curve, 100.0).unwrap();
        for &t in ok.transmittance.iter() {
            assert!((0.0..=1.0 + 1e-12).contains(&t));
        }
    }

    #[test]
    fn test_to_complex_amplitude_is_sqrt_t() {
        let map = GrayscaleMap {
            transmittance: Array2::from_elem((2, 2), 0.25),
        };
        let c = map.to_complex();
        for &v in c.iter() {
            assert_relative_eq!(v.re, 0.5, epsilon = 1e-12);
            assert_relative_eq!(v.im, 0.0, epsilon = 1e-12);
        }
    }

    #[test]
    fn test_staircase_mask_builds_gray_rects() {
        let t = [1.0, 0.5, 0.25, 0.0];
        let mask = staircase_mask(4, 50.0, &t, 200.0);
        assert_eq!(mask.features.len(), 4);
        for (i, feature) in mask.features.iter().enumerate() {
            match feature {
                MaskFeature::GrayRect { transmittance, .. } => {
                    assert_relative_eq!(*transmittance, t[i], epsilon = 1e-12);
                }
                _ => panic!("staircase_mask must emit GrayRect features"),
            }
        }
    }
}
