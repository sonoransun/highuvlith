//! Optical-constant database: complex refractive index `n + ik` by material
//! name and wavelength.
//!
//! Three kinds of entries:
//! - **Sellmeier** (dispersive, `k = 0`) for the VUV-transparent crystals,
//!   each with an accepted wavelength range (see the table below).
//! - **Tabulated VUV** `(lambda, n + ik)` points (126-160 nm), linearly
//!   interpolated; approximate transcriptions (unchanged by the 2026-09-30
//!   EUV/BEUV pass).
//! - **X-ray / EUV / BEUV** entries (`*_euv`, `*_beuv`, the resist stand-ins)
//!   computed at the requested wavelength from the Henke/CXRO scattering
//!   factors ([`crate::materials::henke`]): `n = 1 - delta + i beta` for a
//!   stated formula and density, valid 0.0413-41.3 nm. Any other formula is
//!   available as `"henke:<formula>"` (CXRO default density) or
//!   `"henke:<formula>@<density g/cm^3>"`.
//!
//! # Range policy (no silent extrapolation)
//!
//! Every entry has an accepted wavelength range; a lookup outside it never
//! returns a number from the wrong table:
//!
//! 1. If the name is also a Henke formula with a CXRO default density (`"Si"`,
//!    `"Cr"`, `"SiO2"`) and the wavelength is inside the Henke range, the
//!    lookup **falls back to the Henke value** - identical to
//!    `"henke:<name>"` (e.g. `"Si"` at 13.5 nm is `0.99900 + 0.00183i`, the
//!    same as `"Si_euv"`).
//! 2. Otherwise it fails with
//!    [`LithographyError::WavelengthOutOfRange`]
//!    naming the material, the wavelength, the accepted range and the
//!    alternative key.
//!
//! The only remaining hold is [`TABLE_EDGE_HOLD_NM`]: the VUV tables return
//! their end value up to 1 nm beyond their first/last node. It exists so the
//! F2 line (157.63 nm) can use the 157 nm node of the two-point tables; over
//! 1 nm the tabulated dispersion (e.g. Si: dn/dlambda ~ 0.011 /nm between 140
//! and 157 nm) changes `n` by less than the tables' own transcription
//! accuracy.
//!
//! | Kind | Accepted range (nm) |
//! |---|---|
//! | CaF2 / MgF2 / LiF / BaF2 Sellmeier | 125 / 115 / 105 / 135 - 2000 |
//! | SiO2 (fused silica) Sellmeier | 165 - 2000 (absorbs below ~165 nm) |
//! | Cr, Si tables | 125 - 161 (nodes 126-160 + 1 nm hold) |
//! | AlF3, Na3AlF6, LaF3, GdF3, VUV_resist, VUV_BARC | 125 - 158 (nodes 126-157 + hold) |
//! | Henke entries, `henke:` | 0.0413 - 41.3 |
//!
//! The Sellmeier lower limits are the crystals' approximate VUV transmission
//! edges. The fits themselves come from UV-IR measurements (e.g. the CaF2 and
//! fused-silica coefficients are Malitson's, measured from ~210-230 nm up), so
//! their VUV values are extrapolations of the fit; only CaF2 is checked in
//! the VUV here (1.5570 at 157.63 nm, against the ~1.559 of 157 nm
//! measurements). The 2000 nm upper limit covers every lithography and drive
//! laser wavelength in the code and stays well below the infrared poles.
//!
//! # Model status
//!
//! Sellmeier entries: mature fits (see `dispersion.rs`), VUV use extrapolated
//! as stated above. VUV tables: approximate. EUV/BEUV element and compound
//! entries: computed from the CXRO tables (fixture-tested against the CXRO
//! calculator) at CXRO's bulk default densities - thin films are often less
//! dense. The three resist entries are *representative stand-ins* computed
//! for a stated composition, not measured resist data. Out-of-range lookups
//! fall back to Henke or fail (see the range policy); before 2026-10-01 the
//! VUV tables clamped silently (`"Si"` at 13.5 nm returned the 126 nm value
//! `0.55 + 1.75i`) and the Sellmeier fits were evaluated at any wavelength
//! (`"CaF2"` at 13.5 nm returned 0.973).

use num::Complex;
use std::collections::HashMap;

