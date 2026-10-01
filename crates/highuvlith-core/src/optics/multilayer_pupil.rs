//! Angle-dependent multilayer-mirror response across a reflective optic's
//! pupil (optional apodization and phase for EUV/BEUV mirror systems).
//!
//! Every ray through the pupil reflects off each multilayer-coated mirror at
//! its own angle of incidence, and a Bragg multilayer's complex reflectance
//! depends strongly on that angle. This module maps a wafer-side pupil
//! coordinate `p` to an incidence angle on every mirror, looks up the
//! coating's s and p amplitude reflectances (from
//! [`crate::materials::multilayer::MultilayerMirror`], Parratt recursion)
//! and returns the system's scalar pupil amplitude and phase.
//!
//! # Key equations
//!
//! ```text
//!   θ_j(p) = | θ_c,j + t_j · (p · u_j) + g_j · |p|² |     (deg; u_j = (cos a_j, sin a_j))
//!   R_s(p) = Π_j r_s(λ, θ_j(p)),   R_p(p) = Π_j r_p(λ, θ_j(p))
//!   M(p)   = √((|R_s|² + |R_p|²)/2) · exp(i·arg(R_s + R_p))
//! ```
//! `p` is the pupil coordinate normalized to NA (|p| ≤ 1); `θ_c` is the
//! incidence angle at the pupil centre, `t` a linear tilt across the pupil
//! along azimuth `a` (the mirror's plane of incidence), `g` a rotationally
//! symmetric quadratic term. `|M|²` is the unpolarized intensity
//! transmission of the mirror train along that ray; `arg M` its phase.
//!
//! # Model status
//!
//! Simplified (🔶), opt-in (optics default to a scalar-constant reflectivity):
//! - The incidence-angle maps are **user-supplied** first/second-order
//!   models; real angle maps come from ray-tracing the actual design.
//! - All mirrors share one s/p basis per ray (coaxial system) and one
//!   coating; the scalar amplitude is the polarization average above —
//!   polarization-resolved mirror effects (r_s ≠ r_p acting separately on
//!   the TE/TM field components) are not applied, even in vector mode.
//! - The response is tabulated once per wavelength on 0°–60° in 0.02°
//!   steps and interpolated with cubic (Catmull–Rom) splines, mirrored at 0°
//!   where the response is even in θ (measured relative error ≲ 1e-6 for
//!   Mo/Si up to 30°); angles beyond 60° clamp to the last entry.
//!
//! With the engine's default clear-field normalization the overall
//! reflectance level cancels: the angle dependence appears as pupil
//! apodization and phase (image shape), not as a dose change.

use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::error::{LithographyError, Result};
use crate::materials::multilayer::MultilayerMirror;
use crate::types::Complex64;

/// Angular sampling of the response table (deg).
const TABLE_STEP_DEG: f64 = 0.02;
/// Largest tabulated incidence angle (deg).
const TABLE_MAX_DEG: f64 = 60.0;

/// Incidence-angle map of one mirror over the wafer-side pupil:
/// `θ(p) = |center_deg + tilt_deg · (p·u) + radial_deg · |p|²|`,
/// `u = (cos azimuth, sin azimuth)`, `p` normalized to NA.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MirrorAngleMap {
    /// Incidence angle at the pupil centre (deg).
    pub center_deg: f64,
    /// Linear change of the incidence angle from the pupil centre to the
    /// pupil edge along `azimuth_deg` (deg; the opposite edge gets −tilt).
    pub tilt_deg: f64,
    /// Pupil azimuth of the tilt direction (deg).
    pub azimuth_deg: f64,
    /// Rotationally symmetric change at the pupil edge (deg; ∝ |p|²).
    pub radial_deg: f64,
}

impl MirrorAngleMap {
    /// Constant incidence angle `deg` across the pupil.
    pub fn constant(deg: f64) -> Self {
        Self {
            center_deg: deg,
            tilt_deg: 0.0,
            azimuth_deg: 0.0,
            radial_deg: 0.0,
        }
    }

