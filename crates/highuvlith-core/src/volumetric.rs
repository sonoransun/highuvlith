//! Volumetric (z-resolved) resist exposure, 3D post-exposure bake, and 3D
//! development.
//!
//! Extends the depth-averaged 2D path in [`crate::resist`] to a full
//! `(x, y, z)` latent image: the lateral aerial image is combined with
//! the exact thin-film intensity profile through the resist depth, the
//! Dill exposure is integrated with optional split-step bleaching, the
//! bake is either exact anisotropic Gaussian diffusion or a chemically
//! amplified acid/quencher reaction–diffusion model ([`PebModel`]), and
//! development runs as a cheap per-column threshold, as a fast-marching
//! arrival-time solve for static dissolution-rate fields, or as a
//! level-set moving boundary whose rate is re-evaluated at the front at
//! every time step.
//!
//! # Exposure model (separable approximation)
//!
//! ```text
//!   I(x, y, z) = I_aer(x, y; d0 + z/n_r) * S(z)
//! ```
//!
//! `I_aer` is the in-air aerial image refocused paraxially to depth `z`
//! (computed at a handful of defocus planes and interpolated in z), and
//! `S(z) = (n_r / n_0)·|E(z)|²` is the exposing intensity relative to the
//! incident intensity, with `|E(z)|²` the exact transfer-matrix field
//! intensity at unit incident field from
//! [`crate::thinfilm::FilmStack::intensity_profile`] — standing waves,
//! absorption, and interface effects included, so nothing is counted
//! twice. The `n_r / n_0` factor (real parts of the resist and superstrate
//! indices) converts field intensity to energy flow: the absorbed power
//! density is `α·n_r·|E|²` per unit incident `n_0·|E_0|²`, so a lossless
//! entrance gives `S(0⁺) = T = 4 n_0 n_r / (n_0 + n_r)²` for a pure
//! travelling wave, as the Dill C constant (defined against intensity)
//! requires. Bleaching uses split-step Dill: the total dose is applied in
//! steps, and between steps the resist layer is re-sliced into
//! sublayers whose extinction follows the laterally averaged PAC. All
//! aerial imaging goes through one internal helper (one image per focus
//! plane), so a batched through-focus engine call can replace the loop
//! without touching the exposure integrator.
//!
//! # Key equations
//!
//! ```text
//!   Exposure     m ← m · exp(−C · ΔD · I_aer(x,y; d0 + z/n_r) · S(z)),
//!                S(z) = (Re n_r / Re n_0) · |E(z)|²   (|E_0| = 1 incident)
//!   PEB          Gaussian: σ_xy, σ_z (optional D_z(z)), exact kernels     [resist.rs]
//!                CAR:      ∂m/∂t = −k_amp·m·h,  ∂h/∂t = D_h∇²h − k_q·h·q,
//!                          ∂q/∂t = D_q∇²q − k_q·h·q,  h(0) = 1 − m_exposure
//!   Dissolution  R(x, t) = R_bulk(m(x)) · f_inh(d) · g(t)
//!                f_inh(d) = 1 − (1 − r_s) · exp(−d / δ_inh)   d = depth below the ORIGINAL top
//!                g(t) = 1 | exp(−t/τ_dev) | max(0, 1 − h̄/h_cap)   (h̄ global or Gaussian-local)
//!   FMM          |∇T| = 1 / R                         (static R; T = arrival time)
//!   Level set    φ_t + R |∇φ| = 0,   φ(x, 0) = depth below the top surface
//!                |∇φ| ≈ sqrt(Σ_a max(D⁻_a φ, −D⁺_a φ, 0)²)          (Godunov upwind)
//!                Δt = CFL / (max_band R · sqrt(Σ_a 1/h_a²))
//! ```
//!
//! # Model status
//!
//! Simplified (documented approximations): lateral imaging and vertical
//! film response are decoupled (no vector in-film imaging); the focus
//! mapping into the resist is the paraxial `z / n_r`; bleaching updates
//! use the lateral-mean PAC per sublayer because the 1D transfer matrix
//! cannot carry laterally varying extinction.
//!
//! Correction (2026-10-01): before this date `S(z)` was the bare field
//! intensity `|E(z)|²` without the `n_r / n_0` factor, so every resist with
//! `n_r ≠ n_0` was under-exposed by `1/n_r` (×0.61 for the default
//! `n_r = 1.65` fluoropolymer resist in vacuum). Index-matched (`n_r = 1`)
//! results are unchanged.
//!
//! Development: the fast-marching tier needs a static rate field (it does
//! accept the static surface-inhibition factor). The level-set tier is a
//! first-order Hamilton–Jacobi solver (Godunov upwind in space, forward
//! Euler in time with a CFL-limited step; the exponential developer-ageing
//! factor is integrated exactly through a pseudo-time), with narrow-band
//! updates, periodic fast-marching reinitialization to a signed distance,
//! and extension speeds behind the front. Surface inhibition uses the
//! standard exponential inhibition-layer form; developer ageing and
//! loading are phenomenological first-order models with user-supplied
//! constants, not fitted developer chemistry. Every rate factor provided
//! here is separable into a static spatial field times a time factor, so
//! the static and exponential-ageing cases are also reachable with FMM (plus
//! a time reparametrization) — the level set agrees with FMM within grid
//! error there — while the loading factors depend on the developed
//! geometry's history (and, for local loading, vary laterally) and need the
//! moving boundary.
//!
//! # Memory
//!
//! A `Grid3D<f64>` at 512×512×128 is 268 MB. Prefer lateral 256² and
//! `nz` = 64 unless you need more. The level set holds about five
//! volume-sized `f64` buffers.
//!
//! # References
//!
//! - F. H. Dill et al., IEEE Trans. Electron Devices (1975) — exposure.
//! - C. A. Mack, *Fundamental Principles of Optical Lithography*, Wiley
//!   (2007) — PEB, surface inhibition, development-rate models.
//! - J. A. Sethian, *Level Set Methods and Fast Marching Methods*,
//!   Cambridge University Press (1999) — upwind level-set schemes,
//!   fast marching, reinitialization, and extension velocities.
//! - S. Osher, J. A. Sethian, "Fronts propagating with curvature-dependent
//!   speed: algorithms based on Hamilton–Jacobi formulations," J. Comput.
//!   Phys. 79, 12 (1988) — the level-set method.

use ndarray::{Array2, Array3};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::aerial::AerialImageEngine;
use crate::error::{LithographyError, Result};
use crate::mask::Mask;
use crate::resist::{self, CarParams, CarPebResult, DiffusionBoundary, PebDiffusion, ResistParams};
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
    /// Number of defocus planes at which the aerial image is computed.
    /// Fewer planes than slices: planes evenly span the resist and slices
    /// interpolate linearly between the two bracketing planes (8 is a good
    /// default; 1 reuses a single mid-thickness plane for all depths).
    /// `n_defocus_planes >= nz`: every slice is imaged exactly at its own
    /// centre depth (one focus-exact image per slice).
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

/// Aerial images of `mask` at each requested defocus (nm), in order.
///
/// This is the only place where the volumetric path calls the imaging
/// engine. It uses the engine's batched through-focus call: the mask
/// spectrum is computed once, the planes are imaged in parallel, and each
/// focus gets its own (cached) focus-exact kernel set — identical to
/// calling `engine.compute(mask, z)` per plane.
fn aerial_through_focus(
    engine: &AerialImageEngine,
    mask: &Mask,
    defocus_nm: &[f64],
) -> Vec<Array2<f64>> {
    engine
        .compute_through_focus(mask, defocus_nm)
        .into_iter()
        .map(|image| image.data)
        .collect()
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
    let nz = config.nz;
    let dz = resist_thickness / nz as f64;
    let z_centers: Vec<f64> = (0..nz).map(|k| (k as f64 + 0.5) * dz).collect();

    let n_planes = config.n_defocus_planes.min(nz).max(1);
    // One plane per slice: image every slice at its own centre depth.
    let per_slice = n_planes == nz;
    let plane_z: Vec<f64> = if per_slice {
        z_centers.clone()
    } else if n_planes == 1 {
        vec![resist_thickness / 2.0]
    } else {
        (0..n_planes)
            .map(|p| p as f64 * resist_thickness / (n_planes - 1) as f64)
            .collect()
    };
    let plane_defocus: Vec<f64> = plane_z
        .iter()
        .map(|&z| config.base_defocus_nm + z / n_resist)
        .collect();
    let plane_images = aerial_through_focus(engine, mask, &plane_defocus);

    // Per z-slice: bracketing plane indices and interpolation weight.
    let slice_interp: Vec<(usize, usize, f64)> = z_centers
        .iter()
        .enumerate()
        .map(|(k, &z)| {
            if per_slice {
                return (k, k, 0.0);
            }
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
        // |E(z)|² (unit incident field) → exposing intensity relative to the
        // incident intensity: the absorbed power density is α·Re(n_r)·|E|²
        // per unit incident Re(n_0)·|E_0|², so S(z) = (n_r / n_0)·|E(z)|².
        let intensity_scale = n_resist / film_stack.superstrate.re;
        let s_profile: Vec<f64> = stack
            .intensity_profile(wavelength_nm, 0.0, Polarization::Unpolarized, &absolute_z)
            .into_iter()
            .map(|e2| intensity_scale * e2)
            .collect();

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

// ============================================================================
// Post-exposure bake
// ============================================================================

/// Post-exposure-bake model applied between exposure and development
/// ([`apply_peb`]).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PebModel {
    /// No bake: the latent image goes straight to development.
    #[default]
    None,
    /// Fickian diffusion of the latent image with exact Gaussian kernels:
    /// separate lateral and vertical diffusion lengths and an optional
    /// depth-dependent vertical diffusivity
    /// ([`resist::peb_diffuse_anisotropic`]).
    Gaussian(PebDiffusion),
    /// Chemically amplified resist acid/quencher reaction–diffusion bake
    /// ([`resist::car_peb`]). The latent PAC is read as the remaining
    /// photo-acid-generator fraction (acid `h0 = 1 − m`) and is replaced by
    /// the protected-site fraction, which the development rate consumes.
    ChemicallyAmplified(CarParams),
}

/// `[dz, dy, dx]` sample spacing of a volume, from its extents.
fn grid_spacing(grid: &Grid3D<f64>) -> [f64; 3] {
    [
        grid.pixel_size_z(),
        grid.pixel_size_y(),
        grid.pixel_size_x(),
    ]
}

/// Apply a post-exposure bake to a volumetric latent image, in place.
///
/// Sample spacings come from the latent grid's extents. For
/// [`PebModel::ChemicallyAmplified`] the full CAR state after the bake
/// (acid, quencher, neutralized total) is returned as well; the latent
/// `pac` then holds the protected-site fraction.
pub fn apply_peb(
    latent: &mut VolumetricLatentImage,
    model: &PebModel,
) -> Result<Option<CarPebResult>> {
    let spacing = grid_spacing(&latent.pac);
    match model {
        PebModel::None => Ok(None),
        PebModel::Gaussian(peb) => {
            resist::peb_diffuse_anisotropic(&mut latent.pac.data, spacing, peb)?;
            Ok(None)
        }
        PebModel::ChemicallyAmplified(car) => {
            let acid0 = latent.pac.data.mapv(|m| (1.0 - m).max(0.0));
            let result = resist::car_peb(&acid0, spacing, car)?;
            latent.pac.data.assign(&result.protected);
            Ok(Some(result))
        }
    }
}

/// Anisotropic 3D post-exposure bake with exact Gaussian kernels:
/// separate lateral and vertical diffusion lengths, an optional
/// depth-dependent vertical diffusivity, zero-flux top and bottom, and a
/// selectable lateral boundary ([`resist::peb_diffuse_anisotropic`];
/// spacings from the latent grid's extents).
pub fn peb_diffuse_3d_anisotropic(
    latent: &mut VolumetricLatentImage,
    peb: &PebDiffusion,
) -> Result<()> {
    let spacing = grid_spacing(&latent.pac);
    resist::peb_diffuse_anisotropic(&mut latent.pac.data, spacing, peb)
}

/// Isotropic 3D post-exposure bake (back-compatible signature).
///
/// Equivalent to [`peb_diffuse_3d_anisotropic`] with
/// `lateral_nm = vertical_nm = diffusion_nm`, reflecting (zero-flux)
/// lateral edges, and the explicit pixel sizes given here. The kernels are
/// exact Gaussians in each axis (the transfer function
/// `exp(−2π²σ²f²)`), replacing the earlier sampled kernel truncated at 3σ
/// with clamped edges. `diffusion_nm ≤ 0` or non-positive pixel sizes
/// leave the volume unchanged.
pub fn peb_diffuse_3d(
    latent: &mut VolumetricLatentImage,
    diffusion_nm: f64,
    pixel_xy_nm: f64,
    pixel_z_nm: f64,
) {
    if !(diffusion_nm > 0.0 && pixel_xy_nm > 0.0 && pixel_z_nm > 0.0) {
        return;
    }
    let peb = PebDiffusion {
        lateral_nm: diffusion_nm,
        vertical_nm: diffusion_nm,
        vertical_diffusivity_scale: None,
        lateral_boundary: DiffusionBoundary::Reflecting,
    };
    // Inputs are validated above, so this cannot fail.
    let _ = resist::peb_diffuse_anisotropic(
        &mut latent.pac.data,
        [pixel_z_nm, pixel_xy_nm, pixel_xy_nm],
        &peb,
    );
}

// ============================================================================
// Development: shared options and rate fields
// ============================================================================

/// Tier-1 development: per-column threshold depth map.
///
/// Scans each (x, y) column from the resist top and returns the depth
/// (nm) of contiguous development — the z where the first "blocking"
/// voxel (PAC ≥ threshold, i.e. insufficiently exposed) is met.
/// Adequate for LIGA and grayscale; no lateral etching or undercut
/// (use [`develop_fast_marching`] or [`develop_level_set`] for those).
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

/// Surface inhibition of the dissolution rate near the ORIGINAL top
/// surface of the resist:
/// `f_inh(d) = 1 − (1 − r_s) · exp(−d / δ_inh)`, where `d` is the depth
/// below the original top surface, `r_s = f_inh(0)` the relative surface
/// rate and `δ_inh` the inhibition depth.
///
/// `r_s < 1` models the slow-dissolving skin of DNQ/novolac and chemically
/// amplified resists that produces T-topped / rounded profiles; `r_s = 1`
/// or `δ_inh = 0` disables it. The factor depends only on the position
/// (depth below the original surface), so it is static and both
/// development solvers accept it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SurfaceInhibition {
    /// Relative dissolution rate at the top surface, `r_s = f_inh(0)`
    /// (finite, > 0; < 1 inhibits, 1 = no effect).
    pub surface_rate_ratio: f64,
    /// Inhibition depth `δ_inh` in nm (e-folding depth; 0 disables).
    pub depth_nm: f64,
}

impl SurfaceInhibition {
    /// Validated constructor.
    pub fn new(surface_rate_ratio: f64, depth_nm: f64) -> Result<Self> {
        let s = Self {
            surface_rate_ratio,
            depth_nm,
        };
        s.validate()?;
        Ok(s)
    }

    /// Rate factor at `depth_below_top_nm` below the original top surface.
    pub fn factor(&self, depth_below_top_nm: f64) -> f64 {
        if self.depth_nm <= 0.0 {
            return 1.0;
        }
        1.0 - (1.0 - self.surface_rate_ratio) * (-depth_below_top_nm.max(0.0) / self.depth_nm).exp()
    }

    fn validate(&self) -> Result<()> {
        if !(self.surface_rate_ratio.is_finite() && self.surface_rate_ratio > 0.0) {
            return Err(LithographyError::InvalidParameter {
                name: "surface_rate_ratio",
                value: self.surface_rate_ratio,
                reason: "relative surface rate must be finite and > 0",
            });
        }
        if !(self.depth_nm.is_finite() && self.depth_nm >= 0.0) {
            return Err(LithographyError::InvalidParameter {
                name: "inhibition_depth_nm",
                value: self.depth_nm,
                reason: "inhibition depth must be finite and >= 0",
            });
        }
        Ok(())
    }
}

/// Static (time-independent) development options shared by the
/// fast-marching and level-set solvers.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct DevelopmentOptions {
    /// Optional surface-inhibition factor on the bulk rate.
    #[serde(default)]
    pub surface_inhibition: Option<SurfaceInhibition>,
    /// Lateral (x, y) boundary: `Periodic` matches the periodic field of
    /// the FFT aerial image (default); `Reflecting` treats the field edges
    /// as mirror planes (what [`develop_fast_marching`] always did).
    /// z is never periodic: developer enters through the top surface and
    /// the substrate is impermeable.
    #[serde(default)]
    pub lateral_boundary: DiffusionBoundary,
}

