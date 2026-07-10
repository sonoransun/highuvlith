//! Volumetric (z-resolved) resist exposure and 3D development.
//!
//! Extends the depth-averaged 2D path in [`crate::resist`] to a full
//! `(x, y, z)` latent image: the lateral aerial image is combined with
//! the exact thin-film intensity profile through the resist depth, the
//! Dill exposure is integrated with optional split-step bleaching, and
//! development runs either as a cheap per-column threshold or as a real
//! 3D fast-marching etch front that produces sidewall profiles.
//!
//! # Exposure model (separable approximation)
//!
//! ```text
//!   I(x, y, z) = I_aer(x, y; d0 + z/n_r) * S(z)
//! ```
//!
//! `I_aer` is the in-air aerial image refocused paraxially to depth `z`
//! (computed at a handful of defocus planes and interpolated in z), and
//! `S(z)` is the exact transfer-matrix intensity at unit incidence from
//! [`crate::thinfilm::FilmStack::intensity_profile`] — standing waves,
//! absorption, and interface effects included, so nothing is counted
//! twice. Bleaching uses split-step Dill: the total dose is applied in
//! steps, and between steps the resist layer is re-sliced into
//! sublayers whose extinction follows the laterally averaged PAC.
//!
//! # Model status
//!
//! Simplified (documented approximations): lateral imaging and vertical
//! film response are decoupled (no vector in-film imaging); the focus
//! mapping into the resist is the paraxial `z / n_r`; bleaching updates
//! use the lateral-mean PAC per sublayer because the 1D transfer matrix
//! cannot carry laterally varying extinction. The fast-marching
//! development front assumes the rate field is frozen during develop
//! (standard isotropic wet-etch assumption); a moving-boundary
//! (level-set) model is planned.
//!
//! # Memory
//!
//! A `Grid3D<f64>` at 512×512×128 is 268 MB. Prefer lateral 256² and
//! `nz` = 64 unless you need more.

use ndarray::{Array2, Array3};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::aerial::AerialImageEngine;
use crate::error::{LithographyError, Result};
use crate::mask::Mask;
use crate::resist::ResistParams;
use crate::thinfilm::{FilmLayer, FilmStack};
use crate::types::{Grid2D, Grid3D, Polarization};

/// Volumetric latent image: normalized PAC concentration on a 3D grid.
/// `pac.data[[k, i, j]]` is depth slice k (z from resist top, downward),
/// row i (y), column j (x); m = 1 unexposed, m = 0 fully exposed.
pub struct VolumetricLatentImage {
    pub pac: Grid3D<f64>,
}

/// Configuration for [`expose_volumetric`].
#[derive(Debug, Clone)]
pub struct VolumetricExposureConfig {
    /// Exposure dose in mJ/cm².
    pub dose_mj_cm2: f64,
    /// Number of z slices through the resist.
    pub nz: usize,
    /// Number of defocus planes at which the aerial image is computed
    /// (interpolated in between). 8 is a good default; 1 reuses a single
    /// plane for all depths.
    pub n_defocus_planes: usize,
    /// Split-step Dill dose steps. 1 = static absorption (fine for thin,
    /// weakly bleaching resists); 5–20 for thick / strongly bleaching.
    pub dose_steps: usize,
    /// Focus setting (nm) of the aerial image at the resist top.
    pub base_defocus_nm: f64,
    /// Index of the resist layer inside the film stack.
    pub resist_layer: usize,
}

impl Default for VolumetricExposureConfig {
    fn default() -> Self {
        Self {
            dose_mj_cm2: 30.0,
            nz: 64,
            n_defocus_planes: 8,
            dose_steps: 1,
            base_defocus_nm: 0.0,
            resist_layer: 0,
        }
    }
}

