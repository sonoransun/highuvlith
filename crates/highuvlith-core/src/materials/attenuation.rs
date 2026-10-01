//! X-ray mass attenuation and mass energy-absorption coefficients for LIGA
//! deep X-ray lithography (and any other broadband X-ray dose calculation),
//! 30 eV - 20 MeV.
//!
//! Per-element tables of the total mass attenuation coefficient `mu/rho` and
//! the mass energy-absorption coefficient `mu_en/rho` [cm^2/g], a [`Compound`]
//! mixture type, and presets for the LIGA stack (PMMA/SU-8 resist, Au absorber,
//! Be/Ti/Si3N4 membranes, Kapton/Al/Be filters, Ni/Cu electroforming metals).
//! Consumed by [`crate::deep_xray`]: `mu` attenuates the beam, `mu_en` sets the
//! locally absorbed dose.
//!
//! # Key equations
//!
//! ```text
//!   I(t)          = I0 exp(-mu t),            mu = (mu/rho) rho
//!   (mu/rho)_mix  = sum_i w_i (mu/rho)_i      (mass fractions w_i; same for mu_en)
//!   dose rate     = Phi E (mu_en/rho) rho     (energy fluence rate x mu_en)
//!   mu_a/rho      = 2 r_e lambda f2 N_A / A    (photoabsorption from Henke f2)
//! ```
//!
//! `mu` includes photoabsorption, coherent (Rayleigh) and incoherent (Compton)
//! scattering; `mu_en` counts only the energy that stays local (photoelectron
//! and Compton-electron kinetic energy, minus fluorescence and bremsstrahlung
//! escape). The distinction matters for LIGA: in PMMA `mu_en/mu` is 0.94 at
//! 8 keV but 0.58 at 20 keV (Compton scattering carries energy away), so the
//! old `mu_en ~ mu` shortcut overstated the dose deposited by hard photons.
//!
//! # Data provenance
//!
//! - **1 keV - 20 MeV:** NIST "X-Ray Mass Attenuation Coefficients" tables of
//!   J. H. Hubbell and S. M. Seltzer (physics.nist.gov/PhysRefData/XrayMassCoef,
//!   element pages `ElemTab/zNN.html`, retrieved 2026-09-30), transcribed
//!   verbatim (4 significant figures, every tabulated row including the
//!   duplicated rows at absorption edges). Interpolation is log-log in energy
//!   and coefficient within each edge-free segment, which is exact for the
//!   piecewise power laws of photoabsorption; edges are true discontinuities
//!   (an energy exactly on an edge takes the above-edge value).
//! - **30 eV - 1 keV:** the tables above stop at 1 keV. Below it both `mu/rho`
//!   and `mu_en/rho` are the Henke photoabsorption coefficient
//!   `2 r_e lambda f2 N_A / A` from [`crate::materials::henke`] (scattering is
//!   negligible there and the photoelectron energy is deposited locally). The
//!   two data sets do not join perfectly at 1 keV: Henke/NIST is within 1.5%
//!   for Be, B, C, N, O, Al, Ti, Cr, Cu, 1.02 for Si, 0.94 for H (NIST
//!   includes Compton scattering), 1.05-1.07 for Ni, Mo, W, and 1.16 for Au
//!   (the high-Z M/N-shell region where the semi-empirical Henke and the
//!   theoretical NIST photoabsorption cross sections differ most).
//!
//! Elements: H, Be, B, C, N, O, Al, Si, Ti, Cr, Ni, Cu, Mo, W, Au.
//!
//! # Model status
//!
//! Implemented and fixture-tested (NIST node values, NIST's own PMMA compound
//! table reproduced by the mixture rule, edge positions, the Henke branch).
//! Limitations: independent-atom mixture rule (no chemical binding or
//! crystal effects - a sub-percent effect above ~1 keV); edge fine structure
//! (XANES/EXAFS) is not resolved - only the edge jumps; `mu_en` of compounds
//! is taken by the mixture rule (exact for `mu`, a standard approximation for
//! `mu_en`, which NIST's compound tables agree with to their 4 printed digits
//! for PMMA). Energies outside 30 eV - 20 MeV: the infallible `f64`
//! accessors clamp to the end values as a guard, so every entry point that
//! takes a caller's energy validates it first with [`check_energy_kev`] (the
//! LIGA energy window, [`crate::materials::diamond::xray_transmission`]); no
//! user-facing path evaluates the tables outside their range.

use serde::Serialize;

use crate::materials::henke;

/// Lowest supported photon energy in keV (the Henke table limit).
pub const MIN_ENERGY_KEV: f64 = 0.03;
/// Highest supported photon energy in keV (the NIST table limit).
pub const MAX_ENERGY_KEV: f64 = 20_000.0;
/// `Ok` when `energy_kev` lies in [[`MIN_ENERGY_KEV`], [`MAX_ENERGY_KEV`]],
/// otherwise an `InvalidParameter` error (the tables are not extrapolated).
pub fn check_energy_kev(energy_kev: f64) -> crate::error::Result<()> {
    if (MIN_ENERGY_KEV..=MAX_ENERGY_KEV).contains(&energy_kev) {
        Ok(())
    } else {
        Err(crate::error::LithographyError::InvalidParameter {
            name: "energy_kev",
            value: energy_kev,
            reason: "outside the attenuation data range 0.03 keV - 20 MeV (NIST + Henke); \
                     not extrapolated",
        })
    }
}

/// Energy (keV) below which the Henke photoabsorption branch is used.
pub const NIST_MIN_ENERGY_KEV: f64 = 1.0;

/// Per-element attenuation data: NIST `mu/rho` and `mu_en/rho` from 1 keV to
/// 20 MeV, Henke photoabsorption below 1 keV (see module docs).
#[derive(Debug, Clone)]
pub struct ElementAttenuation {
    /// Element symbol (e.g. `"Au"`).
    pub symbol: &'static str,
    /// Atomic number.
    pub z: u32,
    /// Standard atomic weight in g/mol (used for formula mass fractions).
    pub atomic_weight: f64,
    /// NIST rows `[E keV, mu/rho, mu_en/rho]` (ascending, edges duplicated).
    nist: &'static [[f64; 3]],
}

impl ElementAttenuation {
    /// Total mass attenuation coefficient `mu/rho` [cm^2/g] at `energy_kev`.
    /// Energies outside [[`MIN_ENERGY_KEV`], [`MAX_ENERGY_KEV`]] clamp (a
    /// guard; validate caller energies with [`check_energy_kev`]).
    pub fn mu_over_rho(&self, energy_kev: f64) -> f64 {
        self.coefficients(energy_kev).0
    }

    /// Mass energy-absorption coefficient `mu_en/rho` [cm^2/g] at
    /// `energy_kev` (always <= `mu/rho`). Energies outside the supported range
    /// clamp.
    pub fn mu_en_over_rho(&self, energy_kev: f64) -> f64 {
        self.coefficients(energy_kev).1
    }

