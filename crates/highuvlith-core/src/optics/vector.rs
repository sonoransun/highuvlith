//! Vector (polarized) pupil model for high-NA imaging.
//!
//! At high numerical aperture the image is not a scalar field. Each
//! plane-wave order that the projection lens delivers to the wafer carries
//! an electric field perpendicular to its own propagation direction, and two
//! orders interfere only through the field components they share. Scalar
//! theory lets every pair of orders interfere fully, so at NA ≳ 0.8 it
//! overstates image contrast. This module computes the image-side field
//! `(E_x, E_y, E_z)` of one order at a time. The aerial-image engine
//! (`aerial.rs`) assembles those fields into the transmission
//! cross-coefficient (TCC).
//!
//! # Contract (used by `aerial.rs`)
//!
//! The engine factorizes the TCC as `TCC = A·Aᴴ`. Every source point appends
//! [`columns_per_source_point`] columns to `A`. For each in-support mask
//! frequency `f`, the entries of those columns come from [`field_columns`]
//! evaluated at the shifted pupil coordinate `p = (f + s) / (NA/λ)`; the
//! engine multiplies them by `sqrt(w_s)`. Each column is one mutually
//! incoherent (polarization state, field component) pair, so `A·Aᴴ` sums
//! `|E_x|² + |E_y|² + |E_z|²` over the states automatically.
//!
//! # Key equations
//!
//! ```text
//! direction sines   (α, β) = NA·(px, py) / n_img,   γ = cos θ = √(1 − α² − β²)
//!                   |p| > 1 or α² + β² ≥ 1  →  zero field (outside pupil / evanescent)
//!
//! TE/TM transfer    E = (J·ê_s) ê_s + (J·ê_p,in) ê_p,out            φ = atan2(py, px)
//!                   ê_s     = (−sin φ, cos φ, 0)                   (TE, unchanged by the lens)
//!                   ê_p,in  = (cos φ, sin φ, 0)                    (TM, paraxial object side)
//!                   ê_p,out = (cos θ cos φ, cos θ sin φ, −sin θ)   (TM, tilted with the order)
//!
//!                   equivalently E = M·J with the 3×2 matrix (no atan2 singularity at p = 0)
//!                       M = [ 1 − α²/(1+γ)    −αβ/(1+γ)   ]
//!                           [  −αβ/(1+γ)    1 − β²/(1+γ)  ]
//!                           [     −α             −β       ]
//!                   MᵀM = I₂ (|M·J| = |J|),  M·Mᵀ = I₃ − k̂k̂ᵀ,  k̂ = (α, β, γ)
//!
//! obliquity         A(p) = (cos θ_obj / cos θ_img)^(1/2)
//!                        = [ (1 − (NA|p|/R)²) / (1 − (NA|p|/n_img)²) ]^(1/4)
//!
//! film entrance     N = n + ik,   sin θ₂ = n_img sin θ / N,   N cos θ₂ = √(N² − n_img² sin²θ),  Im ≥ 0
//!                   t_s = 2 n_img cos θ / (n_img cos θ + N cos θ₂)
//!                   t_p = 2 n_img cos θ / (N cos θ + n_img cos θ₂)
//!                   E   = t_s (J·ê_s) ê_s + t_p (J·ê_p,in) ê_p,2
//!                   ê_p,2 = (cos θ₂ cos φ, cos θ₂ sin φ, −sin θ₂)     (complex angle)
//!
//! column entry      out[3k + c] = P(p) · A(p) · [M·J_k]_c      (state k, component c = x, y, z)
//! ```
//!
//! # Derivation of the radiometric (obliquity) factor
//!
//! Let the reticle side have index `n_o = 1` and the lens reduce by `R` (image
//! magnification `1/R`). The lens is aplanatic, so it obeys the Abbe sine
//! condition `n_img sin θ_img = R·n_o sin θ_obj = NA·|p|`. The normalized
//! pupil coordinate `p` therefore labels the same ray bundle on both sides,
//! and pupil areas on the two sides differ only by a constant Jacobian. In the
//! angular-spectrum (Fourier) representation, a plane-wave component of
//! amplitude `E` in a medium of index `n` carries z-directed power
//! `∝ n |E|² cos θ` per unit transverse-frequency area. A lossless lens
//! conserves the power of every ray bundle:
//!
//! ```text
//!   n_img |E_img(p)|² cos θ_img = K · n_o |E_obj(p)|² cos θ_obj      (K independent of p)
//! ⇒ |E_img(p)| ∝ |E_obj(p)| · (cos θ_obj / cos θ_img)^(1/2)
//! ```
//!
//! On the object side the thin mask fixes `|E_obj| = |J|·|T(f)|`. Fixing the
//! pupil-independent constant by unit transfer on axis gives
//! `A = (cos θ_obj/cos θ_img)^(1/2)`. The exponent is 1/2 on the ratio of
//! cosines, which is 1/4 on the ratio of `1 − sin²` terms, because the power
//! balance is quadratic in the field. Two limits check it. For `R → ∞`
//! (object at infinity) `A = 1/√cos θ_img`. That is the Richards–Wolf `√cos θ`
//! aplanatic apodization rewritten from a solid-angle integral to a
//! transverse-frequency integral: `dΩ = d²(sin θ)/cos θ`, so
//! `√cos θ · dΩ = d²(sin θ)/√cos θ`. For a symmetric 1:1 system
//! (`R = 1`, `n_img = n_o`) `A ≡ 1`.
//!
//! Tested energy statement (film off):
//! `|E(p)|² cos θ_img = |P(p)|² |J|² cos θ_obj` at every pupil point.
//!
//! Consequence for image normalization: `A(0) = 1` and `A > 1` off axis. A
//! clear mask under off-axis illumination therefore gives
//! `Σ_s w_s Σ_k |E_k(p = s)|² > 1` when `obliquity` is on. That excess is
//! physical, because a tilted plane wave carries more `|E|²` per unit wafer
//! area. By default the aerial engine divides every image by its exact
//! clear-field value `TCC(0,0)`, so a clear mask still images to 1; the raw
//! value is available from the engine's `clear_field_intensity`. With
//! `obliquity = false` and no film, `|M·J| = |J|`, so the raw clear field is
//! already 1.
//!
//! # Illumination states
//!
//! Each state is a real Jones vector with its incoherent weight folded in as
//! an amplitude:
//!
//! - `Unpolarized`: the pair `(1/√2, 0)`, `(0, 1/√2)`. The result does not
//!   depend on the basis, because the coherency matrix is `½·I`.
//! - `X`, `Y`, `Linear { angle_deg }`: one state `(cos a, sin a)`.
//! - `Te` (azimuthal) and `Tm` (radial): defined relative to the source point
//!   azimuth `φ_s = atan2(sy, sx)`, with `J_TE = (−sin φ_s, cos φ_s)` and
//!   `J_TM = (cos φ_s, sin φ_s)`.
//!
//! **On-axis fallback.** At the on-axis source point (`√(sx² + sy²) < 1e-9`)
//! the azimuth is undefined. A source sample there stands for a cell
//! symmetric about the axis, and averaging the TE (or TM) coherency matrix
//! over the azimuths of that cell gives `½·I`. That point is therefore
//! treated as unpolarized: an incoherent X/Y pair of amplitude `1/√2` each.
//! This is why TE/TM use two state slots (6 columns). At every off-axis
//! source point the second slot is exactly zero, and an engine may drop
//! all-zero columns.
//!
//! # Conventions
//!
//! - `(px, py)`: pupil coordinate in units of NA. `(sx, sy)`: source point in
//!   σ units. `z` is the optical axis, pointing toward the wafer.
//! - Sign of `E_z`: an order tilted toward +x carries a TM field
//!   `(cos θ, 0, −sin θ)`, which is the image of `x̂` under the rotation that
//!   takes `ẑ` to `k̂`. The TE/TM triad satisfies `ê_p × ê_s = k̂`.
//! - Film index `N = n + ik` with `k ≥ 0` for loss and time dependence
//!   `e^{−iωt}`. This is the storage convention of [`crate::thinfilm`]; that
//!   module conjugates internally for its Macleod matrices. The refracted
//!   wave obeys `Im(N cos θ₂) ≥ 0`, so it decays or propagates into the film.
//! - The reticle-side medium is air/vacuum (`n_o = 1`). Pupil-independent
//!   constants (`R`, `n_o/n_img`) are dropped, and the on-axis order has
//!   unit transfer.
//!
//! # Model status
//!
//! Implemented and validated against closed forms (module tests and
//! `tests/vector_pupil.rs`):
//! - TE/TM decomposition relative to each order's own plane of incidence;
//! - the radiometric obliquity factor, with its energy statement;
//! - the homogeneous image-medium index (immersion);
//! - film-entrance Fresnel transmission with complex Snell's law and the
//!   p-vector built from the complex refraction angle.
//!
//! Stated approximations:
//! - **Thin mask (Kirchhoff).** Every order leaves the mask with the
//!   illumination Jones vector. There are no mask-3D/EMF polarization effects.
//! - **Paraxial object side.** The p-vector's object-side tilt is neglected,
//!   so ê_p,in has no z component. The resulting amplitude ambiguity of the TM
//!   component is at most `1 − √(1 − (NA/R)²)`, which is 2.6 % at the pupil
//!   edge for NA 0.9 and R = 4.
//! - **Ideal lens.** There are no polarization aberrations, no lens or
//!   coating birefringence, and no diattenuation (no Jones pupil). Scalar
//!   aberrations and apodization enter through `scalar_pupil`.
//! - **Film entrance only.** The field is evaluated just inside the film's
//!   top surface. Multiple reflections, standing waves, and substrate
//!   reflection are not modeled here; [`crate::thinfilm`] handles them
//!   separately.
//!
//! # References
//!
//! - B. Richards and E. Wolf, "Electromagnetic diffraction in optical systems.
//!   II. Structure of the image field in an aplanatic system," Proc. R. Soc.
//!   Lond. A 253, 358–379 (1959): aplanatic `√cos θ` apodization.
//! - D. G. Flagello, T. Milster, A. E. Rosenbluth, "Theory of high-NA imaging
//!   in homogeneous thin films," J. Opt. Soc. Am. A 13 (1996): TE/TM pupil
//!   decomposition and film imaging for lithography.
//! - M. Born and E. Wolf, *Principles of Optics*: Fresnel formulae.

