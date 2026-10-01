//! Mask (reticle) geometry, complex transmittance and exact spectrum.
//!
//! A [`Mask`] is an ordered list of geometric [`MaskFeature`]s plus a
//! [`MaskType`] and a dark-/bright-field flag. Every feature is a region of
//! constant complex amplitude: clear (1) on a dark-field mask, the absorber
//! value on a bright-field mask, or `sqrt(T)` with zero phase for a
//! [`MaskFeature::GrayRect`] (the grayscale-lithography building block, which
//! ignores `dark_field`/`mask_type`). Where features overlap, the later one
//! wins (painter's rule).
//!
//! The absorber amplitude depends on [`MaskType`]: `Binary` chrome is opaque
//! (amplitude 0), [`MaskType::AttenuatedPSM`] carries `sqrt(T)·exp(iφ)` for its
//! transmission T and phase φ, and alternating PSM is treated as opaque here.
//!
//! # The simulation field is one unit cell
//!
//! An `N × N` grid of pixel pitch `p` covers the field `[−L/2, L/2)²` with
//! `L = N·p`, and the imaging FFTs make that field periodic. The mask model
//! matches this: geometry outside the field is clipped away, and the field is
//! one period of an infinitely repeated mask. The periodic primitives
//! [`MaskFeature::LineSpace`] and [`MaskFeature::RectArray`] describe infinite
//! gratings. They are imaged as such only if the field holds a whole number of
//! periods — see [`Mask::check_commensurate`], [`Mask::commensurate_grid`] and
//! [`GridConfig::commensurate`]. On an incommensurate field the grating is
//! truncated at the field edge and the periodic boundary adds a defect there.
//!
//! # Key equations
//!
//! ```text
//!   pixel centres      x_j = −L/2 + (j + ½)·p                        j = 0 … N−1
//!   FFT index → order  m = k  (k < N/2),   m = k − N  (k ≥ N/2)
//!   Fourier series     c(m) = (1/L²) ∬_field t(x,y)·exp(−2πi(mₓx + m_y·y)/L) dx dy
//!   spectrum           S[k] = N²·c(m)·exp(−iπ(mₓ + m_y)(N − 1)/N)
//!   interval [a, b]    (1/L)∫ₐᵇ exp(−2πimx/L) dx = ((b−a)/L)·sinc(πm(b−a)/L)·exp(−iπm(a+b)/L)
//!   grating (w, P, x₀) on a field L = M·P:
//!                      c(m) = (w/P)·sinc(πqw/P)·exp(−2πiqx₀/P)  if m = q·M, else 0
//!   pixel transfer     S[k] = FFT(pixel-aligned raster)[k]·sinc(πmₓ/N)·sinc(πm_y/N)
//! ```
//!
//! Here `sinc(u) = sin(u)/u`. With this scale and phase, `IFFT(S)` evaluates the
//! band-limited mask exactly at the pixel centres of [`crate::types::Grid2D`].
//! A feature centred at the origin therefore images centred at the origin, a
//! clear mask has `S[0,0] = N²`, and `S` equals `Fft2D::forward(rasterize(grid))`
//! in the fine-pixel limit.
//!
//! # Model status
//!
//! - **Thin-mask (Kirchhoff) approximation.** Every feature is a region of
//!   constant complex amplitude. There is no absorber topography, shadowing or
//!   edge diffraction (no mask-3D effects), and alternating-PSM phase regions
//!   are not modelled.
//! - **[`Mask::spectrum`] is exact for rectangles, gray rectangles and the
//!   periodic primitives.** It uses closed-form Fourier-series coefficients, so
//!   there is no aliasing and no pixel quantization; overlapping rectangles
//!   and gratings are first resolved into disjoint cells.
//! - **Simple polygons are exact too.** Their Fourier integral is evaluated in
//!   closed form edge by edge (divergence theorem) after clipping to the
//!   field.
//! - **Raster fallback.** Self-intersecting polygons, polygons overlapping
//!   another feature, and masks over the analytic cost budget use the
//!   antialiased raster, FFT'd and multiplied by the pixel transfer function.
//!   That is exact only for pixel-aligned edges; a sub-pixel edge leaves an
//!   error of first order in `p/L` at low orders (about 0.4 % of DC at the
//!   second order for 4 nm pixels on a 256 nm field). [`Mask::spectrum_method`]
//!   reports which path a mask takes on a given grid.
//! - **[`Mask::rasterize`] returns the area-averaged complex amplitude** of each
//!   pixel, with exact area coverage for every feature type. The one
//!   exception is the 4 × 4 supersampled painter's fallback, used only when a
//!   polygon overlaps another feature. On edge pixels `|⟨t⟩|² ≠ ⟨|t|²⟩`;
//!   [`Mask::rasterize_intensity`] gives the area-averaged intensity
//!   transmittance for geometric-shadow consumers.
//! - **Coordinates are at wafer scale** (after the projection reduction).

use ndarray::Array2;
use num::Complex;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

use crate::error::{LithographyError, Result};
use crate::math::fft2d::Fft2D;
use crate::types::{Complex64, GridConfig};

/// Relative tolerance of the "field holds a whole number of periods" test:
/// a pitch `P` is commensurate with a field `L` when
/// `|L/P − round(L/P)| ≤ COMMENSURATE_RTOL · L/P` and `round(L/P) ≥ 1`.
pub const COMMENSURATE_RTOL: f64 = 1e-9;

/// Cost budget (complex multiply-adds) of the analytic spectrum; larger masks
/// use the raster path.
const ANALYTIC_BUDGET: usize = 1 << 26;
/// Largest feature count checked pairwise for overlaps. Larger masks are
/// always resolved into disjoint cells (still exact).
const OVERLAP_CHECK_MAX: usize = 1024;
/// Largest cell grid (`n_x · n_y`) used to resolve overlapping rectangles.
const MAX_CELLS: usize = 1 << 20;
/// Supersampling factor per axis of the painter's fallback rasterizer.
const PAINTER_SUPERSAMPLE: usize = 4;
/// Gratings with more periods than this inside the field are replaced by
/// their zeroth order (area-weighted mean amplitude); every higher order lies
/// far beyond the grid's Nyquist frequency.
const MAX_GRATING_PERIODS: f64 = (1u64 << 20) as f64;

/// Mask type (determines phase and transmission of dark regions).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum MaskType {
    /// Binary chrome-on-glass mask.
    #[default]
    Binary,
    /// Attenuated phase-shift mask.
    AttenuatedPSM {
        /// Transmission of the absorber (typically 0.06 = 6%).
        transmission: f64,
        /// Phase shift in degrees (typically 180).
        phase_deg: f64,
    },
    /// Alternating aperture phase-shift mask.
    AlternatingPSM,
}

/// Orientation of a [`MaskFeature::LineSpace`] grating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LineOrientation {
    /// Lines run along y; the pattern repeats along x (default).
    #[default]
    Vertical,
    /// Lines run along x; the pattern repeats along y.
    Horizontal,
}

/// A geometric feature on the mask (at wafer scale, nm).
///
/// Serde uses the external variant tag (`{"Rect": {...}}`); the variants that
/// existed before the periodic primitives keep their names and fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MaskFeature {
    /// Rectangle defined by center (x, y) and dimensions (w, h).
    Rect { x: f64, y: f64, w: f64, h: f64 },
    /// Polygon defined by vertices (x, y pairs), filled with the even-odd rule.
    Polygon { vertices: Vec<(f64, f64)> },
    /// Rectangle with its own intensity transmittance in [0, 1]
    /// (rasterized as amplitude sqrt(T), zero phase). The building block
    /// for grayscale lithography masks; ignores `dark_field`/`mask_type`
    /// because it carries its own transmission.
    GrayRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        transmittance: f64,
    },
    /// Infinite grating of lines of width `cd` repeating every `pitch`, with
    /// one line centred at `offset` along the periodic axis. It spans the
    /// whole field along the line direction. On a bright-field mask the lines
    /// are absorber (opaque lines, clear spaces).
    LineSpace {
        cd: f64,
        pitch: f64,
        #[serde(default)]
        orientation: LineOrientation,
        #[serde(default)]
        offset: f64,
    },
    /// Infinite lattice of `w × h` rectangles repeating every `pitch_x` along
    /// x and `pitch_y` along y, one centred at (`offset_x`, `offset_y`). On a
    /// dark-field mask these are clear contact holes.
    RectArray {
        w: f64,
        h: f64,
        pitch_x: f64,
        pitch_y: f64,
        #[serde(default)]
        offset_x: f64,
        #[serde(default)]
        offset_y: f64,
    },
}

impl MaskFeature {
    /// Width of one element across its edges: `w` of a rectangle or array
    /// element, `cd` of a grating line. `None` for polygons.
    pub fn element_width_nm(&self) -> Option<f64> {
        match *self {
            MaskFeature::Rect { w, .. }
            | MaskFeature::GrayRect { w, .. }
            | MaskFeature::RectArray { w, .. } => Some(w),
            MaskFeature::LineSpace { cd, .. } => Some(cd),
            MaskFeature::Polygon { .. } => None,
        }
    }

    /// The feature with its element edges moved outward by `bias_nm`
    /// (negative shrinks), keeping centres and pitches: rectangles grow in
    /// width only (the OPC convention for lines), grating lines in `cd`, array
    /// elements in both `w` and `h`. Widths are clamped to stay positive and
    /// below the pitch. Polygons are returned unchanged.
    pub fn with_edge_bias(&self, bias_nm: f64) -> MaskFeature {
        let grow =
            |w: f64, limit: f64| (w + 2.0 * bias_nm).clamp(1e-6 * limit, limit * (1.0 - 1e-9));
        let mut out = self.clone();
        match &mut out {
            MaskFeature::Rect { w, .. } | MaskFeature::GrayRect { w, .. } => {
                *w = (*w + 2.0 * bias_nm).max(1e-6)
            }
            MaskFeature::LineSpace { cd, pitch, .. } => *cd = grow(*cd, *pitch),
            MaskFeature::RectArray {
                w,
                h,
                pitch_x,
                pitch_y,
                ..
            } => {
                *w = grow(*w, *pitch_x);
                *h = grow(*h, *pitch_y);
            }
            MaskFeature::Polygon { .. } => {}
        }
        out
    }
}

/// Mask / reticle specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mask {
    pub mask_type: MaskType,
    /// Geometric features, painted in order (later features win overlaps).
    pub features: Vec<MaskFeature>,
    /// `false` (bright field): clear background, features carry the absorber
    /// value. `true` (dark field): absorber background, clear features.
    pub dark_field: bool,
}

/// How [`Mask::spectrum`] evaluates a mask on a given grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpectrumMethod {
    /// Every feature from closed-form Fourier coefficients (exact).
    Analytic,
    /// Everything analytic except self-intersecting polygons, which come from
    /// their exact-coverage raster times the pixel transfer function.
    Mixed,
    /// The whole mask from its antialiased raster times the pixel transfer
    /// function: exact for pixel-aligned edges, approximate otherwise. Used
    /// when a polygon overlaps another feature or the analytic cost budget is
    /// exceeded.
    Raster,
}

impl Mask {
    /// Line/space grating: bright field, opaque lines of width `cd_nm` on a
    /// `pitch_nm` period, one line centred at x = 0, lines running along y.
    ///
    /// The grating is infinite, so it fills the whole field. Image it on a
    /// grid whose field holds a whole number of periods
    /// ([`Mask::commensurate_grid`]).
    pub fn line_space(cd_nm: f64, pitch_nm: f64) -> Result<Self> {
        Self::line_space_with(cd_nm, pitch_nm, LineOrientation::Vertical, 0.0)
    }

    /// Line/space grating with an explicit orientation and the position
    /// (`offset_nm`, along the periodic axis) of one line centre.
    pub fn line_space_with(
        cd_nm: f64,
        pitch_nm: f64,
        orientation: LineOrientation,
        offset_nm: f64,
    ) -> Result<Self> {
        check_positive("cd_nm", cd_nm)?;
        check_positive("pitch_nm", pitch_nm)?;
        if cd_nm >= pitch_nm {
            return Err(LithographyError::InvalidParameter {
                name: "cd_nm",
                value: cd_nm,
                reason: "must be less than pitch_nm",
            });
        }
        check_finite("offset_nm", offset_nm)?;
        Ok(Self {
            mask_type: MaskType::Binary,
            features: vec![MaskFeature::LineSpace {
                cd: cd_nm,
                pitch: pitch_nm,
                orientation,
                offset: offset_nm,
            }],
            dark_field: false,
        })
    }

