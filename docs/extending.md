# Extending highuvlith

**Status:** ✅ Implemented — every recipe below references extension seams that exist in the code today; follow them and the type system plus the test suite will catch most wiring mistakes.

Three extension points cover most needs: a new **light source**, a new **optical system**, or a new **process module**. In all three cases the honesty rules apply: document every approximation in the module header under a `# Model status` section, and update [capability-matrix.md](./capability-matrix.md) plus the relevant docs page's `**Status:**` badge **in the same PR** (that file's header makes this a hard rule).

## (a) Adding a source family

The nine existing families are the template. The pipeline consumes a source only through the `LithographySource` trait (wavelength, spectral weights, pupil intensity, bandwidth, photon density, pulse/coherence metadata), so a new family never touches imaging or resist code.

1. **Create the model file** — `crates/highuvlith-core/src/source_models/<family>.rs`, and register it with a `pub mod <family>;` line in [source_models/mod.rs](../crates/highuvlith-core/src/source_models/mod.rs). Put reusable closed-form physics (resonance conditions, cutoff laws, kinematics) in [source_models/physics.rs](../crates/highuvlith-core/src/source_models/physics.rs) with fixture tests, not inline in the struct.

2. **Implement the trait** — reuse the shared helpers in [source.rs](../crates/highuvlith-core/src/source.rs) rather than re-deriving them:
   - `evaluate_illumination(&self.illumination, fx, fy)` for `intensity_at` (all pupil shapes incl. the graded `CoherentGaussian`),
   - `evaluate_spectral_weights(λ, Δλ, n, shape)` for `spectral_weights` (normalized to sum to 1),
   - `evaluate_multiline_weights` for comb spectra (note its imaging caveat: honest for bookkeeping, approximate for imaging),
   - `sigma_from_coherence(ζ, σ_core)` if the source is high-coherence — default its pupil to `IlluminationShape::CoherentGaussian`.

   Where the physics dictates the wavelength, **derive it** from machine parameters (as `SynchrotronSource` and `IcsSource` do) instead of storing a free set-point; store a set-point only when the real machine is tunable (as `XfelSource` is, gap-tunable). Override the pulse-metadata methods (`pulse_energy_j`, `rep_rate_hz`, `pulse_duration_s`, `transverse_coherence`, `shot_to_shot_rms`) for whatever the model genuinely tracks — `shot_to_shot_rms` feeds live Gamma dose jitter in [stochastic.rs](../crates/highuvlith-core/src/stochastic.rs).

3. **Wire `SourceKind`** (all in [source.rs](../crates/highuvlith-core/src/source.rs)): add the enum variant, one arm in the `for_each_source!` macro (every dispatched trait method then picks it up automatically), one arm in `kind_label()` returning the serde tag, a `From<YourSource> for SourceKind` impl, and a `pub use crate::source_models::<family>::YourSource;` re-export so downstream code keeps the flat `source::*` namespace. The `#[serde(tag = "type", rename_all = "snake_case")]` attribute makes the variant name the TOML/JSON discriminator.

4. **PyO3 factory** — add a `#[staticmethod]` constructor on `PySourceConfig` in [py_config.rs](../crates/highuvlith-py/src/py_config.rs) (pattern: validate → build the core struct → wrap in `SourceKind::<Variant>`). Then update the type stub `python/highuvlith/_native.pyi` — the drift guard `tests/python/test_stub_drift.py` fails if the compiled module and the stub disagree.

5. **CLI tag** — in [config.rs](../crates/highuvlith-cli/src/config.rs), add any family-specific optional fields to `SourceConfig` (grouped by family, all `Option<T>` defaulting to the reference-machine preset) and a match arm in `SimConfig::to_source()` for `type = "<tag>"`. If the wavelength is derived, add the 5% cross-check against an explicitly given `wavelength_nm` (see the `"synchrotron"` arm). `SimConfig::validate()` calls `to_source()`, so per-family physics checks run before any simulation work.

6. **Tests** — minimum set, following the existing families:
   - unit tests in the new file: spectral weights sum to 1, photon energy fixture (`hc/λ`), wavelength bounds, constructor rejects invalid parameters;
   - add the family to `test_new_source_families_toml_roundtrip` in source.rs (tag serialization, wavelength round-trip, trait invariants);
   - add the tag to `test_all_new_source_types_build` in the CLI config tests;
   - a Python binding test in `tests/python/test_sources.py`.

