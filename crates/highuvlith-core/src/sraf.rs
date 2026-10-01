//! Sub-resolution assist features (SRAF): rule-based placement, a
//! model-based print check through the imaging engine, and a depth-of-focus
//! comparison helper.
//!
//! **Placement** ([`place_srafs`]) is rule based. For lines (rectangles with
//! aspect ratio ≥ [`SrafRules::line_aspect_ratio`]) the space between a line
//! edge and the nearest facing feature — including periodic images, since
//! the imaging field is periodic — selects a [`LineSrafRule`] from a
//! space-keyed table: `bars_per_side` scattering bars of width
//! `bar_width_nm` are placed parallel to the line, the first `first_gap_nm`
//! from the main edge and the next ones `bar_gap_nm` further out, filling the
//! space inward from both of its edges; when the next pair would crowd (less
//! than `bar_gap_nm` between its two bars) a single centred bar is placed
//! instead if it keeps the gaps. For isolated
//! contacts (nearest neighbour ≥ [`ContactSrafRule::min_pitch_nm`]) a ring of
//! four bars or a set of square assists is placed around the hole. Assists
//! have the tone of the main features (opaque bars on a bright-field mask,
//! clear bars on a dark-field mask) and are dropped if they come closer than
//! [`SrafRules::min_clearance_nm`] to any other feature.
//!
//! **Print check** ([`insert_srafs`]) images the mask with its assists at
//! every (focus, dose) corner and requires each assist to stay a relative
//! `margin` away from printing: an opaque assist prints (leaves resist) where
//! `dose·I < t`, a clear assist prints where `dose·I > t`. Printing assists
//! are narrowed in `shrink_step_nm` steps down to `min_width_nm`, then
//! removed, and the check repeats until no assist prints at any corner.
//!
//! **DOF helper** ([`line_dof`], [`compare_sraf_dof`]): the CD of a line is
//! measured on a horizontal cut; the threshold is anchored so that the
//! nominal-dose, best-focus CD equals the target (each mask gets its own
//! dose-to-size); the DOF is the length of the focus interval around best
//! focus over which the CD stays within ±tolerance at every dose corner,
//! with interpolated interval ends.
//!
//! Images come from [`AerialImageEngine::compute`], whose mask spectrum is
//! exact and analytic ([`Mask::spectrum`]), so narrow assists are not
//! pixel-quantized.
//!
//! # Key equations
//!
//! ```text
//!   bar k from the left edge of a space [a, b]:  [a + g₁ + k(w + g₂),  a + g₁ + k(w + g₂) + w]
//!   (mirrored from the right edge; pair k kept while its bars are ≥ g₂ apart,
//!    else one centred bar if the remaining gap is ≥ 2·gap + w)
//!   opaque assist safe  ⇔  min_assist dose·I ≥ t (1 + margin)   at every corner
//!   clear assist safe   ⇔  max_assist dose·I ≤ t (1 − margin)   at every corner
//!   dose-to-size        t* : CD(z = 0, t*) = CD_target
//!   DOF                 = |{ z : |CD(z, t*/d) − CD_target| ≤ tol·CD_target ∀ d ∈ doses }|
//! ```
//!
//! # Model status
//!
//! - Placement tables are user inputs; [`SrafRules::for_engine`] supplies
//!   heuristic starting values scaled to λ/NA — not an optimized or
//!   calibrated rule deck.
//! - Line assists are placed for parallel lines (rectangles) only; spaces
//!   are found per line edge against features that overlap it along the
//!   line; general 2D layouts (jogs, line ends, polygons) get no assists.
//! - The resist is a constant threshold on the aerial image; "does not
//!   print" means the assist stays on the non-printing side of that
//!   threshold with the requested margin at the sampled corners — no
//!   resist-blur, development, or etch model.
//! - The DOF depends on the imaging engine's defocus model; see the notes of
//!   [`compare_sraf_dof`].

use ndarray::Array2;
use serde::{Deserialize, Serialize};

use crate::aerial::AerialImageEngine;
use crate::error::{LithographyError, Result};
use crate::mask::{Mask, MaskFeature};
use crate::math::fft2d::Fft2D;
pub use crate::opc::ProcessCondition;
use crate::opc::{sample_image, validate_conditions};
use crate::types::GridConfig;

/// What the SRAF helpers need from an imaging model: its grid and aerial
/// images of geometric masks at a given focus.
pub(crate) trait SocsImager {
    fn imager_grid(&self) -> &GridConfig;
    fn images_at(&self, masks: &[&Mask], defocus_nm: f64, fft: &Fft2D) -> Vec<Array2<f64>>;
}

impl SocsImager for AerialImageEngine {
    fn imager_grid(&self) -> &GridConfig {
        self.grid()
    }
    /// The engine's images (exact analytic mask spectra through the SOCS
    /// kernels at `defocus_nm`).
    fn images_at(&self, masks: &[&Mask], defocus_nm: f64, _fft: &Fft2D) -> Vec<Array2<f64>> {
        masks
            .iter()
            .map(|m| self.compute(m, defocus_nm).data)
            .collect()
    }
}

/// Image of one geometric mask at one focus through an imager.
fn image_at(imager: &impl SocsImager, mask: &Mask, defocus_nm: f64, fft: &Fft2D) -> Array2<f64> {
    imager.images_at(&[mask], defocus_nm, fft).remove(0)
}

// ---------------------------------------------------------------------------
// Rules and placement
// ---------------------------------------------------------------------------

/// Scattering-bar rule for the space beside a line edge, selected when the
/// space `S` to the nearest facing feature satisfies
/// `min_space_nm ≤ S < max_space_nm`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineSrafRule {
    /// Smallest space (nm) this rule applies to.
    pub min_space_nm: f64,
    /// Largest space (nm, exclusive) this rule applies to; `f64::INFINITY`
    /// for isolated edges.
    pub max_space_nm: f64,
    /// Bars placed from each side of the space.
    pub bars_per_side: usize,
    /// Bar width (nm).
    pub bar_width_nm: f64,
    /// Gap from the main-feature edge to the first bar edge (nm).
    pub first_gap_nm: f64,
    /// Gap between successive bars on one side (nm, edge to edge).
    pub bar_gap_nm: f64,
}

/// Assist geometry around an isolated contact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContactSrafStyle {
    /// Four bars (left, right, below, above) whose centres sit
    /// `distance_nm` from the contact centre.
    Bars {
        distance_nm: f64,
        width_nm: f64,
        length_nm: f64,
    },
    /// Square assists of side `size_nm` at `(±d, 0)`, `(0, ±d)` and, with
    /// `diagonal`, `(±d, ±d)`.
    Squares {
        distance_nm: f64,
        size_nm: f64,
        diagonal: bool,
    },
}

/// Assist rule for isolated contacts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContactSrafRule {
    /// Contacts whose nearest neighbour (centre to centre, periodic) is at
    /// least this far get assists (nm).
    pub min_pitch_nm: f64,
    /// Assist geometry.
    pub style: ContactSrafStyle,
}

/// SRAF placement rule deck.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SrafRules {
    /// Space-keyed scattering-bar rules for lines (first match wins).
    pub line_rules: Vec<LineSrafRule>,
    /// Optional rule for isolated contacts.
    pub contact_rule: Option<ContactSrafRule>,
    /// Minimum clearance between an assist and any other feature or assist
    /// (nm); violating assists are dropped at placement.
    pub min_clearance_nm: f64,
    /// A rectangle is a line if its long/short side ratio is at least this.
    pub line_aspect_ratio: f64,
}