    /// Contact-hole array: dark field, clear square holes of side
    /// `diameter_nm` on a (`pitch_x_nm`, `pitch_y_nm`) lattice, one hole
    /// centred at the origin. Like [`Mask::line_space`], the lattice is
    /// infinite (periodic in both axes).
    pub fn contact_hole(diameter_nm: f64, pitch_x_nm: f64, pitch_y_nm: f64) -> Result<Self> {
        check_positive("diameter_nm", diameter_nm)?;
        check_positive("pitch_x_nm", pitch_x_nm)?;
        check_positive("pitch_y_nm", pitch_y_nm)?;
        if diameter_nm >= pitch_x_nm.min(pitch_y_nm) {
            return Err(LithographyError::InvalidParameter {
                name: "diameter_nm",
                value: diameter_nm,
                reason: "must be less than both pitches (neighbouring holes would merge)",
            });
        }
        Ok(Self {
            mask_type: MaskType::Binary,
            features: vec![MaskFeature::RectArray {
                w: diameter_nm,
                h: diameter_nm,
                pitch_x: pitch_x_nm,
                pitch_y: pitch_y_nm,
                offset_x: 0.0,
                offset_y: 0.0,
            }],
            dark_field: true,
        })
    }

    /// Check every feature and the mask type for invalid parameters
    /// (non-finite values, non-positive sizes, widths not smaller than their
    /// pitch, polygons with fewer than three vertices, absorber transmission
    /// outside [0, 1]).
    ///
    /// [`Mask::rasterize`] and [`Mask::spectrum`] never fail: they skip empty
    /// or non-finite features, clamp gray transmittances into [0, 1] and treat
    /// a grating whose width reaches its pitch as fully covered. This method
    /// reports those cases instead.
    pub fn validate(&self) -> Result<()> {
        if let MaskType::AttenuatedPSM {
            transmission,
            phase_deg,
        } = self.mask_type
        {
            if !(0.0..=1.0).contains(&transmission) {
                return Err(LithographyError::InvalidParameter {
                    name: "transmission",
                    value: transmission,
                    reason: "absorber transmission must lie in [0, 1]",
                });
            }
            check_finite("phase_deg", phase_deg)?;
        }
        for feature in &self.features {
            match *feature {
                MaskFeature::Rect { x, y, w, h } => {
                    check_finite("x", x)?;
                    check_finite("y", y)?;
                    check_positive("w", w)?;
                    check_positive("h", h)?;
                }
                MaskFeature::GrayRect {
                    x,
                    y,
                    w,
                    h,
                    transmittance,
                } => {
                    check_finite("x", x)?;
                    check_finite("y", y)?;
                    check_positive("w", w)?;
                    check_positive("h", h)?;
                    if !(0.0..=1.0).contains(&transmittance) {
                        return Err(LithographyError::InvalidParameter {
                            name: "transmittance",
                            value: transmittance,
                            reason: "gray transmittance must lie in [0, 1]",
                        });
                    }
                }
                MaskFeature::Polygon { ref vertices } => {
                    if vertices.len() < 3 {
                        return Err(LithographyError::InvalidParameter {
                            name: "vertices",
                            value: vertices.len() as f64,
                            reason: "a polygon needs at least 3 vertices",
                        });
                    }
                    for &(vx, vy) in vertices {
                        check_finite("vertex x", vx)?;
                        check_finite("vertex y", vy)?;
                    }
                }
                MaskFeature::LineSpace {
                    cd, pitch, offset, ..
                } => {
                    check_positive("cd", cd)?;
                    check_positive("pitch", pitch)?;
                    check_finite("offset", offset)?;
                    if cd >= pitch {
                        return Err(LithographyError::InvalidParameter {
                            name: "cd",
                            value: cd,
                            reason: "line width must be less than the pitch",
                        });
                    }
                }
                MaskFeature::RectArray {
                    w,
                    h,
                    pitch_x,
                    pitch_y,
                    offset_x,
                    offset_y,
                } => {
                    check_positive("w", w)?;
                    check_positive("h", h)?;
                    check_positive("pitch_x", pitch_x)?;
                    check_positive("pitch_y", pitch_y)?;
                    check_finite("offset_x", offset_x)?;
                    check_finite("offset_y", offset_y)?;
                    if w >= pitch_x || h >= pitch_y {
                        return Err(LithographyError::InvalidParameter {
                            name: "w/h",
                            value: w.max(h),
                            reason: "array element must be smaller than its pitch",
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Periods the square simulation field must hold a whole number of, or
    /// `None` if the mask has no periodic primitive.
    ///
    /// Because the field is square, the axis a period belongs to does not
    /// matter. A single distinct period gives `(P, None)` (vertical or
    /// horizontal lines, square arrays); two give `(P₁, Some(P₂))`; more are
    /// folded into their common period. The result feeds
    /// [`GridConfig::commensurate`] directly. If three or more pitches have
    /// no common period, the first two are returned and
    /// [`Mask::check_commensurate`] reports the conflict.
    pub fn periodicity(&self) -> Option<(f64, Option<f64>)> {
        let mut periods: Vec<f64> = Vec::new();
        let mut push = |p: f64| {
            if p.is_finite()
                && p > 0.0
                && !periods.iter().any(|&q| (q - p).abs() <= 1e-9 * q.max(p))
            {
                periods.push(p);
            }
        };
        for feature in &self.features {
            match *feature {
                MaskFeature::LineSpace { pitch, .. } => push(pitch),
                MaskFeature::RectArray {
                    pitch_x, pitch_y, ..
                } => {
                    push(pitch_x);
                    push(pitch_y);
                }
                _ => {}
            }
        }
        match periods.len() {
            0 => None,
            1 => Some((periods[0], None)),
            2 => Some((periods[0], Some(periods[1]))),
            _ => {
                let mut cell = periods[0];
                for &p in &periods[1..] {
                    match crate::types::common_period_nm(cell, p) {
                        Some(c) => cell = c,
                        None => return Some((periods[0], Some(periods[1]))),
                    }
                }
                Some((cell, None))
            }
        }
    }

    /// Check that the simulation field holds a whole number of periods of
    /// every periodic primitive in the mask.
    ///
    /// The FFTs make the field periodic. A grating whose pitch does not
    /// divide the field is therefore imaged with a defect (one wrong line or
    /// space) at the field edge, which silently corrupts contrast, NILS, CD
    /// and process windows. The error names the offending pitch, the
    /// fractional period count, and a commensurate grid that fixes it.
    pub fn check_commensurate(&self, grid: &GridConfig) -> Result<()> {
        let field = grid.field_size_nm();
        for feature in &self.features {
            let (label, pitches): (&str, [f64; 2]) = match *feature {
                MaskFeature::LineSpace { pitch, .. } => ("line/space grating", [pitch, pitch]),
                MaskFeature::RectArray {
                    pitch_x, pitch_y, ..
                } => ("rectangle array", [pitch_x, pitch_y]),
                _ => continue,
            };
            for p in pitches {
                check_positive("pitch", p)?;
                if grid.is_commensurate_with(p) {
                    continue;
                }
                let periods = field / p;
                let fix = match self.periodicity().map(|(p1, p2)| {
                    (
                        p1,
                        p2,
                        GridConfig::commensurate(p1, p2, grid.size, grid.pixel_nm),
                    )
                }) {
                    Some((p1, p2, Ok(g))) => {
                        let k = (g.field_size_nm() / p).round();
                        format!(
                            "use GridConfig::commensurate({p1}, {p2:?}, {}, {}) -> pixel {} nm, \
                             field {} nm ({k} period{}), or pick size·pixel_nm as a whole \
                             multiple of the pitch",
                            grid.size,
                            grid.pixel_nm,
                            g.pixel_nm,
                            g.field_size_nm(),
                            if k == 1.0 { "" } else { "s" }
                        )
                    }
                    Some((_, _, Err(e))) => format!("choose pitches with a common period ({e})"),
                    None => "pick size·pixel_nm as a whole multiple of the pitch".to_string(),
                };
                return Err(LithographyError::DimensionMismatch {
                    expected: format!(
                        "a simulation field ({} px × {} nm = {field} nm) holding a whole number of \
                         periods of the {p} nm pitch ({label})",
                        grid.size, grid.pixel_nm
                    ),
                    got: format!(
                        "{periods:.6} periods — the FFT's periodic boundary would insert a defect at \
                         the field edge. Fix: {fix}"
                    ),
                });
            }
        }
        Ok(())
    }

    /// A `size`-pixel grid whose field holds a whole number of periods of
    /// every periodic primitive, with the pixel as close to (and not coarser
    /// than) `target_pixel_nm` as possible — see [`GridConfig::commensurate`].
    /// Masks without periodic primitives get `GridConfig::new(size,
    /// target_pixel_nm)`.
    pub fn commensurate_grid(&self, size: usize, target_pixel_nm: f64) -> Result<GridConfig> {
        let grid = match self.periodicity() {
            Some((p1, p2)) => GridConfig::commensurate(p1, p2, size, target_pixel_nm)?,
            None => GridConfig::new(size, target_pixel_nm)?,
        };
        self.check_commensurate(&grid)?;
        Ok(grid)
    }

    /// One-line human-readable description (used by the frontends' `repr`).
    pub fn summary(&self) -> String {
        let field = if self.dark_field {
            "dark field"
        } else {
            "bright field"
        };
        let kind = match self.mask_type {
            MaskType::Binary => "binary".to_string(),
            MaskType::AttenuatedPSM {
                transmission,
                phase_deg,
            } => format!("att-PSM {:.0}% {phase_deg:.0}°", transmission * 100.0),
            MaskType::AlternatingPSM => "alt-PSM (treated as binary)".to_string(),
        };
        let what = match self.features.as_slice() {
            [MaskFeature::LineSpace {
                cd,
                pitch,
                orientation,
                ..
            }] => format!("line/space {cd} nm / {pitch} nm ({orientation:?})"),
            [MaskFeature::RectArray {
                w,
                h,
                pitch_x,
                pitch_y,
                ..
            }] => format!("{w}×{h} nm array on {pitch_x}×{pitch_y} nm"),
            features => format!("{} features", features.len()),
        };
        format!("{what}, {field}, {kind}")
    }

    /// Rasterize the mask to its complex transmittance on `grid`.
    ///
    /// Each pixel holds the area average of the complex amplitude over the
    /// pixel (area-coverage antialiasing). Coverage is exact for rectangles,
    /// gray rectangles, gratings and polygons, and overlapping rectangles are
    /// resolved exactly by the painter's rule. The painter's fallback samples
    /// 4 × 4 points per pixel; it is used only when a polygon overlaps
    /// another feature or the overlap resolution would exceed its cell budget.
    pub fn rasterize(&self, grid: &GridConfig) -> Array2<Complex64> {
        self.raster_mapped(grid, |v| v)
    }

    /// Area-averaged intensity transmittance `⟨|t|²⟩` of each pixel.
    ///
    /// Equal to `|rasterize(grid)|²` away from edges. On edge pixels it is the
    /// area-weighted mix of the intensities, not the square of the mixed
    /// amplitude — the right input for geometric shadow projection
    /// (proximity/contact printing, deep X-ray).
    pub fn rasterize_intensity(&self, grid: &GridConfig) -> Array2<f64> {
        self.raster_mapped(grid, |v| Complex64::new(v.norm_sqr(), 0.0))
            .mapv(|v| v.re)
    }

    /// Mask spectrum on `grid`, in the layout and scale of
    /// `fft.forward(rasterize(grid))`: `S[k] = N²·c(m)·exp(−iπ(mₓ+m_y)(N−1)/N)`
    /// with `c(m)` the exact Fourier-series coefficient of the continuous mask
    /// over the field (see the module docs).
    ///
    /// Rectangles, gray rectangles, gratings and simple polygons are evaluated
    /// in closed form: no aliasing, no pixel quantization, and sub-pixel edge
    /// positions are honoured exactly. Self-intersecting polygons, polygons
    /// overlapping other features, and masks over the analytic cost budget
    /// use the antialiased raster times the pixel transfer function
    /// ([`Mask::spectrum_method`]).
    pub fn spectrum(&self, grid: &GridConfig, fft: &Fft2D) -> Array2<Complex64> {
        let field = Field::new(grid);
        let n = field.n;
        if n == 0 {
            return Array2::zeros((0, 0));
        }
        let plan = self.plan(&field);
        match plan.method(n) {
            SpectrumMethod::Raster => {
                let mut t = raster_from_plan(&plan, &field, |v| v);
                fft.forward(&mut t);
                apply_pixel_transfer(&mut t);
                t
            }
            SpectrumMethod::Analytic | SpectrumMethod::Mixed => {
                let mut spec = Array2::zeros((n, n));
                spec[[0, 0]] = plan.bg * (n * n) as f64;
                for term in &plan.terms {
                    add_term_spectrum(&mut spec, term, plan.bg, &field);
                }
                let mut raster: Option<Array2<Complex64>> = None;
                for poly in &plan.polygons {
                    let w = poly.value - plan.bg;
                    match &poly.clipped {
                        Some(clipped) => add_polygon_spectrum(&mut spec, w, clipped, &field),
                        None => add_polygon_raster(
                            raster.get_or_insert_with(|| Array2::zeros((n, n))),
                            w,
                            &poly.vertices,
                            &field,
                        ),
                    }
                }
                if let Some(mut r) = raster {
                    fft.forward(&mut r);
                    apply_pixel_transfer(&mut r);
                    spec += &r;
                }
                spec
            }
        }
    }

    /// Which evaluation path [`Mask::spectrum`] takes for this mask on `grid`.
    pub fn spectrum_method(&self, grid: &GridConfig) -> SpectrumMethod {
        let field = Field::new(grid);
        self.plan(&field).method(field.n)
    }

    /// Complex amplitude of the absorber for this mask type.
    pub fn absorber_transmittance(&self) -> Complex64 {
        match &self.mask_type {
            MaskType::Binary => Complex64::new(0.0, 0.0),
            MaskType::AttenuatedPSM {
                transmission,
                phase_deg,
            } => {
                let amp = transmission.clamp(0.0, 1.0).sqrt();
                let phase = phase_deg.to_radians();
                Complex::from_polar(amp, phase)
            }
            MaskType::AlternatingPSM => Complex64::new(0.0, 0.0),
        }
    }

    /// Complex amplitude of the background (clear on bright field, absorber
    /// on dark field).
    pub fn background_transmittance(&self) -> Complex64 {
        if self.dark_field {
            self.absorber_transmittance()
        } else {
            Complex64::new(1.0, 0.0)
        }
    }

    /// Complex amplitude of every non-gray feature (clear on dark field,
    /// absorber on bright field).
    pub fn feature_transmittance(&self) -> Complex64 {
        if self.dark_field {
            Complex64::new(1.0, 0.0)
        } else {
            self.absorber_transmittance()
        }
    }

    fn raster_mapped(
        &self,
        grid: &GridConfig,
        map: impl Fn(Complex64) -> Complex64 + Copy,
    ) -> Array2<Complex64> {
        let field = Field::new(grid);
        if field.n == 0 {
            return Array2::zeros((0, 0));
        }
        raster_from_plan(&self.plan(&field), &field, map)
    }

    /// Resolve the features against the field and decide how to evaluate
    /// them: independent separable terms (no overlaps), disjoint cells
    /// (overlaps resolved exactly), or the painter's fallback.
    fn plan(&self, field: &Field) -> Plan {
        let bg = self.background_transmittance();
        let fg = self.feature_transmittance();
        let resolved: Vec<Resolved> = self
            .features
            .iter()
            .filter_map(|f| resolve(f, fg, bg, field))
            .collect();
        let mut plan = Plan {
            bg,
            resolved: Vec::new(),
            terms: Vec::new(),
            polygons: Vec::new(),
            painter: false,
        };
        let overlap = resolved.len() > OVERLAP_CHECK_MAX || any_overlap(&resolved, field);
        if !overlap {
            for r in &resolved {
                match &r.shape {
                    Shape::Separable { x, y } => plan.terms.push(Term {
                        y: y.clone(),
                        xs: vec![(r.value, x.clone())],
                    }),
                    Shape::Polygon(v) => plan.polygons.push(PolygonTerm::new(r.value, v, field)),
                }
            }
        } else if let Some(terms) = compress(&resolved, bg, field) {
            plan.terms = terms;
        } else {
            plan.painter = true;
        }
        plan.resolved = resolved;
        plan
    }
}

impl Default for Mask {
    fn default() -> Self {
        Self::line_space(65.0, 180.0).expect("default mask parameters are valid")
    }
}

fn check_finite(name: &'static str, value: f64) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(LithographyError::InvalidParameter {
            name,
            value,
            reason: "must be finite",
        })
    }
}

fn check_positive(name: &'static str, value: f64) -> Result<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(LithographyError::InvalidParameter {
            name,
            value,
            reason: "must be positive and finite",
        })
    }
}

/// `sin(u)/u`, with the removable singularity filled in.
fn sinc(u: f64) -> f64 {
    if u.abs() < 1e-6 {
        1.0 - u * u / 6.0
    } else {
        u.sin() / u
    }
}

/// Signed diffraction order of FFT index `k` on an `n`-point axis (the same
/// convention as the imaging engine's frequency loops).
fn signed_order(k: usize, n: usize) -> i64 {
    if k < n / 2 {
        k as i64
    } else {
        k as i64 - n as i64
    }
}

/// Multiply a spectrum in FFT layout by `sinc(πmₓ/N)·sinc(πm_y/N)`, turning the
/// DFT of a pixel-aligned raster into the exact Fourier coefficients of the
/// piecewise-constant mask it represents.
fn apply_pixel_transfer(spec: &mut Array2<Complex64>) {
    let (ny, nx) = spec.dim();
    let tx: Vec<f64> = (0..nx)
        .map(|k| sinc(PI * signed_order(k, nx) as f64 / nx as f64))
        .collect();
    let ty: Vec<f64> = (0..ny)
        .map(|k| sinc(PI * signed_order(k, ny) as f64 / ny as f64))
        .collect();
    for ((i, j), v) in spec.indexed_iter_mut() {
        *v *= tx[j] * ty[i];
    }
}

/// The simulation field `[−half, half)²` sampled by `n × n` pixels.
#[derive(Debug, Clone, Copy)]
struct Field {
    n: usize,
    pixel: f64,
    half: f64,
}

impl Field {
    fn new(grid: &GridConfig) -> Self {
        Self {
            n: grid.size,
            pixel: grid.pixel_nm,
            half: grid.field_size_nm() / 2.0,
        }
    }