use serde::{Deserialize, Serialize};

use crate::error::{LithographyError, Result};
use crate::types::Complex64;

/// Source-point radius (σ units) below which the TE/TM illumination azimuth
/// is undefined and the on-axis (unpolarized) fallback applies.
const ON_AXIS_SIGMA: f64 = 1e-9;

/// Maximum number of mutually incoherent illumination states per source point.
const MAX_STATES: usize = 2;

/// Polarization state of the illumination at the mask.
///
/// Serialized as a table tagged by `type` (`{ type = "te" }`,
/// `{ type = "linear", angle_deg = 45.0 }`). Deserialization is strict:
/// unknown keys, an unknown `type`, or `angle_deg` on a non-linear state are
/// errors. `azimuthal` and `radial` are accepted as aliases of `te` and `tm`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case", try_from = "PolarizationRepr")]
pub enum IlluminationPolarization {
    /// Unpolarized: incoherent equal-weight sum of two orthogonal states
    /// (amplitude `1/√2` each).
    #[default]
    Unpolarized,
    /// Linear, electric field along x.
    X,
    /// Linear, electric field along y.
    Y,
    /// Transverse-electric (azimuthal) relative to each source point's
    /// plane of incidence: `J = (−sin φ_s, cos φ_s)`, `φ_s = atan2(sy, sx)`.
    /// The on-axis source point falls back to unpolarized (see the module
    /// docs).
    Te,
    /// Transverse-magnetic (radial) relative to each source point's plane of
    /// incidence: `J = (cos φ_s, sin φ_s)`. The on-axis source point falls
    /// back to unpolarized (see the module docs).
    Tm,
    /// Linear at `angle_deg` from the x axis.
    Linear { angle_deg: f64 },
}

