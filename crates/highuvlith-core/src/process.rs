//! Process-window analysis over dose and focus.
//!
//! Builds a focus–exposure matrix (FEM) of printed CDs and reads it the way a
//! lithographer reads a process window: Bossung curves (CD vs focus per
//! dose), exposure latitude (EL), depth of focus (DOF), the
//! exposure–defocus (ED) window with its EL-vs-DOF trade-off (e.g. DOF at 5 %
//! EL), best focus, dose-to-size and the iso-focal dose.
//!
//! Dose enters through a constant-threshold resist ([`ThresholdResist`]):
//! with the aerial image normalized to the clear-field intensity (I = 1 for
//! an open frame), a point receives dose `d·I(x)` and clears where
//! `d·I(x) ≥ E_th`. At dose `d` the printed edge is therefore the intensity
//! contour `I = E_th/d`, and the CD at `(d, z)` is the width of the feature
//! of the measured [`FeatureTone`] on the y = 0 cross-section of the
//! defocus-`z` image at that threshold. One aerial image per focus is
//! computed ([`AerialImageEngine::compute_through_focus`]: the mask spectrum
//! once, focus planes in parallel with Rayon when the `parallel` feature is
//! on); every dose is then evaluated on the stored cross-sections, so dose is
//! a continuous variable: the in-spec dose interval at each focus
//! ([`ProcessWindow::dose_limits`]) is found by bisection on the threshold,
//! not read off the sampled dose grid.
//!
//! Tabulated FEM data (measured or from elsewhere) is supported by
//! [`ProcessWindow::from_cd_matrix`]; there CD is interpolated linearly in
//! dose between the sampled rows.
//!
//! # Key equations
//!
//! ```text
//!   printed edge at dose d:   I(x_edge; z) = E_th / d             (constant threshold)
//!   legacy compute():         E_th = cd_threshold · d_nom,  d_nom = median(doses)
//!   spec:                     CD_t·(1 − tol) ≤ CD(d, z) ≤ CD_t·(1 + tol)
//!   exposure latitude:        EL = (d_max − d_min) / ((d_max + d_min)/2) · 100 %
//!   ED rectangle:             [z1, z2] × [d1, d2] inside the in-spec region;
//!                             DOF = z2 − z1,  EL from (d1, d2)
//!   best focus:               dCD/dz = 0 on the Bossung curve at the dose-to-size
//!                             (least-squares quadratic vertex)
//!   iso-focal dose:           argmin_d [max_z CD(d, z) − min_z CD(d, z)]
//! ```
//!
//! For a Bossung curve of a line whose image is `a − b(z)·cos(2πx/p)`, the
//! iso-focal threshold is exactly `I = a` (CD = p/2 at every focus), so the
//! iso-focal dose is `E_th/a` — one of the closed forms the tests check.
//!
//! # Model status
//!
//! 🔶 Simplified. The resist is a constant-threshold model: no acid
//! diffusion or image blur, no development kinetics, no resist thickness,
//! standing waves or mask-3D effects — the only dose dependence is the
//! threshold shift `E_th/d`. CD is measured on the y = 0 cross-section only
//! (the x-width of a line, or of a contact through its centre). Focus is
//! sampled at the sweep points; between them the in-spec dose limits are
//! interpolated linearly, so DOF resolution is set by the focus step (dose
//! resolution is not). For tabulated matrices CD is linear between sampled
//! doses. The ED analysis uses inscribed rectangles only (no ellipse); EL is
//! relative to the rectangle's centre dose. [`batch_defocus`] returns raw
//! aerial images across a focus list.

use ndarray::Array2;
use serde::{Deserialize, Serialize};

use crate::aerial::AerialImageEngine;
use crate::error::{LithographyError, Result};
use crate::mask::Mask;
use crate::metrics::{self, FeatureTone};
use crate::types::Grid2D;

/// Constant-threshold resist: a point clears where `dose · I(x) ≥ E_th`.
///
/// `I` is the aerial image normalized to the clear-field intensity, so
/// `dose_to_clear_mj_cm2` (= E_th) is the dose that just clears an open-frame
/// exposure. At dose `d` the printed edge is the intensity contour `E_th/d`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThresholdResist {
    /// Open-frame dose to clear E_th (mJ/cm²).
    pub dose_to_clear_mj_cm2: f64,
}

impl ThresholdResist {
    /// Resist with the given open-frame dose to clear (must be positive).
    pub fn new(dose_to_clear_mj_cm2: f64) -> Result<Self> {
        if !(dose_to_clear_mj_cm2.is_finite() && dose_to_clear_mj_cm2 > 0.0) {
            return Err(LithographyError::InvalidParameter {
                name: "dose_to_clear_mj_cm2",
                value: dose_to_clear_mj_cm2,
                reason: "must be positive and finite",
            });
        }
        Ok(Self {
            dose_to_clear_mj_cm2,
        })
    }

    /// Resist whose printed edge at `nominal_dose_mj_cm2` is the intensity
    /// contour `intensity_threshold`: `E_th = intensity_threshold ·
    /// nominal_dose`.
    pub fn from_nominal(intensity_threshold: f64, nominal_dose_mj_cm2: f64) -> Result<Self> {
        if !(intensity_threshold.is_finite() && intensity_threshold > 0.0) {
            return Err(LithographyError::InvalidParameter {
                name: "intensity_threshold",
                value: intensity_threshold,
                reason: "must be positive and finite",
            });
        }
        if !(nominal_dose_mj_cm2.is_finite() && nominal_dose_mj_cm2 > 0.0) {
            return Err(LithographyError::InvalidParameter {
                name: "nominal_dose_mj_cm2",
                value: nominal_dose_mj_cm2,
                reason: "must be positive and finite",
            });
        }
        Self::new(intensity_threshold * nominal_dose_mj_cm2)
    }

    /// Intensity threshold `E_th / d` at dose `d` (mJ/cm²).
    pub fn intensity_threshold(&self, dose_mj_cm2: f64) -> f64 {
        self.dose_to_clear_mj_cm2 / dose_mj_cm2
    }

    /// Dose `E_th / I` whose printed edge is the intensity contour `I`.
    pub fn dose_for_threshold(&self, intensity_threshold: f64) -> f64 {
        self.dose_to_clear_mj_cm2 / intensity_threshold
    }
}

/// A single point on a Bossung curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossungPoint {
    pub dose_mj_cm2: f64,
    pub focus_nm: f64,
    pub cd_nm: f64,
}

/// A rectangle of the exposure–defocus plane lying entirely inside the
/// in-spec region: every (dose, focus) in it prints within tolerance.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProcessRectangle {
    pub focus_min_nm: f64,
    pub focus_max_nm: f64,
    pub dose_min_mj_cm2: f64,
    pub dose_max_mj_cm2: f64,
}

impl ProcessRectangle {
    /// Depth of focus of the rectangle (nm).
    pub fn dof_nm(&self) -> f64 {
        self.focus_max_nm - self.focus_min_nm
    }

    /// Exposure latitude (%) relative to the rectangle's centre dose.
    pub fn exposure_latitude_pct(&self) -> f64 {
        el_pct(self.dose_min_mj_cm2, self.dose_max_mj_cm2)
    }

    /// Focus at the rectangle centre (nm).
    pub fn focus_centre_nm(&self) -> f64 {
        0.5 * (self.focus_min_nm + self.focus_max_nm)
    }

    /// Dose at the rectangle centre (mJ/cm²).
    pub fn dose_centre_mj_cm2(&self) -> f64 {
        0.5 * (self.dose_min_mj_cm2 + self.dose_max_mj_cm2)
    }
}

/// Exposure latitude (%) of a dose interval relative to its centre.
fn el_pct(lo: f64, hi: f64) -> f64 {
    if hi > lo && hi + lo > 0.0 {
        200.0 * (hi - lo) / (hi + lo)
    } else {
        0.0
    }
}

/// Process-window (focus–exposure matrix) analysis result.
#[derive(Debug, Clone)]
pub struct ProcessWindow {
    /// Dose values used (mJ/cm²), in the caller's order.
    pub doses: Vec<f64>,
    /// Focus values used (nm), in the caller's order.
    pub focuses: Vec<f64>,
    /// CD (nm) at each (dose_index, focus_index); `NaN` where the measured
    /// feature does not print or has merged with its neighbours (previous
    /// versions stored 0.0 there).
    pub cd_matrix: Array2<f64>,
    /// Target CD in nm.
    pub cd_target_nm: f64,
    /// CD tolerance in percent (the spec is target·(1 ± tol/100)).
    pub cd_tolerance_pct: f64,
    /// Tone of the measured feature (`None` for tabulated data).
    pub tone: Option<FeatureTone>,
    /// Constant-threshold resist used for the dose axis (`None` for
    /// tabulated data).
    pub resist: Option<ThresholdResist>,
    /// Per focus (same order as `focuses`): the in-spec dose interval
    /// `(d_min, d_max)` in mJ/cm², continuous in dose; `None` if the CD is
    /// in spec at no dose. For model data these come from the resist model
    /// at any dose and are not clipped to the swept dose range.
    pub dose_limits: Vec<Option<(f64, f64)>>,
    /// Pixel-centre x coordinates (nm) of `profiles` (empty for tabulated
    /// data).
    pub x_nm: Vec<f64>,
    /// y = 0 intensity cross-section at each focus (same order as
    /// `focuses`; empty for tabulated data).
    pub profiles: Vec<Vec<f64>>,
    /// Per focus: the reference point (extreme-intensity sample) that
    /// identifies the measured feature at every dose.
    feature_ref_nm: Vec<Option<f64>>,
}

