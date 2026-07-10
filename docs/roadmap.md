# Roadmap

**Status:** 🗺️ Planned — nothing on this page is in code; per the [capability-matrix](./capability-matrix.md) taxonomy, planned items are never described elsewhere as if they run.

Each entry states what exists today, what is missing, and why it earns a slot. Items are roughly ordered by how much they would change simulation fidelity.

## At a glance

| Planned item | Unblocks | Bounded today by |
|---|---|---|
| [Vector / polarized in-film imaging](#vector--polarized-in-film-imaging) | Honest NA ≳ 0.8 and in-resist contrast | Scalar TCC/SOCS; separable volumetric exposure |
| [Per-λ TCC (`compute_multiwavelength`)](#per-wavelength-tcc--compute_multiwavelength) | Honest HHG combs, wide-band imaging | Center-λ TCC + per-sample focus shift |
| [Fresnel–Kirchhoff LIGA diffraction](#full-fresnelkirchhoff-proximity-diffraction-for-liga) | Edge-shape fidelity in shadow printing | Gaussian first-Fresnel-zone blur σ ≈ 0.5√(λg) |
| [Level-set development](#level-set-moving-boundary-development) | Marginal-dose sidewall profiles | Frozen-rate fast-marching front |
| [z-anisotropic PEB](#z-anisotropic-peb-diffusion) | Matching measured top-loss vs lateral blur | Isotropic 3D Gaussian PEB |
| [True adjoint ILT](#true-adjoint-ilt) | Contact/curvilinear targets, co-optimization | Correlation proxy gradient |
| [SRAF insertion](#sraf-insertion) | Isolated-feature DOF at low k₁ | Edge biasing only |
| [GPU `ComputeBackend`](#gpu-computebackend) | Large sweeps, 512²+ grids | CPU rayon backend only |
| [GUI volumetric viewer](#gui-volumetric-viewer) | Inspecting `Grid3D` outputs outside Python | 2D heatmaps only |
| [EUV/BEUV optical-constant pass](#euvbeuv-optical-constant-verification-pass) | Quantitative 13.5 / 6.7 nm work | Approximate Ru/Ta/resist/BEUV entries |
| [Hard X-ray tube, DPP sources](#additional-source-families-beyond-the-nine) | Lab-scale LIGA; source trade studies | Nine families implemented |

## Imaging fidelity

### Vector / polarized in-film imaging

**Today:** the aerial engine is scalar diffraction ([aerial.rs](../crates/highuvlith-core/src/aerial.rs)); the volumetric path couples a scalar lateral image with the 1D thin-film response as a separable product `I_aer(x,y; d₀+z/n) × S(z)` ([volumetric.rs](../crates/highuvlith-core/src/volumetric.rs)). **Missing:** TE/TM pupil decomposition, vector image formation in the film, and the interference-contrast loss between orthogonal polarization components. **Why:** at NA ≳ 0.8 (and generally inside a high-index resist) the scalar approximation overstates contrast; every capability-matrix row that says "🔶 at NA ≳ 0.8" is bounded by this item. This is the single largest fidelity upgrade available.

### Per-wavelength TCC — `compute_multiwavelength`

**Today:** `compute_polychromatic` shifts focus per spectral sample while reusing the TCC built at the center wavelength — honest for the pm-scale bandwidths of excimer/FEL sources, dishonest for wide combs. Consequently the HHG full-comb mode (`full_comb = true`) is **spectral bookkeeping only**, and monochromatized single-harmonic operation is the default. **Missing:** a per-λ TCC/SOCS rebuild (or interpolated kernel families) summed incoherently across the comb. **Why:** it converts the HHG comb, wide-band bending-magnet, and SASE-jitter cases from bookkeeping into honest imaging. Cost is the main obstacle: N_λ full TCC decompositions.

### Full Fresnel–Kirchhoff proximity diffraction for LIGA

**Today:** [deep_xray.rs](../crates/highuvlith-core/src/deep_xray.rs) blurs the shadow image with a Gaussian of the first-Fresnel-zone width, σ ≈ 0.5·√(λg). **Missing:** true Fresnel–Kirchhoff propagation of the mask field across the proximity gap per spectral bin (edge ringing, gap-dependent fringe structure, absorber phase). **Why:** the Gaussian gets the blur *scale* right but not the edge *shape*; sidewall-angle predictions for high-aspect LIGA structures inherit that error.

## Resist and development

### Level-set moving-boundary development

**Today:** 3D development is a fast-marching arrival-time solve that assumes the dissolution-rate field is frozen during develop (standard isotropic wet-etch assumption, stated in the volumetric module header). **Missing:** a level-set front evolution with rate re-evaluation at the moving boundary — needed for surface-inhibition layers, developer depletion, and rate fields that depend on the exposed/developed geometry. **Why:** the frozen-field assumption is fine for well-exposed positive resists, but sidewall-profile fidelity in marginal-dose regimes needs the moving boundary.

### z-anisotropic PEB diffusion

**Today:** volumetric PEB smoothing is an isotropic 3D Gaussian. **Missing:** separate lateral and vertical diffusion lengths (and optionally z-dependent diffusivity). **Why:** bake-plate thermal gradients and the resist/air + resist/BARC interfaces make real acid diffusion anisotropic; measured resist profiles show different top-loss vs. lateral-blur signatures that an isotropic kernel cannot fit.

## Optimization modules

### True adjoint ILT

**Today:** [ilt.rs](../crates/highuvlith-core/src/ilt.rs) uses an adjoint-*inspired* proxy gradient (`2·error·sigmoid'` + TV term). **Missing:** the rigorous adjoint through the SOCS kernels, `2·Re[Σ_k λ_k FFT(H_k*·IFFT(H_k·FFT(m)·(I−I_tgt)))]`, which needs kernel access from the optimizer. **Why:** the proxy converges on smooth line/space targets but stalls on contact arrays and curvilinear targets; the true adjoint is also the prerequisite for dose/focus co-optimization.

### SRAF insertion

**Today:** [opc.rs](../crates/highuvlith-core/src/opc.rs) does edge biasing only — no sub-resolution assist features. **Missing:** rule-based SRAF placement (distance/width tables) and print-check via the aerial engine to reject printing assists. **Why:** isolated-feature depth of focus at low k₁ is dominated by assist features; edge bias alone cannot recover it.

## Infrastructure

### GPU `ComputeBackend`

**Today:** the [compute/](../crates/highuvlith-core/src/compute) trait has a CPU (rustfft + rayon) implementation only; the GPU seam is a stub. **Missing:** a wgpu backend for batched FFTs and kernel convolutions. **Why:** the SOCS image sum and process-window sweeps are embarrassingly parallel over kernels × conditions; 256²–512² grids at sweep counts of 10²–10³ are squarely in GPU-win territory. Blocked mainly on f64 support strategy (or an audited f32 path).

### GUI volumetric viewer

**Today:** the egui GUI renders 2D aerial heatmaps. **Missing:** z-slice scrubbing and 3D isosurface/profile views of `Grid3D` latent images and development fronts from the volumetric/LIGA/interference modules. **Why:** the deep-layer modules produce volumes; without a viewer, their primary output is only inspectable from Python.

### EUV/BEUV optical-constant verification pass

**Today:** VUV tabulated n,k (126–160 nm) are mature; the EUV Si and Mo entries are canonical CXRO values, but Ru, Ta, the resist entries, and the 6.7 nm BEUV entries are approximate placeholders ([materials/](../crates/highuvlith-core/src/materials)). **Missing:** a systematic re-derivation of every EUV/BEUV entry from the CXRO/Henke tables (henke.lbl.gov) with per-entry provenance comments and fixture tests. **Why:** multilayer reflectance and resist absorption at 13.5/6.7 nm are exponentially sensitive to k; quantitative EUV work is gated on this pass. Until then: verify against henke.lbl.gov before quantitative use (this warning also appears on [materials.md](./materials.md)).

## Additional source families (beyond the nine)

The [source recipe](./extending.md#a-adding-a-source-family) makes these mechanical to add; they are queued behind demand:

| Family | Target regime | Rationale / model sketch |
|--------|---------------|--------------------------|
| Hard X-ray tube | 0.01–0.15 nm (8–100 keV) | Lab-scale LIGA and deep-resist exposure without a synchrotron; characteristic lines (W Lα, Cu Kα) + bremsstrahlung continuum feeding the existing `XraySpectrum::Tabulated` path in deep_xray |
| Plasma discharge (DPP/LDP) | 13.5 nm | Historical EUV alternative to LPP (Sn/Xe pinch); lower conversion efficiency and étendue than LPP but a distinct debris/stability profile worth comparing in the stochastic module |

## Explicit non-goals (for now)

- **Rigorous mask 3D (EMF) simulation** — thick-mask topography effects are out of scope; the mask is a thin complex transmittance everywhere.
- **SCFT for DSA** — [dsa.rs](../crates/highuvlith-core/src/dsa.rs) stays analytic; a self-consistent field solver is a research project of its own.
- **Quantum-lithography engineering claims** — [quantum.rs](../crates/highuvlith-core/src/quantum.rs) remains 🧪 theoretical; no roadmap item will promote it without experimental anchors.

## Process

A roadmap item graduates by: implementation + tests → module-header `# Model status` update → capability-matrix row flip (🗺️ → 🔶/✅) → docs page status badge, all in one PR. If you pick one up, check [extending.md](./extending.md) first and delete the item from this page in the same PR.