    fn len(&self) -> f64 {
        2.0 * self.half
    }

    /// Length below which an overlap or interval counts as empty.
    fn eps(&self) -> f64 {
        1e-9 * self.len()
    }
}

/// Closed-form description of a separable support along one axis.
#[derive(Debug, Clone, Copy, PartialEq)]
enum AxisKind {
    /// Coefficients by summing the intervals.
    Finite,
    /// Covers the whole axis: `c(m) = δ(m)`.
    Full,
    /// Grating commensurate with the field (`periods` whole periods).
    Grating {
        width: f64,
        pitch: f64,
        offset: f64,
        periods: i64,
    },
}

/// Support of a separable feature along one axis, clipped to the field.
#[derive(Debug, Clone)]
struct Axis {
    /// Sorted, disjoint intervals inside `[−half, half]`.
    intervals: Vec<(f64, f64)>,
    kind: AxisKind,
}

impl Axis {
    fn full(field: &Field) -> Self {
        Self {
            intervals: vec![(-field.half, field.half)],
            kind: AxisKind::Full,
        }
    }

    fn finite(intervals: Vec<(f64, f64)>) -> Self {
        Self {
            intervals,
            kind: AxisKind::Finite,
        }
    }

    /// `[a, b]` clipped to the field; `None` if nothing is left.
    fn from_interval(a: f64, b: f64, field: &Field) -> Option<Self> {
        let lo = a.max(-field.half);
        let hi = b.min(field.half);
        if hi - lo <= 0.0 {
            return None;
        }
        let eps = field.eps();
        if lo <= -field.half + eps && hi >= field.half - eps {
            return Some(Self::full(field));
        }
        Some(Self::finite(vec![(lo, hi)]))
    }

    /// Number of FFT indices with a non-zero coefficient (upper bound).
    fn nnz(&self, n: usize) -> usize {
        match self.kind {
            AxisKind::Full => 1,
            AxisKind::Grating { periods, .. } => n / periods.max(1) as usize + 1,
            AxisKind::Finite => n,
        }
    }
}

/// Zeroth-order (effective-medium) fraction and axis of a grating of `width`
/// on `pitch` with one element centred at `offset`, resolved on the field.
/// Returns `None` for an empty or invalid grating.
fn grating_axis(width: f64, pitch: f64, offset: f64, field: &Field) -> Option<(Axis, f64)> {
    if !(width.is_finite() && pitch.is_finite() && offset.is_finite()) || width <= 0.0 {
        return None;
    }
    if pitch <= 0.0 {
        return None;
    }
    if width >= pitch {
        return Some((Axis::full(field), 1.0));
    }
    let l = field.len();
    let periods = l / pitch;
    if periods > MAX_GRATING_PERIODS {
        // Far below the pixel: only the zeroth order is representable.
        return Some((Axis::full(field), width / pitch));
    }
    let m = periods.round();
    let commensurate = m >= 1.0 && (periods - m).abs() <= COMMENSURATE_RTOL * periods;
    let half = field.half;
    let k_lo = ((-half - offset - width / 2.0) / pitch).floor() as i64;
    let k_hi = ((half - offset + width / 2.0) / pitch).ceil() as i64;
    let mut intervals = Vec::with_capacity((k_hi - k_lo + 1).max(0) as usize);
    for k in k_lo..=k_hi {
        let c = offset + k as f64 * pitch;
        let lo = (c - width / 2.0).max(-half);
        let hi = (c + width / 2.0).min(half);
        if hi > lo {
            intervals.push((lo, hi));
        }
    }
    if intervals.is_empty() {
        return None;
    }
    let kind = if commensurate {
        AxisKind::Grating {
            width,
            pitch,
            offset,
            periods: m as i64,
        }
    } else {
        AxisKind::Finite
    };
    Some((Axis { intervals, kind }, 1.0))
}

/// A feature resolved against the field.
#[derive(Debug, Clone)]
struct Resolved {
    /// Complex amplitude inside the feature.
    value: Complex64,
    shape: Shape,
    /// Clipped bounding box `[x_min, x_max, y_min, y_max]`.
    bbox: [f64; 4],
}

#[derive(Debug, Clone)]
enum Shape {
    /// Product of an x support and a y support (rectangles, gratings).
    Separable { x: Axis, y: Axis },
    /// Even-odd filled polygon.
    Polygon(Vec<(f64, f64)>),
}

fn separable(value: Complex64, x: Axis, y: Axis) -> Resolved {
    let bbox = [
        x.intervals[0].0,
        x.intervals[x.intervals.len() - 1].1,
        y.intervals[0].0,
        y.intervals[y.intervals.len() - 1].1,
    ];
    Resolved {
        value,
        shape: Shape::Separable { x, y },
        bbox,
    }
}

fn rect_axes(x: f64, y: f64, w: f64, h: f64, field: &Field) -> Option<(Axis, Axis)> {
    if !(x.is_finite() && y.is_finite() && w.is_finite() && h.is_finite()) || w <= 0.0 || h <= 0.0 {
        return None;
    }
    let ax = Axis::from_interval(x - w / 2.0, x + w / 2.0, field)?;
    let ay = Axis::from_interval(y - h / 2.0, y + h / 2.0, field)?;
    Some((ax, ay))
}

fn resolve(feature: &MaskFeature, fg: Complex64, bg: Complex64, field: &Field) -> Option<Resolved> {
    match *feature {
        MaskFeature::Rect { x, y, w, h } => {
            rect_axes(x, y, w, h, field).map(|(ax, ay)| separable(fg, ax, ay))
        }
        MaskFeature::GrayRect {
            x,
            y,
            w,
            h,
            transmittance,
        } => {
            if !transmittance.is_finite() {
                return None;
            }
            let value = Complex64::new(transmittance.clamp(0.0, 1.0).sqrt(), 0.0);
            rect_axes(x, y, w, h, field).map(|(ax, ay)| separable(value, ax, ay))
        }
        MaskFeature::Polygon { ref vertices } => {
            if vertices.len() < 3
                || vertices
                    .iter()
                    .any(|&(vx, vy)| !(vx.is_finite() && vy.is_finite()))
            {
                return None;
            }
            let mut bbox = [
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ];
            for &(vx, vy) in vertices {
                bbox[0] = bbox[0].min(vx);
                bbox[1] = bbox[1].max(vx);
                bbox[2] = bbox[2].min(vy);
                bbox[3] = bbox[3].max(vy);
            }
            bbox[0] = bbox[0].max(-field.half);
            bbox[1] = bbox[1].min(field.half);
            bbox[2] = bbox[2].max(-field.half);
            bbox[3] = bbox[3].min(field.half);
            if bbox[1] <= bbox[0] || bbox[3] <= bbox[2] {
                return None;
            }
            Some(Resolved {
                value: fg,
                shape: Shape::Polygon(vertices.clone()),
                bbox,
            })
        }
        MaskFeature::LineSpace {
            cd,
            pitch,
            orientation,
            offset,
        } => {
            let (grating, fraction) = grating_axis(cd, pitch, offset, field)?;
            let value = bg + (fg - bg) * fraction;
            let (ax, ay) = match orientation {
                LineOrientation::Vertical => (grating, Axis::full(field)),
                LineOrientation::Horizontal => (Axis::full(field), grating),
            };
            Some(separable(value, ax, ay))
        }
        MaskFeature::RectArray {
            w,
            h,
            pitch_x,
            pitch_y,
            offset_x,
            offset_y,
        } => {
            let (ax, fx) = grating_axis(w, pitch_x, offset_x, field)?;
            let (ay, fy) = grating_axis(h, pitch_y, offset_y, field)?;
            let value = bg + (fg - bg) * (fx * fy);
            Some(separable(value, ax, ay))
        }
    }
}

/// True if two sorted interval lists intersect over more than `eps`.
fn intervals_overlap(a: &[(f64, f64)], b: &[(f64, f64)], eps: f64) -> bool {
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        let lo = a[i].0.max(b[j].0);
        let hi = a[i].1.min(b[j].1);
        if hi - lo > eps {
            return true;
        }
        if a[i].1 < b[j].1 {
            i += 1;
        } else {
            j += 1;
        }
    }
    false
}

