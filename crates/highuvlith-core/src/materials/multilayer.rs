//! EUV / BEUV periodic multilayer mirrors (Bragg reflectors).
//!
//! Near-normal-incidence mirrors at 13.5 nm (Mo/Si) and 6.7 nm (La/B4C)
//! reflect by constructive interference of the weak reflections from many
//! periods of a high-/low-index bilayer. This module computes the complex
//! reflection amplitude of such a stack for s (TE) and p (TM) light at any
//! wavelength and angle, with wavelength-dependent optical constants from the
//! Henke/CXRO tables ([`crate::materials::henke`]) and interface roughness,
//! plus the figures of merit (peak reflectance, bandwidth, angular
//! acceptance) and a pupil-response function that a reflective objective
//! (e.g. the Schwarzschild optics) can sample at each ray's incidence angle.
//!
//! # Key equations
//!
//! Media `j = 0` (vacuum) ... `N+1` (substrate), indices `n_j = 1 - delta_j + i
//! beta_j`, Snell invariant `s = sin theta_0` (theta from the surface normal):
//!
//! ```text
//!   q_j   = sqrt(n_j^2 - s^2)            (Im q_j >= 0: the field decays downward)
//!   eta_j = q_j (s / TE),   n_j^2 / q_j (p / TM)
//!   r_jk  = (eta_j - eta_k) / (eta_j + eta_k)                       (Fresnel)
//!   r~_jk = r_jk exp(-2 k0^2 q_j q_k sigma^2)                   (Nevot-Croce)
//!   R_N   = r~_N,N+1;   R_j-1 = (r~_j-1,j + R_j e^{2 i k0 q_j d_j})
//!                              / (1 + r~_j-1,j R_j e^{2 i k0 q_j d_j})   (Parratt)
//!   r = R_0 (phase referred to the top surface),   R = |r|^2
//! ```
//!
//! The p (TM) coefficients are ratios of tangential E fields (the admittance
//! convention of [`crate::thinfilm`]); unpolarized reflectance is the mean of
//! s and p. First-order Bragg condition with refraction (period `d`, mean
//! index decrement `delta_bar`): `m lambda = 2 d sqrt(cos^2 theta - 2
//! delta_bar)`, valid for `delta_bar << cos^2 theta` ([`bragg_period_nm`]);
//! for Mo/Si it places the peak to within ~2% (the exact peak lies slightly
//! longward because of the reflection phases of absorbing interfaces), so
//! use [`tune_period_nm`] for designs.
//!
//! # Validation
//!
//! With `sigma = 0` the Parratt recursion agrees with the independent
//! characteristic-matrix solution of [`crate::thinfilm::FilmStack`] to
//! ~1e-12; against the CXRO multilayer calculator
//! (`henke.lbl.gov/optical_constants/multi2.html`, queried 2026-09-30:
//! Si/Mo d = 6.9 nm, Gamma = 0.4, N = 40 on SiO2, with and without 0.5 nm
//! roughness; B4C/La d = 3.4 nm, Gamma = 0.4, N = 200 on Si) the reflectance
//! agrees to better than 3e-3 absolute across the band and angle scans.
//!
//! # Model status
//!
//! Implemented and fixture-tested: ideal periodic stacks with optional
//! capping layers, s/p/unpolarized reflectance and complex amplitude at any
//! wavelength (0.0413-41.3 nm) and angle, Nevot-Croce interface roughness,
//! peak / FWHM bandwidth / angular acceptance search, period tuning.
//! Idealizations: abrupt, laterally uniform layers at bulk (CXRO default)
//! densities unless you pass others; no interdiffusion layers (e.g. MoSi2
//! at Mo/Si interfaces - add them as explicit layers), no thickness errors or
//! drifts, no oxidation of the capping layer, the same roughness on every
//! interface, scalar plane waves per polarization. Real Mo/Si mirrors reach
//! ~67-70% versus the ~73-74% ideal computed here.

use serde::{Deserialize, Serialize};

use crate::error::{LithographyError, Result};
use crate::materials::henke::Material;
use crate::thinfilm::{normal_q, FilmLayer, FilmStack};
use crate::types::{Complex64, Polarization};

/// Where a layer's complex refractive index comes from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LayerIndex {
    /// Henke/CXRO optical constants of a formula at a density, evaluated at
    /// every wavelength.
    Henke(Material),
    /// A fixed complex index `n + ik` (wavelength independent).
    Fixed {
        /// Real part.
        n: f64,
        /// Extinction coefficient (>= 0 for loss).
        k: f64,
    },
}

impl LayerIndex {
    /// Complex index `n + ik` at `wavelength_nm`.
    pub fn index(&self, wavelength_nm: f64) -> Result<Complex64> {
        match self {
            LayerIndex::Henke(m) => m.refractive_index(wavelength_nm),
            LayerIndex::Fixed { n, k } => Ok(Complex64::new(*n, *k)),
        }
    }