fn invalid(name: &'static str, value: f64, reason: &'static str) -> LithographyError {
    LithographyError::InvalidParameter {
        name,
        value,
        reason,
    }
}

fn validate_doses(doses: &[f64]) -> Result<()> {
    if doses.is_empty() {
        return Err(invalid("doses", 0.0, "at least one dose is required"));
    }
    for &d in doses {
        if !(d.is_finite() && d > 0.0) {
            return Err(invalid(
                "doses",
                d,
                "every dose must be positive and finite",
            ));
        }
    }
    Ok(())
}

fn validate_focuses(focuses: &[f64]) -> Result<()> {
    if focuses.is_empty() {
        return Err(invalid("focuses", 0.0, "at least one focus is required"));
    }
    for &z in focuses {
        if !z.is_finite() {
            return Err(invalid("focuses", z, "every focus must be finite"));
        }
    }
    Ok(())
}

fn validate_spec(cd_target_nm: f64, cd_tolerance_pct: f64) -> Result<()> {
    if !(cd_target_nm.is_finite() && cd_target_nm > 0.0) {
        return Err(invalid("cd_target_nm", cd_target_nm, "must be positive"));
    }
    if !(cd_tolerance_pct.is_finite() && (0.0..100.0).contains(&cd_tolerance_pct)) {
        return Err(invalid(
            "cd_tolerance_pct",
            cd_tolerance_pct,
            "must be in [0, 100)",
        ));
    }
    Ok(())
}

/// Median of a non-empty slice (mean of the two middle values for an even
/// count).
fn median(values: &[f64]) -> f64 {
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));
    let n = v.len();
    if n == 0 {
        return f64::NAN;
    }
    if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

/// Boundary of a monotone predicate on `[a, b]` with `pred(a) = pred_a` and
/// `pred(b) = !pred_a`, located by bisection.
fn bisect_boundary(mut a: f64, mut b: f64, pred_a: bool, pred: impl Fn(f64) -> bool) -> f64 {
    let scale = a.abs().max(b.abs()).max(1e-300);
    for _ in 0..200 {
        if (b - a).abs() <= 1e-14 * scale {
            break;
        }
        let mid = 0.5 * (a + b);
        if pred(mid) == pred_a {
            a = mid;
        } else {
            b = mid;
        }
    }
    0.5 * (a + b)
}

/// Least-squares quadratic through `(z, cd)` points; returns the vertex
/// focus and the sign of the curvature, or `None` if the fit is degenerate
/// (fewer than 3 points or negligible curvature).
fn quadratic_vertex(pts: &[(f64, f64)]) -> Option<(f64, f64)> {
    if pts.len() < 3 {
        return None;
    }
    let zc = pts.iter().map(|p| p.0).sum::<f64>() / pts.len() as f64;
    let s = pts.iter().map(|p| (p.0 - zc).abs()).fold(0.0_f64, f64::max);
    if s.is_nan() || s <= 0.0 {
        return None;
    }
    let mut m = [[0.0_f64; 4]; 3];
    for &(z, y) in pts {
        let u = (z - zc) / s;
        let pw = [1.0, u, u * u, u * u * u, u * u * u * u];
        for (r, row) in m.iter_mut().enumerate() {
            for (c, cell) in row.iter_mut().take(3).enumerate() {
                *cell += pw[r + c];
            }
            row[3] += pw[r] * y;
        }
    }
    // Gaussian elimination with partial pivoting on the 3×3 normal equations.
    for col in 0..3 {
        let piv = (col..3).max_by(|&a, &b| m[a][col].abs().total_cmp(&m[b][col].abs()))?;
        if m[piv][col].abs() < 1e-300 {
            return None;
        }
        m.swap(col, piv);
        for r in 0..3 {
            if r != col {
                let f = m[r][col] / m[col][col];
                let pivot_row = m[col];
                for (cell, &pv) in m[r].iter_mut().zip(pivot_row.iter()).skip(col) {
                    *cell -= f * pv;
                }
            }
        }
    }
    let c1 = m[1][3] / m[1][1];
    let c2 = m[2][3] / m[2][2];
    let mean_abs = pts.iter().map(|p| p.1.abs()).sum::<f64>() / pts.len() as f64;
    if !c1.is_finite() || !c2.is_finite() || c2.abs() <= 1e-9 * mean_abs.max(1e-12) {
        return None;
    }
    Some((zc + s * (-c1 / (2.0 * c2)), c2.signum()))
}

/// In-spec dose interval of one tabulated FEM column (CD linear in dose
/// between finite samples); the longest in-spec run is returned.
fn tabulated_dose_limits(doses: &[f64], cds: &[f64], lo: f64, hi: f64) -> Option<(f64, f64)> {
    let mut pts: Vec<(f64, f64)> = doses.iter().copied().zip(cds.iter().copied()).collect();
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let in_spec = |c: f64| c.is_finite() && c >= lo && c <= hi;
    let mut intervals: Vec<(f64, f64)> = Vec::new();
    for (i, &(d, c)) in pts.iter().enumerate() {
        if in_spec(c) {
            intervals.push((d, d));
        }
        if let Some(&(d1, c1)) = pts.get(i + 1) {
            if c.is_finite() && c1.is_finite() && d1 > d {
                // Sub-interval of the segment where the linear CD is in spec.
                let (mut s0, mut s1) = (0.0_f64, 1.0_f64);
                let dc = c1 - c;
                if dc == 0.0 {
                    if !in_spec(c) {
                        continue;
                    }
                } else {
                    let sa = (lo - c) / dc;
                    let sb = (hi - c) / dc;
                    s0 = s0.max(sa.min(sb));
                    s1 = s1.min(sa.max(sb));
                    if s0 > s1 {
                        continue;
                    }
                }
                intervals.push((d + s0 * (d1 - d), d + s1 * (d1 - d)));
            }
        }
    }
    if intervals.is_empty() {
        return None;
    }
    intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for iv in intervals {
        match merged.last_mut() {
            Some(last) if iv.0 <= last.1 * (1.0 + 1e-12) => last.1 = last.1.max(iv.1),
            _ => merged.push(iv),
        }
    }
    merged
        .into_iter()
        .max_by(|a, b| (a.1 - a.0).total_cmp(&(b.1 - b.0)))
}

/// Linear interpolation of a tabulated CD column at `dose` (finite
/// bracketing samples required).
fn tabulated_cd_at(doses: &[f64], cds: &[f64], dose: f64) -> Option<f64> {
    let mut pts: Vec<(f64, f64)> = doses.iter().copied().zip(cds.iter().copied()).collect();
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    for w in pts.windows(2) {
        let ((d0, c0), (d1, c1)) = (w[0], w[1]);
        if dose >= d0 && dose <= d1 {
            if dose == d0 && c0.is_finite() {
                return Some(c0);
            }
            if dose == d1 && c1.is_finite() {
                return Some(c1);
            }
            if c0.is_finite() && c1.is_finite() && d1 > d0 {
                return Some(c0 + (c1 - c0) * (dose - d0) / (d1 - d0));
            }
            return None;
        }
    }
    if pts.len() == 1 && pts[0].0 == dose && pts[0].1.is_finite() {
        return Some(pts[0].1);
    }
    None
}

/// Position of a focus value among the (sorted) focus samples.
#[derive(Debug, Clone, Copy)]
enum FocusPosition {
    /// On sample `j`.
    At(usize),
    /// Between focus-adjacent samples `(j0, j1)` at fraction `f` from `j0`.
    Between(usize, usize, f64),
}

/// One aerial image per focus: the mask spectrum is computed once and the
/// focus planes run in parallel (Rayon, `parallel` feature).
fn focus_images(engine: &AerialImageEngine, mask: &Mask, focuses: &[f64]) -> Vec<Grid2D<f64>> {
    engine.compute_through_focus(mask, focuses)
}

