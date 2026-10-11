---
description: "Where lithography could go after today's EUV scanners: a readiness ladder for every candidate technology, the physical limits that frame them, and what the highuvlith simulator can and cannot say about each."
---

# The future of lithography

<p class="huv-lead">Chipmaking prints its finest layers with 13.5 nm light through 0.33 NA reflective optics, and the first 0.55 NA scanners are exposing production wafers. Six families compete for what comes next: more NA, a shorter wavelength, a brighter source, better resists, a different way of patterning, and physics still in the laboratory. Every chapter asks how ready each is, what limits it and what the simulator can say about it; this page is the map — six chapters, one readiness ladder, five limits.</p>

**Verdict (as of September 2026):** 0.55 NA EUV is in production on selected layers and tin sources run at 500–600 W with 1 kW demonstrated; everything beyond that — Hyper-NA, 6.x nm, accelerator sources, new patterning and quantum schemes — is a study, a design or a laboratory result.

<figure markdown="span">

<picture><source media="(max-width: 600px)" srcset="../assets/images/future/readiness-matrix-narrow-light.svg"><img class="huv-no-zoom" src="../assets/images/future/readiness-matrix-light.svg#gh-light-mode-only" alt="31 technology chips in seven readiness columns (In production to Speculative) by six family rows, each with the simulator's model glyph: 13 implemented, 12 simplified, 10 theoretical marks, 6 not modelled; Shipping and Roadmap hold no technology today."></picture>
<picture><source media="(max-width: 600px)" srcset="../assets/images/future/readiness-matrix-narrow-dark.svg"><img class="huv-no-zoom" src="../assets/images/future/readiness-matrix-dark.svg#gh-dark-mode-only" alt="31 technology chips in seven readiness columns (In production to Speculative) by six family rows, each with the simulator's model glyph: 13 implemented, 12 simplified, 10 theoretical marks, 6 not modelled; Shipping and Roadmap hold no technology today."></picture>

<figcaption><strong>Readiness of the technology (columns) against what the simulator models (chips).</strong> Rung words are defined on this page as of September 2026; chips are copied from the capability matrix, a second glyph is the grade of the part the technology uses, and ∅ means no code at all. A ✅ model of a source that does not exist is still a model of a source that does not exist. <em>Chips copied from the matrix.</em></figcaption>
</figure>

<div class="grid cards huv-cards" markdown>

-   **[High-NA and Hyper-NA EUV](high-na-and-hyper-na.md)**

    ---

    0.55 NA in production, 0.75 NA on the drawing board: anamorphic optics, half fields, depth of focus and polarization.

-   **[Beyond EUV: 6.x nm and X-rays](beyond-euv.md)**

    ---

    Gadolinium plasmas, lanthanum/boron mirrors, the 11.2 nm alternative, and the X-ray lithography that survives as LIGA.

-   **[Accelerator light sources](accelerator-light-sources.md)**

    ---

    Free-electron lasers, energy-recovery linacs, steady-state microbunching, inverse Compton and laser-plasma accelerators against the kilowatt tin source.

-   **[The stochastic frontier](stochastic-frontier.md)**

    ---

    Photons, electrons and molecules at EUV doses; the resolution–roughness–dose trade-off; failures, throughput and the new resists.

-   **[Alternatives to projection](alternative-patterning.md)**

    ---

    Nanoimprint, self-assembly, electron beams, selective deposition, interference and Talbot lithography, atom-by-atom patterning and 3D printing.

-   **[Quantum and exotic lithography](quantum-and-exotic.md)**

    ---

    Entangled-photon (NOON) lithography, the entangled two-photon absorption dispute, and free-electron and table-top sources.

</div>

<figure markdown="span">

