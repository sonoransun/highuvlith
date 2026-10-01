//! Directed Self-Assembly (DSA) of block copolymers (BCP) on lithographic
//! guiding patterns — analytic morphology model.
//!
//! The assembled pattern is rendered in closed form, not computed by a
//! field theory: lamellae as a smoothed square wave of A-block fraction,
//! cylinders and spheres as smoothed disks on a lattice. For lamellae in
//! **graphoepitaxy** (a template whose open regions are trenches between
//! guiding walls) the lamellae are registered to the trench walls: a trench
//! of width `W` holds the integer number of periods `n` that minimizes the
//! strong-segregation free energy, the period is strained to `L = W/n`, the
//! A block wets the walls (half A domains at both walls), and the
//! commensurability penalty `F(L)/F(L₀)` is reported per trench.
//!
//! # Key equations
//!
//! ```text
//!   free energy per chain (strong segregation)  F(L) = a L² + b / L
//!     (chain stretching + interfacial tension)  ⇒ minimum at L₀ = (b / 2a)^{1/3}
//!   commensurability penalty                    F(L)/F(L₀) = (λ² + 2/λ) / 3,  λ = L/L₀
//!   periods in a trench                         n = argmin_{n ∈ {⌊W/L₀⌋, ⌈W/L₀⌉}, n ≥ 1} F(W/n)
//!   interface profile                           φ_A = ½ [1 + tanh(2d / w)]
//!     (d = signed distance to the nearest A/B interface, w = interface width)
//!   hexagonal cylinders, centre spacing L₀      r = L₀ √(√3 f / 2π)
//!   order–disorder (mean field, f = ½)          χN > 10.495
//! ```
//!
//! # Model status
//!
//! - Analytic, **not SCFT**: no free-energy minimization in space, no
//!   defect nucleation, no chemistry of the guiding surfaces. Registration
//!   is modeled for lamellae in 1D graphoepitaxy trenches only (2D
//!   templates: row by row); cylinders and spheres are a free-running
//!   lattice clipped to the template's open region.
//! - `chi_n` enters only the order–disorder check (mean-field value for a
//!   symmetric diblock; asymmetric blocks order only at larger χN, so the
//!   check is necessary, not sufficient). `interface_width_nm` sets the
//!   tanh interface profile; it is a user input, not derived from χN.
//! - [`DSAResult::defect_density`] is an **illustrative heuristic index**
//!   built from the commensurability mismatch — not calibrated to any
//!   measured defectivity. It is `NaN` (and `is_defect_free` false) when no
//!   commensurability was evaluated.
//! - Spheres use a simple-cubic lattice of constant L₀ (radius from the
//!   simple-cubic volume fraction); real sphere-forming BCPs adopt BCC (or
//!   hexagonal packing in monolayers).
//!
//! # References
//!
//! - L. Leibler, "Theory of microphase separation in block copolymers",
//!   Macromolecules 13 (1980) — mean-field order–disorder transition
//!   (χN)_c = 10.495 for symmetric diblocks.

use ndarray::Array2;
use serde::{Deserialize, Serialize};

use crate::error::LithographyError;

/// Mean-field order–disorder transition of a symmetric diblock copolymer.
pub const CHI_N_ODT_SYMMETRIC: f64 = 10.495;

/// Block copolymer morphology.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DSAMorphology {
    /// Alternating line/space pattern.
    Lamellar,
    /// Hexagonal array of cylinders.
    Cylindrical,
    /// Array of spheres (rendered on a simple-cubic lattice).
    Spherical,
}

/// DSA simulation parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DSAParams {
    /// Natural BCP period L₀ (nm). E.g., 28nm for PS-b-PMMA.
    pub l0_nm: f64,
    /// Flory-Huggins interaction parameter × degree of polymerization (χN).
    /// Used for the order–disorder check (must exceed 10.495).
    pub chi_n: f64,
    /// Volume fraction of the minority (A) block (0.5 for symmetric lamellae).
    pub volume_fraction: f64,
    /// Morphology type.
    pub morphology: DSAMorphology,
    /// A/B interface width w (nm) of the tanh composition profile
    /// (0 = sharp interfaces).
    pub interface_width_nm: f64,
}