impl ProcessWindow {
    /// Compute the process window by sweeping dose and focus through the
    /// engine (legacy signature, now dose-aware).
    ///
    /// `cd_threshold` is the intensity threshold at the **nominal dose**,
    /// the median of `doses`: the resist is
    /// `ThresholdResist { dose_to_clear = cd_threshold · d_nom }`, so at dose
    /// `d` the printed edge is the contour `cd_threshold · d_nom / d`. The
    /// measured feature tone follows the mask ([`FeatureTone::of_mask`]).
    /// See [`Self::compute_threshold_model`].
    pub fn compute(
        engine: &AerialImageEngine,
        mask: &Mask,
        doses: &[f64],
        focuses: &[f64],
        cd_threshold: f64,
        cd_target_nm: f64,
        cd_tolerance_pct: f64,
    ) -> Result<Self> {
        validate_doses(doses)?;
        let resist = ThresholdResist::from_nominal(cd_threshold, median(doses))?;
        Self::compute_threshold_model(
            engine,
            mask,
            doses,
            focuses,
            &resist,
            cd_target_nm,
            cd_tolerance_pct,
        )
    }

    /// Compute the process window with an explicit constant-threshold resist:
    /// one aerial image per focus (parallel over focus), then CD at every
    /// dose from the y = 0 cross-section at threshold `E_th/d`, for the tone
    /// of the mask's features.
    pub fn compute_threshold_model(
        engine: &AerialImageEngine,
        mask: &Mask,
        doses: &[f64],
        focuses: &[f64],
        resist: &ThresholdResist,
        cd_target_nm: f64,
        cd_tolerance_pct: f64,
    ) -> Result<Self> {
        validate_doses(doses)?;
        validate_focuses(focuses)?;
        validate_spec(cd_target_nm, cd_tolerance_pct)?;
        let images = focus_images(engine, mask, focuses);
        let first = &images[0];
        let x_nm = metrics::pixel_centres(first.nx(), first.x_min_nm, first.x_max_nm);
        let profiles = images
            .iter()
            .map(|g| metrics::centre_profile(&g.data))
            .collect();
        Self::from_profiles(
            x_nm,
            focuses,
            profiles,
            doses,
            *resist,
            FeatureTone::of_mask(mask),
            cd_target_nm,
            cd_tolerance_pct,
        )
    }

    /// Engine-free constructor from precomputed intensity cross-sections.
    ///
    /// `profiles[j]` is one full period of the (periodic) image at
    /// `focuses[j]`, sampled at the uniform pixel centres `x_nm`, normalized
    /// to clear-field intensity. The measured feature at each focus is the
    /// `tone` feature nearest the field centre at the nominal-dose threshold
    /// (nominal dose = median of `doses`), tracked through its
    /// extreme-intensity point at every other dose.
    #[allow(clippy::too_many_arguments)]
    pub fn from_profiles(
        x_nm: Vec<f64>,
        focuses: &[f64],
        profiles: Vec<Vec<f64>>,
        doses: &[f64],
        resist: ThresholdResist,
        tone: FeatureTone,
        cd_target_nm: f64,
        cd_tolerance_pct: f64,
    ) -> Result<Self> {
        validate_doses(doses)?;
        validate_focuses(focuses)?;
        validate_spec(cd_target_nm, cd_tolerance_pct)?;
        ThresholdResist::new(resist.dose_to_clear_mj_cm2)?;
        if profiles.len() != focuses.len() {
            return Err(LithographyError::DimensionMismatch {
                expected: format!("{} profiles (one per focus)", focuses.len()),
                got: format!("{} profiles", profiles.len()),
            });
        }
        for p in &profiles {
            if p.len() != x_nm.len() {
                return Err(LithographyError::DimensionMismatch {
                    expected: format!("profiles of length {} (= x_nm)", x_nm.len()),
                    got: format!("a profile of length {}", p.len()),
                });
            }
        }
        let t_nom = resist.intensity_threshold(median(doses));
        let feature_ref_nm: Vec<Option<f64>> = profiles
            .iter()
            .map(|p| metrics::feature_reference_point(p, &x_nm, t_nom, tone))
            .collect();
        if feature_ref_nm.iter().any(|r| r.is_none()) {
            return Err(invalid(
                "x_nm",
                x_nm.len() as f64,
                "must be a uniform grid of at least 3 pixel centres spanning one period",
            ));
        }
        let (n_d, n_f) = (doses.len(), focuses.len());
        let mut cd_matrix = Array2::from_elem((n_d, n_f), f64::NAN);
        for (j, profile) in profiles.iter().enumerate() {
            for (i, &d) in doses.iter().enumerate() {
                let t = resist.intensity_threshold(d);
                if let Some(cd) =
                    Self::printed_cd(profile, &x_nm, t, tone, feature_ref_nm[j].unwrap_or(0.0))
                {
                    cd_matrix[[i, j]] = cd;
                }
            }
        }
        let (lo, hi) = spec_band(cd_target_nm, cd_tolerance_pct);
        let dose_limits = profiles
            .iter()
            .zip(&feature_ref_nm)
            .map(|(p, r)| model_dose_limits(p, &x_nm, r.unwrap_or(0.0), tone, &resist, lo, hi))
            .collect();
        Ok(Self {
            doses: doses.to_vec(),
            focuses: focuses.to_vec(),
            cd_matrix,
            cd_target_nm,
            cd_tolerance_pct,
            tone: Some(tone),
            resist: Some(resist),
            dose_limits,
            x_nm,
            profiles,
            feature_ref_nm,
        })
    }

    /// Engine-free constructor from a tabulated focus–exposure matrix
    /// (`cd_matrix[[dose_index, focus_index]]`, NaN = not measurable), e.g.
    /// measured wafer data. CD is interpolated linearly in dose between
    /// sampled rows.
    pub fn from_cd_matrix(
        doses: &[f64],
        focuses: &[f64],
        cd_matrix: Array2<f64>,
        cd_target_nm: f64,
        cd_tolerance_pct: f64,
    ) -> Result<Self> {
        validate_doses(doses)?;
        validate_focuses(focuses)?;
        validate_spec(cd_target_nm, cd_tolerance_pct)?;
        if cd_matrix.dim() != (doses.len(), focuses.len()) {
            return Err(LithographyError::DimensionMismatch {
                expected: format!("cd_matrix of shape ({}, {})", doses.len(), focuses.len()),
                got: format!("{:?}", cd_matrix.dim()),
            });
        }
        let (lo, hi) = spec_band(cd_target_nm, cd_tolerance_pct);
        let dose_limits = (0..focuses.len())
            .map(|j| {
                let col: Vec<f64> = cd_matrix.column(j).to_vec();
                tabulated_dose_limits(doses, &col, lo, hi)
            })
            .collect();
        Ok(Self {
            doses: doses.to_vec(),
            focuses: focuses.to_vec(),
            cd_matrix,
            cd_target_nm,
            cd_tolerance_pct,
            tone: None,
            resist: None,
            dose_limits,
            x_nm: Vec::new(),
            profiles: Vec::new(),
            feature_ref_nm: vec![None; focuses.len()],
        })
    }

    /// Printed CD of the reference feature at threshold `t`, or `None` when
    /// it does not print or has merged with its neighbours.
    fn printed_cd(
        profile: &[f64],
        x_nm: &[f64],
        t: f64,
        tone: FeatureTone,
        x_ref: f64,
    ) -> Option<f64> {
        let cd = metrics::cd_containing(profile, x_nm, t, tone, x_ref)?;
        (cd.is_finite() && cd > 0.0).then_some(cd)
    }

    /// Nominal dose: the median of the swept doses (mJ/cm²).
    pub fn nominal_dose(&self) -> f64 {
        median(&self.doses)
    }

    /// Lower and upper CD spec limits (nm).
    pub fn spec_limits_nm(&self) -> (f64, f64) {
        spec_band(self.cd_target_nm, self.cd_tolerance_pct)
    }

    /// Focus indices sorted by focus value.
    fn focus_order(&self) -> Vec<usize> {
        let mut idx: Vec<usize> = (0..self.focuses.len()).collect();
        idx.sort_by(|&a, &b| self.focuses[a].total_cmp(&self.focuses[b]));
        idx
    }

    /// CD at an arbitrary dose and focus sample `j` (model data: exact on the
    /// stored cross-section; tabulated data: linear in dose).
    fn cd_at_index(&self, dose: f64, j: usize) -> Option<f64> {
        if !(dose.is_finite() && dose > 0.0) {
            return None;
        }
        match (
            self.resist,
            self.tone,
            self.feature_ref_nm.get(j).copied().flatten(),
        ) {
            (Some(resist), Some(tone), Some(x_ref)) if !self.profiles.is_empty() => {
                let t = resist.intensity_threshold(dose);
                Self::printed_cd(&self.profiles[j], &self.x_nm, t, tone, x_ref)
            }
            _ => {
                let col: Vec<f64> = self.cd_matrix.column(j).to_vec();
                tabulated_cd_at(&self.doses, &col, dose)
            }
        }
    }