/// Compute a z-resolved latent image through the resist depth.
///
/// See the module docs for the model. The `film_stack` must contain the
/// resist as layer `config.resist_layer`; layers above it (e.g. a top
/// coat) shift the resist in z and attenuate the field exactly via the
/// transfer matrix.
pub fn expose_volumetric(
    engine: &AerialImageEngine,
    mask: &Mask,
    film_stack: &FilmStack,
    resist: &ResistParams,
    wavelength_nm: f64,
    config: &VolumetricExposureConfig,
) -> Result<VolumetricLatentImage> {
    if config.resist_layer >= film_stack.layers.len() {
        return Err(LithographyError::InvalidParameter {
            name: "resist_layer",
            value: config.resist_layer as f64,
            reason: "must index an existing film-stack layer",
        });
    }
    if config.nz == 0 {
        return Err(LithographyError::InvalidParameter {
            name: "nz",
            value: 0.0,
            reason: "must be positive",
        });
    }
    if config.dose_steps == 0 {
        return Err(LithographyError::InvalidParameter {
            name: "dose_steps",
            value: 0.0,
            reason: "must be positive",
        });
    }
    if config.n_defocus_planes == 0 {
        return Err(LithographyError::InvalidParameter {
            name: "n_defocus_planes",
            value: 0.0,
            reason: "must be positive",
        });
    }

    let grid = engine.grid();
    let n = grid.size;
    let field = grid.field_size_nm();
    let half = field / 2.0;

    let resist_thickness = film_stack.layers[config.resist_layer].thickness_nm;
    let n_resist = film_stack.layers[config.resist_layer].n.re;
    let z_offset: f64 = film_stack.layers[..config.resist_layer]
        .iter()
        .map(|l| l.thickness_nm)
        .sum();

    // ---- Aerial images at stepped defocus planes (computed once). ----
    let transmittance = mask.rasterize(grid);
    let nz = config.nz;
    let dz = resist_thickness / nz as f64;
    let z_centers: Vec<f64> = (0..nz).map(|k| (k as f64 + 0.5) * dz).collect();

    let n_planes = config.n_defocus_planes.min(nz).max(1);
    let plane_z: Vec<f64> = if n_planes == 1 {
        vec![resist_thickness / 2.0]
    } else {
        (0..n_planes)
            .map(|p| p as f64 * resist_thickness / (n_planes - 1) as f64)
            .collect()
    };
    let plane_images: Vec<Array2<f64>> = plane_z
        .iter()
        .map(|&z| {
            let defocus = config.base_defocus_nm + z / n_resist;
            engine
                .compute_from_transmittance(&transmittance, defocus)
                .data
        })
        .collect();

    // Per z-slice: bracketing plane indices and interpolation weight.
    let slice_interp: Vec<(usize, usize, f64)> = z_centers
        .iter()
        .map(|&z| {
            if n_planes == 1 {
                return (0, 0, 0.0);
            }
            let pos = z / resist_thickness * (n_planes - 1) as f64;
            let lo = (pos.floor() as usize).min(n_planes - 2);
            (lo, lo + 1, pos - lo as f64)
        })
        .collect();

    // ---- Split-step Dill exposure. ----
    let mut pac = Grid3D::<f64>::new(
        n,
        n,
        nz,
        (-half, half),
        (-half, half),
        (0.0, resist_thickness),
    )?;
    pac.data.fill(1.0);

    let delta_dose = config.dose_mj_cm2 / config.dose_steps as f64;
    let absolute_z: Vec<f64> = z_centers.iter().map(|&z| z_offset + z).collect();

    for _step in 0..config.dose_steps {
        // Re-slice the resist layer with extinction from the lateral
        // mean PAC of each sublayer, and get the exact field profile.
        let mean_pac: Vec<f64> = (0..nz)
            .map(|k| {
                let slice = pac.data.index_axis(ndarray::Axis(0), k);
                slice.mean().unwrap_or(1.0)
            })
            .collect();
        let stack = substack_with_sliced_resist(
            film_stack,
            config.resist_layer,
            resist,
            n_resist,
            wavelength_nm,
            dz,
            &mean_pac,
        );
        let s_profile =
            stack.intensity_profile(wavelength_nm, 0.0, Polarization::Unpolarized, &absolute_z);

        for k in 0..nz {
            let (lo, hi, t) = slice_interp[k];
            let s_z = s_profile[k];
            let mut slice = pac.data.index_axis_mut(ndarray::Axis(0), k);
            for i in 0..n {
                for j in 0..n {
                    let aerial =
                        (1.0 - t) * plane_images[lo][[i, j]] + t * plane_images[hi][[i, j]];
                    let intensity = aerial * s_z;
                    slice[[i, j]] *= (-resist.dill_c * delta_dose * intensity).exp();
                }
            }
        }
    }

    Ok(VolumetricLatentImage { pac })
}

