//! Multiple patterning: litho-etch-litho-etch (LELE) double patterning
//! through the imaging engine, and geometric self-aligned double / quadruple
//! patterning (SADP / SAQP).
//!
//! **LELE.** Two exposures with complementary masks (lines at twice the
//! target pitch, the second set offset by one target pitch) are imaged
//! independently; the second image is shifted by the overlay error with the
//! Fourier shift theorem (exact sub-pixel translation of a band-limited
//! periodic image). Two combinations are reported:
//! - the **double-exposure aerial composite** `(d₁ I₁ + d₂ I₂′)/(d₁ + d₂)` —
//!   what a *single* resist exposed twice would see (kept from the original
//!   API; it is not the LELE result);
//! - the **LELE printed pattern** ([`lele_printed_pattern`]): each exposure is
//!   developed on its own (a positive-tone resist line remains where
//!   `dose·I < E_th`) and etched into the hard mask, so the final lines are
//!   the union of the two printed patterns. [`lele_cut`] measures the line
//!   intervals along a cut (cubic interpolation, bisected edges), from
//!   which CD, space, and pitch walk follow.
//!
//! **SADP / SAQP (geometric).** A periodic 1D [`LinePattern`] of mandrels
//! receives a conformal spacer of thickness `t` ([`spacer_process`]): every
//! line is replaced by two sidewall lines `[x₀ − t, x₀]` and `[x₁, x₁ + t]`
//! (deposition, anisotropic spacer etch, mandrel pull). SADP applies it once,
//! SAQP twice (the first spacers are the second mandrels). In the
//! spacer-is-line tone the spacers are the final lines; in the
//! spacer-is-dielectric tone the final lines are the gaps between spacers.
//!
//! # Key equations
//!
//! ```text
//!   LELE overlay shift      I₂′ = F⁻¹[ F(I₂) · e^{−2πi (f_x δx + f_y δy)} ]
//!   LELE printed lines      {d₁ I₁ < E_th} ∪ {d₂ I₂′ < E_th};  pitch walk = 2 δx
//!   SADP (mandrel pitch P, mandrel CD W, spacer t; spacer-is-line):
//!     lines t, core space W, gap space P − W − 2t
//!     pitches p₁ = W + t, p₂ = P − W − t,  pitch walk p₁ − p₂ = 2W + 2t − P
//!     walk-free mandrel CD W = P/2 − t (pitch P/2); requires P − W − 2t > 0
//!   SAQP (W, t₁, t₂; spacer-is-line): lines t₂; spaces t₁, W − 2t₂, t₁,
//!     P − W − 2t₁ − 2t₂; uniform pitch P/4 at t₁ = W − 2t₂ = P − W − 2t₁ − 2t₂
//!     (e.g. W = 3P/8, t₁ = t₂ = P/8)
//! ```
//!
//! # Model status
//!
//! - LELE: aerial images are the engine's; resist is a constant energy
//!   threshold per exposure; no inter-exposure resist interaction, freeze
//!   step, or etch bias. The double-exposure composite is **not** the LELE
//!   result.
//! - SADP/SAQP: purely geometric — ideal conformal deposition with vertical
//!   sidewalls, spacer width equal to the deposited thickness, perfect
//!   mandrel pull and pattern transfer. Deposition and etch physics
//!   (sidewall angle, footing, faceting, loading, etch bias) are **not**
//!   modeled; the mandrel CD is an input (e.g. from a printed image).

use ndarray::Array2;
use serde::{Deserialize, Serialize};

use crate::aerial::AerialImageEngine;
use crate::error::{LithographyError, Result};
use crate::mask::{LineOrientation, Mask};
use crate::math::fft2d::Fft2D;
use crate::metrics;
use crate::opc::{fft_freq, sample_image};
use crate::types::{Complex64, Grid2D};

/// Double patterning configuration.
#[derive(Debug, Clone)]
pub struct DoublePatterningConfig {
    /// Overlay error in x (nm) between first and second exposure.
    pub overlay_x_nm: f64,
    /// Overlay error in y (nm).
    pub overlay_y_nm: f64,
    /// Dose for first exposure (mJ/cm²).
    pub dose1_mj_cm2: f64,
    /// Dose for second exposure (mJ/cm²).
    pub dose2_mj_cm2: f64,
    /// Focus for first exposure (nm).
    pub focus1_nm: f64,
    /// Focus for second exposure (nm).
    pub focus2_nm: f64,
}