    /// CD (nm) at any dose and focus: exact in dose for model data (linear in
    /// dose for tabulated data), linear in focus between sampled focuses.
    /// `None` outside the sampled focus range or where the feature does not
    /// print.
    pub fn cd_at(&self, dose_mj_cm2: f64, focus_nm: f64) -> Option<f64> {
        match self.locate_focus(focus_nm)? {
            FocusPosition::At(j) => self.cd_at_index(dose_mj_cm2, j),
            FocusPosition::Between(j0, j1, f) => {
                let c0 = self.cd_at_index(dose_mj_cm2, j0)?;
                let c1 = self.cd_at_index(dose_mj_cm2, j1)?;
                Some(c0 + f * (c1 - c0))
            }
        }
    }

    /// Where `focus_nm` sits among the focus samples: on a sample, or between
    /// two focus-adjacent samples at fraction `f` from the first.
    fn locate_focus(&self, focus_nm: f64) -> Option<FocusPosition> {
        let order = self.focus_order();
        let tol = 1e-9 * (1.0 + focus_nm.abs());
        if let Some(&j) = order
            .iter()
            .find(|&&j| (self.focuses[j] - focus_nm).abs() <= tol)
        {
            return Some(FocusPosition::At(j));
        }
        order.windows(2).find_map(|w| {
            let (z0, z1) = (self.focuses[w[0]], self.focuses[w[1]]);
            (focus_nm > z0 && focus_nm < z1)
                .then(|| FocusPosition::Between(w[0], w[1], (focus_nm - z0) / (z1 - z0)))
        })
    }

    /// Bossung curve at `dose`: `(focus, CD)` sorted by focus.
    fn bossung_at(&self, dose: f64) -> Vec<(f64, Option<f64>)> {
        self.focus_order()
            .into_iter()
            .map(|j| (self.focuses[j], self.cd_at_index(dose, j)))
            .collect()
    }

    /// Focus of zero dCD/dz on the Bossung curve at `dose`: vertex of a
    /// least-squares quadratic through the 5 samples around the extreme CD
    /// (falling back to all finite samples), if it lies inside the sampled
    /// range.
    fn bossung_extremum(&self, dose: f64) -> Option<f64> {
        let pts: Vec<(f64, f64)> = self
            .bossung_at(dose)
            .into_iter()
            .filter_map(|(z, c)| c.map(|c| (z, c)))
            .collect();
        let (_, sign) = quadratic_vertex(&pts)?;
        let ext = if sign < 0.0 {
            (0..pts.len()).max_by(|&a, &b| pts[a].1.total_cmp(&pts[b].1))?
        } else {
            (0..pts.len()).min_by(|&a, &b| pts[a].1.total_cmp(&pts[b].1))?
        };
        let lo = ext.saturating_sub(2).min(pts.len().saturating_sub(5));
        let hi = (lo + 5).min(pts.len());
        let local = &pts[lo..hi];
        if let Some((zv, s)) = quadratic_vertex(local) {
            if s == sign && zv >= local[0].0 && zv <= local[local.len() - 1].0 {
                return Some(zv);
            }
        }
        let (zv, _) = quadratic_vertex(&pts)?;
        (zv >= pts[0].0 && zv <= pts[pts.len() - 1].0).then_some(zv)
    }

    /// Contiguous focus intervals over which the CD at `dose` is in spec,
    /// with the spec crossings interpolated linearly between focus samples.
    fn in_spec_focus_intervals(&self, dose: f64) -> Vec<(f64, f64)> {
        let (lo, hi) = self.spec_limits_nm();
        let pts = self.bossung_at(dose);
        let ok = |c: Option<f64>| c.is_some_and(|c| c >= lo && c <= hi);
        let crossing = |(za, ca): (f64, f64), (zb, cb): (f64, f64)| -> f64 {
            // (za, ca) out of spec, (zb, cb) in spec.
            let bound = if ca < lo { lo } else { hi };
            if cb == ca {
                zb
            } else {
                za + (bound - ca) / (cb - ca) * (zb - za)
            }
        };
        let mut out = Vec::new();
        let mut k = 0;
        while k < pts.len() {
            if !ok(pts[k].1) {
                k += 1;
                continue;
            }
            let start = k;
            while k + 1 < pts.len() && ok(pts[k + 1].1) {
                k += 1;
            }
            let end = k;
            let mut z0 = pts[start].0;
            if start > 0 {
                if let (Some(ca), Some(cb)) = (pts[start - 1].1, pts[start].1) {
                    z0 = crossing((pts[start - 1].0, ca), (pts[start].0, cb));
                }
            }
            let mut z1 = pts[end].0;
            if end + 1 < pts.len() {
                if let (Some(ca), Some(cb)) = (pts[end + 1].1, pts[end].1) {
                    z1 = crossing((pts[end + 1].0, ca), (pts[end].0, cb));
                }
            }
            out.push((z0, z1));
            k += 1;
        }
        out
    }

    /// Best-focus estimate at `dose`: Bossung extremum, else the centre of
    /// the longest in-spec focus interval, else the sampled focus nearest 0.
    fn focus_estimate(&self, dose: f64) -> f64 {
        if let Some(z) = self.bossung_extremum(dose) {
            return z;
        }
        if let Some((a, b)) = self
            .in_spec_focus_intervals(dose)
            .into_iter()
            .max_by(|x, y| (x.1 - x.0).total_cmp(&(y.1 - y.0)))
        {
            return 0.5 * (a + b);
        }
        self.focuses
            .iter()
            .copied()
            .min_by(|a, b| a.abs().total_cmp(&b.abs()))
            .unwrap_or(0.0)
    }

    /// Best focus (nm): where dCD/dz = 0 on the Bossung curve at the
    /// dose-to-size (least-squares quadratic vertex). The dose-to-size is
    /// first found at the nominal dose's Bossung extremum, then the extremum
    /// is re-evaluated at that dose (one fixed-point step). Falls back to the
    /// centre of the in-spec focus interval, then to the sampled focus
    /// nearest zero.
    pub fn best_focus(&self) -> f64 {
        let d0 = self.nominal_dose();
        let z0 = self.focus_estimate(d0);
        let d1 = self.dose_to_size_at(z0).unwrap_or(d0);
        self.focus_estimate(d1)
    }

    /// Dose (mJ/cm²) that prints the target CD at focus sample `j`.
    fn dose_to_size_index(&self, j: usize) -> Option<f64> {
        let target = self.cd_target_nm;
        match (
            self.resist,
            self.tone,
            self.feature_ref_nm.get(j).copied().flatten(),
        ) {
            (Some(resist), Some(tone), Some(x_ref)) if !self.profiles.is_empty() => {
                let p = &self.profiles[j];
                let cd = |t: f64| {
                    metrics::cd_containing(p, &self.x_nm, t, tone, x_ref).unwrap_or(f64::NAN)
                };
                let (t_a, t_b) = threshold_bracket(p)?;
                let (pa, pb) = (cd(t_a) >= target, cd(t_b) >= target);
                if pa == pb {
                    return None;
                }
                let t = bisect_boundary(t_a, t_b, pa, |t| cd(t) >= target);
                (t > 0.0).then(|| resist.dose_for_threshold(t))
            }
            _ => {
                let col: Vec<f64> = self.cd_matrix.column(j).to_vec();
                let mut pts: Vec<(f64, f64)> = self
                    .doses
                    .iter()
                    .copied()
                    .zip(col.iter().copied())
                    .collect();
                pts.sort_by(|a, b| a.0.total_cmp(&b.0));
                let d_nom = self.nominal_dose();
                let mut best: Option<f64> = None;
                for w in pts.windows(2) {
                    let ((d0, c0), (d1, c1)) = (w[0], w[1]);
                    if !(c0.is_finite() && c1.is_finite()) {
                        continue;
                    }
                    let (cmin, cmax) = (c0.min(c1), c0.max(c1));
                    if target < cmin || target > cmax {
                        continue;
                    }
                    let d = if c1 == c0 {
                        d0
                    } else {
                        d0 + (target - c0) / (c1 - c0) * (d1 - d0)
                    };
                    if best.is_none_or(|b| (d - d_nom).abs() < (b - d_nom).abs()) {
                        best = Some(d);
                    }
                }
                best
            }
        }
    }

    /// Dose-to-size (mJ/cm²) at any focus inside the sampled range (linear
    /// between focus samples).
    pub fn dose_to_size_at(&self, focus_nm: f64) -> Option<f64> {
        match self.locate_focus(focus_nm)? {
            FocusPosition::At(j) => self.dose_to_size_index(j),
            FocusPosition::Between(j0, j1, f) => {
                let d0 = self.dose_to_size_index(j0)?;
                let d1 = self.dose_to_size_index(j1)?;
                Some(d0 + f * (d1 - d0))
            }
        }
    }

    /// Dose-to-size (mJ/cm²) at [`Self::best_focus`].
    pub fn dose_to_size(&self) -> Option<f64> {
        self.dose_to_size_at(self.best_focus())
    }