/// Static dissolution-rate array `R_bulk(m) · f_inh(depth)` (nm/s), with
/// the voxel-centre depth `(k + ½)·dz`.
fn rate_array(
    latent: &VolumetricLatentImage,
    params: &ResistParams,
    inhibition: Option<&SurfaceInhibition>,
    dz: f64,
) -> Result<Array3<f64>> {
    let mut rate = latent.pac.data.mapv(|m| params.development.rate(m));
    if let Some(inh) = inhibition {
        inh.validate()?;
        for (k, mut slice) in rate.outer_iter_mut().enumerate() {
            let f = inh.factor((k as f64 + 0.5) * dz);
            slice.mapv_inplace(|r| r * f);
        }
    }
    Ok(rate)
}

/// Static dissolution-rate volume `R_bulk(m(x)) · f_inh(depth)` in nm/s,
/// evaluated on the latent image with the resist's development model
/// (Mack or threshold) and optional surface inhibition. The depth of slice
/// `k` below the original top surface is `(k + ½)·dz` from the grid.
pub fn development_rate_volume(
    latent: &VolumetricLatentImage,
    params: &ResistParams,
    inhibition: Option<&SurfaceInhibition>,
) -> Result<Grid3D<f64>> {
    let data = rate_array(latent, params, inhibition, latent.pac.pixel_size_z())?;
    Ok(like_grid(&latent.pac, data))
}

/// Indicator volume of the developed region after `dev_time_s`: 1 where
/// the arrival time is `≤ dev_time_s`, 0 elsewhere. Feed it to
/// [`crate::metrics::cd_at_z`] (threshold 0.5) for per-slice developed CD.
pub fn developed_indicator(times: &Grid3D<f64>, dev_time_s: f64) -> Grid3D<f64> {
    let data = times.data.mapv(|t| if t <= dev_time_s { 1.0 } else { 0.0 });
    like_grid(times, data)
}

