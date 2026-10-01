//! Lithographic image metrics.
//!
//! Scalar figures of merit computed from an aerial image or its cross-section:
//! critical dimension (CD), image log-slope (ILS), normalized image log-slope
//! (NILS), mask error enhancement factor (MEEF), contrast, and a
//! modulation-based MTF proxy.
//!
//! Every CD/ILS/NILS function locates threshold crossings with one shared,
//! sub-pixel crossing finder: the profile is interpolated with a monotone
//! cubic Hermite interpolant (five-point central-difference node slopes,
//! Fritsch–Carlson limited so the interpolant is monotone on the bracketing
//! segment), the crossing is the unique root of that cubic inside the
//! segment, and the edge slope `dI/dx` is the derivative of the same cubic
//! *at* the crossing.
//!
//! Two families of CD functions exist:
//!
//! - **Periodic, tone-aware** ([`measure_cd_periodic`], [`periodic_features`],
//!   [`image_log_slope`], [`nils_periodic`]): the profile is one full period of
//!   a periodic image on a uniform grid — exactly what the FFT-based imaging
//!   engine produces — so crossings wrap around the field edge, and the caller
//!   states which [`FeatureTone`] is the feature (dark line or bright
//!   space/hole). These are the recommended functions.
//! - **Legacy, open profile** ([`measure_cd`], [`measure_cd_2d`], [`nils`]):
//!   the width between the two consecutive crossings whose midpoint is nearest
//!   the profile centre, whatever the tone. Kept for API compatibility.
//!
//! # Key equations
//!
//! ```text
//!   CD    = x_right − x_left           (threshold crossings bounding the feature)
//!   ILS   = |dI/dx| / I = |dI/dx| / I_threshold      (at the crossing, 1/nm)
//!   NILS  = w · ILS                    (w = measured CD, or a nominal width)
//!   MEEF  = ∂CD_wafer / ∂CD_mask       (both at wafer scale)
//!   contrast = (I_max − I_min) / (I_max + I_min)
//! ```
//!
//! Closed-form check used by the tests: for `I(x) = a − b·cos(2πx/p)` the dark
//! feature centred on `x = 0` has `CD = (p/π)·arccos((a − t)/b)` at threshold
//! `t`, and `dI/dx = b·(2π/p)·sin(π·CD/p)` at its edges.
//!
//! # Model status
//!
//! These are image-plane metrics: CD here is the width of an intensity
//! threshold contour (a constant-threshold resist), not a developed resist
//! profile. For smooth (band-limited) images the crossing position is
//! fourth-order accurate in the sample spacing and the edge slope
//! third-order (on the cosine fixture at 20 samples per period: CD error
//! ≤ 1.3e-3 nm, NILS error ≤ 2e-4 relative); on hard steps the limiter keeps
//! the interpolant monotone and the crossing lands at the segment midpoint. The periodic
//! functions assume the profile spans exactly one period on a uniform grid
//! (they return `None` otherwise). [`mtf_from_image`] reports the image
//! modulation at whatever pitch the image already contains; it is not a
//! transfer function swept versus spatial frequency.
//!
//! Pitch dependence of the legacy functions (documented, not changed): they
//! measure whichever feature straddles the profile centre, so the *tone* they
//! report depends on where the pattern's phase puts x = 0 (a line for masks
//! with a line centred at the origin, a space otherwise). Before the
//! sub-pixel crossing finder, [`nils`] used a one-sided secant slope between
//! the two samples bracketing the edge, so its error depended on the edge's
//! sub-pixel position — which shifts with pitch on a fixed grid.

use ndarray::Array2;
use serde::{Deserialize, Serialize};

/// Which side of the threshold a measured feature lies on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureTone {
    /// The feature is where the intensity is **below** threshold — e.g. an
    /// opaque line on a bright-field mask (a resist line in positive resist).
    Dark,
    /// The feature is where the intensity is **at or above** threshold — e.g.
    /// a clear space or a contact hole on a dark-field mask.
    Bright,
}

impl FeatureTone {
    /// Tone of the mask's drawn features: clear features on a dark-field
    /// mask image bright ([`FeatureTone::Bright`]); opaque features on a
    /// bright-field mask image dark ([`FeatureTone::Dark`]).
    pub fn of_mask(mask: &crate::mask::Mask) -> Self {
        if mask.dark_field {
            FeatureTone::Bright
        } else {
            FeatureTone::Dark
        }
    }

    /// Whether a sample of the given intensity belongs to a feature of this
    /// tone. Samples exactly at threshold count as bright.
    fn contains(self, intensity: f64, threshold: f64) -> bool {
        match self {
            FeatureTone::Dark => intensity < threshold,
            FeatureTone::Bright => intensity >= threshold,
        }
    }
}

/// One feature of a periodic profile, bounded by two threshold crossings.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FeatureEdges {
    /// Left edge (nm), inside the field `[x_first − Δx/2, x_first − Δx/2 + period)`.
    pub left_nm: f64,
    /// Right edge (nm) = `left_nm + width_nm`; may exceed the field's right
    /// edge when the feature wraps around the periodic boundary.
    pub right_nm: f64,
    /// Feature width (nm) — the CD.
    pub width_nm: f64,
    /// Feature centre (nm), folded back into the field.
    pub centre_nm: f64,
    /// Intensity slope `dI/dx` (1/nm, intensity units) at the left edge.
    pub left_slope: f64,
    /// Intensity slope `dI/dx` (1/nm, intensity units) at the right edge.
    pub right_slope: f64,
}

/// A threshold crossing located on the monotone cubic interpolant.
#[derive(Debug, Clone, Copy)]
struct Crossing {
    /// Position (nm). For periodic profiles it may lie in the wrap segment
    /// `[x_last, x_first + period)`.
    x: f64,
    /// `dI/dx` of the interpolant at the crossing.
    slope: f64,
    /// Intensity rises through the threshold (below → at/above) with x.
    rising: bool,
}