    /// In-spec dose interval at any focus inside the sampled range (the
    /// per-sample [`Self::dose_limits`], linear between focus samples).
    pub fn dose_limits_at(&self, focus_nm: f64) -> Option<(f64, f64)> {
        match self.locate_focus(focus_nm)? {
            FocusPosition::At(j) => self.dose_limits[j],
            FocusPosition::Between(j0, j1, f) => {
                let (a0, b0) = self.dose_limits[j0]?;
                let (a1, b1) = self.dose_limits[j1]?;
                Some((a0 + f * (a1 - a0), b0 + f * (b1 - b0)))
            }
        }
    }

    /// Depth of focus (nm) at the dose-to-size: the length of the contiguous
    /// focus interval around [`Self::best_focus`] over which the CD stays in
    /// spec, with the spec crossings interpolated between focus samples (the
    /// longest in-spec interval if none contains best focus; 0 if none).
    /// For the DOF of a process-window rectangle with a required exposure
    /// latitude use [`Self::dof_at_el`].
    pub fn depth_of_focus(&self) -> f64 {
        let zb = self.best_focus();
        let dose = self.dose_to_size_at(zb).unwrap_or(self.nominal_dose());
        let intervals = self.in_spec_focus_intervals(dose);
        let chosen = intervals
            .iter()
            .copied()
            .find(|&(a, b)| zb >= a && zb <= b)
            .or_else(|| {
                intervals
                    .iter()
                    .copied()
                    .max_by(|x, y| (x.1 - x.0).total_cmp(&(y.1 - y.0)))
            });
        chosen.map_or(0.0, |(a, b)| (b - a).max(0.0))
    }

    /// Exposure latitude (%) at [`Self::best_focus`]: the in-spec dose range
    /// there relative to its centre dose (0 if nothing is in spec).
    pub fn exposure_latitude(&self) -> f64 {
        self.dose_limits_at(self.best_focus())
            .map_or(0.0, |(a, b)| el_pct(a, b))
    }

    /// Extract Bossung curves (CD vs focus at each sampled dose; NaN CDs are
    /// passed through).
    pub fn bossung_curves(&self) -> Vec<Vec<BossungPoint>> {
        self.doses
            .iter()
            .enumerate()
            .map(|(i, &dose)| {
                self.focuses
                    .iter()
                    .enumerate()
                    .map(|(j, &focus)| BossungPoint {
                        dose_mj_cm2: dose,
                        focus_nm: focus,
                        cd_nm: self.cd_matrix[[i, j]],
                    })
                    .collect()
            })
            .collect()
    }

    /// Iso-focal dose (mJ/cm²): the dose in the swept range at which the CD
    /// is least sensitive to focus, i.e. minimizing
    /// `max_z CD − min_z CD` over the focus samples where the feature prints
    /// at every swept dose. `None` if fewer than 3 such focus samples exist
    /// or the dose range is degenerate.
    pub fn iso_focal_dose(&self) -> Option<f64> {
        let support: Vec<usize> = (0..self.focuses.len())
            .filter(|&j| (0..self.doses.len()).all(|i| self.cd_matrix[[i, j]].is_finite()))
            .collect();
        if support.len() < 3 {
            return None;
        }
        let d_lo = self.doses.iter().copied().fold(f64::INFINITY, f64::min);
        let d_hi = self.doses.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        if d_hi <= d_lo {
            return None;
        }
        let spread = |d: f64| -> f64 {
            let mut mn = f64::INFINITY;
            let mut mx = f64::NEG_INFINITY;
            for &j in &support {
                match self.cd_at_index(d, j) {
                    Some(c) => {
                        mn = mn.min(c);
                        mx = mx.max(c);
                    }
                    None => return f64::INFINITY,
                }
            }
            mx - mn
        };
        let n = 121;
        let ratio = d_hi / d_lo;
        let grid: Vec<f64> = (0..n)
            .map(|k| d_lo * ratio.powf(k as f64 / (n - 1) as f64))
            .collect();
        let vals: Vec<f64> = grid.iter().map(|&d| spread(d)).collect();
        let k = (0..n).min_by(|&a, &b| vals[a].total_cmp(&vals[b]))?;
        if !vals[k].is_finite() {
            return None;
        }
        // Golden-section refinement between the neighbouring grid doses.
        let (mut a, mut b) = (grid[k.saturating_sub(1)], grid[(k + 1).min(n - 1)]);
        let g = 0.5 * (5.0_f64.sqrt() - 1.0);
        let mut c = b - g * (b - a);
        let mut e = a + g * (b - a);
        let (mut fc, mut fe) = (spread(c), spread(e));
        for _ in 0..80 {
            if fc <= fe {
                b = e;
                e = c;
                fe = fc;
                c = b - g * (b - a);
                fc = spread(c);
            } else {
                a = c;
                c = e;
                fc = fe;
                e = a + g * (b - a);
                fe = spread(e);
            }
        }
        let d = 0.5 * (a + b);
        if spread(d) <= vals[k] {
            Some(d)
        } else {
            Some(grid[k])
        }
    }

    /// Focus-sorted runs of consecutive samples with in-spec doses, refined
    /// with `refine` linearly interpolated sub-steps per focus step:
    /// `(focus, d_min, d_max)` points.
    fn window_segments(&self, refine: usize) -> Vec<Vec<(f64, f64, f64)>> {
        let order = self.focus_order();
        let mut segs: Vec<Vec<(f64, f64, f64)>> = Vec::new();
        let mut current: Vec<(f64, f64, f64)> = Vec::new();
        for &j in &order {
            let z = self.focuses[j];
            match self.dose_limits[j] {
                Some((a, b)) if b >= a => {
                    if let Some(&(z0, a0, b0)) = current.last() {
                        if z > z0 {
                            for s in 1..refine {
                                let f = s as f64 / refine as f64;
                                current.push((
                                    z0 + f * (z - z0),
                                    a0 + f * (a - a0),
                                    b0 + f * (b - b0),
                                ));
                            }
                            current.push((z, a, b));
                        } else {
                            // Duplicate focus: keep the intersection.
                            let last = current.last_mut().expect("non-empty");
                            last.1 = last.1.max(a);
                            last.2 = last.2.min(b);
                        }
                    } else {
                        current.push((z, a, b));
                    }
                }
                _ => {
                    if !current.is_empty() {
                        segs.push(std::mem::take(&mut current));
                    }
                }
            }
        }
        if !current.is_empty() {
            segs.push(current);
        }
        segs
    }

    /// Best (largest-EL) in-window rectangle of focus extent exactly `dof`
    /// within the refined segments, if any is feasible.
    fn best_rectangle(segs: &[Vec<(f64, f64, f64)>], dof: f64) -> Option<ProcessRectangle> {
        let mut best: Option<(f64, ProcessRectangle)> = None;
        for seg in segs {
            let m = seg.len();
            let z_last = seg[m - 1].0;
            for i in 0..m {
                let z_start = seg[i].0;
                let z_end = z_start + dof;
                if z_end > z_last + 1e-9 * (1.0 + z_last.abs()) {
                    break;
                }
                let mut a_max = f64::NEG_INFINITY;
                let mut b_min = f64::INFINITY;
                let mut k = i;
                while k < m && seg[k].0 <= z_end {
                    a_max = a_max.max(seg[k].1);
                    b_min = b_min.min(seg[k].2);
                    k += 1;
                }
                if k < m && k > i && seg[k - 1].0 < z_end {
                    // Interpolated limits at the exact window end.
                    let (z0, a0, b0) = seg[k - 1];
                    let (z1, a1, b1) = seg[k];
                    let f = (z_end - z0) / (z1 - z0);
                    a_max = a_max.max(a0 + f * (a1 - a0));
                    b_min = b_min.min(b0 + f * (b1 - b0));
                }
                if b_min >= a_max {
                    let el = el_pct(a_max, b_min);
                    if best.is_none_or(|(e, _)| el > e) {
                        best = Some((
                            el,
                            ProcessRectangle {
                                focus_min_nm: z_start,
                                focus_max_nm: z_end.min(z_last),
                                dose_min_mj_cm2: a_max,
                                dose_max_mj_cm2: b_min,
                            },
                        ));
                    }
                }
            }
        }
        best.map(|(_, r)| r)
    }

    /// Longest focus extent available in any contiguous in-spec run (nm).
    fn max_window_dof(segs: &[Vec<(f64, f64, f64)>]) -> f64 {
        segs.iter()
            .map(|s| s[s.len() - 1].0 - s[0].0)
            .fold(0.0, f64::max)
    }

    /// Sub-steps per focus step used by the rectangle search.
    const WINDOW_REFINE: usize = 16;

    /// Exposure–defocus curve: `n_points` DOF values from 0 to the widest
    /// in-spec focus run, each with the largest exposure latitude (%) of any
    /// in-window rectangle of that focus extent (0 where none fits).
    pub fn el_vs_dof(&self, n_points: usize) -> Vec<(f64, f64)> {
        let segs = self.window_segments(Self::WINDOW_REFINE);
        if segs.is_empty() || n_points == 0 {
            return Vec::new();
        }
        let d_max = Self::max_window_dof(&segs);
        (0..n_points)
            .map(|k| {
                let dof = if n_points == 1 {
                    0.0
                } else {
                    d_max * k as f64 / (n_points - 1) as f64
                };
                let el =
                    Self::best_rectangle(&segs, dof).map_or(0.0, |r| r.exposure_latitude_pct());
                (dof, el)
            })
            .collect()
    }

