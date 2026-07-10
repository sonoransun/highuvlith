//! Multi-beam interference lithography and two-photon voxel writing.
//!
//! Interference (holographic) lithography superposes a few coherent plane
//! waves so their standing-wave intensity forms a periodic lattice, which a
//! resist records in a single exposure: two beams give a 1D grating, three
//! give a 2D hexagonal lattice, and a central-plus-umbrella set gives an
//! FCC-like 3D lattice. Two-photon (multiphoton) direct writing instead scans
//! a tightly focused pulsed beam whose `I²` absorption confines polymerization
//! to a sub-diffraction voxel around the focus.
//!
//! The field of a set of plane waves is evaluated on a [`Grid3D`], summing the
//! complex vector amplitude per Cartesian component and taking `Σ_c |E_c|²`.
//! Presets convert an air-side incidence half-angle to the in-resist angle via
//! Snell's law; the resulting fringe period is set by the *air-side* angle and
//! is therefore invariant under the resist index.
//!
//! # Key equations
//!
//! ```text
//!   E_c(r) = Σ_i A_i · a_i(z) · ε̂_{i,c} · exp(i(2π n/λ_vac · k̂_i·r + φ_i))
//!   a_i(z) = exp(−α · z / (2 k̂_{z,i}))            per-beam amplitude decay
//!   I(r)   = Σ_c |E_c(r)|²,           c ∈ {x, y, z}
//!   two-beam fringe period  Λ = λ_vac / (2 sin θ_air)   (index-invariant)
//! ```
//!
//! # Model status
//!
//! Scalar per-component superposition of ideal, infinite plane waves: no
//! reflection at the resist/substrate interface, no vector focusing
//! corrections, and absorption applied as a scalar decay along each beam's own
//! path (`z / k̂_z`) rather than a self-consistent transfer-matrix field.
//! Convention: z runs downward into the resist, so propagating beams have
//! `k̂_z > 0`; absorption scaling is skipped for any beam with `k̂_z ≤ 0`. The
//! exposure and voxel-writing helpers emit photo-active-compound (PAC) volumes
//! that can feed the volumetric development tiers in [`crate::volumetric`].
//!
//! # References
//!
//! - Campbell et al., "Fabrication of photonic crystals ... by holographic
//!   lithography," Nature 404, 53 (2000).
//! - Maruo & Fourkas, "Recent progress in multiphoton microfabrication,"
//!   Laser Photonics Rev. 2, 100 (2008).

use serde::{Deserialize, Serialize};

use crate::error::{LithographyError, Result};
use crate::types::{Complex64, Grid3D};

/// A single coherent plane wave.
///
/// The propagation direction `k_hat` is a unit vector (z downward into the
/// resist, so propagating beams have `k_hat[2] > 0`), and the real
/// polarization `ε̂` is a unit vector orthogonal to `k_hat`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PlaneWave {
    /// Unit propagation direction.
    pub k_hat: [f64; 3],
    /// Field amplitude.
    pub amplitude: f64,
    /// Phase offset in radians.
    pub phase_rad: f64,
    /// Unit real polarization vector, orthogonal to `k_hat`.
    pub polarization: [f64; 3],
}

impl PlaneWave {
    /// Build a plane wave, normalizing `k_hat` and validating the
    /// polarization.
    ///
    /// `k_hat` is normalized to unit length (an error if it is the zero
    /// vector). The polarization must already be unit length and orthogonal to
    /// `k_hat`, both within `1e-6`, otherwise an error is returned.
    pub fn new(
        k_hat: [f64; 3],
        amplitude: f64,
        phase_rad: f64,
        polarization: [f64; 3],
    ) -> Result<Self> {
        let k_norm = norm3(k_hat);
        if k_norm < 1e-12 {
            return Err(LithographyError::InvalidParameter {
                name: "k_hat",
                value: k_norm,
                reason: "propagation direction must be a non-zero vector",
            });
        }
        let k_unit = [k_hat[0] / k_norm, k_hat[1] / k_norm, k_hat[2] / k_norm];

        let p_norm = norm3(polarization);
        if (p_norm - 1.0).abs() > 1e-6 {
            return Err(LithographyError::InvalidParameter {
                name: "polarization",
                value: p_norm,
                reason: "polarization must be a unit vector",
            });
        }
        let dot = dot3(k_unit, polarization);
        if dot.abs() > 1e-6 {
            return Err(LithographyError::InvalidParameter {
                name: "polarization",
                value: dot,
                reason: "polarization must be orthogonal to the propagation direction",
            });
        }

        Ok(Self {
            k_hat: k_unit,
            amplitude,
            phase_rad,
            polarization,
        })
    }
}