    /// Incidence angle (deg, ≥ 0) at pupil coordinate `(px, py)`.
    pub fn angle_deg(&self, px: f64, py: f64) -> f64 {
        let a = self.azimuth_deg.to_radians();
        (self.center_deg
            + self.tilt_deg * (px * a.cos() + py * a.sin())
            + self.radial_deg * (px * px + py * py))
            .abs()
    }
}

/// Per-wavelength table of the coating's s and p amplitude reflectances.
#[derive(Debug)]
struct ResponseTable {
    rs: Vec<Complex64>,
    rp: Vec<Complex64>,
}

impl ResponseTable {
    fn build(coating: &MultilayerMirror, wavelength_nm: f64) -> Result<Self> {
        let n = (TABLE_MAX_DEG / TABLE_STEP_DEG).round() as usize + 1;
        let mut rs = Vec::with_capacity(n);
        let mut rp = Vec::with_capacity(n);
        for i in 0..n {
            let r =
                coating.pupil_response(wavelength_nm, (i as f64 * TABLE_STEP_DEG).to_radians())?;
            rs.push(r.rs);
            rp.push(r.rp);
        }
        Ok(Self { rs, rp })
    }

    /// Cubic (Catmull–Rom) interpolation at `deg` (clamped to the table;
    /// the stencil is mirrored at 0°, where the response is even in θ).
    fn at(&self, deg: f64) -> (Complex64, Complex64) {
        let last = self.rs.len() - 1;
        let x = (deg / TABLE_STEP_DEG).clamp(0.0, last as f64);
        let i = (x.floor() as usize).min(last - 1);
        let t = x - i as f64;
        let idx = |k: isize| -> usize { (i as isize + k).unsigned_abs().min(last) };
        let (t2, t3) = (t * t, t * t * t);
        let w = [
            0.5 * (-t3 + 2.0 * t2 - t),
            0.5 * (3.0 * t3 - 5.0 * t2 + 2.0),
            0.5 * (-3.0 * t3 + 4.0 * t2 + t),
            0.5 * (t3 - t2),
        ];
        let mut out = (Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0));
        for (k, wk) in (-1..=2).zip(w) {
            let j = idx(k);
            out.0 += self.rs[j] * wk;
            out.1 += self.rp[j] * wk;
        }
        out
    }
}

/// Multilayer-coated mirror train of a reflective optic, as seen from the
/// wafer-side pupil. Attach it to `SchwarzschildObjective::multilayer` or
/// `EuvProjectionOptics::multilayer`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultilayerPupil {
    /// Coating of every mirror.
    pub coating: MultilayerMirror,
    /// Incidence-angle map of each mirror, in order.
    pub mirrors: Vec<MirrorAngleMap>,
    /// Response tables keyed by wavelength bits (built on demand, shared by
    /// clones).
    #[serde(skip)]
    tables: TableCache,
}

/// Wavelength-keyed response tables shared by clones of a [`MultilayerPupil`].
type TableCache = Arc<RwLock<Vec<(u64, Arc<ResponseTable>)>>>;

impl MultilayerPupil {
    /// Mirror train with the given coating and per-mirror angle maps.
    pub fn new(coating: MultilayerMirror, mirrors: Vec<MirrorAngleMap>) -> Result<Self> {
        if mirrors.is_empty() {
            return Err(LithographyError::InvalidParameter {
                name: "mirrors",
                value: 0.0,
                reason: "a multilayer pupil needs at least one mirror",
            });
        }
        for m in &mirrors {
            for v in [m.center_deg, m.tilt_deg, m.azimuth_deg, m.radial_deg] {
                if !v.is_finite() {
                    return Err(LithographyError::InvalidParameter {
                        name: "mirror angle map",
                        value: v,
                        reason: "angles must be finite",
                    });
                }
            }
        }
        Ok(Self {
            coating,
            mirrors,
            tables: Arc::default(),
        })
    }