/// True if any two features overlap with positive area. Pairs involving a
/// polygon count as overlapping when their bounding boxes overlap
/// (conservative).
fn any_overlap(res: &[Resolved], field: &Field) -> bool {
    let eps = field.eps();
    for i in 0..res.len() {
        for j in 0..i {
            let (a, b) = (&res[i], &res[j]);
            let bbox_hit = a.bbox[1].min(b.bbox[1]) - a.bbox[0].max(b.bbox[0]) > eps
                && a.bbox[3].min(b.bbox[3]) - a.bbox[2].max(b.bbox[2]) > eps;
            if !bbox_hit {
                continue;
            }
            match (&a.shape, &b.shape) {
                (Shape::Separable { x: ax, y: ay }, Shape::Separable { x: bx, y: by }) => {
                    if intervals_overlap(&ax.intervals, &bx.intervals, eps)
                        && intervals_overlap(&ay.intervals, &by.intervals, eps)
                    {
                        return true;
                    }
                }
                _ => return true,
            }
        }
    }
    false
}

/// A separable term of the mask: a y support times a sum of x supports, each
/// with its own complex amplitude (the background is subtracted when the term
/// is evaluated).
#[derive(Debug, Clone)]
struct Term {
    y: Axis,
    xs: Vec<(Complex64, Axis)>,
}

impl Term {
    /// Estimated complex multiply-adds of [`add_term_spectrum`].
    fn cost(&self, n: usize) -> usize {
        let ny = self.y.nnz(n);
        let nx = self
            .xs
            .iter()
            .map(|(_, a)| a.nnz(n))
            .fold(0usize, usize::saturating_add)
            .min(n);
        let intervals: usize = self.y.intervals.len()
            + self
                .xs
                .iter()
                .map(|(_, a)| a.intervals.len())
                .sum::<usize>();
        ny.saturating_mul(nx)
            .saturating_add(n.saturating_mul(intervals + self.xs.len()))
    }
}

/// Evaluation plan of a mask on one field.
struct Plan {
    bg: Complex64,
    resolved: Vec<Resolved>,
    terms: Vec<Term>,
    /// Non-overlapping polygons.
    polygons: Vec<PolygonTerm>,
    /// Overlaps could not be resolved exactly: use the painter's fallback.
    painter: bool,
}

impl Plan {
    fn method(&self, n: usize) -> SpectrumMethod {
        if self.painter {
            return SpectrumMethod::Raster;
        }
        let cost = self
            .terms
            .iter()
            .map(|t| t.cost(n))
            .chain(self.polygons.iter().map(|p| p.cost(n)))
            .fold(0usize, usize::saturating_add);
        if cost > ANALYTIC_BUDGET {
            SpectrumMethod::Raster
        } else if self.polygons.iter().all(|p| p.clipped.is_some()) {
            SpectrumMethod::Analytic
        } else {
            SpectrumMethod::Mixed
        }
    }
}

/// A polygon that overlaps nothing else.
struct PolygonTerm {
    value: Complex64,
    vertices: Vec<(f64, f64)>,
    /// The polygon clipped to the field, if it is simple (no two edges cross)
    /// and its spectrum can therefore be evaluated in closed form.
    clipped: Option<Vec<(f64, f64)>>,
}

impl PolygonTerm {
    fn new(value: Complex64, vertices: &[(f64, f64)], field: &Field) -> Self {
        let clipped = polygon_is_simple(vertices).then(|| clip_polygon(vertices, field.half));
        Self {
            value,
            vertices: vertices.to_vec(),
            clipped,
        }
    }

    fn cost(&self, n: usize) -> usize {
        match &self.clipped {
            Some(v) => n.saturating_mul(n).saturating_mul(v.len() + 1),
            None => 0,
        }
    }
}

/// Orientation of `c` relative to the directed line `a → b` (twice the
/// signed triangle area).
fn orient(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// True if no two non-adjacent edges cross properly (touching and collinear
/// contacts are allowed). The even-odd area of such a polygon equals its
/// Green's-theorem (winding) area, which [`add_polygon_spectrum`] integrates.
fn polygon_is_simple(v: &[(f64, f64)]) -> bool {
    let n = v.len();
    for a in 0..n {
        let (a0, a1) = (v[a], v[(a + 1) % n]);
        for b in a + 2..n {
            if a == 0 && b == n - 1 {
                continue; // adjacent through the closing vertex
            }
            let (b0, b1) = (v[b], v[(b + 1) % n]);
            if orient(a0, a1, b0) * orient(a0, a1, b1) < 0.0
                && orient(b0, b1, a0) * orient(b0, b1, a1) < 0.0
            {
                return false;
            }
        }
    }
    true
}

/// Sutherland–Hodgman clip of a polygon to the square `[−half, half]²`.
/// Concave polygons may gain zero-width slivers along the square's border;
/// their contributions cancel in the contour integral.
fn clip_polygon(vertices: &[(f64, f64)], half: f64) -> Vec<(f64, f64)> {
    let mut poly = vertices.to_vec();
    for (axis, sign) in [(0, -1.0), (0, 1.0), (1, -1.0), (1, 1.0)] {
        let coord = |p: (f64, f64)| if axis == 0 { p.0 } else { p.1 };
        let inside = |p: (f64, f64)| sign * coord(p) <= half;
        let bound = sign * half;
        let cut = |a: (f64, f64), b: (f64, f64)| {
            let t = (bound - coord(a)) / (coord(b) - coord(a));
            (a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1))
        };
        let len = poly.len();
        let mut out = Vec::with_capacity(len + 4);
        for i in 0..len {
            let cur = poly[i];
            let prev = poly[(i + len - 1) % len];
            match (inside(prev), inside(cur)) {
                (true, true) => out.push(cur),
                (true, false) => out.push(cut(prev, cur)),
                (false, true) => {
                    out.push(cut(prev, cur));
                    out.push(cur);
                }
                (false, false) => {}
            }
        }
        poly = out;
        if poly.len() < 3 {
            return Vec::new();
        }
    }
    poly
}