<picture><source media="(max-width: 600px)" srcset="../assets/images/diagrams/spectrum-strip-ribbon-future-narrow-light.svg"><img class="huv-no-zoom" src="../assets/images/diagrams/spectrum-strip-ribbon-future-light.svg#gh-light-mode-only" alt="The spectrum strip from 500 to 0.05 nm with the EUV → X-ray span highlighted and labelled 'You are here'; the other lines, optics and era pills are faded."></picture>
<picture><source media="(max-width: 600px)" srcset="../assets/images/diagrams/spectrum-strip-ribbon-future-narrow-dark.svg"><img class="huv-no-zoom" src="../assets/images/diagrams/spectrum-strip-ribbon-future-dark.svg#gh-dark-mode-only" alt="The spectrum strip from 500 to 0.05 nm with the EUV → X-ray span highlighted and labelled 'You are here'; the other lines, optics and era pills are faded."></picture>

<figcaption><strong>You are here on the axis of light.</strong> The chapter's wavelengths and eras at full strength, the rest faded; years are first tool models in litho_tools.csv; the log axis is expanded above 100 nm. <em>Data chart from litho_tools.csv; no model badge.</em></figcaption>
</figure>

## Readiness ladder

Every technology in this section sits on one rung of a seven-rung ladder, with the simulator's grade beside it: that is the matrix under the verdict at the top of this page, and the table below is the same ladder as text. The rung is the most advanced *lithography-relevant* result we could verify as of September 2026; the chapters give the evidence and the sources. Rung words follow the [readiness ladder](#readiness-ladder) defined below; chips are the simulator's grades from the [capability matrix](../capability-matrix.md).

<details markdown="1">
<summary>The ladder as a table</summary>

<!-- readiness-table:start -->