    /// `(mu/rho, mu_en/rho)` in cm^2/g.
    fn coefficients(&self, energy_kev: f64) -> (f64, f64) {
        let e = if energy_kev.is_nan() {
            MIN_ENERGY_KEV
        } else {
            energy_kev.clamp(MIN_ENERGY_KEV, MAX_ENERGY_KEV)
        };
        if e < NIST_MIN_ENERGY_KEV {
            // Henke photoabsorption: scattering negligible, all local.
            let mu = henke::element(self.symbol)
                .and_then(|t| t.mass_photoabsorption_cm2_g(e * 1e3).ok())
                .unwrap_or(f64::NAN);
            return (mu, mu);
        }
        let rows = self.nist;
        let n = rows.len();
        let i = rows.partition_point(|r| r[0] <= e);
        if i >= n {
            return (rows[n - 1][1], rows[n - 1][2]);
        }
        // i >= 1 because rows[0][0] == 1 keV <= e.
        let (a, b) = (rows[i - 1], rows[i]);
        let t = (e / a[0]).ln() / (b[0] / a[0]).ln();
        let lerp = |p: f64, q: f64| (p.ln() + t * (q / p).ln()).exp();
        (lerp(a[1], b[1]), lerp(a[2], b[2]))
    }
}

macro_rules! element_static {
    ($name:ident, $sym:literal, $z:literal, $a:literal, $rows:ident) => {
        static $name: ElementAttenuation = ElementAttenuation {
            symbol: $sym,
            z: $z,
            atomic_weight: $a,
            nist: &$rows,
        };
    };
}

element_static!(ELEMENT_H, "H", 1, 1.008, H_NIST);
element_static!(ELEMENT_BE, "Be", 4, 9.012_183_1, BE_NIST);
element_static!(ELEMENT_B, "B", 5, 10.81, B_NIST);
element_static!(ELEMENT_C, "C", 6, 12.011, C_NIST);
element_static!(ELEMENT_N, "N", 7, 14.007, N_NIST);
element_static!(ELEMENT_O, "O", 8, 15.999, O_NIST);
element_static!(ELEMENT_AL, "Al", 13, 26.981_538_5, AL_NIST);
element_static!(ELEMENT_SI, "Si", 14, 28.085, SI_NIST);
element_static!(ELEMENT_TI, "Ti", 22, 47.867, TI_NIST);
element_static!(ELEMENT_CR, "Cr", 24, 51.996_1, CR_NIST);
element_static!(ELEMENT_NI, "Ni", 28, 58.693_4, NI_NIST);
element_static!(ELEMENT_CU, "Cu", 29, 63.546, CU_NIST);
element_static!(ELEMENT_MO, "Mo", 42, 95.95, MO_NIST);
element_static!(ELEMENT_W, "W", 74, 183.84, W_NIST);
element_static!(ELEMENT_AU, "Au", 79, 196.966_569, AU_NIST);

static ELEMENTS: [&ElementAttenuation; 15] = [
    &ELEMENT_H,
    &ELEMENT_BE,
    &ELEMENT_B,
    &ELEMENT_C,
    &ELEMENT_N,
    &ELEMENT_O,
    &ELEMENT_AL,
    &ELEMENT_SI,
    &ELEMENT_TI,
    &ELEMENT_CR,
    &ELEMENT_NI,
    &ELEMENT_CU,
    &ELEMENT_MO,
    &ELEMENT_W,
    &ELEMENT_AU,
];

/// Look up an element's attenuation data by chemical symbol. Covers H, Be, B,
/// C, N, O, Al, Si, Ti, Cr, Ni, Cu, Mo, W, Au; `None` for anything else.
pub fn element(symbol: &str) -> Option<&'static ElementAttenuation> {
    ELEMENTS.iter().copied().find(|e| e.symbol == symbol)
}

/// Symbols of all elements with attenuation data, in atomic-number order.
pub fn available_elements() -> Vec<&'static str> {
    ELEMENTS.iter().map(|e| e.symbol).collect()
}

