//! X-ray mass-attenuation data for LIGA deep X-ray lithography (0.1-20 keV).
//!
//! Provides per-element mass attenuation coefficients `mu/rho` [cm^2/g] on a
//! shared log-spaced energy grid with **log-log interpolation**, plus a
//! [`Compound`] mixture type that combines elements by mass fraction. This is
//! the material layer consumed by [`crate::deep_xray`] to turn a synchrotron
//! bending-magnet spectrum into a depth-resolved absorbed-energy profile in
//! thick resist (PMMA/SU-8), through an Au absorber on a thin membrane.
//!
//! # Physics
//!
//! In the 0.1-20 keV band the photoelectric effect dominates the total cross
//! section for the low-Z resist/membrane elements, so the linear attenuation
//! coefficient `mu` and the energy-absorption coefficient `mu_en` are nearly
//! equal (the photoelectron deposits its energy locally, fluorescence and
//! Compton down-scatter are minor). LIGA dose modelling therefore takes
//! `mu_en ~ mu`. Between absorption edges the photoelectric term scales
//! roughly as `E^-3`, which the log-log interpolation reproduces exactly for a
//! pure power law.
//!
//! # Data provenance and limitations
//!
//! Tabulated values are transcribed from NIST XCOM / FFAST total-attenuation
//! tables (photoelectric + coherent + incoherent). The grid deliberately
//! *brackets* absorption edges rather than resolving them:
//!
//! - Au: M-edges cluster at ~2.2-3.4 keV and L-edges at 11.9-14.4 keV,
//! - Ti: K-edge at 4.966 keV,
//! - Si: K-edge at 1.839 keV,
//! - O:  K-edge at 0.543 keV, N: 0.402 keV, C: 0.284 keV.
//!
//! The interpolation **smooths edge fine structure** (the tabulated node values
//! reflect the jump across the bracketing nodes, but the near-edge XANES/EXAFS
//! wiggles and the exact edge position are not resolved). Sub-keV values below
//! each element's K-edge are approximate; the anchors that matter for LIGA
//! (multi-keV) are pinned by tests: PMMA `mu/rho ~ 6.5 cm^2/g` at 8 keV and
//! `~125 cm^2/g` at 3 keV, C `~4.58` at 8 keV, Au `~111` at 8 keV.

use serde::Serialize;

/// Shared log-spaced energy grid in keV for all element tables. Chosen to
/// bracket the K/L/M edges of the LIGA element set (see module docs).
static ENERGY_GRID_KEV: [f64; 17] = [
    0.1, 0.15, 0.2, 0.3, 0.5, 0.8, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0, 15.0, 20.0,
];

/// Per-element mass attenuation coefficient `mu/rho` [cm^2/g] tabulated on a
/// log-spaced energy grid, interpolated log-log in both energy and coefficient.
#[derive(Debug, Clone)]
pub struct ElementAttenuation {
    /// Element symbol (e.g. `"Au"`).
    pub symbol: &'static str,
    /// Atomic number.
    pub z: u32,
    /// Energy grid in keV (ascending).
    energies_kev: &'static [f64],
    /// Mass attenuation `mu/rho` [cm^2/g] at each grid energy.
    mu_over_rho_cm2_g: &'static [f64],
}

impl ElementAttenuation {
    /// Mass attenuation coefficient `mu/rho` [cm^2/g] at `energy_kev`, by
    /// log-log interpolation on the tabulated grid. Energies outside the grid
    /// are clamped to the endpoint values (no extrapolation). Edge fine
    /// structure between bracketing nodes is smoothed (see module docs).
    pub fn mu_over_rho(&self, energy_kev: f64) -> f64 {
        let e = self.energies_kev;
        let m = self.mu_over_rho_cm2_g;
        let n = e.len();
        if energy_kev <= e[0] {
            return m[0];
        }
        if energy_kev >= e[n - 1] {
            return m[n - 1];
        }
        // Find the bracket e[i] <= energy < e[i + 1].
        let mut i = 0;
        while i + 1 < n && e[i + 1] < energy_kev {
            i += 1;
        }
        let (e0, e1) = (e[i], e[i + 1]);
        let (m0, m1) = (m[i], m[i + 1]);
        // log-log linear interpolation: mu = m0 * (m1/m0)^t, t in [0, 1].
        let t = (energy_kev.ln() - e0.ln()) / (e1.ln() - e0.ln());
        (m0.ln() + t * (m1.ln() - m0.ln())).exp()
    }
}