use super::dispersion::{self, SellmeierCoefficients};
use super::henke;
use crate::error::LithographyError;
use crate::thinfilm::{FilmLayer, FilmStack};

type Complex64 = Complex<f64>;

/// Wavelength margin (nm) beyond the first/last node of a VUV table within
/// which the end value is held (see the module's range policy).
pub const TABLE_EDGE_HOLD_NM: f64 = 1.0;

/// Optical constants database for lithography materials (VUV Sellmeier and
/// tabulated entries, plus Henke-computed EUV/BEUV entries).
pub struct MaterialsDatabase {
    /// Sellmeier fits with their accepted wavelength range `(min, max)` nm.
    sellmeier: HashMap<String, (SellmeierCoefficients, (f64, f64))>,
    /// Fixed complex refractive indices (n, k) for materials without Sellmeier models.
    fixed_nk: HashMap<String, Vec<(f64, Complex64)>>,
    /// Named X-ray/EUV/BEUV entries: `(formula, density g/cm^3)` evaluated
    /// with the Henke tables at the requested wavelength.
    xray: HashMap<String, henke::Material>,
}

impl MaterialsDatabase {
    /// Create database populated with standard VUV lithography materials.
    pub fn new() -> Self {
        let mut sellmeier = HashMap::new();
        // Accepted ranges: lower limit = approximate VUV transmission edge
        // (SiO2: the ~165 nm absorption onset), upper = 2000 nm (module docs).
        for (name, coeffs, lo) in [
            ("CaF2", dispersion::caf2_sellmeier(), 125.0),
            ("MgF2", dispersion::mgf2_ordinary_sellmeier(), 115.0),
            ("LiF", dispersion::lif_sellmeier(), 105.0),
            ("BaF2", dispersion::baf2_sellmeier(), 135.0),
            ("SiO2", dispersion::sio2_sellmeier(), 165.0),
        ] {
            sellmeier.insert(name.to_string(), (coeffs, (lo, 2000.0)));
        }

        let mut fixed_nk: HashMap<String, Vec<(f64, Complex64)>> = HashMap::new();

        // Chrome mask absorber at VUV wavelengths (approximate)
        fixed_nk.insert(
            "Cr".to_string(),
            vec![
                (126.0, Complex64::new(0.85, 1.70)),
                (140.0, Complex64::new(0.95, 1.80)),
                (157.0, Complex64::new(1.06, 2.05)),
                (160.0, Complex64::new(1.08, 2.07)),
            ],
        );

        // Silicon substrate at VUV
        fixed_nk.insert(
            "Si".to_string(),
            vec![
                (126.0, Complex64::new(0.55, 1.75)),
                (140.0, Complex64::new(0.70, 1.95)),
                (157.0, Complex64::new(0.88, 2.10)),
                (160.0, Complex64::new(0.90, 2.12)),
            ],
        );

        // AlF3 (AR coating material at VUV)
        fixed_nk.insert(
            "AlF3".to_string(),
            vec![
                (126.0, Complex64::new(1.42, 0.001)),
                (157.0, Complex64::new(1.38, 0.0005)),
            ],
        );

        // Na3AlF6 (cryolite, low-n coating material)
        fixed_nk.insert(
            "Na3AlF6".to_string(),
            vec![
                (126.0, Complex64::new(1.33, 0.005)),
                (157.0, Complex64::new(1.30, 0.001)),
            ],
        );

        // LaF3 (HR coating material at VUV)
        fixed_nk.insert(
            "LaF3".to_string(),
            vec![
                (126.0, Complex64::new(1.72, 0.01)),
                (157.0, Complex64::new(1.68, 0.002)),
            ],
        );

        // GdF3 (HR coating material at VUV)
        fixed_nk.insert(
            "GdF3".to_string(),
            vec![
                (126.0, Complex64::new(1.70, 0.015)),
                (157.0, Complex64::new(1.65, 0.003)),
            ],
        );

        // Generic VUV fluoropolymer photoresist
        fixed_nk.insert(
            "VUV_resist".to_string(),
            vec![
                (126.0, Complex64::new(1.70, 0.03)),
                (157.0, Complex64::new(1.65, 0.015)),
            ],
        );

        // BARC (bottom anti-reflective coating) for VUV
        fixed_nk.insert(
            "VUV_BARC".to_string(),
            vec![
                (126.0, Complex64::new(1.55, 0.30)),
                (157.0, Complex64::new(1.50, 0.25)),
            ],
        );

        // ---- X-ray / EUV (13.5 nm) / BEUV (6.7 nm) ----
        // Computed at the requested wavelength from the Henke/CXRO atomic
        // scattering factors (n = 1 - delta + i beta; at these energies all
        // condensed matter has n slightly BELOW 1), at the CXRO default bulk
        // densities. Mo/Si is the canonical 13.5 nm multilayer pair (large
        // delta contrast, low beta in Si); 6.7 nm sits just below the boron
        // K edge (188 eV), where B4C is nearly transparent - the basis of
        // La/B4C mirrors.
        let mut xray = HashMap::new();
        for (name, formula, rho) in [
            ("Si_euv", "Si", 2.33),
            ("Mo_euv", "Mo", 10.22),
            ("Ru_euv", "Ru", 12.41),
            ("Ta_euv", "Ta", 16.65),
            ("La_beuv", "La", 6.166),
            ("B4C_beuv", "B4C", 2.52),
            // Representative resist stand-ins (compositions stated, not
            // measured resist data):
            // - organic resist: PMMA stoichiometry C5H8O2 at 1.19 g/cm^3,
            //   standing in for the organic backbone of EUV CARs (PAG/quencher
            //   S, F or I raise the real absorption by roughly 10-40%):
            //   alpha = 4 pi beta / lambda ~ 5.2 /um at 13.5 nm, ~1.2 /um at
            //   6.7 nm (below the C K edge organics are nearly transparent -
            //   the BEUV resist-absorption problem);
            // - metal-oxide resist: the butyltin-oxo "Sn12" cage stoichiometry
            //   Sn12C48H116O22 at an ASSUMED 2.0 g/cm^3: alpha ~ 14 /um at
            //   13.5 nm (~2.7x the organic).
            ("EUV_resist", "C5H8O2", 1.19),
            ("MOx_resist", "Sn12C48H116O22", 2.0),
            ("BEUV_resist", "C5H8O2", 1.19),
        ] {
            xray.insert(
                name.to_string(),
                henke::Material::new(formula, rho).expect("valid built-in formula"),
            );
        }

        Self {
            sellmeier,
            fixed_nk,
            xray,
        }
    }