    /// Largest-EL in-window rectangle with focus extent `dof_nm`.
    pub fn el_at_dof(&self, dof_nm: f64) -> Option<ProcessRectangle> {
        if dof_nm.is_nan() || dof_nm < 0.0 {
            return None;
        }
        let segs = self.window_segments(Self::WINDOW_REFINE);
        Self::best_rectangle(&segs, dof_nm)
    }

    /// Largest-DOF in-window rectangle whose exposure latitude is at least
    /// `el_pct` (e.g. `dof_at_el(5.0)` = DOF at 5 % EL). The returned
    /// rectangle spans the full in-spec dose interval of its focus window
    /// (EL ≥ `el_pct`). `None` if no rectangle reaches that latitude.
    pub fn dof_at_el(&self, el_pct: f64) -> Option<ProcessRectangle> {
        if !el_pct.is_finite() {
            return None;
        }
        let segs = self.window_segments(Self::WINDOW_REFINE);
        if segs.is_empty() {
            return None;
        }
        let feasible = |dof: f64| {
            Self::best_rectangle(&segs, dof).filter(|r| r.exposure_latitude_pct() >= el_pct)
        };
        let first = feasible(0.0)?;
        let d_max = Self::max_window_dof(&segs);
        if let Some(r) = feasible(d_max) {
            return Some(r);
        }
        let (mut lo, mut hi) = (0.0_f64, d_max);
        let mut best = first;
        for _ in 0..60 {
            let mid = 0.5 * (lo + hi);
            match feasible(mid) {
                Some(r) => {
                    lo = mid;
                    best = r;
                }
                None => hi = mid,
            }
            if hi - lo <= 1e-9 * d_max.max(1e-12) {
                break;
            }
        }
        Some(best)
    }

    /// In-window rectangle maximizing the product DOF × EL (a simple
    /// process-window "area" figure of merit), searched over 101 DOF values.
    pub fn max_area_rectangle(&self) -> Option<ProcessRectangle> {
        let segs = self.window_segments(Self::WINDOW_REFINE);
        if segs.is_empty() {
            return None;
        }
        let d_max = Self::max_window_dof(&segs);
        (0..101)
            .filter_map(|k| Self::best_rectangle(&segs, d_max * k as f64 / 100.0))
            .max_by(|a, b| {
                (a.dof_nm() * a.exposure_latitude_pct())
                    .total_cmp(&(b.dof_nm() * b.exposure_latitude_pct()))
            })
    }
}

/// Spec band `target·(1 ∓ tol/100)`.
fn spec_band(target: f64, tol_pct: f64) -> (f64, f64) {
    (
        target * (1.0 - tol_pct / 100.0),
        target * (1.0 + tol_pct / 100.0),
    )
}

/// Threshold bracket just outside the finite intensity range of a profile.
fn threshold_bracket(profile: &[f64]) -> Option<(f64, f64)> {
    let (mut mn, mut mx) = (f64::INFINITY, f64::NEG_INFINITY);
    for &v in profile.iter().filter(|v| v.is_finite()) {
        mn = mn.min(v);
        mx = mx.max(v);
    }
    if mx <= mn {
        return None;
    }
    let pad = 1e-9 * (mx - mn);
    Some((mn - pad, mx + pad))
}

/// In-spec dose interval of the reference feature on one cross-section:
/// the CD is monotone in the threshold (non-decreasing for dark features,
/// non-increasing for bright), so the spec limits map to a threshold
/// interval found by bisection, then to doses via `d = E_th/t`.
fn model_dose_limits(
    profile: &[f64],
    x_nm: &[f64],
    x_ref: f64,
    tone: FeatureTone,
    resist: &ThresholdResist,
    lo: f64,
    hi: f64,
) -> Option<(f64, f64)> {
    let cd = |t: f64| metrics::cd_containing(profile, x_nm, t, tone, x_ref).unwrap_or(f64::NAN);
    let (t_a, t_b) = threshold_bracket(profile)?;
    let ge_lo = |t: f64| cd(t) >= lo;
    let le_hi = |t: f64| cd(t) <= hi;
    let (p_lo_a, p_lo_b) = (ge_lo(t_a), ge_lo(t_b));
    let (p_hi_a, p_hi_b) = (le_hi(t_a), le_hi(t_b));
    // Boundaries of each monotone predicate (or the bracket end if it never
    // flips).
    let b_lo = if p_lo_a != p_lo_b {
        Some(bisect_boundary(t_a, t_b, p_lo_a, ge_lo))
    } else {
        None
    };
    let b_hi = if p_hi_a != p_hi_b {
        Some(bisect_boundary(t_a, t_b, p_hi_a, le_hi))
    } else {
        None
    };
    let (t_min, t_max) = match tone {
        // CD grows with t: CD ≥ lo for t ≥ b_lo; CD ≤ hi for t ≤ b_hi.
        FeatureTone::Dark => {
            let t_min = if p_lo_a { t_a } else { b_lo? };
            let t_max = if p_hi_b { t_b } else { b_hi? };
            (t_min, t_max)
        }
        // CD shrinks with t: CD ≤ hi for t ≥ b_hi; CD ≥ lo for t ≤ b_lo.
        FeatureTone::Bright => {
            let t_min = if p_hi_a { t_a } else { b_hi? };
            let t_max = if p_lo_b { t_b } else { b_lo? };
            (t_min, t_max)
        }
    };
    if t_max.is_nan() || t_min.is_nan() || t_max <= t_min {
        return None;
    }
    let mid = cd(0.5 * (t_min + t_max));
    if !(mid >= lo && mid <= hi) {
        return None;
    }
    let floor = 1e-12 * t_b.abs().max(1e-300);
    let d_min = resist.dose_for_threshold(t_max.max(floor));
    let d_max = resist.dose_for_threshold(t_min.max(floor));
    Some((d_min, d_max))
}

