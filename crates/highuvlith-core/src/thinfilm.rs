//! Thin-film optics by the characteristic-matrix (transfer-matrix) method.
//!
//! Models a stratified stack of absorbing layers between a superstrate (vacuum
//! for VUV) and a semi-infinite substrate. Each layer contributes a 2×2
//! characteristic matrix; their product gives the stack reflectance and
//! transmittance at an arbitrary incidence angle for TE, TM, or unpolarized
//! light, and the internal tangential fields give the standing-wave intensity
//! that exposes the resist.
//!
//! # Standing waves and the two field methods
//!
//! [`FilmStack::intensity_profile`] is the exact result: it propagates the
//! tangential fields `(U, V)` layer by layer, capturing standing waves,
//! absorption, and every interface reflection (for oblique TM it returns the
//! tangential-field intensity `|U|²`, a documented approximation).
//! [`FilmStack::standing_wave`] is an older approximation that combines only
//! the stack-top reflection with single-layer plane-wave propagation; it is
//! kept because the MNSL module depends on its exact numbers. Prefer
//! `intensity_profile` for quantitative work.
//!
//! # Conventions
//!
//! Refractive indices are stored as `n + ik` (k ≥ 0 for loss). The Macleod
//! matrix formalism needs `N = n − ik` for the downward field to decay, so
//! `intensity_profile` conjugates internally; reflectance is invariant to the
//! sign and does not. Standing-wave minima are spaced by `λ / (2n)`.
//!
//! # References
//!
//! - Macleod, *Thin-Film Optical Filters*, 4th ed. — characteristic matrices.

use num::Complex;
use serde::{Deserialize, Serialize};

use crate::types::{Complex64, Polarization};

/// A single layer in a thin-film stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilmLayer {
    pub name: String,
    /// Thickness in nm.
    pub thickness_nm: f64,
    /// Complex refractive index (n + i*k, where k is extinction coefficient).
    #[serde(
        serialize_with = "serialize_complex",
        deserialize_with = "deserialize_complex"
    )]
    pub n: Complex64,
}

/// Thin-film stack for standing wave and reflectance calculations.
/// Layers are ordered from top (superstrate side) to bottom (substrate side).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilmStack {
    /// Thin-film layers from top to bottom.
    pub layers: Vec<FilmLayer>,
    /// Complex refractive index of the substrate (semi-infinite bottom medium).
    #[serde(
        serialize_with = "serialize_complex",
        deserialize_with = "deserialize_complex"
    )]
    pub substrate: Complex64,
    /// Superstrate refractive index (vacuum for VUV: n=1.0).
    #[serde(
        serialize_with = "serialize_complex",
        deserialize_with = "deserialize_complex"
    )]
    pub superstrate: Complex64,
}

impl FilmStack {
    /// Create a VUV film stack with vacuum superstrate.
    pub fn new_vuv(layers: Vec<FilmLayer>, substrate: Complex64) -> Self {
        Self {
            layers,
            substrate,
            superstrate: Complex64::new(1.0, 0.0), // vacuum
        }
    }

    /// Compute reflectance at the top surface for normal incidence.
    pub fn reflectance(&self, wavelength_nm: f64, pol: Polarization) -> f64 {
        self.reflectance_at_angle(wavelength_nm, 0.0, pol)
    }

    /// Compute reflectance for arbitrary incidence angle (radians, from normal).
    pub fn reflectance_at_angle(
        &self,
        wavelength_nm: f64,
        angle_rad: f64,
        pol: Polarization,
    ) -> f64 {
        match pol {
            Polarization::TE => {
                let (r, _) = self.transfer_matrix(wavelength_nm, angle_rad, Polarization::TE);
                r.norm_sqr()
            }
            Polarization::TM => {
                let (r, _) = self.transfer_matrix(wavelength_nm, angle_rad, Polarization::TM);
                r.norm_sqr()
            }
            Polarization::Unpolarized => {
                let r_te = self.reflectance_at_angle(wavelength_nm, angle_rad, Polarization::TE);
                let r_tm = self.reflectance_at_angle(wavelength_nm, angle_rad, Polarization::TM);
                (r_te + r_tm) / 2.0
            }
        }
    }

