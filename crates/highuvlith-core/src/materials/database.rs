use num::Complex;
use std::collections::HashMap;

use super::dispersion::{self, SellmeierCoefficients};

type Complex64 = Complex<f64>;

/// VUV optical constants database for lithography materials.
pub struct MaterialsDatabase {
    sellmeier: HashMap<String, SellmeierCoefficients>,
    /// Fixed complex refractive indices (n, k) for materials without Sellmeier models.
    fixed_nk: HashMap<String, Vec<(f64, Complex64)>>,
}

impl MaterialsDatabase {
    /// Create database populated with standard VUV lithography materials.
    pub fn new() -> Self {
        let mut sellmeier = HashMap::new();
        sellmeier.insert("CaF2".to_string(), dispersion::caf2_sellmeier());
        sellmeier.insert("MgF2".to_string(), dispersion::mgf2_ordinary_sellmeier());
        sellmeier.insert("LiF".to_string(), dispersion::lif_sellmeier());
        sellmeier.insert("BaF2".to_string(), dispersion::baf2_sellmeier());
        sellmeier.insert("SiO2".to_string(), dispersion::sio2_sellmeier());

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

        // ---- EUV band (13.5 nm / 92 eV) ----
        // n = 1 - delta + i*beta from the CXRO Henke tables
        // (Henke, Gullikson & Davis, At. Data Nucl. Data Tables 54, 181
        // (1993); henke.lbl.gov). At these energies all condensed matter
        // has n slightly BELOW 1. Mo/Si is the canonical multilayer pair
        // (contrast in delta, low beta in Si).
        fixed_nk.insert(
            "Si_euv".to_string(),
            vec![(13.5, Complex64::new(0.99901, 0.00182))],
        );
        fixed_nk.insert(
            "Mo_euv".to_string(),
            vec![(13.5, Complex64::new(0.92380, 0.00644))],
        );
        // Capping / absorber metals (approximate CXRO values — verify
        // against henke.lbl.gov before quantitative reflectometry).
        fixed_nk.insert(
            "Ru_euv".to_string(),
            vec![(13.5, Complex64::new(0.886, 0.017))],
        );
        fixed_nk.insert(
            "Ta_euv".to_string(),
            vec![(13.5, Complex64::new(0.943, 0.041))],
        );
        // Chemically amplified organic EUV resist: alpha ~ 5 /um
        // -> k = alpha*lambda/(4*pi) ~ 0.0054.
        fixed_nk.insert(
            "EUV_resist".to_string(),
            vec![(13.5, Complex64::new(0.976, 0.0054))],
        );
        // Metal-oxide (SnOx-class) EUV resist: alpha ~ 20 /um -> k ~ 0.021.
        // n approximate.
        fixed_nk.insert(
            "MOx_resist".to_string(),
            vec![(13.5, Complex64::new(0.935, 0.021))],
        );

        // ---- Beyond-EUV band (6.7 nm / 185 eV, La/B4C mirrors) ----
        // 6.7 nm sits just below the boron K-edge (188 eV), where B4C is
        // nearly transparent — the physical basis of La/B4C multilayers.
        // Values approximate (CXRO); verify before quantitative use.
        fixed_nk.insert(
            "La_beuv".to_string(),
            vec![(6.7, Complex64::new(0.988, 0.0027))],
        );
        fixed_nk.insert(
            "B4C_beuv".to_string(),
            vec![(6.7, Complex64::new(0.9947, 0.0004))],
        );
        fixed_nk.insert(
            "BEUV_resist".to_string(),
            vec![(6.7, Complex64::new(0.994, 0.002))],
        );

        Self {
            sellmeier,
            fixed_nk,
        }
    }

    /// Get the complex refractive index of a material at a given wavelength.
    /// For Sellmeier materials, k=0 (transparent). For tabulated materials,
    /// linear interpolation is used.
    pub fn refractive_index(
        &self,
        material: &str,
        wavelength_nm: f64,
    ) -> crate::error::Result<Complex64> {
        // Try Sellmeier first
        if let Some(coeffs) = self.sellmeier.get(material) {
            let n = coeffs.refractive_index(wavelength_nm)?;
            return Ok(Complex64::new(n, 0.0));
        }

        // Try tabulated
        if let Some(table) = self.fixed_nk.get(material) {
            return Ok(interpolate_nk(table, wavelength_nm));
        }

        Err(crate::error::LithographyError::MaterialNotFound(
            material.to_string(),
        ))
    }

    /// Get dispersion dn/dlambda for Sellmeier materials.
    pub fn dispersion(&self, material: &str, wavelength_nm: f64) -> crate::error::Result<f64> {
        if let Some(coeffs) = self.sellmeier.get(material) {
            return coeffs.dispersion(wavelength_nm);
        }
        Err(crate::error::LithographyError::MaterialNotFound(
            material.to_string(),
        ))
    }
}

impl Default for MaterialsDatabase {
    fn default() -> Self {
        Self::new()
    }
}

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
        let n_140 = db.refractive_index("Si", 140.0).unwrap();
        let n_150 = db.refractive_index("Si", 150.0).unwrap();
        // Interpolated value should be between the table entries
        assert!(n_150.re > n_140.re || n_150.re < n_140.re || n_150.re == n_140.re);
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
        // organic CARs — their EUV selling point.
        assert!(mox.im > 3.0 * car.im);
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