impl Default for DoublePatterningConfig {
    fn default() -> Self {
        Self {
            overlay_x_nm: 0.0,
            overlay_y_nm: 0.0,
            dose1_mj_cm2: 30.0,
            dose2_mj_cm2: 30.0,
            focus1_nm: 0.0,
            focus2_nm: 0.0,
        }
    }
}

/// Result of double patterning simulation.
#[derive(Debug)]
pub struct DoublePatterningResult {
    /// Double-exposure aerial composite `(d₁ I₁ + d₂ I₂′)/(d₁ + d₂)` — the
    /// image a single resist exposed twice would see; not the LELE result.
    pub combined_aerial: Grid2D<f64>,
    /// First exposure aerial image.
    pub aerial1: Grid2D<f64>,
    /// Second exposure aerial image (as imaged, before the overlay shift).
    pub aerial2: Grid2D<f64>,
    /// Second exposure aerial image translated by the overlay error.
    pub aerial2_overlay: Grid2D<f64>,
    /// Contrast of the double-exposure composite.
    pub combined_contrast: f64,
    /// Exposure doses (mJ/cm²) used for [`lele_printed_pattern`].
    pub doses_mj_cm2: (f64, f64),
}

/// Translate a periodic, band-limited image by `(dx, dy)` nm with the
/// Fourier shift theorem (exact sub-pixel interpolation).
fn fourier_shift(image: &Array2<f64>, pixel_nm: f64, dx_nm: f64, dy_nm: f64) -> Array2<f64> {
    if dx_nm == 0.0 && dy_nm == 0.0 {
        return image.clone();
    }
    let (ny, nx) = image.dim();
    let fft = Fft2D::new();
    let mut spec = image.mapv(|v| Complex64::new(v, 0.0));
    fft.forward(&mut spec);
    let (dfx, dfy) = (1.0 / (nx as f64 * pixel_nm), 1.0 / (ny as f64 * pixel_nm));
    let two_pi = 2.0 * std::f64::consts::PI;
    for ((i, j), v) in spec.indexed_iter_mut() {
        // Zero the Nyquist bins on even grids: their shift phase is not
        // Hermitian and would leave an imaginary residue.
        if (nx % 2 == 0 && j == nx / 2) || (ny % 2 == 0 && i == ny / 2) {
            *v = Complex64::new(0.0, 0.0);
            continue;
        }
        let fx = fft_freq(j, nx, dfx);
        let fy = fft_freq(i, ny, dfy);
        *v *= Complex64::from_polar(1.0, -two_pi * (fx * dx_nm + fy * dy_nm));
    }
    fft.inverse(&mut spec);
    spec.mapv(|c| c.re)
}

/// Simulate LELE double patterning with two masks.
///
/// Images both masks (each at its own focus), translates the second image by
/// the overlay error (Fourier shift, sub-pixel exact), and forms the
/// double-exposure composite `(d₁ I₁ + d₂ I₂′)/(d₁ + d₂)`. Use
/// [`lele_printed_pattern`] / [`lele_cut`] for the LELE printed result.
pub fn simulate_double_patterning(
    engine: &AerialImageEngine,
    mask1: &Mask,
    mask2: &Mask,
    config: &DoublePatterningConfig,
) -> DoublePatterningResult {
    let aerial1 = engine.compute(mask1, config.focus1_nm);
    let aerial2 = engine.compute(mask2, config.focus2_nm);
    let pixel = engine.grid().pixel_nm;

    let shifted = fourier_shift(
        &aerial2.data,
        pixel,
        config.overlay_x_nm,
        config.overlay_y_nm,
    );
    let aerial2_overlay = Grid2D {
        data: shifted,
        ..aerial2.clone()
    };

    let total_dose = config.dose1_mj_cm2 + config.dose2_mj_cm2;
    let mut combined =
        &aerial1.data * config.dose1_mj_cm2 + &aerial2_overlay.data * config.dose2_mj_cm2;
    if total_dose > 0.0 {
        combined.mapv_inplace(|v| v / total_dose);
    }
    let combined_contrast = metrics::image_contrast(&combined);

    DoublePatterningResult {
        combined_aerial: Grid2D {
            data: combined,
            ..aerial1.clone()
        },
        aerial1,
        aerial2,
        aerial2_overlay,
        combined_contrast,
        doses_mj_cm2: (config.dose1_mj_cm2, config.dose2_mj_cm2),
    }
}