// -- Element tables (NIST XCOM/FFAST, cm^2/g). See module docs on provenance
//    and edge smoothing. Grid order matches ENERGY_GRID_KEV. --

// Hydrogen (Z=1): no edge in range (K-edge 13.6 eV); Compton-plateau tail.
static H_MU: [f64; 17] = [
    6800.0, 2140.0, 900.0, 267.0, 58.0, 14.0, 7.217, 2.148, 1.059, 0.5612, 0.4483, 0.4130, 0.3959,
    0.3812, 0.3719, 0.3568, 0.3458,
];

// Carbon (Z=6): K-edge 0.284 keV (bracketed by the 0.2/0.3 keV nodes).
static C_MU: [f64; 17] = [
    10500.0, 6800.0, 4200.0, 35000.0, 16490.0, 4223.0, 2211.0, 700.2, 302.6, 90.33, 37.78, 19.12,
    10.95, 4.576, 2.373, 0.80, 0.47,
];

// Nitrogen (Z=7): K-edge 0.402 keV (bracketed by the 0.3/0.5 keV nodes).
static N_MU: [f64; 17] = [
    22000.0, 14000.0, 8500.0, 4200.0, 24700.0, 6324.0, 3311.0, 1083.0, 476.9, 151.6, 64.29, 32.71,
    18.90, 7.984, 4.087, 1.252, 0.5581,
];

// Oxygen (Z=8): K-edge 0.543 keV (bracketed by the 0.5/0.8 keV nodes).
static O_MU: [f64; 17] = [
    22000.0, 13000.0, 8500.0, 4400.0, 2100.0, 8740.0, 4576.0, 1545.0, 686.9, 221.2, 94.10, 48.07,
    27.83, 11.80, 6.058, 1.856, 0.8047,
];

// Silicon (Z=14): K-edge 1.839 keV (bracketed by the 1.5/2.0 keV nodes; the
// jump shows as the 578.9 -> 2460 step).
static SI_MU: [f64; 17] = [
    34000.0, 20000.0, 13000.0, 7500.0, 4200.0, 3100.0, 1570.0, 578.9, 2460.0, 978.4, 461.8, 244.6,
    147.9, 65.32, 33.89, 10.60, 4.464,
];

// Gold (Z=79): M-edges ~2.2-3.4 keV (the 1650 -> 2200 step across 2/3 keV) and
// L-edges 11.9-14.4 keV (the 60 -> 116 step across 10/15 keV). Sub-keV N/O
// edge structure is smoothed into a monotone tail (approximate).
static AU_MU: [f64; 17] = [
    13000.0, 10000.0, 8300.0, 6200.0, 4400.0, 3400.0, 2900.0, 2050.0, 1650.0, 2200.0, 773.0, 416.0,
    246.0, 111.0, 60.0, 116.0, 52.0,
];

// Beryllium (Z=4): K-edge 0.111 keV (the 7000 -> 72500 step across 0.1/0.15 keV).
static BE_MU: [f64; 17] = [
    7000.0, 72500.0, 39600.0, 14900.0, 3660.0, 1081.0, 604.1, 179.7, 74.69, 21.27, 8.685, 4.369,
    2.527, 1.124, 0.6466, 0.3143, 0.2260,
];

// Titanium (Z=22): L-edges ~0.46-0.56 keV (the 3800 -> 24000 step) and K-edge
// 4.966 keV (the 154.9 -> 687.7 step across 4/5 keV).
static TI_MU: [f64; 17] = [
    15000.0, 9800.0, 6500.0, 3800.0, 24000.0, 11000.0, 5869.0, 2096.0, 986.7, 332.2, 154.9, 687.7,
    424.4, 202.5, 110.5, 37.79, 17.16,
];