impl DSAParams {
    /// Create parameters for PS-b-PMMA lamellar DSA.
    pub fn ps_pmma_lamellar(l0_nm: f64) -> crate::error::Result<Self> {
        if l0_nm.is_nan() || l0_nm <= 0.0 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "l0_nm",
                value: if l0_nm.is_nan() { f64::NAN } else { l0_nm },
                reason: "must be positive",
            });
        }
        Ok(Self {
            l0_nm,
            chi_n: 20.0, // typical for PS-b-PMMA
            volume_fraction: 0.5,
            morphology: DSAMorphology::Lamellar,
            interface_width_nm: l0_nm / 10.0, // ~10% of period
        })
    }

    /// Check the parameters: `L₀ > 0`, `0 < f < 1`, `w ≥ 0`, and χN above
    /// the mean-field order–disorder transition.
    pub fn validate(&self) -> crate::error::Result<()> {
        if !(self.l0_nm.is_finite() && self.l0_nm > 0.0) {
            return Err(LithographyError::InvalidParameter {
                name: "l0_nm",
                value: self.l0_nm,
                reason: "must be positive and finite",
            });
        }
        if !(self.volume_fraction > 0.0 && self.volume_fraction < 1.0) {
            return Err(LithographyError::InvalidParameter {
                name: "volume_fraction",
                value: self.volume_fraction,
                reason: "must be in (0, 1)",
            });
        }
        if !(self.interface_width_nm.is_finite() && self.interface_width_nm >= 0.0) {
            return Err(LithographyError::InvalidParameter {
                name: "interface_width_nm",
                value: self.interface_width_nm,
                reason: "must be non-negative and finite",
            });
        }
        if self.chi_n.is_nan() || self.chi_n <= CHI_N_ODT_SYMMETRIC {
            return Err(LithographyError::InvalidParameter {
                name: "chi_n",
                value: self.chi_n,
                reason: "below the mean-field order-disorder transition (chi*N = 10.495): the melt is disordered",
            });
        }
        Ok(())
    }
}

impl Default for DSAParams {
    fn default() -> Self {
        Self::ps_pmma_lamellar(28.0).expect("default L0 28nm is valid")
    }
}

/// Commensurability of lamellae confined in one trench.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommensurabilityReport {
    /// Trench (confinement) width W (nm).
    pub confinement_nm: f64,
    /// Lamellar periods accommodated.
    pub periods: usize,
    /// Strained period L = W / n (nm).
    pub period_nm: f64,
    /// Strain L/L₀ − 1.
    pub strain: f64,
    /// Free-energy ratio F(L)/F(L₀) = (λ² + 2/λ)/3 (≥ 1).
    pub free_energy_ratio: f64,
}

/// Result of DSA simulation.
#[derive(Debug)]
pub struct DSAResult {
    /// A-block fraction (1 = A, 0 = B or guiding-wall material).
    pub pattern: Array2<f64>,
    /// **Illustrative heuristic index** in "defects/μm²":
    /// `100 · min(10 · max mismatch, 1)` with mismatch `|W/L₀ − n|` of the
    /// worst trench. Not calibrated to measured defectivity; `NaN` when no
    /// commensurability was evaluated.
    pub defect_density: f64,
    /// `defect_density < 1` (false when not evaluated).
    pub is_defect_free: bool,
    /// Lamellae: A-domain width f·L of the first trench (f·L₀ unconfined);
    /// cylinders and spheres: domain diameter (nm).
    pub assembled_cd_nm: f64,
    /// One report per distinct confining trench (lamellae only).
    pub commensurability: Vec<CommensurabilityReport>,
}