    /// Human-readable label (formula or "fixed").
    pub fn label(&self) -> String {
        match self {
            LayerIndex::Henke(m) => m.formula.clone(),
            LayerIndex::Fixed { n, k } => format!("n={n:.5}{k:+.5}i"),
        }
    }
}

/// One layer of a multilayer stack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MlLayer {
    /// Optical constants of the layer.
    pub index: LayerIndex,
    /// Thickness in nm.
    pub thickness_nm: f64,
}

impl MlLayer {
    /// A Henke-material layer of the given formula, density and thickness.
    pub fn henke(formula: &str, density_g_cm3: f64, thickness_nm: f64) -> Result<Self> {
        Ok(Self {
            index: LayerIndex::Henke(Material::new(formula, density_g_cm3)?),
            thickness_nm,
        })
    }
}

/// Complex response of a mirror at one wavelength and incidence angle - what
/// a reflective objective samples at each ray to build its pupil apodization
/// and phase.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PupilResponse {
    /// s (TE) amplitude reflection coefficient.
    pub rs: Complex64,
    /// p (TM) amplitude reflection coefficient (tangential-E convention).
    pub rp: Complex64,
    /// Unpolarized reflectance `(|rs|^2 + |rp|^2) / 2`.
    pub reflectance_unpolarized: f64,
    /// `arg(rs)` in rad.
    pub phase_s_rad: f64,
    /// `arg(rp)` in rad.
    pub phase_p_rad: f64,
}

/// A periodic multilayer mirror: optional capping layers on top of
/// `periods` repetitions of a period, on a substrate, in vacuum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultilayerMirror {
    /// Capping layers, outermost first (light meets `capping[0]` first).
    pub capping: Vec<MlLayer>,
    /// The layers of one period, top first (e.g. `[Si, Mo]`).
    pub period: Vec<MlLayer>,
    /// Number of periods.
    pub periods: usize,
    /// Substrate optical constants.
    pub substrate: LayerIndex,
    /// RMS interface roughness / interdiffusion width in nm, applied to every
    /// interface through the Nevot-Croce factor.
    pub roughness_nm: f64,
}

impl MultilayerMirror {
    /// A periodic stack of `periods` x `[top, bottom]` on `substrate`, no
    /// capping, no roughness. Errors on negative thicknesses.
    pub fn periodic(
        top: MlLayer,
        bottom: MlLayer,
        periods: usize,
        substrate: LayerIndex,
    ) -> Result<Self> {
        for l in [&top, &bottom] {
            if l.thickness_nm.is_nan() || l.thickness_nm < 0.0 {
                return Err(LithographyError::InvalidParameter {
                    name: "thickness_nm",
                    value: l.thickness_nm,
                    reason: "layer thickness must be non-negative",
                });
            }
        }
        Ok(Self {
            capping: Vec::new(),
            period: vec![top, bottom],
            periods,
            substrate,
            roughness_nm: 0.0,
        })
    }

    /// Mo/Si mirror for 13.5 nm: `periods` x (Si on top of Mo), period
    /// `period_nm`, Mo fraction `gamma_mo`, on SiO2 (2.2 g/cm^3); CXRO bulk
    /// densities (Si 2.33, Mo 10.22). The ideal textbook design is
    /// `mo_si(40, 6.9, 0.4)` (peak ~73% at 13.48 nm, normal incidence).
    pub fn mo_si(periods: usize, period_nm: f64, gamma_mo: f64) -> Result<Self> {
        Self::check_bilayer(period_nm, gamma_mo)?;
        Self::periodic(
            MlLayer::henke("Si", 2.33, period_nm * (1.0 - gamma_mo))?,
            MlLayer::henke("Mo", 10.22, period_nm * gamma_mo)?,
            periods,
            LayerIndex::Henke(Material::new("SiO2", 2.2)?),
        )
    }

    /// La/B4C mirror for 6.x nm: `periods` x (B4C on top of La), period
    /// `period_nm`, La fraction `gamma_la`, on Si (2.33 g/cm^3); CXRO bulk
    /// densities (B4C 2.52, La 6.166). The period for a 6.7 nm peak at
    /// normal incidence is ~3.37 nm ([`tune_period_nm`]).
    pub fn la_b4c(periods: usize, period_nm: f64, gamma_la: f64) -> Result<Self> {
        Self::check_bilayer(period_nm, gamma_la)?;
        Self::periodic(
            MlLayer::henke("B4C", 2.52, period_nm * (1.0 - gamma_la))?,
            MlLayer::henke("La", 6.166, period_nm * gamma_la)?,
            periods,
            LayerIndex::Henke(Material::new("Si", 2.33)?),
        )
    }