static ELEMENT_H: ElementAttenuation = ElementAttenuation {
    symbol: "H",
    z: 1,
    energies_kev: &ENERGY_GRID_KEV,
    mu_over_rho_cm2_g: &H_MU,
};
static ELEMENT_C: ElementAttenuation = ElementAttenuation {
    symbol: "C",
    z: 6,
    energies_kev: &ENERGY_GRID_KEV,
    mu_over_rho_cm2_g: &C_MU,
};
static ELEMENT_N: ElementAttenuation = ElementAttenuation {
    symbol: "N",
    z: 7,
    energies_kev: &ENERGY_GRID_KEV,
    mu_over_rho_cm2_g: &N_MU,
};
static ELEMENT_O: ElementAttenuation = ElementAttenuation {
    symbol: "O",
    z: 8,
    energies_kev: &ENERGY_GRID_KEV,
    mu_over_rho_cm2_g: &O_MU,
};
static ELEMENT_SI: ElementAttenuation = ElementAttenuation {
    symbol: "Si",
    z: 14,
    energies_kev: &ENERGY_GRID_KEV,
    mu_over_rho_cm2_g: &SI_MU,
};
static ELEMENT_AU: ElementAttenuation = ElementAttenuation {
    symbol: "Au",
    z: 79,
    energies_kev: &ENERGY_GRID_KEV,
    mu_over_rho_cm2_g: &AU_MU,
};
static ELEMENT_BE: ElementAttenuation = ElementAttenuation {
    symbol: "Be",
    z: 4,
    energies_kev: &ENERGY_GRID_KEV,
    mu_over_rho_cm2_g: &BE_MU,
};
static ELEMENT_TI: ElementAttenuation = ElementAttenuation {
    symbol: "Ti",
    z: 22,
    energies_kev: &ENERGY_GRID_KEV,
    mu_over_rho_cm2_g: &TI_MU,
};

/// Look up an element's attenuation table by chemical symbol. Covers the LIGA
/// element set: H, C, N, O, Si, Au, Be, Ti. Returns `None` for anything else.
pub fn element(symbol: &str) -> Option<&'static ElementAttenuation> {
    match symbol {
        "H" => Some(&ELEMENT_H),
        "C" => Some(&ELEMENT_C),
        "N" => Some(&ELEMENT_N),
        "O" => Some(&ELEMENT_O),
        "Si" => Some(&ELEMENT_SI),
        "Au" => Some(&ELEMENT_AU),
        "Be" => Some(&ELEMENT_BE),
        "Ti" => Some(&ELEMENT_TI),
        _ => None,
    }
}