    /// Compute the standing wave intensity pattern inside a film.
    /// Returns intensity values at the given z-positions (measured from top of stack, positive downward).
    ///
    /// NOTE: this is an approximation — it combines the stack-top
    /// reflection coefficient with single-layer plane-wave propagation,
    /// ignoring per-layer internal amplitudes. Kept as-is because the MNSL
    /// module and its tests depend on the exact numbers. For quantitative
    /// work (volumetric exposure, swing curves) use [`Self::intensity_profile`],
    /// which propagates the exact tangential fields layer by layer.
    pub fn standing_wave(&self, wavelength_nm: f64, z_points: &[f64]) -> Vec<f64> {
        let k0 = 2.0 * std::f64::consts::PI / wavelength_nm;

        z_points
            .iter()
            .map(|&z| {
                let (forward, backward) = self.field_at_depth(wavelength_nm, z, k0);
                // Intensity = |E_forward + E_backward|^2
                (forward + backward).norm_sqr()
            })
            .collect()
    }

    /// Exact intensity profile |E(z)|² inside the stack at unit incident
    /// intensity, at the given depths (nm from the stack top, positive
    /// downward).
    ///
    /// Uses the characteristic-matrix formalism: the tangential fields at
    /// the stack top are `U₀ = 1 + r`, `V₀ = η₀(1 − r)`; they are then
    /// propagated downward through each layer with the inverse layer
    /// matrix, and within the containing layer with a partial-thickness
    /// inverse matrix. This captures standing waves, absorption, and all
    /// interface effects exactly (unlike [`Self::standing_wave`]).
    ///
    /// Depths above the stack (z < 0) return the superstrate standing
    /// wave; depths below the stack return the transmitted intensity with
    /// substrate Beer–Lambert decay.
    ///
    /// For TE (and any polarization at normal incidence) the returned
    /// value is the exact |E|²; for oblique TM it is the tangential-field
    /// intensity |U|², a documented approximation.
    pub fn intensity_profile(
        &self,
        wavelength_nm: f64,
        angle_rad: f64,
        pol: Polarization,
        z_points: &[f64],
    ) -> Vec<f64> {
        match pol {
            Polarization::Unpolarized => {
                let te =
                    self.intensity_profile(wavelength_nm, angle_rad, Polarization::TE, z_points);
                let tm =
                    self.intensity_profile(wavelength_nm, angle_rad, Polarization::TM, z_points);
                te.iter()
                    .zip(tm.iter())
                    .map(|(a, b)| (a + b) / 2.0)
                    .collect()
            }
            _ => self.intensity_profile_polarized(wavelength_nm, angle_rad, pol, z_points),
        }
    }