/// A set of coherent plane waves in an absorbing medium.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterferenceSetup {
    /// The interfering beams.
    pub beams: Vec<PlaneWave>,
    /// Vacuum wavelength in nm.
    pub wavelength_vacuum_nm: f64,
    /// Refractive index of the recording medium (resist).
    pub medium_index: f64,
    /// Intensity absorption coefficient in 1/nm, applied per beam along its own
    /// path length.
    pub absorption_per_nm: f64,
}

impl InterferenceSetup {
    /// Evaluate the interference intensity `Σ_c |E_c|²` at every voxel of
    /// `grid`, overwriting `grid.data`.
    ///
    /// Each beam's amplitude decays as `exp(−α · z / (2 k̂_z))` along its own
    /// slanted path (intensity `exp(−α z / k̂_z)`); the decay is skipped for
    /// any beam with `k̂_z ≤ 0` (see the module's model-status note).
    pub fn intensity(&self, grid: &mut Grid3D<f64>) {
        let spatial = 2.0 * std::f64::consts::PI * self.medium_index / self.wavelength_vacuum_nm;
        let nx = grid.nx();
        let ny = grid.ny();
        let nz = grid.nz();

        for k in 0..nz {
            let z = grid.z_at(k);
            for i in 0..ny {
                let y = grid.y_at(i);
                for j in 0..nx {
                    let x = grid.x_at(j);
                    let mut e = [Complex64::new(0.0, 0.0); 3];
                    for beam in &self.beams {
                        let kz = beam.k_hat[2];
                        let decay = if self.absorption_per_nm > 0.0 && kz > 0.0 {
                            (-self.absorption_per_nm * z / (2.0 * kz)).exp()
                        } else {
                            1.0
                        };
                        let phase = spatial
                            * (beam.k_hat[0] * x + beam.k_hat[1] * y + beam.k_hat[2] * z)
                            + beam.phase_rad;
                        let carrier = Complex64::from_polar(1.0, phase);
                        let amp = beam.amplitude * decay;
                        for (ec, &pol) in e.iter_mut().zip(beam.polarization.iter()) {
                            *ec += carrier.scale(amp * pol);
                        }
                    }
                    grid.data[[k, i, j]] = e[0].norm_sqr() + e[1].norm_sqr() + e[2].norm_sqr();
                }
            }
        }
    }

    /// Two TE-polarized (`ε̂ = ŷ`) beams in the xz-plane at `±θ_med`.
    ///
    /// The fringe period `Λ = λ_vac / (2 sin θ_air)` is set by the air-side
    /// half-angle and is **invariant** under the resist index: refraction bends
    /// the beams toward normal (`sin θ_med = sin θ_air / n`), but the in-plane
    /// wavevector `(n/λ) sin θ_med = sin θ_air / λ` — and hence the period — is
    /// unchanged.
    pub fn two_beam(wavelength_nm: f64, n_medium: f64, half_angle_air_deg: f64) -> Result<Self> {
        let theta_med = snell_theta_med(wavelength_nm, n_medium, half_angle_air_deg)?;
        let (s, c) = theta_med.sin_cos();
        let pol = [0.0, 1.0, 0.0]; // ŷ, TE
        let beams = vec![
            PlaneWave::new([s, 0.0, c], 1.0, 0.0, pol)?,
            PlaneWave::new([-s, 0.0, c], 1.0, 0.0, pol)?,
        ];
        Ok(Self {
            beams,
            wavelength_vacuum_nm: wavelength_nm,
            medium_index: n_medium,
            absorption_per_nm: 0.0,
        })
    }

