//! Atomic scattering factors `f1`, `f2` (Henke / CXRO tables) and the X-ray /
//! EUV / BEUV optical constants derived from them.
//!
//! Between roughly 30 eV and 30 keV every condensed material has a refractive
//! index slightly below unity, `n = 1 - delta + i beta`, which follows from the
//! forward-scattering amplitudes of its atoms. This module embeds the CXRO
//! (Center for X-Ray Optics, LBNL) atomic scattering factor tables for 24
//! elements and computes `delta`, `beta` for any element or stoichiometric
//! compound from its chemical formula and density. It is the data layer behind
//! the EUV (13.5 nm) / BEUV (6.7 nm) entries of
//! [`crate::materials::database::MaterialsDatabase`], the multilayer-mirror
//! model [`crate::materials::multilayer`], and the complex (phase-shifting)
//! Au absorber of the LIGA Fresnel propagation in [`crate::deep_xray`].
//!
//! # Key equations
//!
//! Independent-atom model (the basis of the Henke tables):
//!
//! ```text
//!   n     = 1 - delta + i beta
//!   delta = (r_e lambda^2 / 2 pi) * sum_q n_q f1_q(E)
//!   beta  = (r_e lambda^2 / 2 pi) * sum_q n_q f2_q(E)
//!   n_q   = rho N_A x_q / sum_p x_p A_p           (atoms of species q per volume)
//!   mu_a  = 4 pi beta / lambda = 2 r_e lambda sum_q n_q f2_q   (photoabsorption)
//! ```
//!
//! with `r_e = 2.8179403262e-15 m` (CODATA), `x_q` the formula counts, `A_p`
//! the standard atomic weights, `rho` the mass density, and `lambda = hc/E`
//! (`hc = 1239.84193 eV nm`). The sign convention is the physics one: a wave
//! `exp(i(kz - wt))` crossing thickness `t` of the material picks up the factor
//! `exp(i k (n - 1) t) = exp(-k beta t) exp(-i k delta t)` relative to vacuum,
//! so the intensity attenuation length is `lambda / (4 pi beta)`.
//!
//! Interpolation between tabulated energies: `f2` log-log in (E, f2) (it is a
//! positive, piecewise power-law photoabsorption cross section), `f1` linear in
//! `ln E` (it changes sign near some edges, so it cannot be log-interpolated).
//! Edges appear in the tables as two rows at the same energy (below / above);
//! an energy exactly on an edge takes the above-edge row. Against the CXRO
//! on-line calculator (`henke.lbl.gov/cgi-bin/getdb.pl`, queried 2026-09-30
//! for 30 materials at 13.5 nm and 6.7 nm) this reproduces `delta` and `beta`
//! to <= 3e-4 relative wherever `delta` is not near a zero crossing (the
//! residual is the atomic-weight table: CXRO uses slightly older values, e.g.
//! Mo 95.94 vs the IUPAC 95.95 used here), and to <= 2e-6 absolute otherwise.
//!
//! # Data provenance
//!
//! The per-element files in `materials/henke_data/*.nff` are the CXRO tables
//! `https://henke.lbl.gov/optical_constants/sf/<el>.nff`, retrieved 2026-09-30,
//! stored byte-for-byte except that CR-LF line endings were normalized to LF
//! (SHA-256 of the original downloads are listed in `docs/materials.md`; an
//! FNV-1a hash of each stored file is pinned by
//! `test_embedded_tables_unmodified`). They are the Henke, Gullikson & Davis
//! compilation (At. Data Nucl. Data Tables 54, 181 (1993)) as maintained by
//! CXRO, including CXRO's later updates: the Cr, Pt and Ta files carry their
//! own header lines naming the measurement campaigns they were updated from
//! (Cr: F. Delmotte et al. 2018; Pt: R. Soufli et al. 2019; Ta: ALS 2008).
//! Elements: H, Be, B, C, N, O, Al, Si, Ti, Cr, Co, Ni, Cu, Zr, Mo, Ru, Sn, Te,
//! La, Hf, Ta, W, Pt, Au.
//!
//! # Model status
//!
//! Implemented and fixture-tested against CXRO: `f1`/`f2` lookup, compound
//! `delta`/`beta` from formula + density, refractive index at any wavelength in
//! 0.0413-41.3 nm (30 eV - 30 keV). Approximations inherited from the tables:
//! independent (isolated) atoms, so chemical shifts, solid-state near-edge
//! structure (XANES/EXAFS) and density effects beyond `rho` are absent except
//! where CXRO replaced an element's table with measured data; results are only
//! as good as the density the caller supplies (thin films are often 5-10%
//! below bulk). Energies outside 30 eV - 30 keV are rejected rather than
//! extrapolated.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::error::{LithographyError, Result};
use crate::types::Complex64;

/// Lowest photon energy (eV) at which the embedded tables are used (the CXRO
/// calculator's documented lower limit; `f1` is undefined further down for
/// most elements).
pub const HENKE_MIN_EV: f64 = 30.0;
/// Highest photon energy (eV) covered by the tables.
pub const HENKE_MAX_EV: f64 = 30_000.0;