impl SrafRules {
    /// Heuristic starting deck scaled to the resolution unit `R = λ/NA`,
    /// for isolated / semi-isolated lines of width `line_cd_nm` under
    /// illumination whose effective (centre) partial coherence is
    /// `sigma_center` (e.g. the ring centre of an annular source; ≈ 0.6 for
    /// conventional illumination).
    ///
    /// Bars of width `0.14 R` are centred at multiples of the "equivalent
    /// dense pitch" `p* = R / (2σ_c)` (clamped to `[0.7 R, 1.0 R]`) from the
    /// line centre, up to two per side, in any space of at least
    /// `2p* − CD`; contacts farther than `3p*` from their neighbours get a
    /// four-bar ring at `p*`. Heuristic values — validate with the print
    /// check and [`compare_sraf_dof`].
    pub fn for_engine(engine: &AerialImageEngine, line_cd_nm: f64, sigma_center: f64) -> Self {
        let r = engine.wavelength_nm() / engine.na();
        Self::for_resolution(r, line_cd_nm, sigma_center)
    }

    fn for_resolution(r: f64, line_cd_nm: f64, sigma_center: f64) -> Self {
        let p_star = (r / (2.0 * sigma_center.max(1e-3))).clamp(0.7 * r, 1.0 * r);
        let w = 0.14 * r;
        let first_gap = (p_star - line_cd_nm / 2.0 - w / 2.0).max(0.1 * r);
        let bar_gap = (p_star - w).max(0.1 * r);
        Self {
            line_rules: vec![LineSrafRule {
                min_space_nm: 2.0 * p_star - line_cd_nm,
                max_space_nm: f64::INFINITY,
                bars_per_side: 2,
                bar_width_nm: w,
                first_gap_nm: first_gap,
                bar_gap_nm: bar_gap,
            }],
            contact_rule: Some(ContactSrafRule {
                min_pitch_nm: 3.0 * p_star,
                style: ContactSrafStyle::Bars {
                    distance_nm: p_star,
                    width_nm: w,
                    length_nm: line_cd_nm.max(0.5 * r),
                },
            }),
            min_clearance_nm: 0.1 * r,
            line_aspect_ratio: 3.0,
        }
    }
}

/// Axis-aligned box `[x0, x1] × [y0, y1]`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Bbox {
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
}