/// Batch simulation: compute aerial images for multiple defocus values
/// (mask spectrum once, focus planes in parallel).
/// Returns Vec of (focus_nm, aerial_image_data).
pub fn batch_defocus(
    engine: &AerialImageEngine,
    mask: &Mask,
    focuses: &[f64],
) -> Vec<(f64, Array2<f64>)> {
    focuses
        .iter()
        .zip(engine.compute_through_focus(mask, focuses))
        .map(|(&focus, image)| (focus, image.data))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use std::f64::consts::PI;

    // Synthetic analytic window (numpy-checked closed forms):
    // I(x; z) = a − b(z)·cos(2πx/p), b(z) = b0·exp(−((z − z0)/s)²),
    // E_th = 9 mJ/cm² (threshold 0.3 at 30 mJ/cm²), target 60 nm ± 10 %.
    const A: f64 = 0.5;
    const B0: f64 = 0.45;
    const Z0: f64 = 40.0;
    const S: f64 = 150.0;
    const P: f64 = 180.0;
    const E_TH: f64 = 9.0;

    fn b_of(z: f64) -> f64 {
        B0 * (-((z - Z0) / S).powi(2)).exp()
    }

    fn engine_x(n: usize) -> Vec<f64> {
        let h = P / n as f64;
        (0..n).map(|j| -P / 2.0 + (j as f64 + 0.5) * h).collect()
    }

    fn synthetic_window(focuses: &[f64], doses: &[f64]) -> ProcessWindow {
        let x = engine_x(128);
        let profiles: Vec<Vec<f64>> = focuses
            .iter()
            .map(|&z| {
                let b = b_of(z);
                x.iter()
                    .map(|&x| A - b * (2.0 * PI * x / P).cos())
                    .collect()
            })
            .collect();
        ProcessWindow::from_profiles(
            x,
            focuses,
            profiles,
            doses,
            ThresholdResist::new(E_TH).unwrap(),
            FeatureTone::Dark,
            60.0,
            10.0,
        )
        .unwrap()
    }

    fn sym_focuses() -> Vec<f64> {
        // Symmetric about Z0 = 40, 10 nm steps.
        (0..41).map(|k| Z0 - 200.0 + 10.0 * k as f64).collect()
    }

    fn sweep_doses() -> Vec<f64> {
        (0..31).map(|k| 15.0 + k as f64).collect() // 15 … 45, median 30
    }

    #[test]
    fn test_threshold_resist_conventions() {
        let r = ThresholdResist::from_nominal(0.3, 30.0).unwrap();
        assert_relative_eq!(r.dose_to_clear_mj_cm2, 9.0, epsilon = 1e-12);
        assert_relative_eq!(r.intensity_threshold(30.0), 0.3, epsilon = 1e-12);
        // Doubling the dose halves the printed intensity contour.
        assert_relative_eq!(r.intensity_threshold(60.0), 0.15, epsilon = 1e-12);
        assert_relative_eq!(r.dose_for_threshold(0.45), 20.0, epsilon = 1e-12);
        assert!(ThresholdResist::new(0.0).is_err());
        assert!(ThresholdResist::new(f64::NAN).is_err());
        assert!(ThresholdResist::from_nominal(-0.3, 30.0).is_err());
    }

    #[test]
    fn test_median_nominal_dose() {
        assert_relative_eq!(median(&[35.0, 25.0, 30.0]), 30.0);
        assert_relative_eq!(median(&[20.0, 25.0, 35.0, 40.0]), 30.0);
    }

    #[test]
    fn test_synthetic_dose_limits_closed_form() {
        // In spec ⇔ a − b(z)·cos(π·lo/p) ≤ t ≤ a − b(z)·cos(π·hi/p);
        // numpy at z0: d ∈ [28.3939877261, 38.2171063458], EL 29.4939416821 %.
        let focuses = sym_focuses();
        let pw = synthetic_window(&focuses, &sweep_doses());
        let (lo, hi) = pw.spec_limits_nm();
        let (clo, chi) = ((PI * lo / P).cos(), (PI * hi / P).cos());
        for (j, &z) in focuses.iter().enumerate() {
            let b = b_of(z);
            let expect = (E_TH / (A - b * chi), E_TH / (A - b * clo));
            let (d0, d1) = pw.dose_limits[j].expect("in spec at every focus here");
            assert_relative_eq!(d0, expect.0, max_relative = 2e-4);
            assert_relative_eq!(d1, expect.1, max_relative = 2e-4);
        }
        let j0 = focuses.iter().position(|&z| z == Z0).unwrap();
        let (d0, d1) = pw.dose_limits[j0].unwrap();
        assert_relative_eq!(d0, 28.3939877261, max_relative = 2e-4);
        assert_relative_eq!(d1, 38.2171063458, max_relative = 2e-4);
        assert_relative_eq!(el_pct(d0, d1), 29.4939416821, max_relative = 5e-4);
    }

    #[test]
    fn test_synthetic_cd_matrix_is_dose_dependent() {
        let focuses = sym_focuses();
        let doses = vec![25.0, 30.0, 35.0];
        let pw = synthetic_window(&focuses, &doses);
        let j0 = focuses.iter().position(|&z| z == Z0).unwrap();
        // CD at (d, z0) = (p/π)·arccos((a − E_th/d)/b0) — dark line
        // narrows as dose rises.
        for (i, &d) in doses.iter().enumerate() {
            let exact = (P / PI) * ((A - E_TH / d) / B0).acos();
            assert_relative_eq!(pw.cd_matrix[[i, j0]], exact, epsilon = 2e-3);
        }
        assert!(pw.cd_matrix[[0, j0]] > pw.cd_matrix[[1, j0]]);
        assert!(pw.cd_matrix[[1, j0]] > pw.cd_matrix[[2, j0]]);
        assert_relative_eq!(
            pw.cd_at(30.0, Z0).unwrap(),
            pw.cd_matrix[[1, j0]],
            epsilon = 1e-12
        );
    }

    #[test]
    fn test_synthetic_best_focus_dose_to_size_dof_el() {
        let focuses = sym_focuses();
        let pw = synthetic_window(&focuses, &sweep_doses());
        // Symmetric Bossung curves about z0.
        assert_relative_eq!(pw.best_focus(), Z0, epsilon = 0.5);
        // Dose-to-size at z0: t* = a − b0·cos(π·60/180) = 0.275 →
        // d* = 32.727272727273.
        assert_relative_eq!(
            pw.dose_to_size().unwrap(),
            32.727272727273,
            max_relative = 1e-4
        );
        // DOF at the dose-to-size: b(z) ≥ (a − t*)/cos(π·54/180)
        // → 120.6557951451 nm (focus sampled every 10 nm, linear crossing).
        assert_relative_eq!(pw.depth_of_focus(), 120.6557951451, max_relative = 5e-3);
        // EL at best focus: 29.4939416821 %.
        assert_relative_eq!(pw.exposure_latitude(), 29.4939416821, max_relative = 2e-3);
    }

    #[test]
    fn test_synthetic_ed_window_closed_form() {
        // Centred rectangle of focus extent D: d1 = E/(a − b0·chi),
        // d2 = E/(a − b(z0 ± D/2)·clo). numpy: EL(100 nm) = 18.4933043916 %,
        // DOF@5 %EL = 160.7446581188, DOF@10 %EL = 139.2770211412 nm.
        let focuses = sym_focuses();
        let pw = synthetic_window(&focuses, &sweep_doses());
        let r = pw.el_at_dof(100.0).unwrap();
        assert_relative_eq!(
            r.exposure_latitude_pct(),
            18.4933043916,
            max_relative = 3e-3
        );
        assert_relative_eq!(r.focus_centre_nm(), Z0, epsilon = 1.0);
        let r5 = pw.dof_at_el(5.0).unwrap();
        assert_relative_eq!(r5.dof_nm(), 160.7446581188, max_relative = 3e-3);
        assert!(r5.exposure_latitude_pct() >= 5.0 - 1e-9);
        let r10 = pw.dof_at_el(10.0).unwrap();
        assert_relative_eq!(r10.dof_nm(), 139.2770211412, max_relative = 3e-3);
        // The EL–DOF curve starts at the best single-focus EL and never
        // increases with DOF.
        let curve = pw.el_vs_dof(21);
        assert_eq!(curve.len(), 21);
        assert_relative_eq!(curve[0].1, 29.4939416821, max_relative = 2e-3);
        for w in curve.windows(2) {
            assert!(w[1].1 <= w[0].1 + 1e-9);
        }
        // No rectangle reaches 40 % EL.
        assert!(pw.dof_at_el(40.0).is_none());
        let area = pw.max_area_rectangle().unwrap();
        assert!(area.dof_nm() > 0.0 && area.exposure_latitude_pct() > 0.0);
    }

    #[test]
    fn test_synthetic_iso_focal_dose() {
        // At t = a the CD is p/2 at every focus → iso-focal dose E/a = 18.
        let focuses = sym_focuses();
        let pw = synthetic_window(&focuses, &sweep_doses());
        assert_relative_eq!(pw.iso_focal_dose().unwrap(), 18.0, max_relative = 1e-4);
    }

    #[test]
    fn test_bright_tone_window() {
        // Same image read as a bright space of width p − CD_dark: the
        // bright feature widens with dose; target 120 nm ± 10 %.
        let focuses = sym_focuses();
        let x = engine_x(128);
        let profiles: Vec<Vec<f64>> = focuses
            .iter()
            .map(|&z| {
                let b = b_of(z);
                x.iter()
                    .map(|&x| A - b * (2.0 * PI * x / P).cos())
                    .collect()
            })
            .collect();
        let doses = vec![25.0, 30.0, 35.0];
        let pw = ProcessWindow::from_profiles(
            x,
            &focuses,
            profiles,
            &doses,
            ThresholdResist::new(E_TH).unwrap(),
            FeatureTone::Bright,
            120.0,
            10.0,
        )
        .unwrap();
        let j0 = focuses.iter().position(|&z| z == Z0).unwrap();
        for (i, &d) in doses.iter().enumerate() {
            let dark = (P / PI) * ((A - E_TH / d) / B0).acos();
            assert_relative_eq!(pw.cd_matrix[[i, j0]], P - dark, epsilon = 2e-3);
        }
        assert!(pw.cd_matrix[[2, j0]] > pw.cd_matrix[[0, j0]]);
        // In spec ⇔ 108 ≤ p − CD_dark ≤ 132 ⇔ 48 ≤ CD_dark ≤ 72.
        let (d0, d1) = pw.dose_limits[j0].unwrap();
        let (c48, c72) = ((PI * 48.0 / P).cos(), (PI * 72.0 / P).cos());
        assert_relative_eq!(d0, E_TH / (A - B0 * c72), max_relative = 2e-4);
        assert_relative_eq!(d1, E_TH / (A - B0 * c48), max_relative = 2e-4);
    }

    #[test]
    fn test_tabulated_matrix_interpolates_dose() {
        // Linear CD(d) = 100 − d at every focus: spec 60 ± 10 % → d ∈ [34, 46].
        let doses = vec![30.0, 40.0, 50.0];
        let focuses = vec![-50.0, 0.0, 50.0];
        let m = Array2::from_shape_fn((3, 3), |(i, _)| 100.0 - doses[i]);
        let pw = ProcessWindow::from_cd_matrix(&doses, &focuses, m, 60.0, 10.0).unwrap();
        for lim in &pw.dose_limits {
            let (a, b) = lim.unwrap();
            assert_relative_eq!(a, 34.0, epsilon = 1e-9);
            assert_relative_eq!(b, 46.0, epsilon = 1e-9);
        }
        assert_relative_eq!(pw.exposure_latitude(), 200.0 * 12.0 / 80.0, epsilon = 1e-9);
        assert_relative_eq!(pw.dose_to_size().unwrap(), 40.0, epsilon = 1e-9);
        // Flat Bossung curves: DOF spans the whole sampled range.
        assert_relative_eq!(pw.depth_of_focus(), 100.0, epsilon = 1e-9);
        let r = pw.dof_at_el(10.0).unwrap();
        assert_relative_eq!(r.dof_nm(), 100.0, epsilon = 1e-6);
        assert!(pw.tone.is_none() && pw.resist.is_none());
        assert!(
            ProcessWindow::from_cd_matrix(&doses, &focuses, Array2::zeros((2, 3)), 60.0, 10.0)
                .is_err()
        );
    }

    #[test]
    fn test_bossung_curves_shape() {
        let pw = ProcessWindow::from_cd_matrix(
            &[25.0, 30.0, 35.0],
            &[-100.0, 0.0, 100.0],
            Array2::from_shape_vec(
                (3, 3),
                vec![60.0, 65.0, 62.0, 58.0, 63.0, 60.0, 55.0, 60.0, 57.0],
            )
            .unwrap(),
            63.0,
            10.0,
        )
        .unwrap();

        let curves = pw.bossung_curves();
        assert_eq!(curves.len(), 3); // one per dose
        assert_eq!(curves[0].len(), 3); // one per focus
    }

    #[test]
    fn test_dof_positive_for_valid_pw() {
        // target=65, tol=10% -> range [58.5, 71.5]
        let pw = ProcessWindow::from_cd_matrix(
            &[25.0, 30.0, 35.0],
            &[-200.0, -100.0, 0.0, 100.0, 200.0],
            Array2::from_shape_vec(
                (3, 5),
                vec![
                    50.0, 60.0, 65.0, 60.0, 50.0, // dose 25: 60 is in [58.5,71.5]
                    55.0, 63.0, 67.0, 63.0, 55.0, // dose 30
                    58.0, 66.0, 70.0, 66.0, 58.0, // dose 35
                ],
            )
            .unwrap(),
            65.0,
            10.0,
        )
        .unwrap();

        // Dose-to-size at best focus 0 is 25 (CD 65); in spec there from
        // the interpolated crossing at −115 nm to +115 nm.
        assert_relative_eq!(pw.best_focus(), 0.0, epsilon = 1e-9);
        assert_relative_eq!(pw.dose_to_size().unwrap(), 25.0, epsilon = 1e-9);
        let dof = pw.depth_of_focus();
        assert_relative_eq!(dof, 230.0, epsilon = 1e-9);
    }

    #[test]
    fn test_el_positive_for_valid_pw() {
        // target=65, tol=10% -> range [58.5, 71.5]
        let pw = ProcessWindow::from_cd_matrix(
            &[20.0, 25.0, 30.0, 35.0, 40.0],
            &[-100.0, 0.0, 100.0],
            Array2::from_shape_vec(
                (5, 3),
                vec![
                    55.0, 60.0, 55.0, // dose 20: 60 in range
                    58.0, 63.0, 58.0, // dose 25: 63 in range
                    60.0, 65.0, 60.0, // dose 30: 65 in range, 60 in range
                    62.0, 68.0, 62.0, // dose 35: 68 in range, 62 in range
                    64.0, 71.0, 64.0, // dose 40: 71 in range, 64 in range
                ],
            )
            .unwrap(),
            65.0,
            10.0,
        )
        .unwrap();

        // Every sampled dose is in spec at best focus: EL = 200·20/60 %.
        let el = pw.exposure_latitude();
        assert_relative_eq!(el, 200.0 * 20.0 / 60.0, epsilon = 1e-9);
    }

    #[test]
    fn test_invalid_inputs_rejected() {
        let x = engine_x(16);
        let prof = vec![vec![0.5; 16]];
        let r = ThresholdResist::new(9.0).unwrap();
        assert!(ProcessWindow::from_profiles(
            x.clone(),
            &[0.0],
            prof.clone(),
            &[],
            r,
            FeatureTone::Dark,
            60.0,
            10.0
        )
        .is_err());
        assert!(ProcessWindow::from_profiles(
            x.clone(),
            &[0.0],
            prof.clone(),
            &[-1.0],
            r,
            FeatureTone::Dark,
            60.0,
            10.0
        )
        .is_err());
        assert!(ProcessWindow::from_profiles(
            x.clone(),
            &[0.0, 1.0],
            prof.clone(),
            &[30.0],
            r,
            FeatureTone::Dark,
            60.0,
            10.0
        )
        .is_err());
        assert!(ProcessWindow::from_profiles(
            x,
            &[0.0],
            prof,
            &[30.0],
            r,
            FeatureTone::Dark,
            -60.0,
            10.0
        )
        .is_err());
    }

    #[test]
    fn test_flat_image_prints_nothing() {
        // No contrast: no feature, NaN CDs, no window.
        let x = engine_x(32);
        let focuses = vec![-50.0, 0.0, 50.0];
        let profiles = vec![vec![0.4; 32]; 3];
        let pw = ProcessWindow::from_profiles(
            x,
            &focuses,
            profiles,
            &[25.0, 30.0, 35.0],
            ThresholdResist::new(9.0).unwrap(),
            FeatureTone::Dark,
            60.0,
            10.0,
        )
        .unwrap();
        assert!(pw.cd_matrix.iter().all(|c| c.is_nan()));
        assert!(pw.dose_limits.iter().all(|l| l.is_none()));
        assert_eq!(pw.depth_of_focus(), 0.0);
        assert_eq!(pw.exposure_latitude(), 0.0);
        assert!(pw.dof_at_el(5.0).is_none());
        assert!(pw.el_vs_dof(5).is_empty());
    }

    #[test]
    fn test_batch_defocus_returns_correct_count() {
        use crate::aerial::AerialImageEngine;
        use crate::optics::ProjectionOptics;
        use crate::source::VuvSource;
        use crate::types::GridConfig;

        let source = VuvSource::f2_laser(0.5).unwrap();
        let optics = ProjectionOptics::new(0.75).unwrap();
        let grid = GridConfig {
            size: 64,
            pixel_nm: 4.0,
        };
        let engine = AerialImageEngine::new(&source, &optics, grid, 10).unwrap();
        let mask = Mask::line_space(65.0, 180.0).unwrap();

        let focuses = vec![-200.0, -100.0, 0.0, 100.0, 200.0];
        let results = batch_defocus(&engine, &mask, &focuses);
        assert_eq!(results.len(), 5);
        for (focus, image) in &results {
            assert!(focuses.contains(focus));
            assert_eq!(image.dim(), (64, 64));
        }
    }

    #[test]
    fn test_process_window_compute_engine_smoke() {
        use crate::aerial::AerialImageEngine;
        use crate::optics::ProjectionOptics;
        use crate::source::VuvSource;
        use crate::types::GridConfig;

        // 1:1 lines on a 300 nm pitch in a one-period (commensurate) field:
        // the first order (1/300 nm⁻¹) is inside NA/λ, so the image is
        // modulated with any engine version.
        let source = VuvSource::f2_laser(0.5).unwrap();
        let optics = ProjectionOptics::new(0.75).unwrap();
        let grid = GridConfig {
            size: 64,
            pixel_nm: 300.0 / 64.0,
        };
        let engine = AerialImageEngine::new(&source, &optics, grid, 10).unwrap();
        let mask = Mask::line_space(150.0, 300.0).unwrap();

        let doses = vec![25.0, 30.0, 35.0];
        let focuses = vec![-100.0, 0.0, 100.0];
        let pw =
            ProcessWindow::compute(&engine, &mask, &doses, &focuses, 0.3, 150.0, 10.0).unwrap();

        assert_eq!(pw.cd_matrix.dim(), (3, 3));
        assert_eq!(pw.dose_limits.len(), 3);
        assert_eq!(pw.profiles.len(), 3);
        assert_eq!(pw.tone, Some(FeatureTone::Dark));
        assert_relative_eq!(
            pw.resist.unwrap().dose_to_clear_mj_cm2,
            9.0,
            epsilon = 1e-12
        );
        // Dose now matters: the dark line narrows monotonically with dose
        // at best focus (every dose row used to be identical).
        let j0 = 1;
        let (c25, c30, c35) = (
            pw.cd_matrix[[0, j0]],
            pw.cd_matrix[[1, j0]],
            pw.cd_matrix[[2, j0]],
        );
        assert!(c25.is_finite() && c30.is_finite() && c35.is_finite());
        assert!(c25 > c30 && c30 > c35, "CDs {c25}, {c30}, {c35}");
        assert!(pw.best_focus().is_finite());
        assert!(pw.exposure_latitude() >= 0.0);
        assert!(pw.depth_of_focus() >= 0.0);
    }
}
