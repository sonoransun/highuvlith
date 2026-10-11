---
hide:
  - navigation
description: "highuvlith is a lithography simulator that tells you exactly what it computes, from 436 nm to 13.5 nm to X-rays, and a guided tour of how chips have been printed; every capability is graded in one capability matrix."
---

# highuvlith

<p class="huv-lead">A lithography simulator that tells you exactly what it computes — 436 nm to 13.5 nm to X-rays — and a guided tour of how chips have been printed.</p>

<div class="huv-actions" markdown>

[Get started](getting-started.md) · [What does it compute?](capability-matrix.md) · [Start the tour](history/index.md)

</div>

<figure markdown="span">

<picture><source media="(max-width: 600px)" srcset="assets/images/diagrams/spectrum-strip-hero-narrow-light.svg"><img class="huv-no-zoom" src="assets/images/diagrams/spectrum-strip-hero-anim-light.svg#gh-light-mode-only" alt="A horizontal log-wavelength strip from 500 to 0.05 nm coloured by wavelength, with the exposure lines as ticks, the optic each band needs beneath it and the exposure eras pinned at their wavelength with the year their first tool model appears in the data set."></picture>
<picture><source media="(max-width: 600px)" srcset="assets/images/diagrams/spectrum-strip-hero-narrow-dark.svg"><img class="huv-no-zoom" src="assets/images/diagrams/spectrum-strip-hero-anim-dark.svg#gh-dark-mode-only" alt="A horizontal log-wavelength strip from 500 to 0.05 nm coloured by wavelength, with the exposure lines as ticks, the optic each band needs beneath it and the exposure eras pinned at their wavelength with the year their first tool model appears in the data set."></picture>

<figcaption><strong>One axis of light.</strong> Every exposure wavelength lithography has used, the optic each one needed and the year its first tool model appears in the data set (86 tools; years are first tool models, not first use; the 157 nm lens is dashed because no production tool shipped; the log axis is expanded above 100 nm, where the break is marked). <em>Data chart from litho_tools.csv; no model badge.</em></figcaption>
</figure>

highuvlith is a Rust physics engine with Python, command-line, desktop-GUI and notebook front ends.
It images a thin mask through dry, water-immersion and EUV projection optics with exact scalar
Hopkins (TCC/SOCS) imaging and a vector TE/TM mode, lit by any of fourteen light-source families,
and carries the image into thin films, resist exposure, bake and 3-D development, deep-layer
processes such as LIGA and Talbot lithography, and mask optimization. Every one of those
capabilities carries one of four grades:

<!-- huv-legend:start -->
<!-- huv-table: stack -->
| ✅ Implemented · 44 | 🔶 Simplified · 23 | 🧪 Theoretical · 7 | 🗺️ Planned · 1 |
|---|---|---|---|
| Computed by code, covered by tests, physics validated against analytical results | Runs end-to-end but with documented approximations or reduced dimensionality; the assumption is stated inline wherever the capability is described | Parameterized and runnable, but models speculative physics; outputs are research projections, not validated engineering | Not in code; roadmap only. Never described in prose as if it runs |
<!-- huv-legend:end -->

