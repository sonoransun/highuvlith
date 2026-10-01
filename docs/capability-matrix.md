# Capability status matrix

This page is the **single source of truth** for what highuvlith actually
computes versus what it parameterizes, projects, or merely plans. Every
capability-changing PR must update the relevant row here, the module header's
`# Model status` section, and the `**Status:**` line of the capability's
documentation page **in the same PR** (see [CONTRIBUTING.md](../CONTRIBUTING.md)).

**Last audited: 2026-10-01** — every row below was re-checked against the code
(module `# Model status` headers, tests, and the validation numbers the work
packages recorded) after the v2 physics overhaul and its 2026-10-01 follow-up
were merged.

## Status taxonomy

| Badge | Name | Definition |
|-------|------|------------|
| ✅ | **Implemented** | Computed by code, covered by tests, physics validated against analytical results |
| 🔶 | **Simplified** | Runs end-to-end but with documented approximations or reduced dimensionality; the assumption is stated inline wherever the capability is described |
| 🧪 | **Theoretical** | Parameterized and runnable, but models speculative physics; outputs are research projections, not validated engineering |
| 🗺️ | **Planned** | Not in code; roadmap only. Never described in prose as if it runs |

A split badge such as ✅/🔶 means the named parts are exact and tested while the
rest of the capability rests on the stated approximation. Model-coverage tables
on source/process pages additionally tag each struct field as `live` (enters a
computation), `stored (inert)` (carried and exposed, awaiting a consumer), or
`planned` (not yet a field).

## What changed in this revision

The 2026-09-30 audit corrected the following defects. "Before" describes what
the code did, not what the documentation claimed.