/// Clone the coordinate extents of `src` around a new data array.
fn like_grid(src: &Grid3D<f64>, data: Array3<f64>) -> Grid3D<f64> {
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

// ============================================================================
// Grid topology shared by the fast-marching and level-set solvers
// ============================================================================

/// Dimensions, spacings `[dz, dy, dx]`, and the lateral boundary rule of a
/// `(z, y, x)` volume in flat row-major order. z is never periodic.
#[derive(Debug, Clone, Copy)]
struct Lattice {
    nz: usize,
    ny: usize,
    nx: usize,
    h: [f64; 3],
    periodic: bool,
}

impl Lattice {
    fn new(dims: (usize, usize, usize), h: [f64; 3], boundary: DiffusionBoundary) -> Self {
        Self {
            nz: dims.0,
            ny: dims.1,
            nx: dims.2,
            h,
            periodic: boundary == DiffusionBoundary::Periodic,
        }
    }

    fn len(&self) -> usize {
        self.nz * self.ny * self.nx
    }

    #[inline]
    fn unflat(&self, p: usize) -> (usize, usize, usize) {
        let j = p % self.nx;
        let r = p / self.nx;
        (r / self.ny, r % self.ny, j)
    }

    /// Flat index of the neighbour of `(k, i, j)` along `axis`, backward
    /// (`forward = false`) or forward; `None` past a wall.
    #[inline]
    fn neighbor(&self, k: usize, i: usize, j: usize, axis: usize, forward: bool) -> Option<usize> {
        let (k2, i2, j2) = match axis {
            0 => {
                let k2 = if forward {
                    (k + 1 < self.nz).then_some(k + 1)
                } else {
                    k.checked_sub(1)
                }?;
                (k2, i, j)
            }
            1 => (k, step_lateral(i, self.ny, forward, self.periodic)?, j),
            _ => (k, i, step_lateral(j, self.nx, forward, self.periodic)?),
        };
        Some((k2 * self.ny + i2) * self.nx + j2)
    }
}

/// Lateral neighbour index with an optional periodic wrap.
#[inline]
fn step_lateral(x: usize, n: usize, forward: bool, periodic: bool) -> Option<usize> {
    if forward {
        if x + 1 < n {
            Some(x + 1)
        } else if periodic && n > 1 {
            Some(0)
        } else {
            None
        }
    } else if x > 0 {
        Some(x - 1)
    } else if periodic && n > 1 {
        Some(n - 1)
    } else {
        None
    }
}

// ============================================================================
// Development tier 2: fast marching (static rate field)
// ============================================================================

/// Heap entry for the fast-marching method (min-heap via reversed Ord).
struct FmmNode {
    time: f64,
    idx: usize,
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

/// First-order Godunov upwind update: solve
/// `sum_axes max(0, (T - T_axis_min) / h_axis)^2 = 1 / R^2`
/// using the standard sorted-quadratic construction (neighbours past a
/// wall count as infinitely far).
fn eikonal_update(
    times: &[f64],
    lat: &Lattice,
    (k, i, j): (usize, usize, usize),
    inv_rate: f64,
) -> f64 {
    // Candidate (value, spacing) pairs, stably sorted by value.
    let mut cands = [(0.0_f64, 0.0_f64); 3];
    let mut len = 0;
    for axis in 0..3 {
        let lo = lat
            .neighbor(k, i, j, axis, false)
            .map_or(f64::INFINITY, |q| times[q]);
        let hi = lat
            .neighbor(k, i, j, axis, true)
            .map_or(f64::INFINITY, |q| times[q]);
        let v = lo.min(hi);
        if v.is_finite() {
            // Stable insertion keeps equal values in axis order.
            let mut pos = len;
            while pos > 0 && cands[pos - 1].0.total_cmp(&v) == Ordering::Greater {
                cands[pos] = cands[pos - 1];
                pos -= 1;
            }
            cands[pos] = (v, lat.h[axis]);
            len += 1;
        }
    }
    if len == 0 {
        return f64::INFINITY;
    }
    let cands = &cands[..len];

    let mut best = f64::INFINITY;
    // Try solutions using the m smallest candidates, m = 1..=len.
    for m in 1..=len {
        // Solve sum_{i<m} ((T - v_i)/h_i)^2 = inv_rate^2 for T.
        let mut a = 0.0;
        let mut b = 0.0;
        let mut c = -inv_rate * inv_rate;
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
        if t >= cands[m - 1].0 && (m == len || t <= cands[m].0) {
            best = t;
            break;
        }
    }
    if best.is_infinite() {
        // Fallback: one-sided update from the smallest neighbor.
        best = cands[0].0 + cands[0].1 * inv_rate;
    }
    best
}

/// Dijkstra-like fast-marching sweep. `times` holds the trial values of
/// the voxels listed in `seeds` (pushed in order) and INFINITY elsewhere;
/// `fixed` voxels are never updated. Marching stops once the smallest
/// trial value exceeds `stop_above`. `on_accept(p, times)` runs when voxel
/// `p` is accepted.
fn fast_march(
    lat: &Lattice,
    times: &mut [f64],
    seeds: &[usize],
    fixed: Option<&[bool]>,
    inv_rate: impl Fn(usize) -> f64,
    stop_above: f64,
    mut on_accept: impl FnMut(usize, &[f64]),
) {
    let mut accepted = vec![false; lat.len()];
    let mut heap: BinaryHeap<FmmNode> = BinaryHeap::with_capacity(seeds.len());
    for &p in seeds {
        heap.push(FmmNode {
            time: times[p],
            idx: p,
        });
    }

    while let Some(FmmNode { time, idx }) = heap.pop() {
        if accepted[idx] {
            continue;
        }
        if time > stop_above {
            break;
        }
        accepted[idx] = true;
        on_accept(idx, times);

        let (k, i, j) = lat.unflat(idx);
        // Relax the six neighbors.
        for axis in 0..3 {
            for forward in [false, true] {
                let Some(q) = lat.neighbor(k, i, j, axis, forward) else {
                    continue;
                };
                if accepted[q] || fixed.is_some_and(|f| f[q]) {
                    continue;
                }
                let new_time = eikonal_update(times, lat, lat.unflat(q), inv_rate(q));
                if new_time < times[q] {
                    times[q] = new_time;
                    heap.push(FmmNode {
                        time: new_time,
                        idx: q,
                    });
                }
            }
        }
    }
}

/// Arrival times for developer entering through the top surface: the top
/// voxel centres are seeded at `½·dz / R` (half a cell of etching).
fn fmm_from_top(lat: &Lattice, rate: &[f64]) -> Vec<f64> {
    let mut times = vec![f64::INFINITY; lat.len()];
    let top = lat.ny * lat.nx;
    let seeds: Vec<usize> = (0..top).collect();
    for &p in &seeds {
        times[p] = 0.5 * lat.h[0] / rate[p];
    }
    fast_march(
        lat,
        &mut times,
        &seeds,
        None,
        |q| 1.0 / rate[q],
        f64::INFINITY,
        |_, _| {},
    );
    times
}

/// Tier-2 development: fast-marching solution of the Eikonal equation
/// `|∇T(x,y,z)| = 1 / R(x,y,z)` with the developer arriving at the
/// resist top surface at T = 0. `R` is the Mack/threshold development
/// rate (nm/s) evaluated on the latent image. Returns per-voxel arrival
/// times in seconds; the developed region after `t` seconds is
/// `{ T <= t }`, including lateral etching and undercut.
///
/// First-order Godunov upwind updates on the anisotropic grid
/// (`pixel_xy_nm` laterally, `pixel_z_nm` in depth) with mirror
/// (reflecting) lateral edges; the rate field is frozen during
/// development (isotropic wet-etch assumption). For surface inhibition or
/// periodic edges use [`develop_fast_marching_with`]; for time-dependent
/// rates use [`develop_level_set`].
pub fn develop_fast_marching(
    latent: &VolumetricLatentImage,
    params: &ResistParams,
    pixel_xy_nm: f64,
    pixel_z_nm: f64,
) -> Grid3D<f64> {
    let rate: Vec<f64> = latent
        .pac
        .data
        .iter()
        .map(|&m| params.development.rate(m))
        .collect();
    let lat = Lattice::new(
        latent.pac.data.dim(),
        [pixel_z_nm, pixel_xy_nm, pixel_xy_nm],
        DiffusionBoundary::Reflecting,
    );
    let times = fmm_from_top(&lat, &rate);
    like_grid(
        &latent.pac,
        Array3::from_shape_vec(latent.pac.data.dim(), times).expect("shape matches"),
    )
}

/// Fast-marching development with static options: the rate is
/// `R_bulk(m) · f_inh(depth)` (depth of slice `k` = `(k + ½)·pixel_z_nm`)
/// and the lateral boundary is selectable. With default options except
/// `lateral_boundary = Reflecting` this reproduces
/// [`develop_fast_marching`] exactly.
pub fn develop_fast_marching_with(
    latent: &VolumetricLatentImage,
    params: &ResistParams,
    pixel_xy_nm: f64,
    pixel_z_nm: f64,
    options: &DevelopmentOptions,
) -> Result<Grid3D<f64>> {
    for (name, value) in [("pixel_xy_nm", pixel_xy_nm), ("pixel_z_nm", pixel_z_nm)] {
        if !(value.is_finite() && value > 0.0) {
            return Err(LithographyError::InvalidParameter {
                name,
                value,
                reason: "pixel size must be finite and positive",
            });
        }
    }
    let rate = rate_array(
        latent,
        params,
        options.surface_inhibition.as_ref(),
        pixel_z_nm,
    )?;
    let lat = Lattice::new(
        rate.dim(),
        [pixel_z_nm, pixel_xy_nm, pixel_xy_nm],
        options.lateral_boundary,
    );
    let flat: Vec<f64> = rate.iter().copied().collect();
    let times = fmm_from_top(&lat, &flat);
    Ok(like_grid(
        &latent.pac,
        Array3::from_shape_vec(rate.dim(), times).expect("shape matches"),
    ))
}

/// Fast-marching arrival times for an arbitrary static rate volume (nm/s)
/// with developer entering through the top surface; spacings come from the
/// grid's extents.
pub fn fast_marching_times(
    rate: &Grid3D<f64>,
    lateral_boundary: DiffusionBoundary,
) -> Result<Grid3D<f64>> {
    check_rates(&rate.data)?;
    let lat = Lattice::new(rate.data.dim(), grid_spacing(rate), lateral_boundary);
    let flat: Vec<f64> = rate.data.iter().copied().collect();
    let times = fmm_from_top(&lat, &flat);
    Ok(like_grid(
        rate,
        Array3::from_shape_vec(rate.data.dim(), times).expect("shape matches"),
    ))
}

fn check_rates(rate: &Array3<f64>) -> Result<()> {
    if let Some(&bad) = rate.iter().find(|r| !(r.is_finite() && **r >= 0.0)) {
        return Err(LithographyError::InvalidParameter {
            name: "rate",
            value: bad,
            reason: "dissolution rates must be finite and non-negative",
        });
    }
    Ok(())
}

/// Remaining-height map after `dev_time_s` seconds of development, from
/// fast-marching (or level-set) arrival times: per column, the deepest
/// voxel reached within the time defines the removed depth. (Undercut
/// information is in the full 3D `times` volume; a height map cannot
/// represent it.)
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

// ============================================================================
// Development tier 3: level-set moving boundary
// ============================================================================

/// Developer depletion / ageing factor `g` multiplying the dissolution
/// rate during a level-set development.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeveloperDepletion {
    /// `g = 1`: fresh developer throughout.
    #[default]
    None,
    /// Developer ageing, `g(t) = exp(−t / τ)`: the dissolution rate decays
    /// with a time constant τ (e.g. puddle exhaustion or temperature
    /// drift). Integrated exactly in pseudo-time.
    Exponential {
        /// Time constant τ in seconds (> 0).
        time_constant_s: f64,
    },
    /// Global loading, `g = max(0, 1 − h̄ / h_cap)`: `h̄` is the dissolved
    /// resist volume per unit area (nm) over the whole field and `h_cap`
    /// the developer capacity per unit area — developer consumed by what
    /// has already dissolved.
    Loading {
        /// Developer capacity per unit area `h_cap` in nm of resist (> 0).
        capacity_nm: f64,
    },
    /// Local loading: as [`DeveloperDepletion::Loading`] but with `h̄`
    /// replaced by the Gaussian-smoothed (σ = `length_nm`) per-column
    /// dissolved thickness, so densely opened areas deplete their
    /// developer faster than isolated openings (a micro-loading effect).
    LocalLoading {
        /// Developer capacity per unit area `h_cap` in nm of resist (> 0).
        capacity_nm: f64,
        /// Lateral depletion length σ in nm (> 0).
        length_nm: f64,
    },
}

impl DeveloperDepletion {
    fn validate(&self) -> Result<()> {
        let check = |name: &'static str, value: f64| {
            if value.is_finite() && value > 0.0 {
                Ok(())
            } else {
                Err(LithographyError::InvalidParameter {
                    name,
                    value,
                    reason: "must be finite and > 0",
                })
            }
        };
        match *self {
            DeveloperDepletion::None => Ok(()),
            DeveloperDepletion::Exponential { time_constant_s } => {
                check("depletion_time_constant_s", time_constant_s)
            }
            DeveloperDepletion::Loading { capacity_nm } => {
                check("loading_capacity_nm", capacity_nm)
            }
            DeveloperDepletion::LocalLoading {
                capacity_nm,
                length_nm,
            } => {
                check("loading_capacity_nm", capacity_nm)?;
                check("loading_length_nm", length_nm)
            }
        }
    }
}

/// Time-integration settings of [`develop_level_set`] /
/// [`evolve_level_set`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LevelSetConfig {
    /// Development time in seconds.
    pub dev_time_s: f64,
    /// Developer depletion / ageing factor.
    #[serde(default)]
    pub depletion: DeveloperDepletion,
    /// CFL number in (0, 1]: `Δt = cfl / (max R · sqrt(Σ 1/h²))`, which
    /// keeps the monotone upwind scheme stable; 0.5 is conservative.
    pub cfl: f64,
    /// Reinitialize φ to a signed distance every this many steps (≥ 1).
    pub reinit_interval: usize,
    /// Half-width of the narrow band of updated voxels, in units of the
    /// largest grid spacing. Must be at least `reinit_interval · cfl + 2`
    /// so the front cannot leave the band between reinitializations.
    pub band_cells: f64,
    /// Safety cap on the number of time steps (an error is returned if
    /// the development needs more).
    pub max_steps: usize,
}

impl Default for LevelSetConfig {
    fn default() -> Self {
        Self {
            dev_time_s: 60.0,
            depletion: DeveloperDepletion::None,
            cfl: 0.5,
            reinit_interval: 4,
            band_cells: 6.0,
            max_steps: 2_000_000,
        }
    }
}

impl LevelSetConfig {
    /// Default settings for a development time of `dev_time_s` seconds.
    pub fn new(dev_time_s: f64) -> Self {
        Self {
            dev_time_s,
            ..Self::default()
        }
    }

    fn validate(&self) -> Result<()> {
        if !(self.dev_time_s.is_finite() && self.dev_time_s >= 0.0) {
            return Err(LithographyError::InvalidParameter {
                name: "dev_time_s",
                value: self.dev_time_s,
                reason: "development time must be finite and >= 0",
            });
        }
        if !(self.cfl > 0.0 && self.cfl <= 1.0) {
            return Err(LithographyError::InvalidParameter {
                name: "cfl",
                value: self.cfl,
                reason: "CFL number must be in (0, 1]",
            });
        }
        if self.reinit_interval == 0 {
            return Err(LithographyError::InvalidParameter {
                name: "reinit_interval",
                value: 0.0,
                reason: "must be at least 1 (the narrow band needs reinitialization)",
            });
        }
        let min_band = self.reinit_interval as f64 * self.cfl + 2.0;
        if !(self.band_cells.is_finite() && self.band_cells >= min_band) {
            return Err(LithographyError::InvalidParameter {
                name: "band_cells",
                value: self.band_cells,
                reason: "narrow band must be at least reinit_interval * cfl + 2 cells wide",
            });
        }
        if self.max_steps == 0 {
            return Err(LithographyError::InvalidParameter {
                name: "max_steps",
                value: 0.0,
                reason: "must be positive",
            });
        }
        self.depletion.validate()
    }
}

/// Output of the level-set development.
#[derive(Debug, Clone)]
pub struct LevelSetResult {
    /// Level-set function φ (nm) at the end of development: approximately
    /// the signed distance to the resist/developer boundary, negative in
    /// the developed region, clipped to a few cells beyond the narrow
    /// band.
    pub phi: Grid3D<f64>,
    /// Time (s) at which the front crossed each voxel centre; INFINITY
    /// where it did not within `dev_time_s`. Directly comparable with the
    /// fast-marching arrival times (and accepted by
    /// [`height_map_from_times`] / [`developed_indicator`] for any
    /// `t ≤ dev_time_s`).
    pub arrival_times: Grid3D<f64>,
    /// Development time simulated (s).
    pub dev_time_s: f64,
    /// Number of time steps taken.
    pub steps: usize,
    /// Number of signed-distance reinitializations (including the initial
    /// one).
    pub reinitializations: usize,
    /// Mean dissolved resist thickness at the end (dissolved volume per
    /// unit area, nm).
    pub dissolved_thickness_nm: f64,
    /// Depletion factor `g` at the end (lateral mean for local loading;
    /// 1 without depletion).
    pub final_rate_factor: f64,
}