/// Shortest vacuum wavelength (nm) covered by the tables, `hc / HENKE_MAX_EV`
/// (0.0413 nm).
pub const HENKE_MIN_NM: f64 = HC_EV_NM / HENKE_MAX_EV;
/// Longest vacuum wavelength (nm) covered by the tables, `hc / HENKE_MIN_EV`
/// (41.33 nm).
pub const HENKE_MAX_NM: f64 = HC_EV_NM / HENKE_MIN_EV;

/// Whether `wavelength_nm` lies inside the Henke/CXRO table range (decided in
/// energy, `hc / lambda` in [[`HENKE_MIN_EV`], [`HENKE_MAX_EV`]], so the end
/// points agree exactly with [`ScatteringFactorTable::f1_f2`]).
pub fn wavelength_in_range(wavelength_nm: f64) -> bool {
    wavelength_nm > 0.0 && (HENKE_MIN_EV..=HENKE_MAX_EV).contains(&(HC_EV_NM / wavelength_nm))
}

/// Classical electron radius in cm (CODATA 2018: 2.8179403262e-15 m).
const R_E_CM: f64 = 2.817_940_326_2e-13;
/// Avogadro constant (exact, SI 2019).
const AVOGADRO: f64 = 6.022_140_76e23;
/// `hc` in eV nm (CODATA).
const HC_EV_NM: f64 = 1_239.841_93;

/// Static description of one embedded element table.
struct ElementSpec {
    symbol: &'static str,
    z: u32,
    /// Standard atomic weight (IUPAC conventional/abridged value), g/mol.
    atomic_weight: f64,
    /// Raw `.nff` file contents.
    raw: &'static str,
    /// FNV-1a 64-bit hash of `raw` (provenance regression guard; read by
    /// `test_embedded_tables_unmodified`).
    #[cfg_attr(not(test), allow(dead_code))]
    fnv1a: u64,
}

macro_rules! spec {
    ($sym:literal, $z:literal, $a:literal, $file:literal, $hash:literal) => {
        ElementSpec {
            symbol: $sym,
            z: $z,
            atomic_weight: $a,
            raw: include_str!(concat!("henke_data/", $file)),
            fnv1a: $hash,
        }
    };
}

static SPECS: [ElementSpec; 24] = [
    spec!("H", 1, 1.008, "h.nff", 0x781c_5c04_f182_603b),
    spec!("Be", 4, 9.012_183_1, "be.nff", 0x3390_855c_a633_2d4b),
    spec!("B", 5, 10.81, "b.nff", 0x7d16_ac8b_a0c7_42bb),
    spec!("C", 6, 12.011, "c.nff", 0xa15f_ca8c_5aa5_f96d),
    spec!("N", 7, 14.007, "n.nff", 0x6329_196e_74b9_abc2),
    spec!("O", 8, 15.999, "o.nff", 0xa371_bcfc_14d9_9b72),
    spec!("Al", 13, 26.981_538_5, "al.nff", 0xd040_81f5_80dc_e2e5),
    spec!("Si", 14, 28.085, "si.nff", 0x5055_a99c_5b31_bca2),
    spec!("Ti", 22, 47.867, "ti.nff", 0xf0b9_aed1_7194_76f7),
    spec!("Cr", 24, 51.996_1, "cr.nff", 0xaf10_af10_9539_9f4a),
    spec!("Co", 27, 58.933_194, "co.nff", 0xb3ae_a537_f1b0_bbb4),
    spec!("Ni", 28, 58.693_4, "ni.nff", 0x32eb_50f6_c69d_9279),
    spec!("Cu", 29, 63.546, "cu.nff", 0xe74b_5262_cb20_f63d),
    spec!("Zr", 40, 91.224, "zr.nff", 0x02f5_1dab_1f84_8408),
    spec!("Mo", 42, 95.95, "mo.nff", 0x5ccf_9e79_c528_f032),
    spec!("Ru", 44, 101.07, "ru.nff", 0x05aa_9044_5d3b_a5de),
    spec!("Sn", 50, 118.71, "sn.nff", 0x8e4c_4eb9_3172_0bdb),
    spec!("Te", 52, 127.60, "te.nff", 0x72a9_6cd6_cdef_b303),
    spec!("La", 57, 138.905_47, "la.nff", 0x38aa_d424_3e89_9f77),
    spec!("Hf", 72, 178.49, "hf.nff", 0x3c34_fa3f_c211_36bb),
    spec!("Ta", 73, 180.947_88, "ta.nff", 0xacef_0970_6518_466d),
    spec!("W", 74, 183.84, "w.nff", 0x7b57_b84a_0180_053a),
    spec!("Pt", 78, 195.084, "pt.nff", 0x6119_12f6_3369_8478),
    spec!("Au", 79, 196.966_569, "au.nff", 0xb02a_8928_ed71_e9d8),
];