    /// Three TE-polarized beams at azimuths 0°, 120°, 240° and a common polar
    /// angle `θ_med`, producing a 2D hexagonal lattice.
    ///
    /// Each beam is polarized azimuthally (`ε̂ = ẑ × k̂`, normalized), the
    /// natural TE choice for an off-axis beam.
    pub fn three_beam_hex(
        wavelength_nm: f64,
        n_medium: f64,
        half_angle_air_deg: f64,
    ) -> Result<Self> {
        let theta_med = snell_theta_med(wavelength_nm, n_medium, half_angle_air_deg)?;
        let mut beams = Vec::with_capacity(3);
        for azimuth_deg in [0.0, 120.0, 240.0] {
            beams.push(umbrella_beam(theta_med, azimuth_deg)?);
        }
        Ok(Self {
            beams,
            wavelength_vacuum_nm: wavelength_nm,
            medium_index: n_medium,
            absorption_per_nm: 0.0,
        })
    }

    /// A central beam along `+z` plus three umbrella beams at azimuths 0°,
    /// 120°, 240° and polar angle `θ_med`, producing an FCC-like 3D lattice.
    ///
    /// The central beam is polarized along `x̂`; the umbrella beams are
    /// azimuthally (TE) polarized like [`Self::three_beam_hex`].
    pub fn four_beam_umbrella(
        wavelength_nm: f64,
        n_medium: f64,
        half_angle_air_deg: f64,
    ) -> Result<Self> {
        let theta_med = snell_theta_med(wavelength_nm, n_medium, half_angle_air_deg)?;
        let mut beams = Vec::with_capacity(4);
        beams.push(PlaneWave::new([0.0, 0.0, 1.0], 1.0, 0.0, [1.0, 0.0, 0.0])?);
        for azimuth_deg in [0.0, 120.0, 240.0] {
            beams.push(umbrella_beam(theta_med, azimuth_deg)?);
        }
        Ok(Self {
            beams,
            wavelength_vacuum_nm: wavelength_nm,
            medium_index: n_medium,
            absorption_per_nm: 0.0,
        })
    }
}

/// Exposure order for the interference dose→PAC map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExposureKinetics {
    /// Single-photon absorption: `m = exp(−C · dose · I)`.
    OnePhoton,
    /// Two-photon absorption: `m = exp(−C · dose · I²)`.
    TwoPhoton,
}

/// Map an intensity volume to a normalized photo-active-compound (PAC) volume
/// via Dill-style kinetics.
///
/// `m = exp(−C · dose · I)` for one-photon exposure or `exp(−C · dose · I²)`
/// for two-photon. The output (`m = 1` unexposed, `m → 0` fully exposed) can
/// feed the volumetric development tiers in [`crate::volumetric`].
pub fn expose(
    intensity: &Grid3D<f64>,
    dose_scale: f64,
    dill_c: f64,
    kinetics: ExposureKinetics,
) -> Grid3D<f64> {
    let data = intensity.data.mapv(|i| {
        let effective = match kinetics {
            ExposureKinetics::OnePhoton => i,
            ExposureKinetics::TwoPhoton => i * i,
        };
        (-dill_c * dose_scale * effective).exp()
    });
    like_grid(intensity, data)
}

/// A focused Gaussian beam for two-photon direct-write voxel exposure.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GaussianFocus {
    /// `1/e²` intensity waist radius `w₀` in nm.
    pub waist_nm: f64,
    /// Vacuum wavelength in nm.
    pub wavelength_nm: f64,
    /// Refractive index of the medium.
    pub medium_index: f64,
}

impl GaussianFocus {
    /// Rayleigh range `z_R = π w₀² n / λ` in nm.
    pub fn rayleigh_range_nm(&self) -> f64 {
        std::f64::consts::PI * self.waist_nm * self.waist_nm * self.medium_index
            / self.wavelength_nm
    }