impl Bbox {
    fn from_feature(feature: &MaskFeature) -> Option<Self> {
        match feature {
            MaskFeature::Rect { x, y, w, h } | MaskFeature::GrayRect { x, y, w, h, .. } => {
                Some(Self {
                    x0: x - w / 2.0,
                    x1: x + w / 2.0,
                    y0: y - h / 2.0,
                    y1: y + h / 2.0,
                })
            }
            MaskFeature::Polygon { vertices } => {
                if vertices.is_empty() {
                    return None;
                }
                let mut b = Self {
                    x0: f64::INFINITY,
                    x1: f64::NEG_INFINITY,
                    y0: f64::INFINITY,
                    y1: f64::NEG_INFINITY,
                };
                for &(x, y) in vertices {
                    b.x0 = b.x0.min(x);
                    b.x1 = b.x1.max(x);
                    b.y0 = b.y0.min(y);
                    b.y1 = b.y1.max(y);
                }
                Some(b)
            }
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }

    fn transposed(self) -> Self {
        Self {
            x0: self.y0,
            x1: self.y1,
            y0: self.x0,
            y1: self.x1,
        }
    }

    fn to_rect(self) -> MaskFeature {
        MaskFeature::Rect {
            x: 0.5 * (self.x0 + self.x1),
            y: 0.5 * (self.y0 + self.y1),
            w: self.x1 - self.x0,
            h: self.y1 - self.y0,
        }
    }

    fn width(&self) -> f64 {
        self.x1 - self.x0
    }

    fn height(&self) -> f64 {
        self.y1 - self.y0
    }

    /// Clearance between two boxes on a periodic field (0 if they touch or
    /// overlap).
    fn periodic_clearance(&self, other: &Bbox, field: f64) -> f64 {
        let gap_1d = |a0: f64, a1: f64, b0: f64, b1: f64| {
            let mut best = f64::INFINITY;
            for k in [-1.0, 0.0, 1.0] {
                let (c0, c1) = (b0 + k * field, b1 + k * field);
                let g = if c1 < a0 {
                    a0 - c1
                } else if c0 > a1 {
                    c0 - a1
                } else {
                    0.0
                };
                best = best.min(g);
            }
            best
        };
        let gx = gap_1d(self.x0, self.x1, other.x0, other.x1);
        let gy = gap_1d(self.y0, self.y1, other.y0, other.y1);
        (gx * gx + gy * gy).sqrt()
    }
}

/// Bars for one space `[a, b]` (x-coordinates) spanning `[y0, y1]`.
///
/// Pairs of bars are placed inward from both edges (first gap `g₁`, then
/// `g₂` between bars) while the two bars of a pair keep at least `g₂`
/// between them; when the next pair no longer fits, one centred bar is added
/// if it keeps the required gaps to what is already there.
fn bars_in_space(a: f64, b: f64, y0: f64, y1: f64, rule: &LineSrafRule) -> Vec<Bbox> {
    let (w, g1, g2) = (rule.bar_width_nm, rule.first_gap_nm, rule.bar_gap_nm);
    if rule.bars_per_side == 0 || w <= 0.0 || b <= a || y1 <= y0 {
        return Vec::new();
    }
    let bar = |x0: f64| Bbox {
        x0,
        x1: x0 + w,
        y0,
        y1,
    };
    let mut out = Vec::new();
    let (mut left, mut right) = (a, b);
    for k in 0..rule.bars_per_side {
        let gap = if k == 0 { g1 } else { g2 };
        let lx0 = left + gap;
        let rx1 = right - gap;
        if (rx1 - w) - (lx0 + w) >= g2 {
            out.push(bar(lx0));
            out.push(bar(rx1 - w));
            left = lx0 + w;
            right = rx1 - w;
        } else {
            if right - left >= 2.0 * gap + w {
                out.push(bar(0.5 * (left + right) - 0.5 * w));
            }
            break;
        }
    }
    out
}

/// Line-edge spaces for vertical lines (in possibly transposed coordinates):
/// returns deduplicated `(a, b, y0, y1)` spaces.
fn line_spaces(lines: &[Bbox], others: &[Bbox], field: f64) -> Vec<(f64, f64, f64, f64)> {
    let mut spaces: Vec<(f64, f64, f64, f64)> = Vec::new();
    for line in lines {
        // Right side: nearest feature (incl. periodic images) overlapping
        // the line in y and lying to the right of its right edge.
        let mut best_right: Option<(f64, Bbox)> = None;
        let mut best_left: Option<(f64, Bbox)> = None;
        for f in others {
            for k in [-1.0, 0.0, 1.0] {
                let g = Bbox {
                    x0: f.x0 + k * field,
                    x1: f.x1 + k * field,
                    ..*f
                };
                if g.y1 <= line.y0 || g.y0 >= line.y1 {
                    continue;
                }
                if g.x0 >= line.x1 - 1e-9 && (g.x0 - line.x1) > 1e-9 {
                    let d = g.x0 - line.x1;
                    if best_right.is_none_or(|(bd, _)| d < bd) {
                        best_right = Some((d, g));
                    }
                }
                if g.x1 <= line.x0 + 1e-9 && (line.x0 - g.x1) > 1e-9 {
                    let d = line.x0 - g.x1;
                    if best_left.is_none_or(|(bd, _)| d < bd) {
                        best_left = Some((d, g));
                    }
                }
            }
        }
        if let Some((d, g)) = best_right {
            spaces.push((line.x1, line.x1 + d, line.y0.max(g.y0), line.y1.min(g.y1)));
        }
        if let Some((d, g)) = best_left {
            spaces.push((line.x0 - d, line.x0, line.y0.max(g.y0), line.y1.min(g.y1)));
        }
    }
    // Deduplicate spaces modulo the field period.
    let mut unique: Vec<(f64, f64, f64, f64)> = Vec::new();
    for s in spaces {
        let dup = unique.iter().any(|u| {
            let shift = ((s.0 - u.0) / field).round() * field;
            (s.0 - shift - u.0).abs() < 1e-6
                && (s.1 - shift - u.1).abs() < 1e-6
                && (s.2 - u.2).abs() < 1e-6
                && (s.3 - u.3).abs() < 1e-6
        });
        if !dup {
            unique.push(s);
        }
    }
    unique
}

/// Rule-based SRAF placement. Returns the assist rectangles only (same tone
/// as the main features); `mask` is not modified.
pub fn place_srafs(mask: &Mask, grid: &GridConfig, rules: &SrafRules) -> Vec<MaskFeature> {
    let field = grid.field_size_nm();
    let half = field / 2.0;
    let boxes: Vec<Bbox> = mask
        .features
        .iter()
        .filter_map(Bbox::from_feature)
        .collect();
    let is_rect: Vec<bool> = mask
        .features
        .iter()
        .map(|f| matches!(f, MaskFeature::Rect { .. }))
        .collect();
    let rect_boxes: Vec<(Bbox, bool)> = mask
        .features
        .iter()
        .zip(&is_rect)
        .filter_map(|(f, &r)| Bbox::from_feature(f).map(|b| (b, r)))
        .collect();

    let mut candidates: Vec<Bbox> = Vec::new();

    // Lines, vertical and horizontal (horizontal handled by transposition).
    for transpose in [false, true] {
        let tb = |b: &Bbox| if transpose { b.transposed() } else { *b };
        let lines: Vec<Bbox> = rect_boxes
            .iter()
            .filter(|(b, r)| {
                let b = tb(b);
                *r && b.height() >= rules.line_aspect_ratio * b.width()
            })
            .map(|(b, _)| tb(b))
            .collect();
        if lines.is_empty() {
            continue;
        }
        let others: Vec<Bbox> = boxes.iter().map(tb).collect();
        for (a, b, y0, y1) in line_spaces(&lines, &others, field) {
            let s = b - a;
            let Some(rule) = rules
                .line_rules
                .iter()
                .find(|r| s >= r.min_space_nm && s < r.max_space_nm)
            else {
                continue;
            };
            let (y0, y1) = (y0.max(-half), y1.min(half));
            for bar in bars_in_space(a, b, y0, y1, rule) {
                // Wrap bar centres into the field.
                let cx = 0.5 * (bar.x0 + bar.x1);
                let shift = ((cx + half) / field).floor() * field;
                let bar = Bbox {
                    x0: bar.x0 - shift,
                    x1: bar.x1 - shift,
                    ..bar
                };
                candidates.push(if transpose { bar.transposed() } else { bar });
            }
        }
    }

    // Isolated contacts.
    if let Some(rule) = &rules.contact_rule {
        let contacts: Vec<Bbox> = rect_boxes
            .iter()
            .filter(|(b, r)| {
                *r && b.height() < rules.line_aspect_ratio * b.width()
                    && b.width() < rules.line_aspect_ratio * b.height()
            })
            .map(|(b, _)| *b)
            .collect();
        let centre = |b: &Bbox| (0.5 * (b.x0 + b.x1), 0.5 * (b.y0 + b.y1));
        for c in &contacts {
            let (cx, cy) = centre(c);
            let mut nearest = f64::INFINITY;
            for o in &boxes {
                let (ox, oy) = centre(o);
                for kx in [-1.0, 0.0, 1.0] {
                    for ky in [-1.0, 0.0, 1.0] {
                        let d = ((ox + kx * field - cx).powi(2) + (oy + ky * field - cy).powi(2))
                            .sqrt();
                        if d > 1e-9 {
                            nearest = nearest.min(d);
                        }
                    }
                }
            }
            if nearest < rule.min_pitch_nm {
                continue;
            }
            match &rule.style {
                ContactSrafStyle::Bars {
                    distance_nm,
                    width_nm,
                    length_nm,
                } => {
                    let (d, w, l) = (*distance_nm, *width_nm, *length_nm);
                    candidates.push(Bbox {
                        x0: cx - d - w / 2.0,
                        x1: cx - d + w / 2.0,
                        y0: cy - l / 2.0,
                        y1: cy + l / 2.0,
                    });
                    candidates.push(Bbox {
                        x0: cx + d - w / 2.0,
                        x1: cx + d + w / 2.0,
                        y0: cy - l / 2.0,
                        y1: cy + l / 2.0,
                    });
                    candidates.push(Bbox {
                        x0: cx - l / 2.0,
                        x1: cx + l / 2.0,
                        y0: cy - d - w / 2.0,
                        y1: cy - d + w / 2.0,
                    });
                    candidates.push(Bbox {
                        x0: cx - l / 2.0,
                        x1: cx + l / 2.0,
                        y0: cy + d - w / 2.0,
                        y1: cy + d + w / 2.0,
                    });
                }
                ContactSrafStyle::Squares {
                    distance_nm,
                    size_nm,
                    diagonal,
                } => {
                    let (d, s) = (*distance_nm, *size_nm);
                    let mut offsets = vec![(-d, 0.0), (d, 0.0), (0.0, -d), (0.0, d)];
                    if *diagonal {
                        offsets.extend([(-d, -d), (-d, d), (d, -d), (d, d)]);
                    }
                    for (ox, oy) in offsets {
                        candidates.push(Bbox {
                            x0: cx + ox - s / 2.0,
                            x1: cx + ox + s / 2.0,
                            y0: cy + oy - s / 2.0,
                            y1: cy + oy + s / 2.0,
                        });
                    }
                }
            }
        }
    }

    // Clearance filter against main features and already accepted assists.
    let mut accepted: Vec<Bbox> = Vec::new();
    for c in candidates {
        if c.width() <= 0.0 || c.height() <= 0.0 {
            continue;
        }
        let clear_of_main = boxes
            .iter()
            .all(|b| c.periodic_clearance(b, field) >= rules.min_clearance_nm);
        let clear_of_assists = accepted
            .iter()
            .all(|a| c.periodic_clearance(a, field) >= rules.min_clearance_nm);
        if clear_of_main && clear_of_assists {
            accepted.push(c);
        }
    }
    accepted.into_iter().map(Bbox::to_rect).collect()
}

// ---------------------------------------------------------------------------
// Model-based print check
// ---------------------------------------------------------------------------

/// Settings of the model-based assist print check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintCheckConfig {
    /// Print threshold on the aerial image at nominal dose.
    pub threshold: f64,
    /// (focus, dose) corners to check; weights are ignored.
    pub conditions: Vec<ProcessCondition>,
    /// Relative safety margin: an assist must stay `margin·threshold` away
    /// from printing.
    pub margin: f64,
    /// Width reduction per failed check (nm).
    pub shrink_step_nm: f64,
    /// Assists narrower than this are removed instead of shrunk (nm).
    pub min_width_nm: f64,
    /// Maximum shrink/remove passes before all remaining violators are
    /// removed.
    pub max_passes: usize,
}

impl PrintCheckConfig {
    /// Corners `{−z, 0, +z} × {1 − e, 1, 1 + e}` for defocus `z` (nm) and
    /// relative dose excursion `e`.
    pub fn corners(threshold: f64, defocus_nm: f64, dose_excursion: f64) -> Self {
        let mut conditions = Vec::new();
        for z in [-defocus_nm, 0.0, defocus_nm] {
            for d in [1.0 - dose_excursion, 1.0, 1.0 + dose_excursion] {
                conditions.push(ProcessCondition::new(z, d));
            }
        }
        Self {
            threshold,
            conditions,
            margin: 0.05,
            shrink_step_nm: 4.0,
            min_width_nm: 12.0,
            max_passes: 12,
        }
    }
}