| Technology | Readiness (Sep 2026) | highuvlith model | Page |
|---|---|---|---|
| 0.33 NA EUV with tin LPP sources (250–600 W) | **In production** | ✅ [LPP source](../sources/lpp.md), 🔶 [EUV projection optics](../optics.md) | [High-NA](high-na-and-hyper-na.md#readiness-at-a-glance), [accelerators](accelerator-light-sources.md#readiness-at-a-glance) |
| 0.55 NA High-NA EUV | **In production** (selected layers) | 🔶 isotropic EUV projection pupil, ✅ [vector pupil](../vector-imaging.md) | [High-NA](high-na-and-hyper-na.md#readiness-at-a-glance) |
| ArF immersion and multipatterning | **In production** | ✅ [immersion optics](../optics.md), 🔶 [multiple patterning](../research-modules.md) | [research modules](../research-modules.md) |
| LIGA deep X-ray lithography | **In production** (niche) | ✅ [LIGA](../processes/liga-deep-xray.md) (🔶 proximity diffraction) | [beyond EUV](beyond-euv.md#readiness-at-a-glance) |
| Dry metal-organic resist | **In production** (company-reported, DRAM) | not modelled | [stochastics](stochastic-frontier.md#readiness-at-a-glance) |
| Spin-on metal-oxide resists | **In production** (IRDS 2024: "started to be used in production") | 🔶 generic resist parameters ([resist models](../processes/resist-models.md)) | [stochastics](stochastic-frontier.md#readiness-at-a-glance) |
| Multi-beam e-beam mask writing | **In production** | not modelled | [alternatives](alternative-patterning.md#readiness-at-a-glance) |
| Displacement Talbot lithography | **In production** (photonics, per the vendor) | 🔶 [Talbot](../processes/talbot.md) | [alternatives](alternative-patterning.md#readiness-at-a-glance) |
| Two-photon polymerization | **In production** (micro-3D printers) | ✅ [two-photon voxels](../processes/interference-volumetric.md) (🔶 absorption, interfaces) | [alternatives](alternative-patterning.md#readiness-at-a-glance) |
| 6 × 12-inch masks for full-field High-NA | **Announced** | not modelled | [High-NA](high-na-and-hyper-na.md#readiness-at-a-glance) |
| 11.2 nm lithography (Xe plasma, Ru/Be mirrors) | **Announced** (national roadmap) | not modelled | [beyond EUV](beyond-euv.md#readiness-at-a-glance) |
| Tin LPP beyond 1 kW | **Announced** (1 kW shown 2025; 1.5–2 kW roadmap) | ✅ [LPP power chain](../sources/lpp.md) | [accelerators](accelerator-light-sources.md#readiness-at-a-glance) |
| xLight fab-scale FEL | **Announced** | 🧪 ERL preset as proxy ([XFEL](../sources/xfel.md)) | [accelerators](accelerator-light-sources.md#readiness-at-a-glance) |
| Nanoimprint for chips | **Announced** (NAND fab evaluation since 2017; volume production not confirmed) | not modelled | [alternatives](alternative-patterning.md#readiness-at-a-glance) |
| Gd/Tb plasma sources at 6.5–6.7 nm | **Demonstrated** | ✅ [LPP](../sources/lpp.md) (🔶 CE defaults, étendue; 🧪 Gd/Tb power) | [beyond EUV](beyond-euv.md#readiness-at-a-glance) |
| La/B multilayer mirrors | **Demonstrated** | ✅ [multilayer model](../materials.md) | [beyond EUV](beyond-euv.md#readiness-at-a-glance) |
| Laser-plasma-accelerator FEL | **Demonstrated** | 🔶 [LPA-FEL](../sources/lpa-fel.md) | [accelerators](accelerator-light-sources.md#readiness-at-a-glance) |
| Steady-state microbunching (mechanism) | **Demonstrated** (1064 nm, one turn) | 🧪 [SSMB](../sources/ssmb.md) | [accelerators](accelerator-light-sources.md#readiness-at-a-glance) |
| EUV interference and Talbot lithography | **Demonstrated** | ✅ [interference](../processes/interference-volumetric.md) (🔶 absorption, interfaces), 🔶 [Talbot](../processes/talbot.md) | [alternatives](alternative-patterning.md#readiness-at-a-glance) |
| Directed self-assembly | **Demonstrated** (pilot lines) | 🔶 [DSA](../research-modules.md) | [alternatives](alternative-patterning.md#readiness-at-a-glance) |
| Area-selective deposition, STM lithography, volumetric printing | **Demonstrated** | not modelled | [alternatives](alternative-patterning.md#readiness-at-a-glance) |
| HHG and soft-X-ray lasers at EUV wavelengths | **Demonstrated** | ✅ [HHG](../sources/hhg.md) (🔶 conversion efficiency), ✅ [soft-X-ray laser](../sources/soft-xray-laser.md) (🔶 pulse, linewidth, coherence) | [quantum and exotic](quantum-and-exotic.md#readiness-at-a-glance) |
| Smith–Purcell and van der Waals X-ray emitters | **Demonstrated** (not at useful EUV power) | 🧪 [Smith–Purcell](../sources/smith-purcell.md) | [quantum and exotic](quantum-and-exotic.md#readiness-at-a-glance) |
| Hyper-NA (NA ≈ 0.75) | **Proposed** (feasibility studies) | 🔶 ideal pupil, ✅ [vector pupil](../vector-imaging.md) | [High-NA](high-na-and-hyper-na.md#readiness-at-a-glance) |
| Kilowatt 6.x nm sources (LPP, FEL) | **Proposed** (design study) | ✅ [LPP power chain](../sources/lpp.md) (🔶 CE defaults, étendue; 🧪 Gd/Tb power) | [beyond EUV](beyond-euv.md#readiness-at-a-glance) |
| Kilowatt linac and ERL FELs | **Proposed** (design study) | 🧪 `xfel_cw_sc_13nm5`, `xfel_erl_13nm5` presets ([XFEL](../sources/xfel.md)) | [accelerators](accelerator-light-sources.md#readiness-at-a-glance) |
| Kilowatt SSMB-EUV ring | **Proposed** (design study) | 🧪 derived coherent power ([SSMB](../sources/ssmb.md)) | [accelerators](accelerator-light-sources.md#readiness-at-a-glance) |
| 6.x nm (BEUV) projection scanner | **Speculative** | ✅ imaging at 6.7 nm, ideal pupil ([pipeline](../pipeline.md)) | [beyond EUV](beyond-euv.md#readiness-at-a-glance) |
| Inverse Compton source at 13.5 nm | **Speculative** | 🧪 [ICS](../sources/inverse-compton.md) | [accelerators](accelerator-light-sources.md#readiness-at-a-glance) |
| NOON-state quantum lithography | **Speculative** | 🧪 [quantum module](../research-modules.md) | [quantum and exotic](quantum-and-exotic.md#readiness-at-a-glance) |
| Entangled two-photon absorption as exposure | **Speculative** (disputed cross-section) | 🧪 [entangled source](../sources/entangled-photon.md) | [quantum and exotic](quantum-and-exotic.md#readiness-at-a-glance) |

<!-- readiness-table:end -->

</details>

<details markdown="1">
<summary>The seven rungs, defined</summary>

The two scales measure different things. **Readiness** describes the *technology*; the **grades** describe the *simulator's model* and come from the [capability matrix](../capability-matrix.md): ✅ Implemented (computed and validated), 🔶 Simplified (runs, with stated approximations), 🧪 Theoretical (runnable, but it models speculative physics), 🗺️ Planned (not in code). "Not modelled" means there is no code at all. A ✅ model of a source that does not exist is still a model of a source that does not exist.

| Rung | Definition | Label used before October 2026 |
|---|---|---|
| **In production** | running in volume manufacturing, per the company or credible reporting; "(selected layers)", "(niche)", "(reported)" as qualifiers | In production |
| **Shipping** | a production tool delivered to or accepted by at least one customer, per the vendor or the customer | folded into "In production (sold as production tools)" |
| **Announced** | a dated company plan or a national programme with published plans; plans slip | Announced / in development |
| **Demonstrated** | the key physics or result shown experimentally, not at production scale; "(pilot lines)", "(1064 nm, one turn)" as qualifiers | Demonstrated in the lab |
| **Roadmap** | an industry projection (IRDS, imec), not a commitment by anyone | cited as evidence inside other rungs |
| **Proposed** | worked out on paper as a design or feasibility study, not built | Proposed (design study) |
| **Speculative** | physically possible in principle; under consideration or in research; no design, no product plan | Theoretical / speculative |

A row keeps its old evidence as the qualifier after the rung word, so no claim changed meaning when the labels were unified; "Theoretical" is no longer a readiness word — it is the 🧪 model grade. The [Ångström era](../nodes/angstrom-era.md) page uses the same seven words.

</details>

## The limits that frame everything

Almost every argument in this section comes back to five constraints. They are worth having in one place, and every chapter links back to them.

<figure markdown="span">

<picture><source media="(max-width: 600px)" srcset="../assets/images/future/five-limits-narrow-light.svg"><img class="huv-no-zoom" src="../assets/images/future/five-limits-light.svg#gh-light-mode-only" alt="Five tiles, one per constraint: diffraction HP = k1 lambda over NA with k1 at least 0.25; photons per feature proportional to lambda cubed over NA squared; the resist trade-off Z = HP cubed times LER squared times dose; electron blur of about 2 nm; throughput t = t0 plus dose times area over power."></picture>
<picture><source media="(max-width: 600px)" srcset="../assets/images/future/five-limits-narrow-dark.svg"><img class="huv-no-zoom" src="../assets/images/future/five-limits-dark.svg#gh-dark-mode-only" alt="Five tiles, one per constraint: diffraction HP = k1 lambda over NA with k1 at least 0.25; photons per feature proportional to lambda cubed over NA squared; the resist trade-off Z = HP cubed times LER squared times dose; electron blur of about 2 nm; throughput t = t0 plus dose times area over power."></picture>

<figcaption><strong>Five constraints frame every page of this section.</strong> Diffraction (k₁ ≥ 0.25), photons per feature ∝ λ³/NA², the resolution–roughness–dose trade-off Z = HP³·LER²·D, an electron blur of about 2 nm that does not shrink with λ, and a throughput t = t₀ + D·A/P that dose buys. <em>Equations only; no model badge.</em></figcaption>
</figure>

**Five limits:** [k₁ ≥ 0.25](#diffraction-and-the-resolution-floor) · [photons ∝ λ³/NA²](#photons-and-shot-noise) · [Z-factor](#resolution-roughness-and-dose) · [electron blur](#electron-blur) · [throughput](#throughput-and-cost)

### Diffraction and the resolution floor

A lens can only form an image from the diffraction orders it collects. For a grating of
pitch $p$ the first orders leave at $\sin\theta = \lambda/p$, and an image needs at least
two orders inside the pupil. With the orders at opposite edges of a pupil of numerical
aperture NA, the smallest pitch is $\lambda/(2 \mathrm{NA})$. Written as the usual
resolution equation [1, 2, 3]:

```math
\mathrm{HP} = k_1\,\frac{\lambda}{\mathrm{NA}}, \qquad k_1 \ge 0.25 \ \text{(single exposure, dense lines)},
\qquad \mathrm{DOF} = k_2\,\frac{\lambda}{\mathrm{NA}^2}.
```

Production lives above the floor. ASML's resolution specifications imply $k_1 \approx 0.27$
for ArF immersion and $k_1 \approx 0.32$ for both EUV generations [4, 5, 6]:

<!-- huv-table: stack -->
| System | λ (nm) | NA | Half-pitch at k₁ = 0.25 | Vendor half-pitch spec | Implied k₁ |
|---|---|---|---|---|---|
| ArF immersion | 193 | 1.35 | 35.7 nm | 38 nm (dipole) [4] | 0.266 |
| EUV | 13.5 | 0.33 | 10.2 nm | 13 nm [5] | 0.318 |
| High-NA EUV | 13.5 | 0.55 | 6.1 nm | 8 nm [6] | 0.326 |
| Hyper-NA (study) | 13.5 | 0.75 | 4.5 nm | none | — |
| BEUV (speculative) | 6.7 | 0.55 | 3.0 nm | none | — |

Below the floor, the only ways forward are more exposures per layer (multipatterning), a
shorter wavelength, or a larger NA, and each has a price. The
[playground](../playground/index.md) calculates these relations interactively, and
[lithography arithmetic](../nodes/litho-math.md) works them for every node.

### Photons and shot noise

A dose $D$ delivers $n = D\lambda/(hc)$ photons per unit area: 9.72 per nm² per mJ/cm² at
193 nm, 0.680 at 13.5 nm and 0.338 at 6.7 nm (CODATA constants). Photon arrival is a
Poisson process, so a feature that collects $N$ photons has a relative dose noise of
$1/\sqrt N$. At fixed $k_1$ and dose, the photons per feature scale as $\lambda^3/\mathrm{NA}^2$:
every step to a shorter wavelength or a higher NA takes photons away faster than it takes
nanometres.

The chart of incident photons per feature against half-pitch, with the vendor specifications marked, is on the [stochastic frontier](stochastic-frontier.md#counting-photons) page.

### Resolution, roughness and dose

Resists add their own randomness: secondary electrons, acid and quencher molecules.
Gallatin showed that a conventional chemically amplified resist cannot deliver high
resolution, low line-edge roughness (LER) and low dose at the same time [7]. Wallow and
co-workers condensed the trade-off into a figure of merit, $Z = \mathrm{HP}^3 \cdot
\mathrm{LER}^2 \cdot D$ [8]. At constant $Z$, halving the roughness costs four times the
dose. The [stochastic frontier](stochastic-frontier.md) page follows this into failure
rates and resist chemistry.

### Electron blur

A 92 eV EUV photon does not trigger chemistry directly. It ionizes the resist, and the
photoelectron and its secondary electrons deposit the energy over a few nanometres.
Kozawa and Tagawa describe EUV imaging as radiation chemistry rather than photochemistry
[9], and measured and simulated acid-generation blur is of order 2 nm [10]. That blur is set
by electron transport, not by the wavelength, so it becomes a larger fraction of each new,
smaller feature.

### Throughput and cost

A scanner's productivity is roughly a fixed overhead plus a dose term,
$t_\text{wafer} \approx t_0 + D A / P_\text{wafer}$. ASML's NXE:3400C specification, at
least 170 wafers per hour at 20 mJ/cm² and 135 at 30 mJ/cm² [5], shows the exchange rate:
50 % more dose costs about 21 % of the output. Cost per wafer layer is the tool's cost of
ownership divided by the wafers it exposes, so dose, source power and the number of
exposures per layer (multipatterning) all feed straight into it. Source power is the lever
that buys dose without losing throughput, which is why the
[source roadmap](accelerator-light-sources.md) matters as much as the optics.

**Node names stopped being dimensions long ago; an 8 nm High-NA half-pitch is still about fifteen silicon lattice constants wide.**

!!! note "The atomic floor"
    Silicon's lattice constant is 0.5431 nm [11]. An 8 nm High-NA half-pitch is about
    15 lattice constants wide. The 2024 IRDS More Moore roadmap reaches a 14 nm metal
    pitch (7 nm half-pitch) around 2035 while the node *names* continue into "A3.5" and
    below [12]. Node names stopped being dimensions long ago (see
    [what a node is](../nodes/index.md)); the physical pitches are shrinking slowly and are
    still tens of atoms wide.

## What the simulator can and cannot tell you

highuvlith is a physics simulator, not a technology forecast. Used on the future, it is good at some questions and silent on others; every grade below is copied from the capability matrix.

| It can | Grade |
|---|---|
| image any wavelength through an ideal pupil at any NA below the medium index, with exact scalar or vector (polarized) imaging and exact defocus | ✅ Implemented |
| run EUV projection pupils with a central obscuration | 🔶 Simplified |
| compute ideal multilayer mirrors | ✅ Implemented |
| count photons with Poisson shot noise and source dose jitter | ✅ Implemented |
| derive source physics from machine parameters: FEL gain, SSMB coherent power, inverse-Compton yield, plasma power chains | ✅ Implemented (🔶 LPA-FEL; 🧪 SSMB, ICS, ERL presets) |
| turn source power into dose-limited throughput | 🔶 Simplified |
| model interference and two-photon exposure | ✅ Implemented (🔶 absorption, interfaces) |
| model Talbot, DTL and ATL exposure | 🔶 Simplified |
| compute N-photon sharpening and the ideal NOON (λ/N) limit | 🧪 Theoretical |

**It cannot** simulate thick-mask (mask 3D) electromagnetics, anamorphic magnification, secondary-electron transport, molecule-level resist stochastics or failure probabilities, plasma or accelerator dynamics, or cost. Where a page leans on those, it says so and cites the literature instead.

## Key takeaways

- **The verdict, as of September 2026:** 0.55 NA EUV is in production on selected
  layers, tin sources run at 500–600 W in production (500 W in ASML's NXE:3800E
  specification; 600 W per ASML's February 2026 statement to Reuters), and 1 kW has been
  shown. Everything beyond that is a study, a design or a lab result.
- The next steps compete on physics that is easy to state: $k_1 \ge 0.25$ for a single
  exposure, depth of focus $\propto \lambda/\mathrm{NA}^2$, photons per feature
  $\propto \lambda^3/\mathrm{NA}^2$ at fixed dose, and a few-nanometre electron blur that
  does not shrink.
- Higher NA, shorter wavelengths and brighter sources each attack one constraint and
  worsen another. Alternatives to projection trade away generality, overlay or
  throughput. Quantum schemes founder on photon flux.
- highuvlith lets you compute the optics, the photon statistics and the source physics
  honestly, with badges that say how much to trust each number, and says plainly where
  it has no model.

**Where next:** [High-NA and Hyper-NA EUV](high-na-and-hyper-na.md) → · Related: [History](../history/index.md) · [Process Nodes](../nodes/index.md) · [capability matrix](../capability-matrix.md) · [roadmap](../roadmap.md)

## References and further reading

1. E. Abbe, "Beiträge zur Theorie des Mikroskops und der mikroskopischen Wahrnehmung,"
   *Archiv für mikroskopische Anatomie* **9**, 413–468 (1873),
   [doi:10.1007/BF02956173](https://doi.org/10.1007/BF02956173).
2. Lord Rayleigh, "Investigations in optics, with special reference to the spectroscope,"
   *Philosophical Magazine* **8**(49), 261–274 (1879),
   [doi:10.1080/14786447908639684](https://doi.org/10.1080/14786447908639684).
3. C. A. Mack, *Fundamental Principles of Optical Lithography: The Science of
   Microfabrication* (Wiley, 2007).
4. ASML, TWINSCAN NXT:2000i product page ("production resolutions down to 40 nm (C-quad)
   and 38 nm (dipole)"),
   [asml.com](https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt2000i)
   (retrieved 30 Sep 2026).
5. ASML, TWINSCAN NXE:3400C product page (13 nm resolution; ≥ 170 wafers per hour at
   20 mJ/cm², ≥ 135 at 30 mJ/cm²),
   [asml.com](https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe3400c)
   (retrieved 30 Sep 2026).
6. ASML, TWINSCAN EXE:5000 product page (0.55 NA, 8 nm resolution),
   [asml.com](https://www.asml.com/en/products/euv-lithography-systems/twinscan-exe-5000)
   (retrieved 30 Sep 2026).
7. G. M. Gallatin, "Resist blur and line edge roughness," *Proc. SPIE* **5754**, Optical
   Microlithography XVIII (2005),
   [doi:10.1117/12.607233](https://doi.org/10.1117/12.607233).
8. T. Wallow, C. Higgins, R. Brainard, K. Petrillo, W. Montgomery, C.-S. Koay *et al.*,
   "Evaluation of EUV resist materials for use at the 32 nm half-pitch node," *Proc. SPIE*
   **6921**, 69211F (2008), [doi:10.1117/12.772943](https://doi.org/10.1117/12.772943).
9. T. Kozawa, S. Tagawa, "Radiation chemistry in chemically amplified resists," *Jpn. J.
   Appl. Phys.* **49**, 030001 (2010),
   [doi:10.1143/JJAP.49.030001](https://doi.org/10.1143/JJAP.49.030001).
10. S. Bhattarai, A. R. Neureuther, P. P. Naulleau, "Study of shot noise in photoresists
    for extreme ultraviolet lithography through comparative analysis of line edge
    roughness in electron beam and extreme ultraviolet lithography," *J. Vac. Sci.
    Technol. B* **35**, 061602 (2017),
    [doi:10.1116/1.4991054](https://doi.org/10.1116/1.4991054).
11. NIST, CODATA recommended value, "lattice parameter of silicon," 5.431 020 511 × 10⁻¹⁰ m,
    [physics.nist.gov](https://physics.nist.gov/cgi-bin/cuu/Value?asil).
12. IEEE IRDS, *International Roadmap for Devices and Systems, 2024 Edition: More Moore*,
    [pdf](https://irds.ieee.org/images/files/pdf/2024/2024IRDS_MM.pdf) (ground-rule table
    MM-7).