    /// La/B mirror for 6.6-6.7 nm (the pair behind the reflectance records
    /// at 6.x nm): `periods` x (B on top of La), period `period_nm`, La
    /// fraction `gamma_la`, on Si (2.33 g/cm^3); CXRO bulk densities (B 2.34,
    /// La 6.166). Pure boron absorbs less than B4C just below its K edge
    /// (188 eV = 6.595 nm), so the ideal peak is higher and sits just on the
    /// long-wavelength side of the edge.
    pub fn la_b(periods: usize, period_nm: f64, gamma_la: f64) -> Result<Self> {
        Self::check_bilayer(period_nm, gamma_la)?;
        Self::periodic(
            MlLayer::henke("B", 2.34, period_nm * (1.0 - gamma_la))?,
            MlLayer::henke("La", 6.166, period_nm * gamma_la)?,
            periods,
            LayerIndex::Henke(Material::new("Si", 2.33)?),
        )
    }

    fn check_bilayer(period_nm: f64, gamma: f64) -> Result<()> {
        if period_nm.is_nan() || period_nm <= 0.0 {
            return Err(LithographyError::InvalidParameter {
                name: "period_nm",
                value: period_nm,
                reason: "must be positive",
            });
        }
        if !(0.0..=1.0).contains(&gamma) {
            return Err(LithographyError::InvalidParameter {
                name: "gamma",
                value: gamma,
                reason: "bottom-layer fraction of the period must be in [0, 1]",
            });
        }
        Ok(())
    }

    /// Add a capping layer on top (the most recently added layer is the
    /// outermost), e.g. 2 nm Ru on Mo/Si.
    pub fn with_capping(mut self, material: Material, thickness_nm: f64) -> Self {
        self.capping.insert(
            0,
            MlLayer {
                index: LayerIndex::Henke(material),
                thickness_nm,
            },
        );
        self
    }

    /// Set the RMS interface roughness (nm) used on every interface.
    pub fn with_roughness(mut self, sigma_nm: f64) -> Self {
        self.roughness_nm = sigma_nm;
        self
    }

    /// Period thickness in nm.
    pub fn period_nm(&self) -> f64 {
        self.period.iter().map(|l| l.thickness_nm).sum()
    }

    /// All layers from the top down (capping, then the periods).
    fn layers(&self) -> impl Iterator<Item = &MlLayer> {
        self.capping
            .iter()
            .chain((0..self.periods).flat_map(move |_| self.period.iter()))
    }

    /// Complex amplitude reflection coefficient for s (TE) or p (TM) light at
    /// `wavelength_nm` and `angle_rad` from the surface normal (phase
    /// reference: the top surface). Errors for `Unpolarized`, invalid
    /// angles, or wavelengths outside the Henke range.
    pub fn amplitude(
        &self,
        wavelength_nm: f64,
        angle_rad: f64,
        pol: Polarization,
    ) -> Result<Complex64> {
        if matches!(pol, Polarization::Unpolarized) {
            return Err(LithographyError::InvalidParameter {
                name: "pol",
                value: f64::NAN,
                reason: "amplitudes need a definite polarization (TE or TM)",
            });
        }
        if !(0.0..std::f64::consts::FRAC_PI_2).contains(&angle_rad) {
            return Err(LithographyError::InvalidParameter {
                name: "angle_rad",
                value: angle_rad,
                reason: "incidence angle from the normal must be in [0, pi/2)",
            });
        }
        let k0 = 2.0 * std::f64::consts::PI / wavelength_nm;
        let s = Complex64::new(angle_rad.sin(), 0.0);
        let one = Complex64::new(1.0, 0.0);
        let eta = |n: Complex64, q: Complex64| match pol {
            Polarization::TM => n * (n / q),
            _ => q,
        };
        // Media top-down: ambient, layers, substrate (index, q, thickness).
        let mut media: Vec<(Complex64, Complex64, f64)> = Vec::new();
        media.push((one, normal_q(one, s), 0.0));
        for l in self.layers() {
            let n = l.index.index(wavelength_nm)?;
            media.push((n, normal_q(n, s), l.thickness_nm));
        }
        let n_sub = self.substrate.index(wavelength_nm)?;
        media.push((n_sub, normal_q(n_sub, s), 0.0));

        let sigma2 = self.roughness_nm * self.roughness_nm;
        let interface = |a: &(Complex64, Complex64, f64), b: &(Complex64, Complex64, f64)| {
            let (ea, eb) = (eta(a.0, a.1), eta(b.0, b.1));
            let r = (ea - eb) / (ea + eb);
            if sigma2 > 0.0 {
                r * (-2.0 * k0 * k0 * a.1 * b.1 * sigma2).exp()
            } else {
                r
            }
        };
        let last = media.len() - 1;
        let mut big_r = interface(&media[last - 1], &media[last]);
        for j in (1..last).rev() {
            let (_, q, d) = media[j];
            let phase = (Complex64::new(0.0, 2.0 * k0 * d) * q).exp();
            let r = interface(&media[j - 1], &media[j]);
            let rp = big_r * phase;
            big_r = (r + rp) / (one + r * rp);
        }
        Ok(big_r)
    }