/// Print status of one assist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistPrintReport {
    /// Index into the assist list that was checked.
    pub index: usize,
    /// Worst normalized margin over corners and sample points:
    /// `min dose·I / t − 1` for opaque assists, `1 − max dose·I / t` for
    /// clear ones. Negative means the assist prints.
    pub worst_margin: f64,
    /// Condition index of the worst corner.
    pub worst_condition: usize,
}

/// Sample points covering an assist rectangle (spacing ≤ pixel/2).
fn assist_samples(feature: &MaskFeature, pixel_nm: f64) -> Vec<(f64, f64)> {
    let Some(b) = Bbox::from_feature(feature) else {
        return Vec::new();
    };
    let step = 0.5 * pixel_nm;
    let nx = ((b.width() / step).ceil() as usize).max(1);
    let ny = ((b.height() / step).ceil() as usize).max(1);
    let mut pts = Vec::with_capacity((nx + 1) * (ny + 1));
    for i in 0..=ny {
        for j in 0..=nx {
            pts.push((
                b.x0 + b.width() * j as f64 / nx as f64,
                b.y0 + b.height() * i as f64 / ny as f64,
            ));
        }
    }
    pts
}

fn check_with(
    imager: &impl SocsImager,
    main: &Mask,
    assists: &[MaskFeature],
    config: &PrintCheckConfig,
    fft: &Fft2D,
) -> Vec<AssistPrintReport> {
    let grid = imager.imager_grid();
    let mut mask = main.clone();
    mask.features.extend(assists.iter().cloned());
    let clear_assists = main.dark_field;
    let samples: Vec<Vec<(f64, f64)>> = assists
        .iter()
        .map(|a| assist_samples(a, grid.pixel_nm))
        .collect();
    let mut reports: Vec<AssistPrintReport> = (0..assists.len())
        .map(|index| AssistPrintReport {
            index,
            worst_margin: f64::INFINITY,
            worst_condition: 0,
        })
        .collect();
    // One image per distinct focus.
    let mut foci: Vec<f64> = Vec::new();
    for c in &config.conditions {
        if !foci.iter().any(|z| (z - c.defocus_nm).abs() < 1e-12) {
            foci.push(c.defocus_nm);
        }
    }
    for z in foci {
        let image = image_at(imager, &mask, z, fft);
        let values: Vec<(f64, f64)> = samples
            .iter()
            .map(|pts| {
                pts.iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &(x, y)| {
                        let v = sample_image(&image, grid, x, y);
                        (lo.min(v), hi.max(v))
                    })
            })
            .collect();
        for (ci, c) in config.conditions.iter().enumerate() {
            if (c.defocus_nm - z).abs() >= 1e-12 {
                continue;
            }
            for (report, &(lo, hi)) in reports.iter_mut().zip(&values) {
                let m = if clear_assists {
                    1.0 - c.dose * hi / config.threshold
                } else {
                    c.dose * lo / config.threshold - 1.0
                };
                if m < report.worst_margin {
                    report.worst_margin = m;
                    report.worst_condition = ci;
                }
            }
        }
    }
    reports
}

/// Check whether assists print at the corners of `config`.
///
/// `main` holds the main features; `assists` are imaged together with them.
/// An assist is safe when its [`AssistPrintReport::worst_margin`] is at least
/// `config.margin`.
pub fn check_assist_printing(
    engine: &AerialImageEngine,
    main: &Mask,
    assists: &[MaskFeature],
    config: &PrintCheckConfig,
) -> Result<Vec<AssistPrintReport>> {
    validate_print_config(config)?;
    Ok(check_with(engine, main, assists, config, &Fft2D::new()))
}

fn validate_print_config(config: &PrintCheckConfig) -> Result<()> {
    if !(config.threshold.is_finite() && config.threshold > 0.0) {
        return Err(LithographyError::InvalidParameter {
            name: "threshold",
            value: config.threshold,
            reason: "must be positive and finite",
        });
    }
    if !(config.margin.is_finite() && (0.0..1.0).contains(&config.margin)) {
        return Err(LithographyError::InvalidParameter {
            name: "margin",
            value: config.margin,
            reason: "must be in [0, 1)",
        });
    }
    if !(config.shrink_step_nm.is_finite() && config.shrink_step_nm > 0.0) {
        return Err(LithographyError::InvalidParameter {
            name: "shrink_step_nm",
            value: config.shrink_step_nm,
            reason: "must be positive and finite",
        });
    }
    validate_conditions(&config.conditions)
}

/// Result of [`insert_srafs`].
#[derive(Debug, Clone)]
pub struct SrafResult {
    /// Main features followed by the surviving assists.
    pub mask: Mask,
    /// Surviving assists (after shrinking).
    pub assists: Vec<MaskFeature>,
    /// Assists proposed by the rules.
    pub placed: usize,
    /// Assists removed by the print check.
    pub removed: usize,
    /// Shrink operations applied.
    pub shrink_steps: usize,
    /// Final print reports of the surviving assists (all margins ≥
    /// `config.margin`).
    pub reports: Vec<AssistPrintReport>,
}

/// Narrow an assist rectangle across its short side by `step` (keeping its
/// centre); `None` if it would fall below `min_width`.
fn shrink_assist(feature: &MaskFeature, step: f64, min_width: f64) -> Option<MaskFeature> {
    let MaskFeature::Rect { x, y, w, h } = feature else {
        return None;
    };
    let square = (w - h).abs() < 1e-9;
    if square {
        let s = w - step;
        (s >= min_width).then_some(MaskFeature::Rect {
            x: *x,
            y: *y,
            w: s,
            h: s,
        })
    } else if w < h {
        (w - step >= min_width).then_some(MaskFeature::Rect {
            x: *x,
            y: *y,
            w: w - step,
            h: *h,
        })
    } else {
        (h - step >= min_width).then_some(MaskFeature::Rect {
            x: *x,
            y: *y,
            w: *w,
            h: h - step,
        })
    }
}

fn insert_with(
    imager: &impl SocsImager,
    mask: &Mask,
    rules: &SrafRules,
    config: &PrintCheckConfig,
) -> Result<SrafResult> {
    validate_print_config(config)?;
    let fft = Fft2D::new();
    let mut assists = place_srafs(mask, imager.imager_grid(), rules);
    let placed = assists.len();
    let mut removed = 0;
    let mut shrink_steps = 0;
    let mut pass = 0;
    loop {
        let reports = check_with(imager, mask, &assists, config, &fft);
        let failing: Vec<usize> = reports
            .iter()
            .filter(|r| r.worst_margin < config.margin)
            .map(|r| r.index)
            .collect();
        if failing.is_empty() {
            let mut out = mask.clone();
            out.features.extend(assists.iter().cloned());
            return Ok(SrafResult {
                mask: out,
                assists,
                placed,
                removed,
                shrink_steps,
                reports,
            });
        }
        pass += 1;
        let mut next = Vec::with_capacity(assists.len());
        for (i, a) in assists.iter().enumerate() {
            if !failing.contains(&i) {
                next.push(a.clone());
            } else if pass <= config.max_passes {
                match shrink_assist(a, config.shrink_step_nm, config.min_width_nm) {
                    Some(s) => {
                        shrink_steps += 1;
                        next.push(s);
                    }
                    None => removed += 1,
                }
            } else {
                removed += 1;
            }
        }
        assists = next;
    }
}

/// Place assists by `rules`, then shrink or remove every assist that prints
/// (or comes within the margin of printing) at any corner of `config`, until
/// none does. The returned assists are guaranteed not to print at the
/// checked corners.
pub fn insert_srafs(
    mask: &Mask,
    engine: &AerialImageEngine,
    rules: &SrafRules,
    config: &PrintCheckConfig,
) -> Result<SrafResult> {
    insert_with(engine, mask, rules, config)
}