| Defect | Before | Now |
|---|---|---|
| Dropped diffraction orders | The TCC kept mask frequencies only up to NA/λ; orders that only off-axis source points steer into the pupil (up to (1 + σ)·NA/λ) were discarded, so every pitch between λ/((1 + σ)NA) and λ/NA imaged flat (λ 100 nm, NA 0.5, σ 0.7, pitch 160 nm: contrast 0) | Mask support (1 + σ_max)·NA/λ; the same case images at contrast 0.483; SOCS matches a direct Abbe sum to ≤ 1e-8 (`tests/imaging_validation.rs`) |
| Per-frequency defocus | Defocus was a phase per mask frequency, exp(iπzλ\|f\|²), instead of inside the pupil per source point, and paraxial; a symmetric dipole's two-beam image lost contrast 0.89 → 0.15 at 100 nm defocus | Defocus evaluated in the pupil at every source point with the exact (2πn/λ)·z·(1 − √(1 − (NAρ/n)²)) phase; the dipole stays at 0.89. The old behaviour survives as the opt-in `DefocusModel::KernelPhase` |
| Wrong eigenvectors | nalgebra 0.33's `SymmetricEigen` returned an orthonormal but wrong eigenbasis for some complex Hermitian (defocused) TCCs | Own Hermitian solver (`math/linalg.rs`, Householder + implicit QL), reconstruction ≤ 1e-13 |
| Incommensurate masks | `line_space` always drew 10 periods and `contact_hole` a 5 × 5 array whatever the field, so the periodic FFT imaged a different pattern ((90, 190) and (100, 200) gave identical images; 65/180 at NA 0.75 read contrast 0.854) | Periodic `LineSpace` / `RectArray` primitives, commensurate grids chosen automatically by the Python wrappers and CLI, a `UserWarning` otherwise; 65/180 now 0.4545 vs an independent Abbe reference 0.456 |
| Inverted line/space geometry | `line_space(cd, pitch)` painted absorber of width pitch − cd (a nominal 100 nm line measured 199 nm) | Opaque lines of width `cd`, one centred at x = 0 (same case: 107.0 nm on the old engine, 97.76 nm on the new one vs 97.95 nm Abbe) |
| Dose-blind process window | Every dose row of the focus–exposure matrix was identical | Constant-threshold resist E_th/d with continuous dose limits, ED windows, DOF@EL; row is now 🔶 (it was listed ✅) |
| Monopole "dipole" | `IlluminationShape::Dipole` placed both poles at orient ± π — the same point — so it was a single off-axis pole | Poles at orient and orient + π (`test_dipole_has_two_symmetric_poles`); dipole results computed before the fix used a single pole — the SRAF dipole DOF gains (×1.10–1.23) were withdrawn, a true dipole gives ×1.01–1.05 |
| Bending-magnet spectrum weighting | LIGA photon weights used the per-relative-bandwidth function G₁(y) as photons per equal-width bin (a spectrum too hard by a factor E), and dose used μ instead of μ_en | Log-spaced bins weighted by G₁, NIST μ_en for deposition; unfiltered LIGA top/bottom dose ratio 7.4 → 20.8, validated against an independent numpy implementation |
| Au attenuation | The gold μ/ρ table was ~1.9× too transparent at 4–20 keV (111 vs NIST 207 cm²/g at 8 keV) | NIST Hubbell–Seltzer μ/ρ and μ_en/ρ (1 keV–20 MeV) + CXRO/Henke below 1 keV |
| Thin-film reflectance | Absorbing layers used n + ik inside an n − ik characteristic matrix: the default 150 nm resist on Si at 157 nm reflected 0.772 instead of 0.258, and a 50 nm k = 0.3 film reflected 2.75 | q-formulation transfer matrix validated against the Airy formula, Fresnel, R + T = 1 and X-ray total external reflection |
| Kostroun S(y) accuracy | Quadrature step 0.5 gave ~1e-8 near the peak and 7e-4 at y = 10, while documented as 1e-10 | Step 0.25, ≤ 3e-12 against 30-digit mpmath |
| MNSL moiré period | `calculate_moire_period` returned p/(2 sin θ), about half the true period at small angles, ignoring the second pitch when rotated | p₁p₂/√(p₁² + p₂² − 2p₁p₂ cos θ), checked against the FFT beat of two rotated gratings; the MNSL row is re-graded 🧪 (see below) |
| `SourceKind` photon density | The type-erased `SourceKind` used by Python, the CLI and the GUI did not forward `photon_energy_ev` / `photon_density_per_mj_cm2`, so the X-ray tube's photons per dose (set from its mean photon energy) came out 1.59× high there | Every trait method is forwarded (`test_source_kind_forwards_every_trait_method`) |
| Quantum-lithography resolution claim | The module claimed λ/(2N) resolution, but its image was the classical image raised to the N-th power, which keeps the classical period | Classical N-photon absorption and the ideal N00N model are kept apart: closed-form two-beam N00N fringes at λ/(2N sin θ) and Boto's ideal λ/N limit for general masks, mixed with classical N-photon absorption by a fidelity |
| Plasma-source power definition | The Sn LPP preset reported 68.75 W "at wafer", contradicting its own 250 W at intermediate focus (IF) and double-counting optics in any throughput estimate | `average_power_w()` of plasma sources is the in-band power at IF (NXE:3400B chain: 21.5 kW CO₂, 6 % CE → 250 W) |
| DSA overclaims | `simulate_dsa_2d` ignored its template and always reported zero defects; the 1D "pitch estimate" was the first region's width | Registration in trenches, commensurability penalty, defect index labelled an uncalibrated heuristic (`NaN` when not evaluated) |
| GIL / zero-copy claims | Docs claimed zero-copy NumPy results and GIL-free compute, but every result was copied and the single-image `SimulationEngine` calls held the GIL | GIL released in every compute path; the large result arrays are zero-copy read-only NumPy views (writes need `.copy()`); details per result in the [Python API reference](./python-api.md#runtime-behaviour-gil-arrays-honesty-fields) |

### 2026-10-01 follow-up

| Defect | Before | Now |
|---|---|---|
| Volumetric exposing intensity | `expose_volumetric` used the thin-film \|E(z)\|² (unit incident field) as the exposing intensity, so every resist with n_r ≠ n_0 was under-dosed by 1/n_r: ×0.61 for the default n = 1.65 resist in vacuum (a lossless entrance gave 0.570 instead of T = 0.940) | S(z) = (n_r/n_0)·\|E(z)\|², checked against a numpy closed form; index-matched stacks (all earlier tests) are unchanged |
| XFEL machine overrides | A TOML override of the machine, `wavelength_nm` or `pulse_duration_fs` re-derived ρ and the saturation power but kept the preset's pulse energy, so average power ignored the machine (ERL at 600 A kept 10.5 kW) | The pulse energy is rescaled at the preset's calibration, E = E_preset·E_sat(new)/E_sat(preset) (ERL at 600 A: 64.84 → 195.04 µJ, 31.7 kW); an explicit `pulse_energy_uj` wins; unchanged configs are bit-identical |
| Silent material extrapolation | VUV tables and Sellmeier fits were evaluated at any wavelength: `Si` at 13.5 nm returned the VUV value 0.55 + 1.75i | Every entry has a stated range; Si, Cr and SiO₂ fall back to Henke data in the EUV (Si at 13.5 nm: 0.999001 + 0.001826i), anything else out of range raises `WavelengthOutOfRange` (Python `ValueError`, CLI exit 1); see [materials.md](./materials.md#range-policy-no-silent-extrapolation) |
| CLI `[deep]` volumetric defaults | No bake (`peb = "none"`) and 300 nm resist, unlike Python and the core; with standing waves and no bake, development stalled at the first node about 23 nm down | `peb = "gaussian"` at the resist's `peb_diffusion_nm` (30 nm on both axes, as in Python) and 150 nm resist (the core `FilmStack` default); `peb = "none"` still turns the bake off |
| Dark-field example configs | `sim_sxrl.toml` and `sim_synchrotron.toml` imaged through a Schwarzschild NA 0.3 objective in a dark-field configuration (clear field below 1 % of the pupil peak), so their images were not the intended bright-field ones | Both image through `euv_projection` (NA 0.33) and are bright-field; every `examples/*.toml` is run in the CLI test suite |
| GUI HHG power | The GUI built HHG sources without a driver power, so it showed the driverless default instead of the core's derived power | The driver power is passed through (Ne preset 0.8995 µW, the core value) |

Capabilities added in the same overhaul are listed in their rows (vector
imaging, per-wavelength TCCs, immersion and EUV projection optics, clear-field
normalization, an opt-in multilayer pupil, exact mask spectra, five new source families plus the DUV/UV
heritage presets, a throughput calculator, Fresnel LIGA diffraction with
absolute exposure time, CXRO optical constants and multilayer mirrors,
level-set development, anisotropic and chemically amplified bakes,
Talbot/EUV-IL, true-adjoint ILT, fragment OPC, SRAF insertion and SADP/SAQP);
the corresponding roadmap items were deleted from the [roadmap](./roadmap.md).

## Core imaging

| Capability | Status | Assumptions / limits | Code |
|------------|--------|----------------------|------|
| Hopkins imaging (factorized TCC/SOCS) | ✅ | Exact scalar Hopkins imaging of a thin mask: TCC = A·Aᴴ over an adaptive source sampling, mask support (1 + σ)·NA/λ, dense or Gram eigendecomposition (randomized above the dense limit), captured energy reported with every kernel set; matches a direct Abbe sum to ≤ 1e-8 and resolution / dipole-DOF closed forms. Scalar is 🔶 at NA ≳ 0.8 — use vector mode | `aerial.rs`, `math/linalg.rs` |
| Defocus / through-focus imaging | ✅ | Exact non-paraxial defocus phase inside the pupil per source point, in the image-space medium (immersion); kernel sets cached per (focus, λ); `compute_through_focus` images one spectrum at many planes in parallel. Legacy `DefocusModel::KernelPhase` is 🔶 (on-axis coherent only) | `aerial.rs`, `optics/mod.rs::defocus_phase_in_medium` |
| Broadband imaging (`compute_multiwavelength`) | ✅ | TCC rebuilt at every spectral sample (cutoff, source points, pupil and chromatic focus at λᵢ), incoherent weighted sum; equals the weighted sum of single-line engines to 1e-12. Honest for HHG combs and wide bands | `aerial.rs` |
| Narrow-band polychromatic (`compute_polychromatic`) | 🔶 | Focus shift per spectral sample with the center-λ TCC — valid for Δλ/λ ≪ 1 only | `aerial.rs` |
| Vector (polarized) high-NA imaging | ✅ | TE/TM decomposition per diffraction order, radiometric obliquity (cos θ_obj/cos θ_img)^½, immersion index, film-entrance Fresnel transmission (complex Snell); unpolarized/X/Y/linear/TE/TM illumination. Validated against two-beam closed forms (TE 1, TM \|cos 2θ\|), Fresnel and energy conservation. Thin mask (no EMF), ideal lens (no Jones pupil), paraxial reticle side, no multiple film reflections. Rust `ImagingModel::Vector`; Python `SimulationEngine(vector=VectorSettings(...))`; CLI `[imaging.vector]` | `optics/vector.rs`, `aerial.rs` |
| Image normalization | ✅ | Relative intensity by default (÷ the exact clear-field TCC(0,0) of the same illumination, optics and focus); `normalization="absolute"` and `clear_field_intensity()` for radiometry; dark-field configurations (clear field < 1 % of the pupil peak) stay absolute and are flagged | `aerial.rs` |
| Illumination pupil fills | ✅ | Conventional, annular, dipole, quadrupole (axis-aligned poles) and the graded `CoherentGaussian` fill, sampled adaptively on a rotated (atan 1/φ) lattice with area-weighted edge cells and refined peaked fills; contrast within 0.003 (partially coherent, k₁ onset) / 0.02 (σ ≈ 0.05, through the cutoff) of fine-quadrature references (`tests/imaging_source_sampling.rs`); the dipole rendered a single pole before 2026-09-30 | `source.rs::evaluate_illumination`, `aerial.rs::sample_source`, `tests/imaging_source_sampling.rs` |
| Flare | 🔶 | Zeroth-order uniform flare (constant fraction of the mean intensity); no point-spread flare model | `aerial.rs` |
| Parallel, deterministic compute | ✅ | Rayon over kernels, planes and wavelengths with fixed reduction grouping — images are bit-reproducible regardless of thread count; serial build via `--no-default-features` | `compute/parallel.rs` |

## Optical systems

| Capability | Status | Assumptions / limits | Code |
|------------|--------|----------------------|------|
| Refractive projection optics (dry) | ✅ | Scalar pupil, user Zernike aberrations, apodization, linear axial chromatic coefficient (the default 15 nm/pm is a CaF₂/157 nm figure); NA < 1 | `optics/mod.rs::ProjectionOptics` |
| Immersion lithography (193i) | ✅ | `ProjectionOptics::immersion` / `immersion_193i` (NA 1.35, water n = 1.437, NA ≤ 0.95·n), defocus phase in the medium; 90 nm pitch at 193 nm (annular 0.7–0.9) images at contrast 0.368 vs 0 with dry NA 0.93. No water absorption/temperature model; focus inside the resist index not modeled | `optics/mod.rs` |
| EUV scanner projection optics (NA 0.33 / High-NA 0.55) | 🔶 | Isotropic wafer-side pupil with optional central obscuration (0.2·NA assumed for the High-NA preset, not a published figure); anamorphic 4×/8× magnification, mask 3D, multilayer apodization and the non-telecentric chief ray are not modeled; 18 nm pitch at 13.5 nm: NA 0.33 → 0, NA 0.55 → 0.495 | `optics/euv.rs` |
| Schwarzschild reflective objective | ✅ | Annular (centrally obscured) scalar pupil; mirror reflectance a scalar constant by default (optional multilayer pupil, next row); no figure error | `optics/schwarzschild.rs` |
| EUV/BEUV multilayer pupil (opt-in) | 🔶 | Angle-dependent Mo/Si, La/B₄C or La/B amplitude and phase across the pupil of the Schwarzschild or EUV projection optics, from user-supplied incidence-angle maps per mirror (no design ray trace in the repository); one coating and one s/p basis for all mirrors, polarization-averaged amplitude even in vector mode. With clear-field normalization it acts as apodization and focus-shifting phase, not as a dose change | `optics/multilayer_pupil.rs` |
| Fresnel zone plate | ✅ | First-order pupil with cutoff 1/(2Δr_N) and Rayleigh 1.22·Δr_N; constant efficiency; other orders only through the flare fraction; strong chromatic defocus Δf/f = Δλ/λ applied once (engine side) | `optics/zone_plate.rs` |
| Zernike aberrations | ✅ | Fringe (University of Arizona) indexing Z1–Z37; indices 38–48 are a project-specific extension, not standard Fringe terms; beyond 48 an approximate fallback | `math/zernike.rs` |
| Optics honesty guard | ✅ | CLI warns when refractive optics are selected below 50 nm and supports `type = "schwarzschild" \| "zone_plate" \| "euv_projection"`; the GUI's automatic optics switch from refractive to EUV projection optics below 110 nm | `highuvlith-cli/src/config.rs`, `highuvlith-gui` |

## Masks, metrics and process window

| Capability | Status | Assumptions / limits | Code |
|------------|--------|----------------------|------|
| Exact thin-mask spectrum | ✅ | Closed-form Fourier-series spectra for rectangles, gray rectangles, periodic L/S and contact lattices and simple polygons (overlaps resolved exactly); validated against brute-force quadrature (1e-9), the sub-pixel shift theorem and pixel-aligned FFT identities. Raster fallback (first order in the pixel) only for self-intersecting or overlapping polygons | `mask.rs::spectrum` |
| Commensurate periodic grids | ✅ | `GridConfig::commensurate` / `Mask::commensurate_grid` / `check_commensurate`; applied automatically by the Python wrappers and the CLI, `UserWarning` otherwise | `mask.rs`, `types.rs` |
| Antialiased rasterization | ✅ | Exact area coverage for every feature type; `rasterize_intensity` gives ⟨\|t\|²⟩ for shadow consumers | `mask.rs` |
| Mask model and types | 🔶 | Kirchhoff thin mask: binary and attenuated PSM are modeled; `AlternatingPSM` is accepted but treated as binary (no 0/180° phase regions); no mask 3D / EMF | `mask.rs` |
| CD / ILS / NILS / MEEF metrics | ✅ | Sub-pixel monotone-cubic crossings, tone-aware and periodic; CD error ≤ 1.3e-3 nm at 20 samples per period on the cosine fixture | `metrics.rs` |
| Process window / ED analysis | 🔶 | Dose-aware constant-threshold resist, continuous dose limits, EL-vs-DOF, DOF@x %EL, best focus, dose-to-size, iso-focal dose; inscribed rectangles only (no ellipse); CD on the y = 0 cut; no resist blur, diffusion or development | `process.rs` |

## Light sources

All fourteen families implement `LithographySource` and report
`derived_quantities()` (machine-parameter physics, informational). Plasma
sources report `average_power_w()` as in-band power at intermediate focus.

| Capability | Status | Assumptions / limits | Code |
|------------|--------|----------------------|------|
| VUV excimer (F₂ 157.63 nm; Ar₂ 126 nm) | ✅ (Ar₂ 🧪) | Line shape, pupil fill, pulse energy × rep rate, derived photon budget and E95; the Ar₂ preset is hypothetical (Ar₂* is a broad continuum, lasing only e-beam-pumped; 126 nm lithography never left the laboratory) | `source.rs::VuvSource` |
| DUV/UV heritage (ArF 193.368, KrF 248.3, Hg g/h/i) | ✅ (🔶 parameters) | NIST Hg air wavelengths, exact photon budgets, E95/FWHM conventions; laser bandwidths and pulse × rep-rate values are representative; lamps are CW (power not modeled) and use one spectral sample because the engine's default chromatic coefficient is a CaF₂/157 nm figure | `source.rs::VuvSource` |
| LPA-FEL | 🔶 | Demonstrated anchor: BELLA 420 nm SASE at 1 Hz; the 25 nm / 500 MeV / 1 kHz preset is a design projection. Optional undulator + beam derive the resonance (5 % rule), 1D / Ming-Xie gain and the bandwidth 2ρλ (266 pm for the 25 nm preset; explicit override available). 2ρλ is the bandwidth of a lasing FEL, while the illustrative beam's 1 % energy spread violates σ_δ < ρ at 25 nm (no practical gain), so that bandwidth is self-consistent bookkeeping, not a prediction; imaging λ stays a stored set-point | `source.rs::LpaFelSource` |
| Laser-produced plasma (Sn 13.5; Gd 6.7; Tb 6.5 nm) | ✅ (🔶 CE defaults/étendue; Gd/Tb power 🧪) | Drive power × CE × η(2π→IF) × étendue fraction → 250 W at IF on the NXE:3400B chain (a 500 W NXE:3800E-class preset assumes twice the drive power at the same efficiencies); drive-laser CE defaults (2 µm is a projection); Gd/Tb in-band power is a projection (best CE 0.54–0.8 % in 0.6 % bandwidth). No plasma hydrodynamics, debris or collector degradation | `source_models/lpp.rs` |
| Discharge-produced plasma (DPP/LDP, Xe/Sn 13.5 nm) | 🔶 | Power chain P_elec → CE → collector → étendue cut → IF live, with the 2π-at-source and IF conventions kept explicit; presets on reported anchors (Energetiq EQ-10-class Xe; TRINITI 2010 Sn 360 W into 2π, 34 W at IF in the model); CE reported order of magnitude (Xe ~0.5 %, Sn ~2 %); pinch size, collector and illuminator étendue assumed; no discharge dynamics, out-of-band power or debris | `source_models/dpp.rs` |
| Synchrotron (bending magnet / undulator) | ✅ (🔶 coherence) | Wavelength derived from machine parameters; sinc² undulator line, F_n/Q_n harmonics, Kim central-cone flux and power, BM flux and power per mrad (fixture-tested); coherent fraction from emittance is a matched-beta upper bound | `source_models/synchrotron.rs` |
| High-harmonic generation | ✅/🔶 | Comb, cutoff law, Keldysh parameter and critical ionization exact; harmonic power = an assumed driver power × an order-of-magnitude conversion efficiency (λ^−5.5 driver scaling), reported against a curve of measured records (Ne preset ≈ 0.9 µW at 13.5 nm vs ~1 µW measured); the curve is interpolated between 30 and 92 eV, and a derived power above it is labelled a PROJECTION. Full-comb imaging is honest through `compute_multiwavelength` | `source_models/hhg.rs` |
| XFEL (SASE / seeded) | ✅ (🧪 CW-SC/ERL presets) | Pierce parameter derived from the machine (gap-tuned K) → SASE bandwidth; Gamma mode statistics → live dose jitter; 1D + Ming-Xie gain/saturation estimates; machine, wavelength or duration overrides re-derive the pulse energy at the preset's pulse-energy/saturation ratio (an explicit `pulse_energy_uj` wins); the CW-SC (~135 W) and ERL (~10.5 kW) presets are design projections | `source_models/xfel.rs` |
| Plasma soft-X-ray laser (Ne-like Ar 46.9, Ni-like Ag 13.9 / Cd 13.2 / Mo 18.9 nm) | ✅/🔶 | Fixed atomic lasing lines, photon number, coherence length, λ/4 interference limit exact (✅); presets reproduce reported average powers (published Ar and Mo pulse/rate splits; Ag and Cd splits assumed to match reported orders of magnitude), durations, linewidth and coherence assumed, no gain model (🔶) | `source_models/sxrl.rs` |
| Hard X-ray tube (W / Mo / Cu / Rh, 5–300 kV) | 🔶/✅ | Line energies and edges with overvoltage gating, Duane–Hunt cutoff and NIST Be-window filtration (✅); continuum shape, spectral moments and absolute flux (🔶): Kramers thick-target law, empirical η = 1.1e-9·Z·V, isotropic point source, empirical (U − 1)^1.67 line law (±×2), no anode self-absorption, heel effect or backscatter. Broadband: a LIGA/proximity source; the relative spectrum feeds `highuvlith deep`, the absolute `spectral_flux_density` is passed explicitly (`[deep] flux_density`, Python `flux_density=`) | `source_models/xray_tube.rs` |
| Inverse Compton scattering | 🧪 | Kinematics exact; photon yield derived (Thomson luminosity × exact collection fraction): the aggressive design point gives ≈ 71 µW in band, ~3.5 × 10⁶ short of 250 W | `source_models/ics.rs` |
| Laser-wakefield betatron X-rays | 🧪 | Planar-orbit synchrotron-like model: ω_p, ω_β = ω_p/√(2γ), K, ħω_c = (3/2)γ³ħω_β²r_β/c, N_γ ≈ 3.31e-2·K and the S(y) spectrum are textbook and fixture-tested; a keV broadband ~10 Hz imaging source with µW average power, not a lithography source; feeds LIGA as a synchrotron-like spectrum | `source_models/betatron.rs` |
| SSMB storage ring | 🧪 | Integer-harmonic check live; coherent power derived (∝ N·Q₁·b²·I²): 1 kW needs b ≈ 0.146 (≈ 4 nm microbunches every turn) — undemonstrated | `source_models/ssmb.rs` |
| Smith–Purcell free-electron grating | 🧪 | Dispersion λ = (a/m)(1/β − cos θ), tuning and interaction height exact; EUV is purely theoretical (emission demonstrated only to ~230 nm); power α·ε·N·exp(−h/h_int) with an assumed ε = 1e-3 (~5 × 10⁴ × the measured van-der-Waals X-ray yield); sub-nW, ≥ 10¹¹× below HVM | `source_models/smith_purcell.rs` |
| Entangled-photon (NOON) source | 🧪 | Reports the physical λ; N-photon models applied explicitly through the quantum module (`quantum_params()` bridge); no NOON sub-Rayleigh pattern has ever been recorded in a material; ETPA exposure time ≥ 1e16–1e18 s per cm² at the independent cross-section bounds | `source_models/entangled.rs` |
| Machine-parameter physics layer | ✅ | CODATA constants, Bessel functions, undulator and bending-magnet flux, FEL (1D, Ming Xie), Thomson/ICS, HHG and coherent-emission formulas, fixture-tested against scipy/mpmath; Ming Xie is an empirical fit (≈ 10–20 %) | `source_models/physics.rs` |
| Dose-limited wafer throughput | 🔶 | Wafers/hour from any source's `average_power_w()`, a single optics-train transmission, field geometry, lumped overheads and an optional stage limit; pulses per point, photons per CD². Preset scanner numbers are illustrative (Sn LPP at 30 mJ/cm²: 153.9 WPH with the EUV-like defaults) | `source_models/throughput.rs` |

## Resist and development

| Capability | Status | Assumptions / limits | Code |
|------------|--------|----------------------|------|
| 2D resist exposure (Dill ABC) | 🔶 | Depth-averaged: one Beer–Lambert coupling scalar, no z-resolved dose | `resist.rs::expose` |
| 2D development (Mack / threshold) | 🔶 | Center-row vertical etch only; no lateral front or sidewall | `resist.rs::develop` |
| Post-exposure bake (anisotropic Gaussian) | ✅ | Exact spectral Gaussian per axis (σ_xy, σ_z), periodic or reflecting lateral edges, zero-flux z; optional depth-dependent D_z solved exactly in time; variance exactly σ² per axis | `resist.rs::peb_diffuse_anisotropic`, `volumetric.rs::apply_peb` |
| Chemically amplified bake (acid/quencher reaction–diffusion) | 🔶 | Standard Mack/PROLITH-class model normalized to the PAG; exact local reaction + lattice-exact diffusion (Strang, O(Δt²)); no acid loss, constant diffusivities, illustrative default constants | `resist.rs::car_peb`, `car_peb_2d` |
| Volumetric (z-resolved) exposure | 🔶 | Separable I_aer(x, y; d₀ + z/n)·S(z) with the exact thin-film in-resist intensity S(z) = (n_r/n_0)·\|E(z)\|² (before 2026-10-01 the n_r/n_0 factor was missing: ×0.61 under-dose for n = 1.65); paraxial z/n focus mapping; split-step Dill bleaching with a lateral-mean PAC; one engine call per defocus plane | `volumetric.rs` |
| 3D development (fast marching) | ✅ | Eikonal FMM with Godunov updates → sidewalls and undercut; static rate field (incl. static surface inhibition), periodic or mirror edges | `volumetric.rs::develop_fast_marching[_with]` |
| 3D development (level set) | ✅/🔶 | First-order level set (Godunov upwind, CFL, narrow band, FMM reinitialization, extension speeds) validated against analytic fronts and FMM (median difference 1.8–3.8 %); rate re-evaluated at the front: surface inhibition × developer ageing / global / local loading with phenomenological constants | `volumetric.rs::develop_level_set` |
| Grayscale lithography | 🔶 | Log-linear contrast curve, per-pixel 2.5D height map; the fast path ignores standing waves and PEB (the volumetric path captures them) | `grayscale.rs` |

## Deep-layer and non-projection processes

| Capability | Status | Assumptions / limits | Code |
|------------|--------|----------------------|------|
| LIGA / deep X-ray depth dose and exposure time | ✅ | Spectral depth dose with μ (attenuation) and μ_en (deposition) from NIST; bending-magnet photons per ln E ∝ G₁; absolute dose rate and exposure time from ring parameters + scan geometry or an absolute flux-density table (e.g. an X-ray tube's `spectral_flux_density`); the betatron supplies a relative synchrotron-like spectrum; 605 s unfiltered at 200 mA / 15 m / 50 mm scan, validated against an independent numpy implementation. Collimated beam, uniform vertical scan, no beamline mirrors | `deep_xray.rs`, `materials/attenuation.rs` |
| LIGA proximity diffraction | ✅/🔶 | Scalar angular-spectrum (Fresnel) propagation of the complex Au absorber field (NIST μ, Henke δ) per energy bin over gap + resist depth; analytic straight-edge tool; validated against the Fresnel-integral knife edge. 🔶: thin-screen absorber, no photoelectron/secondary-electron transport (optional Grün Gaussian only), no fluorescence, periodic 2D field | `deep_xray.rs` |
| Multi-beam interference / two-photon | ✅/🔶 | Analytic vector-component plane-wave superposition exact; per-beam scalar absorption, no substrate reflection or vector focusing; two-photon I² kinetics with a Gaussian voxel PSF | `interference.rs` |
| Talbot / DTL / ATL proximity lithography | 🔶 | Scalar thin-mask grating orders with exact angular-spectrum propagation; closed-form displacement and achromatic averages; coherent normal illumination, λ-independent grating coefficients, no Fresnel coefficients at the resist | `talbot.rs` |
| Two-grating EUV interference lithography | 🔶 | Fringe period p/(2m) (λ-independent) and visibility exact for ideal TE beams from thin-mask ±m orders; no zero-order background, partial coherence or mask 3D | `talbot.rs::TwoGratingInterference` |

## Optimization and patterning

| Capability | Status | Assumptions / limits | Code |
|------------|--------|----------------------|------|
| ILT mask optimization | ✅ | Exact adjoint gradient through the engine's SOCS kernels incl. flare (finite-difference agreement ~1e-8), nonlinear CG + Armijo line search, TV and binarization regularization, process-window conditions; constant-threshold sigmoid resist; continuous pixel mask (no MRC or polygon extraction) | `ilt.rs` |
| OPC (rule, uniform model, fragment model) | ✅ | Per-fragment EPE feedback on the aerial image (constant threshold) with the exact mask spectrum; line-end pull-back −27/−36 nm → < 1 nm on a 90 nm line; corners slaved (no corner-rounding correction), no MRC beyond bias clamp / jog cleanup / grid snap; rectilinear features only | `opc.rs` |
| SRAF insertion | 🔶 | Heuristic, λ/NA-scaled rule deck (parallel lines, isolated contacts) + model print check at focus/dose corners. DOF gain for an isolated 80–100 nm line: ×1.28–1.49 with conventional or annular illumination on the reference model (×1.31 through the engine for the 80 nm annular case), essentially none with a true two-pole dipole (×1.01–1.05: the first bars print and are removed); constant-threshold resist | `sraf.rs` |
| Multiple patterning (LELE, SADP, SAQP) | 🔶 | LELE: two engine exposures, per-exposure threshold, printed union, sub-pixel overlay; SADP/SAQP purely geometric (no deposition or etch physics) | `double_patterning.rs` |
| DSA block copolymer | 🔶 | Analytic morphologies; lamellar graphoepitaxy registration and strong-segregation commensurability; mean-field ODT check; defect index is an uncalibrated heuristic; not SCFT | `dsa.rs` |

## Stochastics

| Capability | Status | Assumptions / limits | Code |
|------------|--------|----------------------|------|
| Photon shot noise | ✅ | Poisson counts per pixel (Gaussian above 1000 photons) with the photon density derived from the source wavelength (0.68 photons/nm² per mJ/cm² at 13.5 nm, pinned to the literature value); incident by default, absorbed density via `with_resist_absorption`. Per-family overrides (the X-ray tube's mean-photon-energy value) are honoured through the type-erased `SourceKind` too (`test_source_kind_keeps_xray_tube_photon_density`) | `stochastic.rs` |
| Shot-to-shot dose jitter | ✅ | Gamma-distributed per-exposure dose factor from the source's `shot_to_shot_rms()`; `from_source_multi_pulse` averages N pulses (rms/√N) | `stochastic.rs` |
| LER/LWR Monte Carlo | 🔶 | Edge statistics of threshold crossings on one row of noisy aerial images across realizations; no resist chemistry, secondary-electron or acid-diffusion blur in the loop (the empirical acid-noise function exists but is not wired in); N-photon absorption statistics not modeled | `stochastic.rs::compute_ler_lwr` |

## Research modules

| Capability | Status | Assumptions / limits | Code |
|------------|--------|----------------------|------|
| Ptychography (ePIE) | 🔶 | Standard ePIE object + probe updates with far-field (FFT) propagation; fully coherent, noise-free intensities, integer-pixel scan positions, no position refinement; the tests check that the error decreases, not recovery of a known object | `ptychography.rs` |
| Quantum lithography (N-photon / N00N) | 🧪 | Classical N-photon absorption Iᴺ (sharpening at the classical period) kept separate from the ideal N00N model: analytic two-beam fringes at λ/(2N sinθ) and Boto's λ/N ideal limit for general masks (same NA and pupil fill); the fidelity mixes in classical N-photon absorption; flux budget from the ETPA cross-section bounds and the entangled flux ceiling (≥ 4.5×10¹⁰ × exposure time at N = 2) | `quantum.rs` |
| MNSL (moiré nanosphere emission) | 🧪 | Lattice geometry and the (corrected) moiré period are exact; the emission map is a heuristic — static Rayleigh polarizability with an α·e^{ikr}/r² field (neither the dipole near nor far field) applied to spheres comparable to λ, a uniform interlayer phase, intensity-over-mean "enhancement", and one scalar substrate factor (1.346 for the default stack since the thin-film fix, 0.745 before) | `mnsl.rs` |

## Materials and film optics

| Capability | Status | Assumptions / limits | Code |
|------------|--------|----------------------|------|
| X-ray/EUV/BEUV optical constants (CXRO/Henke) | ✅ | Embedded CXRO f₁/f₂ tables (24 elements, 30 eV–30 keV); δ/β and n for any formula + density; 24 fixtures against the CXRO calculator; independent-atom approximation, bulk default densities | `materials/henke.rs` |
| X-ray attenuation (NIST) | ✅ | Hubbell–Seltzer μ/ρ and μ_en/ρ (15 elements, 1 keV–20 MeV, edges resolved) + Henke photoabsorption below 1 keV; mixture rule for compounds (reproduces NIST's PMMA table to 2e-3) | `materials/attenuation.rs` |
| Materials database | 🔶 | EUV/BEUV entries computed from Henke at the requested λ; generic `henke:<formula>@ρ`; VUV tabulated n, k approximate; resist entries are representative stand-ins. Every entry has a stated wavelength range: out-of-range lookups fall back to Henke (Si, Cr, SiO₂) or raise `WavelengthOutOfRange`, with no clamping or extrapolation (before 2026-10-01, `Si` at 13.5 nm returned the VUV value); VUV Sellmeier values are UV–IR fits extrapolated into the VUV (CaF₂: 1.5570 vs ~1.559 at 157 nm) | `materials/database.rs` |
| EUV/BEUV multilayer mirrors | ✅ | Parratt recursion + Névot–Croce roughness, s/p, capping, period tuning; ideal Mo/Si 73.0 % at 13.48 nm, La/B₄C 68.9 % at 6.7 nm; matches the CXRO multilayer calculator to < 3e-3 (real Mo/Si mirrors reach ~67–70 %); feeds the optional multilayer pupil | `materials/multilayer.rs` |
| Thin-film transfer matrix | ✅ | 2×2 characteristic matrix in the q-formulation (complex indices, any angle incl. X-ray grazing); TE/TM/unpolarized R, T, complex r, t and exact in-film intensity; oblique-TM in-film intensity is \|U\|² | `thinfilm.rs` |

## Frontends and infrastructure

| Capability | Status | Assumptions / limits | Code |
|------------|--------|----------------------|------|
| Python bindings | ✅ | PyO3 module `highuvlith._native` + `highuvlith.api` wrappers; every compute path releases the GIL; the large result arrays (`AerialImageResult.intensity`, `VolumetricResult.values`, `HeightMapResult.values`, Talbot/LELE/DSA images) are zero-copy read-only NumPy views, function outputs are moved without copying, small accessors are copies; results of 🔶/🧪 models carry `.status`/`.notes`; `_native.pyi` drift-tested (names, kinds, parameters, defaults); no ptychography binding (🗺️); not published on PyPI (build from source) | `crates/highuvlith-py`, `python/highuvlith` |
| CLI (TOML, strict keys) | ✅ | Seven subcommands: `simulate`, `sweep`, `deep`, `optimize`, `throughput`, `sources`, `materials`; `deep` modes `liga`, `grayscale`, `interference`, `volumetric`, `talbot`; unknown or family-inert keys rejected with suggestions; every example config run end to end in the test suite (`crates/highuvlith-cli/tests/examples.rs`); `[optics.multilayer_pupil]` mirrors the Python multilayer-pupil option | `crates/highuvlith-cli` |
| Desktop GUI | ✅ | egui app over the core: all 14 source families (38 presets) with badges, derived quantities and machine sliders; optics incl. 193i immersion, EUV NXE/High-NA, optional multilayer pupil; scalar/vector imaging, exact defocus, per-wavelength spectra, commensurate grid; dose-aware process window; volume viewer (z-slice and x–z sections of the latent/developed resist, LIGA dose, Talbot carpet); LIGA depth dose; PNG export. No 3D isosurfaces or interference volumes (🗺️) | `crates/highuvlith-gui` |
| Jupyter notebooks | ✅ | Six notebooks (quickstart, source gallery, deep lithography, imaging physics, source physics, process and optimization) executed headlessly in CI | `examples/notebooks/` |
| Documentation site | ✅ | MkDocs Material site (history, process nodes, future, playground, simulator docs) built with `--strict` in CI and deployed to GitHub Pages; every Python example on the site runs in CI and its printed output is checked (`tests/docs/test_site_examples.py`); simulator-generated figure gallery (`docs/figures/sim/make_all.py`); the playground calculators are 🔶 teaching toys, not the Rust engine | `mkdocs.yml`, `docs/` |
| GPU compute backend | 🗺️ | `ComputeBackend` trait with a CPU implementation only; the imaging engine calls its FFTs directly and does not use the trait yet | `compute/` |