/// A material compound: named, with a mass density and a list of
/// `(element symbol, mass fraction)` components. Attenuation follows the
/// mixture rule `(mu/rho)_comp = sum_i w_i (mu/rho)_i`.
///
/// Constructed via [`Compound::new`] (validates the mass fractions) or one of
/// the presets. Serializable but not deserializable: components reference
/// `&'static str` element symbols, so read compositions back through the
/// preset constructors rather than serde.
#[derive(Debug, Clone, Serialize)]
pub struct Compound {
    /// Human-readable name.
    pub name: String,
    /// Mass density in g/cm^3.
    pub density_g_cm3: f64,
    /// `(element symbol, mass fraction)` pairs; fractions sum to ~1.
    components: Vec<(&'static str, f64)>,
}

impl Compound {
    /// Construct a compound from `(element symbol, mass fraction)` components.
    /// Errors if any symbol is unknown or the mass fractions do not sum to 1
    /// within 1%.
    pub fn new(
        name: impl Into<String>,
        density_g_cm3: f64,
        components: Vec<(&'static str, f64)>,
    ) -> crate::error::Result<Self> {
        if density_g_cm3 <= 0.0 || density_g_cm3.is_nan() {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "density_g_cm3",
                value: density_g_cm3,
                reason: "must be positive",
            });
        }
        if components.is_empty() {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "components",
                value: 0.0,
                reason: "must contain at least one element",
            });
        }
        for (sym, frac) in &components {
            if element(sym).is_none() {
                return Err(crate::error::LithographyError::MaterialNotFound(
                    (*sym).to_string(),
                ));
            }
            if *frac < 0.0 || frac.is_nan() {
                return Err(crate::error::LithographyError::InvalidParameter {
                    name: "mass_fraction",
                    value: *frac,
                    reason: "must be non-negative",
                });
            }
        }
        let sum: f64 = components.iter().map(|(_, f)| f).sum();
        if (sum - 1.0).abs() > 0.01 {
            return Err(crate::error::LithographyError::InvalidParameter {
                name: "mass_fractions",
                value: sum,
                reason: "must sum to 1 within 1%",
            });
        }
        Ok(Self {
            name: name.into(),
            density_g_cm3,
            components,
        })
    }

    /// Single-element compound (mass fraction 1.0); used for the pure Au/Be/
    /// Ti/Si conveniences.
    fn single(name: &str, density_g_cm3: f64, symbol: &'static str) -> Self {
        Self {
            name: name.to_string(),
            density_g_cm3,
            components: vec![(symbol, 1.0)],
        }
    }

    /// PMMA resist, C5H8O2, rho = 1.19 g/cm^3. Mass fractions from
    /// stoichiometry (C 59.99%, H 8.05%, O 31.96%).
    pub fn pmma() -> Self {
        Self {
            name: "PMMA".to_string(),
            density_g_cm3: 1.19,
            components: vec![("C", 0.59985), ("H", 0.08055), ("O", 0.31960)],
        }
    }

    /// SU-8 epoxy photoresist, **approximate** stoichiometry C22H22O4,
    /// rho = 1.19 g/cm^3 (the true cross-linked network is more complex; this
    /// captures the C/H/O ratio for attenuation purposes).
    pub fn su8() -> Self {
        // C: 22*12.011 = 264.242, H: 22*1.008 = 22.176, O: 4*15.999 = 63.996
        // total 350.414 -> w_C 0.7541, w_H 0.0633, w_O 0.1826
        Self {
            name: "SU-8".to_string(),
            density_g_cm3: 1.19,
            components: vec![("C", 0.75409), ("H", 0.06328), ("O", 0.18263)],
        }
    }

    /// Pure gold (Au) absorber; pass the plated density (bulk 19.3 g/cm^3,
    /// electroplated films are often lower).
    pub fn gold(density_g_cm3: f64) -> Self {
        Self::single("Gold", density_g_cm3, "Au")
    }

    /// Pure beryllium (Be) window/membrane; bulk rho = 1.85 g/cm^3.
    pub fn beryllium(density_g_cm3: f64) -> Self {
        Self::single("Beryllium", density_g_cm3, "Be")
    }

    /// Pure titanium (Ti) membrane; bulk rho = 4.51 g/cm^3.
    pub fn titanium(density_g_cm3: f64) -> Self {
        Self::single("Titanium", density_g_cm3, "Ti")
    }

    /// Pure silicon (Si); bulk rho = 2.33 g/cm^3.
    pub fn silicon(density_g_cm3: f64) -> Self {
        Self::single("Silicon", density_g_cm3, "Si")
    }

    /// Mixture-rule mass attenuation `mu/rho` [cm^2/g] at `energy_kev`:
    /// `sum_i w_i (mu/rho)_i`. Unknown symbols (rejected at construction)
    /// contribute zero.
    pub fn mu_over_rho(&self, energy_kev: f64) -> f64 {
        self.components
            .iter()
            .map(|(sym, w)| w * element(sym).map_or(0.0, |e| e.mu_over_rho(energy_kev)))
            .sum()
    }

    /// Linear attenuation coefficient in 1/um at `energy_kev`:
    /// `(mu/rho)[cm^2/g] * rho[g/cm^3]` gives `mu` in 1/cm; divide by 1e4 for
    /// 1/um.
    pub fn mu_per_um(&self, energy_kev: f64) -> f64 {
        self.mu_over_rho(energy_kev) * self.density_g_cm3 / 1e4
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_interpolation_exact_at_nodes() {
        for sym in ["H", "C", "N", "O", "Si", "Au", "Be", "Ti"] {
            let el = element(sym).unwrap();
            for (e, m) in ENERGY_GRID_KEV.iter().zip(el.mu_over_rho_cm2_g.iter()) {
                assert_relative_eq!(el.mu_over_rho(*e), *m, max_relative = 1e-9);
            }
        }
    }

    #[test]
    fn test_clamp_outside_grid() {
        let c = element("C").unwrap();
        // Below the grid clamps to the first node, above to the last.
        assert_eq!(c.mu_over_rho(0.01), C_MU[0]);
        assert_eq!(c.mu_over_rho(100.0), C_MU[C_MU.len() - 1]);
    }

    #[test]
    fn test_pmma_anchor_8kev() {
        let pmma = Compound::pmma();
        let mu = pmma.mu_over_rho(8.0);
        assert!(
            (6.3..=6.8).contains(&mu),
            "PMMA mu/rho at 8 keV should be ~6.5, got {mu}"
        );
    }

    #[test]
    fn test_pmma_anchor_3kev() {
        let pmma = Compound::pmma();
        let mu = pmma.mu_over_rho(3.0);
        assert!(
            (110.0..=130.0).contains(&mu),
            "PMMA mu/rho at 3 keV should be ~125, got {mu}"
        );
    }

    #[test]
    fn test_element_anchors() {
        // Carbon and gold at 8 keV (task anchors).
        assert!((4.5..=4.7).contains(&element("C").unwrap().mu_over_rho(8.0)));
        assert!((100.0..=120.0).contains(&element("Au").unwrap().mu_over_rho(8.0)));
    }

    #[test]
    fn test_mixture_rule_sum_validation() {
        // Fractions that do not sum to ~1 are rejected.
        assert!(Compound::new("bad", 1.0, vec![("C", 0.5), ("O", 0.3)]).is_err());
        // Within 1% is accepted.
        assert!(Compound::new("ok", 1.0, vec![("C", 0.6), ("O", 0.405)]).is_ok());
        // Unknown symbol rejected.
        assert!(Compound::new("bad2", 1.0, vec![("Xx", 1.0)]).is_err());
        // Non-positive density rejected.
        assert!(Compound::new("bad3", 0.0, vec![("C", 1.0)]).is_err());
    }

    #[test]
    fn test_carbon_monotone_decreasing_1_to_20_kev() {
        let c = element("C").unwrap();
        let mut prev = f64::INFINITY;
        // Sweep across the edge-free 1-20 keV window (photoelectric E^-3).
        let mut e = 1.0;
        while e <= 20.0 {
            let mu = c.mu_over_rho(e);
            assert!(
                mu < prev,
                "C mu/rho not decreasing at {e} keV: {mu} >= {prev}"
            );
            prev = mu;
            e += 0.25;
        }
    }

    #[test]
    fn test_log_log_between_nodes() {
        let c = element("C").unwrap();
        // 9 keV sits between the 8 keV (4.576) and 10 keV (2.373) nodes.
        let mu = c.mu_over_rho(9.0);
        assert!(
            mu < 4.576 && mu > 2.373,
            "interpolated value {mu} should sit inside the node bracket"
        );
        // Matches the explicit log-log formula.
        let t = (9.0_f64.ln() - 8.0_f64.ln()) / (10.0_f64.ln() - 8.0_f64.ln());
        let expected = (4.576_f64.ln() + t * (2.373_f64.ln() - 4.576_f64.ln())).exp();
        assert_relative_eq!(mu, expected, max_relative = 1e-12);
    }

    #[test]
    fn test_mu_per_um_units() {
        // mu[1/um] = (mu/rho)[cm^2/g] * rho[g/cm^3] / 1e4.
        let pmma = Compound::pmma();
        let expected = pmma.mu_over_rho(8.0) * 1.19 / 1e4;
        assert_relative_eq!(pmma.mu_per_um(8.0), expected, max_relative = 1e-12);
        // ~1.3 mm 1/e depth for PMMA at 8 keV.
        assert!(pmma.mu_per_um(8.0) > 0.0 && pmma.mu_per_um(8.0) < 1e-2);
    }
}