/// Rebuild the film stack with the resist layer replaced by `nz`
/// sublayers whose extinction follows the per-slice mean PAC
/// (`alpha = (A·m̄ + B)` in 1/µm → k = alpha·lambda/(4π)).
fn substack_with_sliced_resist(
    film_stack: &FilmStack,
    resist_layer: usize,
    resist: &ResistParams,
    n_resist: f64,
    wavelength_nm: f64,
    dz: f64,
    mean_pac: &[f64],
) -> FilmStack {
    let mut layers: Vec<FilmLayer> = Vec::with_capacity(film_stack.layers.len() + mean_pac.len());
    layers.extend_from_slice(&film_stack.layers[..resist_layer]);
    for (k, &m) in mean_pac.iter().enumerate() {
        let alpha_per_nm = resist.absorption(m); // (A·m + B)·1e-3, in 1/nm
        let kappa = alpha_per_nm * wavelength_nm / (4.0 * std::f64::consts::PI);
        layers.push(FilmLayer {
            name: format!("resist_sub_{k}"),
            thickness_nm: dz,
            n: crate::types::Complex64::new(n_resist, kappa),
        });
    }
    layers.extend_from_slice(&film_stack.layers[resist_layer + 1..]);
    FilmStack {
        layers,
        substrate: film_stack.substrate,
        superstrate: film_stack.superstrate,
    }
}

/// 3D post-exposure bake: separable Gaussian diffusion of the latent
/// image along x, y, and z with clamped boundaries (isotropic acid
/// diffusion; z-anisotropy is planned).
pub fn peb_diffuse_3d(
    latent: &mut VolumetricLatentImage,
    diffusion_nm: f64,
    pixel_xy_nm: f64,
    pixel_z_nm: f64,
) {
    if diffusion_nm <= 0.0 {
        return;
    }
    gaussian_pass(&mut latent.pac.data, 2, diffusion_nm / pixel_xy_nm);
    gaussian_pass(&mut latent.pac.data, 1, diffusion_nm / pixel_xy_nm);
    gaussian_pass(&mut latent.pac.data, 0, diffusion_nm / pixel_z_nm);
}

/// One clamped-boundary Gaussian convolution pass along the given axis.
fn gaussian_pass(data: &mut Array3<f64>, axis: usize, sigma_px: f64) {
    if sigma_px <= 0.0 {
        return;
    }
    let radius = (3.0 * sigma_px).ceil() as i64;
    let mut kernel = Vec::with_capacity((2 * radius + 1) as usize);
    let mut sum = 0.0;
    for d in -radius..=radius {
        let w = (-(d as f64) * (d as f64) / (2.0 * sigma_px * sigma_px)).exp();
        kernel.push(w);
        sum += w;
    }
    for w in &mut kernel {
        *w /= sum;
    }

    let dim = data.dim();
    let len = [dim.0, dim.1, dim.2][axis] as i64;
    let mut line = vec![0.0; len as usize];

    // Iterate over all lines along `axis`.
    let (a, b) = match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
    let (na, nb) = ([dim.0, dim.1, dim.2][a], [dim.0, dim.1, dim.2][b]);
    for ia in 0..na {
        for ib in 0..nb {
            for t in 0..len {
                let idx = index3(axis, t as usize, a, ia, b, ib);
                line[t as usize] = data[idx];
            }
            for t in 0..len {
                let mut val = 0.0;
                for (ki, w) in kernel.iter().enumerate() {
                    let s = (t + ki as i64 - radius).clamp(0, len - 1);
                    val += line[s as usize] * w;
                }
                let idx = index3(axis, t as usize, a, ia, b, ib);
                data[idx] = val;
            }
        }
    }
}

fn index3(
    axis: usize,
    t: usize,
    a: usize,
    ia: usize,
    b: usize,
    ib: usize,
) -> (usize, usize, usize) {
    let mut idx = [0usize; 3];
    idx[axis] = t;
    idx[a] = ia;
    idx[b] = ib;
    (idx[0], idx[1], idx[2])
}