7. **Example TOML** — `examples/sim_<family>.toml` with the `type = "<tag>"` tag and family-specific fields (see `examples/sim_lpa_fel.toml`).

8. **Docs** — a new page `docs/sources/<family>.md` following the source-page template (Status / Overview / Generation physics / Real-machine parameters / Simulation model / Model coverage / Usage / Validation / References), a link from [sources/index.md](./sources/index.md), and the capability-matrix row.

### Skeleton

The minimum viable model file, showing which pieces are shared and which are yours:

```rust
// crates/highuvlith-core/src/source_models/dpp.rs
use serde::{Deserialize, Serialize};
use crate::source::{
    evaluate_illumination, evaluate_spectral_weights,
    IlluminationShape, LithographySource, SpectralShape,
}; // add validate_sigma when you write the fallible constructor

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DppSource {
    pub wavelength_nm: f64,          // derive from physics if the machine fixes it
    pub bandwidth_pm: f64,
    pub spectral_samples: usize,
    pub spectral_shape: SpectralShape,
    pub illumination: IlluminationShape,
    // family-specific fields: live ones must reach a trait method;
    // anything else is stored (inert) and must be documented as such.
}

impl LithographySource for DppSource {
    fn wavelength_nm(&self) -> f64 { self.wavelength_nm }
    fn bandwidth_pm(&self) -> f64 { self.bandwidth_pm }
    fn intensity_at(&self, fx: f64, fy: f64) -> f64 {
        evaluate_illumination(&self.illumination, fx, fy)
    }
    fn spectral_weights(&self) -> Vec<(f64, f64)> {
        evaluate_spectral_weights(self.wavelength_nm, self.bandwidth_pm,
                                  self.spectral_samples, &self.spectral_shape)
    }
    // override pulse_energy_j / rep_rate_hz / shot_to_shot_rms /
    // transverse_coherence only for what the model genuinely tracks
}
```

Then the four one-line wiring points in [source.rs](../crates/highuvlith-core/src/source.rs): `Dpp(DppSource)` variant, `SourceKind::Dpp($s) => $body` macro arm, `SourceKind::Dpp(_) => "dpp"` label arm, `impl From<DppSource> for SourceKind`.

## (b) Adding an optical system

1. **Create the file** — `crates/highuvlith-core/src/optics/<name>.rs`, declared via `pub mod <name>;` in [optics/mod.rs](../crates/highuvlith-core/src/optics/mod.rs).

2. **Implement `OpticalSystem`** — the required methods are `pupil_function(fx_norm, fy_norm, defocus_nm, wavelength_nm) -> Complex64` (magnitude = transmission/apodization, phase = aberrations + defocus; return zero outside the pupil support), `na()`, `reduction()`, `flare_fraction()`. Override `chromatic_defocus(Δλ_pm)` if the system is dispersive — `ProjectionOptics` uses a linear axial coefficient (nm/pm), `FresnelZonePlate` the zone-plate law Δf/f = Δλ/λ. `cutoff_frequency` and `rayleigh_resolution` have correct defaults. Use a fallible constructor returning `error::Result` (validate NA ∈ (0,1), obscuration ratios, zone widths).

3. **CLI** — extend the match in `SimConfig::to_optics()` ([config.rs](../crates/highuvlith-cli/src/config.rs)) with a new `type = "<name>"` tag; the CLI already carries optics through `Box<dyn OpticalSystem>`, so no other plumbing changes.

4. **PyO3** — `PyOpticsConfig` carries a type-erased `PyOpticsInner` enum (`Refractive` / `Schwarzschild` / `ZonePlate`) with an `as_dyn() -> &dyn OpticalSystem` accessor ([py_config.rs](../crates/highuvlith-py/src/py_config.rs)). Exposing a new system to Python means: a new `PyOpticsInner` variant, an arm in `as_dyn()` and `kind_label()`, a `#[staticmethod]` factory on `PyOpticsConfig` (pattern: `schwarzschild()` / `zone_plate()`), and a `_native.pyi` stub entry for the drift guard.

