//! LIGA deep X-ray lithography: 1:1 proximity shadow printing.
//!
//! LIGA (Lithographie, Galvanoformung, Abformung) exposes very thick resist
//! (hundreds of um of PMMA) with the hard, penetrating white beam of a
//! synchrotron bending magnet to make high-aspect-ratio microstructures. This
//! is **shadow printing**, not projection: the mask sits a small proximity gap
//! above the resist and casts a near-geometric shadow, so there is no pupil,
//! no aerial-imaging TCC, and no [`crate::source::LithographySource`]
//! illumination here. The mask is an Au absorber pattern on a thin low-Z
//! membrane (Be or Ti); high-energy photons leak through the absorber, setting
//! the dose contrast.
//!
//! # Pipeline
//!
//! ```text
//!   synchrotron BM spectrum  (XraySpectrum::sample -> weighted energy bins)
//!     -> beam filters + membrane transmission        (harden the beam)
//!     -> depth dose u(z) = sum_j w_j E_j T_j mu_j exp(-mu_j z)   (absorbed energy)
//!     -> absolute scale so u(bottom) = target bottom dose
//!   lateral: I_j(x,y) = T_abs + (1 - T_abs) |mask|^2, Fresnel proximity blur
//!     -> volumetric dose D(x,y,z), development depth, sidewall gradient
//! ```
//!
//! # Model status
//!
//! Implemented: polychromatic depth dose with spectral hardening, mixture-rule
//! attenuation ([`crate::materials::attenuation`]), partial-absorber lateral
//! contrast, first-Fresnel-zone proximity blur, volumetric dose, threshold
//! development depth, and a direct-scission PMMA rate model.
//!
//! Approximations (documented at each site): the proximity blur uses the
//! first-Fresnel-zone width `sigma ~ 0.5 sqrt(lambda g)` rather than a full
//! Fresnel-Kirchhoff propagation (planned); photoelectron transport is an
//! optional Gaussian of the Gruen range (off by default); there is **no PEB**
//! (PMMA main-chain scission is a direct radiolysis process, so the exposed
//! latent image develops without a bake); and absolute exposure *time* is not
//! reported because that needs the absolute ring flux, which this spectral
//! model carries only in relative units.

use ndarray::Array2;
use serde::Serialize;

use crate::mask::Mask;
use crate::materials::attenuation::Compound;
use crate::source_models::physics::bm_universal_flux;
use crate::source_models::synchrotron::SynchrotronSource;
use crate::types::{Grid3D, GridConfig};

/// Photon energy [keV] -> wavelength [nm] via `lambda = hc/E`, `hc = 1.239842
/// keV*nm`.
fn wavelength_nm(energy_kev: f64) -> f64 {
    1.239842 / energy_kev
}

/// X-ray exposure spectrum for LIGA shadow printing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum XraySpectrum {
    /// Explicit `(energy, relative flux)` table (e.g. a measured beamline
    /// spectrum, or a single line for a monochromatic study).
    Tabulated {
        /// Photon energies in keV (ascending).
        energies_kev: Vec<f64>,
        /// Relative photon flux at each energy (arbitrary units).
        relative_flux: Vec<f64>,
    },
    /// Synchrotron bending-magnet white beam, characterized entirely by its
    /// critical energy; flux follows the universal function `S(E/E_c)`.
    BendingMagnet {
        /// Critical photon energy in keV (`E_c = 0.665 E^2[GeV] B[T]`).
        critical_energy_kev: f64,
    },
}

impl XraySpectrum {
    /// Map a bending-magnet synchrotron beamline to a [`XraySpectrum::BendingMagnet`]
    /// via its critical energy. Returns `None` for undulator beamlines (they
    /// have no bending-magnet critical energy / white-beam spectrum).
    pub fn from_synchrotron(src: &SynchrotronSource) -> Option<Self> {
        src.critical_energy_kev()
            .map(|e_c| XraySpectrum::BendingMagnet {
                critical_energy_kev: e_c,
            })
    }