/// Tier-1 development: per-column threshold depth map.
///
/// Scans each (x, y) column from the resist top and returns the depth
/// (nm) of contiguous development — the z where the first "blocking"
/// voxel (PAC ≥ threshold, i.e. insufficiently exposed) is met.
/// Adequate for LIGA and grayscale; no lateral etching or undercut
/// (use [`develop_fast_marching`] for those).
pub fn develop_depth_map(latent: &VolumetricLatentImage, threshold: f64) -> Grid2D<f64> {
    let (nz, ny, nx) = latent.pac.data.dim();
    let dz = latent.pac.pixel_size_z();
    let mut depth = Array2::zeros((ny, nx));
    for i in 0..ny {
        for j in 0..nx {
            let mut d = 0.0;
            for k in 0..nz {
                if latent.pac.data[[k, i, j]] < threshold {
                    d = (k + 1) as f64 * dz;
                } else {
                    break;
                }
            }
            depth[[i, j]] = d;
        }
    }
    Grid2D {
        data: depth,
        x_min_nm: latent.pac.x_min_nm,
        x_max_nm: latent.pac.x_max_nm,
        y_min_nm: latent.pac.y_min_nm,
        y_max_nm: latent.pac.y_max_nm,
    }
}

/// Heap entry for the fast-marching method (min-heap via reversed Ord).
struct FmmNode {
    time: f64,
    idx: (usize, usize, usize),
}

impl PartialEq for FmmNode {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}
impl Eq for FmmNode {}
impl PartialOrd for FmmNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for FmmNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse: BinaryHeap is a max-heap, we want smallest time first.
        other.time.total_cmp(&self.time)
    }
}

/// Tier-2 development: fast-marching solution of the Eikonal equation
/// `|∇T(x,y,z)| = 1 / R(x,y,z)` with the developer arriving at the
/// resist top surface at T = 0. `R` is the Mack/threshold development
/// rate (nm/s) evaluated on the latent image. Returns per-voxel arrival
/// times in seconds; the developed region after `t` seconds is
/// `{ T <= t }`, including lateral etching and undercut.
///
/// First-order Godunov upwind updates on the anisotropic grid
/// (`pixel_xy_nm` laterally, `pixel_z_nm` in depth); the rate field is
/// frozen during development (isotropic wet-etch assumption).
pub fn develop_fast_marching(
    latent: &VolumetricLatentImage,
    params: &ResistParams,
    pixel_xy_nm: f64,
    pixel_z_nm: f64,
) -> Grid3D<f64> {
    let (nz, ny, nx) = latent.pac.data.dim();

    // Development rate per voxel (nm/s).
    let rate = latent.pac.data.mapv(|m| params.development.rate(m));

    let mut times = Grid3D::<f64> {
        data: Array3::from_elem((nz, ny, nx), f64::INFINITY),
        x_min_nm: latent.pac.x_min_nm,
        x_max_nm: latent.pac.x_max_nm,
        y_min_nm: latent.pac.y_min_nm,
        y_max_nm: latent.pac.y_max_nm,
        z_min_nm: latent.pac.z_min_nm,
        z_max_nm: latent.pac.z_max_nm,
    };
    let mut accepted = Array3::from_elem((nz, ny, nx), false);
    let mut heap: BinaryHeap<FmmNode> = BinaryHeap::new();

    // Seed: developer reaches the top voxel centers after etching half
    // a cell from the surface.
    for i in 0..ny {
        for j in 0..nx {
            let t0 = 0.5 * pixel_z_nm / rate[[0, i, j]];
            times.data[[0, i, j]] = t0;
            heap.push(FmmNode {
                time: t0,
                idx: (0, i, j),
            });
        }
    }

    let h = [pixel_z_nm, pixel_xy_nm, pixel_xy_nm];

    while let Some(FmmNode { time, idx }) = heap.pop() {
        let (k, i, j) = idx;
        if accepted[[k, i, j]] {
            continue;
        }
        accepted[[k, i, j]] = true;
        let _ = time;

        // Relax the six neighbors.
        let neighbors = [
            (k.wrapping_sub(1), i, j),
            (k + 1, i, j),
            (k, i.wrapping_sub(1), j),
            (k, i + 1, j),
            (k, i, j.wrapping_sub(1)),
            (k, i, j + 1),
        ];
        for &(nk, ni, nj) in &neighbors {
            if nk >= nz || ni >= ny || nj >= nx || accepted[[nk, ni, nj]] {
                continue;
            }
            let new_time = godunov_update(&times.data, (nk, ni, nj), &h, rate[[nk, ni, nj]]);
            if new_time < times.data[[nk, ni, nj]] {
                times.data[[nk, ni, nj]] = new_time;
                heap.push(FmmNode {
                    time: new_time,
                    idx: (nk, ni, nj),
                });
            }
        }
    }

    times
}