5. **Tests** — pupil is zero beyond cutoff; in-focus on-axis phase is zero; defocus produces a symmetric intensity response about best focus (see `analytical_validation.rs` for the existing defocus-symmetry test to mirror); one analytical anchor specific to the system (e.g. the Schwarzschild tests pin the annular obscuration fraction).

## (c) Adding a process module

Process modules (existing examples: [volumetric.rs](../crates/highuvlith-core/src/volumetric.rs), [deep_xray.rs](../crates/highuvlith-core/src/deep_xray.rs), [grayscale.rs](../crates/highuvlith-core/src/grayscale.rs), [interference.rs](../crates/highuvlith-core/src/interference.rs)) are single files at the crate root.

1. **Create** `crates/highuvlith-core/src/<name>.rs` and register `pub mod <name>;` in [lib.rs](../crates/highuvlith-core/src/lib.rs) (keep the module list alphabetical — house style of the existing entries).

2. **Consume the pipeline at the right altitude.** Take an `&AerialImageEngine` and call `compute()` / `compute_from_transmittance()` for lateral images (grayscale does the latter for continuous-transmittance masks); take a `&FilmStack` and call `intensity_profile()` for exact z-resolved standing waves (do **not** use the legacy `standing_wave()` — it is the old approximate method kept for MNSL); emit `Grid3D<f64>` PAC volumes if 3D, so results feed the development tiers in `volumetric.rs` (as `interference.rs` does). If the process is not projection imaging at all, bypass the engine explicitly and say so in the header (as `deep_xray.rs` does for LIGA shadow printing).

3. **Write the module header first**: one-paragraph physics summary, a `# Key equations` block, and a `# Model status` section that names every approximation at the site where it matters. This is the house style — the doc pages and capability matrix quote these headers.

4. **PyO3** — wrap the entry points in [crates/highuvlith-py](../crates/highuvlith-py) (free function or result class as appropriate) and update `_native.pyi` for the stub drift guard.

5. **CLI** — if the module warrants command-line exposure, add a subcommand or deep-mode flags under [crates/highuvlith-cli/src/commands/](../crates/highuvlith-cli/src/commands) and TOML sections in `config.rs`, with validation in `SimConfig::validate()`.

6. **Analytical tests are mandatory**, not optional: every existing process module pins at least one closed form — volumetric pins the Beer–Lambert closed-form exposure (`test_closed_form_beer_lambert_exposure`), deep_xray pins the monochromatic exponential depth dose (`test_monochromatic_depth_dose_exponential`), grayscale pins a contrast-curve fixture (`test_gamma_fixture`, `test_contrast_curve_round_trip`), interference pins the two-beam fringe period and its resist-index invariance (`test_two_beam_period_and_index_invariance`). Match that bar.

## Testing conventions

| Tier | Location | What it pins |
|------|----------|--------------|
| Unit tests | inline `#[cfg(test)]` per module | Constructors reject bad input; invariants (weights sum to 1, bounds, symmetry) |
| Analytical validation | [tests/analytical_validation.rs](../crates/highuvlith-core/tests/analytical_validation.rs) | Fresnel reflectance, Brewster angle, quarter-wave AR, defocus symmetry, energy conservation |
| Property-based | [tests/proptest_physics.rs](../crates/highuvlith-core/tests/proptest_physics.rs) | Reflectance ∈ [0,1] over random stacks, monotonicity properties |
| Python integration | [tests/python/](../tests/python) | Binding round-trips, API conveniences, error mapping (`conftest.py` fixtures) |
| Stub drift guard | `tests/python/test_stub_drift.py` | `_native.pyi` matches the compiled extension — fails CI when a new PyO3 symbol lacks a stub entry |

Run the full gauntlet before a PR:

```bash
cargo test -p highuvlith-core        # unit + analytical + proptest
cargo clippy -- -D warnings          # CI enforces -D warnings
cargo fmt --check
maturin develop && pytest tests/python/ -v
```

Regression tests that encode past bugs (e.g. `test_default_photon_density_matches_source_trait` pinning the 1000× photon-density slip) should never be weakened — add a comment explaining the history if you must touch one.

## Related pages

- [Architecture](./architecture.md) — where each extension trait sits
- [Capability matrix](./capability-matrix.md) — the row you must add or update
- [Roadmap](./roadmap.md) — check whether your extension is already planned with a sketched design