    /// Reflectance at `wavelength_nm`, `angle_rad` from the normal; the mean
    /// of s and p for `Unpolarized`.
    pub fn reflectance(
        &self,
        wavelength_nm: f64,
        angle_rad: f64,
        pol: Polarization,
    ) -> Result<f64> {
        match pol {
            Polarization::Unpolarized => Ok(0.5
                * (self.reflectance(wavelength_nm, angle_rad, Polarization::TE)?
                    + self.reflectance(wavelength_nm, angle_rad, Polarization::TM)?)),
            _ => Ok(self.amplitude(wavelength_nm, angle_rad, pol)?.norm_sqr()),
        }
    }

    /// s and p amplitudes and the unpolarized reflectance at one wavelength
    /// and incidence angle (see [`PupilResponse`]).
    pub fn pupil_response(&self, wavelength_nm: f64, angle_rad: f64) -> Result<PupilResponse> {
        let rs = self.amplitude(wavelength_nm, angle_rad, Polarization::TE)?;
        let rp = self.amplitude(wavelength_nm, angle_rad, Polarization::TM)?;
        Ok(PupilResponse {
            rs,
            rp,
            reflectance_unpolarized: 0.5 * (rs.norm_sqr() + rp.norm_sqr()),
            phase_s_rad: rs.arg(),
            phase_p_rad: rp.arg(),
        })
    }