/// A material compound: named, with a mass density and a list of
/// `(element symbol, mass fraction)` components. Attenuation follows the
/// mixture rule `(mu/rho)_comp = sum_i w_i (mu/rho)_i` (likewise `mu_en`).
///
/// Constructed via [`Compound::new`] (validates the mass fractions),
/// [`Compound::from_formula`] (stoichiometry -> mass fractions), or one of the
/// presets. Serializable but not deserializable: components reference
/// `&'static str` element symbols, so read compositions back through the
/// constructors rather than serde.
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

    /// Construct a compound from a chemical formula (e.g. `"C5H8O2"`,
    /// `"Si3N4"`, `"C22H10N2O5"`): mass fractions `w_i = x_i A_i / sum x_j A_j`
    /// from the formula counts `x_i` and standard atomic weights `A_i`.
    /// Errors on an unparseable formula or an element without attenuation
    /// data.
    pub fn from_formula(
        name: impl Into<String>,
        formula: &str,
        density_g_cm3: f64,
    ) -> crate::error::Result<Self> {
        let counts = henke::parse_formula(formula)?;
        let mut weighted = Vec::with_capacity(counts.len());
        for (sym, x) in counts {
            let el = element(sym)
                .ok_or_else(|| crate::error::LithographyError::MaterialNotFound(sym.into()))?;
            weighted.push((el.symbol, x * el.atomic_weight));
        }
        let total: f64 = weighted.iter().map(|(_, m)| m).sum();
        let components = weighted.into_iter().map(|(s, m)| (s, m / total)).collect();
        Self::new(name, density_g_cm3, components)
    }

    /// A compound from a short text spec (used by the Python and CLI
    /// frontends): a preset name, case-insensitive - `pmma`, `su8`/`su-8`,
    /// `kapton`, `diamond`, or an element name/symbol at its bulk density
    /// (`be`/`beryllium` 1.848, `al`/`aluminum` 2.699, `ti`/`titanium` 4.51,
    /// `si`/`silicon` 2.33, `ni`/`nickel` 8.90, `cu`/`copper` 8.96,
    /// `au`/`gold` 19.3, `w`/`tungsten` 19.3 g/cm^3) - or
    /// `"<formula>@<density g/cm^3>"`, e.g. `"C22H10N2O5@1.42"`, `"Au@17.5"`.
    pub fn from_spec(spec: &str) -> crate::error::Result<Self> {
        let s = spec.trim();
        if let Some((formula, rho)) = s.split_once('@') {
            let rho: f64 = rho.trim().parse().map_err(|_| {
                crate::error::LithographyError::MaterialNotFound(format!(
                    "{spec} (expected <formula>@<density g/cm^3>)"
                ))
            })?;
            return Self::from_formula(formula.trim(), formula.trim(), rho);
        }
        Ok(match s.to_ascii_lowercase().as_str() {
            "pmma" => Self::pmma(),
            "su8" | "su-8" => Self::su8(),
            "kapton" | "polyimide" => Self::kapton(),
            "diamond" => Self::diamond(),
            "be" | "beryllium" => Self::beryllium(1.848),
            "al" | "aluminum" | "aluminium" => Self::aluminum(2.699),
            "ti" | "titanium" => Self::titanium(4.51),
            "si" | "silicon" => Self::silicon(2.33),
            "ni" | "nickel" => Self::nickel(8.90),
            "cu" | "copper" => Self::copper(8.96),
            "au" | "gold" => Self::gold(19.3),
            "w" | "tungsten" => Self::tungsten(19.3),
            _ => {
                return Err(crate::error::LithographyError::MaterialNotFound(format!(
                    "{spec} (use a preset such as Be, Al, Ti, Kapton, Au, PMMA, or <formula>@<density>)"
                )))
            }
        })
    }

    /// Single-element compound (mass fraction 1.0).
    fn single(name: &str, density_g_cm3: f64, symbol: &'static str) -> Self {
        Self {
            name: name.to_string(),
            density_g_cm3,
            components: vec![(symbol, 1.0)],
        }
    }

    /// `(element symbol, mass fraction)` components.
    pub fn components(&self) -> &[(&'static str, f64)] {
        &self.components
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

    /// Kapton HN polyimide filter/window, C22H10N2O5, nominal rho = 1.42 g/cm^3.
    pub fn kapton() -> Self {
        Self::from_formula("Kapton", "C22H10N2O5", 1.42).expect("valid Kapton formula")
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

    /// Pure aluminium (Al) filter; bulk rho = 2.70 g/cm^3.
    pub fn aluminum(density_g_cm3: f64) -> Self {
        Self::single("Aluminum", density_g_cm3, "Al")
    }

    /// Pure nickel (Ni), e.g. electroformed LIGA parts; bulk rho = 8.90 g/cm^3.
    pub fn nickel(density_g_cm3: f64) -> Self {
        Self::single("Nickel", density_g_cm3, "Ni")
    }

    /// Pure copper (Cu); bulk rho = 8.96 g/cm^3.
    pub fn copper(density_g_cm3: f64) -> Self {
        Self::single("Copper", density_g_cm3, "Cu")
    }

    /// Pure tungsten (W), an alternative absorber; bulk rho = 19.3 g/cm^3.
    pub fn tungsten(density_g_cm3: f64) -> Self {
        Self::single("Tungsten", density_g_cm3, "W")
    }

    /// Diamond (C), rho = 3.515 g/cm^3 (windows, membranes).
    pub fn diamond() -> Self {
        Self::single("Diamond", 3.515, "C")
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

    /// Mixture-rule mass energy-absorption coefficient `mu_en/rho` [cm^2/g].
    pub fn mu_en_over_rho(&self, energy_kev: f64) -> f64 {
        self.components
            .iter()
            .map(|(sym, w)| w * element(sym).map_or(0.0, |e| e.mu_en_over_rho(energy_kev)))
            .sum()
    }

    /// Linear attenuation coefficient in 1/um at `energy_kev`:
    /// `(mu/rho)[cm^2/g] * rho[g/cm^3]` gives `mu` in 1/cm; divide by 1e4 for
    /// 1/um.
    pub fn mu_per_um(&self, energy_kev: f64) -> f64 {
        self.mu_over_rho(energy_kev) * self.density_g_cm3 / 1e4
    }

    /// Linear energy-absorption coefficient `mu_en` in 1/um at `energy_kev`.
    pub fn mu_en_per_um(&self, energy_kev: f64) -> f64 {
        self.mu_en_over_rho(energy_kev) * self.density_g_cm3 / 1e4
    }
}

// -- NIST tables (Hubbell & Seltzer), rows [E keV, mu/rho cm^2/g,
//    mu_en/rho cm^2/g], transcribed verbatim; edge rows appear twice. --

// Hydrogen (Z = 1): physics.nist.gov/PhysRefData/XrayMassCoef/ElemTab/z01.html
static H_NIST: [[f64; 3]; 36] = [
    [1.0, 7.217, 6.82],
    [1.5, 2.148, 1.752],
    [2.0, 1.059, 0.6643],
    [3.0, 0.5612, 0.1693],
    [4.0, 0.4546, 0.06549],
    [5.0, 0.4193, 0.03278],
    [6.0, 0.4042, 0.01996],
    [8.0, 0.3914, 0.0116],
    [10.0, 0.3854, 0.009849],
    [15.0, 0.3764, 0.01102],
    [20.0, 0.3695, 0.01355],
    [30.0, 0.357, 0.01863],
    [40.0, 0.3458, 0.02315],
    [50.0, 0.3355, 0.02709],
    [60.0, 0.326, 0.03053],
    [80.0, 0.3091, 0.0362],
    [100.0, 0.2944, 0.04063],
    [150.0, 0.2651, 0.04813],
    [200.0, 0.2429, 0.05254],
    [300.0, 0.2112, 0.05695],
    [400.0, 0.1893, 0.0586],
    [500.0, 0.1729, 0.059],
    [600.0, 0.1599, 0.05875],
    [800.0, 0.1405, 0.05739],
    [1000.0, 0.1263, 0.05556],
    [1250.0, 0.1129, 0.05311],
    [1500.0, 0.1027, 0.05075],
    [2000.0, 0.08769, 0.0465],
    [3000.0, 0.06921, 0.03992],
    [4000.0, 0.05806, 0.03523],
    [5000.0, 0.05049, 0.03174],
    [6000.0, 0.04498, 0.02905],
    [8000.0, 0.03746, 0.02515],
    [10000.0, 0.03254, 0.02247],
    [15000.0, 0.02539, 0.01837],
    [20000.0, 0.02153, 0.01606],
];

// Beryllium (Z = 4): physics.nist.gov/PhysRefData/XrayMassCoef/ElemTab/z04.html
static BE_NIST: [[f64; 3]; 36] = [
    [1.0, 604.1, 603.5],
    [1.5, 179.7, 179.1],
    [2.0, 74.69, 74.22],
    [3.0, 21.27, 20.9],
    [4.0, 8.685, 8.367],
    [5.0, 4.369, 4.081],
    [6.0, 2.527, 2.26],
    [8.0, 1.124, 0.8839],
    [10.0, 0.6466, 0.4255],
    [15.0, 0.307, 0.1143],
    [20.0, 0.2251, 0.0478],
    [30.0, 0.1792, 0.01898],
    [40.0, 0.164, 0.01438],
    [50.0, 0.1554, 0.01401],
    [60.0, 0.1493, 0.01468],
    [80.0, 0.1401, 0.01658],
    [100.0, 0.1328, 0.01836],
    [150.0, 0.119, 0.02157],
    [200.0, 0.1089, 0.02353],
    [300.0, 0.09463, 0.02548],
    [400.0, 0.08471, 0.0262],
    [500.0, 0.07739, 0.02639],
    [600.0, 0.07155, 0.02627],
    [800.0, 0.06286, 0.02565],
    [1000.0, 0.05652, 0.02483],
    [1250.0, 0.05054, 0.02373],
    [1500.0, 0.04597, 0.02268],
    [2000.0, 0.03938, 0.02083],
    [3000.0, 0.03138, 0.01806],
    [4000.0, 0.02664, 0.01617],
    [5000.0, 0.02347, 0.01479],
    [6000.0, 0.02121, 0.01377],
    [8000.0, 0.01819, 0.01233],
    [10000.0, 0.01627, 0.01138],
    [15000.0, 0.01361, 0.01001],
    [20000.0, 0.01227, 0.009294],
];

// Boron (Z = 5): physics.nist.gov/PhysRefData/XrayMassCoef/ElemTab/z05.html
static B_NIST: [[f64; 3]; 36] = [
    [1.0, 1229.0, 1228.0],
    [1.5, 376.6, 375.9],
    [2.0, 159.7, 159.1],
    [3.0, 46.67, 46.17],
    [4.0, 19.27, 18.86],
    [5.0, 9.683, 9.332],
    [6.0, 5.538, 5.223],
    [8.0, 2.346, 2.072],
    [10.0, 1.255, 1.006],
    [15.0, 0.4827, 0.2698],
    [20.0, 0.3014, 0.1084],
    [30.0, 0.2063, 0.03506],
    [40.0, 0.1793, 0.02084],
    [50.0, 0.1665, 0.01737],
    [60.0, 0.1583, 0.0168],
    [80.0, 0.1472, 0.01785],
    [100.0, 0.1391, 0.0194],
    [150.0, 0.1243, 0.02255],
    [200.0, 0.1136, 0.02453],
    [300.0, 0.09862, 0.02654],
    [400.0, 0.08834, 0.02731],
    [500.0, 0.08065, 0.02749],
    [600.0, 0.0746, 0.02737],
    [800.0, 0.06549, 0.02671],
    [1000.0, 0.0589, 0.02586],
    [1250.0, 0.05266, 0.02472],
    [1500.0, 0.04791, 0.02362],
    [2000.0, 0.04108, 0.02171],
    [3000.0, 0.03284, 0.01889],
    [4000.0, 0.02798, 0.01698],
    [5000.0, 0.02476, 0.01562],
    [6000.0, 0.02248, 0.01461],
    [8000.0, 0.01945, 0.01322],
    [10000.0, 0.01755, 0.01232],
    [15000.0, 0.01495, 0.01104],
    [20000.0, 0.01368, 0.01039],
];

// Carbon, Graphite (Z = 6): physics.nist.gov/PhysRefData/XrayMassCoef/ElemTab/z06.html
static C_NIST: [[f64; 3]; 36] = [
    [1.0, 2211.0, 2209.0],
    [1.5, 700.2, 699.0],
    [2.0, 302.6, 301.6],
    [3.0, 90.33, 89.63],
    [4.0, 37.78, 37.23],
    [5.0, 19.12, 18.66],
    [6.0, 10.95, 10.54],
    [8.0, 4.576, 4.242],
    [10.0, 2.373, 2.078],
    [15.0, 0.8071, 0.5627],
    [20.0, 0.442, 0.2238],
    [30.0, 0.2562, 0.06614],
    [40.0, 0.2076, 0.03343],
    [50.0, 0.1871, 0.02397],
    [60.0, 0.1753, 0.02098],
    [80.0, 0.161, 0.02037],
    [100.0, 0.1514, 0.02147],
    [150.0, 0.1347, 0.02449],
    [200.0, 0.1229, 0.02655],
    [300.0, 0.1066, 0.0287],
    [400.0, 0.09546, 0.0295],
    [500.0, 0.08715, 0.02969],
    [600.0, 0.08058, 0.02956],
    [800.0, 0.07076, 0.02885],
    [1000.0, 0.06361, 0.02792],
    [1250.0, 0.0569, 0.02669],
    [1500.0, 0.05179, 0.02551],
    [2000.0, 0.04442, 0.02345],
    [3000.0, 0.03562, 0.02048],
    [4000.0, 0.03047, 0.01849],
    [5000.0, 0.02708, 0.0171],
    [6000.0, 0.02469, 0.01607],
    [8000.0, 0.02154, 0.01468],
    [10000.0, 0.01959, 0.0138],
    [15000.0, 0.01698, 0.01258],
    [20000.0, 0.01575, 0.01198],
];

// Nitrogen (Z = 7): physics.nist.gov/PhysRefData/XrayMassCoef/ElemTab/z07.html
static N_NIST: [[f64; 3]; 36] = [
    [1.0, 3311.0, 3306.0],
    [1.5, 1083.0, 1080.0],
    [2.0, 476.9, 475.5],
    [3.0, 145.6, 144.7],
    [4.0, 61.66, 60.94],
    [5.0, 31.44, 30.86],
    [6.0, 18.09, 17.59],
    [8.0, 7.562, 7.17],
    [10.0, 3.879, 3.545],
    [15.0, 1.236, 0.9715],
    [20.0, 0.6178, 0.3867],
    [30.0, 0.3066, 0.1099],
    [40.0, 0.2288, 0.05051],
    [50.0, 0.198, 0.03217],
    [60.0, 0.1817, 0.02548],
    [80.0, 0.1639, 0.02211],
    [100.0, 0.1529, 0.02231],
    [150.0, 0.1353, 0.02472],
    [200.0, 0.1233, 0.02665],
    [300.0, 0.1068, 0.02873],
    [400.0, 0.09557, 0.02952],
    [500.0, 0.08719, 0.02969],
    [600.0, 0.08063, 0.02956],
    [800.0, 0.07081, 0.02886],
    [1000.0, 0.06364, 0.02792],
    [1250.0, 0.05693, 0.02669],
    [1500.0, 0.0518, 0.0255],
    [2000.0, 0.0445, 0.02347],
    [3000.0, 0.03579, 0.02057],
    [4000.0, 0.03073, 0.01867],
    [5000.0, 0.02742, 0.01734],
    [6000.0, 0.02511, 0.01639],
    [8000.0, 0.02209, 0.01512],
    [10000.0, 0.02024, 0.01434],
    [15000.0, 0.01782, 0.01332],
    [20000.0, 0.01673, 0.01285],
];

// Oxygen (Z = 8): physics.nist.gov/PhysRefData/XrayMassCoef/ElemTab/z08.html
static O_NIST: [[f64; 3]; 36] = [
    [1.0, 4590.0, 4576.0],
    [1.5, 1549.0, 1545.0],
    [2.0, 694.9, 692.6],
    [3.0, 217.1, 215.8],
    [4.0, 93.15, 92.21],
    [5.0, 47.9, 47.15],
    [6.0, 27.7, 27.08],
    [8.0, 11.63, 11.16],
    [10.0, 5.952, 5.565],
    [15.0, 1.836, 1.545],
    [20.0, 0.8651, 0.6179],
    [30.0, 0.3779, 0.1729],
    [40.0, 0.2585, 0.0753],
    [50.0, 0.2132, 0.04414],
    [60.0, 0.1907, 0.03207],
    [80.0, 0.1678, 0.02468],
    [100.0, 0.1551, 0.02355],
    [150.0, 0.1361, 0.02506],
    [200.0, 0.1237, 0.02679],
    [300.0, 0.107, 0.02877],
    [400.0, 0.09566, 0.02953],
    [500.0, 0.08729, 0.02971],
    [600.0, 0.0807, 0.02957],
    [800.0, 0.07087, 0.02887],
    [1000.0, 0.06372, 0.02794],
    [1250.0, 0.05697, 0.02669],
    [1500.0, 0.05185, 0.02551],
    [2000.0, 0.04459, 0.0235],
    [3000.0, 0.03597, 0.02066],
    [4000.0, 0.031, 0.01882],
    [5000.0, 0.02777, 0.01757],
    [6000.0, 0.02552, 0.01668],
    [8000.0, 0.02263, 0.01553],
    [10000.0, 0.02089, 0.01483],
    [15000.0, 0.01866, 0.01396],
    [20000.0, 0.0177, 0.0136],
];

// Aluminum (Z = 13): physics.nist.gov/PhysRefData/XrayMassCoef/ElemTab/z13.html
static AL_NIST: [[f64; 3]; 38] = [
    [1.0, 1185.0, 1183.0],
    [1.5, 402.2, 400.1],
    [1.5596, 362.1, 360.0],
    [1.5596, 3957.0, 3829.0], // K edge
    [2.0, 2263.0, 2204.0],
    [3.0, 788.0, 773.2],
    [4.0, 360.5, 354.5],
    [5.0, 193.4, 190.2],
    [6.0, 115.3, 113.3],
    [8.0, 50.33, 49.18],
    [10.0, 26.23, 25.43],
    [15.0, 7.955, 7.487],
    [20.0, 3.441, 3.094],
    [30.0, 1.128, 0.8778],
    [40.0, 0.5685, 0.3601],
    [50.0, 0.3681, 0.184],
    [60.0, 0.2778, 0.1099],
    [80.0, 0.2018, 0.05511],
    [100.0, 0.1704, 0.03794],
    [150.0, 0.1378, 0.02827],
    [200.0, 0.1223, 0.02745],
    [300.0, 0.1042, 0.02816],
    [400.0, 0.09276, 0.02862],
    [500.0, 0.08445, 0.02868],
    [600.0, 0.07802, 0.02851],
    [800.0, 0.06841, 0.02778],
    [1000.0, 0.06146, 0.02686],
    [1250.0, 0.05496, 0.02565],
    [1500.0, 0.05006, 0.02451],
    [2000.0, 0.04324, 0.02266],
    [3000.0, 0.03541, 0.02024],
    [4000.0, 0.03106, 0.01882],
    [5000.0, 0.02836, 0.01795],
    [6000.0, 0.02655, 0.01739],
    [8000.0, 0.02437, 0.01678],
    [10000.0, 0.02318, 0.0165],
    [15000.0, 0.02195, 0.01631],
    [20000.0, 0.02168, 0.01633],
];

// Silicon (Z = 14): physics.nist.gov/PhysRefData/XrayMassCoef/ElemTab/z14.html
static SI_NIST: [[f64; 3]; 38] = [
    [1.0, 1570.0, 1567.0],
    [1.5, 535.5, 533.1],
    [1.8389, 309.2, 307.0],
    [1.8389, 3192.0, 3059.0], // K edge
    [2.0, 2777.0, 2669.0],
    [3.0, 978.4, 951.6],
    [4.0, 452.9, 442.7],
    [5.0, 245.0, 240.0],
    [6.0, 147.0, 143.9],
    [8.0, 64.68, 63.13],
    [10.0, 33.89, 32.89],
    [15.0, 10.34, 9.794],
    [20.0, 4.464, 4.076],
    [30.0, 1.436, 1.164],
    [40.0, 0.7012, 0.4782],
    [50.0, 0.4385, 0.243],
    [60.0, 0.3207, 0.1434],
    [80.0, 0.2228, 0.06896],
    [100.0, 0.1835, 0.04513],
    [150.0, 0.1448, 0.03086],
    [200.0, 0.1275, 0.02905],
    [300.0, 0.1082, 0.02932],
    [400.0, 0.09614, 0.02968],
    [500.0, 0.08748, 0.02971],
    [600.0, 0.08077, 0.02951],
    [800.0, 0.07082, 0.02875],
    [1000.0, 0.06361, 0.02778],
    [1250.0, 0.05688, 0.02652],
    [1500.0, 0.05183, 0.02535],
    [2000.0, 0.0448, 0.02345],
    [3000.0, 0.03678, 0.02101],
    [4000.0, 0.0324, 0.01963],
    [5000.0, 0.02967, 0.01878],
    [6000.0, 0.02788, 0.01827],
    [8000.0, 0.02574, 0.01773],
    [10000.0, 0.02462, 0.01753],
    [15000.0, 0.02352, 0.01746],
    [20000.0, 0.02338, 0.01757],
];

// Titanium (Z = 22): physics.nist.gov/PhysRefData/XrayMassCoef/ElemTab/z22.html
static TI_NIST: [[f64; 3]; 38] = [
    [1.0, 5869.0, 5860.0],
    [1.5, 2096.0, 2091.0],
    [2.0, 986.0, 982.4],
    [3.0, 332.3, 329.5],
    [4.0, 151.7, 149.4],
    [4.9664, 83.8, 81.88],
    [4.9664, 687.8, 568.4], // K edge
    [5.0, 683.8, 565.7],
    [6.0, 432.3, 369.1],
    [8.0, 202.3, 179.3],
    [10.0, 110.7, 100.1],
    [15.0, 35.87, 33.11],
    [20.0, 15.85, 14.65],
    [30.0, 4.972, 4.488],
    [40.0, 2.214, 1.904],
    [50.0, 1.213, 0.9737],
    [60.0, 0.7661, 0.5634],
    [80.0, 0.4052, 0.2422],
    [100.0, 0.2721, 0.1312],
    [150.0, 0.1649, 0.05393],
    [200.0, 0.1314, 0.03726],
    [300.0, 0.1043, 0.03007],
    [400.0, 0.09081, 0.02864],
    [500.0, 0.08191, 0.02804],
    [600.0, 0.07529, 0.02756],
    [800.0, 0.06572, 0.02661],
    [1000.0, 0.05891, 0.02561],
    [1250.0, 0.05263, 0.02439],
    [1500.0, 0.04801, 0.0233],
    [2000.0, 0.0418, 0.02166],
    [3000.0, 0.03512, 0.01989],
    [4000.0, 0.03173, 0.01913],
    [5000.0, 0.02982, 0.01884],
    [6000.0, 0.02868, 0.01879],
    [8000.0, 0.02759, 0.01899],
    [10000.0, 0.02727, 0.01933],
    [15000.0, 0.02762, 0.02013],
    [20000.0, 0.02844, 0.02067],
];

// Chromium (Z = 24): physics.nist.gov/PhysRefData/XrayMassCoef/ElemTab/z24.html
static CR_NIST: [[f64; 3]; 38] = [
    [1.0, 7405.0, 7388.0],
    [1.5, 2694.0, 2687.0],
    [2.0, 1277.0, 1272.0],
    [3.0, 433.9, 430.5],
    [4.0, 198.8, 196.1],
    [5.0, 108.0, 105.7],
    [5.9892, 65.74, 63.83],
    [5.9892, 597.7, 402.7], // K edge
    [6.0, 516.0, 402.7],
    [8.0, 251.3, 208.7],
    [10.0, 138.6, 119.3],
    [15.0, 45.71, 40.93],
    [20.0, 20.38, 18.46],
    [30.0, 6.434, 5.78],
    [40.0, 2.856, 2.482],
    [50.0, 1.55, 1.278],
    [60.0, 0.9639, 0.742],
    [80.0, 0.4905, 0.3182],
    [100.0, 0.3166, 0.1701],
    [150.0, 0.1788, 0.06536],
    [200.0, 0.1378, 0.04211],
    [300.0, 0.1067, 0.0316],
    [400.0, 0.09213, 0.02938],
    [500.0, 0.08281, 0.02849],
    [600.0, 0.07598, 0.02788],
    [800.0, 0.0662, 0.0268],
    [1000.0, 0.0593, 0.02576],
    [1250.0, 0.05295, 0.0245],
    [1500.0, 0.04832, 0.0234],
    [2000.0, 0.04213, 0.02178],
    [3000.0, 0.03559, 0.02011],
    [4000.0, 0.03235, 0.01947],
    [5000.0, 0.03057, 0.01929],
    [6000.0, 0.02956, 0.01933],
    [8000.0, 0.02869, 0.0197],
    [10000.0, 0.02855, 0.02016],
    [15000.0, 0.0292, 0.02112],
    [20000.0, 0.03026, 0.02174],
];

// Nickel (Z = 28): physics.nist.gov/PhysRefData/XrayMassCoef/ElemTab/z28.html
static NI_NIST: [[f64; 3]; 41] = [
    [1.0, 9855.0, 9797.0],
    [1.00404, 9753.0, 9697.0],
    [1.0081, 9654.0, 9598.0],
    [1.0081, 1.099e4, 1.093e4], // L1 edge
    [1.5, 4234.0, 4214.0],
    [2.0, 2049.0, 2039.0],
    [3.0, 709.4, 704.2],
    [4.0, 328.2, 324.4],
    [5.0, 179.3, 176.1],
    [6.0, 109.0, 106.4],
    [8.0, 49.52, 47.58],
    [8.3328, 44.28, 42.42],
    [8.3328, 329.4, 224.0], // K edge
    [10.0, 209.0, 152.4],
    [15.0, 70.81, 57.34],
    [20.0, 32.2, 27.22],
    [30.0, 10.34, 8.982],
    [40.0, 4.6, 3.967],
    [50.0, 2.474, 2.078],
    [60.0, 1.512, 1.219],
    [80.0, 0.7306, 0.5259],
    [100.0, 0.444, 0.2781],
    [150.0, 0.2208, 0.09812],
    [200.0, 0.1582, 0.05649],
    [300.0, 0.1154, 0.03659],
    [400.0, 0.09765, 0.03209],
    [500.0, 0.08698, 0.03036],
    [600.0, 0.07944, 0.02937],
    [800.0, 0.06891, 0.02795],
    [1000.0, 0.0616, 0.02674],
    [1250.0, 0.05494, 0.02536],
    [1500.0, 0.05015, 0.0242],
    [2000.0, 0.04387, 0.02257],
    [3000.0, 0.03745, 0.02107],
    [4000.0, 0.03444, 0.02066],
    [5000.0, 0.03289, 0.0207],
    [6000.0, 0.0321, 0.02094],
    [8000.0, 0.03164, 0.02163],
    [10000.0, 0.03185, 0.02234],
    [15000.0, 0.0332, 0.02368],
    [20000.0, 0.03476, 0.02446],
];

// Copper (Z = 29): physics.nist.gov/PhysRefData/XrayMassCoef/ElemTab/z29.html
static CU_NIST: [[f64; 3]; 41] = [
    [1.0, 1.057e4, 1.049e4],
    [1.04695, 9307.0, 9241.0],
    [1.0961, 8242.0, 8186.0],
    [1.0961, 9347.0, 9282.0], // L1 edge
    [1.5, 4418.0, 4393.0],
    [2.0, 2154.0, 2142.0],
    [3.0, 748.8, 743.0],
    [4.0, 347.3, 343.2],
    [5.0, 189.9, 186.6],
    [6.0, 115.6, 112.8],
    [8.0, 52.55, 50.54],
    [8.9789, 38.29, 36.52],
    [8.9789, 278.4, 182.4], // K edge
    [10.0, 215.9, 148.4],
    [15.0, 74.05, 57.88],
    [20.0, 33.79, 27.88],
    [30.0, 10.92, 9.349],
    [40.0, 4.862, 4.163],
    [50.0, 2.613, 2.192],
    [60.0, 1.593, 1.29],
    [80.0, 0.763, 0.5581],
    [100.0, 0.4584, 0.2949],
    [150.0, 0.2217, 0.1027],
    [200.0, 0.1559, 0.05781],
    [300.0, 0.1119, 0.03617],
    [400.0, 0.09413, 0.03121],
    [500.0, 0.08362, 0.02933],
    [600.0, 0.07625, 0.02826],
    [800.0, 0.06605, 0.02681],
    [1000.0, 0.05901, 0.02562],
    [1250.0, 0.05261, 0.02428],
    [1500.0, 0.04803, 0.02316],
    [2000.0, 0.04205, 0.0216],
    [3000.0, 0.03599, 0.02023],
    [4000.0, 0.03318, 0.01989],
    [5000.0, 0.03177, 0.01998],
    [6000.0, 0.03108, 0.02027],
    [8000.0, 0.03074, 0.021],
    [10000.0, 0.03103, 0.02174],
    [15000.0, 0.03247, 0.02309],
    [20000.0, 0.03408, 0.02387],
];

// Molybdenum (Z = 42): physics.nist.gov/PhysRefData/XrayMassCoef/ElemTab/z42.html
static MO_NIST: [[f64; 3]; 46] = [
    [1.0, 4942.0, 4935.0],
    [1.5, 1925.0, 1918.0],
    [2.0, 959.3, 953.1],
    [2.5202, 541.5, 535.8],
    [2.5202, 1979.0, 1924.0], // L3 edge
    [2.57212, 1854.0, 1802.0],
    [2.6251, 1750.0, 1703.0],
    [2.6251, 2433.0, 2360.0], // L2 edge
    [2.74267, 2183.0, 2119.0],
    [2.8655, 1961.0, 1906.0],
    [2.8655, 2243.0, 2179.0], // L1 edge
    [3.0, 2011.0, 1956.0],
    [4.0, 970.3, 947.5],
    [5.0, 545.0, 532.8],
    [6.0, 337.3, 329.5],
    [8.0, 156.5, 152.2],
    [10.0, 85.76, 82.75],
    [15.0, 28.54, 26.84],
    [19.9995, 13.08, 11.93],
    [19.9995, 80.55, 32.93], // K edge
    [20.0, 80.54, 33.36],
    [30.0, 28.1, 16.64],
    [40.0, 12.94, 8.757],
    [50.0, 7.037, 5.074],
    [60.0, 4.274, 3.178],
    [80.0, 1.962, 1.477],
    [100.0, 1.096, 0.8042],
    [150.0, 0.4208, 0.2693],
    [200.0, 0.2423, 0.1316],
    [300.0, 0.1379, 0.05919],
    [400.0, 0.1047, 0.04117],
    [500.0, 0.08848, 0.03437],
    [600.0, 0.07851, 0.03104],
    [800.0, 0.06619, 0.02764],
    [1000.0, 0.05837, 0.02567],
    [1250.0, 0.05167, 0.0239],
    [1500.0, 0.04713, 0.02263],
    [2000.0, 0.04163, 0.02118],
    [3000.0, 0.03675, 0.02046],
    [4000.0, 0.03496, 0.02084],
    [5000.0, 0.03439, 0.02153],
    [6000.0, 0.0344, 0.02231],
    [8000.0, 0.03523, 0.02382],
    [10000.0, 0.0365, 0.02513],
    [15000.0, 0.03978, 0.02731],
    [20000.0, 0.04264, 0.0284],
];

// Tungsten (Z = 74): physics.nist.gov/PhysRefData/XrayMassCoef/ElemTab/z74.html
static W_NIST: [[f64; 3]; 59] = [
    [1.0, 3683.0, 3671.0],
    [1.5, 1643.0, 1632.0],
    [1.8092, 1108.0, 1097.0],
    [1.8092, 1327.0, 1311.0], // M5 edge
    [1.84014, 1911.0, 1883.0],
    [1.8716, 2901.0, 2853.0],
    [1.8716, 3170.0, 3116.0], // M4 edge
    [2.0, 3922.0, 3853.0],
    [2.281, 2828.0, 2781.0],
    [2.281, 3279.0, 3226.0], // M3 edge
    [2.4235, 2833.0, 2786.0],
    [2.5749, 2445.0, 2407.0],
    [2.5749, 2599.0, 2558.0], // M2 edge
    [2.69447, 2339.0, 2301.0],
    [2.8196, 2104.0, 2071.0],
    [2.8196, 2194.0, 2160.0], // M1 edge
    [3.0, 1902.0, 1873.0],
    [4.0, 956.4, 940.5],
    [5.0, 553.4, 542.3],
    [6.0, 351.4, 342.8],
    [8.0, 170.5, 164.3],
    [10.0, 96.91, 92.04],
    [10.2068, 92.01, 87.24],
    [10.2068, 233.4, 196.6], // L3 edge
    [10.8548, 198.3, 168.4],
    [11.544, 168.9, 144.4],
    [11.544, 231.2, 188.9], // L2 edge
    [11.8186, 226.8, 179.7],
    [12.0998, 206.5, 169.9],
    [12.0998, 238.2, 194.8], // L1 edge
    [15.0, 138.9, 117.2],
    [20.0, 65.73, 56.97],
    [30.0, 22.73, 19.91],
    [40.0, 10.67, 9.24],
    [50.0, 5.949, 5.05],
    [60.0, 3.713, 3.07],
    [69.525, 2.552, 2.049],
    [69.525, 11.23, 3.212], // K edge
    [80.0, 7.81, 2.879],
    [100.0, 4.438, 2.1],
    [150.0, 1.581, 0.9378],
    [200.0, 0.7844, 0.4913],
    [300.0, 0.3238, 0.1973],
    [400.0, 0.1925, 0.11],
    [500.0, 0.1378, 0.0744],
    [600.0, 0.1093, 0.05673],
    [800.0, 0.08066, 0.04028],
    [1000.0, 0.06618, 0.03276],
    [1250.0, 0.05577, 0.02761],
    [1500.0, 0.05, 0.02484],
    [2000.0, 0.04433, 0.02256],
    [3000.0, 0.04075, 0.02236],
    [4000.0, 0.04038, 0.02363],
    [5000.0, 0.04103, 0.0251],
    [6000.0, 0.0421, 0.02649],
    [8000.0, 0.04472, 0.02886],
    [10000.0, 0.04747, 0.03072],
    [15000.0, 0.05384, 0.0336],
    [20000.0, 0.05893, 0.03475],
];

// Gold (Z = 79): physics.nist.gov/PhysRefData/XrayMassCoef/ElemTab/z79.html
static AU_NIST: [[f64; 3]; 59] = [
    [1.0, 4652.0, 4639.0],
    [1.5, 2089.0, 2076.0],
    [2.0, 1137.0, 1125.0],
    [2.2057, 918.7, 907.4],
    [2.2057, 997.1, 983.6], // M5 edge
    [2.24799, 1386.0, 1360.0],
    [2.2911, 2258.0, 2208.0],
    [2.2911, 2389.0, 2336.0], // M4 edge
    [2.50689, 2380.0, 2325.0],
    [2.743, 2203.0, 2154.0],
    [2.743, 2541.0, 2484.0], // M3 edge
    [3.0, 2049.0, 2005.0],
    [3.1478, 1822.0, 1783.0],
    [3.1478, 1933.0, 1892.0], // M2 edge
    [3.28343, 1748.0, 1710.0],
    [3.4249, 1585.0, 1552.0],
    [3.4249, 1652.0, 1618.0], // M1 edge
    [4.0, 1144.0, 1120.0],
    [5.0, 666.1, 651.2],
    [6.0, 425.3, 414.3],
    [8.0, 207.2, 199.9],
    [10.0, 118.1, 112.6],
    [11.9187, 75.82, 71.29],
    [11.9187, 187.0, 152.1], // L3 edge
    [12.794, 154.6, 127.2],
    [13.7336, 128.3, 106.6],
    [13.7336, 176.4, 137.9], // L2 edge
    [14.0398, 176.6, 131.7],
    [14.3528, 158.8, 125.2],
    [14.3528, 183.0, 143.2], // L1 edge
    [15.0, 163.7, 129.4],
    [20.0, 78.83, 65.22],
    [30.0, 27.52, 23.49],
    [40.0, 12.98, 11.09],
    [50.0, 7.256, 6.124],
    [60.0, 4.528, 3.751],
    [80.0, 2.185, 1.72],
    [80.7249, 2.137, 1.678],
    [80.7249, 8.904, 2.512], // K edge
    [100.0, 5.158, 2.074],
    [150.0, 1.86, 1.026],
    [200.0, 0.9214, 0.5563],
    [300.0, 0.3744, 0.2289],
    [400.0, 0.218, 0.1274],
    [500.0, 0.153, 0.08523],
    [600.0, 0.1194, 0.06409],
    [800.0, 0.08603, 0.04427],
    [1000.0, 0.06953, 0.03525],
    [1250.0, 0.05794, 0.02915],
    [1500.0, 0.05167, 0.02593],
    [2000.0, 0.0457, 0.02333],
    [3000.0, 0.04201, 0.02302],
    [4000.0, 0.04166, 0.02432],
    [5000.0, 0.04239, 0.02582],
    [6000.0, 0.04355, 0.02725],
    [8000.0, 0.04633, 0.02968],
    [10000.0, 0.04926, 0.03159],
    [15000.0, 0.05598, 0.0345],
    [20000.0, 0.06136, 0.03565],
];

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_nist_nodes_reproduced_exactly() {
        for el in ELEMENTS {
            for (k, row) in el.nist.iter().enumerate() {
                // Skip the below-edge member of a duplicated edge pair (an
                // energy exactly on an edge returns the above-edge row).
                if el.nist.get(k + 1).is_some_and(|next| next[0] == row[0]) {
                    continue;
                }
                assert_relative_eq!(el.mu_over_rho(row[0]), row[1], max_relative = 1e-12);
                assert_relative_eq!(el.mu_en_over_rho(row[0]), row[2], max_relative = 1e-12);
            }
        }
    }

    #[test]
    fn test_nist_fixture_values() {
        // Spot values read off the NIST pages (Hubbell & Seltzer).
        let au = element("Au").unwrap();
        assert_relative_eq!(au.mu_over_rho(8.0), 207.2, max_relative = 1e-9);
        assert_relative_eq!(au.mu_en_over_rho(8.0), 199.9, max_relative = 1e-9);
        assert_relative_eq!(au.mu_over_rho(10.0), 118.1, max_relative = 1e-9);
        let c = element("C").unwrap();
        assert_relative_eq!(c.mu_over_rho(8.0), 4.576, max_relative = 1e-9);
        assert_relative_eq!(c.mu_en_over_rho(20.0), 0.2238, max_relative = 1e-9);
        let ti = element("Ti").unwrap();
        assert_relative_eq!(ti.mu_over_rho(5.0), 683.8, max_relative = 1e-9);
    }

    #[test]
    fn test_pmma_matches_nist_compound_table() {
        // NIST's own PMMA ("Polymethyl Methacrylate") compound table,
        // ComTab/pmma.html: (E keV, mu/rho, mu_en/rho). Our mixture rule over
        // the element tables must reproduce it to its printed precision.
        let pmma = Compound::pmma();
        for (e, mu, mu_en) in [
            (1.0, 2794.0, 2788.0),
            (3.0, 123.6, 122.8),
            (8.0, 6.494, 6.114),
            (10.0, 3.357, 3.026),
            (15.0, 1.101, 0.8324),
            (20.0, 0.5714, 0.3328),
            (50.0, 0.2074, 0.03067),
        ] {
            assert_relative_eq!(pmma.mu_over_rho(e), mu, max_relative = 2e-3);
            assert_relative_eq!(pmma.mu_en_over_rho(e), mu_en, max_relative = 2e-3);
        }
    }

    #[test]
    fn test_edges_resolved() {
        let au = element("Au").unwrap();
        // Au L3 edge at 11.9187 keV: 75.82 below, 187.0 above.
        assert!((au.mu_over_rho(11.918) - 75.82).abs() < 0.1);
        assert_relative_eq!(au.mu_over_rho(11.9187), 187.0, max_relative = 1e-9);
        assert!(au.mu_over_rho(11.92) > 180.0);
        // Ti K edge at 4.9664 keV: ~8x jump.
        let ti = element("Ti").unwrap();
        assert!(ti.mu_over_rho(4.96) < 90.0 && ti.mu_over_rho(4.97) > 680.0);
    }

    #[test]
    fn test_log_log_between_nodes() {
        let c = element("C").unwrap();
        // 9 keV sits between the 8 keV (4.576) and 10 keV (2.373) nodes.
        let t = (9.0_f64.ln() - 8.0_f64.ln()) / (10.0_f64.ln() - 8.0_f64.ln());
        let expected = (4.576_f64.ln() + t * (2.373_f64.ln() - 4.576_f64.ln())).exp();
        assert_relative_eq!(c.mu_over_rho(9.0), expected, max_relative = 1e-12);
    }

    #[test]
    fn test_mu_en_never_exceeds_mu() {
        for el in ELEMENTS {
            let mut e = MIN_ENERGY_KEV;
            while e < MAX_ENERGY_KEV {
                let (mu, mu_en) = (el.mu_over_rho(e), el.mu_en_over_rho(e));
                assert!(
                    mu.is_finite() && mu > 0.0,
                    "{} at {e} keV: mu {mu}",
                    el.symbol
                );
                assert!(mu_en <= mu * (1.0 + 1e-12), "{} at {e} keV", el.symbol);
                e *= 1.07;
            }
        }
    }

    #[test]
    fn test_compton_dominates_hydrogen_but_not_gold() {
        // Hydrogen at 20 keV: nearly all attenuation is Compton scattering,
        // which deposits only a few percent locally.
        let h = element("H").unwrap();
        assert!(h.mu_en_over_rho(20.0) / h.mu_over_rho(20.0) < 0.05);
        // Gold at 8 keV: photoabsorption dominates, mu_en/mu ~ 0.96.
        let au = element("Au").unwrap();
        assert!(au.mu_en_over_rho(8.0) / au.mu_over_rho(8.0) > 0.95);
        // PMMA: mu_en/mu falls from 0.94 at 8 keV to 0.58 at 20 keV.
        let pmma = Compound::pmma();
        let r8 = pmma.mu_en_over_rho(8.0) / pmma.mu_over_rho(8.0);
        let r20 = pmma.mu_en_over_rho(20.0) / pmma.mu_over_rho(20.0);
        assert!(
            (0.93..0.95).contains(&r8) && (0.57..0.59).contains(&r20),
            "{r8} {r20}"
        );
    }

    #[test]
    fn test_henke_branch_below_1kev() {
        // mu_a/rho = 2 r_e lambda f2 N_A / A from the Henke table, computed
        // independently in numpy (log-log f2 interpolation): C at 500 eV
        // = 13695.6 cm^2/g; Au at 800 eV = 7719.8; Si at 300 eV = 32156.7.
        let c = element("C").unwrap();
        assert_relative_eq!(c.mu_over_rho(0.5), 13695.6, max_relative = 1e-4);
        assert_relative_eq!(c.mu_en_over_rho(0.5), 13695.6, max_relative = 1e-4);
        assert_relative_eq!(
            element("Au").unwrap().mu_over_rho(0.8),
            7719.8,
            max_relative = 1e-4
        );
        assert_relative_eq!(
            element("Si").unwrap().mu_over_rho(0.3),
            32156.7,
            max_relative = 1e-4
        );
        // Junction at 1 keV: Henke/NIST ratio within the documented bounds.
        for el in ELEMENTS {
            let r = el.mu_over_rho(0.999_999) / el.mu_over_rho(1.0);
            assert!(
                (0.93..1.17).contains(&r),
                "{}: junction ratio {r}",
                el.symbol
            );
        }
    }

    #[test]
    fn test_clamp_outside_range() {
        let c = element("C").unwrap();
        assert_eq!(c.mu_over_rho(0.001), c.mu_over_rho(MIN_ENERGY_KEV));
        assert_eq!(c.mu_over_rho(1e6), c.mu_over_rho(MAX_ENERGY_KEV));
        // ... and the validator rejects both.
        assert!(check_energy_kev(0.001).is_err());
        assert!(check_energy_kev(1e6).is_err());
        assert!(check_energy_kev(f64::NAN).is_err());
        assert!(check_energy_kev(MIN_ENERGY_KEV).is_ok());
        assert!(check_energy_kev(MAX_ENERGY_KEV).is_ok());
    }

    #[test]
    fn test_pmma_anchor_8kev() {
        let mu = Compound::pmma().mu_over_rho(8.0);
        assert!(
            (6.3..=6.8).contains(&mu),
            "PMMA mu/rho at 8 keV should be ~6.5, got {mu}"
        );
    }

    #[test]
    fn test_pmma_anchor_3kev() {
        let mu = Compound::pmma().mu_over_rho(3.0);
        assert!(
            (110.0..=130.0).contains(&mu),
            "PMMA mu/rho at 3 keV should be ~124, got {mu}"
        );
    }

    #[test]
    fn test_element_anchors() {
        // Carbon and gold at 8 keV (NIST: 4.576 and 207.2 cm^2/g). The
        // pre-2026-09 hand-transcribed table had Au at 111 - a factor ~1.9
        // too transparent.
        assert!((4.5..=4.7).contains(&element("C").unwrap().mu_over_rho(8.0)));
        assert!((200.0..=215.0).contains(&element("Au").unwrap().mu_over_rho(8.0)));
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
    fn test_from_formula_matches_pmma_preset() {
        let f = Compound::from_formula("PMMA", "C5H8O2", 1.19).unwrap();
        for ((s1, w1), (s2, w2)) in f.components().iter().zip(Compound::pmma().components()) {
            assert_eq!(s1, s2);
            assert!((w1 - w2).abs() < 1e-4, "{s1}: {w1} vs {w2}");
        }
        // Kapton C22H10N2O5: w_C = 22*12.011/382.33 = 0.69113 (hand calc).
        let k = Compound::kapton();
        assert!((k.components()[0].1 - 0.69113).abs() < 1e-4);
        // Elements without attenuation data are rejected.
        assert!(Compound::from_formula("x", "La", 6.0).is_err());
    }

    #[test]
    fn test_from_spec_presets_and_formulas() {
        assert_eq!(Compound::from_spec("Be").unwrap().density_g_cm3, 1.848);
        assert_eq!(Compound::from_spec("kapton").unwrap().name, "Kapton");
        let au = Compound::from_spec("Au@17.5").unwrap();
        assert_eq!(au.density_g_cm3, 17.5);
        assert_eq!(au.components(), &[("Au", 1.0)]);
        let k = Compound::from_spec("C22H10N2O5@1.42").unwrap();
        assert_relative_eq!(
            k.mu_over_rho(8.0),
            Compound::kapton().mu_over_rho(8.0),
            max_relative = 1e-12
        );
        assert!(Compound::from_spec("unobtainium").is_err());
        assert!(Compound::from_spec("Au@heavy").is_err());
    }

    #[test]
    fn test_mu_per_um_units() {
        // mu[1/um] = (mu/rho)[cm^2/g] * rho[g/cm^3] / 1e4.
        let pmma = Compound::pmma();
        let expected = pmma.mu_over_rho(8.0) * 1.19 / 1e4;
        assert_relative_eq!(pmma.mu_per_um(8.0), expected, max_relative = 1e-12);
        assert_relative_eq!(
            pmma.mu_en_per_um(8.0),
            pmma.mu_en_over_rho(8.0) * 1.19 / 1e4,
            max_relative = 1e-12
        );
        // ~1.3 mm 1/e depth for PMMA at 8 keV.
        assert!(pmma.mu_per_um(8.0) > 0.0 && pmma.mu_per_um(8.0) < 1e-2);
    }
}
