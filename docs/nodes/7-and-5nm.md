---
description: "2018–2020: immersion multipatterning versus EUV insertion, EUV single exposure at 30–40 nm pitch, photon shot noise and stochastic defects, pellicles and reflective masks."
---

# 7 nm and 5 nm: EUV arrives

By 2018 the tightest metal pitches had reached about 40 nm, and by 2020 about 28–30 nm. On an
ArF-immersion scanner those pitches mean spacer quadruple patterning plus cut masks, or four
litho-etch passes. On an EUV scanner (13.5 nm, NA 0.33) the optics could print them in one
exposure, at k₁ = 0.49 and 0.34–0.37. After decades of development (told in the history
section's [EUV chapter](../history/euv.md)), EUV entered volume production in exactly this
window, and the industry learned what it costs: a tin-plasma light source, all-reflective
optics and masks, vacuum, and a new enemy, photon shot noise.

## Two roads to 7 nm

The 7 nm generation was built both ways:

| Process | Volume production | Contacted gate pitch | Tightest metal pitch | Critical-layer patterning |
|---|---|---|---|---|
| TSMC N7 | 2018 | 57 nm (64 nm in high-performance cells) | 40 nm | ArF immersion only: SAQP for fins, SADP for gates and M0 (reported) |
| Samsung 7LPP | wafer production from October 2018; first product 2019 | ≈ 54 nm | ≈ 36 nm EUV metal (reported) | EUV on metal layers; 44 nm-pitch layers still quadruple-patterned (reported) |
| TSMC N7+ | Q2 2019 | — | — | EUV on up to 4 layers (reported) |
| TSMC N6 | 2020 | — | — | more EUV than N7+; 18 % higher logic density than N7 |
| Intel 4 | 2023 | 50 nm | 30 nm | Intel's first EUV process; EUV on selected lower metal and via layers, 30 nm M0 by immersion multipatterning (reported) |

TSMC says it was the first foundry to start 7 nm volume production, in 2018, and it did so
without EUV. On 18 October 2018 Samsung announced that it had "started wafer production" of
its EUV-based 7LPP process, arguing that where ArF "can require up to 4 masks" for a layer, EUV
needs one, so that 7LPP "can reduce the total number of masks by about 20%"; its first EUV
product was the Galaxy Note10 in 2019. TSMC's N7+ entered volume production in the second
quarter of 2019, and TSMC calls it "the industry's first commercially available Extreme
Ultraviolet (EUV) lithography technology". Both companies shipped their first EUV-made
products in 2019. Intel reached EUV with Intel 4, whose VLSI 2022 paper reports a 50 nm gate
pitch, 30 nm fin pitch and 30 nm minimum metal pitch, with EUV "used extensively to simplify
the process flow".

## Immersion multipatterning versus EUV, in numbers

For a 36–40 nm metal pitch the trade looks like this ([arithmetic](litho-math.md)):

| | ArF immersion (NA 1.35) | EUV (NA 0.33) |
|---|---|---|
| k₁ for one exposure | 0.13–0.14: impossible | 0.44–0.49: comfortable |
| How the layer is made | SAQP (mandrel at 4× pitch, k₁ ≈ 0.5) + cut and via masks, or LE⁴ | single exposure (plus cuts or vias as needed) |
| Two-dimensional shapes | only through extra cut/block masks | printed directly, within limits |
| Scanner throughput (vendor spec) | ≥ 275–330 wafers/h (NXT:2000i, NXT:1980Fi) | ≥ 125 wafers/h at 20 mJ/cm² (NXE:3400B); ≥ 135 at 30 mJ/cm² (NXE:3400C) |
| Main risks | cost and cycle time of many steps, overlay of cuts, pitch walk | source power and uptime, stochastic defects, masks and pellicles |

Neither column wins everywhere. EUV replaces several immersion exposures, depositions and
etches with one exposure, and the IRDS describes it as "a remedy to pattern tight ground rules
in fewer process steps". But each EUV exposure is slower (see the throughput row), so a layer
moves to EUV only when it removes enough steps to pay for that. That is why EUV entered layer by
layer (up to 4 layers on N7+, more on N6, more than ten on N5) rather than all at once, and why
fins stayed on immersion SAQP even at 5 nm.

## Single exposure at 30–40 nm pitch

TSMC's N5 entered risk production in April 2019 and volume production in 2020. Its IEDM 2019
paper describes a platform "featuring full-fledged EUV": according to reports of the paper and
its presentation, more than ten EUV layers, each replacing more than three immersion layers
(the paper says more than four) at the cut, contact, via and metal masking steps, with a total
mask count several masks lower than 7 nm. Its gate pitch is reported at about 50–51 nm and its tightest metal pitch at about
28–30 nm, and ASML reported in 2020 that a 5 nm M0 layer at 30 nm pitch needed double
patterning.

That is the practical edge of 0.33-NA EUV. Optically, ASML specifies 13 nm half-pitch (26 nm
pitch, k₁ = 0.32). In production the limit is set earlier by stochastics: an industry white paper
by Fractilia puts the stochastic limit of high-volume processes at 16–18 nm half-pitch (32–36 nm
pitch), imec and Synopsys anticipated double patterning from about 34 nm pitch, and, according to
Lam Research, 28 nm pitch direct printing was qualified at imec only in 2025, with a new dry
resist. A reasonable summary:
**about 30–36 nm pitch in single exposure in volume production; below roughly 28–30 nm, double
patterning or High-NA.**

<figure markdown="span">
  ![Illustrative sketch of stochastic printing failures: a contact-hole array with varying hole sizes, a missing contact and two merged contacts; dense lines with rough edges, a microbridge between two lines and a break in one line.](../assets/images/nodes/stochastic-defects-light.svg#only-light){ width="720" }
  ![Illustrative sketch of stochastic printing failures: a contact-hole array with varying hole sizes, a missing contact and two merged contacts; dense lines with rough edges, a microbridge between two lines and a break in one line.](../assets/images/nodes/stochastic-defects-dark.svg#only-dark){ width="720" }
  <figcaption>Stochastic failure modes: size variation, missing and merged contacts, rough
  edges, microbridges and breaks. Schematic, not simulated.</figcaption>
</figure>

## Stochastics: when photons run out

An EUV photon carries 92 eV, fourteen times the energy of an ArF photon, so the same dose
delivers fourteen times fewer photons. A (20 nm)² contact exposed at 30 mJ/cm² receives about
8,000 incident photons, and a thin resist absorbs only a fraction of them: an organic
chemically amplified resist absorbs roughly 5 µm⁻¹ at 13.5 nm, so a 35 nm film takes up about
16 % (worked numbers on the [arithmetic page](litho-math.md#counting-photons)). The absorbed
photons then release secondary electrons, which generate acid, which diffuses and deprotects the
resist, and every one of those steps is random too.

Besides roughness, "stochastic effects can also give rise to local, random printing failures,
such as missing contacts or microbridges in spaces" (P. De Bisschop, imec, 2017), at failure
rates that must be pushed extremely low because a chip contains billions of contacts. The IRDS
lithography roadmap says stochastic variations "can result in poor pattern quality, such as line
width roughness or poor CD uniformity, and they can result in actual pattern defects, such as
missing or bridged features", and that they "get worse as feature sizes shrink". The defects
rise steeply: Fractilia reports a tenfold increase in stochastic defects going from 18 nm to
16 nm half-pitch.

The answer has been more dose. In 2018 chemically amplified resists for 7 nm were expected to
run at 30 to 40 mJ/cm², and resists for the most aggressive pitches were reported to need
twice or more the targeted dose to reach production defect levels. The IRDS 2022 roadmap
projected that dose-to-print would rise about threefold over four nodes, and its 2023 edition
reports that this projection is on track. The scaling is harsh: to keep the same relative
roughness, "if CD shrinks by 0.7, exposure dose must increase by a factor of 3" (C. A. Mack,
2018). Dose costs throughput directly: ASML specifies the NXE:3400C at ≥ 170 wafers per hour at
20 mJ/cm² but ≥ 135 at 30 mJ/cm².

The other lever is the resist. Organic materials all absorb EUV about equally, whatever their
chemistry, so the remedy has been to add strongly absorbing metals: tin-based **metal-oxide
resists** absorb roughly 10–20 µm⁻¹, two to four times more than organic resists, and have
entered production (Inpria, a pioneer of tin-oxide resists, became part of JSR in 2021; Lam
Research announced a vapour-deposited, dry-developed resist in February 2020). The IRDS expects
chemically amplified resists to remain the workhorse to at least the 1 nm node, with
metal-oxide resists becoming important from about the 1.5 nm node, especially with High-NA.

!!! info "Why dose alone is not a free fix"
    Doubling the dose halves the relative variance of the photon count, so the 3σ noise drops
    only by √2 (3.3 % → 2.3 % for the (20 nm)² example at 30 → 60 mJ/cm²), while the exposure
    time roughly doubles. Fractilia puts it the same way: doubling the dose "only reduces the
    stochastics level by about 1.4" while throughput falls by almost a factor of two. That is
    why source power has been one of the defining EUV engineering races, and why resist
    absorption and chemistry matter as much as the scanner.

What happens as the half-pitch keeps shrinking, and what the proposed remedies cost, is the
subject of [the stochastic frontier](../future/stochastic-frontier.md) in the future section.

## Reflective masks and pellicles

Nothing transmits 13.5 nm light well, so the EUV mask is a mirror: 40 to 50 alternating layers
of molybdenum and silicon form a Bragg reflector, capped with ruthenium and topped with a
patterned tantalum-based absorber. "In EUV, light hits the mask at an angle of 6°" so that the
reflected light can clear the incoming beam. Each multilayer mirror reflects only about 70 %, so
after the dozen or so reflections in the illuminator and projection optics only a few percent of
the collected light reaches the wafer. The absorber is tens of nanometres thick, comparable to
the features, so the mask acts as a three-dimensional object: shadowing and phase effects shift
and distort the image depending on feature orientation, pitch and focus (imec measured
best-focus shifts through pitch at NA 0.33). The IRDS calls for new absorber materials "to
reduce mask 3D effects". highuvlith treats masks as thin (Kirchhoff) transmittance maps and
does **not** model these effects.

A pellicle is a thin membrane stretched above the mask so that particles land out of focus
instead of printing. For EUV it must be only tens of nanometres thick, transmit as much as
possible, and survive the heat, and because the light crosses it twice, its losses count
twice (85 % transmission is really 85 % squared). In 2018 ASML's polysilicon-based pellicle
transmitted 83 % (target 90 %), survived a 250 W source but had to be replaced after about
3,000 wafers, and a 250 W source was estimated to heat it to roughly 686 °C. ASML licensed its
pellicle business to Mitsui Chemicals in 2019, which started commercial production in 2021;
carbon-nanotube membranes with vendor-claimed transmission of 92–97 % are the next step. The IRDS
still lists a pellicle "with sufficient transmittance and long life" as an open issue in 2023.

The light itself comes from droplets of tin hit by a CO₂ laser (laser-produced plasma, the
[LPP source](../sources/lpp.md) in highuvlith, ✅ Implemented as a spectral/power model). ASML
demonstrated 250 W in 2017, the power level behind the NXE:3400B's specified 125 wafers per
hour, and the IRDS reports 600 W at intermediate focus achieved under dose-controlled conditions,
with efforts toward 1 kW.

## Try it in highuvlith

!!! example "EUV single exposure at 36 nm and 28 nm pitch"
    Both print on a 0.33-NA EUV system; the contrast drop from 36 to 28 nm pitch is the
    optical part of the story (the stochastic part comes next). With σ = 0.9 the optical
    cut-off is $\lambda / (\mathrm{NA} (1+\sigma)) \approx 21.5$ nm.

    <!-- verify-example -->
    ```python
    import highuvlith as huv

    src = huv.SourceConfig.lpp_sn_13nm5(0.9)
    opt = huv.OpticsConfig.euv_nxe()                      # NA 0.33, unobscured
    for pitch in (36.0, 28.0):
        mask = huv.MaskConfig.line_space(pitch / 2, pitch)
        eng = huv.SimulationEngine(src, opt, mask, grid=huv.GridConfig(128, pitch / 16))
        k1 = pitch / 2 * 0.33 / 13.5
        print(f"{pitch:.0f} nm pitch  k1 = {k1:.2f}  contrast = {eng.compute_aerial_image().image_contrast():.2f}")
    ```

    ??? success "Output"

        ```text
        36 nm pitch  k1 = 0.44  contrast = 0.64
        28 nm pitch  k1 = 0.34  contrast = 0.32
        ```

!!! example "Photon shot noise in a contact at 30 and 60 mJ/cm²"
    Take the aerial image of a 20 nm contact array (intensities are relative to the clear
    field), convert intensity to photons with the source wavelength, and sample Poisson counts
    for many contacts. highuvlith's stochastic module runs the full version of this, per
    pixel and with Gamma-distributed dose jitter (both ✅), and turns it into LER/LWR (🔶,
    `huv.compute_ler_lwr`).

    <!-- verify-example -->
    ```python
    import numpy as np
    import highuvlith as huv

    src = huv.SourceConfig.lpp_sn_13nm5(0.9)
    mask = huv.MaskConfig.contact_hole(20.0, 40.0, 40.0)        # 20 nm holes on a 40 nm grid
    eng = huv.SimulationEngine(src, huv.OpticsConfig.euv_nxe(), mask, grid=huv.GridConfig(64, 1.25))
    img = eng.compute_aerial_image().intensity
    per_nm2 = 1e-17 / (1239.84193 / src.wavelength_nm * 1.602176634e-19)   # ≈ 0.68 per mJ/cm²
    hole = img > 0.5 * img.max()                                 # pixels inside the 4 holes
    for dose in (30.0, 60.0):
        mean = dose * per_nm2 * img[hole].sum() * 1.25**2 / 4    # photons per hole
        n = np.random.default_rng(0).poisson(mean, 20_000)
        print(f"{dose:.0f} mJ/cm²: {mean:,.0f} photons/hole, 3σ = {300 * n.std() / n.mean():.1f} %")
    ```

    ??? success "Output"

        ```text
        30 mJ/cm²: 1,894 photons/hole, 3σ = 6.9 %
        60 mJ/cm²: 3,787 photons/hole, 3σ = 4.9 %
        ```

## Key takeaways

- TSMC reached 7 nm (2018) with immersion multipatterning only; Samsung's 7LPP (wafer
  production from October 2018) and TSMC's N7+ (volume production 2019) were the first EUV
  processes; EUV then spread layer by layer, to more than ten layers on TSMC's N5.
- At 30–40 nm pitch EUV runs at k₁ ≈ 0.37–0.49 in one exposure, where immersion would need SAQP
  plus cuts. In volume production, stochastics limit EUV single exposure to about 30–36 nm
  pitch; tighter pitches need double patterning or High-NA.
- The new limit is stochastic: fourteen times fewer photons per unit dose, missing and bridged
  features, and a dose-to-print that the IRDS projects to triple over four nodes.
- Reflective masks bring mask 3D effects and a hard pellicle problem (83 % transmission in 2018).
  highuvlith models imaging and shot noise, not mask 3D effects.

## References and further reading

- TSMC, "[TSMC's N7+ Technology is First EUV Process Delivering Customer Products to Market in High Volume](https://pr.tsmc.com/english/news/2010)"
  (7 October 2019) and "[TSMC and OIP Ecosystem Partners Deliver Industry's First Complete Design Infrastructure for 5nm Process Technology](https://pr.tsmc.com/english/news/1987)"
  (3 April 2019, N5 risk production); technology pages for
  [7nm](https://www.tsmc.com/english/dedicatedFoundry/technology/logic/l_7nm) and
  [5nm](https://www.tsmc.com/english/dedicatedFoundry/technology/logic/l_5nm).
- Samsung, "[Samsung Electronics Starts Production of EUV-based 7nm LPP Process](https://news.samsung.com/global/samsung-electronics-starts-production-of-euv-based-7nm-lpp-process)"
  (18 October 2018).
- S.-Y. Wu et al., "A 7nm CMOS platform technology featuring 4th generation FinFET transistors
  with a 0.027um² high density 6-T SRAM cell for mobile SoC applications," *IEDM* 2016,
  [doi:10.1109/IEDM.2016.7838333](https://doi.org/10.1109/IEDM.2016.7838333).
- G. Yeap et al., "5nm CMOS Production Technology Platform featuring full-fledged EUV, and High
  Mobility Channel FinFETs with densest 0.021µm² SRAM cells for Mobile SoC and High Performance
  Computing Applications," *IEDM* 2019,
  [doi:10.1109/IEDM19573.2019.8993577](https://doi.org/10.1109/IEDM19573.2019.8993577);
  S. Jones on the IEDM 2019 N5 paper, SemiWiki, 16 December 2019
  ([link](https://semiwiki.com/semiconductor-manufacturers/intel/280519-iedm-2019-tsmc-5nm-process/)).
- *Semiconductor Digest* on Intel's VLSI 2022 papers, including Intel 4
  ([link](https://www.semiconductor-digest.com/intel-has-13-talks-at-the-vlsi-symposia-in-june-including-intel-4/));
  S. Jones on Intel 4 at VLSI 2022, SemiWiki
  ([link](https://semiwiki.com/semiconductor-manufacturers/intel/314047-intel-4-presented-at-vlsi/)).
- IEEE IRDS, *2023 Edition: Lithography*
  ([pdf](https://irds.ieee.org/images/files/pdf/2023/2023IRDS_Litho.pdf)): stochastics,
  dose-to-print trend, EUV sources, masks and pellicles, metal-oxide resists.
- P. De Bisschop, "Stochastic effects in EUV lithography: random, local CD variability, and
  printing failures," *J. Micro/Nanolith. MEMS MOEMS* **16**(4), 041013 (2017),
  [doi:10.1117/1.JMM.16.4.041013](https://doi.org/10.1117/1.JMM.16.4.041013).
- C. A. Mack, "Shot noise: a 100-year history, with applications to lithography,"
  *J. Micro/Nanolith. MEMS MOEMS* **17**(4), 041002 (2018),
  [doi:10.1117/1.JMM.17.4.041002](https://doi.org/10.1117/1.JMM.17.4.041002).
- R. Fallica et al., "Absorption coefficient of metal-containing photoresists in the extreme
  ultraviolet," *J. Micro/Nanolith. MEMS MOEMS* **17**(2), 023505 (2018),
  [doi:10.1117/1.JMM.17.2.023505](https://doi.org/10.1117/1.JMM.17.2.023505).
- Fractilia, "Closing the Stochastics Resolution Gap" (white paper, July 2025,
  [pdf](https://www.fractilia.com/s/Closing-the-Stochastics-Resolution-Gap-White-Paper_Final.pdf)).
- *Semiconductor Engineering*: M. LaPedus, "EUV's New Problem Areas" (March 2018,
  [link](https://semiengineering.com/euvs-new-problem-areas/)), "EUV Pellicle, Uptime And
  Resist Issues Continue" (26 September 2018,
  [link](https://semiengineering.com/euv-pellicle-uptime-and-resist-issues-continue/)); articles
  on EUV mask making (October 2019,
  [link](https://semiengineering.com/mask-making-issues-with-euv/)) and mask protection
  (November 2019, [link](https://semiengineering.com/making-and-protecting-advanced-masks/)).
- Mitsui Chemicals, pellicle licence (31 May 2019,
  [link](https://jp.mitsuichemicals.com/en/release/2019/2019_0531_01/index.htm)) and start of
  commercial production (26 May 2021,
  [link](https://jp.mitsuichemicals.com/en/release/2021/2021_0526/index.htm)).
- V. Wiaux, V. Philipsen, E. Hendrickx (imec), presentation on EUV mask 3D effects, EUVL
  Workshop 2018 ([pdf](https://www.euvlitho.com/2018/P62.pdf)); I. Fomenkov (ASML), presentation
  at the 2017 EUV Source Workshop ([pdf](https://www.euvlitho.com/2017/S1.pdf)), 250 W source.
- Lam Research, "[Lam Research Unveils Technology Breakthrough for EUV Lithography](https://newsroom.lamresearch.com/2020-02-26-Lam-Research-Unveils-Technology-Breakthrough-for-EUV-Lithography)"
  (dry resist, 26 February 2020); JSR, "[JSR Closes Deal to Acquire EUV Pioneer Inpria Corporation](https://www.jsrmicro.be/news/jsr-closes-deal-to-acquire-euv-pioneer-inpria-corporation)"
  (8 November 2021; acquisition completed 29 October 2021).
- Lam Research, press release on 28 nm pitch direct print with dry resist, 14 January 2025
  ([link](https://newsroom.lamresearch.com/2025-01-14-Lam-Research-Establishes-28nm-Pitch-in-High-Resolution-Patterning-Through-Dry-Photoresist-Technology)).
- ASML product pages: [NXE:3400B](https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe3400b),
  [NXE:3400C](https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe3400c),
  [NXT:2000i](https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt2000i),
  [NXT:1980Fi](https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt1980fi)
  (retrieved 2026-09-30).
- Wikipedia: "[7 nm process](https://en.wikipedia.org/wiki/7_nm_process)",
  "[5 nm process](https://en.wikipedia.org/wiki/5_nm_process)" and
  "[EUV lithography](https://en.wikipedia.org/wiki/EUV_lithography)" (chronology, tabulated
  pitches, EUV double-patterning onset).
- H. J. Levinson, *Extreme Ultraviolet Lithography* (SPIE Press, 2020),
  [doi:10.1117/3.2581446](https://doi.org/10.1117/3.2581446).