    /// A sensible default energy window in keV for sampling this spectrum,
    /// clamped to the tabulated attenuation range [0.1, 20] keV.
    ///
    /// For a bending magnet this spans roughly `0.1 E_c` to `5 E_c` (the flux
    /// is negligible far above `E_c`, and the very soft end is fully absorbed
    /// in the filters/membrane anyway). For a table it is the min/max of the
    /// listed energies.
    pub fn default_energy_range_kev(&self) -> (f64, f64) {
        match self {
            XraySpectrum::BendingMagnet {
                critical_energy_kev,
            } => {
                let lo = (0.1 * critical_energy_kev).clamp(0.1, 20.0);
                let hi = (5.0 * critical_energy_kev).clamp(lo, 20.0);
                (lo, hi)
            }
            XraySpectrum::Tabulated { energies_kev, .. } => {
                let lo = energies_kev
                    .iter()
                    .copied()
                    .fold(f64::INFINITY, f64::min)
                    .max(0.1);
                let hi = energies_kev
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max)
                    .min(20.0);
                (lo.min(hi), hi)
            }
        }
    }

    /// Sample the spectrum into `(energy_kev, weight)` bins with weights
    /// proportional to photon flux and normalized to sum to 1.
    ///
    /// A bending magnet is sampled on `n_bins` equal-width bins across
    /// `[e_min, e_max]` with weight `S(E/E_c)`. A table returns its own points
    /// that fall inside `[e_min, e_max]` (so a single-line table stays exactly
    /// monochromatic); `n_bins` is then ignored.
    pub fn sample(&self, n_bins: usize, e_min_kev: f64, e_max_kev: f64) -> Vec<(f64, f64)> {
        let mut pairs: Vec<(f64, f64)> = match self {
            XraySpectrum::BendingMagnet {
                critical_energy_kev,
            } => {
                let n = n_bins.max(1);
                let de = (e_max_kev - e_min_kev) / n as f64;
                (0..n)
                    .map(|j| {
                        let e = e_min_kev + (j as f64 + 0.5) * de;
                        (e, bm_universal_flux(e / critical_energy_kev))
                    })
                    .collect()
            }
            XraySpectrum::Tabulated {
                energies_kev,
                relative_flux,
            } => energies_kev
                .iter()
                .zip(relative_flux.iter())
                .filter(|(e, _)| **e >= e_min_kev && **e <= e_max_kev)
                .map(|(e, f)| (*e, f.max(0.0)))
                .collect(),
        };

        let sum: f64 = pairs.iter().map(|(_, w)| w).sum();
        if sum > 0.0 {
            for (_, w) in &mut pairs {
                *w /= sum;
            }
        }
        pairs
    }
}

/// An attenuating layer in the beam (filter, mask membrane, or mask absorber).
/// Transmission is `exp(-mu(E) t)` with `mu` in 1/um and `t` in um.
#[derive(Debug, Clone, Serialize)]
pub struct BeamFilter {
    /// Material of the layer.
    pub compound: Compound,
    /// Layer thickness in um.
    pub thickness_um: f64,
}

impl BeamFilter {
    /// Construct a beam filter of the given compound and thickness.
    pub fn new(compound: Compound, thickness_um: f64) -> Self {
        Self {
            compound,
            thickness_um,
        }
    }

    /// Intensity transmission `exp(-mu(E) t)` at `energy_kev` (in [0, 1]).
    pub fn transmission(&self, energy_kev: f64) -> f64 {
        (-self.compound.mu_per_um(energy_kev) * self.thickness_um).exp()
    }
}

/// Configuration for a LIGA deep X-ray exposure.
///
/// Serializable but not deserializable (its [`Compound`]s reference `&'static`
/// element symbols); build one with [`DeepXrayConfig::pmma_default`] or the
/// struct literal.
#[derive(Debug, Clone, Serialize)]
pub struct DeepXrayConfig {
    /// Exposure spectrum.
    pub spectrum: XraySpectrum,
    /// Beam-conditioning filters upstream of the mask (harden the beam).
    pub filters: Vec<BeamFilter>,
    /// Mask absorber (patterned Au); high-E photons leak through it.
    pub absorber: BeamFilter,
    /// Mask membrane carrying the absorber (e.g. Be or Ti).
    pub membrane: BeamFilter,
    /// Resist compound (e.g. PMMA).
    pub resist: Compound,
    /// Resist thickness in um.
    pub resist_thickness_um: f64,
    /// Mask-to-resist proximity gap in um (sets the Fresnel blur).
    pub proximity_gap_um: f64,
    /// Target absorbed-energy density at the resist bottom in kJ/cm^3 (the
    /// clearing dose the process is scaled to reach in open features).
    pub target_bottom_dose_kj_cm3: f64,
    /// Absorbed-energy density above which the resist/substrate is damaged
    /// (foaming, T-topping); the top dose should stay below this.
    pub damage_dose_kj_cm3: f64,
    /// Whether to add a photoelectron-transport (Gruen range) blur.
    pub photoelectron_blur: bool,
    /// Number of energy bins for a bending-magnet spectrum.
    pub energy_bins: usize,
}

impl DeepXrayConfig {
    /// Standard thick-PMMA LIGA preset: 500 um PMMA, 20 um Au absorber on a
    /// 2 um Ti membrane, 100 um proximity gap, 3 kJ/cm^3 bottom clearing dose
    /// with a 20 kJ/cm^3 damage ceiling, 100 energy bins, no upstream filters.
    pub fn pmma_default(spectrum: XraySpectrum) -> Self {
        Self {
            spectrum,
            filters: Vec::new(),
            absorber: BeamFilter::new(Compound::gold(19.3), 20.0),
            membrane: BeamFilter::new(Compound::titanium(4.51), 2.0),
            resist: Compound::pmma(),
            resist_thickness_um: 500.0,
            proximity_gap_um: 100.0,
            target_bottom_dose_kj_cm3: 3.0,
            damage_dose_kj_cm3: 20.0,
            photoelectron_blur: false,
            energy_bins: 100,
        }
    }

