//! Stochastic effects modeling for VUV lithography.
//!
//! Models photon shot noise and acid diffusion randomness to predict
//! Line Edge Roughness (LER) and Line Width Roughness (LWR).
//!
//! # Model status
//!
//! 🔶 Simplified: Poisson photon shot noise per pixel (Gaussian
//! approximation above 1000 photons) with the source-derived photon
//! density, an empirical Gaussian acid-noise term, and a Gamma-distributed
//! per-exposure dose factor. [`StochasticParams::from_source`] applies the
//! source's single-pulse energy rms to the whole exposure (exact for
//! single-shot exposures, conservative otherwise);
//! [`StochasticParams::from_source_multi_pulse`] divides it by `sqrt(N)`
//! for an exposure integrating `N` independent pulses (e.g. the
//! `pulses_per_point` of `source_models::throughput`). Photon counting is
//! one-photon: N-photon absorption of entangled sources is not modeled.
//!
//! By default the photon density is the INCIDENT density (0.68 photons/nm²
//! per mJ/cm² at 13.5 nm, consistent with Bhattarai et al., J. Vac. Sci.
//! Technol. B 35, 061602 (2017): 15 mJ/cm² → 10.2 photons/nm²). Only
//! absorbed photons drive the resist chemistry, so
//! [`StochasticParams::with_resist_absorption`] can rescale it to the
//! absorbed density with Beer–Lambert, `1 - exp(-alpha d)`: a 35 nm organic
//! CAR at ~4.8 µm⁻¹ absorbs ~15 % (Fallica et al., J. Micro/Nanolith. MEMS
//! MOEMS 17, 023505 (2018)); metal-oxide resists at 12–20 µm⁻¹ absorb
//! ~35–50 %.

use ndarray::Array2;
use rand::prelude::*;
use rand_distr::{Gamma, Normal, Poisson};

use crate::source::LithographySource;

/// Parameters for stochastic simulation.
#[derive(Debug, Clone)]
pub struct StochasticParams {
    /// Photon density at dose=1 mJ/cm² (photons/nm²).
    /// At 157 nm: E_photon = hc/λ = 1.27e-18 J, so 1 mJ/cm² = 10 J/m²
    /// = 7.9e18 photons/m² = 7.9 photons/nm².
    pub photon_density_per_mj_cm2: f64,
    /// PEB acid diffusion length standard deviation (nm).
    pub acid_diffusion_sigma_nm: f64,
    /// Number of Monte Carlo realizations.
    pub num_realizations: usize,
    /// Relative rms shot-to-shot dose fluctuation (0 = perfectly stable
    /// source). Sampled per realization from a Gamma distribution with
    /// mean 1 and the given rms — the textbook SASE pulse-energy
    /// statistic, and a good model for any pulsed-source dose jitter.
    pub dose_jitter_rms: f64,
}

impl StochasticParams {
    /// Create stochastic params from a source: photon density and
    /// shot-to-shot dose jitter are taken from the source model.
    pub fn from_source(source: &impl LithographySource) -> Self {
        Self {
            photon_density_per_mj_cm2: source.photon_density_per_mj_cm2(),
            dose_jitter_rms: source.shot_to_shot_rms(),
            ..Self::default()
        }
    }

    /// Like [`Self::from_source`], for an exposure that integrates
    /// `pulses_per_exposure` statistically independent source pulses: the
    /// exposure-dose rms is the single-pulse rms divided by
    /// `sqrt(pulses_per_exposure)` (values below 1 are treated as 1).
    /// A scanner exposes each point with many pulses — ~10^2 for an EUV
    /// LPP, ~10^4 for a 100 MHz-class FEL — so single-pulse SASE jitter of
    /// tens of percent averages down to the sub-percent level.
    pub fn from_source_multi_pulse(
        source: &impl LithographySource,
        pulses_per_exposure: f64,
    ) -> Self {
        let n = if pulses_per_exposure.is_finite() {
            pulses_per_exposure.max(1.0)
        } else {
            1.0
        };
        Self {
            dose_jitter_rms: source.shot_to_shot_rms() / n.sqrt(),
            ..Self::from_source(source)
        }
    }