/// Check commensurability between template pitch and BCP period.
/// Returns the commensurability ratio (should be close to integer).
pub fn commensurability_ratio(template_pitch_nm: f64, l0_nm: f64) -> f64 {
    template_pitch_nm / l0_nm
}

/// Check if template and BCP are commensurable within tolerance.
pub fn is_commensurable(template_pitch_nm: f64, l0_nm: f64, tolerance: f64) -> bool {
    let ratio = commensurability_ratio(template_pitch_nm, l0_nm);
    let nearest_int = ratio.round();
    (ratio - nearest_int).abs() < tolerance
}

/// Strong-segregation free energy of lamellae at period `period_nm`
/// relative to the natural period: `F(L)/F(L₀) = (λ² + 2/λ)/3`, `λ = L/L₀`.
pub fn lamellar_free_energy_ratio(period_nm: f64, l0_nm: f64) -> f64 {
    let lambda = period_nm / l0_nm;
    (lambda * lambda + 2.0 / lambda) / 3.0
}

/// Lamellae confined in a trench of width `trench_nm`: the period count
/// minimizing the free energy (among `⌊W/L₀⌋` and `⌈W/L₀⌉`, at least 1)
/// and the resulting strain and penalty.
pub fn confined_lamellae(trench_nm: f64, l0_nm: f64) -> CommensurabilityReport {
    let ratio = trench_nm / l0_nm;
    let candidates = [ratio.floor().max(1.0), ratio.ceil().max(1.0)];
    let n = candidates
        .iter()
        .copied()
        .min_by(|a, b| {
            lamellar_free_energy_ratio(trench_nm / a, l0_nm)
                .total_cmp(&lamellar_free_energy_ratio(trench_nm / b, l0_nm))
        })
        .unwrap_or(1.0);
    let period = trench_nm / n;
    CommensurabilityReport {
        confinement_nm: trench_nm,
        periods: n as usize,
        period_nm: period,
        strain: period / l0_nm - 1.0,
        free_energy_ratio: lamellar_free_energy_ratio(period, l0_nm),
    }
}

/// A-block fraction of a smoothed domain: `½[1 + tanh(2d/w)]` for signed
/// distance `d` to the interface (positive inside A).
fn profile(d: f64, w: f64) -> f64 {
    if w <= 0.0 {
        if d >= 0.0 {
            1.0
        } else {
            0.0
        }
    } else {
        0.5 * (1.0 + (2.0 * d / w).tanh())
    }
}

/// Lamellar A fraction at offset `u` from a registration point where an A
/// domain is centred, for period `period` and A fraction `f`.
fn lamella(u: f64, period: f64, f: f64, w: f64) -> f64 {
    let v = u.rem_euclid(period);
    let dist_to_centre = v.min(period - v);
    profile(0.5 * f * period - dist_to_centre, w)
}

/// Render one 1D row of lamellae on a template (`> 0.5` = open trench,
/// otherwise wall). Returns the row and the reports of fully bounded
/// trenches.
fn lamellae_row(
    template: &[f64],
    x_nm: &[f64],
    params: &DSAParams,
) -> (Vec<f64>, Vec<CommensurabilityReport>) {
    let n = template.len();
    let (l0, f, w) = (
        params.l0_nm,
        params.volume_fraction,
        params.interface_width_nm,
    );
    let mut row = vec![0.0; n];
    let mut reports = Vec::new();
    let open: Vec<bool> = template.iter().map(|&t| t > 0.5).collect();
    if open.iter().all(|&o| o) {
        // Unconfined: free-running lamellae referenced to x = 0.
        for i in 0..n {
            row[i] = lamella(x_nm[i], l0, f, w);
        }
        return (row, reports);
    }
    let dx = |i: usize| {
        if i + 1 < n {
            x_nm[i + 1] - x_nm[i]
        } else {
            x_nm[i] - x_nm[i - 1]
        }
    };
    let mut i = 0;
    while i < n {
        if !open[i] {
            i += 1;
            continue;
        }
        let start = i;
        while i < n && open[i] {
            i += 1;
        }
        let end = i - 1;
        let bounded_left = start > 0;
        let bounded_right = end + 1 < n;
        let left_wall = if bounded_left {
            0.5 * (x_nm[start - 1] + x_nm[start])
        } else {
            x_nm[start] - 0.5 * dx(start)
        };
        let right_wall = if bounded_right {
            0.5 * (x_nm[end] + x_nm[end + 1])
        } else {
            x_nm[end] + 0.5 * dx(end)
        };
        let (anchor, period) = if bounded_left && bounded_right {
            let report = confined_lamellae(right_wall - left_wall, l0);
            let p = report.period_nm;
            reports.push(report);
            (left_wall, p)
        } else if bounded_right {
            (right_wall, l0)
        } else {
            (left_wall, l0)
        };
        for k in start..=end {
            row[k] = lamella(x_nm[k] - anchor, period, f, w);
        }
    }
    (row, reports)
}