    /// Sample the spectrum over its default energy window using `energy_bins`.
    fn sample_spectrum(&self) -> Vec<(f64, f64)> {
        let (lo, hi) = self.spectrum.default_energy_range_kev();
        self.spectrum.sample(self.energy_bins, lo, hi)
    }

    /// Combined transmission of all upstream filters plus the membrane at
    /// `energy_kev` (everything the beam crosses before the resist, in the
    /// open areas). The absorber is *not* included: it applies only under the
    /// patterned regions and is handled laterally.
    fn beam_transmission(&self, energy_kev: f64) -> f64 {
        let mut t = self.membrane.transmission(energy_kev);
        for f in &self.filters {
            t *= f.transmission(energy_kev);
        }
        t
    }
}

/// Depth-dose profile as `(z_um, relative absorbed-energy density)` from the
/// resist top (`z = 0`) to the bottom (`z = resist_thickness`).
///
/// `u(z) = sum_j w_j E_j T_beam(E_j) mu_j exp(-mu_j z)` over the sampled
/// spectrum, with `mu_j` the resist linear attenuation in 1/um. Values are
/// relative (arbitrary units); [`expose_depth`] scales them to absolute dose.
pub fn depth_dose(config: &DeepXrayConfig) -> Vec<(f64, f64)> {
    const DEPTH_SAMPLES: usize = 128;
    let pairs = config.sample_spectrum();
    // Precompute per-bin coefficients: w_j E_j T_beam(E_j) and mu_j.
    let terms: Vec<(f64, f64)> = pairs
        .iter()
        .map(|(e, w)| {
            let amp = w * e * config.beam_transmission(*e);
            let mu = config.resist.mu_per_um(*e);
            (amp * mu, mu)
        })
        .collect();

    let thickness = config.resist_thickness_um;
    let denom = (DEPTH_SAMPLES - 1).max(1) as f64;
    (0..DEPTH_SAMPLES)
        .map(|k| {
            let z = k as f64 / denom * thickness;
            let u: f64 = terms.iter().map(|(c, mu)| c * (-mu * z).exp()).sum();
            (z, u)
        })
        .collect()
}

/// Absolute LIGA depth-dose result: the profile scaled so the resist bottom
/// receives the configured clearing dose.
#[derive(Debug, Clone, Serialize)]
pub struct LigaExposure {
    /// Depth grid in um (top `z=0` to bottom `z=thickness`).
    pub z_um: Vec<f64>,
    /// Absolute absorbed-energy density in kJ/cm^3 at each depth.
    pub dose_kj_cm3: Vec<f64>,
    /// Multiplicative scale applied to the relative profile to hit the target
    /// bottom dose.
    pub scale: f64,
    /// Dose at the resist top in kJ/cm^3.
    pub top_dose_kj_cm3: f64,
    /// Dose at the resist bottom in kJ/cm^3 (= target when reachable).
    pub bottom_dose_kj_cm3: f64,
    /// Top/bottom dose ratio (>= 1; contrast the process must tolerate).
    pub dose_ratio: f64,
    /// Whether the top dose exceeds the damage ceiling.
    pub exceeds_damage_ceiling: bool,
    /// Absolute exposure time: intentionally `None`. It requires the absolute
    /// ring flux (photons/s), which this relative spectral model does not
    /// carry; only relative dose and ratios are physical here.
    pub exposure_time_estimate: Option<f64>,
}

/// Compute the absolute depth-dose exposure for a configuration: run
/// [`depth_dose`], then scale so the bottom of the resist reaches
/// `target_bottom_dose_kj_cm3`.
pub fn expose_depth(config: &DeepXrayConfig) -> LigaExposure {
    let profile = depth_dose(config);
    let z_um: Vec<f64> = profile.iter().map(|(z, _)| *z).collect();
    let u: Vec<f64> = profile.iter().map(|(_, u)| *u).collect();

    let u_bottom = *u.last().unwrap_or(&0.0);
    let scale = if u_bottom > 0.0 {
        config.target_bottom_dose_kj_cm3 / u_bottom
    } else {
        0.0
    };

    let dose_kj_cm3: Vec<f64> = u.iter().map(|v| v * scale).collect();
    let top_dose_kj_cm3 = *dose_kj_cm3.first().unwrap_or(&0.0);
    let bottom_dose_kj_cm3 = *dose_kj_cm3.last().unwrap_or(&0.0);
    let ratio = if bottom_dose_kj_cm3 > 0.0 {
        top_dose_kj_cm3 / bottom_dose_kj_cm3
    } else {
        f64::INFINITY
    };

    LigaExposure {
        z_um,
        dose_kj_cm3,
        scale,
        top_dose_kj_cm3,
        bottom_dose_kj_cm3,
        dose_ratio: ratio,
        exceeds_damage_ceiling: top_dose_kj_cm3 > config.damage_dose_kj_cm3,
        exposure_time_estimate: None,
    }
}