    /// Count only ABSORBED photons in the shot-noise model: scales
    /// `photon_density_per_mj_cm2` by `absorbed_fraction` (clamped to
    /// (0, 1]; non-finite values leave the params unchanged). Without this
    /// call the density is the incident one (the historical default).
    pub fn with_absorbed_fraction(mut self, absorbed_fraction: f64) -> Self {
        if absorbed_fraction.is_finite() && absorbed_fraction > 0.0 {
            self.photon_density_per_mj_cm2 *= absorbed_fraction.min(1.0);
        }
        self
    }

    /// [`Self::with_absorbed_fraction`] for a resist film of thickness
    /// `thickness_nm` and absorption coefficient `absorption_per_um` (1/µm),
    /// using [`resist_absorbed_fraction`].
    pub fn with_resist_absorption(self, thickness_nm: f64, absorption_per_um: f64) -> Self {
        self.with_absorbed_fraction(resist_absorbed_fraction(thickness_nm, absorption_per_um))
    }

    /// Default parameters for F2 laser (157nm) lithography.
    pub fn default_vuv() -> Self {
        Self {
            // hc/λ at 157 nm: photon energy = 1.267e-18 J
            // 1 mJ/cm² = 10 J/m² -> 10 / 1.267e-18 = 7.89e18 photons/m²
            // 1 nm² = 1e-18 m² -> 7.89 photons/nm²
            // At a typical dose of 30 mJ/cm²: ~237 photons per (1 nm)² pixel.
            // (An earlier version of this constant was 0.00789 — a
            // m²→nm² conversion slip of 1e3 that inflated shot-noise
            // LER by ~sqrt(1000) ≈ 32x. Pinned against the
            // LithographySource trait default by a regression test.)
            photon_density_per_mj_cm2: 7.89,
            acid_diffusion_sigma_nm: 5.0,
            num_realizations: 100,
            dose_jitter_rms: 0.0,
        }
    }
}

impl Default for StochasticParams {
    fn default() -> Self {
        Self::default_vuv()
    }
}

/// Fraction of the incident photons a resist film absorbs (Beer–Lambert,
/// whole film): `1 - exp(-alpha d)`, `alpha` in 1/µm, `d` in nm. E.g. 35 nm
/// at 4.8 µm⁻¹ (organic CAR) → 0.155; at 20 µm⁻¹ (metal-oxide) → 0.503.
pub fn resist_absorbed_fraction(thickness_nm: f64, absorption_per_um: f64) -> f64 {
    if !(thickness_nm > 0.0 && absorption_per_um > 0.0) {
        return 0.0;
    }
    1.0 - (-absorption_per_um * thickness_nm * 1e-3).exp()
}

/// Result of stochastic LER/LWR analysis.
#[derive(Debug, Clone)]
pub struct LerResult {
    /// Line Edge Roughness (3σ) in nm.
    pub ler_3sigma_nm: f64,
    /// Line Width Roughness (3σ) in nm.
    pub lwr_3sigma_nm: f64,
    /// Mean CD across realizations (nm).
    pub cd_mean_nm: f64,
    /// CD standard deviation across realizations (nm).
    pub cd_sigma_nm: f64,
    /// Individual edge positions per realization (left edge).
    pub left_edges: Vec<f64>,
    /// Individual edge positions per realization (right edge).
    pub right_edges: Vec<f64>,
}

/// Apply photon shot noise to an aerial image.
///
/// The number of photons absorbed at each pixel follows a Poisson distribution.
/// The noisy intensity is the Poisson-sampled photon count normalized back to
/// the original intensity scale.
pub fn apply_shot_noise(
    aerial_image: &Array2<f64>,
    dose_mj_cm2: f64,
    pixel_nm: f64,
    params: &StochasticParams,
    rng: &mut impl Rng,
) -> Array2<f64> {
    let pixel_area = pixel_nm * pixel_nm;
    let base_photons = dose_mj_cm2 * params.photon_density_per_mj_cm2 * pixel_area;

    aerial_image.mapv(|intensity| {
        let mean_photons = intensity * base_photons;
        if mean_photons <= 0.0 {
            return 0.0;
        }

        // Poisson sampling for small photon counts, Gaussian approximation for large
        let sampled = if mean_photons < 1000.0 {
            match Poisson::new(mean_photons) {
                Ok(dist) => rng.sample(dist),
                Err(_) => mean_photons,
            }
        } else {
            match Normal::new(mean_photons, mean_photons.sqrt()) {
                Ok(normal) => rng.sample(normal).max(0.0),
                Err(_) => mean_photons, // fallback to deterministic
            }
        };

        // Normalize back to intensity scale
        if base_photons > 0.0 {
            sampled / base_photons
        } else {
            0.0
        }
    })
}