/// Strict serde representation of [`IlluminationPolarization`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PolarizationRepr {
    #[serde(rename = "type")]
    kind: String,
    angle_deg: Option<f64>,
}

impl TryFrom<PolarizationRepr> for IlluminationPolarization {
    type Error = String;

    fn try_from(repr: PolarizationRepr) -> std::result::Result<Self, String> {
        use IlluminationPolarization as P;
        let state = match repr.kind.as_str() {
            "unpolarized" => P::Unpolarized,
            "x" => P::X,
            "y" => P::Y,
            "te" | "azimuthal" => P::Te,
            "tm" | "radial" => P::Tm,
            "linear" => {
                return repr
                    .angle_deg
                    .map(|angle_deg| P::Linear { angle_deg })
                    .ok_or_else(|| "polarization type \"linear\" requires `angle_deg`".into());
            }
            other => {
                return Err(format!(
                    "unknown polarization type \"{other}\"; expected one of: unpolarized, x, y, \
                     te (azimuthal), tm (radial), linear"
                ))
            }
        };
        if repr.angle_deg.is_some() {
            return Err(format!(
                "`angle_deg` is only valid for polarization type \"linear\", not \"{}\"",
                repr.kind
            ));
        }
        Ok(state)
    }
}

/// Optional thin-film entrance interface: the image is evaluated just inside
/// a film of complex index `n + i k` (e.g. the resist), with TE/TM Fresnel
/// transmission applied at its top surface. There are no multiple
/// reflections; standing waves are the job of [`crate::thinfilm`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilmInterface {
    /// Real part of the film refractive index.
    pub n: f64,
    /// Extinction coefficient of the film (`k ≥ 0` for loss; default 0).
    #[serde(default)]
    pub k: f64,
}

impl FilmInterface {
    /// Complex refractive index `n + i k`.
    pub fn index(&self) -> Complex64 {
        Complex64::new(self.n, self.k)
    }
}

/// Settings for vector imaging.
///
/// Every field has a default (`#[serde(default)]`), so a TOML table may set
/// only the fields it changes, e.g. `polarization = { type = "te" }`.
/// Unknown keys are rejected (`deny_unknown_fields`), so a misspelled key
/// errors instead of being silently ignored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VectorSettings {
    /// Illumination polarization.
    pub polarization: IlluminationPolarization,
    /// Refractive index of the (homogeneous) image medium; 1.0 = aerial
    /// image in air/vacuum, ≈ 1.44 for water immersion at 193 nm. Must
    /// exceed the image-side NA.
    ///
    /// The aerial engine has one image-space medium, shared by the defocus
    /// phase and these fields, and it belongs to the optics. Left at its
    /// default 1.0, this field inherits the optics' `immersion_index()`
    /// (e.g. from `ProjectionOptics::immersion`). Any other value must equal
    /// it, or the engine rejects the settings.
    pub image_index: f64,
    /// Apply the radiometric (obliquity) amplitude correction of a
    /// reduction imaging system, `(cos θ_obj / cos θ_img)^(1/2)`.
    pub obliquity: bool,
    /// Projection reduction ratio `R` (4.0 for a 4× scanner). Enters only
    /// the obliquity factor through `sin θ_obj = NA·|p|/R`.
    /// `f64::INFINITY` models an object at infinity. The dependence is weak:
    /// going from R = 4 to R = 5 changes the edge-of-pupil amplitude by
    /// 0.5 % at NA 0.9. The aerial engine replaces this value with the
    /// optics' `reduction()`, so setting it matters only for direct
    /// [`field_columns`] callers.
    pub reduction: f64,
    /// Optional film entrance interface (see [`FilmInterface`]).
    pub film: Option<FilmInterface>,
}

impl Default for VectorSettings {
    fn default() -> Self {
        Self {
            polarization: IlluminationPolarization::Unpolarized,
            image_index: 1.0,
            obliquity: true,
            reduction: 4.0,
            film: None,
        }
    }
}