Every capability is graded in the [capability matrix](capability-matrix.md#status-taxonomy); no
page claims more than it.

## Five generations, one pattern

Forty years of lithography reduce to one scaling law: the smallest half-pitch a projection tool
prints is k₁·λ/NA. Hold k₁ fixed and let the published wavelength and numerical aperture of each
generation set the pitch, and the engine draws five images that look alike while the pitch bar
shrinks 12.6× — from a 217 nm pitch under a KrF laser to 17.2 nm under High-NA EUV.

<figure markdown="span">

![Five computed aerial images of equal lines and spaces that look alike: KrF 248 nm at NA 0.80 printing a 217 nm pitch, ArF 193 nm at NA 0.93 (145 nm), ArF immersion NA 1.35 (100 nm), EUV 13.5 nm at NA 0.33 (28.6 nm) and High-NA EUV at NA 0.55 (17.2 nm), all at k1 = 0.35 with contrast about 0.9.](assets/images/sim/home-generations-light.png#gh-light-mode-only)
![Five computed aerial images of equal lines and spaces that look alike: KrF 248 nm at NA 0.80 printing a 217 nm pitch, ArF 193 nm at NA 0.93 (145 nm), ArF immersion NA 1.35 (100 nm), EUV 13.5 nm at NA 0.33 (28.6 nm) and High-NA EUV at NA 0.55 (17.2 nm), all at k1 = 0.35 with contrast about 0.9.](assets/images/sim/home-generations-dark.png#gh-dark-mode-only)

<figcaption><strong>Five generations printing the same pattern at k<sub>1</sub> = 0.35.</strong> Each tile is a computed aerial image of equal lines and spaces under an x-dipole (σ<sub>c</sub> ≈ 0.71) at the half-pitch the published λ and NA allow — published wavelengths and numerical apertures, not models of the tools; the pitch bar shrinks 12.6× while the image barely changes. Thin mask, no flare, no resist. <em>Model: Hopkins imaging ✅ · illumination pupil fills ✅ · refractive and immersion optics ✅ · EUV projection optics 🔶 (isotropic pupil, assumed 0.2·NA obscuration at NA 0.55, no mask 3-D).</em></figcaption>
</figure>

## Choose your path

<div class="grid cards huv-cards" markdown>

-   **Learn**

    ---

    <span class="huv-swatch huv-swatch--history"></span>How chips have been printed, from the printing stone to High-NA EUV, with sourced tool data and the physics of every step.

    [Start the tour →](history/index.md)

-   **Try**

    ---

    <span class="huv-swatch huv-swatch--euv"></span>Three closed-form calculators in your browser: resolution and depth of focus, photon statistics, a 1-D aerial toy.

    [Open the playground →](playground/index.md)

-   **Run**

    ---

    <span class="huv-swatch huv-swatch--f2"></span>The engine, stage by stage, with the exactness of every stage stated; install it and run a first image.

    [Simulator →](simulator/index.md)

-   **Check**

    ---

    <span class="huv-swatch huv-swatch--xray"></span>Every capability graded in one table; nothing on the site claims more than it.

    [Capability matrix →](capability-matrix.md)

</div>

## Explore lithography

<div class="grid cards huv-cards" markdown>

-   **History** · 1796–2026

    ---

    <span class="huv-swatch huv-swatch--history"></span>From a Munich printing stone to High-NA EUV scanners: contact printing, projection steppers, excimer lasers, the 157 nm detour, immersion, multiple patterning and the road to EUV, told with sourced tool data.

    [Read the history →](history/index.md) · [The road to EUV →](history/euv.md)

-   **Process nodes** · 2003–2030s

    ---

    <span class="huv-swatch huv-swatch--nodes"></span>From the 90 nm generation to the Ångström era: what changed in the transistor, which layers became hard to print, how they were printed anyway, and why node names are labels.

    [Tour the nodes →](nodes/index.md) · [Lithography arithmetic →](nodes/litho-math.md)

-   **The future** · 2026 and beyond

    ---

    <span class="huv-swatch huv-swatch--future"></span>High-NA and Hyper-NA EUV, beyond-EUV wavelengths, accelerator light sources, the stochastic frontier, alternatives to projection and quantum schemes, each rated for readiness.

    [Look ahead →](future/index.md) · [High-NA and Hyper-NA →](future/high-na-and-hyper-na.md)

</div>

## Use the simulator

### Run it

<div class="grid cards huv-cards" markdown>

-   **Getting started**

    ---

    Build the Python package from source and run the canonical 65 nm / 180 nm image four ways.

    [Getting started →](getting-started.md)

-   **CLI**

    ---

    Seven subcommands driven by TOML files, with strict keys and every example config tested.

    [CLI →](cli.md) · [Configuration →](configuration.md)

-   **GUI**

    ---

    An egui desktop app with every source family, live aerial images, process windows and a volume viewer.

    [GUI →](gui.md)

-   **Python API**

    ---

    Every class and function of the `highuvlith` package, with verified examples.

    [Python API →](python-api.md)

-   **Notebooks**

    ---

    Six Jupyter notebooks, from a quick-start aerial image to process and optimization, executed in CI.

    [Notebooks →](../examples/notebooks/01_quickstart_aerial.ipynb)

</div>

### Understand it

<div class="grid cards huv-cards" markdown>

-   **Pipeline**

    ---

    Source → optics → mask → Hopkins imaging → thin film → resist → metrics, stage by stage with every approximation stated.

    [Simulation pipeline →](pipeline.md) · [Masks →](masks.md)

-   **Sources**

    ---

    Fourteen families, from mercury lamps and excimer lines to plasma EUV, accelerators and speculative sources, behind one source trait.

    [Sources →](sources/index.md)

-   **Processes**

    ---

    Thin films, resist chains, 3-D development, LIGA deep X-ray, grayscale, interference and Talbot lithography.

    [Processes →](processes/index.md)

-   **Capability matrix**

    ---

    The single source of truth for every grade on the site, with the audit log of what changed.

    [Capability matrix →](capability-matrix.md) · [Reference →](reference/index.md)

</div>

## How resolution scales

The smallest half-pitch a projection scanner can print, and the focus range it can hold,
follow the Rayleigh scaling laws

```math
R = k_1 \frac{\lambda}{\mathrm{NA}}, \qquad \mathrm{DOF} = k_2 \frac{\lambda}{\mathrm{NA}^2}
```

where λ is the exposure wavelength, NA the numerical aperture of the projection lens,
and k₁, k₂ process factors. There are three levers: a shorter wavelength, a larger NA,
and a smaller k₁, which resolution-enhancement techniques push down towards its
single-exposure floor of k₁ = 0.25 for dense lines. The floor is geometry: a grating's
diffraction orders sit at m/(2k₁) in pupil units, and an image needs at least two of them
inside the lens.

<figure markdown="span">

![Animated pupil diagram: the orders of a 1:1 grating slide outward as k1 falls from 1.0 to 0.28 under a sigma 0.7 fill and the image contrast beneath drops to zero at k1 = 0.29; a matched dipole then keeps two-beam contrast down to the 0.25 limit, where pole and order sit on opposite rims.](assets/images/diagrams/pupil-orders-anim-light.svg#gh-light-mode-only)
![Animated pupil diagram: the orders of a 1:1 grating slide outward as k1 falls from 1.0 to 0.28 under a sigma 0.7 fill and the image contrast beneath drops to zero at k1 = 0.29; a matched dipole then keeps two-beam contrast down to the 0.25 limit, where pole and order sit on opposite rims.](assets/images/diagrams/pupil-orders-anim-dark.svg#gh-dark-mode-only)

<figcaption><strong>Where the k₁ limits come from.</strong> Orders of a 1:1 grating sit at m/(2k₁) in pupil units; a conventional fill of radius σ keeps imaging until the ±1 orders leave the support circle at k₁ = 1/(2(1+σ)); a dipole at σ_c = λ/(2pNA) keeps two-beam imaging down to k₁ = 0.25, where pole and order sit on opposite rims. The contrast strip is the closed-form 1-D toy; the support circle is the engine's (1+σ)·NA/λ rule. <em>Closed form and geometry; not the engine. No model badge.</em></figcaption>
</figure>

Each lever has its price — the depth of focus shrinks with NA², which is one reason every new
generation is harder than the last. The scanners that carried each generation sit at nearly
the same k₁:

<!-- huv-table: stack -->
| Scanner class (example) | λ (nm) | NA | λ / NA (nm) | Quoted resolution (nm) | Implied k₁ |
|---|---:|---:|---:|---:|---:|
| [KrF dry](history/excimer-duv.md) (ASML NXT:870) | 248 | 0.80 | 310 | 110 | 0.35 |
| [ArF dry](history/excimer-duv.md) (ASML NXT:1470) | 193 | 0.93 | 208 | 57 | 0.27 |
| [ArF immersion](history/immersion-multipatterning.md) (ASML NXT:2000i) | 193 | 1.35 | 143 | 38 | 0.27 |
| [EUV](history/euv.md) (ASML NXE) | 13.5 | 0.33 | 41 | 13 | 0.32 |
| [High-NA EUV](future/high-na-and-hyper-na.md) (ASML EXE) | 13.5 | 0.55 | 25 | 8 | 0.33 |

NA and resolution are the manufacturer's published figures ([references](#references)); the
implied k₁ = R · NA / λ treats the quoted resolution as a half-pitch, as
[lithography arithmetic](nodes/litho-math.md) explains. Recompute any row, or try your own
numbers, in the [Rayleigh calculator](playground/index.md?calc=rayleigh).

The simulator's optics presets show the same two NA steps. Its water-immersion lens (NA 1.35,
n = 1.437) images a 90 nm pitch at 193 nm with annular illumination (σ 0.7–0.9) at contrast
0.368, where a dry NA 0.93 lens gives none; its EUV projection optics resolve an 18 nm pitch at
NA 0.55 (contrast 0.495) but not at NA 0.33. Both results are recorded in the
[capability matrix](capability-matrix.md#optical-systems), and the two charts below redo each
comparison across pitch with conventional σ 0.9 illumination — a different illumination from the
annular case the matrix quotes, which is why the immersion curve at 90 nm pitch does not read
0.368. The lens models are described under [optical systems](optics.md).

<div class="huv-compare" markdown>

<figure markdown="span">

![Contrast versus half-pitch for a dry 157 nm lens at NA 0.85, a dry 193 nm lens at NA 0.93 and 193 nm water immersion at NA 1.35: immersion keeps imaging down to about 38 nm half-pitch, the dry 157 nm lens only to about 49 nm and dry 193 nm to about 55 nm.](assets/images/sim/site/site-why-immersion-light.png#gh-light-mode-only)
![Contrast versus half-pitch for a dry 157 nm lens at NA 0.85, a dry 193 nm lens at NA 0.93 and 193 nm water immersion at NA 1.35: immersion keeps imaging down to about 38 nm half-pitch, the dry 157 nm lens only to about 49 nm and dry 193 nm to about 55 nm.](assets/images/sim/site/site-why-immersion-dark.png#gh-dark-mode-only)

<figcaption><strong>The comparison that ended the 157 nm programme.</strong> Contrast of 1:1 lines against half-pitch for a dry 157 nm lens at NA 0.85, a dry 193 nm lens at NA 0.93 and 193 nm water immersion at NA 1.35 — conventional σ 0.9, default 2 % flare, two-period commensurate fields, scalar model; dotted lines mark each tool's k<sub>1</sub> = 0.25 half-pitch (with σ 0.9 the image vanishes slightly above it, at λ/(2NA·1.9)) and the dashed line the 45 nm example. <em>Model: Hopkins imaging ✅ · immersion lithography ✅ (water n = 1.437; no polarization in this chart).</em></figcaption>
</figure>

<figure markdown="span">

![Contrast versus pitch from 12 to 40 nm for 0.33 and 0.55 NA EUV optics with a tin plasma source: the 0.33 NA curve reaches zero near 22 nm pitch, the 0.55 NA curve near 13 nm.](assets/images/sim/site/site-euv-nxe-vs-high-na-light.png#gh-light-mode-only)
![Contrast versus pitch from 12 to 40 nm for 0.33 and 0.55 NA EUV optics with a tin plasma source: the 0.33 NA curve reaches zero near 22 nm pitch, the 0.55 NA curve near 13 nm.](assets/images/sim/site/site-euv-nxe-vs-high-na-dark.png#gh-dark-mode-only)

<figcaption><strong>Resolution at NA 0.33 against NA 0.55.</strong> Contrast of 1:1 lines against pitch through the simulator's EUV presets (Sn LPP, conventional σ 0.9, two-period commensurate fields): isotropic pupil — the anamorphic 4×/8× magnification is not modelled — with a 0.2·NA central obscuration assumed for High-NA; thin mask (no mask-3D shadowing), no flare. <em>Model: Hopkins imaging ✅ · EUV projection optics 🔶 (obscuration assumed).</em></figcaption>
</figure>

</div>

**Where next:** [Getting started](getting-started.md) · [Capability matrix](capability-matrix.md) · [History](history/index.md) · [Contributing](https://github.com/sonoransun/highuvlith/blob/main/CONTRIBUTING.md)

## References

- ASML, [TWINSCAN NXT:870](https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt870) &mdash; KrF (248 nm), variable NA 0.55–0.80, resolution at and below 110 nm.
- ASML, [TWINSCAN NXT:1470](https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt1470) &mdash; ArF dry, variable NA 0.70–0.93, production resolution down to 57 nm.
- ASML, [TWINSCAN NXT:2000i](https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt2000i) &mdash; ArF immersion, NA 1.35, resolution down to 38 nm (dipole illumination).
- ASML, [EUV lithography systems](https://www.asml.com/en/products/euv-lithography-systems) &mdash; NXE systems NA 0.33 with 13 nm resolution; EXE (High-NA) systems NA 0.55 with 8 nm resolution.