    fn table(&self, wavelength_nm: f64) -> Result<Arc<ResponseTable>> {
        let key = wavelength_nm.to_bits();
        {
            let tables = self.tables.read().unwrap_or_else(|e| e.into_inner());
            if let Some((_, t)) = tables.iter().find(|(k, _)| *k == key) {
                return Ok(Arc::clone(t));
            }
        }
        let built = Arc::new(ResponseTable::build(&self.coating, wavelength_nm)?);
        let mut tables = self.tables.write().unwrap_or_else(|e| e.into_inner());
        if let Some((_, t)) = tables.iter().find(|(k, _)| *k == key) {
            return Ok(Arc::clone(t));
        }
        tables.push((key, Arc::clone(&built)));
        Ok(built)
    }

    /// Tabulate the coating response at `wavelength_nm` (errors when the
    /// optical constants are unavailable there). Called by the imaging
    /// engine before it samples the pupil.
    pub fn prepare(&self, wavelength_nm: f64) -> Result<()> {
        self.table(wavelength_nm).map(|_| ())
    }

    /// System s and p amplitudes `(R_s, R_p)` at pupil coordinate `(px, py)`.
    ///
    /// # Panics
    /// Panics if the coating response cannot be computed at
    /// `wavelength_nm` (call [`Self::prepare`] first to get an error).
    pub fn system_amplitudes(
        &self,
        px: f64,
        py: f64,
        wavelength_nm: f64,
    ) -> (Complex64, Complex64) {
        let table = self
            .table(wavelength_nm)
            .unwrap_or_else(|e| panic!("multilayer pupil response at {wavelength_nm} nm: {e}"));
        let one = Complex64::new(1.0, 0.0);
        self.mirrors.iter().fold((one, one), |(s, p), m| {
            let (rs, rp) = table.at(m.angle_deg(px, py));
            (s * rs, p * rp)
        })
    }