/// Heuristic defect index and flag from trench reports (see
/// [`DSAResult::defect_density`]).
fn defect_index(reports: &[CommensurabilityReport], l0_nm: f64) -> (f64, bool) {
    if reports.is_empty() {
        return (f64::NAN, false);
    }
    let mismatch = reports
        .iter()
        .map(|r| (r.confinement_nm / l0_nm - r.periods as f64).abs())
        .fold(0.0, f64::max);
    let density = 100.0 * (10.0 * mismatch).min(1.0);
    (density, density < 1.0)
}

/// Simulate DSA assembly on a 1D template pattern.
///
/// `template[i] > 0.5` marks open (trench) sample points where the BCP
/// assembles; other points are guiding walls (A fraction 0). Lamellae in a
/// trench bounded on both sides are registered to its walls with the
/// free-energy-optimal number of periods; open runs touching the array ends
/// use the natural period registered to their one wall; a template without
/// walls gives free-running lamellae referenced to `x = 0`. Only lamellar
/// morphology is rendered in 1D.
pub fn simulate_dsa_1d(
    template_pattern: &[f64],
    x_nm: &[f64],
    params: &DSAParams,
) -> crate::error::Result<DSAResult> {
    params.validate()?;
    let n = template_pattern.len();
    if n < 2 || x_nm.len() != n {
        return Err(LithographyError::DimensionMismatch {
            expected: format!("template and x_nm of equal length ≥ 2 (template {n})"),
            got: format!("x_nm {}", x_nm.len()),
        });
    }
    let (row, reports) = lamellae_row(template_pattern, x_nm, params);
    let (defect_density, is_defect_free) = defect_index(&reports, params.l0_nm);
    let period = reports.first().map_or(params.l0_nm, |r| r.period_nm);

    let pattern = Array2::from_shape_vec((1, n), row).map_err(|e| {
        LithographyError::InternalError(format!("DSA 1D pattern shape error: {}", e))
    })?;

    Ok(DSAResult {
        pattern,
        defect_density,
        is_defect_free,
        assembled_cd_nm: params.volume_fraction * period,
        commensurability: reports,
    })
}