/// Per-energy-bin lateral shadow intensity before blur:
/// `I_j = T_abs(E_j) + (1 - T_abs(E_j)) P`, where `P` is the mask intensity
/// transmittance (`|rasterized amplitude|^2`). Under the absorber `P = 0`, so
/// `I_j = T_abs` (the high-energy leakage floor); in open areas `P = 1`, so
/// `I_j = 1`.
fn absorber_image(config: &DeepXrayConfig, energy_kev: f64, mask_p: &Array2<f64>) -> Array2<f64> {
    let t_abs = config.absorber.transmission(energy_kev);
    mask_p.mapv(|p| t_abs + (1.0 - t_abs) * p)
}

/// First-Fresnel-zone proximity blur width in nm for photon energy
/// `energy_kev` over gap `gap_um`: `sigma = 0.5 sqrt(lambda[nm] g[nm])`.
/// This is the geometric penumbra / diffraction scale; a full
/// Fresnel-Kirchhoff propagation is planned but not implemented.
fn proximity_sigma_nm(energy_kev: f64, gap_um: f64) -> f64 {
    let lambda_nm = wavelength_nm(energy_kev);
    let gap_nm = gap_um * 1e3;
    0.5 * (lambda_nm * gap_nm).sqrt()
}

/// Gruen-range photoelectron blur width in nm for `energy_kev` in a resist of
/// density `density_g_cm3`: `R_G[um] = 0.046 E[keV]^1.75 / rho`, converted to
/// nm. Documented approximation to the practical electron range that limits
/// LIGA sidewall sharpness; off by default.
pub fn grun_range_nm(energy_kev: f64, density_g_cm3: f64) -> f64 {
    let r_g_um = 0.046 * energy_kev.powf(1.75) / density_g_cm3;
    r_g_um * 1e3
}

/// Total per-bin blur width in nm: proximity penumbra combined in quadrature
/// with the optional photoelectron range.
fn bin_sigma_nm(config: &DeepXrayConfig, energy_kev: f64) -> f64 {
    let sigma_prox = proximity_sigma_nm(energy_kev, config.proximity_gap_um);
    if config.photoelectron_blur {
        let sigma_e = grun_range_nm(energy_kev, config.resist.density_g_cm3);
        (sigma_prox * sigma_prox + sigma_e * sigma_e).sqrt()
    } else {
        sigma_prox
    }
}

/// Renormalized per-bin lateral weights `w_j' = w_j T_beam(E_j)` (filters +
/// membrane, absorber excluded), summing to 1. Returned alongside the
/// `(energy, base weight)` pairs.
fn lateral_weights(config: &DeepXrayConfig, pairs: &[(f64, f64)]) -> Vec<f64> {
    let raw: Vec<f64> = pairs
        .iter()
        .map(|(e, w)| w * config.beam_transmission(*e))
        .collect();
    let sum: f64 = raw.iter().sum();
    if sum > 0.0 {
        raw.iter().map(|r| r / sum).collect()
    } else {
        raw
    }
}

/// Lateral shadow image `I(x,y)` accumulated over the spectrum, with the mask
/// rasterized onto `grid`.
///
/// For each energy bin the partial-absorber contrast (`absorber_image`) is
/// blurred by the first-Fresnel-zone Gaussian (and optional photoelectron
/// range), then summed with the renormalized filter/membrane weights. Open
/// areas approach 1, absorber-covered areas approach the (energy-averaged)
/// high-E leakage floor `> 0`.
pub fn shadow_image(config: &DeepXrayConfig, mask: &Mask, grid: &GridConfig) -> Array2<f64> {
    let mask_p = mask.rasterize(grid).mapv(|c| c.norm_sqr());
    let pairs = config.sample_spectrum();
    let weights = lateral_weights(config, &pairs);

    let (ny, nx) = mask_p.dim();
    let mut acc = Array2::<f64>::zeros((ny, nx));
    for ((e, _), wp) in pairs.iter().zip(weights.iter()) {
        if *wp <= 0.0 {
            continue;
        }
        let img = absorber_image(config, *e, &mask_p);
        let sigma_px = bin_sigma_nm(config, *e) / grid.pixel_nm;
        let blurred = gaussian_blur_2d(&img, sigma_px);
        acc.scaled_add(*wp, &blurred);
    }
    acc
}