// ---------------------------------------------------------------------------
// CD and depth of focus
// ---------------------------------------------------------------------------

/// Where to measure a line CD: a horizontal cut at `y_nm` through a line
/// centred at `x_center_nm`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LineCut {
    /// Line centre (nm).
    pub x_center_nm: f64,
    /// Cut height (nm).
    pub y_nm: f64,
    /// Search half-width for the two edges (nm).
    pub half_width_nm: f64,
    /// `true` for a line printed where `I < t` (opaque line on a bright
    /// field); `false` for a line printed where `I > t` (clear trench).
    pub dark_line: bool,
}

/// CD of the line at `cut` for print threshold `threshold` (nm), from the
/// two threshold crossings nearest the line centre; `None` if the centre
/// does not print or an edge lies outside the search window.
pub fn line_cd(
    image: &Array2<f64>,
    grid: &GridConfig,
    cut: &LineCut,
    threshold: f64,
) -> Option<f64> {
    let sign = if cut.dark_line { -1.0 } else { 1.0 };
    let g = |x: f64| sign * (sample_image(image, grid, x, cut.y_nm) - threshold);
    if g(cut.x_center_nm) <= 0.0 {
        return None;
    }
    let step = (grid.pixel_nm / 4.0).min(1.0);
    let edge = |dir: f64| -> Option<f64> {
        let mut s_prev = 0.0;
        let mut s = step;
        while s <= cut.half_width_nm {
            if g(cut.x_center_nm + dir * s) <= 0.0 {
                let (mut lo, mut hi) = (s_prev, s);
                for _ in 0..40 {
                    let mid = 0.5 * (lo + hi);
                    if g(cut.x_center_nm + dir * mid) > 0.0 {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                return Some(0.5 * (lo + hi));
            }
            s_prev = s;
            s += step;
        }
        None
    };
    Some(edge(1.0)? + edge(-1.0)?)
}

/// Depth-of-focus analysis settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DofConfig {
    /// Target CD (nm); the threshold is anchored to print it at best focus.
    pub target_cd_nm: f64,
    /// Allowed CD deviation (percent of target).
    pub cd_tolerance_pct: f64,
    /// Largest |defocus| sampled (nm); the sweep is symmetric about 0.
    pub focus_range_nm: f64,
    /// Number of focus samples (odd keeps 0 on the grid).
    pub focus_steps: usize,
    /// Full exposure latitude (percent): doses `1 ± EL/2` must also stay in
    /// spec (0 = nominal dose only).
    pub exposure_latitude_pct: f64,
}

impl Default for DofConfig {
    fn default() -> Self {
        Self {
            target_cd_nm: 100.0,
            cd_tolerance_pct: 10.0,
            focus_range_nm: 400.0,
            focus_steps: 21,
            exposure_latitude_pct: 0.0,
        }
    }
}

/// Depth-of-focus result for one mask.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DofResult {
    /// Depth of focus (nm).
    pub dof_nm: f64,
    /// Anchored (dose-to-size) threshold.
    pub threshold: f64,
    /// Focus samples (nm).
    pub focus_nm: Vec<f64>,
    /// CD at nominal dose per focus sample (`None` = not printed).
    pub cd_nm: Vec<Option<f64>>,
    /// In-spec focus interval `(low, high)` (nm), interpolated.
    pub window_nm: (f64, f64),
}

/// DOF of a line with and without assists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SrafDofComparison {
    /// Main features only.
    pub without: DofResult,
    /// Main features plus assists.
    pub with: DofResult,
}

impl SrafDofComparison {
    /// DOF gain factor `with / without` (∞ if the bare line has no window).
    pub fn gain(&self) -> f64 {
        self.with.dof_nm / self.without.dof_nm
    }
}

fn validate_dof(cut: &LineCut, config: &DofConfig) -> Result<()> {
    let positive = |name: &'static str, v: f64| {
        if v.is_finite() && v > 0.0 {
            Ok(())
        } else {
            Err(LithographyError::InvalidParameter {
                name,
                value: v,
                reason: "must be positive and finite",
            })
        }
    };
    positive("target_cd_nm", config.target_cd_nm)?;
    positive("cd_tolerance_pct", config.cd_tolerance_pct)?;
    positive("focus_range_nm", config.focus_range_nm)?;
    positive("half_width_nm", cut.half_width_nm)?;
    if config.focus_steps < 3 {
        return Err(LithographyError::InvalidParameter {
            name: "focus_steps",
            value: config.focus_steps as f64,
            reason: "need at least 3 focus samples",
        });
    }
    if !(config.exposure_latitude_pct.is_finite()
        && (0.0..100.0).contains(&config.exposure_latitude_pct))
    {
        return Err(LithographyError::InvalidParameter {
            name: "exposure_latitude_pct",
            value: config.exposure_latitude_pct,
            reason: "must be in [0, 100)",
        });
    }
    Ok(())
}

/// Anchor the threshold so the best-focus, nominal-dose CD equals the
/// target (bisection; the CD grows with `t` for a dark line and shrinks with
/// `t` for a clear one).
fn anchor_threshold(
    image: &Array2<f64>,
    grid: &GridConfig,
    cut: &LineCut,
    target: f64,
) -> Result<f64> {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    let n = 400;
    for k in 0..=n {
        let x = cut.x_center_nm - cut.half_width_nm + 2.0 * cut.half_width_nm * k as f64 / n as f64;
        let v = sample_image(image, grid, x, cut.y_nm);
        lo = lo.min(v);
        hi = hi.max(v);
    }
    let centre = sample_image(image, grid, cut.x_center_nm, cut.y_nm);
    let (mut a, mut b) = (lo, hi);
    for _ in 0..80 {
        let t = 0.5 * (a + b);
        let centre_prints = if cut.dark_line {
            centre < t
        } else {
            centre > t
        };
        let want_bigger = if !centre_prints {
            true
        } else {
            match line_cd(image, grid, cut, t) {
                // Printed but an edge lies beyond the window: CD too large.
                None => false,
                Some(cd) => cd < target,
            }
        };
        // A bigger CD needs a higher threshold for a dark line, a lower one
        // for a clear line.
        if want_bigger == cut.dark_line {
            a = t;
        } else {
            b = t;
        }
    }
    let t = 0.5 * (a + b);
    match line_cd(image, grid, cut, t) {
        Some(cd) if (cd - target).abs() < 0.01 * target.max(1.0) => Ok(t),
        _ => Err(LithographyError::InvalidParameter {
            name: "target_cd_nm",
            value: target,
            reason: "target CD cannot be printed at best focus with any threshold at this cut",
        }),
    }
}

/// Dose-to-size threshold: the print threshold at which the line at `cut`
/// on `mask` prints at `target_cd_nm` in focus at nominal dose (with a
/// constant-threshold resist, anchoring the threshold is equivalent to
/// choosing the dose).
pub fn dose_to_size_threshold(
    engine: &AerialImageEngine,
    mask: &Mask,
    cut: &LineCut,
    target_cd_nm: f64,
) -> Result<f64> {
    let image = image_at(engine, mask, 0.0, &Fft2D::new());
    anchor_threshold(&image, engine.grid(), cut, target_cd_nm)
}