    /// Beam radius `w(z) = w₀ · sqrt(1 + (z/z_R)²)` at axial offset `z_off_nm`.
    pub fn beam_radius_nm(&self, z_off_nm: f64) -> f64 {
        let ratio = z_off_nm / self.rayleigh_range_nm();
        self.waist_nm * (1.0 + ratio * ratio).sqrt()
    }

    /// Normalized intensity `(w₀/w(z))² · exp(−2 r² / w(z)²)` at lateral radius
    /// `r_nm` and axial offset `z_off_nm` (unity on-axis at the focus).
    pub fn intensity_at(&self, r_nm: f64, z_off_nm: f64) -> f64 {
        let w = self.beam_radius_nm(z_off_nm);
        let ratio = self.waist_nm / w;
        ratio * ratio * (-2.0 * r_nm * r_nm / (w * w)).exp()
    }
}

/// Accumulate two-photon (`I²`) exposure into `grid` as a focused beam is
/// scanned through the listed focus positions.
///
/// For each point `(px, py, pz)` in `path` the beam is centered at that
/// position with its axis along z; every voxel accumulates
/// `exposure_per_point · I(r, Δz)²`, where `r` is the lateral distance to the
/// focus axis and `Δz` the axial offset. Feed the result to
/// [`voxels_to_pac`].
pub fn write_voxels(
    grid: &mut Grid3D<f64>,
    focus: &GaussianFocus,
    path: &[(f64, f64, f64)],
    exposure_per_point: f64,
) {
    let nx = grid.nx();
    let ny = grid.ny();
    let nz = grid.nz();
    for &(px, py, pz) in path {
        for k in 0..nz {
            let z = grid.z_at(k);
            for i in 0..ny {
                let y = grid.y_at(i);
                for j in 0..nx {
                    let x = grid.x_at(j);
                    let dx = x - px;
                    let dy = y - py;
                    let r = (dx * dx + dy * dy).sqrt();
                    let intensity = focus.intensity_at(r, z - pz);
                    grid.data[[k, i, j]] += exposure_per_point * intensity * intensity;
                }
            }
        }
    }
}

/// Map an accumulated two-photon exposure volume to a PAC volume:
/// `m = exp(−c2 · E)`.
pub fn voxels_to_pac(exposure: &Grid3D<f64>, c2: f64) -> Grid3D<f64> {
    let data = exposure.data.mapv(|e| (-c2 * e).exp());
    like_grid(exposure, data)
}

/// Fraction of voxels below the development threshold (PAC `m < threshold`) —
/// the developed/written fill fraction of a lattice.
pub fn iso_surface_fill_fraction(pac: &Grid3D<f64>, threshold: f64) -> f64 {
    let total = pac.data.len();
    if total == 0 {
        return 0.0;
    }
    let written = pac.data.iter().filter(|&&m| m < threshold).count();
    written as f64 / total as f64
}

/// Snell refraction: in-medium polar angle from the air-side half-angle.
fn snell_theta_med(wavelength_nm: f64, n_medium: f64, half_angle_air_deg: f64) -> Result<f64> {
    if wavelength_nm.is_nan() || wavelength_nm <= 0.0 {
        return Err(LithographyError::InvalidParameter {
            name: "wavelength_nm",
            value: wavelength_nm,
            reason: "must be positive",
        });
    }
    if n_medium.is_nan() || n_medium < 1.0 {
        return Err(LithographyError::InvalidParameter {
            name: "n_medium",
            value: n_medium,
            reason: "refractive index must be at least 1",
        });
    }
    if half_angle_air_deg.is_nan() || half_angle_air_deg <= 0.0 || half_angle_air_deg >= 90.0 {
        return Err(LithographyError::InvalidParameter {
            name: "half_angle_air_deg",
            value: half_angle_air_deg,
            reason: "must be in the open interval (0, 90) degrees",
        });
    }
    let sin_med = half_angle_air_deg.to_radians().sin() / n_medium;
    Ok(sin_med.asin())
}

