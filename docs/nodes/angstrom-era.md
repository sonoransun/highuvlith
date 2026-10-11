---
description: "Nodes named in ångströms, High-NA EUV (0.55 NA, anamorphic, half-field), CFETs and the IRDS and imec roadmaps into the 2030s, with production facts, company plans, roadmaps and speculation kept apart."
---

# The Ångström era

<p class="huv-lead">Chapter 5 of 5 — Node names have run out of nanometres. Intel's 18A and 14A, TSMC's A16, A14 and A13 and the IRDS's "A10 eq" and "A7 eq" count in ångströms (1 Å = 0.1 nm), and they are still names, not dimensions: as IEEE Spectrum's guide to imec's roadmap puts it for "A7", "it's just a name; there's not necessarily any structure in the transistor that is actually 7 Angstroms." This page mixes several kinds of statement and labels each one with a readiness rung. 2025–2033: High-NA EUV is in production on selected layers, CFETs are a roadmap, Hyper-NA at 0.75 NA is a proposed feasibility study, and anything beyond it is speculative, as of September 2026.</p>

<figure markdown="span">

![The node ruler of the section index with the years 2025–2033 highlighted and the rest dimmed: transistor type, tightest metal pitch, node name and critical-layer tool against the year.](../assets/images/nodes/node-ladder-angstrom-era-light.svg#gh-light-mode-only)
![The node ruler of the section index with the years 2025–2033 highlighted and the rest dimmed: transistor type, tightest metal pitch, node name and critical-layer tool against the year.](../assets/images/nodes/node-ladder-angstrom-era-dark.svg#gh-dark-mode-only)

<figcaption><strong>You are here: 2025–2033.</strong> This chapter's years on the node ruler of the section index. <em>Data chart; no model badge.</em></figcaption>
</figure>

<figure markdown="span">

![A 104 by 132 mm reticle image area maps to a 26 by 33 mm full field at 4x/4x reduction for 0.33 NA, and to a 26 by 16.5 mm half field at 4x/8x reduction for 0.55 NA, where a larger die needs a second, stitched exposure.](../assets/images/nodes/anamorphic-field-light.svg#gh-light-mode-only)
![A 104 by 132 mm reticle image area maps to a 26 by 33 mm full field at 4x/4x reduction for 0.33 NA, and to a 26 by 16.5 mm half field at 4x/8x reduction for 0.55 NA, where a larger die needs a second, stitched exposure.](../assets/images/nodes/anamorphic-field-dark.svg#gh-dark-mode-only)

<figcaption><strong>Anamorphic reduction halves the field.</strong> 4× across the scan and 8× along it lets High-NA optics keep today's reticles at the price of a 26 × 16.5 mm half field; field sizes from the IRDS 2023 lithography chapter. <em>Schematic, not to scale.</em></figcaption>
</figure>

## At a glance

<!-- huv-table: stack -->

| Process | Volume production | Transistor | CPP | MMP | Critical-layer patterning |
|---|---|---|---|---|---|
| Intel 18A | late 2025 | RibbonFET (GAA) + PowerVia backside power | ≈ 50 nm (reported) | ≈ 32 nm (reported) | EUV; High-NA on selected layers (reported, 2026) |
| TSMC A16 | planned 2026 (TSMC, 2024); later reports 2H 2026–2027 | nanosheet + Super Power Rail | — | — | EUV; no High-NA (TSMC) |
| TSMC A14 | planned 2028 | nanosheet | — | — | EUV; no High-NA (TSMC) |
| Intel 14A | risk production 2H 2027, ramp 2028 (plans) | — | — | — | High-NA on the tightest layers, conventional-EUV fallback (reported) |
| IRDS 2027 (G48M22) | projection | nanosheet | 48 nm | 22 nm | High-NA EUV (IRDS) |
| IRDS 2031 (G44M18, "A7 eq") | projection | CFET | 44 nm | 18 nm | High-NA EUV (IRDS) |

Pitches are rounded reported values or roadmap targets; a dash means no value we could
verify. Plans slip: the "Where things stand" table below says which rung every row sits on.

## Where things stand (September 2026)

Every row carries one of the seven readiness rungs. Rung words follow the
[readiness ladder](../future/index.md#readiness-ladder); chips are the simulator's grades from the
[capability matrix](../capability-matrix.md).

<!-- huv-table: stack -->

| Development | Rung | Source |
|---|---|---|
| TSMC N2, first TSMC nanosheet process, volume production 4Q 2025 | **In production** | TSMC |
| Intel 18A: RibbonFET + PowerVia, high-volume production from late 2025 | **In production** | Intel; CNBC |
| High-NA EUV patterning selected Intel 18A layers; > 1 million wafers by September 2026 | **In production** (reported) | ASML and Intel, as reported by the trade press |
| ASML EXE:5000, first 0.55-NA scanner (8 nm resolution, ≥ 110 wafers/h at 50 mJ/cm²); at Intel since December 2023 | **Shipping** | ASML; Intel |
| ASML EXE:5200B (≥ 175 wafers/h at 50 mJ/cm², "sub-2 nm Logic nodes"); accepted by Intel December 2025 | **Shipping** | ASML; trade press |
| TSMC N2P (2H 2026) and A16 with "Super Power Rail" backside power (planned 2026; later reports 2H 2026–2027) | **Announced** | TSMC; trade press |
| TSMC A14 (production 2028); A13 and A12 with Super Power Rail, 2029 | **Announced** | TSMC, April 2026 |
| TSMC High-NA EUV: not for A16 or A14; volume manufacturing from 2030 | **Announced** (reported) | trade press, 2025–2026 |
| Intel 14A: risk production 2H 2027, volume ramp 2028; High-NA with a conventional-EUV fallback | **Announced** (reported) | trade press, 2025–2026 |
| Rapidus 2 nm: prototype wafers 2025; volume manufacturing targeted for 2027 | **Announced** | trade press |
| IRDS 2024: G48M22 in 2027, G46M20 in 2029, CFET from 2031 (G44M18), G44M16 in 2033 | **Roadmap** | IRDS |
| imec (2026): CFETs from about 2033 ("A7"), 2D-material channels around 2041, EUV through the 2030s | **Roadmap** | imec via IEEE Spectrum |
| Hyper-NA EUV (NA ≈ 0.75): "under consideration", ASML feasibility studies | **Proposed** (feasibility studies) | IRDS 2023; ASML statements |
| NA ≈ 0.85: reported "under investigation" | **Speculative** | trade press, May 2026 |
| Beyond-EUV wavelengths, 6–7 nm: "being assessed" | **Speculative** | IRDS 2023 |

<details markdown="1">
<summary>Why these rungs: the ladder's definitions and how this page's earlier labels map onto them</summary>

The seven rungs, ordered by distance from volume manufacturing, are defined once on the
[future index](../future/index.md#readiness-ladder) and used verbatim here. A qualifier in parentheses keeps the
old nuance, so no claim changes meaning, with one exception: this page used to file Hyper-NA under Speculative;
on the shared ladder a feasibility study is a design on paper, so NA ≈ 0.75 now sits on **Proposed** and only
NA ≈ 0.85 on **Speculative**, the same rungs the
[High-NA page](../future/high-na-and-hyper-na.md#readiness-at-a-glance) uses.

| Rung | Definition | This page's earlier label |
|---|---|---|
| **In production** | running in volume manufacturing, per the company or credible reporting; "(reported)" when the evidence is press coverage | In production |
| **Shipping** | a production tool delivered to or accepted by at least one customer, per the vendor or the customer | Shipping |
| **Announced** | a dated company plan or a national programme with published plans (plans slip) | Announced |
| **Demonstrated** | the key physics or result shown experimentally, not at production scale | — (not used on this page) |
| **Roadmap** | an industry projection (IRDS, imec), not a commitment by anyone | Roadmap |
| **Proposed** | worked out on paper as a design or feasibility study, not built | Speculative (the Hyper-NA 0.75 row only; see above) |
| **Speculative** | physically possible in principle; under consideration or in research; no design, no product plan | Speculative |

</details>

## High-NA EUV

Raising the NA from 0.33 to 0.55 cuts λ/NA from 40.9 nm to 24.5 nm. ASML quotes 8 nm
resolution (half-pitch) for the EXE systems, "features 1.7 times smaller" and "transistor
densities 2.9 times higher" than NXE systems in a single exposure, and "40% more imaging
contrast". The catch is geometric. Because the mask multilayer reflects well only over a limited
range of angles, the IRDS explains, "the lens reduction in the scan direction is increased to
8×, while maintaining a reduction of 4× in the perpendicular direction", so the maximum field on
the wafer becomes 26 mm × 16.5 mm, half of today's 26 × 33 mm, as long as today's 6-inch mask
size is kept (the anamorphic field figure above; the
[High-NA page](../future/high-na-and-hyper-na.md#what-it-costs-part-2-the-mask-and-why-the-optics-are-anamorphic) works through the mask side). Large dies must
be stitched from two exposures, or masks must grow: 12 × 6 inch masks are under consideration,
and TSMC and ASML have reportedly planned a pilot line for them in 2031.

The other costs follow from the arithmetic on the [litho-math page](litho-math.md): depth of
focus shrinks as 1/NA², about 2.8× from 0.33 to 0.55 (2.9× with B. J. Lin's non-paraxial
formula), so resists get thinner, which makes stochastics worse unless resists absorb more.
Polarization starts to matter too: already at 0.55 NA a small contrast loss from unpolarized
light is predicted. High-NA projection optics also have a central obscuration in the pupil
(H. J. Levinson, 2022; the simulator's EUV preset carries an assumed one, see
[optical systems](../optics.md#euv-scanner-projection-optics)). The IRDS's list of key High-NA challenges reads: resists with
low stochastic defect levels and no pattern collapse; light sources that support shot-noise and
productivity requirements; small depth of focus; computational lithography; masks and new
absorbers; solutions for large dies; and cost.

<figure markdown="span">

![Contrast versus pitch from 12 to 40 nm for 0.33 and 0.55 NA EUV optics with a tin plasma source: the 0.33 NA curve reaches zero near 22 nm pitch, the 0.55 NA curve near 13 nm.](../assets/images/sim/site/site-euv-nxe-vs-high-na-light.png#gh-light-mode-only)
![Contrast versus pitch from 12 to 40 nm for 0.33 and 0.55 NA EUV optics with a tin plasma source: the 0.33 NA curve reaches zero near 22 nm pitch, the 0.55 NA curve near 13 nm.](../assets/images/sim/site/site-euv-nxe-vs-high-na-dark.png#gh-dark-mode-only)

<figcaption><strong>Resolution at NA 0.33 against NA 0.55.</strong> Contrast of 1:1 lines against pitch through the simulator's EUV presets (Sn LPP, conventional σ 0.9, two-period commensurate fields): isotropic pupil — the anamorphic 4×/8× magnification is not modelled — with a 0.2·NA central obscuration assumed for High-NA; thin mask (no mask-3D shadowing), no flare. <em>Model: Hopkins imaging ✅ · EUV projection optics 🔶 (obscuration assumed).</em></figcaption>
</figure>

Who uses it, and when, is a company decision, and the three leaders have chosen differently.
Intel took the first tool, describes High-NA as the way to extend its roadmap "beyond Intel
18A", and by 2026 was reported to be using it on selected production layers; Intel 14A is
planned around it. TSMC said it would not use High-NA for A16 or A14 and has reportedly
committed to it for volume manufacturing from 2030. Samsung has reportedly placed High-NA at
around its 1 nm-class generation. The IRDS 2024 roadmap expects ground-rule scaling to "slow
down and saturate around 2031", with "High-NA Extreme-ultraviolet (EUV) technology" as "the
enabler" (its 2022 edition said "around 2028", with EUV technology as the enabler).

## CFET: stacking n on p

The next transistor stacks the nFET and pFET of a CMOS pair on top of each other; both the
IRDS and imec put it at the centre of their roadmaps once pitch scaling slows. Research
demonstrations so far:

- TSMC, IEDM 2023: a CFET "demonstration at 48nm gate pitch"; IEDM 2024: the "first
  demonstration of a monolithic CFET inverter at 48nm gate pitch".
- Intel, IEDM 2023: a stacked CMOS inverter at 60 nm gate pitch with a power via and direct
  backside device contacts. Samsung showed stacked devices at IEDM 2023 as well.
- TSMC, IEDM 2025: the "first demonstration of CFET ring oscillator and SRAM bit-cell
  functionality at gate pitch smaller than 48 nm".
- According to IEEE Spectrum (May 2026), Samsung is presenting a six-nanosheet CFET at VLSI 2026,
  and Intel is testing a scheme that builds the top device from a second, bonded wafer.

An intermediate step, imec's **forksheet**, keeps n and p sheets side by side but separates
them with a thin dielectric wall, so they can be packed closer; imec proposed it in 2017.

On the roadmaps: the IRDS 2024 edition makes CFET the platform logic device from 2031
(G44M18, "A7 eq"), and imec's 2026 roadmap places its commercial introduction around 2033.
**Both are projections.** For lithography, CFET is interesting because it gains density
without a tighter gate or metal pitch: the IRDS 2024 table keeps the gate pitch at 42–48 nm
through 2039.

## The pitch roadmap, and how it moved

Between the 2022 and 2024 editions of the IRDS the near- and mid-term targets were *relaxed*:
the gate pitch is larger in every year, the 2025 metal pitch went from 20 to 22 nm, and the
16 nm metal pitch moved from 2028 to 2033 (the 2024 edition then continues to 14 nm from 2035).

<figure markdown="span">

![Two small line charts of IRDS roadmap targets against year, comparing the 2022 and 2024 editions. Gate pitch: the 2024 edition sits above the 2022 edition at every year. Metal pitch: the 2024 edition reaches 16 nm in 2033 where the 2022 edition had it in 2028.](../assets/images/nodes/irds-roadmap-light.svg#gh-light-mode-only)
![Two small line charts of IRDS roadmap targets against year, comparing the 2022 and 2024 editions. Gate pitch: the 2024 edition sits above the 2022 edition at every year. Metal pitch: the 2024 edition reaches 16 nm in 2033 where the 2022 edition had it in 2028.](../assets/images/nodes/irds-roadmap-dark.svg#gh-dark-mode-only)

<figcaption><strong>IRDS ground-rule targets, 2022 versus 2024 edition.</strong> The roadmap relaxed its pitches and leaned on stacking. <em>Data chart from the IRDS More Moore tables; no model badge.</em></figcaption>
</figure>

The long-term gains come from stacking (tiers, CFET) rather than from pitch. A metal pitch of
16 nm would need k₁ = 0.20 on 0.33-NA EUV (impossible in one exposure) and 0.33 on High-NA;
14 nm would need 0.29 on High-NA. imec's own roadmap, as reported in mid-2026, tells a similar
story: contacted gate pitch stops scaling at about the A10 generation (around 2030), High-NA
enters at A14 (around 2028), CFET is a candidate at A7 (around 2033), and further generations
may need Hyper-NA.

<details markdown="1">
<summary>The IRDS targets as a table: 2022 against 2024 edition, year by year</summary>

| Year | IRDS 2022 edition | IRDS 2024 edition |
|---|---|---|
| 2025 | G45M20, "2nm" | G48M22, "2nm" |
| 2027–2028 | G42M16 (2028), "1.5nm" | G48M22 (2027), "1.4nm" |
| 2029 | — | G46M20, "A10 eq" |
| 2031 | G40M16/T2, "1.0nm eq" | G44M18, "A7 eq" (CFET) |
| 2033–2034 | G38M16/T4 (2034), "0.7nm eq" | G44M16 (2033), "A5 eq" |
| 2035 | — | G42M14/T2, "A3.5 eq" |
| 2037 | G38M16/T6, "0.5nm eq" | G42M14/T4, "A2.5 eq" |

Gxx = contacted gate pitch, Mxx = tightest metal pitch, both in nm; /Tn = number of stacked
device tiers.

</details>

## Proposed and speculative: beyond 0.55 NA and 13.5 nm

The IRDS 2023 lithography chapter lists two ways to go further, both explicitly longer-term:

- **Hyper-NA EUV (NA ≈ 0.75)** is "under consideration", and ASML has described it as the
  subject of feasibility studies, not a product plan: Proposed on the ladder, while the 0.85 NA
  the trade press reports as "under investigation" stays Speculative. Depth of focus would be
  so small that resists would have to be thinner than 20 nm; polarized light would be needed to
  reach the optics' potential, which today's tin-plasma sources do not provide (free-electron
  lasers do); and it "could prove less effective than multiple patterning of 0.33 or 0.55 NA
  EUV".
- **Beyond-EUV wavelengths of 6–7 nm** are "being assessed", "a much bigger change than
  increased NA": every mirror and the mask need multilayer coatings designed for the new
  wavelength, and because each photon carries twice the energy, keeping the same photon shot
  noise means delivering twice the light energy to the wafer.

The future section takes both further, in
[High-NA and Hyper-NA EUV](../future/high-na-and-hyper-na.md) and
[Beyond EUV](../future/beyond-euv.md), and compares candidate
[accelerator light sources](../future/accelerator-light-sources.md) with today's tin plasmas.
highuvlith models the relevant sources with their stated status: the 6.7 nm gadolinium
laser-produced plasma is part of the ✅ Implemented [LPP](../sources/lpp.md) family model with
its power graded 🧪 Theoretical, and accelerator sources such as the [XFEL](../sources/xfel.md)
and [steady-state microbunching](../sources/ssmb.md) carry their own grades.

## Try it in highuvlith

**What highuvlith models here.** The 0.33 and 0.55 NA presets are isotropic wafer-side pupils with an assumed obscuration; the anamorphic optics, mask 3-D effects and the field size are outside the model.

<!-- huv-table: stack -->

| Issue | In highuvlith | Grade | Matrix row |
|---|---|---|---|
| 0.33 against 0.55 NA at 24 and 16 nm pitch | Hopkins/SOCS aerial image through `euv_nxe()` and `euv_high_na()` | ✅ Implemented | [Hopkins imaging (factorized TCC/SOCS)](../capability-matrix.md#core-imaging) |
| The High-NA optics | isotropic pupil, 0.2·NA central obscuration assumed; no anamorphic 4×/8× magnification, no mask 3-D | 🔶 Simplified | [EUV scanner projection optics](../capability-matrix.md#optical-systems) |
| Depth of focus at 0.55 NA | exact non-paraxial defocus phase per source point | ✅ Implemented | [Defocus / through-focus imaging](../capability-matrix.md#core-imaging) |
| CFETs, stitching, Hyper-NA polarization, 6.7 nm optics | not modelled | — | — |

### 0.33 versus 0.55 NA at 24 and 16 nm pitch

At 24 nm pitch both systems image a grating, 0.55 NA with far more contrast; at 16 nm
(ASML's 8 nm half-pitch figure) only 0.55 NA forms an image with σ = 0.9, since
$\lambda/(p \mathrm{NA}) = 2.6 > 1 + \sigma$ at 0.33 NA. The High-NA preset includes a
central pupil obscuration of assumed, representative size.

<!-- verify-example -->
```python
import highuvlith as huv

src = huv.SourceConfig.lpp_sn_13nm5(0.9)
for name, opt in (("NA 0.33", huv.OpticsConfig.euv_nxe()), ("NA 0.55", huv.OpticsConfig.euv_high_na())):
    row = []
    for pitch in (24.0, 16.0):
        mask = huv.MaskConfig.line_space(pitch / 2, pitch)
        eng = huv.SimulationEngine(src, opt, mask, grid=huv.GridConfig(64, pitch / 16))
        row.append(round(eng.compute_aerial_image(focus_nm=0.0).image_contrast(), 2))
    print(f"{name}: contrast at 24 / 16 nm pitch = {row}")
```

???+ success "Output"

    ```text
    NA 0.33: contrast at 24 / 16 nm pitch = [0.11, 0.0]
    NA 0.55: contrast at 24 / 16 nm pitch = [0.74, 0.26]
    ```

highuvlith works in wafer coordinates with a scalar thin-mask model and an isotropic
wafer-side pupil, so the anamorphic magnification itself does not enter the image; its real
consequences (mask 3D effects, field size) are outside the model.

### The depth-of-focus price of 0.55 NA

Image a 32 nm pitch grating, which both systems print, through focus. The 0.55-NA image
starts with more contrast but loses it much sooner: the paraxial scaling predicts about
$(0.55/0.33)^2 \approx 2.8$ times less depth of focus.

<!-- verify-example -->
```python
import highuvlith as huv

src = huv.SourceConfig.lpp_sn_13nm5(0.9)
mask = huv.MaskConfig.line_space(16.0, 32.0)
for name, opt in (("NA 0.33", huv.OpticsConfig.euv_nxe()), ("NA 0.55", huv.OpticsConfig.euv_high_na())):
    eng = huv.SimulationEngine(src, opt, mask, grid=huv.GridConfig(64, 2.0))   # 4 periods
    print(name, [round(eng.compute_aerial_image(focus_nm=z).image_contrast(), 2) for z in (0, 20, 40, 80)])
```

???+ success "Output"

    ```text
    NA 0.33 [0.51, 0.5, 0.46, 0.34]
    NA 0.55 [0.79, 0.67, 0.39, 0.01]
    ```

<figure markdown="span">

![Contrast versus focus for 0.33 and 0.55 NA EUV: at the same 32 nm pitch the 0.55 NA image starts higher but fades faster with defocus; at equal k1 both start near 0.5 and the 0.55 NA image fades about three times faster.](../assets/images/sim/site/site-euv-depth-of-focus-light.png#gh-light-mode-only)
![Contrast versus focus for 0.33 and 0.55 NA EUV: at the same 32 nm pitch the 0.55 NA image starts higher but fades faster with defocus; at equal k1 both start near 0.5 and the 0.55 NA image fades about three times faster.](../assets/images/sim/site/site-euv-depth-of-focus-dark.png#gh-dark-mode-only)

<figcaption><strong>The depth-of-focus price of NA 0.55.</strong> Sn LPP, conventional σ 0.9: (a) the angstrom-era example, a 32 nm pitch through the NXE and High-NA presets; (b) the High-NA page's example, equal k<sub>1</sub> (32 nm at 0.33, 19.2 nm at 0.55) through ideal unobscured pupils. The engine applies the exact non-paraxial defocus phase per source point; image in air, no resist, no mask-3D best-focus shifts. <em>Model: defocus / through-focus imaging ✅ · EUV projection optics 🔶.</em></figcaption>
</figure>

## Key takeaways

- "A" node names are generation labels in ångströms; the physical story is still gate pitch,
  metal pitch and, increasingly, the number of stacked device tiers.
- High-NA EUV (0.55 NA, 8 nm half-pitch) is shipping and, at Intel, already patterning selected
  production layers; it buys 1.7× finer single-exposure features at the cost of a half-size
  field (anamorphic 4×/8×), 2.8× less depth of focus and a harder stochastics problem. TSMC
  plans High-NA volume manufacturing from 2030.
- In production: nanosheets (N2, 18A) and backside power (18A). Announced: A16, A14 (2028),
  A13/A12 (2029), Intel 14A. Roadmap: CFET around 2031–2033 (IRDS, imec), 2D channels around
  2041 (imec).
- The IRDS relaxed its pitch roadmap between 2022 and 2024 and now expects ground-rule scaling
  to saturate around 2031, with density coming from stacking; Hyper-NA at 0.75 NA is a
  proposed feasibility study, and 0.85 NA and 6–7 nm lithography remain speculative.

**Where next:** ← [3 nm and 2 nm](3nm-and-2nm.md) · [Lithography arithmetic](litho-math.md) → · Related: [High-NA and Hyper-NA EUV](../future/high-na-and-hyper-na.md) · [The future](../future/index.md) · [Optical systems](../optics.md) · [capability matrix](../capability-matrix.md#optical-systems)

## References and further reading

- ASML product pages: [EXE:5000](https://www.asml.com/en/products/euv-lithography-systems/twinscan-exe-5000),
  [EXE:5200B](https://www.asml.com/en/products/euv-lithography-systems/twinscan-exe-5200b)
  (retrieved 2026-09-30); "[5 things you should know about High NA EUV](https://www.asml.com/en/news/stories/2024/5-things-high-na-euv)"
  (January 2024).
- Intel, "[High-NA EUV press kit](https://www.intel.com/content/www/us/en/newsroom/resources/intel-high-na-euv.html)"
  (first High-NA system received and assembled, April 2024); "[Intel Foundry achieves major
  milestones](https://www.intel.com/content/www/us/en/newsroom/news/intel-foundry-achieves-major-milestones.html)"
  (6 August 2024); "[Continued momentum for Intel 18A](https://www.intel.com/content/www/us/en/newsroom/opinion/continued-momentum-intel-18a.html)"
  (4 September 2024).
- TSMC press releases: [A16](https://pr.tsmc.com/english/news/3136) (24 April 2024),
  [A14](https://pr.tsmc.com/english/news/3228) (April 2025),
  [A13/A12](https://pr.tsmc.com/english/news/3302) (April 2026); technology pages for
  [N2](https://www.tsmc.com/english/dedicatedFoundry/technology/logic/l_2nm) and
  [A16](https://www.tsmc.com/english/dedicatedFoundry/technology/logic/l_A16).
- *Tom's Hardware* coverage of High-NA deployment, the Intel 14A schedule, TSMC's High-NA and
  large-mask plans and imec's 2026 roadmap (2025–2026), and "[ASML's roadmap for chipmaking
  lithography tools examined](https://www.tomshardware.com/tech-industry/semiconductors/asml-lithograpy-roadmap-examined-from-duv-to-hyper-na)"
  (1 May 2026: 0.75 NA the primary Hyper-NA target, 0.85 NA under investigation); CNBC, 19 December 2025, on 18A volume
  production.
- IEEE IRDS, *2023 Edition: Lithography*
  ([pdf](https://irds.ieee.org/images/files/pdf/2023/2023IRDS_Litho.pdf)) and *More Moore*,
  2022 and 2024 editions ([2022 pdf](https://irds.ieee.org/images/files/pdf/2022/2022IRDS_MM.pdf),
  [2022 tables](https://irds.ieee.org/images/files/pdf/2022/2022IRDS_MM_Tables.xlsx),
  [2024 pdf](https://irds.ieee.org/images/files/pdf/2024/2024IRDS_MM.pdf),
  [2024 tables](https://irds.ieee.org/images/files/pdf/2024/2024IRDS_MM_Tables.xlsx)).
- H. J. Levinson, "High-NA EUV lithography: current status and outlook for the future,"
  *Jpn. J. Appl. Phys.* **61**, SD0803 (2022),
  [doi:10.35848/1347-4065/ac49fa](https://doi.org/10.35848/1347-4065/ac49fa).
- I. Lee et al., "Hyper-NA EUV lithography: an imaging perspective," *Proc. SPIE* **12494**
  (2023), [doi:10.1117/12.2659153](https://doi.org/10.1117/12.2659153).
- B. J. Lin, "The k3 coefficient in non-paraxial λ/NA scaling equations for resolution, depth
  of focus, and immersion lithography," *J. Micro/Nanolith. MEMS MOEMS* **1**(1), 7–12 (2002),
  [doi:10.1117/1.1445798](https://doi.org/10.1117/1.1445798).
- S. K. Moore, "The Next 15 Years of Moore's Law, According to Imec," *IEEE Spectrum*,
  19 May 2026 ([link](https://spectrum.ieee.org/semiconductor-technology-roadmap)); IEEE
  Spectrum on the IEDM 2023 CFET results ([link](https://spectrum.ieee.org/cfet-intel-samsung-tsmc)).
- S. Liao et al., "Complementary Field-Effect Transistor (CFET) Demonstration at 48nm Gate
  Pitch for Future Logic Technology Scaling," *IEDM* 2023,
  [doi:10.1109/IEDM45741.2023.10413672](https://doi.org/10.1109/IEDM45741.2023.10413672);
  "First Demonstration of Monolithic CFET Inverter at 48nm Gate Pitch Toward Future Logic
  Technology Scaling," *IEDM* 2024,
  [doi:10.1109/IEDM50854.2024.10873334](https://doi.org/10.1109/IEDM50854.2024.10873334);
  "First Demonstration of CFET Ring Oscillator and SRAM Bit-Cell Functionality at Gate Pitch
  Smaller Than 48 nm for Future Logic and SRAM Technology," *IEDM* 2025,
  [doi:10.1109/IEDM50572.2025.11353820](https://doi.org/10.1109/IEDM50572.2025.11353820).
- M. Radosavljević et al., "Demonstration of a Stacked CMOS Inverter at 60nm Gate Pitch with
  Power Via and Direct Backside Device Contacts," *IEDM* 2023,
  [doi:10.1109/IEDM45741.2023.10413678](https://doi.org/10.1109/IEDM45741.2023.10413678).
- P. Weckx et al., "Stacked nanosheet fork architecture for SRAM design and device
  co-optimization toward 3nm," *IEDM* 2017,
  [doi:10.1109/IEDM.2017.8268430](https://doi.org/10.1109/IEDM.2017.8268430) (forksheet).
- Wikipedia: "[2 nm process](https://en.wikipedia.org/wiki/2_nm_process)" and
  "[Rapidus](https://en.wikipedia.org/wiki/Rapidus)" (production chronology).