/// Volumetric absorbed-dose field `D(x,y,z)` over the resist thickness.
///
/// `D = sum_j w_j' I_j(x,y) E_j mu_j exp(-mu_j z)`, with `I_j` the blurred
/// per-bin lateral shadow, `mu_j` the resist attenuation, and `z` in um. The
/// field is scaled (like [`expose_depth`]) so that a fully open column
/// (`P = 1`) reaches `target_bottom_dose_kj_cm3` at the resist bottom, giving
/// per-voxel dose in kJ/cm^3. The `z` axis is stored in nm in the returned
/// grid (`z_min_nm = 0`, `z_max_nm = thickness * 1000`).
pub fn expose_volumetric(
    config: &DeepXrayConfig,
    mask: &Mask,
    grid: &GridConfig,
    nz: usize,
) -> crate::error::Result<Grid3D<f64>> {
    if config.resist_thickness_um <= 0.0 {
        return Err(crate::error::LithographyError::InvalidParameter {
            name: "resist_thickness_um",
            value: config.resist_thickness_um,
            reason: "must be positive",
        });
    }
    let mask_p = mask.rasterize(grid).mapv(|c| c.norm_sqr());
    let pairs = config.sample_spectrum();
    let weights = lateral_weights(config, &pairs);

    let n = grid.size;
    let half = grid.field_size_nm() / 2.0;
    let thickness_um = config.resist_thickness_um;
    let mut vol = Grid3D::<f64>::new(
        n,
        n,
        nz,
        (-half, half),
        (-half, half),
        (0.0, thickness_um * 1e3),
    )?;

    // Depth centers in um for each z-slice.
    let z_um: Vec<f64> = (0..nz).map(|k| vol.z_at(k) * 1e-3).collect();

    // Open-area (P=1) dose at the deepest sampled voxel center, used for
    // absolute scaling (cell centers sit at thickness - dz/2, so we reference
    // the last slice rather than the nominal thickness).
    let z_bottom = z_um.last().copied().unwrap_or(thickness_um);
    let mut open_bottom = 0.0;
    for ((e, _), wp) in pairs.iter().zip(weights.iter()) {
        let mu = config.resist.mu_per_um(*e);
        open_bottom += wp * e * mu * (-mu * z_bottom).exp();
    }
    let scale = if open_bottom > 0.0 {
        config.target_bottom_dose_kj_cm3 / open_bottom
    } else {
        0.0
    };

    for ((e, _), wp) in pairs.iter().zip(weights.iter()) {
        if *wp <= 0.0 {
            continue;
        }
        let mu = config.resist.mu_per_um(*e);
        let sigma_px = bin_sigma_nm(config, *e) / grid.pixel_nm;
        let lateral = gaussian_blur_2d(&absorber_image(config, *e, &mask_p), sigma_px);
        for (k, z) in z_um.iter().enumerate() {
            let depth = e * mu * (-mu * z).exp();
            let factor = wp * depth * scale;
            for i in 0..n {
                for j in 0..n {
                    vol.data[[k, i, j]] += factor * lateral[[i, j]];
                }
            }
        }
    }

    Ok(vol)
}

/// Top/bottom dose ratio of a depth-dose exposure (the contrast the resist
/// process must span between the over-exposed top and the just-cleared bottom).
pub fn dose_ratio(exposure: &LigaExposure) -> f64 {
    exposure.dose_ratio
}

/// Maximum achievable aspect ratio: resist thickness divided by minimum
/// printable feature. Both are lengths; the result is dimensionless (e.g.
/// 500 um / 5 um = 100).
pub fn max_aspect_ratio(exposure: &LigaExposure, min_feature_nm: f64) -> f64 {
    let thickness_nm = exposure.z_um.last().copied().unwrap_or(0.0) * 1e3;
    if min_feature_nm > 0.0 {
        thickness_nm / min_feature_nm
    } else {
        f64::INFINITY
    }
}

/// Per-z-slice maximum lateral dose gradient `|dD/dx|` (per nm) along the
/// center row, evaluated at the crossing of `threshold`. This is the sidewall
/// steepness: a larger gradient at the threshold crossing means a sharper,
/// more vertical wall. Slices with no crossing return 0.
pub fn sidewall_dose_gradient(volume: &Grid3D<f64>, threshold: f64) -> Vec<f64> {
    let nx = volume.nx();
    let ny = volume.ny();
    let nz = volume.nz();
    let center_row = ny / 2;
    let dx_nm = volume.pixel_size_x();
    (0..nz)
        .map(|k| {
            let mut max_grad = 0.0_f64;
            for j in 0..nx.saturating_sub(1) {
                let a = volume.data[[k, center_row, j]];
                let b = volume.data[[k, center_row, j + 1]];
                let crosses =
                    (a - threshold) * (b - threshold) <= 0.0 && (a - threshold) != (b - threshold);
                if crosses {
                    let grad = (b - a).abs() / dx_nm;
                    if grad > max_grad {
                        max_grad = grad;
                    }
                }
            }
            max_grad
        })
        .collect()
}