/// Worst in-spec margin over dose corners at one focus (nm; ≥ 0 in spec).
fn focus_margin(
    image: &Array2<f64>,
    grid: &GridConfig,
    cut: &LineCut,
    threshold: f64,
    config: &DofConfig,
) -> (f64, Option<f64>) {
    let tol = config.cd_tolerance_pct / 100.0 * config.target_cd_nm;
    let e = config.exposure_latitude_pct / 200.0;
    let nominal_cd = line_cd(image, grid, cut, threshold);
    let mut margin = f64::INFINITY;
    for d in [1.0 - e, 1.0, 1.0 + e] {
        let cd = if d == 1.0 {
            nominal_cd
        } else {
            line_cd(image, grid, cut, threshold / d)
        };
        let m = match cd {
            Some(cd) => tol - (cd - config.target_cd_nm).abs(),
            None => -config.target_cd_nm,
        };
        margin = margin.min(m);
    }
    (margin, nominal_cd)
}

fn dof_from_margins(focus: &[f64], margins: &[f64]) -> (f64, (f64, f64)) {
    // Start from the sample nearest best focus (z = 0).
    let centre = focus
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .map(|(i, _)| i)
        .unwrap_or(0);
    if margins[centre] < 0.0 {
        return (0.0, (focus[centre], focus[centre]));
    }
    let crossing = |i_in: usize, i_out: usize| {
        let (m0, m1) = (margins[i_in], margins[i_out]);
        let t = m0 / (m0 - m1);
        focus[i_in] + t * (focus[i_out] - focus[i_in])
    };
    let mut hi_i = centre;
    while hi_i + 1 < focus.len() && margins[hi_i + 1] >= 0.0 {
        hi_i += 1;
    }
    let high = if hi_i + 1 < focus.len() {
        crossing(hi_i, hi_i + 1)
    } else {
        focus[hi_i]
    };
    let mut lo_i = centre;
    while lo_i > 0 && margins[lo_i - 1] >= 0.0 {
        lo_i -= 1;
    }
    let low = if lo_i > 0 {
        crossing(lo_i, lo_i - 1)
    } else {
        focus[lo_i]
    };
    (high - low, (low, high))
}

fn dof_with(
    imager: &impl SocsImager,
    masks: &[&Mask],
    cut: &LineCut,
    config: &DofConfig,
) -> Result<Vec<DofResult>> {
    validate_dof(cut, config)?;
    let grid = imager.imager_grid();
    let fft = Fft2D::new();
    let n = config.focus_steps;
    let focus: Vec<f64> = (0..n)
        .map(|k| -config.focus_range_nm + 2.0 * config.focus_range_nm * k as f64 / (n - 1) as f64)
        .collect();
    let images_at = |z: f64| imager.images_at(masks, z, &fft);
    let thresholds = images_at(0.0)
        .iter()
        .map(|image| anchor_threshold(image, grid, cut, config.target_cd_nm))
        .collect::<Result<Vec<f64>>>()?;
    let mut margins = vec![Vec::with_capacity(n); masks.len()];
    let mut cds = vec![Vec::with_capacity(n); masks.len()];
    for &z in &focus {
        for (k, image) in images_at(z).iter().enumerate() {
            let (m, cd) = focus_margin(image, grid, cut, thresholds[k], config);
            margins[k].push(m);
            cds[k].push(cd);
        }
    }
    Ok((0..masks.len())
        .map(|k| {
            let (dof_nm, window_nm) = dof_from_margins(&focus, &margins[k]);
            DofResult {
                dof_nm,
                threshold: thresholds[k],
                focus_nm: focus.clone(),
                cd_nm: cds[k].clone(),
                window_nm,
            }
        })
        .collect())
}

/// Depth of focus of the line at `cut` on `mask`.
pub fn line_dof(
    engine: &AerialImageEngine,
    mask: &Mask,
    cut: &LineCut,
    config: &DofConfig,
) -> Result<DofResult> {
    Ok(dof_with(engine, &[mask], cut, config)?.remove(0))
}

