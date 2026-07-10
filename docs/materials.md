# Materials Database

**Status:** 🔶 Simplified — mature Sellmeier fits and VUV tables, canonical CXRO anchors for Mo/Si at 13.5 nm, but the newer EUV/BEUV entries are single-point approximations: verify against [henke.lbl.gov](https://henke.lbl.gov/optical_constants/) before quantitative use.

## Overview

Optical constants decide everything downstream of the aerial image: thin-film standing waves, resist coupling, mirror transmission, LIGA depth dose. highuvlith carries three material layers, each matched to the physics that consumes it:

1. **[`MaterialsDatabase`](../crates/highuvlith-core/src/materials/database.rs)** — complex refractive index n + ik by wavelength, backed by Sellmeier dispersion models ([dispersion.rs](../crates/highuvlith-core/src/materials/dispersion.rs)) for transparent crystals and tabulated (λ, n, k) points for everything else. Queried by the CLI `materials` command; the built-in thin-film `FilmStack` presets carry matching **hardcoded** n + ik values rather than reading the database at runtime — to feed the transfer matrix from the database, build `FilmLayer`s from `refractive_index` lookups yourself.
2. **[X-ray mass attenuation](../crates/highuvlith-core/src/materials/attenuation.rs)** — per-element μ/ρ over 0.1–20 keV with a mixture rule, consumed by the LIGA deep X-ray module (`deep_xray.rs`).
3. **[Diamond](../crates/highuvlith-core/src/materials/diamond.rs)** — substrate/window niceties: Sellmeier index, thermal properties, film-stack presets, X-ray window transmission.

## Refractive-index coverage matrix

`MaterialsDatabase::refractive_index(name, λ_nm)` tries Sellmeier first (k = 0, transparent), then the tabulated store with **linear interpolation in both n and k**; wavelengths outside a table clamp to the endpoints, single-point entries return a constant, and unknown names return `MaterialNotFound`.

### Sellmeier models (dispersive, k = 0)

$$
n^2(\lambda) = 1 + \sum_i \frac{B_i\, \lambda^2}{\lambda^2 - C_i} \qquad (\lambda\ \text{in}\ \mu\mathrm{m})
$$

`dispersion(name, λ)` also returns dn/dλ analytically (surfaced by the CLI `materials` command). It quantifies CaF2's steep VUV dispersion — the physical reason for the large axial chromatic defocus at 157 nm — but note the 15 nm/pm default in `ProjectionOptics` is an independent hardcoded constant, not derived from this function at runtime. Resonance (λ² → C_i) and unphysical n² < 0 are rejected as `NumericalError` rather than returning NaN.

| Material | Validity | Provenance | Status |
|----------|----------|------------|--------|
| CaF2 | ~130 nm – 10 µm | Daimon & Masumura, *Appl. Opt.* 41, 5275 (2002) | ✅ mature |
| MgF2 (ordinary ray) | VUV – IR | Dodge, *Appl. Opt.* 23, 1980 (1984) | ✅ mature |
| LiF | VUV – IR | Li, *J. Phys. Chem. Ref. Data* 5, 329 (1976) | ✅ mature |
| BaF2 | VUV – IR | standard Sellmeier fit | ✅ mature |
| SiO2 (fused silica) | ≥ ~165 nm only — absorbs below; DUV reference | standard fit | ✅ (mind the validity floor) |
| Diamond (type IIa) | ~225 nm – far IR | Peter (1923); in `diamond.rs`, not the database map | ✅ mature |

### Tabulated n + ik — VUV band (126–160 nm) 🔶 mature

Two-to-four-point tables, linearly interpolated. These are the materials of the F2/Ar2 thin-film stacks — though the built-in `FilmStack` presets hardcode the same n + ik values rather than querying the database.

| Material | Role | Entries (λ nm) |
|----------|------|----------------|
| Cr | mask absorber | 126, 140, 157, 160 |
| Si | substrate | 126, 140, 157, 160 |
| AlF3, Na3AlF6 (cryolite) | low-n AR coatings | 126, 157 |
| LaF3, GdF3 | high-n HR coatings | 126, 157 |
| VUV_resist | generic fluoropolymer resist | 126, 157 |
| VUV_BARC | bottom AR coating | 126, 157 |

All VUV metal/coating values are approximate transcriptions (marked so in the source); they are physically ordered (Cr strongly absorbing, fluorides near-transparent) and adequate for standing-wave and reflectance studies.

### Tabulated n + ik — EUV band (13.5 nm / 92 eV) — NEW

Single-point entries, n = 1 − δ + iβ from the CXRO Henke tables. At 92 eV all condensed matter has n slightly **below** 1.

| Material | n, k at 13.5 nm | Provenance | Status |
|----------|-----------------|------------|--------|
| `Si_euv` | 0.99901, 0.00182 | CXRO canonical | ✅ canonical anchor |
| `Mo_euv` | 0.92380, 0.00644 | CXRO canonical | ✅ canonical anchor |
| `Ru_euv` | 0.886, 0.017 | CXRO, approximate | 🔶 verify vs henke.lbl.gov |
| `Ta_euv` | 0.943, 0.041 | CXRO, approximate | 🔶 verify vs henke.lbl.gov |
| `EUV_resist` | 0.976, 0.0054 | organic CAR, α ≈ 5/µm → k = αλ/4π | 🔶 approximate |
| `MOx_resist` | 0.935, 0.021 | SnOx-class, α ≈ 20/µm | 🔶 approximate |

The Mo/Si pair is the physics anchor: Mo has > 3× the absorption and > 10× the index decrement of Si — exactly the contrast that makes Mo/Si multilayers reflect (pinned by `test_euv_mo_si_multilayer_pair`).

### Tabulated n + ik — BEUV band (6.7 nm / 185 eV) — NEW, 🔶 approximate

6.7 nm sits just **below the boron K-edge (188 eV)**, where B4C is nearly transparent — the physical basis of La/B4C multilayer mirrors. Entries: `La_beuv` (0.988, 0.0027), `B4C_beuv` (0.9947, 0.0004), `BEUV_resist` (0.994, 0.002). All approximate CXRO transcriptions; **verify against henke.lbl.gov before quantitative use**.

## X-ray mass attenuation (LIGA, 0.1–20 keV) 🔶

[attenuation.rs](../crates/highuvlith-core/src/materials/attenuation.rs) tabulates per-element μ/ρ [cm²/g] on a shared 17-node log-spaced grid (0.1 → 20 keV) with **log-log interpolation** — exact for the pure power law μ ∝ E⁻³ that photoelectric absorption follows between edges. Elements covered (the LIGA set): **H, C, N, O, Si, Au, Be, Ti**. Values are transcribed from NIST XCOM/FFAST total-attenuation tables.

Compounds follow the mixture rule with mass fractions validated to sum to 1 within 1%:

$$
\left(\frac{\mu}{\rho}\right)_{\!\mathrm{comp}} = \sum_i w_i \left(\frac{\mu}{\rho}\right)_{\!i}, \qquad \mu\,[1/\mu\mathrm{m}] = \frac{(\mu/\rho)\,\rho}{10^4}
$$

Presets: `Compound::pmma()` (C5H8O2, ρ = 1.19), `Compound::su8()` (**approximate** C22H22O4 stoichiometry — the real cross-linked network is more complex), and pure `gold(ρ)` / `beryllium(ρ)` / `titanium(ρ)` / `silicon(ρ)` taking the plated/bulk density as an argument (electroplated Au is often below the 19.3 g/cm³ bulk value).

Honesty notes, exactly as documented in the module:

- **Edges are bracketed, not resolved.** The grid deliberately straddles the Au M-edges (~2.2–3.4 keV) and L-edges (11.9–14.4 keV), Ti K (4.966 keV), Si K (1.839 keV), and the C/N/O K-edges; the interpolation smooths near-edge XANES/EXAFS structure and the exact edge position. Sub-keV values below each K-edge are approximate.
- **μ_en ≈ μ assumption.** In this band the photoelectric effect dominates for low-Z resist elements, so the energy-absorption coefficient is taken equal to the attenuation coefficient (photoelectrons deposit locally; fluorescence/Compton down-scatter minor). LIGA dose modeling in `deep_xray.rs` relies on this.
- Energies outside 0.1–20 keV clamp to the endpoints — no extrapolation.
- The multi-keV anchors that matter for LIGA are pinned by tests: PMMA μ/ρ ≈ 6.5 cm²/g at 8 keV and ≈ 125 at 3 keV; C ≈ 4.58 and Au ≈ 111 at 8 keV.

## Diamond substrate module ✅ / 🔶

[diamond.rs](../crates/highuvlith-core/src/materials/diamond.rs) covers diamond as substrate, membrane, and X-ray window:

- **Index:** `diamond_sellmeier()` (Peter 1923, valid ≥ ~225 nm; n ≈ 2.417 at 589 nm). Below 225 nm (just under the 5.47 eV bandgap cutoff ≈ 227 nm) the code substitutes a fixed n = 2.7 — a documented approximation for the absorbing regime, so VUV resist-on-diamond stacks are qualitative.
- **Thermal:** `DiamondProperties` (2200 W/m·K, α = 1e-6/K, ρ = 3.515 g/cm³) with a simple max-dose-before-distortion estimate (`max_dose_mj_cm2`).
- **Film stacks:** `resist_on_diamond()` and `diamond_on_silicon()` presets returning `FilmStack`s for the transfer-matrix engine.
- **X-ray window:** `xray_transmission(thickness, E)` uses a standalone E⁻³ scaling from μ/ρ ≈ 4.6 cm²/g at 8 keV — 🔶 approximate and *separate from* the attenuation module's carbon table (no edges; fine above ~2 keV where diamond windows live).

## How to add a material

- **Transparent crystal (dispersive):** add a `SellmeierCoefficients` constructor in [dispersion.rs](../crates/highuvlith-core/src/materials/dispersion.rs) with a literature citation, register it in `MaterialsDatabase::new()` via `sellmeier.insert(...)`, and add a test pinning n at a published wavelength (see `test_caf2_at_157nm`).
- **Absorbing material (tabulated):** add `(λ_nm, n + ik)` points to `fixed_nk` in [database.rs](../crates/highuvlith-core/src/materials/database.rs), sorted ascending in λ; for EUV/X-ray take δ, β from CXRO (n = 1 − δ + iβ) and say so in the comment, with the approximate/canonical distinction. Add an ordering test that pins the physics (e.g. `test_beuv_boron_transparency`).
- **LIGA element:** add a 17-value μ/ρ array matching `ENERGY_GRID_KEV` in [attenuation.rs](../crates/highuvlith-core/src/materials/attenuation.rs) (NIST XCOM/FFAST), a static `ElementAttenuation`, and a match arm in `element()`; note which edges the grid brackets and pin at least one multi-keV anchor in tests.

## Usage

```bash
# CLI: query the built-in database at a wavelength
highuvlith materials --wavelength 157
```

```rust
use highuvlith_core::materials::database::MaterialsDatabase;
use highuvlith_core::materials::attenuation::Compound;

let db = MaterialsDatabase::new();
let n_caf2 = db.refractive_index("CaF2", 157.63)?;   // Sellmeier, k = 0
let n_mo   = db.refractive_index("Mo_euv", 13.5)?;   // 0.92380 + 0.00644i
let dn     = db.dispersion("CaF2", 157.63)?;          // dn/dlambda [1/nm]

let pmma = Compound::pmma();
let mu   = pmma.mu_per_um(8.0);                       // ~6.5 * 1.19 / 1e4 per um
```

## Validation

- [database.rs](../crates/highuvlith-core/src/materials/database.rs): `test_caf2_lookup`, `test_cr_lookup`, `test_unknown_material`, `test_interpolation`, `test_euv_mo_si_multilayer_pair`, `test_euv_resist_absorption_ordering` (MOx > 3× CAR absorption), `test_beuv_boron_transparency`.
- [dispersion.rs](../crates/highuvlith-core/src/materials/dispersion.rs): `test_caf2_at_157nm` (n ≈ 1.559), `test_caf2_at_visible` (n ≈ 1.434 at 589 nm), `test_dispersion_negative`, `test_caf2_dispersion_steeper_at_vuv`, `test_mgf2_reasonable_index`, `test_sellmeier_resonance_error`.
- [attenuation.rs](../crates/highuvlith-core/src/materials/attenuation.rs): `test_interpolation_exact_at_nodes`, `test_clamp_outside_grid`, `test_pmma_anchor_8kev`, `test_pmma_anchor_3kev`, `test_element_anchors`, `test_mixture_rule_sum_validation`, `test_carbon_monotone_decreasing_1_to_20_kev`, `test_log_log_between_nodes`, `test_mu_per_um_units`.
- [diamond.rs](../crates/highuvlith-core/src/materials/diamond.rs): `test_diamond_refractive_index`, `test_diamond_uv_cutoff`, `test_diamond_thermal_capacity`, `test_xray_transmission_high_energy`, `test_xray_transmission_low_energy`, `test_resist_on_diamond_stack`.

## References

- B. L. Henke, E. M. Gullikson & J. C. Davis, "X-ray interactions: photoabsorption, scattering, transmission, and reflection at E = 50–30,000 eV, Z = 1–92," *At. Data Nucl. Data Tables* **54**, 181 (1993) — the CXRO tables, [henke.lbl.gov](https://henke.lbl.gov/optical_constants/).
- NIST XCOM / FFAST photon cross-section databases — attenuation provenance.
- M. Daimon & A. Masumura, "High-accuracy measurements of the refractive index of calcium fluoride…," *Appl. Opt.* **41**, 5275 (2002).
- M. J. Dodge, "Refractive properties of magnesium fluoride," *Appl. Opt.* **23**, 1980 (1984).
- H. H. Li, "Refractive index of alkali halides…," *J. Phys. Chem. Ref. Data* **5**, 329 (1976).
- F. Peter, "Über Brechungsindizes und Absorptionskonstanten des Diamanten," *Z. Phys.* **15**, 358 (1923).