/// Add `w·N²·c(m)·exp(−iπ(mₓ+m_y)(N−1)/N)` for a simple polygon, with the
/// exact Fourier integral from the divergence theorem:
///
/// ```text
///   ∬_P e^{−ik·r} d²r = (i/|k|²) Σ_edges (kₓΔy − k_yΔx)·sinc(k·Δ/2)·e^{−ik·r_mid}   (k ≠ 0)
/// ```
///
/// for a counter-clockwise polygon (the sign flips for clockwise), and the
/// area at k = 0. `Δ` is the edge vector and `r_mid` its midpoint.
fn add_polygon_spectrum(
    spec: &mut Array2<Complex64>,
    w: Complex64,
    vertices: &[(f64, f64)],
    field: &Field,
) {
    let nv = vertices.len();
    if nv < 3 || w == Complex64::new(0.0, 0.0) {
        return;
    }
    let n = field.n;
    let nf = n as f64;
    let l = field.len();
    let twice_area: f64 = (0..nv)
        .map(|i| {
            let (a, b) = (vertices[i], vertices[(i + 1) % nv]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum();
    if twice_area == 0.0 {
        return;
    }
    let orientation = twice_area.signum();
    let k_of = |k: usize| 2.0 * PI * signed_order(k, n) as f64 / l;
    // Per edge: Δx, Δy, and the 1-D phase tables e^{−i kₓ x_mid}, e^{−i k_y y_mid}.
    let edges: Vec<(f64, f64, Vec<Complex64>, Vec<Complex64>)> = (0..nv)
        .filter_map(|i| {
            let (a, b) = (vertices[i], vertices[(i + 1) % nv]);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            if dx == 0.0 && dy == 0.0 {
                return None;
            }
            let (cx, cy) = (0.5 * (a.0 + b.0), 0.5 * (a.1 + b.1));
            let px = (0..n)
                .map(|k| Complex::from_polar(1.0, -k_of(k) * cx))
                .collect();
            let py = (0..n)
                .map(|k| Complex::from_polar(1.0, -k_of(k) * cy))
                .collect();
            Some((dx, dy, px, py))
        })
        .collect();
    let scale = w * (nf * nf / (l * l));
    let pixel_phase: Vec<Complex64> = (0..n)
        .map(|k| Complex::from_polar(1.0, -PI * signed_order(k, n) as f64 * (nf - 1.0) / nf))
        .collect();
    for ky in 0..n {
        let kyw = k_of(ky);
        for kx in 0..n {
            let kxw = k_of(kx);
            let f = if kx == 0 && ky == 0 {
                Complex64::new(0.5 * twice_area.abs(), 0.0)
            } else {
                let mut acc = Complex64::new(0.0, 0.0);
                for (dx, dy, px, py) in &edges {
                    let kn = kxw * dy - kyw * dx;
                    if kn != 0.0 {
                        acc += px[kx] * py[ky] * (kn * sinc(0.5 * (kxw * dx + kyw * dy)));
                    }
                }
                Complex64::new(0.0, orientation / (kxw * kxw + kyw * kyw)) * acc
            };
            spec[[ky, kx]] += scale * f * pixel_phase[kx] * pixel_phase[ky];
        }
    }
}

fn dedup_edges(edges: &mut Vec<f64>, eps: f64) {
    edges.sort_by(|a, b| a.total_cmp(b));
    edges.dedup_by(|a, b| (*a - *b).abs() <= eps);
}

fn edge_index(edges: &[f64], v: f64, eps: f64) -> usize {
    edges.partition_point(|&e| e < v - eps)
}

/// Resolve overlapping separable features into disjoint cells with the
/// painter's rule, merged into strips of identical rows. `None` if a polygon
/// is present or the cell grid exceeds [`MAX_CELLS`].
fn compress(res: &[Resolved], bg: Complex64, field: &Field) -> Option<Vec<Term>> {
    let eps = 1e-12 * field.len();
    let mut xe = vec![-field.half, field.half];
    let mut ye = vec![-field.half, field.half];
    for r in res {
        let Shape::Separable { x, y } = &r.shape else {
            return None;
        };
        for &(a, b) in &x.intervals {
            xe.push(a);
            xe.push(b);
        }
        for &(a, b) in &y.intervals {
            ye.push(a);
            ye.push(b);
        }
    }
    dedup_edges(&mut xe, eps);
    dedup_edges(&mut ye, eps);
    let (nx, ny) = (xe.len() - 1, ye.len() - 1);
    if nx == 0 || ny == 0 || nx.saturating_mul(ny) > MAX_CELLS {
        return None;
    }
    let mut cells = Array2::from_elem((ny, nx), bg);
    for r in res {
        let Shape::Separable { x, y } = &r.shape else {
            return None;
        };
        for &(ya, yb) in &y.intervals {
            let (i0, i1) = (edge_index(&ye, ya, eps), edge_index(&ye, yb, eps));
            for &(xa, xb) in &x.intervals {
                let (j0, j1) = (edge_index(&xe, xa, eps), edge_index(&xe, xb, eps));
                for i in i0..i1.min(ny) {
                    for j in j0..j1.min(nx) {
                        cells[[i, j]] = r.value;
                    }
                }
            }
        }
    }
    let mut terms = Vec::new();
    let mut i = 0;
    while i < ny {
        let mut k = i + 1;
        while k < ny && cells.row(k) == cells.row(i) {
            k += 1;
        }
        let row = cells.row(i);
        let mut xs = Vec::new();
        let mut j = 0;
        while j < nx {
            let v = row[j];
            let mut e = j + 1;
            while e < nx && row[e] == v {
                e += 1;
            }
            if v != bg {
                xs.push((v, Axis::finite(vec![(xe[j], xe[e])])));
            }
            j = e;
        }
        if !xs.is_empty() {
            terms.push(Term {
                y: Axis::finite(vec![(ye[i], ye[k])]),
                xs,
            });
        }
        i = k;
    }
    Some(terms)
}

/// Coefficients `N·X(m)·exp(−iπm(N−1)/N)` of one axis support for every FFT
/// index, with `X(m) = (1/L)∫ χ(x)·exp(−2πimx/L) dx`.
fn axis_coefficients(axis: &Axis, field: &Field) -> Vec<Complex64> {
    let n = field.n;
    let nf = n as f64;
    let l = field.len();
    let mut out = vec![Complex64::new(0.0, 0.0); n];
    match axis.kind {
        AxisKind::Full => out[0] = Complex64::new(nf, 0.0),
        AxisKind::Grating {
            width,
            pitch,
            offset,
            periods,
        } => {
            for (k, o) in out.iter_mut().enumerate() {
                let m = signed_order(k, n);
                if m % periods != 0 {
                    continue;
                }
                let q = (m / periods) as f64;
                let amp = width / pitch * sinc(PI * q * width / pitch);
                let phase = -2.0 * PI * q * offset / pitch - PI * m as f64 * (nf - 1.0) / nf;
                *o = Complex::from_polar(nf * amp, phase);
            }
        }
        AxisKind::Finite => {
            for (k, o) in out.iter_mut().enumerate() {
                let m = signed_order(k, n) as f64;
                let mut acc = Complex64::new(0.0, 0.0);
                for &(a, b) in &axis.intervals {
                    let w = b - a;
                    let amp = w / l * sinc(PI * m * w / l);
                    acc += Complex::from_polar(amp, -PI * m * (a + b) / l);
                }
                *o = acc * Complex::from_polar(nf, -PI * m * (nf - 1.0) / nf);
            }
        }
    }
    out
}

fn add_term_spectrum(spec: &mut Array2<Complex64>, term: &Term, bg: Complex64, field: &Field) {
    let n = field.n;
    let zero = Complex64::new(0.0, 0.0);
    let mut xc = vec![zero; n];
    for (value, axis) in &term.xs {
        let w = *value - bg;
        if w == zero {
            continue;
        }
        for (acc, c) in xc.iter_mut().zip(axis_coefficients(axis, field)) {
            *acc += w * c;
        }
    }
    let yc = axis_coefficients(&term.y, field);
    let xnz: Vec<usize> = (0..n).filter(|&k| xc[k] != zero).collect();
    for (i, &yv) in yc.iter().enumerate() {
        if yv == zero {
            continue;
        }
        for &j in &xnz {
            spec[[i, j]] += yv * xc[j];
        }
    }
}

/// Add `weight · overlap/p` to every pixel an interval `[a, b]` touches.
fn add_interval_coverage(cov: &mut [f64], a: f64, b: f64, field: &Field, weight: f64) {
    let n = field.n;
    let (p, x0) = (field.pixel, -field.half);
    let lo = a.max(x0);
    let hi = b.min(field.half);
    if hi <= lo || n == 0 {
        return;
    }
    let ja = (((lo - x0) / p).floor().max(0.0) as usize).min(n - 1);
    let jb = ((((hi - x0) / p).ceil() - 1.0).max(0.0) as usize).min(n - 1);
    for (j, c) in cov.iter_mut().enumerate().take(jb + 1).skip(ja) {
        let left = x0 + j as f64 * p;
        let right = x0 + (j + 1) as f64 * p;
        if lo <= left && hi >= right {
            *c += weight;
        } else {
            let o = hi.min(right) - lo.max(left);
            if o > 0.0 {
                *c += weight * o / p;
            }
        }
    }
}

/// Per-pixel covered fraction of an axis support.
fn axis_coverage(axis: &Axis, field: &Field) -> Vec<f64> {
    let mut cov = vec![0.0; field.n];
    for &(a, b) in &axis.intervals {
        add_interval_coverage(&mut cov, a, b, field, 1.0);
    }
    cov
}

fn add_term_raster(
    out: &mut Array2<Complex64>,
    term: &Term,
    bg: Complex64,
    field: &Field,
    map: impl Fn(Complex64) -> Complex64,
) {
    let n = field.n;
    let zero = Complex64::new(0.0, 0.0);
    let mbg = map(bg);
    let mut row = vec![zero; n];
    for (value, axis) in &term.xs {
        let w = map(*value) - mbg;
        if w == zero {
            continue;
        }
        for (r, c) in row.iter_mut().zip(axis_coverage(axis, field)) {
            if c != 0.0 {
                *r += w * c;
            }
        }
    }
    let cols: Vec<usize> = (0..n).filter(|&j| row[j] != zero).collect();
    for (i, cy) in axis_coverage(&term.y, field).into_iter().enumerate() {
        if cy == 0.0 {
            continue;
        }
        for &j in &cols {
            out[[i, j]] += row[j] * cy;
        }
    }
}

/// x positions where the polygon boundary crosses the horizontal line `y`
/// (half-open rule, so vertices are counted once), sorted.
fn polygon_crossings(vertices: &[(f64, f64)], y: f64, xs: &mut Vec<f64>) {
    xs.clear();
    let n = vertices.len();
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = vertices[i];
        let (xj, yj) = vertices[j];
        if (yi > y) != (yj > y) {
            xs.push(xi + (y - yi) * (xj - xi) / (yj - yi));
        }
        j = i;
    }
    xs.sort_by(|a, b| a.total_cmp(b));
}

/// Add `weight · coverage` of an even-odd polygon, with the covered area of
/// every pixel computed exactly.
///
/// The polygon is cut into horizontal bands at every vertex, every crossing
/// of two edges and every pixel-row boundary. Inside a band the active edges
/// keep their x order, so they pair up (even-odd) into strips bounded by two
/// straight edges. Each band is split again where a strip edge crosses a
/// pixel-column boundary, so that every column's overlap with the strip is
/// linear in y and the trapezoid rule integrates it exactly.
fn add_polygon_raster(
    out: &mut Array2<Complex64>,
    weight: Complex64,
    vertices: &[(f64, f64)],
    field: &Field,
) {
    let n = field.n;
    let (p, y0) = (field.pixel, -field.half);
    // Non-horizontal edges as (x at lower end, y low, x at upper end, y high).
    let nv = vertices.len();
    let mut edges: Vec<(f64, f64, f64, f64)> = Vec::with_capacity(nv);
    for i in 0..nv {
        let (xa, ya) = vertices[i];
        let (xb, yb) = vertices[(i + 1) % nv];
        if ya < yb {
            edges.push((xa, ya, xb, yb));
        } else if yb < ya {
            edges.push((xb, yb, xa, ya));
        }
    }
    let x_at = |e: &(f64, f64, f64, f64), y: f64| e.0 + (y - e.1) * (e.2 - e.0) / (e.3 - e.1);
    let (mut ymin, mut ymax) = (f64::INFINITY, f64::NEG_INFINITY);
    for &(_, vy) in vertices {
        ymin = ymin.min(vy);
        ymax = ymax.max(vy);
    }
    ymin = ymin.max(y0);
    ymax = ymax.min(field.half);
    if ymax <= ymin || edges.is_empty() {
        return;
    }
    // Band boundaries: vertices, edge crossings, pixel rows.
    let mut ys: Vec<f64> = vertices.iter().map(|v| v.1).collect();
    for a in 0..edges.len() {
        for b in 0..a {
            let (ea, eb) = (&edges[a], &edges[b]);
            let (lo, hi) = (ea.1.max(eb.1), ea.3.min(eb.3));
            if hi <= lo {
                continue;
            }
            let (da, db) = (x_at(ea, lo) - x_at(eb, lo), x_at(ea, hi) - x_at(eb, hi));
            if da * db < 0.0 {
                ys.push(lo + (hi - lo) * da / (da - db));
            }
        }
    }
    let i_lo = (((ymin - y0) / p).floor().max(0.0) as usize).min(n - 1);
    let i_hi = ((((ymax - y0) / p).ceil() - 1.0).max(0.0) as usize).min(n - 1);
    ys.extend((i_lo..=i_hi + 1).map(|i| y0 + i as f64 * p));
    ys.retain(|y| *y >= ymin && *y <= ymax);
    ys.push(ymin);
    ys.push(ymax);
    dedup_edges(&mut ys, 1e-12 * field.len());

    let mut row = vec![0.0; n];
    let mut row_index = usize::MAX;
    let mut active: Vec<(f64, usize)> = Vec::new();
    let mut breaks: Vec<f64> = Vec::new();
    let flush = |row: &mut Vec<f64>, i: usize, out: &mut Array2<Complex64>| {
        if i < n {
            for (j, c) in row.iter_mut().enumerate() {
                if *c != 0.0 {
                    out[[i, j]] += weight * *c;
                    *c = 0.0;
                }
            }
        }
    };
    for band in ys.windows(2) {
        let (b0, b1) = (band[0], band[1]);
        if b1 - b0 <= 0.0 {
            continue;
        }
        let ym = 0.5 * (b0 + b1);
        let i = (((ym - y0) / p).floor().max(0.0) as usize).min(n - 1);
        if i != row_index {
            flush(&mut row, row_index, out);
            row_index = i;
        }
        active.clear();
        active.extend(
            edges
                .iter()
                .enumerate()
                .filter(|(_, e)| e.1 < ym && e.3 > ym)
                .map(|(k, e)| (x_at(e, ym), k)),
        );
        active.sort_by(|a, b| a.0.total_cmp(&b.0));
        for pair in active.chunks_exact(2) {
            let (el, er) = (&edges[pair[0].1], &edges[pair[1].1]);
            breaks.clear();
            breaks.push(b0);
            breaks.push(b1);
            for e in [el, er] {
                let (xa, xb) = (x_at(e, b0), x_at(e, b1));
                let (lo, hi) = (xa.min(xb), xa.max(xb));
                let x0 = -field.half;
                let mut c = ((lo - x0) / p).floor() + 1.0;
                while x0 + c * p < hi {
                    let xc = x0 + c * p;
                    breaks.push(b0 + (b1 - b0) * (xc - xa) / (xb - xa));
                    c += 1.0;
                }
            }
            breaks.sort_by(|a, b| a.total_cmp(b));
            for sub in breaks.windows(2) {
                let (ya, yb) = (sub[0], sub[1]);
                let h = yb - ya;
                if h <= 0.0 {
                    continue;
                }
                let w = h / (2.0 * p);
                add_interval_coverage(&mut row, x_at(el, ya), x_at(er, ya), field, w);
                add_interval_coverage(&mut row, x_at(el, yb), x_at(er, yb), field, w);
            }
        }
    }
    flush(&mut row, row_index, out);
}

fn raster_from_plan(
    plan: &Plan,
    field: &Field,
    map: impl Fn(Complex64) -> Complex64 + Copy,
) -> Array2<Complex64> {
    if plan.painter {
        return painter_raster(&plan.resolved, plan.bg, field, map);
    }
    let n = field.n;
    let mut out = Array2::from_elem((n, n), map(plan.bg));
    for term in &plan.terms {
        add_term_raster(&mut out, term, plan.bg, field, map);
    }
    for poly in &plan.polygons {
        add_polygon_raster(
            &mut out,
            map(poly.value) - map(plan.bg),
            &poly.vertices,
            field,
        );
    }
    out
}

/// Painter's-rule rasterization on a `s × s` supersampled grid (the fallback
/// for overlaps that cannot be resolved into cells).
fn painter_raster(
    res: &[Resolved],
    bg: Complex64,
    field: &Field,
    map: impl Fn(Complex64) -> Complex64,
) -> Array2<Complex64> {
    let s = PAINTER_SUPERSAMPLE;
    let n = field.n;
    let sub = field.pixel / s as f64;
    let mbg = map(bg);
    let values: Vec<Complex64> = res.iter().map(|r| map(r.value)).collect();
    let mut out = Array2::from_elem((n, n), mbg);
    let mut buf = vec![mbg; n * s];
    let mut acc = vec![Complex64::new(0.0, 0.0); n];
    let mut xs = Vec::new();
    let norm = 1.0 / (s * s) as f64;
    let paint = |buf: &mut [Complex64], a: f64, b: f64, v: Complex64| {
        let lo = ((a + field.half) / sub - 0.5).ceil().max(0.0) as usize;
        let hi = (((b + field.half) / sub - 0.5).ceil().max(0.0) as usize).min(buf.len());
        if lo < hi {
            buf[lo..hi].iter_mut().for_each(|c| *c = v);
        }
    };
    for i in 0..n {
        acc.iter_mut().for_each(|c| *c = Complex64::new(0.0, 0.0));
        for u in 0..s {
            let y = -field.half + (i as f64 + (u as f64 + 0.5) / s as f64) * field.pixel;
            buf.iter_mut().for_each(|c| *c = mbg);
            for (r, &v) in res.iter().zip(&values) {
                if y < r.bbox[2] || y >= r.bbox[3] {
                    continue;
                }
                match &r.shape {
                    Shape::Separable { x, y: ya } => {
                        if !ya.intervals.iter().any(|&(a, b)| a <= y && y < b) {
                            continue;
                        }
                        for &(a, b) in &x.intervals {
                            paint(&mut buf, a, b, v);
                        }
                    }
                    Shape::Polygon(vertices) => {
                        polygon_crossings(vertices, y, &mut xs);
                        for pair in xs.chunks_exact(2) {
                            paint(&mut buf, pair[0], pair[1], v);
                        }
                    }
                }
            }
            for (j, a) in acc.iter_mut().enumerate() {
                *a += buf[j * s..(j + 1) * s].iter().sum::<Complex64>();
            }
        }
        for (j, a) in acc.iter().enumerate() {
            out[[i, j]] = *a * norm;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn grid(size: usize, pixel_nm: f64) -> GridConfig {
        GridConfig::new(size, pixel_nm).unwrap()
    }

    fn rect_mask(features: Vec<MaskFeature>, dark_field: bool) -> Mask {
        Mask {
            mask_type: MaskType::Binary,
            features,
            dark_field,
        }
    }

    fn rect(x: f64, y: f64, w: f64, h: f64) -> MaskFeature {
        MaskFeature::Rect { x, y, w, h }
    }

    /// Largest |a − b| over all entries, relative to max |b|.
    fn max_rel_diff(a: &Array2<Complex64>, b: &Array2<Complex64>) -> f64 {
        let scale = b.iter().map(|v| v.norm()).fold(0.0, f64::max);
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).norm())
            .fold(0.0, f64::max)
            / scale
    }

    #[test]
    fn test_line_space_creates_features() {
        let mask = Mask::line_space(65.0, 180.0).unwrap();
        assert_eq!(
            mask.features,
            vec![MaskFeature::LineSpace {
                cd: 65.0,
                pitch: 180.0,
                orientation: LineOrientation::Vertical,
                offset: 0.0
            }]
        );
        assert!(!mask.dark_field);
    }

    #[test]
    fn test_rasterize_has_correct_size() {
        let mask = Mask::line_space(65.0, 180.0).unwrap();
        let raster = mask.rasterize(&GridConfig::default());
        assert_eq!(raster.dim(), (512, 512));
    }

    #[test]
    fn test_binary_mask_values_with_pixel_aligned_edges() {
        // 100 nm lines at x = 0, ±200 on 2 nm pixels: every edge lies on a
        // pixel boundary, so area coverage is exactly 0 or 1.
        let mask = Mask::line_space(100.0, 200.0).unwrap();
        let raster = mask.rasterize(&grid(256, 2.0));
        for &v in raster.iter() {
            let norm = v.norm();
            assert!(
                norm.abs() < 1e-12 || (norm - 1.0).abs() < 1e-12,
                "pixel-aligned binary mask must be 0 or 1, got {norm}"
            );
        }
    }

    #[test]
    fn test_attenuated_psm_transmittance() {
        let mask = Mask {
            mask_type: MaskType::AttenuatedPSM {
                transmission: 0.06,
                phase_deg: 180.0,
            },
            features: vec![rect(0.0, 0.0, 100.0, 100.0)],
            dark_field: false,
        };
        let raster = mask.rasterize(&grid(64, 4.0));
        // Center pixel is absorber: sqrt(0.06) amplitude with 180 degree phase.
        let center = raster[[32, 32]];
        assert_relative_eq!(center.norm(), 0.06_f64.sqrt(), epsilon = 1e-12);
        assert_relative_eq!(center.re, -(0.06_f64.sqrt()), epsilon = 1e-12);
    }

    #[test]
    fn test_gray_rect_rasterizes_to_sqrt_transmittance() {
        let gray = |t: f64| {
            rect_mask(
                vec![MaskFeature::GrayRect {
                    x: 0.0,
                    y: 0.0,
                    w: 100.0,
                    h: 100.0,
                    transmittance: t,
                }],
                false,
            )
        };
        let g = grid(64, 4.0);
        // Amplitude sqrt(0.25) = 0.5, zero phase.
        let center = gray(0.25).rasterize(&g)[[32, 32]];
        assert!((center.re - 0.5).abs() < 1e-12);
        assert!(center.im.abs() < 1e-12);
        // Transmittance clamps into [0, 1].
        assert!((gray(1.7).rasterize(&g)[[32, 32]].re - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_contact_hole_is_one_dark_field_array() {
        let mask = Mask::contact_hole(80.0, 200.0, 200.0).unwrap();
        assert!(mask.dark_field);
        assert_eq!(
            mask.features,
            vec![MaskFeature::RectArray {
                w: 80.0,
                h: 80.0,
                pitch_x: 200.0,
                pitch_y: 200.0,
                offset_x: 0.0,
                offset_y: 0.0
            }]
        );
        // Holes at the origin and one pitch away; absorber between them.
        let g = grid(64, 400.0 / 64.0);
        let r = mask.rasterize(&g);
        let px = |x: f64| ((x + 200.0) / g.pixel_nm) as usize;
        assert_relative_eq!(r[[px(0.1), px(0.1)]].re, 1.0, epsilon = 1e-12);
        assert_relative_eq!(r[[px(0.1), px(-199.0)]].re, 1.0, epsilon = 1e-12);
        assert_relative_eq!(r[[px(100.1), px(0.1)]].re, 0.0, epsilon = 1e-12);
        // Clear area fraction = (80/200)^2.
        let mean: f64 = r.iter().map(|v| v.re).sum::<f64>() / (64.0 * 64.0);
        assert_relative_eq!(mean, 0.16, epsilon = 1e-12);
    }

    #[test]
    fn test_contact_hole_rejects_merging_holes() {
        assert!(Mask::contact_hole(150.0, 150.0, 200.0).is_err());
        assert!(Mask::contact_hole(50.0, 150.0, 150.0).is_ok());
    }

    #[test]
    fn test_invalid_cd_exceeds_pitch() {
        // CD >= pitch should return Err
        assert!(Mask::line_space(100.0, 50.0).is_err());
        assert!(Mask::line_space(100.0, 100.0).is_err());
    }

    #[test]
    fn test_invalid_negative_cd() {
        assert!(Mask::line_space(-10.0, 100.0).is_err());
        assert!(Mask::line_space(0.0, 100.0).is_err());
        assert!(Mask::line_space(f64::NAN, 100.0).is_err());
        assert!(Mask::line_space(10.0, f64::INFINITY).is_err());
    }

    #[test]
    fn test_contact_hole_valid() {
        let mask = Mask::contact_hole(50.0, 150.0, 150.0).unwrap();
        assert!(mask.dark_field);
        assert!(!mask.features.is_empty());
    }

    #[test]
    fn test_contact_hole_invalid_diameter() {
        // Diameter <= 0 should return Err
        assert!(Mask::contact_hole(0.0, 150.0, 150.0).is_err());
        assert!(Mask::contact_hole(-10.0, 150.0, 150.0).is_err());
        // Invalid pitch
        assert!(Mask::contact_hole(50.0, 0.0, 150.0).is_err());
        assert!(Mask::contact_hole(50.0, 150.0, -1.0).is_err());
    }

    // ---- exact spectrum -------------------------------------------------

    #[test]
    fn test_spectrum_clear_mask_dc_is_n_squared() {
        let clear = rect_mask(vec![], false);
        let g = grid(32, 3.0);
        let s = clear.spectrum(&g, &Fft2D::new());
        assert_eq!(s[[0, 0]], Complex64::new(1024.0, 0.0));
        assert!(s.iter().skip(1).all(|v| *v == Complex64::new(0.0, 0.0)));
        assert_eq!(clear.spectrum_method(&g), SpectrumMethod::Analytic);
    }

    #[test]
    fn test_spectrum_rect_matches_quadrature_fixture() {
        // Dark-field rectangle with sub-pixel edges on N = 64, p = 4 nm.
        // Fixture: brute-force midpoint quadrature (2·10⁶ points per axis) of
        // N²·c(m)·exp(−iπ(mₓ+m_y)(N−1)/N) in numpy, independent of the
        // closed form used here.
        let mask = rect_mask(vec![rect(3.3, -5.2, 41.7, 60.1)], true);
        let s = mask.spectrum(&grid(64, 4.0), &Fft2D::new());
        let fixtures = [
            ((0, 0), (1.566356250000e2, 0.0)),
            ((0, 1), (-1.498118343911e2, 4.781639045323e0)),
            ((1, 0), (-1.405921075627e2, -2.510656487625e1)),
            ((2, 3), (-6.653606958088e1, -1.753688923051e1)),
            ((63, 5), (2.900396579588e1, -1.013751332586e1)),
            ((60, 61), (-5.385969788449e0, 3.773485996544e0)),
            ((5, 62), (1.074513265635e1, 1.494311273936e1)),
        ];
        for ((ky, kx), (re, im)) in fixtures {
            let v = s[[ky, kx]];
            assert!(
                (v - Complex64::new(re, im)).norm() < 1e-9 * 156.6,
                "S[{ky},{kx}] = {v}, expected {re} + {im}i"
            );
        }
    }

    #[test]
    fn test_line_space_spectrum_only_pitch_harmonics() {
        // Bright-field 65/180 L/S on a field of exactly 2 periods (360 nm):
        // c(q) = δ(q) − (cd/P)·sinc(πq·cd/P) at m = 2q, zero elsewhere.
        // Fixture values from numpy (closed form, cross-checked by quadrature).
        let mask = Mask::line_space(65.0, 180.0).unwrap();
        let g = grid(64, 360.0 / 64.0);
        let s = mask.spectrum(&g, &Fft2D::new());
        assert_eq!(mask.spectrum_method(&g), SpectrumMethod::Analytic);
        let expect = [
            (0, (2.616888888889e3, 0.0)),
            (2, (-1.175951712624e3, -1.158211344566e2)),
            (4, (-4.897878251666e2, -9.742485581761e1)),
            (62, (-1.175951712624e3, 1.158211344566e2)),
            (60, (-4.897878251666e2, 9.742485581761e1)),
        ];
        for (kx, (re, im)) in expect {
            assert!((s[[0, kx]] - Complex64::new(re, im)).norm() < 1e-8 * 2616.9);
        }
        for ((ky, kx), v) in s.indexed_iter() {
            if ky != 0 || kx % 2 == 1 {
                assert_eq!(*v, Complex64::new(0.0, 0.0), "S[{ky},{kx}] must vanish");
            }
        }
    }

    #[test]
    fn test_spectrum_equals_fft_of_raster_in_fine_pixel_limit() {
        // Same 256 nm field sampled at 4 nm and at 1 nm pixels: the FFT of the
        // antialiased raster converges to the exact spectrum at low orders.
        let mask = rect_mask(vec![rect(3.3, -5.2, 41.7, 60.1)], true);
        let fft = Fft2D::new();
        let low_order_error = |n: usize| {
            let g = grid(n, 256.0 / n as f64);
            let exact = mask.spectrum(&g, &fft);
            let mut raster = mask.rasterize(&g);
            fft.forward(&mut raster);
            let mut worst: f64 = 0.0;
            for ky in [0, 1, 2, 3, n - 1, n - 2] {
                for kx in [0, 1, 2, 3, n - 1, n - 2] {
                    worst = worst.max((raster[[ky, kx]] - exact[[ky, kx]]).norm());
                }
            }
            worst / exact[[0, 0]].norm()
        };
        // Sub-pixel edges leave a first-order error (the first moment of the
        // mask inside each edge pixel), so 4× finer pixels give ~4× less.
        let coarse = low_order_error(64);
        let fine = low_order_error(256);
        assert!(fine < 5e-4, "fine-pixel mismatch {fine}");
        assert!(
            fine < coarse / 3.0,
            "error must shrink with the pixel: {coarse} -> {fine}"
        );
    }

    #[test]
    fn test_pixel_aligned_raster_times_pixel_transfer_is_exact() {
        // Edges on pixel boundaries: FFT(raster)·sinc·sinc reproduces the
        // analytic spectrum at every order, Nyquist included.
        let mask = rect_mask(vec![rect(4.0, -8.0, 40.0, 24.0)], true);
        let g = grid(64, 4.0);
        let fft = Fft2D::new();
        let exact = mask.spectrum(&g, &fft);
        let mut raster = mask.rasterize(&g);
        fft.forward(&mut raster);
        apply_pixel_transfer(&mut raster);
        assert!(max_rel_diff(&raster, &exact) < 1e-12);
    }

    #[test]
    fn test_centred_feature_images_centred() {
        // Low-pass the exact spectrum of a centred rectangle with a Gaussian
        // pupil and transform back: the field is real, mirror-symmetric about
        // x = 0 and y = 0 on the Grid2D pixel centres, and peaks there.
        let mask = rect_mask(vec![rect(0.0, 0.0, 37.3, 51.9)], true);
        let n = 64;
        let g = grid(n, 4.0);
        let fft = Fft2D::new();
        let mut s = mask.spectrum(&g, &fft);
        for ((ky, kx), v) in s.indexed_iter_mut() {
            let (my, mx) = (signed_order(ky, n) as f64, signed_order(kx, n) as f64);
            *v *= (-(mx * mx + my * my) / 36.0).exp();
        }
        fft.inverse(&mut s);
        for i in 0..n {
            for j in 0..n {
                let v = s[[i, j]];
                assert!(v.im.abs() < 1e-12, "field must be real, got {v}");
                assert!((v - s[[i, n - 1 - j]]).norm() < 1e-12);
                assert!((v - s[[n - 1 - i, j]]).norm() < 1e-12);
            }
        }
        // Peak at the four central pixels (x, y = ±p/2).
        let peak = s.iter().map(|v| v.re).fold(f64::NEG_INFINITY, f64::max);
        assert_relative_eq!(s[[n / 2, n / 2]].re, peak, epsilon = 1e-12);
    }

    #[test]
    fn test_subpixel_shift_is_an_exact_phase_ramp() {
        // Moving a feature by δ = 0.37 nm (a tenth of a pixel) multiplies
        // S by exp(−2πi·mₓ·δ/L) — sub-pixel geometry is honoured exactly.
        let n = 64;
        let g = grid(n, 4.0);
        let fft = Fft2D::new();
        let s0 = rect_mask(vec![rect(0.0, 0.0, 30.0, 30.0)], true).spectrum(&g, &fft);
        let s1 = rect_mask(vec![rect(0.37, 0.0, 30.0, 30.0)], true).spectrum(&g, &fft);
        for ((ky, kx), v) in s1.indexed_iter() {
            let mx = signed_order(kx, n) as f64;
            let ramp = Complex::from_polar(1.0, -2.0 * PI * mx * 0.37 / 256.0);
            assert!((v - s0[[ky, kx]] * ramp).norm() < 1e-9, "order ({ky},{kx})");
        }
    }

    // ---- periodic primitives and commensurability ------------------------

    #[test]
    fn test_line_space_is_opaque_lines_of_width_cd() {
        // Defect #2 of the old model: the dark region had width pitch − cd.
        let mask = Mask::line_space(65.0, 180.0).unwrap();
        let g = mask.commensurate_grid(256, 1.0).unwrap();
        let r = mask.rasterize(&g);
        let mean: f64 = r.iter().map(|v| v.re).sum::<f64>() / (256.0 * 256.0);
        assert_relative_eq!(mean, 1.0 - 65.0 / 180.0, epsilon = 1e-12);
        // The field is one 180 nm period, [−90, 90): the line is centred at
        // x = 0 (central columns opaque) and the space wraps around the edge.
        assert_relative_eq!(g.field_size_nm(), 180.0, epsilon = 1e-9);
        assert!(r[[0, 127]].norm() < 1e-12 && r[[0, 128]].norm() < 1e-12);
        assert_relative_eq!(r[[0, 0]].re, 1.0, epsilon = 1e-12);
        assert_relative_eq!(r[[0, 255]].re, 1.0, epsilon = 1e-12);
        // Rows are identical (lines run along y).
        assert_eq!(r.row(0), r.row(200));
    }

    #[test]
    fn test_incommensurate_pitches_no_longer_alias() {
        // Defect #1: the old 10-period construction gave bit-identical images
        // for (90, 190) and (100, 200) on the 256 nm field. The rasters (and
        // spectra) now differ, and each is exactly the grating cut to the field.
        let g = grid(256, 1.0);
        let a = Mask::line_space(90.0, 190.0).unwrap().rasterize(&g);
        let b = Mask::line_space(100.0, 200.0).unwrap().rasterize(&g);
        let diff: f64 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).norm()).sum();
        assert!(diff > 10.0, "rasters must differ, diff {diff}");
        // Mean transmission of (100, 200) over [−128, 128): lines at 0 and
        // ±200 → dark length 100 + 2·(128 − 150 < 0 → 0) = 100 nm → 156/256.
        let mean_b: f64 = b.iter().map(|v| v.re).sum::<f64>() / (256.0 * 256.0);
        assert_relative_eq!(mean_b, 156.0 / 256.0, epsilon = 1e-12);
    }

    #[test]
    fn test_line_space_orientation_and_offset() {
        let g = grid(64, 180.0 / 64.0);
        let v = Mask::line_space(65.0, 180.0).unwrap().rasterize(&g);
        let h = Mask::line_space_with(65.0, 180.0, LineOrientation::Horizontal, 0.0)
            .unwrap()
            .rasterize(&g);
        assert_eq!(h, v.t());
        // Offsetting by half a pitch swaps line and space at the centre.
        let o = Mask::line_space_with(65.0, 180.0, LineOrientation::Vertical, 90.0)
            .unwrap()
            .rasterize(&g);
        assert!(v[[0, 32]].norm() < 1e-12);
        assert_relative_eq!(o[[0, 32]].re, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_rect_array_spectrum_harmonics_rectangular_pitch() {
        // 150 × 200 nm lattice: common cell 600 nm = 4·150 = 3·200.
        let mask = Mask::contact_hole(60.0, 150.0, 200.0).unwrap();
        assert_eq!(mask.periodicity(), Some((150.0, Some(200.0))));
        let g = mask.commensurate_grid(64, 10.0).unwrap();
        assert_relative_eq!(g.field_size_nm(), 600.0, epsilon = 1e-9);
        let s = mask.spectrum(&g, &Fft2D::new());
        let n = 64;
        // DC: clear fraction (60/150)(60/200) = 0.12.
        assert_relative_eq!(s[[0, 0]].re, 0.12 * 4096.0, epsilon = 1e-9);
        for ((ky, kx), v) in s.indexed_iter() {
            let (my, mx) = (signed_order(ky, n), signed_order(kx, n));
            if mx % 4 != 0 || my % 3 != 0 {
                assert_eq!(*v, Complex64::new(0.0, 0.0));
            }
        }
        // First x harmonic: N²·(w/Px)sinc(πw/Px)·(h/Py)·phase.
        let expect = 4096.0 * 0.4 * sinc(PI * 0.4) * 0.3;
        assert_relative_eq!(s[[0, 4]].norm(), expect, epsilon = 1e-9);
    }

    #[test]
    fn test_periodicity_and_check_commensurate() {
        let ls = Mask::line_space(65.0, 180.0).unwrap();
        assert_eq!(ls.periodicity(), Some((180.0, None)));
        assert_eq!(
            Mask::contact_hole(50.0, 150.0, 150.0)
                .unwrap()
                .periodicity(),
            Some((150.0, None))
        );
        assert_eq!(
            rect_mask(vec![rect(0.0, 0.0, 10.0, 10.0)], true).periodicity(),
            None
        );

        let bad = grid(256, 1.0);
        let err = ls.check_commensurate(&bad).unwrap_err().to_string();
        assert!(err.contains("1.422222 periods"), "{err}");
        assert!(
            err.contains("GridConfig::commensurate(180, None, 256, 1)"),
            "{err}"
        );
        assert!(err.contains("0.703125"), "{err}");
        assert!(ls.check_commensurate(&grid(256, 180.0 / 256.0)).is_ok());
        // Non-periodic masks are always commensurate.
        assert!(rect_mask(vec![], false).check_commensurate(&bad).is_ok());
    }

    #[test]
    fn test_grid_commensurate_values() {
        // One 180 nm period fits 256 px at ≤ 1 nm: pixel 180/256.
        let g = GridConfig::commensurate(180.0, None, 256, 1.0).unwrap();
        assert_eq!(g.size, 256);
        assert_relative_eq!(g.pixel_nm, 0.703125, epsilon = 1e-15);
        assert!(g.is_commensurate_with(180.0));
        // 100 nm: ⌊2.56⌋ = 2 periods, pixel 200/256 (never coarser than target).
        let g = GridConfig::commensurate(100.0, None, 256, 1.0).unwrap();
        assert_relative_eq!(g.pixel_nm, 0.78125, epsilon = 1e-15);
        // 150 × 200: cell 600 nm.
        let g = GridConfig::commensurate(150.0, Some(200.0), 1024, 1.0).unwrap();
        assert_relative_eq!(g.field_size_nm(), 600.0, epsilon = 1e-9);
        assert!(g.is_commensurate_with(150.0) && g.is_commensurate_with(200.0));
        // A cell larger than size·target coarsens the pixel to hold one cell.
        let g = GridConfig::commensurate(400.0, None, 128, 1.0).unwrap();
        assert_relative_eq!(g.pixel_nm, 3.125, epsilon = 1e-15);
        // Errors.
        assert!(GridConfig::commensurate(180.0, None, 100, 1.0).is_err());
        assert!(GridConfig::commensurate(0.0, None, 256, 1.0).is_err());
        assert!(GridConfig::commensurate(180.0, None, 256, f64::NAN).is_err());
        assert!(GridConfig::commensurate(100.0, Some(100.0 * 2f64.sqrt()), 256, 1.0).is_err());
    }

    #[test]
    fn test_common_period() {
        use crate::types::common_period_nm;
        assert_relative_eq!(
            common_period_nm(150.0, 200.0).unwrap(),
            600.0,
            epsilon = 1e-9
        );
        assert_relative_eq!(
            common_period_nm(180.0, 180.0).unwrap(),
            180.0,
            epsilon = 1e-9
        );
        assert_relative_eq!(
            common_period_nm(100.0 / 3.0, 50.0).unwrap(),
            100.0,
            epsilon = 1e-9
        );
        assert!(common_period_nm(1.0, std::f64::consts::E).is_none());
    }

    // ---- overlaps, polygons, rasterization -------------------------------

    #[test]
    fn test_overlapping_rects_resolved_exactly() {
        // Two overlapping clear rectangles (e.g. a line and an OPC serif):
        // the union is imaged, not the sum. Disjoint decomposition by hand:
        // A = [−20, 20]², B = [0.3, 30.3] × [−7.9, 22.1];
        // B \ A = [20, 30.3] × [−7.9, 22.1]  ∪  [0.3, 20] × [20, 22.1].
        let a = rect(0.0, 0.0, 40.0, 40.0);
        let b = rect(15.3, 7.1, 30.0, 30.0);
        let overlapping = rect_mask(vec![a.clone(), b], true);
        let disjoint = rect_mask(
            vec![
                a,
                rect(25.15, 7.1, 10.3, 30.0),
                rect(10.15, 21.05, 19.7, 2.1),
            ],
            true,
        );
        let g = grid(64, 4.0);
        let fft = Fft2D::new();
        assert_eq!(overlapping.spectrum_method(&g), SpectrumMethod::Analytic);
        let s = overlapping.spectrum(&g, &fft);
        assert!(max_rel_diff(&s, &disjoint.spectrum(&g, &fft)) < 1e-12);
        let union_area = 1600.0 + 10.3 * 30.0 + 19.7 * 2.1;
        assert_relative_eq!(s[[0, 0]].re, 4096.0 * union_area / 65536.0, epsilon = 1e-9);
        let r = overlapping.rasterize(&g);
        let area: f64 = r.iter().map(|v| v.re).sum::<f64>() * 16.0;
        assert_relative_eq!(area, union_area, epsilon = 1e-9);
    }

    #[test]
    fn test_painter_order_gray_over_absorber() {
        // Bright field: an opaque rect, then a T = 0.25 gray rect over half
        // of it. The later feature wins where they overlap.
        let mask = rect_mask(
            vec![
                rect(0.0, 0.0, 40.0, 40.0),
                MaskFeature::GrayRect {
                    x: 20.0,
                    y: 0.0,
                    w: 40.0,
                    h: 40.0,
                    transmittance: 0.25,
                },
            ],
            false,
        );
        let g = grid(64, 4.0);
        let r = mask.rasterize(&g);
        let at = |x: f64| r[[32, ((x + 128.0) / 4.0) as usize]].re;
        assert_relative_eq!(at(-10.0), 0.0, epsilon = 1e-12);
        assert_relative_eq!(at(10.0), 0.5, epsilon = 1e-12);
        assert_relative_eq!(at(30.0), 0.5, epsilon = 1e-12);
        assert_relative_eq!(at(60.0), 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_polygon_rectangle_matches_rect_exactly() {
        // A rectangle drawn as a polygon, in either orientation, has the same
        // exact spectrum and the same exact-coverage raster as the Rect.
        let g = grid(64, 4.0);
        let fft = Fft2D::new();
        let exact = rect_mask(vec![rect(1.4, 10.4, 39.4, 40.6)], true);
        let se = exact.spectrum(&g, &fft);
        let re = exact.rasterize(&g);
        let ccw = vec![(-18.3, -9.9), (21.1, -9.9), (21.1, 30.7), (-18.3, 30.7)];
        let cw: Vec<(f64, f64)> = ccw.iter().rev().cloned().collect();
        for vertices in [ccw, cw] {
            let poly = rect_mask(vec![MaskFeature::Polygon { vertices }], true);
            assert_eq!(poly.spectrum_method(&g), SpectrumMethod::Analytic);
            assert!(max_rel_diff(&poly.spectrum(&g, &fft), &se) < 1e-11);
            assert!(max_rel_diff(&poly.rasterize(&g), &re) < 1e-12);
        }
    }

    #[test]
    fn test_polygon_spectrum_concave_and_clipped() {
        // Concave L-shaped hexagon, partly outside the 256 nm field (clipped
        // at x = 128). References: the clipped area (DC), and the same region
        // written as two disjoint Rects, whose closed form is checked against
        // brute-force quadrature in `test_spectrum_rect_matches_quadrature_fixture`.
        // The edge-sum formula shares no code with the Rect path.
        let l_shape = vec![
            (-40.0, -30.0),
            (150.0, -30.0),
            (150.0, 10.0),
            (-10.0, 10.0),
            (-10.0, 50.0),
            (-40.0, 50.0),
        ];
        let mask = rect_mask(vec![MaskFeature::Polygon { vertices: l_shape }], true);
        let g = grid(64, 4.0);
        assert_eq!(mask.spectrum_method(&g), SpectrumMethod::Analytic);
        let s = mask.spectrum(&g, &Fft2D::new());
        let clipped_area = 168.0 * 40.0 + 30.0 * 40.0;
        assert_relative_eq!(
            s[[0, 0]].re,
            4096.0 * clipped_area / 65536.0,
            epsilon = 1e-9
        );
        // Same shape as two disjoint Rects: exact agreement at every order.
        let rects = rect_mask(
            vec![
                rect(44.0, -10.0, 168.0, 40.0),
                rect(-25.0, 30.0, 30.0, 40.0),
            ],
            true,
        );
        assert!(max_rel_diff(&s, &rects.spectrum(&g, &Fft2D::new())) < 1e-10);
    }

    #[test]
    fn test_self_intersecting_polygon_uses_raster() {
        // A bow-tie: Green's theorem would subtract its lobes, so it takes
        // the exact-coverage raster path (even-odd fill: both lobes clear).
        let bow = rect_mask(
            vec![MaskFeature::Polygon {
                vertices: vec![(-40.0, -20.0), (40.0, 20.0), (40.0, -20.0), (-40.0, 20.0)],
            }],
            true,
        );
        let g = grid(64, 4.0);
        assert_eq!(bow.spectrum_method(&g), SpectrumMethod::Mixed);
        let area: f64 = bow.rasterize(&g).iter().map(|v| v.re).sum::<f64>() * 16.0;
        assert_relative_eq!(area, 1600.0, max_relative = 1e-12);
        let s = bow.spectrum(&g, &Fft2D::new());
        assert_relative_eq!(s[[0, 0]].re, 4096.0 * 1600.0 / 65536.0, epsilon = 1e-9);
    }

    #[test]
    fn test_polygon_coverage_area_and_overlap_fallback() {
        // Right triangle with legs 60 nm: area 1800 nm².
        let tri = MaskFeature::Polygon {
            vertices: vec![(-30.0, -30.0), (30.0, -30.0), (-30.0, 30.0)],
        };
        let g = grid(64, 4.0);
        let r = rect_mask(vec![tri.clone()], true).rasterize(&g);
        let area: f64 = r.iter().map(|v| v.re).sum::<f64>() * 16.0;
        assert_relative_eq!(area, 1800.0, max_relative = 1e-12);
        // Overlapping a rectangle forces the painter's raster path.
        let both = rect_mask(vec![tri, rect(0.0, 0.0, 20.0, 20.0)], true);
        assert_eq!(both.spectrum_method(&g), SpectrumMethod::Raster);
        let area: f64 = both.rasterize(&g).iter().map(|v| v.re).sum::<f64>() * 16.0;
        // Union: triangle + the part of the square above the hypotenuse
        // (x + y > 0 inside [−10, 10]²: area 200).
        assert_relative_eq!(area, 2000.0, max_relative = 1e-2);
    }

    #[test]
    fn test_rasterize_intensity_on_edge_pixels() {
        // A 10 nm clear stripe on 4 nm pixels: edges at ±5 nm cover a quarter
        // of pixels [4, 8) and [−8, −4). The amplitude there is 0.25, so
        // |⟨t⟩|² = 0.0625, while the area-averaged intensity is 0.25.
        let mask = rect_mask(vec![rect(0.0, 0.0, 10.0, 256.0)], true);
        let g = grid(64, 4.0);
        let amp = mask.rasterize(&g);
        let inten = mask.rasterize_intensity(&g);
        assert_relative_eq!(amp[[10, 33]].re, 0.25, epsilon = 1e-12);
        assert_relative_eq!(amp[[10, 30]].re, 0.25, epsilon = 1e-12);
        assert_relative_eq!(amp[[10, 33]].norm_sqr(), 0.0625, epsilon = 1e-12);
        assert_relative_eq!(inten[[10, 33]], 0.25, epsilon = 1e-12);
        assert_relative_eq!(inten[[10, 32]], 1.0, epsilon = 1e-12);
        // Attenuated PSM absorber: intensity T = 0.06 away from edges.
        let psm = Mask {
            mask_type: MaskType::AttenuatedPSM {
                transmission: 0.06,
                phase_deg: 180.0,
            },
            ..mask
        };
        assert_relative_eq!(psm.rasterize_intensity(&g)[[10, 0]], 0.06, epsilon = 1e-12);
    }

    #[test]
    fn test_many_pixel_rects_use_exact_cells() {
        // ILT-style mask: 2048 pixel-sized rects (checkerboard) exceed the
        // pairwise overlap check and are resolved into cells, still exactly.
        let n = 64;
        let g = grid(n, 2.0);
        let mut features = Vec::new();
        for i in 0..n {
            for j in 0..n {
                if (i + j) % 2 == 0 {
                    features.push(rect(
                        -63.0 + 2.0 * j as f64,
                        -63.0 + 2.0 * i as f64,
                        2.0,
                        2.0,
                    ));
                }
            }
        }
        let mask = rect_mask(features, true);
        let r = mask.rasterize(&g);
        for ((i, j), v) in r.indexed_iter() {
            let expect = if (i + j) % 2 == 0 { 1.0 } else { 0.0 };
            assert!((v.re - expect).abs() < 1e-12 && v.im == 0.0);
        }
        let fft = Fft2D::new();
        let s = mask.spectrum(&g, &fft);
        let mut t = r.clone();
        fft.forward(&mut t);
        apply_pixel_transfer(&mut t);
        assert!(max_rel_diff(&s, &t) < 1e-9);
    }

    #[test]
    fn test_serde_back_compat_and_defaults() {
        // Pre-existing variants keep their tags and fields.
        let old = r#"
            dark_field = true
            mask_type = "Binary"
            [[features]]
            [features.Rect]
            x = 0.0
            y = 0.0
            w = 10.0
            h = 20.0
            [[features]]
            [features.GrayRect]
            x = 1.0
            y = 2.0
            w = 3.0
            h = 4.0
            transmittance = 0.5
        "#;
        let m: Mask = toml::from_str(old).unwrap();
        assert_eq!(m.features[0], rect(0.0, 0.0, 10.0, 20.0));
        // New variants: orientation/offset default when omitted.
        let new = r#"
            dark_field = false
            mask_type = "Binary"
            [[features]]
            [features.LineSpace]
            cd = 65.0
            pitch = 180.0
        "#;
        let m: Mask = toml::from_str(new).unwrap();
        assert_eq!(m.features, Mask::line_space(65.0, 180.0).unwrap().features);
        let round: Mask = toml::from_str(&toml::to_string(&m).unwrap()).unwrap();
        assert_eq!(round.features, m.features);
    }

    #[test]
    fn test_edge_bias_and_validate() {
        let ls = MaskFeature::LineSpace {
            cd: 65.0,
            pitch: 180.0,
            orientation: LineOrientation::Vertical,
            offset: 0.0,
        };
        assert_eq!(ls.element_width_nm(), Some(65.0));
        match ls.with_edge_bias(2.5) {
            MaskFeature::LineSpace { cd, pitch, .. } => {
                assert_relative_eq!(cd, 70.0, epsilon = 1e-12);
                assert_relative_eq!(pitch, 180.0, epsilon = 1e-12);
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(Mask::line_space(65.0, 180.0).unwrap().validate().is_ok());
        let bad = rect_mask(
            vec![MaskFeature::LineSpace {
                cd: 200.0,
                pitch: 180.0,
                orientation: LineOrientation::Vertical,
                offset: 0.0,
            }],
            false,
        );
        assert!(bad.validate().is_err());
        // Rasterization still succeeds: a width ≥ pitch covers everything.
        let r = bad.rasterize(&grid(32, 180.0 / 32.0));
        assert!(r.iter().all(|v| v.norm() < 1e-12));
    }

    #[test]
    fn test_engine_images_line_centred_on_origin() {
        // Smoke test through the imaging engine: a 1:1 300 nm-pitch grating
        // (resolved at NA 0.75, λ 157.6 nm) on a commensurate field images
        // with its intensity minimum on the line at x = 0, symmetric.
        use crate::aerial::AerialImageEngine;
        use crate::optics::ProjectionOptics;
        use crate::source::VuvSource;
        let mask = Mask::line_space(150.0, 300.0).unwrap();
        let g = mask.commensurate_grid(64, 5.0).unwrap();
        let engine = AerialImageEngine::new(
            &VuvSource::f2_laser(0.5).unwrap(),
            &ProjectionOptics::new(0.75).unwrap(),
            g,
            8,
        )
        .unwrap();
        let img = engine.compute(&mask, 0.0).data;
        let row: Vec<f64> = img.row(32).to_vec();
        let n = row.len();
        for j in 0..n / 2 {
            assert_relative_eq!(row[j], row[n - 1 - j], epsilon = 1e-9);
        }
        let (min_j, _) = row
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.total_cmp(b.1))
            .unwrap();
        assert!(
            min_j == n / 2 || min_j == n / 2 - 1,
            "minimum at column {min_j}"
        );
        assert!(
            row[n / 2] < row[0],
            "line centre darker than the space centre"
        );
    }
}
