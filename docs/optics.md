---
description: "One pupil contract, four optics: refractive dry and immersion lenses, EUV scanner projection optics, the Schwarzschild objective and the Fresnel zone plate — with the exact defocus phase and the opt-in multilayer pupil."
---

# Optical systems

**Status:** ✅ Implemented — four `OpticalSystem` implementations behind one trait: refractive dry and immersion, Schwarzschild and zone plate ✅, EUV scanner projection optics 🔶 (isotropic pupil, assumed obscuration); scalar pupils with the exact defocus phase in the image medium · [matrix](capability-matrix.md#optical-systems)

<p class="huv-lead">The aerial-image engine never sees a lens, mirror or zone plate — it sees a complex pupil function P(fx, fy; z, λ) whose magnitude is transmission and whose phase is aberration plus defocus. This page is for anyone choosing an optic or reading a pupil map: it states the trait every optic implements, the exact defocus phase they share, and what each of the four implementations gets right and leaves out. Polarization is not here: vector effects are applied by the imaging engine through the vector pupil on the vector-imaging page.</p>

<div class="huv-physics" markdown>

<figure markdown="span">

![Two ray cones converging on a wafer: in air the marginal ray of NA 0.93 leaves the pupil rim at 68.4 degrees, in water (n 1.437) a 70-degree cone carries NA 1.35; rho runs along the exit pupil, a dashed plane marks a defocus z, and an inset plots the defocus phase against rho.](assets/images/diagrams/optics-pupil-cone-light.svg#gh-light-mode-only)
![Two ray cones converging on a wafer: in air the marginal ray of NA 0.93 leaves the pupil rim at 68.4 degrees, in water (n 1.437) a 70-degree cone carries NA 1.35; rho runs along the exit pupil, a dashed plane marks a defocus z, and an inset plots the defocus phase against rho.](assets/images/diagrams/optics-pupil-cone-dark.svg#gh-dark-mode-only)

<figcaption><strong>Mechanism.</strong> The numerical aperture is n·sin θ of the marginal ray, so air caps it below 1 while water (n = 1.437) carries NA 1.35 at a 70° half-angle; the pupil coordinate ρ = n·sin θ/NA puts that ray on the rim, and a defocus z adds the exact phase Φ_z(ρ) = (2πn/λ)·z·(1 − √(1 − (NAρ/n)²)) across the pupil, steepest at the rim. <em>Schematic, not to scale.</em></figcaption>
</figure>

<figure markdown="span">

![Amplitude and phase maps of five pupils at one Rayleigh unit of defocus: a refractive NA 0.75 lens with Z9 spherical, the High-NA EUV ring with its assumed 0.2 NA obscuration, a Schwarzschild annulus, a uniform zone-plate pupil and an EUV pupil shaded by an assumed multilayer angle map.](assets/images/sim/optics-four-pupils-light.png#gh-light-mode-only)
![Amplitude and phase maps of five pupils at one Rayleigh unit of defocus: a refractive NA 0.75 lens with Z9 spherical, the High-NA EUV ring with its assumed 0.2 NA obscuration, a Schwarzschild annulus, a uniform zone-plate pupil and an EUV pupil shaded by an assumed multilayer angle map.](assets/images/sim/optics-four-pupils-dark.png#gh-dark-mode-only)

<figcaption><strong>In the engine: four pupils at one Rayleigh unit of defocus.</strong> Amplitude above, phase below, each at z = λ/NA²: a refractive NA 0.75 lens with 0.05 λ of Fringe Z9 spherical, the High-NA EUV preset with its assumed 0.2·NA obscuration, a Schwarzschild objective and a first-order zone plate; the fifth column adds the opt-in Mo/Si multilayer pupil under an assumed six-mirror angle map. <em>Model: refractive projection optics (dry) ✅ · Zernike aberrations ✅ · EUV scanner projection optics (NA 0.33 / High-NA 0.55) 🔶 (obscuration assumed) · Schwarzschild reflective objective ✅ · Fresnel zone plate ✅ · EUV/BEUV multilayer pupil (opt-in) 🔶 (user-assumed angle map).</em></figcaption>
</figure>

<!-- huv-table: stack -->

| Optic | Pupil | Conventional band | Grade | Details |
|---|---|---|---|---|
| **ProjectionOptics** — refractive, dry | circular; Fringe-Zernike phase, radial apodization, axial chromatic coefficient; NA < 1 | 126–250 nm (CaF₂ VUV, fused-silica DUV) | ✅ Implemented | [refractive](#refractive-projection-optics) |
| **ProjectionOptics** — immersion | the same pupil with the defocus phase in the fluid; NA ≤ 0.95 n | 193 nm in water (193i, NA 1.35) | ✅ Implemented | [immersion](#immersion) |
| **EuvProjectionOptics** | circular, optional central obscuration; isotropic wafer-side stand-in | 13.5 nm scanners, NA 0.33 and 0.55 | 🔶 Simplified | [EUV projection](#euv-scanner-projection-optics) |
| **SchwarzschildObjective** | annular (central obscuration); scalar R² or the multilayer pupil | 13.5 nm, 6.7 nm, soft X-ray | ✅ Implemented | [Schwarzschild](#schwarzschild-objective) |
| **FresnelZonePlate** | uniform first-order pupil, cutoff 1/(2Δr_N), strongly chromatic | 1–30 nm (EUV to soft X-ray) | ✅ Implemented | [zone plate](#fresnel-zone-plate) |
| **MultilayerPupil** (opt-in) | angle-dependent Mo/Si or La/B amplitude and phase on the two reflective optics | 13.5 nm, 6.x nm | 🔶 Simplified | [multilayer pupil](#multilayer-pupil) |

**Same thing elsewhere:** Python `OpticsConfig(numerical_aperture=0.75)` · CLI `highuvlith simulate` · TOML `[optics] type = "refractive"` · GUI Optics panel · physics → [Pipeline, Stage 2](pipeline.md#stage-2--optics)

</div>

## The trait contract

```rust
pub trait OpticalSystem: Send + Sync {
    fn pupil_function(&self, fx_norm: f64, fy_norm: f64,
                      defocus_nm: f64, wavelength_nm: f64) -> Complex64;
    fn na(&self) -> f64;
    fn reduction(&self) -> f64;
    fn flare_fraction(&self) -> f64;
    fn clone_box(&self) -> Box<dyn OpticalSystem>;                           // engine keeps a copy
    fn chromatic_defocus(&self, delta_wavelength_pm: f64) -> f64 { 0.0 }  // default achromatic
    fn cutoff_frequency(&self, wavelength_nm: f64) -> f64 { self.na() / wavelength_nm }
    fn immersion_index(&self) -> f64 { 1.0 }                               // image-space medium
    fn rayleigh_resolution(&self, wavelength_nm: f64) -> f64 { 0.61 * wavelength_nm / self.na() }
}
```

Conventions the imaging engine relies on:

- `(fx, fy)` are normalized to `cutoff_frequency(λ)` at the wavelength being imaged; every implementation returns zero for $\rho > 1$.
- `pupil_function(…, z, λ)` is the pupil **at wavelength λ with the image plane `z` away from best focus at λ**. It must not contain a chromatic focus shift: the engine adds `chromatic_defocus(Δλ_pm)` (relative to the source center wavelength) explicitly in `compute_multiwavelength` and `compute_polychromatic`, so nothing is counted twice.
- `clone_box` lets `AerialImageEngine` own the optics; it re-evaluates the pupil whenever it builds kernels for a new focus plane or wavelength. `Box<dyn OpticalSystem>` implements `Clone` through it.
- `immersion_index()` is the refractive index of the image-space medium (1 dry/vacuum). The defocus phase is evaluated in it, the engine requires `na() < immersion_index()`, and vector imaging requires `VectorSettings::image_index` to equal it (inherited when left at 1.0) — one image-space medium for the defocus phase and the vector fields.
- `reduction()` enters only the vector radiometric (obliquity) factor: the engine copies it into `VectorSettings::reduction`. Mask coordinates are wafer-scale everywhere.
- **Pupil magnitude sets absolute radiometry only.** Images are reported as relative intensity by default — divided by the clear-field intensity $I_{\text{clear}} = \sum_s w_s |P(s)|^2$ of the same illumination, optics and focus — so a clear mask images to 1 for every optic here (zone-plate efficiency, mirror reflectivity, obscuration and apodization included). The absolute value is `AerialImageEngine::clear_field_intensity(z)`; `ImageNormalization::Absolute` skips the division (see the [pipeline](pipeline.md#image-normalization-relative-intensity)).

## Defocus phase

All implementations use the shared helpers `optics::defocus_phase_in_medium` / `optics::defocus_phase` (the latter is the `n = 1` case): the exact phase that the plane-wave order at $n \sin\theta = \mathrm{NA} \rho$ accumulates over a focus displacement $z$ in an image-space medium of index $n$, relative to the axial wave (angular-spectrum propagation; $\lambda$ is the vacuum wavelength):

```math
\Phi_z(\rho) = \frac{2\pi n}{\lambda}\, z\, \left(1 - \sqrt{1 - \left(\frac{\mathrm{NA}\,\rho}{n}\right)^2}\right)
\;\xrightarrow{\ \mathrm{NA}\rho \ll n\ }\; \frac{\pi z\, \mathrm{NA}^2 \rho^2}{n\,\lambda},
```

evaluated as $(2\pi n/\lambda) z \sin^2\theta/(1+\cos\theta)$ to avoid cancellation. It is continuous in $n$ and reduces exactly to the dry formula $(2\pi/\lambda) z (1 - \sqrt{1-\mathrm{NA}^2\rho^2})$ at $n = 1$. The paraxial form (the first engine's behaviour, generalized to $n$) is selected per optic with `paraxial_defocus = true` (a `#[serde(default)]` field, so old configs parse). Dry: the two agree to $\sin^2\theta/4$ relative (0.25 % at NA 0.1); at NA 0.8 the paraxial phase is 20 % short at the pupil edge and images at two Rayleigh depths differ by ~0.1 in intensity — the known failure of paraxial λ/NA² depth-of-focus scaling at high NA (Lin 2002). In water at fixed NA·ρ the phase is smaller (1.83 → 1.03 rad at NA·ρ = 0.9, z = 100 nm, λ = 193.368 nm): immersion buys depth of focus.

<figure markdown="span">

![Defocus phase across the pupil at 100 nm defocus and 193.368 nm for NA 0.1 and 0.8 in air and NA 1.35 in water, exact against paraxial, 1.83 rad in air falling to 1.03 rad in water at pupil position 0.9; a second panel shows the paraxial rim error rising from 0.25 % at NA 0.1 to 20 % at NA 0.8.](assets/images/sim/optics-defocus-phase-light.png#gh-light-mode-only)
![Defocus phase across the pupil at 100 nm defocus and 193.368 nm for NA 0.1 and 0.8 in air and NA 1.35 in water, exact against paraxial, 1.83 rad in air falling to 1.03 rad in water at pupil position 0.9; a second panel shows the paraxial rim error rising from 0.25 % at NA 0.1 to 20 % at NA 0.8.](assets/images/sim/optics-defocus-phase-dark.png#gh-dark-mode-only)

<figcaption><strong>The exact defocus phase and its paraxial limit.</strong> Φ<sub>z</sub>(ρ) at z = 100 nm and 193.368 nm for NA 0.1 and 0.8 in air and NA 1.35 in water: the paraxial πzNA²ρ²/(nλ) is 0.25 % low at the rim for NA 0.1 and 20 % low at NA 0.8, and in water at the same pupil position NA·ρ = 0.9 the phase is smaller (1.83 → 1.03 rad) — immersion buys depth of focus. The curves agree with <code>OpticsConfig.pupil_map</code> to 1e-6 rad. <em>Model: defocus / through-focus imaging ✅ · immersion lithography (193i) ✅.</em></figcaption>
</figure>

The pupil is

```math
P(\rho, \theta) = A(\rho)\, e^{\,i\left( W_{\mathrm{aberr}}(\rho,\theta) + \Phi_z(\rho) \right)}, \qquad \rho \le 1 .
```

**Honesty:** $n$ is the immersion medium (water for 193i, 1 otherwise); focus *inside the resist* (index of the resist) is not modeled by the pupil — the volumetric path maps depth to defocus with its own $z/n$ approximation.

`chromatic_defocus` is the hook both spectral paths use: each spectral sample's wavelength offset (in pm) becomes an extra focus shift. `compute_multiwavelength` additionally rebuilds the kernels at every wavelength (cutoff, source-point spacing and pupil all at λ_i); `compute_polychromatic` keeps the center-wavelength kernels (narrow-band approximation) — see the [pipeline](pipeline.md#spectral-imaging).

## Refractive projection optics

[`ProjectionOptics`](../crates/highuvlith-core/src/optics/mod.rs) ✅ models a refractive projection lens: circular pupil, Fringe-Zernike aberration phase from user-supplied `(index, coefficient_in_waves)` pairs (table extends to fringe index 48 in `math/zernike`), radial apodization, uniform flare, and a **linear axial-chromatic coefficient** in nm of defocus per pm of wavelength offset. `ProjectionOptics::new(na)` builds a **dry** lens: it validates NA ∈ (0, 1) (NA ≥ 1 is rejected) and defaults to 4× reduction, 2% flare, and `axial_chromatic_nm_per_pm = 15.0` — the CaF2-only value at 157 nm (no achromatization partner exists at VUV; typical published values are 10–30 nm/pm). Apodization options: `Uniform`, `Quadratic` T(ρ) = 1 − αρ², `Gaussian` T(ρ) = exp(−αρ²). Convenience: `rayleigh_dof(λ) = n λ/(2 NA²)` (λ/(2 NA²) dry). The first column of the pupil gallery above is this optic with 0.05 λ of Z9 spherical.

### Immersion

Immersion lithography ✅ fills the gap between the last lens element and the wafer with a liquid of index $n > 1$, so the image-side numerical aperture $\mathrm{NA} = n \sin\theta_{\max}$ can exceed 1 — the mechanism drawing in the Physics Strip shows the 70° cone that carries NA 1.35 in water:

- `ProjectionOptics::immersion(na, immersion_index)` — validates $n \ge 1$ and $0 < \mathrm{NA} \le 0.95 n$ (`IMMERSION_NA_MARGIN`: the marginal ray must stay clear of grazing incidence in the fluid); chromatic aberration defaults to 0 (the 157 nm CaF2 value of `new` does not apply — set your lens's coefficient).
- `ProjectionOptics::immersion_193i()` — the ArF water-immersion preset: NA 1.35, $n$ = `WATER_INDEX_193NM` = 1.437 (published measurements of ultrapure water near 193.4 nm and room temperature give 1.436–1.437; 1.44 is the common rounded figure), 4×, 2 % flare. NA/n = 0.94.
- What changes physically: the defocus phase is evaluated in the medium (above), `immersion_index()` reports $n$, and vector imaging evaluates the field in the medium. What does not: the scalar spatial-frequency cutoff stays $\mathrm{NA}/\lambda$ (the medium changes angles, not the mask frequencies that pass), so resolution follows $\lambda/(\mathrm{NA}(1+\sigma))$ with the larger NA.
- **Honesty:** at NA 1.35 the scalar pupil misses polarization effects (TM contrast loss) — use `ImagingModel::Vector`, which takes $n$ as the image-medium index. Water absorption, bubbles, and temperature dependence of $n$ are not modeled.

<figure markdown="span">

![Contrast versus pitch at 193 nm: dry NA 0.93 stops imaging near 109 nm pitch, water immersion at NA 1.35 keeps imaging down to about 75 nm; vector imaging shows unpolarized light losing contrast and TE polarization keeping it.](assets/images/sim/imaging-immersion-193i-light.png#gh-light-mode-only)
![Contrast versus pitch at 193 nm: dry NA 0.93 stops imaging near 109 nm pitch, water immersion at NA 1.35 keeps imaging down to about 75 nm; vector imaging shows unpolarized light losing contrast and TE polarization keeping it.](assets/images/sim/imaging-immersion-193i-dark.png#gh-dark-mode-only)

<figcaption><strong>Water immersion: NA 1.35 prints pitches that dry NA 0.93 cannot.</strong> Simulated contrast of 1:1 lines versus pitch for ArF (193.368 nm, conventional σ 0.9, no flare): dry optics at NA 0.93 against water immersion at NA 1.35 (n = 1.437, <code>OpticsConfig.immersion</code>). The scalar curves show the NA gain; the vector curves show why 193i needs polarization control — unpolarized light loses much of the gain, TE (y-) polarized light keeps it. <em>Model: immersion lithography (193i) ✅ · Hopkins imaging (factorized TCC/SOCS) ✅ · vector (polarized) high-NA imaging ✅; thin mask, no water absorption or in-resist focus.</em></figcaption>
</figure>

## EUV scanner projection optics

[`EuvProjectionOptics`](../crates/highuvlith-core/src/optics/euv.rs) 🔶 is the wafer-side pupil of an EUV scanner projection box: circular pupil of numerical aperture NA with an optional **central obscuration** (radius `central_obscuration` as a fraction of NA, default 0), Fringe-Zernike aberrations, the exact defocus phase in vacuum, a uniform flare fraction (0 in the presets — flare is not modeled by default; set a measured value), and an intensity transmission `transmission` (default 1; scales only absolute radiometry). The second column of the pupil gallery is the High-NA preset: the ring is the assumed obscuration. Presets:

| Preset | NA | Central obscuration | Reduction | Basis |
|--------|----|---------------------|-----------|-------|
| `nxe_033()` | 0.33 | 0 | 4× | 0.33-NA (NXE-class) EUV projection optics — unobscured pupil |
| `high_na_055()` | 0.55 | 0.2·NA (**assumed**) | 4× (isotropic stand-in) | 0.55-NA (High-NA / EXE-class) — centrally obscured pupil (Levinson 2022) |

**Why this is 🔶, not exact:**

- **Isotropic wafer-side pupil.** High-NA EUV uses *anamorphic* magnification (4× in the slit direction, 8× in the scan direction); that changes mask-side dimensions and angles only. Mask coordinates in highuvlith are wafer-scale and the pupil is circular, so anamorphic effects are **not modeled**; `reduction` = 4 feeds only the vector radiometric factor.
- **Obscuration size is an assumption.** High-NA projection optics are centrally obscured; 0.2·NA is a representative value, not a published design figure — set `central_obscuration` for your study. The obscuration blocks the zero order of source points with |σ| < 0.2 and removes low pupil frequencies; with near-on-axis illumination imaging becomes dark field (the engine then reports absolute rather than relative intensity).
- **Not modeled:** mask 3D effects (absorber thickness, shadowing at the 6° chief ray), the non-telecentric chief ray, and polarization (use `ImagingModel::Vector`; at NA 0.55 unpolarized imaging already loses some contrast). Multilayer angular/spectral reflectance is off by default (uniform pupil) and available through the optional `multilayer` field ([below](#multilayer-pupil)).

The [High-NA chapter](future/high-na-and-hyper-na.md) puts this pupil beside the published tool numbers.

## Fresnel zone plate

[`FresnelZonePlate`](../crates/highuvlith-core/src/optics/zone_plate.rs) ✅ models the workhorse optic of X-ray microscopy. Zone radii follow $r_n = \sqrt{n \lambda f}$; resolution is set by the outermost zone width, giving

```math
\mathrm{NA} = \frac{\lambda}{2\,\Delta r_N}, \qquad f = \frac{r_N^2}{N \lambda}, \qquad \frac{\Delta f}{f} = \frac{\Delta\lambda}{\lambda}
```

The last relation is the zone plate's defining weakness: it is **strongly chromatic** (`chromatic_defocus_per_pm` = f·10⁻³/λ per pm), which is why zone-plate systems demand narrow-band or monochromatized sources. Because the outermost zone diffracts at $\sin\theta = \lambda/(2\Delta r_N)$, the transmitted spatial-frequency cutoff $1/(2\Delta r_N)$ does **not** depend on wavelength: `cutoff_frequency` and `rayleigh_resolution` (= 1.22 Δr_N) are overridden accordingly, and the defocus phase uses the NA at the imaging wavelength, $\mathrm{NA}(\lambda) = \lambda/(2\Delta r_N)$ (`na()` reports the design-wavelength value). Following the trait convention, the pupil carries **no** chromatic phase: defocus is measured from best focus at the imaging wavelength, and the in-band focal shift relative to the source center wavelength comes from `chromatic_defocus` in the spectral imaging paths. The static focal shift of operating a zone plate away from its design wavelength is therefore treated as refocused, not simulated.

- First-order diffraction efficiency depends on the zone profile — `ZonePlateEfficiency::Binary` (1/π² ≈ 10.1%), `Phase` (4/π² ≈ 40.5% for a π-shift), `Blazed { efficiency }` (up to ~100% theoretical) — applied as a uniform pupil amplitude `sqrt(efficiency)` (absolute clear-field intensity η; relative images are normalized to 1). The fourth column of the pupil gallery is this uniform pupil.
- An optional central stop blocks ρ < `central_stop_fraction` (zero-order suppression); `flare_fraction()` is a two-value heuristic: 5% with a stop, 15% without (zero-order leakage).

## Schwarzschild objective

[`SchwarzschildObjective`](../crates/highuvlith-core/src/optics/schwarzschild.rs) ✅ models the two-concentric-mirror objective used where no lens can exist. The secondary mirror shadows the pupil center, producing an **annular pupil**: zero for ρ < `obscuration_ratio` (typical 0.2–0.4) — the third column of the pupil gallery. Mirror coatings enter as a single scalar per-mirror reflectivity; the two-mirror system transmission R² is applied as pupil amplitude √(R²) = R. Presets:

| Preset | NA | Obscuration | R (per mirror) | Coating basis |
|--------|----|-------------|----------------|---------------|
| `euv_standard()` | 0.33 | 0.25 | 0.67 | Mo/Si multilayer at 13.5 nm |
| `beuv()` | 0.25 | 0.30 | 0.50 | La/B4C multilayer at 6.7 nm |
| `soft_xray(na)` | user | 0.30 | 0.30 | grazing-incidence / multilayer, ~1 nm |

**Honesty:** by default the reflectivity is a **scalar constant** — a documented choice, not a hard limitation: attach a [`MultilayerPupil`](#multilayer-pupil) (`multilayer` field) to replace it with the angle-dependent Mo/Si or La/B₄C multilayer response across the pupil. There is no per-mirror wavefront error (the pupil phase is defocus only, no Zernike terms on this type), and the trait's default `chromatic_defocus = 0` applies (mirrors are achromatic to first order; the multilayer's spectral response enters per wavelength in `compute_multiwavelength` when a `MultilayerPupil` is attached). The obscuration also removes the zero order of near-axis source points, so the absolute clear-field intensity is $R^2 \times$ (fraction of the source outside the obscuration); relative images are normalized to 1 (dark field — clear field below 1 % of $R^2$, e.g. a near-coherent beam inside the obscuration — stays absolute).

## Multilayer pupil

The angle-dependent mirror response 🔶. [`MultilayerPupil`](../crates/highuvlith-core/src/optics/multilayer_pupil.rs) (opt-in, `multilayer` field of `SchwarzschildObjective` and `EuvProjectionOptics`) replaces the scalar-constant mirror reflectivity with the angle-dependent response of Bragg multilayer coatings. Each ray through the wafer-side pupil point $p$ (normalized to NA) meets mirror $j$ at the incidence angle

```math
\theta_j(p) = \left| \theta_{c,j} + t_j \, (p \cdot u_j) + g_j \, |p|^2 \right| ,
```

a user-supplied `MirrorAngleMap` (`center_deg` $\theta_c$, linear `tilt_deg` $t$ along `azimuth_deg`, rotationally symmetric `radial_deg` $g$). The coating's s and p amplitude reflectances $r_s(\lambda, \theta)$, $r_p(\lambda, \theta)$ come from the [`MultilayerMirror`](../crates/highuvlith-core/src/materials/multilayer.rs) model of the [materials page](materials.md) (Parratt recursion with Névot–Croce roughness, Henke optical constants; Mo/Si 40×6.9 nm: 72.9 % at 13.5 nm, normal incidence), multiplied along the mirror train and combined into one scalar pupil factor:

```math
R_s(p) = \prod_j r_s(\lambda, \theta_j(p)), \qquad R_p(p) = \prod_j r_p(\lambda, \theta_j(p)), \qquad
M(p) = \sqrt{\frac{|R_s|^2 + |R_p|^2}{2}} \; e^{\, i \arg(R_s + R_p)} .
```

$|M|^2$ is the unpolarized intensity transmission of the mirror train along that ray; $\arg M$ its phase. The response is tabulated once per wavelength (0°–60°, 0.02° steps, cubic interpolation, relative error ≲ 1e-6) when the engine calls `OpticalSystem::prepare(λ)` before a kernel build — pupil sampling never re-runs the Parratt recursion, and images reuse the kernels. The fifth column of the pupil gallery shows what an assumed six-mirror angle map does to the NA 0.33 pupil.

**With the default clear-field normalization the overall reflectance level cancels**: a uniform angle map (same angle everywhere) leaves relative images unchanged and only rescales `clear_field_intensity` (by $|r|^4$ for two mirrors) — test `uniform_multilayer_reflectance_is_only_a_dose_change`. The angle *dependence* shows up in the image as pupil apodization and phase, not as a dose change: with 0° at the pupil centre rising to 15° at the rim on two Mo/Si mirrors, a 60 nm grating at 13.5 nm / NA 0.33 loses in-focus contrast (rim darkened, reflection phase acting as an aberration) and its best focus moves (contrast higher at z = +60 nm than at z = 0) — test `angle_dependent_multilayer_apodizes_and_shifts_focus`.

**Why this is 🔶:** the angle maps are **user assumptions** (first/second-order models; real maps come from ray-tracing the actual design, which the repo does not contain); all mirrors share one coating and one s/p basis per ray (coaxial); the scalar polarization average above is applied even in vector mode (polarization-resolved $r_s \ne r_p$ on the TE/TM field components is not modeled); `mirror_reflectivity` (Schwarzschild) is ignored while a multilayer is attached, `transmission` (EUV projection) multiplies it. Python: `OpticsConfig.schwarzschild(...).with_multilayer_pupil([(center_deg, tilt_deg, azimuth_deg, radial_deg), ...], coating="mo_si", periods=40, period_nm=6.9, gamma=0.4)` (also for `euv_projection` optics). CLI: the `[optics.multilayer_pupil]` table with the same parameters ([configuration](configuration.md#optics)).

## Wavelength bands

**Wavelength ranges are conventional, not enforced.** Nothing in the trait stops you evaluating a CaF2 lens at 13.5 nm — the math will happily produce an image. **Physically, no transparent lens material exists below ~110 nm** (CaF2's transmission edge), so refractive results at shorter wavelengths are not physical. The frontends guard this:

- **CLI** ([config.rs](../crates/highuvlith-cli/src/config.rs)): selecting `type = "refractive"` with a source wavelength below 50 nm prints a warning — *"no transparent lens material exists below ~110 nm; results are not physical"* — and suggests `type = "schwarzschild"` or `"zone_plate"`. The `schwarzschild` type auto-picks the preset by band (soft-X-ray < 3 nm, BEUV < 10 nm, else EUV) and then applies your NA/flare; `zone_plate` defaults the outer zone width to λ/(2 NA) if unset.
- **GUI** ([imaging.rs](../crates/highuvlith-gui/src/imaging.rs)): the default *Auto* optics choose a refractive lens above 110 nm and reflective EUV projection optics below; a refractive lens forced below 110 nm is flagged as unphysical in the panel (see [gui.md](gui.md#optics)).
- The "conventional band" column of the table at the top of this page is documentation, not code — pick the optic that is physical for your source.

<figure markdown="span">

![Bars for each optic family over 1 to 250 nm — refractive above 110 nm, Mo/Si at 13.5, La/B at 6.x, the Schwarzschild auto-pick bands and the zone plate across the range — with the CLI 50 nm warning and the GUI 110 nm switch as lines and preset wavelengths as ticks.](assets/images/diagrams/spectrum-strip-bands-light.svg#gh-light-mode-only)
![Bars for each optic family over 1 to 250 nm — refractive above 110 nm, Mo/Si at 13.5, La/B at 6.x, the Schwarzschild auto-pick bands and the zone plate across the range — with the CLI 50 nm warning and the GUI 110 nm switch as lines and preset wavelengths as ticks.](assets/images/diagrams/spectrum-strip-bands-dark.svg#gh-dark-mode-only)

<figcaption><strong>Conventional use, documentation not code.</strong> The engine does not enforce these ranges; the CLI warns below 50 nm and the GUI switches optics at 110 nm. <em>No model badge.</em></figcaption>
</figure>

## Model coverage

One schema for every optic: which field is read where, and whether it is live.

### ProjectionOptics (dry and immersion)

| Field | Unit | Consumed by | Status |
|-------|------|-------------|--------|
| `na` | — | Pupil cutoff, defocus phase, TCC, resolution/DOF queries | live |
| `reduction` | × | Vector radiometric factor (engine copies it into `VectorSettings`); inert in scalar mode | live (vector) |
| `zernike_coefficients` | waves | Pupil phase in every TCC build | live |
| `flare_fraction` | 0–1 | Uniform flare blend in the aerial image | live |
| `axial_chromatic_nm_per_pm` | nm/pm | `chromatic_defocus` → per-sample focus shifts in `compute_multiwavelength` / `compute_polychromatic` | live |
| `apodization` | enum | Pupil magnitude (absolute radiometry; relative images are clear-field normalized) | live |
| `paraxial_defocus` | bool | Selects the paraxial instead of the exact defocus phase (default `false`) | live |
| `immersion_index` | — | Image-space medium: defocus phase, NA limit, vector image index (default 1.0 = dry) | live |

### EuvProjectionOptics

| Field | Unit | Consumed by | Status |
|-------|------|-------------|--------|
| `numerical_aperture` | — | Pupil cutoff, defocus phase, TCC | live |
| `central_obscuration` | 0–1 of NA | Central pupil hole | live |
| `reduction` | × | Vector radiometric factor only | live (vector) |
| `zernike_coefficients` | waves | Pupil phase | live |
| `flare` | 0–1 | Uniform flare blend | live |
| `transmission` | 0–1 | Pupil amplitude √T → absolute radiometry only | live (absolute mode) |
| `paraxial_defocus` | bool | Paraxial instead of exact defocus phase | live |
| `multilayer` | `Option<MultilayerPupil>` | Angle-dependent multilayer apodization + phase (default `None` = uniform pupil) | live (opt-in) |

### FresnelZonePlate

| Field | Unit | Consumed by | Status |
|-------|------|-------------|--------|
| `outermost_zone_width_nm` | nm | Cutoff 1/(2Δr_N), NA(λ), resolution, defocus phase | live |
| `num_zones` | — | Focal length → chromatic defocus | live |
| `design_wavelength_nm` | nm | `na()` (design value), focal length → chromatic defocus | live |
| `efficiency` | enum | Pupil amplitude | live |
| `central_stop_fraction` | 0–1 | Annular pupil hole + flare heuristic | live |
| `reduction_ratio` | × | Vector radiometric factor only (1:1 typical) | live (vector) |
| `paraxial_defocus` | bool | Paraxial instead of exact defocus phase (default `false`) | live |

### SchwarzschildObjective

| Field | Unit | Consumed by | Status |
|-------|------|-------------|--------|
| `numerical_aperture` | — | Pupil cutoff, defocus phase, TCC | live |
| `obscuration_ratio` | 0–1 | Annular pupil hole | live |
| `reduction_ratio` | × | Vector radiometric factor only | live (vector) |
| `mirror_reflectivity` | 0–1 | Pupil amplitude (R per mirror, R² system); ignored when `multilayer` is set | live |
| `flare` | 0–1 | Uniform flare blend | live |
| `paraxial_defocus` | bool | Paraxial instead of exact defocus phase (default `false`) | live |
| `multilayer` | `Option<MultilayerPupil>` | Angle-dependent multilayer apodization + phase replacing the constant (default `None`) | live (opt-in) |

## Usage

### Python

Every optics family is available from Python; `.kind` reports the family and `.immersion_index` the image medium.

<!-- verify-example -->
```python
import highuvlith as huv

optics = huv.OpticsConfig(numerical_aperture=0.75, reduction=4.0, flare_fraction=0.02)
arf_i  = huv.OpticsConfig.immersion_193i()             # NA 1.35 in water (n = 1.437)
wet    = huv.OpticsConfig.immersion(numerical_aperture=1.2, immersion_index=1.437)
euv    = huv.OpticsConfig.schwarzschild(numerical_aperture=0.33, mirror_reflectivity=0.67)
nxe    = huv.OpticsConfig.euv_nxe()                     # NA 0.33, unobscured
hna    = huv.OpticsConfig.euv_high_na()                 # NA 0.55, central obscuration 0.2 NA (assumed)
custom = huv.OpticsConfig.euv_projection(numerical_aperture=0.55, central_obscuration=0.15)
xray   = huv.OpticsConfig.zone_plate(outer_zone_width_nm=20.0, design_wavelength_nm=2.5)
for o in (optics, arf_i, euv, hna, xray):
    print(f"{o.kind:15s} NA {o.numerical_aperture:.2f}  image medium n = {o.immersion_index:.3f}")
```

???+ success "Output"

    ```text
    refractive      NA 0.75  image medium n = 1.000
    refractive      NA 1.35  image medium n = 1.437
    schwarzschild   NA 0.33  image medium n = 1.000
    euv_projection  NA 0.55  image medium n = 1.000
    zone_plate      NA 0.06  image medium n = 1.000
    ```

### CLI and TOML

```toml
# CLI: [optics] table, type = "refractive" (default) | "schwarzschild" | "zone_plate" | "euv_projection"
[optics]
type = "schwarzschild"        # band-matched preset, then na/flare applied
na = 0.33
flare_fraction = 0.03

# Water immersion (refractive): NA up to 0.95 × immersion_index
# [optics]
# na = 1.35
# immersion_index = 1.437

# EUV scanner projection optics (isotropic wafer-side pupil):
# [optics]
# type = "euv_projection"
# na = 0.55
# central_obscuration = 0.2   # fraction of NA; 0 for NA 0.33
# flare_fraction = 0.0

# Zone plate variant:
# [optics]
# type = "zone_plate"
# na = 0.1
# outer_zone_width_nm = 25.0  # optional; defaults to lambda / (2 * na)
```

The full key list is on the [configuration page](configuration.md#optics).

## Limits

- **Scalar pupils.** Polarization, the radiometric factor and film-entrance transmission live in the [vector model](vector-imaging.md), not in these pupils.
- **EUV projection optics are a wafer-side stand-in:** isotropic pupil, assumed obscuration, no anamorphic magnification, no mask 3-D, no chief-ray tilt.
- **Multilayer response is opt-in and assumption-driven:** user angle maps, one coating for every mirror, polarization-averaged.
- **No in-resist focus:** the pupil's medium is the immersion fluid or vacuum; depth inside the resist is the volumetric path's $z/n$ mapping.
- **Zone-plate off-design focal shift** is treated as refocused; only the in-band shift is simulated.
- **No water absorption, bubbles or thermal index change** for immersion; no mirror figure error for the Schwarzschild objective.

<details markdown="1">
<summary>Validation — 56 tests pin the pupils on this page</summary>

| Claim | Test | Tolerance |
|---|---|---|
| Cutoff, Rayleigh resolution, pupil inside / outside / at the edge, defocus adds phase, aberrated pupil, invalid NA 0 and 1 | `test_cutoff_frequency` … `test_invalid_na_one` ([optics/mod.rs](../crates/highuvlith-core/src/optics/mod.rs)) | exact |
| Exact and paraxial defocus phases vs independently computed values; ratio → 1 + sin²θ/4, linear in z, ∝ 1/λ; exact by default; serde default `false`; `clone_box` preserves the pupil | `test_defocus_phase_fixtures`, `test_defocus_phase_paraxial_limit_and_scaling`, `test_pupil_uses_exact_defocus_by_default`, `test_paraxial_flag_defaults_false_in_serde`, `test_clone_box_preserves_pupil` | fixture |
| Zone plate: NA = λ/(2Δr_N), resolution ≈ Δr_N, focal length, efficiencies 1/π² and 4/π², central stop, strong chromatic aberration, zero zone width rejected, λ-independent cutoff, defocus at the wavelength's NA with no chromatic phase | `test_zone_plate_na` … `test_pupil_defocus_uses_wavelength_na_and_no_chromatic_phase` ([zone_plate.rs](../crates/highuvlith-core/src/optics/zone_plate.rs)) | exact |
| Schwarzschild: EUV NA, annular pupil, system transmission 0.67², BEUV reflectivity, resolution ≈ 24.9 nm at 13.5 nm, invalid soft-X-ray NA, defocus matches the shared formula | `test_euv_na` … `test_defocus_matches_shared_formula` ([schwarzschild.rs](../crates/highuvlith-core/src/optics/schwarzschild.rs)) | exact |
| Immersion: medium phase fixtures, bit-identical to the dry formula at n = 1 and O(Δn) at n = 1 + 1e-9, constructors and validation (`new(1.2)` rejected, NA > 0.95 n rejected, DOF = nλ/(2NA²)), the pupil uses the medium defocus | `test_defocus_phase_in_medium_fixtures`, `test_defocus_phase_in_medium_is_continuous_at_n_1`, `test_immersion_constructors_and_validation`, `test_immersion_pupil_uses_medium_defocus` | fixture |
| EUV projection: presets, obscured pupil and defocus, transmission and aberrations, validation and serde defaults | `test_presets`, `test_obscured_pupil_and_defocus`, `test_transmission_and_aberrations`, `test_validation_and_serde_defaults` ([euv.rs](../crates/highuvlith-core/src/optics/euv.rs)) | exact |
| Multilayer pupil: angle-map geometry; $r_s = r_p$ at 0° and $\lvert M\rvert^2 = R(0)^2$ vs the direct multilayer reflectance (R(0°) ≈ 0.729); interpolated response vs direct evaluation; steep angles apodize the rim; validation, `prepare`, serde; the Schwarzschild constant is replaced; a uniform map is only a dose change; an angle-dependent map apodizes and shifts focus; wavelengths without optical constants are rejected | `angle_map_geometry`, `normal_incidence_two_mirror_transmission`, `interpolated_response_matches_direct_evaluation`, `steep_angles_apodize_the_pupil_edge`, `validation_prepare_and_serde`, `test_multilayer_pupil_replaces_constant_reflectivity`, `uniform_multilayer_reflectance_is_only_a_dose_change`, `angle_dependent_multilayer_apodizes_and_shifts_focus`, `multilayer_pupil_rejects_wavelengths_without_optical_constants` | 1e-12 / ≤ 2e-6 |
| Imaging with these pupils: the non-paraxial phase reduces to the paraxial one at low NA; the exact phase inside a coherent grating image | `nonparaxial_defocus_reduces_to_paraxial_at_low_na`, `test_coherent_three_beam_image_closed_form` (the [pipeline page](pipeline.md) lists the imaging suite) | ≤ 3e-3 at NA 0.1, > 0.03 at NA 0.8; 1e-12 |
| Immersion resolves a 90 nm pitch at 193 nm that dry NA 0.93 cannot; the water phase reproduces the closed-form image; NA above the medium index is rejected; vector mode inherits the index; NA 0.55 (obscured) resolves an 18 nm pitch NA 0.33 cannot; obscured pupil + on-axis light is dark field; lossy pupils image a clear field to exactly one | [tests/imaging_optics_presets.rs](../crates/highuvlith-core/tests/imaging_optics_presets.rs) (seven tests) | 1e-12 |

</details>

<details markdown="1">
<summary>Changes — behaviour that differs from the first engine</summary>

- **Zone-plate chromatic phase.** Up to v1 the zone-plate pupil also added an off-design focal-shift phase, which `compute_polychromatic` then counted a second time; the pupil now carries no chromatic phase and the engine adds `chromatic_defocus` once.
- **Defocus phase.** The first engine used the paraxial πzNA²ρ²/(nλ) everywhere; it survives per optic behind `paraxial_defocus = true`. The matrix's [change log](capability-matrix.md#audit-log) records the switch.
- **Defocus-phase figure of merit.** The water-versus-air phase at NA·ρ = 0.9 quoted on this page is now the computed 1.83 → 1.03 rad (previously typed as 1.84); the figure's generator asserts both numbers against `OpticsConfig.pupil_map`.

</details>

## References

- M. Born & E. Wolf, *Principles of Optics*, 7th ed. — pupil function, Zernike polynomials, defocus.
- D. Attwood & A. Sakdinawat, *X-Rays and Extreme Ultraviolet Radiation*, 2nd ed. — zone plate relations (NA = λ/2Δr, Δf/f = Δλ/λ, efficiency by profile) and multilayer reflectors.
- K. Schwarzschild, "Untersuchungen zur geometrischen Optik II" (1905) — the two-mirror aplanat.
- E. Louis et al., "Nanometer interface and materials control for multilayer EUV-optical applications," *Prog. Surf. Sci.* **86** (2011) — Mo/Si reflectivity ~0.67–0.70 at 13.5 nm.
- B. J. Lin, "The k3 coefficient in non-paraxial λ/NA scaling equations for resolution, depth of focus, and immersion lithography," *J. Micro/Nanolith. MEMS MOEMS* **1**(1), 7–12 (2002), doi:10.1117/1.1445798 — non-paraxial depth of focus; paraxial λ/NA² scaling overestimates DOF at high NA.
- H. J. Levinson, "High-NA EUV lithography: current status and outlook for the future," *Jpn. J. Appl. Phys.* **61**, SD0803 (2022), doi:10.35848/1347-4065/ac49fa — High-NA EUV projection optics with central obscuration and anamorphic 4×/8× magnification.

**Read next:** [Vector imaging](vector-imaging.md) · related: [Masks](masks.md) · [Materials — multilayer mirrors](materials.md) · [High-NA and Hyper-NA EUV](future/high-na-and-hyper-na.md) · [Beyond EUV](future/beyond-euv.md) · [matrix rows](capability-matrix.md#optical-systems)