impl LevelSetResult {
    /// Remaining-height map at the end of development.
    pub fn height_map(&self) -> Grid2D<f64> {
        height_map_from_times(&self.arrival_times, self.dev_time_s)
    }
}

/// Tier-3 development: level-set moving boundary with the dissolution rate
/// re-evaluated at the front every step,
/// `R(x, t) = R_bulk(m(x)) · f_inh(depth) · g(t)` (see the module docs).
///
/// Developer enters through the top surface at `t = 0`; the substrate is
/// impermeable; the lateral boundary comes from `options`. Spacings come
/// from the latent grid's extents. For a static rate (no depletion) the
/// arrival times agree with [`develop_fast_marching_with`] within grid
/// error; FMM is the faster choice there.
pub fn develop_level_set(
    latent: &VolumetricLatentImage,
    params: &ResistParams,
    options: &DevelopmentOptions,
    config: &LevelSetConfig,
) -> Result<LevelSetResult> {
    let rate = development_rate_volume(latent, params, options.surface_inhibition.as_ref())?;
    let dz = latent.pac.pixel_size_z();
    let mut phi0 = rate.clone();
    for (k, mut slice) in phi0.data.outer_iter_mut().enumerate() {
        slice.fill((k as f64 + 0.5) * dz);
    }
    evolve_level_set(&phi0, &rate, true, options.lateral_boundary, config)
}

/// Evolve a level-set front `φ_t + g·R|∇φ| = 0` from an arbitrary initial
/// level-set function `phi0` (nm; `φ ≤ 0` = developed) through a static
/// rate volume `rate` (nm/s), with the depletion/ageing factor `g` from
/// `config`.
///
/// `top_developer = true` keeps developer above the top surface (the
/// top boundary is a developer reservoir, as in [`develop_level_set`]);
/// `false` makes the top boundary a mirror, e.g. for fronts growing from
/// an interior seed. The bottom is always a mirror (impermeable
/// substrate). `phi0` need not be a signed distance: it is reinitialized
/// before the first step.
pub fn evolve_level_set(
    phi0: &Grid3D<f64>,
    rate: &Grid3D<f64>,
    top_developer: bool,
    lateral_boundary: DiffusionBoundary,
    config: &LevelSetConfig,
) -> Result<LevelSetResult> {
    config.validate()?;
    if phi0.data.dim() != rate.data.dim() {
        return Err(LithographyError::InvalidParameter {
            name: "rate",
            value: rate.data.len() as f64,
            reason: "rate volume must have the same shape as the level-set grid",
        });
    }
    check_rates(&rate.data)?;
    if let Some(&bad) = phi0.data.iter().find(|v| v.is_nan()) {
        return Err(LithographyError::InvalidParameter {
            name: "phi0",
            value: bad,
            reason: "initial level-set function must not contain NaN",
        });
    }

    let dims = phi0.data.dim();
    let h = grid_spacing(phi0);
    let lat = Lattice::new(dims, h, lateral_boundary);
    let n = lat.len();
    let ncol = lat.ny * lat.nx;
    let h_max = h[0].max(h[1]).max(h[2]);
    let band = config.band_cells * h_max;
    let cap = band + 2.0 * h_max;
    let inv_h_norm = h.iter().map(|x| 1.0 / (x * x)).sum::<f64>().sqrt();
    let t_end = config.dev_time_s;

    let rate_v: Vec<f64> = rate.data.iter().copied().collect();
    let mut phi: Vec<f64> = phi0.data.iter().copied().collect();
    let mut arrival: Vec<f64> = phi
        .iter()
        .map(|&v| if v <= 0.0 { 0.0 } else { f64::INFINITY })
        .collect();
    let mut ext = vec![f64::NAN; n];
    // Loading factor per column: at the current state, at the previous
    // state, and the second-order (Adams–Bashforth) estimate over a step.
    let mut gcol = vec![1.0; ncol];
    let mut gprev = vec![1.0; ncol];
    let mut gstep = vec![1.0; ncol];
    let mut dt_prev = 0.0;

    reinitialize(&mut phi, &lat, top_developer, cap, &rate_v, &mut ext, true);
    let mut reinits = 1;
    // Narrow band: the voxels within `band` of the front, rebuilt at every
    // reinitialization (the front cannot leave it in between).
    let mut active: Vec<usize> = (0..n).filter(|&p| phi[p].abs() <= band).collect();
    let mut updated: Vec<f64> = Vec::with_capacity(active.len());
    let mut crossings: Vec<(usize, f64)> = Vec::new();
    let mut resist_left = phi.iter().filter(|&&v| v > 0.0).count();
    let loading = matches!(
        config.depletion,
        DeveloperDepletion::Loading { .. } | DeveloperDepletion::LocalLoading { .. }
    );
    if loading {
        let dissolved = column_dissolved(&phi, &lat, top_developer);
        update_loading_factor(
            &config.depletion,
            &dissolved,
            &lat,
            lateral_boundary,
            &mut gcol,
        );
        gstep.copy_from_slice(&gcol);
    }

    let mut t = 0.0;
    let mut tau_elapsed = 0.0; // pseudo-time ∫g dt (exponential ageing only)
    let mut steps = 0;
    let tol = 1e-12 * t_end.max(1.0);
    while t < t_end - tol && resist_left > 0 {
        if steps >= config.max_steps {
            return Err(LithographyError::NumericalError(format!(
                "level-set development needs more than {} steps (t = {t:.4} of {t_end} s); \
                 coarsen the grid, shorten the development, or raise max_steps",
                config.max_steps
            )));
        }

        // The fastest speed at the front — developed band voxels and resist
        // within one cell of the front — sets the time step. Resist further
        // ahead moves at most this fast (it cannot overtake its upwind
        // neighbour anyway), so fast resist the front has not reached yet
        // does not shrink the step.
        let mut vmax = 0.0_f64;
        for &p in &active {
            let v = phi[p];
            if v <= h_max {
                vmax = vmax.max(voxel_speed(v, ext[p], rate_v[p], gcol[p % ncol]));
            }
        }
        if vmax <= 0.0 {
            break; // the front has stalled
        }
        let w_cfl = config.cfl / (vmax * inv_h_norm);
        // (real time step, pseudo-time weight applied to the speeds)
        let (dt, weight) = match config.depletion {
            DeveloperDepletion::Exponential {
                time_constant_s: tau,
            } => {
                let e_now = (-t / tau).exp();
                let remaining = tau * (e_now - (-t_end / tau).exp());
                if w_cfl >= remaining {
                    (t_end - t, remaining)
                } else {
                    (-tau * (1.0 - w_cfl / (tau * e_now)).ln(), w_cfl)
                }
            }
            _ => {
                let dt = w_cfl.min(t_end - t);
                (dt, dt)
            }
        };
        if loading {
            // g(t) only decreases; extrapolate it to the step midpoint
            // (second order in time), never above its current value. The
            // CFL step above used the larger current value, so it stays
            // stable.
            for ((gs, &gc), &gp) in gstep.iter_mut().zip(&gcol).zip(&gprev) {
                *gs = if dt_prev > 0.0 {
                    (gc + 0.5 * (gc - gp) * dt / dt_prev).clamp(0.0, gc)
                } else {
                    gc
                };
            }
        }

        // Jacobi update of the band from the old φ.
        updated.clear();
        crossings.clear();
        for &p in &active {
            let v = phi[p];
            let kij = lat.unflat(p);
            let grad = upwind_gradient(&phi, &lat, kij, p, top_developer);
            let mut speed = voxel_speed(v, ext[p], rate_v[p], gstep[p % ncol]);
            if v > h_max {
                speed = speed.min(vmax);
            }
            let nv = v - weight * speed * grad;
            if v > 0.0 && nv <= 0.0 {
                // Newly developed: from now on it moves with the resist it
                // borders (no plunge at its own, possibly much faster, rate).
                crossings.push((p, nearest_resist_rate(&phi, &lat, kij, p, &rate_v)));
                if arrival[p].is_infinite() {
                    let frac = v / (v - nv);
                    arrival[p] = match config.depletion {
                        DeveloperDepletion::Exponential {
                            time_constant_s: tau,
                        } => {
                            let tau_cross = tau_elapsed + frac * weight;
                            (-tau * (1.0 - tau_cross / tau).ln()).min(t_end)
                        }
                        _ => t + frac * dt,
                    };
                }
            }
            updated.push(nv);
        }
        for (&p, &nv) in active.iter().zip(&updated) {
            phi[p] = nv;
        }
        resist_left -= crossings.len();
        for &(p, rate_across) in &crossings {
            ext[p] = rate_across;
        }
        t = if dt >= t_end - t { t_end } else { t + dt };
        tau_elapsed += weight;
        steps += 1;
        if steps % config.reinit_interval == 0 && resist_left > 0 {
            reinitialize(&mut phi, &lat, top_developer, cap, &rate_v, &mut ext, false);
            reinits += 1;
            active.clear();
            active.extend((0..n).filter(|&p| phi[p].abs() <= band));
        }
        if loading {
            gprev.copy_from_slice(&gcol);
            dt_prev = dt;
            let dissolved = column_dissolved(&phi, &lat, top_developer);
            update_loading_factor(
                &config.depletion,
                &dissolved,
                &lat,
                lateral_boundary,
                &mut gcol,
            );
        }
    }

    let dissolved = column_dissolved(&phi, &lat, top_developer);
    let dissolved_mean = dissolved.iter().sum::<f64>() / ncol as f64;
    let final_rate_factor = match config.depletion {
        DeveloperDepletion::None => 1.0,
        DeveloperDepletion::Exponential { time_constant_s } => (-t_end / time_constant_s).exp(),
        _ => gcol.iter().sum::<f64>() / ncol as f64,
    };
    Ok(LevelSetResult {
        phi: like_grid(
            phi0,
            Array3::from_shape_vec(dims, phi).expect("shape matches"),
        ),
        arrival_times: like_grid(
            phi0,
            Array3::from_shape_vec(dims, arrival).expect("shape matches"),
        ),
        dev_time_s: t_end,
        steps,
        reinitializations: reinits,
        dissolved_thickness_nm: dissolved_mean,
        final_rate_factor,
    })
}

/// Front speed at a voxel: resist voxels (`φ > 0`) move with their own
/// rate; developed voxels near the front carry the extension speed of the
/// resist they border (set at the last reinitialization), so the zero
/// level set moves at the rate of the resist being dissolved.
#[inline]
fn voxel_speed(phi: f64, ext: f64, rate: f64, g: f64) -> f64 {
    let base = if phi > 0.0 || ext.is_nan() { rate } else { ext };
    base * g
}

/// Rate of the resist neighbour (`φ > 0`) across the nearest
/// linear-interpolation crossing of the developed voxel `p` (measured from
/// `p`, i.e. along the steepest ascent of φ) — the extension speed a
/// developed voxel takes (its own rate if it borders no resist).
fn nearest_resist_rate(
    phi: &[f64],
    lat: &Lattice,
    (k, i, j): (usize, usize, usize),
    p: usize,
    rate: &[f64],
) -> f64 {
    let v = phi[p];
    let mut best = (f64::INFINITY, rate[p]);
    for axis in 0..3 {
        for forward in [false, true] {
            if let Some(q) = lat.neighbor(k, i, j, axis, forward) {
                if phi[q] > 0.0 {
                    let d = (-v / (phi[q] - v)) * lat.h[axis];
                    if d < best.0 {
                        best = (d, rate[q]);
                    }
                }
            }
        }
    }
    best.1
}