impl VectorSettings {
    /// Check the settings against the image-side numerical aperture `na`.
    ///
    /// Rejects the following:
    /// - a non-positive or non-finite `na` or `image_index`;
    /// - `na ≥ image_index`, where edge orders could not propagate in the
    ///   image medium and would be silently zeroed;
    /// - `reduction ≤ na`, where the object-side NA `na/reduction` would
    ///   reach 1;
    /// - a film with non-positive or non-finite `n` or with `k < 0` (gain);
    /// - a non-finite linear polarization angle.
    pub fn validate(&self, na: f64) -> Result<()> {
        let invalid = |name, value, reason| {
            Err(LithographyError::InvalidParameter {
                name,
                value,
                reason,
            })
        };
        if !(na.is_finite() && na > 0.0) {
            return invalid("numerical_aperture", na, "must be finite and positive");
        }
        if !(self.image_index.is_finite() && self.image_index > 0.0) {
            return invalid(
                "image_index",
                self.image_index,
                "must be finite and positive",
            );
        }
        if na >= self.image_index {
            return invalid(
                "numerical_aperture",
                na,
                "must be below image_index: orders with NA·|p| ≥ n_img cannot propagate in the image medium",
            );
        }
        if self.reduction.is_nan() || self.reduction <= na {
            return invalid(
                "reduction",
                self.reduction,
                "must exceed the image-side NA (object-side NA = NA/reduction must stay below 1)",
            );
        }
        if let Some(film) = self.film {
            if !(film.n.is_finite() && film.n > 0.0) {
                return invalid("film.n", film.n, "must be finite and positive");
            }
            if !(film.k.is_finite() && film.k >= 0.0) {
                return invalid(
                    "film.k",
                    film.k,
                    "must be finite and non-negative (k ≥ 0 is loss)",
                );
            }
        }
        if let IlluminationPolarization::Linear { angle_deg } = self.polarization {
            if !angle_deg.is_finite() {
                return invalid("polarization.angle_deg", angle_deg, "must be finite");
            }
        }
        Ok(())
    }
}

/// Number of mutually incoherent illumination states (column triples):
/// 2 for unpolarized and for TE/TM (on-axis fallback), 1 for linear states.
fn state_count(polarization: IlluminationPolarization) -> usize {
    match polarization {
        IlluminationPolarization::Unpolarized
        | IlluminationPolarization::Te
        | IlluminationPolarization::Tm => 2,
        IlluminationPolarization::X
        | IlluminationPolarization::Y
        | IlluminationPolarization::Linear { .. } => 1,
    }
}

/// Number of `A`-matrix columns one source point contributes:
/// 3 field components × number of mutually incoherent polarization states.
/// That is 6 for [`IlluminationPolarization::Unpolarized`] and for `Te`/`Tm`,
/// and 3 for `X`, `Y`, and `Linear`. `Te`/`Tm` need the second state only at
/// the on-axis source point (unpolarized fallback); elsewhere its three
/// columns are exactly zero.
pub fn columns_per_source_point(settings: &VectorSettings) -> usize {
    3 * state_count(settings.polarization)
}

/// Real Jones vectors of the incoherent illumination states at source point
/// `(sx, sy)` (σ units), with the state weight folded in as amplitude.
/// Slots beyond [`state_count`] are zero.
fn illumination_states(
    polarization: IlluminationPolarization,
    sx: f64,
    sy: f64,
) -> [[f64; 2]; MAX_STATES] {
    let h = std::f64::consts::FRAC_1_SQRT_2;
    let unpolarized = [[h, 0.0], [0.0, h]];
    match polarization {
        IlluminationPolarization::Unpolarized => unpolarized,
        IlluminationPolarization::X => [[1.0, 0.0], [0.0, 0.0]],
        IlluminationPolarization::Y => [[0.0, 1.0], [0.0, 0.0]],
        IlluminationPolarization::Linear { angle_deg } => {
            let a = angle_deg.to_radians();
            [[a.cos(), a.sin()], [0.0, 0.0]]
        }
        IlluminationPolarization::Te | IlluminationPolarization::Tm => {
            let r = sx.hypot(sy);
            if r.is_nan() || r < ON_AXIS_SIGMA {
                // On-axis fallback: the azimuth-averaged coherency matrix is ½·I.
                return unpolarized;
            }
            let (c, s) = (sx / r, sy / r);
            let j = if polarization == IlluminationPolarization::Tm {
                [c, s]
            } else {
                [-s, c]
            };
            [j, [0.0, 0.0]]
        }
    }
}

/// TE/TM polarization transfer matrix `M` (3×2, rows `x, y, z`) of an ideal
/// aplanatic lens for an order with image-side direction sines `(α, β)`,
/// assuming a paraxial object side: `E = M·J`.
///
/// `M` keeps the TE component `(J·ê_s) ê_s` and rotates the TM component
/// onto `ê_p,out = (cos θ cos φ, cos θ sin φ, −sin θ)`. Its columns are
/// orthonormal (`MᵀM = I₂`), and `M = [I₂; 0]` on axis. Returns the zero
/// matrix for `α² + β² ≥ 1` (evanescent).
pub fn polarization_transfer_matrix(alpha: f64, beta: f64) -> [[f64; 2]; 3] {
    let sin2 = alpha * alpha + beta * beta;
    if sin2.is_nan() || sin2 >= 1.0 {
        return [[0.0; 2]; 3];
    }
    let inv = 1.0 / (1.0 + (1.0 - sin2).sqrt());
    [
        [1.0 - alpha * alpha * inv, -alpha * beta * inv],
        [-alpha * beta * inv, 1.0 - beta * beta * inv],
        [-alpha, -beta],
    ]
}