    /// Scalar pupil amplitude `M(p) = √((|R_s|² + |R_p|²)/2)·exp(i·arg(R_s + R_p))`.
    ///
    /// # Panics
    /// As [`Self::system_amplitudes`].
    pub fn amplitude(&self, px: f64, py: f64, wavelength_nm: f64) -> Complex64 {
        let (s, p) = self.system_amplitudes(px, py, wavelength_nm);
        let magnitude = (0.5 * (s.norm_sqr() + p.norm_sqr())).sqrt();
        Complex64::from_polar(magnitude, (s + p).arg())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Polarization;
    use approx::assert_relative_eq;

    fn mo_si() -> MultilayerMirror {
        MultilayerMirror::mo_si(40, 6.9, 0.4).unwrap()
    }

    #[test]
    fn angle_map_geometry() {
        let m = MirrorAngleMap {
            center_deg: 5.0,
            tilt_deg: 2.0,
            azimuth_deg: 90.0,
            radial_deg: 1.5,
        };
        assert_relative_eq!(m.angle_deg(0.0, 0.0), 5.0);
        assert_relative_eq!(m.angle_deg(0.0, 1.0), 5.0 + 2.0 + 1.5, epsilon = 1e-12);
        assert_relative_eq!(m.angle_deg(0.0, -1.0), 5.0 - 2.0 + 1.5, epsilon = 1e-12);
        assert_relative_eq!(m.angle_deg(1.0, 0.0), 5.0 + 1.5, epsilon = 1e-12);
        // Negative values fold back (angle of incidence is |θ|).
        assert_relative_eq!(MirrorAngleMap::constant(-3.0).angle_deg(0.2, 0.3), 3.0);
    }

    #[test]
    fn normal_incidence_two_mirror_transmission() {
        // At normal incidence r_s = r_p (tangential-E convention), so the
        // two-mirror intensity transmission is R(0°)² exactly.
        let coating = mo_si();
        let r0 = coating.pupil_response(13.5, 0.0).unwrap();
        assert!((r0.rs - r0.rp).norm() < 1e-12 * r0.rs.norm());
        let r_direct = coating
            .reflectance(13.5, 0.0, Polarization::Unpolarized)
            .unwrap();
        let pupil = MultilayerPupil::new(
            coating,
            vec![MirrorAngleMap::constant(0.0), MirrorAngleMap::constant(0.0)],
        )
        .unwrap();
        let m = pupil.amplitude(0.3, -0.2, 13.5);
        assert_relative_eq!(m.norm_sqr(), r_direct * r_direct, max_relative = 1e-12);
        assert_relative_eq!(m.arg(), (r0.rs * r0.rs).arg(), epsilon = 1e-12);
        // Ideal Mo/Si at 13.5 nm: R ≈ 0.729 (E1 fixture).
        assert!((r_direct - 0.729).abs() < 2e-3, "R(0°) = {r_direct}");
    }

    #[test]
    fn interpolated_response_matches_direct_evaluation() {
        let coating = mo_si();
        let theta: f64 = 7.337;
        let direct = coating.pupil_response(13.5, theta.to_radians()).unwrap();
        let pupil = MultilayerPupil::new(coating, vec![MirrorAngleMap::constant(theta)]).unwrap();
        let (rs, rp) = pupil.system_amplitudes(0.0, 0.0, 13.5);
        assert!((rs - direct.rs).norm() < 2e-6 * direct.rs.norm());
        assert!((rp - direct.rp).norm() < 2e-6 * direct.rp.norm());
        // Grid points are reproduced exactly; the stencil is mirrored at 0°.
        let at0 = MultilayerPupil::new(mo_si(), vec![MirrorAngleMap::constant(0.01)]).unwrap();
        let r = mo_si().pupil_response(13.5, 0.01f64.to_radians()).unwrap();
        assert!((at0.system_amplitudes(0.0, 0.0, 13.5).0 - r.rs).norm() < 2e-6 * r.rs.norm());
    }

    #[test]
    fn steep_angles_apodize_the_pupil_edge() {
        // Mo/Si acceptance ≈ ±11° (s): 0° at the centre → 20° at the edge
        // on each of two mirrors strongly darkens the pupil rim.
        let pupil = MultilayerPupil::new(
            mo_si(),
            vec![
                MirrorAngleMap {
                    radial_deg: 20.0,
                    ..MirrorAngleMap::constant(0.0)
                };
                2
            ],
        )
        .unwrap();
        let centre = pupil.amplitude(0.0, 0.0, 13.5).norm_sqr();
        let mid = pupil.amplitude(0.5, 0.0, 13.5).norm_sqr();
        let edge = pupil.amplitude(1.0, 0.0, 13.5).norm_sqr();
        assert!(centre > mid && mid > edge);
        assert!(edge < 0.05 * centre, "edge {edge} vs centre {centre}");
    }

    #[test]
    fn validation_prepare_and_serde() {
        assert!(MultilayerPupil::new(mo_si(), vec![]).is_err());
        assert!(MultilayerPupil::new(mo_si(), vec![MirrorAngleMap::constant(f64::NAN)]).is_err());
        let pupil = MultilayerPupil::new(mo_si(), vec![MirrorAngleMap::constant(4.0)]).unwrap();
        assert!(pupil.prepare(13.5).is_ok());
        // Far outside the tabulated optical constants.
        assert!(pupil.prepare(2000.0).is_err());
        let text = toml::to_string(&pupil).unwrap();
        let back: MultilayerPupil = toml::from_str(&text).unwrap();
        assert_eq!(back.mirrors, pupil.mirrors);
        assert_eq!(
            back.amplitude(0.1, 0.1, 13.5),
            pupil.amplitude(0.1, 0.1, 13.5)
        );
        // Clones share the tables.
        let c = pupil.clone();
        assert!(Arc::ptr_eq(&c.tables, &pupil.tables));
    }
}