    /// All built-in material names (sorted); `"henke:<formula>[@density]"`
    /// lookups are available in addition.
    pub fn material_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .sellmeier
            .keys()
            .chain(self.fixed_nk.keys())
            .chain(self.xray.keys())
            .cloned()
            .collect();
        names.sort();
        names
    }

    /// Names of the Henke-computed X-ray/EUV/BEUV entries (sorted).
    pub fn xray_material_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.xray.keys().cloned().collect();
        names.sort();
        names
    }

    /// The formula and density behind a named X-ray/EUV/BEUV entry.
    pub fn xray_material(&self, name: &str) -> Option<&henke::Material> {
        self.xray.get(name)
    }

    /// Get the complex refractive index of a material at a given wavelength.
    /// For Sellmeier materials, k=0 (transparent). For tabulated materials,
    /// linear interpolation is used. Wavelengths outside an entry's accepted
    /// range fall back to the Henke tables or fail with
    /// [`LithographyError::WavelengthOutOfRange`] (module docs, range policy).
    pub fn refractive_index(
        &self,
        material: &str,
        wavelength_nm: f64,
    ) -> crate::error::Result<Complex64> {
        if !(wavelength_nm.is_finite() && wavelength_nm > 0.0) {
            return Err(LithographyError::InvalidParameter {
                name: "wavelength_nm",
                value: wavelength_nm,
                reason: "must be positive and finite",
            });
        }

        // Sellmeier entries
        if let Some((coeffs, range)) = self.sellmeier.get(material) {
            if in_range(wavelength_nm, *range) {
                let n = coeffs.refractive_index(wavelength_nm)?;
                return Ok(Complex64::new(n, 0.0));
            }
            return self.outside_vuv_range(material, wavelength_nm, *range);
        }

        // Tabulated VUV entries
        if let Some(table) = self.fixed_nk.get(material) {
            let range = table_range(table);
            if in_range(wavelength_nm, range) {
                return Ok(interpolate_nk(table, wavelength_nm));
            }
            return self.outside_vuv_range(material, wavelength_nm, range);
        }

        // Named X-ray/EUV/BEUV entries (Henke, valid 0.0413-41.3 nm).
        if let Some(m) = self.xray.get(material) {
            return m
                .refractive_index(wavelength_nm)
                .map_err(|e| self.rename_range_error(e, material, &m.formula));
        }

        // Generic Henke lookup: "henke:<formula>" or "henke:<formula>@<rho>".
        if let Some(spec) = material.strip_prefix("henke:") {
            let m = match spec.split_once('@') {
                Some((formula, rho)) => {
                    let rho: f64 = rho
                        .trim()
                        .parse()
                        .map_err(|_| LithographyError::MaterialNotFound(material.to_string()))?;
                    henke::Material::new(formula, rho)?
                }
                None => henke::Material::with_default_density(spec)?,
            };
            return m
                .refractive_index(wavelength_nm)
                .map_err(|e| self.rename_range_error(e, material, &m.formula));
        }

        Err(LithographyError::MaterialNotFound(material.to_string()))
    }

    /// Accepted wavelength ranges `(min, max)` in nm of a built-in entry, in
    /// increasing order: the Sellmeier/VUV-table range, plus the Henke range
    /// when the name falls back to Henke (see the module's range policy).
    /// `henke:` names return the Henke range. Errors for unknown names.
    pub fn wavelength_ranges_nm(&self, material: &str) -> crate::error::Result<Vec<(f64, f64)>> {
        let henke_range = (henke::HENKE_MIN_NM, henke::HENKE_MAX_NM);
        let vuv = self
            .sellmeier
            .get(material)
            .map(|(_, r)| *r)
            .or_else(|| self.fixed_nk.get(material).map(|t| table_range(t)));
        if let Some(r) = vuv {
            let mut out = Vec::with_capacity(2);
            if henke_fallback(material).is_some() {
                out.push(henke_range);
            }
            out.push(r);
            return Ok(out);
        }
        if self.xray.contains_key(material) || material.starts_with("henke:") {
            return Ok(vec![henke_range]);
        }
        Err(LithographyError::MaterialNotFound(material.to_string()))
    }

    /// Get dispersion dn/dlambda for Sellmeier materials (inside their
    /// accepted range only).
    pub fn dispersion(&self, material: &str, wavelength_nm: f64) -> crate::error::Result<f64> {
        if let Some((coeffs, range)) = self.sellmeier.get(material) {
            if in_range(wavelength_nm, *range) {
                return coeffs.dispersion(wavelength_nm);
            }
            return Err(LithographyError::WavelengthOutOfRange {
                material: material.to_string(),
                wavelength_nm,
                range_nm: *range,
                hint: "dn/dlambda is only defined for the Sellmeier entries inside their fit range"
                    .to_string(),
            });
        }
        Err(LithographyError::MaterialNotFound(material.to_string()))
    }

    /// The default single-resist-on-silicon film stack (vacuum above) at
    /// `wavelength_nm`, built from this database: `VUV_resist` on `Si` in the
    /// VUV tables' range, the `EUV_resist` stand-in on Si (Henke) in the
    /// X-ray/EUV range. Other wavelengths have no resist entry and fail with
    /// [`LithographyError::WavelengthOutOfRange`]; build a [`FilmStack`]
    /// explicitly there.
    pub fn resist_on_silicon_stack(
        &self,
        resist_thickness_nm: f64,
        wavelength_nm: f64,
    ) -> crate::error::Result<FilmStack> {
        let resist = if henke::wavelength_in_range(wavelength_nm) {
            "EUV_resist"
        } else {
            "VUV_resist"
        };
        let n_resist = self
            .refractive_index(resist, wavelength_nm)
            .map_err(|e| match e {
                LithographyError::WavelengthOutOfRange {
                    material,
                    wavelength_nm,
                    range_nm,
                    ..
                } => LithographyError::WavelengthOutOfRange {
                    material,
                    wavelength_nm,
                    range_nm,
                    hint: format!(
                        "the default resist-on-Si stack exists only in the VUV ({}-{} nm) and \
                     X-ray/EUV ({:.4}-{:.2} nm) ranges; build the film stack explicitly",
                        range_nm.0,
                        range_nm.1,
                        henke::HENKE_MIN_NM,
                        henke::HENKE_MAX_NM
                    ),
                },
                other => other,
            })?;
        let n_si = self.refractive_index("Si", wavelength_nm)?;
        Ok(FilmStack::new_vuv(
            vec![FilmLayer {
                name: "resist".to_string(),
                thickness_nm: resist_thickness_nm,
                n: n_resist,
            }],
            n_si,
        ))
    }

    /// Henke fallback for a VUV/Sellmeier name outside its range, or the
    /// typed range error with a hint naming the alternative.
    fn outside_vuv_range(
        &self,
        material: &str,
        wavelength_nm: f64,
        range_nm: (f64, f64),
    ) -> crate::error::Result<Complex64> {
        let hint = match henke_fallback(material) {
            Some(m) if henke::wavelength_in_range(wavelength_nm) => {
                return m.refractive_index(wavelength_nm);
            }
            Some(_) => format!(
                "'{material}' also covers {:.4}-{:.2} nm from the Henke/CXRO tables \
                 (same as 'henke:{material}'); there is no data in between",
                henke::HENKE_MIN_NM,
                henke::HENKE_MAX_NM
            ),
            None => {
                let euv: Vec<String> = self.xray_material_names();
                format!(
                    "no X-ray/EUV table for this material (Henke elements: {}); \
                     for 0.0413-41.3 nm use a named entry ({}) or 'henke:<formula>[@density]'",
                    henke::available_elements().join(" "),
                    euv.join(", ")
                )
            }
        };
        Err(LithographyError::WavelengthOutOfRange {
            material: material.to_string(),
            wavelength_nm,
            range_nm,
            hint,
        })
    }

    /// Re-label a Henke range error with the queried name and point at the
    /// VUV entry of the same formula, if any.
    fn rename_range_error(
        &self,
        e: LithographyError,
        material: &str,
        formula: &str,
    ) -> LithographyError {
        match e {
            LithographyError::WavelengthOutOfRange {
                wavelength_nm,
                range_nm,
                hint,
                ..
            } => {
                let hint = if self.sellmeier.contains_key(formula)
                    || self.fixed_nk.contains_key(formula)
                {
                    format!("{hint}; VUV values for {formula} are under '{formula}'")
                } else {
                    hint
                };
                LithographyError::WavelengthOutOfRange {
                    material: material.to_string(),
                    wavelength_nm,
                    range_nm,
                    hint,
                }
            }
            other => other,
        }
    }
}