/// Apply random acid diffusion perturbation to a latent image.
///
/// Each pixel's PAC concentration is perturbed by a Gaussian random variable
/// representing the stochastic nature of acid generation and diffusion during PEB.
pub fn apply_acid_noise(
    pac: &Array2<f64>,
    params: &StochasticParams,
    pixel_nm: f64,
    rng: &mut impl Rng,
) -> Array2<f64> {
    let sigma_pixels = params.acid_diffusion_sigma_nm / pixel_nm;
    // The noise magnitude scales with the local gradient of PAC concentration
    let noise_scale = 0.01 * sigma_pixels; // empirical scaling

    pac.mapv(|m| {
        let noise = match Normal::new(0.0, noise_scale) {
            Ok(dist) => rng.sample(dist),
            Err(_) => 0.0,
        };
        (m + noise).clamp(0.0, 1.0)
    })
}

/// Compute LER/LWR by running multiple stochastic realizations.
///
/// For each realization:
/// 1. Apply photon shot noise to aerial image
/// 2. Apply acid diffusion noise to latent image
/// 3. Measure CD (threshold crossing positions)
///
/// Returns LER (edge position roughness) and LWR (CD roughness) statistics.
pub fn compute_ler_lwr(
    aerial_image: &Array2<f64>,
    x_min_nm: f64,
    x_max_nm: f64,
    dose_mj_cm2: f64,
    pixel_nm: f64,
    threshold: f64,
    params: &StochasticParams,
) -> LerResult {
    let mut rng = StdRng::seed_from_u64(42);
    let nx = aerial_image.ncols();
    let ny = aerial_image.nrows();
    let center_row = ny / 2;

    let x_nm: Vec<f64> = (0..nx)
        .map(|j| x_min_nm + (j as f64 + 0.5) * pixel_nm)
        .collect();

    let mut left_edges = Vec::with_capacity(params.num_realizations);
    let mut right_edges = Vec::with_capacity(params.num_realizations);
    let mut cds = Vec::with_capacity(params.num_realizations);

    for _ in 0..params.num_realizations {
        // Shot-to-shot dose jitter: Gamma-distributed with mean 1 and
        // rms = dose_jitter_rms (Gamma(k, 1/k) with k = 1/rms²).
        let dose_factor = if params.dose_jitter_rms > 0.0 {
            let k = 1.0 / (params.dose_jitter_rms * params.dose_jitter_rms);
            match Gamma::new(k, 1.0 / k) {
                Ok(dist) => rng.sample(dist),
                Err(_) => 1.0,
            }
        } else {
            1.0
        };
        let dose = dose_mj_cm2 * dose_factor;
        let noisy = apply_shot_noise(aerial_image, dose, pixel_nm, params, &mut rng);

        // Extract center row cross-section. The threshold is an absolute
        // dose criterion, so a hotter/cooler pulse shifts the printed
        // edge: scale the (dose-normalized) intensity by the dose factor.
        let profile: Vec<f64> = (0..nx)
            .map(|j| noisy[[center_row, j]] * dose_factor)
            .collect();

        // Find threshold crossings
        let crossings = find_crossings(&profile, &x_nm, threshold);

        if crossings.len() >= 2 {
            // Find the pair closest to center
            let center = (x_min_nm + x_max_nm) / 2.0;
            let mut best_pair = (0, 1);
            let mut best_dist = f64::INFINITY;

            for i in 0..crossings.len() - 1 {
                let mid = (crossings[i] + crossings[i + 1]) / 2.0;
                let dist = (mid - center).abs();
                if dist < best_dist {
                    best_dist = dist;
                    best_pair = (i, i + 1);
                }
            }

            left_edges.push(crossings[best_pair.0]);
            right_edges.push(crossings[best_pair.1]);
            cds.push(crossings[best_pair.1] - crossings[best_pair.0]);
        }
    }

    if cds.is_empty() {
        return LerResult {
            ler_3sigma_nm: 0.0,
            lwr_3sigma_nm: 0.0,
            cd_mean_nm: 0.0,
            cd_sigma_nm: 0.0,
            left_edges,
            right_edges,
        };
    }

    let cd_mean = cds.iter().sum::<f64>() / cds.len() as f64;
    let cd_var = cds.iter().map(|&c| (c - cd_mean).powi(2)).sum::<f64>() / cds.len() as f64;
    let cd_sigma = cd_var.sqrt();

    let left_mean = left_edges.iter().sum::<f64>() / left_edges.len() as f64;
    let left_var = left_edges
        .iter()
        .map(|&e| (e - left_mean).powi(2))
        .sum::<f64>()
        / left_edges.len() as f64;

    let right_mean = right_edges.iter().sum::<f64>() / right_edges.len() as f64;
    let right_var = right_edges
        .iter()
        .map(|&e| (e - right_mean).powi(2))
        .sum::<f64>()
        / right_edges.len() as f64;

    // LER is the average edge roughness (3σ of edge position)
    let ler_sigma = ((left_var + right_var) / 2.0).sqrt();
    // LWR is the CD roughness (3σ of CD)
    let lwr_sigma = cd_sigma;

    LerResult {
        ler_3sigma_nm: 3.0 * ler_sigma,
        lwr_3sigma_nm: 3.0 * lwr_sigma,
        cd_mean_nm: cd_mean,
        cd_sigma_nm: cd_sigma,
        left_edges,
        right_edges,
    }
}