/// LELE printed pattern: 1 where a resist line from either exposure remains
/// (`dose·I < E_th` for exposure 1, or for the overlay-shifted exposure 2),
/// 0 elsewhere. `threshold_mj_cm2` is the resist's clearing energy `E_th`.
pub fn lele_printed_pattern(result: &DoublePatterningResult, threshold_mj_cm2: f64) -> Array2<f64> {
    let (d1, d2) = result.doses_mj_cm2;
    let mut out = Array2::zeros(result.aerial1.data.dim());
    ndarray::Zip::from(&mut out)
        .and(&result.aerial1.data)
        .and(&result.aerial2_overlay.data)
        .for_each(|o, &a, &b| {
            if d1 * a < threshold_mj_cm2 || d2 * b < threshold_mj_cm2 {
                *o = 1.0;
            }
        });
    out
}

/// Printed LELE line intervals `(x₀, x₁)` (nm) along image row `row`, with
/// sub-pixel edges: the printing margin `E_th − min(d₁ I₁, d₂ I₂′)` is
/// sampled with cubic interpolation and its zero crossings refined by
/// bisection. Lines cut by the field boundary are dropped.
pub fn lele_cut(
    result: &DoublePatterningResult,
    threshold_mj_cm2: f64,
    row: usize,
) -> Vec<(f64, f64)> {
    let (d1, d2) = result.doses_mj_cm2;
    let g = &result.aerial1;
    let n = g.data.ncols();
    let pixel = g.pixel_size_x();
    let grid = crate::types::GridConfig {
        size: n,
        pixel_nm: pixel,
    };
    let y = g.y_at(row);
    let margin = |x: f64| {
        threshold_mj_cm2
            - (d1 * sample_image(&g.data, &grid, x, y))
                .min(d2 * sample_image(&result.aerial2_overlay.data, &grid, x, y))
    };
    let refine = |mut lo: f64, mut hi: f64| {
        let inside_lo = margin(lo) > 0.0;
        for _ in 0..50 {
            let mid = 0.5 * (lo + hi);
            if (margin(mid) > 0.0) == inside_lo {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    };
    let step = pixel / 4.0;
    let steps = (4 * n) as i64 - 1;
    let x_at = |k: i64| g.x_min_nm + 0.5 * pixel + k as f64 * step;
    let mut lines = Vec::new();
    let mut start: Option<f64> = None;
    let mut prev = margin(x_at(0));
    for k in 1..steps {
        let cur = margin(x_at(k));
        if prev <= 0.0 && cur > 0.0 {
            start = Some(refine(x_at(k - 1), x_at(k)));
        } else if prev > 0.0 && cur <= 0.0 {
            if let Some(s) = start.take() {
                lines.push((s, refine(x_at(k - 1), x_at(k))));
            }
        }
        prev = cur;
    }
    lines
}

/// Create the complementary mask pair for LELE double patterning.
///
/// The target is opaque lines of width `cd_nm` at pitch `pitch_nm`. Each
/// mask carries every other line — an infinite bright-field grating of
/// opaque lines of width `cd_nm` at twice the pitch
/// ([`Mask::line_space_with`]) — with one line centred at `x = 0` (mask 1) or
/// `x = pitch_nm` (mask 2). Image them on a field that holds a whole number
/// of double pitches ([`Mask::commensurate_grid`]).
pub fn split_mask_lele(cd_nm: f64, pitch_nm: f64) -> crate::error::Result<(Mask, Mask)> {
    if !(cd_nm.is_finite() && cd_nm > 0.0) {
        return Err(LithographyError::InvalidParameter {
            name: "cd_nm",
            value: cd_nm,
            reason: "must be positive",
        });
    }
    if !(pitch_nm.is_finite() && pitch_nm > cd_nm) {
        return Err(LithographyError::InvalidParameter {
            name: "pitch_nm",
            value: pitch_nm,
            reason: "must exceed cd_nm",
        });
    }
    let double_pitch = 2.0 * pitch_nm;
    Ok((
        Mask::line_space_with(cd_nm, double_pitch, LineOrientation::Vertical, 0.0)?,
        Mask::line_space_with(cd_nm, double_pitch, LineOrientation::Vertical, pitch_nm)?,
    ))
}

// ---------------------------------------------------------------------------
// Geometric spacer patterning (SADP / SAQP)
// ---------------------------------------------------------------------------

/// A periodic 1D line pattern: line intervals `(x₀, x₁)` (nm) in one period,
/// sorted by `x₀ ∈ [0, period)`; a line may extend past the period end.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinePattern {
    /// Period (nm).
    pub period_nm: f64,
    /// Line intervals, sorted by start.
    pub lines: Vec<(f64, f64)>,
}

impl LinePattern {
    /// Build a pattern, normalizing starts into `[0, period)` and sorting.
    /// Fails if a line is empty or longer than the period, or if two lines
    /// touch or overlap (including across the period boundary).
    pub fn new(period_nm: f64, lines: Vec<(f64, f64)>) -> Result<Self> {
        if !(period_nm.is_finite() && period_nm > 0.0) {
            return Err(LithographyError::InvalidParameter {
                name: "period_nm",
                value: period_nm,
                reason: "must be positive and finite",
            });
        }
        let mut norm = Vec::with_capacity(lines.len());
        for (a, b) in lines {
            let w = b - a;
            if !(w.is_finite() && w > 0.0 && w < period_nm) {
                return Err(LithographyError::InvalidParameter {
                    name: "line width",
                    value: w,
                    reason: "every line must have 0 < width < period",
                });
            }
            let s = a.rem_euclid(period_nm);
            norm.push((s, s + w));
        }
        norm.sort_by(|a, b| a.0.total_cmp(&b.0));
        let pattern = Self {
            period_nm,
            lines: norm,
        };
        if pattern.spaces().iter().any(|&s| s <= 1e-9) {
            return Err(LithographyError::InvalidParameter {
                name: "lines",
                value: pattern
                    .spaces()
                    .iter()
                    .cloned()
                    .fold(f64::INFINITY, f64::min),
                reason: "lines touch or overlap (spacer pinch-off / merge)",
            });
        }
        Ok(pattern)
    }

    /// Number of lines per period.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether the pattern has no lines.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Line widths (nm), in order.
    pub fn line_cds(&self) -> Vec<f64> {
        self.lines.iter().map(|(a, b)| b - a).collect()
    }

    /// Space after each line up to the next one (periodic), in order.
    pub fn spaces(&self) -> Vec<f64> {
        let n = self.lines.len();
        (0..n)
            .map(|i| {
                let next = if i + 1 < n {
                    self.lines[i + 1].0
                } else {
                    self.lines[0].0 + self.period_nm
                };
                next - self.lines[i].1
            })
            .collect()
    }

    /// Centre-to-centre distance from each line to the next (periodic).
    pub fn pitches(&self) -> Vec<f64> {
        let n = self.lines.len();
        let c = |i: usize| 0.5 * (self.lines[i].0 + self.lines[i].1);
        (0..n)
            .map(|i| {
                if i + 1 < n {
                    c(i + 1) - c(i)
                } else {
                    c(0) + self.period_nm - c(i)
                }
            })
            .collect()
    }

    /// Mean pitch `period / lines`.
    pub fn mean_pitch_nm(&self) -> f64 {
        self.period_nm / self.lines.len() as f64
    }

    /// Pitch walk: largest minus smallest centre-to-centre pitch (nm).
    pub fn pitch_walk_nm(&self) -> f64 {
        let p = self.pitches();
        p.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            - p.iter().cloned().fold(f64::INFINITY, f64::min)
    }

    /// Tone reversal: the spaces become the lines.
    pub fn complement(&self) -> Result<Self> {
        let n = self.lines.len();
        let lines = (0..n)
            .map(|i| {
                let next = if i + 1 < n {
                    self.lines[i + 1].0
                } else {
                    self.lines[0].0 + self.period_nm
                };
                (self.lines[i].1, next)
            })
            .collect();
        Self::new(self.period_nm, lines)
    }
}

/// Conformal spacer of thickness `t` on every line of `mandrels`, followed
/// by spacer etch and mandrel removal: each mandrel `[x₀, x₁]` becomes the
/// two sidewall lines `[x₀ − t, x₀]` and `[x₁, x₁ + t]`. Fails with a
/// pinch-off error if spacers of neighbouring mandrels touch or merge
/// (space between mandrels ≤ 2t).
pub fn spacer_process(mandrels: &LinePattern, thickness_nm: f64) -> Result<LinePattern> {
    if !(thickness_nm.is_finite() && thickness_nm > 0.0) {
        return Err(LithographyError::InvalidParameter {
            name: "spacer_thickness_nm",
            value: thickness_nm,
            reason: "must be positive and finite",
        });
    }
    if let Some(&min_space) = mandrels.spaces().iter().min_by(|a, b| a.total_cmp(b)) {
        if min_space <= 2.0 * thickness_nm {
            return Err(LithographyError::InvalidParameter {
                name: "spacer_thickness_nm",
                value: thickness_nm,
                reason: "spacers of neighbouring mandrels merge (space ≤ 2·thickness)",
            });
        }
    }
    let lines = mandrels
        .lines
        .iter()
        .flat_map(|&(a, b)| [(a - thickness_nm, a), (b, b + thickness_nm)])
        .collect();
    LinePattern::new(mandrels.period_nm, lines)
}

/// Final-line tone of a spacer process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SpacerTone {
    /// The spacers are the final lines (default).
    #[default]
    SpacerIsLine,
    /// The gaps between spacers are the final lines (tone reversal).
    SpacerIsDielectric,
}

