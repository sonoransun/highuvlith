# Capability status matrix

This page is the **single source of truth** for what highuvlith actually
computes versus what it parameterizes, projects, or merely plans. Every
capability-changing PR must update the relevant row here (and the
`**Status:**` badge on the capability's documentation page) in the same PR.

## Status taxonomy

| Badge | Name | Definition |
|-------|------|------------|
| ✅ | **Implemented** | Computed by code, covered by tests, physics validated against analytical results |
| 🔶 | **Simplified** | Runs end-to-end but with documented approximations or reduced dimensionality; the assumption is stated inline wherever the capability is described |
| 🧪 | **Theoretical** | Parameterized and runnable, but models speculative physics; outputs are research projections, not validated engineering |
| 🗺️ | **Planned** | Not in code; roadmap only. Never described in prose as if it runs |

Model-coverage tables on source/process pages additionally tag each struct
field as `live` (enters a computation), `stored (inert)` (carried and
exposed, awaiting a consumer), or `planned` (not yet a field).

## Core imaging & process

| Capability | Status | Assumptions / limits | Code |
|------------|--------|----------------------|------|
| Hopkins TCC/SOCS aerial imaging | ✅ | Scalar diffraction; no polarization/vector high-NA effects (🔶 at NA ≳ 0.8) | `crates/highuvlith-core/src/aerial.rs` |
| Polychromatic imaging | 🔶 | Per-sample focus shift only; TCC built at center λ — valid for Δλ/λ ≪ 1, not for wide combs | `aerial.rs::compute_polychromatic` |
| Refractive projection optics (CaF₂) | ✅ | Zernike aberrations, apodization, chromatic defocus | `optics/mod.rs` |
| Fresnel zone plate optics | ✅ | Efficiency by zone profile; strong chromatic Δf/f = Δλ/λ | `optics/zone_plate.rs` |
| Schwarzschild reflective optics | ✅ | Annular pupil; multilayer reflectance as a scalar constant | `optics/schwarzschild.rs` |
| Thin-film transfer matrix | ✅ | 2×2 characteristic matrix; TE/TM/unpolarized | `thinfilm.rs` |
| Resist exposure (Dill ABC) | 🔶 | **Depth-averaged 2D**: single Beer–Lambert coupling scalar, no z-resolved dose | `resist.rs::expose` |
| Resist development (Mack / threshold) | 🔶 | Center-row vertical etch only; no lateral front, no sidewall profile | `resist.rs::develop` |
| Process window / Bossung analysis | ✅ | — | `process.rs` |
| Photon shot noise + LER/LWR Monte Carlo | ✅ | Poisson (Gaussian for large counts); Gamma-distributed shot-to-shot dose jitter | `stochastic.rs` |
| OPC (rule- and model-based) | ✅ | No SRAF insertion | `opc.rs` |
| Double patterning (LELE) | 🔶 | Aerial-domain dose sum with overlay shift; no inter-exposure resist chemistry | `double_patterning.rs` |

## Light sources

| Capability | Status | Assumptions / limits | Code |
|------------|--------|----------------------|------|
| VUV excimer (F₂ 157.63 nm, Ar₂ 126 nm) | ✅ | Line shape, pupil fill; pulse energy × rep rate → average power (live) | `source.rs::VuvSource` |
| LPA-FEL (20–30 nm, BELLA-class) | 🔶 | Spectral/pupil model live; pulse metadata + jitter now feed average power and stochastic dose jitter (live); electron energy documentary; coherence reported but default pupil still Conventional | `source.rs::LpaFelSource` |
| LPP (Sn 13.5 nm / Gd 6.7 nm / Tb 6.5 nm) | ✅ | In-band Gaussian spectrum; power chain drive × CE × transport live; plasma dynamics/debris out of scope | `source_models/lpp.rs` |
| Synchrotron (bending magnet / undulator) | ✅ | **Wavelength DERIVED from machine parameters** (resonance / E_c); Kostroun S(y) fixture-tested; Gaussian line instead of sinc² | `source_models/synchrotron.rs` |
| HHG table-top EUV | ✅/🔶 | Comb spacing + cutoff law exact, cutoff rejection live; plateau rolloff empirical; full-comb mode is spectral bookkeeping, not honest imaging | `source_models/hhg.rs` |
| XFEL (SASE / self-seeded) | ✅ | SASE bandwidth 2ρ; Gamma statistics → live LER dose jitter; spike-realization mode research-only | `source_models/xfel.rs` |
| Inverse Compton scattering | 🧪 | Kinematics exact, electron energy derived from target λ; flux at litho dose rates undemonstrated | `source_models/ics.rs` |
| SSMB storage ring | 🧪 | Harmonic-consistency check live; kW power is a projection (PoP: Deng et al., Nature 590 (2021)) | `source_models/ssmb.rs` |
| Entangled-photon (NOON) source | 🧪 | Reports physical λ; quantum sharpening applied explicitly via `quantum_params()` bridge | `source_models/entangled.rs` |

## Deep-layer processes

| Capability | Status | Assumptions / limits | Code |
|------------|--------|----------------------|------|
| Volumetric (z-resolved) exposure | 🔶 | Separable I_aer(x,y;d0+z/n)·S(z); paraxial z/n focus mapping; split-step Dill bleaching with lateral-mean PAC; exact TMM S(z) | `volumetric.rs` |
| LIGA / deep X-ray depth dose | 🔶 | Shadow printing (bypasses projection pipeline); Gaussian ½√(λg) proximity blur; μ_en ≈ μ; smoothed absorption edges | `deep_xray.rs` + `materials/attenuation.rs` |
| Grayscale lithography | 🔶 | Log-linear contrast curve; fast 2.5D path ignores standing waves (volumetric path captures them) | `grayscale.rs` |
| Multi-beam interference / two-photon | ✅/🔶 | Analytic vector-sum field exact; per-beam scalar absorption; two-photon I² kinetics with Gaussian voxel PSF | `interference.rs` |
| 3D development (fast marching) | ✅ | Eikonal FMM with Godunov updates → sidewalls/undercut; rate field frozen during develop (level-set planned) | `volumetric.rs::develop_fast_marching` |

## Research modules

| Capability | Status | Assumptions / limits | Code |
|------------|--------|----------------------|------|
| ILT mask optimization | 🔶 | Adjoint-**inspired** proxy gradient (correlation-based), not a true adjoint solve through the SOCS kernels | `ilt.rs` |
| DSA block copolymer | 🔶 | Analytic morphology composition + empirical defectivity; not an SCFT solver | `dsa.rs` |
| Ptychography (ePIE) | ✅ | Genuine iterative object+probe reconstruction | `ptychography.rs` |
| Quantum lithography (NOON λ/2N) | 🧪 | N-photon sharpening with fidelity mixing and η^(N−1) flux penalty; entirely theoretical | `quantum.rs` |
| MNSL moiré nanosphere lattices | ✅ | Rayleigh scattering; single-z substrate coupling | `mnsl.rs` |

## Infrastructure

| Capability | Status | Assumptions / limits | Code |
|------------|--------|----------------------|------|
| Materials database | 🔶 | VUV tabulated n,k (126–160 nm) mature; EUV 13.5 nm Si/Mo canonical CXRO, Ru/Ta/resists approximate; BEUV 6.7 nm entries approximate (verify vs henke.lbl.gov); X-ray mass attenuation 0.1–20 keV for LIGA (smoothed edges) | `materials/` |
| Optics honesty guard | ✅ | CLI warns on refractive optics < 50 nm; `[optics] type` tag selects Schwarzschild/zone plate; GUI auto-selects Schwarzschild < 50 nm | `highuvlith-cli/src/config.rs`, `highuvlith-gui` |
| Shot-to-shot dose jitter → LER | ✅ | Gamma-distributed dose factor from `shot_to_shot_rms()` (SASE statistic); photon density fixed (was 1000× low) | `stochastic.rs` |
| Python bindings (zero-copy numpy, GIL release) | ✅ | Stub drift-guarded by `tests/python/test_stub_drift.py` | `crates/highuvlith-py` |
| Jupyter notebooks | ✅ | Three example notebooks (quickstart, source gallery, deep litho) executed headlessly in CI, plus ipywidgets helpers | `examples/notebooks/`, `python/highuvlith/interactive.py` |
| GPU compute backend | 🗺️ | `ComputeBackend` trait stub only | `compute/` |