/// Radiometric (obliquity) amplitude factor of an aplanatic reduction lens,
/// `A = (cos θ_obj / cos θ_img)^(1/2)`, with `sin θ_img = pupil_na / image_index`
/// and `sin θ_obj = pupil_na / reduction` (reticle side in air/vacuum).
///
/// `pupil_na = NA·|p| = n_img sin θ_img` is the order's position in NA
/// units. `A = 1` on axis. Returns 0 when the order is evanescent on either
/// side. See the module docs for the energy-conservation derivation.
pub fn radiometric_factor(pupil_na: f64, image_index: f64, reduction: f64) -> f64 {
    let sin_img = pupil_na / image_index;
    let sin_obj = pupil_na / reduction;
    let (si2, so2) = (sin_img * sin_img, sin_obj * sin_obj);
    if si2.is_nan() || so2.is_nan() || si2 >= 1.0 || so2 >= 1.0 {
        return 0.0;
    }
    ((1.0 - so2) / (1.0 - si2)).sqrt().sqrt()
}

/// Fresnel amplitude coefficients of one planar interface, from a real
/// medium of index `n1` into a medium of complex index `n2 = n + ik`
/// (`k ≥ 0`), with complex Snell's law for the refracted wave.
///
/// `t_s`/`t_p` are the ratios of transmitted to incident field amplitude
/// along `ê_s` and along the p-vectors `(cos θ, 0, −sin θ)` (incident) and
/// `(cos θ₂, 0, −sin θ₂)` (refracted, complex angle). `r_p` refers to the
/// reflected p-vector `(−cos θ₁, 0, −sin θ₁)`, so `r_p = −r_s` at normal
/// incidence. The local frame has x along the plane of incidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FresnelCoefficients {
    /// Index of the incidence medium.
    pub n1: f64,
    /// Complex index of the transmission medium.
    pub n2: Complex64,
    /// Sine of the (real) incidence angle.
    pub sin_theta1: f64,
    /// Cosine of the incidence angle.
    pub cos_theta1: f64,
    /// Sine of the complex refraction angle, `n1 sin θ₁ / n2`.
    pub sin_theta2: Complex64,
    /// Cosine of the complex refraction angle, with `Im(n2 cos θ₂) ≥ 0`.
    pub cos_theta2: Complex64,
    /// TE (s) amplitude reflection coefficient.
    pub r_s: Complex64,
    /// TM (p) amplitude reflection coefficient.
    pub r_p: Complex64,
    /// TE (s) amplitude transmission coefficient.
    pub t_s: Complex64,
    /// TM (p) amplitude transmission coefficient.
    pub t_p: Complex64,
}

impl FresnelCoefficients {
    /// Evaluate the Fresnel coefficients at incidence `sin θ₁ ∈ [0, 1)`.
    pub fn new(n1: f64, n2: Complex64, sin_theta1: f64) -> Self {
        let cos1 = (1.0 - sin_theta1 * sin_theta1).sqrt();
        let one = Complex64::new(1.0, 0.0);
        // Longitudinal wavenumber in the film (units of k0): N cos θ₂.
        let (kz2, sin2, cos2) = if sin_theta1 == 0.0 {
            (n2, Complex64::new(0.0, 0.0), one)
        } else {
            let nt = n1 * sin_theta1;
            let mut kz2 = (n2 * n2 - nt * nt).sqrt();
            if kz2.im < 0.0 {
                kz2 = -kz2;
            }
            (kz2, nt / n2, kz2 / n2)
        };
        let n1c = n1 * cos1;
        let t_s = 2.0 * n1c / (n1c + kz2);
        let r_s = (n1c - kz2) / (n1c + kz2);
        let den_p = n2 * cos1 + n1 * cos2;
        let t_p = 2.0 * n1c / den_p;
        let r_p = (n2 * cos1 - n1 * cos2) / den_p;
        Self {
            n1,
            n2,
            sin_theta1,
            cos_theta1: cos1,
            sin_theta2: sin2,
            cos_theta2: cos2,
            r_s,
            r_p,
            t_s,
            t_p,
        }
    }

    /// TE power reflectance `|r_s|²`.
    pub fn reflectance_s(&self) -> f64 {
        self.r_s.norm_sqr()
    }

    /// TM power reflectance `|r_p|²`.
    pub fn reflectance_p(&self) -> f64 {
        self.r_p.norm_sqr()
    }

    /// TE power transmittance (z-directed Poynting flux just inside medium
    /// 2 over incident flux): `Re(n2 cos θ₂) |t_s|² / (n1 cos θ₁)`.
    pub fn transmittance_s(&self) -> f64 {
        (self.n2 * self.cos_theta2).re * self.t_s.norm_sqr() / (self.n1 * self.cos_theta1)
    }

    /// TM power transmittance: `Re(n2* cos θ₂) |t_p|² / (n1 cos θ₁)`. The
    /// conjugate matters only for an absorbing medium 2.
    pub fn transmittance_p(&self) -> f64 {
        (self.n2.conj() * self.cos_theta2).re * self.t_p.norm_sqr() / (self.n1 * self.cos_theta1)
    }
}

/// Complex 3×2 transfer matrix (rows `x, y, z`) from the illumination Jones
/// vector to the image-side field for direction sines `(α, β)` with
/// `sin²θ = α² + β² < 1`, optionally just inside a film.
fn transfer_matrix(
    alpha: f64,
    beta: f64,
    image_index: f64,
    film: Option<FilmInterface>,
) -> [[Complex64; 2]; 3] {
    let c = |v: f64| Complex64::new(v, 0.0);
    match film {
        None => {
            let m = polarization_transfer_matrix(alpha, beta);
            [
                [c(m[0][0]), c(m[0][1])],
                [c(m[1][0]), c(m[1][1])],
                [c(m[2][0]), c(m[2][1])],
            ]
        }
        Some(film) => {
            // E = t_s J + (t_p cos θ₂ − t_s)(J·û) û − t_p sin θ₂ (J·û) ẑ, where
            // û is the order's azimuth unit vector (any unit vector on axis,
            // where t_p cos θ₂ = t_s exactly and sin θ₂ = 0).
            let s = alpha.hypot(beta);
            let (ux, uy) = if s > 0.0 {
                (alpha / s, beta / s)
            } else {
                (1.0, 0.0)
            };
            let f = FresnelCoefficients::new(image_index, film.index(), s);
            let d = f.t_p * f.cos_theta2 - f.t_s;
            let tz = -f.t_p * f.sin_theta2;
            [
                [f.t_s + d * (ux * ux), d * (ux * uy)],
                [d * (ux * uy), f.t_s + d * (uy * uy)],
                [tz * ux, tz * uy],
            ]
        }
    }
}