/// Geometric SADP / SAQP input: one mandrel line of width
/// `mandrel_cd_nm` per `mandrel_pitch_nm`, then one (SADP) or two (SAQP)
/// conformal spacer steps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpacerPatterningConfig {
    /// Mandrel (lithographic) pitch P (nm).
    pub mandrel_pitch_nm: f64,
    /// Printed mandrel CD W (nm).
    pub mandrel_cd_nm: f64,
    /// Spacer thicknesses: one entry for SADP, two for SAQP.
    pub spacer_thickness_nm: Vec<f64>,
    /// Final-line tone.
    pub tone: SpacerTone,
}

/// Outcome of a geometric spacer patterning flow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpacerPatterningResult {
    /// Final lines in one mandrel period.
    pub pattern: LinePattern,
    /// Final line CDs (nm), in order.
    pub line_cds_nm: Vec<f64>,
    /// Final spaces (nm), in order.
    pub spaces_nm: Vec<f64>,
    /// Centre-to-centre pitches (nm), in order.
    pub pitches_nm: Vec<f64>,
    /// Nominal pitch `P / 2ⁿ` for `n` spacer steps (nm).
    pub nominal_pitch_nm: f64,
    /// Largest minus smallest pitch (nm).
    pub pitch_walk_nm: f64,
    /// Largest minus smallest line CD (nm).
    pub cd_range_nm: f64,
}

