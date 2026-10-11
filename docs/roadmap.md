---
description: "What highuvlith does not do yet: every planned item with what it would unblock, what bounds it today, which part of the code it touches and the capability-matrix row it would flip."
---

# Roadmap

**Status:** 🗺️ Planned — nothing on this page is in code; per the matrix taxonomy a planned item is never described elsewhere as if it runs · [matrix](capability-matrix.md#frontends-and-infrastructure)

<p class="huv-lead">Each entry states what exists today, what is missing and why it earns a slot, roughly ordered by how much it would change simulation fidelity. Everything the 2026-09-30 overhaul delivered has left this page. An item leaves this page in the same pull request that flips its matrix row.</p>

What that overhaul delivered — vector imaging, per-wavelength TCCs, immersion and EUV projection optics, the opt-in multilayer pupil, Fresnel LIGA diffraction with absolute exposure time, the CXRO optical constants, the X-ray tube and discharge-plasma families, level-set development, the anisotropic and chemically amplified bakes, true-adjoint ILT, SRAF insertion, zero-copy Python results and the GUI volume views — is recorded in the [capability-matrix audit log](capability-matrix.md#audit-log), not here.

**Same thing elsewhere:** the one 🗺️ matrix row → [GPU compute backend](capability-matrix.md#frontends-and-infrastructure) · the graduation loop → [How a capability changes grade](reference/index.md#how-a-capability-changes-grade) · where a change touches the code → [Extending highuvlith](extending.md)

## At a glance

Touches says which part of the code an item changes (core module · core + front ends · new backend), not when or whether anyone will do it; the matrix row is the one the item would flip.

<!-- huv-table: stack -->

| Planned item | Unblocks | Bounded today by | Touches | Matrix row |
|---|---|---|---|---|
| [Depth-resolved vector in-stack imaging](#depth-resolved-vector-in-stack-imaging) | In-resist contrast and standing waves at high NA | Vector field at the film entrance × separable S(z) | core module | [Volumetric (z-resolved) exposure](capability-matrix.md#resist-and-development) |
| [Jones-pupil lens polarization](#jones-pupil-lens-polarization) | Polarization aberrations, CaF₂ birefringence at 157 nm | Ideal (non-polarizing) lens in vector mode | core module | [Vector (polarized) high-NA imaging](capability-matrix.md#core-imaging) |
| [Anamorphic High-NA mask side](#anamorphic-high-na-mask-side) | Mask-side High-NA effects (4×/8×, oblique incidence) | Isotropic wafer-side EUV pupil | core module | [EUV scanner projection optics](capability-matrix.md#optical-systems) |
| [Multilayer pupil from a design ray trace](#multilayer-pupil-from-a-design-ray-trace) | Predictive EUV/BEUV apodization and multilayer phase | User-supplied incidence-angle maps; polarization-averaged amplitude | core module | [EUV/BEUV multilayer pupil (opt-in)](capability-matrix.md#optical-systems) |
| [Point-spread flare and in-resist focus](#point-spread-flare-and-in-resist-focus) | Long-range flare, focus inside a high-index resist | Uniform zeroth-order flare; image-medium defocus only | core module | [Flare](capability-matrix.md#core-imaging) |
| [Stochastic resist Monte Carlo](#stochastic-resist-monte-carlo) | Along-edge LER, stochastic defects | Shot noise on the aerial image, one-row edge statistics | core module | [LER/LWR Monte Carlo](capability-matrix.md#stochastics) |
| [Resist models inside OPC, ILT and the process window](#resist-models-inside-opc-ilt-and-the-process-window) | Resist-aware correction and windows | Constant-threshold resist | core module | [ILT, OPC, SRAF](capability-matrix.md#optimization-and-patterning) · [Process window / ED analysis](capability-matrix.md#masks-metrics-and-process-window) |
| [Development and CAR chemistry refinements](#development-and-car-chemistry-refinements) | Developer transport, acid loss | Separable rate factors; constant diffusivities | core module | [3D development (level set)](capability-matrix.md#resist-and-development) · [Chemically amplified bake](capability-matrix.md#resist-and-development) |
| [Process-window analysis extensions](#process-window-analysis-extensions) | Ellipse ED windows, 2D contour CD | Rectangles; y = 0 cut | core module | [Process window / ED analysis](capability-matrix.md#masks-metrics-and-process-window) |
| [MRC-aware ILT and model-based SRAF](#mrc-aware-ilt-and-model-based-sraf) | Manufacturable curvilinear masks | Continuous pixel masks; heuristic SRAF rules | core module | [ILT mask optimization](capability-matrix.md#optimization-and-patterning) · [SRAF insertion](capability-matrix.md#optimization-and-patterning) |
| [LIGA sidewall physics](#liga-sidewall-physics) | Sidewall roughness and runout | Thin-screen absorber, no electron transport | core module | [LIGA proximity diffraction](capability-matrix.md#deep-layer-and-non-projection-processes) |
| [Talbot / EUV-IL partial coherence](#talbot--euv-il-partial-coherence) | Realistic DTL/ATL and EUV-IL contrast | Perfectly coherent, thin-mask model | core module | [Talbot / DTL / ATL](capability-matrix.md#deep-layer-and-non-projection-processes) · [Two-grating EUV-IL](capability-matrix.md#deep-layer-and-non-projection-processes) |
| [Interference lithography interfaces and kinetics](#interference-lithography-interfaces-and-kinetics) | Substrate standing waves, voxel growth | Scalar per-beam absorption, no interfaces | core module | [Multi-beam interference / two-photon](capability-matrix.md#deep-layer-and-non-projection-processes) |
| [Grayscale phase elements](#grayscale-phase-elements) | Phase and phase-amplitude gray masks | Zero-phase gray features | core + front ends | [Grayscale lithography](capability-matrix.md#resist-and-development) |
| [Source-model refinements](#source-model-refinements) | Tighter power and spectrum estimates | Stated order-of-magnitude assumptions | core module | [Light sources](capability-matrix.md#light-sources) (per family) |
| [GPU compute backend](#gpu-compute-backend) | Large sweeps, 512²+ grids | CPU (rayon) only | new backend | [GPU compute backend](capability-matrix.md#frontends-and-infrastructure) — the one 🗺️ row |
| [GUI 3D volumes](#gui-3d-volumes) | 3D isosurfaces; interference-lithography volumes in the GUI | z-slice + x–z section views | core + front ends | [Desktop GUI](capability-matrix.md#frontends-and-infrastructure) |
| [Python coverage gaps](#python-coverage-gaps) | Ptychography from Python | Rust API only | core + front ends | [Python bindings](capability-matrix.md#frontends-and-infrastructure) |

## Imaging fidelity

### Depth-resolved vector in-stack imaging

- [ ] **Today:** the vector pupil ([vector-imaging.md](./vector-imaging.md)) applies the film-entrance Fresnel transmission at z = 0⁺; the volumetric path is still the separable product `I_aer(x, y; d₀ + z/n) × S(z)` ([volumetric.rs](../crates/highuvlith-core/src/volumetric.rs)). **Missing:** per-order, per-polarization (TE/TM) transfer-matrix propagation through the film stack, with multiple reflections and the substrate, to give a z-resolved `|E(x, y, z)|²` inside the resist (reusing the `thinfilm.rs` r/t per order). **Why:** standing waves and in-resist contrast at high NA are where the separable model is weakest.

### Jones-pupil lens polarization

- [ ] **Today:** vector mode treats the projection lens as ideal — no retardance, diattenuation or birefringence. **Missing:** a 2×2 Jones pupil multiplying the TE/TM field per pupil point (retardance and diattenuation maps; intrinsic CaF₂ birefringence at 157 nm). **Why:** polarization aberrations limit hyper-NA imaging, and CaF₂ birefringence was one of the reasons 157 nm lithography stalled.

### Anamorphic High-NA mask side

- [ ] **Today:** [`EuvProjectionOptics`](../crates/highuvlith-core/src/optics/euv.rs) is an isotropic wafer-side pupil; mask coordinates are wafer-scale and the High-NA central obscuration (0.2·NA) is an assumed value. **Missing:** the 4× / 8× anamorphic magnification on the mask side, oblique mask incidence (~6°) and the non-telecentric chief ray, with a published obscuration geometry. **Why:** High-NA mask-side effects (shadowing, pitch-dependent best focus) are first-order at 0.55 NA and cannot be represented by a wafer-side pupil alone.

### Multilayer pupil from a design ray trace

- [ ] **Today:** the opt-in [`MultilayerPupil`](../crates/highuvlith-core/src/optics/multilayer_pupil.rs) applies the angle-dependent multilayer amplitude and phase across the Schwarzschild or EUV projection pupil (🔶), but the incidence-angle map of each mirror is a user-supplied first/second-order model, all mirrors share one coating and one s/p basis, and the amplitude is polarization-averaged even in vector mode. **Missing:** angle maps from an actual objective prescription (a ray trace), per-mirror coatings, and r_s ≠ r_p applied to the TE/TM field components. **Why:** only then does the multilayer apodization and focus shift become a prediction for a real design rather than a what-if.

### Point-spread flare and in-resist focus

- [ ] **Today:** flare is a uniform zeroth-order fraction ([aerial.rs](../crates/highuvlith-core/src/aerial.rs)); the defocus phase uses the image-space medium (air or immersion fluid). **Missing:** a point-spread flare kernel (power-spectral-density scatter), and focus measured inside a resist of index n_r rather than in the fluid. **Why:** long-range flare sets dense/isolated CD offsets in EUV, and in-resist focus matters for thick or high-index resists.

## Resist, stochastics and process

### Stochastic resist Monte Carlo

- [ ] **Today:** [stochastic.rs](../crates/highuvlith-core/src/stochastic.rs) applies Poisson photon noise to the aerial image and a Gamma dose factor per exposure, then reads threshold-crossing statistics on one row across realizations. **Missing:** a voxel Monte Carlo of absorbed photons, secondary-electron blur, acid generation, diffusion and deprotection, with along-edge LER/LWR and defect (bridge/break) probabilities. **Why:** stochastic failures, not mean CD, limit EUV dose and pitch; the current model gives the photon-noise floor only.

### Resist models inside OPC, ILT and the process window

- [ ] **Today:** OPC, ILT, SRAF print checks and the process window all use a constant-threshold resist on the aerial image. **Missing:** the existing PEB/CAR models (and development) in the loop, with their gradients for ILT. **Why:** resist blur shifts optimal biases and shrinks process windows; correcting against the aerial image alone overestimates what prints.

### Development and CAR chemistry refinements

- [ ] **Today:** level-set development re-evaluates separable rate factors (static field × time factor) at the front; the CAR bake uses constant diffusivities and no acid loss. **Missing:** non-separable development (developer transport into trenches, orientation/curvature-dependent rates), acid evaporation/trapping, deprotection-dependent and depth-dependent lateral diffusivity, higher reaction orders, and parallel (rayon) FMM / level-set / CAR loops. **Why:** these set T-top and footing shapes and resist-profile sensitivity at small pitch.

### Process-window analysis extensions

- [ ] **Today:** ED analysis uses inscribed rectangles and measures CD on the y = 0 cut ([process.rs](../crates/highuvlith-core/src/process.rs)). **Missing:** ellipse-fitted ED windows and 2D contour CD metrics (contacts, line ends). **Why:** industry process-window figures are usually ellipse-based, and 2D features need contour metrics.

## Optimization

### MRC-aware ILT and model-based SRAF

- [ ] **Today:** [ilt.rs](../crates/highuvlith-core/src/ilt.rs) returns a continuous pixel mask with a 0.5 threshold; [sraf.rs](../crates/highuvlith-core/src/sraf.rs) places assists from a heuristic rule deck. **Missing:** mask-rule constraints (minimum width/space, curvature) inside the ILT objective with polygon extraction, and ILT-seeded (model-based) assist placement for 2D layouts and line ends. **Why:** unconstrained pixel masks are not manufacturable, and rule decks do not cover 2D layouts.

## Deep-layer processes

### LIGA sidewall physics

- [ ] **Today:** [deep_xray.rs](../crates/highuvlith-core/src/deep_xray.rs) propagates a thin-screen Au absorber field with scalar Fresnel diffraction and has only an optional Gaussian photoelectron blur. **Missing:** multislice propagation through the thick absorber, photoelectron/secondary-electron Monte Carlo and fluorescence from absorber, membrane and substrate, and a beamline model (mirrors, horizontal-fan runout, source-size penumbra). **Why:** these set the sidewall roughness and slope of high-aspect-ratio LIGA structures.

### Talbot / EUV-IL partial coherence

- [ ] **Today:** [talbot.rs](../crates/highuvlith-core/src/talbot.rs) assumes perfectly coherent normal illumination, a thin mask and λ-independent grating coefficients. **Missing:** source angular spread and partial spatial coherence, wavelength-dependent grating coefficients (the phase step scales as 1/λ; EUV absorber n, k), Fresnel coefficients and substrate reflection at the resist, a finite illuminated field (walk-off of high orders at large gaps), zero-order background for EUV-IL, and mask-3D grating efficiencies. **Why:** they set the achievable contrast of displacement/achromatic Talbot and EUV interference lithography.

### Interference lithography interfaces and kinetics

- [ ] **Today:** [interference.rs](../crates/highuvlith-core/src/interference.rs) superposes ideal plane waves exactly but applies absorption as a scalar decay along each beam, with no resist/substrate interfaces; two-photon voxels use a fixed Gaussian PSF. **Missing:** Fresnel transmission at the resist surface and substrate reflection (standing waves across the lattice), and polymerization threshold and diffusion kinetics for two-photon writing. **Why:** substrate reflections modulate interference lattices in depth, and voxel size is set by the kinetics, not by the optical PSF alone.

### Grayscale phase elements

- [ ] **Today:** gray features (`MaskFeature::GrayRect`) carry an amplitude √T with zero phase, and the 2.5D height map follows a contrast curve ([grayscale.md](./processes/grayscale.md)). **Missing:** non-zero-phase and phase-plus-amplitude gray features. **Why:** phase-shifting grayscale and diffractive elements need them.

## Sources

### Source-model refinements

The source families state their assumptions inline. These refinements would replace stored or order-of-magnitude values with derived ones, or add physics the models leave out; each entry matches a `planned` row in the family page's model-coverage table:

| Family | Today | Missing |
|---|---|---|
| DUV/UV heritage | Representative laser bandwidth and pulse values; lamps CW with no power model | Lamp output power and arc spectrum; laser pulse-energy jitter and E95 stabilization; a colour-corrected lamp-era lens model (an optics item) |
| VUV excimer | Trait-default coherence and jitter; Ar₂ preset a hypothetical narrow line | Excimer jitter/coherence model; Ar₂ continuum and line-narrowing physics |
| LPA-FEL | Stored pulse energy (FEL physics reported alongside; the bandwidth is already derived as 2ρλ) | Pulse energy derived from the FEL estimate |
| Synchrotron | Filament-beam flux; emittance/energy-spread broadening reported only | Broadening folded into the undulator flux and line shape |
| HHG | Order-of-magnitude conversion efficiency; plane-wave critical ionization | ADK-based phase-matched cutoff and efficiency; HHG intensity noise as dose jitter |
| XFEL | Gamma longitudinal-mode statistics | Time-domain pulse structure, slippage and start-up noise beyond the Gamma statistic |
| Discharge plasma | Power chain to IF | Out-of-band spectrum, debris and electrode lifetime |
| SSMB | Coherent power from an assumed bunching factor | Microbunching dynamics (laser power, lattice, IBS / quantum-excitation limits on the bunch length) |
| ICS | Linear Thomson, head-on, round beams | Nonlinear (a₀ ~ 1), hourglass and crossing-angle corrections; a₀ consistent with the laser pulse energy and spot |
| Entangled photons | Imaged classically; quantum models applied explicitly | N-photon absorption statistics in the stochastic module |
| X-ray tube | Isotropic point source; Be window only | Heel effect, self-absorption, focal-spot size, Al/Cu filters |
| Betatron | Single energy and amplitude | Energy/amplitude ensembles over the acceleration history |
| Smith–Purcell | Assumed coupling ε | Grating efficiency (surface-current theory) and angular pattern |
| Soft-X-ray laser | Output parameters are presets | Gain and saturation model, beam profile, polarization |
| Throughput | Illustrative scanner parameters | Vendor-calibrated optics train and stage model |

## Infrastructure

### GPU compute backend

- [ ] **Today:** the [compute/](../crates/highuvlith-core/src/compute) `ComputeBackend` trait has a CPU implementation only, and the imaging engine calls its FFTs directly rather than through the trait. **Missing:** routing the SOCS image sum and FFTs through the trait, then a wgpu backend for batched FFTs and kernel convolutions. **Why:** the SOCS sum and process-window sweeps are embarrassingly parallel over kernels × conditions; 256²–512² grids at 10²–10³ conditions are GPU territory. Blocked mainly on an f64 strategy (or an audited f32 path). This is the matrix's one 🗺️ row ([Frontends and infrastructure](capability-matrix.md#frontends-and-infrastructure)), and that row links back here.

### GUI 3D volumes

- [ ] **Today:** the GUI's volume tab shows z-slice scrubbing and x–y / x–z sections with a colour bar and hover readout for the volumetric latent/developed resist, the LIGA dose and the Talbot carpet ([gui.md](./gui.md)). **Missing:** 3D isosurface rendering of `Grid3D` latent images and development fronts, and the interference-lithography volumes, which the GUI does not compute. **Why:** sidewall and undercut shapes are easier to judge in 3D than slice by slice.

### Python coverage gaps

- [ ] **Today:** the Python bindings cover imaging, sources, resist, volumetric, deep-layer, optimization, stochastic LER/LWR and research modules (see [python-api.md](./python-api.md)). **Missing:** a binding for ptychography (`ptychography.rs`, ePIE), which is reachable from Rust only. **Why:** it is the only research module without a Python entry point, so its reconstructions cannot be scripted or plotted from notebooks.

## Explicit non-goals (for now)

- **Rigorous mask 3D (EMF) simulation** — thick-mask topography, absorber shadowing and polarization-dependent order amplitudes are out of scope; the mask is a thin complex transmittance everywhere, and every page that touches masks says so.
- **SCFT for DSA** — [dsa.rs](../crates/highuvlith-core/src/dsa.rs) stays analytic; a self-consistent field solver is a research project of its own.
- **Quantum-lithography engineering claims** — [quantum.rs](../crates/highuvlith-core/src/quantum.rs) remains 🧪 theoretical; no roadmap item will promote it without experimental anchors.

## Process

A roadmap item graduates by: implementation + tests → module-header `# Model status` update → capability-matrix row flip from 🗺️ Planned to the grade the code earns → docs page Status badge, all in one PR — the loop drawn under [How a capability changes grade](reference/index.md#how-a-capability-changes-grade) on the reference index. If you pick one up, check [extending.md](./extending.md) and [CONTRIBUTING.md](../CONTRIBUTING.md) first and delete the item from this page in the same PR.

**Where next:** [Reference](reference/index.md) · [Capability matrix](capability-matrix.md#audit-log) · [Extending highuvlith](extending.md) · [Research modules](research-modules.md)