/// Development depth (in nm) reached from the resist top at each `(y, x)`
/// column, for a positive-tone threshold model: scanning `z` downward from the
/// top, the depth of the *contiguous* run of voxels whose dose is at or above
/// `threshold_dose`, stopping at the first voxel below it.
///
/// There is no post-exposure bake: PMMA main-chain scission is a direct
/// radiolytic process, so the above-threshold region dissolves directly.
pub fn develop_depth(volume: &Grid3D<f64>, threshold_dose: f64) -> Array2<f64> {
    let nx = volume.nx();
    let ny = volume.ny();
    let nz = volume.nz();
    let dz_nm = volume.pixel_size_z();
    let mut depth = Array2::<f64>::zeros((ny, nx));
    for i in 0..ny {
        for j in 0..nx {
            let mut count = 0usize;
            for k in 0..nz {
                if volume.data[[k, i, j]] >= threshold_dose {
                    count += 1;
                } else {
                    break;
                }
            }
            depth[[i, j]] = count as f64 * dz_nm;
        }
    }
    depth
}

/// PMMA development rate `R = R0 (D/D0)^q` for dose `dose_kj_cm3`, reference
/// rate `r0_um_min` [um/min] at reference dose `d0_kj_cm3`, and contrast
/// exponent `q`. A power-law fit to GG/PGMEA developer data; returns 0 for
/// non-positive dose.
pub fn pmma_development_rate(dose_kj_cm3: f64, r0_um_min: f64, d0_kj_cm3: f64, q: f64) -> f64 {
    if dose_kj_cm3 <= 0.0 || d0_kj_cm3 <= 0.0 {
        return 0.0;
    }
    r0_um_min * (dose_kj_cm3 / d0_kj_cm3).powf(q)
}