/// Run a geometric SADP (one spacer) or SAQP (two spacers) flow.
pub fn spacer_patterning(config: &SpacerPatterningConfig) -> Result<SpacerPatterningResult> {
    let (p, w) = (config.mandrel_pitch_nm, config.mandrel_cd_nm);
    if !(w.is_finite() && w > 0.0 && w < p) {
        return Err(LithographyError::InvalidParameter {
            name: "mandrel_cd_nm",
            value: w,
            reason: "must satisfy 0 < mandrel CD < mandrel pitch",
        });
    }
    if config.spacer_thickness_nm.is_empty() {
        return Err(LithographyError::InvalidParameter {
            name: "spacer_thickness_nm",
            value: 0.0,
            reason: "at least one spacer step is required",
        });
    }
    // Mandrel centred in the period.
    let mut pattern = LinePattern::new(p, vec![(0.5 * (p - w), 0.5 * (p + w))])?;
    for &t in &config.spacer_thickness_nm {
        pattern = spacer_process(&pattern, t)?;
    }
    if config.tone == SpacerTone::SpacerIsDielectric {
        pattern = pattern.complement()?;
    }
    let line_cds_nm = pattern.line_cds();
    let cd_range_nm = line_cds_nm
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max)
        - line_cds_nm.iter().cloned().fold(f64::INFINITY, f64::min);
    Ok(SpacerPatterningResult {
        spaces_nm: pattern.spaces(),
        pitches_nm: pattern.pitches(),
        nominal_pitch_nm: p / 2f64.powi(config.spacer_thickness_nm.len() as i32),
        pitch_walk_nm: pattern.pitch_walk_nm(),
        cd_range_nm,
        line_cds_nm,
        pattern,
    })
}

/// Geometric SADP: mandrel pitch `P`, mandrel CD `W`, spacer `t`.
pub fn sadp(
    mandrel_pitch_nm: f64,
    mandrel_cd_nm: f64,
    spacer_nm: f64,
    tone: SpacerTone,
) -> Result<SpacerPatterningResult> {
    spacer_patterning(&SpacerPatterningConfig {
        mandrel_pitch_nm,
        mandrel_cd_nm,
        spacer_thickness_nm: vec![spacer_nm],
        tone,
    })
}

