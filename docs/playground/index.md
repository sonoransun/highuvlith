---
hide:
  - navigation
---

# Playground

**Status:** 🔶 Simplified — three calculators that run in your browser on textbook formulas (Rayleigh scaling, Poisson photon statistics, a 1-D scalar Abbe image); they are teaching toys, not the Rust engine.

These calculators build intuition for the numbers that recur across the site: how
wavelength and numerical aperture set resolution and depth of focus, why shorter
wavelengths make photon shot noise worse, and how a partially coherent image forms from
the diffraction orders a lens lets through. Every formula is written out below its
calculator with a "verify" note so you can check the arithmetic yourself.

!!! warning "Teaching toys, not the simulator"
    The code on this page is a few hundred lines of JavaScript using closed-form
    textbook physics. The real simulator is the Rust engine described in the
    [simulation pipeline](../pipeline.md): 2-D masks, 2-D sources, Hopkins TCC/SOCS
    imaging, thin films, resist models and process metrics, each graded in the
    [capability matrix](../capability-matrix.md). Use the toys to get a feel for
    scaling, and the engine for anything you want to rely on.

## Resolution and depth of focus

Pick a wavelength preset (it also sets an example NA), then vary NA and the process
factors. The table compares every preset at your current k₁ and k₂.

<div class="huv-calc" data-huv-calc="rayleigh">
<noscript>This calculator needs JavaScript. The formulas below give the same numbers by hand.</noscript>
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

!!! note "Verify"
    ArF immersion at NA 1.35 and k₁ = 0.27 gives R = 0.27 × 193 / 1.35 = 38.6 nm, close
    to the 38 nm that ASML quotes for its NA 1.35 scanners with dipole illumination. EUV at
    NA 0.33 and k₁ = 0.32 gives 13.1 nm (ASML quotes 13 nm), and High-NA EUV at NA 0.55
    gives 7.9 nm (quoted: 8 nm). For DOF with k₂ = 1 at NA 0.33: 13.5 / 0.33² = 124 nm
    paraxial and 13.5 / (2 × (1 − cos 19.27°)) = 120 nm exact. Lin reports that the
    paraxial formula overestimates the depth of focus by 10 % at NA 0.6 and 20 % at NA 0.8;
    the exact/paraxial ratio here is 2(1 − cos θ)/NA² = 0.90 and 0.80 at those apertures.
    The example NAs for 436, 365, 157 and 6.7 nm are illustrative round numbers, not a
    particular tool's specification.

## Photon shot noise

A dose is a count of photons. Shorter wavelengths carry more energy per photon, so the
same dose delivers fewer of them, and the Poisson fluctuation in a small area grows.

<div class="huv-calc" data-huv-calc="shotnoise">
<noscript>This calculator needs JavaScript. The formulas below give the same numbers by hand.</noscript>
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

!!! note "Verify"
    At 13.5 nm a photon carries 1239.84193 / 13.5 = 91.84 eV = 1.471 × 10⁻¹⁷ J, so
    30 mJ/cm² delivers 3 × 10⁻¹⁶ / 1.471 × 10⁻¹⁷ = 20.4 photons/nm². A 20 nm × 20 nm
    square then receives about 8,160 photons, a 1σ fluctuation of 1.1 %. At 193 nm
    (6.42 eV) the same dose is 291 photons/nm², 14.3 times more.

The simulator's stochastic module goes further: Poisson photon counts per pixel ✅ (with the
same 0.68 photons/nm² per mJ/cm² at 13.5 nm used here) and shot-to-shot dose jitter ✅, feeding
line-edge and line-width roughness estimates 🔶; see [research modules](../research-modules.md)
and the stochastics rows of the [capability matrix](../capability-matrix.md).

## Partially coherent aerial image (1-D toy)

A line/space mask diffracts light into orders at spatial frequencies m/p. The projection
lens passes only the orders inside its pupil, |f| ≤ NA/λ, and the image is what those
orders rebuild. Each point of the illumination source tilts the orders by s·NA/λ, and
partially coherent imaging adds up the intensity images of all source points, which is
Abbe's method.

<div class="huv-calc" data-huv-calc="aerial">
<noscript>This calculator needs JavaScript. The formulas below describe what it computes.</noscript>
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
printed line width where the image stays below a constant resist threshold.

!!! note "Verify"
    - A clear mask (w → 0) gives I = 1 everywhere, and a pitch below
      λ / ((1 + σ<sub>out</sub>)·NA) lets no first order through for any source point, so
      the image is flat and the contrast is zero.
    - With σ = 0 and only orders −1, 0 and +1 inside the pupil, the image is
      I(x) = [c₀ + 2c₁ cos(2πx/p)]², a single cosine fringe.
    - For a pitch between λ/(2NA) and 3λ/(2NA), set σ<sub>in</sub> = σ<sub>out</sub> =
      λ/(2p·NA) (two-beam dipole illumination): each pole then passes just two orders,
      placed symmetrically about the pupil centre, their defocus phases are equal, and
      the image no longer changes with focus.

What the toy leaves out, all of which matter in practice: the source is a line through the
pupil rather than a disk or a freeform shape; the light is scalar (no polarization, so no
high-NA TM contrast loss); the mask is infinitely thin (no mask 3-D effects); there are no
aberrations besides defocus, no flare, no thin-film stack and no resist blur. The Rust engine
uses 2-D pupil fills ✅, adds a vector TE/TM mode for high NA ✅
([vector imaging](../vector-imaging.md)), Zernike aberrations ✅, a uniform flare term 🔶 and
thin-film stacks ✅; it shares the toy's thin-mask assumption. The
[pipeline](../pipeline.md) page states each of these stage by stage.

!!! example "Try it in highuvlith"
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

    ??? success "Output"

        ```text
        focus     0 nm: contrast 0.650, NILS 2.04
        focus   100 nm: contrast 0.417, NILS 1.27
        focus   200 nm: contrast 0.069, NILS 0.10
        ```

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