/// First-order Godunov upwind update: solve
/// `sum_axes max(0, (T - T_axis_min) / h_axis)^2 = 1 / R^2`
/// using the standard sorted-quadratic construction.
fn godunov_update(times: &Array3<f64>, idx: (usize, usize, usize), h: &[f64; 3], rate: f64) -> f64 {
    let (k, i, j) = idx;
    let dim = times.dim();
    let axis_min = |axis: usize| -> f64 {
        let (lo, hi) = match axis {
            0 => (
                if k > 0 {
                    times[[k - 1, i, j]]
                } else {
                    f64::INFINITY
                },
                if k + 1 < dim.0 {
                    times[[k + 1, i, j]]
                } else {
                    f64::INFINITY
                },
            ),
            1 => (
                if i > 0 {
                    times[[k, i - 1, j]]
                } else {
                    f64::INFINITY
                },
                if i + 1 < dim.1 {
                    times[[k, i + 1, j]]
                } else {
                    f64::INFINITY
                },
            ),
            _ => (
                if j > 0 {
                    times[[k, i, j - 1]]
                } else {
                    f64::INFINITY
                },
                if j + 1 < dim.2 {
                    times[[k, i, j + 1]]
                } else {
                    f64::INFINITY
                },
            ),
        };
        lo.min(hi)
    };

    // Candidate (value, spacing) pairs sorted by value.
    let mut cands: Vec<(f64, f64)> = (0..3)
        .map(|a| (axis_min(a), h[a]))
        .filter(|(v, _)| v.is_finite())
        .collect();
    if cands.is_empty() {
        return f64::INFINITY;
    }
    cands.sort_by(|a, b| a.0.total_cmp(&b.0));

    let inv_r = 1.0 / rate;
    let mut best = f64::INFINITY;
    // Try solutions using the m smallest candidates, m = 1..=len.
    for m in 1..=cands.len() {
        // Solve sum_{i<m} ((T - v_i)/h_i)^2 = inv_r^2 for T.
        let mut a = 0.0;
        let mut b = 0.0;
        let mut c = -inv_r * inv_r;
        for &(v, hh) in &cands[..m] {
            let w = 1.0 / (hh * hh);
            a += w;
            b += -2.0 * v * w;
            c += v * v * w;
        }
        let disc = b * b - 4.0 * a * c;
        if disc < 0.0 {
            continue;
        }
        let t = (-b + disc.sqrt()) / (2.0 * a);
        // Valid if T exceeds every candidate used (upwind condition) and
        // does not exceed the next unused candidate.
        if t >= cands[m - 1].0 && (m == cands.len() || t <= cands[m].0) {
            best = t;
            break;
        }
    }
    if best.is_infinite() {
        // Fallback: one-sided update from the smallest neighbor.
        best = cands[0].0 + cands[0].1 * inv_r;
    }
    best
}