/// Fill `out` (length ≥ [`columns_per_source_point`]) with the image-side
/// electric-field amplitudes `(E_x, E_y, E_z)` of one plane-wave order, for
/// each incoherent polarization state in turn (`out[3k + c]`), with the state
/// weights folded in as amplitudes (e.g. `1/√2` each for unpolarized light).
///
/// - `scalar_pupil`: the optics' scalar pupil value at this order
///   (aberration, defocus, apodization); multiplies every component.
/// - `px, py`: pupil coordinate of the order, normalized to NA (`|p| ≤ 1`).
///   Orders outside the pupil or evanescent in the image medium
///   (`NA·|p| ≥ n_img`) get zero field.
/// - `sx, sy`: the source point in σ units; defines the TE/TM illumination
///   basis (on-axis fallback: unpolarized).
/// - `na`: image-side numerical aperture.
///
/// The field includes the TE/TM rotation relative to the order's plane of
/// incidence, the radiometric factor if `settings.obliquity`, the image
/// medium index, and the film-entrance Fresnel transmission if
/// `settings.film` is set. See the module docs for the equations.
///
/// # Panics
/// Panics if `out` is shorter than `columns_per_source_point(settings)`.
#[allow(clippy::too_many_arguments)]
pub fn field_columns(
    scalar_pupil: Complex64,
    px: f64,
    py: f64,
    sx: f64,
    sy: f64,
    na: f64,
    settings: &VectorSettings,
    out: &mut [Complex64],
) {
    let n_states = state_count(settings.polarization);
    let out = &mut out[..3 * n_states];
    let zero = Complex64::new(0.0, 0.0);
    out.fill(zero);

    let rho2 = px * px + py * py;
    if rho2.is_nan() || rho2 > 1.0 || scalar_pupil == zero {
        return;
    }
    let n_img = settings.image_index;
    let alpha = na * px / n_img;
    let beta = na * py / n_img;
    let sin2 = alpha * alpha + beta * beta;
    if sin2.is_nan() || sin2 >= 1.0 {
        return; // evanescent (or grazing) in the image medium
    }

    let mut amp = scalar_pupil;
    if settings.obliquity {
        let a = radiometric_factor(na * rho2.sqrt(), n_img, settings.reduction);
        if a == 0.0 {
            return;
        }
        amp *= a;
    }

    let m = transfer_matrix(alpha, beta, n_img, settings.film);
    let states = illumination_states(settings.polarization, sx, sy);
    for (k, j) in states.iter().take(n_states).enumerate() {
        if j[0] == 0.0 && j[1] == 0.0 {
            continue;
        }
        for (c, row) in m.iter().enumerate() {
            out[3 * k + c] = amp * (row[0] * j[0] + row[1] * j[1]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    const TOL: f64 = 1e-12;

    fn c(re: f64, im: f64) -> Complex64 {
        Complex64::new(re, im)
    }

    /// Explicit angle form ê_s ê_sᵀ + ê_p,out ê_p,inᵀ (independent route).
    fn angle_form(alpha: f64, beta: f64) -> [[f64; 2]; 3] {
        let s = alpha.hypot(beta);
        let theta = s.asin();
        let phi = if s > 0.0 { beta.atan2(alpha) } else { 0.0 };
        let es = [-phi.sin(), phi.cos(), 0.0];
        let epin = [phi.cos(), phi.sin()];
        let epout = [
            theta.cos() * phi.cos(),
            theta.cos() * phi.sin(),
            -theta.sin(),
        ];
        let mut m = [[0.0; 2]; 3];
        for (r, row) in m.iter_mut().enumerate() {
            for (col, v) in row.iter_mut().enumerate() {
                *v = es[r] * es[col] + epout[r] * epin[col];
            }
        }
        m
    }

    #[test]
    fn transfer_matrix_matches_angle_form() {
        for &(a, b) in &[
            (0.3, -0.2),
            (-0.7, 0.5),
            (0.0, 0.94),
            (0.61, 0.0),
            (-0.4, -0.8),
        ] {
            let m = polarization_transfer_matrix(a, b);
            let e = angle_form(a, b);
            for r in 0..3 {
                for col in 0..2 {
                    assert_relative_eq!(m[r][col], e[r][col], epsilon = TOL);
                }
            }
        }
    }

    #[test]
    fn transfer_matrix_columns_orthonormal_and_projector() {
        for &(a, b) in &[(0.1, 0.2), (0.6, -0.7), (-0.99, 0.1), (0.0, -0.3)] {
            let m = polarization_transfer_matrix(a, b);
            // MᵀM = I₂
            for i in 0..2 {
                for j in 0..2 {
                    let dot: f64 = (0..3).map(|r| m[r][i] * m[r][j]).sum();
                    assert_relative_eq!(dot, if i == j { 1.0 } else { 0.0 }, epsilon = TOL);
                }
            }
            // M·Mᵀ = I₃ − k̂k̂ᵀ (projector onto the plane transverse to k̂)
            let k = [a, b, (1.0 - a * a - b * b).sqrt()];
            for i in 0..3 {
                for j in 0..3 {
                    let mm: f64 = (0..2).map(|col| m[i][col] * m[j][col]).sum();
                    let want = if i == j { 1.0 } else { 0.0 } - k[i] * k[j];
                    assert_relative_eq!(mm, want, epsilon = TOL);
                }
            }
        }
    }

    #[test]
    fn transfer_matrix_identity_on_axis_and_zero_when_evanescent() {
        assert_eq!(
            polarization_transfer_matrix(0.0, 0.0),
            [[1.0, 0.0], [0.0, 1.0], [0.0, 0.0]]
        );
        assert_eq!(polarization_transfer_matrix(0.8, 0.6), [[0.0; 2]; 3]);
        assert_eq!(polarization_transfer_matrix(f64::NAN, 0.0), [[0.0; 2]; 3]);
    }

    #[test]
    fn radiometric_factor_fixtures_and_limits() {
        // Fixtures from an independent numpy evaluation.
        assert_relative_eq!(
            radiometric_factor(0.9, 1.0, 4.0),
            1.495_102_774_997_347_3,
            epsilon = TOL
        );
        assert_relative_eq!(
            radiometric_factor(1.35, 1.44, 4.0),
            1.644_710_580_224_799_7,
            epsilon = TOL
        );
        // Object at infinity: angular-spectrum form of Richards–Wolf, 1/√cos θ.
        let a = radiometric_factor(0.5, 1.0, f64::INFINITY);
        assert_relative_eq!(a, 1.0 / (0.75_f64).sqrt().sqrt(), epsilon = TOL);
        assert_relative_eq!(a, 1.074_569_931_823_542, epsilon = TOL);
        // Unit on axis; symmetric 1:1 system in a common medium → no correction.
        assert_eq!(radiometric_factor(0.0, 1.0, 4.0), 1.0);
        for &pna in &[0.1, 0.5, 0.9] {
            assert_relative_eq!(radiometric_factor(pna, 1.0, 1.0), 1.0, epsilon = TOL);
        }
        // Evanescent on the image side → 0.
        assert_eq!(radiometric_factor(1.0, 1.0, 4.0), 0.0);
        assert_eq!(radiometric_factor(0.9, 1.0, 0.5), 0.0);
    }

    #[test]
    fn fresnel_matches_boundary_condition_solve() {
        // Reference values: numpy solve of the tangential E/H boundary
        // conditions (not the closed forms used here).
        let f = FresnelCoefficients::new(1.0, c(1.7, 0.03), 0.6);
        assert_relative_eq!(f.t_s.re, 0.669_157_150_517_369_1, epsilon = TOL);
        assert_relative_eq!(f.t_s.im, -0.008_974_562_862_638_537, epsilon = TOL);
        assert_relative_eq!(f.t_p.re, 0.696_860_163_202_311_3, epsilon = TOL);
        assert_relative_eq!(f.t_p.im, -0.007_997_848_995_639_045, epsilon = TOL);

        let f = FresnelCoefficients::new(1.0, c(0.9, 0.2), 0.7);
        assert_relative_eq!(f.t_s.re, 1.029_428_805_389_820_6, epsilon = TOL);
        assert_relative_eq!(f.t_s.im, -0.231_252_565_665_561_16, epsilon = TOL);
        assert_relative_eq!(f.t_p.re, 1.000_360_235_835_709_5, epsilon = TOL);
        assert_relative_eq!(f.t_p.im, -0.232_018_589_122_350_18, epsilon = TOL);

        // n2 < n1 sin θ₁ with k = 0: evanescent refracted wave (Im cos θ₂ > 0).
        let f = FresnelCoefficients::new(1.0, c(0.95, 0.0), 0.99);
        assert_relative_eq!(f.t_s.re, 0.408_205_128_205_127_6, epsilon = 1e-11);
        assert_relative_eq!(f.t_s.im, -0.806_088_599_173_372_8, epsilon = 1e-11);
        assert!(f.cos_theta2.im > 0.0);
        assert_relative_eq!(f.transmittance_s(), 0.0, epsilon = TOL);
        assert_relative_eq!(f.reflectance_s(), 1.0, epsilon = 1e-12);
    }

    #[test]
    fn fresnel_energy_balance() {
        for &n2 in &[
            c(1.7, 0.0),
            c(1.7, 0.03),
            c(1.44, 0.0),
            c(0.9, 0.2),
            c(2.5, 1.1),
        ] {
            for &s1 in &[0.0, 0.2, 0.5, 0.8, 0.95, 0.999] {
                let f = FresnelCoefficients::new(1.0, n2, s1);
                assert_relative_eq!(
                    f.reflectance_s() + f.transmittance_s(),
                    1.0,
                    epsilon = 1e-12
                );
                assert_relative_eq!(
                    f.reflectance_p() + f.transmittance_p(),
                    1.0,
                    epsilon = 1e-12
                );
                // Passive film: refracted wave never grows into the film.
                assert!((n2 * f.cos_theta2).im >= 0.0);
            }
        }
    }

    #[test]
    fn fresnel_normal_incidence_and_brewster() {
        let n2 = c(1.7, 0.03);
        let f = FresnelCoefficients::new(1.0, n2, 0.0);
        let t = 2.0 / (1.0 + n2);
        assert_relative_eq!((f.t_s - t).norm(), 0.0, epsilon = TOL);
        assert_relative_eq!((f.t_p - t).norm(), 0.0, epsilon = TOL);
        assert_relative_eq!((f.r_p + f.r_s).norm(), 0.0, epsilon = TOL);

        // Brewster: tan θ_B = n2/n1 → r_p = 0, t_p = n1/n2, T_p = 1.
        for &(n1, n2r) in &[(1.0_f64, 1.7_f64), (1.44, 1.7), (1.0, 1.5)] {
            let sb = (n2r / n1).atan().sin();
            let f = FresnelCoefficients::new(n1, c(n2r, 0.0), sb);
            assert_relative_eq!(f.r_p.norm(), 0.0, epsilon = TOL);
            assert_relative_eq!(f.t_p.re, n1 / n2r, epsilon = TOL);
            assert_relative_eq!(f.transmittance_p(), 1.0, epsilon = TOL);
            assert!(f.reflectance_s() > 0.01);
        }
    }

    #[test]
    fn illumination_states_bases() {
        let h = std::f64::consts::FRAC_1_SQRT_2;
        use IlluminationPolarization as P;
        assert_eq!(
            illumination_states(P::X, 0.3, 0.1),
            [[1.0, 0.0], [0.0, 0.0]]
        );
        assert_eq!(
            illumination_states(P::Unpolarized, 0.3, 0.1),
            [[h, 0.0], [0.0, h]]
        );
        // TE/TM relative to the source azimuth; orthonormal pair.
        let (sx, sy) = (0.3, -0.4);
        let te = illumination_states(P::Te, sx, sy)[0];
        let tm = illumination_states(P::Tm, sx, sy)[0];
        assert_relative_eq!(te[0], 0.8, epsilon = TOL);
        assert_relative_eq!(te[1], 0.6, epsilon = TOL);
        assert_relative_eq!(tm[0], 0.6, epsilon = TOL);
        assert_relative_eq!(tm[1], -0.8, epsilon = TOL);
        assert_relative_eq!(te[0] * tm[0] + te[1] * tm[1], 0.0, epsilon = TOL);
        assert_eq!(illumination_states(P::Te, sx, sy)[1], [0.0, 0.0]);
        // On-axis fallback: unpolarized pair.
        for pol in [P::Te, P::Tm] {
            assert_eq!(illumination_states(pol, 0.0, 0.0), [[h, 0.0], [0.0, h]]);
            assert_eq!(
                illumination_states(pol, 1e-17, -1e-16),
                [[h, 0.0], [0.0, h]]
            );
        }
        let lin = illumination_states(P::Linear { angle_deg: 30.0 }, 0.0, 0.0)[0];
        assert_relative_eq!(lin[0], 3.0_f64.sqrt() / 2.0, epsilon = TOL);
        assert_relative_eq!(lin[1], 0.5, epsilon = TOL);
    }

    #[test]
    fn columns_per_source_point_counts() {
        use IlluminationPolarization as P;
        let cols = |polarization| {
            columns_per_source_point(&VectorSettings {
                polarization,
                ..Default::default()
            })
        };
        assert_eq!(cols(P::Unpolarized), 6);
        assert_eq!(cols(P::Te), 6);
        assert_eq!(cols(P::Tm), 6);
        assert_eq!(cols(P::X), 3);
        assert_eq!(cols(P::Y), 3);
        assert_eq!(cols(P::Linear { angle_deg: 10.0 }), 3);
    }

    #[test]
    fn field_columns_leaves_unused_tail_untouched_and_zero_fills() {
        let settings = VectorSettings {
            polarization: IlluminationPolarization::X,
            ..Default::default()
        };
        let sentinel = c(7.0, 7.0);
        let mut out = [sentinel; 6];
        field_columns(c(1.0, 0.0), 2.0, 0.0, 0.0, 0.0, 0.9, &settings, &mut out);
        assert!(out[..3].iter().all(|v| *v == c(0.0, 0.0)));
        assert!(out[3..].iter().all(|v| *v == sentinel));
    }

    #[test]
    fn validate_accepts_defaults_and_rejects_bad_settings() {
        let ok = VectorSettings::default();
        assert!(ok.validate(0.9).is_ok());
        let immersion = VectorSettings {
            image_index: 1.44,
            ..Default::default()
        };
        assert!(immersion.validate(1.35).is_ok());
        assert!(VectorSettings {
            reduction: f64::INFINITY,
            ..Default::default()
        }
        .validate(0.9)
        .is_ok());

        assert!(ok.validate(1.0).is_err()); // NA ≥ n_img
        assert!(ok.validate(0.0).is_err());
        assert!(ok.validate(f64::NAN).is_err());
        let bad = |s: VectorSettings| s.validate(0.9).is_err();
        assert!(bad(VectorSettings {
            image_index: 0.0,
            ..Default::default()
        }));
        assert!(bad(VectorSettings {
            reduction: 0.5,
            ..Default::default()
        }));
        assert!(bad(VectorSettings {
            reduction: f64::NAN,
            ..Default::default()
        }));
        assert!(bad(VectorSettings {
            film: Some(FilmInterface { n: 1.7, k: -0.1 }),
            ..Default::default()
        }));
        assert!(bad(VectorSettings {
            film: Some(FilmInterface { n: 0.0, k: 0.0 }),
            ..Default::default()
        }));
        assert!(bad(VectorSettings {
            polarization: IlluminationPolarization::Linear {
                angle_deg: f64::INFINITY
            },
            ..Default::default()
        }));
    }
}