/// Tabulated atomic scattering factors of one element.
#[derive(Debug, Clone)]
pub struct ScatteringFactorTable {
    /// Chemical symbol (e.g. `"Mo"`).
    pub symbol: &'static str,
    /// Atomic number.
    pub z: u32,
    /// Standard atomic weight in g/mol used to convert density to number
    /// density.
    pub atomic_weight: f64,
    energies_ev: Vec<f64>,
    f1: Vec<f64>,
    f2: Vec<f64>,
}

impl ScatteringFactorTable {
    /// Parse a CXRO `.nff` file: whitespace-separated `E(eV) f1 f2` rows;
    /// header/comment lines (anything not parsing as three numbers) are
    /// skipped, and the `-9999` "not available" sentinel for `f1` becomes NaN.
    fn parse(spec: &ElementSpec) -> Self {
        let mut energies_ev = Vec::new();
        let mut f1 = Vec::new();
        let mut f2 = Vec::new();
        for line in spec.raw.lines() {
            let mut it = line.split_whitespace().map(str::parse::<f64>);
            let (Some(Ok(e)), Some(Ok(a)), Some(Ok(b))) = (it.next(), it.next(), it.next()) else {
                continue;
            };
            energies_ev.push(e);
            f1.push(if a <= -9000.0 { f64::NAN } else { a });
            f2.push(b);
        }
        Self {
            symbol: spec.symbol,
            z: spec.z,
            atomic_weight: spec.atomic_weight,
            energies_ev,
            f1,
            f2,
        }
    }

    /// Tabulated photon energies in eV (ascending; edges appear twice).
    pub fn energies_ev(&self) -> &[f64] {
        &self.energies_ev
    }

    /// Number of tabulated rows.
    pub fn len(&self) -> usize {
        self.energies_ev.len()
    }

    /// Whether the table is empty (never true for the embedded data).
    pub fn is_empty(&self) -> bool {
        self.energies_ev.is_empty()
    }

    /// Interpolated `(f1, f2)` at `energy_ev` (electrons per atom).
    ///
    /// `f2` is interpolated log-log, `f1` linearly in `ln E` (see module
    /// docs). Errors outside [[`HENKE_MIN_EV`], [`HENKE_MAX_EV`]] or where the
    /// table has no `f1`.
    pub fn f1_f2(&self, energy_ev: f64) -> Result<(f64, f64)> {
        if !(HENKE_MIN_EV..=HENKE_MAX_EV).contains(&energy_ev) {
            return Err(LithographyError::InvalidParameter {
                name: "energy_ev",
                value: energy_ev,
                reason: "outside the Henke/CXRO table range 30 eV - 30 keV",
            });
        }
        let e = &self.energies_ev;
        let n = e.len();
        // First index with e > energy: bracket [i - 1, i].
        let i = e.partition_point(|&x| x <= energy_ev);
        let (f1, f2) = if i == 0 {
            (self.f1[0], self.f2[0])
        } else if i >= n {
            (self.f1[n - 1], self.f2[n - 1])
        } else {
            let (e0, e1) = (e[i - 1], e[i]);
            let t = (energy_ev / e0).ln() / (e1 / e0).ln();
            let f1 = self.f1[i - 1] + t * (self.f1[i] - self.f1[i - 1]);
            let (a, b) = (self.f2[i - 1], self.f2[i]);
            let f2 = if a > 0.0 && b > 0.0 {
                (a.ln() + t * (b / a).ln()).exp()
            } else {
                a + t * (b - a)
            };
            (f1, f2)
        };
        if f1.is_nan() {
            return Err(LithographyError::InvalidParameter {
                name: "energy_ev",
                value: energy_ev,
                reason: "f1 is not tabulated at this energy for this element",
            });
        }
        Ok((f1, f2))
    }

    /// Mass photoabsorption coefficient `mu_a / rho = 2 r_e lambda f2 N_A / A`
    /// in cm^2/g at `energy_ev` (photoabsorption only: no Compton or
    /// Rayleigh scattering, which the `f2` tables do not contain).
    pub fn mass_photoabsorption_cm2_g(&self, energy_ev: f64) -> Result<f64> {
        let (_, f2) = self.f1_f2(energy_ev)?;
        let lambda_cm = HC_EV_NM / energy_ev * 1e-7;
        Ok(2.0 * R_E_CM * lambda_cm * f2 * AVOGADRO / self.atomic_weight)
    }
}

fn tables() -> &'static [ScatteringFactorTable] {
    static TABLES: OnceLock<Vec<ScatteringFactorTable>> = OnceLock::new();
    TABLES.get_or_init(|| SPECS.iter().map(ScatteringFactorTable::parse).collect())
}

/// Look up an element's scattering-factor table by chemical symbol
/// (case-sensitive, e.g. `"Mo"`, `"B"`). `None` if it is not embedded.
pub fn element(symbol: &str) -> Option<&'static ScatteringFactorTable> {
    tables().iter().find(|t| t.symbol == symbol)
}

/// Symbols of all embedded elements, in atomic-number order.
pub fn available_elements() -> Vec<&'static str> {
    SPECS.iter().map(|s| s.symbol).collect()
}