/// Value of φ in the virtual developer layer above the top surface.
#[inline]
fn top_ghost(phi_top: f64, dz: f64) -> f64 {
    (phi_top - dz).min(-0.5 * dz)
}

/// Neighbour values of voxel `p` along `axis`: (backward, forward), with
/// mirror walls, the periodic lateral wrap, and the top developer ghost.
#[inline]
fn axis_neighbors(
    phi: &[f64],
    lat: &Lattice,
    (k, i, j): (usize, usize, usize),
    p: usize,
    axis: usize,
    top_developer: bool,
) -> (f64, f64) {
    let v = phi[p];
    let back = match lat.neighbor(k, i, j, axis, false) {
        Some(q) => phi[q],
        None if axis == 0 && top_developer => top_ghost(v, lat.h[0]),
        None => v,
    };
    let fwd = lat.neighbor(k, i, j, axis, true).map_or(v, |q| phi[q]);
    (back, fwd)
}

/// Godunov upwind approximation of `|∇φ|` for a front moving with
/// non-negative speed.
///
/// On the top layer with a developer reservoir above, the reservoir's
/// vertical attack is combined with the in-grid Godunov gradient as an
/// independent source (`max`, i.e. the earlier arrival wins) instead of
/// inside the quadratic sum: otherwise a top-edge voxel attacked from above
/// and from the side would see the `√2` kink gradient and dissolve early.
/// This mirrors the fast-marching treatment, where the top voxels are
/// seeded separately from their lateral updates.
#[inline]
fn upwind_gradient(
    phi: &[f64],
    lat: &Lattice,
    kij: (usize, usize, usize),
    p: usize,
    top_developer: bool,
) -> f64 {
    let v = phi[p];
    let reservoir_above = top_developer && kij.0 == 0;
    let mut sum = 0.0;
    for axis in 0..3 {
        // The reservoir ghost is handled separately below.
        let (back, fwd) = axis_neighbors(phi, lat, kij, p, axis, top_developer && !reservoir_above);
        let h = lat.h[axis];
        let a = ((v - back) / h).max((v - fwd) / h).max(0.0);
        sum += a * a;
    }
    let grid = sum.sqrt();
    if reservoir_above {
        let dz = lat.h[0];
        grid.max(((v - top_ghost(v, dz)) / dz).max(0.0))
    } else {
        grid
    }
}

/// Rebuild φ as the signed distance (nm) to its zero level set, capped at
/// `±cap`, and refresh the extension speeds of developed voxels.
///
/// Voxels with a sign change towards a neighbour (the interface band) are
/// frozen and seed a unit-speed fast march that rebuilds the rest of both
/// sides. With `estimate_interface` the seeds are first re-estimated from
/// the planar interface through their linear-interpolation crossings
/// (`1/d² = Σ_a 1/d_a²`) — used once, to turn an arbitrary initial φ into a
/// distance; afterwards the seeds keep their evolved values, because
/// re-estimating them every time shifts the front by O(h²·curvature) per
/// reinitialization (forward at convex resist corners, backward on convex
/// developed regions). A developed seed takes the rate of the resist voxel
/// across its nearest crossing as its extension speed; deeper developed
/// voxels inherit it from their upwind neighbour.
fn reinitialize(
    phi: &mut [f64],
    lat: &Lattice,
    top_developer: bool,
    cap: f64,
    rate: &[f64],
    ext: &mut [f64],
    estimate_interface: bool,
) {
    let n = lat.len();
    let mut dist = vec![f64::INFINITY; n];
    let mut fixed = vec![false; n];
    let mut seeds = Vec::new();
    ext.fill(f64::NAN);

    for p in 0..n {
        let (k, i, j) = lat.unflat(p);
        let v = phi[p];
        let inside = v <= 0.0;
        let mut inv_d2 = 0.0_f64;
        let mut found = false;
        let mut nearest = (f64::INFINITY, f64::NAN); // (distance, resist rate across it)
        for axis in 0..3 {
            // Nearest linear-interpolation crossing along this axis.
            let mut da = f64::INFINITY;
            for forward in [false, true] {
                let (q_val, q_rate) = match lat.neighbor(k, i, j, axis, forward) {
                    Some(q) => (phi[q], rate[q]),
                    None if axis == 0 && !forward && top_developer => {
                        (top_ghost(v, lat.h[0]), f64::NAN)
                    }
                    None => continue,
                };
                if (q_val <= 0.0) == inside {
                    continue;
                }
                let d = v / (v - q_val) * lat.h[axis];
                da = da.min(d);
                if d < nearest.0 {
                    nearest = (d, q_rate);
                }
            }
            if da.is_finite() {
                found = true;
                // Per-axis crossings combined as a local plane.
                inv_d2 += if da > 1e-12 * lat.h[axis] {
                    1.0 / (da * da)
                } else {
                    f64::INFINITY
                };
            }
        }
        if !found {
            continue;
        }
        dist[p] = if !estimate_interface {
            v.abs()
        } else if inv_d2.is_infinite() {
            0.0
        } else {
            1.0 / inv_d2.sqrt()
        };
        fixed[p] = true;
        seeds.push(p);
        if inside {
            ext[p] = nearest.1;
        }
    }

    // Both sides march together: a non-seed voxel only has same-side
    // neighbours, so the two distance fields never mix.
    fast_march(
        lat,
        &mut dist,
        &seeds,
        Some(&fixed),
        |_| 1.0,
        cap,
        |r, d| {
            if fixed[r] || phi[r] > 0.0 {
                return;
            }
            let (k, i, j) = lat.unflat(r);
            let mut parent: Option<usize> = None;
            for axis in 0..3 {
                for forward in [false, true] {
                    if let Some(q) = lat.neighbor(k, i, j, axis, forward) {
                        if d[q] < d[r] && parent.is_none_or(|b| d[q] < d[b]) {
                            parent = Some(q);
                        }
                    }
                }
            }
            if let Some(q) = parent {
                ext[r] = ext[q];
            }
        },
    );

    for p in 0..n {
        let d = dist[p].min(cap);
        phi[p] = if phi[p] <= 0.0 { -d } else { d };
    }
}

/// Developed fraction of a voxel from φ: the voxel's extent along the
/// local normal `w = Σ_a |n_a|·h_a` and `f = clamp(½ − φ/w, 0, 1)` (exact
/// for planar fronts).
fn developed_fraction(
    phi: &[f64],
    lat: &Lattice,
    kij: (usize, usize, usize),
    p: usize,
    top_developer: bool,
    h_max: f64,
) -> f64 {
    let v = phi[p];
    if v >= h_max {
        return 0.0;
    }
    if v <= -h_max {
        return 1.0;
    }
    let mut g = [0.0; 3];
    for (axis, ga) in g.iter_mut().enumerate() {
        let (back, fwd) = axis_neighbors(phi, lat, kij, p, axis, top_developer);
        *ga = (fwd - back) / (2.0 * lat.h[axis]);
    }
    let norm = (g[0] * g[0] + g[1] * g[1] + g[2] * g[2]).sqrt();
    let w = if norm > 1e-12 {
        (g[0].abs() * lat.h[0] + g[1].abs() * lat.h[1] + g[2].abs() * lat.h[2]) / norm
    } else {
        lat.h[0]
    };
    (0.5 - v / w).clamp(0.0, 1.0)
}

/// Dissolved thickness per (y, x) column (nm): `Σ_k dz · f_k`.
fn column_dissolved(phi: &[f64], lat: &Lattice, top_developer: bool) -> Vec<f64> {
    let ncol = lat.ny * lat.nx;
    let h_max = lat.h[0].max(lat.h[1]).max(lat.h[2]);
    let mut out = vec![0.0; ncol];
    for (p, value) in phi.iter().enumerate() {
        let f = if *value >= h_max {
            0.0
        } else if *value <= -h_max {
            1.0
        } else {
            developed_fraction(phi, lat, lat.unflat(p), p, top_developer, h_max)
        };
        out[p % ncol] += f * lat.h[0];
    }
    out
}