impl Default for MaterialsDatabase {
    fn default() -> Self {
        Self::new()
    }
}

/// Henke material behind a VUV/Sellmeier name, when the name is itself a
/// formula of embedded elements with a CXRO default density.
fn henke_fallback(material: &str) -> Option<henke::Material> {
    henke::Material::with_default_density(material).ok()
}

/// Accepted range of a VUV table: its nodes plus the edge hold.
fn table_range(table: &[(f64, Complex64)]) -> (f64, f64) {
    (
        table[0].0 - TABLE_EDGE_HOLD_NM,
        table[table.len() - 1].0 + TABLE_EDGE_HOLD_NM,
    )
}

fn in_range(wavelength_nm: f64, (lo, hi): (f64, f64)) -> bool {
    (lo..=hi).contains(&wavelength_nm)
}

/// Linear interpolation in `n` and `k`; inside the edge hold
/// ([`TABLE_EDGE_HOLD_NM`]) the end value is returned. Callers check the
/// range first.
fn interpolate_nk(table: &[(f64, Complex64)], wavelength_nm: f64) -> Complex64 {
    if table.len() == 1 {
        return table[0].1;
    }
    if wavelength_nm <= table[0].0 {
        return table[0].1;
    }
    if wavelength_nm >= table[table.len() - 1].0 {
        return table[table.len() - 1].1;
    }

    let pos = table.partition_point(|(wl, _)| *wl < wavelength_nm);
    if pos == 0 {
        return table[0].1;
    }
    let (wl0, nk0) = table[pos - 1];
    let (wl1, nk1) = table[pos];
    let t = (wavelength_nm - wl0) / (wl1 - wl0);
    Complex64::new(
        nk0.re + t * (nk1.re - nk0.re),
        nk0.im + t * (nk1.im - nk0.im),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_caf2_lookup() {
        let db = MaterialsDatabase::new();
        let n = db.refractive_index("CaF2", 157.0).unwrap();
        assert!(n.re > 1.5 && n.re < 1.6);
        assert_relative_eq!(n.im, 0.0); // transparent
    }

    #[test]
    fn test_cr_lookup() {
        let db = MaterialsDatabase::new();
        let n = db.refractive_index("Cr", 157.0).unwrap();
        assert!(n.im > 1.0); // absorbing
    }

    #[test]
    fn test_unknown_material() {
        let db = MaterialsDatabase::new();
        assert!(db.refractive_index("Unobtainium", 157.0).is_err());
    }

    #[test]
    fn test_interpolation() {
        let db = MaterialsDatabase::new();
        // Hand interpolation of the Si table: 157 -> 0.88 + 2.10i,
        // 160 -> 0.90 + 2.12i; F2 line 157.63 nm is t = 0.63 / 3 = 0.21.
        let n = db.refractive_index("Si", 157.63).unwrap();
        assert_relative_eq!(n.re, 0.8842, epsilon = 1e-12);
        assert_relative_eq!(n.im, 2.1042, epsilon = 1e-12);
        // Nodes are reproduced exactly.
        assert_eq!(
            db.refractive_index("Cr", 140.0).unwrap(),
            Complex64::new(0.95, 1.80)
        );
    }

    #[test]
    fn test_table_edge_hold() {
        let db = MaterialsDatabase::new();
        // Two-point tables end at 157 nm; within TABLE_EDGE_HOLD_NM the end
        // value is held (the F2 line).
        let r = db.refractive_index("VUV_resist", 157.63).unwrap();
        assert_eq!(r, Complex64::new(1.65, 0.015));
        let r = db.refractive_index("AlF3", 125.2).unwrap();
        assert_eq!(r, Complex64::new(1.42, 0.001));
        // Beyond the hold: typed error (Xe2 excimer 172 nm, ArF 193 nm).
        for (name, wl) in [("VUV_resist", 172.0), ("Cr", 193.0), ("LaF3", 158.5)] {
            match db.refractive_index(name, wl) {
                Err(LithographyError::WavelengthOutOfRange {
                    material,
                    wavelength_nm,
                    ..
                }) => {
                    assert_eq!(material, name);
                    assert_eq!(wavelength_nm, wl);
                }
                other => panic!("{name} at {wl}: expected range error, got {other:?}"),
            }
        }
        assert_eq!(
            db.wavelength_ranges_nm("LaF3").unwrap(),
            vec![(125.0, 158.0)]
        );
    }

    #[test]
    fn test_vuv_names_fall_back_to_henke_in_euv() {
        // The P1 bug: "Si" at 13.5 nm used to return the 126 nm table value
        // 0.55 + 1.75i. CXRO calculator (henke.lbl.gov getdb, 2026-09-30,
        // default densities Si 2.33, Cr 7.19, SiO2 2.2): (name, delta, beta).
        let db = MaterialsDatabase::new();
        for (name, delta, beta) in [
            ("Si", 0.000998703763, 0.00182646094),
            ("Cr", 0.0751224086, 0.043694526),
            ("SiO2", 0.0219678637, 0.010773153),
        ] {
            let n = db.refractive_index(name, 13.5).unwrap();
            assert!(
                (1.0 - n.re - delta).abs() < 5e-4 * delta + 2e-6,
                "{name}: {n}"
            );
            assert!((n.im - beta).abs() < 5e-4 * beta + 2e-6, "{name}: {n}");
            assert_eq!(
                n,
                db.refractive_index(&format!("henke:{name}"), 13.5).unwrap()
            );
        }
        assert_eq!(
            db.refractive_index("Si", 13.5).unwrap(),
            db.refractive_index("Si_euv", 13.5).unwrap()
        );
        let ranges = db.wavelength_ranges_nm("Si").unwrap();
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[1], (125.0, 161.0));
    }

    #[test]
    fn test_out_of_range_errors_name_the_alternative() {
        let db = MaterialsDatabase::new();
        // Between the Henke range (<= 41.3 nm) and the VUV table: no data.
        let msg = db.refractive_index("Si", 80.0).unwrap_err().to_string();
        assert!(msg.contains("henke:Si") && msg.contains("125-161"), "{msg}");
        // Fluorides have no Henke table (no F): error, pointing at EUV keys.
        let msg = db.refractive_index("CaF2", 13.5).unwrap_err().to_string();
        assert!(msg.contains("Mo_euv") && msg.contains("henke:"), "{msg}");
        // The Henke entries point back at the VUV name of the same formula.
        let msg = db
            .refractive_index("Si_euv", 157.0)
            .unwrap_err()
            .to_string();
        assert!(msg.starts_with("Si_euv:") && msg.contains("'Si'"), "{msg}");
        let msg = db
            .refractive_index("henke:Mo", 193.0)
            .unwrap_err()
            .to_string();
        assert!(
            msg.starts_with("henke:Mo:") && msg.contains("0.0413-41.3281"),
            "{msg}"
        );
        // Non-physical wavelengths.
        assert!(db.refractive_index("Si", -1.0).is_err());
        assert!(db.refractive_index("CaF2", f64::NAN).is_err());
        assert!(matches!(
            db.wavelength_ranges_nm("Unobtainium"),
            Err(LithographyError::MaterialNotFound(_))
        ));
    }

    #[test]
    fn test_sellmeier_ranges() {
        let db = MaterialsDatabase::new();
        // Independent evaluation of the CaF2 Sellmeier (Malitson
        // coefficients) at 157.63 nm: 1.55702 (numpy, by hand from the
        // coefficients); measured CaF2 at 157 nm is ~1.559.
        let n = db.refractive_index("CaF2", 157.63).unwrap();
        assert!((n.re - 1.55702).abs() < 1e-5, "{n}");
        // Inside: Ar2 126 nm for CaF2/MgF2/LiF; outside: BaF2 at 126 nm
        // (edge ~134 nm), SiO2 at 157 nm (absorbs), anything at 13.5 nm or
        // in the IR beyond 2 um.
        for name in ["CaF2", "MgF2", "LiF"] {
            assert!(db.refractive_index(name, 126.0).is_ok(), "{name}");
        }
        for (name, wl) in [
            ("BaF2", 126.0),
            ("SiO2", 157.63),
            ("CaF2", 13.5),
            ("MgF2", 6.7),
            ("LiF", 2500.0),
        ] {
            assert!(
                matches!(
                    db.refractive_index(name, wl),
                    Err(LithographyError::WavelengthOutOfRange { .. })
                ),
                "{name} at {wl}"
            );
        }
        assert!(db.dispersion("CaF2", 157.0).is_ok());
        assert!(db.dispersion("CaF2", 13.5).is_err());
        // SiO2 at 13.5 nm falls back to Henke (amorphous SiO2, 2.2 g/cm^3).
        assert!(db.refractive_index("SiO2", 13.5).unwrap().re < 1.0);
    }

    #[test]
    fn test_resist_on_silicon_stack() {
        let db = MaterialsDatabase::new();
        let vuv = db.resist_on_silicon_stack(150.0, 157.63).unwrap();
        assert_eq!(vuv.layers[0].n, Complex64::new(1.65, 0.015));
        assert_relative_eq!(vuv.substrate.re, 0.8842, epsilon = 1e-12);
        let euv = db.resist_on_silicon_stack(50.0, 13.5).unwrap();
        assert_eq!(euv.layers[0].thickness_nm, 50.0);
        assert_eq!(
            euv.layers[0].n,
            db.refractive_index("EUV_resist", 13.5).unwrap()
        );
        assert_eq!(euv.substrate, db.refractive_index("Si_euv", 13.5).unwrap());
        let msg = db
            .resist_on_silicon_stack(100.0, 193.0)
            .unwrap_err()
            .to_string();
        assert!(msg.contains("build the film stack explicitly"), "{msg}");
    }

    #[test]
    fn test_euv_entries_match_cxro() {
        // CXRO on-line calculator (henke.lbl.gov getdb, 2026-09-30), default
        // densities: (name, lambda, delta, beta).
        let db = MaterialsDatabase::new();
        for (name, lambda, delta, beta) in [
            ("Si_euv", 13.5, 0.000998703763, 0.00182646094),
            ("Mo_euv", 13.5, 0.0762064755, 0.00643542456),
            ("Ru_euv", 13.5, 0.113639966, 0.0170648936),
            ("Ta_euv", 13.5, 0.0433237441, 0.0343398191),
            ("La_beuv", 6.69999981, 0.0161006916, 0.00136326044),
            ("B4C_beuv", 6.69999981, 0.00103260903, 0.000527112803),
        ] {
            let n = db.refractive_index(name, lambda).unwrap();
            assert!(
                (1.0 - n.re - delta).abs() < 5e-4 * delta + 2e-6,
                "{name}: {n}"
            );
            assert!((n.im - beta).abs() < 5e-4 * beta + 2e-6, "{name}: {n}");
        }
    }

    #[test]
    fn test_euv_entries_are_dispersive_and_bounded() {
        let db = MaterialsDatabase::new();
        // CXRO Mo scan: delta 0.0615031 at 12.5 nm, 0.0934877 at 14.5 nm.
        let a = db.refractive_index("Mo_euv", 12.5).unwrap();
        let b = db.refractive_index("Mo_euv", 14.5).unwrap();
        assert!((1.0 - a.re - 0.0615031272).abs() < 5e-5);
        assert!((1.0 - b.re - 0.0934876874).abs() < 5e-5);
        // Outside the Henke range the X-ray entries refuse to extrapolate.
        assert!(db.refractive_index("Mo_euv", 157.0).is_err());
    }

    #[test]
    fn test_henke_prefix_lookup() {
        let db = MaterialsDatabase::new();
        let a = db.refractive_index("henke:B4C@2.52", 6.7).unwrap();
        let b = db.refractive_index("B4C_beuv", 6.7).unwrap();
        assert_eq!(a, b);
        assert!(db.refractive_index("henke:Mo", 13.5).is_ok());
        assert!(db.refractive_index("henke:Xx", 13.5).is_err());
        assert!(db.refractive_index("henke:O", 13.5).is_err()); // no default density
        assert!(db.refractive_index("henke:Mo@abc", 13.5).is_err());
        assert!(db.material_names().contains(&"Mo_euv".to_string()));
        assert_eq!(db.xray_material("Mo_euv").unwrap().density_g_cm3, 10.22);
    }

    #[test]
    fn test_euv_mo_si_multilayer_pair() {
        let db = MaterialsDatabase::new();
        let si = db.refractive_index("Si_euv", 13.5).unwrap();
        let mo = db.refractive_index("Mo_euv", 13.5).unwrap();
        // At 92 eV all condensed matter has n slightly below 1.
        assert!(si.re < 1.0 && si.re > 0.99);
        assert!(mo.re < 1.0);
        // The canonical Mo/Si contrast: Mo absorbs > 3x more than Si and
        // has the larger index decrement (that is why the pair reflects).
        assert!(mo.im > 3.0 * si.im, "Mo k={} vs Si k={}", mo.im, si.im);
        assert!((1.0 - mo.re) > 10.0 * (1.0 - si.re));
    }

    #[test]
    fn test_euv_resist_absorption_ordering() {
        let db = MaterialsDatabase::new();
        let car = db.refractive_index("EUV_resist", 13.5).unwrap();
        let mox = db.refractive_index("MOx_resist", 13.5).unwrap();
        // Metal-oxide resists absorb several times more strongly than
        // organic CARs — their EUV selling point (2.75x for the stand-ins).
        assert!(mox.im > 2.5 * car.im);
        // Organic stand-in: alpha = 4 pi k / lambda ~ 5.2 /um at 13.5 nm.
        let alpha = 4.0 * std::f64::consts::PI * car.im / 13.5e-3;
        assert!((4.5..6.0).contains(&alpha), "alpha {alpha} /um");
    }

    #[test]
    fn test_beuv_boron_transparency() {
        let db = MaterialsDatabase::new();
        let la = db.refractive_index("La_beuv", 6.7).unwrap();
        let b4c = db.refractive_index("B4C_beuv", 6.7).unwrap();
        // 6.7 nm sits just below the boron K-edge: B4C is nearly
        // transparent while La provides the contrast.
        assert!(b4c.im < 0.001);
        assert!(la.im > 2.0 * b4c.im);
    }
}