/// Mass densities (g/cm^3) the CXRO calculator uses by default ("tabulated
/// density") for elements and a few common compounds, as reported by
/// `henke.lbl.gov/cgi-bin/getdb.pl` on 2026-09-30. Bulk values: thin films
/// are frequently less dense, so pass a measured density when you have one.
/// H, N and O are gases and have no solid default.
static DEFAULT_DENSITIES: [(&str, f64); 32] = [
    ("Be", 1.848),
    ("B", 2.34),
    ("C", 2.2),
    ("Al", 2.699),
    ("Si", 2.33),
    ("Ti", 4.54),
    ("Cr", 7.19),
    ("Co", 8.9),
    ("Ni", 8.902),
    ("Cu", 8.96),
    ("Zr", 6.506),
    ("Mo", 10.22),
    ("Ru", 12.41),
    ("Sn", 7.3),
    ("Te", 6.24),
    ("La", 6.166),
    ("Hf", 13.31),
    ("Ta", 16.65),
    ("W", 19.3),
    ("Pt", 21.45),
    ("Au", 19.32),
    ("B4C", 2.52),
    ("SiO2", 2.2),
    ("Si3N4", 3.44),
    ("MoSi2", 6.31),
    ("HfO2", 9.68),
    ("RuO2", 6.97),
    ("TaN", 16.3),
    ("BN", 2.25),
    // CXRO's named polymers: PMMA-formvar, polyimide (Kapton), polycarbonate.
    ("C5H8O2", 1.19),
    ("C22H10N2O5", 1.43),
    ("C16H14O3", 1.2),
];

/// The CXRO default ("tabulated") density in g/cm^3 for `formula`, if listed
/// (see [`Material::with_default_density`]).
pub fn default_density(formula: &str) -> Option<f64> {
    DEFAULT_DENSITIES
        .iter()
        .find(|(f, _)| *f == formula)
        .map(|(_, d)| *d)
}

/// Parse a simple chemical formula (`"B4C"`, `"C5H8O2"`, `"Si3N4"`,
/// `"SiO1.5"`) into `(symbol, count)` pairs, merging repeats. Parentheses
/// are not supported.
pub fn parse_formula(formula: &str) -> Result<Vec<(&'static str, f64)>> {
    let bad = |reason: &'static str| LithographyError::InvalidParameter {
        name: "formula",
        value: f64::NAN,
        reason,
    };
    let chars: Vec<char> = formula.trim().chars().collect();
    if chars.is_empty() {
        return Err(bad("empty chemical formula"));
    }
    let mut out: Vec<(&'static str, f64)> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if !c.is_ascii_uppercase() {
            return Err(bad(
                "formula must be element symbols (e.g. B4C, C5H8O2); parentheses are not supported",
            ));
        }
        let mut sym = c.to_string();
        i += 1;
        while i < chars.len() && chars[i].is_ascii_lowercase() {
            sym.push(chars[i]);
            i += 1;
        }
        let start = i;
        while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
            i += 1;
        }
        let count = if start == i {
            1.0
        } else {
            let s: String = chars[start..i].iter().collect();
            s.parse::<f64>()
                .map_err(|_| bad("unparseable element count in formula"))?
        };
        if count <= 0.0 {
            return Err(bad("element counts must be positive"));
        }
        let table = element(&sym).ok_or_else(|| LithographyError::MaterialNotFound(sym.clone()))?;
        match out.iter_mut().find(|(s, _)| *s == table.symbol) {
            Some(entry) => entry.1 += count,
            None => out.push((table.symbol, count)),
        }
    }
    Ok(out)
}

/// A homogeneous material for X-ray / EUV optics: a chemical formula plus a
/// mass density, from which `delta` and `beta` follow (see module docs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Material {
    /// Chemical formula as given (e.g. `"B4C"`).
    pub formula: String,
    /// Mass density in g/cm^3.
    pub density_g_cm3: f64,
}

impl Material {
    /// A material of the given formula and density (g/cm^3). Errors on an
    /// unparseable formula, an element without an embedded table, or a
    /// non-positive density.
    pub fn new(formula: &str, density_g_cm3: f64) -> Result<Self> {
        if density_g_cm3.is_nan() || density_g_cm3 <= 0.0 {
            return Err(LithographyError::InvalidParameter {
                name: "density_g_cm3",
                value: density_g_cm3,
                reason: "must be positive",
            });
        }
        parse_formula(formula)?;
        Ok(Self {
            formula: formula.trim().to_string(),
            density_g_cm3,
        })
    }

    /// A material at the CXRO default density (see [`default_density`]).
    /// Errors if the formula has no listed default.
    pub fn with_default_density(formula: &str) -> Result<Self> {
        let rho = default_density(formula.trim()).ok_or_else(|| {
            LithographyError::MaterialNotFound(format!(
                "{formula} (no default density listed; use Material::new with an explicit density)"
            ))
        })?;
        Self::new(formula, rho)
    }