/// Remaining-height map after `dev_time_s` seconds of development, from
/// fast-marching arrival times: per column, the deepest voxel reached
/// within the time defines the removed depth. (Undercut information is
/// in the full 3D `times` volume; a height map cannot represent it.)
pub fn height_map_from_times(times: &Grid3D<f64>, dev_time_s: f64) -> Grid2D<f64> {
    let (nz, ny, nx) = times.data.dim();
    let dz = times.pixel_size_z();
    let thickness = times.z_max_nm - times.z_min_nm;
    let mut height = Array2::zeros((ny, nx));
    for i in 0..ny {
        for j in 0..nx {
            let mut removed = 0.0;
            for k in 0..nz {
                if times.data[[k, i, j]] <= dev_time_s {
                    removed = (k + 1) as f64 * dz;
                }
            }
            height[[i, j]] = (thickness - removed).max(0.0);
        }
    }
    Grid2D {
        data: height,
        x_min_nm: times.x_min_nm,
        x_max_nm: times.x_max_nm,
        y_min_nm: times.y_min_nm,
        y_max_nm: times.y_max_nm,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mask::MaskType;
    use crate::optics::ProjectionOptics;
    use crate::resist::DevelopmentModel;
    use crate::source::VuvSource;
    use crate::types::{Complex64, GridConfig};
    use approx::assert_relative_eq;

    fn uniform_clear_mask() -> Mask {
        Mask {
            mask_type: MaskType::Binary,
            features: vec![],
            dark_field: false,
        }
    }

    fn small_engine() -> AerialImageEngine {
        let source = VuvSource::f2_laser(0.5).unwrap();
        let optics = ProjectionOptics::new(0.75).unwrap();
        let grid = GridConfig {
            size: 32,
            pixel_nm: 4.0,
        };
        AerialImageEngine::new(&source, &optics, grid, 8).unwrap()
    }

    /// Index-matched, non-bleaching resist under a uniform clear mask:
    /// m(z) must follow the closed form exp(-C * D * I0 * e^{-alpha z}).
    #[test]
    fn test_closed_form_beer_lambert_exposure() {
        let engine = small_engine();
        let mask = uniform_clear_mask();

        let wavelength = 157.63;
        let resist = ResistParams {
            thickness_nm: 300.0,
            dill_a: 0.0, // no bleaching -> alpha independent of m
            dill_b: 0.9,
            dill_c: 0.02,
            peb_diffusion_nm: 0.0,
            development: DevelopmentModel::default(),
        };
        let alpha = resist.absorption(1.0); // 1/nm
        let kappa = alpha * wavelength / (4.0 * std::f64::consts::PI);

        // Index-matched stack: superstrate, resist, substrate all n = 1
        // (+ the same k in resist and substrate) -> no reflections, pure
        // Beer-Lambert decay.
        let stack = FilmStack {
            layers: vec![FilmLayer {
                name: "resist".to_string(),
                thickness_nm: 300.0,
                n: Complex64::new(1.0, kappa),
            }],
            substrate: Complex64::new(1.0, kappa),
            superstrate: Complex64::new(1.0, 0.0),
        };

        let config = VolumetricExposureConfig {
            dose_mj_cm2: 30.0,
            nz: 32,
            n_defocus_planes: 1,
            dose_steps: 1,
            base_defocus_nm: 0.0,
            resist_layer: 0,
        };
        let latent =
            expose_volumetric(&engine, &mask, &stack, &resist, wavelength, &config).unwrap();

        // Uniform clear mask -> aerial intensity ~1 everywhere.
        let (nz, ny, nx) = latent.pac.data.dim();
        let center = (ny / 2, nx / 2);
        for k in [0, nz / 4, nz / 2, nz - 1] {
            let z = latent.pac.z_at(k);
            let expected = (-resist.dill_c * 30.0 * (-alpha * z).exp()).exp();
            let actual = latent.pac.data[[k, center.0, center.1]];
            assert_relative_eq!(actual, expected, epsilon = 0.02);
        }
    }

    /// The z-mean of the exact S(z) must agree with the 2D path's
    /// depth-averaged coupling factor for the same non-bleaching stack.
    #[test]
    fn test_z_mean_matches_effective_coupling() {
        let wavelength = 157.63;
        let resist = ResistParams {
            thickness_nm: 300.0,
            dill_a: 0.0,
            dill_b: 0.9,
            dill_c: 0.02,
            peb_diffusion_nm: 0.0,
            development: DevelopmentModel::default(),
        };
        let alpha = resist.absorption(1.0);
        let kappa = alpha * wavelength / (4.0 * std::f64::consts::PI);
        let stack = FilmStack {
            layers: vec![FilmLayer {
                name: "resist".to_string(),
                thickness_nm: 300.0,
                n: Complex64::new(1.0, kappa),
            }],
            substrate: Complex64::new(1.0, kappa),
            superstrate: Complex64::new(1.0, 0.0),
        };

        let nz = 128;
        let z: Vec<f64> = (0..nz)
            .map(|k| (k as f64 + 0.5) * 300.0 / nz as f64)
            .collect();
        let s = stack.intensity_profile(wavelength, 0.0, Polarization::Unpolarized, &z);
        let mean_s: f64 = s.iter().sum::<f64>() / nz as f64;

        // effective_coupling = (1 - e^{-alpha d}) / (alpha d)
        let expected = (1.0 - (-alpha * 300.0).exp()) / (alpha * 300.0);
        assert_relative_eq!(mean_s, expected, epsilon = 1e-3);
    }

    #[test]
    fn test_dose_monotonicity_and_bounds() {
        let engine = small_engine();
        let mask = Mask::line_space(64.0, 128.0).unwrap();
        let resist = ResistParams::default();
        let stack = FilmStack::default();

        let low = expose_volumetric(
            &engine,
            &mask,
            &stack,
            &resist,
            157.63,
            &VolumetricExposureConfig {
                dose_mj_cm2: 10.0,
                nz: 8,
                n_defocus_planes: 2,
                ..Default::default()
            },
        )
        .unwrap();
        let high = expose_volumetric(
            &engine,
            &mask,
            &stack,
            &resist,
            157.63,
            &VolumetricExposureConfig {
                dose_mj_cm2: 40.0,
                nz: 8,
                n_defocus_planes: 2,
                ..Default::default()
            },
        )
        .unwrap();

        for (l, h) in low.pac.data.iter().zip(high.pac.data.iter()) {
            assert!((0.0..=1.0).contains(l));
            assert!((0.0..=1.0).contains(h));
            assert!(h <= l, "higher dose must not raise PAC");
        }
    }

    #[test]
    fn test_split_step_converges_toward_more_exposure_at_depth() {
        // With strong bleaching (large A), split-stepping lets the top
        // bleach and transmit more light downward: deep PAC should be
        // LOWER (more exposed) with more dose steps.
        let engine = small_engine();
        let mask = uniform_clear_mask();
        let resist = ResistParams {
            thickness_nm: 400.0,
            dill_a: 5.0, // strongly bleaching
            dill_b: 0.05,
            dill_c: 0.05,
            peb_diffusion_nm: 0.0,
            development: DevelopmentModel::default(),
        };
        let stack = FilmStack {
            layers: vec![FilmLayer {
                name: "resist".to_string(),
                thickness_nm: 400.0,
                n: Complex64::new(1.65, 0.02),
            }],
            substrate: Complex64::new(1.65, 0.02),
            superstrate: Complex64::new(1.0, 0.0),
        };
        let base = VolumetricExposureConfig {
            dose_mj_cm2: 60.0,
            nz: 16,
            n_defocus_planes: 1,
            dose_steps: 1,
            base_defocus_nm: 0.0,
            resist_layer: 0,
        };
        let static_ss = expose_volumetric(&engine, &mask, &stack, &resist, 157.63, &base).unwrap();
        let stepped = expose_volumetric(
            &engine,
            &mask,
            &stack,
            &resist,
            157.63,
            &VolumetricExposureConfig {
                dose_steps: 10,
                ..base
            },
        )
        .unwrap();

        let (nz, ny, nx) = static_ss.pac.data.dim();
        let bottom_static = static_ss.pac.data[[nz - 1, ny / 2, nx / 2]];
        let bottom_stepped = stepped.pac.data[[nz - 1, ny / 2, nx / 2]];
        assert!(
            bottom_stepped < bottom_static,
            "bleaching must open the resist at depth: static {bottom_static}, stepped {bottom_stepped}"
        );
    }

    #[test]
    fn test_develop_depth_map_synthetic() {
        // Synthetic PAC: left half fully exposed to depth 3 slices,
        // right half unexposed.
        let mut pac = Grid3D::<f64>::new(4, 4, 8, (0.0, 4.0), (0.0, 4.0), (0.0, 80.0)).unwrap();
        pac.data.fill(1.0);
        for k in 0..3 {
            for i in 0..4 {
                for j in 0..2 {
                    pac.data[[k, i, j]] = 0.0;
                }
            }
        }
        let latent = VolumetricLatentImage { pac };
        let depth = develop_depth_map(&latent, 0.5);
        // dz = 10 nm; 3 developed slices -> 30 nm.
        assert_relative_eq!(depth.data[[0, 0]], 30.0, epsilon = 1e-9);
        assert_relative_eq!(depth.data[[0, 3]], 0.0, epsilon = 1e-9);
    }

    #[test]
    fn test_fmm_uniform_rate_front() {
        // Uniform fully-exposed resist with threshold development:
        // rate = 1000 nm/s everywhere; the front at depth z arrives at
        // t = z / R.
        let mut pac = Grid3D::<f64>::new(8, 8, 32, (0.0, 8.0), (0.0, 8.0), (0.0, 320.0)).unwrap();
        pac.data.fill(0.0); // fully exposed
        let latent = VolumetricLatentImage { pac };
        let params = ResistParams {
            development: DevelopmentModel::Threshold { threshold: 0.5 },
            ..ResistParams::default()
        };
        let times = develop_fast_marching(&latent, &params, 1.0, 10.0);
        let rate = 1000.0;
        for k in 0..32 {
            let z = (k as f64 + 0.5) * 10.0;
            let t = times.data[[k, 4, 4]];
            let tol = 0.02 * (z.max(10.0) / rate);
            assert!(
                (t - z / rate).abs() < tol,
                "depth {z}: expected {} got {t}",
                z / rate
            );
        }

        // Height map: after enough time for 160 nm, half the resist is gone.
        let hm = height_map_from_times(&times, 160.0 / rate);
        assert!((hm.data[[4, 4]] - 160.0).abs() <= 10.0 + 1e-9);
    }

    #[test]
    fn test_fmm_blocked_column_stays_undeveloped() {
        // Unexposed resist (m = 1) with threshold model develops at the
        // trickle rate only: arrival times far exceed exposed ones.
        let mut pac = Grid3D::<f64>::new(4, 4, 8, (0.0, 4.0), (0.0, 4.0), (0.0, 80.0)).unwrap();
        pac.data.fill(1.0);
        // One exposed column.
        for k in 0..8 {
            pac.data[[k, 2, 2]] = 0.0;
        }
        let latent = VolumetricLatentImage { pac };
        let params = ResistParams {
            development: DevelopmentModel::Threshold { threshold: 0.5 },
            ..ResistParams::default()
        };
        let times = develop_fast_marching(&latent, &params, 1.0, 10.0);
        // Bottom of the exposed column develops quickly...
        let t_open = times.data[[7, 2, 2]];
        // ...an unexposed corner column does not.
        let t_blocked = times.data[[7, 0, 0]];
        assert!(
            t_blocked > 10.0 * t_open,
            "blocked {t_blocked} vs open {t_open}"
        );
    }

    #[test]
    fn test_peb_3d_reduces_variance_preserves_bounds() {
        let mut pac = Grid3D::<f64>::new(8, 8, 8, (0.0, 8.0), (0.0, 8.0), (0.0, 80.0)).unwrap();
        // Checkerboard-ish pattern.
        for k in 0..8 {
            for i in 0..8 {
                for j in 0..8 {
                    pac.data[[k, i, j]] = ((k + i + j) % 2) as f64;
                }
            }
        }
        let mut latent = VolumetricLatentImage { pac };
        let variance = |d: &Array3<f64>| {
            let mean = d.mean().unwrap();
            d.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / d.len() as f64
        };
        let v0 = variance(&latent.pac.data);
        peb_diffuse_3d(&mut latent, 15.0, 10.0, 10.0);
        let v1 = variance(&latent.pac.data);
        assert!(v1 < v0, "diffusion must reduce variance");
        for v in latent.pac.data.iter() {
            assert!((0.0..=1.0).contains(v));
        }
    }

    #[test]
    fn test_invalid_config_rejected() {
        let engine = small_engine();
        let mask = uniform_clear_mask();
        let resist = ResistParams::default();
        let stack = FilmStack::default();
        for bad in [
            VolumetricExposureConfig {
                resist_layer: 5,
                ..Default::default()
            },
            VolumetricExposureConfig {
                nz: 0,
                ..Default::default()
            },
            VolumetricExposureConfig {
                dose_steps: 0,
                ..Default::default()
            },
        ] {
            assert!(
                expose_volumetric(&engine, &mask, &stack, &resist, 157.63, &bad).is_err(),
                "config should be rejected: {bad:?}"
            );
        }
    }
}