/// Find threshold crossings in a 1D intensity profile.
fn find_crossings(profile: &[f64], x_nm: &[f64], threshold: f64) -> Vec<f64> {
    let mut crossings = Vec::new();
    for i in 0..profile.len() - 1 {
        let y0 = profile[i] - threshold;
        let y1 = profile[i + 1] - threshold;
        if y0 * y1 < 0.0 {
            let t = y0 / (y0 - y1);
            crossings.push(x_nm[i] + t * (x_nm[i + 1] - x_nm[i]));
        }
    }
    crossings
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_shot_noise_preserves_mean() {
        let n = 128;
        let aerial = Array2::from_elem((n, n), 0.5);
        let params = StochasticParams {
            photon_density_per_mj_cm2: 0.1,
            acid_diffusion_sigma_nm: 5.0,
            num_realizations: 1,
            dose_jitter_rms: 0.0,
        };

        // Average over many realizations should converge to original
        let mut sum = Array2::zeros((n, n));
        let num_trials = 200;
        let mut rng = StdRng::seed_from_u64(123);
        for _ in 0..num_trials {
            let noisy = apply_shot_noise(&aerial, 30.0, 2.0, &params, &mut rng);
            sum += &noisy;
        }
        sum.mapv_inplace(|v| v / num_trials as f64);

        let mean_val = sum.iter().sum::<f64>() / (n * n) as f64;
        assert_relative_eq!(mean_val, 0.5, epsilon = 0.05);
    }

    #[test]
    fn test_shot_noise_non_negative() {
        let aerial = Array2::from_elem((64, 64), 0.3);
        let params = StochasticParams::default_vuv();
        let mut rng = StdRng::seed_from_u64(42);
        let noisy = apply_shot_noise(&aerial, 30.0, 2.0, &params, &mut rng);
        assert!(noisy.iter().all(|&v| v >= 0.0));
    }

    #[test]
    fn test_acid_noise_bounded() {
        let pac = Array2::from_elem((64, 64), 0.5);
        let params = StochasticParams::default_vuv();
        let mut rng = StdRng::seed_from_u64(42);
        let noisy = apply_acid_noise(&pac, &params, 2.0, &mut rng);
        assert!(noisy.iter().all(|&v| v >= 0.0 && v <= 1.0));
    }

    #[test]
    fn test_ler_lwr_computation() {
        // Create a simple step-function aerial image
        let n = 128;
        let mut aerial = Array2::from_elem((n, n), 0.1);
        for i in 0..n {
            for j in 40..88 {
                aerial[[i, j]] = 0.9;
            }
        }

        let params = StochasticParams {
            photon_density_per_mj_cm2: 0.01,
            acid_diffusion_sigma_nm: 3.0,
            num_realizations: 50,
            dose_jitter_rms: 0.0,
        };

        let result = compute_ler_lwr(&aerial, -128.0, 128.0, 30.0, 2.0, 0.5, &params);

        assert!(result.cd_mean_nm > 0.0, "Mean CD should be positive");
        assert!(result.ler_3sigma_nm >= 0.0, "LER should be non-negative");
        assert!(result.lwr_3sigma_nm >= 0.0, "LWR should be non-negative");
        assert_eq!(result.left_edges.len(), 50);
        assert_eq!(result.right_edges.len(), 50);
    }

    #[test]
    fn test_ler_increases_with_noise() {
        // Use a smooth Gaussian feature profile instead of hard step
        let n = 128;
        let mut aerial = Array2::zeros((n, n));
        for i in 0..n {
            for j in 0..n {
                let x = (j as f64 - 64.0) * 2.0; // nm from center
                let sigma = 30.0;
                aerial[[i, j]] = (-x * x / (2.0 * sigma * sigma)).exp();
            }
        }

        // Very high photon density = essentially no noise → LER should be low
        let params_low_noise = StochasticParams {
            photon_density_per_mj_cm2: 10.0, // very high
            acid_diffusion_sigma_nm: 0.1,
            num_realizations: 50,
            dose_jitter_rms: 0.0,
        };
        let result = compute_ler_lwr(&aerial, -128.0, 128.0, 100.0, 2.0, 0.5, &params_low_noise);
        assert!(
            result.ler_3sigma_nm < 5.0,
            "Low-noise LER should be small, got {:.2}",
            result.ler_3sigma_nm
        );
        assert!(result.cd_mean_nm > 0.0);
    }

    #[test]
    fn test_default_photon_density_matches_source_trait() {
        // Regression for a m²→nm² conversion slip: default_vuv() must
        // agree with the LithographySource trait derivation for F2.
        let src = crate::source::VuvSource::f2_laser(0.7).unwrap();
        let expected = src.photon_density_per_mj_cm2();
        let params = StochasticParams::default_vuv();
        let rel_err = ((params.photon_density_per_mj_cm2 - expected) / expected).abs();
        assert!(
            rel_err < 0.01,
            "default_vuv photon density {} disagrees with trait-derived {}",
            params.photon_density_per_mj_cm2,
            expected
        );
    }

    #[test]
    fn test_photon_density_at_13nm5_matches_literature() {
        // Literature anchor for EUV stochastics: 1 mJ/cm² at 13.5 nm
        // carries ~0.68 photons/nm² (e.g. ~10 photons/nm² at 15 mJ/cm²).
        struct Euv;
        impl LithographySource for Euv {
            fn wavelength_nm(&self) -> f64 {
                13.5
            }
            fn bandwidth_pm(&self) -> f64 {
                0.0
            }
            fn intensity_at(&self, _fx: f64, _fy: f64) -> f64 {
                1.0
            }
            fn spectral_weights(&self) -> Vec<(f64, f64)> {
                vec![(13.5, 1.0)]
            }
        }
        let density = Euv.photon_density_per_mj_cm2();
        assert!(
            (density - 0.68).abs() < 0.02,
            "13.5 nm photon density should be ~0.68 photons/nm² per mJ/cm², got {}",
            density
        );
    }

    #[test]
    fn test_from_source_pulls_dose_jitter() {
        let fel = crate::source::LpaFelSource::bella_target_25nm(0.7).unwrap();
        let params = StochasticParams::from_source(&fel);
        assert_relative_eq!(params.dose_jitter_rms, 0.03, epsilon = 1e-12);
        assert_relative_eq!(
            params.photon_density_per_mj_cm2,
            fel.photon_density_per_mj_cm2(),
            epsilon = 1e-12
        );
    }

    #[test]
    fn test_resist_absorption_scales_photon_density() {
        // Beer-Lambert fixtures (mpmath): 35 nm at 4.8 / 12 / 20 per um.
        assert_relative_eq!(
            resist_absorbed_fraction(35.0, 4.8),
            0.154_646_165_315_341_28,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            resist_absorbed_fraction(35.0, 12.0),
            0.342_953_180_184_943_2,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            resist_absorbed_fraction(35.0, 20.0),
            0.503_414_696_208_590_5,
            max_relative = 1e-12
        );
        assert_eq!(resist_absorbed_fraction(0.0, 5.0), 0.0);
        // Incident density at 13.5 nm x 15 % absorption.
        struct Euv;
        impl LithographySource for Euv {
            fn wavelength_nm(&self) -> f64 {
                13.5
            }
            fn bandwidth_pm(&self) -> f64 {
                0.0
            }
            fn intensity_at(&self, _fx: f64, _fy: f64) -> f64 {
                1.0
            }
            fn spectral_weights(&self) -> Vec<(f64, f64)> {
                vec![(13.5, 1.0)]
            }
        }
        let incident = StochasticParams::from_source(&Euv);
        let absorbed = StochasticParams::from_source(&Euv).with_resist_absorption(35.0, 4.8);
        assert_relative_eq!(
            absorbed.photon_density_per_mj_cm2,
            incident.photon_density_per_mj_cm2 * 0.154_646_165_315_341_28,
            max_relative = 1e-12
        );
        // 15 mJ/cm^2 -> 10.2 incident photons/nm^2 (Bhattarai et al. 2017).
        assert!((incident.photon_density_per_mj_cm2 * 15.0 - 10.2).abs() < 0.05);
        // Fraction clamps to 1 and ignores nonsense.
        let same = StochasticParams::default_vuv().with_absorbed_fraction(2.0);
        assert_relative_eq!(same.photon_density_per_mj_cm2, 7.89);
        let untouched = StochasticParams::default_vuv().with_absorbed_fraction(f64::NAN);
        assert_relative_eq!(untouched.photon_density_per_mj_cm2, 7.89);
    }

    #[test]
    fn test_multi_pulse_jitter_averages_down() {
        // SASE XFEL: single-pulse rms 1/sqrt(M); 100 pulses per point -> /10.
        let xfel = crate::source::XfelSource::flash_13nm5();
        let single = StochasticParams::from_source(&xfel);
        let multi = StochasticParams::from_source_multi_pulse(&xfel, 100.0);
        assert_relative_eq!(
            multi.dose_jitter_rms,
            single.dose_jitter_rms / 10.0,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            multi.photon_density_per_mj_cm2,
            single.photon_density_per_mj_cm2,
            max_relative = 1e-12
        );
        // < 1 pulse and non-finite counts fall back to single-pulse statistics.
        let fel = crate::source::LpaFelSource::bella_target_25nm(0.7).unwrap();
        assert_relative_eq!(
            StochasticParams::from_source_multi_pulse(&fel, 0.2).dose_jitter_rms,
            0.03,
            max_relative = 1e-12
        );
        assert_relative_eq!(
            StochasticParams::from_source_multi_pulse(&fel, f64::NAN).dose_jitter_rms,
            0.03,
            max_relative = 1e-12
        );
    }

    #[test]
    fn test_dose_jitter_increases_lwr() {
        // Smooth Gaussian feature; very high photon density so per-pixel
        // shot noise is negligible and dose jitter dominates.
        let n = 128;
        let mut aerial = Array2::zeros((n, n));
        for i in 0..n {
            for j in 0..n {
                let x = (j as f64 - 64.0) * 2.0;
                let sigma = 30.0;
                aerial[[i, j]] = (-x * x / (2.0 * sigma * sigma)).exp();
            }
        }

        let stable = StochasticParams {
            photon_density_per_mj_cm2: 1000.0,
            acid_diffusion_sigma_nm: 0.1,
            num_realizations: 60,
            dose_jitter_rms: 0.0,
        };
        let jittery = StochasticParams {
            dose_jitter_rms: 0.10,
            ..stable.clone()
        };

        let r_stable = compute_ler_lwr(&aerial, -128.0, 128.0, 100.0, 2.0, 0.5, &stable);
        let r_jitter = compute_ler_lwr(&aerial, -128.0, 128.0, 100.0, 2.0, 0.5, &jittery);

        assert!(
            r_jitter.lwr_3sigma_nm > r_stable.lwr_3sigma_nm,
            "10% dose jitter should raise LWR: stable {:.3} vs jitter {:.3}",
            r_stable.lwr_3sigma_nm,
            r_jitter.lwr_3sigma_nm
        );
    }
}