    /// Atom number densities `(symbol, atoms per cm^3)` of each species.
    pub fn number_densities_cm3(&self) -> Result<Vec<(&'static str, f64)>> {
        let comp = parse_formula(&self.formula)?;
        let formula_weight: f64 = comp
            .iter()
            .map(|(s, x)| x * element(s).map_or(0.0, |t| t.atomic_weight))
            .sum();
        Ok(comp
            .into_iter()
            .map(|(s, x)| (s, self.density_g_cm3 * AVOGADRO * x / formula_weight))
            .collect())
    }

    /// `(delta, beta)` at photon energy `energy_ev` (30 eV - 30 keV).
    pub fn delta_beta_at_energy(&self, energy_ev: f64) -> Result<(f64, f64)> {
        let lambda_cm = HC_EV_NM / energy_ev * 1e-7;
        let pref = R_E_CM * lambda_cm * lambda_cm / (2.0 * std::f64::consts::PI);
        let mut sum_f1 = 0.0;
        let mut sum_f2 = 0.0;
        for (sym, n_cm3) in self.number_densities_cm3()? {
            let table =
                element(sym).ok_or_else(|| LithographyError::MaterialNotFound(sym.into()))?;
            let (f1, f2) = table.f1_f2(energy_ev)?;
            sum_f1 += n_cm3 * f1;
            sum_f2 += n_cm3 * f2;
        }
        Ok((pref * sum_f1, pref * sum_f2))
    }

    /// `(delta, beta)` at vacuum wavelength `wavelength_nm` (0.0413-41.3 nm).
    pub fn delta_beta(&self, wavelength_nm: f64) -> Result<(f64, f64)> {
        if wavelength_nm.is_nan() || wavelength_nm <= 0.0 {
            return Err(LithographyError::InvalidParameter {
                name: "wavelength_nm",
                value: wavelength_nm,
                reason: "must be positive",
            });
        }
        if !wavelength_in_range(wavelength_nm) {
            return Err(LithographyError::WavelengthOutOfRange {
                material: format!("henke:{}", self.formula),
                wavelength_nm,
                range_nm: (HENKE_MIN_NM, HENKE_MAX_NM),
                hint: "the Henke/CXRO tables cover 30 eV - 30 keV and are not extrapolated"
                    .to_string(),
            });
        }
        self.delta_beta_at_energy(HC_EV_NM / wavelength_nm)
    }

    /// Complex refractive index `n = 1 - delta + i beta` (the `n + ik`,
    /// `k >= 0` convention of [`crate::thinfilm::FilmLayer`]).
    pub fn refractive_index(&self, wavelength_nm: f64) -> Result<Complex64> {
        let (delta, beta) = self.delta_beta(wavelength_nm)?;
        Ok(Complex64::new(1.0 - delta, beta))
    }

    /// Intensity attenuation length `lambda / (4 pi beta)` in nm (photo-
    /// absorption only; Compton/Rayleigh losses are not in `f2`).
    pub fn attenuation_length_nm(&self, wavelength_nm: f64) -> Result<f64> {
        let (_, beta) = self.delta_beta(wavelength_nm)?;
        Ok(wavelength_nm / (4.0 * std::f64::consts::PI * beta))
    }

    /// Photoabsorption coefficient `mu_a = 4 pi beta / lambda` in 1/um at
    /// photon energy `energy_kev`.
    pub fn photoabsorption_per_um(&self, energy_kev: f64) -> Result<f64> {
        let wavelength_nm = HC_EV_NM / (energy_kev * 1e3);
        Ok(1e3 / self.attenuation_length_nm(wavelength_nm)?)
    }
}

/// `(delta, beta)` at `energy_ev` for a mixture given by `(symbol, mass
/// fraction)` components (the representation of
/// [`crate::materials::attenuation::Compound`]) at `density_g_cm3`:
/// `n_q = rho N_A w_q / A_q`. Errors for symbols without a table or energies
/// outside 30 eV - 30 keV.
pub fn delta_beta_from_mass_fractions(
    components: &[(&str, f64)],
    density_g_cm3: f64,
    energy_ev: f64,
) -> Result<(f64, f64)> {
    let lambda_cm = HC_EV_NM / energy_ev * 1e-7;
    let pref = R_E_CM * lambda_cm * lambda_cm / (2.0 * std::f64::consts::PI);
    let mut sum_f1 = 0.0;
    let mut sum_f2 = 0.0;
    for (sym, w) in components {
        let table =
            element(sym).ok_or_else(|| LithographyError::MaterialNotFound((*sym).into()))?;
        let (f1, f2) = table.f1_f2(energy_ev)?;
        let n_cm3 = density_g_cm3 * AVOGADRO * w / table.atomic_weight;
        sum_f1 += n_cm3 * f1;
        sum_f2 += n_cm3 * f2;
    }
    Ok((pref * sum_f1, pref * sum_f2))
}