    fn intensity_profile_polarized(
        &self,
        wavelength_nm: f64,
        angle_rad: f64,
        pol: Polarization,
        z_points: &[f64],
    ) -> Vec<f64> {
        // The stack stores extinction as n + ik, but the Macleod
        // characteristic-matrix formalism (and its downward-propagating
        // field e^{-i delta}) requires N = n - ik for absorption to DECAY.
        // Reflectance is conjugation-invariant so `transfer_matrix` can
        // ignore this; the internal field cannot. Work in the conjugated
        // convention throughout this method.
        let k0 = 2.0 * std::f64::consts::PI / wavelength_nm;
        let cos_0 = angle_rad.cos();
        let n_super = self.superstrate.conj();
        let n0_sin = n_super * Complex64::new(angle_rad.sin(), 0.0);
        let i_unit = Complex64::new(0.0, 1.0);

        let eta_of = |n: Complex64| -> (Complex64, Complex64) {
            // Returns (cos_theta_in_layer, admittance) for this polarization.
            let cos_j = (Complex64::new(1.0, 0.0) - (n0_sin / n).powi(2)).sqrt();
            let eta = match pol {
                Polarization::TE | Polarization::Unpolarized => n * cos_j,
                Polarization::TM => n / cos_j,
            };
            (cos_j, eta)
        };

        // First pass: build the full characteristic matrix (conjugated
        // convention) to get the reflection coefficient.
        let mut m = [
            [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
            [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
        ];
        for layer in &self.layers {
            let n = layer.n.conj();
            let (cos_j, eta_j) = eta_of(n);
            let delta = k0 * n * cos_j * layer.thickness_nm;
            let l = [
                [delta.cos(), i_unit * delta.sin() / eta_j],
                [i_unit * eta_j * delta.sin(), delta.cos()],
            ];
            m = mat2_mul(&m, &l);
        }

        let n_sub = self.substrate.conj();
        let (_, eta_s) = eta_of(n_sub);
        let (_, eta_0) = eta_of(n_super);

        let r_num = eta_0 * m[0][0] + eta_0 * eta_s * m[0][1] - m[1][0] - eta_s * m[1][1];
        let r_den = eta_0 * m[0][0] + eta_0 * eta_s * m[0][1] + m[1][0] + eta_s * m[1][1];
        let r = r_num / r_den;

        // Tangential fields at the stack top for unit incident amplitude.
        let mut u = Complex64::new(1.0, 0.0) + r;
        let mut v = eta_0 * (Complex64::new(1.0, 0.0) - r);

        // Second pass: record fields at each layer top, propagating (U, V)
        // down with inverse layer matrices (det = 1):
        //   [[cos δ, -i sin δ / η], [-i η sin δ, cos δ]]
        struct LayerFields {
            top_z: f64,
            thickness: f64,
            kz: Complex64,  // k0 * N * cos_theta (conjugated convention)
            eta: Complex64, // layer admittance
            u_top: Complex64,
            v_top: Complex64,
        }

        let mut layer_fields: Vec<LayerFields> = Vec::with_capacity(self.layers.len());
        let mut z_cursor = 0.0;

        for layer in &self.layers {
            let n = layer.n.conj();
            let (cos_j, eta_j) = eta_of(n);
            let kz = k0 * n * cos_j;

            layer_fields.push(LayerFields {
                top_z: z_cursor,
                thickness: layer.thickness_nm,
                kz,
                eta: eta_j,
                u_top: u,
                v_top: v,
            });

            let delta = kz * layer.thickness_nm;
            let cos_d = delta.cos();
            let sin_d = delta.sin();
            let u_next = cos_d * u - i_unit * sin_d / eta_j * v;
            let v_next = -i_unit * eta_j * sin_d * u + cos_d * v;
            u = u_next;
            v = v_next;
            z_cursor += layer.thickness_nm;
        }

        let total_thickness = z_cursor;
        let u_bottom = u;

        // Substrate propagation constant: Im(kz_s) <= 0 in this
        // convention, so intensity decays as exp(2 * Im(kz_s) * dz).
        let (cos_s, _) = eta_of(n_sub);
        let kz_s = k0 * n_sub * cos_s;

        // Superstrate (real index assumed) for z < 0.
        let kz_0 = k0 * n_super.re * cos_0;

        z_points
            .iter()
            .map(|&z| {
                if z < 0.0 {
                    // Incident + reflected standing wave above the stack.
                    let down = Complex64::from_polar(1.0, -kz_0 * z);
                    let up = r * Complex64::from_polar(1.0, kz_0 * z);
                    return (down + up).norm_sqr();
                }
                if z >= total_thickness {
                    // Transmitted field decaying into the substrate.
                    let dz = z - total_thickness;
                    return u_bottom.norm_sqr() * (2.0 * kz_s.im * dz).exp();
                }
                // Inside the stack: locate the containing layer.
                let lf = layer_fields
                    .iter()
                    .rev()
                    .find(|lf| z >= lf.top_z)
                    .expect("z >= 0 is inside some layer");
                let zeta = (z - lf.top_z).min(lf.thickness);
                let delta = lf.kz * zeta;
                let u_z = delta.cos() * lf.u_top - i_unit * delta.sin() / lf.eta * lf.v_top;
                u_z.norm_sqr()
            })
            .collect()
    }

    /// Transfer matrix method for the entire stack.
    /// Returns (reflection_coefficient, transmission_coefficient).
    fn transfer_matrix(
        &self,
        wavelength_nm: f64,
        angle_rad: f64,
        pol: Polarization,
    ) -> (Complex64, Complex64) {
        let k0 = 2.0 * std::f64::consts::PI / wavelength_nm;
        let cos_0 = angle_rad.cos();

        // Snell's law for complex media
        let n0_sin = self.superstrate * Complex64::new(angle_rad.sin(), 0.0);

        // Build transfer matrix M = product of layer matrices
        let mut m = [
            [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
            [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
        ];

        for layer in &self.layers {
            let cos_j = (Complex64::new(1.0, 0.0) - (n0_sin / layer.n).powi(2)).sqrt();
            let phase = k0 * layer.n * cos_j * layer.thickness_nm;

            let eta_j = match pol {
                Polarization::TE | Polarization::Unpolarized => layer.n * cos_j,
                Polarization::TM => layer.n / cos_j,
            };

            // Layer matrix
            let cos_phase = phase.cos();
            let sin_phase = phase.sin();
            let l = [
                [cos_phase, Complex64::new(0.0, 1.0) * sin_phase / eta_j],
                [Complex64::new(0.0, 1.0) * eta_j * sin_phase, cos_phase],
            ];

            // Matrix multiply: m = m * l
            let new_m = mat2_mul(&m, &l);
            m = new_m;
        }

        // Substrate admittance
        let cos_s = (Complex64::new(1.0, 0.0) - (n0_sin / self.substrate).powi(2)).sqrt();
        let eta_s = match pol {
            Polarization::TE | Polarization::Unpolarized => self.substrate * cos_s,
            Polarization::TM => self.substrate / cos_s,
        };

        // Superstrate admittance
        let eta_0 = match pol {
            Polarization::TE | Polarization::Unpolarized => {
                self.superstrate * Complex64::new(cos_0, 0.0)
            }
            Polarization::TM => self.superstrate / Complex64::new(cos_0, 0.0),
        };

        // r = (eta_0 * M11 + eta_0*eta_s*M12 - M21 - eta_s*M22) /
        //     (eta_0 * M11 + eta_0*eta_s*M12 + M21 + eta_s*M22)
        let r_num = eta_0 * m[0][0] + eta_0 * eta_s * m[0][1] - m[1][0] - eta_s * m[1][1];
        let r_den = eta_0 * m[0][0] + eta_0 * eta_s * m[0][1] + m[1][0] + eta_s * m[1][1];

        let r = r_num / r_den;
        let t = Complex64::new(2.0, 0.0) * eta_0 / r_den;

        (r, t)
    }

    /// Compute forward and backward propagating field amplitudes at a given depth.
    fn field_at_depth(&self, wavelength_nm: f64, z: f64, k0: f64) -> (Complex64, Complex64) {
        // Find which layer contains this z-position
        let mut depth = 0.0;
        let mut layer_idx = None;
        let mut z_in_layer = z;

        for (i, layer) in self.layers.iter().enumerate() {
            if z >= depth && z < depth + layer.thickness_nm {
                layer_idx = Some(i);
                z_in_layer = z - depth;
                break;
            }
            depth += layer.thickness_nm;
        }

        let (r_top, _) = self.transfer_matrix(wavelength_nm, 0.0, Polarization::Unpolarized);

        match layer_idx {
            Some(idx) => {
                let n = self.layers[idx].n;
                let phase = k0 * n * z_in_layer;
                let forward = Complex::from_polar(1.0, -phase.re) * (-phase.im).exp();
                let backward = r_top * Complex::from_polar(1.0, phase.re) * (-phase.im).exp();
                (forward, backward)
            }
            None => {
                // In substrate or above stack
                (Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0))
            }
        }
    }
}

fn mat2_mul(a: &[[Complex64; 2]; 2], b: &[[Complex64; 2]; 2]) -> [[Complex64; 2]; 2] {
    [
        [
            a[0][0] * b[0][0] + a[0][1] * b[1][0],
            a[0][0] * b[0][1] + a[0][1] * b[1][1],
        ],
        [
            a[1][0] * b[0][0] + a[1][1] * b[1][0],
            a[1][0] * b[0][1] + a[1][1] * b[1][1],
        ],
    ]
}

impl Default for FilmStack {
    fn default() -> Self {
        // Default VUV stack: resist on silicon
        Self::new_vuv(
            vec![FilmLayer {
                name: "resist".to_string(),
                thickness_nm: 150.0,
                n: Complex64::new(1.65, 0.015),
            }],
            Complex64::new(0.88, 2.10), // Si at 157nm
        )
    }
}

fn serialize_complex<S>(c: &Complex64, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeTuple;
    let mut tup = serializer.serialize_tuple(2)?;
    tup.serialize_element(&c.re)?;
    tup.serialize_element(&c.im)?;
    tup.end()
}

fn deserialize_complex<'de, D>(deserializer: D) -> std::result::Result<Complex64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let (re, im) = <(f64, f64)>::deserialize(deserializer)?;
    Ok(Complex64::new(re, im))
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_bare_substrate_reflectance() {
        let stack = FilmStack::new_vuv(vec![], Complex64::new(1.5, 0.0));
        let r = stack.reflectance(157.0, Polarization::Unpolarized);
        // Fresnel: R = ((n1-n2)/(n1+n2))^2 = ((1-1.5)/(1+1.5))^2 = 0.04
        assert_relative_eq!(r, 0.04, epsilon = 0.001);
    }

    #[test]
    fn test_quarter_wave_ar() {
        // Quarter-wave AR coating: n_film = sqrt(n_substrate), thickness = lambda/(4*n_film)
        let n_sub = 1.5_f64;
        let n_film = n_sub.sqrt();
        let wavelength = 157.0;
        let thickness = wavelength / (4.0 * n_film);

        let stack = FilmStack::new_vuv(
            vec![FilmLayer {
                name: "AR".to_string(),
                thickness_nm: thickness,
                n: Complex64::new(n_film, 0.0),
            }],
            Complex64::new(n_sub, 0.0),
        );
        let r = stack.reflectance(wavelength, Polarization::TE);
        // Should be near zero for ideal AR
        assert!(
            r < 0.001,
            "Quarter-wave AR reflectance should be near zero, got {}",
            r
        );
    }

    #[test]
    fn test_quarter_wave_is_minimum() {
        let n_sub = 1.5_f64;
        let n_film = n_sub.sqrt();
        let wavelength = 157.0;
        let thickness_qw = wavelength / (4.0 * n_film);

        let stack_qw = FilmStack::new_vuv(
            vec![FilmLayer {
                name: "AR".to_string(),
                thickness_nm: thickness_qw,
                n: Complex64::new(n_film, 0.0),
            }],
            Complex64::new(n_sub, 0.0),
        );

        let stack_off = FilmStack::new_vuv(
            vec![FilmLayer {
                name: "AR".to_string(),
                thickness_nm: thickness_qw * 1.3,
                n: Complex64::new(n_film, 0.0),
            }],
            Complex64::new(n_sub, 0.0),
        );

        let r_qw = stack_qw.reflectance(wavelength, Polarization::TE);
        let r_off = stack_off.reflectance(wavelength, Polarization::TE);
        assert!(r_qw < r_off);
    }

    #[test]
    fn test_standing_wave_not_empty() {
        let stack = FilmStack::default();
        let z_points: Vec<f64> = (0..100).map(|i| i as f64 * 1.5).collect();
        let sw = stack.standing_wave(157.0, &z_points);
        assert_eq!(sw.len(), 100);
        assert!(sw.iter().all(|&v| v >= 0.0));
    }

    #[test]
    fn test_intensity_profile_matches_fresnel_at_bare_interface() {
        // Bare substrate: intensity just below the surface must equal
        // |1 + r|² = |t|² (field continuity), with r from Fresnel.
        let n_sub = 1.5_f64;
        let stack = FilmStack::new_vuv(vec![], Complex64::new(n_sub, 0.0));
        let r = (1.0 - n_sub) / (1.0 + n_sub);
        let expected = (1.0 + r) * (1.0 + r);
        let i = stack.intensity_profile(157.0, 0.0, Polarization::TE, &[0.0]);
        assert_relative_eq!(i[0], expected, epsilon = 1e-9);
    }

    #[test]
    fn test_intensity_profile_beer_lambert_when_index_matched() {
        // Absorbing layer index-matched to superstrate and substrate:
        // no reflections anywhere, so I(z) = exp(-alpha z) exactly with
        // alpha = 4 pi k / lambda.
        let k = 0.02;
        let wavelength = 157.0;
        let stack = FilmStack {
            layers: vec![FilmLayer {
                name: "absorber".to_string(),
                thickness_nm: 300.0,
                n: Complex64::new(1.0, k),
            }],
            substrate: Complex64::new(1.0, k),
            superstrate: Complex64::new(1.0, 0.0),
        };
        let alpha = 4.0 * std::f64::consts::PI * k / wavelength;
        let z_points = [0.0, 50.0, 100.0, 200.0, 299.0];
        let profile = stack.intensity_profile(wavelength, 0.0, Polarization::TE, &z_points);
        for (z, i) in z_points.iter().zip(profile.iter()) {
            let expected = (-alpha * z).exp();
            assert_relative_eq!(*i, expected, epsilon = 2e-2);
        }
    }

    #[test]
    fn test_intensity_profile_standing_wave_period() {
        // Lossless resist on a highly reflective substrate: standing wave
        // minima spaced by lambda / (2 n_resist).
        let n_resist = 1.65;
        let wavelength = 157.0;
        let stack = FilmStack::new_vuv(
            vec![FilmLayer {
                name: "resist".to_string(),
                thickness_nm: 400.0,
                n: Complex64::new(n_resist, 0.0),
            }],
            Complex64::new(0.88, 2.10), // Si: strong reflector
        );
        let nz = 4000;
        let z_points: Vec<f64> = (0..nz).map(|i| i as f64 * 400.0 / nz as f64).collect();
        let profile = stack.intensity_profile(wavelength, 0.0, Polarization::TE, &z_points);

        // Find local minima
        let mut minima = Vec::new();
        for i in 1..nz - 1 {
            if profile[i] < profile[i - 1] && profile[i] < profile[i + 1] {
                minima.push(z_points[i]);
            }
        }
        assert!(minima.len() >= 2, "expected multiple standing-wave minima");
        let period_expected = wavelength / (2.0 * n_resist);
        for pair in minima.windows(2) {
            let spacing = pair[1] - pair[0];
            assert_relative_eq!(spacing, period_expected, epsilon = 0.5);
        }
    }

    #[test]
    fn test_intensity_profile_continuous_at_interfaces() {
        // Tangential E is continuous across interfaces: the profile must
        // not jump between the bottom of one layer and the top of the next.
        let stack = FilmStack::new_vuv(
            vec![
                FilmLayer {
                    name: "arc".to_string(),
                    thickness_nm: 60.0,
                    n: Complex64::new(1.8, 0.3),
                },
                FilmLayer {
                    name: "resist".to_string(),
                    thickness_nm: 150.0,
                    n: Complex64::new(1.65, 0.015),
                },
            ],
            Complex64::new(0.88, 2.10),
        );
        let eps = 1e-6;
        for boundary in [60.0, 210.0] {
            let i = stack.intensity_profile(
                157.0,
                0.0,
                Polarization::TE,
                &[boundary - eps, boundary + eps],
            );
            assert_relative_eq!(i[0], i[1], epsilon = 1e-4);
        }
    }

    #[test]
    fn test_intensity_profile_unpolarized_is_te_tm_mean() {
        let stack = FilmStack::default();
        let z = [0.0, 40.0, 80.0, 120.0];
        let angle = 0.3;
        let te = stack.intensity_profile(157.0, angle, Polarization::TE, &z);
        let tm = stack.intensity_profile(157.0, angle, Polarization::TM, &z);
        let un = stack.intensity_profile(157.0, angle, Polarization::Unpolarized, &z);
        for i in 0..z.len() {
            assert_relative_eq!(un[i], (te[i] + tm[i]) / 2.0, epsilon = 1e-12);
        }
    }

    #[test]
    fn test_brewster_angle() {
        // At Brewster's angle, TM reflectance should be zero for dielectric
        let n_sub = 1.5;
        let stack = FilmStack::new_vuv(vec![], Complex64::new(n_sub, 0.0));
        let brewster = (n_sub / 1.0).atan(); // atan(n2/n1)
        let r_tm = stack.reflectance_at_angle(157.0, brewster, Polarization::TM);
        assert!(
            r_tm < 0.001,
            "TM reflectance at Brewster's angle should be near zero, got {}",
            r_tm
        );
    }
}