/// Refresh the per-column loading factor `g = max(0, 1 − h̄/h_cap)`.
fn update_loading_factor(
    depletion: &DeveloperDepletion,
    dissolved: &[f64],
    lat: &Lattice,
    lateral_boundary: DiffusionBoundary,
    gcol: &mut [f64],
) {
    match *depletion {
        DeveloperDepletion::Loading { capacity_nm } => {
            let mean = dissolved.iter().sum::<f64>() / dissolved.len() as f64;
            gcol.fill((1.0 - mean / capacity_nm).max(0.0));
        }
        DeveloperDepletion::LocalLoading {
            capacity_nm,
            length_nm,
        } => {
            let mut map = Array3::from_shape_vec((1, lat.ny, lat.nx), dissolved.to_vec())
                .expect("shape matches");
            resist::gaussian_diffuse_axis(&mut map, 1, length_nm, lat.h[1], lateral_boundary);
            resist::gaussian_diffuse_axis(&mut map, 2, length_nm, lat.h[2], lateral_boundary);
            for (g, &d) in gcol.iter_mut().zip(map.iter()) {
                *g = (1.0 - d / capacity_nm).clamp(0.0, 1.0);
            }
        }
        _ => {}
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

    /// Exposing intensity inside a resist with n_r ≠ n_0: a vacuum
    /// superstrate over n = 1.65 + iκ resist on an index-matched substrate
    /// (no back reflection) is a pure travelling wave, so the dose-driving
    /// intensity is S(z) = T·e^{−αz} with T = Re(n_r)·|2/(1 + n_r)|²
    /// ≈ 0.9398 — not the bare |E|² = |t|²e^{−αz} (which under-exposes by
    /// 1/1.65). Fixture values computed independently (numpy, closed form):
    /// C·D = 0.6, B = 0.9/µm → κ = 0.0112894, T = 0.939819;
    /// m(z = 4.6875) = 0.570343, m(79.6875) = 0.591635,
    /// m(154.6875) = 0.612255, m(295.3125) = 0.649026
    /// (the pre-2026-10-01 |E|² model gave 0.7115 at the top).
    #[test]
    fn test_exposing_intensity_includes_resist_index() {
        let engine = small_engine();
        let mask = uniform_clear_mask();
        let wavelength = 157.63;
        let resist = ResistParams {
            thickness_nm: 300.0,
            dill_a: 0.0,
            dill_b: 0.9,
            dill_c: 0.02,
            peb_diffusion_nm: 0.0,
            development: DevelopmentModel::default(),
        };
        let kappa = resist.absorption(1.0) * wavelength / (4.0 * std::f64::consts::PI);
        let n_r = Complex64::new(1.65, kappa);
        let matched = |superstrate: Complex64| FilmStack {
            layers: vec![FilmLayer {
                name: "resist".to_string(),
                thickness_nm: 300.0,
                n: n_r,
            }],
            substrate: n_r,
            superstrate,
        };
        let config = VolumetricExposureConfig {
            dose_mj_cm2: 30.0,
            nz: 32,
            n_defocus_planes: 1,
            dose_steps: 1,
            base_defocus_nm: 0.0,
            resist_layer: 0,
        };
        let latent = expose_volumetric(
            &engine,
            &mask,
            &matched(Complex64::new(1.0, 0.0)),
            &resist,
            wavelength,
            &config,
        )
        .unwrap();
        let (_, ny, nx) = latent.pac.data.dim();
        let at = |lat: &VolumetricLatentImage, k: usize| lat.pac.data[[k, ny / 2, nx / 2]];
        for (k, expected) in [(0, 0.570343), (8, 0.591635), (16, 0.612255), (31, 0.649026)] {
            assert_relative_eq!(at(&latent, k), expected, max_relative = 2e-3);
        }

        // Index-matched superstrate (n_0 = n_r): no entrance loss, T = 1, so
        // the top slice sees the full dose · e^{−α z} (scale n_r/n_0 = 1).
        let immersed = expose_volumetric(
            &engine,
            &mask,
            &matched(Complex64::new(1.65, 0.0)),
            &resist,
            wavelength,
            &config,
        )
        .unwrap();
        let z0 = latent.pac.z_at(0);
        let expected = (-0.6 * (-resist.absorption(1.0) * z0).exp()).exp();
        assert_relative_eq!(at(&immersed, 0), expected, max_relative = 2e-3);
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

    /// Depth slices are exposed with the engine's own focus-exact images:
    /// with fewer planes than slices, planes span the resist at
    /// `base + z/n_r` and a slice interpolates between its two bracketing
    /// planes; with one plane per slice, each slice is imaged exactly at its
    /// own centre depth.
    #[test]
    fn test_per_plane_imaging_uses_engine_focus() {
        let engine = small_engine();
        let mask = Mask::line_space(64.0, 128.0).unwrap();
        let resist = ResistParams {
            dill_a: 0.0,
            dill_b: 0.0,
            ..ResistParams::default()
        };
        // Index-matched, non-absorbing stack: S(z) = 1 exactly.
        let stack = FilmStack {
            layers: vec![FilmLayer {
                name: "resist".to_string(),
                thickness_nm: 100.0,
                n: Complex64::new(1.0, 0.0),
            }],
            substrate: Complex64::new(1.0, 0.0),
            superstrate: Complex64::new(1.0, 0.0),
        };
        let dill = |aerial: f64| (-resist.dill_c * 20.0 * aerial).exp();

        // Interpolated: nz = 4, 2 planes at z = 0 (focus 50) and z = 100
        // (focus 150); slice 0 (z = 12.5 nm) sits at t = 0.125.
        let config = VolumetricExposureConfig {
            dose_mj_cm2: 20.0,
            nz: 4,
            n_defocus_planes: 2,
            dose_steps: 1,
            base_defocus_nm: 50.0,
            resist_layer: 0,
        };
        let latent = expose_volumetric(&engine, &mask, &stack, &resist, 157.63, &config).unwrap();
        let top = engine.compute(&mask, 50.0).data;
        let bottom = engine.compute(&mask, 150.0).data;
        for (idx, &m) in latent
            .pac
            .data
            .index_axis(ndarray::Axis(0), 0)
            .indexed_iter()
        {
            let aerial = 0.875 * top[idx] + 0.125 * bottom[idx];
            assert_relative_eq!(m, dill(aerial), epsilon = 1e-12);
        }

        // Per slice: nz = 2 planes = 2 -> slices at z = 25 and 75 nm are
        // imaged exactly at focus 75 and 125 nm.
        let per_slice = VolumetricExposureConfig { nz: 2, ..config };
        let latent =
            expose_volumetric(&engine, &mask, &stack, &resist, 157.63, &per_slice).unwrap();
        for (k, focus) in [(0usize, 75.0), (1, 125.0)] {
            let image = engine.compute(&mask, focus).data;
            for (idx, &m) in latent
                .pac
                .data
                .index_axis(ndarray::Axis(0), k)
                .indexed_iter()
            {
                assert_relative_eq!(m, dill(image[idx]), epsilon = 1e-12);
            }
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
            VolumetricExposureConfig {
                n_defocus_planes: 0,
                ..Default::default()
            },
        ] {
            assert!(
                expose_volumetric(&engine, &mask, &stack, &resist, 157.63, &bad).is_err(),
                "config should be rejected: {bad:?}"
            );
        }
    }

    // ------------------------------------------------------------------
    // Development: fast marching with options, level set, PEB models.
    // ------------------------------------------------------------------

    fn uniform_grid(nx: usize, ny: usize, nz: usize, dx: f64, dz: f64) -> Grid3D<f64> {
        Grid3D::<f64>::new(
            nx,
            ny,
            nz,
            (0.0, nx as f64 * dx),
            (0.0, ny as f64 * dx),
            (0.0, nz as f64 * dz),
        )
        .unwrap()
    }

    fn fill_depth(grid: &mut Grid3D<f64>) {
        let dz = grid.pixel_size_z();
        for (k, mut slice) in grid.data.outer_iter_mut().enumerate() {
            slice.fill((k as f64 + 0.5) * dz);
        }
    }

    /// A Gaussian exposure spot decaying with depth (Mack rates span
    /// ~0.1–100 nm/s across it).
    fn spot_latent(n: usize, nz: usize, dx: f64, dz: f64) -> VolumetricLatentImage {
        let mut pac = uniform_grid(n, n, nz, dx, dz);
        let c = n as f64 * dx / 2.0;
        for ((k, i, j), m) in pac.data.indexed_iter_mut() {
            let x = (j as f64 + 0.5) * dx - c;
            let y = (i as f64 + 0.5) * dx - 0.9 * c;
            let z = (k as f64 + 0.5) * dz;
            let dose = 2.5 * (-(x * x + y * y) / (2.0 * 18.0 * 18.0)).exp() * (-z / 150.0).exp();
            *m = (-dose).exp();
        }
        VolumetricLatentImage { pac }
    }

    /// An exposed space (Gaussian in x, uniform in y and z) centred in the
    /// field.
    fn line_latent(nx: usize, nz: usize, dx: f64, dz: f64, sigma: f64) -> VolumetricLatentImage {
        let mut pac = uniform_grid(nx, 2, nz, dx, dz);
        let c = nx as f64 * dx / 2.0;
        for ((_, _, j), m) in pac.data.indexed_iter_mut() {
            let x = (j as f64 + 0.5) * dx - c;
            *m = 1.0 - 0.9 * (-(x * x) / (2.0 * sigma * sigma)).exp();
        }
        VolumetricLatentImage { pac }
    }

    /// Width of the region `{f ≤ level}` along x in row 0 of slice `k`,
    /// from linearly interpolated crossings (outermost pair).
    fn crossing_width(field: &Grid3D<f64>, k: usize, level: f64) -> f64 {
        let dx = field.pixel_size_x();
        let row: Vec<f64> = (0..field.nx())
            .map(|j| field.data[[k, 0, j]].min(1e12) - level)
            .collect();
        let xs: Vec<f64> = row
            .windows(2)
            .enumerate()
            .filter(|(_, w)| (w[0] <= 0.0) != (w[1] <= 0.0))
            .map(|(j, w)| (j as f64 + 0.5 + w[0] / (w[0] - w[1])) * dx)
            .collect();
        xs[xs.len() - 1] - xs[0]
    }

    /// Developed depth of column (i, j) from the zero crossing of φ.
    fn phi_depth(phi: &Grid3D<f64>, i: usize, j: usize) -> f64 {
        let dz = phi.pixel_size_z();
        let nz = phi.nz();
        for k in 0..nz {
            let v = phi.data[[k, i, j]];
            if v > 0.0 {
                if k == 0 {
                    return 0.5 * dz - v.min(0.5 * dz);
                }
                let u = phi.data[[k - 1, i, j]];
                return (k as f64 - 0.5 + u / (u - v)) * dz;
            }
        }
        nz as f64 * dz
    }

    #[test]
    fn test_surface_inhibition_factor_fixture() {
        let inh = SurfaceInhibition::new(0.2, 10.0).unwrap();
        assert_relative_eq!(inh.factor(0.0), 0.2, epsilon = 1e-15);
        // 1 − 0.8·e^{−1} (independent fixture).
        assert_relative_eq!(inh.factor(10.0), 0.705_696_447_062_846_2, epsilon = 1e-15);
        assert_relative_eq!(inh.factor(1e6), 1.0, epsilon = 1e-15);
        assert_relative_eq!(SurfaceInhibition::new(0.2, 0.0).unwrap().factor(0.0), 1.0);
        assert!(SurfaceInhibition::new(0.0, 10.0).is_err());
        assert!(SurfaceInhibition::new(0.5, -1.0).is_err());
        assert!(SurfaceInhibition::new(f64::NAN, 1.0).is_err());
    }

    /// With default options except mirror edges, the options entry point
    /// reproduces the legacy solver bit for bit.
    #[test]
    fn test_fmm_with_options_reproduces_legacy_solver() {
        let latent = spot_latent(10, 6, 4.0, 5.0);
        let params = ResistParams::default();
        let legacy = develop_fast_marching(&latent, &params, 4.0, 5.0);
        let options = DevelopmentOptions {
            surface_inhibition: None,
            lateral_boundary: DiffusionBoundary::Reflecting,
        };
        let with = develop_fast_marching_with(&latent, &params, 4.0, 5.0, &options).unwrap();
        for (a, b) in legacy.data.iter().zip(with.data.iter()) {
            assert_eq!(a.to_bits(), b.to_bits());
        }
        let rate = development_rate_volume(&latent, &params, None).unwrap();
        let general = fast_marching_times(&rate, DiffusionBoundary::Reflecting).unwrap();
        for (a, b) in legacy.data.iter().zip(general.data.iter()) {
            assert_eq!(a.to_bits(), b.to_bits());
        }
    }

    /// Uniform rate, developer from the top: the upwind scheme advances a
    /// planar signed distance exactly, so slice k arrives at (k + ½)·dz/R.
    #[test]
    fn test_level_set_uniform_rate_planar_front_is_exact() {
        let (rate_nm_s, dz, t_end) = (100.0, 5.0, 0.8);
        let mut rate = uniform_grid(6, 6, 20, 5.0, dz);
        rate.data.fill(rate_nm_s);
        let mut phi0 = rate.clone();
        fill_depth(&mut phi0);
        for boundary in [DiffusionBoundary::Periodic, DiffusionBoundary::Reflecting] {
            let result =
                evolve_level_set(&phi0, &rate, true, boundary, &LevelSetConfig::new(t_end))
                    .unwrap();
            for k in 0..20 {
                let exact = (k as f64 + 0.5) * dz / rate_nm_s;
                for (i, j) in [(0, 0), (3, 3), (5, 2)] {
                    let t = result.arrival_times.data[[k, i, j]];
                    if exact < t_end {
                        assert_relative_eq!(t, exact, max_relative = 1e-9);
                    } else {
                        assert!(t.is_infinite(), "slice {k} must not develop, got {t}");
                    }
                }
            }
            assert_relative_eq!(
                result.dissolved_thickness_nm,
                rate_nm_s * t_end,
                epsilon = 1e-9
            );
            assert_relative_eq!(result.final_rate_factor, 1.0);
            // Slices 0..=15 (centres ≤ 77.5 nm) are reached; the height map
            // removes whole slices: 100 − 16·5 = 20 nm.
            let hm = result.height_map();
            assert_relative_eq!(hm.data[[2, 2]], 20.0, epsilon = 1e-9);
        }
    }

    /// For a static rate field (a Gaussian exposure spot, Mack rates from
    /// ~0.1 to ~100 nm/s) the level set and fast marching solve the same
    /// problem: arrival times agree to a few percent, and every voxel the two
    /// classify differently at the end lies on the FMM front (≤ 1 voxel).
    #[test]
    fn test_level_set_matches_fast_marching_for_static_rates() {
        let (n, nz, dx, dz, t_end) = (16usize, 12usize, 4.0, 5.0, 4.0);
        let latent = spot_latent(n, nz, dx, dz);
        let params = ResistParams::default();
        for inhibition in [None, Some(SurfaceInhibition::new(0.2, 10.0).unwrap())] {
            let options = DevelopmentOptions {
                surface_inhibition: inhibition,
                lateral_boundary: DiffusionBoundary::Periodic,
            };
            let fmm = develop_fast_marching_with(&latent, &params, dx, dz, &options).unwrap();
            let ls =
                develop_level_set(&latent, &params, &options, &LevelSetConfig::new(t_end)).unwrap();

            let mut rel: Vec<f64> = fmm
                .data
                .iter()
                .zip(ls.arrival_times.data.iter())
                .filter(|(&a, &b)| a <= 0.9 * t_end && b.is_finite())
                .map(|(&a, &b)| (a - b).abs() / a)
                .collect();
            assert!(rel.len() > 500, "too few developed voxels: {}", rel.len());
            rel.sort_by(|a, b| a.total_cmp(b));
            let median = rel[rel.len() / 2];
            let p90 = rel[rel.len() * 9 / 10];
            assert!(median < 0.05, "median relative difference {median}");
            assert!(p90 < 0.08, "90th-percentile relative difference {p90}");

            let dev_fmm = |k: usize, i: usize, j: usize| fmm.data[[k, i, j]] <= t_end;
            for ((k, i, j), &t) in ls.arrival_times.data.indexed_iter() {
                if (t <= t_end) == dev_fmm(k, i, j) {
                    continue;
                }
                let mut on_front = false;
                for dk in -1i64..=1 {
                    let kk = k as i64 + dk;
                    if kk < 0 || kk >= nz as i64 {
                        continue;
                    }
                    for di in -1i64..=1 {
                        for dj in -1i64..=1 {
                            let ii = (i as i64 + di).rem_euclid(n as i64) as usize;
                            let jj = (j as i64 + dj).rem_euclid(n as i64) as usize;
                            on_front |= dev_fmm(kk as usize, ii, jj) != dev_fmm(k, i, j);
                        }
                    }
                }
                assert!(
                    on_front,
                    "voxel ({k},{i},{j}) differs away from the FMM front"
                );
            }
        }
    }

    /// Spherical growth from an interior seed at uniform speed: the front
    /// radius is r0 + R·t. The first-order upwind scheme lags on a barely
    /// resolved sphere (3 cells initial radius) by less than half a cell.
    #[test]
    fn test_level_set_spherical_growth_from_a_seed() {
        let (n, h, r0, rate_nm_s, t_end) = (21usize, 4.0, 12.0, 50.0, 0.3);
        let mut rate = uniform_grid(n, n, n, h, h);
        rate.data.fill(rate_nm_s);
        let c = n as f64 * h / 2.0;
        let radius = |k: usize, i: usize, j: usize| {
            let (x, y, z) = (
                (j as f64 + 0.5) * h - c,
                (i as f64 + 0.5) * h - c,
                (k as f64 + 0.5) * h - c,
            );
            (x * x + y * y + z * z).sqrt()
        };
        let mut phi0 = rate.clone();
        for ((k, i, j), v) in phi0.data.indexed_iter_mut() {
            *v = radius(k, i, j) - r0;
        }
        let result = evolve_level_set(
            &phi0,
            &rate,
            false,
            DiffusionBoundary::Reflecting,
            &LevelSetConfig::new(t_end),
        )
        .unwrap();
        let mut errors = Vec::new();
        for ((k, i, j), &t) in result.arrival_times.data.indexed_iter() {
            let exact = (radius(k, i, j) - r0) / rate_nm_s;
            if exact > 0.02 && exact < 0.8 * t_end {
                assert!(
                    t.is_finite(),
                    "voxel at r = {} never reached",
                    radius(k, i, j)
                );
                errors.push((t - exact) * rate_nm_s); // radial lag in nm
            }
        }
        let max_abs = errors.iter().fold(0.0_f64, |m, e| m.max(e.abs()));
        let mean_abs = errors.iter().map(|e| e.abs()).sum::<f64>() / errors.len() as f64;
        assert!(errors.len() > 300);
        assert!(
            max_abs <= h,
            "max radial error {max_abs} nm exceeds one cell"
        );
        assert!(mean_abs <= 0.5 * h, "mean radial error {mean_abs} nm");
    }

    /// Planar front through a surface-inhibited layer with uniform bulk
    /// rate: t(z) = ∫dz/(R f_inh) = (δ/R)·ln((e^{z/δ} − a)/(1 − a)),
    /// a = 1 − r_s. Both solvers converge to it at first order in dz.
    #[test]
    fn test_inhibited_planar_front_converges_to_analytic_integral() {
        let params = ResistParams {
            development: DevelopmentModel::Mack {
                rmax: 100.0,
                rmin: 0.0,
                mth: 0.5,
                n: 3.0,
            },
            ..ResistParams::default()
        };
        let bulk = params.development.rate(0.0);
        assert_relative_eq!(bulk, 100.0, epsilon = 1e-12);
        let (r_s, delta) = (0.1, 10.0);
        let options = DevelopmentOptions {
            surface_inhibition: Some(SurfaceInhibition::new(r_s, delta).unwrap()),
            lateral_boundary: DiffusionBoundary::Periodic,
        };
        let exact_time = |z: f64| {
            let a = 1.0 - r_s;
            delta / bulk * (((z / delta).exp() - a) / (1.0 - a)).ln()
        };
        // Independent fixture: t(40.25 nm) = 0.631137737...
        assert_relative_eq!(exact_time(40.25), 0.631_137_737_289_623_3, epsilon = 1e-12);

        let mut fmm_err = Vec::new();
        let mut ls_err = Vec::new();
        for dz in [2.0, 1.0, 0.5] {
            let nz = (48.0 / dz) as usize;
            let mut pac = uniform_grid(3, 3, nz, 5.0, dz);
            pac.data.fill(0.0);
            let latent = VolumetricLatentImage { pac };
            let fmm = develop_fast_marching_with(&latent, &params, 5.0, dz, &options).unwrap();
            let ls =
                develop_level_set(&latent, &params, &options, &LevelSetConfig::new(1.0)).unwrap();
            let k = (40.0 / dz) as usize;
            let exact = exact_time((k as f64 + 0.5) * dz);
            fmm_err.push(((fmm.data[[k, 1, 1]] - exact) / exact).abs());
            ls_err.push(((ls.arrival_times.data[[k, 1, 1]] - exact) / exact).abs());
        }
        for err in [&fmm_err, &ls_err] {
            assert!(
                err[0] > err[1] && err[1] > err[2],
                "no convergence: {err:?}"
            );
            assert!(err[2] < 0.04, "error at dz = 0.5 nm: {err:?}");
            assert!(err[0] / err[2] > 2.0, "slower than first order: {err:?}");
        }
    }

    /// Surface inhibition turns the tapered opening of an exposed space into
    /// a T-top: the opening necks just below the surface. Both solvers show
    /// it and agree within one pixel.
    #[test]
    fn test_surface_inhibition_produces_t_top_in_both_solvers() {
        let (nx, nz, dx, dz, t_dev) = (64usize, 25usize, 2.0, 4.0, 3.0);
        let latent = line_latent(nx, nz, dx, dz, 20.0);
        let params = ResistParams::default();
        for inhibition in [None, Some(SurfaceInhibition::new(0.1, 10.0).unwrap())] {
            let options = DevelopmentOptions {
                surface_inhibition: inhibition,
                lateral_boundary: DiffusionBoundary::Periodic,
            };
            let fmm = develop_fast_marching_with(&latent, &params, dx, dz, &options).unwrap();
            let ls =
                develop_level_set(&latent, &params, &options, &LevelSetConfig::new(t_dev)).unwrap();
            let w_fmm: Vec<f64> = (0..=8).map(|k| crossing_width(&fmm, k, t_dev)).collect();
            let w_ls: Vec<f64> = (0..=8).map(|k| crossing_width(&ls.phi, k, 0.0)).collect();
            for (a, b) in w_fmm.iter().zip(&w_ls) {
                assert!((a - b).abs() <= dx, "FMM {w_fmm:?} vs LS {w_ls:?}");
            }
            for w in [&w_fmm, &w_ls] {
                if inhibition.is_none() {
                    // Tapered: the top slice is the widest.
                    let widest = w.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    assert_relative_eq!(w[0], widest);
                    assert!(w[0] > w[1] + dx, "no taper: {w:?}");
                } else {
                    // T-top: necked just below the surface, wider deeper down.
                    let neck = w[1].min(w[2]);
                    assert!(neck < w[0] && neck < w[8] - dx, "no T-top: {w:?}");
                }
            }
        }
    }

    /// Exponential developer ageing g(t) = e^{−t/τ} is a pure time
    /// reparametrization τ(t) = τ(1 − e^{−t/τ}) of the static problem,
    /// which the level set integrates exactly.
    #[test]
    fn test_exponential_ageing_is_exact_time_reparametrization() {
        let (tau, t_end, dz, rate_nm_s) = (1.0, 3.0, 5.0, 100.0);
        let ageing = DeveloperDepletion::Exponential {
            time_constant_s: tau,
        };
        // (a) Planar front: arrival −τ·ln(1 − τ_k/τ), τ_k = (k + ½)dz/R;
        // slices beyond R·τ(t_end) = 95.02 nm never develop.
        let mut rate = uniform_grid(4, 4, 24, 5.0, dz);
        rate.data.fill(rate_nm_s);
        let mut phi0 = rate.clone();
        fill_depth(&mut phi0);
        let config = LevelSetConfig {
            depletion: ageing,
            ..LevelSetConfig::new(t_end)
        };
        let planar =
            evolve_level_set(&phi0, &rate, true, DiffusionBoundary::Periodic, &config).unwrap();
        // Independent fixtures (k = 0, 5, 10).
        for (k, expected) in [
            (0, 0.025_317_807_984_289_897),
            (5, 0.321_583_624_127_462_33),
            (10, 0.744_440_474_947_495_9),
        ] {
            assert_relative_eq!(
                planar.arrival_times.data[[k, 1, 1]],
                expected,
                max_relative = 1e-9
            );
        }
        for k in 19..24 {
            assert!(planar.arrival_times.data[[k, 1, 1]].is_infinite());
        }
        assert_relative_eq!(planar.final_rate_factor, (-3.0_f64).exp(), epsilon = 1e-15);
        assert_relative_eq!(
            planar.dissolved_thickness_nm,
            95.021_293_163_213_6,
            epsilon = 1e-6
        );

        // (b) Non-uniform static field: aged arrival = mapped static arrival.
        let latent = spot_latent(12, 8, 4.0, 5.0);
        let params = ResistParams::default();
        let options = DevelopmentOptions::default();
        let tau_end = tau * (1.0 - (-t_end / tau).exp());
        let fresh =
            develop_level_set(&latent, &params, &options, &LevelSetConfig::new(tau_end)).unwrap();
        let aged = develop_level_set(&latent, &params, &options, &config).unwrap();
        let mut compared = 0;
        for (&a, &b) in fresh
            .arrival_times
            .data
            .iter()
            .zip(aged.arrival_times.data.iter())
        {
            assert_eq!(a.is_finite(), b.is_finite());
            if a.is_finite() {
                let mapped = -tau * (1.0 - a / tau).ln();
                assert_relative_eq!(b, mapped, max_relative = 1e-9, epsilon = 1e-12);
                compared += 1;
            }
        }
        assert!(compared > 100);
    }

    /// Global loading g = 1 − h̄/h_cap on a planar front at uniform rate:
    /// ds/dt = R(1 − s/h_cap) ⇒ arrival at depth z is −(h_cap/R)·ln(1 − z/h_cap).
    #[test]
    fn test_global_loading_planar_front_matches_analytic() {
        let (rate_nm_s, capacity, dz, t_end) = (1000.0, 100.0, 5.0, 0.3);
        let mut pac = uniform_grid(4, 4, 30, 5.0, dz);
        pac.data.fill(0.0);
        let latent = VolumetricLatentImage { pac };
        let params = ResistParams {
            development: DevelopmentModel::Threshold { threshold: 0.5 },
            ..ResistParams::default()
        };
        let config = LevelSetConfig {
            depletion: DeveloperDepletion::Loading {
                capacity_nm: capacity,
            },
            ..LevelSetConfig::new(t_end)
        };
        let result =
            develop_level_set(&latent, &params, &DevelopmentOptions::default(), &config).unwrap();
        for k in 0..30 {
            let z = (k as f64 + 0.5) * dz;
            let t = result.arrival_times.data[[k, 1, 1]];
            let exact = if z < capacity {
                -(capacity / rate_nm_s) * (1.0 - z / capacity).ln()
            } else {
                f64::INFINITY
            };
            if exact < t_end {
                assert_relative_eq!(t, exact, max_relative = 0.01);
            } else {
                assert!(t.is_infinite());
            }
        }
        // s(0.3 s) = 100·(1 − e^{−3}) = 95.021 nm; g = e^{−3}.
        assert!((result.dissolved_thickness_nm - 95.021_293_163_213_6).abs() < 0.5);
        assert!((result.final_rate_factor - (-3.0_f64).exp()).abs() < 0.01);
    }

    /// Local loading couples an opening to the dissolution around it: a
    /// narrow trench develops less deep next to a wide open area within the
    /// depletion length, and not at all differently when the length is tiny.
    #[test]
    fn test_local_loading_slows_openings_near_dense_areas() {
        let (nx, nz, dx, dz) = (48usize, 16usize, 5.0, 5.0);
        let latent = |with_band: bool| {
            let mut pac = uniform_grid(nx, 2, nz, dx, dz);
            for ((_, _, j), m) in pac.data.indexed_iter_mut() {
                let x = (j as f64 + 0.5) * dx;
                let trench = (x - 60.0).abs() < 10.0;
                let band = with_band && (x - 170.0).abs() < 50.0;
                *m = if trench || band { 0.0 } else { 1.0 };
            }
            VolumetricLatentImage { pac }
        };
        let params = ResistParams::default();
        let depth = |depletion: DeveloperDepletion, with_band: bool| {
            let config = LevelSetConfig {
                depletion,
                ..LevelSetConfig::new(1.0)
            };
            let r = develop_level_set(
                &latent(with_band),
                &params,
                &DevelopmentOptions::default(),
                &config,
            )
            .unwrap();
            phi_depth(&r.phi, 0, 12)
        };
        let local = DeveloperDepletion::LocalLoading {
            capacity_nm: 60.0,
            length_nm: 60.0,
        };
        assert!(depth(local, true) < depth(local, false) - dz);
        let isolated = DeveloperDepletion::LocalLoading {
            capacity_nm: 60.0,
            length_nm: 1.0,
        };
        // (The band-limited 1 nm kernel on a 5 nm grid still couples distant
        // columns at the 1e-3 nm level.)
        assert!((depth(isolated, true) - depth(isolated, false)).abs() < 0.05);
        let global = DeveloperDepletion::Loading { capacity_nm: 60.0 };
        assert!(depth(global, true) < depth(global, false) - dz);
    }

    /// Periodic lateral edges make both solvers exactly translation
    /// invariant for a field rolled by whole pixels; mirror edges do not.
    #[test]
    fn test_periodic_lateral_boundary_is_translation_invariant() {
        let (n, nz, dx, dz, shift) = (16usize, 8usize, 4.0, 5.0, 5usize);
        let latent = spot_latent(n, nz, dx, dz);
        let mut rolled = latent.pac.clone();
        for ((k, i, j), &m) in latent.pac.data.indexed_iter() {
            rolled.data[[k, i, (j + shift) % n]] = m;
        }
        let rolled = VolumetricLatentImage { pac: rolled };
        let params = ResistParams::default();
        let max_roll_diff = |a: &Grid3D<f64>, b: &Grid3D<f64>| {
            let mut d = 0.0_f64;
            for ((k, i, j), &x) in a.data.indexed_iter() {
                let y = b.data[[k, i, (j + shift) % n]];
                if x.is_finite() || y.is_finite() {
                    d = d.max((x - y).abs());
                }
            }
            d
        };
        let periodic = DevelopmentOptions {
            surface_inhibition: None,
            lateral_boundary: DiffusionBoundary::Periodic,
        };
        let a = develop_fast_marching_with(&latent, &params, dx, dz, &periodic).unwrap();
        let b = develop_fast_marching_with(&rolled, &params, dx, dz, &periodic).unwrap();
        assert!(max_roll_diff(&a, &b) < 1e-12);
        let config = LevelSetConfig::new(2.0);
        let a = develop_level_set(&latent, &params, &periodic, &config).unwrap();
        let b = develop_level_set(&rolled, &params, &periodic, &config).unwrap();
        assert!(max_roll_diff(&a.arrival_times, &b.arrival_times) < 1e-12);

        let mirror = DevelopmentOptions {
            lateral_boundary: DiffusionBoundary::Reflecting,
            ..periodic
        };
        let a = develop_fast_marching_with(&latent, &params, dx, dz, &mirror).unwrap();
        let b = develop_fast_marching_with(&rolled, &params, dx, dz, &mirror).unwrap();
        assert!(max_roll_diff(&a, &b) > 1e-3);
    }

    #[test]
    fn test_level_set_rejects_invalid_configuration() {
        let latent = spot_latent(4, 3, 4.0, 5.0);
        let params = ResistParams::default();
        let options = DevelopmentOptions::default();
        let base = LevelSetConfig::new(1.0);
        let bad_configs = [
            LevelSetConfig { cfl: 0.0, ..base },
            LevelSetConfig { cfl: 1.5, ..base },
            LevelSetConfig {
                reinit_interval: 0,
                ..base
            },
            LevelSetConfig {
                band_cells: 3.0,
                ..base
            },
            LevelSetConfig {
                dev_time_s: -1.0,
                ..base
            },
            LevelSetConfig {
                dev_time_s: f64::NAN,
                ..base
            },
            LevelSetConfig {
                max_steps: 0,
                ..base
            },
            LevelSetConfig {
                depletion: DeveloperDepletion::Exponential {
                    time_constant_s: 0.0,
                },
                ..base
            },
            LevelSetConfig {
                depletion: DeveloperDepletion::Loading { capacity_nm: -1.0 },
                ..base
            },
            LevelSetConfig {
                depletion: DeveloperDepletion::LocalLoading {
                    capacity_nm: 10.0,
                    length_nm: 0.0,
                },
                ..base
            },
        ];
        for config in bad_configs {
            assert!(
                develop_level_set(&latent, &params, &options, &config).is_err(),
                "config should be rejected: {config:?}"
            );
        }
        // Step budget exhausted -> NumericalError, not a silent truncation.
        let starved = LevelSetConfig {
            max_steps: 1,
            ..base
        };
        assert!(matches!(
            develop_level_set(&latent, &params, &options, &starved),
            Err(LithographyError::NumericalError(_))
        ));
        // Shape mismatch, negative rates, NaN φ0.
        let rate = development_rate_volume(&latent, &params, None).unwrap();
        let other = uniform_grid(5, 4, 3, 4.0, 5.0);
        let periodic = DiffusionBoundary::Periodic;
        assert!(evolve_level_set(&other, &rate, true, periodic, &base).is_err());
        let mut negative = rate.clone();
        negative.data[[0, 0, 0]] = -1.0;
        assert!(evolve_level_set(&rate, &negative, true, periodic, &base).is_err());
        let mut nan_phi = rate.clone();
        nan_phi.data[[1, 1, 1]] = f64::NAN;
        assert!(evolve_level_set(&nan_phi, &rate, true, periodic, &base).is_err());
        assert!(develop_fast_marching_with(&latent, &params, 0.0, 5.0, &options).is_err());
    }

    #[test]
    fn test_developed_indicator_matches_height_map() {
        let latent = spot_latent(8, 6, 4.0, 5.0);
        let params = ResistParams::default();
        let result = develop_level_set(
            &latent,
            &params,
            &DevelopmentOptions::default(),
            &LevelSetConfig::new(2.0),
        )
        .unwrap();
        let indicator = developed_indicator(&result.arrival_times, 2.0);
        let hm = result.height_map();
        for ((i, j), &h) in hm.data.indexed_iter() {
            // Contiguous developed depth from the top equals thickness − height
            // wherever the column's developed voxels form a single top run.
            let deepest = (0..6)
                .filter(|&k| indicator.data[[k, i, j]] > 0.5)
                .map(|k| (k + 1) as f64 * 5.0)
                .fold(0.0, f64::max);
            assert_relative_eq!(30.0 - deepest, h, epsilon = 1e-9);
        }
    }

    #[test]
    fn test_apply_peb_models() {
        // A field that varies only in x (uniform in y and z).
        let mut pac = uniform_grid(16, 4, 6, 4.0, 5.0);
        for ((_, _, j), m) in pac.data.indexed_iter_mut() {
            *m = if (4..10).contains(&j) { 0.2 } else { 0.9 };
        }
        let original = pac.data.clone();
        let mut latent = VolumetricLatentImage { pac };

        assert!(apply_peb(&mut latent, &PebModel::None).unwrap().is_none());
        assert_eq!(latent.pac.data, original);

        // Vertical-only diffusion leaves a z-uniform field unchanged.
        let vertical = PebModel::Gaussian(PebDiffusion::anisotropic(0.0, 20.0));
        assert!(apply_peb(&mut latent, &vertical).unwrap().is_none());
        for (a, b) in latent.pac.data.iter().zip(original.iter()) {
            assert_relative_eq!(a, b, epsilon = 1e-12);
        }
        // Lateral diffusion blurs the x profile but conserves its mean.
        let lateral = PebModel::Gaussian(PebDiffusion::anisotropic(6.0, 0.0));
        apply_peb(&mut latent, &lateral).unwrap();
        assert_relative_eq!(
            latent.pac.data.mean().unwrap(),
            original.mean().unwrap(),
            epsilon = 1e-12
        );
        assert!(latent.pac.data[[0, 0, 6]] > 0.2 + 1e-3);

        // CAR without quencher or diffusion: m = exp(−k_amp·(1 − m_exp)·t).
        let mut latent = VolumetricLatentImage {
            pac: like_grid(&latent.pac, original.clone()),
        };
        let car = CarParams {
            peb_time_s: 30.0,
            k_amp_per_s: 0.2,
            quencher_initial: 0.0,
            acid_diffusivity_nm2_s: 0.0,
            quencher_diffusivity_nm2_s: 0.0,
            ..CarParams::default()
        };
        let state = apply_peb(&mut latent, &PebModel::ChemicallyAmplified(car))
            .unwrap()
            .expect("CAR returns its state");
        for (m, &m_exp) in latent.pac.data.iter().zip(original.iter()) {
            let expected = (-0.2 * (1.0 - m_exp) * 30.0).exp();
            assert_relative_eq!(*m, expected, max_relative = 1e-12);
        }
        assert_relative_eq!(state.neutralized_total, 0.0);
    }

    /// Exact Gaussian kernels: a narrow interior peak spreads with variance
    /// σ_xy² laterally and σ_z² vertically (anisotropic PEB), and the legacy
    /// isotropic entry point applies σ on every axis.
    #[test]
    fn test_peb_3d_anisotropic_axis_variances() {
        let (n, nz, dx, dz) = (48usize, 48usize, 2.0, 1.0);
        let mut pac = uniform_grid(n, n, nz, dx, dz);
        pac.data[[nz / 2, n / 2, n / 2]] = 1.0;
        let variances = |data: &Array3<f64>| {
            let total: f64 = data.sum();
            let mut var = [0.0; 3];
            for ((k, i, j), &v) in data.indexed_iter() {
                let d = [
                    (k as f64 - (nz / 2) as f64) * dz,
                    (i as f64 - (n / 2) as f64) * dx,
                    (j as f64 - (n / 2) as f64) * dx,
                ];
                for a in 0..3 {
                    var[a] += v * d[a] * d[a] / total;
                }
            }
            var
        };
        let mut latent = VolumetricLatentImage { pac: pac.clone() };
        peb_diffuse_3d_anisotropic(&mut latent, &PebDiffusion::anisotropic(8.0, 3.0)).unwrap();
        let var = variances(&latent.pac.data);
        assert_relative_eq!(var[0], 9.0, max_relative = 1e-6);
        assert_relative_eq!(var[1], 64.0, max_relative = 1e-6);
        assert_relative_eq!(var[2], 64.0, max_relative = 1e-6);

        let mut legacy = VolumetricLatentImage { pac };
        peb_diffuse_3d(&mut legacy, 4.0, dx, dz);
        let var = variances(&legacy.pac.data);
        for v in var {
            assert_relative_eq!(v, 16.0, max_relative = 1e-6);
        }
    }
}