/// Convenience: `n = 1 - delta + i beta` of `formula` at `density_g_cm3` and
/// `wavelength_nm`.
pub fn refractive_index(
    formula: &str,
    density_g_cm3: f64,
    wavelength_nm: f64,
) -> Result<Complex64> {
    Material::new(formula, density_g_cm3)?.refractive_index(wavelength_nm)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CXRO on-line calculator values (henke.lbl.gov/cgi-bin/getdb.pl,
    /// "tabulated" density, queried 2026-09-30): formula, density, lambda nm,
    /// delta, beta. Independent of this implementation.
    const CXRO_REFERENCE: [(&str, f64, f64, f64, f64); 24] = [
        ("Si", 2.33, 13.5, 0.000998703763, 0.00182646094),
        ("Mo", 10.22, 13.5, 0.0762064755, 0.00643542456),
        ("Ru", 12.41, 13.5, 0.113639966, 0.0170648936),
        ("Ta", 16.65, 13.5, 0.0433237441, 0.0343398191),
        ("La", 6.166, 13.5, 0.00263774605, 0.00495616579),
        ("B4C", 2.52, 13.5, 0.0362285152, 0.00514584174),
        ("Sn", 7.3, 13.5, 0.0584564693, 0.0725575313),
        ("Au", 19.32, 13.5, 0.101221479, 0.0517498069),
        ("Ni", 8.902, 13.5, 0.0517767183, 0.0727213696),
        ("Pt", 21.45, 13.5, 0.108686924, 0.0581844486),
        ("Cr", 7.19, 13.5, 0.0751224086, 0.043694526),
        ("TaN", 16.3, 13.5, 0.061500866, 0.037757013),
        ("Si3N4", 3.44, 13.5, 0.0268646851, 0.00931774173),
        ("HfO2", 9.68, 13.5, 0.0510135442, 0.0338059366),
        ("Si", 2.33, 6.69999981, 0.00796837453, 0.0094193425),
        ("Mo", 10.22, 6.69999981, 0.0127606466, 0.00283752894),
        ("La", 6.166, 6.69999981, 0.0161006916, 0.00136326044),
        ("B", 2.34, 6.69999981, -0.00135282124, 0.000408262247),
        ("B4C", 2.52, 6.69999981, 0.00103260903, 0.000527112803),
        ("Ru", 12.41, 6.69999981, 0.0184547398, 0.00398221193),
        ("Ti", 4.54, 6.69999981, 0.0126586016, 0.00401955144),
        ("Be", 1.848, 6.69999981, 0.00939260423, 0.00543388724),
        ("Zr", 6.506, 6.69999981, 0.00412964495, 0.0019918594),
        ("Te", 6.24, 6.69999981, 0.0106867021, 0.00177389919),
    ];

    fn close(ours: f64, reference: f64) -> bool {
        (ours - reference).abs() <= 5e-4 * reference.abs() + 2e-6
    }

    #[test]
    fn test_embedded_tables_unmodified() {
        for spec in &SPECS {
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for b in spec.raw.bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
            assert_eq!(
                h, spec.fnv1a,
                "{} table differs from the CXRO download",
                spec.symbol
            );
        }
    }

    #[test]
    fn test_tables_parse_sorted_and_cover_range() {
        for t in tables() {
            assert!(t.len() > 400, "{} has only {} rows", t.symbol, t.len());
            assert!(
                t.energies_ev().windows(2).all(|w| w[1] >= w[0]),
                "{} unsorted",
                t.symbol
            );
            assert!(
                t.energies_ev()[0] <= HENKE_MIN_EV
                    && *t.energies_ev().last().unwrap() >= HENKE_MAX_EV
            );
            // f1 is available over the whole supported window.
            for e in [30.0, 91.84, 185.05, 1000.0, 8000.0, 30000.0] {
                let (f1, f2) = t.f1_f2(e).unwrap();
                assert!(
                    f1.is_finite() && f2 > 0.0,
                    "{} at {e} eV: {f1} {f2}",
                    t.symbol
                );
            }
        }
    }

    #[test]
    fn test_high_energy_f1_approaches_z() {
        // Far above all edges f1 -> Z up to small anomalous-dispersion and
        // relativistic corrections (a few percent); f2 -> small.
        for sym in ["C", "Si", "Mo", "Au"] {
            let t = element(sym).unwrap();
            let (f1, f2) = t.f1_f2(30_000.0).unwrap();
            let z = t.z as f64;
            assert!(
                (f1 - z).abs() < 0.05 * z,
                "{sym}: f1(30 keV) = {f1}, Z = {z}"
            );
            assert!(f2 < 0.1 * z, "{sym}: f2(30 keV) = {f2}");
        }
    }

    #[test]
    fn test_interpolation_exact_at_nodes_and_log_log_between() {
        let t = element("Si").unwrap();
        // Pick a node well away from edges (row index 300).
        let k = 300;
        let (e0, e1) = (t.energies_ev[k], t.energies_ev[k + 1]);
        assert!(e1 > e0);
        let (f1, f2) = t.f1_f2(e0).unwrap();
        assert_eq!(f1, t.f1[k]);
        assert_eq!(f2, t.f2[k]);
        let em = (e0 * e1).sqrt(); // geometric midpoint: t = 1/2
        let (f1m, f2m) = t.f1_f2(em).unwrap();
        assert!((f1m - 0.5 * (t.f1[k] + t.f1[k + 1])).abs() < 1e-12);
        assert!((f2m - (t.f2[k] * t.f2[k + 1]).sqrt()).abs() < 1e-12 * f2m);
    }

    #[test]
    fn test_out_of_range_rejected() {
        let mo = element("Mo").unwrap();
        assert!(mo.f1_f2(20.0).is_err());
        assert!(mo.f1_f2(31_000.0).is_err());
        let m = Material::new("Mo", 10.22).unwrap();
        assert!(m.delta_beta(50.0).is_err()); // 24.8 eV
        assert!(m.delta_beta(0.03).is_err()); // 41 keV
                                              // Wavelength lookups fail with the typed range error.
        match m.refractive_index(157.0) {
            Err(LithographyError::WavelengthOutOfRange {
                material, range_nm, ..
            }) => {
                assert_eq!(material, "henke:Mo");
                // hc / 30 keV and hc / 30 eV with hc = 1239.84193 eV nm.
                assert!((range_nm.0 - 0.041_328_064).abs() < 1e-8);
                assert!((range_nm.1 - 41.328_064).abs() < 1e-5);
            }
            other => panic!("expected WavelengthOutOfRange, got {other:?}"),
        }
        // The end points themselves are inside (decided in energy).
        assert!(m.refractive_index(HENKE_MAX_NM).is_ok());
        assert!(m.refractive_index(HENKE_MIN_NM).is_ok());
        assert!(!wavelength_in_range(HENKE_MAX_NM * 1.000_001));
    }

    #[test]
    fn test_formula_parsing() {
        assert_eq!(parse_formula("B4C").unwrap(), vec![("B", 4.0), ("C", 1.0)]);
        assert_eq!(
            parse_formula("C5H8O2").unwrap(),
            vec![("C", 5.0), ("H", 8.0), ("O", 2.0)]
        );
        assert_eq!(
            parse_formula("SiO1.5").unwrap(),
            vec![("Si", 1.0), ("O", 1.5)]
        );
        // Repeated symbols merge.
        assert_eq!(
            parse_formula("CH3CH3").unwrap(),
            vec![("C", 2.0), ("H", 6.0)]
        );
        assert!(parse_formula("Xx2").is_err());
        assert!(parse_formula("Ca(OH)2").is_err());
        assert!(parse_formula("").is_err());
        assert!(Material::new("Mo", 0.0).is_err());
    }

    #[test]
    fn test_number_density_fixture() {
        // Si at 2.33 g/cm^3: n = 2.33 * 6.02214076e23 / 28.085 = 4.9961e22 cm^-3
        // (hand calculation).
        let si = Material::new("Si", 2.33).unwrap();
        let n = si.number_densities_cm3().unwrap();
        assert_eq!(n.len(), 1);
        assert!((n[0].1 / 4.9961e22 - 1.0).abs() < 1e-4, "{}", n[0].1);
        // B4C at 2.52: formula weight 4*10.81 + 12.011 = 55.251 g/mol;
        // n_B = 4 * 2.52 * N_A / 55.251 = 1.09868e23, n_C = 2.74670e22.
        let b4c = Material::new("B4C", 2.52)
            .unwrap()
            .number_densities_cm3()
            .unwrap();
        assert!((b4c[0].1 / 1.09868e23 - 1.0).abs() < 1e-4);
        assert!((b4c[1].1 / 2.74670e22 - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_delta_beta_fixture_by_hand() {
        // delta = r_e lambda^2 n f1 / 2pi evaluated by hand for Mo at a
        // tabulated energy node (86.0 eV row: f1 = 15.3621, f2 = 1.46390):
        // lambda = 1239.84193/86 nm = 1.441677e-6 cm,
        // n = 10.22 * 6.02214076e23 / 95.95 = 6.41441e22 cm^-3,
        // pref = 2.8179403e-13 * (1.441677e-6)^2 / 2pi = 9.32154e-26 cm^3,
        // delta = pref * n * f1 = 0.0918534, beta = 0.00875298.
        let mo = Material::new("Mo", 10.22).unwrap();
        let (d, b) = mo.delta_beta_at_energy(86.0).unwrap();
        assert!((d / 0.0918534 - 1.0).abs() < 1e-5, "delta {d}");
        assert!((b / 0.00875298 - 1.0).abs() < 1e-5, "beta {b}");
    }

    #[test]
    fn test_matches_cxro_calculator() {
        for (formula, rho, lambda, delta_ref, beta_ref) in CXRO_REFERENCE {
            let m = Material::new(formula, rho).unwrap();
            let (d, b) = m.delta_beta(lambda).unwrap();
            assert!(
                close(d, delta_ref),
                "{formula} @ {lambda} nm: delta {d} vs CXRO {delta_ref}"
            );
            assert!(
                close(b, beta_ref),
                "{formula} @ {lambda} nm: beta {b} vs CXRO {beta_ref}"
            );
        }
    }

    #[test]
    fn test_matches_cxro_scan_between_nodes() {
        // CXRO wavelength scans (log-spaced points that fall between table
        // nodes) for Mo and Si across the 13.5 nm band.
        let mo = Material::new("Mo", 10.22).unwrap();
        for (lambda, dref, bref) in [
            (12.5, 0.0615031272, 0.00507026212),
            (13.0691509, 0.0695943087, 0.00563455233),
            (13.868536, 0.0821957588, 0.0072598625),
            (14.5, 0.0934876874, 0.00900915638),
        ] {
            let (d, b) = mo.delta_beta(lambda).unwrap();
            assert!(close(d, dref) && close(b, bref), "Mo @ {lambda}: {d} {b}");
        }
    }

    #[test]
    fn test_au_hard_xray_fixture() {
        // CXRO getdb, Au at 19.32 g/cm^3, 8000 eV: delta 4.77303183e-5,
        // beta 4.96011535e-6 (used for the LIGA absorber phase).
        let au = Material::new("Au", 19.32).unwrap();
        let (d, b) = au.delta_beta_at_energy(8000.0).unwrap();
        assert!((d / 4.77303183e-5 - 1.0).abs() < 5e-4, "delta {d}");
        assert!((b / 4.96011535e-6 - 1.0).abs() < 5e-4, "beta {b}");
    }

    #[test]
    fn test_delta_scales_with_density_and_lambda_squared_far_from_edges() {
        // delta, beta are linear in density at fixed wavelength.
        let a = Material::new("Si", 2.33).unwrap().delta_beta(1.0).unwrap();
        let b = Material::new("Si", 4.66).unwrap().delta_beta(1.0).unwrap();
        assert!((b.0 / a.0 - 2.0).abs() < 1e-12 && (b.1 / a.1 - 2.0).abs() < 1e-12);
        // Far above the K edge f1 ~ const, so delta ~ lambda^2: halving the
        // wavelength (8 -> 16 keV, both above all C edges) quarters delta.
        let c = Material::new("C", 2.2).unwrap();
        let d8 = c.delta_beta_at_energy(8000.0).unwrap().0;
        let d16 = c.delta_beta_at_energy(16000.0).unwrap().0;
        assert!((d8 / d16 - 4.0).abs() < 0.01, "ratio {}", d8 / d16);
    }

    #[test]
    fn test_default_densities_and_refractive_index() {
        assert_eq!(default_density("Mo"), Some(10.22));
        assert_eq!(default_density("B4C"), Some(2.52));
        assert_eq!(default_density("N"), None);
        let mo = Material::with_default_density("Mo").unwrap();
        let n = mo.refractive_index(13.5).unwrap();
        assert!((n.re - (1.0 - 0.0762064755)).abs() < 5e-5 && (n.im - 0.00643542456).abs() < 5e-6);
        assert!(Material::with_default_density("O").is_err());
        let n2 = refractive_index("Mo", 10.22, 13.5).unwrap();
        assert_eq!(n, n2);
    }

    #[test]
    fn test_mass_fraction_route_matches_formula_route() {
        // PMMA by mass fractions (C 0.59985, H 0.08055, O 0.31960; rounded to
        // 5 digits) vs the C5H8O2 formula at 1.19 g/cm^3.
        let comps = [("C", 0.59985), ("H", 0.08055), ("O", 0.31960)];
        let (d1, b1) = delta_beta_from_mass_fractions(&comps, 1.19, 8000.0).unwrap();
        let (d2, b2) = Material::new("C5H8O2", 1.19)
            .unwrap()
            .delta_beta_at_energy(8000.0)
            .unwrap();
        assert!((d1 / d2 - 1.0).abs() < 2e-4 && (b1 / b2 - 1.0).abs() < 2e-4);
        // Mass photoabsorption of Au at 8 keV: 2 r_e lambda f2 N_A / A
        // = 208.1 cm^2/g (numpy, same table) vs NIST total mu/rho 207.2.
        let mu = element("Au")
            .unwrap()
            .mass_photoabsorption_cm2_g(8000.0)
            .unwrap();
        assert!((mu - 208.1).abs() < 0.2, "{mu}");
        assert!(delta_beta_from_mass_fractions(&[("Xx", 1.0)], 1.0, 8000.0).is_err());
    }

    #[test]
    fn test_attenuation_length_and_photoabsorption() {
        // Attenuation length = lambda / (4 pi beta): Si at 13.5 nm
        // = 13.5 / (4 pi * 0.00182646) = 588.2 nm (CXRO beta).
        let si = Material::new("Si", 2.33).unwrap();
        let l = si.attenuation_length_nm(13.5).unwrap();
        assert!((l / 588.18 - 1.0).abs() < 1e-3, "{l}");
        // mu_a [1/um] = 1e3 / L[nm].
        let mu = si.photoabsorption_per_um(1.23984193 / 13.5).unwrap();
        assert!((mu * l / 1e3 - 1.0).abs() < 1e-9);
    }
}