    /// The equivalent [`FilmStack`] at one wavelength (roughness ignored),
    /// for cross-checks with the characteristic-matrix solver.
    pub fn to_film_stack(&self, wavelength_nm: f64) -> Result<FilmStack> {
        let layers = self
            .layers()
            .map(|l| {
                Ok(FilmLayer {
                    name: l.index.label(),
                    thickness_nm: l.thickness_nm,
                    n: l.index.index(wavelength_nm)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(FilmStack::new_vuv(
            layers,
            self.substrate.index(wavelength_nm)?,
        ))
    }

    /// Wavelength of maximum reflectance in `[lambda_min, lambda_max]` and
    /// that maximum, by a 400-point scan refined with golden-section search.
    pub fn peak(
        &self,
        angle_rad: f64,
        pol: Polarization,
        lambda_min_nm: f64,
        lambda_max_nm: f64,
    ) -> Result<(f64, f64)> {
        let f = |l: f64| self.reflectance(l, angle_rad, pol);
        maximize(f, lambda_min_nm, lambda_max_nm, 400)
    }

    /// Full width at half maximum (nm) of the reflectance peak found in
    /// `[lambda_min, lambda_max]` (crossings located by bisection; errors if
    /// the half-maximum is not reached inside the window).
    pub fn bandwidth_fwhm_nm(
        &self,
        angle_rad: f64,
        pol: Polarization,
        lambda_min_nm: f64,
        lambda_max_nm: f64,
    ) -> Result<f64> {
        let f = |l: f64| self.reflectance(l, angle_rad, pol);
        let (l_peak, r_peak) = maximize(f, lambda_min_nm, lambda_max_nm, 400)?;
        let step = (lambda_max_nm - lambda_min_nm) / 400.0;
        let lo = half_crossing(f, l_peak, -step, lambda_min_nm, 0.5 * r_peak)?;
        let hi = half_crossing(f, l_peak, step, lambda_max_nm, 0.5 * r_peak)?;
        Ok(hi - lo)
    }

    /// Angular acceptance: full width at half maximum (degrees) of the
    /// reflectance versus incidence angle at fixed `wavelength_nm`, around
    /// its maximum in 0-60 deg. When the maximum is at normal incidence the
    /// curve is symmetric in +/- theta and the full width is twice the
    /// half-maximum angle.
    pub fn angular_acceptance_fwhm_deg(
        &self,
        wavelength_nm: f64,
        pol: Polarization,
    ) -> Result<f64> {
        let f = |deg: f64| self.reflectance(wavelength_nm, deg.to_radians(), pol);
        let (a_peak, r_peak) = maximize(f, 0.0, 60.0, 600)?;
        let step = 0.1;
        let hi = half_crossing(f, a_peak, step, 60.0, 0.5 * r_peak)?;
        let lo = if a_peak <= step {
            -hi
        } else {
            match half_crossing(f, a_peak, -step, 0.0, 0.5 * r_peak) {
                Ok(v) => v,
                // Still above half maximum at normal incidence: continue
                // symmetrically through theta = 0.
                Err(_) => {
                    let x = half_crossing(|d| f(d.abs()), 0.0, -step, -60.0, 0.5 * r_peak)?;
                    x.min(-hi)
                }
            }
        };
        Ok(hi - lo)
    }
}

/// Refraction-corrected Bragg period (nm) for order `order` at
/// `wavelength_nm`, incidence `angle_rad` from the normal and period-averaged
/// index decrement `mean_delta`: `d = m lambda / (2 sqrt(cos^2 theta - 2
/// delta_bar))`.
pub fn bragg_period_nm(wavelength_nm: f64, angle_rad: f64, mean_delta: f64, order: usize) -> f64 {
    let c2 = angle_rad.cos().powi(2);
    order as f64 * wavelength_nm / (2.0 * (c2 - 2.0 * mean_delta).max(1e-12).sqrt())
}

/// Tune the period of a mirror family so its reflectance peak (at
/// `angle_rad`, `pol`) sits at `target_nm`: fixed-point iteration `d <- d
/// target / lambda_peak` (the peak scales ~linearly with the period) from
/// `initial_period_nm`, searching the peak within +/-10% of the target.
/// `build(period_nm)` constructs the mirror (e.g. `|d|
/// MultilayerMirror::la_b4c(200, d, 0.4)`).
pub fn tune_period_nm<F>(
    build: F,
    target_nm: f64,
    angle_rad: f64,
    pol: Polarization,
    initial_period_nm: f64,
) -> Result<f64>
where
    F: Fn(f64) -> Result<MultilayerMirror>,
{
    let mut d = initial_period_nm;
    for _ in 0..8 {
        let m = build(d)?;
        let (l_peak, _) = m.peak(angle_rad, pol, 0.9 * target_nm, 1.1 * target_nm)?;
        let next = d * target_nm / l_peak;
        if (next - d).abs() < 1e-7 * d {
            return Ok(next);
        }
        d = next;
    }
    Ok(d)
}

/// Maximize `f` on `[a, b]`: `n`-point scan, then golden-section refinement
/// around the best sample. Returns `(argmax, max)`.
fn maximize<F: Fn(f64) -> Result<f64>>(f: F, a: f64, b: f64, n: usize) -> Result<(f64, f64)> {
    if a.is_nan() || b.is_nan() || b <= a {
        return Err(LithographyError::InvalidParameter {
            name: "range",
            value: b - a,
            reason: "search range must satisfy max > min",
        });
    }
    let step = (b - a) / n as f64;
    let mut best = (a, f(a)?);
    for i in 1..=n {
        let x = a + i as f64 * step;
        let v = f(x)?;
        if v > best.1 {
            best = (x, v);
        }
    }
    let (mut lo, mut hi) = ((best.0 - step).max(a), (best.0 + step).min(b));
    let g = 0.5 * (5f64.sqrt() - 1.0);
    let (mut x1, mut x2) = (hi - g * (hi - lo), lo + g * (hi - lo));
    let (mut f1, mut f2) = (f(x1)?, f(x2)?);
    for _ in 0..60 {
        if f1 > f2 {
            hi = x2;
            x2 = x1;
            f2 = f1;
            x1 = hi - g * (hi - lo);
            f1 = f(x1)?;
        } else {
            lo = x1;
            x1 = x2;
            f1 = f2;
            x2 = lo + g * (hi - lo);
            f2 = f(x2)?;
        }
    }
    let x = 0.5 * (lo + hi);
    let v = f(x)?;
    Ok(if v >= best.1 { (x, v) } else { best })
}

/// Walk from `start` in steps of `step` until `f` drops below `level` (not
/// passing `limit`), then bisect the crossing.
fn half_crossing<F: Fn(f64) -> Result<f64>>(
    f: F,
    start: f64,
    step: f64,
    limit: f64,
    level: f64,
) -> Result<f64> {
    let mut x0 = start;
    loop {
        let x1 = x0 + step;
        if (step > 0.0 && x1 > limit) || (step < 0.0 && x1 < limit) {
            return Err(LithographyError::InvalidParameter {
                name: "range",
                value: limit,
                reason: "half maximum not reached inside the search range",
            });
        }
        if f(x1)? < level {
            let (mut a, mut b) = (x0, x1);
            for _ in 0..60 {
                let m = 0.5 * (a + b);
                if f(m)? >= level {
                    a = m;
                } else {
                    b = m;
                }
            }
            return Ok(0.5 * (a + b));
        }
        x0 = x1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn mo_si_40() -> MultilayerMirror {
        MultilayerMirror::mo_si(40, 6.9, 0.4).unwrap()
    }

    #[test]
    fn test_parratt_matches_characteristic_matrix() {
        // Two independent formalisms (Parratt recursion here, Macleod
        // characteristic matrix in thinfilm) must agree without roughness.
        let m = mo_si_40().with_capping(Material::new("Ru", 12.41).unwrap(), 2.0);
        for (lambda, deg) in [(13.5, 0.0), (13.2, 10.0), (12.9, 25.0)] {
            let stack = m.to_film_stack(lambda).unwrap();
            for pol in [Polarization::TE, Polarization::TM] {
                let a = m.amplitude(lambda, f64::to_radians(deg), pol).unwrap();
                let (b, _) = stack
                    .amplitude_coefficients(lambda, f64::to_radians(deg), pol)
                    .unwrap();
                assert!(
                    (a - b).norm() < 1e-10,
                    "{lambda} nm {deg} deg {pol:?}: {a} vs {b}"
                );
            }
        }
    }

    #[test]
    fn test_bare_substrate_is_fresnel() {
        let m = MultilayerMirror::mo_si(0, 6.9, 0.4).unwrap();
        let n = Material::new("SiO2", 2.2)
            .unwrap()
            .refractive_index(13.5)
            .unwrap();
        let one = Complex64::new(1.0, 0.0);
        let r = m.amplitude(13.5, 0.0, Polarization::TE).unwrap();
        assert!((r - (one - n) / (one + n)).norm() < 1e-14);
    }

    #[test]
    fn test_mo_si_matches_cxro_wavelength_scan() {
        // CXRO multilayer calculator, Si/Mo d = 6.9 nm, Gamma = 0.4, N = 40,
        // SiO2 substrate, normal incidence, unpolarized, sigma = 0.
        let m = mo_si_40();
        for (lambda, r_cxro) in [
            (12.5, 1.910444e-2),
            (12.9, 0.119366),
            (13.1, 0.345087),
            (13.3, 0.679410),
            (13.5, 0.729309),
            (13.7, 0.513626),
            (13.9, 0.157729),
            (14.3, 4.790088e-2),
        ] {
            let r = m
                .reflectance(lambda, 0.0, Polarization::Unpolarized)
                .unwrap();
            assert!(
                (r - r_cxro).abs() < 3e-3,
                "{lambda} nm: {r} vs CXRO {r_cxro}"
            );
        }
        let (l_peak, r_peak) = m.peak(0.0, Polarization::TE, 13.0, 14.0).unwrap();
        assert!(
            (l_peak - 13.48).abs() < 0.01 && (r_peak - 0.730034).abs() < 3e-3,
            "{l_peak} {r_peak}"
        );
    }

    #[test]
    fn test_mo_si_roughness_matches_cxro() {
        // Same stack with 0.5 nm Nevot-Croce roughness: CXRO peak 0.698780 at
        // 13.46 nm; R(13.1) = 0.253518, R(13.7) = 0.379275.
        let m = mo_si_40().with_roughness(0.5);
        for (lambda, r_cxro) in [(13.1, 0.253518), (13.5, 0.695204), (13.7, 0.379275)] {
            let r = m
                .reflectance(lambda, 0.0, Polarization::Unpolarized)
                .unwrap();
            assert!(
                (r - r_cxro).abs() < 3e-3,
                "{lambda} nm: {r} vs CXRO {r_cxro}"
            );
        }
        let smooth = mo_si_40().reflectance(13.5, 0.0, Polarization::TE).unwrap();
        let rough = m.reflectance(13.5, 0.0, Polarization::TE).unwrap();
        assert!(rough < smooth);
    }

    #[test]
    fn test_mo_si_matches_cxro_angle_scans() {
        // CXRO at 91.84 eV (13.5 nm), grazing angle -> incidence from normal.
        let m = mo_si_40();
        let lambda = 1239.84193 / 91.84014;
        for (grazing, rs, rp) in [
            (65.0, 1.375045e-2, 7.705517e-3),
            (75.0, 0.158190, 0.102140),
            (80.0, 0.531494, 0.440423),
            (85.0, 0.721491, 0.714822),
            (90.0, 0.729308, 0.729307),
        ] {
            let th = f64::to_radians(90.0 - grazing);
            let s = m.reflectance(lambda, th, Polarization::TE).unwrap();
            let p = m.reflectance(lambda, th, Polarization::TM).unwrap();
            assert!((s - rs).abs() < 3e-3, "s at {grazing}: {s} vs {rs}");
            assert!((p - rp).abs() < 3e-3, "p at {grazing}: {p} vs {rp}");
        }
    }

    #[test]
    fn test_s_phase_is_conjugate_of_cxro() {
        // CXRO's phase column (the s-amplitude phase) at normal incidence:
        // -176.654 deg at 13.5 nm, 139.335 at 13.3, -120.184 at 13.7. CXRO
        // uses the exp(-i(kz - wt)) sign convention, so its amplitude is the
        // complex conjugate of ours: arg r = -phase_CXRO.
        let m = mo_si_40();
        for (lambda, deg) in [(13.5, -176.654), (13.3, 139.335), (13.7, -120.184)] {
            let r = m.amplitude(lambda, 0.0, Polarization::TE).unwrap();
            let diff = (r.arg().to_degrees() + deg + 540.0).rem_euclid(360.0) - 180.0;
            assert!(
                diff.abs() < 0.1,
                "{lambda} nm: {} vs -({deg})",
                r.arg().to_degrees()
            );
        }
    }

    #[test]
    fn test_la_b4c_matches_cxro() {
        // CXRO B4C/La d = 3.4 nm, Gamma = 0.4, N = 200 on Si: peak 0.666246
        // at 6.754 nm; R(6.72) = 0.410876, R(6.80) = 0.116575.
        let m = MultilayerMirror::la_b4c(200, 3.4, 0.4).unwrap();
        for (lambda, r_cxro) in [(6.72, 0.410876), (6.76, 0.656690), (6.80, 0.116575)] {
            let r = m
                .reflectance(lambda, 0.0, Polarization::Unpolarized)
                .unwrap();
            assert!(
                (r - r_cxro).abs() < 3e-3,
                "{lambda} nm: {r} vs CXRO {r_cxro}"
            );
        }
        let (l_peak, r_peak) = m.peak(0.0, Polarization::TE, 6.6, 6.9).unwrap();
        assert!(
            (l_peak - 6.754).abs() < 0.003 && (r_peak - 0.666246).abs() < 3e-3,
            "{l_peak} {r_peak}"
        );
    }

    #[test]
    fn test_peak_follows_refraction_corrected_bragg_law() {
        // The first-order formula lambda = 2 d sqrt(cos^2 theta - 2 delta_bar)
        // (thickness-weighted delta evaluated at the peak wavelength) places
        // the Mo/Si peak to within ~2%; the exact peak sits slightly longward
        // because of the reflection phases of the absorbing interfaces. The
        // CXRO angle scans above are the quantitative check.
        let m = mo_si_40();
        let mean_delta = |lambda: f64| {
            let d =
                |f: &str, rho: f64| Material::new(f, rho).unwrap().delta_beta(lambda).unwrap().0;
            0.6 * d("Si", 2.33) + 0.4 * d("Mo", 10.22)
        };
        let mut previous = f64::INFINITY;
        for deg in [0.0, 10.0, 20.0] {
            let th = f64::to_radians(deg);
            let (l_peak, _) = m.peak(th, Polarization::TE, 11.0, 14.5).unwrap();
            let db = mean_delta(l_peak);
            let l_bragg = 2.0 * 6.9 * (th.cos().powi(2) - 2.0 * db).sqrt();
            assert!(
                l_peak > l_bragg && l_peak / l_bragg - 1.0 < 0.02,
                "{deg} deg: {l_peak} vs {l_bragg}"
            );
            assert!(
                l_peak < previous,
                "peak moves to shorter wavelength with angle"
            );
            previous = l_peak;
            assert_relative_eq!(
                bragg_period_nm(l_bragg, th, db, 1),
                6.9,
                max_relative = 1e-12
            );
        }
    }

    #[test]
    fn test_reflectance_saturates_with_periods() {
        let r = |n: usize| {
            MultilayerMirror::mo_si(n, 6.9, 0.4)
                .unwrap()
                .peak(0.0, Polarization::TE, 13.0, 14.0)
                .unwrap()
                .1
        };
        let (r10, r40, r80, r200) = (r(10), r(40), r(80), r(200));
        assert!(r10 < r40 && r40 < r80);
        assert!((r200 - r80).abs() < 5e-3, "{r80} vs {r200}");
        assert!((0.70..0.76).contains(&r200), "saturated Mo/Si {r200}");
    }

    #[test]
    fn test_polarization_and_angular_behaviour() {
        let m = mo_si_40();
        // s = p at normal incidence.
        let s0 = m.reflectance(13.5, 0.0, Polarization::TE).unwrap();
        let p0 = m.reflectance(13.5, 0.0, Polarization::TM).unwrap();
        assert!((s0 - p0).abs() < 1e-12);
        // Retuned to 20 deg incidence (peak moves to shorter lambda), p is
        // weaker than s (Brewster angle ~45 deg at EUV).
        let th = f64::to_radians(20.0);
        let (ls, rs) = m.peak(th, Polarization::TE, 12.0, 13.5).unwrap();
        let rp = m.reflectance(ls, th, Polarization::TM).unwrap();
        assert!(rp < rs, "p {rp} vs s {rs}");
        let resp = m.pupil_response(13.5, f64::to_radians(5.0)).unwrap();
        assert_relative_eq!(
            resp.reflectance_unpolarized,
            m.reflectance(13.5, f64::to_radians(5.0), Polarization::Unpolarized)
                .unwrap(),
            max_relative = 1e-12
        );
        assert_relative_eq!(resp.phase_s_rad, resp.rs.arg());
    }

    #[test]
    fn test_bandwidth_and_angular_acceptance_are_sensible() {
        let m = mo_si_40();
        let bw = m
            .bandwidth_fwhm_nm(0.0, Polarization::TE, 12.8, 14.2)
            .unwrap();
        assert!((0.45..0.7).contains(&bw), "Mo/Si FWHM {bw} nm");
        // At 13.5 nm (just longward of the 13.48 nm normal-incidence peak) the
        // maximum is at normal incidence; the symmetric full width is ~20 deg.
        let acc = m
            .angular_acceptance_fwhm_deg(13.5, Polarization::TE)
            .unwrap();
        assert!((10.0..40.0).contains(&acc), "acceptance {acc} deg");
        // La/B4C: far narrower band (weaker contrast, more periods).
        let lb = MultilayerMirror::la_b4c(200, 3.4, 0.4).unwrap();
        let bw_lb = lb
            .bandwidth_fwhm_nm(0.0, Polarization::TE, 6.6, 6.9)
            .unwrap();
        assert!(
            bw_lb < 0.1 && bw_lb / 6.75 < bw / 13.48,
            "La/B4C FWHM {bw_lb} nm"
        );
    }

    #[test]
    fn test_ru_capping_and_period_tuning() {
        let m = mo_si_40().with_capping(Material::new("Ru", 12.41).unwrap(), 2.0);
        assert_eq!(m.capping.len(), 1);
        let (_, r) = m.peak(0.0, Polarization::TE, 13.0, 14.0).unwrap();
        assert!((0.6..0.76).contains(&r), "Ru-capped peak {r}");
        // Tune La/B4C so the normal-incidence peak is at 6.7 nm.
        let d = tune_period_nm(
            |d| MultilayerMirror::la_b4c(200, d, 0.4),
            6.7,
            0.0,
            Polarization::TE,
            3.4,
        )
        .unwrap();
        let (l_peak, _) = MultilayerMirror::la_b4c(200, d, 0.4)
            .unwrap()
            .peak(0.0, Polarization::TE, 6.6, 6.8)
            .unwrap();
        assert!(
            (l_peak - 6.7).abs() < 1e-3 && (3.3..3.45).contains(&d),
            "d {d} peak {l_peak}"
        );
    }

    #[test]
    fn test_la_b_boron_edge_and_ideal_peak() {
        // The boron K edge (188 eV = 6.595 nm) sits just shortward of the
        // La/B working wavelength: above the edge in energy (6.5 nm) boron
        // absorbs ~25x more, and a mirror tuned for 6.65 nm is dark there.
        let b = Material::new("B", 2.34).unwrap();
        let beta = |l: f64| b.delta_beta(l).unwrap().1;
        assert!(
            beta(6.5) > 20.0 * beta(6.7),
            "{} vs {}",
            beta(6.5),
            beta(6.7)
        );
        let d = tune_period_nm(
            |d| MultilayerMirror::la_b(200, d, 0.4),
            6.65,
            0.0,
            Polarization::TE,
            3.33,
        )
        .unwrap();
        let m = MultilayerMirror::la_b(200, d, 0.4).unwrap();
        let (l, r) = m.peak(0.0, Polarization::TE, 6.55, 6.75).unwrap();
        // Ideal sharp interfaces: ~80% (measured records are ~64%).
        assert!(
            (l - 6.65).abs() < 1e-3 && (0.75..0.85).contains(&r),
            "{l} {r}"
        );
        let bw = m
            .bandwidth_fwhm_nm(0.0, Polarization::TE, 6.55, 6.75)
            .unwrap();
        assert!((0.05..0.08).contains(&bw), "FWHM {bw}");
        assert!(m.reflectance(6.5, 0.0, Polarization::TE).unwrap() < 0.05);
        // Pure B beats B4C (no carbon absorption) for the same design.
        let lb4c = MultilayerMirror::la_b4c(200, 3.37, 0.4).unwrap();
        let (_, r_b4c) = lb4c.peak(0.0, Polarization::TE, 6.6, 6.8).unwrap();
        assert!(r > r_b4c);
    }

    #[test]
    fn test_invalid_inputs() {
        assert!(MultilayerMirror::mo_si(40, -1.0, 0.4).is_err());
        assert!(MultilayerMirror::mo_si(40, 6.9, 1.5).is_err());
        let m = mo_si_40();
        assert!(m.amplitude(13.5, 0.0, Polarization::Unpolarized).is_err());
        assert!(m.amplitude(13.5, 2.0, Polarization::TE).is_err());
        assert!(m.reflectance(100.0, 0.0, Polarization::TE).is_err());
        assert!(m.peak(0.0, Polarization::TE, 14.0, 13.0).is_err());
    }
}