/// DOF of the same line with and without assists (each at its own
/// dose-to-size threshold).
///
/// The result is only as good as the engine's defocus model: SRAFs improve
/// isolated-line DOF mainly under off-axis illumination, through focus
/// behaviour that requires the defocus phase to be applied inside the pupil
/// at every source point (and mask frequencies up to (1 + σ)·NA/λ to be
/// kept). An engine that applies defocus as a phase on the mask-frequency
/// kernels cannot represent that mechanism.
pub fn compare_sraf_dof(
    engine: &AerialImageEngine,
    without: &Mask,
    with: &Mask,
    cut: &LineCut,
    config: &DofConfig,
) -> Result<SrafDofComparison> {
    let mut r = dof_with(engine, &[without, with], cut, config)?;
    let with = r.remove(1);
    let without = r.remove(0);
    Ok(SrafDofComparison { without, with })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mask::MaskType;
    use crate::optics::{OpticalSystem, ProjectionOptics};
    use crate::source::{IlluminationShape, LithographySource, VuvSource};

    /// Textbook scalar Abbe imaging: `I = Σ_s w_s |F⁻¹(P((f + s)/f_c; z) ⊙ M)|²`
    /// over source points `s` (normalized weights `w_s`), with the optics'
    /// defocus phase (exact angular-spectrum by default, paraxial only with
    /// `paraxial_defocus`) applied inside the pupil at `f + s`, plus the
    /// optics' uniform flare. Used as a reference model independent of the
    /// engine's kernel construction.
    struct AbbeImager {
        grid: GridConfig,
        points: Vec<(f64, f64, f64)>,
        optics: ProjectionOptics,
        wavelength_nm: f64,
    }

    impl AbbeImager {
        fn new(illumination: IlluminationShape, grid: GridConfig, samples: usize) -> Self {
            let source = VuvSource {
                illumination,
                ..VuvSource::f2_laser(0.5).unwrap()
            };
            let mut points = Vec::new();
            for iy in 0..samples {
                for ix in 0..samples {
                    let sx = -1.0 + (ix as f64 + 0.5) * 2.0 / samples as f64;
                    let sy = -1.0 + (iy as f64 + 0.5) * 2.0 / samples as f64;
                    let w = source.intensity_at(sx, sy);
                    if w > 0.0 {
                        points.push((sx, sy, w));
                    }
                }
            }
            let total: f64 = points.iter().map(|p| p.2).sum();
            for p in &mut points {
                p.2 /= total;
            }
            Self {
                grid,
                points,
                optics: ProjectionOptics::new(0.75).unwrap(),
                wavelength_nm: source.wavelength_nm(),
            }
        }
    }

    impl SocsImager for AbbeImager {
        fn imager_grid(&self) -> &GridConfig {
            &self.grid
        }
        fn images_at(&self, masks: &[&Mask], defocus_nm: f64, fft: &Fft2D) -> Vec<Array2<f64>> {
            let n = self.grid.size;
            let df = self.grid.freq_step();
            let cutoff = self.optics.na() / self.wavelength_nm;
            let kernels: Vec<Array2<crate::types::Complex64>> = self
                .points
                .iter()
                .map(|&(sx, sy, _)| {
                    Array2::from_shape_fn((n, n), |(i, j)| {
                        let fx = crate::opc::fft_freq(j, n, df) / cutoff + sx;
                        let fy = crate::opc::fft_freq(i, n, df) / cutoff + sy;
                        self.optics
                            .pupil_function(fx, fy, defocus_nm, self.wavelength_nm)
                    })
                })
                .collect();
            masks
                .iter()
                .map(|m| {
                    let spectrum = m.spectrum(&self.grid, fft);
                    let coherent =
                        |(k, &(_, _, w)): (&Array2<crate::types::Complex64>, &(f64, f64, f64))| {
                            let mut field = k * &spectrum;
                            fft.inverse(&mut field);
                            field.mapv(|a| w * a.norm_sqr())
                        };
                    #[cfg(feature = "parallel")]
                    let parts: Vec<Array2<f64>> = {
                        use rayon::prelude::*;
                        kernels
                            .par_iter()
                            .zip(self.points.par_iter())
                            .map(coherent)
                            .collect()
                    };
                    #[cfg(not(feature = "parallel"))]
                    let parts: Vec<Array2<f64>> = kernels
                        .iter()
                        .zip(self.points.iter())
                        .map(coherent)
                        .collect();
                    let mut image = Array2::zeros((n, n));
                    for part in &parts {
                        image += part;
                    }
                    crate::opc::apply_flare(&mut image, self.optics.flare_fraction());
                    image
                })
                .collect()
        }
    }

    fn iso_line(cd: f64, field: f64) -> Mask {
        Mask {
            mask_type: MaskType::Binary,
            features: vec![MaskFeature::Rect {
                x: 0.0,
                y: 0.0,
                w: cd,
                h: field,
            }],
            dark_field: false,
        }
    }

    fn annular() -> IlluminationShape {
        IlluminationShape::Annular {
            sigma_inner: 0.5,
            sigma_outer: 0.8,
        }
    }

    fn grid_768() -> GridConfig {
        GridConfig {
            size: 64,
            pixel_nm: 12.0,
        }
    }

    fn cut_80() -> LineCut {
        LineCut {
            x_center_nm: 0.0,
            y_nm: 0.0,
            half_width_nm: 120.0,
            dark_line: true,
        }
    }

    /// Independent no-print verification: image the final mask at every
    /// corner and sample each assist on a fine grid.
    fn assert_assists_do_not_print(
        imager: &impl SocsImager,
        result: &SrafResult,
        check: &PrintCheckConfig,
    ) {
        let grid = imager.imager_grid();
        let fft = Fft2D::new();
        for cond in &check.conditions {
            let image = image_at(imager, &result.mask, cond.defocus_nm, &fft);
            for a in &result.assists {
                let b = Bbox::from_feature(a).unwrap();
                for i in 0..=8 {
                    for j in 0..=8 {
                        let x = b.x0 + b.width() * j as f64 / 8.0;
                        let y = b.y0 + b.height() * i as f64 / 8.0;
                        let v = cond.dose * sample_image(&image, grid, x, y);
                        let printed = if result.mask.dark_field {
                            v > check.threshold
                        } else {
                            v < check.threshold
                        };
                        assert!(
                            !printed,
                            "assist {a:?} prints at {cond:?}: {v} vs {}",
                            check.threshold
                        );
                    }
                }
            }
        }
    }

    fn test_rule() -> LineSrafRule {
        LineSrafRule {
            min_space_nm: 260.0,
            max_space_nm: f64::INFINITY,
            bars_per_side: 2,
            bar_width_nm: 30.0,
            first_gap_nm: 115.0,
            bar_gap_nm: 140.0,
        }
    }

    #[test]
    fn test_bars_in_space_fixture() {
        let rule = test_rule();
        let xs =
            |bars: Vec<Bbox>| -> Vec<(f64, f64)> { bars.iter().map(|b| (b.x0, b.x1)).collect() };
        // Wide space: one pair at g₁, the second pair would crowd (58 nm <
        // g₂ apart) so one centred bar fills the middle.
        assert_eq!(
            xs(bars_in_space(40.0, 728.0, 0.0, 1.0, &rule)),
            vec![(155.0, 185.0), (583.0, 613.0), (369.0, 399.0)]
        );
        // Semi-dense space: the first pair crowds → one centred bar.
        assert_eq!(
            xs(bars_in_space(0.0, 300.0, 0.0, 1.0, &rule)),
            vec![(135.0, 165.0)]
        );
        // Too narrow for a bar with g₁ on both sides (250 < 2·115 + 30).
        assert!(bars_in_space(0.0, 250.0, 0.0, 1.0, &rule).is_empty());
    }

    #[test]
    fn test_place_srafs_iso_dense_and_horizontal_lines() {
        let grid = grid_768();
        let rules = SrafRules {
            line_rules: vec![test_rule()],
            contact_rule: None,
            min_clearance_nm: 20.0,
            line_aspect_ratio: 3.0,
        };
        let centres = |a: &[MaskFeature], vertical: bool| -> Vec<f64> {
            let mut c: Vec<f64> = a
                .iter()
                .map(|f| match f {
                    MaskFeature::Rect { x, y, .. } => {
                        if vertical {
                            *x
                        } else {
                            *y
                        }
                    }
                    _ => f64::NAN,
                })
                .collect();
            c.sort_by(f64::total_cmp);
            c
        };
        // Isolated vertical line (space 688 nm to its periodic image).
        let iso = place_srafs(&iso_line(80.0, 768.0), &grid, &rules);
        let c = centres(&iso, true);
        assert_eq!(c.len(), 3);
        for (got, want) in c.iter().zip([-384.0, -170.0, 170.0]) {
            assert!((got - want).abs() < 1e-9, "{c:?}");
        }
        // Dense lines (space 120 nm) get nothing.
        let mut dense = iso_line(80.0, 768.0);
        dense.features = (-2..2)
            .map(|k| MaskFeature::Rect {
                x: 100.0 + 200.0 * k as f64,
                y: 0.0,
                w: 80.0,
                h: 768.0,
            })
            .collect();
        assert!(place_srafs(&dense, &grid, &rules).is_empty());
        // A horizontal line gets the same bars along y.
        let mut horiz = iso_line(80.0, 768.0);
        horiz.features = vec![MaskFeature::Rect {
            x: 0.0,
            y: 0.0,
            w: 768.0,
            h: 80.0,
        }];
        let c = centres(&place_srafs(&horiz, &grid, &rules), false);
        assert_eq!(c.len(), 3);
        assert!((c[2] - 170.0).abs() < 1e-9);
    }

    #[test]
    fn test_contact_ring_and_clearance_filter() {
        let grid = grid_768();
        let rules = SrafRules::for_resolution(157.63 / 0.75, 100.0, 0.65);
        let p_star = 157.63 / 0.75 / 1.3;
        let contact = |x: f64, y: f64| MaskFeature::Rect {
            x,
            y,
            w: 100.0,
            h: 100.0,
        };
        let iso = Mask {
            mask_type: MaskType::Binary,
            features: vec![contact(0.0, 0.0)],
            dark_field: true,
        };
        let ring = place_srafs(&iso, &grid, &rules);
        assert_eq!(ring.len(), 4);
        for a in &ring {
            let MaskFeature::Rect { x, y, .. } = a else {
                panic!("rect expected")
            };
            assert!(((x * x + y * y).sqrt() - p_star).abs() < 1e-9);
        }
        // A 2×2 array at 200 nm pitch is dense: no assists.
        let dense = Mask {
            features: vec![
                contact(-100.0, -100.0),
                contact(100.0, -100.0),
                contact(-100.0, 100.0),
                contact(100.0, 100.0),
            ],
            ..iso.clone()
        };
        assert!(place_srafs(&dense, &grid, &rules).is_empty());
        // A pad where the right-hand bar would go removes just that bar
        // (isolation pitch lowered so the pad does not make the contact
        // "dense").
        let mut blocked = iso.clone();
        let (px, h) = (p_star + 10.0, 15.0);
        // A polygon pad: counted for clearance, not itself a contact.
        blocked.features.push(MaskFeature::Polygon {
            vertices: vec![(px - h, -h), (px + h, -h), (px + h, h), (px - h, h)],
        });
        let mut rules = rules.clone();
        if let Some(c) = rules.contact_rule.as_mut() {
            c.min_pitch_nm = 100.0;
        }
        let kept = place_srafs(&blocked, &grid, &rules);
        assert_eq!(kept.len(), 3);
        assert!(kept
            .iter()
            .all(|a| matches!(a, MaskFeature::Rect { x, .. } if *x < p_star - 1.0)));
    }

    #[test]
    fn test_print_check_shrinks_or_removes_printing_assists() {
        // Deliberately wide (70 nm) opaque bars beside an 80 nm line print;
        // the check must narrow or remove them until nothing prints.
        let grid = grid_768();
        let source = VuvSource {
            illumination: annular(),
            ..VuvSource::f2_laser(0.5).unwrap()
        };
        let engine = AerialImageEngine::new(
            &source,
            &ProjectionOptics::new(0.75).unwrap(),
            grid.clone(),
            16,
        )
        .unwrap();
        let bare = iso_line(80.0, 768.0);
        let t = dose_to_size_threshold(&engine, &bare, &cut_80(), 80.0).unwrap();
        let rules = SrafRules {
            line_rules: vec![LineSrafRule {
                bar_width_nm: 70.0,
                first_gap_nm: 90.0,
                ..test_rule()
            }],
            contact_rule: None,
            min_clearance_nm: 20.0,
            line_aspect_ratio: 3.0,
        };
        let check = PrintCheckConfig::corners(t, 150.0, 0.08);
        let wide = place_srafs(&bare, &grid, &rules);
        let before = check_assist_printing(&engine, &bare, &wide, &check).unwrap();
        assert!(
            before.iter().any(|r| r.worst_margin < 0.0),
            "wide bars should print"
        );
        let result = insert_srafs(&bare, &engine, &rules, &check).unwrap();
        assert!(result.shrink_steps + result.removed > 0);
        assert!(result
            .reports
            .iter()
            .all(|r| r.worst_margin >= check.margin));
        assert_assists_do_not_print(&engine, &result, &check);
    }

    #[test]
    fn test_line_cd_and_dof_interpolation_fixtures() {
        // Dark Gaussian line I(x) = 1 − 0.8·exp(−x²/(2·40²)): at t = 0.5 the
        // printed width is 2·√(3200·ln 1.6) = 77.56 nm.
        let grid = GridConfig {
            size: 64,
            pixel_nm: 4.0,
        };
        let image = Array2::from_shape_fn((64, 64), |(_, j)| {
            let x = -128.0 + (j as f64 + 0.5) * 4.0;
            1.0 - 0.8 * (-x * x / 3200.0).exp()
        });
        let cd = line_cd(&image, &grid, &cut_80(), 0.5).unwrap();
        let expected = 2.0 * (3200.0 * 1.6_f64.ln()).sqrt();
        assert!((cd - expected).abs() < 0.05, "{cd} vs {expected}");
        // Threshold below the line minimum: nothing prints.
        assert!(line_cd(&image, &grid, &cut_80(), 0.1).is_none());
        // Interval interpolation: margins −1, 1, 2, 1, −3 at z = −2…2 give
        // low = −1.5, high = 1.25.
        let (dof, (lo, hi)) =
            dof_from_margins(&[-2.0, -1.0, 0.0, 1.0, 2.0], &[-1.0, 1.0, 2.0, 1.0, -3.0]);
        assert!(
            (lo + 1.5).abs() < 1e-12 && (hi - 1.25).abs() < 1e-12 && (dof - 2.75).abs() < 1e-12
        );
    }

    #[test]
    fn test_sraf_improves_isolated_line_dof_reference_model() {
        // Representative low-k₁ case on the textbook Abbe reference model:
        // F2 157.63 nm, NA 0.75, annular σ 0.5–0.8; isolated 80 nm opaque
        // line (k₁ = 0.38) in a 768 nm periodic field; heuristic rule deck;
        // assists checked at ±150 nm focus × ±8 % dose with a 5 % margin.
        // Same source sampling (15 × 15) and 50 nm focus spacing as the docs
        // table, whose wider ±500 nm sweep only adds out-of-window samples:
        // DOF ≈ 205 → 265 nm (×1.29) with the exact (non-paraxial) defocus
        // phase; asserted as a ≥ 20 % gain.
        let grid = grid_768();
        let abbe = AbbeImager::new(annular(), grid.clone(), 15);
        let bare = iso_line(80.0, 768.0);
        let fft = Fft2D::new();
        let t =
            anchor_threshold(&image_at(&abbe, &bare, 0.0, &fft), &grid, &cut_80(), 80.0).unwrap();
        let rules = SrafRules::for_resolution(157.63 / 0.75, 80.0, 0.65);
        let check = PrintCheckConfig::corners(t, 150.0, 0.08);
        let result = insert_with(&abbe, &bare, &rules, &check).unwrap();
        assert!(result.assists.len() >= 2);
        assert_assists_do_not_print(&abbe, &result, &check);
        let config = DofConfig {
            target_cd_nm: 80.0,
            focus_range_nm: 400.0,
            focus_steps: 17,
            ..Default::default()
        };
        let r = dof_with(&abbe, &[&bare, &result.mask], &cut_80(), &config).unwrap();
        let gain = r[1].dof_nm / r[0].dof_nm;
        assert!(
            gain > 1.2,
            "DOF {} -> {} (x{gain:.2})",
            r[0].dof_nm,
            r[1].dof_nm
        );
    }

    #[test]
    fn test_sraf_improves_isolated_line_dof_engine() {
        // Same case through the aerial engine. Its gain depends on the
        // engine's defocus model (see `compare_sraf_dof`); the assertion is
        // deliberately weaker than the reference-model one.
        let grid = grid_768();
        let source = VuvSource {
            illumination: annular(),
            ..VuvSource::f2_laser(0.5).unwrap()
        };
        let engine = AerialImageEngine::new(
            &source,
            &ProjectionOptics::new(0.75).unwrap(),
            grid.clone(),
            16,
        )
        .unwrap();
        let bare = iso_line(80.0, 768.0);
        let t = dose_to_size_threshold(&engine, &bare, &cut_80(), 80.0).unwrap();
        let rules = SrafRules::for_engine(&engine, 80.0, 0.65);
        let check = PrintCheckConfig::corners(t, 150.0, 0.08);
        let result = insert_srafs(&bare, &engine, &rules, &check).unwrap();
        assert!(result.assists.len() >= 2);
        assert_assists_do_not_print(&engine, &result, &check);
        let config = DofConfig {
            target_cd_nm: 80.0,
            focus_range_nm: 300.0,
            focus_steps: 13,
            ..Default::default()
        };
        let cmp = compare_sraf_dof(&engine, &bare, &result.mask, &cut_80(), &config).unwrap();
        assert!(
            cmp.gain() > 1.08,
            "DOF {} -> {}",
            cmp.without.dof_nm,
            cmp.with.dof_nm
        );
        // Dose-to-size anchoring prints the target CD in focus.
        let image = engine.compute(&bare, 0.0).data;
        let cd = line_cd(&image, &grid, &cut_80(), t).unwrap();
        assert!((cd - 80.0).abs() < 0.8);
    }

    #[test]
    fn test_invalid_inputs_rejected() {
        let grid = grid_768();
        let source = VuvSource::f2_laser(0.5).unwrap();
        let engine =
            AerialImageEngine::new(&source, &ProjectionOptics::new(0.75).unwrap(), grid, 4)
                .unwrap();
        let bare = iso_line(80.0, 768.0);
        let bad_dof = DofConfig {
            focus_steps: 2,
            ..Default::default()
        };
        assert!(line_dof(&engine, &bare, &cut_80(), &bad_dof).is_err());
        // A target CD far beyond what the cut window can hold is rejected.
        let huge = DofConfig {
            target_cd_nm: 500.0,
            ..Default::default()
        };
        assert!(line_dof(&engine, &bare, &cut_80(), &huge).is_err());
        let bad_check = PrintCheckConfig {
            margin: 1.5,
            ..PrintCheckConfig::corners(0.3, 100.0, 0.05)
        };
        assert!(check_assist_printing(&engine, &bare, &[], &bad_check).is_err());
    }
}