/// Separable Gaussian blur of a 2D field with clamped (edge-replicated)
/// boundaries; `sigma_px` is the standard deviation in pixels. Mirrors
/// [`crate::resist::peb_diffuse`]. A `sigma_px <= 0` is a no-op (returns a copy).
fn gaussian_blur_2d(img: &Array2<f64>, sigma_px: f64) -> Array2<f64> {
    if sigma_px <= 0.0 {
        return img.clone();
    }
    let radius = (3.0 * sigma_px).ceil() as usize;
    let size = 2 * radius + 1;
    let mut kernel = vec![0.0; size];
    let mut sum = 0.0;
    for (i, k) in kernel.iter_mut().enumerate() {
        let d = i as f64 - radius as f64;
        *k = (-d * d / (2.0 * sigma_px * sigma_px)).exp();
        sum += *k;
    }
    for k in &mut kernel {
        *k /= sum;
    }

    let (ny, nx) = img.dim();
    let mut temp = Array2::<f64>::zeros((ny, nx));
    // Along x.
    for i in 0..ny {
        for j in 0..nx {
            let mut val = 0.0;
            for (k, &kv) in kernel.iter().enumerate() {
                let jj = (j as i64 + k as i64 - radius as i64).clamp(0, nx as i64 - 1) as usize;
                val += img[[i, jj]] * kv;
            }
            temp[[i, j]] = val;
        }
    }
    // Along y.
    let mut out = Array2::<f64>::zeros((ny, nx));
    for i in 0..ny {
        for j in 0..nx {
            let mut val = 0.0;
            for (k, &kv) in kernel.iter().enumerate() {
                let ii = (i as i64 + k as i64 - radius as i64).clamp(0, ny as i64 - 1) as usize;
                val += temp[[ii, j]] * kv;
            }
            out[[i, j]] = val;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn mono_config(energy_kev: f64, thickness_um: f64) -> DeepXrayConfig {
        // Monochromatic, no filters, transparent membrane (0 um): isolates the
        // resist exponential.
        DeepXrayConfig {
            spectrum: XraySpectrum::Tabulated {
                energies_kev: vec![energy_kev],
                relative_flux: vec![1.0],
            },
            filters: Vec::new(),
            absorber: BeamFilter::new(Compound::gold(19.3), 20.0),
            membrane: BeamFilter::new(Compound::beryllium(1.85), 0.0),
            resist: Compound::pmma(),
            resist_thickness_um: thickness_um,
            proximity_gap_um: 10.0,
            target_bottom_dose_kj_cm3: 3.0,
            damage_dose_kj_cm3: 20.0,
            photoelectron_blur: false,
            energy_bins: 32,
        }
    }

    #[test]
    fn test_monochromatic_depth_dose_exponential() {
        let energy = 5.0;
        let thickness = 200.0;
        let config = mono_config(energy, thickness);
        let exposure = expose_depth(&config);

        // Single energy => u(z) = C exp(-mu z), so top/bottom = exp(mu T).
        let mu = config.resist.mu_per_um(energy);
        let expected = (mu * thickness).exp();
        assert_relative_eq!(exposure.dose_ratio, expected, max_relative = 1e-9);

        // Every sampled point matches the exact exponential.
        let scale = exposure.scale;
        let c0 = exposure.dose_kj_cm3[0] / scale; // relative u(0)
        for (z, d) in exposure.z_um.iter().zip(exposure.dose_kj_cm3.iter()) {
            let u = d / scale;
            assert_relative_eq!(u, c0 * (-mu * z).exp(), max_relative = 1e-9);
        }
        // Bottom hits the target dose.
        assert_relative_eq!(exposure.bottom_dose_kj_cm3, 3.0, max_relative = 1e-9);
    }

    #[test]
    fn test_spectral_hardening_decay_slows_with_depth() {
        let config = DeepXrayConfig::pmma_default(XraySpectrum::BendingMagnet {
            critical_energy_kev: 6.234,
        });
        let profile = depth_dose(&config);
        // Local decay rate -d ln u / dz between successive samples.
        let rate = |i: usize| {
            let (z0, u0) = profile[i];
            let (z1, u1) = profile[i + 1];
            -(u1.ln() - u0.ln()) / (z1 - z0)
        };
        let shallow = rate(0);
        let mid = rate(profile.len() / 2);
        let deep = rate(profile.len() - 2);
        // Beam hardens with depth: the soft, strongly-absorbed photons are
        // gone, so the effective decay rate falls monotonically.
        assert!(
            shallow > mid && mid > deep,
            "decay should slow with depth: {shallow} > {mid} > {deep}"
        );
    }

    #[test]
    fn test_filter_hardens_beam_lowers_ratio() {
        let spectrum = XraySpectrum::BendingMagnet {
            critical_energy_kev: 6.234,
        };
        let base = DeepXrayConfig::pmma_default(spectrum.clone());
        let ratio_base = expose_depth(&base).dose_ratio;

        let mut filtered = DeepXrayConfig::pmma_default(spectrum);
        filtered
            .filters
            .push(BeamFilter::new(Compound::beryllium(1.85), 100.0));
        let ratio_filtered = expose_depth(&filtered).dose_ratio;

        assert!(
            ratio_filtered < ratio_base,
            "Be filter should harden the beam and lower the top/bottom ratio: \
             {ratio_filtered} < {ratio_base}"
        );
    }

    #[test]
    fn test_shadow_image_open_brighter_than_absorber() {
        let config = DeepXrayConfig {
            proximity_gap_um: 10.0,
            ..DeepXrayConfig::pmma_default(XraySpectrum::BendingMagnet {
                critical_energy_kev: 6.234,
            })
        };
        // Wide lines/spaces so feature centers survive the proximity blur.
        let mask = Mask::line_space(400.0, 800.0).unwrap();
        let grid = GridConfig::new(128, 20.0).unwrap();
        let img = shadow_image(&config, &mask, &grid);

        let max = img.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min = img.iter().cloned().fold(f64::INFINITY, f64::min);
        // Open areas brighter than absorber-covered areas.
        assert!(max > min, "open ({max}) should exceed absorber ({min})");
        // Au is partially transparent to hard photons: the floor is nonzero.
        assert!(
            min > 0.0,
            "absorber floor should be > 0 (leakage), got {min}"
        );
        // Open areas approach full transmission (weights sum to 1).
        assert!(max <= 1.0 + 1e-9 && max > 0.5, "open value {max}");
    }

    #[test]
    fn test_develop_depth_full_and_zero() {
        let mut vol =
            Grid3D::<f64>::new(4, 4, 10, (-100.0, 100.0), (-100.0, 100.0), (0.0, 500_000.0))
                .unwrap();
        // Column (0,0): full dose everywhere; column (1,1): zero dose.
        for k in 0..vol.nz() {
            vol.data[[k, 0, 0]] = 5.0;
        }
        let depth = develop_depth(&vol, 1.0);
        // Full-dose column develops to the full thickness.
        assert_relative_eq!(depth[[0, 0]], 500_000.0, max_relative = 1e-12);
        // Zero-dose column does not develop at all.
        assert_relative_eq!(depth[[1, 1]], 0.0);
    }

    #[test]
    fn test_max_aspect_ratio_sanity() {
        // 500 um resist, 5 um minimum feature => aspect 100.
        let exposure = LigaExposure {
            z_um: vec![0.0, 250.0, 500.0],
            dose_kj_cm3: vec![0.0; 3],
            scale: 1.0,
            top_dose_kj_cm3: 0.0,
            bottom_dose_kj_cm3: 0.0,
            dose_ratio: 1.0,
            exceeds_damage_ceiling: false,
            exposure_time_estimate: None,
        };
        assert_relative_eq!(
            max_aspect_ratio(&exposure, 5000.0),
            100.0,
            max_relative = 1e-12
        );
    }

    #[test]
    fn test_grun_range_fixture() {
        // 0.046 * 8^1.75 / 1.19 ~ 1.47 um for PMMA at 8 keV (formula check).
        let r_g_um = grun_range_nm(8.0, 1.19) / 1e3;
        assert_relative_eq!(
            r_g_um,
            0.046 * 8.0_f64.powf(1.75) / 1.19,
            max_relative = 1e-12
        );
        assert!((1.46..=1.50).contains(&r_g_um), "Gruen range {r_g_um} um");
    }

    #[test]
    fn test_from_synchrotron_bending_magnet() {
        let bm = SynchrotronSource::liga_bending_magnet();
        let spectrum = XraySpectrum::from_synchrotron(&bm).unwrap();
        match spectrum {
            XraySpectrum::BendingMagnet {
                critical_energy_kev,
            } => assert_relative_eq!(critical_energy_kev, 6.234, epsilon = 1e-3),
            _ => panic!("expected bending magnet"),
        }
        // Undulator beamlines have no white-beam spectrum.
        let und = SynchrotronSource::compact_euv_undulator().unwrap();
        assert!(XraySpectrum::from_synchrotron(&und).is_none());
    }

    #[test]
    fn test_sample_weights_sum_to_one() {
        let bm = XraySpectrum::BendingMagnet {
            critical_energy_kev: 6.234,
        };
        let sum: f64 = bm.sample(50, 0.5, 30.0).iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, max_relative = 1e-12);

        let tab = XraySpectrum::Tabulated {
            energies_kev: vec![2.0, 5.0, 10.0],
            relative_flux: vec![1.0, 2.0, 1.0],
        };
        let sum: f64 = tab.sample(10, 0.1, 20.0).iter().map(|(_, w)| w).sum();
        assert_relative_eq!(sum, 1.0, max_relative = 1e-12);
    }

    #[test]
    fn test_volumetric_open_column_hits_target() {
        let config = DeepXrayConfig {
            proximity_gap_um: 5.0,
            resist_thickness_um: 100.0,
            ..DeepXrayConfig::pmma_default(XraySpectrum::BendingMagnet {
                critical_energy_kev: 6.234,
            })
        };
        // Fully clear mask => every column is open (P = 1): bright field with
        // no absorber features leaves the whole field transparent.
        let mask = Mask {
            mask_type: crate::mask::MaskType::Binary,
            features: Vec::new(),
            dark_field: false,
        };
        let grid = GridConfig::new(16, 50.0).unwrap();
        let vol = expose_volumetric(&config, &mask, &grid, 20).unwrap();
        // Bottom slice, center column: open-area dose should hit the target.
        let bottom = vol.nz() - 1;
        let center = vol.data[[bottom, 8, 8]];
        assert_relative_eq!(center, 3.0, max_relative = 1e-6);
        // Dose decreases from top to bottom (attenuation).
        let top = vol.data[[0, 8, 8]];
        assert!(top > center, "top {top} should exceed bottom {center}");
    }

    #[test]
    fn test_pmma_development_rate_power_law() {
        // R = R0 (D/D0)^q; at D = D0 the rate is R0.
        assert_relative_eq!(
            pmma_development_rate(5.0, 2.0, 5.0, 3.0),
            2.0,
            max_relative = 1e-12
        );
        // Doubling dose with q=3 gives 8x rate.
        assert_relative_eq!(
            pmma_development_rate(10.0, 2.0, 5.0, 3.0),
            2.0 * 8.0,
            max_relative = 1e-12
        );
        // Non-positive dose develops nothing.
        assert_eq!(pmma_development_rate(0.0, 2.0, 5.0, 3.0), 0.0);
    }

    #[test]
    fn test_sidewall_gradient_detects_edge() {
        let config = DeepXrayConfig {
            proximity_gap_um: 5.0,
            resist_thickness_um: 100.0,
            ..DeepXrayConfig::pmma_default(XraySpectrum::BendingMagnet {
                critical_energy_kev: 6.234,
            })
        };
        let mask = Mask::line_space(400.0, 800.0).unwrap();
        let grid = GridConfig::new(64, 20.0).unwrap();
        let vol = expose_volumetric(&config, &mask, &grid, 8).unwrap();
        // With a patterned mask there is a lateral dose step, so at least one
        // slice has a nonzero sidewall gradient at a mid-level threshold.
        let grads = sidewall_dose_gradient(&vol, 1.5);
        assert_eq!(grads.len(), vol.nz());
        assert!(
            grads.iter().any(|&g| g > 0.0),
            "expected a nonzero sidewall gradient somewhere"
        );
    }
}