/// Node slope `dI/dx` at sample `k`: the five-point central difference
/// `(−y₊₂ + 8y₊₁ − 8y₋₁ + y₋₂)/(12Δx)` (fourth order) where the four
/// neighbours exist on a locally uniform grid, else the three-point central
/// difference, else one-sided at the ends of open profiles. Periodic
/// profiles wrap their neighbours.
fn node_slope(profile: &[f64], x: &[f64], k: usize, period: Option<f64>) -> f64 {
    let n = profile.len();
    if n < 2 {
        return 0.0;
    }
    // Neighbour `k + off` as (x, y), wrapping for periodic profiles.
    let at = |off: isize| -> Option<(f64, f64)> {
        let idx = k as isize + off;
        match period {
            Some(p) => {
                let wraps = idx.div_euclid(n as isize);
                let i = idx.rem_euclid(n as isize) as usize;
                Some((x[i] + wraps as f64 * p, profile[i]))
            }
            None => {
                if idx < 0 || idx >= n as isize {
                    None
                } else {
                    let i = idx as usize;
                    Some((x[i], profile[i]))
                }
            }
        }
    };
    if n >= 5 {
        if let (Some(m2), Some(m1), Some(p1), Some(p2)) = (at(-2), at(-1), at(1), at(2)) {
            let (x0, _) = at(0).expect("centre sample exists");
            let h = p1.0 - x0;
            let uniform = [m1.0 - m2.0, x0 - m1.0, p2.0 - p1.0]
                .iter()
                .all(|&d| (d - h).abs() <= 1e-9 * h.abs());
            if h > 0.0 && uniform {
                return (-p2.1 + 8.0 * p1.1 - 8.0 * m1.1 + m2.1) / (12.0 * h);
            }
        }
    }
    match (at(-1), at(1)) {
        (Some(m1), Some(p1)) => (p1.1 - m1.1) / (p1.0 - m1.0),
        (None, Some(p1)) => {
            let (x0, y0) = at(0).expect("centre sample exists");
            (p1.1 - y0) / (p1.0 - x0)
        }
        (Some(m1), None) => {
            let (x0, y0) = at(0).expect("centre sample exists");
            (y0 - m1.1) / (x0 - m1.0)
        }
        (None, None) => 0.0,
    }
}

