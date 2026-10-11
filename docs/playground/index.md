---
hide:
  - navigation
description: "Three browser calculators on closed-form physics — Rayleigh resolution and depth of focus, photon shot noise and a 1-D partially coherent aerial image — each verified against published numbers and reproduced in the engine."
---

# Playground

**Status:** 🔶 Simplified — three calculators on closed-form textbook physics (Rayleigh scaling, Poisson photon statistics, a 1-D scalar Abbe image), computed in your browser; teaching toys, not the Rust engine · [matrix](../capability-matrix.md#frontends-and-infrastructure)

<p class="huv-lead">Three teaching toys, not the simulator: how wavelength and NA set resolution and DOF, why shorter wavelengths worsen photon shot noise, and how the orders a lens admits build a partially coherent image. Each is checked against published values and reproduced in the <a href="../pipeline.md">engine</a>, the one to rely on.</p>

<ul class="huv-tiles">
<li><span class="huv-tile__label"><a href="#resolution-and-depth-of-focus">Resolution and DOF</a></span><span class="huv-tile__value">k₁λ/NA</span><span class="huv-tile__note">Rayleigh scaling, paraxial and exact DOF · <a href="?calc=rayleigh">deep link</a></span></li>
<li><span class="huv-tile__label"><a href="#photon-shot-noise">Photon shot noise</a></span><span class="huv-tile__value">1/√N</span><span class="huv-tile__note">photons per feature at a dose · <a href="?calc=shotnoise">deep link</a></span></li>
<li><span class="huv-tile__label"><a href="#partially-coherent-aerial-image-1-d-toy">1-D aerial toy</a></span><span class="huv-tile__value">Abbe sum</span><span class="huv-tile__note">orders in the pupil, live, with focus · <a href="?calc=aerial">deep link</a></span></li>
</ul>

## Resolution and depth of focus

Pick a wavelength preset (it also sets an example NA), then vary NA and the process factors;
the plot traces R and DOF against NA with every preset as a dot, and the table compares the
presets at your current k₁ and k₂.

<figure markdown="span">

![Two charts against numerical aperture from 0.2 to 1.45: the half-pitch k₁λ/NA at k₁ = 0.3 and the depth of focus k₂λ/NA² at k₂ = 1 (paraxial dashed, exact solid) for KrF 248 nm, ArF 193 nm dry and in water and EUV 13.5 nm, the calculator's presets as dots; EUV at NA 0.33 gives 12.3 nm and 120 nm.](../assets/images/playground/playground-rayleigh-light.svg#gh-light-mode-only)
![Two charts against numerical aperture from 0.2 to 1.45: the half-pitch k₁λ/NA at k₁ = 0.3 and the depth of focus k₂λ/NA² at k₂ = 1 (paraxial dashed, exact solid) for KrF 248 nm, ArF 193 nm dry and in water and EUV 13.5 nm, the calculator's presets as dots; EUV at NA 0.33 gives 12.3 nm and 120 nm.](../assets/images/playground/playground-rayleigh-dark.svg#gh-dark-mode-only)

<figcaption><strong>Resolution and depth of focus against NA.</strong> Half-pitch k₁λ/NA at k₁ = 0.3 (left) and depth of focus at k₂ = 1 (right: paraxial k₂λ/NA² dashed, the exact quarter-wave form solid, ending where NA reaches the index n) for KrF 248 nm, ArF 193 nm dry and in water (n 1.437) and EUV 13.5 nm; dots are the calculator's presets, hollow where the NA is illustrative. <em>Closed-form toy, same formulas as the calculator; not the engine. No model badge.</em></figcaption>
</figure>

<div class="huv-calc" data-huv-calc="rayleigh">
<noscript>This calculator needs JavaScript. The figure above and the formulas below give the same numbers by hand.</noscript>
</div>

```math
R = k_1 \frac{\lambda}{\mathrm{NA}}, \qquad p_{\min} = 2R
```

```math
\mathrm{DOF}_{\text{paraxial}} = k_2 \frac{\lambda}{\mathrm{NA}^2}, \qquad
\mathrm{DOF}_{\text{exact}} = \frac{k_2 \lambda}{2n (1-\cos\theta)}, \quad \sin\theta = \frac{\mathrm{NA}}{n}
```

- **R** is the half-pitch of the densest printable lines and p<sub>min</sub> the matching
  pitch. For a single exposure of dense lines k₁ cannot go below 0.25: at smaller pitches
  no first diffraction order fits through the pupil together with the zeroth.
- **DOF** is the total focus range. With k₂ = 1 the paraxial formula is Rayleigh's
  quarter-wave criterion: defocusing by ±λ/(2NA²) adds a quarter-wave path difference at
  the pupil edge.
- **The exact form** applies the same quarter-wave criterion to the non-paraxial defocus
  path difference n·z·(1 − cos θ) in a medium of index n (n ≈ 1.437 for water at 193 nm,
  the value of the simulator's immersion preset).
  Since 1 − cos θ = 2 sin²(θ/2), it scales with the inverse square of the numerical *half*
  aperture, as in B. J. Lin's non-paraxial scaling law (his k₃ corresponds to k₂/4 here).
  It matches the paraxial formula when NA is small and n = 1; at fixed NA an immersion
  medium raises the paraxial limit to k₂·n·λ/NA², the depth-of-focus gain that made
  immersion attractive.

### Verify

<!-- huv-table: stack -->
| Calculator gives | Published value |
|---|---|
| ArF immersion, NA 1.35, k₁ = 0.27: R = 0.27 × 193 / 1.35 = 38.6 nm | 38 nm, ASML NXT:2000i with dipole illumination |
| EUV, NA 0.33, k₁ = 0.32: R = 13.1 nm | 13 nm, ASML NXE systems |
| High-NA EUV, NA 0.55, k₁ = 0.32: R = 7.9 nm | 8 nm, ASML EXE systems |
| DOF at 13.5 nm, NA 0.33, k₂ = 1: 124 nm paraxial, 13.5 / (2 × (1 − cos 19.27°)) = 120 nm exact | Lin: the paraxial formula overestimates DOF by 10 % at NA 0.6 and 20 % at NA 0.8; here 2(1 − cos θ)/NA² = 0.90 and 0.80 |

The example NAs for 436, 365, 157 and 6.7 nm are illustrative round numbers, not a
particular tool's specification.

### In the engine

The engine has no k₁; it images a grating and reports the contrast. Through the
[immersion preset](../optics.md) (NA 1.35, water n = 1.437) with conventional σ 0.9
illumination, the smallest pitch that images is λ/(NA(1 + σ)) = 75.4 nm — the same cutoff the
toy's k₁ floor of 1/(2(1 + σ)) = 0.263 encodes — and 2R at k₁ = 0.27 sits just above it:

<!-- verify-example -->
```python
import highuvlith as huv

for pitch_nm in (70.0, 78.0, 100.0):
    r = huv.simulate_line_space(pitch_nm / 2, pitch_nm, wavelength_nm=193.368, na=1.35, sigma=0.9, immersion=True)
    print(f"pitch {pitch_nm:5.1f} nm: contrast {r.contrast:.3f}")
```

???+ success "Output"

    ```text
    pitch  70.0 nm: contrast 0.000
    pitch  78.0 nm: contrast 0.017
    pitch 100.0 nm: contrast 0.353
    ```

<!-- huv-table: stack -->
| Case (193 nm, NA 1.35, σ 0.9) | Toy (this page) | Engine (recorded above) |
|---|---|---|
| 70 nm pitch, k₁ = 0.24 | below the 0.263 cutoff: nothing images | contrast 0.000 |
| 78 nm pitch, k₁ = 0.27 (2R at the ASML k₁) | above the cutoff: an image forms | contrast 0.017 — resolvable in principle, far too faint to print |
| 100 nm pitch, k₁ = 0.35 | an image forms | contrast 0.353 |

## Photon shot noise

A dose is a count of photons. Shorter wavelengths carry more energy per photon, so the
same dose delivers fewer of them, and the Poisson fluctuation in a small area grows; the
plot traces the photon count and its 1σ against feature size for every preset wavelength.

<figure markdown="span">

![Two log–log charts against feature size from 5 to 100 nm at 30 mJ/cm²: incident photons in a square feature and the Poisson 1σ fluctuation 1/√N in percent for g-line 436, KrF 248, ArF 193, EUV 13.5 and 6.7 nm; a 20 nm square at 13.5 nm holds 8155 photons, a 1σ of 1.11 %.](../assets/images/playground/playground-shotnoise-light.svg#gh-light-mode-only)
![Two log–log charts against feature size from 5 to 100 nm at 30 mJ/cm²: incident photons in a square feature and the Poisson 1σ fluctuation 1/√N in percent for g-line 436, KrF 248, ArF 193, EUV 13.5 and 6.7 nm; a 20 nm square at 13.5 nm holds 8155 photons, a 1σ of 1.11 %.](../assets/images/playground/playground-shotnoise-dark.svg#gh-dark-mode-only)

<figcaption><strong>Photons per feature and shot noise.</strong> Incident photons × f<sub>abs</sub> (here 1) in a square feature at 30 mJ/cm² (left) and the Poisson 1σ fluctuation 1/√N (right) against feature size: the count falls with λ at fixed dose, so a 20 nm square holds 8155 photons at 13.5 nm, a 1σ of 1.11 %. <em>Closed-form toy, same formulas as the calculator; not the engine. No model badge.</em></figcaption>
</figure>

<div class="huv-calc" data-huv-calc="shotnoise">
<noscript>This calculator needs JavaScript. The figure above and the formulas below give the same numbers by hand.</noscript>
</div>

```math
E_{\text{ph}} = \frac{hc}{\lambda}, \qquad
n = \frac{D}{E_{\text{ph}}}, \qquad
N = n A f_{\text{abs}}, \qquad
\frac{\sigma_N}{N} = \frac{1}{\sqrt{N}}
```

- hc = 1239.84193 eV·nm and 1 eV = 1.602176634 × 10⁻¹⁹ J (CODATA); a dose of
  1 mJ/cm² is 10⁻¹⁷ J/nm², because 1 cm² = 10¹⁴ nm².
- Photon arrivals are Poisson distributed, so a mean count N fluctuates by √N; the
  relative fluctuation 1/√N is what varies the printed size of small features.
- f<sub>abs</sub> is the fraction of incident photons absorbed in the resist. Leave it at
  1 to count incident photons; a real resist film absorbs only part of the light, which
  makes the statistics worse.

### Verify

<!-- huv-table: stack -->
| Calculator gives | By hand |
|---|---|
| Photon energy at 13.5 nm: 91.84 eV = 1.471 × 10⁻¹⁷ J | 1239.84193 / 13.5 eV, × 1.602 × 10⁻¹⁹ J/eV |
| 30 mJ/cm² at 13.5 nm: 20.4 photons/nm² | 3 × 10⁻¹⁶ J/nm² ÷ 1.471 × 10⁻¹⁷ J |
| 20 nm × 20 nm square: about 8,160 photons, 1σ = 1.1 % | 20.4 × 400; 1/√8155 |
| 193 nm (6.42 eV): 291 photons/nm², 14.3 times more | 1239.84193 / 193 = 6.42 eV; 13.5 / 193 energy ratio |

### In the engine

The engine's stochastic module starts from the same photon count — `photons_per_square` and
`relative_shot_noise` are the closed forms above, exposed from the Rust core — and then goes
further: Poisson photon counts per pixel and shot-to-shot dose jitter feed its line-edge and
line-width roughness estimates (see [research modules](../research-modules.md) and the
[stochastics rows](../capability-matrix.md#stochastics) of the matrix).

<!-- verify-example -->
```python
import highuvlith as huv

for wavelength_nm in (13.5, 193.0):
    n = huv.photons_per_square(30.0, wavelength_nm, 20.0)
    print(f"{wavelength_nm:5.1f} nm: {n:8.0f} photons in a 20 nm square at 30 mJ/cm2, 1-sigma {100 * huv.relative_shot_noise(30.0, wavelength_nm, 20.0):.2f} %")
```

???+ success "Output"

    ```text
     13.5 nm:     8155 photons in a 20 nm square at 30 mJ/cm2, 1-sigma 1.11 %
    193.0 nm:   116590 photons in a 20 nm square at 30 mJ/cm2, 1-sigma 0.29 %
    ```

<!-- huv-table: stack -->
| 20 nm square at 30 mJ/cm² | Toy (this page) | Engine (recorded above) |
|---|---|---|
| Photons at 13.5 nm · 1σ | 8155 · 1.11 % | 8155 · 1.11 % |
| Photons at 193 nm · 1σ | 116,590 · 0.29 % | 116,590 · 0.29 % |
| Beyond the count | nothing: incident photons only | Poisson counts per pixel ✅, dose jitter ✅, LER/LWR Monte Carlo 🔶 |

## Partially coherent aerial image (1-D toy)

A line/space mask diffracts light into orders at spatial frequencies m/p. The projection
lens passes only the orders inside its pupil, |f| ≤ NA/λ, and the image is what those
orders rebuild. Each point of the illumination source tilts the orders by s·NA/λ, and
partially coherent imaging adds up the intensity images of all source points, which is
Abbe's method. The live pupil beside the plot shows which orders pass for the on-axis source
point and for the edges of the source; the k₁ slider walks the pitch through the three limits.

<figure markdown="span">

![The aerial toy's opening state, ArF 193 nm, NA 0.93, σ 0.7, 94 nm lines on a 187 nm pitch: left the pupil with the ±1 orders at ±1.11 pupil units, outside the pupil on axis but inside the (1+σ) support; right the 1-D partially coherent image over two periods, contrast 0.792, with the 0.3 threshold.](../assets/images/playground/playground-aerial-light.svg#gh-light-mode-only)
![The aerial toy's opening state, ArF 193 nm, NA 0.93, σ 0.7, 94 nm lines on a 187 nm pitch: left the pupil with the ±1 orders at ±1.11 pupil units, outside the pupil on axis but inside the (1+σ) support; right the 1-D partially coherent image over two periods, contrast 0.792, with the 0.3 threshold.](../assets/images/playground/playground-aerial-dark.svg#gh-dark-mode-only)

<figcaption><strong>The aerial toy's opening state.</strong> ArF 193 nm, NA 0.93, σ 0.7, 94 nm lines on a 187 nm pitch: the pupil with the orders at m·λ/(p·NA) = ±1.11 — outside the pupil for the on-axis source point, inside the (1+σ) support, so they pass for part of the source — and the 1-D scalar Abbe image over two periods, contrast 0.792, with the 0.3 resist threshold. <em>Closed-form toy, same formulas as the calculator; not the engine. No model badge.</em></figcaption>
</figure>

<div class="huv-calc" data-huv-calc="aerial">
<noscript>This calculator needs JavaScript. The figure above shows its opening state and the formulas below describe what it computes.</noscript>
</div>

For opaque lines of width w on a clear background with period p, the mask's Fourier
coefficients, the image and the defocus phase are

```math
c_0 = 1 - \frac{w}{p}, \qquad c_m = -\frac{\sin(\pi m w/p)}{\pi m}\ \ (m \neq 0),
```

```math
I(x) = \frac{1}{J}\sum_{j=1}^{J}\Bigl|\sum_{m: |f_{mj}| \le \mathrm{NA}/\lambda} c_m e^{i\phi_{mj}} e^{2\pi i m x/p}\Bigr|^2,
\qquad f_{mj} = \frac{m}{p} + s_j \frac{\mathrm{NA}}{\lambda},
```

```math
\phi_{mj} = 2\pi z\left(\sqrt{\left(\frac{n}{\lambda}\right)^2 - f_{mj}^2} - \frac{n}{\lambda}\right),
```

with J source points s<sub>j</sub> spread evenly over σ<sub>in</sub> ≤ |s| ≤ σ<sub>out</sub>
and z the defocus. Intensities are in units of the clear-field intensity. The readouts
are the image contrast (I<sub>max</sub> − I<sub>min</sub>)/(I<sub>max</sub> + I<sub>min</sub>),
the normalized image log-slope NILS = w·|d ln I/dx| at the nominal line edge, and the
printed line width where the image stays below a constant resist threshold. Each symbol
maps to one control:

<!-- huv-table: stack -->
| Symbol | Control | What it sets |
|---|---|---|
| λ, n | Wavelength · index | the photon and the image-space medium (n = 1.437 for the 193i preset) |
| NA, k₁ | NA · k₁ of the half-pitch | the pupil radius NA/λ; the k₁ slider sets the pitch to 2·k₁·λ/NA |
| σ<sub>out</sub>, σ<sub>in</sub> | σ outer · σ inner | the source extent on one diameter of the pupil; equal values make a dipole |
| p, w | Pitch · line width | the grating and its Fourier coefficients c<sub>m</sub> |
| z, threshold | Defocus · threshold | the defocus phase φ<sub>mj</sub>; the constant resist threshold the printed width is read at |

### Verify

<!-- huv-table: stack -->
| Setting | What the calculator shows |
|---|---|
| A clear mask (w → 0) | I = 1 everywhere |
| A pitch below λ / ((1 + σ<sub>out</sub>)·NA) | no first order passes for any source point: the image is flat and the contrast is zero |
| σ = 0 with only the orders −1, 0 and +1 inside the pupil | I(x) = [c₀ + 2c₁ cos(2πx/p)]², a single cosine fringe |
| σ<sub>in</sub> = σ<sub>out</sub> = λ/(2p·NA) for a pitch between λ/(2NA) and 3λ/(2NA) | two-beam dipole illumination: each pole passes two orders symmetric about the pupil centre, their defocus phases are equal, and the image no longer changes with focus |

The three limits the k₁ slider names are the same geometry drawn once: a 1:1 grating's orders
sit at m/(2k₁) in pupil units, so the ±1 orders leave a conventional fill's support at
k₁ = 1/(2(1 + σ)) and a matched dipole keeps two-beam imaging down to k₁ = 0.25.

<figure markdown="span">

![Three pupil diagrams: a sigma 0.7 fill at k1 0.40 with the first orders inside the support circle, a matched dipole at k1 0.30 with its zeroth and first order placed symmetrically, and the k1 = 0.25 limit with pole and order on opposite rims; the toy's image cut and contrast sit under each.](../assets/images/diagrams/pupil-orders-still-light.svg#gh-light-mode-only)
![Three pupil diagrams: a sigma 0.7 fill at k1 0.40 with the first orders inside the support circle, a matched dipole at k1 0.30 with its zeroth and first order placed symmetrically, and the k1 = 0.25 limit with pole and order on opposite rims; the toy's image cut and contrast sit under each.](../assets/images/diagrams/pupil-orders-still-dark.svg#gh-dark-mode-only)

<figcaption><strong>Three pupils, one rule.</strong> A conventional σ 0.7 fill still images at k₁ = 0.40 (the ±1 orders are inside the (1+σ)·NA/λ support circle); a dipole matched to the pitch images at k₁ = 0.30 with two beams; at k₁ = 0.25 pole and order sit on opposite rims and nothing below it prints in one exposure. The resting frame of the animation on the lithography-arithmetic page. <em>Closed form and geometry; not the engine. No model badge.</em></figcaption>
</figure>

What the toy leaves out, all of which matter in practice: the source is a line through the
pupil rather than a disk or a freeform shape; the light is scalar (no polarization, so no
high-NA TM contrast loss); the mask is infinitely thin (no mask 3-D effects); there are no
aberrations besides defocus, no flare, no thin-film stack and no resist blur. The Rust engine
models each of these, graded as the matrix grades it, and the [pipeline](../pipeline.md)
page states them stage by stage; it shares the toy's thin-mask assumption.

| What the toy leaves out | In the engine | Grade |
|---|---|---|
| A 2-D source | conventional, annular, dipole and quadrupole fills | ✅ Implemented |
| Polarization at high NA | the vector TE/TM mode ([vector imaging](../vector-imaging.md)) | ✅ Implemented |
| Aberrations beyond defocus | Fringe-indexed Zernike aberrations | ✅ Implemented |
| Flare | a uniform zeroth-order fraction | 🔶 Simplified |
| The film stack | the thin-film transfer matrix | ✅ Implemented |
| Mask thickness | a Kirchhoff thin mask, like the toy | 🔶 Simplified |

### In the engine

The same kind of calculation in the engine, with a 2-D source and Hopkins TCC/SOCS
imaging, for the toy's opening state: 94 nm lines at 187 nm pitch (k₁ = 0.45), a 193 nm
source, NA 0.93 and σ = 0.7. The numbers differ somewhat from the toy's, because the
engine's source is a filled disc and it adds 2 % flare.

<!-- verify-example -->
```python
import highuvlith as huv

for focus_nm in (0.0, 100.0, 200.0):
    r = huv.simulate_line_space(
        94.0, 187.0, wavelength_nm=193.0, na=0.93, sigma=0.7, focus_nm=focus_nm
    )
    nils = "n/a" if r.nils is None else f"{r.nils:.2f}"  # None: no line prints
    print(f"focus {focus_nm:5.0f} nm: contrast {r.contrast:.3f}, NILS {nils}")
```

???+ success "Output"

    ```text
    focus     0 nm: contrast 0.650, NILS 2.04
    focus   100 nm: contrast 0.417, NILS 1.27
    focus   200 nm: contrast 0.069, NILS 0.10
    ```

<!-- huv-table: stack -->
| Opening state (193 nm, NA 0.93, σ 0.7, 94/187 nm) | Toy (1-D, this page) | Engine (recorded above) |
|---|---|---|
| Contrast at best focus | 0.792 | 0.650 |
| Source | 41 points on one diameter of the pupil | a filled 2-D disc of radius σ, Hopkins TCC/SOCS |
| Flare, mask, polarization | none · thin mask · scalar | 2 % uniform flare · thin mask · scalar here (a vector mode exists) |

The engine's pupil fills are two-dimensional, which is where the toy's line of source points
stops being enough: an x-dipole raises the contrast of a dense grating only in its own
orientation, which the engine shows and the toy cannot.

<figure markdown="span">

![Three pupil fills (a filled disk, a ring and two poles on the x axis), the aerial-image cross-sections they give for 65 nm lines on a 130 nm pitch, below the coherent limit lambda/NA of 210 nm, with the dipole giving the deepest modulation, and contrast versus pitch for the three fills.](../assets/images/sim/imaging-illumination-shapes-light.png#gh-light-mode-only)
![Three pupil fills (a filled disk, a ring and two poles on the x axis), the aerial-image cross-sections they give for 65 nm lines on a 130 nm pitch, below the coherent limit lambda/NA of 210 nm, with the dipole giving the deepest modulation, and contrast versus pitch for the three fills.](../assets/images/sim/imaging-illumination-shapes-dark.png#gh-dark-mode-only)

<figcaption><strong>Off-axis illumination below λ/NA.</strong> F<sub>2</sub> 157.63 nm, NA 0.75, 1:1 lines on a 130 nm pitch (k<sub>1</sub> ≈ 0.31; λ/NA = 210 nm): (a–c) pupil fills of the source (circle = NA), all with outer σ ≤ 0.8 so they share the λ/(NA·1.8) ≈ 117 nm cutoff; (d) aerial-image cross-sections at best focus — moving the light to the pupil edge in the direction of the grating (annular, and most of all the x-dipole) puts more of it into two-beam (0, ±1) imaging and raises the contrast. (e) The same fills across pitch: the dipole gives the highest contrast here, but only for gratings of this orientation — the same pattern rotated by 90° does not get its off-axis benefit. <em>Model: Hopkins imaging (factorized TCC/SOCS) ✅ · illumination pupil fills ✅; no flare, thin mask, no resist.</em></figcaption>
</figure>

**Where next:** [Lithography arithmetic](../nodes/litho-math.md) · [Pipeline — Hopkins imaging](../pipeline.md#stage-4--aerial-image) · [The stochastic frontier](../future/stochastic-frontier.md) · [Getting started](../getting-started.md) · [Capability matrix](../capability-matrix.md)

## References

- M. Born and E. Wolf, *Principles of Optics*, 7th ed., Cambridge University Press (1999)
  — Abbe and Hopkins theories of partially coherent imaging.
- C. A. Mack, *Fundamental Principles of Optical Lithography*, Wiley (2007) — Rayleigh
  scaling, depth of focus and image metrics such as NILS.
- B. J. Lin, "The k₃ coefficient in nonparaxial λ/NA scaling equations for resolution,
  depth of focus, and immersion lithography", *J. Microlithography, Microfabrication, and
  Microsystems* (JM3, today *J. Micro/Nanopatterning, Materials, and Metrology*) **1**(1),
  7–12 (2002), [doi:10.1117/1.1445798](https://doi.org/10.1117/1.1445798)
  — the non-paraxial depth of focus and its 10–20 % correction at NA 0.6–0.8.
- NIST, [CODATA values of the fundamental constants](https://physics.nist.gov/cuu/Constants/) —
  h, c and the elementary charge used in the photon-energy calculation.
- ASML product pages for the example NAs:
  [NXT:870](https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt870) (KrF, NA up to 0.80),
  [NXT:1470](https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt1470) (ArF, NA up to 0.93),
  [NXT:2000i](https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt2000i) (ArF immersion, NA 1.35),
  [EUV systems](https://www.asml.com/en/products/euv-lithography-systems) (NA 0.33 and 0.55).