/// An off-axis beam at polar angle `theta_med` and the given azimuth, with
/// azimuthal (TE) polarization `ẑ × k̂`.
fn umbrella_beam(theta_med: f64, azimuth_deg: f64) -> Result<PlaneWave> {
    let (st, ct) = theta_med.sin_cos();
    let (sa, ca) = azimuth_deg.to_radians().sin_cos();
    let k_hat = [st * ca, st * sa, ct];
    // ẑ × k̂ = (−k_y, k_x, 0); normalized (magnitude sin θ) = (−sin a, cos a, 0).
    let pol = [-sa, ca, 0.0];
    PlaneWave::new(k_hat, 1.0, 0.0, pol)
}

fn norm3(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Clone the coordinate extents of `src` around a freshly computed data array.
fn like_grid(src: &Grid3D<f64>, data: ndarray::Array3<f64>) -> Grid3D<f64> {
    Grid3D {
        data,
        x_min_nm: src.x_min_nm,
        x_max_nm: src.x_max_nm,
        y_min_nm: src.y_min_nm,
        y_max_nm: src.y_max_nm,
        z_min_nm: src.z_min_nm,
        z_max_nm: src.z_max_nm,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    /// Local minima positions (physical x) of a 1D intensity slice.
    fn minima_positions(slice: &[f64], x0: f64, dx: f64) -> Vec<f64> {
        let mut mins = Vec::new();
        for j in 1..slice.len() - 1 {
            if slice[j] < slice[j - 1] && slice[j] < slice[j + 1] {
                mins.push(x0 + (j as f64 + 0.5) * dx);
            }
        }
        mins
    }

    /// x-slice at fixed z (k) and y (i).
    fn x_slice(grid: &Grid3D<f64>, k: usize, i: usize) -> Vec<f64> {
        (0..grid.nx()).map(|j| grid.data[[k, i, j]]).collect()
    }

    fn two_beam_grid() -> Grid3D<f64> {
        // dx = 8 nm, and with λ = 200 nm, θ_air = 30° (sin = 0.5) the fringe
        // period is Λ = 200 nm = 25 pixels, and fringe minima fall exactly on
        // grid nodes.
        Grid3D::<f64>::new(128, 4, 8, (0.0, 1024.0), (0.0, 32.0), (0.0, 64.0)).unwrap()
    }

    #[test]
    fn test_two_beam_period_and_index_invariance() {
        let expected_period = 200.0; // λ / (2 sin θ_air) = 200 / 1
        let dx = 8.0;

        let mut spacings = Vec::new();
        for &n in &[1.0_f64, 1.7] {
            let setup = InterferenceSetup::two_beam(200.0, n, 30.0).unwrap();
            let mut grid = two_beam_grid();
            setup.intensity(&mut grid);
            let slice = x_slice(&grid, 0, 0);
            let mins = minima_positions(&slice, grid.x_min_nm, dx);
            assert!(
                mins.len() >= 4,
                "expected several fringe minima, got {mins:?}"
            );
            let spacing = (mins[mins.len() - 1] - mins[0]) / (mins.len() as f64 - 1.0);
            // Measured period matches the closed form to within one pixel.
            assert!(
                (spacing - expected_period).abs() < dx,
                "n = {n}: period {spacing} nm off from {expected_period} nm"
            );
            spacings.push(spacing);
        }
        // Refraction invariance: index 1.0 and 1.7 give the same period.
        assert_relative_eq!(spacings[0], spacings[1], epsilon = 1e-9);
    }

    #[test]
    fn test_two_beam_visibility_unity() {
        let setup = InterferenceSetup::two_beam(200.0, 1.0, 30.0).unwrap();
        let mut grid = two_beam_grid();
        setup.intensity(&mut grid);
        let slice = x_slice(&grid, 0, 0);
        let max = slice.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min = slice.iter().cloned().fold(f64::INFINITY, f64::min);
        // Equal-amplitude TE beams interfere fully destructively; the grid is
        // aligned so a node lands on a null, so I_min ≈ 0 and visibility = 1.
        assert!(min < 1e-9, "expected a near-zero fringe null, got {min}");
        let visibility = (max - min) / (max + min);
        assert_relative_eq!(visibility, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_two_beam_absorption_decays_exponentially() {
        let alpha = 0.005; // 1/nm
        let n = 1.0; // θ_med = θ_air = 30°
        let mut setup = InterferenceSetup::two_beam(200.0, n, 30.0).unwrap();
        setup.absorption_per_nm = alpha;

        let mut grid =
            Grid3D::<f64>::new(64, 2, 16, (0.0, 512.0), (0.0, 16.0), (0.0, 200.0)).unwrap();
        setup.intensity(&mut grid);

        let cos_theta = 30.0_f64.to_radians().cos();
        let mean_at = |k: usize| -> f64 {
            let slice = x_slice(&grid, k, 0);
            slice.iter().sum::<f64>() / slice.len() as f64
        };

        // The z-dependence factorizes out of the x-average, so the ratio of
        // slice means is the exact intensity decay exp(−α Δz / cos θ).
        let m0 = mean_at(0);
        let m8 = mean_at(8);
        assert!(m8 < m0, "intensity must decay with depth");
        let dz = grid.z_at(8) - grid.z_at(0);
        let expected = (-alpha * dz / cos_theta).exp();
        assert_relative_eq!(m8 / m0, expected, epsilon = 1e-9);
    }

    #[test]
    fn test_intensity_non_negative() {
        let setup = InterferenceSetup::three_beam_hex(200.0, 1.5, 20.0).unwrap();
        let mut grid =
            Grid3D::<f64>::new(32, 32, 4, (0.0, 400.0), (0.0, 400.0), (0.0, 50.0)).unwrap();
        setup.intensity(&mut grid);
        for &v in grid.data.iter() {
            assert!(v >= 0.0, "intensity must be non-negative, got {v}");
        }
    }

    #[test]
    fn test_three_beam_hex_c3_symmetry() {
        // The three fringe wavevectors are the pairwise beam-direction
        // differences; equal magnitudes ⇒ equal fringe periods ⇒ a hexagonal
        // (C3-symmetric) lattice.
        let setup = InterferenceSetup::three_beam_hex(200.0, 1.5, 20.0).unwrap();
        assert_eq!(setup.beams.len(), 3);
        let diff = |a: [f64; 3], b: [f64; 3]| norm3([a[0] - b[0], a[1] - b[1], a[2] - b[2]]);
        let d01 = diff(setup.beams[0].k_hat, setup.beams[1].k_hat);
        let d12 = diff(setup.beams[1].k_hat, setup.beams[2].k_hat);
        let d20 = diff(setup.beams[2].k_hat, setup.beams[0].k_hat);
        assert_relative_eq!(d01, d12, epsilon = 1e-9);
        assert_relative_eq!(d12, d20, epsilon = 1e-9);
    }

    #[test]
    fn test_four_beam_umbrella_shape() {
        let setup = InterferenceSetup::four_beam_umbrella(200.0, 1.5, 25.0).unwrap();
        assert_eq!(setup.beams.len(), 4);
        // Central beam straight down.
        assert_relative_eq!(setup.beams[0].k_hat[2], 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_gaussian_focus_waist_and_two_photon_falloff() {
        let focus = GaussianFocus {
            waist_nm: 200.0,
            wavelength_nm: 800.0,
            medium_index: 1.5,
        };
        let zr = focus.rayleigh_range_nm();
        // w(z_R) = w₀·√2.
        assert_relative_eq!(
            focus.beam_radius_nm(zr),
            focus.waist_nm * 2.0_f64.sqrt(),
            epsilon = 1e-9
        );
        // On-axis intensity is unity at focus, ½ at z_R.
        assert_relative_eq!(focus.intensity_at(0.0, 0.0), 1.0, epsilon = 1e-12);
        assert_relative_eq!(focus.intensity_at(0.0, zr), 0.5, epsilon = 1e-9);
        // Two-photon exposure ∝ I², so it falls 4× at z_R.
        let e_focus = focus.intensity_at(0.0, 0.0).powi(2);
        let e_zr = focus.intensity_at(0.0, zr).powi(2);
        assert_relative_eq!(e_focus / e_zr, 4.0, epsilon = 1e-9);
    }

    #[test]
    fn test_write_voxels_peak_at_focus_and_fill_fraction() {
        let focus = GaussianFocus {
            waist_nm: 40.0,
            wavelength_nm: 800.0,
            medium_index: 1.5,
        };
        let mut grid =
            Grid3D::<f64>::new(21, 21, 5, (-105.0, 105.0), (-105.0, 105.0), (-50.0, 50.0)).unwrap();
        // Single exposure point at the grid center.
        write_voxels(&mut grid, &focus, &[(0.0, 0.0, 0.0)], 10.0);

        // The central voxel (nearest the focus) carries the largest exposure.
        let center = grid.data[[2, 10, 10]];
        let corner = grid.data[[0, 0, 0]];
        assert!(center > corner);
        let max = grid.data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert_relative_eq!(center, max, epsilon = 1e-12);

        // A tight two-photon voxel writes only a small fraction of the volume.
        let pac = voxels_to_pac(&grid, 5.0);
        let fill = iso_surface_fill_fraction(&pac, 0.5);
        assert!(fill > 0.0 && fill < 0.2, "unexpected fill fraction {fill}");
    }

    #[test]
    fn test_expose_two_photon_sharper_than_one_photon() {
        let mut grid = Grid3D::<f64>::new(1, 1, 1, (0.0, 1.0), (0.0, 1.0), (0.0, 1.0)).unwrap();
        grid.data[[0, 0, 0]] = 0.5;
        let one = expose(&grid, 2.0, 1.0, ExposureKinetics::OnePhoton);
        let two = expose(&grid, 2.0, 1.0, ExposureKinetics::TwoPhoton);
        // m = exp(−C·dose·I) vs exp(−C·dose·I²); at I = 0.5, I² < I so the
        // two-photon PAC is less consumed (larger m).
        assert_relative_eq!(one.data[[0, 0, 0]], (-1.0_f64).exp(), epsilon = 1e-12);
        assert_relative_eq!(two.data[[0, 0, 0]], (-0.5_f64).exp(), epsilon = 1e-12);
        assert!(two.data[[0, 0, 0]] > one.data[[0, 0, 0]]);
    }

    #[test]
    fn test_planewave_zero_k_rejected() {
        assert!(PlaneWave::new([0.0, 0.0, 0.0], 1.0, 0.0, [0.0, 1.0, 0.0]).is_err());
    }

    #[test]
    fn test_planewave_non_orthogonal_polarization_rejected() {
        // Unit polarization, but not orthogonal to k = ẑ.
        let pol = [1.0 / 2.0_f64.sqrt(), 0.0, 1.0 / 2.0_f64.sqrt()];
        assert!(PlaneWave::new([0.0, 0.0, 1.0], 1.0, 0.0, pol).is_err());
    }

    #[test]
    fn test_planewave_non_unit_polarization_rejected() {
        assert!(PlaneWave::new([0.0, 0.0, 1.0], 1.0, 0.0, [2.0, 0.0, 0.0]).is_err());
    }

    #[test]
    fn test_planewave_normalizes_non_unit_k() {
        // A non-unit but non-zero k is normalized, not rejected.
        let wave = PlaneWave::new([0.0, 0.0, 3.0], 1.0, 0.0, [1.0, 0.0, 0.0]).unwrap();
        assert_relative_eq!(norm3(wave.k_hat), 1.0, epsilon = 1e-12);
        assert_relative_eq!(wave.k_hat[2], 1.0, epsilon = 1e-12);
    }

    #[test]
    fn test_invalid_preset_parameters() {
        assert!(InterferenceSetup::two_beam(-200.0, 1.5, 30.0).is_err());
        assert!(InterferenceSetup::two_beam(200.0, 0.5, 30.0).is_err());
        assert!(InterferenceSetup::two_beam(200.0, 1.5, 0.0).is_err());
        assert!(InterferenceSetup::two_beam(200.0, 1.5, 90.0).is_err());
    }
}