/// Locate the crossing of `threshold` inside one segment `[x0, x1]` with
/// node values `(y0, y1)` and node slopes `(m0, m1)`. The node slopes are
/// Fritsch–Carlson limited against the secant, so the cubic Hermite is
/// monotone on the segment and the root is unique. Returns
/// `(x_cross, dI/dx at x_cross)`.
fn segment_crossing(x0: f64, x1: f64, y0: f64, y1: f64, m0: f64, m1: f64, t: f64) -> (f64, f64) {
    let h = x1 - x0;
    let delta = (y1 - y0) / h;
    let (mut m0, mut m1) = (m0, m1);
    if delta == 0.0 || !m0.is_finite() || !m1.is_finite() {
        m0 = delta;
        m1 = delta;
    } else {
        let mut alpha = m0 / delta;
        let mut beta = m1 / delta;
        if alpha < 0.0 {
            alpha = 0.0;
        }
        if beta < 0.0 {
            beta = 0.0;
        }
        let r2 = alpha * alpha + beta * beta;
        if r2 > 9.0 {
            let tau = 3.0 / r2.sqrt();
            alpha *= tau;
            beta *= tau;
        }
        m0 = alpha * delta;
        m1 = beta * delta;
    }
    let hermite = |s: f64| -> f64 {
        let s2 = s * s;
        let s3 = s2 * s;
        (2.0 * s3 - 3.0 * s2 + 1.0) * y0
            + (s3 - 2.0 * s2 + s) * h * m0
            + (-2.0 * s3 + 3.0 * s2) * y1
            + (s3 - s2) * h * m1
    };
    let hermite_slope = |s: f64| -> f64 {
        let s2 = s * s;
        ((6.0 * s2 - 6.0 * s) * y0
            + (3.0 * s2 - 4.0 * s + 1.0) * h * m0
            + (-6.0 * s2 + 6.0 * s) * y1
            + (3.0 * s2 - 2.0 * s) * h * m1)
            / h
    };
    // Bisection on the monotone cubic: f(lo) and f(hi) bracket the root.
    let rising = y1 > y0;
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        let above = hermite(mid) >= t;
        if above == rising {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let s = 0.5 * (lo + hi);
    (x0 + s * h, hermite_slope(s))
}

/// All threshold crossings of a profile, in increasing x. With
/// `period = Some(p)` the segment from the last sample back to the first
/// (at `x_first + p`) is included.
fn find_crossings(
    profile: &[f64],
    x: &[f64],
    threshold: f64,
    period: Option<f64>,
) -> Vec<Crossing> {
    let n = profile.len();
    let mut out = Vec::new();
    if n < 2 || x.len() != n {
        return out;
    }
    let n_seg = if period.is_some() { n } else { n - 1 };
    for k in 0..n_seg {
        let k1 = (k + 1) % n;
        let (y0, y1) = (profile[k], profile[k1]);
        if !y0.is_finite() || !y1.is_finite() {
            continue;
        }
        let above0 = y0 >= threshold;
        let above1 = y1 >= threshold;
        if above0 == above1 {
            continue;
        }
        let x0 = x[k];
        let x1 = if k1 == 0 {
            x[0] + period.unwrap_or(0.0)
        } else {
            x[k1]
        };
        if x1.is_nan() || x1 <= x0 {
            continue;
        }
        let m0 = node_slope(profile, x, k, period);
        let m1 = node_slope(profile, x, k1, period);
        let (xc, slope) = segment_crossing(x0, x1, y0, y1, m0, m1, threshold);
        out.push(Crossing {
            x: xc,
            slope,
            rising: above1,
        });
    }
    out
}

/// Uniform sample spacing of `x`, or `None` if the grid is not uniform
/// (relative tolerance 1e-6) or has fewer than 3 points.
fn uniform_spacing(x: &[f64]) -> Option<f64> {
    if x.len() < 3 {
        return None;
    }
    let dx = x[1] - x[0];
    if !dx.is_finite() || dx <= 0.0 {
        return None;
    }
    let tol = 1e-6 * dx;
    if x.windows(2).all(|w| ((w[1] - w[0]) - dx).abs() <= tol) {
        Some(dx)
    } else {
        None
    }
}

/// Field geometry of a periodic profile: `(x_lo, period)` where the field is
/// `[x_lo, x_lo + period)` and `x_lo = x_first − Δx/2`.
fn periodic_field(profile: &[f64], x: &[f64]) -> Option<(f64, f64)> {
    if profile.len() != x.len() {
        return None;
    }
    let dx = uniform_spacing(x)?;
    let period = dx * x.len() as f64;
    Some((x[0] - 0.5 * dx, period))
}

/// Features of the given tone from the crossings of one periodic profile.
fn features_from_crossings(
    crossings: &[Crossing],
    tone: FeatureTone,
    x_lo: f64,
    period: f64,
) -> Vec<FeatureEdges> {
    let m = crossings.len();
    let mut out = Vec::new();
    if m < 2 {
        return out;
    }
    // A feature of tone Dark starts where the intensity falls below
    // threshold and ends at the next rising crossing (Bright: the reverse).
    let starts_feature = |c: &Crossing| match tone {
        FeatureTone::Dark => !c.rising,
        FeatureTone::Bright => c.rising,
    };
    for i in 0..m {
        let start = crossings[i];
        if !starts_feature(&start) {
            continue;
        }
        let end = crossings[(i + 1) % m];
        if starts_feature(&end) {
            // Not alternating (cannot happen for a clean periodic profile).
            continue;
        }
        let mut width = end.x - start.x;
        if width <= 0.0 {
            width += period;
        }
        let mut left = start.x;
        if left >= x_lo + period {
            left -= period;
        }
        let mut centre = left + 0.5 * width;
        while centre >= x_lo + period {
            centre -= period;
        }
        out.push(FeatureEdges {
            left_nm: left,
            right_nm: left + width,
            width_nm: width,
            centre_nm: centre,
            left_slope: start.slope,
            right_slope: end.slope,
        });
    }
    out
}

/// Wrap-aware distance between two positions on a ring of circumference
/// `period`.
fn ring_distance(a: f64, b: f64, period: f64) -> f64 {
    let d = (a - b).rem_euclid(period);
    d.min(period - d)
}

/// All features of the given tone in one period of a periodic profile.
///
/// `intensity_profile` must span exactly one period of a periodic image on a
/// uniform grid (`x_nm` = pixel centres, as produced by the imaging engine);
/// crossings wrap around the field edge. Returns an empty list if the grid is
/// not uniform, the lengths differ, or nothing crosses the threshold.
pub fn periodic_features(
    intensity_profile: &[f64],
    x_nm: &[f64],
    threshold: f64,
    tone: FeatureTone,
) -> Vec<FeatureEdges> {
    let Some((x_lo, period)) = periodic_field(intensity_profile, x_nm) else {
        return Vec::new();
    };
    let crossings = find_crossings(intensity_profile, x_nm, threshold, Some(period));
    features_from_crossings(&crossings, tone, x_lo, period)
}

/// The feature of `features` whose centre is nearest the field centre
/// (wrap-aware).
fn nearest_centre_feature(
    features: &[FeatureEdges],
    x_lo: f64,
    period: f64,
) -> Option<FeatureEdges> {
    let centre = x_lo + 0.5 * period;
    features.iter().copied().min_by(|a, b| {
        ring_distance(a.centre_nm, centre, period).total_cmp(&ring_distance(
            b.centre_nm,
            centre,
            period,
        ))
    })
}

/// Tone-aware CD of a periodic profile: the width of the `tone` feature whose
/// centre is nearest the field centre (x = 0 on the engine's centred grid).
///
/// Crossings are wrap-aware, so a feature straddling the field edge is
/// measured whole. See [`periodic_features`] for the input contract.
pub fn measure_cd_periodic(
    intensity_profile: &[f64],
    x_nm: &[f64],
    threshold: f64,
    tone: FeatureTone,
) -> Option<f64> {
    let (x_lo, period) = periodic_field(intensity_profile, x_nm)?;
    let features = periodic_features(intensity_profile, x_nm, threshold, tone);
    nearest_centre_feature(&features, x_lo, period).map(|f| f.width_nm)
}

/// Image log-slope `|d ln I/dx| = |dI/dx| / I_threshold` (1/nm) at the edges
/// of the `tone` feature nearest the field centre (mean of its two edges).
pub fn image_log_slope(
    intensity_profile: &[f64],
    x_nm: &[f64],
    threshold: f64,
    tone: FeatureTone,
) -> Option<f64> {
    if threshold.is_nan() || threshold <= 0.0 {
        return None;
    }
    let (x_lo, period) = periodic_field(intensity_profile, x_nm)?;
    let features = periodic_features(intensity_profile, x_nm, threshold, tone);
    let f = nearest_centre_feature(&features, x_lo, period)?;
    Some(0.5 * (f.left_slope.abs() + f.right_slope.abs()) / threshold)
}

/// Normalized image log-slope of a periodic profile:
/// `NILS = w · |dI/dx| / I_threshold`, with the slope taken exactly at the two
/// threshold crossings of the `tone` feature nearest the field centre
/// (averaged) and `w` the measured CD of that feature — or `width_nm` when
/// given (e.g. the nominal/target width, the other common convention).
pub fn nils_periodic(
    intensity_profile: &[f64],
    x_nm: &[f64],
    threshold: f64,
    tone: FeatureTone,
    width_nm: Option<f64>,
) -> Option<f64> {
    if threshold.is_nan() || threshold <= 0.0 {
        return None;
    }
    let (x_lo, period) = periodic_field(intensity_profile, x_nm)?;
    let features = periodic_features(intensity_profile, x_nm, threshold, tone);
    let f = nearest_centre_feature(&features, x_lo, period)?;
    let ils = 0.5 * (f.left_slope.abs() + f.right_slope.abs()) / threshold;
    Some(width_nm.unwrap_or(f.width_nm) * ils)
}

/// Mask error enhancement factor from two (mask CD, wafer CD) pairs:
/// `MEEF = (CD_wafer,hi − CD_wafer,lo) / (CD_mask,hi − CD_mask,lo)`.
///
/// Both CDs must be at **wafer scale** (this crate's [`crate::mask::Mask`]
/// coordinates are post-reduction), so a perfectly linear imaging process
/// has MEEF = 1. Returns `None` for equal mask CDs or non-finite inputs.
pub fn meef(
    mask_cd_lo_nm: f64,
    wafer_cd_lo_nm: f64,
    mask_cd_hi_nm: f64,
    wafer_cd_hi_nm: f64,
) -> Option<f64> {
    let dm = mask_cd_hi_nm - mask_cd_lo_nm;
    let dw = wafer_cd_hi_nm - wafer_cd_lo_nm;
    if !dm.is_finite() || !dw.is_finite() || dm == 0.0 {
        return None;
    }
    Some(dw / dm)
}

/// Central-difference MEEF around `mask_cd_nm`: calls `wafer_cd` at
/// `mask_cd_nm ± delta_nm` (wafer-scale mask CDs) and applies [`meef`].
/// `wafer_cd` typically builds the mask, images it and measures the printed
/// CD; returning `None` aborts.
pub fn meef_central<F: FnMut(f64) -> Option<f64>>(
    mask_cd_nm: f64,
    delta_nm: f64,
    mut wafer_cd: F,
) -> Option<f64> {
    if delta_nm.is_nan() || delta_nm <= 0.0 || !mask_cd_nm.is_finite() {
        return None;
    }
    let lo = wafer_cd(mask_cd_nm - delta_nm)?;
    let hi = wafer_cd(mask_cd_nm + delta_nm)?;
    meef(mask_cd_nm - delta_nm, lo, mask_cd_nm + delta_nm, hi)
}

/// The y = 0 cross-section of an image on the engine's centred grid.
///
/// For an even number of rows the pixel centres sit at `y = ±Δy/2, ±3Δy/2,
/// …` around the origin, so no row lies on y = 0: the profile is
/// interpolated at the midpoint with the four-point cubic (Lagrange) rule
/// `(−r₋₂ + 9·r₋₁ + 9·r₊₁ − r₊₂)/16`, exact for cubics in y (the plain mean of
/// the two central rows when only two rows exist). For an odd row count the
/// central row is used. Images that are uniform in y (line/space) are
/// unaffected; for 2D features (contacts) this removes the half-pixel
/// offset of the old single-row cut.
pub fn centre_profile(image: &Array2<f64>) -> Vec<f64> {
    let (ny, nx) = image.dim();
    if ny == 0 {
        return Vec::new();
    }
    if ny % 2 == 1 {
        let r = ny / 2;
        return (0..nx).map(|j| image[[r, j]]).collect();
    }
    let (r0, r1) = (ny / 2 - 1, ny / 2);
    if ny < 4 {
        return (0..nx)
            .map(|j| 0.5 * (image[[r0, j]] + image[[r1, j]]))
            .collect();
    }
    let (rm, rp) = (r0 - 1, r1 + 1);
    (0..nx)
        .map(|j| {
            (-image[[rm, j]] + 9.0 * image[[r0, j]] + 9.0 * image[[r1, j]] - image[[rp, j]]) / 16.0
        })
        .collect()
}

/// Pixel-centre x coordinates `x_j = x_min + (j + ½)·Δx` of an `nx`-column
/// image spanning `[x_min_nm, x_max_nm]`.
pub fn pixel_centres(nx: usize, x_min_nm: f64, x_max_nm: f64) -> Vec<f64> {
    let pixel = (x_max_nm - x_min_nm) / nx as f64;
    (0..nx)
        .map(|j| x_min_nm + (j as f64 + 0.5) * pixel)
        .collect()
}

/// Measure critical dimension (CD) from an intensity cross-section
/// (legacy, open-profile variant).
///
/// Returns the width (nm) between the two consecutive threshold crossings
/// whose midpoint is nearest the profile centre — whichever tone that
/// feature has (a line or a space). Crossings are located with the shared
/// sub-pixel monotone-cubic finder. For periodic engine images prefer
/// [`measure_cd_periodic`], which is wrap-aware and tone-explicit.
pub fn measure_cd(intensity_profile: &[f64], x_nm: &[f64], threshold: f64) -> Option<f64> {
    if intensity_profile.len() != x_nm.len() || intensity_profile.len() < 2 {
        return None;
    }
    let crossings = find_crossings(intensity_profile, x_nm, threshold, None);
    if crossings.len() < 2 {
        return None;
    }
    let center = (x_nm[0] + x_nm[x_nm.len() - 1]) / 2.0;
    let mut best_pair = (0, 1);
    let mut best_dist = f64::INFINITY;
    for i in 0..crossings.len() - 1 {
        let mid = (crossings[i].x + crossings[i + 1].x) / 2.0;
        let dist = (mid - center).abs();
        if dist < best_dist {
            best_dist = dist;
            best_pair = (i, i + 1);
        }
    }
    Some((crossings[best_pair.1].x - crossings[best_pair.0].x).abs())
}

/// Measure CD from a 2D aerial image on its y = 0 cross-section
/// ([`centre_profile`]: cubic midpoint interpolation of the four central
/// rows for an even row count).
pub fn measure_cd_2d(
    image: &Array2<f64>,
    x_min_nm: f64,
    x_max_nm: f64,
    threshold: f64,
) -> Option<f64> {
    let x_nm = pixel_centres(image.ncols(), x_min_nm, x_max_nm);
    let profile = centre_profile(image);
    measure_cd(&profile, &x_nm, threshold)
}

/// Normalized Image Log-Slope (NILS) at a feature edge (legacy, open-profile
/// variant).
///
/// NILS = w · |dI/dx| / I_threshold, with the slope evaluated exactly at the
/// threshold crossing nearest the profile centre (derivative of the monotone
/// cubic interpolant, not a one-sided difference) and `w` the width returned
/// by [`measure_cd`]. For periodic engine images prefer [`nils_periodic`].
pub fn nils(intensity_profile: &[f64], x_nm: &[f64], threshold: f64) -> Option<f64> {
    if intensity_profile.len() != x_nm.len() || intensity_profile.len() < 3 {
        return None;
    }
    if threshold.abs() < 1e-15 {
        return None;
    }
    let crossings = find_crossings(intensity_profile, x_nm, threshold, None);
    let center = (x_nm[0] + x_nm[x_nm.len() - 1]) / 2.0;
    let edge = crossings
        .iter()
        .min_by(|a, b| (a.x - center).abs().total_cmp(&(b.x - center).abs()))?;
    let cd = measure_cd(intensity_profile, x_nm, threshold)?;
    Some(cd * edge.slope.abs() / threshold.abs())
}

/// Image contrast (modulation): (Imax - Imin) / (Imax + Imin).
pub fn image_contrast(image: &Array2<f64>) -> f64 {
    let max = image.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min = image.iter().cloned().fold(f64::INFINITY, f64::min);

    if max + min > 0.0 {
        (max - min) / (max + min)
    } else {
        0.0
    }
}

/// Depth-resolved CD: measure the center-row CD in every z-slice of a
/// volumetric latent image (threshold on the PAC value: a point is
/// "inside the feature" where the profile crosses `threshold`).
/// Returns one `Option<f64>` per slice, top to bottom.
pub fn cd_at_z(volume: &crate::types::Grid3D<f64>, threshold: f64) -> Vec<Option<f64>> {
    let (nz, _ny, _nx) = volume.data.dim();
    (0..nz)
        .map(|k| {
            let slice = volume.data.index_axis(ndarray::Axis(0), k);
            let owned = slice.to_owned();
            measure_cd_2d(&owned, volume.x_min_nm, volume.x_max_nm, threshold)
        })
        .collect()
}

/// Sidewall angle in degrees from top and bottom CDs of a developed
/// feature (trench convention: 90° = perfectly vertical walls; < 90°
/// means the trench narrows toward the bottom).
pub fn sidewall_angle_deg(cd_top_nm: f64, cd_bottom_nm: f64, thickness_nm: f64) -> f64 {
    (2.0 * thickness_nm)
        .atan2(cd_top_nm - cd_bottom_nm)
        .to_degrees()
}

/// Aspect ratio of a developed feature: depth / lateral width.
pub fn aspect_ratio(depth_nm: f64, width_nm: f64) -> f64 {
    if width_nm <= 0.0 {
        return f64::INFINITY;
    }
    depth_nm / width_nm
}

/// Modulation Transfer Function (MTF) at a given spatial frequency.
/// Computed from the aerial image of a line/space pattern at that pitch.
pub fn mtf_from_image(image: &Array2<f64>) -> f64 {
    // Use the center row
    let ny = image.nrows();
    let nx = image.ncols();
    let center = ny / 2;

    let row: Vec<f64> = (0..nx).map(|j| image[[center, j]]).collect();
    let max = row.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min = row.iter().cloned().fold(f64::INFINITY, f64::min);

    if max + min > 0.0 {
        (max - min) / (max + min)
    } else {
        0.0
    }
}

/// CD of the `tone` feature that contains `x_ref` on a periodic profile at
/// intensity threshold `threshold` — the building block of dose-dependent
/// process-window analysis ([`crate::process`]).
///
/// Returns `0.0` when `x_ref` lies outside every feature of that tone (the
/// feature does not print), `f64::INFINITY` when the whole period has that
/// tone (neighbouring features merged), otherwise the feature width. `None`
/// if the grid is not uniform.
pub(crate) fn cd_containing(
    intensity_profile: &[f64],
    x_nm: &[f64],
    threshold: f64,
    tone: FeatureTone,
    x_ref: f64,
) -> Option<f64> {
    let (x_lo, period) = periodic_field(intensity_profile, x_nm)?;
    let crossings = find_crossings(intensity_profile, x_nm, threshold, Some(period));
    if crossings.is_empty() {
        let all_tone = intensity_profile
            .iter()
            .filter(|v| v.is_finite())
            .all(|&v| tone.contains(v, threshold));
        return Some(if all_tone { f64::INFINITY } else { 0.0 });
    }
    let features = features_from_crossings(&crossings, tone, x_lo, period);
    for f in &features {
        let offset = (x_ref - f.left_nm).rem_euclid(period);
        if offset <= f.width_nm {
            return Some(f.width_nm);
        }
    }
    Some(0.0)
}

/// Reference point of the `tone` feature nearest the field centre at
/// `threshold`: the sample of extreme intensity inside it (minimum for dark,
/// maximum for bright). This point stays inside the feature as the threshold
/// moves, which makes [`cd_containing`] monotone in the threshold. Falls back
/// to the extreme sample nearest the field centre when nothing prints.
pub(crate) fn feature_reference_point(
    intensity_profile: &[f64],
    x_nm: &[f64],
    threshold: f64,
    tone: FeatureTone,
) -> Option<f64> {
    let (x_lo, period) = periodic_field(intensity_profile, x_nm)?;
    let n = intensity_profile.len();
    let better = |a: f64, b: f64| match tone {
        FeatureTone::Dark => a < b,
        FeatureTone::Bright => a > b,
    };
    let centre = x_lo + 0.5 * period;
    let features = periodic_features(intensity_profile, x_nm, threshold, tone);
    if let Some(f) = nearest_centre_feature(&features, x_lo, period) {
        let mut best: Option<(f64, f64)> = None;
        for j in 0..n {
            let offset = (x_nm[j] - f.left_nm).rem_euclid(period);
            if offset > f.width_nm {
                continue;
            }
            let v = intensity_profile[j];
            if !v.is_finite() {
                continue;
            }
            match best {
                None => best = Some((x_nm[j], v)),
                Some((_, bv)) if better(v, bv) => best = Some((x_nm[j], v)),
                _ => {}
            }
        }
        if let Some((x, _)) = best {
            return Some(x);
        }
        return Some(f.centre_nm);
    }
    // Nothing prints at this threshold: extreme sample nearest the centre.
    let mut best: Option<(usize, f64)> = None;
    for (j, &v) in intensity_profile.iter().enumerate() {
        if !v.is_finite() {
            continue;
        }
        best = match best {
            None => Some((j, v)),
            Some((bj, bv)) => {
                if better(v, bv)
                    || (v == bv
                        && ring_distance(x_nm[j], centre, period)
                            < ring_distance(x_nm[bj], centre, period))
                {
                    Some((j, v))
                } else {
                    Some((bj, bv))
                }
            }
        };
    }
    best.map(|(j, _)| x_nm[j])
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use std::f64::consts::PI;

    /// Engine-style pixel centres for a field of `n` pixels of size `h`.
    fn engine_x(n: usize, h: f64) -> Vec<f64> {
        let half = n as f64 * h / 2.0;
        (0..n).map(|j| -half + (j as f64 + 0.5) * h).collect()
    }

    /// I(x) = a − b cos(2πx/p): one dark feature per period, centred at 0.
    fn cos_profile(x: &[f64], a: f64, b: f64, p: f64) -> Vec<f64> {
        x.iter()
            .map(|&x| a - b * (2.0 * PI * x / p).cos())
            .collect()
    }

    fn cd_dark_exact(a: f64, b: f64, p: f64, t: f64) -> f64 {
        (p / PI) * ((a - t) / b).acos()
    }

    #[test]
    fn test_cd_measurement_simple() {
        // Simple step function: 0 from -100 to -25, 1 from -25 to 25, 0 from 25 to 100
        let n = 200;
        let x_nm: Vec<f64> = (0..n).map(|i| -100.0 + i as f64).collect();
        let intensity: Vec<f64> = x_nm
            .iter()
            .map(|&x| if x.abs() < 25.0 { 1.0 } else { 0.0 })
            .collect();

        let cd = measure_cd(&intensity, &x_nm, 0.5).unwrap();
        assert_relative_eq!(cd, 50.0, epsilon = 2.0);
    }

    #[test]
    fn test_cos_fixture_cd_nils_closed_form() {
        // Closed forms (numpy check): a=0.5, b=0.4, p=180, t=0.3 →
        // CD_dark = 60 nm, CD_bright = 120 nm, slope = 1.209199576156e-2 /nm,
        // NILS_dark = 2.418399152312, NILS_bright = 4.836798304625.
        let (a, b, p, t) = (0.5, 0.4, 180.0, 0.3);
        let x = engine_x(90, 2.0);
        let prof = cos_profile(&x, a, b, p);
        let cd_d = measure_cd_periodic(&prof, &x, t, FeatureTone::Dark).unwrap();
        let cd_b = measure_cd_periodic(&prof, &x, t, FeatureTone::Bright).unwrap();
        assert_relative_eq!(cd_d, 60.0, epsilon = 2e-3);
        assert_relative_eq!(cd_b, 120.0, epsilon = 2e-3);
        let nils_d = nils_periodic(&prof, &x, t, FeatureTone::Dark, None).unwrap();
        let nils_b = nils_periodic(&prof, &x, t, FeatureTone::Bright, None).unwrap();
        assert_relative_eq!(nils_d, 2.418399152312, max_relative = 1e-3);
        assert_relative_eq!(nils_b, 4.836798304625, max_relative = 1e-3);
        let ils = image_log_slope(&prof, &x, t, FeatureTone::Dark).unwrap();
        assert_relative_eq!(ils, 4.030665253854e-2, max_relative = 1e-3);
        // Supplying a nominal width scales NILS linearly.
        let nils_w = nils_periodic(&prof, &x, t, FeatureTone::Dark, Some(65.0)).unwrap();
        assert_relative_eq!(nils_w, 65.0 * ils, max_relative = 1e-12);
    }

    #[test]
    fn test_cos_fixture_other_thresholds() {
        // numpy: (a,b,t)=(0.5,0.4,0.27) → CD_dark = 54.900367804606,
        // NILS_dark = 2.322809472986; (0.45,0.35,0.62) → CD_bright =
        // 60.940718931959, NILS_bright = 1.049690269867.
        let x = engine_x(128, 180.0 / 128.0);
        let p1 = cos_profile(&x, 0.5, 0.4, 180.0);
        let cd = measure_cd_periodic(&p1, &x, 0.27, FeatureTone::Dark).unwrap();
        assert_relative_eq!(cd, 54.900367804606, epsilon = 2e-3);
        let n = nils_periodic(&p1, &x, 0.27, FeatureTone::Dark, None).unwrap();
        assert_relative_eq!(n, 2.322809472986, max_relative = 5e-4);
        let p2 = cos_profile(&x, 0.45, 0.35, 180.0);
        let cd2 = measure_cd_periodic(&p2, &x, 0.62, FeatureTone::Bright).unwrap();
        assert_relative_eq!(cd2, 60.940718931959, epsilon = 2e-3);
        let n2 = nils_periodic(&p2, &x, 0.62, FeatureTone::Bright, None).unwrap();
        assert_relative_eq!(n2, 1.049690269867, max_relative = 5e-4);
        // Complementary tones tile the period.
        let cd_d = measure_cd_periodic(&p2, &x, 0.62, FeatureTone::Dark).unwrap();
        assert_relative_eq!(cd2 + cd_d, 180.0, epsilon = 1e-9);
    }

    #[test]
    fn test_crossing_accuracy_converges_with_pixel() {
        // Worst case over 16 sub-pixel phases of the pattern. Measured
        // envelopes (fourth-order CD, third-order slope) sit inside these
        // bounds: CD 1.32e-3 / 7.1e-5 / 3.4e-6 / 2.2e-7 nm and NILS
        // 3.65e-4 / 1.5e-5 / 6.0e-6 / 6.9e-7 for 20/40/80/160 px per period.
        let (a, b, p, t) = (0.5, 0.4, 180.0, 0.27);
        let exact_cd = cd_dark_exact(a, b, p, t);
        let exact_nils = 2.322809472986;
        let bounds = [
            (20usize, 3e-3, 5e-4),
            (40, 2e-4, 5e-5),
            (80, 1e-5, 1.5e-5),
            (160, 1e-6, 2e-6),
        ];
        for &(n, cd_bound, nils_bound) in &bounds {
            let h = p / n as f64;
            let x = engine_x(n, h);
            let mut worst = (0.0_f64, 0.0_f64);
            for q in 0..16 {
                // Shifting the pattern moves the edges relative to the
                // pixels but keeps the line nearest the field centre.
                let shift = h * q as f64 / 16.0;
                let prof: Vec<f64> = x
                    .iter()
                    .map(|&x| a - b * (2.0 * PI * (x - shift) / p).cos())
                    .collect();
                let cd = measure_cd_periodic(&prof, &x, t, FeatureTone::Dark).unwrap();
                let nl = nils_periodic(&prof, &x, t, FeatureTone::Dark, None).unwrap();
                worst.0 = worst.0.max((cd - exact_cd).abs());
                worst.1 = worst.1.max((nl - exact_nils).abs());
            }
            assert!(worst.0 < cd_bound, "n={n}: CD error {}", worst.0);
            assert!(worst.1 < nils_bound, "n={n}: NILS error {}", worst.1);
        }
    }

    #[test]
    fn test_periodic_feature_wrapping_boundary() {
        // Shift the pattern so a dark line straddles the field edge: the
        // wrap-aware measurement still reports the full width, and the
        // number of features equals the number of periods in the field.
        let (a, b, p, t) = (0.5, 0.4, 90.0, 0.3);
        let x = engine_x(128, 180.0 / 128.0); // field 180 nm = 2 periods
                                              // Line centres at 85 (spans 70…100: straddles the +90 field edge)
                                              // and −5.
        let shift = 85.0;
        let prof: Vec<f64> = x
            .iter()
            .map(|&x| a - b * (2.0 * PI * (x - shift) / p).cos())
            .collect();
        let feats = periodic_features(&prof, &x, t, FeatureTone::Dark);
        assert_eq!(feats.len(), 2);
        let exact = cd_dark_exact(a, b, p, t); // 30 nm
        for f in &feats {
            assert_relative_eq!(f.width_nm, exact, epsilon = 2e-3);
            assert_relative_eq!(f.right_nm - f.left_nm, f.width_nm, epsilon = 1e-12);
            assert!(f.left_slope < 0.0 && f.right_slope > 0.0);
            assert!(f.centre_nm >= -90.0 && f.centre_nm < 90.0);
        }
        // The straddling line is reported whole, folded into the field.
        let wrapped = feats.iter().find(|f| f.right_nm > 90.0).unwrap();
        assert_relative_eq!(wrapped.centre_nm, 85.0, epsilon = 1e-3);
        // The line at −5 is nearest the field centre.
        let cd = measure_cd_periodic(&prof, &x, t, FeatureTone::Dark).unwrap();
        assert_relative_eq!(cd, exact, epsilon = 2e-3);
        // Bright spaces between them also measure whole: p − CD.
        let cd_b = measure_cd_periodic(&prof, &x, t, FeatureTone::Bright).unwrap();
        assert_relative_eq!(cd_b, p - exact, epsilon = 2e-3);
    }

    #[test]
    fn test_legacy_nils_uses_slope_at_crossing() {
        // numpy: the old one-sided secant gave 2.41319 (4 nm) / 2.41710
        // (2 nm); exact 2.418399152312. The interpolated slope is closer.
        let (a, b, p, t) = (0.5, 0.4, 180.0, 0.3);
        for &(h, old) in &[(4.0, 2.4131872006464867), (2.0, 2.417095827251354)] {
            let n = (p / h) as usize;
            let x = engine_x(n, h);
            let prof = cos_profile(&x, a, b, p);
            let new = nils(&prof, &x, t).unwrap();
            let exact = 2.418399152312;
            assert!(
                (new - exact).abs() < (old - exact).abs(),
                "h={h}: new {new} not better than old {old}"
            );
            assert_relative_eq!(new, exact, max_relative = 5e-4);
        }
    }

    #[test]
    fn test_tone_selection_and_threshold_out_of_range() {
        let x = engine_x(64, 180.0 / 64.0);
        let prof = cos_profile(&x, 0.5, 0.4, 180.0);
        // Threshold outside (I_min, I_max): no crossings.
        assert!(measure_cd_periodic(&prof, &x, 0.05, FeatureTone::Dark).is_none());
        assert!(measure_cd_periodic(&prof, &x, 0.95, FeatureTone::Bright).is_none());
        assert!(periodic_features(&prof, &x, 0.95, FeatureTone::Dark).is_empty());
        // Non-uniform grid → None.
        let mut xb = x.clone();
        xb[10] += 0.3;
        assert!(measure_cd_periodic(&prof, &xb, 0.3, FeatureTone::Dark).is_none());
        // Length mismatch → None.
        assert!(measure_cd_periodic(&prof[..63], &x, 0.3, FeatureTone::Dark).is_none());
    }

    #[test]
    fn test_feature_tone_of_mask() {
        use crate::mask::{Mask, MaskType};
        let bright_field = Mask {
            mask_type: MaskType::Binary,
            features: Vec::new(),
            dark_field: false,
        };
        let dark_field = Mask {
            dark_field: true,
            ..bright_field.clone()
        };
        assert_eq!(FeatureTone::of_mask(&bright_field), FeatureTone::Dark);
        assert_eq!(FeatureTone::of_mask(&dark_field), FeatureTone::Bright);
    }

    #[test]
    fn test_meef_helpers() {
        assert_relative_eq!(meef(60.0, 50.0, 70.0, 72.0).unwrap(), 2.2, epsilon = 1e-12);
        assert!(meef(60.0, 50.0, 60.0, 72.0).is_none());
        assert!(meef(60.0, f64::NAN, 70.0, 72.0).is_none());
        let m = meef_central(65.0, 1.0, |m| Some(2.0 * m - 10.0)).unwrap();
        assert_relative_eq!(m, 2.0, epsilon = 1e-12);
        assert!(meef_central(65.0, 0.0, |m| Some(m)).is_none());
        assert!(meef_central(65.0, 1.0, |_| None).is_none());
    }

    #[test]
    fn test_meef_two_beam_fixture() {
        // Two-beam image of an opaque line of width w on pitch p with
        // modulation transfer M: I = (1 − w/p) − 2M·sin(πw/p)/π·cos(2πx/p).
        // numpy (p=180, M=0.8, t=0.3): CD(58)=28.9936788593,
        // CD(62)=37.7506099965 → central-difference MEEF 2.1892327843
        // (analytic derivative at w=60: 2.1620752268).
        let (p, m_t, t) = (180.0, 0.8, 0.3);
        let x = engine_x(256, p / 256.0);
        let wafer_cd = |w: f64| {
            let a = 1.0 - w / p;
            let b = 2.0 * m_t * (PI * w / p).sin() / PI;
            let prof = cos_profile(&x, a, b, p);
            measure_cd_periodic(&prof, &x, t, FeatureTone::Dark)
        };
        assert_relative_eq!(wafer_cd(58.0).unwrap(), 28.9936788593, epsilon = 1e-3);
        assert_relative_eq!(wafer_cd(62.0).unwrap(), 37.7506099965, epsilon = 1e-3);
        let m = meef_central(60.0, 2.0, wafer_cd).unwrap();
        assert_relative_eq!(m, 2.1892327843, max_relative = 1e-3);
    }

    #[test]
    fn test_centre_profile_even_and_odd() {
        let img = Array2::from_shape_fn((4, 3), |(i, j)| (10 * i + j) as f64);
        // Rows 1 and 2 averaged.
        assert_eq!(centre_profile(&img), vec![15.0, 16.0, 17.0]);
        let img3 = Array2::from_shape_fn((3, 2), |(i, j)| (10 * i + j) as f64);
        assert_eq!(centre_profile(&img3), vec![10.0, 11.0]);
    }

    #[test]
    fn test_measure_cd_2d_uses_y0_profile() {
        // Gaussian bright spot centred at (0,0): the y=0 profile (mean of the
        // rows at ±Δy/2) gives a CD closer to the exact on-axis width than a
        // single off-axis row does.
        let n = 64;
        let h = 4.0;
        let x = engine_x(n, h);
        let s: f64 = 20.0;
        let img = Array2::from_shape_fn((n, n), |(i, j)| {
            (-(x[j] * x[j] + x[i] * x[i]) / (2.0 * s * s)).exp()
        });
        let t: f64 = 0.5;
        let exact = 2.0 * s * (2.0 * (1.0 / t).ln()).sqrt();
        let cd = measure_cd_2d(&img, -128.0, 128.0, t).unwrap();
        let row: Vec<f64> = (0..n).map(|j| img[[n / 2, j]]).collect();
        let cd_row = measure_cd(&row, &x, t).unwrap();
        assert!(
            (cd - exact).abs() < (cd_row - exact).abs(),
            "y0 {cd} vs row {cd_row} vs exact {exact}"
        );
    }

    #[test]
    fn test_cd_containing_monotone_and_limits() {
        let (a, b, p) = (0.5, 0.4, 180.0);
        let x = engine_x(128, p / 128.0);
        let prof = cos_profile(&x, a, b, p);
        let x_ref = feature_reference_point(&prof, &x, 0.3, FeatureTone::Dark).unwrap();
        assert!(x_ref.abs() < p / 128.0);
        let mut last = 0.0;
        for k in 0..40 {
            let t = 0.12 + k as f64 * 0.02;
            let cd = cd_containing(&prof, &x, t, FeatureTone::Dark, x_ref).unwrap();
            assert!(cd >= last, "CD must grow with threshold");
            last = cd;
        }
        // Below I_min: the line does not print; above I_max: merged.
        assert_eq!(
            cd_containing(&prof, &x, 0.05, FeatureTone::Dark, x_ref),
            Some(0.0)
        );
        assert_eq!(
            cd_containing(&prof, &x, 0.95, FeatureTone::Dark, x_ref),
            Some(f64::INFINITY)
        );
        let cd = cd_containing(&prof, &x, 0.3, FeatureTone::Dark, x_ref).unwrap();
        assert_relative_eq!(cd, 60.0, epsilon = 2e-3);
    }

    #[test]
    fn test_sidewall_angle_conventions() {
        // Vertical walls: top CD == bottom CD -> 90 degrees.
        assert_relative_eq!(
            sidewall_angle_deg(100.0, 100.0, 500.0),
            90.0,
            epsilon = 1e-9
        );
        // Narrowing trench (bottom smaller): angle < 90.
        assert!(sidewall_angle_deg(120.0, 80.0, 500.0) < 90.0);
        // Re-entrant (bottom wider): angle > 90.
        assert!(sidewall_angle_deg(80.0, 120.0, 500.0) > 90.0);
        // 45-degree taper: width difference of 2*thickness.
        assert_relative_eq!(
            sidewall_angle_deg(1100.0, 100.0, 500.0),
            45.0,
            epsilon = 1e-9
        );
    }

    #[test]
    fn test_aspect_ratio() {
        assert_relative_eq!(aspect_ratio(500_000.0, 5_000.0), 100.0, epsilon = 1e-12);
        assert!(aspect_ratio(1.0, 0.0).is_infinite());
    }

    #[test]
    fn test_cd_at_z_synthetic() {
        // Volume with a 40 nm-wide exposed stripe in the top half only.
        let mut vol =
            crate::types::Grid3D::<f64>::new(20, 20, 4, (-40.0, 40.0), (-40.0, 40.0), (0.0, 40.0))
                .unwrap();
        vol.data.fill(1.0);
        for k in 0..2 {
            for i in 0..20 {
                for j in 5..15 {
                    vol.data[[k, i, j]] = 0.0;
                }
            }
        }
        let cds = cd_at_z(&vol, 0.5);
        assert_eq!(cds.len(), 4);
        // Top slices: a ~40 nm feature; bottom slices: nothing to measure.
        assert!(cds[0].is_some());
        assert!((cds[0].unwrap() - 40.0).abs() < 8.0);
        assert!(cds[3].is_none());
    }

    #[test]
    fn test_contrast_full() {
        let image = Array2::from_shape_vec((2, 2), vec![0.0, 1.0, 0.0, 1.0]).unwrap();
        assert_relative_eq!(image_contrast(&image), 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_contrast_zero() {
        let image = Array2::from_elem((4, 4), 0.5);
        assert_relative_eq!(image_contrast(&image), 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_nils_positive() {
        // Smooth feature (Gaussian bump centered at 0)
        let n = 200;
        let x_nm: Vec<f64> = (0..n).map(|i| -100.0 + i as f64).collect();
        let sigma = 25.0;
        let intensity: Vec<f64> = x_nm
            .iter()
            .map(|&x| (-x * x / (2.0 * sigma * sigma)).exp())
            .collect();

        let nils_val = nils(&intensity, &x_nm, 0.5);
        assert!(nils_val.is_some());
        assert!(nils_val.unwrap() > 0.0);
    }

    #[test]
    fn test_cd_no_crossings_returns_none() {
        // Flat profile (all same value) has no threshold crossings
        let n = 100;
        let x_nm: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let intensity: Vec<f64> = vec![0.8; n];
        assert!(measure_cd(&intensity, &x_nm, 0.5).is_none());
    }

    #[test]
    fn test_cd_empty_profile() {
        let result = measure_cd(&[], &[], 0.5);
        assert!(result.is_none());
        // Single-element slices (len < 2) should also return None
        let result2 = measure_cd(&[1.0], &[0.0], 0.5);
        assert!(result2.is_none());
    }

    #[test]
    fn test_nils_flat_profile_returns_none() {
        // Flat profile has no threshold crossings, so NILS is None
        let n = 100;
        let x_nm: Vec<f64> = (0..n).map(|i| -50.0 + i as f64).collect();
        let intensity: Vec<f64> = vec![0.7; n];
        assert!(nils(&intensity, &x_nm, 0.5).is_none());
    }

    #[test]
    fn test_mtf_uniform_is_zero() {
        // Uniform image has zero modulation
        let image = Array2::from_elem((64, 64), 0.5);
        assert_relative_eq!(mtf_from_image(&image), 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_cd_2d_measure() {
        // Create a step image: left half = 0, right half = 1
        // with a transition at x=0 and another at some other position
        let n = 128;
        let mut image = Array2::zeros((n, n));
        // Create a bright stripe in the center ~25% of the columns wide
        let left = n / 4;
        let right = 3 * n / 4;
        for i in 0..n {
            for j in left..right {
                image[[i, j]] = 1.0;
            }
        }
        // x range: -128 to 128 nm (pixel_nm = 2.0)
        let x_min = -128.0;
        let x_max = 128.0;
        let cd = measure_cd_2d(&image, x_min, x_max, 0.5);
        assert!(cd.is_some());
        // The stripe is half the width = 128nm, so CD should be ~128nm
        assert_relative_eq!(cd.unwrap(), 128.0, epsilon = 5.0);
    }
}