/// Simulate DSA on a 2D template (`> 0.5` = open region).
///
/// Lamellae (stripes along y) are registered row by row to the trench walls
/// of each template row, as in [`simulate_dsa_1d`]. Cylinders (hexagonal
/// lattice, centre spacing L₀) and spheres (simple-cubic lattice, constant
/// L₀) are a free-running lattice clipped to the open region — no
/// registration and no commensurability evaluation for them.
///
/// # Panics
/// Panics if `params` fails [`DSAParams::validate`] (kept infallible for
/// API compatibility; validate first to handle errors).
pub fn simulate_dsa_2d(template: &Array2<f64>, pixel_nm: f64, params: &DSAParams) -> DSAResult {
    params
        .validate()
        .expect("invalid DSA parameters (call DSAParams::validate first)");
    let (ny, nx) = template.dim();
    let mut pattern = Array2::zeros((ny, nx));
    let (l0, f, w) = (
        params.l0_nm,
        params.volume_fraction,
        params.interface_width_nm,
    );
    let mut reports: Vec<CommensurabilityReport> = Vec::new();

    let assembled_cd = match params.morphology {
        DSAMorphology::Lamellar => {
            let x_nm: Vec<f64> = (0..nx).map(|j| j as f64 * pixel_nm).collect();
            for i in 0..ny {
                let row_template: Vec<f64> = template.row(i).to_vec();
                let (row, row_reports) = lamellae_row(&row_template, &x_nm, params);
                for (j, v) in row.into_iter().enumerate() {
                    pattern[[i, j]] = v;
                }
                for r in row_reports {
                    if !reports
                        .iter()
                        .any(|q| (q.confinement_nm - r.confinement_nm).abs() < 1e-6)
                    {
                        reports.push(r);
                    }
                }
            }
            f * reports.first().map_or(l0, |r| r.period_nm)
        }
        DSAMorphology::Cylindrical => {
            // Hexagonal lattice with centre spacing L₀: rows every
            // L₀·√3/2, odd rows offset by L₀/2; check the two nearest rows.
            let r_cyl = l0 * (3.0_f64.sqrt() * f / (2.0 * std::f64::consts::PI)).sqrt();
            let row_h = l0 * 3.0_f64.sqrt() / 2.0;
            for i in 0..ny {
                for j in 0..nx {
                    let (x, y) = (j as f64 * pixel_nm, i as f64 * pixel_nm);
                    let base = (y / row_h).floor() as i64;
                    let mut dist = f64::INFINITY;
                    for row in [base - 1, base, base + 1] {
                        let x_off = if row.rem_euclid(2) == 0 {
                            0.0
                        } else {
                            l0 / 2.0
                        };
                        let cx = ((x - x_off) / l0).round() * l0 + x_off;
                        let cy = row as f64 * row_h;
                        dist = dist.min(((x - cx).powi(2) + (y - cy).powi(2)).sqrt());
                    }
                    pattern[[i, j]] = profile(r_cyl - dist, w);
                }
            }
            2.0 * r_cyl
        }
        DSAMorphology::Spherical => {
            let r_sph = l0 * (3.0 * f / (4.0 * std::f64::consts::PI)).cbrt();
            for i in 0..ny {
                for j in 0..nx {
                    let (x, y) = (j as f64 * pixel_nm, i as f64 * pixel_nm);
                    let cx = (x / l0).round() * l0;
                    let cy = (y / l0).round() * l0;
                    let dist = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
                    pattern[[i, j]] = profile(r_sph - dist, w);
                }
            }
            2.0 * r_sph
        }
    };

    // Guiding walls carry no BCP.
    ndarray::Zip::from(&mut pattern)
        .and(template)
        .for_each(|p, &t| {
            if t <= 0.5 {
                *p = 0.0;
            }
        });

    let (defect_density, is_defect_free) = defect_index(&reports, l0);
    DSAResult {
        pattern,
        defect_density,
        is_defect_free,
        assembled_cd_nm: assembled_cd,
        commensurability: reports,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commensurability_exact() {
        assert!(is_commensurable(84.0, 28.0, 0.1)); // 3× exact
    }

    #[test]
    fn test_commensurability_mismatch() {
        assert!(!is_commensurable(85.0, 28.0, 0.01)); // 3.036×, off by 3.6%
    }

    #[test]
    fn test_free_energy_ratio_fixture() {
        // (λ² + 2/λ)/3: 1 at λ = 1; (1.21 + 1.818182)/3 = 1.0093939 at 1.1;
        // (0.81 + 2.222222)/3 = 1.0107407 at 0.9 (compression costs more).
        assert!((lamellar_free_energy_ratio(28.0, 28.0) - 1.0).abs() < 1e-15);
        assert!((lamellar_free_energy_ratio(30.8, 28.0) - 1.009_393_9).abs() < 1e-6);
        assert!((lamellar_free_energy_ratio(25.2, 28.0) - 1.010_740_7).abs() < 1e-6);
        // Minimum at λ = 1: dF/dλ = (2λ − 2/λ²)/3 = 0.
        let h = 1e-5;
        let d = (lamellar_free_energy_ratio(28.0 + h, 28.0)
            - lamellar_free_energy_ratio(28.0 - h, 28.0))
            / (2.0 * h);
        assert!(d.abs() < 1e-9);
    }

    #[test]
    fn test_confined_lamellae_period_choice() {
        let exact = confined_lamellae(84.0, 28.0);
        assert_eq!(exact.periods, 3);
        assert!(exact.strain.abs() < 1e-12);
        // W/L₀ = 3.214: n = 3 (λ = 1.0714, F = 1.00488) beats n = 4
        // (λ = 0.8036, F = 1.0449) → L = 30 nm.
        let r = confined_lamellae(90.0, 28.0);
        assert_eq!(r.periods, 3);
        assert!((r.period_nm - 30.0).abs() < 1e-12);
        assert!((r.free_energy_ratio - 1.004_88).abs() < 1e-4);
        // A trench narrower than L₀ still holds one (compressed) period.
        assert_eq!(confined_lamellae(20.0, 28.0).periods, 1);
    }

    #[test]
    fn test_dsa_1d_registers_lamellae_to_trench_walls() {
        // Walls at x < 20 and x > 104 (trench W = 84 = 3 L₀): A domains
        // centred on the walls and at +28, +56; B centred at +14, +42, +70.
        let params = DSAParams::ps_pmma_lamellar(28.0).unwrap();
        let x_nm: Vec<f64> = (0..125).map(|i| i as f64 + 0.5).collect();
        let template: Vec<f64> = x_nm
            .iter()
            .map(|&x| if (20.0..104.0).contains(&x) { 1.0 } else { 0.0 })
            .collect();
        let r = simulate_dsa_1d(&template, &x_nm, &params).unwrap();
        assert_eq!(r.commensurability.len(), 1);
        let c = &r.commensurability[0];
        assert!((c.confinement_nm - 84.0).abs() < 1e-9 && c.periods == 3);
        let at = |x: f64| r.pattern[[0, (x - 0.5) as usize]];
        for a in [48.5, 76.5] {
            assert!(at(a) > 0.99, "A centre at {a}: {}", at(a));
        }
        for b in [34.5, 62.5, 90.5] {
            assert!(at(b) < 0.01, "B centre at {b}: {}", at(b));
        }
        assert_eq!(at(10.5), 0.0); // wall
        assert!(r.is_defect_free && r.defect_density < 1.0);
        assert!((r.assembled_cd_nm - 14.0).abs() < 1e-9);
    }

    #[test]
    fn test_dsa_1d_basic() {
        let params = DSAParams::ps_pmma_lamellar(28.0).unwrap();
        let n = 256;
        let x_nm: Vec<f64> = (0..n).map(|i| i as f64 * 1.0).collect();
        let template: Vec<f64> = x_nm
            .iter()
            .map(|&x| if (x / 56.0) as i64 % 2 == 0 { 1.0 } else { 0.0 })
            .collect();

        let result = simulate_dsa_1d(&template, &x_nm, &params).unwrap();
        assert_eq!(result.pattern.ncols(), n);
        assert!(result.assembled_cd_nm > 0.0);
        // 56 nm trenches hold exactly two periods.
        assert!(result
            .commensurability
            .iter()
            .all(|c| c.periods == 2 && c.strain.abs() < 1e-9));
    }

    #[test]
    fn test_mismatched_trench_flags_heuristic_defects() {
        // W = 98 nm = 3.5 L₀: maximal mismatch → heuristic index 100.
        let params = DSAParams::ps_pmma_lamellar(28.0).unwrap();
        let x_nm: Vec<f64> = (0..140).map(|i| i as f64 + 0.5).collect();
        let template: Vec<f64> = x_nm
            .iter()
            .map(|&x| if (20.0..118.0).contains(&x) { 1.0 } else { 0.0 })
            .collect();
        let r = simulate_dsa_1d(&template, &x_nm, &params).unwrap();
        assert!(!r.is_defect_free);
        assert!((r.defect_density - 100.0).abs() < 1e-9);
        assert!(r.commensurability[0].free_energy_ratio > 1.01);
    }

    #[test]
    fn test_order_disorder_check() {
        let params = DSAParams {
            chi_n: 9.0,
            ..DSAParams::ps_pmma_lamellar(28.0).unwrap()
        };
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        assert!(simulate_dsa_1d(&[1.0; 10], &x, &params).is_err());
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_dsa_2d_lamellar() {
        let params = DSAParams::ps_pmma_lamellar(28.0).unwrap();
        let template = Array2::ones((64, 64));
        let result = simulate_dsa_2d(&template, 2.0, &params);
        assert_eq!(result.pattern.dim(), (64, 64));
        // Unconfined: nothing evaluated → no defect claim.
        assert!(result.defect_density.is_nan() && !result.is_defect_free);
    }

    #[test]
    fn test_dsa_2d_cylindrical() {
        let params = DSAParams {
            morphology: DSAMorphology::Cylindrical,
            volume_fraction: 0.3,
            interface_width_nm: 0.0,
            ..DSAParams::ps_pmma_lamellar(28.0).unwrap()
        };
        let template = Array2::ones((128, 128));
        let result = simulate_dsa_2d(&template, 1.0, &params);
        assert_eq!(result.pattern.dim(), (128, 128));
        // Full hexagonal disks: the A area fraction equals f.
        let frac = result.pattern.sum() / result.pattern.len() as f64;
        assert!((frac - 0.3).abs() < 0.02, "area fraction {frac}");
        let r = 28.0 * (3.0_f64.sqrt() * 0.3 / (2.0 * std::f64::consts::PI)).sqrt();
        assert!((result.assembled_cd_nm - 2.0 * r).abs() < 1e-12);
    }

    #[test]
    fn test_dsa_2d_template_clips_pattern() {
        let params = DSAParams {
            morphology: DSAMorphology::Cylindrical,
            volume_fraction: 0.3,
            ..DSAParams::ps_pmma_lamellar(28.0).unwrap()
        };
        let template = Array2::from_shape_fn((64, 64), |(_, j)| if j < 32 { 1.0 } else { 0.0 });
        let result = simulate_dsa_2d(&template, 2.0, &params);
        for i in 0..64 {
            for j in 32..64 {
                assert_eq!(result.pattern[[i, j]], 0.0);
            }
        }
        assert!(result.pattern.iter().any(|&v| v > 0.5));
    }

    #[test]
    fn test_invalid_l0_zero() {
        assert!(DSAParams::ps_pmma_lamellar(0.0).is_err());
        assert!(DSAParams::ps_pmma_lamellar(-5.0).is_err());
        assert!(DSAParams::ps_pmma_lamellar(f64::NAN).is_err());
    }

    #[test]
    fn test_dsa_2d_spherical() {
        let params = DSAParams {
            morphology: DSAMorphology::Spherical,
            ..DSAParams::ps_pmma_lamellar(28.0).unwrap()
        };
        let template = Array2::ones((64, 64));
        let result = simulate_dsa_2d(&template, 2.0, &params);
        assert_eq!(result.pattern.dim(), (64, 64));
        // Spherical morphology should produce discrete sphere regions (1.0) and matrix (0.0)
        let has_ones = result.pattern.iter().any(|&v| v > 0.5);
        let has_zeros = result.pattern.iter().any(|&v| v < 0.5);
        assert!(
            has_ones && has_zeros,
            "Spherical DSA should produce both sphere and matrix regions"
        );
    }
}