/// Geometric SAQP: mandrel pitch `P`, mandrel CD `W`, spacers `t₁`, `t₂`.
pub fn saqp(
    mandrel_pitch_nm: f64,
    mandrel_cd_nm: f64,
    spacer1_nm: f64,
    spacer2_nm: f64,
    tone: SpacerTone,
) -> Result<SpacerPatterningResult> {
    spacer_patterning(&SpacerPatterningConfig {
        mandrel_pitch_nm,
        mandrel_cd_nm,
        spacer_thickness_nm: vec![spacer1_nm, spacer2_nm],
        tone,
    })
}

/// Closed-form SADP (spacer-is-line) pitch walk `p₁ − p₂ = 2W + 2t − P`,
/// with `p₁ = W + t` across the mandrel and `p₂ = P − W − t` across the gap.
pub fn sadp_pitch_walk_nm(mandrel_pitch_nm: f64, mandrel_cd_nm: f64, spacer_nm: f64) -> f64 {
    2.0 * mandrel_cd_nm + 2.0 * spacer_nm - mandrel_pitch_nm
}

/// Mandrel CD that makes SADP walk-free: `W = P/2 − t`.
pub fn sadp_walk_free_mandrel_cd_nm(mandrel_pitch_nm: f64, spacer_nm: f64) -> f64 {
    0.5 * mandrel_pitch_nm - spacer_nm
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optics::ProjectionOptics;
    use crate::source::VuvSource;
    use crate::types::GridConfig;

    fn engine() -> AerialImageEngine {
        let source = VuvSource::f2_laser(0.7).unwrap();
        let optics = ProjectionOptics::new(0.75).unwrap();
        // 64 × 10 nm = 640 nm field: a multiple of the 320 nm double pitch.
        let grid = GridConfig {
            size: 64,
            pixel_nm: 10.0,
        };
        AerialImageEngine::new(&source, &optics, grid, 10).unwrap()
    }

    #[test]
    fn test_double_patterning_basic() {
        let engine = engine();
        let (mask1, mask2) = split_mask_lele(80.0, 160.0).unwrap();
        let config = DoublePatterningConfig::default();
        let result = simulate_double_patterning(&engine, &mask1, &mask2, &config);

        assert!(result.combined_contrast > 0.0);
        assert!(result.combined_aerial.data.iter().all(|&v| v >= -1e-10));
    }

    #[test]
    fn test_overlay_shifts_result() {
        let engine = engine();
        let (mask1, mask2) = split_mask_lele(80.0, 160.0).unwrap();

        let config_zero = DoublePatterningConfig::default();
        let config_shifted = DoublePatterningConfig {
            overlay_x_nm: 10.0,
            ..Default::default()
        };

        let result_zero = simulate_double_patterning(&engine, &mask1, &mask2, &config_zero);
        let result_shifted = simulate_double_patterning(&engine, &mask1, &mask2, &config_shifted);

        // Overlay error should change the combined image
        let diff: f64 = result_zero
            .combined_aerial
            .data
            .iter()
            .zip(result_shifted.combined_aerial.data.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();

        assert!(
            diff > 0.01,
            "Overlay shift should change the combined image"
        );
    }

    #[test]
    fn test_split_mask_creates_pair() {
        let (mask1, mask2) = split_mask_lele(65.0, 180.0).unwrap();
        // Opaque-line gratings of width cd at twice the pitch, offset by one
        // pitch, on bright-field masks.
        let grating = |m: &Mask| match m.features.as_slice() {
            [crate::mask::MaskFeature::LineSpace {
                cd, pitch, offset, ..
            }] => (*cd, *pitch, *offset),
            other => panic!("expected one grating, got {other:?}"),
        };
        assert_eq!(grating(&mask1), (65.0, 360.0, 0.0));
        assert_eq!(grating(&mask2), (65.0, 360.0, 180.0));
        assert!(!mask1.dark_field && !mask2.dark_field);
        assert!(split_mask_lele(100.0, 80.0).is_err());
    }

    #[test]
    fn test_fourier_shift_is_exact_for_band_limited_images() {
        // cos(2π·3x/L) shifted by a sub-pixel 3.7 nm equals the analytic
        // shifted cosine.
        let (n, p) = (32usize, 10.0);
        let l = n as f64 * p;
        let f = |x: f64| 1.0 + 0.5 * (2.0 * std::f64::consts::PI * 3.0 * x / l).cos();
        let image = Array2::from_shape_fn((n, n), |(_, j)| f(j as f64 * p));
        let shifted = fourier_shift(&image, p, 3.7, 0.0);
        for j in 0..n {
            assert!((shifted[[5, j]] - f(j as f64 * p - 3.7)).abs() < 1e-12);
        }
    }

    #[test]
    fn test_lele_overlay_produces_pitch_walk_of_twice_the_overlay() {
        // Target: 80 nm lines at 160 nm pitch from two exposures at 320 nm.
        // An x-overlay error δ moves every second line by δ: the printed
        // pitches alternate p ± δ (pitch walk 2δ) while CDs are unchanged.
        let engine = engine();
        let (mask1, mask2) = split_mask_lele(80.0, 160.0).unwrap();
        let e_th = 30.0 * 0.35;
        let row = 32;
        let measure = |dx: f64| {
            let config = DoublePatterningConfig {
                overlay_x_nm: dx,
                ..Default::default()
            };
            let r = simulate_double_patterning(&engine, &mask1, &mask2, &config);
            lele_cut(&r, e_th, row)
        };
        let nominal = measure(0.0);
        let walked = measure(6.0);
        assert!(nominal.len() >= 3 && walked.len() == nominal.len());
        let centres =
            |v: &[(f64, f64)]| -> Vec<f64> { v.iter().map(|(a, b)| 0.5 * (a + b)).collect() };
        let pitches =
            |v: &[(f64, f64)]| -> Vec<f64> { centres(v).windows(2).map(|w| w[1] - w[0]).collect() };
        for p in pitches(&nominal) {
            assert!((p - 160.0).abs() < 0.05, "nominal pitch {p}");
        }
        let pw = pitches(&walked);
        for w in pw.windows(2) {
            assert!(((w[0] - w[1]).abs() - 12.0).abs() < 0.05, "pitches {pw:?}");
        }
        let cds = |v: &[(f64, f64)]| -> Vec<f64> { v.iter().map(|(a, b)| b - a).collect() };
        for (a, b) in cds(&nominal).iter().zip(cds(&walked)) {
            assert!((a - b).abs() < 0.05);
        }
        // The printed map is the union of both exposures.
        let r =
            simulate_double_patterning(&engine, &mask1, &mask2, &DoublePatterningConfig::default());
        let printed = lele_printed_pattern(&r, e_th);
        let ones = printed.iter().filter(|&&v| v > 0.5).count();
        assert!(ones > 0 && ones < printed.len());
    }

    #[test]
    fn test_sadp_exact_pitch_halving() {
        // P = 128, W = t = 32: lines 32 and spaces 32 at exactly P/2 = 64.
        let r = sadp(128.0, 32.0, 32.0, SpacerTone::SpacerIsLine).unwrap();
        assert_eq!(r.pattern.len(), 2);
        assert_eq!(r.nominal_pitch_nm, 64.0);
        for v in r.line_cds_nm.iter().chain(&r.spaces_nm) {
            assert!((v - 32.0).abs() < 1e-12);
        }
        for p in &r.pitches_nm {
            assert!((p - 64.0).abs() < 1e-12);
        }
        assert!(r.pitch_walk_nm.abs() < 1e-12);
        assert_eq!(sadp_walk_free_mandrel_cd_nm(128.0, 32.0), 32.0);
    }

    #[test]
    fn test_sadp_pitch_walk_formula() {
        // Mandrel CD error +2 nm: spaces W = 34 (core) and P − W − 2t = 30
        // (gap); pitches W + t = 66 and P − W − t = 62; walk 2·2 = 4 nm.
        let r = sadp(128.0, 34.0, 32.0, SpacerTone::SpacerIsLine).unwrap();
        let mut spaces = r.spaces_nm.clone();
        spaces.sort_by(f64::total_cmp);
        assert!((spaces[0] - 30.0).abs() < 1e-12 && (spaces[1] - 34.0).abs() < 1e-12);
        let mut pitches = r.pitches_nm.clone();
        pitches.sort_by(f64::total_cmp);
        assert!((pitches[0] - 62.0).abs() < 1e-12 && (pitches[1] - 66.0).abs() < 1e-12);
        assert!((r.pitch_walk_nm - 4.0).abs() < 1e-12);
        assert!((sadp_pitch_walk_nm(128.0, 34.0, 32.0) - 4.0).abs() < 1e-12);
        // Spacer +1.5 nm: lines 33.5, walk 2·1.5 = 3 nm, CDs stay uniform.
        let r = sadp(128.0, 32.0, 33.5, SpacerTone::SpacerIsLine).unwrap();
        assert!((r.pitch_walk_nm - 3.0).abs() < 1e-12);
        assert!(r.line_cds_nm.iter().all(|c| (c - 33.5).abs() < 1e-12));
        assert!(r.cd_range_nm.abs() < 1e-12);
        // The closed form matches the interval model over a sweep.
        for w in [26.0, 30.0, 35.5, 40.0] {
            for t in [24.0, 29.0, 33.0] {
                let r = sadp(128.0, w, t, SpacerTone::SpacerIsLine).unwrap();
                assert!((r.pitch_walk_nm - sadp_pitch_walk_nm(128.0, w, t).abs()).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn test_sadp_spacer_is_dielectric_tone() {
        // Tone reversal: lines are the core (W) and gap (P − W − 2t)
        // regions; the spaces are the spacers.
        let r = sadp(128.0, 36.0, 30.0, SpacerTone::SpacerIsDielectric).unwrap();
        let mut cds = r.line_cds_nm.clone();
        cds.sort_by(f64::total_cmp);
        assert!((cds[0] - 32.0).abs() < 1e-12 && (cds[1] - 36.0).abs() < 1e-12);
        assert!(r.spaces_nm.iter().all(|s| (s - 30.0).abs() < 1e-12));
        assert!((r.cd_range_nm - 4.0).abs() < 1e-12);
    }

    #[test]
    fn test_saqp_exact_pitch_quartering_and_walk() {
        // P = 128, W = 3P/8 = 48, t₁ = t₂ = P/8 = 16: 16 nm lines and
        // spaces at exactly P/4 = 32 nm.
        let r = saqp(128.0, 48.0, 16.0, 16.0, SpacerTone::SpacerIsLine).unwrap();
        assert_eq!(r.pattern.len(), 4);
        assert_eq!(r.nominal_pitch_nm, 32.0);
        for v in r.line_cds_nm.iter().chain(&r.spaces_nm) {
            assert!((v - 16.0).abs() < 1e-12);
        }
        assert!(r.pitch_walk_nm.abs() < 1e-12);
        // Perturbed (W, t₁, t₂) = (50, 17, 15): spaces t₁ = 17 (twice),
        // W − 2t₂ = 20, P − W − 2t₁ − 2t₂ = 14; pitches t₁ + t₂ = 32 (twice),
        // W − t₂ = 35, P − W − 2t₁ − t₂ = 29 → walk 6 nm.
        let r = saqp(128.0, 50.0, 17.0, 15.0, SpacerTone::SpacerIsLine).unwrap();
        let mut spaces = r.spaces_nm.clone();
        spaces.sort_by(f64::total_cmp);
        for (got, want) in spaces.iter().zip([14.0, 17.0, 17.0, 20.0]) {
            assert!((got - want).abs() < 1e-12, "{spaces:?}");
        }
        let mut pitches = r.pitches_nm.clone();
        pitches.sort_by(f64::total_cmp);
        for (got, want) in pitches.iter().zip([29.0, 32.0, 32.0, 35.0]) {
            assert!((got - want).abs() < 1e-12, "{pitches:?}");
        }
        assert!((r.pitch_walk_nm - 6.0).abs() < 1e-12);
        assert!(r.line_cds_nm.iter().all(|c| (c - 15.0).abs() < 1e-12));
    }

    #[test]
    fn test_spacer_pinch_off_and_invalid_inputs() {
        // Space between mandrels P − W = 60 ≤ 2t = 64: spacers merge.
        assert!(sadp(128.0, 68.0, 32.0, SpacerTone::SpacerIsLine).is_err());
        assert!(sadp(128.0, 0.0, 32.0, SpacerTone::SpacerIsLine).is_err());
        assert!(sadp(128.0, 32.0, -1.0, SpacerTone::SpacerIsLine).is_err());
        assert!(LinePattern::new(100.0, vec![(0.0, 60.0), (50.0, 70.0)]).is_err());
        // Wrap-around overlap is detected too.
        assert!(LinePattern::new(100.0, vec![(10.0, 40.0), (90.0, 115.0)]).is_err());
    }
}
